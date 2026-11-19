// SPDX-License-Identifier: EUPL-1.2

//! Grand declarer `DrawTrumpf` dialogue (#175): ordinary route search,
//! privacy of the whole dialogue and actual replies.
use super::*;
use crate::{GuidedAction as G, GuidedAnswer as A, GuidedStep as S};

fn config(seed: u64) -> SessionConfig {
    SessionConfig {
        seed,
        series: SeriesLength::Free,
        opponents: Opponents::Levels([Level::Easy; 2]),
        kontra_re: false,
        display: TableDisplay::Closed,
        ..SessionConfig::default()
    }
}

/// Explicit human commands up to the first own Grand `DrawTrumpf` lead:
/// lowest bid or hold, Hand game, plain Grand, then the first legal card
/// until a DrawTrumpf lesson is reached. Returns the session at that lesson.
fn drive(seed: u64, hand: bool) -> Option<(Session, u32, usize)> {
    let mut s = Session::new(config(seed));
    for game in 1..=3u32 {
        loop {
            s.run_opponents();
            let v = s.view();
            if v.can_next_game {
                break;
            }
            let seat = PlayerId::HUMAN.seat(s.dealer);
            let pv = s.game.view(seat);
            if let Some(l) = skat_coach::guided::lesson(&pv)
                && l.rule.branch == crate::GuidedBranch::DrawTrumpf
            {
                return Some((s, game, pv.tricks.len()));
            }
            let legal = &v.legal_actions;
            // Bid or hold only up to the plain Grand value 24, otherwise pass.
            let a = legal
                .iter()
                .copied()
                .find(|a| matches!(a, Action::Hold) && v.bid <= 24)
                .or_else(|| {
                    legal
                        .iter()
                        .copied()
                        .find(|a| matches!(a, Action::Bid(x) if *x <= 24))
                })
                .or_else(|| legal.iter().copied().find(|a| matches!(a, Action::Pass)))
                .or_else(|| {
                    legal
                        .iter()
                        .copied()
                        .find(|a| hand && matches!(a, Action::Hand))
                })
                .or_else(|| {
                    legal.iter().copied().find(|a| {
                        matches!(a, Action::Announce(x) if x.game == skat_core::GameType::Grand
                            && !x.ouvert && !x.schneider && !x.schwarz)
                    })
                })
                .unwrap_or(legal[0]);
            s.execute(Command::Act(a)).unwrap();
        }
        s.execute(Command::NextGame).unwrap();
    }
    None
}

#[allow(dead_code)]
#[path = "../../skat-analysis/tests/draw_trumpf_policy.rs"]
mod witnesses;

fn card(n: u8) -> skat_core::Card {
    skat_core::Card::from_index(n).unwrap()
}

fn fixture_session(c: &witnesses::Case) -> Session {
    let mut s = Session::new(SessionConfig {
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    s.dealer = (0..3)
        .map(PlayerId)
        .find(|&d| PlayerId::HUMAN.seat(d) == c.viewer)
        .unwrap();
    s.game = c.game();
    s.recording.deal = c.deal();
    s
}

fn respond(s: &mut Session, action: G) -> Result<Vec<AppEvent>, AppError> {
    s.execute(Command::Guided(s.guided().unwrap().id, action))
}

fn same(x: &Session, y: &Session, what: &str) {
    let gx = x.guided().unwrap();
    let mut gy = y.guided().unwrap();
    gy.id = gx.id;
    assert_eq!(gx, gy, "{what}");
}

/// The first ordinary `Session::new` route found by the bounded search:
/// seeds 0..32, at most three games each, Easy opponents, Kontra/Re off, human
/// commands exactly as in `drive` (bid or hold up to 24, Hand, plain Grand,
/// else the first legal card). Seed 1 is the first hit (seed 0 never
/// reaches the lesson under this policy).
#[test]
fn ordinary_route_search_reports_the_first_hits() {
    let hits: Vec<_> = (0..32u64)
        .filter_map(|seed| drive(seed, true).map(|(_, game, tricks)| (seed, game, tricks)))
        .take(4)
        .collect();
    assert_eq!(hits, [(1, 1, 0), (3, 1, 0), (4, 1, 0), (5, 1, 0)]);
}

#[test]
fn ordinary_route_every_legal_card_replies_winner_and_free_return() {
    let (route, _, _) = drive(1, true).unwrap();
    let legal: Vec<_> = route.view().legal_actions;
    assert_eq!(legal.len(), 10);
    for &a in &legal {
        let mut s = route.clone();
        let before = s.view();
        let recording = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        let old = s.guided().unwrap().id;
        assert!(s.guided().unwrap().rule.is_none() && s.guided().unwrap().memory.is_none());
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
        assert!(s.guided().unwrap().lead_trumpf || s.guided().unwrap().lead.is_some());
        respond(&mut s, G::Reveal).unwrap();
        let rule = s.guided().unwrap().rule.unwrap();
        assert_eq!(rule.branch, crate::GuidedBranch::DrawTrumpf);
        let easy = rule.card;
        assert!(s.guided().unwrap().lead_trumpf);
        respond(&mut s, G::Memory(true)).unwrap();
        assert!(s.guided().unwrap().memory.is_some());
        let Action::Play(played) = a else { panic!() };
        s.execute(Command::Act(a)).unwrap();
        assert_eq!(s.guided().unwrap().step, S::Waiting);
        s.run_opponents();
        let g = s.guided().unwrap();
        assert_eq!((g.id, g.step), (id, S::Observe));
        assert_eq!(g.lead_replies.len(), 2);
        assert!(g.memory.is_none() && g.observation.is_none());
        respond(&mut s, G::Reveal).unwrap();
        let t = *s.game.tricks().last().unwrap();
        assert_eq!(t.cards[0], played);
        assert_eq!(
            s.guided().unwrap().observation.unwrap().winner,
            Some(t.winner)
        );
        let frozen = s.guided().unwrap();
        s.run_opponents();
        assert_eq!(s.guided().unwrap(), frozen);
        eprintln!(
            "ordinary seed1 lead={} easy={} replies={:?} winner={:?}",
            played.index(),
            easy.index(),
            &t.cards.map(|c| c.index())[1..],
            t.winner
        );
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}

#[test]
fn full_non_absent_dialogue_privacy_including_god_mode() {
    let cs = witnesses::cases();
    let a = cs.iter().find(|c| c.id == "01-vh-hand").unwrap();
    for god in [false, true] {
        let mut other = witnesses::cases().into_iter().next().unwrap();
        // Same permitted view, different concealed defender cards.
        for s in [1usize, 2] {
            other.deal[s] = other.deal[s]
                .iter()
                .map(|&n| match n {
                    1 => 15,
                    15 => 1,
                    n => n,
                })
                .collect();
        }
        assert_ne!(other.deal(), a.deal());
        let build = |c: &witnesses::Case| {
            let mut s = fixture_session(c);
            if god {
                s.config.opponents = Opponents::GodMode;
            }
            s
        };
        let (mut x, mut y) = (build(a), build(&other));
        x.execute(Command::StartGuided).unwrap();
        y.execute(Command::StartGuided).unwrap();
        for step in [
            None,
            Some(G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: false,
            })),
            Some(G::Retry),
            Some(G::Answer(A::Lead {
                seat: Seat::Vorhand,
                declarer: true,
            })),
            Some(G::Answer(A::Card(card(7)))),
            Some(G::Retry),
            Some(G::Answer(A::Card(card(12)))),
            // SJ is a Trumpf candidate but Easy leads CJ.
            Some(G::Retry),
            Some(G::Answer(A::Card(card(4)))),
            Some(G::Memory(true)),
            Some(G::Memory(false)),
        ] {
            if let Some(g) = step {
                respond(&mut x, g).unwrap();
                respond(&mut y, g).unwrap();
            }
            same(&x, &y, &format!("god={god}/{step:?}"));
        }
        assert!(x.guided().unwrap().rule.is_some());
        // Common legal public continuation: CJ, then each defender replies.
        for n in [4, 20, 19] {
            x.apply(Action::Play(card(n))).unwrap();
            y.apply(Action::Play(card(n))).unwrap();
            assert_eq!(x.game.view(Seat::Vorhand), y.game.view(Seat::Vorhand));
            same(&x, &y, &format!("god={god}/public={n}"));
        }
        assert_eq!(x.guided().unwrap().step, S::Observe);
        for g in [
            G::Answer(A::Void(false)),
            G::Retry,
            G::Answer(A::Void(true)),
        ] {
            respond(&mut x, g).unwrap();
            respond(&mut y, g).unwrap();
            same(&x, &y, &format!("god={god}/{g:?}"));
        }
    }
}
