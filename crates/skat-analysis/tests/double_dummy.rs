// SPDX-License-Identifier: EUPL-1.2

//! Double-dummy solver: hand-solved positions for Farbspiel, Grand and Null,
//! and a cross-check of small random positions against a plain exhaustive
//! minimax over `skat_core::settle` (no pruning, no tables, no outcome
//! classes).

mod common;

use common::*;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_analysis::Meter;
use skat_analysis::double_dummy::{
    AugenClass, DdValue, Position, Solver, Stakes, TrickClass, solve,
};
use skat_core::{
    Announcement, Card, CardSet, Contract, Deal, GameOutcome, GameType, KontraLevel, Seat, Suit,
    Trick, settle, spitzen,
};

/// Position with `tricks_played` completed tricks, `leader` to lead.
#[allow(clippy::too_many_arguments)]
fn pos(
    game: GameType,
    declarer: Seat,
    hands: [&str; 3],
    skat: &str,
    leader: Seat,
    declarer_augen: u8,
    declarer_tricks: u8,
    tricks_played: u8,
) -> Position {
    let p = Position {
        game,
        declarer,
        hands: hands.map(cards),
        skat: cards(skat),
        trick: Trick {
            leader: Some(leader),
            cards: [None; 3],
        },
        declarer_augen,
        declarer_tricks,
        tricks_played,
    };
    assert_eq!(p.check(), Ok(()));
    p
}

fn dd(p: &Position) -> DdValue {
    solve(p, &mut Meter::unlimited()).unwrap()
}

fn exact(p: &Position) -> u8 {
    Solver::new(p.game, p.declarer)
        .exact_augen(p, &mut Meter::unlimited())
        .unwrap()
}

fn stakes(game: GameType, hand: bool, bid: u16, declarer_cards: CardSet) -> Stakes {
    Stakes {
        contract: Contract::new(Announcement::plain(game), hand),
        bid,
        spitzen: spitzen(game, declarer_cards),
        kontra: KontraLevel::None,
    }
}

#[test]
fn grand_declarer_with_all_top_trumps_takes_every_trick() {
    // VH holds the three top Jacks; DJ falls under CJ.
    let p = pos(
        GameType::Grand,
        Seat::Vorhand,
        ["CJ SJ HJ", "DJ H7 H8", "S7 S8 S9"],
        "C7 C8",
        Seat::Vorhand,
        112,
        7,
        7,
    );
    let v = dd(&p);
    assert_eq!(
        v,
        DdValue {
            augen: Some(AugenClass::of(120)),
            tricks: TrickClass::All
        }
    );
    // Mit 3, game, Schneider, Schwarz: 24 * 6.
    let s = stakes(GameType::Grand, false, 18, cards("CJ SJ HJ"));
    let result = v.settle(Seat::Vorhand, &s);
    assert!(result.won && result.schwarz);
    assert_eq!(result.score, 144);
}

#[test]
fn grand_defender_trumps_the_last_trick() {
    // Declarer must cash HA first (H7 first loses both tricks); then MH
    // trumps H7 with SJ. 85 + 11 = 96: Schneider, not Schwarz.
    let p = pos(
        GameType::Grand,
        Seat::Vorhand,
        ["HA H7", "H8 SJ", "DT D7"],
        "C7 C8",
        Seat::Vorhand,
        85,
        8,
        8,
    );
    assert_eq!(
        dd(&p),
        DdValue {
            augen: Some(AugenClass::of(96)),
            tricks: TrickClass::Some
        }
    );
    assert_eq!(exact(&p), 96);
}

#[test]
fn grand_defenders_keep_the_declarer_without_a_trick() {
    let p = pos(
        GameType::Grand,
        Seat::Vorhand,
        ["H7 H8", "HA HK", "D7 D8"],
        "C7 C8",
        Seat::Vorhand,
        0,
        0,
        8,
    );
    let v = dd(&p);
    assert_eq!(
        v,
        DdValue {
            augen: Some(AugenClass::of(0)),
            tricks: TrickClass::None
        }
    );
    // Lost Schwarz: ohne 4, game, Schneider, Schwarz = 7 levels, doubled.
    let s = stakes(GameType::Grand, false, 18, cards("H7 H8 C7 C8"));
    let result = v.settle(Seat::Vorhand, &s);
    assert!(!result.won && result.schwarz);
    assert_eq!(result.score, -2 * 24 * 7);
}

#[test]
fn herz_defenders_cash_the_ace_before_the_ten_falls() {
    // VH leads SA (MH must follow with ST), then S7 to the declarer's H7:
    // the declarer gets no Augen. Leading S7 first would give 25.
    let herz = GameType::Farbe(Suit::Herz);
    let p = pos(
        herz,
        Seat::Mittelhand,
        ["SA S7", "H7 ST", "SK D7"],
        "C7 C8",
        Seat::Vorhand,
        40,
        3,
        8,
    );
    let v = dd(&p);
    assert_eq!(
        v,
        DdValue {
            augen: Some(AugenClass::of(40)),
            tricks: TrickClass::Some
        }
    );
    assert_eq!(exact(&p), 40);
    let result = v.settle(Seat::Mittelhand, &stakes(herz, false, 18, cards("H7")));
    assert!(!result.won && !result.schneider);
}

#[test]
fn herz_boundary_at_61_decides_won_or_lost() {
    // Declarer HH trumps CA with H7: 50 + 11 = 61 with one trick left that
    // the defenders win (D8 under DA). Exactly 61: won.
    let herz = GameType::Farbe(Suit::Herz);
    let p = pos(
        herz,
        Seat::Hinterhand,
        ["CA DA", "C7 D7", "H7 D8"],
        "C8 C9",
        Seat::Vorhand,
        50,
        5,
        8,
    );
    let v = dd(&p);
    assert_eq!(v.augen, Some(AugenClass::UpTo89));
    assert_eq!(exact(&p), 61);
    let result = v.settle(Seat::Hinterhand, &stakes(herz, false, 20, cards("H7")));
    assert!(result.won);
    // Announced Schneider (Hand) with the same cards is lost.
    let schneider = Stakes {
        contract: Contract::new(
            Announcement {
                schneider: true,
                ..Announcement::plain(herz)
            },
            true,
        ),
        ..stakes(herz, true, 20, cards("H7"))
    };
    assert!(!v.settle(Seat::Hinterhand, &schneider).won);
}

#[test]
fn null_declarer_ducks_under_both_defenders() {
    let p = pos(
        GameType::Null,
        Seat::Vorhand,
        ["H7 S7", "H8 S8", "H9 S9"],
        "C7 C8",
        Seat::Vorhand,
        0,
        0,
        8,
    );
    let v = dd(&p);
    assert_eq!(
        v,
        DdValue {
            augen: None,
            tricks: TrickClass::None
        }
    );
    let null = Stakes {
        contract: Contract::new(Announcement::plain(GameType::Null), false),
        bid: 23,
        spitzen: None,
        kontra: KontraLevel::None,
    };
    let result = v.settle(Seat::Vorhand, &null);
    assert!(result.won);
    assert_eq!(result.score, 23);
}

#[test]
fn null_declarer_with_a_bare_ace_is_caught() {
    // Leading H7 lets MH win with H8 or H9 and lead the other heart under
    // which HA must fall; leading HA wins at once.
    let p = pos(
        GameType::Null,
        Seat::Vorhand,
        ["HA H7", "H8 H9", "S7 S8"],
        "C7 C8",
        Seat::Vorhand,
        0,
        0,
        8,
    );
    assert_eq!(dd(&p).tricks, TrickClass::Some);
    // A Null that is already lost stays lost.
    let mut lost = p;
    lost.declarer_tricks = 1;
    assert_eq!(dd(&lost).tricks, TrickClass::Some);
}

#[test]
fn null_ouvert_lost_scores_double_the_fixed_value() {
    let p = pos(
        GameType::Null,
        Seat::Vorhand,
        ["HA H7", "H8 H9", "S7 S8"],
        "C7 C8",
        Seat::Vorhand,
        0,
        0,
        8,
    );
    let ouvert = Stakes {
        contract: Contract::new(
            Announcement {
                ouvert: true,
                ..Announcement::plain(GameType::Null)
            },
            false,
        ),
        bid: 46,
        spitzen: None,
        kontra: KontraLevel::Kontra,
    };
    let result = dd(&p).settle(Seat::Vorhand, &ouvert);
    assert!(!result.won);
    assert_eq!(result.score, -2 * 46 * 2);
}

#[test]
fn full_deal_from_the_first_trick_is_solved() {
    // VH holds all Jacks and six Kreuz: a Kreuz game is Schwarz.
    let d = deal(
        "CJ SJ HJ DJ CA CT CK CQ C9 C8",
        "SA ST SK SQ S9 S8 S7 HA HT HK",
        "HQ H9 H8 H7 DA DT DK DQ D9 D8",
        "C7 D7",
    );
    let p = start(&d, GameType::Farbe(Suit::Kreuz), Seat::Vorhand);
    let mut meter = Meter::unlimited();
    let v = solve(&p, &mut meter).unwrap();
    assert_eq!(v.tricks, TrickClass::All);
    assert_eq!(v.augen, Some(AugenClass::From90));
    assert!(meter.nodes() > 0);
}

#[test]
fn budget_exhaustion_is_reported() {
    let d = Deal::from_seed(11);
    let p = start(&d, GameType::Grand, Seat::Mittelhand);
    let mut meter = Meter::new(100, None);
    assert_eq!(
        solve(&p, &mut meter),
        Err(skat_analysis::Interrupted::BudgetExhausted)
    );
}

/// Start of card play for a Hand game of `declarer`.
fn start(d: &Deal, game: GameType, declarer: Seat) -> Position {
    Position {
        game,
        declarer,
        hands: Seat::ALL.map(|s| d.hand(s)),
        skat: d.skat(),
        trick: Trick::default(),
        declarer_augen: 0,
        declarer_tricks: 0,
        tricks_played: 0,
    }
}

/// Plain minimax over the settled score (declarer maximises).
fn reference(p: &Position, s: &Stakes) -> i32 {
    if p.is_over() {
        return settle(&GameOutcome {
            declarer: p.declarer,
            contract: s.contract,
            bid: s.bid,
            spitzen: s.spitzen,
            declarer_augen: p.declarer_augen + p.skat.augen(),
            declarer_tricks: p.declarer_tricks,
            tricks_played: p.tricks_played,
            kontra: s.kontra,
        })
        .score;
    }
    let values = p.legal_cards().iter().map(|card| {
        let mut child = *p;
        child.play(card);
        reference(&child, s)
    });
    if p.to_move() == p.declarer {
        values.max().unwrap()
    } else {
        values.min().unwrap()
    }
}

/// Plain minimax over the declarer's final Augen.
fn reference_augen(p: &Position) -> u8 {
    if p.is_over() {
        return p.declarer_augen + p.skat.augen();
    }
    let values = p.legal_cards().iter().map(|card| {
        let mut child = *p;
        child.play(card);
        reference_augen(&child)
    });
    if p.to_move() == p.declarer {
        values.max().unwrap()
    } else {
        values.min().unwrap()
    }
}

/// Contracts that make the outcome classes matter: plain, Schneider and
/// Schwarz announced, overbid, with Kontra and Re.
fn stakes_variants(game: GameType, declarer_cards: CardSet) -> Vec<Stakes> {
    if game == GameType::Null {
        return [(false, false), (true, false), (false, true), (true, true)]
            .map(|(hand, ouvert)| Stakes {
                contract: Contract::new(
                    Announcement {
                        ouvert,
                        ..Announcement::plain(GameType::Null)
                    },
                    hand,
                ),
                bid: 23,
                spitzen: None,
                kontra: if hand {
                    KontraLevel::Re
                } else {
                    KontraLevel::None
                },
            })
            .to_vec();
    }
    let base = stakes(game, false, 18, declarer_cards);
    let announced = |schneider, schwarz| Stakes {
        contract: Contract::new(
            Announcement {
                schneider,
                schwarz,
                ..Announcement::plain(game)
            },
            true,
        ),
        ..stakes(game, true, 18, declarer_cards)
    };
    vec![
        base,
        announced(true, false),
        announced(true, true),
        Stakes { bid: 120, ..base },
        Stakes {
            kontra: KontraLevel::Re,
            ..base
        },
    ]
}

#[test]
fn small_positions_match_exhaustive_minimax() {
    let games = [
        GameType::Grand,
        GameType::Farbe(Suit::Kreuz),
        GameType::Farbe(Suit::Karo),
        GameType::Null,
    ];
    let mut rng = ChaCha8Rng::seed_from_u64(2026);
    let mut checked = 0;
    for seed in 0..240u64 {
        let d = Deal::from_seed(seed);
        let game = games[(seed % 4) as usize];
        let declarer = Seat::from_index(seed as usize / 4);
        let mut p = start(&d, game, declarer);
        let declarer_cards = d.hand(declarer) | d.skat();
        let mut solver = Solver::new(game, declarer);
        // Play randomly down to 3 or 4 tricks left, sometimes mid-trick.
        let left = 3 + (seed % 2) as u8;
        let extra = (seed % 3) as usize;
        while !p.is_over() && (p.tricks_played < 10 - left || p.trick.len() < extra) {
            let legal: Vec<Card> = p.legal_cards().iter().collect();
            p.play(legal[rng.next_u32() as usize % legal.len()]);
            if p.tricks_played == 10 - left && p.trick.len() == extra {
                break;
            }
        }
        if p.is_over() {
            continue;
        }
        assert_eq!(p.check(), Ok(()));
        let v = solver.solve(&p, &mut Meter::unlimited()).unwrap();
        let mut experimental = Solver::new(game, declarer).with_discard_bound();
        assert_eq!(
            experimental.solve(&p, &mut Meter::unlimited()).unwrap(),
            v,
            "seed {seed}"
        );
        if game != GameType::Null {
            let augen = reference_augen(&p);
            assert_eq!(v.augen, Some(AugenClass::of(augen)), "seed {seed}");
            for threshold in [31, 61, 90] {
                assert_eq!(
                    experimental
                        .reaches_augen(&p, threshold, &mut Meter::unlimited())
                        .unwrap(),
                    augen >= threshold,
                    "seed {seed}, threshold {threshold}"
                );
            }
            for target in [augen, augen + 1] {
                let proved = Solver::new(game, declarer)
                    .with_discard_bound()
                    .reaches_augen(&p, target, &mut Meter::unlimited())
                    .unwrap();
                assert_eq!(
                    proved,
                    augen >= target,
                    "seed {seed} on either side of its exact value"
                );
            }
        }
        let exact = solver.exact_augen(&p, &mut Meter::unlimited()).unwrap();
        assert_eq!(exact, reference_augen(&p), "seed {seed}");
        for s in stakes_variants(game, declarer_cards) {
            assert_eq!(
                v.settle(declarer, &s).score,
                reference(&p, &s),
                "seed {seed} {s:?}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 150, "{checked}");
}

#[test]
fn positions_from_omniscient_view_need_card_play() {
    let game = skat_core::Game::new(5, skat_core::RuleConfig::default());
    assert_eq!(Position::from_omniscient(&game.omniscient_view()), None);
}

#[test]
fn inconsistent_positions_are_rejected() {
    let mut p = pos(
        GameType::Grand,
        Seat::Vorhand,
        ["HA H7", "H8 SJ", "DT D7"],
        "C7 C8",
        Seat::Vorhand,
        85,
        8,
        8,
    );
    p.hands[1].insert(c("HA"));
    assert_eq!(
        p.check(),
        Err(skat_analysis::double_dummy::PositionError::CardTwice(c(
            "HA"
        )))
    );
    p.hands[1].remove(c("HA"));
    p.skat.remove(c("C7"));
    assert_eq!(
        p.check(),
        Err(skat_analysis::double_dummy::PositionError::SkatSize)
    );
}

#[test]
fn classic_budget_counts_match_the_reference_search() {
    let mut rng = ChaCha8Rng::seed_from_u64(239);
    // A lead, second hand and third hand across all contracts. A changed
    // visit count can change a finite-budget teacher despite exact values.
    for (i, game) in [
        GameType::Grand,
        GameType::Farbe(Suit::Karo),
        GameType::Farbe(Suit::Herz),
        GameType::Farbe(Suit::Pik),
        GameType::Farbe(Suit::Kreuz),
        GameType::Null,
    ]
    .into_iter()
    .enumerate()
    {
        let mut p = start(&Deal::from_seed(i as u64), game, Seat::from_index(i));
        for _ in 0..(12 + i % 3) {
            if p.is_over() {
                break;
            }
            let legal: Vec<_> = p.legal_cards().into_iter().collect();
            p.play(legal[rng.next_u32() as usize % legal.len()]);
        }
        let mut meter = Meter::unlimited();
        solve(&p, &mut meter).unwrap();
        assert_eq!(meter.nodes(), [6546, 145, 208, 3786, 278, 0][i]);
    }
}
