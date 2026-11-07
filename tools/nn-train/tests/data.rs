// SPDX-License-Identifier: EUPL-1.2

//! Training data storage: the on-disk cache gives exactly the loaded data
//! and changes its key with the data and the target settings; training
//! with the rows resident on the device gives the same model as training
//! from RAM.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use burn::tensor::Device;
use nn_train::cache::{CacheUse, key, load_cached, read, sources, write};
use nn_train::features::FeatureCodes;
use nn_train::resident::ResidentRows;
use nn_train::train::{
    Data, Hyper, PHASES, Recipe, TRAIN_FORMAT, TrainLimits, load_with, train, train_with,
};
use skat_nn::{ENCODER_VERSION, FEATURES};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/selfplay")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("nn-train-data-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn same(a: &Data, b: &Data) {
    assert_eq!(a.train, b.train);
    assert_eq!(a.validation, b.validation);
    assert_eq!(a.test, b.test);
    assert_eq!(a.coverage, b.coverage);
    assert_eq!(a.sources, b.sources);
}

#[test]
fn the_cache_returns_exactly_the_loaded_data() {
    let cache = scratch("cache");
    let hyper = Hyper {
        soft_weight: 0.25,
        ..Hyper::default()
    };
    for beliefs in [false, true] {
        let loaded = load_with(&[fixture()], &hyper, beliefs).unwrap();
        let (first, used) = load_cached(&[fixture()], &hyper, beliefs, &cache).unwrap();
        assert_eq!(used, CacheUse::Written);
        same(&first, &loaded);
        let (second, used) = load_cached(&[fixture()], &hyper, beliefs, &cache).unwrap();
        assert_eq!(used, CacheUse::Hit);
        same(&second, &loaded);
    }
    // The cheap sources (from .done files) equal the loaded ones.
    let loaded = load_with(&[fixture()], &hyper, false).unwrap();
    assert_eq!(sources(&[fixture()]).unwrap(), loaded.sources);
    // A damaged cache file is rebuilt (the file of this key; the cache
    // also holds the one with belief targets).
    let file = cache.join(format!("{}.gcdata", key(&loaded.sources, &hyper, false)));
    let bytes = fs::read(&file).unwrap();
    fs::write(&file, &bytes[..bytes.len() / 2]).unwrap();
    let (data, used) = load_cached(&[fixture()], &hyper, false, &cache).unwrap();
    assert_eq!(used, CacheUse::Written);
    same(&data, &loaded);
}

#[test]
fn cache_files_keep_wide_features() {
    let mut data = load_with(&[fixture()], &Hyper::default(), false).unwrap();
    let f = &data.train.features;
    let rows = f.rows();
    let mut codes = f.codes().to_vec();
    let mut tables = f.tables();
    for r in 0..rows {
        codes[r * FEATURES + 3] = 0;
    }
    tables[3] = vec![0.0];
    let column = (0..rows).map(|r| r as f32 / 7.0).collect();
    data.train.features = FeatureCodes::from_parts(codes, tables, vec![(3, column)]).unwrap();
    let dir = scratch("wide");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("data.gcdata");
    write(&path, "key", &data).unwrap();
    same(&read(&path, "key").unwrap(), &data);
    assert!(read(&path, "other key").is_err());
}

#[test]
fn the_cache_key_follows_data_and_target_settings() {
    let s = sources(&[fixture()]).unwrap();
    let h = Hyper::default();
    let k = key(&s, &h, false);
    assert_ne!(k, key(&s, &h, true));
    let soft = Hyper {
        soft_weight: 0.5,
        ..h.clone()
    };
    assert_ne!(k, key(&s, &soft, false));
    let mix = Hyper {
        value_mix: 0.25,
        ..h.clone()
    };
    assert_ne!(k, key(&s, &mix, false));
    // Settings that do not touch the examples keep the key.
    let other = Hyper {
        hidden: 64,
        learning_rate: 0.1,
        seed: 9,
        ..h.clone()
    };
    assert_eq!(k, key(&s, &other, false));
    let mut changed = s.clone();
    changed[0].shards -= 1;
    assert_ne!(k, key(&changed, &h, false));
}

#[test]
fn training_with_resident_rows_gives_the_same_model() {
    let hyper = Hyper {
        hidden: 16,
        batch: 32,
        epochs: 3,
        learning_rate: 1e-2,
        ..Hyper::default()
    };
    let data = load_with(&[fixture()], &hyper, false).unwrap();
    let recipe = Recipe {
        train_format: TRAIN_FORMAT,
        encoder_version: ENCODER_VERSION,
        phases: PHASES.bits(),
        data: data.sources.clone(),
        hyper: hyper.clone(),
    };
    let limits = TrainLimits {
        wall_time: Duration::from_secs(3600),
    };
    let device = Device::flex().autodiff();
    let ram = train(&scratch("ram"), &recipe, &data, &device, limits, |_| true).unwrap();
    let resident = ResidentRows::upload(&data.train.features, skat_nn::FEATURES, &device);
    let dev = train_with(
        &scratch("resident"),
        &recipe,
        &data,
        Some(&resident),
        &device,
        limits,
        |_| true,
    )
    .unwrap();
    // One block: the batches keep their order, so the runs are identical.
    assert_eq!(dev.model, ram.model);
    let losses = |t: &nn_train::train::Trained| {
        t.state
            .logs
            .iter()
            .map(|l| (l.train_loss, l.validation))
            .collect::<Vec<_>>()
    };
    assert_eq!(losses(&dev), losses(&ram));
}
