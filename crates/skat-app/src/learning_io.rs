// SPDX-License-Identifier: EUPL-1.2

//! The learning history file (D-70): bounded reading with structured
//! errors, atomic replacement on save, and setting a damaged file aside
//! under a new name instead of overwriting it.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::learning::{LEARNING_FORMAT, LearningError, LearningHistory};

/// Maximum file size, including whitespace.
pub const MAX_LEARNING_BYTES: usize = 16 * 1024 * 1024;

/// Most numbered names tried when setting a damaged file aside.
const MAX_ASIDE_NAMES: u32 = 999;

/// Recoverable file failure. Frontends translate variants, never OS messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningFileError {
    /// Malformed JSON or malformed schema fields.
    InvalidJson,
    /// Another format version, for example written by a newer version.
    UnsupportedFormat,
    /// Well-formed but inconsistent content.
    Invalid(LearningError),
    /// Byte, event or attempt limit exceeded.
    TooLarge,
    /// The file does not exist (an empty history, not an error to report).
    Missing,
    /// Other filesystem/read/write failure.
    Io,
}

fn io_error(error: std::io::Error) -> LearningFileError {
    match error.kind() {
        std::io::ErrorKind::NotFound => LearningFileError::Missing,
        _ => LearningFileError::Io,
    }
}

impl LearningHistory {
    /// Reads at most the byte limit plus one sentinel byte, checks the
    /// format version before decoding and validates the whole content.
    pub fn read_local(reader: impl Read) -> Result<Self, LearningFileError> {
        let mut bytes = Vec::new();
        reader
            .take((MAX_LEARNING_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() > MAX_LEARNING_BYTES {
            return Err(LearningFileError::TooLarge);
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| LearningFileError::InvalidJson)?;
        match value.get("format").and_then(serde_json::Value::as_u64) {
            Some(format) if format == u64::from(LEARNING_FORMAT) => {}
            Some(_) => return Err(LearningFileError::UnsupportedFormat),
            None => return Err(LearningFileError::InvalidJson),
        }
        let mut history: Self =
            serde_json::from_value(value).map_err(|_| LearningFileError::InvalidJson)?;
        history.validate().map_err(|e| match e {
            LearningError::UnsupportedFormat => LearningFileError::UnsupportedFormat,
            LearningError::TooLarge => LearningFileError::TooLarge,
            e => LearningFileError::Invalid(e),
        })?;
        Ok(history)
    }

    /// Opens the history at `path`; [`LearningFileError::Missing`] if there
    /// is none yet.
    pub fn open_local(path: impl AsRef<Path>) -> Result<Self, LearningFileError> {
        let path = path.as_ref();
        // Reject directories/devices/FIFOs before opening (a FIFO could block).
        if !std::fs::metadata(path).map_err(io_error)?.is_file() {
            return Err(LearningFileError::Io);
        }
        let file = File::open(path).map_err(io_error)?;
        Self::read_local(file)
    }

    /// Writes the history to `path`: first completely to `<path>.tmp`, then
    /// renamed over `path`, so an interrupted save leaves the previous file
    /// intact. Creates missing parent directories. Callers must only save
    /// over a file they read successfully (or that did not exist); a damaged
    /// file is set aside with [`LearningHistory::set_aside`] first.
    pub fn save_local(&self, path: impl AsRef<Path>) -> Result<(), LearningFileError> {
        let path = path.as_ref();
        let bytes = serde_json::to_vec(self).map_err(|_| LearningFileError::InvalidJson)?;
        if bytes.len() > MAX_LEARNING_BYTES {
            return Err(LearningFileError::TooLarge);
        }
        if let Ok(meta) = std::fs::symlink_metadata(path)
            && !meta.is_file()
        {
            return Err(LearningFileError::Io);
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(io_error)?;
        }
        let temp = temp_path(path);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(io_error)?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(io_error)?;
        drop(file);
        std::fs::rename(&temp, path).map_err(io_error)
    }

    /// Renames the file at `path` to the first free
    /// `<stem>.damaged-<n>.json` next to it and returns the new path. No
    /// existing file is replaced; the content is kept unchanged.
    pub fn set_aside(path: impl AsRef<Path>) -> Result<PathBuf, LearningFileError> {
        let path = path.as_ref();
        std::fs::symlink_metadata(path).map_err(io_error)?;
        let stem = path
            .file_stem()
            .map_or_else(|| "learning-history".into(), |s| s.to_string_lossy());
        for n in 1..=MAX_ASIDE_NAMES {
            let candidate = path.with_file_name(format!("{stem}.damaged-{n}.json"));
            if std::fs::symlink_metadata(&candidate).is_err() {
                std::fs::rename(path, &candidate).map_err(io_error)?;
                return Ok(candidate);
            }
        }
        Err(LearningFileError::Io)
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}
