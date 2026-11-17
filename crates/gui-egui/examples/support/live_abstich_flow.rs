// SPDX-License-Identifier: EUPL-1.2

//! Real app/widget flow: no injected deals, recordings or lesson previews.
#![allow(dead_code)]
use eframe::egui::Role;
use egui_kittest::{Harness, kittest::Queryable};
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::{Action, Card, Command, LiveAbstichView};

pub fn click(h: &mut Harness<'_, GrandCoachApp>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(5);
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
pub fn card_label(card: Card, language: Language) -> String {
    let i = I18n::new(language);
    let suit =
        i.t(["suit-kreuz", "suit-pik", "suit-herz", "suit-karo"][card.suit().index() as usize]);
    let rank = i.t([
        "rank-sieben",
        "rank-acht",
        "rank-neun",
        "rank-zehn",
        "rank-bube",
        "rank-dame",
        "rank-koenig",
        "rank-ass",
    ][card.rank().index() as usize]);
    i.tf("card-name", &[("suit", suit.into()), ("rank", rank.into())])
}

/// Probe only a clone with the public app API, then deliberately select the
/// same legal card through the actual widgets. No hidden view is accessed.
pub fn reach(h: &mut Harness<'_, GrandCoachApp>, language: Language) -> Card {
    h.state_mut().set_language(Language::De);
    for _ in 0..150 {
        settle(h);
        let v = h.state().session().view();
        let mut probe = h.state().session().clone();
        if probe.execute(Command::StartLiveAbstich).is_ok() {
            let live = probe.live_abstich().unwrap();
            if !live.unsupported {
                for card in &live.candidates {
                    probe
                        .execute(Command::SelectLiveAbstich(live.boundary, *card))
                        .unwrap();
                    if !probe.live_abstich().unwrap().unsupported {
                        h.state_mut().set_language(language);
                        h.run_steps(4);
                        let i = I18n::new(language);
                        click(h, &i.t("live-abstich-start"));
                        return *card;
                    }
                }
            }
        }
        let label = if v.acting.is_none() {
            "Nächstes Spiel".to_owned()
        } else if v.legal_actions.contains(&Action::Pass) {
            "Passe".to_owned()
        } else if v.legal_actions.contains(&Action::KeinKontra) {
            "Kein Kontra".to_owned()
        } else if let Some(Action::Play(c)) = v.legal_actions.first() {
            card_label(*c, Language::De)
        } else {
            panic!("unexpected scenario actions {:?}", v.legal_actions)
        };
        click(h, &label);
    }
    panic!("no supported live candidate reached");
}
pub fn select(
    h: &mut Harness<'_, GrandCoachApp>,
    card: Card,
    language: Language,
) -> LiveAbstichView {
    // The selected candidate is a selectable Button as well as a card in the
    // underlying hand. The window's button is the last matching widget.
    let label = card_label(card, language);
    h.get_all_by_role_and_label(Role::Button, &label)
        .last()
        .unwrap()
        .click();
    h.run_steps(5);
    h.state().session().live_abstich().unwrap()
}
pub fn close(h: &mut Harness<'_, GrandCoachApp>, language: Language) {
    click(h, &I18n::new(language).t("live-abstich-return"));
}

pub fn stage(h: &mut Harness<'_, GrandCoachApp>, language: Language, stage: &str) {
    let card = reach(h, language);
    if stage == "entry" {
        return;
    }
    let before = h
        .state()
        .session()
        .export_current_recording_privileged()
        .unwrap();
    let v = select(h, card, language);
    assert!(v.evidence.is_none() && v.response.is_none() && !v.unsupported);
    let i = I18n::new(language);
    match stage {
        "answer" | "overview" | "return" => {
            click(h, &i.t("focus-certain"));
        }
        "help" => {
            click(h, &i.t("live-abstich-help"));
        }
        "skip" => {
            click(h, &i.t("quiz-skip"));
        }
        "unsupported" => {
            let candidates = v.candidates;
            let chosen = candidates
                .into_iter()
                .find(|c| {
                    let mut probe = h.state().session().clone();
                    probe
                        .execute(Command::SelectLiveAbstich(v.boundary, *c))
                        .unwrap();
                    probe.live_abstich().unwrap().unsupported
                })
                .expect("at least one unsupported card");
            assert!(select(h, chosen, language).unsupported);
        }
        "candidate" => {}
        other => panic!("unknown live stage {other}"),
    }
    assert_eq!(
        h.state()
            .session()
            .export_current_recording_privileged()
            .unwrap(),
        before
    );
    if matches!(stage, "overview" | "return") {
        close(h, language);
    }
    if stage == "overview" {
        h.state_mut().show_learning();
        h.run_steps(5);
    }
}
