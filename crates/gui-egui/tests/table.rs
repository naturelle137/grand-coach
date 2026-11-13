// SPDX-License-Identifier: EUPL-1.2

//! The modern table of issue #134 through real widgets: concealed Augen
//! that show only while the button is held with the mouse or the keyboard,
//! the named opponents with their portraits, and the language switch.

use eframe::egui::{Event, Key, Modifiers, PointerButton, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use gui_egui::{GrandCoachApp, Language, i18n::I18n};
use skat_app::{
    Action, Card, FigurePool, Level, Opponents, PlayerId, SeriesCast, SeriesLength, SessionConfig,
    TableView,
};

const REVEAL: &str = "Augen zeigen (gedrückt halten)";

fn harness(seed: u64) -> Harness<'static, GrandCoachApp> {
    let config = SessionConfig {
        seed,
        series: SeriesLength::Fixed(3),
        kontra_re: false,
        opponents: Opponents::Levels([Level::Easy, Level::Medium]),
        ..SessionConfig::default()
    };
    let mut h = Harness::builder()
        .with_size([1600.0, 1000.0])
        .build_eframe(move |cc| GrandCoachApp::new(&cc.egui_ctx, config));
    h.state_mut().opponent_delay = 0.0;
    h.run_steps(2);
    h
}

fn view(h: &Harness<'_, GrandCoachApp>) -> TableView {
    h.state().session().view()
}

fn card_label(card: Card) -> String {
    let suit = ["Kreuz", "Pik", "Herz", "Karo"][usize::from(card.suit().index())];
    let rank = [
        "Sieben", "Acht", "Neun", "Zehn", "Bube", "Dame", "König", "Ass",
    ][usize::from(card.rank().index())];
    format!("{suit}-{rank}")
}

fn settle(h: &mut Harness<'_, GrandCoachApp>) {
    for _ in 0..5000 {
        if !view(h).opponent_to_move {
            h.run_steps(3);
            return;
        }
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("opponents did not finish their moves");
}

/// Plays as a defender by clicks (passes, then the first legal card) until
/// `stop` holds.
fn play_until(h: &mut Harness<'_, GrandCoachApp>, stop: impl Fn(&TableView) -> bool) {
    for _ in 0..400 {
        settle(h);
        let v = view(h);
        if stop(&v) {
            return;
        }
        let label = if v.acting.is_none() {
            "Nächstes Spiel".to_owned()
        } else if v.legal_actions.contains(&Action::Pass) {
            "Passe".to_owned()
        } else if let Some(&Action::Play(card)) = v.legal_actions.first() {
            card_label(card)
        } else {
            panic!("no scripted click for {:?}", v.legal_actions);
        };
        h.get_by_role_and_label(Role::Button, &label).click();
        h.run_steps(4);
    }
    panic!("did not reach the state");
}

fn name(language: Language, view: &TableView, player: PlayerId) -> String {
    let i18n = I18n::new(language);
    if player.is_human() {
        return i18n.t("player-human");
    }
    let pool = i18n.pool();
    let figure = view.cast.figure(pool, player).unwrap();
    i18n.t(&format!("figure-{}", figure.key()))
}

/// The two lines the revealed area shows for `view`.
fn revealed_lines(view: &TableView) -> (String, String) {
    let d = view.declarer.unwrap();
    let a = view.trick_augen;
    let defenders: Vec<_> = PlayerId::ALL.into_iter().filter(|&p| p != d).collect();
    let names: Vec<_> = defenders
        .iter()
        .map(|&p| name(Language::De, view, p))
        .collect();
    let sum: u16 = defenders.iter().map(|p| u16::from(a[p.index()])).sum();
    (
        format!(
            "Alleinspieler {}: {}",
            name(Language::De, view, d),
            a[d.index()]
        ),
        format!("Gegenpartei {}: {sum}", names.join(" + ")),
    )
}

/// `true` if any accessible node's label or value contains `text`.
fn shown(h: &Harness<'_, GrandCoachApp>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
        || h.query_all_by_value(text).next().is_some()
}

/// Asserts that no node shows a revealed Augen line.
fn concealed(h: &Harness<'_, GrandCoachApp>) {
    assert!(!shown(h, "Alleinspieler ") || !shown(h, "Gegenpartei "));
    assert!(!shown(h, "Gegenpartei "), "defence total visible");
    assert!(shown(h, "Verdeckt. Zum Ansehen"));
}

fn pointer(h: &mut Harness<'_, GrandCoachApp>, pressed: bool) {
    let pos = h
        .get_by_role_and_label(Role::Button, REVEAL)
        .rect()
        .center();
    h.input_mut().events.push(Event::PointerMoved(pos));
    h.input_mut().events.push(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, GrandCoachApp>, key: Key, pressed: bool) {
    h.input_mut().events.push(Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

#[test]
fn augen_show_only_while_the_button_is_held_by_mouse_or_keyboard() {
    let mut h = harness(21);
    play_until(&mut h, |v| {
        v.tricks_played >= 3 && v.trick_augen.iter().filter(|&&a| a > 0).count() >= 1
    });
    let v = view(&h);
    let (declarer_line, defence_line) = revealed_lines(&v);
    // The running totals are the completed tricks' Augen; never the Skat.
    let tricks: u8 = v.trick_augen.iter().sum();
    assert!(tricks > 0 && tricks <= 120);

    // Concealed by default, also in the button's own accessible label and
    // on hover: no tooltip carries a number.
    concealed(&h);
    let button = h.get_by_role_and_label(Role::Button, REVEAL);
    button.hover();
    h.run_steps(30);
    concealed(&h);

    // Mouse: shown while pressed, however long, concealed on release.
    pointer(&mut h, true);
    assert!(shown(&h, &declarer_line), "{declarer_line}");
    h.run_steps(40);
    assert!(shown(&h, &declarer_line), "long press");
    assert!(shown(&h, &defence_line), "{defence_line}");
    assert_eq!(
        view(&h).trick_augen,
        v.trick_augen,
        "holding changes no game"
    );
    pointer(&mut h, false);
    concealed(&h);

    // Keyboard: focus the button, hold Space; release conceals.
    h.get_by_role_and_label(Role::Button, REVEAL).focus();
    h.run_steps(2);
    key(&mut h, Key::Space, true);
    assert!(shown(&h, &defence_line));
    h.run_steps(40);
    assert!(shown(&h, &defence_line), "long key press");
    key(&mut h, Key::Space, false);
    concealed(&h);
    // Enter works the same way.
    key(&mut h, Key::Enter, true);
    assert!(shown(&h, &defence_line));
    key(&mut h, Key::Enter, false);
    concealed(&h);

    // Losing the window focus while holding conceals at once.
    key(&mut h, Key::Space, true);
    assert!(shown(&h, &defence_line));
    h.input_mut().events.push(Event::WindowFocused(false));
    h.run_steps(2);
    concealed(&h);
    h.input_mut().events.push(Event::WindowFocused(true));
    key(&mut h, Key::Space, false);
    concealed(&h);

    // Moving the keyboard focus away while holding conceals as well.
    h.get_by_role_and_label(Role::Button, REVEAL).focus();
    h.run_steps(2);
    key(&mut h, Key::Space, true);
    assert!(shown(&h, &defence_line));
    h.get_by_role_and_label(Role::Button, "Spiel").focus();
    h.run_steps(2);
    concealed(&h);
    key(&mut h, Key::Space, false);

    // The single-trick score and the score sheet stay visible throughout.
    assert!(shown(&h, " sticht ("));
    assert!(shown(&h, "Spielliste"));
}

#[test]
fn before_the_first_trick_nothing_is_counted() {
    let mut h = harness(21);
    settle(&mut h);
    assert_eq!(view(&h).tricks_played, 0);
    assert!(shown(&h, "Noch kein Stich abgeschlossen."));
    pointer(&mut h, true);
    assert!(!shown(&h, "Gegenpartei "));
    pointer(&mut h, false);
}

#[test]
fn opponents_keep_their_figures_and_switch_with_the_language() {
    let mut h = harness(1134);
    let cast = SeriesCast::for_seed(1134);
    let check = |h: &Harness<'_, GrandCoachApp>, language: Language| {
        let v = view(h);
        assert_eq!(v.cast, cast);
        for p in [PlayerId(1), PlayerId(2)] {
            let n = name(language, &v, p);
            let caricature = match language {
                Language::De => format!("Karikatur von {n}"),
                Language::En => format!("Caricature of {n}"),
            };
            assert!(shown(h, &caricature), "{caricature}");
        }
        // The human stays "you" without a portrait.
        assert!(!shown(h, "Karikatur von Du") && !shown(h, "Caricature of You"));
    };
    check(&h, Language::De);
    let german = cast.pool(FigurePool::German);
    // Two games later the same figures sit on the same physical seats
    // while the roles have rotated.
    let dealer = view(&h).dealer;
    play_until(&mut h, |v| v.game_number == 2 && !v.opponent_to_move);
    assert_ne!(view(&h).dealer, dealer);
    check(&h, Language::De);
    assert_eq!(view(&h).cast.pool(FigurePool::German), german);

    // English names and portraits replace the German pair; switching back
    // restores the earlier cast, and the game is untouched.
    let before = view(&h);
    h.state_mut().set_language(Language::En);
    h.run_steps(4);
    check(&h, Language::En);
    h.state_mut().set_language(Language::De);
    h.run_steps(4);
    check(&h, Language::De);
    let after = view(&h);
    assert_eq!(after.hand, before.hand);
    assert_eq!(after.legal_actions, before.legal_actions);
    assert_eq!(after.cast, before.cast);
}
