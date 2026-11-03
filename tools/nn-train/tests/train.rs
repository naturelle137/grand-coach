// SPDX-License-Identifier: EUPL-1.2

//! Training of the card-play network (#73) on the fixture dataset: only
//! card-play decisions become examples, targets follow the recipe, and a
//! run is reproducible from its recipe, also when it is interrupted and
//! resumed.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use burn::tensor::Device;
use nn_train::selfplay::{DecisionKind, Split};
use nn_train::shards::read_shard;
use nn_train::train::{
    Hyper, PHASES, Recipe, TRAIN_FORMAT, TrainLimits, TrainStop, Trained, load, train,
};
use skat_core::Phase;
use skat_nn::{ENCODER_VERSION, NeuralModel, sha256};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/selfplay")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("nn-train-train-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn hyper() -> Hyper {
    Hyper {
        hidden: 16,
        batch: 32,
        epochs: 6,
        learning_rate: 1e-2,
        ..Hyper::default()
    }
}

fn recipe(hyper: &Hyper) -> (Recipe, nn_train::train::Data) {
    let data = load(&[fixture()], hyper).unwrap();
    let recipe = Recipe {
        train_format: TRAIN_FORMAT,
        encoder_version: ENCODER_VERSION,
        phases: PHASES.bits(),
        data: data.sources.clone(),
        hyper: hyper.clone(),
    };
    (recipe, data)
}

const NO_CAP: TrainLimits = TrainLimits {
    wall_time: Duration::from_secs(3600),
};

fn run(dir: &Path, stop_after: Option<usize>) -> Trained {
    let (recipe, data) = recipe(&hyper());
    train(
        dir,
        &recipe,
        &data,
        &Device::flex().autodiff(),
        NO_CAP,
        |log| Some(log.epoch) != stop_after,
    )
    .unwrap()
}

#[test]
fn examples_are_the_card_play_decisions_with_their_targets() {
    let h = Hyper {
        value_mix: 0.0,
        ..hyper()
    };
    let data = load(&[fixture()], &h).unwrap();
    let records: Vec<_> = (0..2)
        .flat_map(|i| read_shard(&fixture(), i).unwrap())
        .collect();
    let spiel: Vec<_> = records
        .iter()
        .filter(|r| r.view.phase == Phase::Spiel)
        .collect();
    let rows = data.train.rows() + data.validation.rows() + data.test.rows();
    assert_eq!(data.sources[0].manifest.encoder_version, 1);
    assert_eq!(rows, spiel.len());
    assert!(data.train.rows() > 0);
    for c in &data.coverage {
        assert!(matches!(
            c.kind,
            DecisionKind::Spiel | DecisionKind::KontraRe
        ));
    }
    // With `value_mix` 0 the value target is the seat's final points; the
    // policy target is the teacher's action.
    let train: Vec<_> = spiel.iter().filter(|r| r.split == Split::Train).collect();
    for (row, r) in train.iter().enumerate() {
        let e = &data.train;
        let width = skat_nn::FEATURES;
        assert_eq!(
            &e.features[row * width..][..width],
            skat_nn::encode(&r.view)
        );
        assert_eq!(e.value[row], r.final_points as f32);
        let legal = &e.target[e.offsets[row]..e.offsets[row + 1]];
        assert_eq!(legal.len(), r.view.legal_actions.len());
        let chosen = e.chosen[row] as usize;
        assert_eq!(r.view.legal_actions[chosen], r.action);
        assert_eq!(legal[chosen], 1.0);
    }

    // Soft targets spread probability over the teacher's options but
    // still sum to one; the value mixes in the teacher's option value.
    let soft = load(
        &[fixture()],
        &Hyper {
            soft_weight: 0.5,
            value_mix: 1.0,
            ..hyper()
        },
    )
    .unwrap();
    let e = &soft.train;
    let mut spread = false;
    for row in 0..e.rows() {
        let t = &e.target[e.offsets[row]..e.offsets[row + 1]];
        assert!((t.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        spread |= t.len() > 1 && t.iter().all(|&p| p > 0.0);
    }
    assert!(spread, "complete analyses give soft targets");
    // Option targets: the teacher's expected points and win chance per
    // legal action; with `value_mix` 1 the value is the chosen option's.
    let mut with_options = 0;
    for row in 0..e.rows() {
        let range = e.offsets[row]..e.offsets[row + 1];
        assert!(
            e.option_win[range.clone()]
                .iter()
                .all(|w| (0.0..=1.0).contains(w))
        );
        if e.has_options[row] {
            with_options += 1;
            let chosen = range.start + e.chosen[row] as usize;
            assert_eq!(e.value[row], e.option_score[chosen]);
        }
    }
    assert!(with_options > 0);
    assert_ne!(e.value, data.train.value);
}

#[test]
fn training_is_reproducible_and_resumes_exactly() {
    let a = run(&scratch("a"), None);
    let b = run(&scratch("b"), None);
    assert_eq!(a.stop, TrainStop::Finished);
    assert_eq!(a.state.logs.len(), 6);
    assert_eq!(a.model, b.model, "same recipe, same model");
    let first = a.state.logs.first().unwrap().train_loss;
    let last = a.state.logs.last().unwrap().train_loss;
    assert!(last < first, "{first} -> {last}");

    // Interrupted after the first epoch, then resumed from the checkpoint.
    let dir = scratch("resumed");
    let part = run(&dir, Some(1));
    assert_eq!(part.stop, TrainStop::Stopped);
    assert_eq!(part.state.logs.len(), 1);
    let resumed = run(&dir, None);
    assert_eq!(resumed.state.logs.len(), 6);
    assert_eq!(resumed.model, a.model, "resuming changes nothing");
    let losses = |t: &Trained| -> Vec<f64> { t.state.logs.iter().map(|l| l.train_loss).collect() };
    assert_eq!(losses(&resumed), losses(&a));

    // The model file loads into the CPU runtime and covers card play only.
    let model = NeuralModel::load(&a.model, &sha256(&a.model)).unwrap();
    assert_eq!(model.header().phases, PHASES);
    assert!(!model.header().phases.contains(Phase::Reizen));
}

#[test]
fn a_run_directory_refuses_another_recipe_and_a_corrupt_checkpoint() {
    let dir = scratch("refuse");
    run(&dir, Some(1));
    let (mut other, data) = recipe(&hyper());
    other.hyper.seed += 1;
    let refused = train(
        &dir,
        &other,
        &data,
        &Device::flex().autodiff(),
        NO_CAP,
        |_| true,
    );
    assert!(refused.is_err());

    let checkpoint = fs::read_dir(dir.join("checkpoint"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    fs::write(checkpoint.join("weights.bpk"), b"garbage").unwrap();
    let (recipe, data) = recipe(&hyper());
    let corrupt = train(
        &dir,
        &recipe,
        &data,
        &Device::flex().autodiff(),
        NO_CAP,
        |_| true,
    );
    assert!(corrupt.unwrap_err().to_string().contains("corrupt"));
}

#[test]
fn no_epoch_starts_after_the_wall_time_cap() {
    let (recipe, data) = recipe(&hyper());
    let t = train(
        &scratch("cap"),
        &recipe,
        &data,
        &Device::flex().autodiff(),
        TrainLimits {
            wall_time: Duration::ZERO,
        },
        |_| true,
    )
    .unwrap();
    assert_eq!(t.stop, TrainStop::WallTime);
    assert!(t.state.logs.is_empty());
}

#[test]
fn deeper_networks_train_and_recipes_of_two_layers_read_as_before() {
    let h = Hyper {
        layers: 3,
        epochs: 2,
        ..hyper()
    };
    let (recipe, data) = recipe(&h);
    let dir = scratch("deep");
    let t = train(
        &dir,
        &recipe,
        &data,
        &Device::flex().autodiff(),
        NO_CAP,
        |_| true,
    )
    .unwrap();
    let model = NeuralModel::load(&t.model, &sha256(&t.model)).unwrap();
    assert_eq!(model.header().shape.layers, 3);
    let json = serde_json::to_value(&h).unwrap();
    assert_eq!(json["layers"], 3);

    // Two layers are not written, and a recipe without the field reads as
    // two layers, so earlier runs resume and compare unchanged.
    let two = hyper();
    let mut json = serde_json::to_value(&two).unwrap();
    assert!(json.get("layers").is_none());
    json.as_object_mut().unwrap().remove("layers");
    assert_eq!(serde_json::from_value::<Hyper>(json).unwrap(), two);
}
