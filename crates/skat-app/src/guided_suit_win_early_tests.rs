// SPDX-License-Identifier: EUPL-1.2
use super::*;
#[path = "../../skat-analysis/tests/common/suit_win_early.rs"]
mod cases;

fn session(c: &cases::Case) -> Session {
    grand_session(c.view.seat, c.deal, false, &c.history)
}
fn equal(a: &Session, b: &Session) {
    assert_eq!(
        a.game.view(PlayerId::HUMAN.seat(a.dealer)),
        b.game.view(PlayerId::HUMAN.seat(b.dealer))
    );
    let x = a.guided().unwrap();
    let mut y = b.guided().unwrap();
    y.id = x.id;

    assert_eq!(x, y);
}

#[test]
fn suit_win_early_all_cases_all_legal_completions_actual_third_and_individual_winner() {
    for c in cases::cases()
        .into_iter()
        .filter(|c| c.rule.branch == crate::GuidedBranch::WinEarly)
    {
        for (cards, winner, _) in &c.paths {
            let mut s = session(&c);
            let before = s.view();
            s.execute(Command::StartGuided).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Orient);
            assert!(g.rule.is_none() && g.memory.is_none() && g.early_condition.is_none());
            assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
            respond(&mut s, G::Answer(A::Seat(Some(c.view.seat)))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            respond(&mut s, G::Retry).unwrap();
            respond(&mut s, G::Answer(A::Seat(c.view.current_trick.leader))).unwrap();
            respond(&mut s, G::Answer(A::Seat(Some(c.view.seat.next())))).unwrap();
            for a in &c.view.legal_actions {
                let Action::Play(n) = a else { unreachable!() };
                if !c.rule.candidates.contains(n) {
                    respond(&mut s, G::Answer(A::Card(*n))).unwrap();
                    assert_eq!(s.guided().unwrap().correct, Some(false));
                    respond(&mut s, G::Retry).unwrap();
                }
            }
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(s.guided().unwrap().rule, Some(c.rule.clone()));
            assert!(s.guided().unwrap().memory.is_none());
            respond(&mut s, G::Memory(true)).unwrap();
            assert_eq!(s.guided().unwrap().memory.unwrap().own, c.view.hand);
            assert_eq!(s.view().legal_actions, before.legal_actions);
            s.execute(Command::Act(Action::Play(cards[1]))).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Waiting);
            s.apply(Action::Play(cards[2])).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Observe);
            assert!(s.guided().unwrap().observation.is_none());
            respond(&mut s, G::Reveal).unwrap();
            let o = s.guided().unwrap().observation.unwrap();
            assert_eq!(o.winner, Some(*winner));
            assert_eq!(
                o.source,
                skat_coach::Source::Trick {
                    index: c.view.tricks.len() as u8,
                    seat: c.view.seat.next(),
                    led: cards[0],
                    card: cards[2]
                }
            );
        }
    }
}

#[test]
fn suit_win_early_complete_dialogue_both_positive_hidden_pairs_and_god_mode() {
    let cs = cases::cases();
    for (a, b) in [(6, 7), (8, 9)] {
        for god in [false, true] {
            let mut x = session(&cs[a]);
            let mut y = session(&cs[b]);
            if god {
                y.config.opponents = Opponents::GodMode;
            }
            for s in [&mut x, &mut y] {
                s.execute(Command::StartGuided).unwrap();
            }
            equal(&x, &y);
            for action in [
                G::Answer(A::Seat(Some(cs[a].view.seat))),
                G::Retry,
                G::Answer(A::Seat(cs[a].view.current_trick.leader)),
                G::Answer(A::Seat(Some(cs[a].view.seat.next()))),
                G::Reveal,
                G::Memory(true),
                G::Memory(false),
            ] {
                for s in [&mut x, &mut y] {
                    respond(s, action).unwrap();
                }
                equal(&x, &y);
            }
            let path = cs[a]
                .paths
                .iter()
                .find(|p| cs[b].paths.contains(p))
                .unwrap();
            for s in [&mut x, &mut y] {
                s.execute(Command::Act(Action::Play(path.0[1]))).unwrap();
            }
            equal(&x, &y);
            for s in [&mut x, &mut y] {
                s.apply(Action::Play(path.0[2])).unwrap();
            }
            equal(&x, &y);
            for action in [
                G::Answer(A::Void(
                    cs[a].view.contract.unwrap().game.follow(path.0[0])
                        == cs[a].view.contract.unwrap().game.follow(path.0[2]),
                )),
                G::Retry,
                G::Reveal,
                G::Memory(true),
                G::Memory(false),
            ] {
                for s in [&mut x, &mut y] {
                    respond(s, action).unwrap();
                }
                equal(&x, &y);
            }
            assert_eq!(x.guided().unwrap().step, S::Complete);
        }
    }
}

fn ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 0,
        series: SeriesLength::Fixed(3),
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        display: TableDisplay::Closed,
    });
    let cmds: Vec<Command> = serde_json::from_str(include_str!(
        "../../gui-egui/tests/fixtures/suit-win-early-route.json"
    ))
    .unwrap();
    for cmd in cmds {
        s.execute(cmd).unwrap();
    }
    assert_eq!(s.game_seed(), 12896916468484187878);
    assert_eq!(s.view().game_number, 3);
    s
}

#[test]
fn suit_win_early_ordinary_consent_all_legal_replay_stale_return_reset_and_automated_replies() {
    let initial = ordinary();
    for action in initial.view().legal_actions {
        let mut s = initial.clone();
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.export_current_recording_privileged(), before);
        s.execute(Command::StartGuided).unwrap();
        assert_eq!(
            s.execute(Command::Guided(id, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::Act(action)).unwrap();
        s.run_opponents();
        let t = s.game.tricks().last().unwrap();
        let actual = *t;
        assert_eq!(s.guided().unwrap().step, S::Observe);
        respond(&mut s, G::Reveal).unwrap();
        let o = s.guided().unwrap().observation.unwrap();
        assert_eq!(o.winner, Some(actual.winner));
        if action == Action::Play(cases::card(24)) {
            assert_eq!(actual.cards.map(|c| c.index()), [7, 24, 1]);
            assert_eq!(actual.winner, Seat::Mittelhand);
        }
        if action == Action::Play(cases::card(9)) {
            assert_eq!(actual.cards.map(|c| c.index()), [7, 9, 5]);
            assert_eq!(actual.winner, Seat::Vorhand);
        }
        let recording = s.export_current_recording_privileged().unwrap();
        let replay = recording
            .reconstruct_privileged(recording.actions.len())
            .unwrap();
        assert_eq!(replay.state.seats[1], s.game.view(Seat::Mittelhand));
    }
    let cs = cases::cases();
    for level in [
        crate::CoachLevel::Off,
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
    ] {
        let mut s = session(&cs[6]);
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
    let mut s = ordinary();
    s.execute(Command::StartGuided).unwrap();
    let id = s.guided().unwrap().id;
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    // Last game ends the Fixed3 series; reset via a fresh ordinary session.
    let mut reset = ordinary();
    assert!(reset.guided().is_none());
    assert_eq!(
        reset.execute(Command::Guided(id, G::Reveal)),
        Err(AppError::StaleGuided)
    );
}

#[test]
fn suit_win_early_return_every_stage_and_estimates_stay_original_decision() {
    let cs = cases::cases();
    for stage in 0..5 {
        let mut s = session(&cs[6]);
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        if stage >= 1 {
            respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
        }
        if stage >= 2 {
            respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
        }
        if stage >= 3 {
            respond(&mut s, G::Reveal).unwrap();
        }
        if stage >= 4 {
            respond(&mut s, G::Memory(true)).unwrap();
        }
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
        assert_eq!(s.export_current_recording_privileged(), before);
    }
    let mut s = session(&cs[6]);
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let rule = s.guided().unwrap().rule;
    let id = s.guided().unwrap().id;
    let result = s
        .coach_job()
        .unwrap()
        .run(None)
        .guided_rank_first(Action::Play(cases::card(1)));
    s.finish_coach(result).unwrap();
    s.execute(Command::Act(Action::Play(cases::card(7))))
        .unwrap();
    s.apply(Action::Play(cases::card(4))).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    assert!(s.guided().unwrap().figures.is_none());
    s.execute(Command::SetCoach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    }))
    .unwrap();
    respond(&mut s, G::Figures).unwrap();
    let g = s.guided().unwrap();
    assert_eq!(g.id, id);
    assert_eq!(g.rule, rule);
    assert!(g.figures_requested);
    if let Some(h) = g.figures {
        assert_eq!(
            h.situation.unwrap().options[0],
            Action::Play(cases::card(1))
        );
    }
}

#[test]
fn suit_win_early_fixture_widget_models_match_checked_sessions() {
    let mut frames = Vec::new();
    for c in cases::cases().into_iter().skip(6) {
        let mut s = session(&c);
        s.execute(Command::StartGuided).unwrap();
        let save = |stage: &str, s: &Session, frames: &mut Vec<serde_json::Value>| {
            let mut g = serde_json::to_value(s.guided().unwrap()).unwrap();
            g["id"]["lineage"] = 0.into();
            g["id"]["generation"] = 0.into();
            frames.push(serde_json::json!({"case":c.id,"stage":stage,"table":s.view(),"guided":g}));
        };
        if c.rule.branch != crate::GuidedBranch::WinEarly {
            save("unsupported", &s, &mut frames);
            continue;
        }
        respond(&mut s, G::Reveal).unwrap();
        save("rule", &s, &mut frames);
        respond(&mut s, G::Memory(true)).unwrap();
        save("memory", &s, &mut frames);
        // Named real legal trumping/overtrumping completion, fixture provenance.
        if let Some((cards, winner, _)) = c
            .paths
            .iter()
            .find(|p| p.0[1] == c.rule.card && p.1 != c.view.seat)
        {
            s.execute(Command::Act(Action::Play(cards[1]))).unwrap();
            s.apply(Action::Play(cards[2])).unwrap();
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(*winner)
            );
            save("overtaken", &s, &mut frames);
        }
    }
    let value = serde_json::to_string_pretty(&frames).unwrap() + "\n";
    if let Ok(path) = std::env::var("SKAT_SUIT_EARLY_WIDGET_FIXTURES_OUT") {
        std::fs::write(path, value).unwrap();
    } else {
        assert_eq!(
            value,
            include_str!("../../gui-egui/tests/fixtures/suit-win-early.json")
        );
    }
}

#[test]
fn suit_win_early_god_mode_estimates_reject_privileged_cache_and_equal_full_dialogue() {
    let cs = cases::cases();
    for (a, b) in [(6, 7), (8, 9)] {
        let mut x = session(&cs[a]);
        let mut y = session(&cs[b]);
        y.config.opponents = Opponents::GodMode;
        for s in [&mut x, &mut y] {
            s.set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 1,
            });
            // Existing God Mode analysis must never become guided estimates.
            let old = s.coach_job().unwrap().run(None);
            s.finish_coach(old).unwrap();
            s.execute(Command::StartGuided).unwrap();
            respond(s, G::Reveal).unwrap();
            s.execute(Command::SetCoach(CoachSettings {
                show_figures: true,
                ..CoachSettings::default()
            }))
            .unwrap();
            if let Some(job) = s.coach_job() {
                assert!(!job.sees_all_cards());
                let r = job.run(None);
                s.finish_coach(r).unwrap();
            }
        }
        equal(&x, &y);
        let p = cs[a]
            .paths
            .iter()
            .find(|p| cs[b].paths.contains(p))
            .unwrap();
        for s in [&mut x, &mut y] {
            s.execute(Command::Act(Action::Play(p.0[1]))).unwrap();
            s.apply(Action::Play(p.0[2])).unwrap();
            respond(s, G::Reveal).unwrap();
            assert!(!s.guided().unwrap().figures_requested);
            respond(s, G::Figures).unwrap();
            assert!(s.guided().unwrap().figures_requested);
        }
        equal(&x, &y);
    }
}

#[test]
fn suit_win_early_late_estimates_use_frozen_input_and_next_game_invalidates_dialogue() {
    let cs = cases::cases();
    let mut s = session(&cs[6]);
    s.set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    s.execute(Command::StartGuided).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    let job = s.coach_job().unwrap();
    s.execute(Command::Act(Action::Play(cases::card(7))))
        .unwrap();
    s.apply(Action::Play(cases::card(4))).unwrap();
    assert_eq!(
        s.coach_job().unwrap().analysis_input(),
        job.analysis_input()
    );
    let result = s.coach_job().unwrap().run(None);
    s.finish_coach(result).unwrap();
    respond(&mut s, G::Reveal).unwrap();
    s.execute(Command::SetCoach(CoachSettings {
        show_figures: true,
        ..CoachSettings::default()
    }))
    .unwrap();
    respond(&mut s, G::Figures).unwrap();
    assert!(s.guided().unwrap().figures_requested);
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
