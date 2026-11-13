// SPDX-License-Identifier: EUPL-1.2

use super::*;
use crate::{GuidedAction as G, GuidedAnswer as A, GuidedStep as S};
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
fn hidden_deal_substitution_keeps_prompts_and_rule_but_changes_only_actual_observation() {
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
    }
}

#[test]
fn guidance_events_keep_entry_identity_sequence_and_assistance_without_storage() {
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
    let mut s = Session::new(SessionConfig::default());
    s.run_opponents();
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Unsupported);
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
