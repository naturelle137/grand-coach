// SPDX-License-Identifier: EUPL-1.2

//! Versioned legal positions for #50's future focus sessions.
//! See `docs/practice-catalogue.md` for the independent reference contract.
//! No session or learning-history format is changed by this catalogue.

mod reserved;
pub use reserved::{RESERVED_CASES, ValidationSet};

use serde::{Deserialize, Serialize};
use skat_analysis::Knowledge;
use skat_analysis::counting::{Derivation, QuestionKind, derive};
use skat_analysis::mechanism::mechanisms;
pub use skat_analysis::mechanism::{Certainty, Mechanism, MechanismKind};
use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Follow, Game, GameType, Phase, PlayerView, Rank,
    RuleConfig, Seat, Suit,
};

/// The four bounded practice families. Independent of persisted training topics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PracticeFamily {
    /// Exact Trumpf outside own hand, public play and known own Skat.
    RemainingTrumpf,
    /// Other seats publicly proven unable to serve a follow class.
    ProvenVoid,
    /// Conditional possibility of trumping a full side card.
    SideCardVulnerability,
    /// Conditional possibility of a later partner beating the declarer's lead.
    PartnerOvertaking,
}

/// Intended position within a future family sequence, not exposure evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseRole {
    /// Candidate for a worked example.
    Introduction,
    /// Changed seat/surface candidate; held out only if not previously shown.
    Changed,
    /// Boundary or counterexample to prevent overgeneralisation.
    Boundary,
}

/// Stable catalogue identity. A changed position or meaning raises `version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PracticeCase {
    index: u8,
}

/// All twelve revision-1 cases, in family then introduction/changed/boundary order.
pub const PRACTICE_CASES: [PracticeCase; 12] = [
    PracticeCase { index: 0 },
    PracticeCase { index: 1 },
    PracticeCase { index: 2 },
    PracticeCase { index: 3 },
    PracticeCase { index: 4 },
    PracticeCase { index: 5 },
    PracticeCase { index: 6 },
    PracticeCase { index: 7 },
    PracticeCase { index: 8 },
    PracticeCase { index: 9 },
    PracticeCase { index: 10 },
    PracticeCase { index: 11 },
];

/// A factual question, without its answer or an option-quality judgement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PracticeQuestion {
    /// An exact count derived by shared analysis.
    Count(QuestionKind),
    /// Check a narrow consequence of this hypothetical legal card.
    Consequence {
        /// The hypothetical card; no preference over other legal cards.
        action: Action,
        /// The consequence being checked.
        kind: MechanismKind,
    },
}

/// Only the historical permitted view and the question; no answer or seed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticePosition {
    view: PlayerView,
    question: PracticeQuestion,
}

/// Explicit feedback, computed only from the frozen permitted view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PracticeFeedback {
    /// Exact count with its shared derivation; None means unsupported.
    Count(Option<Derivation>),
    /// Exact feasibility deductions, not a recommendation or model comparison.
    Consequences(Vec<Mechanism>),
}

/// A defective or unsupported catalogue identity/position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogueError {
    /// Unknown internal catalogue index.
    UnknownCase,
    /// An anchored deal or recorded action is invalid.
    InvalidPosition,
}

impl PracticePosition {
    /// Historical acting seat's view. Unknown Skat remains unknown.
    pub fn view(&self) -> &PlayerView {
        &self.view
    }

    /// Question only. Feedback requires a separate call.
    pub fn question(&self) -> PracticeQuestion {
        self.question
    }

    /// Exact answer from existing analysis; no search or option ranking.
    pub fn feedback(&self) -> PracticeFeedback {
        match self.question {
            PracticeQuestion::Count(q) => {
                PracticeFeedback::Count(derive(&Knowledge::from_view(&self.view), q))
            }
            PracticeQuestion::Consequence { action, kind } => PracticeFeedback::Consequences(
                mechanisms(&self.view, action)
                    .into_iter()
                    .filter(|m| m.kind() == kind)
                    .collect(),
            ),
        }
    }
}

impl PracticeCase {
    /// Resolve a stable ID and revision. Unknown revisions are never upgraded.
    pub fn find(id: &str, version: u16) -> Option<Self> {
        PRACTICE_CASES
            .into_iter()
            .chain(RESERVED_CASES)
            .find(|c| c.id() == id && c.version() == version)
    }

    /// Stable reference key; not learner-facing prompt copy.
    pub fn id(self) -> &'static str {
        const IDS: [&str; 12] = [
            "remaining-trumpf.intro",
            "remaining-trumpf.changed",
            "remaining-trumpf.boundary",
            "proven-void.intro",
            "proven-void.changed",
            "proven-void.boundary",
            "side-card.intro",
            "side-card.changed",
            "side-card.boundary",
            "partner-overtaking.intro",
            "partner-overtaking.changed",
            "partner-overtaking.boundary",
        ];
        if self.index >= 12 {
            return reserved::id(self.index);
        }
        IDS.get(usize::from(self.index))
            .copied()
            .unwrap_or("unknown")
    }

    /// Revision of the deal, prefix and question semantics.
    pub fn version(self) -> u16 {
        1
    }

    /// Deterministic completion seed for the anchored deal (not an answer source).
    pub fn seed(self) -> u64 {
        501 + u64::from(self.index)
    }

    /// Family, or None for an invalid catalogue index.
    pub fn family(self) -> Option<PracticeFamily> {
        use PracticeFamily::*;
        if self.index >= 12 {
            return [
                RemainingTrumpf,
                ProvenVoid,
                SideCardVulnerability,
                PartnerOvertaking,
            ]
            .get(usize::from((self.index - 12) % 4))
            .copied();
        }
        [
            RemainingTrumpf,
            ProvenVoid,
            SideCardVulnerability,
            PartnerOvertaking,
        ]
        .get(usize::from(self.index / 3))
        .copied()
    }

    /// Intended role, never proof that a learner has not seen the position.
    pub fn role(self) -> CaseRole {
        if self.index >= 12 {
            return CaseRole::Changed;
        }
        [
            CaseRole::Introduction,
            CaseRole::Changed,
            CaseRole::Boundary,
        ][usize::from(self.index % 3)]
    }

    /// Reconstruct through core, then drop the privileged fixture setup.
    pub fn position(self) -> Result<PracticePosition, CatalogueError> {
        let definition = self.definition()?;
        let game = definition.replay(self.seed())?;
        let view = game.view(game.current_seat().ok_or(CatalogueError::InvalidPosition)?);
        if view.phase != Phase::Spiel || view.legal_actions.is_empty() {
            return Err(CatalogueError::InvalidPosition);
        }
        if let PracticeQuestion::Consequence { action, .. } = definition.question
            && !view.legal_actions.contains(&action)
        {
            return Err(CatalogueError::InvalidPosition);
        }
        Ok(PracticePosition {
            view,
            question: definition.question,
        })
    }

    fn definition(self) -> Result<Definition, CatalogueError> {
        if self.index >= 12 {
            return reserved::definition(self.index);
        }
        use Rank::*;
        use Seat::*;
        use Suit::*;
        let c = Card::new;
        let set = |cards: &[(Suit, Rank)]| cards.iter().map(|&(s, r)| c(s, r)).collect();
        let count = |q| PracticeQuestion::Count(q);
        let consequence = |s, r, kind| PracticeQuestion::Consequence {
            action: Action::Play(c(s, r)),
            kind,
        };
        let mut d = Definition {
            anchors: [CardSet::EMPTY; 3],
            declarer: Vorhand,
            game: GameType::Farbe(Herz),
            played: Vec::new(),
            question: count(QuestionKind::TrumpfOutside),
        };
        match self.index {
            0 => {
                d.game = GameType::Farbe(Karo);
                d.anchors[0] = set(&[
                    (Karo, Sieben),
                    (Karo, Acht),
                    (Karo, Neun),
                    (Karo, Zehn),
                    (Karo, Ass),
                    (Kreuz, Bube),
                    (Pik, Sieben),
                    (Pik, Acht),
                    (Pik, Neun),
                    (Pik, Zehn),
                ]);
            }
            1 => {
                d.game = GameType::Grand;
                d.anchors[0] = set(&[(Herz, Bube)]);
                d.anchors[1] = set(&[
                    (Kreuz, Bube),
                    (Pik, Bube),
                    (Kreuz, Sieben),
                    (Kreuz, Acht),
                    (Kreuz, Neun),
                    (Kreuz, Zehn),
                    (Kreuz, Ass),
                    (Kreuz, Koenig),
                    (Kreuz, Dame),
                    (Pik, Sieben),
                ]);
                d.played = vec![c(Herz, Bube)];
            }
            2 => {
                d.game = GameType::Grand;
                d.anchors[0] = set(&[(Kreuz, Sieben)]);
                d.anchors[1] = set(&[(Kreuz, Acht)]);
                d.anchors[2] = set(&[
                    (Kreuz, Bube),
                    (Pik, Bube),
                    (Herz, Bube),
                    (Karo, Bube),
                    (Herz, Sieben),
                    (Herz, Acht),
                    (Herz, Neun),
                    (Herz, Koenig),
                    (Herz, Dame),
                    (Herz, Ass),
                ]);
                d.played = vec![c(Kreuz, Sieben), c(Kreuz, Acht)];
            }
            3 => {
                d.anchors[0] = set(&[(Kreuz, Ass)]);
                d.anchors[1] = set(&[
                    (Pik, Ass),
                    (Pik, Zehn),
                    (Pik, Koenig),
                    (Pik, Dame),
                    (Pik, Neun),
                    (Pik, Acht),
                    (Pik, Sieben),
                    (Kreuz, Bube),
                    (Herz, Sieben),
                    (Herz, Acht),
                ]);
                d.played = vec![c(Kreuz, Ass), c(Kreuz, Bube)];
                d.question = count(QuestionKind::ProvenVoids {
                    class: Follow::Farbe(Kreuz),
                });
            }
            4 => {
                d.game = GameType::Grand;
                d.declarer = Hinterhand;
                d.anchors[0] = set(&[(Kreuz, Bube)]);
                d.anchors[1] = set(&[
                    (Herz, Ass),
                    (Herz, Zehn),
                    (Herz, Koenig),
                    (Herz, Dame),
                    (Herz, Neun),
                    (Herz, Acht),
                    (Herz, Sieben),
                    (Pik, Sieben),
                    (Pik, Acht),
                    (Pik, Neun),
                ]);
                d.played = vec![c(Kreuz, Bube), c(Herz, Ass)];
                d.question = count(QuestionKind::ProvenVoids {
                    class: Follow::Trumpf,
                });
            }
            5 => {
                d.game = GameType::Null;
                d.declarer = Mittelhand;
                d.anchors[0] = set(&[(Kreuz, Neun)]);
                d.anchors[1] = set(&[(Kreuz, Bube)]);
                d.played = vec![c(Kreuz, Neun), c(Kreuz, Bube)];
                d.question = count(QuestionKind::ProvenVoids {
                    class: Follow::Farbe(Kreuz),
                });
            }
            6 => {
                d.anchors[0] = set(&[
                    (Kreuz, Ass),
                    (Kreuz, Zehn),
                    (Kreuz, Koenig),
                    (Kreuz, Dame),
                    (Kreuz, Neun),
                    (Kreuz, Acht),
                    (Kreuz, Sieben),
                    (Herz, Ass),
                    (Kreuz, Bube),
                    (Pik, Bube),
                ]);
                d.anchors[1] = set(&[(Herz, Zehn)]);
                d.anchors[2] = set(&[(Herz, Sieben)]);
                d.question = consequence(Kreuz, Ass, MechanismKind::Abstich);
            }
            7 => {
                d.game = GameType::Grand;
                d.declarer = Hinterhand;
                d.anchors[0] = set(&[(Pik, Sieben)]);
                d.anchors[1] = set(&[
                    (Pik, Ass),
                    (Pik, Zehn),
                    (Pik, Koenig),
                    (Pik, Dame),
                    (Pik, Neun),
                    (Pik, Acht),
                    (Kreuz, Bube),
                    (Kreuz, Sieben),
                    (Kreuz, Acht),
                    (Kreuz, Neun),
                ]);
                d.anchors[2] = set(&[(Herz, Bube)]);
                d.played = vec![c(Pik, Sieben)];
                d.question = consequence(Pik, Ass, MechanismKind::Abstich);
            }
            8 => {
                d.game = GameType::Null;
                d.anchors[0] = set(&[(Kreuz, Ass)]);
                d.question = consequence(Kreuz, Ass, MechanismKind::Abstich);
            }
            9 => {
                d.anchors[0] = set(&[(Kreuz, Neun)]);
                d.anchors[1] = set(&[
                    (Kreuz, Sieben),
                    (Kreuz, Acht),
                    (Kreuz, Koenig),
                    (Kreuz, Dame),
                    (Kreuz, Zehn),
                    (Pik, Sieben),
                    (Pik, Acht),
                    (Herz, Sieben),
                    (Karo, Sieben),
                    (Karo, Acht),
                ]);
                d.anchors[2] = set(&[(Kreuz, Ass)]);
                d.played = vec![c(Kreuz, Neun)];
                d.question = consequence(Kreuz, Sieben, MechanismKind::Overtake);
            }
            10 => {
                d.game = GameType::Grand;
                d.declarer = Hinterhand;
                d.anchors[0] = set(&[
                    (Pik, Bube),
                    (Pik, Sieben),
                    (Pik, Acht),
                    (Pik, Neun),
                    (Pik, Koenig),
                    (Pik, Dame),
                    (Kreuz, Sieben),
                    (Kreuz, Acht),
                    (Kreuz, Neun),
                    (Kreuz, Zehn),
                ]);
                d.anchors[1] = set(&[(Herz, Bube), (Pik, Ass)]);
                d.anchors[2] = set(&[(Kreuz, Bube), (Pik, Zehn)]);
                d.played = vec![c(Pik, Bube), c(Herz, Bube), c(Kreuz, Bube), c(Pik, Zehn)];
                d.question = consequence(Pik, Sieben, MechanismKind::Overtake);
            }
            11 => {
                d.game = GameType::Farbe(Karo);
                d.anchors[0] = set(&[(Kreuz, Bube)]);
                d.anchors[1] = set(&[
                    (Pik, Bube),
                    (Herz, Bube),
                    (Karo, Bube),
                    (Karo, Sieben),
                    (Karo, Acht),
                    (Karo, Neun),
                    (Karo, Zehn),
                    (Karo, Koenig),
                    (Karo, Dame),
                    (Karo, Ass),
                ]);
                d.played = vec![c(Kreuz, Bube)];
                d.question = consequence(Pik, Bube, MechanismKind::Overtake);
            }
            _ => return Err(CatalogueError::UnknownCase),
        }
        Ok(d)
    }
}

// Privileged authoring data never leaves this module. Completing anchors is
// deterministic fixture construction, not another replay or analysis engine.
struct Definition {
    anchors: [CardSet; 3],
    declarer: Seat,
    game: GameType,
    played: Vec<Card>,
    question: PracticeQuestion,
}

impl Definition {
    fn deal(&self, seed: u64) -> Result<Deal, CatalogueError> {
        let mut hands = self.anchors;
        let reserved = hands
            .into_iter()
            .fold(CardSet::EMPTY, |all, hand| all | hand);
        if reserved.len() != hands.iter().map(|h| h.len()).sum::<usize>()
            || hands.iter().any(|h| h.len() > 10)
        {
            return Err(CatalogueError::InvalidPosition);
        }
        let shuffled = Deal::from_seed(seed);
        let mut remaining = Seat::ALL
            .into_iter()
            .flat_map(|s| shuffled.hand(s))
            .chain(shuffled.skat())
            .filter(|c| !reserved.contains(*c));
        for hand in &mut hands {
            while hand.len() < 10 {
                hand.insert(remaining.next().ok_or(CatalogueError::InvalidPosition)?);
            }
        }
        Deal::new(hands, remaining.collect()).map_err(|_| CatalogueError::InvalidPosition)
    }

    fn actions(&self) -> Vec<Action> {
        let mut actions = match self.declarer {
            Seat::Vorhand => vec![Action::Pass, Action::Pass, Action::Bid(18)],
            Seat::Mittelhand => vec![Action::Bid(18), Action::Pass, Action::Pass],
            Seat::Hinterhand => vec![Action::Pass, Action::Bid(18), Action::Pass],
        };
        actions.extend([
            Action::Hand,
            Action::Announce(Announcement::plain(self.game)),
        ]);
        actions.extend(self.played.iter().copied().map(Action::Play));
        actions
    }

    fn replay(&self, seed: u64) -> Result<Game, CatalogueError> {
        self.replay_deal(self.deal(seed)?)
    }

    fn replay_deal(&self, deal: Deal) -> Result<Game, CatalogueError> {
        let mut game = Game::from_deal(deal, RuleConfig::default());
        for action in self.actions() {
            game.apply(action)
                .map_err(|_| CatalogueError::InvalidPosition)?;
        }
        Ok(game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_analysis::counting::Derivation;
    use skat_core::Rank::*;
    use skat_core::Seat::*;
    use skat_core::Suit::*;

    // Independent references are specified in docs/practice-catalogue.md.
    // No expected count, certainty or candidate set is copied from analysis.
    #[test]
    fn twelve_legal_positions_match_independent_counting_references() {
        let expected = [5, 1, 0, 1, 1, 0];
        let seats = [
            Vorhand, Mittelhand, Hinterhand, Hinterhand, Hinterhand, Hinterhand,
        ];
        for (i, (&answer, &seat)) in expected.iter().zip(&seats).enumerate() {
            let p = PRACTICE_CASES[i].position().unwrap();
            assert_eq!(p.view().seat, seat);
            assert_eq!(p.view().own_skat, None, "Hand Skat stays unknown");
            let PracticeFeedback::Count(Some(derivation)) = p.feedback() else {
                panic!("count supported")
            };
            assert_eq!(derivation.expected(), answer, "{}", PRACTICE_CASES[i].id());
            if let Derivation::ProvenVoids { seats, .. } = derivation {
                // Only M has failed to follow in the two positive prefixes.
                assert_eq!(seats, [false, i != 5, false]);
            }
        }
        for case in PRACTICE_CASES {
            let p = case.position().unwrap();
            let d = case.definition().unwrap();
            let game = d.replay(case.seed()).unwrap();
            assert_eq!(game.view(p.view.seat), p.view);
            for &action in &p.view.legal_actions {
                let mut continued = game.clone();
                continued
                    .apply(action)
                    .expect("every offered action is core-legal");
            }
        }
    }

    #[test]
    fn mechanism_answers_match_independent_conditions_and_alternatives() {
        for (index, case) in PRACTICE_CASES.iter().enumerate().skip(6) {
            let p = case.position().unwrap();
            let PracticeFeedback::Consequences(found) = p.feedback() else {
                panic!("mechanism")
            };
            match index {
                6 | 7 => {
                    let expected = if index == 6 {
                        vec![Mittelhand, Hinterhand]
                    } else {
                        vec![Hinterhand]
                    };
                    assert_eq!(found.len(), expected.len());
                    for (m, seat) in found.iter().zip(expected) {
                        let Mechanism::Abstich(a) = m else {
                            panic!("Abstich")
                        };
                        assert_eq!(a.seat, seat);
                        assert_eq!(a.certainty, Certainty::Possible);
                        assert!(a.suit_unseen.is_empty(), "all side cards own or played");
                        assert!(!a.trumpf_unseen.is_empty());
                    }
                }
                9 | 10 => {
                    let [Mechanism::Overtake(o)] = found.as_slice() else {
                        panic!("one partner")
                    };
                    assert_eq!(o.partner, if index == 9 { Hinterhand } else { Mittelhand });
                    assert_eq!(o.declarer, if index == 9 { Vorhand } else { Hinterhand });
                    assert_eq!(o.certainty, Certainty::Possible);
                    assert_eq!(
                        o.higher,
                        CardSet::single(Card::new(if index == 9 { Kreuz } else { Pik }, Ass))
                    );
                    assert!(o.trumping);
                }
                8 | 11 => assert!(found.is_empty(), "Null / highest Trumpf counterexamples"),
                _ => unreachable!(),
            }
        }
        // Five equivalent legal undercards in the changed-seat Grand. They
        // preserve the conditional mechanism, not necessarily total utility.
        let p = PRACTICE_CASES[10].position().unwrap();
        assert_eq!(p.view.legal_actions.len(), 5);
        for rank in [Sieben, Acht, Neun, Dame, Koenig] {
            let action = Action::Play(Card::new(Pik, rank));
            assert!(p.view.legal_actions.contains(&action));
            let result = mechanisms(&p.view, action);
            let [Mechanism::Overtake(o)] = result.as_slice() else {
                panic!("equivalent undercard")
            };
            assert_eq!(o.higher, CardSet::single(Card::new(Pik, Ass)));
            assert!(o.trumping);
            assert_eq!(o.certainty, Certainty::Possible);
        }
        let p = PRACTICE_CASES[11].position().unwrap();
        assert_eq!(p.view.legal_actions.len(), 10);
        for &action in &p.view.legal_actions {
            assert!(
                mechanisms(&p.view, action).is_empty(),
                "no card beats Kreuz J"
            );
        }
    }

    #[test]
    fn replay_rejects_bad_deals_and_actions_and_void_evidence_is_historical() {
        let case = PRACTICE_CASES[3];
        let mut d = case.definition().unwrap();
        d.played.pop(); // M has not failed to follow yet.
        let before = d.replay(case.seed()).unwrap();
        assert!(
            before
                .view(Hinterhand)
                .proven_voids()
                .iter()
                .all(|v| v.is_empty())
        );
        d.played.push(Card::new(Kreuz, Bube));
        let after = d.replay(case.seed()).unwrap();
        assert!(!after.view(Hinterhand).proven_voids()[Mittelhand.index()].is_empty());
        assert!(
            before
                .view(Hinterhand)
                .proven_voids()
                .iter()
                .all(|v| v.is_empty())
        );
        d.played.push(Card::new(Kreuz, Ass)); // Already played, never legal again.
        assert!(matches!(
            d.replay(case.seed()),
            Err(CatalogueError::InvalidPosition)
        ));
        d.anchors[1] |= d.anchors[0]; // Duplicate card / overfull hand.
        assert_eq!(d.deal(case.seed()), Err(CatalogueError::InvalidPosition));
    }

    fn swapped(deal: Deal, a: Card, b: Card) -> Deal {
        let swap = |mut cards: CardSet| {
            if cards.contains(a) != cards.contains(b) {
                if cards.contains(a) {
                    cards.remove(a);
                    cards.insert(b);
                } else {
                    cards.remove(b);
                    cards.insert(a);
                }
            }
            cards
        };
        Deal::new(Seat::ALL.map(|s| swap(deal.hand(s))), swap(deal.skat())).unwrap()
    }

    #[test]
    fn hidden_hands_and_unknown_skat_substitutions_preserve_all_task_data() {
        for case in PRACTICE_CASES.into_iter().chain(RESERVED_CASES) {
            let d = case.definition().unwrap();
            let deal = d.deal(case.seed()).unwrap();
            let game = d.replay_deal(deal).unwrap();
            let original = case.position().unwrap();
            let all = game.omniscient_view();
            let others: Vec<_> = Seat::ALL
                .into_iter()
                .filter(|&s| s != original.view.seat)
                .collect();
            let mut hands_checked = 0;
            let mut skat_checked = 0;
            for a in all.hands[others[0].index()] {
                for b in all.hands[others[1].index()] | all.skat {
                    let Ok(other) = d.replay_deal(swapped(deal, a, b)) else {
                        continue;
                    };
                    let view = other.view(original.view.seat);
                    if view != original.view {
                        continue;
                    }
                    assert_ne!(other.omniscient_view(), all);
                    let position = PracticePosition {
                        view,
                        question: original.question,
                    };
                    assert_eq!(position, original);
                    assert_eq!(position.feedback(), original.feedback());
                    for &action in &position.view.legal_actions {
                        assert_eq!(
                            mechanisms(&position.view, action),
                            mechanisms(&original.view, action)
                        );
                    }
                    if all.skat.contains(b) {
                        skat_checked += 1;
                    } else {
                        hands_checked += 1;
                    }
                }
            }
            assert!(hands_checked > 0 && skat_checked > 0, "{}", case.id());
        }
    }

    // Construct a *different* legal hidden distribution in which the target
    // cannot win, or can trump, as requested. Keep all already played cards
    // and the viewer's entire original hand unchanged.
    fn hidden_witness(case: PracticeCase, target: Seat, trumping: bool) -> Game {
        let d = case.definition().unwrap();
        let original = d.deal(case.seed()).unwrap();
        let game = d.replay_deal(original).unwrap();
        let all = game.omniscient_view();
        let viewer = game.current_seat().unwrap();
        let other = Seat::ALL
            .into_iter()
            .find(|&s| s != viewer && s != target)
            .unwrap();
        let mut pool = all.hands[target.index()] | all.hands[other.index()] | all.skat;
        let overtake = matches!(
            d.question,
            PracticeQuestion::Consequence {
                kind: MechanismKind::Overtake,
                ..
            }
        );
        let led = if let PracticeQuestion::Consequence {
            action: Action::Play(card),
            kind: MechanismKind::Abstich,
        } = d.question
        {
            card
        } else {
            game.view(viewer).current_trick.led().unwrap()
        };
        let required = if trumping {
            CardSet::single((pool & d.game.trumps()).iter().next().unwrap())
        } else {
            CardSet::EMPTY
        };
        let chosen: CardSet = required
            | (pool - required)
                .iter()
                .filter(|&c| {
                    // Exclude Trumpf for the negative witness; always exclude
                    // the led side class for either overtaking witness.
                    (trumping || !d.game.trumps().contains(c))
                        && (!overtake || d.game.follow(c) != d.game.follow(led))
                })
                .take(all.hands[target.index()].len() - required.len())
                .collect::<CardSet>();
        assert_eq!(chosen.len(), all.hands[target.index()].len());
        pool -= chosen;
        let other_hand: CardSet = pool.iter().take(all.hands[other.index()].len()).collect();
        pool -= other_hand;
        let mut hands = all.original_hands;
        hands[target.index()] = chosen | (original.hand(target) - all.hands[target.index()]);
        hands[other.index()] = other_hand | (original.hand(other) - all.hands[other.index()]);
        let witness = d.replay_deal(Deal::new(hands, pool).unwrap()).unwrap();
        assert_eq!(witness.view(viewer), game.view(viewer));
        witness
    }

    // Independent core oracle for the *one trick* claim. Enumerate legal
    // continuations in a concrete distribution, never call mechanisms or
    // shared sampling/feasibility code to determine this witness result.
    fn can_complete(mut game: Game, action: Action, target: Seat, overtake: bool) -> bool {
        let game_type = game.contract().unwrap().game;
        game.apply(action).unwrap();
        let completed = game.view(target).tricks.len();
        fn visit(
            game: &Game,
            target: Seat,
            overtake: bool,
            game_type: GameType,
            completed: usize,
        ) -> bool {
            let view = game.view(target);
            if view.tricks.len() > completed {
                return overtake && view.tricks.last().unwrap().winner == target;
            }
            for action in game.legal_actions() {
                let seat = game.current_seat().unwrap();
                let mut next = game.clone();
                next.apply(action).unwrap();
                if !overtake
                    && seat == target
                    && matches!(action, Action::Play(c) if game_type.follow(c) == Follow::Trumpf)
                {
                    return true;
                }
                if visit(&next, target, overtake, game_type, completed) {
                    return true;
                }
            }
            false
        }
        visit(&game, target, overtake, game_type, completed)
    }

    #[test]
    fn concrete_legal_witnesses_prove_possible_and_not_certain_without_engine_oracle() {
        for (index, targets) in [
            (6, vec![Mittelhand, Hinterhand]),
            (7, vec![Hinterhand]),
            (9, vec![Hinterhand]),
            (10, vec![Mittelhand]),
            (14, vec![Mittelhand, Hinterhand]),
            (15, vec![Hinterhand]),
        ] {
            let case = PracticeCase { index: index as u8 };
            let overtake = matches!(
                case.position().unwrap().question(),
                PracticeQuestion::Consequence {
                    kind: MechanismKind::Overtake,
                    ..
                }
            );
            let PracticeQuestion::Consequence { action, .. } = case.position().unwrap().question()
            else {
                unreachable!()
            };
            for target in targets {
                let actual = case.definition().unwrap().replay(case.seed()).unwrap();
                assert!(
                    can_complete(actual, action, target, overtake),
                    "positive witness {}",
                    case.id()
                );
                if overtake {
                    assert!(
                        can_complete(hidden_witness(case, target, true), action, target, true),
                        "separate core-legal trumping witness {}",
                        case.id()
                    );
                }
                assert!(
                    !can_complete(
                        hidden_witness(case, target, false),
                        action,
                        target,
                        overtake
                    ),
                    "negative witness {}",
                    case.id()
                );
            }
        }
    }

    #[test]
    fn identities_versions_and_seed_reconstruction_are_deterministic() {
        for case in PRACTICE_CASES {
            assert_eq!(PracticeCase::find(case.id(), 1), Some(case));
            assert_eq!(PracticeCase::find(case.id(), 2), None);
            assert_eq!(case.position(), case.position());
            assert_eq!(
                case.definition().unwrap().deal(case.seed()),
                case.definition().unwrap().deal(case.seed())
            );
            assert!(case.family().is_some());
            let deal = case.definition().unwrap().deal(case.seed()).unwrap();
            let fingerprint = Seat::ALL
                .into_iter()
                .map(|s| deal.hand(s))
                .chain([deal.skat()])
                .fold(0u64, |hash, cards| {
                    cards
                        .iter()
                        .fold(hash.wrapping_mul(37).wrapping_add(33), |hash, c| {
                            hash.wrapping_mul(37).wrapping_add(u64::from(c.index()))
                        })
                });
            // Mechanical version-1 deal fingerprints, not strategic oracles.
            const DEALS_V1: [u64; 12] = [
                6603650931012932804,
                16396792070140690284,
                6120153699476858896,
                5937786575545013724,
                11167381227663647440,
                9894812411341126644,
                1689532884444888628,
                17615805915700891608,
                12734977507600124480,
                9247904279603279512,
                14760203069699986548,
                16154748840130836108,
            ];
            assert_eq!(fingerprint, DEALS_V1[usize::from(case.index)]);
            // No answer, seed, family/role or preferred move on the position.
            let wire = serde_json::to_value(case.position().unwrap()).unwrap();
            assert_eq!(wire.as_object().unwrap().len(), 2);
        }
        assert_eq!(PracticeCase::find("absent", 1), None);
        let invalid = PracticeCase { index: 255 };
        assert_eq!(invalid.position(), Err(CatalogueError::UnknownCase));
    }
}
