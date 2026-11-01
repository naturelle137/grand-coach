// SPDX-License-Identifier: EUPL-1.2

//! Option evaluation for every decision: Monte Carlo over the seat's
//! information set, double-dummy card play and the continuation policy.
//!
//! [`analyze`] takes one seat's [`PlayerView`] and nothing else (D-23). For
//! each hypothetical distribution of the hidden cards ([`World`]) it plays
//! every legal option to the end of the game:
//!
//! - the option itself is the only enumerated decision,
//! - every later decision before card play (bidding, Hand or pickup,
//!   Drücken, announcement, Kontra/Re) follows [`crate::policy`] with the
//!   deciding seat's own cards in that distribution,
//! - card play is double-dummy ([`crate::double_dummy`]),
//! - the finished game is settled by `skat-core`, including Überreizen and
//!   Kontra/Re.
//!
//! Results are averaged with the bidding-evidence weights
//! ([`crate::evidence`]). If the number of distributions does not exceed
//! the sample budget, all of them are enumerated and the result is marked
//! exact (exact over the hidden cards, under the same model). See
//! `docs/analysis.md` for semantics and budgets.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_core::{
    Action, Call, Card, CardSet, Deal, Follow, Game, GameResult, GameType, KontraLevel, Phase,
    PlayerView, Seat, kontra_eligible, spitzen,
};

use crate::budget::{Budget, Cancel, Interrupted, Meter};
use crate::double_dummy::{DdValue, Position, Solver, Stakes};
use crate::evidence;
use crate::knowledge::{Knowledge, TrumpfSets};
use crate::policy;
use crate::preplay::{self, GameValues, Structure};
use crate::sampling::{Diagnostic, HiddenCards, Place, World};

/// Result of analysing one decision of one seat.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Analysis {
    /// The deciding seat; every estimate is from its point of view.
    pub seat: Seat,
    /// Phase of the decision.
    pub phase: Phase,
    /// Whether the evaluation finished.
    pub status: Status,
    /// Every legal action, in the order of `PlayerView::legal_actions`.
    pub options: Vec<OptionValue>,
    /// Exact facts the seat knows (card counting).
    pub observations: Vec<Observation>,
    /// Model assumptions behind the estimates.
    pub assumptions: Vec<Assumption>,
    /// What the weighted distributions say about hidden cards (opponent
    /// model): inferred, not known, unless marked certain.
    pub inferences: Vec<Inference>,
    /// Work done and uncertainty.
    pub effort: Effort,
}

/// Whether an analysis finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Status {
    /// Every planned distribution was evaluated.
    Complete,
    /// The node budget ran out; estimates use only finished distributions.
    BudgetExhausted,
    /// Cancelled; estimates use only finished distributions.
    Cancelled,
    /// It is not this seat's turn: nothing to evaluate.
    NoDecision,
}

/// One legal action and its value.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionValue {
    /// The action.
    pub action: Action,
    /// Its value; `None` if no distribution finished.
    pub estimate: Option<Estimate>,
}

/// Value of an option for the deciding seat, averaged over distributions.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Estimate {
    /// Probability that the deciding seat's party wins the game (as
    /// declarer: the declarer wins; as defender: the declarer loses). A
    /// thrown-in deal counts as not won.
    pub win_chance: f64,
    /// Expected Seeger-Fabian points of the deciding seat for this game
    /// (D-05; 0 for a thrown-in deal).
    pub expected_score: f64,
    /// Expected game points of the declarer's list entry
    /// ([`GameResult::score`]: positive if won, doubled negative if lost,
    /// with Kontra/Re), whoever the declarer is; 0 if thrown in.
    pub expected_game_points: f64,
    /// Probability that the deal is thrown in.
    pub thrown_in: f64,
    /// Standard error of `expected_score` (0 for exact results).
    pub score_std_error: f64,
    /// Distributions this option was evaluated on (fewer than planned for
    /// options dropped after screening).
    pub samples: u32,
    /// `true` if every distribution was evaluated (exact under the model);
    /// `false` for a Monte Carlo estimate.
    pub exact: bool,
}

/// A fact the seat knows exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Observation {
    /// The seat cannot hold any card of this class (did not follow suit).
    Void {
        /// The seat.
        seat: Seat,
        /// Trumpf or the side suit.
        follow: Follow,
    },
    /// Trumps neither played nor held by the deciding seat.
    TrumpsOut {
        /// Number of such trumps.
        count: u8,
    },
    /// Where the Trumpf are as far as the deciding seat knows: own, played,
    /// own Skat, revealed Ouvert cards and unseen, kept apart. Farbspiel
    /// and Grand only. [`Observation::TrumpsOut`] keeps its meaning.
    TrumpfSets(TrumpfSets),
    /// Augen won in completed tricks (without the Skat).
    AugenWon {
        /// The declarer.
        declarer: u8,
        /// The defenders together.
        defenders: u8,
    },
    /// Augen neither played nor held by the deciding seat.
    AugenOut {
        /// Their sum.
        augen: u8,
    },
    /// The deciding seat knows the Skat (its own discard).
    SkatKnown {
        /// The two cards.
        cards: CardSet,
    },
    /// Highest value a seat bid or held.
    BidShown {
        /// The seat.
        seat: Seat,
        /// The value.
        value: u16,
    },
    /// A seat may still say Kontra (D-26).
    KontraEligible {
        /// The seat.
        seat: Seat,
    },
    /// The own Buben, Trumpf and Spitzen of one candidate Farbspiel or
    /// Grand (bidder or declarer before card play; [`crate::preplay`]).
    Structure(Structure),
    /// The game values one game type can have with the own cards (bidder or
    /// declarer before card play; [`crate::preplay`]).
    GameValues(GameValues),
    /// The game must be worth at least `value`, or it is lost as überreizt
    /// (ISkO 5.4): the bid reached, or during bidding the lowest value an
    /// own Bid or Hold commits to ([`crate::preplay::commitment`]).
    Commitment {
        /// The value.
        value: u16,
        /// Kontra/Re is configured (D-03): it multiplies the list points,
        /// not the game value.
        kontra_re: bool,
    },
}

/// A model assumption behind the estimates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Assumption {
    /// Hidden cards are distributed uniformly within the hard constraints.
    UniformHiddenCards,
    /// Distributions are weighted by bidding evidence (`evidence`).
    BiddingEvidence,
    /// Card play is double-dummy: every seat plays as if it saw all cards.
    DoubleDummyPlay,
    /// Later decisions before card play follow the continuation policy.
    ContinuationPolicy,
    /// A bid option `Bid(v)` means: say `v` now and go on bidding up to `v`;
    /// `Hold` means hold now and go on up to the policy limit or the
    /// current bid, whichever is higher.
    BidAsLimit,
    /// The options are evaluated in the real distribution of every card
    /// ([`analyze_known`]); replaces [`Assumption::UniformHiddenCards`] and
    /// [`Assumption::BiddingEvidence`].
    KnownDeal,
}

/// A class of cards whose whereabouts the opponent model estimates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardClass {
    /// The trumps of the announced suit game or Grand.
    Trumps,
    /// The four Jacks (Buben), before a game is announced.
    Jacks,
}

/// How many cards of a class a hidden place holds, over the weighted
/// distributions the estimates were computed on (finished ones only).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inference {
    /// A seat other than the deciding one, or the Skat.
    pub holder: Place,
    /// The class of cards.
    pub class: CardClass,
    /// Weighted mean number of such cards there.
    pub expected: f64,
    /// Fewest such cards there in any of these distributions.
    pub min: u8,
    /// Most such cards there in any of these distributions.
    pub max: u8,
    /// `true` only if the hard constraints prove the count: the place may
    /// hold none of these cards (a proven void), or every consistent
    /// distribution was enumerated and all agree (`min == max`). Otherwise
    /// the numbers are an estimate.
    pub certain: bool,
}

/// Work done and uncertainty.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Effort {
    /// Distributions planned (budget samples, or all if enumerated).
    pub planned: u32,
    /// Distributions evaluated completely.
    pub finished: u32,
    /// Number of distributions consistent with the hard constraints.
    pub distributions: f64,
    /// `true` if all distributions were enumerated instead of sampled.
    pub enumerated: bool,
    /// Effective sample size of the evidence weights over finished
    /// distributions.
    pub effective_samples: f64,
    /// Search nodes used.
    pub nodes: u64,
    /// The seed used for sampling.
    pub seed: u64,
    /// `true` if the options were evaluated in one known distribution
    /// ([`analyze_known`]) instead of the seat's information set.
    pub known: bool,
}

/// Evaluates every legal action of `view.seat` with `budget`, sampling with
/// `seed`. Fails with diagnostics if the view is inconsistent.
pub fn analyze(
    view: &PlayerView,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
) -> Result<Analysis, Vec<Diagnostic>> {
    analyze_with(view, budget, seed, cancel, Worlds::Sampled)
}

/// Like [`analyze`], but the seat does not remember where the `forgotten`
/// cards went (cards of completed tricks): in every sampled distribution
/// each forgotten card takes the place of a randomly chosen hidden card,
/// which counts as played instead. The seat thus believes some played
/// cards are still out. Only card play is affected; other phases behave
/// like [`analyze`]. Results are never marked exact.
pub fn analyze_forgetful(
    view: &PlayerView,
    forgotten: CardSet,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
) -> Result<Analysis, Vec<Diagnostic>> {
    let played: CardSet = view.tricks.iter().flat_map(|t| t.cards).collect();
    let forgotten = forgotten & played;
    if view.phase != Phase::Spiel || forgotten.is_empty() {
        return analyze(view, budget, seed, cancel);
    }
    analyze_with(view, budget, seed, cancel, Worlds::Forgetful(forgotten))
}

/// Evaluates every legal action of `view.seat` in one known distribution
/// of the hidden cards, exactly (one distribution, enumerated). With the
/// real distribution ([`World::from_omniscient`]) this is full knowledge;
/// only for uses that are allowed to see every card. Fails if `world` does
/// not fit the view. The inferences stay those of the seat's own
/// information set (what it could deduce without seeing the cards,
/// `budget.samples` distributions with seed 0); the estimates are marked
/// [`Effort::known`] and the assumptions name [`Assumption::KnownDeal`].
pub fn analyze_known(
    view: &PlayerView,
    world: World,
    budget: Budget,
    cancel: Option<&Cancel>,
) -> Result<Analysis, Vec<Diagnostic>> {
    analyze_with(view, budget, 0, cancel, Worlds::Known(world))
}

/// Where the distributions of an analysis come from.
enum Worlds {
    /// Sampled (or enumerated) from the seat's information set.
    Sampled,
    /// Sampled, then forgotten cards swapped in.
    Forgetful(CardSet),
    /// One given distribution.
    Known(World),
}

fn analyze_with(
    view: &PlayerView,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
    source: Worlds,
) -> Result<Analysis, Vec<Diagnostic>> {
    let hidden = HiddenCards::from_view(view)?;
    let ctx = Context::new(view)?;
    let observations = observations(view);
    let known = matches!(source, Worlds::Known(_));
    let assumptions = assumptions(view.phase, known);
    let distributions = hidden.count() as f64;
    if view.legal_actions.is_empty() {
        return Ok(Analysis {
            seat: view.seat,
            phase: view.phase,
            status: Status::NoDecision,
            options: Vec::new(),
            observations,
            assumptions,
            inferences: Vec::new(),
            effort: Effort {
                planned: 0,
                finished: 0,
                distributions,
                enumerated: false,
                effective_samples: 0.0,
                nodes: 0,
                seed,
                known,
            },
        });
    }

    let samples = budget.samples.max(1) as usize;
    let (worlds, enumerated) = match source {
        Worlds::Known(world) => {
            if !hidden.admits(&world) {
                return Err(vec![Diagnostic::NoConsistentDeal]);
            }
            (vec![world], true)
        }
        Worlds::Sampled | Worlds::Forgetful(_) => {
            let (mut worlds, enumerated) = own_worlds(&hidden, samples, seed);
            if let Worlds::Forgetful(forgotten) = source {
                let mut rng = ChaCha8Rng::seed_from_u64(seed ^ FORGET_STREAM);
                let voids = Knowledge::from_view(view).voids;
                for world in &mut worlds {
                    forget(world, &hidden, &voids, forgotten, &mut rng);
                }
                (worlds, false)
            } else {
                (worlds, enumerated)
            }
        }
    };
    if view.phase != Phase::Spiel && ctx.replay(&worlds[0]).is_none() {
        return Err(vec![Diagnostic::InvalidHistory]);
    }
    let n_options = view.legal_actions.len();
    let nodes = budget.nodes_per_task;
    // Drücken has 66 options, each a different deal to solve; other phases
    // share solved positions between options and evaluate all of them.
    let staged = view.phase == Phase::Druecken && worlds.len() > SCREEN_WORLDS;
    let mut table = Table::new(worlds.len(), n_options);
    let all: Vec<usize> = (0..n_options).collect();
    if staged {
        // Screening: every option on the first distributions, one task per
        // option; then the best options on the remaining distributions.
        let tasks: Vec<Task> = (0..SCREEN_WORLDS)
            .flat_map(|w| (0..n_options).map(move |o| (w, vec![o])))
            .collect();
        table.run(&ctx, &worlds, &tasks, nodes, cancel);
        let weights: Vec<f64> = worlds.iter().map(|w| evidence::weight(view, w)).collect();
        let mut ranked: Vec<(usize, f64)> = all
            .iter()
            .map(|&o| {
                let e = table.estimate(&weights, o, false);
                (o, e.map_or(f64::NEG_INFINITY, |e| e.expected_score))
            })
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let keep: Vec<usize> = ranked.iter().take(KEEP_OPTIONS).map(|&(o, _)| o).collect();
        let tasks: Vec<Task> = (SCREEN_WORLDS..worlds.len())
            .flat_map(|w| keep.iter().map(move |&o| (w, vec![o])))
            .collect();
        table.run(&ctx, &worlds, &tasks, nodes, cancel);
    } else if worlds.len() == 1 && view.phase != Phase::Spiel {
        // One distribution before card play: one task per option, so the
        // options are solved in parallel.
        let tasks: Vec<Task> = (0..n_options).map(|o| (0, vec![o])).collect();
        table.run(&ctx, &worlds, &tasks, nodes, cancel);
    } else {
        let tasks: Vec<Task> = (0..worlds.len()).map(|w| (w, all.clone())).collect();
        table.run(&ctx, &worlds, &tasks, nodes, cancel);
    }
    let weights: Vec<f64> = worlds
        .iter()
        .zip(&table.complete)
        .map(|(w, &ok)| if ok { evidence::weight(view, w) } else { 0.0 })
        .collect();
    let exact = enumerated && table.status == Status::Complete && !staged;
    let options = view
        .legal_actions
        .iter()
        .enumerate()
        .map(|(i, &action)| OptionValue {
            action,
            estimate: table.estimate(&weights, i, exact),
        })
        .collect();
    let total: f64 = weights.iter().sum();
    let squares: f64 = weights.iter().map(|w| w * w).sum();
    let inferences = if known {
        // What the seat could deduce from its own view, not the known deal.
        let (own, enumerated) = own_worlds(&hidden, samples, seed);
        let weights: Vec<f64> = own.iter().map(|w| evidence::weight(view, w)).collect();
        inferences(view, &hidden, &own, &weights, enumerated)
    } else {
        inferences(view, &hidden, &worlds, &weights, enumerated)
    };
    Ok(Analysis {
        seat: view.seat,
        phase: view.phase,
        status: table.status,
        options,
        observations,
        assumptions,
        inferences,
        effort: Effort {
            planned: worlds.len() as u32,
            finished: table.complete.iter().filter(|&&c| c).count() as u32,
            distributions,
            enumerated,
            effective_samples: if squares > 0.0 {
                total * total / squares
            } else {
                0.0
            },
            nodes: table.nodes,
            seed,
            known,
        },
    })
}

/// Distributions of the seat's information set: all of them if there are
/// at most `samples`, else `samples` drawn with `seed`; and whether they
/// were enumerated.
fn own_worlds(hidden: &HiddenCards, samples: usize, seed: u64) -> (Vec<World>, bool) {
    match hidden.enumerate(samples) {
        Some(all) => (all, true),
        None => {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            (
                (0..samples).map(|_| hidden.sample(&mut rng)).collect(),
                false,
            )
        }
    }
}

/// Stream of the random choices that place forgotten cards.
const FORGET_STREAM: u64 = 0x676f_6c64_6669_7368;

/// Swaps every forgotten card into the place of a random hidden card of
/// `world` (a hand or Skat with a slot, never one already swapped in, and
/// never a hand proven void in the card's suit, so the position stays
/// playable).
fn forget(
    world: &mut World,
    hidden: &HiddenCards,
    voids: &[CardSet; 3],
    forgotten: CardSet,
    rng: &mut ChaCha8Rng,
) {
    let mut swapped_in = CardSet::EMPTY;
    for card in forgotten {
        let mut candidates: Vec<(Place, Card)> = Vec::new();
        for slot in hidden.slots() {
            let cards = match slot.place {
                Place::Seat(seat) if voids[seat.index()].contains(card) => continue,
                Place::Seat(seat) => world.hands[seat.index()],
                Place::Skat => world.skat,
            };
            candidates.extend((cards - swapped_in).into_iter().map(|c| (slot.place, c)));
        }
        if candidates.is_empty() {
            continue;
        }
        let (place, out) = candidates[(rng.next_u64() % candidates.len() as u64) as usize];
        let target = match place {
            Place::Seat(seat) => &mut world.hands[seat.index()],
            Place::Skat => &mut world.skat,
        };
        target.remove(out);
        target.insert(card);
        swapped_in.insert(card);
    }
}

/// Distributions every Drücken option is evaluated on when screening.
const SCREEN_WORLDS: usize = 3;
/// Drücken options evaluated on the remaining distributions after
/// screening.
const KEEP_OPTIONS: usize = 6;

/// A distribution index and the option indices to evaluate on it.
type Task = (usize, Vec<usize>);

/// Outcomes per distribution and option.
struct Table {
    outcomes: Vec<Vec<Option<Outcome>>>,
    /// Distributions on which every planned task finished.
    complete: Vec<bool>,
    status: Status,
    nodes: u64,
}

impl Table {
    fn new(worlds: usize, options: usize) -> Table {
        Table {
            outcomes: vec![vec![None; options]; worlds],
            complete: vec![false; worlds],
            status: Status::Complete,
            nodes: 0,
        }
    }

    /// Runs `tasks` on all cores with `nodes` per task. A distribution
    /// with an interrupted task counts as unfinished for every option.
    fn run(
        &mut self,
        ctx: &Context<'_>,
        worlds: &[World],
        tasks: &[Task],
        nodes: u64,
        cancel: Option<&Cancel>,
    ) {
        let results = run_parallel(tasks, |(w, options)| {
            let mut meter = Meter::new(nodes, cancel.cloned());
            let outcome = ctx.evaluate(&worlds[*w], options, &mut meter);
            (outcome, meter.nodes())
        });
        let mut failed = vec![false; worlds.len()];
        for ((w, _), (outcome, used)) in tasks.iter().zip(results) {
            self.nodes += used;
            match outcome {
                Ok(values) => {
                    for (o, value) in values {
                        self.outcomes[*w][o] = Some(value);
                    }
                }
                Err(reason) => {
                    failed[*w] = true;
                    self.status = match (self.status, reason) {
                        (_, Interrupted::Cancelled) | (Status::Cancelled, _) => Status::Cancelled,
                        _ => Status::BudgetExhausted,
                    };
                }
            }
        }
        for (w, _) in tasks {
            self.complete[*w] = !failed[*w];
        }
        for (w, outcomes) in self.outcomes.iter_mut().enumerate() {
            if failed[w] {
                outcomes.fill(None);
            }
        }
    }

    /// Weighted estimate of `option` over the finished distributions on
    /// which it was evaluated.
    fn estimate(&self, weights: &[f64], option: usize, exact: bool) -> Option<Estimate> {
        let rows: Vec<(f64, Outcome)> = self
            .outcomes
            .iter()
            .zip(weights)
            .filter_map(|(o, &w)| o[option].map(|o| (w, o)))
            .filter(|&(w, _)| w > 0.0)
            .collect();
        let total: f64 = rows.iter().map(|(w, _)| w).sum();
        if rows.is_empty() {
            return None;
        }
        let mean =
            |f: &dyn Fn(&Outcome) -> f64| rows.iter().map(|(w, o)| w * f(o)).sum::<f64>() / total;
        let score = mean(&|o| f64::from(o.score));
        let variance = mean(&|o| (f64::from(o.score) - score).powi(2));
        let squares: f64 = rows.iter().map(|(w, _)| w * w).sum();
        let ess = total * total / squares;
        Some(Estimate {
            win_chance: mean(&|o| f64::from(u8::from(o.won))),
            expected_score: score,
            expected_game_points: mean(&|o| f64::from(o.game_points)),
            thrown_in: mean(&|o| f64::from(u8::from(o.thrown_in))),
            score_std_error: if exact { 0.0 } else { (variance / ess).sqrt() },
            samples: rows.len() as u32,
            exact,
        })
    }
}

/// Runs `f` for every item on all cores, or on the threads allowed by
/// [`crate::with_max_threads`]; results in item order, so the outcome does
/// not depend on scheduling or the thread count.
fn run_parallel<I: Sync, T: Send>(items: &[I], f: impl Fn(&I) -> T + Sync) -> Vec<T> {
    let threads = crate::budget::max_threads().min(items.len());
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<T>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let value = f(item);
                    results.lock().expect("no panic while holding the lock")[i] = Some(value);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("no panic while holding the lock")
        .into_iter()
        .map(|r| r.expect("every item evaluated"))
        .collect()
}

/// Result of one option in one distribution.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Outcome {
    /// Seeger-Fabian points of the deciding seat.
    score: i32,
    /// Declarer's game points.
    game_points: i32,
    /// The deciding seat's party won.
    won: bool,
    /// Thrown in.
    thrown_in: bool,
}

/// Facts shared by all distributions.
struct Context<'a> {
    view: &'a PlayerView,
    /// Cards each seat played so far.
    played_by: [CardSet; 3],
}

impl<'a> Context<'a> {
    fn new(view: &'a PlayerView) -> Result<Context<'a>, Vec<Diagnostic>> {
        let mut played_by = [CardSet::EMPTY; 3];
        for trick in &view.tricks {
            for (i, &card) in trick.cards.iter().enumerate() {
                played_by[trick.seat_of(i).index()].insert(card);
            }
        }
        if let Some(leader) = view.current_trick.leader {
            for (i, card) in view.current_trick.cards.iter().enumerate() {
                if let Some(card) = card {
                    played_by[Seat::from_index(leader.index() + i).index()].insert(*card);
                }
            }
        }
        let needs_contract = view.phase == Phase::Spiel;
        if needs_contract && (view.contract.is_none() || view.declarer.is_none()) {
            return Err(vec![Diagnostic::InvalidHistory]);
        }
        Ok(Context { view, played_by })
    }

    /// Outcomes of the options with indices `options` in `world`.
    fn evaluate(
        &self,
        world: &World,
        options: &[usize],
        meter: &mut Meter,
    ) -> Result<Vec<(usize, Outcome)>, Interrupted> {
        meter.check_cancel()?;
        let mut solvers = Solvers::default();
        if self.view.phase == Phase::Spiel {
            return self.evaluate_play(world, options, meter, &mut solvers);
        }
        let mut out = Vec::with_capacity(options.len());
        for &o in options {
            let action = self.view.legal_actions[o];
            out.push((
                o,
                match self.simulate(world, action) {
                    Some((pos, stakes)) => {
                        let value = solvers.solve(&pos, meter)?;
                        self.outcome(value.settle(pos.declarer, &stakes))
                    }
                    None => Outcome {
                        score: 0,
                        game_points: 0,
                        won: false,
                        thrown_in: true,
                    },
                },
            ));
        }
        Ok(out)
    }

    fn outcome(&self, result: GameResult) -> Outcome {
        let declarer = result.outcome.declarer;
        Outcome {
            score: result.seeger_fabian()[self.view.seat.index()],
            game_points: result.score,
            won: (self.view.seat == declarer) == result.won,
            thrown_in: false,
        }
    }

    /// Card play, Kontra and Re during the play phase.
    fn evaluate_play(
        &self,
        world: &World,
        options: &[usize],
        meter: &mut Meter,
        solvers: &mut Solvers,
    ) -> Result<Vec<(usize, Outcome)>, Interrupted> {
        let view = self.view;
        let declarer = view.declarer.expect("checked in Context::new");
        let contract = view.contract.expect("checked in Context::new");
        let game = contract.game;
        let won: Vec<_> = view
            .tricks
            .iter()
            .filter(|t| t.winner == declarer)
            .collect();
        let pos = Position {
            game,
            declarer,
            hands: world.hands,
            skat: world.skat,
            trick: view.current_trick,
            declarer_augen: won.iter().map(|t| t.augen()).sum(),
            declarer_tricks: won.len() as u8,
            tricks_played: view.tricks.len() as u8,
        };
        debug_assert_eq!(pos.check(), Ok(()), "sampled position is consistent");
        let original =
            world.hands[declarer.index()] | self.played_by[declarer.index()] | world.skat;
        let stakes = |kontra| Stakes {
            contract,
            bid: view.bid,
            spitzen: spitzen(game, original),
            kontra,
        };
        let declarer_cards = world.hands[declarer.index()]
            | if contract.hand {
                CardSet::EMPTY
            } else {
                world.skat
            };
        let re = |level: KontraLevel| {
            if level == KontraLevel::Kontra && policy::wants_re(game, declarer_cards) {
                KontraLevel::Re
            } else {
                level
            }
        };
        // A defender who has not played yet may still say Kontra (policy).
        let later_kontra = |exclude: Option<Seat>| {
            let possible = view.config.kontra_re && view.kontra == KontraLevel::None;
            let any = possible
                && Seat::ALL.iter().any(|&d| {
                    d != declarer
                        && Some(d) != exclude
                        && self.played_by[d.index()].is_empty()
                        && kontra_eligible(&view.bidding, d)
                        && policy::wants_kontra(game, world.hands[d.index()])
                });
            if any {
                re(KontraLevel::Kontra)
            } else {
                view.kontra
            }
        };
        let mut here: Option<DdValue> = None;
        let mut out = Vec::with_capacity(options.len());
        for &o in options {
            let (value, level) = match view.legal_actions[o] {
                Action::Play(card) => {
                    let mut child = pos;
                    child.play(card);
                    let level = if contract.ouvert {
                        view.kontra
                    } else {
                        later_kontra(Some(view.seat))
                    };
                    (solvers.solve(&child, meter)?, level)
                }
                Action::Kontra => (
                    cached(&mut here, solvers, &pos, meter)?,
                    re(KontraLevel::Kontra),
                ),
                Action::KeinKontra => {
                    // Defenders after this one in the Ouvert round decide by
                    // the policy.
                    let later = Seat::ALL[view.seat.index() + 1..].iter().any(|&d| {
                        d != declarer
                            && kontra_eligible(&view.bidding, d)
                            && policy::wants_kontra(game, world.hands[d.index()])
                    });
                    let level = if later {
                        re(KontraLevel::Kontra)
                    } else {
                        KontraLevel::None
                    };
                    (cached(&mut here, solvers, &pos, meter)?, level)
                }
                Action::Re => (cached(&mut here, solvers, &pos, meter)?, KontraLevel::Re),
                Action::KeinRe => (
                    cached(&mut here, solvers, &pos, meter)?,
                    KontraLevel::Kontra,
                ),
                _ => unreachable!("only card play, Kontra and Re in the play phase"),
            };
            out.push((o, self.outcome(value.settle(declarer, &stakes(level)))));
        }
        Ok(out)
    }

    /// The hypothetical game for `world` with the public history replayed
    /// up to the decision; `None` if the history does not replay.
    fn replay(&self, world: &World) -> Option<(Game, Deal)> {
        let view = self.view;
        let viewer = view.seat;
        let mut hands = world.hands;
        let mut skat = world.skat;
        if view.phase == Phase::Druecken {
            // The original Skat does not matter once the declarer holds all
            // twelve cards: any two of them rebuild the same position.
            skat = view.hand.iter().take(2).collect();
            hands[viewer.index()] = view.hand - skat;
        } else if let Some(own) = view.own_skat {
            skat = own;
        }
        let deal = Deal::new(hands, skat).expect("worlds conserve all cards");
        let mut game = Game::from_deal(deal, view.config);
        let mut history: Vec<Action> = view
            .bidding
            .iter()
            .map(|r| match r.call {
                Call::Bid(v) => Action::Bid(v),
                Call::Hold(_) => Action::Hold,
                Call::Pass => Action::Pass,
            })
            .collect();
        match view.hand_game {
            Some(true) => history.push(Action::Hand),
            Some(false) => {
                history.push(Action::TakeSkat);
                if let Some(own) = view.own_skat {
                    history.push(Action::Discard(own));
                }
            }
            None => {}
        }
        for action in history {
            game.apply(action).ok()?;
        }
        (game.phase() == view.phase && game.current_seat() == view.current_seat)
            .then_some((game, deal))
    }

    /// Plays the game in `world` from the decision with `option` up to the
    /// first card, every later decision by the continuation policy. `None`
    /// if the deal is thrown in.
    fn simulate(&self, world: &World, option: Action) -> Option<(Position, Stakes)> {
        let view = self.view;
        let viewer = view.seat;
        let (mut game, deal) = self.replay(world).expect("history checked in analyze");
        game.apply(option)
            .expect("option is legal in the replayed game");
        let viewer_limit = match option {
            Action::Bid(v) => v,
            Action::Hold => policy::choose_game(view.hand).1.max(view.bid),
            _ => policy::choose_game(view.hand).1,
        };
        loop {
            let seat = game.current_seat()?;
            let seat_view = game.view(seat);
            let legal = &seat_view.legal_actions;
            let action = match game.phase() {
                Phase::Reizen => {
                    let limit = if seat == viewer {
                        viewer_limit
                    } else {
                        policy::choose_game(seat_view.hand).1
                    };
                    policy::bid(limit, game.bid(), legal)
                }
                Phase::SkatAufnahme => policy::hand_or_pickup(seat_view.hand),
                Phase::Druecken => Action::Discard(policy::discard(seat_view.hand)),
                Phase::Ansage => policy::announce(
                    seat_view.hand,
                    seat_view.own_skat.unwrap_or_default(),
                    seat_view.bid,
                    seat_view.hand_game == Some(true),
                    legal,
                ),
                Phase::Spiel => {
                    let contract = game.contract().expect("contract in play");
                    if legal.iter().any(|a| matches!(a, Action::Play(_))) {
                        break;
                    }
                    // Ouvert Kontra round before the reveal.
                    let own = seat_view.hand | seat_view.own_skat.unwrap_or_default();
                    if legal.contains(&Action::Re) {
                        if policy::wants_re(contract.game, own) {
                            Action::Re
                        } else {
                            Action::KeinRe
                        }
                    } else if policy::wants_kontra(contract.game, seat_view.hand) {
                        Action::Kontra
                    } else {
                        Action::KeinKontra
                    }
                }
                Phase::Beendet | Phase::Eingepasst => return None,
            };
            let action = if legal.contains(&action) {
                action
            } else {
                legal[0]
            };
            game.apply(action).expect("policy picks a legal action");
        }
        let declarer = game.declarer().expect("declarer in play");
        let contract = game.contract().expect("contract in play");
        let mut kontra = game.kontra();
        let declarer_view = game.view(declarer);
        if view.config.kontra_re && !contract.ouvert && kontra == KontraLevel::None {
            let says_kontra = Seat::ALL.iter().any(|&d| {
                d != declarer
                    && kontra_eligible(game.bidding_history(), d)
                    && policy::wants_kontra(contract.game, game.view(d).hand)
            });
            if says_kontra {
                let own = declarer_view.hand | declarer_view.own_skat.unwrap_or_default();
                kontra = if policy::wants_re(contract.game, own) {
                    KontraLevel::Re
                } else {
                    KontraLevel::Kontra
                };
            }
        }
        let pos = Position {
            game: contract.game,
            declarer,
            hands: Seat::ALL.map(|s| game.view(s).hand),
            skat: declarer_view.own_skat.unwrap_or(deal.skat()),
            trick: skat_core::Trick::default(),
            declarer_augen: 0,
            declarer_tricks: 0,
            tricks_played: 0,
        };
        let stakes = Stakes {
            contract,
            bid: game.bid(),
            spitzen: spitzen(contract.game, deal.hand(declarer) | deal.skat()),
            kontra,
        };
        Some((pos, stakes))
    }
}

fn cached(
    slot: &mut Option<DdValue>,
    solvers: &mut Solvers,
    pos: &Position,
    meter: &mut Meter,
) -> Result<DdValue, Interrupted> {
    if let Some(value) = slot {
        return Ok(*value);
    }
    let value = solvers.solve(pos, meter)?;
    *slot = Some(value);
    Ok(value)
}

/// Solvers and solved positions of one distribution (fresh per
/// distribution, so node counts do not depend on evaluation order).
#[derive(Default)]
struct Solvers {
    solvers: Vec<((GameType, Seat), Solver)>,
    solved: Vec<(Position, DdValue)>,
}

impl Solvers {
    fn solve(&mut self, pos: &Position, meter: &mut Meter) -> Result<DdValue, Interrupted> {
        if let Some((_, value)) = self.solved.iter().find(|(p, _)| p == pos) {
            return Ok(*value);
        }
        let key = (pos.game, pos.declarer);
        let index = match self.solvers.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                self.solvers
                    .push((key, Solver::new(pos.game, pos.declarer)));
                self.solvers.len() - 1
            }
        };
        let value = self.solvers[index].1.solve(pos, meter)?;
        self.solved.push((*pos, value));
        Ok(value)
    }
}

fn observations(view: &PlayerView) -> Vec<Observation> {
    let k = Knowledge::from_view(view);
    let mut out = Vec::new();
    if let Some(game) = k.game {
        for seat in Seat::ALL {
            if seat == view.seat {
                continue;
            }
            let mut classes: Vec<Follow> = Vec::new();
            for card in k.voids[seat.index()] {
                if !classes.contains(&game.follow(card)) {
                    classes.push(game.follow(card));
                }
            }
            for follow in classes {
                out.push(Observation::Void { seat, follow });
            }
        }
        if game != GameType::Null {
            out.push(Observation::TrumpsOut {
                count: k.trumps_out().len() as u8,
            });
        }
        if let Some(sets) = k.trumpf_sets() {
            out.push(Observation::TrumpfSets(sets));
        }
    }
    if let Some((declarer, defenders)) = k.party_augen() {
        out.push(Observation::AugenWon {
            declarer,
            defenders,
        });
    }
    out.push(Observation::AugenOut {
        augen: k.augen_out(),
    });
    if let Some(cards) = k.skat {
        out.push(Observation::SkatKnown { cards });
    }
    for (i, signal) in evidence::signals(&view.bidding).iter().enumerate() {
        if signal.shown > 0 {
            out.push(Observation::BidShown {
                seat: Seat::from_index(i),
                value: signal.shown,
            });
        }
    }
    out.extend(
        preplay::structures(view)
            .into_iter()
            .map(Observation::Structure),
    );
    out.extend(
        preplay::game_values(view)
            .into_iter()
            .map(Observation::GameValues),
    );
    if let Some(value) = preplay::commitment(view) {
        out.push(Observation::Commitment {
            value,
            kontra_re: view.config.kontra_re,
        });
    }
    if view.config.kontra_re && view.kontra == KontraLevel::None && view.phase == Phase::Spiel {
        for seat in Seat::ALL {
            if Some(seat) != view.declarer && kontra_eligible(&view.bidding, seat) {
                out.push(Observation::KontraEligible { seat });
            }
        }
    }
    out
}

/// Trumps (or Jacks before the announcement) per hidden place, over the
/// distributions with positive weight. None in Null and when no
/// distribution finished.
fn inferences(
    view: &PlayerView,
    hidden: &HiddenCards,
    worlds: &[World],
    weights: &[f64],
    enumerated: bool,
) -> Vec<Inference> {
    let (class, cards) = match view.contract.map(|c| c.game) {
        Some(GameType::Null) => return Vec::new(),
        Some(game) => (CardClass::Trumps, game.trumps()),
        None => (CardClass::Jacks, GameType::Grand.trumps()),
    };
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return Vec::new();
    }
    hidden
        .slots()
        .iter()
        .map(|slot| {
            let held = |world: &World| match slot.place {
                Place::Seat(seat) => (world.hands[seat.index()] & cards).len() as u8,
                Place::Skat => (world.skat & cards).len() as u8,
            };
            let mut sum = 0.0;
            let (mut min, mut max) = (u8::MAX, 0);
            for (world, &w) in worlds.iter().zip(weights) {
                if w > 0.0 {
                    let n = held(world);
                    sum += w * f64::from(n);
                    min = min.min(n);
                    max = max.max(n);
                }
            }
            Inference {
                holder: slot.place,
                class,
                expected: sum / total,
                min,
                max,
                certain: (slot.allowed & cards).is_empty()
                    || (enumerated && min == max && weights.iter().all(|&w| w > 0.0)),
            }
        })
        .collect()
}

/// What an option is worth to the deciding seat of `view`: its own
/// expected Seeger-Fabian points; a defender also gains half of every
/// point the declarer loses, since both defenders gain relative to the
/// declarer (this makes Kontra worth saying). The opponents (`skat-ai`)
/// choose by it and the coach compares options by it.
pub fn utility(view: &PlayerView, estimate: &Estimate) -> f64 {
    let defender = view.declarer.is_some_and(|d| d != view.seat);
    if defender {
        estimate.expected_score - estimate.expected_game_points / 2.0
    } else {
        estimate.expected_score
    }
}

fn assumptions(phase: Phase, known: bool) -> Vec<Assumption> {
    let mut out = if known {
        vec![Assumption::KnownDeal, Assumption::DoubleDummyPlay]
    } else {
        vec![
            Assumption::UniformHiddenCards,
            Assumption::BiddingEvidence,
            Assumption::DoubleDummyPlay,
        ]
    };
    if phase != Phase::Spiel {
        out.push(Assumption::ContinuationPolicy);
    }
    if phase == Phase::Reizen {
        out.push(Assumption::BidAsLimit);
    }
    out
}
