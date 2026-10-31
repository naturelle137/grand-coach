// SPDX-License-Identifier: EUPL-1.2

//! Information boundary of the human's view, checked against the full game
//! state that only the session itself holds.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use skat_core::{CardSet, Seat};

use crate::{Command, Level, Opponents, PlayerId, SeriesLength, Session, SessionConfig, TableView};

/// Every card the view shows.
fn shown_cards(view: &TableView) -> CardSet {
    let mut shown: CardSet = view.hand.iter().copied().collect();
    shown |= view.own_skat.iter().flatten().copied().collect();
    shown |= view.declarer_cards.iter().flatten().copied().collect();
    shown |= view.current_trick.iter().map(|p| p.card).collect();
    if let Some(trick) = &view.last_trick {
        shown |= trick.cards.iter().map(|p| p.card).collect();
    }
    shown
}

/// Cards the human may not see right now: the opponents' hands (except the
/// revealed cards of an Ouvert declarer) and the Skat unless the human
/// discarded it.
fn hidden_cards(session: &Session) -> CardSet {
    let omniscient = session.game.omniscient_view();
    let human = PlayerId::HUMAN.seat(session.dealer);
    let mut hidden = CardSet::EMPTY;
    for seat in Seat::ALL.into_iter().filter(|&s| s != human) {
        let revealed = session.game.is_revealed() && session.game.declarer() == Some(seat);
        if !revealed {
            hidden |= omniscient.hands[seat.index()];
        }
    }
    let human_discarded = omniscient.seats[human.index()].own_skat.is_some();
    if !human_discarded {
        hidden |= omniscient.skat;
    }
    hidden
}

#[test]
fn human_view_never_shows_hidden_cards() {
    let mut checked = 0;
    for seed in 0..30 {
        let mut session = Session::new(SessionConfig {
            seed,
            series: SeriesLength::Fixed(12),
            kontra_re: true,
            opponents: Opponents::Levels([Level::Easy; 2]),
        });
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        while !session.view().series_finished {
            let view = session.view();
            assert!(shown_cards(&view).is_disjoint(hidden_cards(&session)));
            assert_eq!(view.hand.len(), usize::from(view.players[0].cards));
            checked += 1;
            let command = if view.opponent_to_move {
                Command::AdvanceOpponent
            } else if view.legal_actions.is_empty() {
                Command::NextGame
            } else {
                let i = rng.next_u32() as usize % view.legal_actions.len().min(6);
                Command::Act(view.legal_actions[i])
            };
            session.execute(command).unwrap();
        }
    }
    assert!(checked > 5000);
}

// Snapshots of own decisions (issue #47).

mod snapshots {
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::{Rng, SeedableRng};
    use skat_ai::Budget;
    use skat_core::{Action, Announcement, CardSet, Deal, Game, GameType, Phase, RuleConfig, Seat};

    use crate::coach::{CoachResult, DecisionKey};
    use crate::{
        CoachLevel, CoachSettings, Command, DecisionContext, Level, Opponents, PlayerId,
        SeriesLength, Session, SessionConfig,
    };

    /// The human sits in Vorhand: player 2 deals the first game.
    const DEALER: PlayerId = PlayerId(2);
    const HUMAN: Seat = Seat::Vorhand;
    const KONTRA_RE: RuleConfig = RuleConfig { kontra_re: true };

    /// Every card a snapshot shows.
    fn shown(c: &DecisionContext) -> CardSet {
        let mut shown: CardSet = c.hand.iter().copied().collect();
        shown |= c.own_skat.iter().flatten().copied().collect();
        shown |= c.declarer_cards.iter().flatten().copied().collect();
        shown |= c.current_trick.iter().map(|p| p.card).collect();
        for t in &c.tricks {
            shown |= t.cards.iter().map(|p| p.card).collect();
        }
        shown
    }

    /// Cards the human may not see in `game` right now.
    fn hidden(game: &Game) -> CardSet {
        let all = game.omniscient_view();
        let mut hidden = CardSet::EMPTY;
        for seat in Seat::ALL.into_iter().filter(|&s| s != HUMAN) {
            if !(game.is_revealed() && game.declarer() == Some(seat)) {
                hidden |= all.hands[seat.index()];
            }
        }
        if all.seats[HUMAN.index()].own_skat.is_none() {
            hidden |= all.skat;
        }
        hidden
    }

    /// The snapshot of the human's decision `actions` in `game`.
    fn snapshot(game: &Game, actions: usize, action: Action) -> DecisionContext {
        let key = DecisionKey { number: 1, actions };
        DecisionContext::new(key, DEALER, &game.view(HUMAN), action)
    }

    /// Plays `deal` with `choose` and returns the snapshot of every human
    /// decision with the game before it, after checking that it shows no
    /// card hidden then.
    fn play(
        deal: Deal,
        mut choose: impl FnMut(&Game) -> Action,
    ) -> (Vec<Action>, Vec<(usize, Game, DecisionContext)>) {
        let mut game = Game::from_deal(deal, KONTRA_RE);
        let (mut actions, mut decisions) = (Vec::new(), Vec::new());
        while let Some(seat) = game.current_seat() {
            let action = choose(&game);
            if seat == HUMAN {
                let c = snapshot(&game, actions.len(), action);
                assert!(shown(&c).is_disjoint(hidden(&game)), "{c:?}");
                decisions.push((actions.len(), game.clone(), c));
            }
            game.apply(action).unwrap();
            actions.push(action);
        }
        (actions, decisions)
    }

    /// First legal action that is not a pass for `bidder`; every other
    /// seat passes in the bidding. Then `announce` if legal, else the
    /// first legal action.
    fn scripted(bidder: Seat, announce: Announcement) -> impl FnMut(&Game) -> Action {
        move |game: &Game| {
            let legal = game.legal_actions();
            let seat = game.current_seat().unwrap();
            match game.phase() {
                Phase::Reizen if seat == bidder => {
                    *legal.iter().find(|a| **a != Action::Pass).unwrap()
                }
                Phase::Reizen => Action::Pass,
                Phase::SkatAufnahme => Action::TakeSkat,
                Phase::Ansage if legal.contains(&Action::Announce(announce)) => {
                    Action::Announce(announce)
                }
                _ => legal[0],
            }
        }
    }

    #[test]
    fn own_skat_shows_only_once_discarded() {
        let deal = Deal::from_seed(5);
        let grand = Announcement::plain(GameType::Grand);
        let (actions, decisions) = play(deal, scripted(HUMAN, grand));
        let discard = actions
            .iter()
            .find_map(|a| match a {
                Action::Discard(cards) => Some(*cards),
                _ => None,
            })
            .expect("the human discards");
        let mut phases = Vec::new();
        for (_, game, c) in &decisions {
            phases.push(c.phase);
            if c.phase != Phase::Reizen {
                assert_eq!(c.declarer, Some(PlayerId::HUMAN));
            }
            match c.phase {
                Phase::Reizen | Phase::SkatAufnahme => {
                    assert_eq!((c.hand.len(), &c.own_skat), (10, &None));
                }
                // The picked-up Skat is in the hand, not yet "own Skat".
                Phase::Druecken => assert_eq!((c.hand.len(), &c.own_skat), (12, &None)),
                _ => {
                    let skat: Vec<_> = c.own_skat.clone().expect("known after Drücken");
                    assert_eq!(skat.into_iter().collect::<CardSet>(), discard);
                    assert_eq!(c.hand.len(), 10 - c.tricks.len());
                }
            }
            assert_eq!(
                c.hand.iter().copied().collect::<CardSet>(),
                game.view(HUMAN).hand
            );
        }
        for phase in [Phase::Druecken, Phase::Ansage, Phase::Spiel] {
            assert!(phases.contains(&phase), "{phases:?}");
        }
    }

    #[test]
    fn ouvert_cards_show_only_once_revealed_and_as_they_lay() {
        let null_ouvert = Announcement {
            game: GameType::Null,
            schneider: false,
            schwarz: false,
            ouvert: true,
        };
        let mut checked = (0, 0);
        for seed in 0..10 {
            // The human holds once, so it may say Kontra before the reveal.
            let mut declarer = scripted(Seat::Mittelhand, null_ouvert);
            let (_, decisions) = play(Deal::from_seed(seed), |game| {
                if game.phase() == Phase::Reizen && game.current_seat() == Some(HUMAN) {
                    if game.bid() < 20 && game.legal_actions().contains(&Action::Hold) {
                        return Action::Hold;
                    }
                    return Action::Pass;
                }
                declarer(game)
            });
            for (_, game, c) in &decisions {
                assert_eq!(c.own_skat, None, "a defender never sees the Skat");
                if game.is_revealed() {
                    let open = game.omniscient_view().hands[Seat::Mittelhand.index()];
                    let shown: CardSet = c.declarer_cards.clone().unwrap().into_iter().collect();
                    assert_eq!(shown, open, "the open cards of that moment");
                    checked.1 += 1;
                } else {
                    assert_eq!(c.declarer_cards, None);
                    if c.contract.is_some_and(|k| k.ouvert) {
                        // Kontra before the reveal.
                        assert_eq!(c.phase, Phase::Spiel);
                        checked.0 += 1;
                    }
                }
            }
        }
        assert!(checked.0 > 0 && checked.1 > 0, "{checked:?}");
    }

    /// Swaps cards `a` and `b` between the places of the deal.
    fn swapped(deal: Deal, a: skat_core::Card, b: skat_core::Card) -> Deal {
        let swap = |set: CardSet| {
            let mut out = set;
            if set.contains(a) != set.contains(b) {
                for c in [a, b] {
                    if out.contains(c) {
                        out.remove(c);
                    } else {
                        out.insert(c);
                    }
                }
            }
            out
        };
        Deal::new(Seat::ALL.map(|s| swap(deal.hand(s))), swap(deal.skat())).unwrap()
    }

    #[test]
    fn concealed_cards_do_not_change_the_snapshot() {
        let mut substituted = 0;
        for seed in 0..12 {
            let deal = Deal::from_seed(seed);
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let (actions, decisions) = play(deal, |game| {
                let legal = game.legal_actions();
                legal[rng.next_u32() as usize % legal.len().min(4)]
            });
            for (at, game, c) in decisions {
                // Pairs of concealed cards dealt to different places: two
                // opponents' hands, or a hand and the Skat.
                let all = game.omniscient_view();
                let concealed: Vec<_> = hidden(&game).iter().collect();
                for (i, &a) in concealed.iter().enumerate() {
                    for &b in &concealed[i + 1..] {
                        let place = |card| {
                            Seat::ALL
                                .into_iter()
                                .position(|s| deal.hand(s).contains(card))
                        };
                        if place(a) == place(b) {
                            continue;
                        }
                        let other = swapped(deal, a, b);
                        let mut replay = Game::from_deal(other, KONTRA_RE);
                        // The same public history must stay legal.
                        if !actions[..at].iter().all(|&x| replay.apply(x).is_ok()) {
                            continue;
                        }
                        assert_ne!(replay.omniscient_view(), all, "the hidden deal differs");
                        let again = snapshot(&replay, at, c.action);
                        assert_eq!(again, c);
                        assert_eq!(
                            serde_json::to_string(&again).unwrap(),
                            serde_json::to_string(&c).unwrap()
                        );
                        substituted += 1;
                        break;
                    }
                }
            }
        }
        assert!(substituted > 50, "{substituted}");
    }

    #[test]
    fn snapshots_round_trip_through_serde() {
        let (_, decisions) = play(Deal::from_seed(3), |game| game.legal_actions()[0]);
        assert!(!decisions.is_empty());
        for (_, _, c) in decisions {
            let json = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<DecisionContext>(&json).unwrap(), c);
        }
    }

    /// Plays to the human's next decision, opponents included.
    /// `false` once the game is over.
    fn to_human(s: &mut Session) -> bool {
        s.run_opponents();
        !s.view().legal_actions.is_empty()
    }

    #[test]
    fn failed_reviews_keep_no_snapshot_and_a_new_game_drops_them_all() {
        let mut s = Session::new(SessionConfig {
            seed: 4,
            series: SeriesLength::Fixed(3),
            kontra_re: false,
            opponents: Opponents::Levels([Level::Easy; 2]),
        });
        s.set_coach_budget(Budget {
            samples: 2,
            nodes_per_task: 20_000,
        });
        s.execute(Command::SetCoach(CoachSettings {
            level: CoachLevel::EveryMove,
            ..CoachSettings::default()
        }))
        .unwrap();
        // Two own decisions played before their analyses finished.
        let mut held = Vec::new();
        while held.len() < 2 && to_human(&mut s) {
            let view = s.view();
            let job = s.coach_job().unwrap();
            assert!(!job.is_review());
            let action = view.legal_actions[0];
            held.push((job, action, view));
            s.execute(Command::Act(action)).unwrap();
        }
        assert_eq!(held.len(), 2);
        assert_eq!(s.coach.queue_len(), 2);
        // The first fails, the second finishes: only the second keeps a
        // review, with its own snapshot.
        s.finish_coach(CoachResult::failed(&held[0].0)).unwrap();
        s.finish_coach(held[1].0.run(None)).unwrap();
        assert_eq!(s.coach.queue_len(), 0);
        let reviewed: Vec<_> = s.coach.reviews.iter().map(|r| &*r.context).collect();
        let (_, action, view) = &held[1];
        match reviewed.as_slice() {
            [c] => {
                assert_eq!((c.action, &c.hand, c.bid), (*action, &view.hand, view.bid));
                assert_eq!(c.game_number, view.game_number);
            }
            other => panic!("one review expected: {other:?}"),
        }
        // Finish the game and move on: nothing of it is kept.
        while to_human(&mut s) {
            let action = s.view().legal_actions[0];
            s.execute(Command::Act(action)).unwrap();
        }
        let decisions = s.coach.reviews.len() + s.coach.queue_len();
        assert!(decisions >= 1);
        s.execute(Command::NextGame).unwrap();
        assert_eq!((s.coach.reviews.len(), s.coach.queue_len()), (0, 0));
        assert_eq!(s.view().coach.debriefing, None);
    }
}
