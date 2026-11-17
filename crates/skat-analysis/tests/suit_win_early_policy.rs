// SPDX-License-Identifier: EUPL-1.2

//! Locked default-feature policy regression. Physical predictions and disclosed
//! source-informed calibration remain distinct in the accepted evidence.
#[path = "common/suit_win_early.rs"]
mod fixture;
use skat_analysis::{Knowledge, baseline};
use skat_core::*;

#[test]
fn accepted_full_histories_views_knowledge_eleven_rule_fields_and_all_356_completions() {
    let mut count = 0;
    for c in fixture::cases() {
        let g = c.game();
        let v = g.view(c.view.seat);
        assert_eq!(v, c.view, "{} whole permitted view", c.id);
        assert_eq!(Knowledge::from_view(&v), c.knowledge, "{}", c.id);
        let r = baseline::rule(&v).unwrap();
        assert_eq!(r, c.rule, "{} all eleven fields", c.id);
        let game = v.contract.unwrap().game;
        let led = v.current_trick.led().unwrap();
        let winners: Vec<_> = v
            .legal_actions
            .iter()
            .filter_map(|a| {
                let Action::Play(card) = a else {
                    panic!("not pure")
                };
                game.beats(*card, led, led).then_some(*card)
            })
            .collect();
        assert_eq!(r.winners, winners);
        if r.branch == baseline::Branch::WinEarly {
            assert_eq!(r.candidates, winners);
            assert_eq!(
                Some(r.card),
                winners
                    .iter()
                    .copied()
                    .min_by_key(|&c| (game.is_trump(c), skat_analysis::hand::power(game, c)))
            );
            assert!(game.is_trump(r.card) || c.knowledge.is_master(r.card));
        }
        let mut actual = Vec::new();
        for a in g.legal_actions() {
            let mut second = g.clone();
            second.apply(a).unwrap();
            for reply in second.legal_actions() {
                let mut done = second.clone();
                done.apply(reply).unwrap();
                let t = done.tricks().last().unwrap();
                actual.push((t.cards, t.winner, t.augen()));
            }
        }
        assert_eq!(
            actual, c.paths,
            "{} complete legal own × third partition",
            c.id
        );
        count += actual.len();
    }
    assert_eq!(count, 356);
}

#[test]
fn positive_hidden_pairs_are_whole_view_equal_and_do_not_guarantee_the_trick() {
    let cs = fixture::cases();
    for (a, b, counts) in [(6, 7, (5, 5)), (8, 9, (3, 4))] {
        assert_eq!(cs[a].view, cs[b].view);
        assert_eq!(cs[a].knowledge, cs[b].knowledge);
        assert_eq!(cs[a].rule, cs[b].rule);
        for (i, n) in [(a, counts.0), (b, counts.1)] {
            let selected: Vec<_> = cs[i]
                .paths
                .iter()
                .filter(|p| p.0[1] == cs[i].rule.card)
                .collect();
            assert_eq!(selected.len(), 10);
            assert_eq!(
                selected.iter().filter(|p| p.1 == cs[i].view.seat).count(),
                n
            );
        }
    }
}

#[test]
fn accepted_four_late_terminal_paths_preserve_individual_tricks_without_numeric_settlement_claim() {
    let cs = fixture::cases();
    let c = &cs[5];
    {
        let mut g = c.game();
        g.apply(Action::Play(fixture::card(12))).unwrap();
        g.apply(Action::Play(fixture::card(21))).unwrap();
        g.apply(Action::Play(fixture::card(20))).unwrap();
        g.apply(Action::Play(fixture::card(26))).unwrap();
        g.apply(Action::Play(fixture::card(24))).unwrap();
        assert_eq!(g.phase(), Phase::Beendet);
        let t = &g.tricks()[g.tricks().len() - 2];
        assert_eq!(t.leader, Seat::Vorhand);
        assert_eq!(t.cards.map(|c| c.index()), [0, 12, 21]);
        assert_eq!(t.winner, Seat::Mittelhand);
        let t = &g.tricks()[g.tricks().len() - 1];
        assert_eq!(t.leader, Seat::Mittelhand);
        assert_eq!(t.cards.map(|c| c.index()), [20, 26, 24]);
        assert_eq!(t.winner, Seat::Mittelhand);
    }
    {
        let mut g = c.game();
        g.apply(Action::Play(fixture::card(12))).unwrap();
        g.apply(Action::Play(fixture::card(26))).unwrap();
        g.apply(Action::Play(fixture::card(20))).unwrap();
        g.apply(Action::Play(fixture::card(21))).unwrap();
        g.apply(Action::Play(fixture::card(24))).unwrap();
        assert_eq!(g.phase(), Phase::Beendet);
        let t = &g.tricks()[g.tricks().len() - 2];
        assert_eq!(t.leader, Seat::Vorhand);
        assert_eq!(t.cards.map(|c| c.index()), [0, 12, 26]);
        assert_eq!(t.winner, Seat::Mittelhand);
        let t = &g.tricks()[g.tricks().len() - 1];
        assert_eq!(t.leader, Seat::Mittelhand);
        assert_eq!(t.cards.map(|c| c.index()), [20, 21, 24]);
        assert_eq!(t.winner, Seat::Mittelhand);
    }
    {
        let mut g = c.game();
        g.apply(Action::Play(fixture::card(20))).unwrap();
        g.apply(Action::Play(fixture::card(21))).unwrap();
        g.apply(Action::Play(fixture::card(12))).unwrap();
        g.apply(Action::Play(fixture::card(26))).unwrap();
        g.apply(Action::Play(fixture::card(24))).unwrap();
        assert_eq!(g.phase(), Phase::Beendet);
        let t = &g.tricks()[g.tricks().len() - 2];
        assert_eq!(t.leader, Seat::Vorhand);
        assert_eq!(t.cards.map(|c| c.index()), [0, 20, 21]);
        assert_eq!(t.winner, Seat::Mittelhand);
        let t = &g.tricks()[g.tricks().len() - 1];
        assert_eq!(t.leader, Seat::Mittelhand);
        assert_eq!(t.cards.map(|c| c.index()), [12, 26, 24]);
        assert_eq!(t.winner, Seat::Mittelhand);
    }
    {
        let mut g = c.game();
        g.apply(Action::Play(fixture::card(20))).unwrap();
        g.apply(Action::Play(fixture::card(26))).unwrap();
        g.apply(Action::Play(fixture::card(12))).unwrap();
        g.apply(Action::Play(fixture::card(21))).unwrap();
        g.apply(Action::Play(fixture::card(24))).unwrap();
        assert_eq!(g.phase(), Phase::Beendet);
        let t = &g.tricks()[g.tricks().len() - 2];
        assert_eq!(t.leader, Seat::Vorhand);
        assert_eq!(t.cards.map(|c| c.index()), [0, 20, 26]);
        assert_eq!(t.winner, Seat::Mittelhand);
        let t = &g.tricks()[g.tricks().len() - 1];
        assert_eq!(t.leader, Seat::Mittelhand);
        assert_eq!(t.cards.map(|c| c.index()), [12, 21, 24]);
        assert_eq!(t.winner, Seat::Mittelhand);
    }
}
