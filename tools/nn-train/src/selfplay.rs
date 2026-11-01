// SPDX-License-Identifier: EUPL-1.2

//! Raw decision records from teacher self-play (#70).
//!
//! Every decision of every seat in a seeded game of three teacher
//! [`Opponent`]s (by default Strong) becomes one [`DecisionRecord`]: the
//! acting seat's [`PlayerView`] (which carries the phase and the legal
//! actions), the chosen action, the teacher's Monte-Carlo option values
//! with their status, and the seat's final Seeger-Fabian points. Records
//! hold raw data, not tensors: the encoder runs at training time, so data
//! survive encoder changes.
//!
//! Seeds: deal and teacher seeds come from separate keys mixed with the
//! run seed and the deal index; teacher seeds are never derived from a deal
//! seed. Seeds and the deal index are bookkeeping, not model inputs; the
//! only model input is [`DecisionRecord::view`].

use serde::{Deserialize, Serialize};
use skat_ai::{Decision, Level, Opponent, OptionValue, Profile, Status};
use skat_core::{Action, Game, Phase, PlayerView, RuleConfig, Seat};

/// Version of the record format; bumped on every incompatible change.
pub const RECORD_FORMAT: u32 = 1;

/// Rules of the generated games: ISkO with Kontra/Re (D-03, D-26).
pub const RULES: RuleConfig = RuleConfig { kontra_re: true };

/// Which part of the data a deal belongs to. All records of one deal (all
/// seats, all variants) share it, so no deal leaks between the parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Split {
    /// Training (80 % of the deals).
    Train,
    /// Validation (10 %).
    Validation,
    /// Test (10 %).
    Test,
}

impl Split {
    /// The split of the deal dealt from `deal_seed`. It depends on the deal
    /// alone, never on seats, teachers or the record.
    pub fn of(deal_seed: u64) -> Split {
        match mix(deal_seed, SPLIT_KEY) % 100 {
            0..80 => Split::Train,
            80..90 => Split::Validation,
            _ => Split::Test,
        }
    }
}

/// What the teacher computed for a decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Teacher {
    /// Status of the teacher's analysis: `Complete`, or incomplete
    /// (`BudgetExhausted`, `Cancelled`); `None` if the rules of thumb
    /// decided without analysis (Strong: bidding and declaring).
    pub status: Option<Status>,
    /// `true` if the analysis gave no usable estimate and the rules of
    /// thumb decided instead.
    pub fallback: bool,
    /// Monte-Carlo option values, one per legal action in the view's order;
    /// empty without analysis.
    pub options: Vec<OptionValue>,
}

/// One decision of one seat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Index of the deal in the run (grouping and bookkeeping only; not a
    /// model input).
    pub deal: u64,
    /// The deal's split ([`Split::of`]).
    pub split: Split,
    /// The acting seat's permitted view: the only model input. It holds the
    /// seat, the phase and the legal actions.
    pub view: PlayerView,
    /// The teacher's chosen action, one of `view.legal_actions`.
    pub action: Action,
    /// The teacher's option values.
    pub teacher: Teacher,
    /// The acting seat's Seeger-Fabian points at the end of the game (0 for
    /// a thrown-in deal). A future outcome: a training target, never an
    /// input.
    pub final_points: i32,
}

/// Kind of a decision, for statistics and coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DecisionKind {
    /// Bidding.
    Reizen,
    /// Hand or Skat pickup.
    HandOrPickup,
    /// Drücken.
    Druecken,
    /// Announcement.
    Ansage,
    /// A decision that offers Kontra or Re (in card play together with the
    /// cards).
    KontraRe,
    /// Card play without a Kontra option.
    Spiel,
}

impl DecisionKind {
    /// All kinds in game order.
    pub const ALL: [DecisionKind; 6] = [
        DecisionKind::Reizen,
        DecisionKind::HandOrPickup,
        DecisionKind::Druecken,
        DecisionKind::Ansage,
        DecisionKind::KontraRe,
        DecisionKind::Spiel,
    ];

    /// The kind of the decision in `view`.
    pub fn of(view: &PlayerView) -> DecisionKind {
        match view.phase {
            Phase::Reizen => DecisionKind::Reizen,
            Phase::SkatAufnahme => DecisionKind::HandOrPickup,
            Phase::Druecken => DecisionKind::Druecken,
            Phase::Ansage => DecisionKind::Ansage,
            _ if view.legal_actions.iter().any(|a| {
                matches!(
                    a,
                    Action::Kontra | Action::KeinKontra | Action::Re | Action::KeinRe
                )
            }) =>
            {
                DecisionKind::KontraRe
            }
            _ => DecisionKind::Spiel,
        }
    }
}

/// Seeds of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSeed(pub u64);

const DEAL_KEY: u64 = 0x6465_616c;
const TEACHER_KEY: u64 = 0x7465_6163;
const SPLIT_KEY: u64 = 0x7370_6c74;

impl RunSeed {
    /// Seed of the cards of deal `deal`.
    pub fn deal_seed(self, deal: u64) -> u64 {
        mix(mix(self.0, DEAL_KEY), deal)
    }

    /// Seed of the teacher on `seat` in deal `deal`, from its own key; it
    /// does not depend on the deal seed.
    pub fn teacher_seed(self, deal: u64, seat: Seat) -> u64 {
        mix(mix(mix(self.0, TEACHER_KEY), deal), seat.index() as u64)
    }
}

/// The teacher profile of a run, with a stable name for the manifest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TeacherProfile {
    /// Name in the manifest, e.g. `strong`.
    pub name: &'static str,
    /// The profile.
    pub profile: Profile,
}

impl TeacherProfile {
    /// Strong (`Budget::INTERACTIVE`), the default teacher.
    pub const STRONG: TeacherProfile = TeacherProfile {
        name: "strong",
        profile: Level::Strong.profile(),
    };
}

/// The teacher's decision and option values for `view`; `None` if it is not
/// this seat's turn.
pub fn teach(view: &PlayerView, teacher: &Opponent) -> Option<(Action, Teacher)> {
    let (
        Decision {
            action,
            fallback,
            status,
        },
        analysis,
    ) = teacher.decide_analysed(view, None)?;
    let options = analysis.map(|a| a.options).unwrap_or_default();
    Some((
        action,
        Teacher {
            status,
            fallback,
            options,
        },
    ))
}

/// Plays `game` from its current state to the end with `teachers` (indexed
/// by seat) and records every decision. `deal` and `split` are copied into
/// the records. Returns the records in decision order and the finished game.
///
/// # Panics
///
/// If a teacher chooses no action or an illegal one.
pub fn play(
    mut game: Game,
    teachers: &[Opponent; 3],
    deal: u64,
    split: Split,
) -> (Vec<DecisionRecord>, Game) {
    let mut records = Vec::new();
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        let (action, teacher) = teach(&view, &teachers[seat.index()]).expect("seat to move");
        game.apply(action).expect("teachers choose legal actions");
        records.push(DecisionRecord {
            deal,
            split,
            view,
            action,
            teacher,
            final_points: 0,
        });
    }
    if let Some(result) = game.result() {
        let points = result.seeger_fabian();
        for r in &mut records {
            r.final_points = points[r.view.seat.index()];
        }
    }
    (records, game)
}

/// Plays deal `deal` of run `seed` with three teachers of `profile` and
/// returns its records.
pub fn play_deal(seed: RunSeed, profile: Profile, deal: u64) -> Vec<DecisionRecord> {
    let deal_seed = seed.deal_seed(deal);
    let teachers = Seat::ALL.map(|s| Opponent {
        profile,
        seed: seed.teacher_seed(deal, s),
    });
    play(
        Game::new(deal_seed, RULES),
        &teachers,
        deal,
        Split::of(deal_seed),
    )
    .0
}

/// Checks that a record is usable for training: the action is legal and
/// has an index in the action space, the view encodes, it carries no
/// outcome, and the teacher values (if any) cover the legal actions.
pub fn check_record(r: &DecisionRecord) -> Result<(), String> {
    if !r.view.legal_actions.contains(&r.action) {
        return Err(format!("illegal action {:?}", r.action));
    }
    if r.view
        .legal_actions
        .iter()
        .any(|&a| skat_nn::action_index(a).is_none())
    {
        return Err("an action without index".into());
    }
    if skat_nn::encode(&r.view).len() != skat_nn::FEATURES {
        return Err("encoding has the wrong length".into());
    }
    if r.view.result.is_some() {
        return Err("the view carries the result".into());
    }
    let actions: Vec<Action> = r.teacher.options.iter().map(|o| o.action).collect();
    match r.teacher.status {
        Some(_) if actions != r.view.legal_actions => {
            Err("options differ from legal actions".into())
        }
        None if !actions.is_empty() => Err("options without analysis".into()),
        _ => Ok(()),
    }
}

/// Mixes `value` into `hash` (SplitMix64 finaliser).
fn mix(hash: u64, value: u64) -> u64 {
    let mut z = hash ^ value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
