// SPDX-License-Identifier: EUPL-1.2

//! Complete seeded series through the public application interface, with a
//! seeded random human. Checks series bookkeeping against the core results,
//! command rejection and determinism.

use std::collections::BTreeSet;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_app::{
    Action, AppError, AppEvent, Cancel, Command, GameType, KontraLevel, Level, Opponents, Phase,
    PlayerId, RuleError, SeriesLength, Session, SessionConfig,
};

fn config(seed: u64, series: SeriesLength, kontra_re: bool) -> SessionConfig {
    SessionConfig {
        seed,
        series,
        kontra_re,
        opponents: Opponents::Levels([Level::Easy; 2]),
    }
}

/// A random but reproducible human: picks a legal action with `rng`. Bids
/// are biased low so that games are not all overbid.
fn human_command(session: &Session, rng: &mut ChaCha8Rng) -> Command {
    let view = session.view();
    let legal = &view.legal_actions;
    if legal.is_empty() {
        return if view.can_end_series && rng.next_u32().is_multiple_of(8) {
            Command::EndSeries
        } else {
            Command::NextGame
        };
    }
    let mut i = rng.next_u32() as usize % legal.len();
    if let Action::Bid(_) = legal[i]
        && !rng.next_u32().is_multiple_of(3)
    {
        i = rng.next_u32() as usize % legal.len().min(4);
    }
    Command::Act(legal[i])
}

/// Plays until the series is over and returns all events.
fn play_series(session: &mut Session, rng: &mut ChaCha8Rng) -> Vec<AppEvent> {
    let mut events = Vec::new();
    for _ in 0..10_000 {
        events.extend(session.run_opponents());
        if session.view().series_finished {
            return events;
        }
        let command = human_command(session, rng);
        events.extend(session.execute(command).expect("legal human command"));
    }
    panic!("series did not finish");
}

#[test]
fn fixed_series_of_36_games() {
    let mut thrown_in = 0;
    for seed in 0..6 {
        let mut session = Session::new(config(seed, SeriesLength::Fixed(36), seed % 2 == 0));
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let events = play_series(&mut session, &mut rng);
        let view = session.view();
        assert!(view.series_finished);
        assert!(!view.can_next_game);
        assert_eq!(view.sheet.records.len(), 36);
        assert!(matches!(
            events.last(),
            Some(AppEvent::SeriesFinished { games: 36 })
        ));
        let mut totals = [0; 3];
        let mut won = [0; 3];
        let mut lost = [0; 3];
        for (i, record) in view.sheet.records.iter().enumerate() {
            assert_eq!(record.number as usize, i + 1);
            // Player 2 deals first, then the deal passes to the left (D-31),
            // also after a thrown-in deal.
            assert_eq!(record.dealer, PlayerId(((2 + i) % 3) as u8));
            match (record.result, record.declarer) {
                (Some(result), Some(declarer)) => {
                    let sf = result.seeger_fabian();
                    let seat = declarer.seat(record.dealer);
                    assert_eq!(record.points[declarer.index()], sf[seat.index()]);
                    assert_eq!(record.points.iter().sum::<i32>(), sf.iter().sum::<i32>());
                    assert_eq!(Some(result.outcome.contract), record.contract);
                    assert_eq!(result.outcome.bid, record.bid);
                    if result.won {
                        won[declarer.index()] += 1;
                        assert_eq!(record.points[declarer.index()], result.score + 50);
                    } else {
                        lost[declarer.index()] += 1;
                        assert_eq!(record.points[declarer.index()], result.score - 50);
                        for p in PlayerId::ALL.into_iter().filter(|&p| p != declarer) {
                            assert_eq!(record.points[p.index()], 40);
                        }
                    }
                }
                (None, None) => {
                    thrown_in += 1;
                    assert!(record.eingepasst());
                    assert_eq!(record.points, [0; 3]);
                }
                other => panic!("inconsistent record {other:?}"),
            }
            for (total, points) in totals.iter_mut().zip(record.points) {
                *total += points;
            }
            assert_eq!(record.totals, totals);
        }
        let result = view.series_result.expect("final result");
        assert_eq!(result.games, 36);
        assert_eq!(result.totals, totals);
        assert_eq!(result.won, won);
        assert_eq!(result.lost, lost);
        assert!(
            result
                .ranking
                .windows(2)
                .all(|w| totals[w[0].index()] >= totals[w[1].index()])
        );
        for p in PlayerId::ALL {
            assert_eq!(view.players[p.index()].total, totals[p.index()]);
        }
        assert_eq!(
            session.execute(Command::NextGame),
            Err(AppError::SeriesFinished)
        );
    }
    assert!(thrown_in > 0, "no thrown-in deal was covered");
}

#[test]
fn free_play_ends_on_request() {
    let mut session = Session::new(config(5, SeriesLength::Free, true));
    let mut rng = ChaCha8Rng::seed_from_u64(5);
    let events = play_series(&mut session, &mut rng);
    let view = session.view();
    assert!(view.series_finished);
    let games = view.sheet.records.len() as u32;
    assert!(games >= 1);
    assert_eq!(events.last(), Some(&AppEvent::SeriesFinished { games }));
    assert_eq!(view.series_result.unwrap().games, games);
}

#[test]
fn end_series_rules() {
    let mut fixed = Session::new(config(1, SeriesLength::Fixed(3), false));
    assert_eq!(
        fixed.execute(Command::EndSeries),
        Err(AppError::NotFreePlay)
    );
    let mut free = Session::new(config(1, SeriesLength::Free, false));
    assert_eq!(
        free.execute(Command::EndSeries),
        Err(AppError::GameInProgress)
    );
    let mut empty = Session::new(config(1, SeriesLength::Fixed(0), false));
    assert!(empty.view().series_finished);
    assert_eq!(
        empty.execute(Command::NextGame),
        Err(AppError::SeriesFinished)
    );
}

/// Every rejected command leaves the session unchanged.
fn assert_rejected(session: &mut Session, command: Command, expected: AppError) {
    let before = session.view();
    assert_eq!(session.execute(command), Err(expected));
    assert_eq!(session.view(), before, "view changed after {command:?}");
}

#[test]
fn illegal_commands_are_rejected_without_change() {
    let mut session = Session::new(config(11, SeriesLength::Fixed(2), true));
    // Human is Vorhand in game 1; Mittelhand bids first.
    let view = session.view();
    assert_eq!(view.acting, Some(PlayerId(1)));
    assert!(view.opponent_to_move);
    assert_rejected(
        &mut session,
        Command::Act(Action::Pass),
        AppError::NotYourTurn,
    );
    assert_rejected(&mut session, Command::NextGame, AppError::GameInProgress);
    session.execute(Command::AdvanceOpponent).unwrap();
    session.run_opponents();
    let view = session.view();
    if view.acting == Some(PlayerId::HUMAN) {
        assert_rejected(
            &mut session,
            Command::AdvanceOpponent,
            AppError::NoOpponentToMove,
        );
        let illegal = if view.legal_actions.contains(&Action::Hold) {
            Action::Bid(18)
        } else {
            Action::Hold
        };
        assert!(matches!(
            session.execute(Command::Act(illegal)),
            Err(AppError::Rule(RuleError::UnexpectedAction {
                phase: Phase::Reizen
            }))
        ));
        assert_rejected(
            &mut session,
            Command::Act(Action::Bid(19)),
            if view.legal_actions.contains(&Action::Hold) {
                AppError::Rule(RuleError::UnexpectedAction {
                    phase: Phase::Reizen,
                })
            } else {
                AppError::Rule(RuleError::InvalidBidValue { value: 19 })
            },
        );
    }
    // Play the game out and check rejections after it ended.
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    while session.view().acting.is_some() {
        session.run_opponents();
        if session.view().acting == Some(PlayerId::HUMAN) {
            let command = human_command(&session, &mut rng);
            session.execute(command).unwrap();
        }
    }
    assert_rejected(
        &mut session,
        Command::Act(Action::Pass),
        AppError::Rule(RuleError::GameOver),
    );
    assert_rejected(
        &mut session,
        Command::AdvanceOpponent,
        AppError::NoOpponentToMove,
    );
}

#[test]
fn rejected_discard_and_card() {
    // Find a game where the human becomes declarer and must discard.
    for seed in 0..200 {
        let mut session = Session::new(config(seed, SeriesLength::Fixed(1), false));
        session.run_opponents();
        while session.view().phase == Phase::Reizen {
            let view = session.view();
            let action = if view.legal_actions.contains(&Action::Hold) {
                Action::Hold
            } else if let Some(&bid) = view.legal_actions.first() {
                bid
            } else {
                break;
            };
            session.execute(Command::Act(action)).unwrap();
            session.run_opponents();
        }
        if session.view().declarer != Some(PlayerId::HUMAN) {
            continue;
        }
        session.execute(Command::Act(Action::TakeSkat)).unwrap();
        let view = session.view();
        assert_eq!(view.hand.len(), 12);
        let one = skat_app::CardSet::single(view.hand[0]);
        assert_rejected(
            &mut session,
            Command::Act(Action::Discard(one)),
            AppError::Rule(RuleError::InvalidDiscard),
        );
        let two = one | skat_app::CardSet::single(view.hand[1]);
        session.execute(Command::Act(Action::Discard(two))).unwrap();
        assert_eq!(session.view().own_skat.as_deref(), Some(&view.hand[..2]));
        session
            .execute(Command::Act(Action::Announce(
                skat_app::Announcement::plain(GameType::Grand),
            )))
            .unwrap();
        // Vorhand (the human) leads: a card not in hand is rejected.
        let missing = skat_app::Card::new(skat_app::Suit::Karo, skat_app::Rank::Sieben);
        let view = session.view();
        if view.acting == Some(PlayerId::HUMAN) && !view.hand.contains(&missing) {
            assert_rejected(
                &mut session,
                Command::Act(Action::Play(missing)),
                AppError::Rule(RuleError::CardNotInHand { card: missing }),
            );
        }
        return;
    }
    panic!("no seed made the human declarer");
}

#[test]
fn same_seed_and_commands_give_the_same_series() {
    let run = |seed: u64| {
        let mut session = Session::new(config(seed, SeriesLength::Fixed(12), true));
        let mut rng = ChaCha8Rng::seed_from_u64(99);
        let events = play_series(&mut session, &mut rng);
        (events, session.view())
    };
    let (events_a, view_a) = run(42);
    let (events_b, view_b) = run(42);
    assert_eq!(events_a, events_b);
    assert_eq!(view_a, view_b);
    let (events_c, _) = run(43);
    assert_ne!(events_a, events_c);
}

#[test]
fn every_variant_is_reachable_for_the_human() {
    let mut contracts = BTreeSet::new();
    let mut kontra_levels = BTreeSet::new();
    for seed in 0..40 {
        let mut session = Session::new(config(seed, SeriesLength::Fixed(36), true));
        let mut rng = ChaCha8Rng::seed_from_u64(seed + 1000);
        play_series(&mut session, &mut rng);
        for record in &session.view().sheet.records {
            let (Some(result), Some(PlayerId::HUMAN)) = (record.result, record.declarer) else {
                continue;
            };
            let c = result.outcome.contract;
            let family = match c.game {
                GameType::Farbe(_) => "Farbe",
                GameType::Grand => "Grand",
                GameType::Null => "Null",
            };
            contracts.insert((family, c.hand, c.schneider, c.schwarz, c.ouvert));
            kontra_levels.insert(format!("{:?}", result.outcome.kontra));
        }
    }
    for family in ["Farbe", "Grand"] {
        for wanted in [
            (family, false, false, false, false),
            (family, true, false, false, false),
            (family, true, true, false, false),
            (family, true, true, true, false),
            (family, true, true, true, true),
        ] {
            assert!(contracts.contains(&wanted), "missing {wanted:?}");
        }
    }
    for (hand, ouvert) in [(false, false), (true, false), (false, true), (true, true)] {
        assert!(
            contracts.contains(&("Null", hand, false, false, ouvert)),
            "missing Null hand={hand} ouvert={ouvert}"
        );
    }
    for level in [KontraLevel::None, KontraLevel::Kontra, KontraLevel::Re] {
        assert!(kontra_levels.contains(&format!("{level:?}")), "{level:?}");
    }
}

#[test]
fn api_types_round_trip_as_json() {
    let mut session = Session::new(config(8, SeriesLength::Fixed(2), true));
    let mut rng = ChaCha8Rng::seed_from_u64(8);
    let events = play_series(&mut session, &mut rng);
    let view = session.view();
    let json = serde_json::to_string(&view).unwrap();
    assert_eq!(
        serde_json::from_str::<skat_app::TableView>(&json).unwrap(),
        view
    );
    let json = serde_json::to_string(&events).unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<AppEvent>>(&json).unwrap(),
        events
    );
    let command = Command::Act(Action::Bid(18));
    let json = serde_json::to_string(&command).unwrap();
    assert_eq!(serde_json::from_str::<Command>(&json).unwrap(), command);
    let error = AppError::Rule(RuleError::InvalidDiscard);
    let json = serde_json::to_string(&error).unwrap();
    assert_eq!(serde_json::from_str::<AppError>(&json).unwrap(), error);
    let config = SessionConfig {
        opponents: Opponents::Levels([Level::Strong, Level::Easy]),
        ..config(3, SeriesLength::Free, true)
    };
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(
        serde_json::from_str::<SessionConfig>(&json).unwrap(),
        config
    );
}

#[test]
fn each_opponent_has_its_own_level() {
    let config = SessionConfig {
        opponents: Opponents::Levels([Level::Easy, Level::Strong]),
        ..config(4, SeriesLength::Fixed(1), false)
    };
    assert_eq!(config.level(PlayerId::HUMAN), None);
    assert_eq!(config.level(PlayerId(1)), Some(Level::Easy));
    assert_eq!(config.level(PlayerId(2)), Some(Level::Strong));
    let session = Session::new(config);
    assert_eq!(
        session.view().config.opponents,
        Opponents::Levels([Level::Easy, Level::Strong])
    );
    assert_eq!(session.goldfish(), None);
    // Human is Vorhand in game 1: player 1 (Mittelhand) bids first.
    let job = session.opponent_job().expect("opponent to move");
    assert_eq!(job.player(), PlayerId(1));
    assert_eq!(job.level(), Some(Level::Easy));
    assert!(!job.sees_all_cards());
    assert_eq!(
        SessionConfig::default().opponents,
        Opponents::Levels([Level::Medium; 2])
    );
}

/// Background moves: the same move as `AdvanceOpponent`; a move computed
/// for an earlier position is rejected without change.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn opponent_jobs_match_advance_and_reject_stale_moves() {
    for seed in 0..3 {
        let config = SessionConfig {
            opponents: Opponents::Levels([Level::Medium, Level::Easy]),
            ..config(seed, SeriesLength::Fixed(2), true)
        };
        let mut a = Session::new(config);
        let mut b = Session::new(config);
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        while !a.view().series_finished {
            if let Some(job) = a.opponent_job() {
                let mv = job.run(None);
                let thread = std::thread::spawn(move || job.run(Some(&Cancel::new())));
                assert_eq!(thread.join().unwrap(), mv);
                let events = a.finish_opponent(mv).unwrap();
                assert_eq!(b.execute(Command::AdvanceOpponent).unwrap(), events);
                let before = a.view();
                assert!(matches!(
                    a.finish_opponent(mv),
                    Err(AppError::StaleOpponentMove
                        | AppError::NoOpponentToMove
                        | AppError::SeriesFinished)
                ));
                assert_eq!(a.view(), before);
                continue;
            }
            let command = human_command(&a, &mut rng);
            assert_eq!(a.execute(command), b.execute(command));
        }
        assert_eq!(a.view(), b.view());
    }
}

/// A cancelled opponent still moves legally (rules-of-thumb fallback).
#[test]
fn cancelled_opponents_still_play_legally() {
    let config = SessionConfig {
        opponents: Opponents::Levels([Level::Strong, Level::Strong]),
        ..config(6, SeriesLength::Fixed(2), true)
    };
    let mut session = Session::new(config);
    let mut rng = ChaCha8Rng::seed_from_u64(6);
    let cancel = Cancel::new();
    cancel.cancel();
    while !session.view().series_finished {
        if let Some(job) = session.opponent_job() {
            session.finish_opponent(job.run(Some(&cancel))).unwrap();
        } else {
            let command = human_command(&session, &mut rng);
            session.execute(command).unwrap();
        }
    }
    assert_eq!(session.view().sheet.records.len(), 2);
}

/// The human's Kontra decision before an opponent's Ouvert game is reached
/// and answered through the application interface.
#[test]
fn human_decides_kontra_before_an_opponents_ouvert_game() {
    for seed in 0..400 {
        let mut session = Session::new(config(seed, SeriesLength::Fixed(36), true));
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        for _ in 0..10_000 {
            session.run_opponents();
            let view = session.view();
            if view.series_finished {
                break;
            }
            if view.legal_actions.contains(&Action::KeinKontra) {
                assert!(view.contract.is_some_and(|c| c.ouvert));
                assert_ne!(view.declarer, Some(PlayerId::HUMAN));
                assert!(view.declarer_cards.is_none(), "revealed before Kontra");
                session.execute(Command::Act(Action::KeinKontra)).unwrap();
                session.run_opponents();
                assert!(session.view().declarer_cards.is_some());
                return;
            }
            // Bid or hold 18 so the human may say Kontra (D-26), then pass.
            let command = if view.phase == Phase::Reizen && view.acting == Some(PlayerId::HUMAN) {
                let legal = &view.legal_actions;
                let action = if legal.contains(&Action::Hold) && view.bid <= 18 {
                    Action::Hold
                } else if legal.contains(&Action::Bid(18)) {
                    Action::Bid(18)
                } else {
                    Action::Pass
                };
                Command::Act(action)
            } else {
                human_command(&session, &mut rng)
            };
            session.execute(command).unwrap();
        }
    }
    panic!("no Ouvert Kontra round for the human found");
}

/// Goldfish mode: one opponent per series is the Goldfish, drawn from the
/// opponents' seed stream; nobody sees foreign cards.
#[test]
fn goldfish_mode_draws_one_forgetful_opponent() {
    let mut drawn = BTreeSet::new();
    for seed in 0..12 {
        let config = SessionConfig {
            opponents: Opponents::Goldfish,
            ..config(seed, SeriesLength::Fixed(1), true)
        };
        let session = Session::new(config);
        let goldfish = session.goldfish().expect("Goldfish mode");
        assert!(goldfish == PlayerId(1) || goldfish == PlayerId(2));
        assert_eq!(Session::new(config).goldfish(), Some(goldfish));
        assert_eq!(config.level(PlayerId(1)), None);
        drawn.insert(goldfish);
    }
    assert_eq!(drawn.len(), 2);
    let config = SessionConfig {
        opponents: Opponents::Goldfish,
        ..config(2, SeriesLength::Fixed(1), true)
    };
    let mut session = Session::new(config);
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    while !session.view().series_finished {
        if let Some(job) = session.opponent_job() {
            assert!(!job.sees_all_cards());
            assert_eq!(job.level(), None);
            session.finish_opponent(job.run(None)).unwrap();
        } else {
            let command = human_command(&session, &mut rng);
            session.execute(command).unwrap();
        }
    }
    assert_eq!(session.view().sheet.records.len(), 1);
}

/// Mephisto and God Mode: every computer decision before card play sees
/// all cards; in card play the computer declarer does and its partner
/// does not, and against the human declarer both defenders do. Seed 2 has
/// a computer declarer, seed 21 the human; that Mephisto never loses is
/// checked broadly in `skat-ai/tests/variants.rs` (deep run). Only God
/// Mode shows the human every card.
#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn mephisto_mode_sees_all_cards_where_configured() {
    for opponents in [Opponents::Mephisto, Opponents::GodMode] {
        mephisto_sees_all_cards_where_configured(opponents);
    }
}

fn mephisto_sees_all_cards_where_configured(opponents: Opponents) {
    let mut cases = BTreeSet::new();
    for seed in [2, 21] {
        let config = SessionConfig {
            opponents,
            ..config(seed, SeriesLength::Fixed(1), true)
        };
        let mut session = Session::new(config);
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        while !session.view().series_finished {
            let Some(job) = session.opponent_job() else {
                let command = human_command(&session, &mut rng);
                session.execute(command).unwrap();
                continue;
            };
            let view = session.view();
            let open = view.open_cards.as_ref();
            assert_eq!(open.is_some(), opponents.god_mode(), "seed {seed}");
            if let Some(open) = open {
                for p in PlayerId::ALL {
                    assert_eq!(
                        open.hands[p.index()].len(),
                        usize::from(view.players[p.index()].cards)
                    );
                }
            }
            let expected = match (view.phase, view.declarer) {
                (Phase::Spiel, Some(PlayerId::HUMAN)) => {
                    cases.insert("against the human");
                    true
                }
                (Phase::Spiel, Some(declarer)) if declarer == job.player() => {
                    cases.insert("declarer");
                    true
                }
                (Phase::Spiel, _) => {
                    cases.insert("partner");
                    false
                }
                _ => true,
            };
            assert_eq!(job.sees_all_cards(), expected, "seed {seed}");
            session.finish_opponent(job.run(None)).unwrap();
        }
        let record = session.view().sheet.records[0];
        if let (Some(declarer), Some(result)) = (record.declarer, record.result)
            && !declarer.is_human()
        {
            assert!(result.won, "seed {seed}: Mephisto lost {record:?}");
        }
    }
    assert_eq!(cases.len(), 3, "{cases:?}");
}
