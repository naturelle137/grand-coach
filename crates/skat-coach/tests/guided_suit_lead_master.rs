// SPDX-License-Identifier: EUPL-1.2

//! #182: non-Ouvert Farbspiel own empty-trick `LeadMaster` lessons and boundaries.
#[allow(dead_code)]
#[path = "../../skat-analysis/tests/suit_lead_master_policy.rs"]
mod fixture;
use fixture::{card, cases, deal, play};
use skat_coach::{
    Source,
    guided::{Answer, Step, lesson},
};
use skat_core::{Action, GameType, Seat, Suit};

#[test]
fn suit_lead_master_lessons_admit_the_shared_branch_with_every_legal_choice() {
    for c in cases() {
        let v = c.game.view(Seat::Vorhand);
        let l = lesson(&v).unwrap_or_else(|| panic!("{}", c.id));
        assert_eq!(l.rule.card, card(c.card), "{}", c.id);
        assert_eq!(
            l.source,
            Source::Lead {
                index: v.tricks.len() as u8,
                seat: Seat::Vorhand
            }
        );
        assert_eq!(l.lead_declarer, Some(false));
        assert!(l.check(
            Step::Orient,
            Answer::Lead {
                seat: Seat::Vorhand,
                declarer: false
            }
        ));
        assert!(!l.check(
            Step::Orient,
            Answer::Lead {
                seat: Seat::Vorhand,
                declarer: true
            }
        ));
        for a in &v.legal_actions {
            let Action::Play(x) = a else { panic!() };
            assert_eq!(
                l.check(Step::Check, Answer::Card(*x)),
                c.candidates.contains(&x.index())
            );
            assert_eq!(l.check(Step::Narrow, Answer::Card(*x)), x.index() == c.card);
        }
        // No hidden holder is assigned.
        assert_eq!(l.memory().skat, v.own_skat);
    }
}

#[test]
fn suit_lead_master_boundaries_stay_excluded() {
    let c = &cases()[0];
    let v = c.game.view(Seat::Vorhand);
    type Edit = fn(&mut skat_core::PlayerView);
    let edits: &[Edit] = &[
        |v| v.legal_actions.reverse(),
        |v| v.legal_actions.truncate(1),
        |v| v.legal_actions.push(Action::Kontra),
        |v| v.current_trick.cards[1] = Some(card(1)),
        |v| v.current_seat = Some(Seat::Hinterhand),
        |v| v.current_seat = None,
        |v| v.declarer = None,
        |v| v.hand_game = None,
        |v| v.own_skat = Some(fixture::set(&[17, 29])),
        |v| v.contract.as_mut().unwrap().ouvert = true,
        |v| v.contract = None,
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut x = v.clone();
        edit(&mut x);
        assert!(lesson(&x).is_none(), "edit{i}");
    }
    let mut null = v.clone();
    null.contract.as_mut().unwrap().game = GameType::Null;
    assert!(lesson(&null).is_none_or(|l| l.rule.branch != skat_coach::guided::Branch::LeadMaster));
}

#[test]
fn suit_lead_master_declarer_with_trumpf_keeps_draw_trumpf_priority() {
    // Declarer Vorhand (Pass, Pass, Bid) holds Herz trumps while others remain out: DrawTrumpf.
    let g = play(
        deal(&[3, 7, 4, 11, 14, 31, 26, 16, 23, 12], &[], &[]),
        Suit::Herz,
        [Action::Pass, Action::Pass, Action::Bid(18)],
        &[],
    );
    assert_eq!(g.current_seat(), Some(Seat::Vorhand));
    assert_eq!(
        lesson(&g.view(Seat::Vorhand)).unwrap().rule.branch,
        skat_coach::guided::Branch::DrawTrumpf
    );
}

#[test]
fn suit_lead_master_declarer_without_trumpf_is_a_lead_master_lesson() {
    // Declarer Vorhand with no Trumpf at all: plain masters KaroA/KreuzA/Pik cards.
    let g = play(
        deal(&[7, 3, 15, 11, 14, 31, 26, 25, 5, 10], &[], &[]),
        Suit::Herz,
        [Action::Pass, Action::Pass, Action::Bid(18)],
        &[],
    );
    let l = lesson(&g.view(Seat::Vorhand)).expect("declarer lead");
    assert_eq!(l.lead_declarer, Some(true));
    assert_eq!(l.rule.card, card(31)); // CA/PikA/KaroA all 11 Augen: last in core order
}

#[test]
fn ouvert_declarer_lead_uses_the_same_permitted_master_rule() {
    use skat_analysis::baseline::{self, Branch, TieBreak};
    use skat_core::{Announcement, Game, RuleConfig};
    let d = deal(
        &[7, 3, 15, 11, 14, 31, 26, 25, 5, 10],
        &[24],
        &[16, 17, 18, 19, 20, 21, 22, 23, 0, 1],
    );
    let mut g = Game::from_deal(d, RuleConfig { kontra_re: false });
    for a in [
        Action::Pass,
        Action::Pass,
        Action::Bid(18),
        Action::Hand,
        Action::Announce(Announcement {
            game: GameType::Farbe(Suit::Herz),
            schneider: true,
            schwarz: true,
            ouvert: true,
        }),
    ] {
        g.apply(a).unwrap();
    }
    let v = g.view(Seat::Vorhand);
    let l = lesson(&v).expect("Ouvert declarer lead");
    assert_eq!(l.rule, baseline::rule(&v).unwrap());
    assert_eq!(l.rule.branch, Branch::LeadMaster);
    assert_eq!(l.rule.tie_break, TieBreak::HighestAugenLast);
    assert_eq!(l.rule.card, card(31));
    assert!(l.rule.candidates.len() > 1);
    for a in &v.legal_actions {
        let Action::Play(c) = a else { panic!() };
        assert_eq!(
            l.check(Step::Check, Answer::Card(*c)),
            l.rule.candidates.contains(c)
        );
    }
    let mut restricted = v.clone();
    restricted.legal_actions.truncate(1);
    assert!(lesson(&restricted).is_none());
    let mut unrevealed = v.clone();
    unrevealed.declarer_cards = None;
    assert!(lesson(&unrevealed).is_none());
    let mut invalid = v.clone();
    invalid.contract.as_mut().unwrap().schwarz = false;
    assert!(lesson(&invalid).is_none());
    for n in [31, 24, 16] {
        g.apply(Action::Play(card(n))).unwrap();
    }
    assert_eq!(g.tricks().last().unwrap().winner, Seat::Hinterhand);
}

#[test]
fn ouvert_grand_declarer_lead_uses_shared_rule() {
    use skat_analysis::baseline::{self, Branch};
    use skat_core::{Announcement, Game, RuleConfig};
    let d = deal(
        &[7, 3, 15, 11, 14, 31, 26, 25, 5, 10],
        &[24],
        &[16, 17, 18, 19, 20, 21, 22, 23, 0, 1],
    );
    let mut g = Game::from_deal(d, RuleConfig { kontra_re: false });
    for a in [
        Action::Pass,
        Action::Pass,
        Action::Bid(18),
        Action::Hand,
        Action::Announce(Announcement {
            game: GameType::Grand,
            schneider: true,
            schwarz: true,
            ouvert: true,
        }),
    ] {
        g.apply(a).unwrap();
    }
    let v = g.view(Seat::Vorhand);
    let l = lesson(&v).expect("Grand Ouvert declarer lead");
    assert_eq!(l.rule, baseline::rule(&v).unwrap());
    assert_eq!(l.rule.branch, Branch::LeadMaster);
    assert_eq!(l.rule.card, card(31));
    for a in &v.legal_actions {
        let Action::Play(c) = a else { panic!() };
        assert_eq!(
            l.check(Step::Check, Answer::Card(*c)),
            l.rule.candidates.contains(c)
        );
    }
    for edit in [
        (|v: &mut skat_core::PlayerView| v.declarer_cards = None) as fn(&mut skat_core::PlayerView),
        |v| v.contract.as_mut().unwrap().schwarz = false,
        |v| v.legal_actions.truncate(1),
    ] {
        let mut invalid = v.clone();
        edit(&mut invalid);
        assert!(lesson(&invalid).is_none());
    }
}

#[test]
fn ouvert_grand_defender_lead_requires_the_revealed_hand_and_exact_actions() {
    use skat_analysis::baseline::{self, Branch};
    use skat_core::{Announcement, Game, RuleConfig};
    let d = cases().remove(0).deal;
    let mut g = Game::from_deal(d, RuleConfig { kontra_re: false });
    for a in [
        Action::Bid(18),
        Action::Pass,
        Action::Pass,
        Action::Hand,
        Action::Announce(Announcement {
            game: GameType::Grand,
            schneider: true,
            schwarz: true,
            ouvert: true,
        }),
    ] {
        g.apply(a).unwrap();
    }
    let v = g.view(Seat::Vorhand);
    let l = lesson(&v).expect("Grand Ouvert defender lead");
    assert_eq!(l.rule, baseline::rule(&v).unwrap());
    assert_eq!(l.rule.branch, Branch::LeadMaster);
    assert_eq!(l.lead_declarer, Some(false));
    for edit in [
        (|v: &mut skat_core::PlayerView| v.declarer_cards = None) as fn(&mut skat_core::PlayerView),
        |v| v.contract.as_mut().unwrap().schwarz = false,
        |v| v.legal_actions.truncate(1),
    ] {
        let mut invalid = v.clone();
        edit(&mut invalid);
        assert!(lesson(&invalid).is_none());
    }
}
