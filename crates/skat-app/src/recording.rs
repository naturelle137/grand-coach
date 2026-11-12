// SPDX-License-Identifier: EUPL-1.2

//! Explicit privileged local recordings. Never pass these to live analysis.

use serde::{Deserialize, Serialize};
use skat_core::{
    Action, Deal, Event, Game, GameResult, OmniscientView, Phase, RuleConfig, RuleError,
};

use crate::{Opponents, PlayerId, SeriesLength, SessionConfig};

/// Wire schema supported by this build.
pub const RECORDING_FORMAT: u16 = 1;
/// Replay semantics, including dealing and rules. Bump on incompatible changes.
pub const RECORDING_ENGINE: &str = "grand-coach-recording-1";
/// Conservative bound above a complete bidding ladder and thirty card plays.
pub const MAX_RECORDING_ACTIONS: usize = 512;

/// Original application context; descriptive, never used to rerun an AI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingContext {
    /// Package version of the recording producer.
    pub application_version: String,
    /// Original series configuration (its seed is not the game's seed).
    pub session: SessionConfig,
    /// One-based game number in that series.
    pub number: u32,
    /// Dealer in stable series player coordinates.
    pub dealer: PlayerId,
    /// Actual seeds for opponents 1 and 2 in this game.
    pub opponent_seeds: [u64; 2],
    /// Selected forgetful opponent, only at a Goldfish table.
    pub goldfish: Option<PlayerId>,
}

/// Expected status after the last recorded action; distinguishes a deliberate
/// live prefix from a truncated completed recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingEnd {
    /// The last action left a legal decision outstanding.
    InProgress,
    /// All three players passed.
    Eingepasst,
    /// Core settlement, checked against the reconstructed game.
    Finished(GameResult),
}

impl RecordingEnd {
    pub(crate) fn of(game: &Game) -> Self {
        match game.phase() {
            Phase::Eingepasst => Self::Eingepasst,
            Phase::Beendet => Self::Finished(*game.result().expect("finished core game")),
            _ => Self::InProgress,
        }
    }
}

/// Untrusted, versioned plain data. Exporting this discloses concealed cards,
/// seeds and choices. This is not a TableView, score entry or analysis input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameRecording {
    /// Wire format discriminator.
    pub format: u16,
    /// Supported core/dealing semantics discriminator.
    pub engine: String,
    /// Application/AI provenance; reconstruction never executes opponents.
    pub context: RecordingContext,
    /// Actual per-game seed, separate from the series RNG.
    pub seed: u64,
    /// Actual core rule configuration.
    pub rules: RuleConfig,
    /// Original deal, also checked against the seed to detect dealing drift.
    pub deal: Deal,
    /// Successfully applied actions, in order; actor is core current_seat.
    #[serde(deserialize_with = "bounded_actions")]
    pub actions: Vec<Action>,
    /// Expected completion and settlement.
    pub end: RecordingEnd,
}

/// Data from an explicitly privileged reconstruction at an action boundary.
/// `state.seats` are historical permitted views derived at that boundary;
/// they do not acquire knowledge from later actions or the final deal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegedRecordingPosition {
    /// Number of actions applied (zero is before bidding).
    pub applied_actions: usize,
    /// Full state and each seat's permitted view at this prefix.
    pub state: OmniscientView,
    /// Public core events through this prefix, in order.
    pub events: Vec<Event>,
}

/// Structured validation failure. No active session is accessed or changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingError {
    /// Unrecognised wire format.
    UnsupportedFormat {
        /// Encountered format discriminator.
        found: u16,
    },
    /// Unrecognised core/dealing semantics.
    UnsupportedEngine,
    /// Invalid game number, dealer, series length or Goldfish identity.
    InvalidContext,
    /// Stored rules disagree with the original session configuration.
    RulesMismatch,
    /// Seed no longer describes the stored initial cards.
    DealMismatch,
    /// Too many actions, including for directly constructed Rust data.
    TooManyActions {
        /// Encountered number of actions.
        count: usize,
    },
    /// Requested boundary exceeds the recorded sequence.
    InvalidPrefix {
        /// Requested action boundary.
        requested: usize,
        /// Number of recorded actions.
        available: usize,
    },
    /// Core rejected an action; index is zero-based.
    IllegalAction {
        /// Zero-based action index.
        index: usize,
        /// Core legality failure.
        error: RuleError,
    },
    /// Actions continue after a terminal state.
    ActionsAfterCompletion {
        /// Zero-based index of the first excess action.
        index: usize,
    },
    /// Completion or result disagrees with the legal sequence.
    EndMismatch,
}

impl GameRecording {
    pub(crate) fn start(context: RecordingContext, seed: u64, rules: RuleConfig) -> Self {
        Self {
            format: RECORDING_FORMAT,
            engine: RECORDING_ENGINE.into(),
            context,
            seed,
            rules,
            deal: Deal::from_seed(seed),
            actions: Vec::new(),
            end: RecordingEnd::InProgress,
        }
    }

    /// Validate the entire recording, then return the historical prefix.
    /// This is an isolated replay through the core legal transition engine;
    /// no AI, coach, RNG stream or score sheet is run or mutated.
    pub fn reconstruct_privileged(
        &self,
        prefix: usize,
    ) -> Result<PrivilegedRecordingPosition, RecordingError> {
        self.validate_header()?;
        if prefix > self.actions.len() {
            return Err(RecordingError::InvalidPrefix {
                requested: prefix,
                available: self.actions.len(),
            });
        }
        let mut game = Game::from_deal(self.deal, self.rules);
        let mut events = Vec::new();
        let mut position = (prefix == 0).then(|| game.omniscient_view());
        for (index, &action) in self.actions.iter().enumerate() {
            if game.current_seat().is_none() {
                return Err(RecordingError::ActionsAfterCompletion { index });
            }
            let applied = game
                .apply(action)
                .map_err(|error| RecordingError::IllegalAction { index, error })?;
            if index < prefix {
                events.extend(applied);
            }
            if index + 1 == prefix {
                position = Some(game.omniscient_view());
            }
        }
        if RecordingEnd::of(&game) != self.end {
            return Err(RecordingError::EndMismatch);
        }
        Ok(PrivilegedRecordingPosition {
            applied_actions: prefix,
            state: position.expect("validated prefix was reached"),
            events,
        })
    }

    fn validate_header(&self) -> Result<(), RecordingError> {
        if self.format != RECORDING_FORMAT {
            return Err(RecordingError::UnsupportedFormat { found: self.format });
        }
        if self.engine != RECORDING_ENGINE {
            return Err(RecordingError::UnsupportedEngine);
        }
        if self.actions.len() > MAX_RECORDING_ACTIONS {
            return Err(RecordingError::TooManyActions {
                count: self.actions.len(),
            });
        }
        let c = &self.context;
        let goldfish_valid = match (c.session.opponents, c.goldfish) {
            (Opponents::Goldfish, Some(PlayerId(1 | 2))) => true,
            (Opponents::Goldfish, _) => false,
            (_, None) => true,
            _ => false,
        };
        if c.number == 0
            || c.dealer.0 >= 3
            || c.dealer.0 != ((u64::from(c.number) + 1) % 3) as u8
            || matches!(c.session.series, SeriesLength::Fixed(n) if c.number > u32::from(n))
            || !goldfish_valid
            || c.application_version.is_empty()
            || c.application_version.len() > 128
        {
            return Err(RecordingError::InvalidContext);
        }
        if self.rules.kontra_re != c.session.kontra_re {
            return Err(RecordingError::RulesMismatch);
        }
        if self.deal != Deal::from_seed(self.seed) {
            return Err(RecordingError::DealMismatch);
        }
        Ok(())
    }
}

// Bound sequence allocation while deserialising, not just during replay.
fn bounded_actions<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Action>, D::Error> {
    struct Actions;
    impl<'de> serde::de::Visitor<'de> for Actions {
        type Value = Vec<Action>;

        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "at most {MAX_RECORDING_ACTIONS} game actions")
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut actions = Vec::new();
            while let Some(action) = seq.next_element()? {
                if actions.len() == MAX_RECORDING_ACTIONS {
                    return Err(serde::de::Error::custom("too many recording actions"));
                }
                actions.push(action);
            }
            Ok(actions)
        }
    }
    deserializer.deserialize_seq(Actions)
}
