// SPDX-License-Identifier: EUPL-1.2
//! Default-feature locked target, independent physical predictions separated
//! from the accepted post-freeze source-informed API observations.
#[path = "common/suit_feed_partner.rs"]
mod fixture;
use skat_analysis::{Knowledge, baseline};
use skat_core::*;

#[test]
fn full_legal_histories_views_knowledge_eleven_rule_fields_and_all_37_completions() {
    let mut count = 0;
    let mut terminals = 0;
    for c in fixture::cases() {
        let g = c.game();
        let v = g.view(c.view.seat);
        assert_eq!(v, c.view, "{} entire view", c.id);
        assert_eq!(Knowledge::from_view(&v), c.knowledge, "{}", c.id);
        let r = baseline::rule(&v).unwrap();
        assert_eq!(r, c.rule, "{} all eleven rule fields", c.id);
        assert_eq!(r.branch, baseline::Branch::FeedPartner);
        assert!(r.last && r.partner_ahead);
        let legal: Vec<_> = v
            .legal_actions
            .iter()
            .map(|a| {
                let Action::Play(c) = a else {
                    panic!("pure Play required")
                };
                *c
            })
            .collect();
        assert_eq!(r.candidates, legal);
        let game = v.contract.unwrap().game;
        // max_by_key retains the LAST exact tie. No power component.
        let selected = legal
            .iter()
            .copied()
            .max_by_key(|&c| (!game.is_trump(c), c.augen()));
        assert_eq!(Some(r.card), selected);
        assert_eq!(r.tie_break, baseline::TieBreak::FeedLast);
        assert_eq!(
            r.tie_key,
            (
                i32::from(!game.is_trump(r.card)),
                i32::from(r.card.augen()),
                0
            )
        );
        let table = skat_analysis::TrickState::of(game, &v.current_trick);
        let winners: Vec<_> = legal
            .iter()
            .copied()
            .filter(|&n| game.beats(n, table.best.unwrap(), table.led.unwrap()))
            .collect();
        let losers: Vec<_> = legal
            .iter()
            .copied()
            .filter(|n| !winners.contains(n))
            .collect();
        assert_eq!(r.winners, winners);
        assert_eq!(r.losers, losers);
        assert_eq!(c.completions.iter().map(|p| p.0).collect::<Vec<_>>(), legal);
        for (card, winner) in c.completions {
            let mut done = g.clone();
            done.apply(Action::Play(card)).unwrap();
            let t = done.tricks().last().unwrap();
            assert_eq!(
                t.cards,
                [
                    v.current_trick.cards[0].unwrap(),
                    v.current_trick.cards[1].unwrap(),
                    card
                ]
            );
            assert_eq!(t.winner, winner, "{} card {}", c.id, card.index());
            count += 1;
        }
        for (own, final_cards, winner) in c.terminal {
            let mut done = g.clone();
            done.apply(Action::Play(own)).unwrap();
            for card in final_cards {
                done.apply(Action::Play(card)).unwrap();
            }
            assert_eq!(done.phase(), Phase::Beendet);
            assert_eq!(done.tricks().len(), 10);
            assert_eq!(done.tricks().last().unwrap().winner, winner);
            terminals += 1;
        }
    }
    assert_eq!(count, 37);
    assert_eq!(terminals, 4);
}

#[test]
fn both_positive_hidden_pairs_are_entire_view_equal_and_f_tie_has_no_power_key() {
    let cs = fixture::cases();
    for (a, b) in [(0, 6), (3, 7)] {
        assert_ne!(cs[a].deal, cs[b].deal);
        assert_eq!(cs[a].view, cs[b].view);
        assert_eq!(cs[a].knowledge, cs[b].knowledge);
        assert_eq!(cs[a].rule, cs[b].rule);
    }
    let f = &cs[5];
    assert_eq!(f.rule.candidates, vec![fixture::card(4), fixture::card(12)]);
    assert_eq!(f.rule.card, fixture::card(12)); // Both 2 Augen, distinct Buben powers.
    assert!(f.completions.iter().all(|p| p.1 == Seat::Mittelhand)); // Actual partner overtake.
}

#[test]
fn full_api_mutations_preserve_actual_baseline_observations_and_knowledge() {
    let expected = fixture::mutation_observations();
    for (id, v, _) in fixture::mutations() {
        let (_, k, r) = expected.iter().find(|e| e.0 == id).unwrap();
        assert_eq!(&Knowledge::from_view(&v), k, "{id}");
        if id == "missingdeclarer" {
            continue;
        } // Observed panic, never successful API rejection.
        assert_eq!(&baseline::rule(&v), r, "{id}");
    }
}
