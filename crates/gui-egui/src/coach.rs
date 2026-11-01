// SPDX-License-Identifier: EUPL-1.2

//! Translated texts of the live coach (M5, D-49): turns the staged hints,
//! the note on the last move, the debriefing and the warning of
//! `skat-coach` (via `skat-app`) into lines. Before the move nothing ranks
//! or labels an option. Estimated figures (chances, expected points,
//! uncertainty, expected card counts) appear only with "show figures";
//! exact facts such as counted cards, Augen and bid values always do. A
//! debriefing moment shows a short summary without any estimate; the
//! comparison, figures, facts and assumptions are its details (D-52).
//! Each moment shows the table as the player saw it at that decision, from
//! its snapshot only (issue #47).

use skat_app::coach_model::{
    Abstich, Amount, Assumption, Basis, CardClass, Certainty, ComparedOption, Concept,
    Consideration, ContractValue, Decision, Estimate, Follow, Hints, Judgement, LessonContent,
    MomentKind, MoveReview, Note, Objective, Observation, Overtake, Place, Range, Reach, Reason,
    Role, Situation, SkatState, Spitzen, Stage, Structure, TrumpfSets, Warning,
};
use skat_app::{
    Action, Card, CardSet, Contract, Debriefing, DebriefingMoment, DecisionContext, GameType,
    LastMove, Phase, PlayerId, Seat, TableView, Unavailable,
};

use crate::i18n::I18n;
use crate::text;

/// How a line is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Section heading.
    Heading,
    /// Emphasised line.
    Strong,
    /// Ordinary line.
    Normal,
    /// Secondary line (uncertainty, assumptions).
    Weak,
}

/// One line of coach text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The text.
    pub text: String,
    /// How to show it.
    pub style: Style,
}

fn line(text: String, style: Style) -> Line {
    Line { text, style }
}

/// Options listed in full; the rest only count.
const SHOWN_OPTIONS: usize = 12;

/// Player sitting on `seat` in the current deal.
fn seat_player(view: &TableView, seat: Seat) -> PlayerId {
    view.players
        .iter()
        .find(|p| p.seat == seat)
        .map_or(PlayerId::HUMAN, |p| p.player)
}

fn percent(x: f64) -> String {
    format!("{:.0} %", x * 100.0)
}

fn points(x: f64) -> String {
    format!("{x:+.1}")
}

/// A legal action of the current decision in words.
pub fn action(i18n: &I18n, view: &TableView, action: Action) -> String {
    action_at(i18n, view.bid, view.hand_game, action)
}

/// An action in words, at bid value `bid` and with `hand_game` as when it
/// was decided.
fn action_at(i18n: &I18n, bid: u16, hand_game: Option<bool>, action: Action) -> String {
    match action {
        Action::Bid(value) => i18n.tf("coach-action-bid", &[("value", value.into())]),
        Action::Hold => i18n.tf("coach-action-hold", &[("value", bid.into())]),
        Action::Pass => i18n.t("button-pass"),
        Action::Hand => i18n.t("button-hand"),
        Action::TakeSkat => i18n.t("button-take-skat"),
        Action::Discard(cards) => {
            let names: Vec<String> = cards.iter().map(|c| text::card(i18n, c)).collect();
            i18n.tf(
                "coach-action-discard",
                &[("cards", names.join(", ").into())],
            )
        }
        Action::Announce(a) => {
            let contract = Contract::new(a, hand_game == Some(true));
            i18n.tf(
                "coach-action-announce",
                &[("game", text::contract(i18n, contract).into())],
            )
        }
        Action::Play(card) => text::card(i18n, card),
        Action::Kontra => i18n.t("button-kontra"),
        Action::KeinKontra => i18n.t("button-kein-kontra"),
        Action::Re => i18n.t("button-re"),
        Action::KeinRe => i18n.t("button-kein-re"),
    }
}

/// The name of a teaching concept (the learning focus).
pub fn concept(i18n: &I18n, concept: Concept) -> String {
    i18n.t(match concept {
        Concept::HandStructure => "coach-concept-hand-structure",
        Concept::GameValue => "coach-concept-game-value",
        Concept::ProvenVoid => "coach-concept-proven-void",
        Concept::RemainingTrumpf => "coach-concept-remaining-trumpf",
        Concept::TrumpfVulnerability => "coach-concept-trumpf-vulnerability",
        Concept::PartnerOvertake => "coach-concept-partner-overtake",
    })
}

/// The learning focus, or that none is chosen.
pub fn focus(i18n: &I18n, focus: Option<Concept>) -> String {
    focus.map_or_else(|| i18n.t("coach-focus-none"), |c| concept(i18n, c))
}

fn decision(i18n: &I18n, decision: Decision) -> String {
    i18n.t(match decision {
        Decision::Bid => "coach-decision-bid",
        Decision::Answer => "coach-decision-answer",
        Decision::Pickup => "coach-decision-pickup",
        Decision::Discard => "coach-decision-discard",
        Decision::Announce => "coach-decision-announce",
        Decision::Card => "coach-decision-card",
        Decision::Kontra => "coach-decision-kontra",
        Decision::Re => "coach-decision-re",
    })
}

fn role(i18n: &I18n, role: Role) -> String {
    i18n.t(match role {
        Role::Bidder => "coach-role-bidder",
        Role::Declarer => "coach-role-declarer",
        Role::Defender => "coach-role-defender",
    })
}

fn objective(i18n: &I18n, role: Role, objective: Objective) -> String {
    let defender = role == Role::Defender;
    match objective {
        Objective::Undecided => i18n.t("coach-objective-undecided"),
        Objective::Augen { target } => i18n.tf(
            if defender {
                "coach-objective-augen-defender"
            } else {
                "coach-objective-augen"
            },
            &[("target", target.into())],
        ),
        Objective::AllTricks => i18n.t(if defender {
            "coach-objective-all-tricks-defender"
        } else {
            "coach-objective-all-tricks"
        }),
        Objective::NoTrick => i18n.t(if defender {
            "coach-objective-no-trick-defender"
        } else {
            "coach-objective-no-trick"
        }),
    }
}

fn holder(i18n: &I18n, view: &TableView, place: Place) -> String {
    match place {
        Place::Seat(seat) => text::player(i18n, seat_player(view, seat)),
        Place::Skat => i18n.t("coach-holder-skat"),
    }
}

fn class(i18n: &I18n, class: CardClass) -> String {
    i18n.t(match class {
        CardClass::Trumps => "coach-class-trumps",
        CardClass::Jacks => "coach-class-jacks",
    })
}

fn fact(i18n: &I18n, view: &TableView, o: Observation) -> String {
    let name = |seat: Seat| text::player(i18n, seat_player(view, seat));
    match o {
        Observation::Void { seat, follow } => {
            let what = match follow {
                Follow::Trumpf => i18n.t("coach-class-trumps"),
                Follow::Farbe(suit) => text::suit(i18n, suit),
            };
            i18n.tf(
                "coach-fact-void",
                &[("player", name(seat).into()), ("class", what.into())],
            )
        }
        Observation::TrumpsOut { count } => {
            i18n.tf("coach-fact-trumps-out", &[("count", count.into())])
        }
        Observation::AugenWon {
            declarer,
            defenders,
        } => i18n.tf(
            "coach-fact-augen-won",
            &[
                ("declarer", declarer.into()),
                ("defenders", defenders.into()),
            ],
        ),
        Observation::AugenOut { augen } => {
            i18n.tf("coach-fact-augen-out", &[("augen", augen.into())])
        }
        Observation::SkatKnown { cards } => {
            let names: Vec<String> = cards.iter().map(|c| text::card(i18n, c)).collect();
            i18n.tf(
                "coach-fact-skat-known",
                &[("cards", names.join(", ").into())],
            )
        }
        Observation::BidShown { seat, value } => i18n.tf(
            "coach-fact-bid-shown",
            &[("player", name(seat).into()), ("value", value.into())],
        ),
        Observation::KontraEligible { seat } => i18n.tf(
            "coach-fact-kontra-eligible",
            &[("player", name(seat).into())],
        ),
        Observation::TrumpfSets(sets) => i18n.tf(
            "coach-fact-trumpf-unseen",
            &[("count", sets.unseen.len().into())],
        ),
        Observation::Structure(s) => i18n.tf(
            "coach-fact-structure",
            &[
                ("game", text::game_type(i18n, s.game).into()),
                ("count", s.trumpf.len().into()),
            ],
        ),
        Observation::GameValues(v) => {
            let ranges: Vec<String> = v.contracts().iter().map(|&(_, r)| range(r)).collect();
            i18n.tf(
                "coach-fact-game-values",
                &[
                    ("game", text::game_type(i18n, v.game).into()),
                    ("value", ranges.join(", ").into()),
                ],
            )
        }
        Observation::Commitment { value, .. } => {
            i18n.tf("coach-fact-commitment", &[("value", value.into())])
        }
    }
}

/// A value range: one number if fixed, otherwise lowest to highest.
fn range(r: Range) -> String {
    if r.fixed() {
        r.low.to_string()
    } else {
        format!("{}–{}", r.low, r.high)
    }
}

/// Cards in display order of `game`, highest first.
fn card_list(i18n: &I18n, cards: CardSet, game: GameType) -> String {
    let names: Vec<String> = skat_app::sort_hand(cards, Some(game))
        .into_iter()
        .map(|c| text::card(i18n, c))
        .collect();
    names.join(", ")
}

/// The attention question of a lesson, before the move (D-49: it names no
/// option and gives no answer). Mechanisms need the move and have none.
fn lesson_question(i18n: &I18n, lesson: &LessonContent) -> Vec<Line> {
    let id = match lesson {
        LessonContent::RemainingTrumpf { .. } => "coach-lesson-remaining-trumpf-question",
        LessonContent::ProvenVoid { .. } => "coach-lesson-proven-void-question",
        LessonContent::HandStructure { .. } => "coach-lesson-hand-structure-question",
        LessonContent::GameValue { .. } => "coach-lesson-game-value-question",
        LessonContent::TrumpfVulnerability { .. } | LessonContent::PartnerOvertake { .. } => {
            return Vec::new();
        }
    };
    vec![
        line(i18n.t("coach-section-lesson"), Style::Heading),
        line(i18n.t(id), Style::Strong),
    ]
}

/// A lesson after the move: the evidence with the cards it rests on, the
/// exception and the transfer question.
pub fn lesson(i18n: &I18n, view: &TableView, lesson: &LessonContent) -> Vec<Line> {
    let heading = match lesson {
        LessonContent::RemainingTrumpf { .. }
        | LessonContent::ProvenVoid { .. }
        | LessonContent::HandStructure { .. }
        | LessonContent::GameValue { .. } => "coach-section-lesson-review",
        LessonContent::TrumpfVulnerability { .. } | LessonContent::PartnerOvertake { .. } => {
            "coach-section-lesson-mechanism"
        }
    };
    let mut out = vec![line(i18n.t(heading), Style::Heading)];
    match lesson {
        LessonContent::TrumpfVulnerability { threats, .. } => {
            out.extend(trumpf_vulnerability(i18n, view, threats));
        }
        LessonContent::PartnerOvertake { game, overtake } => {
            out.extend(partner_overtake(i18n, view, *game, overtake));
        }
        LessonContent::RemainingTrumpf { game, sets } => {
            out.extend(remaining_trumpf(i18n, *game, sets));
        }
        LessonContent::HandStructure { games } => out.extend(hand_structure(i18n, games)),
        LessonContent::GameValue {
            value,
            bidding,
            kontra_re,
            skat,
            games,
        } => out.extend(game_value(i18n, *value, *bidding, *kontra_re, *skat, games)),
        LessonContent::ProvenVoid { game, voids } => {
            let name = |seat: Seat| text::player(i18n, seat_player(view, seat));
            let mut tail: Vec<Line> = Vec::new();
            for v in voids {
                let (what, follow) = match v.follow {
                    Follow::Trumpf => (i18n.t("coach-lesson-class-trumpf"), "trumpf"),
                    Follow::Farbe(suit) => (text::suit(i18n, suit), "suit"),
                };
                out.push(line(
                    i18n.tf(
                        "coach-lesson-proven-void-evidence",
                        &[
                            ("trick", v.trick.into()),
                            ("player", name(v.seat).into()),
                            ("card", text::card(i18n, v.card).into()),
                            ("led", text::card(i18n, v.led).into()),
                            ("class", what.clone().into()),
                            ("follow", follow.into()),
                        ],
                    ),
                    Style::Normal,
                ));
                let case = match (game, v.follow) {
                    (GameType::Null, _) => "null",
                    (_, Follow::Trumpf) => "trumpf",
                    _ => "suit",
                };
                for l in [
                    line(
                        i18n.tf(
                            "coach-lesson-proven-void-exception",
                            &[("case", case.into()), ("class", what.clone().into())],
                        ),
                        Style::Normal,
                    ),
                    line(
                        i18n.tf(
                            "coach-lesson-proven-void-transfer",
                            &[("class", what.into())],
                        ),
                        Style::Strong,
                    ),
                ] {
                    if !tail.contains(&l) {
                        tail.push(l);
                    }
                }
            }
            // Exceptions first, then the transfer questions.
            tail.sort_by_key(|l| l.style == Style::Strong);
            out.extend(tail);
        }
    }
    out
}

fn skat_state(skat: SkatState) -> &'static str {
    match skat {
        SkatState::Unknown => "unknown",
        SkatState::Held => "held",
        SkatState::Discarded => "discarded",
    }
}

fn spitzen(i18n: &I18n, s: Spitzen) -> String {
    i18n.tf(
        "coach-lesson-spitzen",
        &[
            ("case", if s.mit { "mit" } else { "ohne" }.into()),
            ("count", s.count.into()),
        ],
    )
}

/// Cards in display order of `game`, or a dash for none.
fn cards_or_dash(i18n: &I18n, cards: CardSet, game: GameType) -> String {
    if cards.is_empty() {
        "–".to_owned()
    } else {
        card_list(i18n, cards, game)
    }
}

/// The hand-structure lesson: per candidate game the own Buben or Trumpf
/// and Spitzen, what an unknown Skat can change, the exception, the model
/// limitation of the note and the transfer question.
fn hand_structure(i18n: &I18n, games: &[Structure]) -> Vec<Line> {
    let mut out = Vec::new();
    for s in games {
        let kind = if s.game == GameType::Grand {
            "grand"
        } else {
            "suit"
        };
        out.push(line(
            i18n.tf(
                "coach-lesson-hand-structure-game",
                &[
                    ("game", text::game_type(i18n, s.game).into()),
                    ("kind", kind.into()),
                    ("count", s.trumpf.len().into()),
                    ("cards", cards_or_dash(i18n, s.trumpf, s.game).into()),
                    ("value", spitzen(i18n, s.spitzen).into()),
                ],
            ),
            Style::Normal,
        ));
        if !s.skat_trumpf.is_empty() {
            out.push(line(
                i18n.tf(
                    "coach-lesson-hand-structure-discarded",
                    &[
                        ("count", s.skat_trumpf.len().into()),
                        ("cards", card_list(i18n, s.skat_trumpf, s.game).into()),
                    ],
                ),
                Style::Normal,
            ));
        }
        if s.skat == SkatState::Unknown {
            out.push(line(
                i18n.tf(
                    "coach-lesson-hand-structure-range",
                    &[
                        ("kind", kind.into()),
                        (
                            "case",
                            if s.counts.fixed() { "fixed" } else { "range" }.into(),
                        ),
                        ("min", s.counts.low.into()),
                        ("max", s.counts.high.into()),
                        ("count", s.trumpf_most.into()),
                    ],
                ),
                Style::Normal,
            ));
        }
    }
    if let Some(first) = games.first() {
        out.push(line(
            i18n.tf(
                "coach-lesson-hand-structure-skat",
                &[("skat", skat_state(first.skat).into())],
            ),
            Style::Normal,
        ));
    }
    out.push(line(
        i18n.t("coach-lesson-hand-structure-exception"),
        Style::Normal,
    ));
    out.push(line(i18n.t("coach-lesson-preplay-model"), Style::Normal));
    out.push(line(
        i18n.t("coach-lesson-hand-structure-transfer"),
        Style::Strong,
    ));
    out
}

fn reach(r: Reach) -> &'static str {
    match r {
        Reach::Reached => "reached",
        Reach::Skat => "skat",
        Reach::Play => "play",
        Reach::Never => "never",
    }
}

/// The game-value lesson: the value to reach, per game type each contract
/// with its value range and how it relates, the levels and Kontra/Re, the
/// exception, the model limitation of the note and the transfer question.
fn game_value(
    i18n: &I18n,
    value: u16,
    bidding: bool,
    kontra_re: bool,
    skat: SkatState,
    games: &[(GameType, Vec<ContractValue>)],
) -> Vec<Line> {
    let mut out = vec![line(
        i18n.tf(
            "coach-lesson-game-value-bid",
            &[
                ("case", if bidding { "call" } else { "declarer" }.into()),
                ("value", value.into()),
            ],
        ),
        Style::Strong,
    )];
    for (game, contracts) in games {
        let entries: Vec<String> = contracts
            .iter()
            .map(|c| {
                let kind = match (c.contract.hand, c.contract.ouvert) {
                    (false, false) => "plain",
                    (true, false) => "hand",
                    (false, true) => "ouvert",
                    (true, true) => "handouvert",
                };
                i18n.tf(
                    "coach-lesson-game-value-entry",
                    &[
                        ("kind", kind.into()),
                        ("value", range(c.range).into()),
                        ("case", reach(c.reach).into()),
                    ],
                )
            })
            .collect();
        out.push(line(
            i18n.tf(
                "coach-lesson-game-value-game",
                &[
                    ("game", text::game_type(i18n, *game).into()),
                    ("value", entries.join(" · ").into()),
                ],
            ),
            Style::Normal,
        ));
    }
    let skat = match skat {
        SkatState::Unknown => "unknown",
        SkatState::Held | SkatState::Discarded => "known",
    };
    out.push(line(
        i18n.tf("coach-lesson-game-value-skat", &[("skat", skat.into())]),
        Style::Normal,
    ));
    out.push(line(
        i18n.t("coach-lesson-game-value-levels"),
        Style::Normal,
    ));
    if kontra_re {
        out.push(line(
            i18n.t("coach-lesson-game-value-kontra"),
            Style::Normal,
        ));
    }
    out.push(line(
        i18n.t("coach-lesson-game-value-exception"),
        Style::Normal,
    ));
    out.push(line(i18n.t("coach-lesson-preplay-model"), Style::Normal));
    out.push(line(
        i18n.t("coach-lesson-game-value-transfer"),
        Style::Strong,
    ));
    out
}

fn certainty(c: Certainty) -> &'static str {
    match c {
        Certainty::Possible => "possible",
        Certainty::Certain => "certain",
    }
}

/// The Abstich mechanism: per opponent the condition and whether it is
/// possible or certain; the rule, the exception and the transfer question.
fn trumpf_vulnerability(i18n: &I18n, view: &TableView, threats: &[Abstich]) -> Vec<Line> {
    let Some(first) = threats.first() else {
        return Vec::new();
    };
    let suit = text::suit(i18n, first.card.suit());
    let mut out: Vec<Line> = threats
        .iter()
        .map(|t| {
            line(
                i18n.tf(
                    "coach-lesson-trumpf-vulnerability-evidence",
                    &[
                        (
                            "player",
                            text::player(i18n, seat_player(view, t.seat)).into(),
                        ),
                        ("card", text::card(i18n, t.card).into()),
                        ("suit", suit.clone().into()),
                        ("certainty", certainty(t.certainty).into()),
                        ("count", t.suit_unseen.len().into()),
                        ("trumps", t.trumpf_unseen.len().into()),
                    ],
                ),
                Style::Normal,
            )
        })
        .collect();
    out.push(line(
        i18n.tf(
            "coach-lesson-trumpf-vulnerability-rule",
            &[("suit", suit.into())],
        ),
        Style::Normal,
    ));
    out.push(line(
        i18n.t("coach-lesson-trumpf-vulnerability-exception"),
        Style::Normal,
    ));
    out.push(line(
        i18n.t("coach-lesson-trumpf-vulnerability-transfer"),
        Style::Strong,
    ));
    out
}

/// The partner-overtake mechanism: the trick, the ways the partner can beat
/// the declarer's card, possible or certain; the rule, the exception and
/// the transfer question.
fn partner_overtake(i18n: &I18n, view: &TableView, game: GameType, o: &Overtake) -> Vec<Line> {
    let partner = text::player(i18n, seat_player(view, o.partner));
    let (class, case) = match o.led {
        Follow::Trumpf => (i18n.t("coach-lesson-class-trumpf"), "trumpf"),
        Follow::Farbe(suit) => (text::suit(i18n, suit), "suit"),
    };
    let way = match (o.higher.is_empty(), o.trumping) {
        (false, true) => "both",
        (true, true) => "trump",
        _ => "follow",
    };
    vec![
        line(
            i18n.tf(
                "coach-lesson-partner-overtake-evidence",
                &[
                    (
                        "declarer",
                        text::player(i18n, seat_player(view, o.declarer)).into(),
                    ),
                    ("best", text::card(i18n, o.best).into()),
                    ("card", text::card(i18n, o.card).into()),
                    ("player", partner.clone().into()),
                ],
            ),
            Style::Normal,
        ),
        line(
            i18n.tf(
                "coach-lesson-partner-overtake-ways",
                &[
                    ("player", partner.clone().into()),
                    ("way", way.into()),
                    ("cards", card_list(i18n, o.higher, game).into()),
                    ("class", class.clone().into()),
                    ("certainty", certainty(o.certainty).into()),
                ],
            ),
            Style::Normal,
        ),
        line(
            i18n.tf(
                "coach-lesson-partner-overtake-rule",
                &[
                    ("player", partner.into()),
                    ("class", class.into()),
                    ("case", case.into()),
                ],
            ),
            Style::Normal,
        ),
        line(
            i18n.t("coach-lesson-partner-overtake-exception"),
            Style::Normal,
        ),
        line(
            i18n.t("coach-lesson-partner-overtake-transfer"),
            Style::Strong,
        ),
    ]
}

fn remaining_trumpf(i18n: &I18n, game: GameType, sets: &TrumpfSets) -> Vec<Line> {
    let kind = if game == GameType::Grand {
        "grand"
    } else {
        "suit"
    };
    let skat = if sets.skat_unknown {
        "unknown"
    } else {
        "known"
    };
    let count = |id: &str, cards: CardSet| {
        line(
            i18n.tf(
                id,
                &[
                    ("count", cards.len().into()),
                    ("cards", card_list(i18n, cards, game).into()),
                    ("skat", skat.into()),
                ],
            ),
            Style::Normal,
        )
    };
    let mut out = vec![line(
        i18n.tf(
            "coach-lesson-remaining-trumpf-total",
            &[
                ("kind", kind.into()),
                ("game", text::game_type(i18n, game).into()),
                ("count", sets.total.len().into()),
            ],
        ),
        Style::Normal,
    )];
    out.push(count("coach-lesson-remaining-trumpf-own", sets.own));
    out.push(count("coach-lesson-remaining-trumpf-played", sets.played));
    if !sets.own_skat.is_empty() {
        out.push(count("coach-lesson-remaining-trumpf-skat", sets.own_skat));
    }
    if !sets.revealed.is_empty() {
        out.push(count(
            "coach-lesson-remaining-trumpf-revealed",
            sets.revealed,
        ));
    }
    let mut unseen = count("coach-lesson-remaining-trumpf-unseen", sets.unseen);
    unseen.style = Style::Strong;
    out.push(unseen);
    out.push(line(
        if sets.unseen.is_empty() {
            i18n.t("coach-lesson-remaining-trumpf-exception-none")
        } else {
            i18n.tf(
                "coach-lesson-remaining-trumpf-exception",
                &[("skat", skat.into())],
            )
        },
        Style::Normal,
    ));
    out.push(line(
        i18n.t("coach-lesson-remaining-trumpf-transfer"),
        Style::Strong,
    ));
    out
}

fn inferred(
    i18n: &I18n,
    view: &TableView,
    i: &skat_app::coach_model::Inferred,
    figures: bool,
) -> String {
    let who = holder(i18n, view, i.inference.holder);
    let what = class(i18n, i.inference.class);
    if i.inference.certain {
        return i18n.tf(
            "coach-inferred-certain",
            &[
                ("holder", who.into()),
                ("count", i.inference.max.into()),
                ("class", what.into()),
            ],
        );
    }
    let id = match i.amount {
        Amount::Hardly => "coach-inferred-hardly",
        Amount::AboutOne => "coach-inferred-about-one",
        Amount::Several => "coach-inferred-several",
    };
    let mut out = i18n.tf(id, &[("holder", who.into()), ("class", what.into())]);
    if figures {
        out.push(' ');
        out.push_str(&i18n.tf(
            "coach-inferred-figures",
            &[
                ("expected", format!("{:.1}", i.inference.expected).into()),
                ("min", i.inference.min.into()),
                ("max", i.inference.max.into()),
            ],
        ));
    }
    out
}

fn judgement(i18n: &I18n, j: Judgement) -> String {
    i18n.t(match j {
        Judgement::Best => "coach-judgement-best",
        Judgement::Equivalent => "coach-judgement-equivalent",
        Judgement::Worse => "coach-judgement-worse",
        Judgement::Weak => "coach-judgement-weak",
        Judgement::Screened => "coach-judgement-screened",
        Judgement::NotEvaluated => "coach-judgement-not-evaluated",
    })
}

fn reason(i18n: &I18n, other: &str, r: Reason, figures: bool) -> String {
    let compare = |id_more: &str, id_less: &str, this: f64, that: f64, fmt: fn(f64) -> String| {
        let id = if this > that { id_more } else { id_less };
        let mut out = i18n.tf(id, &[("other", other.into())]);
        if figures {
            out.push_str(&format!(" ({} / {})", fmt(this), fmt(that)));
        }
        out
    };
    match r {
        Reason::WinChance { this, other: that } => compare(
            "coach-reason-win-more",
            "coach-reason-win-less",
            this,
            that,
            percent,
        ),
        Reason::Points { this, other: that } => compare(
            "coach-reason-points-more",
            "coach-reason-points-less",
            this,
            that,
            points,
        ),
        Reason::DeclarerPoints { this, other: that } => compare(
            "coach-reason-declarer-more",
            "coach-reason-declarer-less",
            this,
            that,
            points,
        ),
        Reason::ThrownIn { this, other: that } => compare(
            "coach-reason-thrown-in-more",
            "coach-reason-thrown-in-less",
            this,
            that,
            percent,
        ),
        Reason::SameValue => i18n.tf("coach-reason-same", &[("other", other.into())]),
        Reason::WithinUncertainty => i18n.t("coach-reason-uncertain"),
    }
}

/// Whose winning chance an estimate gives.
fn party(i18n: &I18n, role: Role) -> String {
    i18n.t(match role {
        Role::Bidder => "coach-party-own",
        Role::Declarer => "coach-party-declarer",
        Role::Defender => "coach-party-defenders",
    })
}

/// The figures of one estimate.
fn metrics(i18n: &I18n, role: Role, e: &Estimate) -> String {
    let party = party(i18n, role);
    let mut out = i18n.tf(
        "coach-metric-win",
        &[
            ("party", party.into()),
            ("chance", percent(e.win_chance).into()),
        ],
    );
    out.push_str(" · ");
    out.push_str(&i18n.tf(
        "coach-metric-score",
        &[
            ("score", points(e.expected_score).into()),
            ("se", format!("{:.1}", e.score_std_error).into()),
        ],
    ));
    if role != Role::Declarer {
        out.push_str(" · ");
        out.push_str(&i18n.tf(
            "coach-metric-game-points",
            &[("points", points(e.expected_game_points).into())],
        ));
    }
    out
}

/// What the seat knows and can infer, in two sections.
fn considerations(
    i18n: &I18n,
    view: &TableView,
    considerations: &[Consideration],
    figures: bool,
) -> Vec<Line> {
    let mut out = Vec::new();
    let facts: Vec<Line> = considerations
        .iter()
        .filter_map(|c| match c {
            Consideration::Known(o) => {
                Some(line(format!("• {}", fact(i18n, view, *o)), Style::Normal))
            }
            Consideration::Inferred(_) => None,
        })
        .collect();
    if !facts.is_empty() {
        out.push(line(i18n.t("coach-section-facts"), Style::Heading));
        out.extend(facts);
    }
    let guesses: Vec<Line> = considerations
        .iter()
        .filter_map(|c| match c {
            Consideration::Inferred(i) => Some(line(
                format!("• {}", inferred(i18n, view, i, figures)),
                Style::Normal,
            )),
            Consideration::Known(_) => None,
        })
        .collect();
    if !guesses.is_empty() {
        out.push(line(i18n.t("coach-section-inferred"), Style::Heading));
        out.extend(guesses);
    }
    out
}

/// The basis and the assumptions of the estimates.
fn basis(i18n: &I18n, basis: &Basis, assumptions: &[Assumption], figures: bool) -> Vec<Line> {
    let mut out = vec![line(i18n.t("coach-section-basis"), Style::Heading)];
    out.push(line(
        if basis.enumerated {
            i18n.t("coach-basis-exact")
        } else if figures {
            i18n.tf(
                "coach-basis-sampled-figures",
                &[("finished", basis.finished.into())],
            )
        } else {
            i18n.t("coach-basis-sampled")
        },
        Style::Weak,
    ));
    for a in assumptions {
        out.push(line(
            format!(
                "• {}",
                i18n.t(match a {
                    Assumption::UniformHiddenCards => "coach-assumption-uniform",
                    Assumption::BiddingEvidence => "coach-assumption-bidding",
                    Assumption::DoubleDummyPlay => "coach-assumption-double-dummy",
                    Assumption::ContinuationPolicy => "coach-assumption-policy",
                    Assumption::BidAsLimit => "coach-assumption-bid-limit",
                })
            ),
            Style::Weak,
        ));
    }
    out
}

/// The label of the help button that reveals `stage`.
pub fn stage_button(i18n: &I18n, stage: Stage) -> String {
    i18n.t(match stage {
        Stage::Considerations => "button-coach-considerations",
        Stage::Situation => "button-coach-situation",
        Stage::Figures => "button-coach-figures",
    })
}

/// The hints before the move, as lines: the revealed stages only, the
/// options in neutral order and never judged (D-49).
pub fn hints(i18n: &I18n, view: &TableView, h: &Hints, figures: bool) -> Vec<Line> {
    let complete = h.status == skat_app::coach_model::Status::Complete;
    let mut out = vec![
        line(
            format!("{} · {}", decision(i18n, h.decision), role(i18n, h.role)),
            Style::Heading,
        ),
        line(objective(i18n, h.role, h.objective), Style::Normal),
    ];
    if !complete {
        out.push(line(i18n.t("coach-provisional"), Style::Strong));
    }
    if let Some(l) = &h.lesson {
        out.extend(lesson_question(i18n, l));
    }
    out.extend(considerations(i18n, view, &h.considerations, figures));
    if let Some(s) = &h.situation {
        out.push(line(i18n.t("coach-section-situation"), Style::Heading));
        out.push(line(
            match s.situation {
                Situation::Forced => i18n.t("coach-situation-forced"),
                Situation::OneStandsOut => i18n.t("coach-situation-one"),
                Situation::SeveralEqual { count } => {
                    i18n.tf("coach-situation-several", &[("count", count.into())])
                }
                Situation::Unclear => i18n.t("coach-situation-unclear"),
            },
            Style::Strong,
        ));
        if s.important && complete {
            out.push(line(i18n.t("coach-important"), Style::Strong));
        }
        if h.compare_question {
            // The same question for every option and ranking (D-49).
            out.push(line(i18n.t("coach-question-compare"), Style::Strong));
        }
        match (&h.figures, figures) {
            (Some(all), true) => {
                out.push(line(i18n.t("coach-section-figures"), Style::Heading));
                for f in all.iter().take(SHOWN_OPTIONS) {
                    out.push(line(action(i18n, view, f.action), Style::Normal));
                    out.push(line(
                        match &f.estimate {
                            Some(e) => metrics(i18n, h.role, e),
                            None => i18n.t("coach-judgement-not-evaluated"),
                        },
                        Style::Weak,
                    ));
                }
            }
            _ => {
                out.push(line(i18n.t("coach-section-options"), Style::Heading));
                for &a in s.options.iter().take(SHOWN_OPTIONS) {
                    out.push(line(format!("• {}", action(i18n, view, a)), Style::Normal));
                }
            }
        }
        let rest = s.options.len().saturating_sub(SHOWN_OPTIONS);
        if rest > 0 {
            out.push(line(
                i18n.tf("coach-more-options", &[("count", rest.into())]),
                Style::Weak,
            ));
        }
    }
    out.extend(basis(i18n, &h.basis, &h.assumptions, figures));
    out
}

/// The note on the human's last move.
pub fn note(i18n: &I18n, last: &LastMove) -> Line {
    let text = i18n.t(match last.note {
        None => "coach-note-computing",
        Some(Note::Good) => "coach-note-good",
        Some(Note::AboutEqual) => "coach-note-about-equal",
        Some(Note::SomewhatWeaker) => "coach-note-somewhat-weaker",
        Some(Note::ClearlyWeaker) => "coach-note-clearly-weaker",
        Some(Note::Forced) => "coach-note-forced",
        Some(Note::Uncertain) => "coach-note-uncertain",
    });
    let played = action_at(i18n, last.bid, last.hand_game, last.action);
    line(
        i18n.tf(
            "coach-note",
            &[("action", played.into()), ("note", text.into())],
        ),
        match last.note {
            Some(Note::Good) => Style::Strong,
            _ => Style::Normal,
        },
    )
}

/// What a reason compares, with the party it is counted for.
fn metric_label(i18n: &I18n, role: Role, r: Reason) -> Option<String> {
    Some(match r {
        Reason::WinChance { .. } => i18n.tf(
            "coach-metric-label-win",
            &[("party", party(i18n, role).into())],
        ),
        Reason::Points { .. } => i18n.t(if role == Role::Defender {
            "coach-metric-label-points-defender"
        } else {
            "coach-metric-label-points"
        }),
        Reason::DeclarerPoints { .. } => i18n.t("coach-metric-label-declarer"),
        Reason::ThrownIn { .. } => i18n.t("coach-metric-label-thrown-in"),
        Reason::SameValue | Reason::WithinUncertainty => return None,
    })
}

/// The model comparison of `o` with the option it was compared with, each
/// reason under the name of its metric.
fn comparison(
    i18n: &I18n,
    role: Role,
    o: &ComparedOption,
    name: impl Fn(Action) -> String,
    figures: bool,
) -> Vec<Line> {
    let Some(other) = o.compared_with else {
        return Vec::new();
    };
    let other = name(other);
    let mut out = vec![line(
        i18n.tf(
            "coach-details-comparison",
            &[
                ("action", name(o.action).into()),
                ("other", other.clone().into()),
            ],
        ),
        Style::Strong,
    )];
    out.extend(o.reasons.iter().map(|&r| {
        let text = reason(i18n, &other, r, figures);
        let text = match metric_label(i18n, role, r) {
            Some(label) => format!("• {label}: {text}"),
            None => format!("• {text}"),
        };
        line(text, Style::Normal)
    }));
    out
}

/// The model's judgement of the chosen option, qualified as the model's.
fn chosen(i18n: &I18n, r: &MoveReview, name: &dyn Fn(Action) -> String) -> String {
    let action = name(r.chosen.action);
    let best = r.best.as_ref().map(|b| name(b.action));
    let complete = r.status == skat_app::coach_model::Status::Complete;
    let id = match (r.chosen.judgement, &best) {
        // Estimates of an unfinished analysis back no judgement.
        _ if !complete => "coach-moment-chosen-provisional",
        (Judgement::Best, _) => "coach-moment-chosen-best",
        (Judgement::Equivalent, Some(_)) => "coach-moment-chosen-equivalent",
        (Judgement::Worse, Some(_)) => "coach-moment-chosen-worse",
        (Judgement::Weak, Some(_)) => "coach-moment-chosen-weak",
        _ => "coach-moment-chosen-unrated",
    };
    i18n.tf(
        id,
        &[
            ("action", action.into()),
            ("best", best.unwrap_or_default().into()),
        ],
    )
}

/// The table at a debriefing moment as the player saw it then (issue
/// #47): from the decision's snapshot only, never the final hand or a
/// later fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SituationText {
    /// Seat, role and dealer; bid or contract; Kontra; own Skat and open
    /// cards when known then.
    pub lines: Vec<Line>,
    /// Label of the hand.
    pub hand_label: String,
    /// The player's cards before the move, in display order.
    pub hand: Vec<Card>,
    /// The cards of the chosen action (played or discarded), marked in
    /// the hand.
    pub chosen: Vec<Card>,
    /// Label of the trick in progress; empty before card play.
    pub trick_label: String,
    /// The trick in progress then: player name and card, in play order.
    pub trick: Vec<(String, Card)>,
    /// Heading of the history; collapsed by default.
    pub history_title: String,
    /// The bidding and the completed tricks before the decision.
    pub history: Vec<Line>,
}

/// The table of the moment's snapshot as text and cards.
fn situation(i18n: &I18n, role: Role, c: &DecisionContext) -> SituationText {
    let name = |p: PlayerId| text::player(i18n, p);
    let card_of = |p: PlayerId, card: Card| {
        i18n.tf(
            "coach-context-card",
            &[
                ("player", name(p).into()),
                ("card", text::card(i18n, card).into()),
            ],
        )
    };
    // Already in display order.
    let cards = |cards: &[Card]| {
        let names: Vec<String> = cards.iter().map(|&c| text::card(i18n, c)).collect();
        names.join(", ")
    };
    let mut lines = vec![line(
        i18n.tf(
            "coach-context-position",
            &[
                ("seat", text::seat(i18n, c.seat(PlayerId::HUMAN)).into()),
                ("role", self::role(i18n, role).into()),
                ("dealer", name(c.dealer).into()),
            ],
        ),
        Style::Normal,
    )];
    lines.push(line(
        match (c.declarer, c.contract) {
            (Some(p), Some(k)) => i18n.tf(
                "coach-context-contract",
                &[
                    ("game", text::contract(i18n, k).into()),
                    ("player", name(p).into()),
                    ("value", c.bid.into()),
                ],
            ),
            (Some(p), None) => i18n.tf(
                "coach-context-declarer",
                &[("player", name(p).into()), ("value", c.bid.into())],
            ),
            _ if c.bid > 0 => i18n.tf("coach-context-bid", &[("value", c.bid.into())]),
            _ => i18n.t("coach-context-no-bid"),
        },
        Style::Normal,
    ));
    if let Some(level) = text::kontra(i18n, c.kontra) {
        lines.push(line(
            i18n.tf("coach-context-kontra", &[("level", level.into())]),
            Style::Normal,
        ));
    }
    if let Some(skat) = &c.own_skat {
        lines.push(line(
            i18n.tf("coach-context-own-skat", &[("cards", cards(skat).into())]),
            Style::Normal,
        ));
    }
    if let (Some(open), Some(p)) = (&c.declarer_cards, c.declarer) {
        lines.push(line(
            i18n.tf(
                "coach-context-open",
                &[("player", name(p).into()), ("cards", cards(open).into())],
            ),
            Style::Normal,
        ));
    }
    let trick_label = match c.trick() {
        Some(t) if c.current_trick.is_empty() => {
            i18n.tf("coach-context-lead", &[("trick", t.into())])
        }
        Some(t) => i18n.tf("coach-context-trick", &[("trick", t.into())]),
        None => String::new(),
    };
    let mut history = vec![line(
        if c.bidding.is_empty() {
            i18n.t("coach-context-no-calls")
        } else {
            let calls: Vec<String> = c
                .bidding
                .iter()
                .map(|&(p, call)| {
                    i18n.tf(
                        "coach-context-call",
                        &[
                            ("player", name(p).into()),
                            ("call", text::call(i18n, call).into()),
                        ],
                    )
                })
                .collect();
            i18n.tf("coach-context-calls", &[("calls", calls.join(", ").into())])
        },
        Style::Normal,
    )];
    if c.phase == Phase::Spiel && c.tricks.is_empty() {
        history.push(line(i18n.t("coach-context-no-tricks"), Style::Normal));
    }
    for (n, t) in c.tricks.iter().enumerate() {
        let played: Vec<String> = t.cards.iter().map(|p| card_of(p.player, p.card)).collect();
        history.push(line(
            i18n.tf(
                "coach-context-past-trick",
                &[
                    ("trick", (n + 1).into()),
                    ("cards", played.join(", ").into()),
                    ("winner", name(t.winner).into()),
                    ("augen", t.augen.into()),
                ],
            ),
            Style::Normal,
        ));
    }
    SituationText {
        lines,
        hand_label: i18n.t("coach-context-hand"),
        hand: c.hand.clone(),
        chosen: match c.action {
            Action::Play(card) => vec![card],
            Action::Discard(cards) => cards.iter().collect(),
            _ => Vec::new(),
        },
        trick_label,
        trick: c
            .current_trick
            .iter()
            .map(|p| (name(p.player), p.card))
            .collect(),
        history_title: i18n.t("coach-context-history"),
        history,
    }
}

/// One moment of the debriefing: a short summary, the table then, its
/// lesson's evidence and its analysis details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MomentText {
    /// Trick and decision (and the focus mark).
    pub title: Line,
    /// The table as the player saw it at the decision.
    pub situation: SituationText,
    /// What was decided, how the model rates it and the lesson's checking
    /// question; never an estimate.
    pub summary: Vec<Line>,
    /// Heading of the lesson's evidence (the lesson's section title).
    pub lesson_title: String,
    /// The lesson's evidence and exceptions; collapsed by default.
    pub lesson: Vec<Line>,
    /// Comparison by metric, figures (if shown), what was known and the
    /// model assumptions; collapsed by default.
    pub details: Vec<Line>,
}

/// One moment of the debriefing. The summary names the decision, the
/// choice and the model's judgement, then the moment's lesson (question
/// context, evidence, exception, transfer question), and says honestly
/// that the lesson does not explain the model's rating: score differences
/// are no cause and stay in the details (D-52). The table, trick number
/// and seat labels come from the moment's snapshot (issue #47).
fn moment(i18n: &I18n, view: &TableView, dm: &DebriefingMoment, figures: bool) -> MomentText {
    let (m, c) = (&dm.moment, &dm.context);
    let r = &m.review;
    // Seat names in the lesson and the facts follow the seats of the
    // decision's game, whatever the table shows now.
    let mut view = view.clone();
    for p in &mut view.players {
        p.seat = c.seat(p.player);
    }
    let view = &view;
    let name = |a: Action| action_at(i18n, r.bid, r.hand_game, a);
    let mut what = decision(i18n, r.decision);
    if let Some(trick) = c.trick() {
        what = format!(
            "{} · {what}",
            i18n.tf("coach-moment-trick", &[("trick", trick.into())])
        );
    }
    if m.focused {
        what = format!("{what} · {}", i18n.t("coach-moment-focus"));
    }
    let title = line(what, Style::Heading);
    let mut summary = vec![line(chosen(i18n, r, &name), Style::Strong)];
    let trap = r.trap.as_ref().filter(|_| m.kind == MomentKind::Good);
    if let Some(trap) = trap {
        summary.push(line(
            i18n.tf("coach-moment-trap", &[("trap", name(trap.action).into())]),
            Style::Normal,
        ));
    }
    // The lesson: its checking questions in the summary, its evidence and
    // exceptions one click away.
    let mut taught = lesson(i18n, view, &m.lesson).into_iter();
    let lesson_title = taught.next().map(|l| l.text).unwrap_or_default();
    let (questions, evidence): (Vec<Line>, Vec<Line>) =
        taught.partition(|l| l.style == Style::Strong);
    summary.extend(questions);
    summary.push(line(i18n.t("coach-moment-no-reason"), Style::Weak));
    // One level below the moment: sections as emphasised lines.
    let below = |l: Line| match l.style {
        Style::Heading => line(l.text, Style::Strong),
        _ => l,
    };

    let mut details = Vec::new();
    let compared = match m.kind {
        MomentKind::Bad | MomentKind::Neutral => Some(&r.chosen),
        MomentKind::Good => trap,
    };
    if let Some(o) = compared {
        details.extend(comparison(i18n, r.role, o, name, figures));
    }
    // The model's rating of each option named, with its figures if shown.
    let mut shown = Vec::new();
    for o in [Some(&r.chosen), r.best.as_ref(), trap]
        .into_iter()
        .flatten()
    {
        if shown.contains(&o.action) {
            continue;
        }
        shown.push(o.action);
        details.push(line(
            i18n.tf(
                "coach-details-rating",
                &[
                    ("action", name(o.action).into()),
                    ("judgement", judgement(i18n, o.judgement).into()),
                ],
            ),
            Style::Normal,
        ));
        if let Some(e) = o.estimate.filter(|_| figures) {
            details.push(line(metrics(i18n, r.role, &e), Style::Weak));
        }
    }
    if let Some(trap) = trap.filter(|_| figures) {
        details.push(line(
            i18n.tf(
                "coach-moment-trap-loss",
                &[("points", format!("{:.1}", trap.gap).into())],
            ),
            Style::Weak,
        ));
    }
    details.extend(
        considerations(i18n, view, &r.considerations, figures)
            .into_iter()
            .map(below),
    );
    details.extend(
        basis(i18n, &r.basis, &r.assumptions, figures)
            .into_iter()
            .map(below),
    );
    MomentText {
        title,
        situation: situation(i18n, r.role, c),
        summary,
        lesson_title,
        lesson: evidence,
        details,
    }
}

/// The end-of-game debriefing (D-49): heading and status lines, then at
/// most three moments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebriefingText {
    /// Heading and status.
    pub lines: Vec<Line>,
    /// The moments in game order.
    pub moments: Vec<MomentText>,
}

/// The end-of-game debriefing (D-49).
pub fn debriefing(i18n: &I18n, view: &TableView, d: &Debriefing, figures: bool) -> DebriefingText {
    let mut lines = vec![line(i18n.t("coach-debriefing-title"), Style::Heading)];
    let mut moments = Vec::new();
    match d {
        Debriefing::Computing { remaining } => lines.push(line(
            i18n.tf(
                "coach-debriefing-computing",
                &[("count", (*remaining).into())],
            ),
            Style::Weak,
        )),
        Debriefing::Ready(all) if all.is_empty() => {
            lines.push(line(i18n.t("coach-debriefing-none"), Style::Normal));
        }
        Debriefing::Ready(all) => {
            moments = all.iter().map(|m| moment(i18n, view, m, figures)).collect();
        }
    }
    DebriefingText { lines, moments }
}

/// The warning before a weak action, as lines. It names no alternative
/// (D-49).
pub fn warning(i18n: &I18n, view: &TableView, w: &Warning, figures: bool) -> Vec<Line> {
    let chosen = action(i18n, view, w.chosen.action);
    let mut out = vec![line(
        i18n.tf("coach-warning-text", &[("action", chosen.into())]),
        Style::Strong,
    )];
    if figures {
        out.push(line(
            i18n.tf(
                "coach-warning-loss",
                &[("points", format!("{:.1}", w.chosen.gap).into())],
            ),
            Style::Weak,
        ));
    }
    out
}

/// Status of an unavailable analysis.
pub fn unavailable(i18n: &I18n, reason: Unavailable) -> String {
    i18n.t(match reason {
        Unavailable::Cancelled => "coach-unavailable-cancelled",
        Unavailable::Failed => "coach-unavailable-failed",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;
    use skat_app::coach_model::{self as model, Explanation, Moment};
    use skat_app::{
        Budget, Call, Card, CoachLevel, CoachSettings, Command, KontraLevel, Level, Opponents,
        Phase, PlayedCard, Rank, Seat, SeriesLength, Session, SessionConfig, Suit, TrickView,
    };

    /// Explanations of the human's card-play decisions in a seeded series.
    fn explanations() -> Vec<(TableView, Explanation)> {
        let mut out = Vec::new();
        let mut s = Session::new(SessionConfig {
            seed: 11,
            series: SeriesLength::Fixed(3),
            kontra_re: true,
            opponents: Opponents::Levels([Level::Easy; 2]),
        });
        s.set_coach_budget(Budget {
            samples: 6,
            nodes_per_task: 20_000_000,
        });
        s.execute(Command::SetCoach(CoachSettings {
            level: CoachLevel::EveryMove,
            warnings: false,
            show_figures: true,
            focus: None,
        }))
        .unwrap();
        while !s.view().series_finished {
            s.run_opponents();
            let view = s.view();
            if view.series_finished {
                break;
            }
            if view.legal_actions.is_empty() {
                s.execute(Command::NextGame).unwrap();
                continue;
            }
            if view.phase == Phase::Spiel && view.tricks_played >= 3 {
                let result = s.coach_job().unwrap().run(None);
                if let Some(e) = result.explanation() {
                    out.push((view.clone(), e.clone()));
                }
            }
            let action = if view.legal_actions.contains(&Action::Pass) {
                Action::Pass
            } else {
                view.legal_actions[0]
            };
            s.execute(Command::Act(action)).unwrap();
        }
        out
    }

    /// Each moment with the snapshot `context` makes for its chosen action.
    fn with_contexts(
        moments: Vec<Moment>,
        context: impl Fn(Action) -> DecisionContext,
    ) -> Vec<DebriefingMoment> {
        moments
            .into_iter()
            .map(|moment| DebriefingMoment {
                context: DecisionContext {
                    action: moment.review.chosen.action,
                    ..context(moment.review.chosen.action)
                },
                moment,
            })
            .collect()
    }

    /// The snapshot of the human's decision shown by `view`.
    fn context_of(view: &TableView) -> DecisionContext {
        DecisionContext {
            game_number: view.game_number,
            actions: 0,
            dealer: view.dealer,
            seats: view.players.map(|p| p.seat),
            phase: view.phase,
            hand: view.hand.clone(),
            hand_sizes: view.players.map(|p| p.cards),
            bidding: view.bidding.clone(),
            bid: view.bid,
            declarer: view.declarer,
            hand_game: view.hand_game,
            contract: view.contract,
            kontra: view.kontra,
            own_skat: view.own_skat.clone(),
            declarer_cards: view.declarer_cards.clone(),
            // Only the last trick is on the table; the count is right.
            tricks: vec![
                view.last_trick.clone().unwrap_or(TrickView {
                    cards: Vec::new(),
                    winner: PlayerId::HUMAN,
                    augen: 0,
                });
                usize::from(view.tricks_played)
            ],
            current_trick: view.current_trick.clone(),
            action: Action::Pass,
        }
    }

    fn joined(lines: &[Line]) -> String {
        lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The texts shown before the move at every stage, and the
    /// debriefing of every option as if chosen.
    fn rendered(i18n: &I18n, view: &TableView, e: &Explanation, figures: bool) -> Vec<Vec<Line>> {
        let mut out: Vec<Vec<Line>> = [Stage::Considerations, Stage::Situation, Stage::Figures]
            .into_iter()
            .map(|stage| hints(i18n, view, &model::hints(e, stage), figures))
            .collect();
        let reviews: Vec<_> = view
            .legal_actions
            .iter()
            .filter_map(|&a| model::review(e, a))
            .collect();
        let moments = with_contexts(model::moments(&reviews, None), |_| context_of(view));
        out.push(flat(&debriefing(
            i18n,
            view,
            &Debriefing::Ready(moments),
            figures,
        )));
        out
    }

    /// Every line of a debriefing, details included.
    fn flat(d: &DebriefingText) -> Vec<Line> {
        let mut out = d.lines.clone();
        for m in &d.moments {
            out.push(m.title.clone());
            out.extend(m.situation.lines.iter().cloned());
            out.extend(m.situation.history.iter().cloned());
            out.extend(m.summary.iter().cloned());
            out.push(line(m.lesson_title.clone(), Style::Heading));
            out.extend(m.lesson.iter().cloned());
            out.extend(m.details.iter().cloned());
        }
        out
    }

    #[test]
    fn hints_never_name_or_rank_the_best_option() {
        let all = explanations();
        for language in Language::ALL {
            let i18n = I18n::new(language);
            // "About as good" also reads the situation, so it is left out.
            let judged: Vec<String> = [Judgement::Best, Judgement::Worse, Judgement::Weak]
                .into_iter()
                .map(|j| judgement(&i18n, j))
                .collect();
            for (view, e) in &all {
                for stage in [Stage::Considerations, Stage::Situation, Stage::Figures] {
                    let text = joined(&hints(&i18n, view, &model::hints(e, stage), true));
                    for j in &judged {
                        assert!(!text.contains(j.as_str()), "{stage:?}: {text}");
                    }
                    if stage == Stage::Considerations {
                        // No option is named before the situation stage.
                        for &a in &view.legal_actions {
                            assert!(!text.contains(&format!("• {}", action(&i18n, view, a))));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn hidden_figures_keep_the_reasoning_without_estimates() {
        let all = explanations();
        assert!(all.len() >= 5, "{}", all.len());
        let mut moments = 0;
        for language in Language::ALL {
            let i18n = I18n::new(language);
            for (view, e) in &all {
                let with = rendered(&i18n, view, e, true);
                let without = rendered(&i18n, view, e, false);
                moments += with.last().map_or(0, Vec::len);
                for (with, without) in with.iter().zip(&without) {
                    // The verbal reasoning stays: same headings and reasons
                    // (the figures stage falls back to the plain options).
                    let headings =
                        |lines: &[Line]| lines.iter().filter(|l| l.style == Style::Heading).count();
                    assert_eq!(headings(with), headings(without));
                    let without = joined(without);
                    // No estimated figure leaks: no percentages, no ±, none
                    // of the formatted estimates.
                    assert!(
                        !without.contains('%') && !without.contains('±'),
                        "{without}"
                    );
                    for o in &e.options {
                        if let Some(est) = o.estimate {
                            assert!(!without.contains(&percent(est.win_chance)));
                            assert!(!without.contains(&points(est.expected_score)));
                        }
                    }
                    for c in &e.considerations {
                        if let Consideration::Inferred(i) = c
                            && !i.inference.certain
                        {
                            assert!(!without.contains(&format!("{:.1}", i.inference.expected)));
                        }
                    }
                }
                let figures_stage = joined(&with[2]);
                assert!(
                    figures_stage.contains('%') || e.options.iter().all(|o| o.estimate.is_none())
                );
            }
        }
        assert!(moments > 0, "some debriefing was rendered");
    }

    #[test]
    fn requested_figures_list_every_estimate_in_neutral_order_without_labels() {
        // D-60: only an explicitly requested stage 3 with figures on shows
        // per-option estimates, in the legal order, with no label. Their
        // values may still let a player infer the model's preference; that
        // is the opt-in exception, not checked away here.
        let all = explanations();
        let mut checked = 0;
        for language in Language::ALL {
            let i18n = I18n::new(language);
            let heading = i18n.t("coach-section-figures");
            // Every judgement label (not evaluated is no judgement: it
            // stands for a missing estimate).
            let labels: Vec<String> = [
                Judgement::Best,
                Judgement::Equivalent,
                Judgement::Worse,
                Judgement::Weak,
                Judgement::Screened,
            ]
            .into_iter()
            .map(|j| judgement(&i18n, j))
            .collect();
            for (view, e) in &all {
                let h = model::hints(e, Stage::Figures);
                let lines = hints(&i18n, view, &h, true);
                let at = lines
                    .iter()
                    .position(|l| l.text == heading)
                    .expect("figures heading");
                let shown = view.legal_actions.len().min(SHOWN_OPTIONS);
                let listed = &lines[at + 1..at + 1 + 2 * shown];
                for (pair, &a) in listed.chunks(2).zip(&view.legal_actions) {
                    // The option as it is, no bullet, mark or label …
                    assert_eq!(pair[0].text, action(&i18n, view, a));
                    assert_eq!(pair[0].style, Style::Normal);
                    // … then its own figures, never a judgement.
                    let o = e.options.iter().find(|o| o.action == a).unwrap();
                    let expected = o.estimate.map_or_else(
                        || i18n.t("coach-judgement-not-evaluated"),
                        |est| metrics(&i18n, h.role, &est),
                    );
                    assert_eq!(pair[1].text, expected);
                    for label in &labels {
                        assert!(!pair[1].text.contains(label.as_str()), "{label}");
                    }
                }
                // Figures off (or not requested): no figures section at
                // all, only the plain options.
                for (stage, figures) in [
                    (Stage::Considerations, true),
                    (Stage::Situation, true),
                    (Stage::Situation, false),
                    (Stage::Figures, false),
                ] {
                    let lines = hints(&i18n, view, &model::hints(e, stage), figures);
                    assert!(lines.iter().all(|l| l.text != heading), "{stage:?}");
                }
                checked += 1;
            }
        }
        assert!(checked >= 10, "{checked}");
    }

    // ------------------------------------------------------------------
    // Debriefing fixtures (D-52). The decision: trick 5, the human defends;
    // Kreuz-Bube is best, Pik-Sieben a natural trap clearly lower, the
    // Herz-Sieben in between. The figures are invented for the fixture
    // only; what matters is where they may appear.

    const BEST: Card = Card::new(Suit::Kreuz, Rank::Bube);
    const MIDDLE: Card = Card::new(Suit::Herz, Rank::Sieben);
    const TRAP: Card = Card::new(Suit::Pik, Rank::Sieben);

    fn estimate(win_chance: f64, expected_score: f64) -> model::Estimate {
        model::Estimate {
            win_chance,
            expected_score,
            expected_game_points: -2.0 * expected_score,
            thrown_in: 0.0,
            score_std_error: 1.5,
            samples: 24,
            exact: false,
        }
    }

    fn option(card: Card, judgement: Judgement, gap: f64, reasons: Vec<Reason>) -> ComparedOption {
        let e = match card {
            BEST => estimate(0.62, 21.4),
            MIDDLE => estimate(0.55, 15.4),
            _ => estimate(0.31, 7.1),
        };
        ComparedOption {
            action: Action::Play(card),
            natural: true,
            judgement,
            estimate: Some(e),
            value: Some(e.expected_score),
            gap,
            margin: 3.0,
            compared_with: Some(Action::Play(if card == BEST { MIDDLE } else { BEST })),
            reasons,
        }
    }

    fn trap() -> ComparedOption {
        option(
            TRAP,
            Judgement::Weak,
            14.3,
            vec![
                Reason::WinChance {
                    this: 0.31,
                    other: 0.62,
                },
                Reason::Points {
                    this: 7.1,
                    other: 21.4,
                },
            ],
        )
    }

    /// The decision with `chosen` played; it teaches the remaining Trumpf.
    fn fixture(chosen: ComparedOption) -> model::MoveReview {
        let lesson = fixture_lesson(Concept::RemainingTrumpf);
        let comparison = model::Comparison {
            chosen: chosen.action,
            best: Some(Action::Play(BEST)),
            reliability: model::Reliability::Reliable,
            mechanisms: Vec::new(),
        };
        model::MoveReview {
            decision: Decision::Card,
            role: Role::Defender,
            status: model::Status::Complete,
            bid: 18,
            hand_game: Some(false),
            trick: Some(5),
            important: true,
            forced: false,
            considerations: vec![
                Consideration::Known(Observation::TrumpsOut { count: 4 }),
                Consideration::Inferred(model::Inferred {
                    inference: model::Inference {
                        holder: Place::Seat(Seat::Hinterhand),
                        class: CardClass::Trumps,
                        expected: 1.2,
                        min: 0,
                        max: 3,
                        certain: false,
                    },
                    amount: Amount::AboutOne,
                }),
            ],
            chosen,
            best: Some(option(BEST, Judgement::Best, 0.0, Vec::new())),
            trap: Some(trap()),
            basis: model::Basis {
                finished: 24,
                planned: 24,
                enumerated: false,
                effective_samples: 20.0,
            },
            assumptions: vec![Assumption::UniformHiddenCards, Assumption::DoubleDummyPlay],
            evidence: model::Evidence {
                version: 1,
                snapshot: model::Snapshot {
                    seat: Seat::Hinterhand,
                    phase: Phase::Spiel,
                    role: Role::Defender,
                    game: None,
                    bids: 3,
                    own_calls: 1,
                    tricks: 4,
                    trick_cards: 1,
                    own: Default::default(),
                    own_skat: None,
                    revealed: None,
                },
                status: model::Status::Complete,
                items: Vec::new(),
            },
            comparison,
            teaching: lesson.teaching.clone(),
            lesson: Some(lesson.content.clone()),
            taught: vec![lesson],
        }
    }

    /// The fixture's lesson: where the Trumpf are after a Trumpf trick.
    fn fixture_lesson(concept: Concept) -> model::Taught {
        model::Taught {
            concept,
            teaching: model::Teaching::Lesson {
                id: model::LessonId {
                    key: concept.key().into(),
                    version: 1,
                },
                items: Vec::new(),
            },
            content: LessonContent::RemainingTrumpf {
                game: GameType::Farbe(Suit::Herz),
                sets: herz_sets(),
            },
        }
    }

    /// `r` with only the lesson of `concept` (the content stays the
    /// fixture's; the selection reads the concept).
    fn teaching(mut r: model::MoveReview, concept: Concept) -> model::MoveReview {
        r.taught = vec![fixture_lesson(concept)];
        r
    }

    /// `r` without any supported lesson.
    fn fallback(mut r: model::MoveReview) -> model::MoveReview {
        r.taught.clear();
        r.teaching = model::Teaching::Fallback;
        r.lesson = None;
        r
    }

    fn good() -> model::MoveReview {
        fixture(option(BEST, Judgement::Best, 0.0, Vec::new()))
    }

    fn equivalent() -> model::MoveReview {
        let reasons = vec![Reason::WithinUncertainty];
        fixture(option(MIDDLE, Judgement::Equivalent, 0.5, reasons))
    }

    fn weak() -> model::MoveReview {
        fixture(trap())
    }

    fn worse() -> model::MoveReview {
        let reasons = vec![Reason::Points {
            this: 15.4,
            other: 21.4,
        }];
        fixture(option(MIDDLE, Judgement::Worse, 6.0, reasons))
    }

    fn uncertain() -> model::MoveReview {
        model::MoveReview {
            status: model::Status::BudgetExhausted,
            ..weak()
        }
    }

    fn forced() -> model::MoveReview {
        model::MoveReview {
            forced: true,
            ..good()
        }
    }

    const fn card(suit: Suit, rank: Rank) -> Card {
        Card::new(suit, rank)
    }

    /// The hand of the fixture decision: six cards before trick 5.
    const FIXTURE_HAND: [Card; 6] = [
        BEST,
        card(Suit::Pik, Rank::Dame),
        TRAP,
        card(Suit::Herz, Rank::Koenig),
        MIDDLE,
        card(Suit::Karo, Rank::Zehn),
    ];

    /// The snapshot of the fixture decision: game 1, player 1 (Links) deals,
    /// so the human sits in Mittelhand and Rechts in Vorhand. Rechts won
    /// the bidding at 18, plays Grand with Kontra, has won the first four
    /// tricks and led Herz-Ass to trick 5.
    fn fixture_context(action: Action) -> DecisionContext {
        let (human, left, right) = (PlayerId::HUMAN, PlayerId(1), PlayerId(2));
        let trick = |cards: [Card; 3], augen| TrickView {
            cards: [right, human, left]
                .into_iter()
                .zip(cards)
                .map(|(player, card)| PlayedCard { player, card })
                .collect(),
            winner: right,
            augen,
        };
        DecisionContext {
            game_number: 1,
            actions: 17,
            dealer: left,
            seats: [Seat::Mittelhand, Seat::Hinterhand, Seat::Vorhand],
            phase: Phase::Spiel,
            hand: FIXTURE_HAND.to_vec(),
            hand_sizes: [6, 6, 5],
            bidding: vec![
                (human, Call::Bid(18)),
                (right, Call::Hold(18)),
                (human, Call::Pass),
                (left, Call::Pass),
            ],
            bid: 18,
            declarer: Some(right),
            hand_game: Some(false),
            contract: Some(Contract::new(
                skat_app::Announcement::plain(GameType::Grand),
                false,
            )),
            kontra: KontraLevel::Kontra,
            own_skat: None,
            declarer_cards: None,
            tricks: vec![
                trick(
                    [
                        card(Suit::Kreuz, Rank::Ass),
                        card(Suit::Kreuz, Rank::Sieben),
                        card(Suit::Kreuz, Rank::Zehn),
                    ],
                    21,
                ),
                trick(
                    [
                        card(Suit::Pik, Rank::Bube),
                        card(Suit::Karo, Rank::Bube),
                        card(Suit::Karo, Rank::Sieben),
                    ],
                    4,
                ),
                trick(
                    [
                        card(Suit::Pik, Rank::Ass),
                        card(Suit::Pik, Rank::Acht),
                        card(Suit::Pik, Rank::Zehn),
                    ],
                    21,
                ),
                trick(
                    [
                        card(Suit::Karo, Rank::Ass),
                        card(Suit::Karo, Rank::Acht),
                        card(Suit::Karo, Rank::Koenig),
                    ],
                    15,
                ),
            ],
            current_trick: vec![PlayedCard {
                player: right,
                card: card(Suit::Herz, Rank::Ass),
            }],
            action,
        }
    }

    #[test]
    fn a_moment_shows_the_table_of_its_decision_not_the_current_one() {
        // The current table: a new deal, the human in Vorhand, no bid.
        let view = table();
        assert_eq!(view.players[0].seat, Seat::Vorhand);
        assert_ne!(view.hand, FIXTURE_HAND);
        for (language, expected) in [
            (
                Language::De,
                [
                    "Stich 5 · Karte spielen",
                    "Damals: du in Mittelhand, du bist Gegenspieler; Geber Links.",
                    "Spiel: Grand von Rechts, gereizt bis 18",
                    "Angesagt: Kontra",
                    "Deine Karten vor dem Zug:",
                    "Stich 5 bis dahin:",
                    "Rechts",
                    "Verlauf bis zu dieser Entscheidung",
                    "Reizverlauf: Du 18, Rechts Ja (18), Du Passe, Links Passe",
                    "Stich 1: Rechts Kreuz-Ass, Du Kreuz-Sieben, Links Kreuz-Zehn – Rechts sticht (21 Augen)",
                    "Stich 4: Rechts Karo-Ass, Du Karo-Acht, Links Karo-König – Rechts sticht (15 Augen)",
                    "Links hat vermutlich etwa einen der",
                ],
            ),
            (
                Language::En,
                [
                    "Trick 5 · Play a card",
                    "Then: you in Middlehand, you are a defender; dealer Left.",
                    "Game: Grand by Right, bid 18",
                    "Announced: Kontra",
                    "Your cards before the move:",
                    "Trick 5 so far:",
                    "Right",
                    "History up to this decision",
                    "Bidding so far: You 18, Right Yes (18), You Pass, Left Pass",
                    "Trick 1: Right Ace of Clubs, You Seven of Clubs, Left Ten of Clubs – Right takes it (21 card points)",
                    "Trick 4: Right Ace of Diamonds, You Eight of Diamonds, Left King of Diamonds – Right takes it (15 card points)",
                    "Left probably holds about one of",
                ],
            ),
        ] {
            let i18n = I18n::new(language);
            let d = debriefing(&i18n, &view, &ready(&[weak()]), false);
            let m = &d.moments[0];
            let s = &m.situation;
            assert_eq!(m.title.text, expected[0]);
            assert_eq!(texts(&s.lines), expected[1..4]);
            assert_eq!(s.hand_label, expected[4]);
            assert_eq!(s.hand, FIXTURE_HAND);
            assert_eq!(s.chosen, [TRAP], "the chosen card is marked");
            assert_eq!(s.trick_label, expected[5]);
            let led = card(Suit::Herz, Rank::Ass);
            assert_eq!(s.trick, [(expected[6].to_owned(), led)]);
            assert_eq!(s.history_title, expected[7]);
            assert_eq!(s.history.len(), 5, "the bidding and four tricks");
            assert_eq!(s.history[0].text, expected[8]);
            assert_eq!(s.history[1].text, expected[9]);
            assert_eq!(s.history[4].text, expected[10]);
            // The facts name the seats of that game: Hinterhand was Links.
            assert!(joined(&m.details).contains(expected[11]), "{:?}", m.details);
        }
    }

    #[test]
    fn a_situation_before_card_play_shows_no_trick_and_what_was_known() {
        let i18n = I18n::new(Language::De);
        let skat = vec![card(Suit::Herz, Rank::Acht), card(Suit::Karo, Rank::Neun)];
        let c = DecisionContext {
            phase: Phase::Ansage,
            hand: FIXTURE_HAND.to_vec(),
            bidding: vec![(PlayerId::HUMAN, Call::Bid(18)), (PlayerId(2), Call::Pass)],
            declarer: Some(PlayerId::HUMAN),
            contract: None,
            kontra: KontraLevel::None,
            own_skat: Some(skat),
            tricks: Vec::new(),
            current_trick: Vec::new(),
            ..fixture_context(Action::Pass)
        };
        let s = situation(&i18n, Role::Declarer, &c);
        assert_eq!(
            texts(&s.lines),
            [
                "Damals: du in Mittelhand, du bist Alleinspieler; Geber Links.",
                "Alleinspieler: Du, gereizt bis 18",
                "Gedrückt: Herz-Acht, Karo-Neun",
            ]
        );
        assert!(s.trick_label.is_empty() && s.trick.is_empty() && s.chosen.is_empty());
        assert_eq!(texts(&s.history), ["Reizverlauf: Du 18, Rechts Passe"]);
        // Ouvert cards and the lead of a trick.
        let open = vec![card(Suit::Kreuz, Rank::Ass)];
        let c = DecisionContext {
            own_skat: None,
            declarer_cards: Some(open),
            tricks: Vec::new(),
            current_trick: Vec::new(),
            ..fixture_context(Action::Play(BEST))
        };
        let s = situation(&i18n, Role::Defender, &c);
        assert_eq!(s.lines[3].text, "Offene Karten von Rechts: Kreuz-Ass");
        assert_eq!(s.trick_label, "Du spielst Stich 1 an.");
        assert_eq!(texts(&s.history)[1], "Noch kein Stich gespielt.");
    }

    fn table() -> TableView {
        Session::new(SessionConfig::default()).view()
    }

    fn ready(reviews: &[model::MoveReview]) -> Debriefing {
        Debriefing::Ready(with_contexts(
            model::moments(reviews, None),
            fixture_context,
        ))
    }

    /// Estimated figures of the fixture, as the frontend formats them.
    const ESTIMATES: [&str; 8] = [
        "62 %", "55 %", "31 %", "+21.4", "+15.4", "+7.1", "14.3", "1.2",
    ];

    fn assert_no_estimates(text: &str) {
        for figure in ESTIMATES {
            assert!(!text.contains(figure), "{figure} in {text}");
        }
        assert!(!text.contains('%') && !text.contains('±'), "{text}");
    }

    #[test]
    fn fixture_notes_follow_the_judgement() {
        let notes =
            [good(), equivalent(), worse(), weak(), uncertain(), forced()].map(|r| r.note());
        assert_eq!(
            notes,
            [
                Note::Good,
                Note::AboutEqual,
                Note::SomewhatWeaker,
                Note::ClearlyWeaker,
                Note::Uncertain,
                Note::Forced,
            ]
        );
        for language in Language::ALL {
            let i18n = I18n::new(language);
            let text = |note| {
                let last = LastMove {
                    action: Action::Play(MIDDLE),
                    bid: 18,
                    hand_game: Some(false),
                    note: Some(note),
                    lesson: None,
                };
                super::note(&i18n, &last).text
            };
            assert!(text(Note::Forced).contains(&i18n.t("coach-note-forced")));
            assert!(text(Note::Uncertain).contains(&i18n.t("coach-note-uncertain")));
        }
    }

    #[test]
    fn no_moments_says_so() {
        let view = table();
        for language in Language::ALL {
            let i18n = I18n::new(language);
            // Nothing, no lesson despite a clear loss, a forced move.
            for reviews in [vec![], vec![fallback(weak()), forced()]] {
                let d = debriefing(&i18n, &view, &ready(&reviews), true);
                assert!(d.moments.is_empty());
                assert_eq!(d.lines.len(), 2);
                assert_eq!(d.lines[1].text, i18n.t("coach-debriefing-none"));
            }
        }
    }

    /// The lesson lines of the fixture: title, questions, evidence.
    fn lesson_lines(i18n: &I18n, view: &TableView) -> (String, Vec<Line>, Vec<Line>) {
        let content = fixture_lesson(Concept::RemainingTrumpf).content;
        let mut all = lesson(i18n, view, &content).into_iter();
        let title = all.next().unwrap().text;
        let (questions, evidence) = all.partition(|l| l.style == Style::Strong);
        (title, questions, evidence)
    }

    #[test]
    fn one_weak_moment_teaches_its_lesson_and_keeps_the_rating_apart() {
        let view = table();
        for (language, head, chosen, apart) in [
            (
                Language::De,
                "Stich 5 · Karte spielen",
                "Du hast Pik-Sieben gewählt. Das Modell bewertet das deutlich niedriger als Kreuz-Bube.",
                "Die Lektion begründet nicht die Bewertung des Modells; den Modellvergleich findest du in den Analysedetails.",
            ),
            (
                Language::En,
                "Trick 5 · Play a card",
                "You chose Seven of Spades. The model rates it clearly lower than Jack of Clubs.",
                "The lesson does not explain the model's rating; the model's comparison is in the analysis details.",
            ),
        ] {
            let i18n = I18n::new(language);
            let d = debriefing(&i18n, &view, &ready(&[weak()]), false);
            assert_eq!(d.moments.len(), 1);
            let summary = &d.moments[0].summary;
            let title = d.moments[0].title.text.as_str();
            assert_eq!((title, summary[0].text.as_str()), (head, chosen));
            let (title, questions, evidence) = lesson_lines(&i18n, &view);
            assert!(!questions.is_empty() && !evidence.is_empty());
            // The checking question in the summary, the evidence apart.
            assert_eq!(summary[1..summary.len() - 1], questions[..]);
            assert_eq!(summary.last().unwrap().text, apart);
            assert_eq!(d.moments[0].lesson_title, title);
            assert_eq!(d.moments[0].lesson, evidence);
            assert_eq!(title, i18n.t("coach-section-lesson-review"));
            assert_no_estimates(&joined(summary));
            assert_no_estimates(&joined(&d.moments[0].lesson));
        }
    }

    #[test]
    fn provisional_and_focused_moments_say_so() {
        let view = table();
        for language in Language::ALL {
            let i18n = I18n::new(language);
            let reviews = [uncertain()];
            let focused = Debriefing::Ready(with_contexts(
                model::moments(&reviews, Some(Concept::RemainingTrumpf)),
                fixture_context,
            ));
            for (d, focus) in [(ready(&reviews), false), (focused, true)] {
                let d = debriefing(&i18n, &view, &d, true);
                assert_eq!(d.moments.len(), 1);
                let summary = &d.moments[0].summary;
                // Weak on the finished samples only: no rating at all.
                let provisional = i18n.tf(
                    "coach-moment-chosen-provisional",
                    &[("action", text::card(&i18n, TRAP).into())],
                );
                assert_eq!(summary[0].text, provisional);
                assert_eq!(
                    d.moments[0]
                        .title
                        .text
                        .ends_with(&i18n.t("coach-moment-focus")),
                    focus
                );
                assert_no_estimates(&joined(summary));
            }
        }
    }

    #[test]
    fn good_and_equivalent_moments_name_the_trap_without_claiming_it_was_considered() {
        let view = table();
        for language in Language::ALL {
            let i18n = I18n::new(language);
            for (review, played, id) in [
                (good(), BEST, "coach-moment-chosen-best"),
                (equivalent(), MIDDLE, "coach-moment-chosen-equivalent"),
            ] {
                let d = debriefing(&i18n, &view, &ready(&[review]), false);
                assert_eq!(d.moments.len(), 1, "{id}");
                let m = &d.moments[0];
                let name = |c| text::card(&i18n, c);
                let chosen = i18n.tf(
                    id,
                    &[("action", name(played).into()), ("best", name(BEST).into())],
                );
                let trap = i18n.tf("coach-moment-trap", &[("trap", name(TRAP).into())]);
                let texts: Vec<&str> = m.summary.iter().map(|l| l.text.as_str()).collect();
                assert_eq!(texts[0..2], [chosen.as_str(), trap.as_str()]);
                assert_eq!(
                    texts.last().copied(),
                    Some(i18n.t("coach-moment-no-reason").as_str())
                );
            }
        }
    }

    #[test]
    fn many_moments_keep_three_summaries_without_estimates() {
        let view = table();
        // One concept each, so none is dropped as a repetition.
        let all = [
            teaching(good(), Concept::HandStructure),
            teaching(equivalent(), Concept::GameValue),
            teaching(worse(), Concept::ProvenVoid),
            teaching(weak(), Concept::RemainingTrumpf),
            teaching(uncertain(), Concept::TrumpfVulnerability),
            forced(),
        ];
        for language in Language::ALL {
            let i18n = I18n::new(language);
            for figures in [false, true] {
                let d = debriefing(&i18n, &view, &ready(&all), figures);
                // Rated clearly first: good, equivalent (both avoid the
                // 14.3 trap) and weak (14.3) before worse (6.0) and the
                // provisional one; in game order.
                assert_eq!(d.moments.len(), 3);
                let chosen: Vec<String> = d
                    .moments
                    .iter()
                    .map(|m| m.summary[0].text.clone())
                    .collect();
                let name = |c| text::card(&i18n, c);
                assert!(chosen[0].contains(&name(BEST)), "{chosen:?}");
                assert!(chosen[1].contains(&name(MIDDLE)), "{chosen:?}");
                assert!(chosen[2].contains(&name(TRAP)), "{chosen:?}");
                for m in &d.moments {
                    // Never an estimate in the summary, figures or not.
                    assert_no_estimates(&joined(&m.summary));
                    let details = joined(&m.details);
                    // Party and metric labels, ratings and assumptions.
                    assert!(details.contains(&i18n.t("coach-assumption-uniform")));
                    assert!(details.contains(&i18n.t("coach-assumption-double-dummy")));
                    assert!(details.contains(&i18n.t("coach-section-facts")));
                    assert!(
                        details.contains(&i18n.tf("coach-fact-trumps-out", &[("count", 4.into())]))
                    );
                    assert!(details.contains(&i18n.tf(
                        "coach-details-rating",
                        &[
                            ("action", name(BEST).into()),
                            ("judgement", judgement(&i18n, Judgement::Best).into()),
                        ],
                    )));
                    assert!(details.contains(&i18n.t("coach-metric-label-points-defender")));
                    if figures {
                        assert!(details.contains("62 %") && details.contains("+21.4"));
                        assert!(details.contains(&i18n.t("coach-party-defenders")));
                    } else {
                        assert_no_estimates(&details);
                    }
                }
                let weak = joined(&d.moments[2].details);
                assert!(weak.contains(&i18n.tf(
                    "coach-metric-label-win",
                    &[("party", i18n.t("coach-party-defenders").into())],
                )));
            }
        }
    }

    fn texts(lines: &[Line]) -> Vec<&str> {
        lines.iter().map(|l| l.text.as_str()).collect()
    }

    /// A Herz game seen by a defender after a Trumpf trick: hand-made
    /// sets (11 Trumpf: own 2, played 3, unseen 6, unknown Skat).
    fn herz_sets() -> TrumpfSets {
        let h = |r| Card::new(Suit::Herz, r);
        let own: CardSet = [Card::bube(Suit::Pik), h(Rank::Ass)].into_iter().collect();
        let played: CardSet = [Card::bube(Suit::Kreuz), h(Rank::Sieben), h(Rank::Acht)]
            .into_iter()
            .collect();
        let total = GameType::Farbe(Suit::Herz).trumps();
        TrumpfSets {
            total,
            own,
            played,
            own_skat: CardSet::EMPTY,
            revealed: CardSet::EMPTY,
            unseen: total - own - played,
            skat_unknown: true,
        }
    }

    #[test]
    fn remaining_trumpf_lesson_cites_the_cards_and_assigns_no_seat() {
        let view = table();
        let lesson = LessonContent::RemainingTrumpf {
            game: GameType::Farbe(Suit::Herz),
            sets: herz_sets(),
        };
        let de = I18n::new(Language::De);
        assert_eq!(
            texts(&super::lesson(&de, &view, &lesson)),
            [
                "Was sicher feststeht",
                "Im Herz-Spiel gibt es 11 Trumpf: die 4 Buben und die 7 Herz-Karten.",
                "In deiner Hand: 2 (Pik-Bube, Herz-Ass).",
                "Schon gespielt: 3 (Kreuz-Bube, Herz-Acht, Herz-Sieben).",
                "Nicht gesehen: 6 (Herz-Bube, Karo-Bube, Herz-Zehn, Herz-König, Herz-Dame, Herz-Neun), in einer verdeckten Hand oder im Skat.",
                "Achtung: Wer die nicht gesehenen Trumpf hat, steht damit nicht fest. Sie können auf die verdeckten Hände verteilt sein oder im Skat liegen.",
                "Beim nächsten Trumpfstich: Wie viele Trumpf sind danach noch nicht gesehen?",
            ]
        );
        // Known Skat with a discarded Trumpf, nothing unseen (exhausted).
        let mut sets = herz_sets();
        sets.own_skat = sets.unseen;
        sets.unseen = CardSet::EMPTY;
        sets.skat_unknown = false;
        let exhausted = LessonContent::RemainingTrumpf {
            game: GameType::Grand,
            sets,
        };
        let en = I18n::new(Language::En);
        let lines = super::lesson(&en, &view, &exhausted);
        let text = texts(&lines);
        assert_eq!(text[1], "In Grand only the 11 Jacks are trumps.");
        assert!(
            text[4].starts_with("In the Skat you discarded: 6 ("),
            "{text:?}"
        );
        assert_eq!(text[5], "Not seen: none. No trump is concealed any more.");
        assert_eq!(
            text[6],
            "Note: this only tells you about trumps, not about the side suits."
        );
        // Before the move only the question, no counts.
        let question = texts(&lesson_question(&de, &lesson)).join(" ");
        assert!(!question.chars().any(|c| c.is_ascii_digit()), "{question}");
    }

    #[test]
    fn proven_void_lesson_cites_the_trick_and_teaches_an_exclusion() {
        let view = table();
        let seat = Seat::Mittelhand;
        let void = model::ProvenVoid {
            seat,
            follow: Follow::Farbe(Suit::Pik),
            trick: 3,
            led: Card::new(Suit::Pik, Rank::Ass),
            card: Card::new(Suit::Karo, Rank::Sieben),
        };
        for (language, expected) in [
            (
                Language::De,
                [
                    "Stich 3: {p} hat Karo-Sieben auf Pik-Ass gespielt, also nicht Pik bedient. Wer bedienen kann, muss bedienen (ISkO) – {p} hat kein Pik mehr.",
                    "Das schließt nur Pik aus. Wer frei ist, darf stechen oder abwerfen – ob er das tut und wer den Stich gewinnt, folgt daraus nicht.",
                    "Wenn wieder Pik gespielt wird: Wer kann dann noch bedienen?",
                ],
            ),
            (
                Language::En,
                [
                    "Trick 3: {p} played Seven of Diamonds to Ace of Spades and so did not follow Spades. A player who can follow suit must (ISkO) – {p} has no Spades left.",
                    "This only rules out Spades. A player who is void may trump or discard – whether they do and who wins the trick does not follow from it.",
                    "When Spades is led again: who can still follow?",
                ],
            ),
        ] {
            let i18n = I18n::new(language);
            let p = text::player(&i18n, seat_player(&view, seat));
            let lesson = LessonContent::ProvenVoid {
                game: GameType::Farbe(Suit::Herz),
                voids: vec![void],
            };
            let lines = super::lesson(&i18n, &view, &lesson);
            let expected: Vec<String> = expected.iter().map(|e| e.replace("{p}", &p)).collect();
            assert_eq!(texts(&lines)[1..], expected);
        }
        // Trumpf void and Null: other exceptions.
        let de = I18n::new(Language::De);
        let trumpf = LessonContent::ProvenVoid {
            game: GameType::Grand,
            voids: vec![model::ProvenVoid {
                follow: Follow::Trumpf,
                led: Card::bube(Suit::Kreuz),
                ..void
            }],
        };
        let lines = super::lesson(&de, &view, &trumpf);
        assert!(
            lines[1].text.ends_with("keinen Trumpf mehr."),
            "{:?}",
            lines[1]
        );
        assert_eq!(
            lines[2].text,
            "Das schließt nur Trumpf aus. Wer einen späteren Stich gewinnt, folgt daraus nicht."
        );
        let null = LessonContent::ProvenVoid {
            game: GameType::Null,
            voids: vec![void],
        };
        assert!(
            super::lesson(&de, &view, &null)[2]
                .text
                .starts_with("Im Null gibt es keinen Trumpf")
        );
    }

    #[test]
    fn trumpf_vulnerability_states_the_condition_and_certainty() {
        let view = table();
        let seat = Seat::Hinterhand;
        let threat = model::Abstich {
            seat,
            card: Card::new(Suit::Pik, Rank::Ass),
            certainty: Certainty::Possible,
            suit_unseen: CardSet::single(Card::new(Suit::Pik, Rank::Zehn))
                | Card::new(Suit::Pik, Rank::Neun).into(),
            trumpf_unseen: CardSet::single(Card::bube(Suit::Karo)),
        };
        let lesson = LessonContent::TrumpfVulnerability {
            game: GameType::Farbe(Suit::Kreuz),
            threats: vec![threat],
        };
        let de = I18n::new(Language::De);
        let p = text::player(&de, seat_player(&view, seat));
        let expected: Vec<String> = [
            "Was in diesem Stich möglich war",
            "{p} spielt in diesem Stich nach deiner Karte (Pik-Ass). {p} kann nur stechen, wenn er kein Pik mehr und mindestens einen Trumpf hat. Beides ist möglich, nicht sicher: 2 Pik und 1 Trumpf hast du nicht gesehen.",
            "Regel (ISkO): Wer bedienen kann, muss bedienen. Nur wer kein Pik hat, darf stechen – oder stattdessen abwerfen.",
            "Achtung: Stechen können heißt nicht stechen werden. Ein möglicher Abstich allein zeigt nicht, dass vorher Trumpfziehen besser gewesen wäre; die Bewertung deines Zugs oben ist ein eigenes Modellurteil.",
            "Bevor du ein Ass oder eine Zehn einer Fehlfarbe spielst: Wer spielt nach dir, kann er in der Farbe frei sein – und hat er noch Trumpf?",
        ]
        .iter()
        .map(|e| e.replace("{p}", &p))
        .collect();
        assert_eq!(texts(&super::lesson(&de, &view, &lesson)), expected);
        // Certain, in English.
        let en = I18n::new(Language::En);
        let p = text::player(&en, seat_player(&view, seat));
        let certain = LessonContent::TrumpfVulnerability {
            game: GameType::Farbe(Suit::Kreuz),
            threats: vec![model::Abstich {
                certainty: Certainty::Certain,
                ..threat
            }],
        };
        assert_eq!(
            texts(&super::lesson(&en, &view, &certain))[1],
            format!(
                "{p} plays after your Ace of Spades in this trick. It is certain that {p} holds no Spades and at least one trump (this follows from the cards you have seen and the proven voids), so {p} can trump it."
            )
        );
        // Mechanisms have no question before the move.
        assert!(lesson_question(&de, &lesson).is_empty());
    }

    #[test]
    fn partner_overtake_names_the_ways_and_certainty() {
        let view = table();
        let overtake = model::Overtake {
            partner: Seat::Hinterhand,
            declarer: Seat::Vorhand,
            best: Card::new(Suit::Pik, Rank::Koenig),
            led: Follow::Farbe(Suit::Pik),
            card: Card::new(Suit::Karo, Rank::Sieben),
            certainty: Certainty::Certain,
            higher: CardSet::single(Card::new(Suit::Pik, Rank::Ass))
                | Card::new(Suit::Pik, Rank::Zehn).into(),
            trumping: false,
        };
        let lesson = LessonContent::PartnerOvertake {
            game: GameType::Farbe(Suit::Kreuz),
            overtake,
        };
        let de = I18n::new(Language::De);
        let p = text::player(&de, seat_player(&view, Seat::Hinterhand));
        let d = text::player(&de, seat_player(&view, Seat::Vorhand));
        let expected: Vec<String> = [
            "Was in diesem Stich möglich war",
            "{d} hat als Alleinspieler Pik-König ausgespielt; nach deiner Karte (Karo-Sieben) nimmt sie den Stich weiterhin. {p} spielt als Letzter.",
            "{p} kann sie mit einer höheren Karte (Pik-Ass, Pik-Zehn) schlagen. Das ist sicher: Wie die verdeckten Karten auch liegen, {p} hat eine solche Karte.",
            "Regel (ISkO): {p} muss Pik bedienen, wenn er kann; dann gewinnt nur eine höhere Pik-Karte. Ohne Pik gewinnt jeder Trumpf.",
            "Achtung: Schlagen können heißt nicht schlagen werden. Über spätere Stiche und darüber, welche Karte du hättest spielen sollen, sagt das nichts.",
            "Wenn der Alleinspieler ausspielt und dein Partner hinter dir sitzt: Mit welchen Karten kann er die Karte des Alleinspielers noch schlagen?",
        ]
        .iter()
        .map(|e| e.replace("{p}", &p).replace("{d}", &d))
        .collect();
        assert_eq!(texts(&super::lesson(&de, &view, &lesson)), expected);
        // Possible, by trumping only, in English; and a Trumpf lead.
        let en = I18n::new(Language::En);
        let p = text::player(&en, seat_player(&view, Seat::Hinterhand));
        let trumping = LessonContent::PartnerOvertake {
            game: GameType::Grand,
            overtake: model::Overtake {
                certainty: Certainty::Possible,
                higher: CardSet::EMPTY,
                trumping: true,
                ..overtake
            },
        };
        assert_eq!(
            texts(&super::lesson(&en, &view, &trumping))[2],
            format!(
                "{p} can beat it only when holding no Spades, with a trump. That is possible, not certain: it depends on cards you have not seen."
            )
        );
        let trumpf = LessonContent::PartnerOvertake {
            game: GameType::Farbe(Suit::Herz),
            overtake: model::Overtake {
                best: Card::new(Suit::Herz, Rank::Neun),
                led: Follow::Trumpf,
                higher: CardSet::single(Card::bube(Suit::Kreuz)),
                ..overtake
            },
        };
        assert_eq!(
            texts(&super::lesson(&en, &view, &trumpf))[3],
            format!(
                "Rule (ISkO): {p} must follow with a trump if possible; only a higher trump wins."
            )
        );
    }

    /// MH's opening call in the `skat-analysis` fixture deal: CJ HJ HA HT
    /// HK H9 H7 SA S7 D7, Skat unknown (hand-made from its comments).
    fn opening_structures() -> Vec<Structure> {
        let set = |cards: &[Card]| cards.iter().copied().collect::<CardSet>();
        let bube = |s| Card::new(s, Rank::Bube);
        let buben = set(&[bube(Suit::Kreuz), bube(Suit::Herz)]);
        let herz: Vec<Card> = [
            Rank::Ass,
            Rank::Zehn,
            Rank::Koenig,
            Rank::Neun,
            Rank::Sieben,
        ]
        .into_iter()
        .map(|r| Card::new(Suit::Herz, r))
        .collect();
        let s = |game, trumpf: CardSet, high, trumpf_most| Structure {
            game,
            skat: SkatState::Unknown,
            buben,
            trumpf,
            skat_trumpf: CardSet::EMPTY,
            spitzen: Spitzen {
                mit: true,
                count: 1,
            },
            counts: Range { low: 1, high },
            trumpf_most,
        };
        vec![
            s(GameType::Farbe(Suit::Herz), buben | set(&herz), 7, 9),
            s(GameType::Grand, buben, 4, 4),
        ]
    }

    #[test]
    fn hand_structure_lesson_counts_every_candidate_and_names_no_choice() {
        let view = table();
        let lesson = LessonContent::HandStructure {
            games: opening_structures(),
        };
        let de = I18n::new(Language::De);
        assert_eq!(
            texts(&super::lesson(&de, &view, &lesson)),
            [
                "Was sicher feststeht",
                "Herz: 7 Trumpf (Kreuz-Bube, Herz-Bube, Herz-Ass, Herz-Zehn, Herz-König, Herz-Neun, Herz-Sieben), mit 1.",
                "Mit dem unbekannten Skat: 1 bis 7 Spitzen möglich (mit oder ohne), bis zu 9 Trumpf.",
                "Grand: 2 Buben (Kreuz-Bube, Herz-Bube), mit 1.",
                "Mit dem unbekannten Skat: 1 bis 4 Spitzen möglich (mit oder ohne), bis zu 4 Buben.",
                "Der Skat ist unbekannt. Seine zwei Karten zählen für die Spitzen des Alleinspielers (auch im Handspiel), die Spitzen können sich also noch ändern.",
                "Achtung: Diese Zählung beschreibt deine Karten. Ob ein Spiel gewonnen wird, welches Spiel du spielen, wie hoch du reizen oder was du drücken solltest, folgt daraus nicht.",
                "Die Bewertung deines Zugs oben ist eine grobe Modellschätzung: Vor dem Kartenspiel simuliert das Modell nur wenige Verteilungen, und darin spielen alle so, als sähen sie alle Karten.",
                "Beim nächsten Blatt: Welche Buben und wie viele Trumpf hast du für jede Farbe und für Grand – und mit oder ohne wie viele?",
            ]
        );
        // Held after pickup, Spitzen fixed, a Trumpf in the own Skat.
        let mut games = opening_structures();
        for g in &mut games {
            g.skat = SkatState::Discarded;
            g.counts.high = 1;
            g.skat_trumpf = CardSet::single(Card::new(Suit::Karo, Rank::Bube));
        }
        let en = I18n::new(Language::En);
        let text = super::lesson(&en, &view, &LessonContent::HandStructure { games });
        let text = texts(&text);
        assert_eq!(
            text[2],
            "Of these, in the Skat you discarded: 1 (Jack of Diamonds). They count for the matadors but cannot be played."
        );
        assert!(!text.iter().any(|t| t.contains("unknown Skat")), "{text:?}");
        // Before the move only the question: no counts, no option.
        let question = texts(&lesson_question(&de, &lesson)).join(" ");
        assert!(!question.chars().any(|c| c.is_ascii_digit()), "{question}");
    }

    #[test]
    fn game_value_lesson_states_the_rule_and_every_contract() {
        let view = table();
        let herz = GameType::Farbe(Suit::Herz);
        let entry = |contract, low, high, reach| ContractValue {
            contract,
            range: Range { low, high },
            reach,
        };
        let plain = |g| Contract::new(skat_app::Announcement::plain(g), false);
        let hand = |g| Contract::new(skat_app::Announcement::plain(g), true);
        let null_ouvert = |h| {
            Contract::new(
                skat_app::Announcement {
                    ouvert: true,
                    ..skat_app::Announcement::plain(GameType::Null)
                },
                h,
            )
        };
        // MH declarer at 48 deciding Hand or pickup (fixture comments).
        let lesson = |kontra_re| LessonContent::GameValue {
            value: 48,
            bidding: false,
            kontra_re,
            skat: SkatState::Unknown,
            games: vec![
                (
                    herz,
                    vec![
                        entry(plain(herz), 20, 80, Reach::Skat),
                        entry(hand(herz), 30, 90, Reach::Skat),
                    ],
                ),
                (
                    GameType::Grand,
                    vec![entry(plain(GameType::Grand), 48, 120, Reach::Reached)],
                ),
                (
                    GameType::Null,
                    vec![
                        entry(plain(GameType::Null), 23, 23, Reach::Never),
                        entry(null_ouvert(true), 59, 59, Reach::Reached),
                    ],
                ),
            ],
        };
        let de = I18n::new(Language::De);
        let text = super::lesson(&de, &view, &lesson(false));
        assert_eq!(
            texts(&text)[..5],
            [
                "Was sicher feststeht",
                "Gereizt ist 48. Ein Spiel, das weniger wert ist, ist überreizt und verloren (ISkO), egal wie viele Augen du bekommst.",
                "Herz: mit Skataufnahme 20–80 (nur mit passendem Skat) · Hand 30–90 (nur mit passendem Skat)",
                "Grand: mit Skataufnahme 48–120 (reicht)",
                "Null: mit Skataufnahme 23 (reicht nie) · Hand Ouvert 59 (reicht)",
            ]
        );
        let en = I18n::new(Language::En);
        let off = super::lesson(&en, &view, &lesson(false));
        let on = super::lesson(&en, &view, &lesson(true));
        assert_eq!(on.len(), off.len() + 1, "one Kontra/Re line");
        let on = texts(&on);
        assert!(on.contains(
            &"Kontra and Re double the points written down, not the game value: they do not change whether the bid is reached."
        ));
        assert!(
            on.iter()
                .any(|t| t.starts_with("The rating of your move above is a coarse model estimate"))
        );
        assert!(on.iter().any(|t| {
            t.starts_with("Note: reaching the bid only rules out losing by overbidding.")
        }));
        // No line names an action, and the question has no figures.
        for a in [Action::Pass, Action::Bid(48), Action::Hold] {
            let name = action_at(&en, 48, None, a);
            assert!(!on.iter().any(|t| t.contains(&name)), "{name}");
        }
        let question = texts(&lesson_question(&en, &lesson(true))).join(" ");
        assert!(!question.chars().any(|c| c.is_ascii_digit()), "{question}");
    }
}
