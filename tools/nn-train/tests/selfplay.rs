// SPDX-License-Identifier: EUPL-1.2

//! Self-play records (#70): only the acting seat's permitted view goes in,
//! hidden cards do not change a record, every phase, role and seat is
//! covered, Skat and Ouvert visibility hold, and the split is by deal.

use std::collections::BTreeSet;

use nn_train::selfplay::{
    DecisionKind, DecisionRecord, RULES, RunSeed, Split, TeacherProfile, play, play_deal, teach,
};
use skat_ai::{Budget, Level, Opponent, Profile, Status};
use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Game, GameType, Phase, Rank, Seat, Suit,
};

/// Strong with a small sample budget, so the tests stay fast; the record
/// logic is the same as with `Budget::INTERACTIVE`.
const TEST_TEACHER: TeacherProfile = TeacherProfile {
    name: "test",
    profile: Profile {
        budget: Some(Budget {
            samples: 2,
            nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
        }),
        ..Level::Strong.profile()
    },
};

const SEED: RunSeed = RunSeed(70);

fn teacher(deal: u64, seat: Seat) -> Opponent {
    Opponent {
        profile: TEST_TEACHER.profile,
        seed: SEED.teacher_seed(deal, seat),
    }
}

/// The game of `deal` after the scripted `prefix` and the first `n`
/// recorded actions.
fn replay(deal: Deal, prefix: &[Action], records: &[DecisionRecord], n: usize) -> Game {
    let mut game = Game::from_deal(deal, RULES);
    for &a in prefix {
        game.apply(a).expect("scripted actions are legal");
    }
    for r in &records[..n] {
        game.apply(r.action).expect("recorded actions are legal");
    }
    game
}

/// Swaps one card between the dealt hands of the two seats other than
/// `seat` such that every action before the decision stays legal: neither
/// card was played or discarded before it, and both follow the same suit
/// in the announced game. `None` if no such pair exists.
fn substitute(deal: Deal, records: &[DecisionRecord], k: usize) -> Option<Deal> {
    let seat = records[k].view.seat;
    let [p, q] = [seat.next(), seat.next().next()];
    let mut used = CardSet::EMPTY;
    for r in &records[..k] {
        match r.action {
            Action::Play(c) => {
                used.insert(c);
            }
            Action::Discard(cards) => used |= cards,
            _ => {}
        }
    }
    let game_type = records[k].view.contract.map(|c| c.game);
    for x in deal.hand(p).iter().filter(|c| !used.contains(*c)) {
        for y in deal.hand(q).iter().filter(|c| !used.contains(*c)) {
            if game_type.is_some_and(|g| g.follow(x) != g.follow(y)) {
                continue;
            }
            let mut hands = Seat::ALL.map(|s| deal.hand(s));
            hands[p.index()].remove(x);
            hands[p.index()].insert(y);
            hands[q.index()].remove(y);
            hands[q.index()].insert(x);
            return Some(Deal::new(hands, deal.skat()).expect("a swap keeps a valid deal"));
        }
    }
    None
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn hidden_cards_do_not_change_a_record() {
    let mut checked = 0;
    for deal in 0..2 {
        let records = play_deal(SEED, TEST_TEACHER.profile, deal);
        let original = Deal::from_seed(SEED.deal_seed(deal));
        for k in 0..records.len() {
            let Some(other) = substitute(original, &records, k) else {
                continue;
            };
            assert_ne!(other, original);
            let record = &records[k];
            let seat = record.view.seat;
            let game = replay(other, &[], &records, k);
            let view = game.view(seat);
            if view != record.view {
                // Only Ouvert reveals the other hands; self-play has none.
                assert!(
                    record.view.declarer_cards.is_some(),
                    "deal {deal}, decision {k}"
                );
                continue;
            }
            let (action, taught) = teach(&view, &teacher(deal, seat)).expect("seat to move");
            let again = DecisionRecord {
                view,
                action,
                teacher: taught,
                final_points: record.final_points,
                ..record.clone()
            };
            assert_eq!(&again, record, "deal {deal}, decision {k}");
            checked += 1;
        }
    }
    assert!(checked > 40, "only {checked} decisions checked");
}

/// Each record holds exactly the acting seat's permitted view: the Skat
/// only for the declarer after pickup, the declarer's cards only once an
/// Ouvert game revealed them.
fn check_visibility(deal: Deal, prefix: &[Action], records: &[DecisionRecord]) {
    for (k, r) in records.iter().enumerate() {
        let game = replay(deal, prefix, records, k);
        let seat = r.view.seat;
        assert_eq!(r.view, game.view(seat), "decision {k}");
        assert_eq!(game.current_seat(), Some(seat));
        assert!(r.view.legal_actions.contains(&r.action));
        assert_eq!(r.view.result, None, "no outcome before the end");
        let declarer = r.view.declarer == Some(seat);
        let skat = deal.skat();
        if !declarer {
            assert_eq!(r.view.hand & skat, CardSet::EMPTY);
            assert_eq!(r.view.own_skat, None);
        }
        if r.view.phase == Phase::Druecken {
            assert!(declarer);
            assert_eq!(r.view.hand.len(), 12);
            assert_eq!(r.view.hand & skat, skat, "the declarer holds the Skat");
        }
        if declarer && r.view.hand_game == Some(false) && r.view.phase == Phase::Spiel {
            assert!(r.view.own_skat.is_some_and(|s| s.len() == 2));
        }
        let ouvert = r.view.contract.is_some_and(|c| c.ouvert);
        match r.view.declarer_cards {
            Some(cards) => {
                assert!(ouvert && game.is_revealed());
                let d = r.view.declarer.expect("declarer");
                assert_eq!(cards, game.view(d).hand);
            }
            None => assert!(!ouvert || !game.is_revealed() || declarer),
        }
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn records_cover_phases_roles_seats_and_visibility() {
    let mut kinds = BTreeSet::new();
    let mut seats = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut analysed = 0;
    for deal in 0..6 {
        let records = play_deal(SEED, TEST_TEACHER.profile, deal);
        assert!(!records.is_empty());
        check_visibility(Deal::from_seed(SEED.deal_seed(deal)), &[], &records);
        let split = Split::of(SEED.deal_seed(deal));
        for r in &records {
            assert_eq!((r.deal, r.split), (deal, split));
            kinds.insert(DecisionKind::of(&r.view));
            seats.insert(r.view.seat.index());
            roles.insert(match r.view.declarer {
                None => "open",
                Some(d) if d == r.view.seat => "declarer",
                Some(_) => "defender",
            });
            if r.teacher.status.is_some() {
                analysed += 1;
                let actions: Vec<Action> = r.teacher.options.iter().map(|o| o.action).collect();
                assert_eq!(actions, r.view.legal_actions);
            } else {
                assert!(r.teacher.options.is_empty());
            }
        }
    }
    assert_eq!(seats.len(), 3);
    assert_eq!(roles.len(), 3, "{roles:?}");
    for kind in [
        DecisionKind::Reizen,
        DecisionKind::HandOrPickup,
        DecisionKind::Druecken,
        DecisionKind::Ansage,
        DecisionKind::KontraRe,
        DecisionKind::Spiel,
    ] {
        assert!(kinds.contains(&kind), "{kind:?} missing: {kinds:?}");
    }
    assert!(analysed > 50);
}

fn card(suit: Suit, rank: Rank) -> Card {
    Card::new(suit, rank)
}

/// VH: all Jacks and Kreuz; MH: Pik and Herz; HH: Herz and Karo.
fn fixed_deal() -> Deal {
    use Rank::*;
    use Suit::*;
    let set = |cards: &[(Suit, Rank)]| cards.iter().map(|&(s, r)| card(s, r)).collect();
    Deal::new(
        [
            set(&[
                (Kreuz, Bube),
                (Pik, Bube),
                (Herz, Bube),
                (Karo, Bube),
                (Kreuz, Ass),
                (Kreuz, Zehn),
                (Kreuz, Koenig),
                (Kreuz, Dame),
                (Kreuz, Neun),
                (Kreuz, Acht),
            ]),
            set(&[
                (Pik, Ass),
                (Pik, Zehn),
                (Pik, Koenig),
                (Pik, Dame),
                (Pik, Neun),
                (Pik, Acht),
                (Pik, Sieben),
                (Herz, Ass),
                (Herz, Zehn),
                (Herz, Koenig),
            ]),
            set(&[
                (Herz, Dame),
                (Herz, Neun),
                (Herz, Acht),
                (Herz, Sieben),
                (Karo, Ass),
                (Karo, Zehn),
                (Karo, Koenig),
                (Karo, Dame),
                (Karo, Neun),
                (Karo, Acht),
            ]),
        ],
        [card(Kreuz, Sieben), card(Karo, Sieben)]
            .into_iter()
            .collect(),
    )
    .expect("valid deal")
}

/// MH 18, VH holds, MH and HH pass, VH plays Hand.
const VH_HAND_AT_18: [Action; 5] = [
    Action::Bid(18),
    Action::Hold,
    Action::Pass,
    Action::Pass,
    Action::Hand,
];

fn teachers() -> [Opponent; 3] {
    Seat::ALL.map(|s| teacher(99, s))
}

#[test]
fn ouvert_records_show_the_revealed_declarer_cards() {
    let mut game = Game::from_deal(fixed_deal(), RULES);
    let mut history = Vec::new();
    let ouvert = Announcement {
        game: GameType::Grand,
        schneider: true,
        schwarz: true,
        ouvert: true,
    };
    for a in VH_HAND_AT_18.into_iter().chain([Action::Announce(ouvert)]) {
        game.apply(a).expect("legal");
        history.push(a);
    }
    let (records, end) = play(game, &teachers(), 0, Split::Train);
    assert!(end.result().is_some());
    check_visibility(fixed_deal(), &history, &records);
    // The Kontra round comes before the reveal; in card play every
    // defender sees the declarer's current cards.
    let defenders_in_play: Vec<_> = records
        .iter()
        .filter(|r| {
            r.view.seat != Seat::Vorhand && DecisionKind::of(&r.view) == DecisionKind::Spiel
        })
        .collect();
    assert!(!defenders_in_play.is_empty());
    assert!(
        defenders_in_play
            .iter()
            .all(|r| r.view.declarer_cards.is_some())
    );
    assert_eq!(records[0].view.declarer_cards, None);
    assert_eq!(
        DecisionKind::of(&records[0].view),
        DecisionKind::KontraRe,
        "the Ouvert Kontra round"
    );
}

#[test]
fn re_decisions_are_recorded() {
    let mut game = Game::from_deal(fixed_deal(), RULES);
    for a in VH_HAND_AT_18 {
        game.apply(a).expect("legal");
    }
    game.apply(Action::Announce(Announcement::plain(GameType::Grand)))
        .expect("legal");
    game.apply(Action::Play(card(Suit::Kreuz, Rank::Bube)))
        .expect("legal");
    game.apply(Action::Kontra).expect("MH may say Kontra");
    let (records, _) = play(game, &teachers(), 0, Split::Train);
    let re = &records[0];
    assert_eq!(re.view.seat, Seat::Vorhand);
    assert_eq!(re.view.legal_actions, vec![Action::Re, Action::KeinRe]);
    assert_eq!(DecisionKind::of(&re.view), DecisionKind::KontraRe);
    assert_eq!(re.teacher.status, Some(Status::Complete));
    assert_eq!(re.teacher.options.len(), 2);
}

#[test]
fn split_is_by_deal() {
    let mut counts = [0u32; 3];
    for deal in 0..2000 {
        let seed = SEED.deal_seed(deal);
        let split = Split::of(seed);
        assert_eq!(split, Split::of(seed), "a deal has one split");
        counts[split as usize] += 1;
    }
    // About 80/10/10.
    assert!((1500..1700).contains(&counts[0]), "{counts:?}");
    assert!((120..280).contains(&counts[1]), "{counts:?}");
    assert!((120..280).contains(&counts[2]), "{counts:?}");
    // Teacher seeds vary by seat and deal but never change the split.
    let a = SEED.teacher_seed(3, Seat::Vorhand);
    assert_ne!(a, SEED.teacher_seed(3, Seat::Mittelhand));
    assert_ne!(a, SEED.teacher_seed(4, Seat::Vorhand));
    assert_ne!(a, SEED.deal_seed(3));
}
