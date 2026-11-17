// SPDX-License-Identifier: EUPL-1.2
use super::*;

fn ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 22,
        series: SeriesLength::Fixed(3),
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    let route: Vec<Command> = serde_json::from_str(include_str!(
        "../../gui-egui/tests/fixtures/early-feed-route.json"
    ))
    .unwrap();
    for cmd in route {
        s.run_opponents();
        s.execute(cmd).unwrap();
    }
    s.run_opponents();
    s
}

#[test]
fn grand_early_feed_ordinary_every_card_actual_reply_consent_stale_and_return() {
    let base = ordinary();
    let v = base.view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.players[0].seat),
        (3, 0, Seat::Mittelhand)
    );
    assert_eq!(
        v.legal_actions,
        [9, 10, 13].map(|n| Action::Play(feed_fixture::card(n)))
    );
    for action in v.legal_actions.clone() {
        let mut s = base.clone();
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        assert!(s.guided().unwrap().rule.is_none());
        assert!(s.guided().unwrap().task.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Mittelhand)))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
        respond(&mut s, G::Answer(A::Seat(None))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
        assert_eq!(
            s.guided().unwrap().task,
            Some(crate::GuidedBranch::FeedPartner)
        );
        let Action::Play(card) = action else {
            unreachable!()
        };
        respond(&mut s, G::Answer(A::Card(card))).unwrap();
        assert_eq!(s.guided().unwrap().rule.unwrap().card.index(), 13);
        respond(&mut s, G::Memory(true)).unwrap();
        assert_eq!(
            s.guided().unwrap().memory.unwrap().own,
            base.game.view(Seat::Mittelhand).hand
        );
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.export_current_recording_privileged(), before);
        s.execute(Command::StartGuided).unwrap();
        let old = s.guided().unwrap().id;
        respond(&mut s, G::Return).unwrap();
        s.execute(Command::StartGuided).unwrap();
        assert_eq!(
            s.execute(Command::Guided(old, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::Act(action)).unwrap();
        assert_eq!(s.guided().unwrap().step, S::Waiting);
        s.run_opponents();
        assert_eq!(s.guided().unwrap().step, S::Observe);
        let actual = s.game.tricks()[0];
        assert_eq!(
            actual.cards,
            [feed_fixture::card(15), card, feed_fixture::card(28)]
        );
        assert_eq!(actual.winner, Seat::Hinterhand);
        assert_eq!(
            s.guided().unwrap().observed_source,
            Some(skat_coach::Source::Trick {
                index: 0,
                seat: Seat::Hinterhand,
                led: feed_fixture::card(15),
                card: actual.cards[2]
            })
        );
        assert!(s.guided().unwrap().observation.is_none());
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(actual.winner)
        );
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}

fn fixture_session(hidden: bool) -> Session {
    let mut c = feed_fixture::cases()
        .into_iter()
        .find(|c| c.id == "B-early-master-can-be-trumped")
        .unwrap();
    if hidden {
        c.hands[0][3] = 16;
        c.hands[2][2] = 13;
        c.hands[2][3] = 20;
        c.skat[0] = 17;
    }
    let mut s = Session::new(SessionConfig::default());
    s.dealer = PlayerId(1); // Human physical Mittelhand.
    s.game = c.position();
    s
}

fn equal(x: &Session, y: &Session) {
    assert_eq!(x.game.view(Seat::Mittelhand), y.game.view(Seat::Mittelhand));
    let a = x.guided().unwrap();
    let mut b = y.guided().unwrap();
    b.id = a.id;
    assert_eq!(a, b);
}

#[test]
fn grand_early_feed_full_dialogue_hidden_substitution_god_mode_and_original_estimates() {
    for god in [false, true] {
        let mut x = fixture_session(false);
        let mut y = fixture_session(true);
        if god {
            y.config.opponents = Opponents::GodMode;
        }
        for s in [&mut x, &mut y] {
            s.set_coach_budget(Budget {
                samples: 1,
                nodes_per_task: 1,
            });
            let mode = s.config.opponents;
            s.config.opponents = Opponents::GodMode;
            let cache = s.coach_job().unwrap().run(None);
            s.finish_coach(cache).unwrap();
            s.config.opponents = mode;
            s.execute(Command::StartGuided).unwrap();
        }
        equal(&x, &y);
        for action in [
            G::Answer(A::Seat(Some(Seat::Mittelhand))),
            G::Retry,
            G::Answer(A::Seat(Some(Seat::Vorhand))),
            G::Answer(A::Seat(Some(Seat::Hinterhand))),
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
            s.execute(Command::SetCoach(CoachSettings {
                show_figures: true,
                ..CoachSettings::default()
            }))
            .unwrap();
            let job = s.coach_job().unwrap();
            assert!(!job.sees_all_cards());
            let frozen = job.analysis_input();
            s.execute(Command::Act(Action::Play(feed_fixture::card(5))))
                .unwrap();
            assert_eq!(s.coach_job().unwrap().analysis_input(), frozen);
            s.apply(Action::Play(feed_fixture::card(4))).unwrap(); // CJ trumps CA.
            let result = job.run(None);
            s.finish_coach(result).unwrap();
            respond(s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(Seat::Hinterhand)
            );
            assert!(s.guided().unwrap().observation.unwrap().proves_void);
            assert!(!s.guided().unwrap().figures_requested);
            respond(s, G::Figures).unwrap();
            assert!(s.guided().unwrap().figures_requested);
            assert!(s.guided().unwrap().figures.is_some());
        }
        equal(&x, &y);
    }
}
