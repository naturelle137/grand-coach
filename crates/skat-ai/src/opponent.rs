// SPDX-License-Identifier: EUPL-1.2

//! Opponents with strength levels (M4, D-07, D-40, D-41).
//!
//! A level is a [`Profile`]: how much analysis effort the opponent spends,
//! how much it deviates from the best option, how well it remembers played
//! cards and which mistakes it makes. Every strategic number comes from
//! `skat-analysis`; this module only picks one of the analysed options or
//! follows the shared rules of thumb ([`BaselineOpponent`]).
//!
//! Information boundary: [`Opponent::decide`] takes one seat's
//! [`PlayerView`] and nothing else (D-23). Its randomness comes from the
//! opponent's own seed and the view, never from the deal seed, so the same
//! view, seed and profile give the same decision. The only opponent with
//! full knowledge is [`crate::Mephisto`].

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_analysis::{
    Analysis, Budget, Cancel, Kernel, OptionValue, StageReport, Status, TwoStage,
    analyze_forgetful, analyze_two_stage, utility,
};
use skat_core::{Action, CardSet, KontraLevel, Phase, PlayerView};

use crate::BaselineOpponent;
use crate::rookie::{self, Mistakes};

/// Strength level of an opponent, from weakest to strongest. Goldfish and
/// Mephisto are table modes, not levels (`skat-app`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Level {
    /// Rules of thumb with seeded mistakes.
    Rookie,
    /// Rules of thumb without search.
    Easy,
    /// Analysed card play and Re with a small budget; Kontra by the rules
    /// of thumb.
    #[default]
    Medium,
    /// Analysed card play and Re with the interactive budget; Kontra by
    /// the rules of thumb.
    Strong,
}

impl Level {
    /// All levels from weakest to strongest.
    pub const ALL: [Level; 4] = [Level::Rookie, Level::Easy, Level::Medium, Level::Strong];

    /// The documented profile of this level (`docs/ai.md`).
    pub const fn profile(self) -> Profile {
        match self {
            Level::Rookie => Profile {
                mistakes: Profile::ROOKIE_MISTAKES,
                ..Profile::RULES
            },
            Level::Easy => Profile::RULES,
            Level::Medium => Profile {
                budget: Some(Budget {
                    samples: 6,
                    nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
                }),
                ..Profile::RULES
            },
            Level::Strong => Profile {
                budget: Some(Budget::INTERACTIVE),
                ..Profile::RULES
            },
        }
    }
}

/// What makes a level strong or weak.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Profile {
    /// Analysis budget; `None` plays the rules of thumb of
    /// [`BaselineOpponent`] without search.
    pub budget: Option<Budget>,
    /// Selection temperature in Seeger-Fabian points: an option is chosen
    /// with probability proportional to `exp(utility / temperature)`, so
    /// options a few points worse than the best are chosen now and then.
    /// 0 always chooses the best option.
    pub temperature: f64,
    /// Whether the analysis decides the bidding; otherwise the rules of
    /// thumb do.
    pub search_bidding: bool,
    /// Whether the analysis decides Hand or pickup, Drücken and the
    /// announcement; otherwise the rules of thumb do. Card play and Re are
    /// always analysed when there is a budget.
    pub search_declaring: bool,
    /// Whether the analysis decides Kontra; otherwise the rules of thumb
    /// do and the analysis only chooses the card when they say no Kontra.
    /// The double-dummy analysis is pessimistic for the declarer and says
    /// Kontra almost always (`docs/ai.md`).
    pub search_kontra: bool,
    /// Probability that a card of a completed trick other than the last
    /// one is forgotten in an analysed card-play decision (it is then
    /// believed to be still out, `skat_analysis::analyze_forgetful`).
    pub forget: f64,
    /// Seeded deviations from the rules of thumb (decisions the analysis
    /// does not take).
    pub mistakes: Mistakes,
}

impl Profile {
    /// Plain rules of thumb (level Easy).
    pub const RULES: Profile = Profile {
        budget: None,
        temperature: 0.0,
        search_bidding: false,
        search_declaring: false,
        search_kontra: false,
        forget: 0.0,
        mistakes: Mistakes::NONE,
    };

    /// Mistakes of level Rookie.
    pub const ROOKIE_MISTAKES: Mistakes = Mistakes {
        bid: 0.35,
        game: 0.4,
        discard: 0.2,
        kontra: 0.15,
        card: 0.12,
    };

    /// The forgetful opponent of the Goldfish table mode: Strong, but every
    /// card of an older trick is forgotten with probability 1/2.
    pub const GOLDFISH: Profile = Profile {
        budget: Some(Budget::INTERACTIVE),
        forget: 0.5,
        ..Profile::RULES
    };
}

/// An automated player with a profile and its own seed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opponent {
    /// How the opponent decides.
    pub profile: Profile,
    /// Seed of the opponent's randomness (sampling, selection, forgetting
    /// and mistakes). It must not be derived from the deal seed.
    pub seed: u64,
}

/// A decision and how it was reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decision {
    /// The chosen legal action.
    pub action: Action,
    /// Status of the analysis; `None` if none ran (rules of thumb, or the
    /// view was rejected).
    pub status: Option<Status>,
    /// `true` if the rules of thumb decided because the analysis gave no
    /// estimate (cancelled before any distribution finished, or an
    /// inconsistent view). Always a legal action.
    pub fallback: bool,
}

impl Opponent {
    /// An opponent of `level` with `seed`.
    pub const fn new(level: Level, seed: u64) -> Opponent {
        Opponent {
            profile: level.profile(),
            seed,
        }
    }

    /// Chooses one of `view.legal_actions`; `None` if the list is empty,
    /// i.e. it is not this seat's turn. `cancel` stops the analysis early
    /// (M3 semantics): the choice then uses the distributions finished so
    /// far, or falls back to the rules of thumb if there are none.
    pub fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        self.decide_analysed(view, cancel).map(|(d, _)| d)
    }

    /// [`Opponent::decide`] together with the analysis the decision was
    /// taken from; `None` as analysis if none ran or it failed (rules of
    /// thumb). Training data record these option values (#70).
    pub fn decide_analysed(
        &self,
        view: &PlayerView,
        cancel: Option<&Cancel>,
    ) -> Option<(Decision, Option<Analysis>)> {
        self.decide_with(view, cancel, None).map(|(d, a, _)| (d, a))
    }

    /// Experimental, opt-in (#104): [`Opponent::decide_analysed`] with the
    /// two-stage card-play analysis of `skat-analysis`
    /// ([`analyze_two_stage`], classic kernel) and its stage report. No
    /// shipped level uses it. The decision is always one of the report's
    /// survivors. Decisions the screening excludes are analysed
    /// exactly like the single-stage path (reason in the report); a
    /// forgetful profile with forgotten cards uses the single-stage
    /// forgetful analysis and reports `None`. For paired measurements
    /// against [`Opponent::decide_analysed`] only.
    pub fn decide_two_stage(
        &self,
        view: &PlayerView,
        cancel: Option<&Cancel>,
        screening: TwoStage,
    ) -> Option<(Decision, Option<Analysis>, Option<StageReport>)> {
        self.decide_with(view, cancel, Some(screening))
    }

    fn decide_with(
        &self,
        view: &PlayerView,
        cancel: Option<&Cancel>,
        screening: Option<TwoStage>,
    ) -> Option<(Decision, Option<Analysis>, Option<StageReport>)> {
        let mut rules = BaselineOpponent.decide(view)?;
        if !self.profile.mistakes.is_none() {
            let game_seed = mix(mix(self.seed, u64::from(view.hand.bits())), 7);
            let seed = mix(decision_seed(self.seed, view), 3);
            rules = rookie::decide(view, rules, &self.profile.mistakes, game_seed, seed);
        }
        let kontra_by_rules = !self.profile.search_kontra && offers_kontra(view);
        if kontra_by_rules && is_kontra_choice(rules) {
            let decision = Decision {
                action: rules,
                status: None,
                fallback: false,
            };
            return Some((decision, None, None));
        }
        let Some((budget, seed)) = self.analysis_input(view) else {
            let decision = Decision {
                action: rules,
                status: None,
                fallback: false,
            };
            return Some((decision, None, None));
        };
        let forgotten = self.forgotten(view);
        let analysed = match screening {
            Some(screening) if forgotten.is_empty() => {
                analyze_two_stage(view, budget, seed, cancel, Kernel::Classic, screening)
                    .map(|t| (t.analysis, Some(t.report)))
            }
            _ => analyze_forgetful(view, forgotten, budget, seed, cancel).map(|a| (a, None)),
        };
        let Ok((analysis, report)) = analysed else {
            let decision = Decision {
                action: rules,
                status: None,
                fallback: true,
            };
            return Some((decision, None, None));
        };
        let mut rng = ChaCha8Rng::seed_from_u64(mix(seed, 2));
        // Screened-out options are never candidates (#104): with a single
        // survivor every option keeps the stage-1 sample count, so the
        // sample filter in `select` alone could pick a discarded action.
        let screened = screened_options(&analysis.options, report.as_ref());
        let options = if kontra_by_rules {
            without_kontra(&screened)
        } else {
            screened
        };
        let decision = decision(
            select(view, &options, rules, self.profile.temperature, &mut rng),
            rules,
            analysis.status,
        );
        Some((decision, Some(analysis), report))
    }

    /// Budget and sampling seed the opponent passes to the analysis
    /// together with `view` (and [`Opponent::forgotten`]), or `None` if it
    /// decides by the rules of thumb. Both depend only on the profile, the
    /// opponent's seed and the view.
    pub fn analysis_input(&self, view: &PlayerView) -> Option<(Budget, u64)> {
        if view.legal_actions.is_empty() {
            return None;
        }
        let searched = match view.phase {
            Phase::Reizen => self.profile.search_bidding,
            Phase::SkatAufnahme | Phase::Druecken | Phase::Ansage => self.profile.search_declaring,
            _ => true,
        };
        let budget = self.profile.budget.filter(|_| searched)?;
        Some((budget, mix(decision_seed(self.seed, view), 1)))
    }

    /// Played cards the opponent does not remember in this decision: each
    /// card of a completed trick before the last one, with probability
    /// [`Profile::forget`]. Empty outside card play.
    pub fn forgotten(&self, view: &PlayerView) -> CardSet {
        let mut out = CardSet::EMPTY;
        if self.profile.forget <= 0.0 || view.phase != Phase::Spiel || view.tricks.len() < 2 {
            return out;
        }
        let mut rng = ChaCha8Rng::seed_from_u64(mix(decision_seed(self.seed, view), 4));
        for trick in &view.tricks[..view.tricks.len() - 1] {
            for &card in &trick.cards {
                if unit(&mut rng) < self.profile.forget {
                    out.insert(card);
                }
            }
        }
        out
    }
}

/// Temporary fair opponent using exact experimental bounds. The sample
/// budget is explicit so the latency and calibration tools exercise the same path.
#[derive(Clone, Copy, Debug)]
pub struct ExperimentalOpponent {
    /// Independent teacher seed.
    pub seed: u64,
    /// World budget per card-play decision.
    pub samples: u32,
}

impl ExperimentalOpponent {
    /// Default experimental budget for interactive play.
    pub const SAMPLES: u32 = 24;

    /// Final decision with Strong's normal rules, tie breaking and node budget.
    pub fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        let strong = Opponent::new(Level::Strong, self.seed);
        let Some((mut budget, seed)) = strong.analysis_input(view) else {
            return strong.decide(view, cancel);
        };
        budget.samples = self.samples;
        let rules = BaselineOpponent.decide(view)?;
        if is_kontra_choice(rules) {
            return Some(Decision {
                action: rules,
                status: None,
                fallback: false,
            });
        }
        let Ok(a) = skat_analysis::analyze_experimental(view, budget, seed, cancel, None) else {
            return Some(Decision {
                action: rules,
                status: None,
                fallback: true,
            });
        };
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        Some(decision(
            select(view, &without_kontra(&a.options), rules, 0.0, &mut rng),
            rules,
            a.status,
        ))
    }
}

/// The decision for a selected option, or the rules' fallback.
pub(crate) fn decision(selected: Option<Action>, rules: Action, status: Status) -> Decision {
    match selected {
        Some(action) => Decision {
            action,
            status: Some(status),
            fallback: false,
        },
        None => Decision {
            action: rules,
            status: Some(status),
            fallback: true,
        },
    }
}

/// Uniform number in [0, 1).
fn unit(rng: &mut ChaCha8Rng) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

/// Bonus in points for the rules-of-thumb action. Double-dummy values only
/// resolve scoring classes, so many options tie exactly; the bonus breaks
/// such ties towards the sensible card instead of the first listed one.
const RULES_BONUS: f64 = 0.5;

/// Whether `action` is a defender's Kontra choice (Kontra, or KeinKontra in
/// the Ouvert round).
pub(crate) fn is_kontra_choice(action: Action) -> bool {
    matches!(action, Action::Kontra | Action::KeinKontra)
}

/// Whether the seat may say Kontra in this decision.
pub(crate) fn offers_kontra(view: &PlayerView) -> bool {
    view.legal_actions.iter().any(|&a| is_kontra_choice(a))
}

/// `options` without the Kontra choices.
/// The options a stage report kept, or all of them if nothing was
/// screened out (or no two-stage analysis ran).
fn screened_options(options: &[OptionValue], report: Option<&StageReport>) -> Vec<OptionValue> {
    match report {
        Some(r) if r.survivors.len() < options.len() => r
            .survivors
            .iter()
            .filter_map(|&i| options.get(i).cloned())
            .collect(),
        _ => options.to_vec(),
    }
}

pub(crate) fn without_kontra(options: &[OptionValue]) -> Vec<OptionValue> {
    options
        .iter()
        .filter(|o| !is_kontra_choice(o.action))
        .cloned()
        .collect()
}

/// How a seat decides when the rules of thumb say Kontra (#108).
#[derive(Clone, Debug, PartialEq)]
pub enum KontraRule {
    /// The decision offers no Kontra: decide the view as it is.
    Free,
    /// The rules of thumb take this Kontra choice (Kontra, or KeinKontra
    /// in the Ouvert round).
    Rules(Action),
    /// The rules say no Kontra: choose among the other actions of this
    /// view (the original with the Kontra choices removed).
    Without(PlayerView),
}

/// [`KontraRule`] for `view`.
pub fn kontra_rule(view: &PlayerView) -> KontraRule {
    if !offers_kontra(view) {
        return KontraRule::Free;
    }
    match BaselineOpponent.decide(view) {
        Some(rules) if is_kontra_choice(rules) => KontraRule::Rules(rules),
        _ => KontraRule::Without(PlayerView {
            legal_actions: view
                .legal_actions
                .iter()
                .copied()
                .filter(|&a| !is_kontra_choice(a))
                .collect(),
            ..view.clone()
        }),
    }
}

/// The action an analysed opponent (temperature 0, Kontra by the rules of
/// thumb) takes in `view` given the option values `options` of an analysis
/// of `view`: the rules' Kontra choice if they say Kontra (or KeinKontra
/// in the Ouvert round), otherwise the best other option. Training relabels
/// recorded teacher decisions with it (#108). `None` if no option to choose
/// has an estimate or it is not this seat's turn.
pub fn kontra_by_rules(view: &PlayerView, options: &[OptionValue]) -> Option<Action> {
    let rules = BaselineOpponent.decide(view)?;
    if is_kontra_choice(rules) {
        return Some(rules);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    select(view, &without_kontra(options), rules, 0.0, &mut rng)
}

/// Picks one of `options` (option values of an analysis). Only options
/// evaluated on the most distributions compete (after Drücken screening,
/// the kept ones), so a lucky estimate from few distributions cannot win.
/// `None` if no option has an estimate.
pub(crate) fn select(
    view: &PlayerView,
    options: &[OptionValue],
    rules: Action,
    temperature: f64,
    rng: &mut ChaCha8Rng,
) -> Option<Action> {
    let most = options
        .iter()
        .filter_map(|o| o.estimate.map(|e| e.samples))
        .max()?;
    let candidates: Vec<(Action, f64)> = options
        .iter()
        .filter_map(|o| match o.estimate {
            Some(e) if e.samples == most => {
                let bonus = if o.action == rules { RULES_BONUS } else { 0.0 };
                Some((o.action, utility(view, &e) + bonus))
            }
            _ => None,
        })
        .collect();
    let best = candidates
        .iter()
        .map(|&(_, u)| u)
        .fold(f64::NEG_INFINITY, f64::max);
    if temperature <= 0.0 {
        return candidates
            .iter()
            .find(|&&(_, u)| u == best)
            .map(|&(a, _)| a);
    }
    let weights: Vec<f64> = candidates
        .iter()
        .map(|&(_, u)| ((u - best) / temperature).exp())
        .collect();
    let total: f64 = weights.iter().sum();
    let mut x = unit(rng) * total;
    for (&(action, _), w) in candidates.iter().zip(&weights) {
        if x < *w {
            return Some(action);
        }
        x -= w;
    }
    candidates.last().map(|&(a, _)| a)
}

/// Seed of one decision: the opponent's seed mixed with public facts of the
/// view that change from decision to decision. It depends only on the view.
fn decision_seed(seed: u64, view: &PlayerView) -> u64 {
    let phase = match view.phase {
        Phase::Reizen => 0,
        Phase::SkatAufnahme => 1,
        Phase::Druecken => 2,
        Phase::Ansage => 3,
        Phase::Spiel => 4,
        Phase::Beendet => 5,
        Phase::Eingepasst => 6,
    };
    let kontra = match view.kontra {
        KontraLevel::None => 0,
        KontraLevel::Kontra => 1,
        KontraLevel::Re => 2,
    };
    let facts = [
        view.seat.index() as u64,
        phase,
        u64::from(view.hand.bits()),
        view.bidding.len() as u64,
        view.tricks.len() as u64,
        view.current_trick.len() as u64,
        kontra,
        view.legal_actions.len() as u64,
    ];
    facts.iter().fold(seed, |h, &f| mix(h, f))
}

/// Mixes `value` into `hash` (SplitMix64 finaliser).
pub(crate) fn mix(hash: u64, value: u64) -> u64 {
    let mut z = hash ^ value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
