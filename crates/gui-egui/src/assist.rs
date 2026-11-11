// SPDX-License-Identifier: EUPL-1.2

//! Texts of the counting aid and the counting quiz (D-64): structured
//! facts from `skat-app` in the player's language. No facts are computed
//! here.

use skat_app::{
    CountingView, Derivation, Follow, PlayerId, QuestionKind, QuizFeedback, QuizOutcome, Suit,
    Voids,
};

use crate::i18n::I18n;
use crate::text;

/// Name of a follow class: "Trumpf" or the suit on the shown faces.
fn class(i18n: &I18n, class: Follow) -> String {
    match class {
        Follow::Trumpf => i18n.t("quiz-class-trumpf"),
        Follow::Farbe(suit) => text::suit(i18n, suit),
    }
}

/// The classes `voids` names, e.g. "Trumpf, Hearts".
fn void_classes(i18n: &I18n, voids: &Voids) -> String {
    let mut names = Vec::new();
    if voids.trumpf {
        names.push(i18n.t("aid-void-trumpf"));
    }
    for suit in Suit::ALL {
        if voids.suits[usize::from(suit.index())] {
            names.push(text::suit(i18n, suit));
        }
    }
    names.join(", ")
}

/// The lines of the counting aid.
pub fn aid_lines(i18n: &I18n, aid: &CountingView) -> Vec<String> {
    let mut lines = vec![i18n.tf("aid-played", &[("count", aid.played.len().into())])];
    match (aid.trumpf, aid.game) {
        (Some(t), _) => {
            lines.push(i18n.tf(
                "aid-trumpf",
                &[
                    ("outside", t.outside.into()),
                    ("total", t.total.into()),
                    ("own", t.own.into()),
                    ("played", t.played.into()),
                ],
            ));
            if t.discarded > 0 {
                lines.push(i18n.tf("aid-trumpf-discard", &[("discarded", t.discarded.into())]));
            }
            if t.skat_unknown {
                lines.push(i18n.t("aid-trumpf-skat"));
            }
        }
        (None, Some(_)) => lines.push(i18n.t("aid-trumpf-null")),
        (None, None) => lines.push(i18n.t("aid-trumpf-later")),
    }
    match (aid.augen.declarer, aid.augen.defenders) {
        (Some(declarer), Some(defenders)) => lines.push(i18n.tf(
            "aid-augen",
            &[
                ("declarer", declarer.into()),
                ("defenders", defenders.into()),
            ],
        )),
        _ => lines.push(i18n.t("aid-augen-later")),
    }
    lines.push(i18n.tf(
        "aid-augen-outstanding",
        &[("count", aid.augen.outstanding.into())],
    ));
    if let Some(skat) = aid.augen.skat {
        lines.push(i18n.tf("aid-augen-skat", &[("count", skat.into())]));
    }
    lines
}

/// The proven voids of the opponents, one line per opponent with a void.
pub fn void_lines(i18n: &I18n, aid: &CountingView) -> Vec<String> {
    PlayerId::ALL
        .into_iter()
        .filter(|p| !p.is_human() && !aid.voids[p.index()].is_empty())
        .map(|p| {
            i18n.tf(
                "aid-void-line",
                &[
                    ("player", text::player(i18n, p).into()),
                    ("classes", void_classes(i18n, &aid.voids[p.index()]).into()),
                ],
            )
        })
        .collect()
}

/// The wording of a question.
pub fn question(i18n: &I18n, kind: QuestionKind) -> String {
    match kind {
        QuestionKind::TrumpfOutside => i18n.t("quiz-q-trumpf"),
        QuestionKind::AugenWon { declarer: true } => i18n.t("quiz-q-augen-declarer"),
        QuestionKind::AugenWon { declarer: false } => i18n.t("quiz-q-augen-defenders"),
        QuestionKind::ClassPlayed { class: c } => {
            i18n.tf("quiz-q-played", &[("class", class(i18n, c).into())])
        }
    }
}

/// The result of a question with the derivation of the exact answer.
pub fn feedback_lines(i18n: &I18n, f: &QuizFeedback) -> Vec<String> {
    let expected = f.derivation.expected();
    let mut lines = vec![match f.outcome {
        QuizOutcome::Correct => i18n.tf("quiz-correct", &[("expected", expected.into())]),
        QuizOutcome::Incorrect { given } => i18n.tf(
            "quiz-incorrect",
            &[("given", given.into()), ("expected", expected.into())],
        ),
        QuizOutcome::Skipped => i18n.tf("quiz-skipped", &[("expected", expected.into())]),
    }];
    match f.derivation {
        Derivation::TrumpfOutside(t) => {
            lines.push(i18n.tf(
                "quiz-why-trumpf",
                &[
                    ("total", t.total.into()),
                    ("own", t.own.into()),
                    ("played", t.played.into()),
                    ("discarded", t.discarded.into()),
                    ("outside", t.outside.into()),
                ],
            ));
            if t.skat_unknown {
                lines.push(i18n.t("quiz-why-trumpf-skat"));
            }
        }
        Derivation::AugenWon { augen, other, .. } => lines.push(i18n.tf(
            "quiz-why-augen",
            &[("augen", augen.into()), ("other", other.into())],
        )),
        Derivation::ClassPlayed {
            class: c,
            size,
            played,
            unplayed,
        } => lines.push(i18n.tf(
            "quiz-why-played",
            &[
                ("class", class(i18n, c).into()),
                ("size", size.into()),
                ("played", played.into()),
                ("unplayed", unplayed.into()),
            ],
        )),
    }
    lines
}
