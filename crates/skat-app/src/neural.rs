// SPDX-License-Identifier: EUPL-1.2

//! The card-play model of the [`Opponents::Neural`](crate::Opponents)
//! table mode, loaded once per process from `assets/models/` (#73).
//!
//! Search order for the directory: the environment variable
//! `GRAND_COACH_MODELS`, `assets/models` next to the executable, then the
//! workspace's `assets/models` (development builds). The expected hash is
//! the `model_sha256` of the training manifest `card-play.json`; without a
//! manifest the file's own hash is used. A missing or broken model is not
//! an error: the neural opponents then decide with their Classic fallback.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use skat_nn::{ModelError, NeuralModel, sha256};

const MODEL: &str = "card-play.gcnn";
const MANIFEST: &str = "card-play.json";

static LOADED: OnceLock<Result<Arc<NeuralModel>, ModelError>> = OnceLock::new();

/// The shipped card-play model, or why it is not available.
pub fn neural_model() -> Result<Arc<NeuralModel>, ModelError> {
    LOADED.get_or_init(load).clone()
}

/// `true` if the card-play model loaded; otherwise the
/// [`Opponents::Neural`](crate::Opponents) mode plays Classic Strong.
pub fn neural_model_available() -> bool {
    neural_model().is_ok()
}

fn directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("GRAND_COACH_MODELS") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
    {
        dirs.push(exe_dir.join("assets/models"));
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/models"));
    dirs
}

fn load() -> Result<Arc<NeuralModel>, ModelError> {
    let dir = directories()
        .into_iter()
        .find(|d| d.join(MODEL).is_file())
        .ok_or(ModelError::Missing)?;
    let bytes = std::fs::read(dir.join(MODEL)).map_err(|_| ModelError::Missing)?;
    let expected = match std::fs::read_to_string(dir.join(MANIFEST)) {
        Ok(manifest) => recorded_hash(&manifest).ok_or(ModelError::HashMismatch)?,
        Err(_) => sha256(&bytes),
    };
    NeuralModel::load(&bytes, &expected).map(Arc::new)
}

/// The `model_sha256` recorded in a training manifest.
fn recorded_hash(manifest: &str) -> Option<[u8; 32]> {
    let hex = manifest
        .split("\"model_sha256\"")
        .nth(1)?
        .split('"')
        .nth(1)?;
    if hex.len() != 64 {
        return None;
    }
    let mut hash = [0; 32];
    for (i, byte) in hash.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(hash)
}

#[cfg(test)]
mod tests {
    use super::recorded_hash;

    #[test]
    fn bundled_manifest_loads_the_selected_v2() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
        let manifest = std::fs::read_to_string(dir.join("card-play.json")).unwrap();
        let bytes = std::fs::read(dir.join("card-play.gcnn")).unwrap();
        let expected = recorded_hash(&manifest).unwrap();
        let model = skat_nn::NeuralModel::load(&bytes, &expected).unwrap();
        assert_eq!(model.header().encoder_version, 2);
        assert_eq!(model.header().shape.layers, 4);
        assert_eq!(model.header().shape.hidden, 384);
    }

    #[test]
    fn reads_the_manifest_hash() {
        let hex = "ab".repeat(32);
        let manifest = format!("{{\n  \"model_bytes\": 1,\n  \"model_sha256\": \"{hex}\"\n}}");
        assert_eq!(recorded_hash(&manifest), Some([0xab; 32]));
        assert_eq!(recorded_hash("{\"model_sha256\": \"abc\"}"), None);
        assert_eq!(recorded_hash("{}"), None);
    }
}
