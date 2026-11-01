// SPDX-License-Identifier: EUPL-1.2

//! The learned opponent engine (D-07, #69): a [`NeuralOpponent`] asks a
//! [`NeuralModel`] of `skat-nn` and falls back to a Classic [`Opponent`]
//! whenever the model cannot decide.
//!
//! Information boundary (D-23): the model sees only the deciding seat's
//! [`PlayerView`]. Fallbacks are deterministic: the Classic opponent with
//! its own seed decides, and the reason is reported.

use std::sync::Arc;

use skat_analysis::Cancel;
use skat_core::PlayerView;
use skat_nn::{Fallback, ModelError, NeuralModel};

use crate::{Decision, Opponent};

/// Anything that decides for one seat from that seat's view. The arena
/// plays games between players.
pub trait Player {
    /// Chooses one of `view.legal_actions`; `None` if it is not this
    /// seat's turn. `cancel` may stop a search early.
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision>;
}

impl Player for Opponent {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        Opponent::decide(self, view, cancel)
    }
}

/// A learned opponent: the model decides; the Classic opponent decides
/// whenever the model is missing or broken, does not cover the phase, or
/// produces an unusable output.
#[derive(Clone, Debug)]
pub struct NeuralOpponent {
    model: Result<Arc<NeuralModel>, ModelError>,
    /// The Classic opponent used for every fallback.
    pub classic: Opponent,
}

/// A decision of a [`NeuralOpponent`] and how it was reached.
#[derive(Clone, Debug, PartialEq)]
pub struct NeuralDecision {
    /// The chosen legal action (and, for a fallback, the Classic analysis
    /// status).
    pub decision: Decision,
    /// The model's value estimate for the acting seat when the model
    /// decided.
    pub value: Option<f32>,
    /// Why the Classic opponent decided instead of the model.
    pub fallback: Option<Fallback>,
}

impl NeuralOpponent {
    /// An opponent using `model` (or the error that prevented loading it,
    /// e.g. [`ModelError::Missing`]) with `classic` as fallback.
    pub fn new(model: Result<Arc<NeuralModel>, ModelError>, classic: Opponent) -> NeuralOpponent {
        NeuralOpponent { model, classic }
    }

    /// The model, or why there is none.
    pub fn model(&self) -> Result<&NeuralModel, &ModelError> {
        self.model.as_deref()
    }

    /// Decides for `view` and says whether the model or the Classic
    /// fallback chose; `None` if it is not this seat's turn.
    pub fn decide_explained(
        &self,
        view: &PlayerView,
        cancel: Option<&Cancel>,
    ) -> Option<NeuralDecision> {
        if view.legal_actions.is_empty() {
            return None;
        }
        let evaluation = match &self.model {
            Ok(model) => model.evaluate(view),
            Err(e) => Err(Fallback::Model(e.clone())),
        };
        match evaluation {
            Ok(e) => Some(NeuralDecision {
                decision: Decision {
                    action: e.action,
                    status: None,
                    fallback: false,
                },
                value: Some(e.value),
                fallback: None,
            }),
            Err(reason) => Some(NeuralDecision {
                decision: self.classic.decide(view, cancel)?,
                value: None,
                fallback: Some(reason),
            }),
        }
    }
}

impl Player for NeuralOpponent {
    fn decide(&self, view: &PlayerView, cancel: Option<&Cancel>) -> Option<Decision> {
        self.decide_explained(view, cancel).map(|d| d.decision)
    }
}
