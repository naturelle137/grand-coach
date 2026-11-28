// SPDX-License-Identifier: EUPL-1.2

//! gdext class `SkatApp`: GDScript-facing wrapper of [`Adapter`].
//! Methods return JSON text; failures are `{"error": ...}` objects.

#![allow(unsafe_code)] // gdext entry point only; no other unsafe here

use godot::prelude::*;

use crate::adapter::{Adapter, AdapterError};

struct GrandCoachExtension;

// SAFETY: the one `unsafe impl` gdext requires of every extension library.
#[gdextension]
unsafe impl ExtensionLibrary for GrandCoachExtension {}

/// Godot node holding one optional session.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
pub struct SkatApp {
    adapter: Option<Adapter>,
}

fn error_text(e: &AdapterError) -> GString {
    GString::from(e.to_json().as_str())
}

#[godot_api]
impl SkatApp {
    /// Starts (or restarts) a session from a `SessionConfig` JSON text.
    /// Returns `""` on success, else an error JSON.
    #[func]
    fn start(&mut self, config_json: GString) -> GString {
        let config = config_json.to_string();
        let result = match self.adapter.as_mut() {
            Some(adapter) => adapter.restart(&config),
            None => Adapter::new(&config).map(|a| self.adapter = Some(a)),
        };
        result.err().map(|e| error_text(&e)).unwrap_or_default()
    }

    /// Leaves the session; running work is cancelled.
    #[func]
    fn stop(&mut self) {
        self.adapter = None;
    }

    /// The permitted `TableView` as JSON, or `""` without a session.
    #[func]
    fn view(&self) -> GString {
        self.adapter
            .as_ref()
            .map(|a| GString::from(a.view_json().as_str()))
            .unwrap_or_default()
    }

    /// Figure catalogue keys as a JSON array, indexed by the view's `FigureId`.
    #[func]
    fn figure_keys(&self) -> GString {
        GString::from(Adapter::figure_keys_json().as_str())
    }

    /// Executes a `Command` JSON; returns the events or an error JSON.
    #[func]
    fn execute(&mut self, command_json: GString) -> GString {
        let Some(adapter) = self.adapter.as_mut() else {
            return GString::from(r#"{"error":"NoSession"}"#);
        };
        match adapter.execute_json(&command_json.to_string()) {
            Ok(events) => GString::from(events.as_str()),
            Err(e) => error_text(&e),
        }
    }

    /// Keeps the learning history in the file at `path` (absolute, e.g. the
    /// globalised `user://` path). No-op without a session.
    #[func]
    fn use_learning_file(&mut self, path: GString) {
        if let Some(adapter) = self.adapter.as_mut() {
            adapter.use_learning_file(path.to_string().into());
        }
    }

    /// Number of recorded learning events of this history.
    #[func]
    fn learning_events(&self) -> i64 {
        self.adapter
            .as_ref()
            .map_or(0, |a| a.learning_events() as i64)
    }

    /// Start or retry the current deal as an isolated practice.
    #[func]
    fn practice(&mut self) -> GString {
        let Some(adapter) = self.adapter.as_mut() else {
            return GString::from(r#"{"error":"NoSession"}"#);
        };
        adapter
            .practice()
            .err()
            .map(|e| error_text(&e))
            .unwrap_or_default()
    }

    /// Save a completed game to a new local file.
    #[func]
    fn save_recording(&self, path: GString) -> GString {
        let Some(adapter) = &self.adapter else {
            return GString::from(r#"{"error":"NoSession"}"#);
        };
        adapter
            .save_recording(std::path::Path::new(&path.to_string()))
            .err()
            .map(|e| error_text(&e))
            .unwrap_or_default()
    }

    /// Return from practice to the parked series.
    #[func]
    fn return_to_series(&mut self) {
        if let Some(adapter) = self.adapter.as_mut() {
            adapter.return_to_series();
        }
    }

    /// Descriptive learning summary without deal data.
    #[func]
    fn learning_summary(&self) -> GString {
        self.adapter
            .as_ref()
            .map(|a| GString::from(a.learning_summary_json().as_str()))
            .unwrap_or_default()
    }

    /// Open a local recording.
    #[func]
    fn open_recording(&mut self, path: GString) -> GString {
        let Some(adapter) = &mut self.adapter else {
            return GString::from(r#"{"error":"NoSession"}"#);
        };
        adapter
            .open_recording(std::path::Path::new(&path.to_string()))
            .err()
            .map(|e| error_text(&e))
            .unwrap_or_default()
    }

    /// Historical permitted view at the requested action.
    #[func]
    fn replay(&self, index: i64) -> GString {
        let Some(adapter) = &self.adapter else {
            return GString::from(r#"{"error":"NoSession"}"#);
        };
        match usize::try_from(index)
            .ok()
            .and_then(|i| adapter.replay_json(i).ok())
        {
            Some(view) => GString::from(view.as_str()),
            None => GString::from(r#"{"error":"InvalidReplayPosition"}"#),
        }
    }

    /// Starts the pending opponent computation on a worker thread.
    #[func]
    fn start_opponent(&mut self) -> bool {
        self.adapter.as_mut().is_some_and(Adapter::start_opponent)
    }

    /// Collects a finished opponent move: events JSON, `""` if nothing
    /// changed, or an error JSON.
    #[func]
    fn poll_opponent(&mut self) -> GString {
        let Some(adapter) = self.adapter.as_mut() else {
            return GString::new();
        };
        match adapter.poll_opponent() {
            Ok(Some(events)) => GString::from(events.as_str()),
            Ok(None) => GString::new(),
            Err(e) => error_text(&e),
        }
    }
}
