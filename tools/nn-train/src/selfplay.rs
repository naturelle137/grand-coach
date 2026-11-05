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
//! With a mixed [`Table`] (`easy-mix`, `rookie-mix`, `medium-mix`,
//! `level-mix`) the teacher still bids and declares at every seat, but card
//! play (with Kontra/Re) seats 0 to 3 weaker players, drawn as [`Roles`]
//! once the declarer is known. At such a seat the teacher still analyses
//! and labels the decision; the weaker player's action only drives the game
//! ([`DecisionRecord::played`]). Weak players thus widen the positions
//! without ever becoming a training target.
//!
//! Seeds: deal and teacher seeds come from separate keys mixed with the
//! run seed and the deal index; teacher seeds are never derived from a deal
//! seed. Seeds and the deal index are bookkeeping, not model inputs; the
//! only model input is [`DecisionRecord::view`].

use serde::{Deserialize, Serialize};
use skat_ai::{Decision, Level, Opponent, OptionValue, Profile, Status};
use skat_core::{Action, Game, Phase, PlayerView, RuleConfig, Seat};

/// Version of the record format; bumped on every incompatible change.
/// Format 2 adds the optional [`DecisionRecord::played`] and
/// [`DecisionRecord::lineup`]; format-1 records read as teacher-only.
pub const RECORD_FORMAT: u32 = 2;

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
    /// The teacher's chosen action, one of `view.legal_actions`: the
    /// policy label, also at an Easy seat.
    pub action: Action,
    /// The action applied to the game when another player than the teacher
    /// held the seat (an Easy seat of [`Table::EasyMix`]); `None`: the
    /// teacher's `action`. Bookkeeping for the replay, never a target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub played: Option<Action>,
    /// Card-play lineup, indexed by seat, in a [`Table::EasyMix`] game from
    /// the start of card play on; `None` before it and in teacher-only
    /// games. Bookkeeping (grouping, replay check), never a model input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineup: Option<[Player; 3]>,
    /// The teacher's option values.
    pub teacher: Teacher,
    /// The acting seat's Seeger-Fabian points at the end of the game (0 for
    /// a thrown-in deal). A future outcome: a training target, never an
    /// input.
    pub final_points: i32,
}

impl DecisionRecord {
    /// The action that moved the game on: [`played`](Self::played) if set,
    /// else the teacher's [`action`](Self::action).
    pub fn applied(&self) -> Action {
        self.played.unwrap_or(self.action)
    }

    /// Who played the acting seat.
    pub fn player(&self) -> Player {
        self.lineup
            .map_or(Player::Teacher, |l| l[self.view.seat.index()])
    }

    /// Card-play roles of the record's game; teacher-only without a
    /// lineup or declarer.
    pub fn roles(&self) -> Roles {
        match (self.lineup, self.view.declarer) {
            (Some(lineup), Some(declarer)) => Roles::of(lineup, declarer),
            _ => Roles::TEACHERS,
        }
    }
}

/// Makes the opponent that plays a non-teacher seat.
pub type MakePlayer<'a> = &'a dyn Fn(Player, Seat) -> Opponent;

/// Who holds a seat in card play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Player {
    /// The run's teacher profile (Strong).
    Teacher,
    /// Classic Rookie: rules of thumb with seeded mistakes.
    Rookie,
    /// Classic Easy: rules of thumb without search.
    Easy,
    /// Classic Medium: analysed card play with a small budget.
    Medium,
}

impl Player {
    /// All players, the teacher first.
    pub const ALL: [Player; 4] = [
        Player::Teacher,
        Player::Rookie,
        Player::Easy,
        Player::Medium,
    ];

    /// The classic level of a non-teacher player; `None` for the teacher.
    pub fn level(self) -> Option<Level> {
        match self {
            Player::Teacher => None,
            Player::Rookie => Some(Level::Rookie),
            Player::Easy => Some(Level::Easy),
            Player::Medium => Some(Level::Medium),
        }
    }

    /// Letter in [`Roles::label`].
    fn letter(self) -> char {
        match self {
            Player::Teacher => 'T',
            Player::Rookie => 'R',
            Player::Easy => 'E',
            Player::Medium => 'M',
        }
    }
}

/// Card-play players by role: the declarer and the two defenders, the
/// defender after the declarer first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Roles {
    /// The declarer.
    pub declarer: Player,
    /// The defenders: the seat after the declarer, then the seat after
    /// that.
    pub defenders: [Player; 2],
}

impl Roles {
    /// Teachers in every role.
    pub const TEACHERS: Roles = Roles {
        declarer: Player::Teacher,
        defenders: [Player::Teacher; 2],
    };

    /// The lineup where bit 0 of `bits` makes the declarer `weak`, bit 1
    /// the defender after the declarer and bit 2 the other defender; every
    /// other role is the teacher. `bits` 0 to 7 give all eight lineups.
    pub const fn from_bits(bits: u8, weak: Player) -> Roles {
        const fn pick(on: bool, weak: Player) -> Player {
            if on { weak } else { Player::Teacher }
        }
        Roles {
            declarer: pick(bits & 1 != 0, weak),
            defenders: [pick(bits & 2 != 0, weak), pick(bits & 4 != 0, weak)],
        }
    }

    /// Number of players other than the teacher.
    pub fn weak(self) -> usize {
        [self.declarer, self.defenders[0], self.defenders[1]]
            .into_iter()
            .filter(|&p| p != Player::Teacher)
            .count()
    }

    /// The lineup by seat with `declarer` as declarer.
    pub fn lineup(self, declarer: Seat) -> [Player; 3] {
        let mut lineup = [Player::Teacher; 3];
        lineup[declarer.index()] = self.declarer;
        lineup[declarer.next().index()] = self.defenders[0];
        lineup[declarer.next().next().index()] = self.defenders[1];
        lineup
    }

    /// The roles of a seat lineup with `declarer` as declarer.
    pub fn of(lineup: [Player; 3], declarer: Seat) -> Roles {
        Roles {
            declarer: lineup[declarer.index()],
            defenders: [
                lineup[declarer.next().index()],
                lineup[declarer.next().next().index()],
            ],
        }
    }

    /// Short label, declarer first: `T|TE` is a teacher declarer against a
    /// teacher and an Easy defender (T teacher, R Rookie, E Easy, M
    /// Medium).
    pub fn label(self) -> String {
        format!(
            "{}|{}{}",
            self.declarer.letter(),
            self.defenders[0].letter(),
            self.defenders[1].letter()
        )
    }
}

/// Who plays the cards in a run's games.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Table {
    /// The teacher at every seat.
    #[default]
    Teacher,
    /// The teacher bids and declares; card play seats 0 to 3 Easy players
    /// by [`EASY_MIX_WEIGHTS`].
    EasyMix,
    /// Like `EasyMix` with Rookie players; each of the eight lineups of
    /// teacher and Rookie draws equally often.
    RookieMix,
    /// Like `RookieMix` with Medium players.
    MediumMix,
    /// Every card-play role is Easy, Medium or the teacher, each with
    /// probability 1/3, independently (27 lineups).
    LevelMix,
}

/// Weights of the role lineups of [`Table::EasyMix`], by the `bits` of
/// [`Roles::from_bits`]. Every lineup with exactly one Easy player weighs 2,
/// the rest 1: one weak player with two teachers is the trainer's table
/// (one learner, two AI seats), whether the learner declares (`E|TT`) or
/// defends beside a strong partner (`T|ET`, `T|TE`). The declarer's seat
/// follows from the deal, so all 24 seat-and-role lineups occur.
pub const EASY_MIX_WEIGHTS: [u32; 8] = [1, 2, 2, 1, 2, 1, 1, 1];

impl Table {
    /// All tables.
    pub const ALL: [Table; 5] = [
        Table::Teacher,
        Table::EasyMix,
        Table::RookieMix,
        Table::MediumMix,
        Table::LevelMix,
    ];

    /// Name in the manifest and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Table::Teacher => "teacher",
            Table::EasyMix => "easy-mix",
            Table::RookieMix => "rookie-mix",
            Table::MediumMix => "medium-mix",
            Table::LevelMix => "level-mix",
        }
    }

    /// The table named `name`.
    pub fn parse(name: &str) -> Option<Table> {
        Table::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The card-play roles for the draw `draw` (uniform `u64`); `None` for
    /// a teacher-only table.
    pub fn roles(self, draw: u64) -> Option<Roles> {
        match self {
            Table::Teacher => None,
            Table::EasyMix => {
                let total: u32 = EASY_MIX_WEIGHTS.iter().sum();
                let mut pick = (draw % u64::from(total)) as u32;
                for (bits, &w) in EASY_MIX_WEIGHTS.iter().enumerate() {
                    if pick < w {
                        return Some(Roles::from_bits(bits as u8, Player::Easy));
                    }
                    pick -= w;
                }
                unreachable!("pick is below the total weight")
            }
            Table::RookieMix => Some(Roles::from_bits((draw % 8) as u8, Player::Rookie)),
            Table::MediumMix => Some(Roles::from_bits((draw % 8) as u8, Player::Medium)),
            Table::LevelMix => {
                let mut rest = draw % 27;
                let mut role = || {
                    let p = [Player::Easy, Player::Medium, Player::Teacher][(rest % 3) as usize];
                    rest /= 3;
                    p
                };
                let declarer = role();
                let first = role();
                let second = role();
                Some(Roles {
                    declarer,
                    defenders: [first, second],
                })
            }
        }
    }
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
const TABLE_KEY: u64 = 0x7461_626c;
const PLAYER_KEY: u64 = 0x706c_6179;

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

    /// Draw of the card-play roles of deal `deal` ([`Table::roles`]), from
    /// its own key; it does not depend on the cards.
    pub fn table_draw(self, deal: u64) -> u64 {
        mix(mix(self.0, TABLE_KEY), deal)
    }

    /// Seed of the non-teacher player on `seat` in deal `deal`.
    pub fn player_seed(self, deal: u64, seat: Seat) -> u64 {
        mix(mix(mix(self.0, PLAYER_KEY), deal), seat.index() as u64)
    }
}

/// The card-play lineup in `view`'s game when `roles` are drawn: `None`
/// before card play or without roles.
pub fn lineup(view: &PlayerView, roles: Option<Roles>) -> Option<[Player; 3]> {
    match (roles, view.phase, view.declarer) {
        (Some(roles), Phase::Spiel, Some(declarer)) => Some(roles.lineup(declarer)),
        _ => None,
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
/// by seat) and records every decision. In card play a non-teacher seat of
/// `roles` is played by the opponent `players` makes for it while its
/// teacher still labels the decision. `deal` and `split` are copied into the records.
/// Returns the records in decision order and the finished game.
///
/// # Panics
///
/// If a teacher or player chooses no action or an illegal one.
pub fn play(
    mut game: Game,
    teachers: &[Opponent; 3],
    seating: Option<(Roles, MakePlayer<'_>)>,
    deal: u64,
    split: Split,
) -> (Vec<DecisionRecord>, Game) {
    let mut records = Vec::new();
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        let (action, teacher) = teach(&view, &teachers[seat.index()]).expect("seat to move");
        let lineup = lineup(&view, seating.map(|(r, _)| r));
        let played = match (lineup, seating) {
            (Some(l), Some((_, players))) if l[seat.index()] != Player::Teacher => Some(
                players(l[seat.index()], seat)
                    .decide(&view, None)
                    .expect("seat to move")
                    .action,
            ),
            _ => None,
        };
        game.apply(played.unwrap_or(action))
            .expect("teachers and players choose legal actions");
        records.push(DecisionRecord {
            deal,
            split,
            view,
            action,
            played,
            lineup,
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

/// Plays deal `deal` of run `seed` at `table` with teachers of `profile`
/// and returns its records.
pub fn play_deal(seed: RunSeed, profile: Profile, table: Table, deal: u64) -> Vec<DecisionRecord> {
    let deal_seed = seed.deal_seed(deal);
    let teachers = Seat::ALL.map(|s| Opponent {
        profile,
        seed: seed.teacher_seed(deal, s),
    });
    let players = |player: Player, seat: Seat| Opponent {
        profile: player.level().expect("a non-teacher player").profile(),
        seed: seed.player_seed(deal, seat),
    };
    let seating = table
        .roles(seed.table_draw(deal))
        .map(|r| (r, &players as MakePlayer));
    play(
        Game::new(deal_seed, RULES),
        &teachers,
        seating,
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
    if let Some(played) = r.played
        && !r.view.legal_actions.contains(&played)
    {
        return Err(format!("illegal played action {played:?}"));
    }
    if r.played.is_some() != (r.player() != Player::Teacher) {
        return Err("played action and lineup disagree".into());
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
pub(crate) fn mix(hash: u64, value: u64) -> u64 {
    let mut z = hash ^ value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
