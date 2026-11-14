// SPDX-License-Identifier: EUPL-1.2

use super::*;
use crate::{GuidedAction as G, GuidedAnswer as A, GuidedStep as S};
use skat_core::KontraLevel;
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
        Action::Hand,
        Action::Announce(game),
    ] {
        s.apply(a).unwrap();
    }
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
    let mut s = kontra_session(
        Seat::Vorhand,
        kontra_deal(KONTRA_VH, KONTRA_HH, "D8 D9"),
        herz(),
    );
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
fn grand_null_ouvert_and_off_kontra_offers_get_honest_fallbacks() {
    use crate::GuidedKontraExclusion as X;
    let deal = kontra_deal(KONTRA_VH, KONTRA_HH, "D8 D9");
    let ouvert = skat_core::Announcement {
        ouvert: true,
        ..skat_core::Announcement::plain(skat_core::GameType::Null)
    };
    for (game, exclusion) in [
        (
            skat_core::Announcement::plain(skat_core::GameType::Grand),
            X::Grand,
        ),
        (
            skat_core::Announcement::plain(skat_core::GameType::Null),
            X::Null,
        ),
        (ouvert, X::OuvertRound),
    ] {
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
