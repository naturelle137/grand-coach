// SPDX-License-Identifier: EUPL-1.2
use super::*;
#[path = "../../skat-analysis/tests/common/suit_feed_partner.rs"]
mod cases;
fn session(c: &cases::Case) -> Session {
    let mut s = Session::new(SessionConfig::default());
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == c.view.seat)
        .unwrap();
    s.game = c.game();
    s
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
fn ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 0,
        series: SeriesLength::Fixed(3),
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        display: TableDisplay::Closed,
        ..SessionConfig::default()
    });
    let commands: Vec<Command> = serde_json::from_str(include_str!(
        "../../gui-egui/tests/fixtures/suit-feed-partner-route.json"
    ))
    .unwrap();
    for cmd in commands {
        s.execute(cmd).unwrap();
    }
    assert_eq!(s.game_seed(), 13080132717333068652);
    assert_eq!(s.view().game_number, 1);
    s
}
#[test]
fn suit_feed_partner_every_case_every_legal_card_attempt_and_completing_winner() {
    for c in cases::cases() {
        for (card, winner) in &c.completions {
            let mut s = session(&c);
            let before = s.export_current_recording_privileged();
            s.execute(Command::StartGuided).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.step, S::Orient);
            assert_eq!(g.feed_partner, c.rule.winner);
            assert!(
                g.task.is_none() && g.rule.is_none() && g.memory.is_none() && g.figures.is_none()
            );
            assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
            assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
            respond(&mut s, G::Answer(A::Seat(Some(c.view.seat)))).unwrap();
            assert_eq!(s.guided().unwrap().correct, Some(false));
            respond(&mut s, G::Retry).unwrap();
            respond(&mut s, G::Answer(A::Seat(c.rule.winner))).unwrap();
            assert!(s.guided().unwrap().task.is_none());
            respond(&mut s, G::Answer(A::Seat(None))).unwrap();
            assert_eq!(
                s.guided().unwrap().task,
                Some(crate::GuidedBranch::FeedPartner)
            );
            assert!(s.guided().unwrap().rule.is_none());
            respond(&mut s, G::Answer(A::Card(*card))).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Act);
            assert_eq!(s.guided().unwrap().rule, Some(c.rule.clone()));
            assert!(s.guided().unwrap().memory.is_none());
            respond(&mut s, G::Memory(true)).unwrap();
            assert_eq!(s.guided().unwrap().memory.unwrap().own, c.view.hand);
            respond(&mut s, G::Memory(false)).unwrap();
            assert_eq!(s.export_current_recording_privileged(), before);
            s.execute(Command::Act(Action::Play(*card))).unwrap();
            assert_eq!(s.guided().unwrap().step, S::Observe);
            let source = s.guided().unwrap().observed_source.unwrap();
            assert_eq!(
                source,
                skat_coach::Source::Trick {
                    index: c.view.tricks.len() as u8,
                    seat: c.view.seat,
                    led: c.view.current_trick.led().unwrap(),
                    card: *card
                }
            );
            assert!(s.guided().unwrap().observation.is_none());
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(*winner)
            );
        }
    }
}
#[test]
fn suit_feed_partner_both_hidden_pairs_complete_dialogue_and_god_mode_privacy() {
    let cs = cases::cases();
    for (a, b) in [(0, 6), (3, 7)] {
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
                G::Answer(A::Seat(cs[a].rule.winner)),
                G::Answer(A::Seat(None)),
                G::Reveal,
                G::Memory(true),
                G::Memory(false),
            ] {
                for s in [&mut x, &mut y] {
                    respond(s, action).unwrap();
                }
                equal(&x, &y);
            }
            for s in [&mut x, &mut y] {
                s.execute(Command::Act(Action::Play(cs[a].rule.card)))
                    .unwrap();
            }
            equal(&x, &y);
            for action in [
                G::Answer(A::Void(
                    cs[a]
                        .view
                        .contract
                        .unwrap()
                        .game
                        .follow(cs[a].view.current_trick.led().unwrap())
                        == cs[a].view.contract.unwrap().game.follow(cs[a].rule.card),
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
#[test]
fn suit_feed_partner_ordinary_all_four_legal_choices_replay_stale_reset_automation() {
    let initial = ordinary();
    assert_eq!(
        initial.view().legal_actions,
        vec![4, 12, 16, 28]
            .into_iter()
            .map(|n| Action::Play(cases::card(n)))
            .collect::<Vec<_>>()
    );
    for action in initial.view().legal_actions {
        let mut s = initial.clone();
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        let old = s.guided().unwrap().id;
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.export_current_recording_privileged(), before);
        s.execute(Command::StartGuided).unwrap();
        assert_eq!(
            s.execute(Command::Guided(old, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::Act(action)).unwrap();
        let actual = *s.game.tricks().last().unwrap();
        let Action::Play(card) = action else {
            unreachable!()
        };
        assert_eq!(actual.cards.map(|c| c.index()), [23, 20, card.index()]);
        let winner = if card.index() == 4 || card.index() == 12 {
            Seat::Vorhand
        } else {
            Seat::Hinterhand
        };
        assert_eq!(actual.winner, winner);
        s.run_opponents(); // Observation must survive later automatic public play.
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(winner)
        );
        let r = s.export_current_recording_privileged().unwrap();
        let p = r.reconstruct_privileged(r.actions.len()).unwrap();
        assert_eq!(p.state.seats[0], s.game.view(Seat::Vorhand));
    }
    let mut s = ordinary();
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
    let mut reset = ordinary();
    assert_eq!(
        reset.execute(Command::Guided(id, G::Reveal)),
        Err(AppError::StaleGuided)
    );
}
#[test]
fn suit_feed_partner_return_each_stage_no_autoplay_and_automatic_help_boundaries() {
    let cs = cases::cases();
    for stage in 0..6 {
        let mut s = session(&cs[0]);
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        if stage >= 1 {
            respond(&mut s, G::Answer(A::Seat(cs[0].rule.winner))).unwrap();
        }
        if stage >= 2 {
            respond(&mut s, G::Answer(A::Seat(None))).unwrap();
        }
        if stage >= 3 {
            respond(&mut s, G::Reveal).unwrap();
        }
        if stage >= 4 {
            respond(&mut s, G::Memory(true)).unwrap();
        }
        if stage >= 5 {
            respond(&mut s, G::Memory(false)).unwrap();
        }
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
        assert_eq!(s.export_current_recording_privileged(), before);
    }
    for level in [
        crate::CoachLevel::Off,
        crate::CoachLevel::Important,
        crate::CoachLevel::EveryMove,
    ] {
        let mut s = ordinary();
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
}
#[test]
fn suit_feed_partner_frozen_permitted_estimates_reject_god_cache_consent_and_full_equality() {
    let cs = cases::cases();
    for (a, b) in [(0, 6), (3, 7)] {
        let mut x = session(&cs[a]);
        let mut y = session(&cs[b]);
        y.config.opponents = Opponents::GodMode;
        for s in [&mut x, &mut y] {
            s.set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 1,
            });
            let mode = s.config.opponents;
            s.config.opponents = Opponents::GodMode;
            let cached = s.coach_job().unwrap().run(None);
            s.finish_coach(cached).unwrap();
            s.config.opponents = mode;
            s.execute(Command::StartGuided).unwrap();
            respond(s, G::Reveal).unwrap();
            assert_eq!(respond(s, G::Figures), Err(AppError::NoQuestion));
            s.execute(Command::SetCoach(CoachSettings {
                show_figures: true,
                ..CoachSettings::default()
            }))
            .unwrap();
            let job = s.coach_job().unwrap();
            assert!(!job.sees_all_cards());
            let frozen = job.analysis_input();
            s.execute(Command::Act(Action::Play(cs[a].rule.card)))
                .unwrap();
            assert_eq!(s.coach_job().unwrap().analysis_input(), frozen);
            let result = job.run(None);
            s.finish_coach(result).unwrap();
            respond(s, G::Reveal).unwrap();
            assert!(!s.guided().unwrap().figures_requested);
            respond(s, G::Figures).unwrap();
            let g = s.guided().unwrap();
            assert!(g.figures_requested);
            assert!(g.figures.is_some());
            assert_eq!(g.rule, Some(cs[a].rule.clone()));
        }
        equal(&x, &y);
    }
}

#[test]
fn suit_feed_partner_fixture_widget_models_match_actual_sessions() {
    let mut frames = Vec::new();
    let save = |id: &str, stage: &str, s: &Session, frames: &mut Vec<serde_json::Value>| {
        let mut g = serde_json::to_value(s.guided().unwrap()).unwrap();
        g["id"]["lineage"] = 0.into();
        g["id"]["generation"] = 0.into();
        frames.push(serde_json::json!({"case":id,"stage":stage,"table":s.view(),"guided":g}));
    };
    for c in cases::cases().into_iter().take(6) {
        let mut s = session(&c);
        s.execute(Command::StartGuided).unwrap();
        respond(&mut s, G::Reveal).unwrap();
        save(c.id, "rule", &s, &mut frames);
        if c.id == "E" || c.id == "F" {
            respond(&mut s, G::Memory(true)).unwrap();
            save(c.id, "memory", &s, &mut frames);
            s.execute(Command::Act(Action::Play(c.rule.card))).unwrap();
            respond(&mut s, G::Reveal).unwrap();
            save(c.id, "completed", &s, &mut frames);
        }
    }
    let mut c = cases::cases().remove(0);
    c.declarer = Seat::Hinterhand;
    c.table.pop();
    c.view.seat = Seat::Mittelhand;
    let mut s = session(&c);
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Unsupported);
    save("early", "unsupported", &s, &mut frames);
    let c = cases::cases().remove(4);
    let mut s = session(&c);
    for n in [3, 7, 28] {
        s.apply(Action::Play(cases::card(n))).unwrap();
    }
    s.execute(Command::StartGuided).unwrap();
    assert_eq!(s.guided().unwrap().step, S::Unsupported);
    save("single", "unsupported", &s, &mut frames);
    let text = serde_json::to_string_pretty(&frames).unwrap() + "\n";
    if let Ok(path) = std::env::var("SKAT_SUIT_FEED_WIDGET_FIXTURES_OUT") {
        std::fs::write(path, text).unwrap();
    } else {
        assert_eq!(
            text,
            include_str!("../../gui-egui/tests/fixtures/suit-feed-partner.json")
        );
    }
}
