// SPDX-License-Identifier: EUPL-1.2
//! #182: app-level evidence for the non-Ouvert Farbspiel `LeadMaster` lesson.
use super::*;
#[allow(dead_code)]
#[path = "../../skat-analysis/tests/suit_lead_master_policy.rs"]
mod fx;

fn session(deal: skat_core::Deal, game: skat_core::Game) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == Seat::Vorhand)
        .unwrap();
    s.game = game;
    s.recording.deal = deal;
    s
}
fn case_session(c: &fx::Case) -> Session {
    session(c.deal, c.game.clone())
}
/// Genuine ordinary route: seed 2969, the human passes at the first chance and defends a Farbspiel
/// that Easy opponents declare; Easy opponents play through the normal automation.
fn ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 2969,
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
    assert_eq!((v.game_number, v.tricks_played), (1, 0));
    assert_eq!(v.dealer, PlayerId(2));
    assert_eq!(
        v.legal_actions,
        [0, 1, 5, 7, 17, 21, 24, 27, 30, 31].map(|n| Action::Play(fx::card(n)))
    );
    s
}
fn same(x: &Session, y: &Session, what: &str) {
    assert_eq!(
        x.game.view(Seat::Vorhand),
        y.game.view(Seat::Vorhand),
        "{what}"
    );
    let gx = x.guided().unwrap();
    let mut gy = y.guided().unwrap();
    gy.id = gx.id;
    assert_eq!(gx, gy, "{what}");
}

#[test]
fn suit_lead_master_every_case_and_oracle_completion_waits_for_both_replies_and_winner() {
    let mut total = 0;
    for c in fx::cases() {
        let list = fx::completions(&c.game);
        let late = c.game.view(Seat::Vorhand).tricks.len() as u8;
        for (cards, winner) in &list {
            let mut s = case_session(&c);
            s.execute(Command::StartGuided).unwrap();
            let id = s.guided().unwrap().id;
            assert_eq!(s.guided().unwrap().step, S::Orient, "{}", c.id);
            assert!(s.guided().unwrap().rule.is_none());
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(s.guided().unwrap().rule.unwrap().card, fx::card(c.card));
            s.execute(Command::Act(Action::Play(fx::card(cards[0]))))
                .unwrap();
            assert_eq!(s.guided().unwrap().step, S::Waiting);
            s.apply(Action::Play(fx::card(cards[1]))).unwrap();
            let g = s.guided().unwrap();
            assert_eq!((g.step, g.lead_replies.len()), (S::Waiting, 1));
            assert!(g.observation.is_none());
            s.apply(Action::Play(fx::card(cards[2]))).unwrap();
            let g = s.guided().unwrap();
            assert_eq!(g.id, id);
            assert_eq!((g.step, g.lead_replies.len()), (S::Observe, 2));
            assert!(g.observation.is_none());
            assert_eq!(
                g.observed_source,
                Some(skat_coach::Source::Trick {
                    index: late,
                    seat: Seat::Hinterhand,
                    led: fx::card(cards[0]),
                    card: fx::card(cards[2])
                })
            );
            respond(&mut s, G::Reveal).unwrap();
            assert_eq!(
                s.guided().unwrap().observation.unwrap().winner,
                Some(*winner)
            );
            total += 1;
        }
    }
    assert_eq!(total, 540 + 193 + 265 + 500 + 305);
}

#[test]
fn suit_lead_master_full_non_absent_dialogue_privacy_including_god_mode() {
    let d = fx::hidden_deals();
    let build = |deal: skat_core::Deal, god: bool| {
        let g = fx::play(deal, skat_core::Suit::Herz, fx::MH_DECLARES, &[]);
        let mut s = session(deal, g);
        if god {
            s.config.opponents = Opponents::GodMode;
        }
        s
    };
    for (pair, alt) in [(1, d[1]), (2, d[2])] {
        for god in [false, true] {
            let mut x = build(d[0], god);
            let mut y = build(alt, god);
            x.execute(Command::StartGuided).unwrap();
            y.execute(Command::StartGuided).unwrap();
            same(&x, &y, "start");
            for a in [
                G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: true,
                }),
                G::Retry,
                G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: false,
                }),
                G::Answer(A::Card(fx::card(3))), // master, but not Easy's pick
                G::Answer(A::Card(fx::card(7))), // master, loses the exact last tie
                G::Retry,
                G::Answer(A::Card(fx::card(31))),
                G::Memory(true),
                G::Memory(false),
            ] {
                respond(&mut x, a).unwrap();
                respond(&mut y, a).unwrap();
                same(&x, &y, &format!("pair{pair}/god={god}/{a:?}"));
            }
            assert!(x.guided().unwrap().rule.is_some());
            for s in [&mut x, &mut y] {
                s.execute(Command::Act(Action::Play(fx::card(31)))).unwrap();
            }
            same(&x, &y, "after lead");
            if pair == 1 {
                // The hidden hands are exchanged: no common legal reply exists, so the dialogue
                // equality is checked up to the human's own card.
                continue;
            }
            for _ in 0..2 {
                let (lx, ly) = (x.game.legal_actions(), y.game.legal_actions());
                let common = *lx.iter().find(|a| ly.contains(a)).unwrap();
                x.apply(common).unwrap();
                y.apply(common).unwrap();
                same(&x, &y, "reply");
            }
            assert_eq!(x.guided().unwrap().step, S::Observe);
            for a in [G::Reveal, G::Memory(true), G::Memory(false)] {
                respond(&mut x, a).unwrap();
                respond(&mut y, a).unwrap();
                same(&x, &y, &format!("{a:?}"));
            }
        }
    }
}

#[test]
fn suit_lead_master_ordinary_all_legal_choices_against_easy_with_consent_stale_and_reset() {
    let initial = ordinary();
    let pv = initial.game.view(Seat::Vorhand);
    let hand: Vec<u8> = pv.hand.iter().map(|c| c.index()).collect();
    // Pik is announced (trump index 1).
    assert_eq!(fx::oracle_lead(1, &hand, &[]), (vec![7, 27, 30, 31], 31));
    let completions = fx::completions(&initial.game);
    for action in initial.view().legal_actions {
        let Action::Play(card) = action else { panic!() };
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
        assert_eq!(s.guided().unwrap().rule.unwrap().card.index(), 31);
        respond(&mut s, G::Memory(true)).unwrap();
        respond(&mut s, G::Memory(false)).unwrap();
        s.execute(Command::Act(action)).unwrap();
        s.run_opponents(); // Easy answers through the normal automation.
        let t = *s.game.tricks().last().unwrap();
        assert_eq!(t.cards[0], card);
        let played = t.cards.map(|c| c.index());
        assert!(
            completions
                .iter()
                .any(|(c, w)| *c == played && *w == t.winner),
            "{played:?}"
        );
        respond(&mut s, G::Reveal).unwrap();
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(t.winner)
        );
        let r = s.export_current_recording_privileged().unwrap();
        let p = r.reconstruct_privileged(r.actions.len()).unwrap();
        assert_eq!(p.state.seats[0], s.game.view(Seat::Vorhand));
    }
    // Easy's own reply differs from a different legal lead only through the played card.
    let (mut a, mut b) = (initial.clone(), initial.clone());
    a.execute(Command::Act(Action::Play(fx::card(31)))).unwrap();
    b.execute(Command::Act(Action::Play(fx::card(7)))).unwrap();
    a.run_opponents();
    b.run_opponents();
    assert_ne!(a.game.tricks()[0].cards[0], b.game.tricks()[0].cards[0]);
    // Stale and reset.
    let mut s = initial.clone();
    s.execute(Command::StartGuided).unwrap();
    let id = s.guided().unwrap().id;
    while s.game.current_seat().is_some() {
        s.apply(s.game.legal_actions()[0]).unwrap();
    }
    s.execute(Command::NextGame).unwrap_or_default();
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
fn suit_lead_master_return_each_stage_and_help_levels_never_autoplay() {
    let c = &fx::cases()[0];
    for stage in 0..4 {
        let mut s = case_session(c);
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        if stage >= 1 {
            respond(
                &mut s,
                G::Answer(A::Lead {
                    seat: Seat::Vorhand,
                    declarer: false,
                }),
            )
            .unwrap();
        }
        if stage >= 2 {
            respond(&mut s, G::Reveal).unwrap();
        }
        if stage >= 3 {
            respond(&mut s, G::Memory(true)).unwrap();
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
fn suit_lead_master_estimates_are_original_unaided_and_off_by_default_levels() {
    // Farbspiel LeadMaster follows the Grand LeadMaster estimate contract: no permitted-only
    // estimate hold; the figures are requested separately and describe the original decision.
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
        let result = s
            .coach_job()
            .unwrap()
            .run(None)
            .guided_rank_first(Action::Play(fx::card(0)));
        s.finish_coach(result).unwrap();
        s.execute(Command::Act(Action::Play(fx::card(0)))).unwrap();
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
        assert_eq!((g.id, g.rule.clone()), (id, rule));
        assert!(g.figures_requested);
        if let Some(h) = g.figures {
            assert_eq!(
                h.situation.expect("original-decision estimates").options[0],
                Action::Play(fx::card(0))
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
