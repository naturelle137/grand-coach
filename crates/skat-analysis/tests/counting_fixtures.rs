// SPDX-License-Identifier: EUPL-1.2

//! Counting aid on legal games (M6, D-64, coach-validation A2): fixed deals
//! are replayed through `skat-core`, each seat's view goes through
//! `Knowledge::from_view`, and the aid is compared with counts stated here
//! from the played card lists (own card values, no code under test).

use skat_analysis::Knowledge;
use skat_analysis::counting::CountingAid;
use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Game, GameType, Phase, Rank, RuleConfig, Seat, Suit,
};

/// Augen of a card, stated independently of `skat-core`.
fn augen(card: Card) -> u32 {
    match card.rank() {
        Rank::Ass => 11,
        Rank::Zehn => 10,
        Rank::Koenig => 4,
        Rank::Dame => 3,
        Rank::Bube => 2,
        _ => 0,
    }
}

fn sum(cards: &[Card]) -> u32 {
    cards.iter().map(|&c| augen(c)).sum()
}

struct Scenario {
    game: Game,
    declarer: Seat,
    defender: Seat,
    /// The two discarded cards, if the declarer picked up the Skat.
    discard: Vec<Card>,
    /// Cards played in order.
    played: Vec<Card>,
}

/// Plays `deal` with Vorhand declaring `announcement`; `hand` plays Hand,
/// otherwise the Skat is taken and the two highest-Augen cards of the
/// declarer (first found) are discarded. Then `plays` cards are played, each
/// the first legal one.
fn scenario(seed: u64, announcement: Announcement, hand: bool, plays: usize) -> Scenario {
    let mut game = Game::from_deal(Deal::from_seed(seed), RuleConfig::default());
    let declarer = Seat::Vorhand;
    let mut discard = Vec::new();
    while game.phase() != Phase::Spiel {
        let seat = game.current_seat().expect("game in progress");
        let legal = game.legal_actions();
        let action = match game.phase() {
            Phase::Reizen if seat == declarer => legal
                .iter()
                .copied()
                .find(|a| matches!(a, Action::Bid(18)))
                .unwrap_or(Action::Hold),
            Phase::Reizen => Action::Pass,
            Phase::SkatAufnahme if hand => Action::Hand,
            Phase::SkatAufnahme => Action::TakeSkat,
            Phase::Druecken => {
                let mut cards: Vec<Card> = game.view(declarer).hand.iter().collect();
                cards.sort_by_key(|&c| std::cmp::Reverse(augen(c)));
                discard = cards[..2].to_vec();
                Action::Discard(discard.iter().copied().collect())
            }
            Phase::Ansage => Action::Announce(announcement),
            Phase::Beendet | Phase::Eingepasst => panic!("seed {seed} ended early"),
            Phase::Spiel => unreachable!(),
        };
        game.apply(action).unwrap();
    }
    let mut played = Vec::new();
    while played.len() < plays {
        let legal = game.legal_actions();
        let action = match legal.iter().find(|a| matches!(a, Action::Play(_))) {
            Some(&a) => a,
            None => *legal
                .iter()
                .find(|a| matches!(a, Action::KeinKontra | Action::KeinRe))
                .unwrap_or(&legal[0]),
        };
        if let Action::Play(card) = action {
            played.push(card);
        }
        game.apply(action).unwrap();
    }
    let defender = Seat::Mittelhand;
    Scenario {
        game,
        declarer,
        defender,
        discard,
        played,
    }
}

/// The cards of `list` as a set, for stating fixtures readably.
fn set(list: &[(Suit, Rank)]) -> CardSet {
    list.iter().map(|&(s, r)| Card::new(s, r)).collect()
}

fn aid(s: &Scenario, seat: Seat) -> (CountingAid, Knowledge) {
    let k = Knowledge::from_view(&s.game.view(seat));
    (CountingAid::from_knowledge(&k), k)
}

/// Checks what the fixture itself claims, so that a change of the deal or
/// of the replay cannot silently change what the numbers below mean.
fn assert_fixture(s: &Scenario, played: &[(Suit, Rank)], discard: &[(Suit, Rank)]) {
    let played: Vec<Card> = played.iter().map(|&(s, r)| Card::new(s, r)).collect();
    assert_eq!(s.played, played);
    let discard: Vec<Card> = discard.iter().map(|&(s, r)| Card::new(s, r)).collect();
    assert_eq!(s.discard, discard);
    assert_eq!(sum(&s.played), sum(&played));
}

use Rank::*;
use Suit::*;

/// Seed 1, Farbspiel Herz, Skat taken (discard Karo-Ass and Herz-Zehn),
/// four tricks and two cards of the fifth. Trumpf: the four Buben and the
/// seven Herz cards.
///
/// Played Augen: Kreuz-Dame 3, Kreuz-Koenig 4, Kreuz-Bube 2, Pik-Ass 11,
/// Kreuz-Ass 11, Pik-Dame 3, Kreuz-Zehn 10, Pik-Bube 2 = 46. Tricks: the
/// declarer won Koenig+Bube+Neun (6), the defenders Dame (3), Pik-Ass (11)
/// and the Ass trick (24) = 38.
#[test]
fn farbspiel_declarer_and_defender_from_a_legal_game() {
    let s = scenario(1, Announcement::plain(GameType::Farbe(Herz)), false, 14);
    assert_fixture(
        &s,
        &[
            (Kreuz, Sieben),
            (Kreuz, Acht),
            (Kreuz, Dame),
            (Kreuz, Koenig),
            (Kreuz, Bube),
            (Kreuz, Neun),
            (Pik, Sieben),
            (Pik, Acht),
            (Pik, Ass),
            (Kreuz, Ass),
            (Pik, Dame),
            (Kreuz, Zehn),
            (Pik, Bube),
            (Herz, Sieben),
        ],
        &[(Karo, Ass), (Herz, Zehn)],
    );

    // Declarer: hand Pik-Koenig 4, Herz-Bube 2, Herz-Dame 3, two blanks = 9;
    // discard 11 + 10 = 21; outstanding 120 - 9 - 46 - 21 = 44. Trumpf:
    // own Herz-Bube and Herz-Dame, played Kreuz-Bube, Pik-Bube and
    // Herz-Sieben, discarded Herz-Zehn: 11 - 2 - 3 - 1 = 5.
    let (a, k) = aid(&s, s.declarer);
    assert_eq!(
        k.own,
        set(&[
            (Pik, Koenig),
            (Herz, Bube),
            (Herz, Dame),
            (Karo, Sieben),
            (Karo, Neun)
        ])
    );
    let t = a.trumpf.unwrap();
    assert_eq!(
        (t.total, t.own, t.played, t.discarded, t.outside),
        (11, 2, 3, 1, 5)
    );
    assert!(!t.skat_unknown);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(6), Some(38)));
    assert_eq!(a.augen.outstanding, 44);
    assert_eq!(a.augen.skat, Some(21));
    assert_eq!(a.played.len(), 14);
    assert_eq!(
        u32::from(a.augen.outstanding),
        120 - sum(&k.own.iter().collect::<Vec<_>>()) - sum(&s.played) - sum(&s.discard)
    );

    // Defender (Mittelhand): hand Pik-Zehn 10, Herz-Koenig 4, Herz-Ass 11,
    // Pik-Neun, Karo-Acht, Karo-Dame 3 = 28; outstanding 120 - 28 - 46 = 46.
    // Trumpf: own Herz-Koenig and Herz-Ass, three played: 11 - 2 - 3 = 6,
    // the unknown Skat among them.
    let (a, k) = aid(&s, s.defender);
    assert_eq!(
        k.own,
        set(&[
            (Pik, Neun),
            (Pik, Zehn),
            (Herz, Koenig),
            (Herz, Ass),
            (Karo, Acht),
            (Karo, Dame)
        ])
    );
    let t = a.trumpf.unwrap();
    assert_eq!(
        (t.total, t.own, t.played, t.discarded, t.outside),
        (11, 2, 3, 0, 6)
    );
    assert!(t.skat_unknown);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(6), Some(38)));
    assert_eq!(a.augen.outstanding, 46);
    assert_eq!(a.augen.skat, None);
    // Vorhand (the declarer) played Pik-Dame on Kreuz-Ass: a proven Kreuz
    // void, visible to the defender; nobody else has shown one.
    let v = a.voids[Seat::Vorhand.index()];
    assert!(v.suits[usize::from(Kreuz.index())] && !v.trumpf);
    assert!(a.voids[Seat::Hinterhand.index()].is_empty());
}

/// Seed 1, Grand Hand: the declarer never sees the Skat. Played Augen:
/// 46 as above, but Herz-Bube (2) instead of Herz-Sieben (0): 48. Tricks as
/// in the Farbspiel fixture: declarer 6, defenders 38.
#[test]
fn grand_hand_declarer_and_defender_from_a_legal_game() {
    let s = scenario(1, Announcement::plain(GameType::Grand), true, 14);
    assert_fixture(
        &s,
        &[
            (Kreuz, Sieben),
            (Kreuz, Acht),
            (Kreuz, Dame),
            (Kreuz, Koenig),
            (Kreuz, Bube),
            (Kreuz, Neun),
            (Pik, Sieben),
            (Pik, Acht),
            (Pik, Ass),
            (Kreuz, Ass),
            (Pik, Dame),
            (Kreuz, Zehn),
            (Pik, Bube),
            (Herz, Bube),
        ],
        &[],
    );
    // Declarer: Pik-Koenig 4, Herz-Dame 3, Karo-Ass 11, two blanks = 18;
    // outstanding 120 - 18 - 48 = 54. The Skat is unknown (Hand), so the
    // one Bube not played and not own may lie in it.
    let (a, _) = aid(&s, s.declarer);
    let t = a.trumpf.unwrap();
    assert_eq!(
        (t.total, t.own, t.played, t.discarded, t.outside),
        (4, 0, 3, 0, 1)
    );
    assert!(t.skat_unknown);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(6), Some(38)));
    assert_eq!(a.augen.outstanding, 54);
    assert_eq!(a.augen.skat, None);
    // Defender: Pik-Zehn 10, Herz-Koenig 4, Herz-Ass 11, Karo-Dame 3 and two
    // blanks = 28; outstanding 120 - 28 - 48 = 44.
    let (a, _) = aid(&s, s.defender);
    assert_eq!(a.augen.outstanding, 44);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(6), Some(38)));
    assert_eq!(a.trumpf.unwrap().outside, 1);
}

/// Seed 1, Null with the Skat taken (discard Karo-Ass and Herz-Zehn, 21
/// Augen), five cards: one trick (Hinterhand won Kreuz-Dame, 3 Augen) and
/// Kreuz-Koenig, Kreuz-Bube in the next. Null has no Trumpf; Augen won by
/// the declarer 0, by the defenders 3 (the trick in progress is not won).
#[test]
fn null_declarer_and_defender_from_a_legal_game() {
    let s = scenario(1, Announcement::plain(GameType::Null), false, 5);
    assert_fixture(
        &s,
        &[
            (Kreuz, Sieben),
            (Kreuz, Acht),
            (Kreuz, Dame),
            (Kreuz, Koenig),
            (Kreuz, Bube),
        ],
        &[(Karo, Ass), (Herz, Zehn)],
    );
    // Played: Dame 3 + Koenig 4 + Bube 2 = 9.
    // Declarer: Pik-Dame 3, Pik-Koenig 4, Herz-Bube 2, Herz-Dame 3 = 12;
    // outstanding 120 - 12 - 9 - 21 = 78.
    let (a, _) = aid(&s, s.declarer);
    assert_eq!(a.trumpf, None);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(0), Some(3)));
    assert_eq!(a.augen.outstanding, 78);
    assert_eq!(a.augen.skat, Some(21));
    // Defender: Kreuz-Neun, Kreuz-Zehn 10, Pik-Zehn 10, Herz-Koenig 4,
    // Herz-Ass 11, Karo-Dame 3 and blanks = 38; outstanding 120 - 38 - 9
    // = 73 (the unknown Skat's 21 Augen stay in it).
    let (a, _) = aid(&s, s.defender);
    assert_eq!(a.trumpf, None);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(0), Some(3)));
    assert_eq!(a.augen.outstanding, 73);
    assert_eq!(a.augen.skat, None);
}

/// Seed 10, Grand Ouvert (a Hand game): the defenders see the declarer's
/// cards. Eight cards played: two tricks (the defenders' Hinterhand won
/// Kreuz 7-8-9 for 0, the declarer Kreuz-Dame, Ass, Zehn for 24) and Pik-Zehn,
/// Pik-Acht on the table. The declarer still holds Herz-Bube and Karo-Bube.
#[test]
fn grand_ouvert_shows_the_declarer_cards_to_the_defender() {
    let ouvert = Announcement {
        game: GameType::Grand,
        schneider: true,
        schwarz: true,
        ouvert: true,
    };
    let s = scenario(10, ouvert, true, 8);
    assert!(s.game.contract().unwrap().ouvert);
    assert_fixture(
        &s,
        &[
            (Kreuz, Sieben),
            (Kreuz, Acht),
            (Kreuz, Neun),
            (Kreuz, Dame),
            (Kreuz, Ass),
            (Kreuz, Zehn),
            (Pik, Zehn),
            (Pik, Acht),
        ],
        &[],
    );
    let declarer_hand = set(&[
        (Pik, Dame),
        (Pik, Koenig),
        (Pik, Ass),
        (Herz, Neun),
        (Herz, Bube),
        (Karo, Bube),
        (Karo, Ass),
    ]);
    // Played Augen 3 + 11 + 10 + 10 = 34.
    // Declarer: hand 3 + 4 + 11 + 2 + 2 + 11 = 33; outstanding 120 - 33 - 34
    // = 53. Two of the four Buben are his, none played: outside 2.
    let (a, k) = aid(&s, s.declarer);
    assert_eq!(k.own, declarer_hand);
    let t = a.trumpf.unwrap();
    assert_eq!((t.own, t.played, t.outside), (2, 0, 2));
    assert!(t.skat_unknown);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(24), Some(0)));
    assert_eq!(a.augen.outstanding, 53);
    // Defender: hand Kreuz-Bube 2, Pik-Bube 2, Herz-Koenig 4, Karo-Zehn 10,
    // Karo-Koenig 4, two blanks = 22; outstanding 120 - 22 - 34 = 64. The
    // two outside Buben are the declarer's, seen face up: revealed, not
    // unseen, but still outside by definition.
    let (a, k) = aid(&s, s.defender);
    assert_eq!(
        k.own,
        set(&[
            (Kreuz, Bube),
            (Pik, Bube),
            (Herz, Sieben),
            (Herz, Koenig),
            (Karo, Neun),
            (Karo, Zehn),
            (Karo, Koenig)
        ])
    );
    assert_eq!(k.known_elsewhere, declarer_hand);
    let sets = k.trumpf_sets().unwrap();
    assert_eq!(sets.revealed, set(&[(Herz, Bube), (Karo, Bube)]));
    assert!(sets.unseen.is_empty());
    let t = a.trumpf.unwrap();
    assert_eq!((t.own, t.played, t.outside), (2, 0, 2));
    assert_eq!(a.augen.outstanding, 64);
    assert_eq!((a.augen.declarer, a.augen.defenders), (Some(24), Some(0)));
}
