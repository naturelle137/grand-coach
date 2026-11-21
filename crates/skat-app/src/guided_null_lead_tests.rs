// SPDX-License-Identifier: EUPL-1.2

use super::*;
#[allow(dead_code)]
#[path = "../../skat-analysis/tests/null_lead_policy.rs"]
mod null_lead_fixture;
use null_lead_fixture::{Case, card, cases};

fn fixture_session(c: &Case) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    s.dealer = PlayerId(2);
    s.game = c.game();
    s.recording.deal = c.deal;
    s
}
fn ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 1,
        series: SeriesLength::Fixed(1),
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        display: TableDisplay::Closed,
        ..SessionConfig::default()
    });
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    for _ in 0..2 {
        s.execute(Command::AdvanceOpponent).unwrap();
    }
    for _ in 0..5 {
        s.execute(Command::Act(Action::Hold)).unwrap();
        s.execute(Command::AdvanceOpponent).unwrap();
    }
    s.execute(Command::Act(Action::Hand)).unwrap();
    s.execute(Command::Act(Action::Announce(
        skat_core::Announcement::plain(NULL),
    )))
    .unwrap();
    assert_eq!(s.game_seed(), 7424550030962593201);
    assert_eq!(s.view().bid, 24);
    assert_eq!(s.view().dealer, PlayerId(2));
    assert_eq!(
        s.view().legal_actions,
        [8, 14, 15, 22, 23, 25, 26, 28, 29, 31].map(|n| Action::Play(card(n)))
    );
    s
}
fn equal_dialogue(x: &Session, y: &Session) {
    let a = x.guided().expect("non-absent dialogue");
    let mut b = y.guided().expect("non-absent dialogue");
    b.id = a.id;
    assert_eq!(a, b);
}

#[test]
fn null_ouvert_opening_dialogue_remains_manual_and_observes_replies() {
    let mut s = Session::new(SessionConfig {
        seed: 1,
        series: SeriesLength::Fixed(1),
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        display: TableDisplay::Closed,
        ..SessionConfig::default()
    });
    s.execute(Command::SetCoach(CoachSettings::default()))
        .unwrap();
    for _ in 0..2 {
        s.execute(Command::AdvanceOpponent).unwrap();
    }
    for _ in 0..5 {
        s.execute(Command::Act(Action::Hold)).unwrap();
        s.execute(Command::AdvanceOpponent).unwrap();
    }
    s.execute(Command::Act(Action::Hand)).unwrap();
    s.execute(Command::Act(Action::Announce(skat_core::Announcement {
        game: NULL,
        schneider: false,
        schwarz: false,
        ouvert: true,
    })))
    .unwrap();
    for action in s.view().legal_actions {
        let mut s = s.clone();
        let legal = s.view().legal_actions.clone();
        s.execute(Command::StartGuided).unwrap();
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(s.guided().unwrap().step, S::Act);
        assert_eq!(s.view().legal_actions, legal);
        s.execute(Command::Act(action)).unwrap();
        for _ in 0..2 {
            s.execute(Command::AdvanceOpponent).unwrap();
        }
        assert_eq!(s.guided().unwrap().step, S::Observe);
        assert_eq!(s.guided().unwrap().lead_replies.len(), 2);
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(s.game.tricks()[0].winner)
        );
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}
#[test]
fn null_lead_all_286_completed_observations_and_94_immediate_losses() {
    let mut total = 0;
    let mut losses = 0;
    for c in cases() {
        for &(cards, winner) in &c.paths {
            let mut s = fixture_session(&c);
            s.execute(Command::StartGuided).unwrap();
            let id = s.guided().unwrap().id;
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(s.guided().unwrap().rule, Some(c.expected.clone()));
            s.execute(Command::Act(Action::Play(cards[0]))).unwrap();
            for (i, &n) in cards.iter().enumerate().skip(1) {
                assert_eq!(s.guided().unwrap().step, S::Waiting);
                assert!(s.guided().unwrap().observation.is_none());
                s.apply(Action::Play(n)).unwrap();
                assert_eq!(s.guided().unwrap().lead_replies.len(), i);
            }
            assert_eq!(s.guided().unwrap().step, S::Observe);
            assert_eq!(s.guided().unwrap().id, id);
            respond(&mut s, G::Reveal).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.observation.unwrap().winner, Some(winner));
            for (i, src) in g.lead_replies.iter().enumerate() {
                assert_eq!(
                    *src,
                    skat_coach::Source::Trick {
                        index: 0,
                        seat: Seat::from_index(i + 1),
                        led: cards[0],
                        card: cards[i + 1]
                    }
                );
            }
            assert_eq!(s.game.phase() == Phase::Beendet, winner == Seat::Vorhand);
            total += 1;
            if winner == Seat::Vorhand {
                losses += 1;
                assert!(!s.game.result().unwrap().won);
            }
        }
    }
    assert_eq!((total, losses), (286, 94));
}
#[test]
fn null_lead_whole_dialogue_two_hidden_pairs_including_god_mode() {
    let cs = cases();
    for (a, b) in [(0, 1), (2, 3)] {
        for god in [false, true] {
            let mut x = fixture_session(&cs[a]);
            let mut y = fixture_session(&cs[b]);
            if god {
                x.config.opponents = Opponents::GodMode;
                y.config.opponents = Opponents::GodMode;
            }
            for s in [&mut x, &mut y] {
                s.execute(Command::StartGuided).unwrap();
            }
            equal_dialogue(&x, &y);
            for action in [
                G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: false,
                }),
                G::Retry,
                G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: true,
                }),
                G::Answer(A::Card(*cs[a].expected.candidates.last().unwrap())),
                G::Answer(A::Card(*cs[a].expected.candidates.last().unwrap())),
                G::Retry,
                G::Answer(A::Card(cs[a].expected.card)),
                G::Memory(true),
                G::Memory(false),
            ] {
                for s in [&mut x, &mut y] {
                    respond(s, action).unwrap();
                }
                equal_dialogue(&x, &y);
            }
            assert!(x.guided().unwrap().rule.is_some());
            let common = cs[a]
                .paths
                .iter()
                .find(|p| cs[b].paths.contains(p))
                .unwrap()
                .0;
            for n in common {
                for s in [&mut x, &mut y] {
                    s.apply(Action::Play(n)).unwrap();
                }
                equal_dialogue(&x, &y);
            }
            for action in [
                G::Answer(A::Void(true)),
                G::Retry,
                G::Reveal,
                G::Memory(true),
                G::Memory(false),
            ] {
                for s in [&mut x, &mut y] {
                    respond(s, action).unwrap();
                }
                equal_dialogue(&x, &y);
            }
            assert_eq!(x.guided().unwrap().step, S::Complete);
        }
    }
}
#[test]
fn null_lead_ordinary_all_cards_consent_return_stale_reentry_replay_and_real_replies() {
    for n in [8, 14, 15, 22, 23, 25, 26, 28, 29, 31] {
        let mut s = ordinary();
        let before = s.view();
        let rec = s.export_current_recording_privileged();
        assert!(s.guided().is_none());
        s.execute(Command::StartGuided).unwrap();
        let old = s.guided().unwrap().id;
        assert!(s.guided().unwrap().rule.is_none() && s.guided().unwrap().memory.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.view(), before);
        assert_eq!(s.export_current_recording_privileged(), rec);
        s.execute(Command::StartGuided).unwrap();
        let id = s.guided().unwrap().id;
        assert_ne!(id, old);
        assert_eq!(
            s.execute(Command::Guided(old, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(
            &mut s,
            G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: false,
            }),
        )
        .unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(
            &mut s,
            G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: true,
            }),
        )
        .unwrap();
        respond(&mut s, G::Answer(A::Card(card(n)))).unwrap();
        assert_eq!(s.guided().unwrap().step, S::Narrow);
        respond(&mut s, G::Answer(A::Card(card(14)))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Card(card(8)))).unwrap();
        assert_eq!(s.guided().unwrap().rule.unwrap().card, card(8));
        respond(&mut s, G::Memory(true)).unwrap();
        assert!(s.guided().unwrap().memory.unwrap().skat.is_none());
        assert_eq!(s.export_current_recording_privileged(), rec);
        s.execute(Command::Act(Action::Play(card(n)))).unwrap();
        for i in 1..=2 {
            s.execute(Command::AdvanceOpponent).unwrap();
            assert_eq!(s.guided().unwrap().lead_replies.len(), i);
        }
        assert_eq!(s.guided().unwrap().step, S::Observe);
        assert!(s.guided().unwrap().memory.is_none());
        respond(&mut s, G::Reveal).unwrap();
        let t = s.game.tricks()[0];
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(t.winner)
        );
        if n == 8 {
            assert_eq!(t.cards.map(|c| c.index()), [8, 10, 9]);
            assert_eq!(t.winner, Seat::Mittelhand);
        }
        if n == 14 {
            assert_eq!(t.cards.map(|c| c.index()), [14, 12, 13]);
            assert_eq!(t.winner, Seat::Vorhand);
            assert_eq!(s.game.phase(), Phase::Beendet);
            assert_eq!(s.game.result().unwrap().score, -70);
        }
        let frozen = s.guided().unwrap();
        s.run_opponents();
        assert_eq!(s.guided().unwrap(), frozen);
        let r = s.export_current_recording_privileged().unwrap();
        let p = r.reconstruct_privileged(r.actions.len()).unwrap();
        assert_eq!(p.state.seats[0], s.game.view(Seat::Vorhand));
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}

#[test]
fn null_lead_off_automatic_unaided_estimates_origin_and_reset() {
    for level in crate::CoachLevel::ALL {
        let mut s = ordinary();
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
        s.execute(Command::RequestAdvice).unwrap();
        let result = s
            .coach_job()
            .unwrap()
            .run(None)
            .guided_rank_first(Action::Play(card(14)));
        let expected_hints =
            skat_coach::hints(result.explanation().unwrap(), skat_coach::Stage::Figures);
        s.finish_coach(result).unwrap();
        s.execute(Command::Act(Action::Play(card(14)))).unwrap();
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
        assert_eq!(g.figures, Some(expected_hints));
        assert_eq!(
            g.figures
                .as_ref()
                .unwrap()
                .situation
                .as_ref()
                .unwrap()
                .options,
            [8, 14, 15, 22, 23, 25, 26, 28, 29, 31].map(|n| Action::Play(card(n)))
        );
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
fn null_lead_fixture_widget_models_match_checked_sessions() {
    let mut frames = Vec::new();
    let mut save = |id: &str, stage: &str, s: &Session| {
        let mut g = serde_json::to_value(s.guided().unwrap()).unwrap();
        g["id"]["lineage"] = 0.into();
        g["id"]["generation"] = 0.into();
        frames.push(serde_json::json!({"case":id,"stage":stage,"table":s.view(),"guided":g}));
    };
    for c in cases().into_iter().filter(|c| !c.id.ends_with("hidden")) {
        let mut s = fixture_session(&c);
        s.execute(Command::StartGuided).unwrap();
        respond(&mut s, G::Reveal).unwrap();
        save(c.id, "rule", &s);
        respond(&mut s, G::Memory(true)).unwrap();
        save(c.id, "memory", &s);
        let (cards, winner) = c.paths.iter().find(|p| p.1 == Seat::Vorhand).unwrap();
        for &card in cards {
            s.apply(Action::Play(card)).unwrap();
        }
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(*winner)
        );
        save(c.id, "null-end", &s);
    }
    let c = cases().remove(0);
    for (id, history) in [
        (
            "MH-otherlead",
            vec![Action::Bid(18), Action::Pass, Action::Pass],
        ),
        (
            "HH-otherlead",
            vec![Action::Pass, Action::Bid(18), Action::Pass],
        ),
    ] {
        let mut s = fixture_session(&c);
        s.game = Game::from_deal(c.deal, RuleConfig { kontra_re: false });
        for a in history.into_iter().chain([
            Action::Hand,
            Action::Announce(skat_core::Announcement::plain(NULL)),
        ]) {
            s.apply(a).unwrap();
        }
        s.dealer = if id == "MH-otherlead" {
            PlayerId(0)
        } else {
            PlayerId(1)
        };
        // Genuine core history, but explicit component fixture, not Session::new route.
        let key = crate::coach::DecisionKey {
            lineage: s.lineage,
            number: s.number,
            actions: s.actions,
        };
        let v = s.game.view(PlayerId::HUMAN.seat(s.dealer));
        s.guided = Some(crate::guided::GuidedState::new(key, &v, None));
        assert_eq!(s.guided().unwrap().step, S::Unsupported);
        save(id, "unsupported", &s);
    }
    let value = serde_json::to_string_pretty(&frames).unwrap() + "\n";
    if let Ok(path) = std::env::var("SKAT_NULL_LEAD_WIDGET_FIXTURES_OUT") {
        std::fs::write(path, value).unwrap();
    } else {
        assert_eq!(
            value,
            include_str!("../../gui-egui/tests/fixtures/null-lead.json")
        );
    }
}

#[test]
fn null_lead_return_at_every_stage_preserves_game_and_next_game_clears_identity() {
    for stage in 0..6 {
        let mut s = ordinary();
        s.execute(Command::StartGuided).unwrap();
        if stage >= 1 {
            respond(
                &mut s,
                G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: true,
                }),
            )
            .unwrap();
        }
        if stage >= 2 {
            respond(&mut s, G::Answer(A::Card(card(31)))).unwrap();
        }
        if stage >= 3 {
            respond(&mut s, G::Reveal).unwrap();
        }
        if stage >= 4 {
            s.execute(Command::Act(Action::Play(card(14)))).unwrap();
        }
        if stage >= 5 {
            s.run_opponents();
        }
        let v = s.game.view(Seat::Vorhand);
        let rec = s.export_current_recording_privileged();
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
        assert_eq!(s.game.view(Seat::Vorhand), v);
        assert_eq!(s.export_current_recording_privileged(), rec);
    }
    let mut s = ordinary();
    s.config.series = SeriesLength::Free;
    s.execute(Command::StartGuided).unwrap();
    let id = s.guided().unwrap().id;
    s.execute(Command::Act(Action::Play(card(14)))).unwrap();
    s.run_opponents();
    assert_eq!(s.game.phase(), Phase::Beendet);
    s.execute(Command::NextGame).unwrap();
    assert!(s.guided().is_none());
    assert_eq!(
        s.execute(Command::Guided(id, G::Reveal)),
        Err(AppError::StaleGuided)
    );
}

#[test]
fn null_defender_opening_dialogue_observes_both_replies_for_every_lead() {
    let c = cases().remove(0);
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        ..SessionConfig::default()
    });
    s.dealer = PlayerId(2);
    s.game = skat_core::Game::from_deal(c.deal, skat_core::RuleConfig { kontra_re: false });
    s.recording.deal = c.deal;
    for a in [
        Action::Bid(18),
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(skat_core::Announcement::plain(NULL)),
    ] {
        s.apply(a).unwrap();
    }
    for action in s.view().legal_actions {
        let mut s = s.clone();
        s.execute(Command::StartGuided).unwrap();
        assert!(s.guided().unwrap().rule.is_none());
        respond(
            &mut s,
            G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: false,
            }),
        )
        .unwrap();
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::Act(action)).unwrap();
        for _ in 0..2 {
            s.execute(Command::AdvanceOpponent).unwrap();
        }
        assert_eq!(s.guided().unwrap().lead_replies.len(), 2);
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(s.game.tricks()[0].winner)
        );
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}
