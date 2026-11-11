// SPDX-License-Identifier: EUPL-1.2

//! The counting aid and the card-counting questions (M6, D-64).
//!
//! Both are derived from one seat's [`Knowledge`] only, so they never
//! depend on hidden cards (D-23): the same view and history give the same
//! aid, the same questions and the same expected answers. Every quantity
//! here is exact; nothing is estimated. What the seat cannot know, such as
//! the contents of an unseen Skat, is reported as such (`skat_unknown`) and
//! never filled in.
//!
//! Definitions (all counts include the trick in progress):
//!
//! - *played*: cards laid on the table in tricks, completed or not.
//! - *Trumpf outside*: Trumpf that are neither in the seat's hand, nor
//!   played, nor in the seat's own discarded Skat. They lie in the other
//!   hands, among them the revealed hand of an Ouvert declarer, or in a
//!   Skat the seat does not know. Null has no Trumpf.
//! - *Augen won*: Augen of completed tricks by the declarer and by the two
//!   defenders together. The Skat is not included and the trick in progress
//!   is not yet won.
//! - *Augen outstanding*: Augen in cards neither in the seat's hand, nor
//!   played, nor in the seat's own discarded Skat.
//! - *void*: a follow class (Trumpf or a side suit) in which the seat
//!   failed to follow suit. Only publicly proven voids count.

use skat_core::{CardSet, Follow, GameType, Seat, Suit};

use crate::Knowledge;

/// Follow classes a seat can be proven void in: the Trumpf and the four
/// side suits. A suit the game makes entirely Trumpf has no side class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Voids {
    /// Proven to hold no Trumpf.
    pub trumpf: bool,
    /// Proven to hold no card of the side suit (index: [`Suit::index`]).
    pub suits: [bool; 4],
}

impl Voids {
    /// `true` if no void is proven.
    pub fn is_empty(&self) -> bool {
        !self.trumpf && !self.suits.contains(&true)
    }
}

/// Trumpf counts of a Farbspiel or Grand, as far as the seat knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TrumpfCount {
    /// Trumpf of the game: 11 in a Farbspiel, 4 in Grand.
    pub total: u8,
    /// In the seat's hand.
    pub own: u8,
    /// Played.
    pub played: u8,
    /// In the seat's own discarded Skat.
    pub discarded: u8,
    /// Outside: `total - own - played - discarded`.
    pub outside: u8,
    /// The seat does not know the Skat, so some outside Trumpf may lie
    /// there.
    pub skat_unknown: bool,
}

/// Augen counts, as far as the seat knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AugenCount {
    /// Won in tricks by the declarer; `None` before the declarer is known.
    pub declarer: Option<u8>,
    /// Won in tricks by the two defenders together.
    pub defenders: Option<u8>,
    /// Outstanding (see the module documentation).
    pub outstanding: u8,
    /// Augen of the seat's own discarded Skat, if it knows one.
    pub skat: Option<u8>,
}

/// What the counting aid shows: exact facts of one seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CountingAid {
    /// The viewing seat.
    pub seat: Seat,
    /// The announced game, if any.
    pub game: Option<GameType>,
    /// Cards played so far, including the trick in progress.
    pub played: CardSet,
    /// Trumpf counts; `None` before the announcement and in Null.
    pub trumpf: Option<TrumpfCount>,
    /// Augen counts.
    pub augen: AugenCount,
    /// Proven voids of each seat (index: [`Seat::index`]); the viewing
    /// seat's own entry is always empty.
    pub voids: [Voids; 3],
}

impl CountingAid {
    /// The aid of the seat `k` describes.
    pub fn from_knowledge(k: &Knowledge) -> CountingAid {
        let trumpf = k.trumpf_sets().map(|t| {
            let discarded = t.own_skat.len() as u8;
            let (total, own, played) = (t.total.len(), t.own.len(), t.played.len());
            TrumpfCount {
                total: total as u8,
                own: own as u8,
                played: played as u8,
                discarded,
                outside: (total - own - played) as u8 - discarded,
                skat_unknown: t.skat_unknown,
            }
        });
        let party = k.party_augen();
        let outstanding_cards = CardSet::ALL - k.own - k.played - k.skat.unwrap_or_default();
        let mut voids = [Voids::default(); 3];
        if let Some(game) = k.game {
            for seat in Seat::ALL.into_iter().filter(|&s| s != k.seat) {
                voids[seat.index()] = voids_of(game, k.voids[seat.index()]);
            }
        }
        CountingAid {
            seat: k.seat,
            game: k.game,
            played: k.played,
            trumpf,
            augen: AugenCount {
                declarer: party.map(|p| p.0),
                defenders: party.map(|p| p.1),
                outstanding: outstanding_cards.augen(),
                skat: k.skat.map(|s| s.augen()),
            },
            voids,
        }
    }
}

/// The classes `set` covers completely.
fn voids_of(game: GameType, set: CardSet) -> Voids {
    let covers = |class: Follow| {
        let cards = game.cards_of(class);
        !cards.is_empty() && (cards - set).is_empty()
    };
    let mut voids = Voids {
        trumpf: covers(Follow::Trumpf),
        suits: [false; 4],
    };
    for suit in Suit::ALL {
        voids.suits[usize::from(suit.index())] = covers(Follow::Farbe(suit));
    }
    voids
}

/// A counting question the seat can answer from its own view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QuestionKind {
    /// How many Trumpf are outside (see [`TrumpfCount::outside`])?
    TrumpfOutside,
    /// How many Augen did the declarer (`true`) or the two defenders
    /// together win in tricks so far?
    AugenWon {
        /// The declarer's side instead of the defenders'.
        declarer: bool,
    },
    /// How many cards of a follow class were played so far?
    ClassPlayed {
        /// The class: Trumpf or a side suit.
        class: Follow,
    },
}

/// The exact answer to a question with the numbers that make it up, so
/// that feedback can show the derivation. Parameters, not text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Derivation {
    /// `total - own - played - discarded`.
    TrumpfOutside(TrumpfCount),
    /// Augen won by one side.
    AugenWon {
        /// Declarer's side asked for.
        declarer: bool,
        /// Augen of the asked side.
        augen: u8,
        /// Augen won by the other side.
        other: u8,
    },
    /// Played cards of a class.
    ClassPlayed {
        /// The class.
        class: Follow,
        /// Cards in the class.
        size: u8,
        /// Played.
        played: u8,
        /// Cards of the class not yet played: `size - played`.
        unplayed: u8,
    },
}

impl Derivation {
    /// The expected answer.
    pub fn expected(&self) -> u8 {
        match *self {
            Derivation::TrumpfOutside(t) => t.outside,
            Derivation::AugenWon { augen, .. } => augen,
            Derivation::ClassPlayed { played, .. } => played,
        }
    }
}

/// The questions the seat can answer now, in a fixed order: Trumpf outside,
/// Augen of the declarer, Augen of the defenders, then the played cards of
/// the Trumpf class and of each side suit with at least one card.
pub fn applicable_questions(k: &Knowledge) -> Vec<QuestionKind> {
    let Some(game) = k.game else {
        return Vec::new();
    };
    let mut kinds = Vec::new();
    if game != GameType::Null {
        kinds.push(QuestionKind::TrumpfOutside);
        if k.declarer.is_some() {
            kinds.push(QuestionKind::AugenWon { declarer: true });
            kinds.push(QuestionKind::AugenWon { declarer: false });
        }
        kinds.push(QuestionKind::ClassPlayed {
            class: Follow::Trumpf,
        });
    }
    for suit in Suit::ALL {
        if !game.cards_of(Follow::Farbe(suit)).is_empty() {
            kinds.push(QuestionKind::ClassPlayed {
                class: Follow::Farbe(suit),
            });
        }
    }
    kinds
}

/// The exact answer to `kind` from `k`; `None` if `k` cannot answer it.
pub fn derive(k: &Knowledge, kind: QuestionKind) -> Option<Derivation> {
    let game = k.game?;
    let aid = CountingAid::from_knowledge(k);
    match kind {
        QuestionKind::TrumpfOutside => aid.trumpf.map(Derivation::TrumpfOutside),
        QuestionKind::AugenWon { declarer } => {
            if game == GameType::Null {
                return None;
            }
            let (d, o) = (aid.augen.declarer?, aid.augen.defenders?);
            let (augen, other) = if declarer { (d, o) } else { (o, d) };
            Some(Derivation::AugenWon {
                declarer,
                augen,
                other,
            })
        }
        QuestionKind::ClassPlayed { class } => {
            let cards = game.cards_of(class);
            if cards.is_empty() {
                return None;
            }
            let played = (cards & k.played).len() as u8;
            Some(Derivation::ClassPlayed {
                class,
                size: cards.len() as u8,
                played,
                unplayed: cards.len() as u8 - played,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{Action, Announcement, Card, Deal, Game, Phase, Rank, RuleConfig};

    fn cards(list: &[(Suit, Rank)]) -> CardSet {
        list.iter().map(|&(s, r)| Card::new(s, r)).collect()
    }

    fn base(game: Option<GameType>) -> Knowledge {
        Knowledge {
            seat: Seat::Vorhand,
            game,
            declarer: None,
            own: CardSet::EMPTY,
            played: CardSet::EMPTY,
            known_elsewhere: CardSet::EMPTY,
            skat: None,
            voids: [CardSet::EMPTY; 3],
            hand_sizes: [10; 3],
            augen_won: [0; 3],
        }
    }

    /// Farbspiel Herz seen by a defender; every number below is counted
    /// by hand from the card lists, not computed by the code under test.
    #[test]
    fn farbspiel_defender_counts() {
        use Rank::*;
        use Suit::*;
        let mut k = base(Some(GameType::Farbe(Herz)));
        k.declarer = Some(Seat::Mittelhand);
        // Trumpf in hand: Kreuz-Bube, Herz-Ass, Herz-Zehn (3); Pik-Ass is
        // a side card. Own Augen: 2 + 11 + 10 + 11 = 34.
        k.own = cards(&[(Kreuz, Bube), (Herz, Ass), (Herz, Zehn), (Pik, Ass)]);
        // Played: Pik-Bube (2 Augen, Trumpf), Herz-Sieben (Trumpf), Karo-Ass
        // (11 Augen, side). 13 Augen, 2 Trumpf.
        k.played = cards(&[(Pik, Bube), (Herz, Sieben), (Karo, Ass)]);
        k.augen_won = [0, 14, 11];
        let aid = CountingAid::from_knowledge(&k);
        let t = aid.trumpf.unwrap();
        assert_eq!((t.total, t.own, t.played, t.discarded), (11, 3, 2, 0));
        assert_eq!(t.outside, 6);
        assert!(t.skat_unknown);
        assert_eq!(aid.augen.declarer, Some(14));
        assert_eq!(aid.augen.defenders, Some(11));
        assert_eq!(aid.augen.outstanding, 120 - 34 - 13);
        assert_eq!(aid.augen.skat, None);
        assert_eq!(aid.played.len(), 3);
        assert!(aid.voids.iter().all(Voids::is_empty));
    }

    /// Grand declarer after Drücken: the own discard is known, outside the
    /// game and not counted as outstanding.
    #[test]
    fn grand_declarer_knows_the_discarded_skat() {
        use Rank::*;
        use Suit::*;
        let mut k = base(Some(GameType::Grand));
        k.declarer = Some(Seat::Vorhand);
        k.own = cards(&[(Pik, Bube)]);
        let discard = cards(&[(Kreuz, Bube), (Herz, Zehn)]);
        k.skat = Some(discard);
        k.known_elsewhere = discard;
        k.played = cards(&[(Herz, Bube)]);
        let aid = CountingAid::from_knowledge(&k);
        let t = aid.trumpf.unwrap();
        // 4 Buben: one in hand, one played, one discarded, one outside.
        assert_eq!(
            (t.total, t.own, t.played, t.discarded, t.outside),
            (4, 1, 1, 1, 1)
        );
        assert!(!t.skat_unknown);
        // Own 2, played 2, discard 12 Augen leave 104.
        assert_eq!(aid.augen.outstanding, 104);
        assert_eq!(aid.augen.skat, Some(12));
    }

    /// A defender against an Ouvert declarer sees the declarer's cards:
    /// revealed Trumpf are still outside (not in the own hand).
    #[test]
    fn revealed_ouvert_trumpf_are_outside_and_outstanding() {
        use Rank::*;
        use Suit::*;
        let mut k = base(Some(GameType::Grand));
        k.declarer = Some(Seat::Hinterhand);
        k.known_elsewhere = cards(&[(Karo, Bube), (Karo, Ass)]);
        let t = CountingAid::from_knowledge(&k).trumpf.unwrap();
        assert_eq!((t.own, t.played, t.discarded, t.outside), (0, 0, 0, 4));
        assert!(t.skat_unknown);
        assert_eq!(CountingAid::from_knowledge(&k).augen.outstanding, 120);
    }

    /// Null has no Trumpf and no Augen questions; the Buben are ordinary
    /// cards of their suit.
    #[test]
    fn null_has_only_suit_counts_and_voids() {
        use Rank::*;
        use Suit::*;
        let mut k = base(Some(GameType::Null));
        k.declarer = Some(Seat::Mittelhand);
        k.played = cards(&[(Kreuz, Sieben), (Kreuz, Bube), (Herz, Neun)]);
        k.voids[Seat::Hinterhand.index()] = Kreuz.cards();
        let aid = CountingAid::from_knowledge(&k);
        assert_eq!(aid.trumpf, None);
        assert!(aid.voids[Seat::Hinterhand.index()].suits[usize::from(Kreuz.index())]);
        assert!(!aid.voids[Seat::Hinterhand.index()].trumpf);
        assert!(aid.voids[Seat::Mittelhand.index()].is_empty());
        let kinds = applicable_questions(&k);
        assert_eq!(kinds.len(), 4);
        assert!(
            kinds
                .iter()
                .all(|q| matches!(q, QuestionKind::ClassPlayed { .. }))
        );
        let d = derive(
            &k,
            QuestionKind::ClassPlayed {
                class: Follow::Farbe(Kreuz),
            },
        );
        assert_eq!(d.map(|d| d.expected()), Some(2));
        let d = derive(
            &k,
            QuestionKind::ClassPlayed {
                class: Follow::Farbe(Herz),
            },
        );
        assert_eq!(d.map(|d| d.expected()), Some(1));
        assert_eq!(derive(&k, QuestionKind::TrumpfOutside), None);
        assert_eq!(derive(&k, QuestionKind::AugenWon { declarer: true }), None);
    }

    /// Void classes: the whole class must be proven, a partial set is not
    /// a void, and in a Farbspiel the Trumpf are one class across suits.
    #[test]
    fn voids_need_the_whole_follow_class() {
        use Suit::*;
        let game = GameType::Farbe(Herz);
        let mut k = base(Some(game));
        k.voids[1] = game.trumps();
        k.voids[2] =
            game.cards_of(Follow::Farbe(Karo)) - CardSet::single(Card::new(Karo, Rank::Sieben));
        let aid = CountingAid::from_knowledge(&k);
        assert!(aid.voids[1].trumpf);
        assert_eq!(aid.voids[1].suits, [false; 4]);
        assert!(!aid.voids[2].trumpf);
        assert!(!aid.voids[2].suits[usize::from(Karo.index())]);
        // The viewing seat's own entry stays empty even if its set is not.
        k.voids[0] = game.trumps();
        assert!(CountingAid::from_knowledge(&k).voids[0].is_empty());
    }

    /// Boundaries: nothing known before the announcement, no cards played,
    /// everything of a class played.
    #[test]
    fn boundaries() {
        let k = base(None);
        let aid = CountingAid::from_knowledge(&k);
        assert_eq!(aid.trumpf, None);
        assert_eq!(aid.augen.declarer, None);
        assert!(applicable_questions(&k).is_empty());
        assert_eq!(derive(&k, QuestionKind::TrumpfOutside), None);

        let mut k = base(Some(GameType::Grand));
        k.declarer = Some(Seat::Vorhand);
        assert_eq!(
            derive(
                &k,
                QuestionKind::ClassPlayed {
                    class: Follow::Trumpf
                }
            )
            .map(|d| d.expected()),
            Some(0)
        );
        assert_eq!(
            derive(&k, QuestionKind::AugenWon { declarer: false }).map(|d| d.expected()),
            Some(0)
        );
        k.played = GameType::Grand.trumps();
        let d = derive(
            &k,
            QuestionKind::ClassPlayed {
                class: Follow::Trumpf,
            },
        )
        .unwrap();
        assert_eq!(d.expected(), 4);
        assert!(matches!(d, Derivation::ClassPlayed { unplayed: 0, .. }));
        assert_eq!(
            derive(&k, QuestionKind::TrumpfOutside).unwrap().expected(),
            0
        );
        // Grand: Trumpf class plus four side suits, three Augen kinds.
        assert_eq!(applicable_questions(&k).len(), 1 + 2 + 1 + 4);
        // A Farbspiel makes the trump suit's side class empty: no question.
        let mut k = base(Some(GameType::Farbe(Suit::Herz)));
        k.declarer = Some(Seat::Vorhand);
        assert!(
            !applicable_questions(&k).contains(&QuestionKind::ClassPlayed {
                class: Follow::Farbe(Suit::Herz)
            })
        );
        assert_eq!(
            derive(
                &k,
                QuestionKind::ClassPlayed {
                    class: Follow::Farbe(Suit::Herz)
                }
            ),
            None
        );
    }

    /// Plays `actions` on `deal`; `None` if one is illegal.
    fn replay(deal: Deal, actions: &[Action]) -> Option<Game> {
        let mut game = Game::from_deal(deal, RuleConfig::default());
        for &a in actions {
            game.apply(a).ok()?;
        }
        Some(game)
    }

    /// Hidden-card substitution (D-23): swapping two cards the human
    /// cannot see, such that the same public history stays legal, changes
    /// neither the aid, the questions, the expected answers nor the
    /// derivation.
    #[test]
    fn hidden_cards_do_not_change_aid_questions_or_answers() {
        let mut checked = 0;
        for seed in 1..40u64 {
            let deal = Deal::from_seed(seed);
            let mut game = Game::from_deal(deal, RuleConfig::default());
            let mut actions = Vec::new();
            while game.phase() != Phase::Spiel {
                let action = match game.phase() {
                    Phase::Reizen if game.bid() == 0 => Action::Bid(18),
                    Phase::Reizen => Action::Pass,
                    Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
                    Phase::Beendet | Phase::Eingepasst => break,
                    _ => game.legal_actions()[0],
                };
                game.apply(action).unwrap();
                actions.push(action);
            }
            if game.phase() != Phase::Spiel {
                continue;
            }
            // Vorhand is the human; the opponents hold the hidden hands.
            let human = Seat::Vorhand;
            for _ in 0..7 {
                let view = game.view(human);
                let reference = Knowledge::from_view(&view);
                let touched: CardSet = actions
                    .iter()
                    .flat_map(|a| match *a {
                        Action::Play(c) => CardSet::single(c),
                        Action::Discard(s) => s,
                        _ => CardSet::EMPTY,
                    })
                    .collect();
                let (a, b) = (Seat::Mittelhand, Seat::Hinterhand);
                let hands = game.omniscient_view().original_hands;
                let free = |seat: Seat| hands[seat.index()] - touched;
                let mut alt = None;
                'search: for x in free(a).iter() {
                    for y in free(b).iter() {
                        if x.suit() == y.suit() {
                            continue;
                        }
                        let mut h = hands;
                        h[a.index()].remove(x);
                        h[a.index()].insert(y);
                        h[b.index()].remove(y);
                        h[b.index()].insert(x);
                        let deal = Deal::new(h, deal.skat()).unwrap();
                        if let Some(g) = replay(deal, &actions) {
                            alt = Some(g);
                            break 'search;
                        }
                    }
                }
                let Some(alt) = alt else {
                    break;
                };
                let other = alt.omniscient_view().hands;
                assert_ne!(other[a.index()], game.omniscient_view().hands[a.index()]);
                let alt_view = alt.view(human);
                assert_eq!(alt_view, view, "the human's view is unchanged");
                let k = Knowledge::from_view(&alt_view);
                assert_eq!(k, reference);
                assert_eq!(
                    CountingAid::from_knowledge(&k),
                    CountingAid::from_knowledge(&reference)
                );
                let kinds = applicable_questions(&k);
                assert_eq!(kinds, applicable_questions(&reference));
                for kind in kinds {
                    assert_eq!(derive(&k, kind), derive(&reference, kind));
                }
                checked += 1;
                // Advance by the human's or an opponent's first legal card.
                let next = game.legal_actions()[0];
                game.apply(next).unwrap();
                actions.push(next);
            }
        }
        assert!(checked >= 20, "only {checked} substitutions found");
    }
}
