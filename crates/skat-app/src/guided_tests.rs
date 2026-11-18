// SPDX-License-Identifier: EUPL-1.2

use super::*;
use crate::{GuidedAction as G, GuidedAnswer as A, GuidedStep as S};
use skat_core::{Card, KontraLevel};
#[path = "../../skat-coach/tests/support/guided.rs"]
mod fixture;

fn session(declarer: bool) -> Session {
    session_with(declarer, fixture::deal())
}

fn session_with(declarer: bool, deal: skat_core::Deal) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        series: SeriesLength::Free,
        ..SessionConfig::default()
    });
    s.dealer = PlayerId(if declarer { 0 } else { 1 });
    s.game = Game::from_deal(deal, RuleConfig::default());
    s.recording.deal = deal;
    while s.game.phase() != Phase::Spiel {
        let a = match s.game.phase() {
            Phase::Reizen if s.game.current_seat() == Some(Seat::Hinterhand) => Action::Bid(18),
            Phase::Reizen => Action::Pass,
            Phase::SkatAufnahme => Action::Hand,
            Phase::Ansage => Action::Announce(skat_core::Announcement::plain(
                skat_core::GameType::Farbe(skat_core::Suit::Herz),
            )),
            _ => panic!("phase"),
        };
        s.apply(a).unwrap();
    }
    s.apply(Action::Play(fixture::card(if declarer {
        "C9"
    } else {
        "CK"
    })))
    .unwrap();
    if declarer {
        s.apply(Action::Play(fixture::card("C8"))).unwrap();
    }
    s
}

fn respond(s: &mut Session, action: G) -> Result<Vec<AppEvent>, AppError> {
    s.execute(Command::Guided(s.guided().unwrap().id, action))
}

#[test]
fn guided_wrong_retry_reveal_memory_and_every_legal_action_never_autoplay() {
    for declarer in [false, true] {
        let s = session(declarer);
        for action in s.view().legal_actions {
            let mut s = s.clone();
            let before = s.view();
            assert!(s.guided().is_none());
            s.execute(Command::StartGuided).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Orient);
            assert!(g.rule.is_none() && g.memory.is_none() && g.figures.is_none());
            assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
            respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            assert_eq!(s.guided().unwrap().step, S::Orient);
            respond(&mut s, G::Retry).unwrap();
            assert_eq!(s.guided().unwrap().correct, None);
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Act);
            assert_eq!(
                s.guided().unwrap().rule.unwrap().card,
                fixture::card(if declarer { "CQ" } else { "C7" })
            );
            respond(&mut s, G::Memory(true)).unwrap();
            assert!(s.guided().unwrap().memory.is_some());
            assert_eq!(s.view().hand, before.hand);
            assert_eq!(s.view().current_trick, before.current_trick);
            assert_eq!(s.view().legal_actions, before.legal_actions);
            // Including the heuristic-excluded CT: no automatic choice or forced move.
            s.execute(Command::Act(action)).unwrap();
            if !declarer {
                s.run_opponents();
            }
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Observe);
            assert!(g.memory.is_none() && g.observation.is_none());
            respond(&mut s, G::Reveal).unwrap();
            let o = s.guided().unwrap().observation.unwrap();
            assert!(o.winner.is_some());
            if let skat_coach::Source::Trick {
                index,
                seat,
                card,
                led,
            } = o.source
            {
                assert_eq!(index, 0);
                assert_eq!(led, fixture::card(if declarer { "C9" } else { "CK" }));
                assert_eq!(seat, Seat::Hinterhand);
                let actual = s.game.view(Seat::Hinterhand).tricks[0];
                assert_eq!(actual.cards[2], card);
                assert_eq!(o.winner, Some(actual.winner));
            } else {
                panic!("public trick source");
            }
            respond(&mut s, G::Return).unwrap();
            assert!(s.guided().is_none());
        }
    }
}

#[test]
fn correct_answers_narrow_without_play_and_observe_real_automated_card() {
    let mut s = session(false);
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Check);
    respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Narrow);
    respond(&mut s, G::Answer(A::Card(fixture::card("CT")))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    respond(&mut s, G::Answer(A::Card(fixture::card("C8")))).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Act);
    assert!(s.view().hand.contains(&fixture::card("C8")));
    s.execute(Command::Act(Action::Play(fixture::card("CT"))))
        .unwrap();
    assert_eq!(s.guided().unwrap().step, S::Waiting);
    let id = s.guided().unwrap().id;
    let mv = s.opponent_job().unwrap().run(None);
    s.finish_opponent(mv).unwrap();
    assert_eq!(s.guided().unwrap().id, id);
    assert_eq!(s.guided().unwrap().step, S::Observe);
    respond(&mut s, G::Answer(A::Void(true))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    respond(&mut s, G::Answer(A::Void(false))).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Complete);
    assert!(!s.guided().unwrap().observation.unwrap().proves_void);
}

#[test]
fn stale_reentry_session_game_and_free_play_boundaries_reject_old_responses() {
    let mut s = session(true);
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    s.execute(Command::StartGuided).unwrap();
    assert_ne!(old, s.guided().unwrap().id);
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    let mut other = session(true);
    other.execute(Command::StartGuided).unwrap();
    assert_eq!(
        other.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    s.execute(Command::Act(Action::Play(fixture::card("CA"))))
        .unwrap();
    // Next human move closes the frozen observation, rather than reusing it.
    while s.opponent_to_move() {
        s.execute(Command::AdvanceOpponent).unwrap();
    }
    let next = s.view().legal_actions[0];
    s.execute(Command::Act(next)).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
}

#[test]
fn off_default_automatic_quiz_and_skip_protections_do_not_start_guidance() {
    for level in [
        crate::CoachLevel::OnRequest,
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::Off,
    ] {
        let mut s = session(false);
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        if level == crate::CoachLevel::Off {
            assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
        } else {
            let before = s.view();
            s.execute(Command::StartGuided).unwrap();
            respond(&mut s, G::Return).unwrap();
            assert_eq!(s.view(), before);
            s.execute(Command::StartGuided).unwrap();
            s.execute(Command::SetCoach(CoachSettings {
                level: crate::CoachLevel::Off,
                ..CoachSettings::default()
            }))
            .unwrap();
            assert!(s.guided().is_none());
        }
    }
}

#[test]
fn estimates_require_figures_and_request_and_keep_the_original_rule() {
    let mut s = session(true);
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let rule = s.guided().unwrap().rule;
    assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
    let job = s.coach_job().unwrap();
    let result = job
        .run(None)
        .guided_rank_first(Action::Play(fixture::card("CA")));
    assert_eq!(
        result.explanation().unwrap().options[0].action,
        Action::Play(fixture::card("CA"))
    );
    // Ranking intentionally disagrees; numerical values remain the real
    // incomplete analysis values, never product fixture estimates.
    s.finish_coach(result).unwrap();
    s.execute(Command::Act(Action::Play(fixture::card("CQ"))))
        .unwrap();
    respond(&mut s, G::Reveal).unwrap();
    assert!(s.guided().unwrap().figures.is_none());
    s.execute(Command::SetCoach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    }))
    .unwrap();
    assert!(s.guided().unwrap().figures.is_none());
    respond(&mut s, G::Figures).unwrap();
    assert!(s.guided().unwrap().figures_requested);
    assert_eq!(s.guided().unwrap().rule, rule);
    if let Some(h) = s.guided().unwrap().figures {
        assert_ne!(h.status, skat_coach::Status::Complete);
    }
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    assert!(s.guided().unwrap().figures.is_none());
}

#[test]
fn hidden_substitutions_keep_dialogue_until_actual_public_play_differs() {
    for declarer in [false, true] {
        let mut a = session(declarer);
        let d = fixture::swap(
            fixture::deal(),
            fixture::card(if declarer { "H7" } else { "CA" }),
            fixture::card("DK"),
        );
        let mut b = session_with(declarer, d);
        // Both fixture positions also replay through an independent core constructor.
        assert_eq!(
            a.game.view(if declarer {
                Seat::Hinterhand
            } else {
                Seat::Mittelhand
            }),
            fixture::position(fixture::deal(), declarer).view(if declarer {
                Seat::Hinterhand
            } else {
                Seat::Mittelhand
            })
        );
        for s in [&mut a, &mut b] {
            s.execute(Command::StartGuided).unwrap();
        }
        assert_eq!(a.guided().unwrap().step, b.guided().unwrap().step);
        for s in [&mut a, &mut b] {
            respond(s, G::Reveal).unwrap();
        }
        assert_eq!(a.guided().unwrap().rule, b.guided().unwrap().rule);
        for s in [&mut a, &mut b] {
            respond(s, G::Memory(true)).unwrap();
        }
        assert_eq!(a.guided().unwrap().memory, b.guided().unwrap().memory);
        for s in [&mut a, &mut b] {
            s.execute(Command::Act(Action::Play(fixture::card(if declarer {
                "CQ"
            } else {
                "C7"
            }))))
            .unwrap();
            s.run_opponents();
            assert_eq!(s.guided().unwrap().step, S::Observe);
            respond(s, G::Reveal).unwrap();
        }
        let oa = a.guided().unwrap().observation.unwrap();
        let ob = b.guided().unwrap().observation.unwrap();
        if declarer {
            // The same own completing card is public in both hidden deals.
            assert_eq!(oa, ob);
        } else {
            // CA in HH wins; with CA in the unknown Skat, HH follows CQ
            // and the partner's CK actually wins. These are later facts,
            // not premises allowed in the original dialogue.
            assert_ne!(oa, ob);
            assert_eq!(
                oa.source,
                skat_coach::Source::Trick {
                    index: 0,
                    seat: Seat::Hinterhand,
                    led: fixture::card("CK"),
                    card: fixture::card("CA")
                }
            );
            assert_eq!(
                ob.source,
                skat_coach::Source::Trick {
                    index: 0,
                    seat: Seat::Hinterhand,
                    led: fixture::card("CK"),
                    card: fixture::card("CQ")
                }
            );
            assert_eq!(oa.winner, Some(Seat::Hinterhand));
            assert_eq!(ob.winner, Some(Seat::Vorhand));
            assert!(!oa.proves_void && !ob.proves_void);
        }
    }
}

#[test]
fn guidance_events_keep_entry_identity_sequence_and_assistance() {
    let mut s = session(false);
    let start = s.execute(Command::StartGuided).unwrap();
    let AppEvent::Guided(start) = start[0] else {
        panic!("entry event");
    };
    assert_eq!(start.sequence, 0);
    assert!(!start.assisted);
    let reveal = respond(&mut s, G::Reveal).unwrap();
    let AppEvent::Guided(reveal) = reveal[0] else {
        panic!("reveal event");
    };
    assert_eq!(reveal.id, start.id);
    assert_eq!(reveal.sequence, 1);
    assert!(reveal.assisted);
    let played = s
        .execute(Command::Act(Action::Play(fixture::card("C7"))))
        .unwrap();
    let waiting = played
        .iter()
        .find_map(|e| {
            if let AppEvent::Guided(g) = e {
                Some(g)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(waiting.step, S::Waiting);
    let observed = s.run_opponents();
    let observation = observed
        .iter()
        .find_map(|e| {
            if let AppEvent::Guided(g) = e {
                Some(g)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(observation.id, start.id);
    assert_eq!(observation.step, S::Observe);
    assert!(observation.assisted);
    assert!(observation.sequence > waiting.sequence);
}

#[test]
fn unsupported_and_single_choice_have_no_rule_and_final_series_allows_observation_exit() {
    // The first ordinary decision is Reizen, now a supported threshold dialogue.
    let mut s = Session::new(SessionConfig::default());
    s.run_opponents();
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Orient);
    assert!(s.guided().unwrap().reizen.is_some());
    assert!(s.guided().unwrap().rule.is_none());
    respond(&mut s, G::Return).unwrap();
    let mut s = session(true);
    while s.game.view(Seat::Hinterhand).hand.len() > 1 {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    while s.game.current_seat() != Some(Seat::Hinterhand) {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    assert_eq!(s.view().legal_actions.len(), 1);
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Unsupported);
    // A final-series boundary must still allow putting a visible dialogue away.
    s.finished = true;
    respond(&mut s, G::Return).unwrap();
    assert!(s.guided().is_none());
}

/// Human on `seat` at a Reizen decision after core-accepted `calls`.
fn reizen(seat: Seat, calls: &[Action]) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        series: SeriesLength::Free,
        ..SessionConfig::default()
    });
    s.dealer = PlayerId(match seat {
        Seat::Hinterhand => 0,
        Seat::Mittelhand => 1,
        Seat::Vorhand => 2,
    });
    s.game = Game::from_deal(fixture::deal(), RuleConfig::default());
    s.recording.deal = fixture::deal();
    for &a in calls {
        s.apply(a).unwrap();
    }
    assert_eq!(
        s.human_decision().map(|_| s.game.current_seat()),
        Some(Some(seat))
    );
    s
}

#[test]
fn reizen_wrong_retry_reveal_and_every_legal_action_never_autoplay() {
    // MH asks at 0 (rule bids 18 within 22); VH holds above its ceiling 20.
    for (seat, calls, holds, stake, action) in [
        (Seat::Mittelhand, vec![], false, Some(18), Action::Bid(18)),
        (
            Seat::Vorhand,
            vec![Action::Bid(22)],
            true,
            Some(22),
            Action::Pass,
        ),
    ] {
        let s = reizen(seat, &calls);
        let legal = s.view().legal_actions;
        // First calls (incl. jumps) and Pass; all are free alternatives.
        let mut choices: Vec<_> = legal.iter().copied().take(3).collect();
        choices.push(*legal.last().unwrap());
        for chosen in choices {
            let mut s = s.clone();
            let before = s.view();
            s.execute(Command::StartGuided).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Orient);
            // Initial prompt: only the public bid, no stake, role or rule.
            let r = g.reizen.unwrap();
            assert_eq!(r.current, before.bid);
            assert_eq!((r.holds, r.stake), (None, None));
            assert!(g.bid_rule.is_none() && g.rule.is_none() && g.legal_cards.is_empty());
            assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
            respond(&mut s, G::Answer(A::Holds(!holds))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            assert_eq!(s.guided().unwrap().step, S::Orient);
            respond(&mut s, G::Retry).unwrap();
            respond(&mut s, G::Answer(A::Holds(holds))).unwrap();
            let g = s.guided().unwrap();
            assert_eq!((g.step, g.correct), (S::Threshold, None));
            assert_eq!(g.reizen.unwrap().holds, Some(holds));
            assert_eq!(g.reizen.unwrap().stake, stake);
            assert!(g.bid_rule.is_none());
            respond(&mut s, G::Answer(A::Within(action == Action::Pass))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            assert!(s.guided().unwrap().bid_rule.is_none());
            respond(&mut s, G::Reveal).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Act);
            let rule = g.bid_rule.unwrap();
            assert_eq!(rule.action, action);
            assert_eq!(rule.current, before.bid);
            assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
            assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
            assert_eq!(s.view().legal_actions, before.legal_actions);
            assert_eq!(s.view().hand, before.hand);
            let id = g.id;
            // Any legal call, including ones the rule would not make, ends the dialogue.
            s.execute(Command::Act(chosen)).unwrap();
            assert!(s.guided().is_none());
            assert_eq!(
                s.execute(Command::Guided(id, G::Reveal)),
                Err(AppError::StaleGuided)
            );
        }
    }
}

#[test]
fn reizen_correct_attempt_reveals_rule_and_jump_context_stays_public() {
    // HH after MH jumps to 35: next standard value 36 is exactly its ceiling.
    let mut s = reizen(Seat::Hinterhand, &[Action::Bid(35), Action::Pass]);
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(s.guided().unwrap().reizen.unwrap().current, 35);
    respond(&mut s, G::Answer(A::Holds(false))).unwrap();
    assert_eq!(s.guided().unwrap().reizen.unwrap().stake, Some(36));
    respond(&mut s, G::Answer(A::Within(true))).unwrap();
    let g = s.guided().unwrap();
    assert_eq!((g.step, g.correct), (S::Act, Some(true)));
    let rule = g.bid_rule.unwrap();
    assert_eq!((rule.action, rule.ceiling), (Action::Bid(36), 36));
    assert_eq!(rule.branch, crate::GuidedGameBranch::Suit);
    // Deliberate reveal straight from orientation also shows the stake.
    let mut s = reizen(Seat::Hinterhand, &[Action::Bid(35), Action::Pass]);
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.reizen.unwrap().stake, Some(36));
    assert_eq!(g.bid_rule.unwrap().action, Action::Bid(36));
}

#[test]
fn reizen_skip_stale_off_and_automatic_boundaries() {
    let mut s = reizen(Seat::Mittelhand, &[]);
    let before = s.view();
    for level in [
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::OnRequest,
    ] {
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        // Automatic coach levels never enter or expose the dialogue.
        assert!(s.guided().is_none());
    }
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    respond(&mut s, G::Return).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.view(), before);
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    s.execute(Command::StartGuided).unwrap();
    assert_ne!(s.guided().unwrap().id, old);
    s.execute(Command::SetCoach(CoachSettings {
        level: crate::CoachLevel::Off,
        ..CoachSettings::default()
    }))
    .unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
    // A new game drops the dialogue as well.
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    s.execute(Command::StartGuided).unwrap();
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
}

/// Human on Hinterhand wins the bidding at 18 with `hh`, then takes `after`.
fn preplay(hh: &str, skat: &str, after: &[Action]) -> Session {
    preplay_at(18, hh, skat, after)
}

/// Human on Hinterhand wins the bidding at `bid` with `hh`, then takes `after`.
fn preplay_at(bid: u16, hh: &str, skat: &str, after: &[Action]) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        series: SeriesLength::Free,
        ..SessionConfig::default()
    });
    s.dealer = PlayerId(0);
    let deal = fixture::deal_hh(hh, skat);
    s.game = Game::from_deal(deal, RuleConfig::default());
    s.recording.deal = deal;
    for &a in [Action::Pass, Action::Bid(bid), Action::Pass]
        .iter()
        .chain(after)
    {
        s.apply(a).unwrap();
    }
    assert_eq!(s.game.current_seat(), Some(Seat::Hinterhand));
    assert!(s.human_decision().is_some());
    s
}

/// Herz Hand from the own ten cards: strength 106 (skat-coach fixtures).
const SUIT_HAND: &str = "CJ HA HT HK HQ H9 H8 H7 CA SA";

#[test]
fn pickup_wrong_retry_reveal_and_both_legal_actions_never_autoplay() {
    let s = preplay(SUIT_HAND, "DQ DK", &[]);
    assert_eq!(s.game.phase(), Phase::SkatAufnahme);
    for chosen in [Action::Hand, Action::TakeSkat] {
        let mut s = s.clone();
        let before = s.view();
        s.execute(Command::StartGuided).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Orient);
        // Initial prompt: phase only; no game, threshold, strength or rule.
        let p = g.preplay.clone().unwrap();
        assert_eq!(p.phase, Phase::SkatAufnahme);
        assert_eq!((p.game, p.threshold), (None, None));
        assert!(p.cards.is_empty());
        assert!(g.pre_rule.is_none() && g.bid_rule.is_none() && g.reizen.is_none());
        respond(&mut s, G::Answer(A::Game(crate::GuidedGameBranch::Grand))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        assert_eq!(s.guided().unwrap().preplay.unwrap().game, None);
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Game(crate::GuidedGameBranch::Suit))).unwrap();
        let g = s.guided().unwrap();
        assert_eq!((g.step, g.correct), (S::Threshold, None));
        let p = g.preplay.unwrap();
        let herz = skat_core::GameType::Farbe(skat_core::Suit::Herz);
        assert_eq!((p.game, p.threshold), (Some(herz), Some(100)));
        assert!(g.pre_rule.is_none());
        respond(&mut s, G::Answer(A::Within(false))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        assert!(s.guided().unwrap().pre_rule.is_none());
        respond(&mut s, G::Reveal).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Act);
        let crate::GuidedPreRule::Pickup(rule) = g.pre_rule.unwrap() else {
            panic!("pickup rule");
        };
        assert_eq!(
            (rule.action, rule.strength, rule.fallback),
            (Action::Hand, 106, false)
        );
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        assert_eq!(s.view().legal_actions, before.legal_actions);
        assert_eq!(s.view().hand, before.hand);
        let id = g.id;
        // Either action, also the one the rule would not take, ends the dialogue.
        s.execute(Command::Act(chosen)).unwrap();
        assert!(s.guided().is_none());
        assert_eq!(s.view().hand_game, Some(chosen == Action::Hand));
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
}

#[test]
fn discard_correct_card_reveals_rule_any_legal_discard_stays_free() {
    let mut s = preplay(
        "CJ HA HT HK HQ H9 H8 CA CT S7",
        "ST D8",
        &[Action::TakeSkat],
    );
    assert_eq!(s.game.phase(), Phase::Druecken);
    let before = s.view();
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    let p = g.preplay.unwrap();
    assert_eq!(p.phase, Phase::Druecken);
    assert_eq!(p.cards.len(), 12);
    assert_eq!((p.game, p.threshold), (None, None));
    respond(&mut s, G::Answer(A::Game(crate::GuidedGameBranch::Suit))).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Threshold);
    // A kept Trumpf is wrong; the singleton D8 is one of Easy's two cards.
    respond(&mut s, G::Answer(A::Card(fixture::card("CJ")))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    assert!(s.guided().unwrap().pre_rule.is_none());
    respond(&mut s, G::Retry).unwrap();
    respond(&mut s, G::Answer(A::Card(fixture::card("D8")))).unwrap();
    let g = s.guided().unwrap();
    assert_eq!((g.step, g.correct), (S::Act, Some(true)));
    let crate::GuidedPreRule::Discard(rule) = g.pre_rule.unwrap() else {
        panic!("discard rule");
    };
    assert_eq!(rule.action, Action::Discard(fixture::cards("D8 ST")));
    assert_eq!(s.view().legal_actions, before.legal_actions);
    // Put away two trumps instead: legal, applied, and the dialogue ends.
    let mine = Action::Discard(fixture::cards("CJ HA"));
    s.execute(Command::Act(mine)).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.game.phase(), Phase::Ansage);
    assert!(!s.view().hand.contains(&fixture::card("CJ")));
}

fn announce_rule(g: &crate::GuidedView) -> crate::GuidedAnnounceRule {
    match g.pre_rule {
        Some(crate::GuidedPreRule::Announce(r)) => r,
        _ => panic!("announce rule expected"),
    }
}

const HERZ: skat_core::GameType = skat_core::GameType::Farbe(skat_core::Suit::Herz);
const PIK: skat_core::GameType = skat_core::GameType::Farbe(skat_core::Suit::Pik);

#[test]
fn announce_wrong_retry_reveal_and_every_legal_announcement_never_autoplays() {
    // Bid 33 above Herz Hand 30: Easy announces Pik (tie with Kreuz at 61).
    let s = preplay_at(33, SUIT_HAND, "DQ DK", &[Action::Hand]);
    assert_eq!(s.game.phase(), Phase::Ansage);
    let mut s = s.clone();
    let before = s.view();
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.step, S::Orient);
    // Initial prompt: public bid and Hand state only; no game, value or rule.
    let p = g.preplay.clone().unwrap();
    assert_eq!(p.phase, Phase::Ansage);
    assert_eq!((p.bid, p.hand_game), (Some(33), Some(true)));
    assert_eq!((p.game, p.threshold), (None, None));
    assert!(p.cards.is_empty());
    assert!(g.pre_rule.is_none() && g.bid_rule.is_none() && g.rule.is_none());
    respond(&mut s, G::Answer(A::Game(crate::GuidedGameBranch::Null))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    assert_eq!(s.guided().unwrap().preplay.unwrap().game, None);
    respond(&mut s, G::Retry).unwrap();
    respond(&mut s, G::Answer(A::Game(crate::GuidedGameBranch::Suit))).unwrap();
    let g = s.guided().unwrap();
    assert_eq!((g.step, g.correct), (S::Threshold, None));
    assert_eq!(g.preplay.unwrap().game, Some(HERZ));
    assert!(g.pre_rule.is_none());
    // Herz Hand is worth 30 < 33: "reaches" is wrong and reveals nothing.
    respond(&mut s, G::Answer(A::Within(true))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    assert!(s.guided().unwrap().pre_rule.is_none());
    respond(&mut s, G::Reveal).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.step, S::Act);
    let rule = announce_rule(&g);
    assert_eq!(
        (rule.action, rule.value, rule.fits, rule.fallback),
        (
            Action::Announce(skat_core::Announcement::plain(PIK)),
            30,
            false,
            false
        )
    );
    assert_eq!(rule.tied().len(), 2);
    assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
    assert_eq!(s.view().legal_actions, before.legal_actions);
    assert_eq!(s.view().hand, before.hand);
    let id = g.id;
    // Every legal announcement, also those the rule would not take, is applied
    // as chosen and ends the dialogue.
    assert!(before.legal_actions.len() > 5);
    for &chosen in &before.legal_actions {
        let mut s = s.clone();
        s.execute(Command::Act(chosen)).unwrap();
        assert!(s.guided().is_none());
        let Action::Announce(a) = chosen else {
            panic!("announcement expected");
        };
        assert_eq!(s.game.contract().unwrap().game, a.game);
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
}

#[test]
fn announce_correct_attempt_skip_reenter_off_and_next_game_boundaries() {
    let mut s = preplay(SUIT_HAND, "DQ DK", &[Action::Hand]);
    let before = s.view();
    for level in [
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::OnRequest,
    ] {
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        // Automatic coach levels never enter or expose the dialogue.
        assert!(s.guided().is_none());
    }
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    respond(&mut s, G::Return).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.view(), before);
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    s.execute(Command::StartGuided).unwrap();
    assert_ne!(s.guided().unwrap().id, old);
    respond(&mut s, G::Answer(A::Game(crate::GuidedGameBranch::Suit))).unwrap();
    // Herz Hand 30 reaches 18: a correct attempt reveals the rule.
    respond(&mut s, G::Answer(A::Within(true))).unwrap();
    let g = s.guided().unwrap();
    assert_eq!((g.step, g.correct), (S::Act, Some(true)));
    let rule = announce_rule(&g);
    assert_eq!(
        rule.action,
        Action::Announce(skat_core::Announcement::plain(HERZ))
    );
    assert!(rule.fits && rule.skat.is_empty() && rule.tied().is_empty());
    s.execute(Command::SetCoach(CoachSettings {
        level: crate::CoachLevel::Off,
        ..CoachSettings::default()
    }))
    .unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    s.execute(Command::StartGuided).unwrap();
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
}

#[test]
fn announce_views_depend_only_on_the_permitted_view() {
    // Hand: different unknown Skat. Pickup: the known own Skat is shown.
    let walk = |s: &mut Session| {
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        respond(s, G::Answer(A::Game(crate::GuidedGameBranch::Suit))).unwrap();
        views.push(s.guided().unwrap());
        respond(s, G::Reveal).unwrap();
        views.push(s.guided().unwrap());
        views
    };
    let mut a = walk(&mut preplay_at(33, SUIT_HAND, "DQ DK", &[Action::Hand]));
    let b = walk(&mut preplay_at(33, SUIT_HAND, "CK SK", &[Action::Hand]));
    for (a, b) in a.iter_mut().zip(&b) {
        a.id = b.id;
        assert_eq!(a, b);
    }
    let mut s = preplay(
        "CJ SJ HJ DJ HA HT HK HQ H9 CA",
        "S7 D7",
        &[Action::TakeSkat, Action::Discard(fixture::cards("S7 D7"))],
    );
    s.execute(Command::StartGuided).unwrap();
    let p = s.guided().unwrap().preplay.unwrap();
    assert_eq!((p.bid, p.hand_game, p.game), (Some(18), Some(false), None));
    respond(&mut s, G::Reveal).unwrap();
    let rule = announce_rule(&s.guided().unwrap());
    assert_eq!(rule.skat, fixture::cards("S7 D7"));
    assert_eq!(
        rule.action,
        Action::Announce(skat_core::Announcement::plain(skat_core::GameType::Grand))
    );
}

/// Core-checked Kontra/Re position (see `skat-coach/tests/guided_kontra.rs`):
/// MH bids 18, VH holds, MH passes, HH bids 20, VH passes, HH plays `game`
/// Hand; the human sits on `human`. Kontra/Re is switched on.
fn kontra_session(human: Seat, deal: skat_core::Deal, game: skat_core::Announcement) -> Session {
    kontra_session_after(human, deal, &[Action::Hand], game)
}

/// As `kontra_session`, with `after` (Hand, or pickup and discard) instead of Hand.
fn kontra_session_after(
    human: Seat,
    deal: skat_core::Deal,
    after: &[Action],
    game: skat_core::Announcement,
) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        series: SeriesLength::Free,
        kontra_re: true,
        ..SessionConfig::default()
    });
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == human)
        .unwrap();
    s.game = Game::from_deal(deal, RuleConfig { kontra_re: true });
    s.recording.deal = deal;
    for a in [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Bid(20),
        Action::Pass,
    ] {
        s.apply(a).unwrap();
    }
    for &a in after {
        s.apply(a).unwrap();
    }
    s.apply(Action::Announce(game)).unwrap();
    s
}

/// VH holds `vh`, HH `hh`, Skat `skat`; MH the rest.
fn kontra_deal(vh: &str, hh: &str, skat: &str) -> skat_core::Deal {
    let (vh, hh, skat) = (fixture::cards(vh), fixture::cards(hh), fixture::cards(skat));
    skat_core::Deal::new([vh, skat_core::CardSet::ALL - vh - hh - skat, hh], skat).unwrap()
}

/// VH: five Herz Trumpf (CJ HA H7 H8 H9) → Easy says Kontra.
const KONTRA_VH: &str = "CJ HA H7 H8 H9 SA ST S9 DA D7";
/// VH: four Herz Trumpf (S8 for H9) → Easy plays DA.
const NO_KONTRA_VH: &str = "CJ HA H7 H8 S8 SA ST S9 DA D7";
const KONTRA_HH: &str = "SJ HJ DJ HT HK HQ CA CT C9 C8";
/// Herz Hand strength 90 (Re) and 89 with C7 for CT (no Re).
const RE_90: &str = "CJ SJ HJ HA HT H9 SA CT D7 D8";
const RE_89: &str = "CJ SJ HJ HA HT H9 SA C7 D7 D8";

fn herz() -> skat_core::Announcement {
    skat_core::Announcement::plain(HERZ)
}

fn kontra_rule(g: &crate::GuidedView) -> crate::GuidedKontraRule {
    g.kontra_rule.clone().expect("revealed Kontra/Re rule")
}

fn run(s: &mut Session) {
    while s.opponent_to_move() {
        s.execute(Command::AdvanceOpponent).unwrap();
    }
}

#[test]
fn kontra_wrong_retry_attempt_reveal_and_every_legal_action_never_autoplay() {
    let s = kontra_session(
        Seat::Vorhand,
        kontra_deal(KONTRA_VH, KONTRA_HH, "D8 D9"),
        herz(),
    );
    let before = s.view();
    assert!(before.legal_actions.contains(&Action::Kontra));
    let mut s = s.clone();
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.step, S::Orient);
    // Orientation: public call, contract and level only; no factor, threshold or rule.
    let k = g.kontra.unwrap();
    assert_eq!(
        (k.call, k.game, k.level, k.hand_game),
        (
            crate::GuidedKontraCall::Kontra,
            HERZ,
            KontraLevel::None,
            true
        )
    );
    assert_eq!((k.factor, k.threshold), (None, None));
    assert!(g.kontra_rule.is_none() && g.kontra_observation.is_none() && g.rule.is_none());
    assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
    respond(&mut s, G::Answer(A::Factor(4))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    assert_eq!(s.guided().unwrap().kontra.unwrap().factor, None);
    respond(&mut s, G::Retry).unwrap();
    respond(&mut s, G::Answer(A::Factor(2))).unwrap();
    let g = s.guided().unwrap();
    assert_eq!((g.step, g.correct), (S::Threshold, None));
    let k = g.kontra.unwrap();
    assert_eq!((k.factor, k.threshold), (Some(2), Some(5)));
    assert!(g.kontra_rule.is_none());
    // Five Trumpf: "no" is wrong and reveals nothing; "yes" reveals the rule.
    respond(&mut s, G::Answer(A::Within(false))).unwrap();
    assert_eq!(s.guided().unwrap().correct, Some(false));
    assert!(s.guided().unwrap().kontra_rule.is_none());
    respond(&mut s, G::Retry).unwrap();
    respond(&mut s, G::Answer(A::Within(true))).unwrap();
    let g = s.guided().unwrap();
    assert_eq!((g.step, g.correct), (S::Act, Some(true)));
    let rule = kontra_rule(&g);
    assert_eq!(
        (rule.action, rule.count, rule.threshold),
        (Action::Kontra, 5, 5)
    );
    assert_eq!(rule.legal, before.legal_actions);
    assert_eq!(s.view().legal_actions, before.legal_actions);
    assert_eq!(s.view().hand, before.hand);
    assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
    let id = g.id;
    // Every legal action, including every card that declines Kontra, is
    // applied as chosen; the actual next public transition is observed.
    for &chosen in &before.legal_actions {
        let mut s = s.clone();
        s.execute(Command::Act(chosen)).unwrap();
        let g = s.guided().unwrap();
        match chosen {
            Action::Kontra => {
                assert_eq!(s.game.kontra(), KontraLevel::Kontra);
                assert_eq!(g.step, S::Waiting);
                assert!(g.kontra_observation.is_none());
                assert_eq!(respond(&mut s, G::Reveal), Err(AppError::NoQuestion));
                assert_eq!(
                    respond(&mut s, G::Answer(A::Within(true))),
                    Err(AppError::NoQuestion)
                );
                // The automated declarer replies; nothing else is played.
                s.execute(Command::AdvanceOpponent).unwrap();
                let g = s.guided().unwrap();
                assert_eq!(g.step, S::Complete);
                let re = s.game.kontra() == KontraLevel::Re;
                assert_eq!(
                    g.kontra_observation,
                    Some(crate::GuidedKontraObservation::Reply {
                        seat: Seat::Hinterhand,
                        re,
                        level: s.game.kontra(),
                    })
                );
            }
            Action::Play(card) => {
                assert_eq!(g.step, S::Complete);
                assert_eq!(
                    g.kontra_observation,
                    Some(crate::GuidedKontraObservation::Declined {
                        seat: Seat::Vorhand,
                        card,
                        trick: 0,
                        winner: None,
                        level: KontraLevel::None,
                    })
                );
            }
            _ => panic!("unexpected legal action"),
        }
        assert_eq!(g.id, id);
        run(&mut s);
        // The learner's next own action ends the frozen observation.
        if s.game.current_seat() == Some(Seat::Vorhand) {
            let next = s.view().legal_actions[0];
            s.execute(Command::Act(next)).unwrap();
            assert!(s.guided().is_none());
            assert_eq!(
                s.execute(Command::Guided(id, G::Reveal)),
                Err(AppError::StaleGuided)
            );
        }
    }
}

#[test]
fn kontra_decline_reveal_shows_easys_exact_card_and_returns() {
    let mut s = kontra_session(
        Seat::Vorhand,
        kontra_deal(NO_KONTRA_VH, KONTRA_HH, "D8 D9"),
        herz(),
    );
    let before = s.view();
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.step, S::Act);
    let rule = kontra_rule(&g);
    assert_eq!((rule.count, rule.wants), (4, false));
    assert_eq!(rule.action, Action::Play(fixture::card("DA")));
    assert_eq!(rule.card.unwrap().card, fixture::card("DA"));
    let k = g.kontra.unwrap();
    assert_eq!((k.factor, k.threshold), (Some(2), Some(5)));
    respond(&mut s, G::Return).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.view(), before);
}

#[test]
fn re_reply_observes_resumed_play_and_unseen_cards_keep_the_dialogue() {
    let walk = |s: &mut Session| {
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        respond(s, G::Answer(A::Factor(4))).unwrap();
        views.push(s.guided().unwrap());
        respond(s, G::Reveal).unwrap();
        views.push(s.guided().unwrap());
        views
    };
    let re_session = |hh: &str, skat: &str| {
        let deal = fixture::deal_hh(hh, skat);
        let mut s = kontra_session(Seat::Hinterhand, deal, herz());
        // VH (an automated seat) says Kontra as a legal fixture action.
        s.apply(Action::Kontra).unwrap();
        assert_eq!(s.view().legal_actions, vec![Action::Re, Action::KeinRe]);
        s
    };
    for (hh, wants) in [(RE_90, true), (RE_89, false)] {
        let mut s = re_session(hh, "D9 DT");
        let mut a = walk(&mut s);
        let k = a[0].kontra.unwrap();
        assert_eq!(
            (k.call, k.level, k.factor),
            (crate::GuidedKontraCall::Re, KontraLevel::Kontra, None)
        );
        assert_eq!(a[1].kontra.unwrap().threshold, Some(90));
        let rule = kontra_rule(&a[2]);
        assert_eq!(rule.wants, wants);
        assert_eq!(rule.action, if wants { Action::Re } else { Action::KeinRe });
        assert!(rule.skat.is_empty() && rule.hand_game);
        // A different unknown Skat and unseen hands: equal dialogue.
        let b = walk(&mut re_session(hh, "ST CA"));
        for (a, b) in a.iter_mut().zip(&b) {
            a.id = b.id;
            assert_eq!(a, b);
        }
        for reply in [Action::Re, Action::KeinRe] {
            let mut s = s.clone();
            s.execute(Command::Act(reply)).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Waiting);
            s.execute(Command::AdvanceOpponent).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Complete);
            let Some(crate::GuidedKontraObservation::Resumed { seat, card, level }) =
                g.kontra_observation
            else {
                panic!("resumed card play expected: {g:?}");
            };
            assert_eq!(seat, Seat::Vorhand);
            assert_eq!(
                s.game.view(Seat::Vorhand).current_trick.cards[0],
                Some(card)
            );
            let expected = if reply == Action::Re {
                KontraLevel::Re
            } else {
                KontraLevel::Kontra
            };
            assert_eq!((level, s.game.kontra()), (expected, expected));
        }
    }
}

#[test]
fn kontra_same_phase_replacement_stale_off_and_next_game_boundaries() {
    for (vh, hh, skat, game) in [
        (KONTRA_VH, KONTRA_HH, "D8 D9", HERZ),
        (GRAND_VH, GRAND_HH, "D9 DT", GRAND),
        (NULL_VH, GRAND_HH, "D9 DT", NULL),
    ] {
        kontra_boundaries(kontra_session(
            Seat::Vorhand,
            kontra_deal(vh, hh, skat),
            skat_core::Announcement::plain(game),
        ));
    }
}

fn kontra_boundaries(mut s: Session) {
    for level in [
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::OnRequest,
    ] {
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
    }
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    s.execute(Command::Act(Action::Kontra)).unwrap();
    s.execute(Command::AdvanceOpponent).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Complete);
    // Same seat, same card-play phase, new decision after the reply: a new
    // entry gets a new identity; the Kontra decision is gone and old answers
    // are stale.
    assert_eq!(s.game.current_seat(), Some(Seat::Vorhand));
    assert!(!s.view().legal_actions.contains(&Action::Kontra));
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    assert_ne!(g.id, old);
    assert!(g.kontra.is_none() && g.kontra_exclusion.is_none());
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    s.execute(Command::SetCoach(CoachSettings {
        level: crate::CoachLevel::Off,
        ..CoachSettings::default()
    }))
    .unwrap();
    assert!(s.guided().is_none());
    assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    s.execute(Command::StartGuided).unwrap();
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
}

#[test]
fn ouvert_kontra_offers_get_honest_fallbacks() {
    use crate::GuidedKontraExclusion as X;
    let deal = kontra_deal(KONTRA_VH, KONTRA_HH, "D8 D9");
    let ouvert = skat_core::Announcement {
        ouvert: true,
        ..skat_core::Announcement::plain(skat_core::GameType::Null)
    };
    for (game, exclusion) in [(ouvert, X::OuvertRound)] {
        let mut s = kontra_session(Seat::Vorhand, deal, game);
        let before = s.view();
        s.execute(Command::StartGuided).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(
            (g.step, g.kontra_exclusion),
            (S::Unsupported, Some(exclusion))
        );
        assert!(g.kontra.is_none() && g.kontra_rule.is_none());
        assert_eq!(respond(&mut s, G::Reveal), Err(AppError::NoQuestion));
        // All legal calls and cards stay usable.
        for &a in &before.legal_actions {
            s.clone().execute(Command::Act(a)).unwrap();
        }
        if exclusion == X::OuvertRound {
            // Before the reveal, no declarer card reaches the view.
            assert!(before.open_cards.is_none());
            s.execute(Command::Act(Action::Kontra)).unwrap();
            assert!(s.guided().is_none());
        }
    }
    // Null ouvert after the separate round: the declarer's Re reply stays excluded.
    let mut s = kontra_session(Seat::Hinterhand, deal, ouvert);
    s.apply(Action::Kontra).unwrap();
    assert_eq!(s.view().legal_actions, vec![Action::Re, Action::KeinRe]);
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(
        (g.step, g.kontra_exclusion),
        (S::Unsupported, Some(X::Ouvert))
    );
    assert!(g.kontra.is_none() && g.kontra_rule.is_none());
}

#[test]
fn god_mode_does_not_broaden_the_kontra_lesson_input() {
    // God Mode shows every card and analyses the real deal, but the guided
    // Kontra/Re lesson still uses the closed permitted view.
    let walk = |god: bool| {
        let mut s = kontra_session(
            Seat::Vorhand,
            kontra_deal(KONTRA_VH, KONTRA_HH, "D8 D9"),
            herz(),
        );
        if god {
            s.config.opponents = Opponents::GodMode;
            assert!(s.view().open_cards.is_some());
        }
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        respond(&mut s, G::Answer(A::Factor(2))).unwrap();
        views.push(s.guided().unwrap());
        respond(&mut s, G::Reveal).unwrap();
        views.push(s.guided().unwrap());
        views
    };
    let closed = walk(false);
    for (mut god, closed) in walk(true).into_iter().zip(closed) {
        god.id = closed.id;
        god.basis = closed.basis;
        assert_eq!(god, closed);
    }
}

// Grand and Null Kontra/Re (#155), fixtures from `skat-coach/tests/guided_kontra.rs`.
const GRAND: skat_core::GameType = skat_core::GameType::Grand;
const NULL: skat_core::GameType = skat_core::GameType::Null;
/// VH with all four Buben (4 < 5 Trumpf): Easy plays DA.
const GRAND_VH: &str = "CJ SJ HJ DJ SA ST S9 DA D7 D8";
/// VH for Null: Easy leads its lowest card C7.
const NULL_VH: &str = "C7 CA SA ST S9 H9 DA CJ SJ HJ";
const GRAND_HH: &str = "HA HT HK HQ CT C9 SK SQ DK DQ";
/// Grand Hand strength 90 (Re) and 89 (SJ for HA, no Re).
const GRAND_90: &str = "CJ DJ CA SA ST S7 HA HT DA DT";
const GRAND_89: &str = "CJ SJ DJ CA SA ST S7 HT DA DT";

#[test]
fn grand_and_null_kontra_dialogue_every_legal_action_never_autoplay() {
    // Grand: count question, threshold 5, wrong "yes". Null: applicability
    // question without a threshold; "applies" and any count answer are wrong.
    for (game, vh, card, wrong, right) in [
        (
            GRAND,
            GRAND_VH,
            "DA",
            vec![A::Within(true)],
            A::Within(false),
        ),
        (
            NULL,
            NULL_VH,
            "C7",
            vec![A::Applies(true), A::Within(false)],
            A::Applies(false),
        ),
    ] {
        let s = kontra_session(
            Seat::Vorhand,
            kontra_deal(vh, GRAND_HH, "D9 DT"),
            skat_core::Announcement::plain(game),
        );
        let before = s.view();
        let mut s = s.clone();
        s.execute(Command::StartGuided).unwrap();
        let g = s.guided().unwrap();
        let k = g.kontra.unwrap();
        assert_eq!(
            (g.step, k.call, k.game, k.factor, k.threshold),
            (S::Orient, crate::GuidedKontraCall::Kontra, game, None, None)
        );
        assert!(g.kontra_rule.is_none() && g.kontra_exclusion.is_none());
        respond(&mut s, G::Answer(A::Factor(2))).unwrap();
        let k = s.guided().unwrap().kontra.unwrap();
        let threshold = (game == GRAND).then_some(5);
        assert_eq!((k.factor, k.threshold), (Some(2), threshold));
        for w in wrong {
            respond(&mut s, G::Answer(w)).unwrap();
            let g = s.guided().unwrap();
            assert_eq!((g.step, g.correct), (S::Threshold, Some(false)));
            assert!(g.kontra_rule.is_none());
            respond(&mut s, G::Retry).unwrap();
        }
        respond(&mut s, G::Answer(right)).unwrap();
        let g = s.guided().unwrap();
        assert_eq!((g.step, g.correct), (S::Act, Some(true)));
        let rule = kontra_rule(&g);
        assert_eq!(
            (rule.action, rule.wants, rule.null),
            (Action::Play(fixture::card(card)), false, game == NULL)
        );
        assert_eq!(rule.legal, before.legal_actions);
        // Nothing was played for the learner.
        assert_eq!(s.view().legal_actions, before.legal_actions);
        assert_eq!(s.view().hand, before.hand);
        let id = g.id;
        for &chosen in &before.legal_actions {
            let mut s = s.clone();
            s.execute(Command::Act(chosen)).unwrap();
            if chosen == Action::Kontra {
                assert_eq!(s.guided().unwrap().step, S::Waiting);
                s.execute(Command::AdvanceOpponent).unwrap();
                let g = s.guided().unwrap();
                assert_eq!(
                    g.kontra_observation,
                    Some(crate::GuidedKontraObservation::Reply {
                        seat: Seat::Hinterhand,
                        re: s.game.kontra() == KontraLevel::Re,
                        level: s.game.kontra(),
                    })
                );
                // Easy's own reply: Re only from strength 90, never in Null.
                if game == NULL {
                    assert_eq!(s.game.kontra(), KontraLevel::Kontra);
                }
            } else {
                let Action::Play(played) = chosen else {
                    panic!("unexpected legal action {chosen:?}");
                };
                assert_eq!(
                    s.guided().unwrap().kontra_observation,
                    Some(crate::GuidedKontraObservation::Declined {
                        seat: Seat::Vorhand,
                        card: played,
                        trick: 0,
                        winner: None,
                        level: KontraLevel::None,
                    })
                );
            }
            let g = s.guided().unwrap();
            assert_eq!((g.step, g.id), (S::Complete, id));
        }
        // New game: the frozen dialogue does not carry over.
        let mut s = s;
        while s.game.current_seat().is_some() {
            s.apply(s.game.legal_actions()[0]).unwrap();
        }
        s.execute(Command::NextGame).unwrap();
        assert!(s.guided().is_none());
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
}

/// Contract, declarer cards, Skat, Hand or pickup, expected count, correct answer.
type ReCase<'a> = (skat_core::GameType, &'a str, &'a str, &'a [Action], i32, A);

#[test]
fn grand_and_null_re_reply_both_answers_and_unseen_cards_keep_the_dialogue() {
    let walk = |s: &mut Session, right: A| {
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        respond(s, G::Answer(A::Factor(4))).unwrap();
        views.push(s.guided().unwrap());
        respond(s, G::Answer(right)).unwrap();
        views.push(s.guided().unwrap());
        views
    };
    let re_session = |game, hh: &str, skat: &str, after: &[Action]| {
        let mut s = kontra_session_after(
            Seat::Hinterhand,
            fixture::deal_hh(hh, skat),
            after,
            skat_core::Announcement::plain(game),
        );
        // VH (an automated seat) says Kontra as a legal fixture action.
        s.apply(Action::Kontra).unwrap();
        assert_eq!(s.view().legal_actions, vec![Action::Re, Action::KeinRe]);
        s
    };
    let hand = [Action::Hand];
    let pickup_90 = [Action::TakeSkat, Action::Discard(fixture::cards("SA ST"))];
    let pickup_89 = [Action::TakeSkat, Action::Discard(fixture::cards("HJ ST"))];
    let cases: [ReCase; 6] = [
        (GRAND, GRAND_90, "D9 D8", &hand, 90, A::Within(true)),
        (GRAND, GRAND_89, "D9 D8", &hand, 89, A::Within(false)),
        (
            GRAND,
            "CJ SJ CA CT HA HT DA DQ SA ST",
            "H9 S8",
            &pickup_90,
            90,
            A::Within(true),
        ),
        (
            GRAND,
            "CJ SJ CA CT HA HT DA DQ HJ ST",
            "H9 S8",
            &pickup_89,
            89,
            A::Within(false),
        ),
        (NULL, GRAND_90, "D9 D8", &hand, 0, A::Applies(false)),
        (
            NULL,
            "CJ SJ CA CT HA HT DA DQ SA ST",
            "H9 S8",
            &pickup_90,
            0,
            A::Applies(false),
        ),
    ];
    for (game, hh, skat, after, count, right) in cases {
        let mut s = re_session(game, hh, skat, after);
        let mut a = walk(&mut s, right);
        assert_eq!(a[0].kontra.unwrap().threshold, None);
        assert_eq!(
            a[1].kontra.unwrap().threshold,
            (game == GRAND).then_some(90)
        );
        assert_eq!((a[2].step, a[2].correct), (S::Act, Some(true)));
        let rule = kontra_rule(&a[2]);
        let wants = count >= 90;
        assert_eq!((rule.count, rule.wants), (count, wants));
        assert_eq!(rule.action, if wants { Action::Re } else { Action::KeinRe });
        assert_eq!(rule.hand_game, after == hand);
        if after == hand {
            // A different unknown Skat and unseen hands: equal dialogue.
            let b = walk(&mut re_session(game, hh, "HJ SK", after), right);
            for (a, b) in a.iter_mut().zip(&b) {
                a.id = b.id;
                assert_eq!(a, b);
            }
        }
        for reply in [Action::Re, Action::KeinRe] {
            let mut s = s.clone();
            s.execute(Command::Act(reply)).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Waiting);
            s.execute(Command::AdvanceOpponent).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Complete);
            let Some(crate::GuidedKontraObservation::Resumed { seat, card, level }) =
                g.kontra_observation
            else {
                panic!("resumed card play expected: {g:?}");
            };
            assert_eq!(seat, Seat::Vorhand);
            assert_eq!(
                s.game.view(Seat::Vorhand).current_trick.cards[0],
                Some(card)
            );
            let expected = if reply == Action::Re {
                KontraLevel::Re
            } else {
                KontraLevel::Kontra
            };
            assert_eq!((level, s.game.kontra()), (expected, expected));
        }
    }
}

#[test]
fn god_mode_keeps_the_grand_and_null_kontra_lesson_input() {
    for (game, vh, right) in [
        (GRAND, GRAND_VH, A::Within(false)),
        (NULL, NULL_VH, A::Applies(false)),
    ] {
        let walk = |god: bool| {
            let mut s = kontra_session(
                Seat::Vorhand,
                kontra_deal(vh, GRAND_HH, "D9 DT"),
                skat_core::Announcement::plain(game),
            );
            if god {
                s.config.opponents = Opponents::GodMode;
                assert!(s.view().open_cards.is_some());
            }
            s.execute(Command::StartGuided).unwrap();
            let mut views = vec![s.guided().unwrap()];
            respond(&mut s, G::Answer(A::Factor(2))).unwrap();
            views.push(s.guided().unwrap());
            respond(&mut s, G::Answer(right)).unwrap();
            views.push(s.guided().unwrap());
            views
        };
        let closed = walk(false);
        for (mut god, closed) in walk(true).into_iter().zip(closed) {
            god.id = closed.id;
            god.basis = closed.basis;
            assert_eq!(god, closed);
        }
    }
}

/// Human on `human` after core-accepted `actions` (see
/// `skat-coach/tests/guided_grand.rs` for the hand-derived expectations).
fn grand_session(
    human: Seat,
    deal: skat_core::Deal,
    kontra_re: bool,
    actions: &[Action],
) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        series: SeriesLength::Free,
        kontra_re,
        ..SessionConfig::default()
    });
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == human)
        .unwrap();
    s.game = Game::from_deal(deal, RuleConfig { kontra_re });
    s.recording.deal = deal;
    for &a in actions {
        s.apply(a).unwrap();
    }
    s
}

/// Grand Hand by `declarer`, then `plays`; the human sits last in Hinterhand.
fn grand_last(declarer: Seat, deal: skat_core::Deal, plays: &[&str]) -> Session {
    let game = fixture::hand_game(GRAND, false);
    let actions = fixture::contract_actions(deal, declarer, game, plays);
    grand_session(Seat::Hinterhand, deal, false, &actions)
}

#[test]
fn grand_win_last_both_roles_attempt_reveal_every_legal_card_and_actual_trick() {
    use fixture::card as c;
    // (declarer, lead, partner/opponent card, Easy's card, winning cards)
    for (declarer, led, second, easy, winning) in [
        (Seat::Hinterhand, "C9", "C8", "CQ", vec!["CQ", "CA"]),
        (Seat::Hinterhand, "CK", "C8", "CA", vec!["CA"]),
        (Seat::Vorhand, "C9", "C7", "CQ", vec!["CQ", "CA"]),
        (Seat::Vorhand, "CK", "C7", "CA", vec!["CA"]),
    ] {
        let s = grand_last(declarer, fixture::deal(), &[led, second]);
        let legal = s.view().legal_actions;
        assert_eq!(
            legal,
            vec![Action::Play(c("CQ")), Action::Play(c("CA"))],
            "free legal choice"
        );
        for (n, &action) in legal.iter().enumerate() {
            let mut s = s.clone();
            let before = s.view();
            s.execute(Command::StartGuided).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Orient);
            assert!(g.rule.is_none() && g.task.is_none() && g.memory.is_none());
            assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
            respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            respond(&mut s, G::Retry).unwrap();
            respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
            let g = s.guided().unwrap();
            assert_eq!((g.step, g.correct), (S::Check, None));
            // Last seat: nobody follows; a named seat is wrong.
            respond(&mut s, G::Answer(A::Seat(Some(Seat::Mittelhand)))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            respond(&mut s, G::Answer(A::Seat(None))).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(
                (g.step, g.task),
                (S::Narrow, Some(crate::GuidedBranch::WinLast))
            );
            assert!(g.rule.is_none(), "no early card or key");
            if winning.len() == 1 {
                respond(&mut s, G::Answer(A::Card(c("CQ")))).unwrap();
                assert_eq!(s.guided().unwrap().correct, Some(false));
            }
            if n == 0 {
                // Any winning card is a correct attempt, not only Easy's.
                let attempt = winning[winning.len() - 1];
                respond(&mut s, G::Answer(A::Card(c(attempt)))).unwrap();
            } else {
                respond(&mut s, G::Reveal).unwrap();
            }
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Act);
            let rule = g.rule.unwrap();
            assert_eq!(
                (rule.card, rule.branch),
                (c(easy), crate::GuidedBranch::WinLast)
            );
            let expected: Vec<_> = winning.iter().map(|w| c(w)).collect();
            assert_eq!(rule.candidates, expected);
            respond(&mut s, G::Memory(true)).unwrap();
            assert_eq!(s.guided().unwrap().memory.unwrap().skat, None);
            assert_eq!(s.view().legal_actions, before.legal_actions);
            assert_eq!(s.view().hand, before.hand);
            // The learner's own last card completes the trick: no remaining seat to await.
            s.execute(Command::Act(action)).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Observe);
            assert!(g.memory.is_none());
            respond(&mut s, G::Answer(A::Void(true))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            respond(&mut s, G::Answer(A::Void(false))).unwrap();
            let o = s.guided().unwrap().observation.unwrap();
            let Action::Play(played) = action else {
                panic!("card");
            };
            assert_eq!(
                o.source,
                skat_coach::Source::Trick {
                    index: 0,
                    seat: Seat::Hinterhand,
                    led: c(led),
                    card: played
                }
            );
            let actual = s.game.view(Seat::Hinterhand).tricks[0];
            assert_eq!(actual.cards, [c(led), c(second), played]);
            assert_eq!(o.winner, Some(actual.winner));
            let wins = winning.iter().any(|w| c(w) == played);
            assert_eq!(
                actual.winner,
                if wins {
                    Seat::Hinterhand
                } else {
                    Seat::Vorhand
                }
            );
            assert!(!o.proves_void);
            respond(&mut s, G::Return).unwrap();
            assert!(s.guided().is_none());
        }
    }
}

#[test]
fn grand_win_last_hidden_swap_and_god_mode_keep_the_permitted_lesson() {
    let walk = |deal: skat_core::Deal, god: bool| {
        let mut s = grand_last(Seat::Hinterhand, deal, &["C9", "C8"]);
        if god {
            s.config.opponents = Opponents::GodMode;
            assert!(s.view().open_cards.is_some());
        }
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        for action in [
            G::Answer(A::Seat(Some(Seat::Vorhand))),
            G::Answer(A::Seat(None)),
            G::Answer(A::Card(fixture::card("CQ"))),
            G::Memory(true),
        ] {
            respond(&mut s, action).unwrap();
            views.push(s.guided().unwrap());
        }
        s.execute(Command::Act(Action::Play(fixture::card("CA"))))
            .unwrap();
        views.push(s.guided().unwrap());
        respond(&mut s, G::Reveal).unwrap();
        views.push(s.guided().unwrap());
        views
    };
    let base = walk(fixture::deal(), false);
    let swapped = fixture::swap(fixture::deal(), fixture::card("H7"), fixture::card("DK"));
    for other in [walk(swapped, false), walk(fixture::deal(), true)] {
        assert_eq!(other.len(), base.len());
        for (mut v, b) in other.into_iter().zip(&base) {
            v.id = b.id;
            v.basis = b.basis;
            assert_eq!(&v, b);
        }
    }
    assert_eq!(base.last().unwrap().step, S::Complete);
}

#[test]
fn grand_kontra_precedence_exclusions_and_lifecycle_boundaries() {
    // Kontra-eligible defender (HH bid 18): the Kontra dialogue wins.
    let actions = [
        Action::Pass,
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Hand,
        Action::Announce(fixture::hand_game(GRAND, false)),
        Action::Play(fixture::card("C9")),
        Action::Play(fixture::card("C7")),
    ];
    let mut s = grand_session(Seat::Hinterhand, fixture::deal(), true, &actions);
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.step, S::Orient);
    assert!(g.kontra.is_some());
    respond(&mut s, G::Reveal).unwrap();
    assert!(s.guided().unwrap().rule.is_none());

    // Grand partner-ahead Fallback without a losing card (MH holds only
    // CT/CK in Kreuz, #159) and Ouvert Grand WinLast: honest fallback.
    let no_loser = fixture::swap(
        fixture::swap(fixture::deal(), fixture::card("C7"), fixture::card("CK")),
        fixture::card("C8"),
        fixture::card("H7"),
    );
    let under = fixture::contract_actions(
        no_loser,
        Seat::Hinterhand,
        fixture::hand_game(GRAND, false),
        &["C9"],
    );
    let ouvert = fixture::contract_actions(
        fixture::deal(),
        Seat::Hinterhand,
        fixture::hand_game(GRAND, true),
        &["C9", "C8"],
    );
    for (human, deal, actions) in [
        (Seat::Mittelhand, no_loser, under),
        (Seat::Hinterhand, fixture::deal(), ouvert),
    ] {
        let mut s = grand_session(human, deal, false, &actions);
        assert!(s.view().legal_actions.len() >= 2);
        s.execute(Command::StartGuided).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Unsupported);
        assert!(g.rule.is_none() && g.kontra_exclusion.is_none());
        respond(&mut s, G::Return).unwrap();
    }

    // Automatic levels never start it; off rejects it; re-entry is stale;
    // the next game drops it.
    for level in [
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::Off,
    ] {
        let mut s = grand_last(Seat::Vorhand, fixture::deal(), &["C9", "C7"]);
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        if level == crate::CoachLevel::Off {
            assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
        }
    }
    let mut s = grand_last(Seat::Vorhand, fixture::deal(), &["C9", "C7"]);
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
}

/// Grand Hand by HH after VH's CK; the human defends in Mittelhand (#159).
fn grand_under(deal: skat_core::Deal) -> Session {
    let game = fixture::hand_game(GRAND, false);
    let actions = fixture::contract_actions(deal, Seat::Hinterhand, game, &["CK"]);
    grand_session(Seat::Mittelhand, deal, false, &actions)
}

#[test]
fn grand_under_partner_attempt_reveal_every_legal_card_and_actual_declarer_reply() {
    use fixture::card as c;
    let s = grand_under(fixture::deal());
    let legal = s.view().legal_actions;
    assert_eq!(
        legal,
        ["C7", "C8", "CT"].map(|x| Action::Play(c(x))).to_vec(),
        "free legal choice"
    );
    for (n, &action) in legal.iter().enumerate() {
        let mut s = s.clone();
        let before = s.view();
        s.execute(Command::StartGuided).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Orient);
        assert!(g.rule.is_none() && g.task.is_none() && g.memory.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        // The declarer is not ahead; VH's CK is.
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
        assert_eq!(s.guided().unwrap().step, S::Check);
        // Not last: the declarer HH still acts after the learner.
        respond(&mut s, G::Answer(A::Seat(None))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(
            (g.step, g.task),
            (S::Narrow, Some(crate::GuidedBranch::UnderPartner))
        );
        assert!(g.rule.is_none(), "no early card or key");
        // CT overtakes the partner: not a candidate.
        respond(&mut s, G::Answer(A::Card(c("CT")))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        if n == 0 {
            // C8 is a correct non-Easy candidate.
            respond(&mut s, G::Answer(A::Card(c("C8")))).unwrap();
        } else {
            respond(&mut s, G::Reveal).unwrap();
        }
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Act);
        let rule = g.rule.unwrap();
        assert_eq!(
            (rule.card, rule.branch),
            (c("C7"), crate::GuidedBranch::UnderPartner)
        );
        assert_eq!(rule.candidates, vec![c("C7"), c("C8")]);
        respond(&mut s, G::Memory(true)).unwrap();
        assert_eq!(s.guided().unwrap().memory.unwrap().skat, None);
        assert_eq!(s.view().legal_actions, before.legal_actions);
        assert_eq!(s.view().hand, before.hand);
        s.execute(Command::Act(action)).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Waiting);
        assert!(g.memory.is_none() && g.observation.is_none());
        // The real automated declarer answers: Easy HH, last, overtakes with CA.
        let id = g.id;
        let mv = s.opponent_job().unwrap().run(None);
        s.finish_opponent(mv).unwrap();
        let g = s.guided().unwrap();
        assert_eq!((g.id, g.step), (id, S::Observe));
        respond(&mut s, G::Answer(A::Void(true))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Answer(A::Void(false))).unwrap();
        let o = s.guided().unwrap().observation.unwrap();
        let Action::Play(played) = action else {
            panic!("card");
        };
        let actual = s.game.view(Seat::Mittelhand).tricks[0];
        assert_eq!(actual.cards, [c("CK"), played, c("CA")]);
        assert_eq!(actual.winner, Seat::Hinterhand);
        assert_eq!(
            o.source,
            skat_coach::Source::Trick {
                index: 0,
                seat: Seat::Hinterhand,
                led: c("CK"),
                card: c("CA")
            }
        );
        assert_eq!(o.winner, Some(actual.winner));
        assert!(!o.proves_void);
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}

#[test]
fn grand_under_partner_hidden_swap_and_god_mode_keep_the_permitted_lesson() {
    let walk = |deal: skat_core::Deal, god: bool| {
        let mut s = grand_under(deal);
        if god {
            s.config.opponents = Opponents::GodMode;
            assert!(s.view().open_cards.is_some());
        }
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        for action in [
            G::Answer(A::Seat(Some(Seat::Vorhand))),
            G::Answer(A::Seat(Some(Seat::Hinterhand))),
            G::Answer(A::Card(fixture::card("C8"))),
            G::Memory(true),
        ] {
            respond(&mut s, action).unwrap();
            views.push(s.guided().unwrap());
        }
        s.execute(Command::Act(Action::Play(fixture::card("CT"))))
            .unwrap();
        views.push(s.guided().unwrap());
        s.run_opponents();
        views.push(s.guided().unwrap());
        respond(&mut s, G::Reveal).unwrap();
        views.push(s.guided().unwrap());
        views
    };
    let base = walk(fixture::deal(), false);
    // HH's unplayed D8 swaps with the unknown Hand Skat's DK.
    let swapped = fixture::swap(fixture::deal(), fixture::card("D8"), fixture::card("DK"));
    for other in [walk(swapped, false), walk(fixture::deal(), true)] {
        assert_eq!(other.len(), base.len());
        for (mut v, b) in other.into_iter().zip(&base) {
            v.id = b.id;
            v.basis = b.basis;
            assert_eq!(&v, b);
        }
    }
    let last = base.last().unwrap();
    assert_eq!(last.step, S::Complete);
    assert_eq!(last.observation.unwrap().winner, Some(Seat::Hinterhand));
}

#[test]
fn grand_under_partner_kontra_precedence_skip_off_stale_and_next_game() {
    // MH bid 18 before passing: Kontra-eligible, the Kontra dialogue wins.
    let actions = [
        Action::Bid(18),
        Action::Hold,
        Action::Pass,
        Action::Bid(20),
        Action::Pass,
        Action::Hand,
        Action::Announce(fixture::hand_game(GRAND, false)),
        Action::Play(fixture::card("CK")),
    ];
    let mut s = grand_session(Seat::Mittelhand, fixture::deal(), true, &actions);
    s.execute(Command::StartGuided).unwrap();
    let g = s.guided().unwrap();
    assert!(g.kontra.is_some() && g.task.is_none());
    respond(&mut s, G::Reveal).unwrap();
    assert!(s.guided().unwrap().rule.is_none());

    // Automatic levels never start it; off rejects it.
    for level in [
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::Off,
    ] {
        let mut s = grand_under(fixture::deal());
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        if level == crate::CoachLevel::Off {
            assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
        }
    }
    // Skip: return before answering, play unaided; re-entry is stale; the
    // next game drops it.
    let mut s = grand_under(fixture::deal());
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Return).unwrap();
    assert!(s.guided().is_none());
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    respond(&mut s, G::Return).unwrap();
    s.execute(Command::Act(Action::Play(fixture::card("CT"))))
        .unwrap();
    assert!(s.guided().is_none(), "unaided play stays unaided");
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
}

#[test]
fn grand_under_partner_estimates_keep_the_original_rule() {
    let mut s = grand_under(fixture::deal());
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let rule = s.guided().unwrap().rule;
    assert_eq!(rule.as_ref().unwrap().card, fixture::card("C7"));
    // Ranking intentionally disagrees (CT first); it cannot rewrite the rule.
    let ct = Action::Play(fixture::card("CT"));
    let result = s.coach_job().unwrap().run(None).guided_rank_first(ct);
    assert_eq!(result.explanation().unwrap().options[0].action, ct);
    s.finish_coach(result).unwrap();
    s.execute(Command::Act(Action::Play(fixture::card("C8"))))
        .unwrap();
    s.run_opponents();
    respond(&mut s, G::Reveal).unwrap();
    s.execute(Command::SetCoach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    }))
    .unwrap();
    assert!(s.guided().unwrap().figures.is_none());
    respond(&mut s, G::Figures).unwrap();
    let g = s.guided().unwrap();
    assert!(g.figures_requested);
    assert_eq!(g.rule, rule);
    if let Some(h) = g.figures {
        assert_ne!(h.status, skat_coach::Status::Complete);
    }
}

/// Bounded ordinary-app route search (#159), mirroring the egui walkthrough
/// helper: seeds 0..128, at most three games each, Kontra/Re off, Easy
/// opponents; the human passes every bid and otherwise plays its first legal
/// card. Records each seed's first human decision whose permitted view gets
/// the Grand UnderPartner lesson as (seed, game, completed tricks).
#[test]
fn ordinary_grand_under_partner_routes_within_the_bounded_seed_search() {
    use fixture::card as c;
    let mut routes = Vec::new();
    let mut seed_119 = None;
    for seed in 0..128u64 {
        let mut s = Session::new(SessionConfig {
            seed,
            series: SeriesLength::Free,
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        });
        'games: for game in 1..=3u32 {
            loop {
                s.run_opponents();
                let v = s.view();
                if v.can_next_game {
                    break;
                }
                let seat = PlayerId::HUMAN.seat(s.dealer);
                assert_eq!(s.game.current_seat(), Some(seat));
                let pv = s.game.view(seat);
                if let Some(l) = skat_coach::guided::lesson(&pv)
                    && l.game == GRAND
                    && l.rule.branch == crate::GuidedBranch::UnderPartner
                {
                    routes.push((seed, game, pv.tricks.len()));
                    if seed == 119 {
                        seed_119 = Some((s.clone(), l));
                    }
                    break 'games;
                }
                let a = if v.legal_actions.contains(&Action::Pass) {
                    Action::Pass
                } else {
                    v.legal_actions[0]
                };
                s.execute(Command::Act(a)).unwrap();
            }
            s.execute(Command::NextGame).unwrap();
        }
    }
    // Seed 97 (VH's S7, every legal card loses) is first; the egui walkthrough
    // uses seed 119, the first route with both a losing and an overtaking card.
    assert_eq!(routes, vec![(97, 3, 0), (106, 3, 2), (119, 3, 5)]);
    let (s, l) = seed_119.unwrap();
    let v = s.view();
    assert_eq!(PlayerId::HUMAN.seat(s.dealer), Seat::Mittelhand);
    assert_eq!(
        s.game.view(Seat::Mittelhand).declarer,
        Some(Seat::Hinterhand)
    );
    // VH leads H8; H7 stays below, H9 overtakes; Easy H7.
    assert_eq!(
        v.current_trick.iter().map(|p| p.card).collect::<Vec<_>>(),
        vec![c("H8")]
    );
    assert_eq!(
        v.legal_actions,
        vec![Action::Play(c("H7")), Action::Play(c("H9"))]
    );
    assert_eq!(
        (l.rule.losers, l.rule.winners),
        (vec![c("H7")], vec![c("H9")])
    );
    assert_eq!(l.rule.card, c("H7"));
    // Actual automated declarer reply: HQ wins after either card.
    for card in ["H7", "H9"] {
        let mut t = s.clone();
        t.execute(Command::Act(Action::Play(c(card)))).unwrap();
        t.run_opponents();
        let trick = *t.game.view(Seat::Mittelhand).tricks.last().unwrap();
        assert_eq!(trick.cards, [c("H8"), c(card), c("HQ")]);
        assert_eq!(trick.winner, Seat::Hinterhand);
    }
}

#[path = "../../skat-coach/tests/support/win_early.rs"]
mod early_oracle;

/// Fixture-only MH role variants; ordinary widget route covers MH declarer.
fn grand_early(deal: skat_core::Deal, declarer: Seat) -> Session {
    let actions =
        fixture::contract_actions(deal, declarer, fixture::hand_game(GRAND, false), &["CK"]);
    grand_session(Seat::Mittelhand, deal, false, &actions)
}

#[test]
fn grand_win_early_a_to_e_both_fixture_roles_attempt_memory_all_legal_actual_replies() {
    use fixture::card as c;
    for declarer in [Seat::Mittelhand, Seat::Vorhand] {
        for (n, deal) in early_oracle::deals().into_iter().enumerate() {
            let original = grand_early(deal, declarer);
            let legal: Vec<_> = fixture::cards(early_oracle::LEGAL[n])
                .iter()
                .map(Action::Play)
                .collect();
            assert_eq!(original.view().legal_actions, legal);
            for mine in early_oracle::LEGAL[n].split_whitespace() {
                let mut s = original.clone();
                let before = s.view();
                s.execute(Command::StartGuided).unwrap();
                let g = s.guided().unwrap();
                assert!(g.rule.is_none() && g.early_condition.is_none() && g.memory.is_none());
                assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
                if n == 4 {
                    assert_eq!(g.step, S::Unsupported);
                    s.execute(Command::Act(Action::Play(c(mine)))).unwrap();
                    s.apply(Action::Play(c(early_oracle::REPLY[n]))).unwrap();
                    assert_eq!(s.game.tricks()[0].winner, early_oracle::winner(n, mine));
                    continue;
                }
                assert_eq!(g.step, S::Orient);
                respond(&mut s, G::Answer(A::Seat(Some(Seat::Mittelhand)))).unwrap();
                assert_eq!(s.guided().unwrap().correct, Some(false));
                respond(&mut s, G::Retry).unwrap();
                respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
                assert!(s.guided().unwrap().early_condition.is_none());
                respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
                let loser = if n < 2 { "C8" } else { "D7" };
                respond(&mut s, G::Answer(A::Card(c(loser)))).unwrap();
                assert_eq!(s.guided().unwrap().correct, Some(false));
                respond(&mut s, G::Retry).unwrap();
                // Any current winner, including the non-Easy CA, is accepted.
                if declarer == Seat::Mittelhand {
                    respond(
                        &mut s,
                        G::Answer(A::Card(c(if n < 2 { "CA" } else { "SJ" }))),
                    )
                    .unwrap();
                } else {
                    respond(&mut s, G::Reveal).unwrap();
                }
                let g = s.guided().unwrap();
                assert_eq!(g.step, S::Act);
                assert_eq!(g.rule.unwrap().card, c(early_oracle::EASY[n]));
                assert!(g.early_condition.is_some());
                assert!(g.memory.is_none() && g.figures.is_none());
                respond(&mut s, G::Memory(true)).unwrap();
                let m = s.guided().unwrap().memory.unwrap();
                assert_eq!(m.skat, None);
                assert_eq!(m.own, before.hand.iter().copied().collect());
                respond(&mut s, G::Memory(false)).unwrap();
                assert!(s.guided().unwrap().memory.is_none());
                assert_eq!(s.view().legal_actions, before.legal_actions);
                s.execute(Command::Act(Action::Play(c(mine)))).unwrap();
                assert_eq!(s.guided().unwrap().step, S::Waiting);
                assert!(s.guided().unwrap().memory.is_none());
                s.apply(Action::Play(c(early_oracle::REPLY[n]))).unwrap();
                assert_eq!(s.guided().unwrap().step, S::Observe);
                assert!(s.guided().unwrap().observation.is_none());
                respond(&mut s, G::Reveal).unwrap();
                let o = s.guided().unwrap().observation.unwrap();
                assert_eq!(o.winner, Some(early_oracle::winner(n, mine)));
                assert_eq!(o.proves_void, n == 1 || n == 3);
                assert_eq!(
                    o.source,
                    skat_coach::Source::Trick {
                        index: 0,
                        seat: Seat::Hinterhand,
                        led: c("CK"),
                        card: c(early_oracle::REPLY[n])
                    }
                );
                respond(&mut s, G::Return).unwrap();
                assert!(s.guided().is_none());
            }
        }
    }
}

#[test]
fn grand_win_early_hidden_pairs_god_mode_and_estimates_keep_permitted_dialogue() {
    let walk = |deal, decl, god| {
        let mut s = grand_early(deal, decl);
        if god {
            s.config.opponents = Opponents::GodMode;
        }
        s.execute(Command::StartGuided).unwrap();
        let mut views = vec![s.guided().unwrap()];
        for a in [
            G::Answer(A::Seat(Some(Seat::Vorhand))),
            G::Answer(A::Seat(Some(Seat::Hinterhand))),
            G::Reveal,
            G::Memory(true),
        ] {
            respond(&mut s, a).unwrap();
            views.push(s.guided().unwrap());
        }
        views
    };
    for decl in [Seat::Mittelhand, Seat::Vorhand] {
        let deals = early_oracle::deals();
        for (a, b) in [
            (deals[0], deals[1]),
            (deals[2], deals[3]),
            (
                deals[0],
                fixture::swap(deals[0], fixture::card("DK"), fixture::card("SA")),
            ),
        ] {
            let base = walk(a, decl, false);
            for other in [walk(b, decl, false), walk(a, decl, true)] {
                for (mut o, b) in other.into_iter().zip(&base) {
                    o.id = b.id;
                    o.basis = b.basis;
                    assert_eq!(&o, b);
                }
            }
        }
    }
    // Rule availability/selection never waits for optional estimates.
    let mut s = grand_early(early_oracle::deals()[0], Seat::Mittelhand);
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    assert_eq!(
        s.guided().unwrap().rule.unwrap().card,
        fixture::card(early_oracle::EASY[0])
    );
    assert!(s.guided().unwrap().figures.is_none());
    assert_eq!(early_oracle::WINNERS[0], "CT CA");
}

#[test]
fn grand_win_early_stale_reentry_off_automatic_and_new_game_reset() {
    let mut s = grand_early(early_oracle::deals()[0], Seat::Mittelhand);
    for level in [
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
        crate::CoachLevel::Off,
    ] {
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        if level == crate::CoachLevel::Off {
            assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
        }
    }
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    s.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    respond(&mut s, G::Return).unwrap();
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    let id = s.guided().unwrap().id;
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(
        s.execute(Command::Guided(id, G::Memory(true))),
        Err(AppError::StaleGuided)
    );
}

#[test]
fn grand_win_early_optional_incomplete_disagreeing_estimates_never_select_rule() {
    let mut s = grand_early(early_oracle::deals()[0], Seat::Mittelhand);
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let rule = s.guided().unwrap().rule;
    let condition = s.guided().unwrap().early_condition;
    let result = s
        .coach_job()
        .unwrap()
        .run(None)
        .guided_rank_first(Action::Play(fixture::card("C8")));
    assert_eq!(
        result.explanation().unwrap().options[0].action,
        Action::Play(fixture::card("C8"))
    );
    s.finish_coach(result).unwrap();
    s.execute(Command::Act(Action::Play(fixture::card("CT"))))
        .unwrap();
    s.apply(Action::Play(fixture::card("CQ"))).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    assert!(s.guided().unwrap().figures.is_none());
    s.execute(Command::SetCoach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    }))
    .unwrap();
    respond(&mut s, G::Figures).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.rule, rule);
    assert_eq!(g.early_condition, condition);
    assert!(g.figures_requested);
    if let Some(h) = g.figures {
        assert_ne!(h.status, skat_coach::Status::Complete);
    }
}

#[test]
fn grand_win_early_exclusions_preserve_existing_app_lessons_and_all_legal_actions() {
    use fixture::{card as c, hand_game, swap};
    let d = early_oracle::deals();
    let no = swap(d[4], c("CT"), c("C9"));
    let one = swap(swap(d[0], c("C8"), c("H7")), c("CT"), c("H8"));
    for (deal, decl, game, plays, human, expected) in [
        (
            d[4],
            Seat::Hinterhand,
            hand_game(GRAND, false),
            vec!["CK"],
            Seat::Mittelhand,
            Some(crate::GuidedBranch::UnderPartner),
        ),
        (
            no,
            Seat::Mittelhand,
            hand_game(GRAND, false),
            vec!["CK"],
            Seat::Mittelhand,
            None,
        ),
        (
            one,
            Seat::Mittelhand,
            hand_game(GRAND, false),
            vec!["CK"],
            Seat::Mittelhand,
            None,
        ),
        (
            d[4],
            Seat::Vorhand,
            hand_game(GRAND, false),
            vec!["CK", "C7"],
            Seat::Hinterhand,
            Some(crate::GuidedBranch::WinLast),
        ),
        (
            d[4],
            Seat::Mittelhand,
            hand_game(NULL, false),
            vec!["CK"],
            Seat::Mittelhand,
            Some(crate::GuidedBranch::Null),
        ),
        (
            d[0],
            Seat::Mittelhand,
            hand_game(GRAND, true),
            vec!["CK"],
            Seat::Mittelhand,
            None,
        ),
        (
            d[0],
            Seat::Mittelhand,
            hand_game(HERZ, false),
            vec!["CK"],
            Seat::Mittelhand,
            Some(crate::GuidedBranch::WinEarly),
        ),
    ] {
        let actions = fixture::contract_actions(deal, decl, game, &plays);
        let mut s = grand_session(human, deal, false, &actions);
        let legal = s.view().legal_actions;
        s.execute(Command::StartGuided).unwrap();
        assert_eq!(
            s.guided().unwrap().step,
            if expected.is_some() {
                S::Orient
            } else {
                S::Unsupported
            }
        );
        if let Some(branch) = expected {
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(s.guided().unwrap().rule.unwrap().branch, branch);
            assert_eq!(
                s.guided().unwrap().early_condition.is_some(),
                branch == crate::GuidedBranch::WinEarly
            );
        }
        for action in legal {
            s.clone().execute(Command::Act(action)).unwrap();
        }
    }
}

#[test]
fn grand_feed_frozen_ordinary_session_every_legal_card_and_stale_return() {
    for n in [8, 10, 11, 13, 14, 17, 18, 25] {
        let mut s = Session::new(SessionConfig {
            seed: 15,
            series: SeriesLength::Fixed(3),
            opponents: Opponents::Levels([Level::Easy; 2]),
            ..SessionConfig::default()
        });
        for a in [
            Action::Pass,
            Action::Play(Card::from_index(2).unwrap()),
            Action::Play(Card::from_index(5).unwrap()),
        ] {
            s.run_opponents();
            s.execute(Command::Act(a)).unwrap();
        }
        s.run_opponents();
        let v = s.view();
        assert_eq!(
            (v.game_number, v.tricks_played, v.players[0].seat),
            (1, 2, Seat::Vorhand)
        );
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        assert!(s.guided().unwrap().rule.is_none());
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
        respond(&mut s, G::Answer(A::Seat(None))).unwrap();
        assert_eq!(
            s.guided().unwrap().task,
            Some(crate::GuidedBranch::FeedPartner)
        );
        respond(&mut s, G::Answer(A::Card(Card::from_index(n).unwrap()))).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Act);
        assert_eq!(g.rule.unwrap().card.index(), 11);
        assert_eq!(s.view().legal_actions, v.legal_actions);
        assert_eq!(s.view().hand, v.hand);
        assert_eq!(s.view().current_trick, v.current_trick);
        s.execute(Command::Act(Action::Play(Card::from_index(n).unwrap())))
            .unwrap();
        assert_eq!(s.guided().unwrap().step, S::Observe);
        assert!(s.guided().unwrap().observation.is_none());
        respond(&mut s, G::Answer(A::Void(true))).unwrap();
        let o = s.guided().unwrap().observation.unwrap();
        assert_eq!(o.winner, Some(Seat::Hinterhand));
        assert!(o.proves_void);
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
}

#[allow(dead_code)]
#[path = "../../skat-coach/tests/support/feed_partner.rs"]
mod feed_fixture;

#[test]
fn grand_feed_fixture_only_overtake_names_human_winner_and_preserves_all_candidates() {
    for c in feed_fixture::cases().into_iter().filter(|c| c.supported) {
        for (&n, &winner) in c.legal.iter().zip(c.completed) {
            let mut s = Session::new(SessionConfig::default());
            s.dealer = PlayerId(0); // human is HH in these explicitly labelled fixtures
            s.game = c.position();
            s.execute(Command::StartGuided).unwrap();
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().rule.unwrap().card.index(),
                c.choice.unwrap()
            );
            s.execute(Command::Act(Action::Play(feed_fixture::card(n))))
                .unwrap();
            respond(&mut s, G::Reveal).unwrap();
            let o = s.guided().unwrap().observation.unwrap();
            assert_eq!(
                o.winner,
                Some(Seat::from_index(winner)),
                "{} card {n}",
                c.id
            );
        }
    }
}

#[allow(dead_code)]
#[path = "../../skat-coach/tests/support/null_last.rs"]
mod null_fixture;

fn null_ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 6,
        series: SeriesLength::Fixed(3),
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    // QA1458 route.json, unchanged ordinary Session; no injected state.
    for (start, cards) in [
        (
            vec![Action::Hold, Action::Pass],
            vec![2, 24, 19, 21, 3, 8, 5, 27, 6, 29],
        ),
        (
            vec![Action::Pass],
            vec![0, 1, 26, 10, 16, 27, 2, 13, 18, 23],
        ),
    ] {
        for a in start.into_iter().chain(
            cards
                .into_iter()
                .map(|n| Action::Play(Card::from_index(n).unwrap())),
        ) {
            s.run_opponents();
            s.execute(Command::Act(a)).unwrap();
        }
        s.run_opponents();
        s.execute(Command::NextGame).unwrap();
    }
    for a in [
        Action::Bid(18),
        Action::Hand,
        Action::Announce(skat_core::Announcement::plain(skat_core::GameType::Null)),
        Action::Play(Card::from_index(1).unwrap()),
    ] {
        s.run_opponents();
        s.execute(Command::Act(a)).unwrap();
    }
    s.run_opponents();
    s
}

#[test]
fn null_last_ordinary_every_card_consent_retry_return_and_stale_reset() {
    let base = null_ordinary();
    let v = base.view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.players[0].seat),
        (3, 1, Seat::Mittelhand)
    );
    assert_eq!(v.declarer, Some(PlayerId::HUMAN));
    assert_eq!(
        v.legal_actions,
        [16, 18, 20].map(|n| Action::Play(Card::from_index(n).unwrap()))
    );
    for action in v.legal_actions.clone() {
        let mut s = base.clone();
        assert!(s.guided().is_none());
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Orient);
        assert!(g.rule.is_none() && g.memory.is_none() && g.figures.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Mittelhand)))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        assert!(s.guided().unwrap().rule.is_none());
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
        respond(&mut s, G::Answer(A::Seat(None))).unwrap();
        assert_eq!(s.guided().unwrap().task, Some(crate::GuidedBranch::Null));
        let Action::Play(c) = action else {
            panic!("card")
        };
        respond(&mut s, G::Answer(A::Card(c))).unwrap();
        assert_eq!(s.guided().unwrap().rule.unwrap().card.index(), 20);
        respond(&mut s, G::Memory(true)).unwrap();
        assert_eq!(s.guided().unwrap().memory.unwrap().trumpf, None);
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.view(), v);
        s.execute(Command::StartGuided).unwrap();
        assert_ne!(s.guided().unwrap().id, id);
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::Act(action)).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.step, S::Observe);
        assert!(g.observation.is_none() && g.memory.is_none() && g.figures.is_none());
        respond(&mut s, G::Answer(A::Void(false))).unwrap();
        let o = s.guided().unwrap().observation.unwrap();
        assert_eq!(o.winner, Some(Seat::Vorhand));
        assert_eq!(
            s.view()
                .last_trick
                .unwrap()
                .cards
                .into_iter()
                .map(|p| p.card.index())
                .collect::<Vec<_>>(),
            [17, 21, c.index()]
        );
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        s.execute(Command::SetCoach(CoachSettings {
            level: crate::CoachLevel::Off,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
    let mut s = base;
    // Reset boundary after the frozen Fixed3 route: allow a following game.
    s.config.series = SeriesLength::Free;
    s.execute(Command::StartGuided).unwrap();
    let id = s.guided().unwrap().id;
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(
        s.execute(Command::Guided(id, G::Reveal)),
        Err(AppError::StaleGuided)
    );
}

#[test]
fn null_last_fixtures_every_candidate_actual_winner_and_hidden_views() {
    for c in null_fixture::cases().into_iter().filter(|c| c.supported) {
        for (&n, &w) in c.legal.iter().zip(c.completed) {
            let mut s = Session::new(SessionConfig::default());
            s.dealer = PlayerId(0); // Explicit fixture: human HH, not ordinary route.
            s.game = c.position();
            let before = s.view();
            s.execute(Command::StartGuided).unwrap();
            respond(
                &mut s,
                G::Answer(A::Seat(Some(null_fixture::seat(c.winner.unwrap())))),
            )
            .unwrap();
            respond(&mut s, G::Answer(A::Seat(None))).unwrap();
            respond(&mut s, G::Answer(A::Card(null_fixture::card(n)))).unwrap();
            let loses = c.losers.contains(&n);
            assert_eq!(s.guided().unwrap().correct, Some(loses));
            if !loses {
                // Winning alternatives are wrong for the task, still playable.
                assert!(s.guided().unwrap().rule.is_none());
                respond(&mut s, G::Retry).unwrap();
                respond(&mut s, G::Reveal).unwrap();
            }
            assert_eq!(s.view().legal_actions, before.legal_actions);
            assert_eq!(
                s.guided().unwrap().rule.unwrap().card,
                null_fixture::card(c.choice.unwrap())
            );
            s.execute(Command::Act(Action::Play(null_fixture::card(n))))
                .unwrap();
            assert_eq!(s.guided().unwrap().step, S::Observe);
            assert!(s.guided().unwrap().observation.is_none());
            respond(&mut s, G::Reveal).unwrap();
            let o = s.guided().unwrap().observation.unwrap();
            assert_eq!(o.winner, Some(null_fixture::seat(w)), "{} {n}", c.id);
            assert_eq!(
                o.proves_void,
                null_fixture::card(n).suit() != before.current_trick[0].card.suit()
            );
        }
    }
    let cases = null_fixture::cases();
    for (name, a, b) in [
        ("mixed-ten", "VH H7", "Skat DK"),
        ("mixed-ten", "VH H7", "MH S7"),
        ("pickup-mixed", "VH H7", "MH S7"),
    ] {
        let c = cases.iter().find(|c| c.id == name).unwrap();
        let mut x = Session::new(SessionConfig::default());
        x.dealer = PlayerId(0);
        x.game = c.position();
        let mut y = x.clone();
        y.game = c.swapped(a, b);
        assert_eq!(x.view(), y.view());
        x.execute(Command::StartGuided).unwrap();
        y.execute(Command::StartGuided).unwrap();
        for action in [G::Reveal, G::Memory(true)] {
            respond(&mut x, action).unwrap();
            respond(&mut y, action).unwrap();
            let gx = x.guided().unwrap();
            let mut gy = y.guided().unwrap();
            assert!(gx.rule.is_some());
            gy.id = gx.id;
            assert_eq!(gx, gy);
        }
    }
}

/// Serialize ACTUAL fixture dialogue views for frontend component rendering.
/// These are not ordinary-route coverage and not the domain oracle: the tests
/// above compare every winner/choice against QA1458's handwritten expectations.
#[test]
fn null_last_fixture_widget_models_match_checked_session_outputs() {
    let mut frames = Vec::new();
    for c in null_fixture::cases().into_iter().filter(|c| c.supported) {
        let mut s = Session::new(SessionConfig::default());
        s.dealer = PlayerId(0);
        s.game = c.position();
        s.execute(Command::StartGuided).unwrap();
        respond(&mut s, G::Reveal).unwrap();
        let mut save = |stage: &str, s: &Session| {
            if !matches!(
                (c.id, stage),
                ("mixed-ten", "rule" | "observe" | "complete")
                    | ("void-last-tie", "rule" | "complete")
                    | ("pickup-mixed", "memory")
                    | ("void-changed-queen" | "printed-bube", "rule")
            ) {
                return;
            }
            let mut g = serde_json::to_value(s.guided().unwrap()).unwrap();
            // Entry identity is deliberately inert in a component fixture;
            // lifecycle/identity is tested on real sessions, never this replay.
            g["id"]["lineage"] = 0.into();
            g["id"]["generation"] = 0.into();
            frames.push(
                serde_json::json!({"case": c.id, "stage": stage, "table":s.view(), "guided":g}),
            );
        };
        save("rule", &s);
        respond(&mut s, G::Memory(true)).unwrap();
        save("memory", &s);
        // Mixed positions deliberately take the trick with a winning alternative;
        // void fixtures discard. All choices separately exercised exhaustively.
        let n = if c.id == "mixed-ten" || c.id == "pickup-mixed" || c.id == "printed-bube" {
            c.legal[c.legal.len() - 1]
        } else {
            c.choice.unwrap()
        };
        s.execute(Command::Act(Action::Play(null_fixture::card(n))))
            .unwrap();
        save("observe", &s);
        respond(&mut s, G::Reveal).unwrap();
        save("complete", &s);
    }
    let value = serde_json::to_string_pretty(&frames).unwrap() + "\n";
    if let Ok(path) = std::env::var("SKAT_NULL_WIDGET_FIXTURES_OUT") {
        std::fs::write(path, &value).unwrap();
    } else {
        assert_eq!(
            value,
            include_str!("../../gui-egui/tests/fixtures/null-last.json")
        );
    }
}

#[test]
fn null_last_separate_incomplete_estimates_never_change_rule_or_leak_on_entry() {
    let mut s = null_ordinary();
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let rule = s.guided().unwrap().rule;
    let result = s
        .coach_job()
        .unwrap()
        .run(None)
        .guided_rank_first(Action::Play(null_fixture::card("H7")));
    assert_eq!(
        result.explanation().unwrap().options[0].action,
        Action::Play(null_fixture::card("H7"))
    );
    s.finish_coach(result).unwrap();
    s.execute(Command::Act(Action::Play(null_fixture::card("H7"))))
        .unwrap();
    respond(&mut s, G::Reveal).unwrap();
    s.execute(Command::SetCoach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    }))
    .unwrap();
    assert!(s.guided().unwrap().figures.is_none());
    respond(&mut s, G::Figures).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.rule, rule);
    assert!(g.figures_requested);
    if let Some(h) = g.figures {
        assert_ne!(h.status, skat_coach::Status::Complete);
    }
}

#[test]
fn null_last_real_exclusions_keep_unsupported_and_every_card_playable() {
    for c in null_fixture::cases().into_iter().filter(|c| !c.supported) {
        let mut s = Session::new(SessionConfig::default());
        let human = null_fixture::seat(c.actor);
        s.dealer = (0..3)
            .map(PlayerId)
            .find(|&d| PlayerId::HUMAN.seat(d) == human)
            .unwrap();
        s.game = c.position();
        let before = s.view();
        s.execute(Command::StartGuided).unwrap();
        assert_eq!(
            s.guided().unwrap().step,
            // These remain outside #165, but #169/#176 admit them.
            if matches!(c.id, "real-early" | "real-lead") {
                S::Orient
            } else {
                S::Unsupported
            },
            "{}",
            c.id
        );
        assert!(s.guided().unwrap().rule.is_none());
        for a in &before.legal_actions {
            s.clone().execute(Command::Act(*a)).unwrap();
        }
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.view(), before);
    }
}

#[test]
fn null_last_rematched_session_and_assistance_change_reject_stale_answers() {
    let mut s = null_ordinary();
    let mut other = null_ordinary();
    s.execute(Command::StartGuided).unwrap();
    other.execute(Command::StartGuided).unwrap();
    let old = s.guided().unwrap().id;
    assert_eq!(
        other.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
    s.execute(Command::SetAssist(crate::AssistSettings {
        counting_aid: true,
        ..crate::AssistSettings::default()
    }))
    .unwrap();
    assert!(s.guided().is_none());
    assert_eq!(
        s.execute(Command::Guided(old, G::Reveal)),
        Err(AppError::StaleGuided)
    );
}

#[allow(dead_code)]
#[path = "../../skat-analysis/tests/null_middle_policy.rs"]
mod middle_fixture;

fn middle_fixture_session(c: &middle_fixture::Case) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == c.seat)
        .unwrap();
    s.game = c.game();
    s.recording.deal = c.deal;
    s
}

fn middle_ordinary() -> Session {
    use middle_fixture::card as c;
    let mut s = Session::new(SessionConfig {
        seed: 1,
        series: SeriesLength::Fixed(3),
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    for a in [
        Action::Hold,
        Action::Hold,
        Action::Hold,
        Action::Hold,
        Action::Pass,
    ]
    .into_iter()
    .chain([31, 28, 8, 25, 26, 29, 23, 15, 14, 22].map(|n| Action::Play(c(n))))
    {
        s.run_opponents();
        s.execute(Command::Act(a)).unwrap();
    }
    s.run_opponents();
    s.execute(Command::NextGame).unwrap();
    for a in [
        Action::Bid(20),
        Action::Hand,
        Action::Announce(skat_core::Announcement::plain(NULL)),
        Action::Play(c(19)),
        Action::Play(c(24)),
        Action::Play(c(4)),
    ] {
        s.run_opponents();
        s.execute(Command::Act(a)).unwrap();
    }
    s.run_opponents();
    let v = s.view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.players[0].seat),
        (2, 3, Seat::Hinterhand)
    );
    assert_eq!(
        v.current_trick
            .iter()
            .map(|p| p.card.index())
            .collect::<Vec<_>>(),
        [12]
    );
    assert_eq!(v.legal_actions, [9, 10, 13, 14].map(|n| Action::Play(c(n))));
    s
}

#[test]
fn null_middle_every_candidate_and_continuation_observes_actual_third_winner() {
    for c in middle_fixture::cases().into_iter().filter(|c| c.admitted) {
        for &(candidate, third, winner) in &c.completions {
            let mut s = middle_fixture_session(&c);
            let before = s.view();
            s.execute(Command::StartGuided).unwrap();
            let id = s.guided().unwrap().id;
            assert_eq!(s.guided().unwrap().step, S::Orient);
            assert!(s.guided().unwrap().rule.is_none());
            respond(
                &mut s,
                G::Answer(A::Seat(c.expected.as_ref().unwrap().winner)),
            )
            .unwrap();
            respond(
                &mut s,
                G::Answer(A::Seat(Some(Seat::from_index(c.seat.index() + 1)))),
            )
            .unwrap();
            let chosen = middle_fixture::card(candidate);
            respond(&mut s, G::Answer(A::Card(chosen))).unwrap();
            let loses = c.expected.as_ref().unwrap().losers.contains(&chosen);
            assert_eq!(s.guided().unwrap().correct, Some(loses));
            if !loses {
                respond(&mut s, G::Retry).unwrap();
                respond(&mut s, G::Reveal).unwrap();
            }
            assert_eq!(s.guided().unwrap().rule, c.expected);
            assert!(s.guided().unwrap().memory.is_none());
            respond(&mut s, G::Memory(true)).unwrap();
            assert_eq!(
                s.guided().unwrap().memory.unwrap().skat,
                before.own_skat.map(|cs| cs.into_iter().collect())
            );
            s.execute(Command::Act(Action::Play(chosen))).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Waiting);
            s.apply(Action::Play(middle_fixture::card(third))).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.id, id);
            assert_eq!(g.step, S::Observe);
            assert!(g.observation.is_none() && g.memory.is_none());
            assert_eq!(
                g.observed_source,
                Some(skat_coach::Source::Trick {
                    index: before.tricks_played,
                    seat: Seat::from_index(c.seat.index() + 1),
                    led: before.current_trick[0].card,
                    card: middle_fixture::card(third),
                })
            );
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(winner)
            );
            respond(&mut s, G::Return).unwrap();
            assert!(s.guided().is_none());
        }
    }
}

#[test]
fn null_middle_full_dialogue_hidden_substitutions_even_in_god_mode() {
    let cs = middle_fixture::cases();
    for (a, b) in [
        ("mixed-hand", "privacy-opponent-opponent"),
        ("mixed-hand", "privacy-opponent-unknown-Hand-Skat"),
        ("mixed-pickup", "privacy-pickup-opponents"),
    ] {
        for god in [false, true] {
            let build = |id| {
                let mut s = middle_fixture_session(cs.iter().find(|c| c.id == id).unwrap());
                if god {
                    s.config.opponents = Opponents::GodMode;
                }
                s
            };
            let mut x = build(a);
            let mut y = build(b);
            x.execute(Command::StartGuided).unwrap();
            y.execute(Command::StartGuided).unwrap();
            for action in [
                None,
                Some(G::Answer(A::Seat(Some(Seat::Mittelhand)))),
                Some(G::Retry),
                Some(G::Answer(A::Seat(Some(Seat::Vorhand)))),
                Some(G::Answer(A::Seat(Some(Seat::Hinterhand)))),
                Some(G::Answer(A::Card(middle_fixture::card(0)))),
                Some(G::Memory(true)),
                Some(G::Memory(false)),
            ] {
                if let Some(action) = action {
                    respond(&mut x, action).unwrap();
                    respond(&mut y, action).unwrap();
                }
                let gx = x.guided().unwrap();
                let mut gy = y.guided().unwrap();
                gy.id = gx.id;
                assert_eq!(gx, gy, "{a}/{b}/god={god}");
            }
            assert!(x.guided().unwrap().rule.is_some());
        }
    }
}

#[test]
fn null_middle_ordinary_recording_all_alternatives_stale_return_and_no_early_answer() {
    for n in [9, 10, 13, 14] {
        let mut s = middle_ordinary();
        let recording = s.export_current_recording_privileged();
        let before = s.view();
        assert!(s.guided().is_none());
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.view(), before);
        assert_eq!(s.export_current_recording_privileged(), recording);
        s.execute(Command::StartGuided).unwrap();
        assert_ne!(s.guided().unwrap().id, id);
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(s.guided().unwrap().rule.unwrap().card.index(), 10);
        assert_eq!(s.view().legal_actions, before.legal_actions);
        s.execute(Command::Act(Action::Play(middle_fixture::card(n))))
            .unwrap();
        assert_eq!(s.guided().unwrap().step, S::Waiting);
        s.run_opponents();
        assert_eq!(s.guided().unwrap().step, S::Observe);
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(Seat::Vorhand)
        );
        assert_eq!(
            s.view()
                .last_trick
                .unwrap()
                .cards
                .into_iter()
                .map(|p| p.card.index())
                .collect::<Vec<_>>(),
            [12, n, 15]
        );
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        s.execute(Command::SetCoach(CoachSettings {
            level: crate::CoachLevel::Off,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
    }
}

/// Real app-emitted boundary models, not an ordinary route or an oracle.
#[test]
fn null_middle_fixture_widget_models_match_checked_sessions() {
    let mut frames = Vec::new();
    for c in middle_fixture::cases().into_iter().filter(|c| {
        matches!(
            c.id,
            "mixed-hand"
                | "mixed-pickup"
                | "void-last-exact-tie"
                | "changed-leading-rank"
                | "all-loser-follow"
                | "no-loser"
                | "single-legal"
                | "ouvert"
        )
    }) {
        let mut s = middle_fixture_session(&c);
        s.execute(Command::StartGuided).unwrap();
        let mut save = |stage: &str, s: &Session| {
            let mut g = serde_json::to_value(s.guided().unwrap()).unwrap();
            g["id"]["lineage"] = 0.into();
            g["id"]["generation"] = 0.into();
            frames.push(serde_json::json!({"case":c.id,"stage":stage,"table":s.view(),"guided":g}));
        };
        if !c.admitted {
            save("unsupported", &s);
            continue;
        }
        respond(&mut s, G::Reveal).unwrap();
        save("rule", &s);
        if c.id == "mixed-pickup" {
            respond(&mut s, G::Memory(true)).unwrap();
            save("memory", &s);
        }
        if c.id == "mixed-hand" || c.id == "void-last-exact-tie" {
            let (candidate, third, winner) = if c.id == "mixed-hand" {
                (6, 2, Seat::Mittelhand)
            } else {
                (31, 6, Seat::Hinterhand)
            };
            // Candidate6 overtakes; third2 cannot catch it. Void31 loses,
            // but third6 changes the winning defender from VH to HH.
            s.execute(Command::Act(Action::Play(middle_fixture::card(candidate))))
                .unwrap();
            s.apply(Action::Play(middle_fixture::card(third))).unwrap();
            save("observe", &s);
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(winner)
            );
            save("complete", &s);
        }
    }
    let value = serde_json::to_string_pretty(&frames).unwrap() + "\n";
    if let Ok(path) = std::env::var("SKAT_NULL_MIDDLE_WIDGET_FIXTURES_OUT") {
        std::fs::write(path, value).unwrap();
    } else {
        assert_eq!(
            value,
            include_str!("../../gui-egui/tests/fixtures/null-middle.json")
        );
    }
}

#[test]
fn null_middle_fresh_ordinary_replay_evidence() {
    let mut s = middle_ordinary();
    let before = s.export_current_recording_privileged().unwrap();
    let p = before.reconstruct_privileged(before.actions.len()).unwrap();
    assert_eq!(
        p.state.seats[Seat::Hinterhand.index()],
        s.game.view(Seat::Hinterhand)
    );
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    s.execute(Command::Act(Action::Play(middle_fixture::card(9))))
        .unwrap();
    s.execute(Command::AdvanceOpponent).unwrap();
    let after = s.export_current_recording_privileged().unwrap();
    let p = after.reconstruct_privileged(after.actions.len()).unwrap();
    assert_eq!(
        p.state.seats[Seat::Hinterhand.index()],
        s.game.view(Seat::Hinterhand)
    );
    assert_eq!(s.guided().unwrap().step, S::Observe);
    if let Ok(path) = std::env::var("SKAT_NULL_MIDDLE_ROUTE_OUT") {
        std::fs::write(path,serde_json::to_string_pretty(&serde_json::json!({
            "entry":"Session::new","injection":false,"recording_before":before,"recording_after":after,
            "fresh_replay_equal":true,"actual_trick":s.view().last_trick,"guided":s.guided(),
        })).unwrap()).unwrap();
    }
}

#[test]
fn null_middle_early_automatic_off_memory_estimates_and_reset_keep_consent() {
    for level in crate::CoachLevel::ALL {
        let mut s = middle_ordinary();
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(
            s.guided().is_none(),
            "ordinary/automatic help cannot enter dialogue"
        );
        if level == crate::CoachLevel::Off {
            assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
            continue;
        }
        s.set_coach_budget(Budget {
            samples: 1,
            nodes_per_task: 1,
        });
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        assert!(s.guided().unwrap().rule.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        respond(&mut s, G::Reveal).unwrap();
        let rule = s.guided().unwrap().rule;
        let result = s
            .coach_job()
            .unwrap()
            .run(None)
            .guided_rank_first(Action::Play(middle_fixture::card(14)));
        s.finish_coach(result).unwrap();
        s.execute(Command::Act(Action::Play(middle_fixture::card(9))))
            .unwrap();
        s.execute(Command::AdvanceOpponent).unwrap();
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::SetCoach(CoachSettings {
            level,
            show_figures: true,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().unwrap().figures.is_none());
        respond(&mut s, G::Figures).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.id, id);
        assert_eq!(g.rule, rule);
        assert!(g.figures_requested);
        if let Some(h) = g.figures {
            assert_ne!(h.status, skat_coach::Status::Complete);
        }
        s.execute(Command::SetAssist(crate::AssistSettings {
            counting_aid: true,
            ..crate::AssistSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
    let mut s = middle_ordinary();
    s.execute(Command::StartGuided).unwrap();
    let id = s.guided().unwrap().id;
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(
        s.execute(Command::Guided(id, G::Reveal)),
        Err(AppError::StaleGuided)
    );
}

#[allow(dead_code)]
#[path = "../../skat-analysis/tests/lead_master_policy.rs"]
mod lead_fixture;

fn lead_fixture_session(c: &lead_fixture::Case) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == c.seat)
        .unwrap();
    s.game = c.game();
    s.recording.deal = c.deal;
    s
}
fn lead_ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 3,
        series: SeriesLength::Fixed(1),
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        display: TableDisplay::Closed,
        ..SessionConfig::default()
    });
    s.run_opponents();
    s.execute(Command::Act(Action::Pass)).unwrap();
    s.run_opponents();
    let v = s.view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.dealer),
        (1, 0, PlayerId(2))
    );
    assert_eq!(
        v.legal_actions,
        [5, 6, 8, 13, 14, 22, 23, 27, 28, 29].map(|n| Action::Play(lead_fixture::card(n)))
    );
    s
}

#[test]
fn lead_master_every_legal_completion_waits_for_both_replies_and_individual_winner() {
    for c in lead_fixture::cases().into_iter().filter(|c| c.admitted()) {
        for (_, cards, winner) in lead_fixture::tricks(c.next) {
            let mut s = lead_fixture_session(&c);
            s.execute(Command::StartGuided).unwrap();
            let id = s.guided().unwrap().id;
            assert_eq!(s.guided().unwrap().step, S::Orient);
            assert!(s.guided().unwrap().rule.is_none());
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(s.guided().unwrap().rule, Some(c.expected.clone()));
            s.execute(Command::Act(Action::Play(cards[0]))).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Waiting);
            s.apply(Action::Play(cards[1])).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Waiting);
            assert_eq!(g.lead_replies.len(), 1);
            assert!(g.observation.is_none());
            s.apply(Action::Play(cards[2])).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.id, id);
            assert_eq!(g.step, S::Observe);
            assert_eq!(g.lead_replies.len(), 2);
            assert!(g.observation.is_none());
            assert_eq!(
                g.observed_source,
                Some(skat_coach::Source::Trick {
                    index: s.game.tricks().len() as u8 - 1,
                    seat: Seat::from_index(c.seat.index() + 2),
                    led: cards[0],
                    card: cards[2]
                })
            );
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(winner)
            );
        }
    }
}

#[test]
fn lead_master_full_non_absent_dialogue_privacy_including_god_mode() {
    let cs = lead_fixture::cases();
    for other in ["02", "01-hidden-swap"] {
        for god in [false, true] {
            let build = |id| {
                let mut s = lead_fixture_session(cs.iter().find(|c| c.id == id).unwrap());
                if god {
                    s.config.opponents = Opponents::GodMode;
                }
                s
            };
            let mut x = build("01");
            let mut y = build(other);
            x.execute(Command::StartGuided).unwrap();
            y.execute(Command::StartGuided).unwrap();
            for a in [
                None,
                Some(G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: true,
                })),
                Some(G::Retry),
                Some(G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: false,
                })),
                Some(G::Answer(A::Card(lead_fixture::card(3)))),
                // CA is a valid master, but loses the exact last tie to SA.
                Some(G::Answer(A::Card(lead_fixture::card(7)))),
                Some(G::Retry),
                Some(G::Answer(A::Card(lead_fixture::card(15)))),
                Some(G::Memory(true)),
                Some(G::Memory(false)),
            ] {
                if let Some(a) = a {
                    respond(&mut x, a).unwrap();
                    respond(&mut y, a).unwrap();
                }
                if a == Some(G::Answer(A::Card(lead_fixture::card(7)))) {
                    let g = x.guided().unwrap();
                    assert_eq!(g.step, S::Narrow);
                    assert_eq!(g.correct, Some(false));
                    assert!(g.rule.is_none());
                }
                let gx = x.guided().unwrap();
                let mut gy = y.guided().unwrap();
                gy.id = gx.id;
                assert_eq!(gx, gy, "{other}/god={god}");
            }
            assert!(x.guided().unwrap().rule.is_some());
            // Common legal public continuation in BOTH concealed deals:
            // SA/S8/S10. Compare the entire non-absent dialogue through
            // waiting, each reply, observation and completed feedback too.
            for n in [15, 9, 11] {
                x.apply(Action::Play(lead_fixture::card(n))).unwrap();
                y.apply(Action::Play(lead_fixture::card(n))).unwrap();
                assert_eq!(x.game.view(Seat::Vorhand), y.game.view(Seat::Vorhand));
                let gx = x.guided().unwrap();
                let mut gy = y.guided().unwrap();
                gy.id = gx.id;
                assert_eq!(gx, gy, "{other}/god={god}/public-card={n}");
            }
            assert_eq!(x.guided().unwrap().step, S::Observe);
            for a in [
                G::Answer(A::Void(true)),
                G::Retry,
                G::Answer(A::Void(false)),
            ] {
                respond(&mut x, a).unwrap();
                respond(&mut y, a).unwrap();
                let gx = x.guided().unwrap();
                let mut gy = y.guided().unwrap();
                gy.id = gx.id;
                assert_eq!(gx, gy, "{other}/god={god}/feedback={a:?}");
            }
            assert_eq!(x.guided().unwrap().step, S::Complete);
            assert!(x.guided().unwrap().observation.is_some());
        }
    }
}

#[test]
fn lead_master_ordinary_all_legal_choices_consent_stale_reentry_replay_and_automation() {
    for n in [5, 6, 8, 13, 14, 22, 23, 27, 28, 29] {
        let mut s = lead_ordinary();
        let before = s.view();
        let recording = s.export_current_recording_privileged();
        assert!(s.guided().is_none());
        s.execute(Command::StartGuided).unwrap();
        let old = s.guided().unwrap().id;
        assert!(s.guided().unwrap().rule.is_none());
        assert!(s.guided().unwrap().memory.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.view(), before);
        assert_eq!(s.export_current_recording_privileged(), recording);
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        assert_ne!(old, id);
        assert_eq!(
            s.execute(Command::Guided(old, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(
            &mut s,
            G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: true,
            }),
        )
        .unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        assert_eq!(
            s.guided().unwrap().evidence,
            Some(skat_coach::Source::Lead {
                index: 0,
                seat: Seat::Vorhand
            })
        );
        respond(&mut s, G::Retry).unwrap();
        respond(
            &mut s,
            G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: false,
            }),
        )
        .unwrap();
        assert_eq!(s.guided().unwrap().step, S::Check);
        respond(&mut s, G::Answer(A::Card(lead_fixture::card(5)))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Card(lead_fixture::card(23)))).unwrap();
        assert!(s.guided().unwrap().rule.is_none());
        assert_eq!(s.guided().unwrap().step, S::Narrow);
        respond(&mut s, G::Answer(A::Card(lead_fixture::card(23)))).unwrap();
        assert_eq!(s.guided().unwrap().rule.unwrap().card.index(), 23);
        respond(&mut s, G::Memory(true)).unwrap();
        assert!(s.guided().unwrap().memory.unwrap().skat.is_none());
        s.execute(Command::Act(Action::Play(lead_fixture::card(n))))
            .unwrap();
        s.run_opponents();
        let g = s.guided().unwrap();
        assert_eq!(g.id, id);
        assert_eq!(g.step, S::Observe);
        assert_eq!(g.lead_replies.len(), 2);
        assert!(g.memory.is_none() && g.observation.is_none());
        respond(&mut s, G::Reveal).unwrap();
        let t = s.game.tricks()[0];
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(t.winner)
        );
        if n == 5 {
            assert_eq!(t.cards.map(|c| c.index()), [5, 0, 3]);
            assert_eq!(t.winner, Seat::Hinterhand);
        }
        let frozen = s.guided().unwrap();
        s.run_opponents();
        assert_eq!(s.guided().unwrap(), frozen);
        let r = s.export_current_recording_privileged().unwrap();
        let pos = r.reconstruct_privileged(r.actions.len()).unwrap();
        assert_eq!(pos.state.seats[0], s.game.view(Seat::Vorhand));
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}

#[test]
fn lead_master_off_automatic_unaided_estimates_origin_and_reset() {
    for level in crate::CoachLevel::ALL {
        let mut s = lead_ordinary();
        s.execute(Command::SetCoach(CoachSettings {
            level,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        if level == crate::CoachLevel::Off {
            assert_eq!(s.execute(Command::StartGuided), Err(AppError::CoachOff));
            continue;
        }
        s.set_coach_budget(Budget {
            samples: 1,
            nodes_per_task: 1,
        });
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        assert!(s.guided().unwrap().rule.is_none());
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        respond(&mut s, G::Reveal).unwrap();
        let rule = s.guided().unwrap().rule;
        let result = s
            .coach_job()
            .unwrap()
            .run(None)
            .guided_rank_first(Action::Play(lead_fixture::card(5)));
        s.finish_coach(result).unwrap();
        s.execute(Command::Act(Action::Play(lead_fixture::card(5))))
            .unwrap();
        s.run_opponents();
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::SetCoach(CoachSettings {
            level,
            show_figures: true,
            ..CoachSettings::default()
        }))
        .unwrap();
        assert!(s.guided().unwrap().figures.is_none());
        respond(&mut s, G::Figures).unwrap();
        let g = s.guided().unwrap();
        assert_eq!(g.id, id);
        assert_eq!(g.rule, rule);
        assert!(g.figures_requested);
        if let Some(h) = g.figures {
            assert_eq!(
                h.situation.expect("original-decision estimates").options[0],
                Action::Play(lead_fixture::card(5))
            );
        }
        s.execute(Command::SetAssist(crate::AssistSettings {
            counting_aid: true,
            ..crate::AssistSettings::default()
        }))
        .unwrap();
        assert!(s.guided().is_none());
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
    }
}

#[test]
fn lead_master_fixture_widget_models_match_checked_sessions() {
    let mut frames = Vec::new();
    for c in lead_fixture::cases().into_iter().filter(|c| {
        matches!(
            c.id,
            "01" | "02"
                | "03"
                | "04"
                | "09"
                | "11"
                | "12"
                | "13-postfreeze-actual-DrawTrumpf"
                | "14-postfreeze-known-discard-higher"
        )
    }) {
        let mut s = lead_fixture_session(&c);
        s.execute(Command::StartGuided).unwrap();
        let mut save = |stage: &str, s: &Session| {
            let mut g = serde_json::to_value(s.guided().unwrap()).unwrap();
            g["id"]["lineage"] = 0.into();
            g["id"]["generation"] = 0.into();
            frames.push(serde_json::json!({"case":c.id,"stage":stage,"table":s.view(),"guided":g}));
        };
        if !c.admitted() {
            save("unsupported", &s);
            continue;
        }
        respond(&mut s, G::Reveal).unwrap();
        if c.id == "03" {
            respond(&mut s, G::Memory(true)).unwrap();
            save("memory", &s);
        } else {
            save("rule", &s);
        }
        if c.id == "09" || c.id == "02" {
            // Accepted case02: C10/C8/CJ loses to HH. Case09 owns all
            // Buben: its side master wins C10/C7/H9, with no opposing Trumpf.
            // Preserve BOTH observations; all3324 continuations remain tested.
            let expected_cards = if c.id == "02" { [3, 1, 4] } else { [3, 0, 18] };
            let expected_winner = if c.id == "02" {
                Seat::Hinterhand
            } else {
                Seat::Vorhand
            };
            let (_, cards, winner) = lead_fixture::tricks(c.next)
                .into_iter()
                .find(|(_, cards, _)| cards.map(|c| c.index()) == expected_cards)
                .unwrap();
            assert_eq!(winner, expected_winner);
            s.execute(Command::Act(Action::Play(cards[0]))).unwrap();
            s.apply(Action::Play(cards[1])).unwrap();
            s.apply(Action::Play(cards[2])).unwrap();
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(winner)
            );
            save(
                if c.id == "02" {
                    "trumped"
                } else {
                    "all-buben-own-completed"
                },
                &s,
            );
        }
    }
    let value = serde_json::to_string_pretty(&frames).unwrap() + "\n";
    if let Ok(path) = std::env::var("SKAT_LEAD_MASTER_WIDGET_FIXTURES_OUT") {
        std::fs::write(path, value).unwrap();
    } else {
        assert_eq!(
            value,
            include_str!("../../gui-egui/tests/fixtures/lead-master.json")
        );
    }
}

#[path = "guided_null_lead_tests.rs"]
mod null_lead_tests;

#[path = "guided_suit_win_early_tests.rs"]
mod suit_win_early_tests;

#[path = "guided_suit_feed_partner_tests.rs"]
mod suit_feed_partner;

#[path = "guided_early_feed_tests.rs"]
mod early_feed;
