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
    enter_kontra_in(h, language, re, true);
}

/// As [`enter_kontra`]; `supported`: false stops at the first Kontra/Re
/// offer in Grand, Null or Ouvert instead, where the dialogue falls back.
#[allow(dead_code)]
pub fn enter_kontra_in(
    h: &mut Harness<'_, GrandCoachApp>,
    language: Language,
    re: bool,
    supported: bool,
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
        let farbe = v
            .contract
            .is_some_and(|c| matches!(c.game, GameType::Farbe(_)) && !c.ouvert);
        if v.contract.is_some() && farbe == supported && v.legal_actions.contains(&wanted) {
            click(h, &i.t("guided-start"));
            let g = h.state().session().guided().expect("explicit dialogue");
            if supported {
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
