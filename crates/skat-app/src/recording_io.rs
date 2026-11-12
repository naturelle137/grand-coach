// SPDX-License-Identifier: EUPL-1.2

//! Bounded local game files; deliberately independent of a live Session.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    GameRecording, MAX_RECORDING_ACTIONS, RECORDING_ENGINE, RECORDING_FORMAT, RecordingError,
};

/// Maximum wire size, including whitespace. Readers may grow after opening.
pub const MAX_RECORDING_BYTES: usize = 1024 * 1024;

/// Recoverable file failure. Frontends translate variants, never OS messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingFileError {
    /// Malformed JSON or malformed schema fields.
    InvalidJson,
    /// Unsupported schema version.
    UnsupportedFormat,
    /// Unsupported replay/dealing semantics.
    UnsupportedEngine,
    /// Invalid legal history, deal, context or settlement.
    InvalidRecording(RecordingError),
    /// Byte or action limit exceeded.
    TooLarge,
    /// Source does not exist.
    Missing,
    /// Exclusive creation refused to replace an existing path.
    AlreadyExists,
    /// Other filesystem/read/write failure.
    Io,
}

fn io_error(error: std::io::Error) -> RecordingFileError {
    match error.kind() {
        std::io::ErrorKind::NotFound => RecordingFileError::Missing,
        std::io::ErrorKind::AlreadyExists => RecordingFileError::AlreadyExists,
        _ => RecordingFileError::Io,
    }
}

fn validation(error: RecordingError) -> RecordingFileError {
    match error {
        RecordingError::UnsupportedFormat { .. } => RecordingFileError::UnsupportedFormat,
        RecordingError::UnsupportedEngine => RecordingFileError::UnsupportedEngine,
        RecordingError::TooManyActions { .. } => RecordingFileError::TooLarge,
        _ => RecordingFileError::InvalidRecording(error),
    }
}

impl GameRecording {
    /// Read at most the byte limit plus one sentinel byte before parsing.
    /// Full semantic validation succeeds before any recording is published.
    pub fn read_local(reader: impl Read) -> Result<Self, RecordingFileError> {
        let mut bytes = Vec::new();
        reader
            .take((MAX_RECORDING_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > MAX_RECORDING_BYTES {
            return Err(RecordingFileError::TooLarge);
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| RecordingFileError::InvalidJson)?;
        // Inspect discriminators before decoding this build's typed payload.
        if let Some(format) = value.get("format").and_then(serde_json::Value::as_u64)
            && format != u64::from(RECORDING_FORMAT)
        {
            return Err(RecordingFileError::UnsupportedFormat);
        }
        if let Some(engine) = value.get("engine").and_then(serde_json::Value::as_str)
            && engine != RECORDING_ENGINE
        {
            return Err(RecordingFileError::UnsupportedEngine);
        }
        if value
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| actions.len() > MAX_RECORDING_ACTIONS)
        {
            return Err(RecordingFileError::TooLarge);
        }
        let recording: Self =
            serde_json::from_value(value).map_err(|_| RecordingFileError::InvalidJson)?;
        recording
            .reconstruct_privileged(recording.actions.len())
            .map_err(validation)?;
        Ok(recording)
    }

    /// Open a local recording. This is a game file, not session resumption.
    pub fn open_local(path: impl AsRef<Path>) -> Result<Self, RecordingFileError> {
        let path = path.as_ref();
        // Reject directories/devices/FIFOs before opening (a FIFO could block).
        if !std::fs::metadata(path).map_err(io_error)?.is_file() {
            return Err(RecordingFileError::Io);
        }
        let file = File::open(path).map_err(io_error)?;
        if !file.metadata().map_err(io_error)?.is_file() {
            return Err(RecordingFileError::Io);
        }
        Self::read_local(file)
    }

    /// Validate and serialize before exclusive creation. Existing files and
    /// symlinks are never truncated. A failed write may leave a partial new
    /// file; it must be removed or a different name selected before retrying.
    pub fn save_local_new(&self, path: impl AsRef<Path>) -> Result<(), RecordingFileError> {
        self.reconstruct_privileged(self.actions.len())
            .map_err(validation)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| RecordingFileError::InvalidJson)?;
        if bytes.len() > MAX_RECORDING_BYTES {
            return Err(RecordingFileError::TooLarge);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(io_error)?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(io_error)
    }
}
