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
