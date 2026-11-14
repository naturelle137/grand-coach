// SPDX-License-Identifier: EUPL-1.2

//! Reserved revision-1 positions. Never include these in the practice chooser
//! or image galleries. See the preregistration for independent reference facts.
use super::*;

/// Deliberately selected preregistered session, not ordinary practice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSet {
    /// First human session.
    Initial,
    /// Different positions, targeted 7–14 calendar days after Initial.
    Delayed,
}

impl ValidationSet {
    /// Immutable four-case order, one per bounded family.
    pub fn cases(self) -> [PracticeCase; 4] {
        let start = if self == Self::Initial { 12 } else { 16 };
        std::array::from_fn(|i| PracticeCase {
            index: start + i as u8,
        })
    }
}

/// Reserved cases are resolvable for provenance but absent from PRACTICE_CASES.
pub const RESERVED_CASES: [PracticeCase; 8] = [
    PracticeCase { index: 12 },
    PracticeCase { index: 13 },
    PracticeCase { index: 14 },
    PracticeCase { index: 15 },
    PracticeCase { index: 16 },
    PracticeCase { index: 17 },
    PracticeCase { index: 18 },
    PracticeCase { index: 19 },
];

pub(super) fn id(index: u8) -> &'static str {
    [
        "validation54.initial.trumpf",
        "validation54.initial.void",
        "validation54.initial.side",
        "validation54.initial.partner",
        "validation54.delayed.trumpf",
        "validation54.delayed.void",
        "validation54.delayed.side",
        "validation54.delayed.partner",
    ]
    .get(usize::from(index - 12))
    .copied()
    .unwrap_or("unknown")
}

pub(super) fn definition(index: u8) -> Result<Definition, CatalogueError> {
    use Rank::*;
    use Seat::*;
    use Suit::*;
    let c = Card::new;
    let set = |cards: &[(Suit, Rank)]| cards.iter().map(|&(s, r)| c(s, r)).collect();
    let mut d = Definition {
        anchors: [CardSet::EMPTY; 3],
        declarer: Vorhand,
        game: GameType::Grand,
        played: Vec::new(),
        question: PracticeQuestion::Count(QuestionKind::TrumpfOutside),
    };
    match index {
        12 => {
            d.anchors[0] = set(&[(Karo, Ass), (Kreuz, Bube)]);
            d.anchors[1] = set(&[
                (Karo, Sieben),
                (Herz, Bube),
                (Herz, Ass),
                (Herz, Zehn),
                (Herz, Koenig),
                (Herz, Dame),
                (Herz, Neun),
                (Herz, Acht),
                (Herz, Sieben),
                (Pik, Ass),
            ]);
            d.anchors[2] = set(&[(Karo, Acht)]);
            d.played = vec![c(Karo, Ass), c(Karo, Sieben), c(Karo, Acht), c(Kreuz, Bube)];
        }
        13 => {
            d.game = GameType::Null;
            // H is declarer so the won first trick cannot terminate Null.
            d.declarer = Hinterhand;
            d.anchors[0] = set(&[(Herz, Bube), (Kreuz, Ass)]);
            d.anchors[1] = set(&[
                (Pik, Sieben),
                (Karo, Sieben),
                (Pik, Acht),
                (Pik, Neun),
                (Pik, Zehn),
                (Pik, Bube),
                (Pik, Dame),
                (Pik, Koenig),
                (Pik, Ass),
                (Karo, Acht),
            ]);
            d.anchors[2] = set(&[(Herz, Sieben)]);
            d.played = vec![
                c(Herz, Bube),
                c(Pik, Sieben),
                c(Herz, Sieben),
                c(Kreuz, Ass),
                c(Karo, Sieben),
            ];
            d.question = PracticeQuestion::Count(QuestionKind::ProvenVoids {
                class: Follow::Farbe(Herz),
            });
        }
        14 => {
            d.game = GameType::Farbe(Karo);
            d.anchors[0] = set(&[
                (Pik, Ass),
                (Pik, Zehn),
                (Pik, Koenig),
                (Pik, Dame),
                (Pik, Neun),
                (Pik, Acht),
                (Pik, Sieben),
                (Kreuz, Ass),
                (Herz, Bube),
                (Karo, Sieben),
            ]);
            d.anchors[1] = set(&[(Kreuz, Sieben), (Kreuz, Bube)]);
            d.anchors[2] = set(&[(Kreuz, Acht), (Pik, Bube)]);
            d.played = vec![c(Kreuz, Ass), c(Kreuz, Sieben), c(Kreuz, Acht)];
            d.question = PracticeQuestion::Consequence {
                action: Action::Play(c(Pik, Ass)),
                kind: MechanismKind::Abstich,
            };
        }
        15 => {
            d.anchors[0] = set(&[(Kreuz, Bube), (Herz, Zehn)]);
            d.anchors[1] = set(&[
                (Herz, Bube),
                (Herz, Sieben),
                (Herz, Acht),
                (Herz, Neun),
                (Herz, Dame),
                (Herz, Koenig),
                (Herz, Ass),
                (Kreuz, Sieben),
                (Kreuz, Acht),
                (Kreuz, Neun),
            ]);
            d.anchors[2] = set(&[(Karo, Bube), (Pik, Bube)]);
            d.played = vec![c(Kreuz, Bube), c(Herz, Bube), c(Karo, Bube), c(Herz, Zehn)];
            d.question = PracticeQuestion::Consequence {
                action: Action::Play(c(Herz, Sieben)),
                kind: MechanismKind::Overtake,
            };
        }
        16 => {
            d.game = GameType::Farbe(Kreuz);
            d.anchors[0] = set(&[
                (Karo, Ass),
                (Kreuz, Ass),
                (Kreuz, Zehn),
                (Kreuz, Koenig),
                (Karo, Bube),
                (Pik, Sieben),
                (Pik, Acht),
                (Pik, Neun),
                (Herz, Sieben),
                (Herz, Acht),
            ]);
            d.anchors[1] = set(&[(Karo, Sieben)]);
            d.anchors[2] = set(&[(Karo, Acht)]);
            d.played = vec![c(Karo, Ass), c(Karo, Sieben), c(Karo, Acht)];
        }
        17 => {
            d.anchors[0] = set(&[(Kreuz, Bube), (Karo, Bube)]);
            d.anchors[1] = set(&[
                (Pik, Bube),
                (Herz, Sieben),
                (Herz, Acht),
                (Herz, Neun),
                (Herz, Dame),
                (Herz, Koenig),
                (Herz, Zehn),
                (Herz, Ass),
                (Kreuz, Sieben),
                (Kreuz, Acht),
            ]);
            d.anchors[2] = set(&[(Herz, Bube)]);
            d.played = vec![
                c(Kreuz, Bube),
                c(Pik, Bube),
                c(Herz, Bube),
                c(Karo, Bube),
                c(Herz, Sieben),
            ];
            d.question = PracticeQuestion::Count(QuestionKind::ProvenVoids {
                class: Follow::Trumpf,
            });
        }
        18 => {
            d.anchors[0] = set(&[
                (Kreuz, Bube),
                (Pik, Bube),
                (Herz, Bube),
                (Karo, Bube),
                (Kreuz, Ass),
                (Kreuz, Zehn),
                (Kreuz, Sieben),
                (Herz, Sieben),
                (Herz, Acht),
                (Herz, Neun),
            ]);
            d.anchors[1] = set(&[(Kreuz, Acht)]);
            d.anchors[2] = set(&[(Kreuz, Neun)]);
            d.played = vec![c(Kreuz, Ass), c(Kreuz, Acht), c(Kreuz, Neun)];
            d.question = PracticeQuestion::Consequence {
                action: Action::Play(c(Kreuz, Zehn)),
                kind: MechanismKind::Abstich,
            };
        }
        19 => {
            d.anchors[0] = set(&[(Herz, Ass), (Kreuz, Bube)]);
            d.anchors[1] = set(&[
                (Herz, Sieben),
                (Pik, Bube),
                (Herz, Bube),
                (Karo, Bube),
                (Karo, Sieben),
                (Karo, Acht),
                (Karo, Neun),
                (Karo, Dame),
                (Karo, Koenig),
                (Karo, Zehn),
            ]);
            d.anchors[2] = set(&[(Herz, Acht)]);
            d.played = vec![c(Herz, Ass), c(Herz, Sieben), c(Herz, Acht), c(Kreuz, Bube)];
            d.question = PracticeQuestion::Consequence {
                action: Action::Play(c(Pik, Bube)),
                kind: MechanismKind::Overtake,
            };
        }
        _ => return Err(CatalogueError::UnknownCase),
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FocusAnswer, focus_expected};

    #[test]
    fn reserved_legal_views_match_handwritten_facts_and_are_distinct_from_public_cases() {
        // Frozen reference arithmetic/conditions, not captured engine output.
        let expected = [
            FocusAnswer::Count(2),
            FocusAnswer::Count(1),
            FocusAnswer::Possible,
            FocusAnswer::Possible,
            FocusAnswer::Count(7),
            FocusAnswer::Count(1),
            FocusAnswer::Absent,
            FocusAnswer::Absent,
        ];
        let mut views = Vec::new();
        for (case, expected) in RESERVED_CASES.into_iter().zip(expected) {
            let position = case.position().unwrap();
            assert_eq!(
                focus_expected(&position.feedback()),
                Some(expected),
                "{}",
                case.id()
            );
            assert!(
                PRACTICE_CASES
                    .into_iter()
                    .all(|c| c.position().unwrap().view != position.view)
            );
            assert!(!views.contains(&position.view));
            views.push(position.view.clone());
            let game = case.definition().unwrap().replay(case.seed()).unwrap();
            for action in game.legal_actions() {
                let mut next = game.clone();
                next.apply(action).unwrap();
            }
            assert_eq!(PracticeCase::find(case.id(), 0), None);
            assert_eq!(PracticeCase::find(case.id(), 2), None);
        }
    }
}
