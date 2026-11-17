// SPDX-License-Identifier: EUPL-1.2

use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, coach, i18n::I18n};
use skat_app::{Action, GuidedStep, Phase};

pub fn click(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(4);
}

pub fn settle(h: &mut Harness<'_, GrandCoachApp>) {
    for _ in 0..5000 {
        h.run_steps(1);
        if !h.state().session().view().opponent_to_move {
            h.run_steps(3);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("opponents did not settle");
}

/// Ordinary app, real legal widget moves, no injected session or mock view.
pub fn enter(h: &mut Harness<'_, GrandCoachApp>) {
    let i = I18n::new(Language::De);
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(skat_app::Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    for _ in 0..300 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == Phase::Spiel && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            if g.step == GuidedStep::Orient {
                return;
            }
            click(h, &i.t("guided-return"));
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a supported guided branch");
}

/// Ordinary app up to the human's next Reizen decision; enters the dialogue
/// by its real button in `language`.
#[allow(dead_code)]
pub fn enter_reizen(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    for _ in 0..300 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == Phase::Reizen && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            assert_eq!(g.step, GuidedStep::Orient);
            assert!(g.reizen.is_some());
            return;
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human Reizen decision");
}

/// Ordinary app: the human holds or bids until declarer, picks up for
/// `Phase::Druecken`, and enters the dialogue at `phase` by its real button.
#[allow(dead_code)]
pub fn enter_preplay(h: &mut Harness<'_, GrandCoachApp>, language: Language, phase: Phase) {
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    for _ in 0..600 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == phase && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            assert_eq!(g.step, GuidedStep::Orient);
            assert_eq!(g.preplay.expect("pre-play dialogue").phase, phase);
            return;
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        let bid = v.legal_actions.iter().find_map(|a| match a {
            Action::Bid(value) => Some(*value),
            _ => None,
        });
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Hold) {
            i.tf("button-hold", &[("value", v.bid.into())])
        } else if let Some(value) = bid {
            i.tf("button-bid", &[("value", value.into())])
        } else if v.legal_actions.contains(&Action::TakeSkat) {
            i.t("button-take-skat")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human {phase:?} decision");
}

/// Ordinary app: the human holds or bids until declarer, plays Hand
/// (`hand`) or picks up and puts away its first two cards, and enters the
/// Ansage dialogue by its real button.
#[allow(dead_code)]
pub fn enter_ansage(h: &mut Harness<'_, GrandCoachApp>, language: Language, hand: bool) {
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    for _ in 0..600 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == Phase::Ansage && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            assert_eq!(g.step, GuidedStep::Orient);
            assert_eq!(g.preplay.expect("pre-play dialogue").phase, Phase::Ansage);
            return;
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        if v.phase == Phase::Druecken && !v.legal_actions.is_empty() {
            for &card in &v.hand[..2] {
                click(h, &card_name(&i, card));
            }
            click(h, &i.t("button-discard"));
            continue;
        }
        let bid = v.legal_actions.iter().find_map(|a| match a {
            Action::Bid(value) => Some(*value),
            _ => None,
        });
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Hold) {
            i.tf("button-hold", &[("value", v.bid.into())])
        } else if let Some(value) = bid {
            i.tf("button-bid", &[("value", value.into())])
        } else if v.legal_actions.contains(&Action::TakeSkat) {
            i.t(if hand {
                "button-hand"
            } else {
                "button-take-skat"
            })
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human Ansage decision");
}

/// Card button label on the default French faces.
#[allow(dead_code)]
pub fn card_name(i: &I18n, card: skat_app::Card) -> String {
    let suit =
        ["suit-kreuz", "suit-pik", "suit-herz", "suit-karo"][usize::from(card.suit().index())];
    let rank = [
        "rank-sieben",
        "rank-acht",
        "rank-neun",
        "rank-zehn",
        "rank-bube",
        "rank-dame",
        "rank-koenig",
        "rank-ass",
    ][usize::from(card.rank().index())];
    i.tf(
        "card-name",
        &[("suit", i.t(suit).into()), ("rank", i.t(rank).into())],
    )
}

/// Announcement radio label on the default French faces.
#[allow(dead_code)]
pub fn game_name(i: &I18n, game: skat_app::GameType) -> String {
    use skat_app::{GameType, Suit};
    i.t(match game {
        GameType::Farbe(Suit::Kreuz) => "suit-kreuz",
        GameType::Farbe(Suit::Pik) => "suit-pik",
        GameType::Farbe(Suit::Herz) => "suit-herz",
        GameType::Farbe(Suit::Karo) => "suit-karo",
        GameType::Grand => "game-grand",
        GameType::Null => "game-null",
    })
}

/// Ordinary app with Kontra/Re on, driven only by real widgets, up to the
/// human's first Kontra (`re`: false) or immediate Re decision in a
/// non-Ouvert Farbspiel; enters the dialogue by its real button.
///
/// As a defender (`re`: false) the human holds or bids 18 only, so it may
/// say Kontra after losing the bidding. As declarer (`re`: true) it holds
/// or bids up to 40, picks up, puts away its first two cards and announces
/// the suit it holds fewest of, so that a defender may say Kontra. Every
/// other move is the first legal action.
#[allow(dead_code)]
pub fn enter_kontra(h: &mut Harness<'_, GrandCoachApp>, language: Language, re: bool) {
    enter_kontra_in(h, language, re, KontraAt::Farbe);
}

/// Which contract's first human Kontra/Re offer [`enter_kontra_in`] stops at.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KontraAt {
    /// Non-Ouvert Farbspiel (guided).
    Farbe,
    /// Non-Ouvert Grand (guided).
    Grand,
    /// Non-Ouvert Null (guided).
    Null,
    /// Any Ouvert contract, including the round before the reveal (fallback).
    Ouvert,
}

/// As [`enter_kontra`], stopping at the first offer in the contract `at`;
/// Ouvert offers get the honest fallback, the others the dialogue.
#[allow(dead_code)]
pub fn enter_kontra_in(
    h: &mut Harness<'_, GrandCoachApp>,
    language: Language,
    re: bool,
    at: KontraAt,
) {
    use skat_app::{Announcement, GameType, Suit};
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    let limit = if re { 40 } else { 18 };
    let wanted = if re { Action::Re } else { Action::Kontra };
    for _ in 0..1500 {
        settle(h);
        let v = h.state().session().view();
        let here = v.contract.is_some_and(|c| match at {
            _ if c.ouvert => at == KontraAt::Ouvert,
            KontraAt::Farbe => matches!(c.game, GameType::Farbe(_)),
            KontraAt::Grand => c.game == GameType::Grand,
            KontraAt::Null => c.game == GameType::Null,
            KontraAt::Ouvert => false,
        });
        if here && v.legal_actions.contains(&wanted) {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            if at != KontraAt::Ouvert {
                assert_eq!(g.step, GuidedStep::Orient);
                assert!(g.kontra.is_some(), "Kontra/Re dialogue");
            } else {
                assert_eq!(g.step, GuidedStep::Unsupported);
                assert!(g.kontra_exclusion.is_some(), "honest exclusion");
            }
            return;
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        if v.phase == Phase::Druecken && !v.legal_actions.is_empty() {
            for &card in &v.hand[..2] {
                click(h, &card_name(&i, card));
            }
            click(h, &i.t("button-discard"));
            continue;
        }
        if v.phase == Phase::Ansage && !v.legal_actions.is_empty() {
            let suit = Suit::ALL
                .into_iter()
                .min_by_key(|&s| v.hand.iter().filter(|c| c.suit() == s).count())
                .unwrap();
            let game = GameType::Farbe(suit);
            assert!(
                v.legal_actions
                    .contains(&Action::Announce(Announcement::plain(game)))
            );
            h.get_by_role_and_label(Role::RadioButton, &game_name(&i, game))
                .click();
            h.run_steps(4);
            click(h, &i.t("button-announce"));
            continue;
        }
        let bid = v.legal_actions.iter().find_map(|a| match a {
            Action::Bid(value) if *value <= limit => Some(*value),
            _ => None,
        });
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if v.legal_actions.contains(&Action::Hold) && v.bid <= limit {
            i.tf("button-hold", &[("value", v.bid.into())])
        } else if let Some(value) = bid {
            i.tf("button-bid", &[("value", value.into())])
        } else if v.legal_actions.contains(&Action::TakeSkat) {
            i.t("button-take-skat")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human Kontra/Re decision");
}

/// Seat name as used by the guided texts.
#[allow(dead_code)]
pub fn seat_name(i: &I18n, seat: skat_app::Seat) -> String {
    i.t(match seat {
        skat_app::Seat::Vorhand => "seat-vorhand",
        skat_app::Seat::Mittelhand => "seat-mittelhand",
        skat_app::Seat::Hinterhand => "seat-hinterhand",
    })
}

/// Ordinary app (Kontra/Re off), real widgets only, up to the human's first
/// card decision in a non-Ouvert Grand whose dialogue is the supported
/// `target` branch (orientation) or, for `None`, the honest fallback; enters
/// it by its real button. Public trick length and party roles distinguish
/// WinLast from the two middle-seat branches: UnderPartner (partner ahead)
/// and WinEarly (opposing party ahead). No hidden task or rule is revealed
/// to select the route; callers assert the task after orientation.
///
/// As a defender (`declare`: false) the human passes every bid. As declarer
/// it holds or bids up to 30, picks up, puts away its first two cards and
/// announces Grand. Every other move is the first legal action; other
/// decisions' dialogues are entered and returned unchanged.
#[allow(dead_code)]
pub fn enter_grand(
    h: &mut Harness<'_, GrandCoachApp>,
    language: Language,
    declare: bool,
    target: Option<skat_app::GuidedBranch>,
) {
    use skat_app::{GameType, GuidedBranch};
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(skat_app::Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    for _ in 0..600 {
        settle(h);
        let v = h.state().session().view();
        if v.phase == Phase::Spiel && !v.legal_actions.is_empty() {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            let grand = v
                .contract
                .is_some_and(|c| c.game == GameType::Grand && !c.ouvert);
            let branch = match v.current_trick.len() {
                1 if v.declarer != Some(skat_app::PlayerId::HUMAN)
                    && v.declarer != Some(v.current_trick[0].player) =>
                {
                    Some(GuidedBranch::UnderPartner)
                }
                1 => Some(GuidedBranch::WinEarly),
                2 => Some(GuidedBranch::WinLast),
                _ => None,
            };
            let reached = match target {
                Some(t) => g.step == GuidedStep::Orient && branch == Some(t),
                None => g.step == GuidedStep::Unsupported,
            };
            if grand && reached && g.kontra.is_none() {
                return;
            }
            click(h, &i.t("guided-return"));
        }
        assert!(!v.series_finished, "guided walkthrough needs free play");
        if declare && v.phase == Phase::Druecken && !v.legal_actions.is_empty() {
            for &card in &v.hand[..2] {
                click(h, &card_name(&i, card));
            }
            click(h, &i.t("button-discard"));
            continue;
        }
        if declare && v.phase == Phase::Ansage && !v.legal_actions.is_empty() {
            h.get_by_role_and_label(Role::RadioButton, &game_name(&i, GameType::Grand))
                .click();
            h.run_steps(4);
            click(h, &i.t("button-announce"));
            continue;
        }
        let bid = v.legal_actions.iter().find_map(|a| match a {
            Action::Bid(value) if declare && *value <= 30 => Some(*value),
            _ => None,
        });
        let label = if v.can_next_game {
            i.t("button-next-game")
        } else if declare && v.legal_actions.contains(&Action::Hold) && v.bid <= 30 {
            i.tf("button-hold", &[("value", v.bid.into())])
        } else if let Some(value) = bid {
            i.tf("button-bid", &[("value", value.into())])
        } else if declare && v.legal_actions.contains(&Action::TakeSkat) {
            i.t("button-take-skat")
        } else if v.legal_actions.contains(&Action::Pass) {
            i.t("button-pass")
        } else {
            coach::action(&i, &v, v.legal_actions[0])
        };
        click(h, &label);
    }
    panic!("seeded ordinary app did not reach a human Grand card decision");
}

/// Frozen ordinary #161 route: seed0/game3/trick2, MH DECLARER only.
/// All navigation uses actual widgets. No fixture or privileged state injection.
#[allow(dead_code)]
pub fn enter_early(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    use skat_app::*;
    h.state_mut().opponent_delay = 0.0;
    h.run_steps(4);
    click(h, "Einstellungen");
    if language == Language::En {
        h.get_by_role_and_label(Role::RadioButton, "English")
            .click();
        h.run_steps(4);
    }
    let i = I18n::new(language);
    if language == Language::En {
        // Selecting a language closes the Settings menu.
        click(h, &i.t("menu-settings"));
    }
    h.get_by_role_and_label(Role::RadioButton, &i.t("coach-level-off"))
        .click();
    h.run_steps(4);
    h.key_press(eframe::egui::Key::Escape);
    h.run_steps(4);
    let actions = [
        Some(Action::Hold),
        Some(Action::Hold),
        Some(Action::Hold),
        Some(Action::Hold),
        Some(Action::Hold),
        Some(Action::Hold),
        Some(Action::Hold),
        Some(Action::Pass),
        Some(Action::Play(Card::from_index(3).unwrap())),
        Some(Action::Play(Card::from_index(4).unwrap())),
        Some(Action::Play(Card::from_index(5).unwrap())),
        Some(Action::Play(Card::from_index(12).unwrap())),
        Some(Action::Play(Card::from_index(13).unwrap())),
        Some(Action::Play(Card::from_index(16).unwrap())),
        Some(Action::Play(Card::from_index(24).unwrap())),
        Some(Action::Play(Card::from_index(26).unwrap())),
        Some(Action::Play(Card::from_index(30).unwrap())),
        Some(Action::Play(Card::from_index(28).unwrap())),
        None,
        Some(Action::Bid(24)),
        Some(Action::Bid(27)),
        Some(Action::Hand),
        Some(Action::Announce(Announcement::plain(GameType::Grand))),
        Some(Action::Play(Card::from_index(26).unwrap())),
        Some(Action::Play(Card::from_index(17).unwrap())),
        Some(Action::Play(Card::from_index(11).unwrap())),
        Some(Action::Play(Card::from_index(29).unwrap())),
        Some(Action::Play(Card::from_index(14).unwrap())),
        Some(Action::Play(Card::from_index(1).unwrap())),
        Some(Action::Play(Card::from_index(6).unwrap())),
        Some(Action::Play(Card::from_index(18).unwrap())),
        Some(Action::Play(Card::from_index(19).unwrap())),
        Some(Action::Play(Card::from_index(30).unwrap())),
        None,
        Some(Action::Bid(18)),
        Some(Action::Bid(20)),
        Some(Action::Bid(22)),
        Some(Action::Bid(23)),
        Some(Action::Bid(24)),
        Some(Action::Bid(27)),
        Some(Action::Hand),
        Some(Action::Announce(Announcement::plain(GameType::Grand))),
        Some(Action::Play(Card::from_index(0).unwrap())),
    ];
    for (n, action) in actions.into_iter().enumerate() {
        settle(h);
        let v = h.state().session().view();
        let label = match action {
            None => i.t("button-next-game"),
            Some(Action::Announce(_)) => {
                h.get_by_role_and_label(Role::RadioButton, "Grand").click();
                h.run_steps(4);
                i.t("button-announce")
            }
            Some(Action::Hold) => i.tf("button-hold", &[("value", v.bid.into())]),
            Some(Action::Bid(value)) => i.tf("button-bid", &[("value", value.into())]),
            Some(Action::Pass) => i.t("button-pass"),
            Some(Action::Hand) => i.t("button-hand"),
            Some(Action::Play(card)) => card_name(&i, card),
            Some(a) => panic!("unexpected route action {n}: {a:?}"),
        };

        click(h, &label);
    }
    settle(h);
    let v = h.state().session().view();
    assert_eq!(v.game_number, 3);
    assert_eq!(v.tricks_played, 1);
    assert_eq!(v.players[0].seat, Seat::Mittelhand);
    assert_eq!(v.contract.unwrap().game, GameType::Grand);
    assert_eq!(h.state().session().game_seed(), 12896916468484187878);

    assert_eq!(v.declarer, Some(PlayerId::HUMAN));
    click(h, &i.t("menu-settings"));
    h.get_by_role_and_label(Role::RadioButton, &i.t("coach-level-on-request"))
        .click();
    h.run_steps(4);
    h.key_press(eframe::egui::Key::Escape);
    h.run_steps(4);
    click(h, &i.t("guided-start"));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Orient
    );
}

/// Frozen ordinary #163 route: seed15/fixed3/twoEasy, closed, Kontra/Re off.
/// Human initial VH defends LAST to trick3. No injected session or view.
#[allow(dead_code)]
pub fn enter_feed(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    use skat_app::*;
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    let i = I18n::new(language);
    assert_eq!(
        h.state().session().coach_settings(),
        CoachSettings::default()
    );
    for action in [
        Action::Pass,
        Action::Play(Card::from_index(2).unwrap()),
        Action::Play(Card::from_index(5).unwrap()),
    ] {
        settle(h);
        assert!(h.state().session().view().legal_actions.contains(&action));
        click(
            h,
            &match action {
                Action::Play(c) => card_name(&i, c),
                _ => i.t("button-pass"),
            },
        );
    }
    settle(h);
    let v = h.state().session().view();
    assert_eq!((v.game_number, v.tricks_played), (1, 2));
    assert_eq!(
        v.players[0].seat,
        Seat::Vorhand,
        "initial seat, not trick order"
    );
    assert_eq!(v.declarer, Some(PlayerId(1)));
    assert_eq!(v.contract.unwrap().game, GameType::Grand);
    assert!(!v.contract.unwrap().ouvert);
    assert_eq!(
        v.current_trick
            .iter()
            .map(|p| (p.player.0, p.card.index()))
            .collect::<Vec<_>>(),
        vec![(1, 20), (2, 12)]
    );
    assert_eq!(
        v.legal_actions,
        [8, 10, 11, 13, 14, 17, 18, 25].map(|n| Action::Play(Card::from_index(n).unwrap()))
    );
    click(h, &i.t("guided-start"));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Orient
    );
}

/// Exercise each ordinary #163 stage through widgets; shared by tests/screenshots.
/// 0 orientation, 1 candidate, 2 rule, 3 completed observation, 4 return.
#[allow(dead_code)]
pub fn feed_flow(h: &mut Harness<'_, GrandCoachApp>, language: Language, stage: u8, chosen: u8) {
    use skat_app::*;
    enter_feed(h, language);
    let i = I18n::new(language);
    if stage == 0 {
        return;
    }
    click(h, &seat_name(&i, Seat::Hinterhand));
    click(h, &i.t("guided-none"));
    assert_eq!(
        h.state().session().guided().unwrap().task,
        Some(GuidedBranch::FeedPartner)
    );
    if stage == 1 {
        return;
    }
    let before = h.state().session().view();
    click(
        h,
        &i.tf(
            "guided-consider",
            &[(
                "card",
                card_name(&i, Card::from_index(chosen).unwrap()).into(),
            )],
        ),
    );
    let g = h.state().session().guided().unwrap();
    assert_eq!(g.step, GuidedStep::Act);
    assert_eq!(g.rule.unwrap().card.index(), 11);
    assert_eq!(h.state().session().view().hand, before.hand);
    assert_eq!(
        h.state().session().view().legal_actions,
        before.legal_actions
    );
    assert_eq!(
        h.state().session().view().current_trick,
        before.current_trick
    );
    if stage == 2 {
        return;
    }
    click(h, &card_name(&i, Card::from_index(chosen).unwrap()));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Observe
    );
    click(h, &i.t("guided-void-yes"));
    let g = h.state().session().guided().unwrap();
    assert_eq!(g.step, GuidedStep::Complete);
    assert_eq!(g.observation.unwrap().winner, Some(Seat::Hinterhand));
    assert_eq!(
        h.state().session().view().last_trick.unwrap().winner,
        PlayerId(2)
    );
    if stage == 3 {
        return;
    }
    click(h, &i.t("guided-return"));
    assert!(h.state().session().guided().is_none());
}

/// Frozen QA1458 ordinary #165 seed6 route. Initial MH, CURRENT last in trick2.
/// No injected deal, table view or session. Used by tests and screenshot scenarios.
#[allow(dead_code)]
pub fn enter_null(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    use skat_app::*;
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    let i = I18n::new(language);
    assert_eq!(
        h.state().session().coach_settings(),
        CoachSettings::default()
    );
    for (first, cards) in [
        (
            vec![Action::Hold, Action::Pass],
            vec![2, 24, 19, 21, 3, 8, 5, 27, 6, 29],
        ),
        (
            vec![Action::Pass],
            vec![0, 1, 26, 10, 16, 27, 2, 13, 18, 23],
        ),
    ] {
        for a in first.into_iter().chain(
            cards
                .into_iter()
                .map(|n| Action::Play(Card::from_index(n).unwrap())),
        ) {
            settle(h);
            let v = h.state().session().view();
            assert!(v.legal_actions.contains(&a));
            let label = match a {
                Action::Hold => i.tf("button-hold", &[("value", v.bid.into())]),
                Action::Pass => i.t("button-pass"),
                Action::Play(c) => card_name(&i, c),
                _ => unreachable!(),
            };
            click(h, &label);
        }
        settle(h);
        assert!(h.state().session().view().can_next_game);
        click(h, &i.t("button-next-game"));
    }
    for a in [
        Action::Bid(18),
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Null)),
        Action::Play(Card::from_index(1).unwrap()),
    ] {
        settle(h);
        assert!(h.state().session().view().legal_actions.contains(&a));
        let label = match a {
            Action::Bid(b) => i.tf("button-bid", &[("value", b.into())]),
            Action::Hand => i.t("button-hand"),
            Action::Announce(_) => {
                h.get_by_role_and_label(Role::RadioButton, "Null").click();
                h.run_steps(4);
                i.t("button-announce")
            }
            Action::Play(c) => card_name(&i, c),
            _ => unreachable!(),
        };
        click(h, &label);
    }
    settle(h);
    let v = h.state().session().view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.players[0].seat),
        (3, 1, Seat::Mittelhand)
    );
    assert_eq!(v.declarer, Some(PlayerId::HUMAN));
    assert_eq!(
        v.current_trick
            .iter()
            .map(|p| (p.player.0, p.card.index()))
            .collect::<Vec<_>>(),
        vec![(1, 17), (2, 21)]
    );
    assert_eq!(
        v.legal_actions,
        [16, 18, 20].map(|n| Action::Play(Card::from_index(n).unwrap()))
    );
    assert!(v.own_skat.is_none() && v.open_cards.is_none());
    click(h, &i.t("guided-start"));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Orient
    );
}

/// #165 ordinary views: orientation, wrong/retry, remaining, candidate, attempt,
/// deliberate reveal, memory, actual observation, feedback and immediate return.
#[allow(dead_code)]
pub fn null_flow(h: &mut Harness<'_, GrandCoachApp>, language: Language, stage: u8, chosen: u8) {
    use skat_app::*;
    enter_null(h, language);
    let i = I18n::new(language);
    if stage == 0 {
        return;
    }
    null_click(h, &seat_name(&i, Seat::Mittelhand));
    assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
    if stage == 1 {
        return;
    }
    null_click(h, &i.t("guided-retry"));
    null_click(h, &seat_name(&i, Seat::Vorhand));
    if stage == 2 {
        return;
    }
    null_click(h, &i.t("guided-none"));
    assert_eq!(
        h.state().session().guided().unwrap().task,
        Some(GuidedBranch::Null)
    );
    if stage == 3 {
        return;
    }
    let before = h.state().session().view();
    if stage == 5 {
        null_click(h, &i.t("guided-reveal"));
    } else {
        null_click(
            h,
            &i.tf(
                "guided-consider",
                &[(
                    "card",
                    card_name(&i, Card::from_index(chosen).unwrap()).into(),
                )],
            ),
        );
    }
    assert_eq!(
        h.state()
            .session()
            .guided()
            .unwrap()
            .rule
            .unwrap()
            .card
            .index(),
        20
    );
    assert_eq!(
        h.state().session().view().legal_actions,
        before.legal_actions
    );
    assert_eq!(
        h.state().session().view().current_trick,
        before.current_trick
    );
    if stage == 4 || stage == 5 {
        return;
    }
    if stage == 6 {
        null_click(h, &i.t("guided-memory"));
        h.get_by_label(&i.t("guided-unknown-skat")).scroll_to_me();
        h.run_steps(4);
        return;
    }
    // Hold opponents after completion so the actual trick remains on the table.
    h.state_mut().opponent_delay = 1000.0;
    null_click(h, &card_name(&i, Card::from_index(chosen).unwrap()));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Observe
    );
    assert_eq!(
        h.state()
            .session()
            .view()
            .last_trick
            .unwrap()
            .cards
            .into_iter()
            .map(|p| p.card.index())
            .collect::<Vec<_>>(),
        [17, 21, chosen]
    );
    if stage == 7 {
        return;
    }
    null_click(h, &i.t("guided-void-no"));
    assert_eq!(
        h.state()
            .session()
            .guided()
            .unwrap()
            .observation
            .unwrap()
            .winner,
        Some(Seat::Vorhand)
    );
    if stage == 8 {
        return;
    }
    null_click(h, &i.t("guided-return"));
    assert!(h.state().session().guided().is_none());
}

/// Scroll a new Null-dialogue control into view before a real pointer click.
#[allow(dead_code)]
pub fn null_click(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).scroll_to_me();
    h.run_steps(4);
    click(h, label);
}

/// #169 exact accepted seed1/game2/trick4 route through ordinary widgets.
#[allow(dead_code)]
pub fn enter_null_middle(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    use skat_app::*;
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    for a in [
        Action::Hold,
        Action::Hold,
        Action::Hold,
        Action::Hold,
        Action::Pass,
    ]
    .into_iter()
    .chain(
        [31, 28, 8, 25, 26, 29, 23, 15, 14, 22].map(|n| Action::Play(Card::from_index(n).unwrap())),
    ) {
        settle(h);
        let v = h.state().session().view();
        assert!(v.legal_actions.contains(&a));
        let label = match a {
            Action::Hold => i.tf("button-hold", &[("value", v.bid.into())]),
            Action::Pass => i.t("button-pass"),
            Action::Play(c) => card_name(&i, c),
            _ => unreachable!(),
        };
        click(h, &label);
    }
    settle(h);
    click(h, &i.t("button-next-game"));
    for a in [
        Action::Bid(20),
        Action::Hand,
        Action::Announce(Announcement::plain(GameType::Null)),
    ]
    .into_iter()
    .chain([19, 24, 4].map(|n| Action::Play(Card::from_index(n).unwrap())))
    {
        settle(h);
        assert!(h.state().session().view().legal_actions.contains(&a));
        let label = match a {
            Action::Bid(b) => i.tf("button-bid", &[("value", b.into())]),
            Action::Hand => i.t("button-hand"),
            Action::Announce(_) => {
                h.get_by_role_and_label(Role::RadioButton, "Null").click();
                h.run_steps(4);
                i.t("button-announce")
            }
            Action::Play(c) => card_name(&i, c),
            _ => unreachable!(),
        };
        click(h, &label);
    }
    settle(h);
    let v = h.state().session().view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.players[0].seat),
        (2, 3, Seat::Hinterhand)
    );
    assert_eq!(v.declarer, Some(PlayerId::HUMAN));
    assert_eq!(
        v.current_trick
            .iter()
            .map(|p| p.card.index())
            .collect::<Vec<_>>(),
        [12]
    );
    assert_eq!(
        v.legal_actions,
        [9, 10, 13, 14].map(|n| Action::Play(Card::from_index(n).unwrap()))
    );
    assert!(v.own_skat.is_none() && v.open_cards.is_none());
    click(h, &i.t("guided-start"));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Orient
    );
}

/// All #169 ordinary stages; no session/view injection. `chosen` stays free.
#[allow(dead_code)]
pub fn null_middle_flow(
    h: &mut Harness<'_, GrandCoachApp>,
    language: Language,
    stage: u8,
    chosen: u8,
) {
    use skat_app::*;
    enter_null_middle(h, language);
    let i = I18n::new(language);
    if stage == 0 {
        return;
    }
    null_click(h, &seat_name(&i, Seat::Hinterhand));
    assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
    if stage == 1 {
        return;
    }
    null_click(h, &i.t("guided-retry"));
    null_click(h, &seat_name(&i, Seat::Mittelhand));
    if stage == 2 {
        return;
    }
    null_click(h, &seat_name(&i, Seat::Vorhand));
    if stage == 3 {
        return;
    }
    if stage == 5 {
        null_click(h, &i.t("guided-unknown"));
    } else {
        null_click(
            h,
            &i.tf(
                "guided-consider",
                &[(
                    "card",
                    card_name(&i, Card::from_index(chosen).unwrap()).into(),
                )],
            ),
        );
        if chosen == 13 || chosen == 14 {
            assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
            null_click(h, &i.t("guided-retry"));
            null_click(h, &i.t("guided-reveal"));
        }
    }
    assert_eq!(
        h.state()
            .session()
            .guided()
            .unwrap()
            .rule
            .unwrap()
            .card
            .index(),
        10
    );
    if stage == 4 || stage == 5 {
        return;
    }
    if stage == 10 {
        h.get_by_role_and_label(Role::DisclosureTriangle, &i.t("guided-null-middle-details"))
            .scroll_to_me();
        h.run_steps(4);
        h.get_by_role_and_label(Role::DisclosureTriangle, &i.t("guided-null-middle-details"))
            .click();
        h.run_steps(4);
        return;
    }
    if stage == 6 {
        null_click(h, &i.t("guided-memory"));
        h.get_by_label(&i.t("guided-unknown-skat")).scroll_to_me();
        h.run_steps(4);
        return;
    }
    null_click(h, &card_name(&i, Card::from_index(chosen).unwrap()));
    for _ in 0..5000 {
        h.run_steps(1);
        if h.state().session().guided().unwrap().step == GuidedStep::Observe {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    h.state_mut().opponent_delay = 1000.0;
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Observe
    );
    assert_eq!(
        h.state()
            .session()
            .view()
            .last_trick
            .unwrap()
            .cards
            .into_iter()
            .map(|p| p.card.index())
            .collect::<Vec<_>>(),
        [12, chosen, 15]
    );
    if stage == 7 {
        return;
    }
    null_click(h, &i.t("guided-void-no"));
    assert_eq!(
        h.state()
            .session()
            .guided()
            .unwrap()
            .observation
            .unwrap()
            .winner,
        Some(Seat::Vorhand)
    );
    if stage == 8 {
        return;
    }
    null_click(h, &i.t("guided-return"));
    assert!(h.state().session().guided().is_none());
}

/// #171 accepted seed3, game1/trick1, through real ordinary widgets only.
#[allow(dead_code)]
pub fn lead_master_flow(
    h: &mut Harness<'_, GrandCoachApp>,
    language: Language,
    stage: u8,
    chosen: u8,
) {
    use skat_app::*;
    let i = I18n::new(language);
    h.state_mut().set_language(language);
    h.state_mut().opponent_delay = 0.0;
    h.state_mut().set_coach_budget(Budget {
        samples: 1,
        nodes_per_task: 1,
    });
    settle(h);
    click(h, &i.t("button-pass"));
    settle(h);
    let v = h.state().session().view();
    assert_eq!((v.game_number, v.tricks_played), (1, 0));
    assert!(v.current_trick.is_empty());
    assert_eq!(
        v.legal_actions,
        [5, 6, 8, 13, 14, 22, 23, 27, 28, 29].map(|n| Action::Play(Card::from_index(n).unwrap()))
    );
    click(h, &i.t("guided-start"));
    if stage == 0 {
        return;
    }
    null_click(h, &i.t("guided-lead-declarer"));
    assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
    if stage == 1 {
        return;
    }
    null_click(h, &i.t("guided-retry"));
    null_click(h, &i.t("guided-lead-defender"));
    if stage == 2 {
        return;
    }
    let consider = |n| {
        i.tf(
            "guided-consider",
            &[("card", card_name(&i, Card::from_index(n).unwrap()).into())],
        )
    };
    null_click(h, &consider(5));
    assert_eq!(h.state().session().guided().unwrap().correct, Some(false));
    null_click(h, &i.t("guided-retry"));
    null_click(h, &consider(23));
    let prompt = h.get_by_label(&i.t("guided-lead-attempt")).rect();
    assert!(
        prompt.min.y >= 75.0 && prompt.max.y < 190.0,
        "new lead question must be visible after scrolling to the master answer: {prompt:?}"
    );
    if stage == 3 {
        return;
    }
    if stage == 5 {
        null_click(h, &i.t("guided-unknown"));
    } else {
        null_click(h, &consider(23));
    }
    assert_eq!(
        h.state()
            .session()
            .guided()
            .unwrap()
            .rule
            .unwrap()
            .card
            .index(),
        23
    );
    if stage == 4 || stage == 5 {
        return;
    }
    if stage == 11 {
        h.get_by_role_and_label(Role::DisclosureTriangle, &i.t("guided-lead-details"))
            .scroll_to_me();
        h.run_steps(4);
        h.get_by_role_and_label(Role::DisclosureTriangle, &i.t("guided-lead-details"))
            .click();
        h.run_steps(4);
        return;
    }
    if stage == 6 {
        null_click(h, &i.t("guided-memory"));
        return;
    }
    h.state_mut().opponent_delay = 1000.0;
    null_click(h, &card_name(&i, Card::from_index(chosen).unwrap()));
    assert_eq!(
        h.state().session().guided().unwrap().step,
        GuidedStep::Waiting
    );
    if stage == 7 {
        return;
    }
    h.state_mut().opponent_delay = 0.0;
    for _ in 0..5000 {
        h.run_steps(1);
        if h.state().session().guided().unwrap().step == GuidedStep::Observe {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    h.state_mut().opponent_delay = 1000.0;
    let g = h.state().session().guided().unwrap();
    assert_eq!(g.step, GuidedStep::Observe);
    assert_eq!(g.lead_replies.len(), 2);
    assert!(g.observation.is_none());
    if chosen == 5 {
        assert_eq!(
            h.state()
                .session()
                .view()
                .last_trick
                .unwrap()
                .cards
                .into_iter()
                .map(|p| p.card.index())
                .collect::<Vec<_>>(),
            [5, 0, 3]
        );
    }
    if stage == 8 {
        return;
    }
    null_click(h, &i.t("guided-reveal"));
    if chosen == 5 {
        assert_eq!(
            h.state()
                .session()
                .guided()
                .unwrap()
                .observation
                .unwrap()
                .winner,
            Some(Seat::Hinterhand)
        );
    }
    if stage == 9 {
        return;
    }
    null_click(h, &i.t("guided-return"));
    assert!(h.state().session().guided().is_none());
}
