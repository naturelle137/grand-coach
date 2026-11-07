// SPDX-License-Identifier: EUPL-1.2

//! Training tool for the learned engines (D-07, D-11). Not part of the
//! app; it trains the network definition of `skat-nn` with Burn's autodiff
//! on the CPU (Flex) or, with the feature `vulkan`, on a GPU via
//! wgpu/Vulkan.
//!
//! ```sh
//! cargo run --locked --release -p nn-train -- smoke cpu [GAMES] [STEPS] [OUT_DIR]
//! cargo run --locked --release -p nn-train --features vulkan -- smoke vulkan [GAMES] [STEPS] [OUT_DIR]
//! cargo run --locked --release -p nn-train -- fixture SEED OUT_FILE
//! cargo run --locked --release -p nn-train -- generate OUT_DIR [--deals N]
//!     [--shard-deals N] [--seed N] [--minutes N] [--max-mb N] [--cpus N]
//!     [--workers N] [--search-backend cpu|profile|cpu-endgame|vulkan] [--rest-tricks 2|3|4]
//!     [--table teacher|easy-mix|rookie-mix|medium-mix|level-mix]
//! cargo run --locked --release -p nn-train -- verify DIR
//! cargo run --locked --release -p nn-train -- train OUT_DIR DATA_DIR...
//!     [--hidden N] [--batch N] [--lr X] [--epochs N] [--value-weight X]
//!     [--value-mix X] [--soft-weight X] [--soft-temperature X]
//!     [--score-weight X] [--win-weight X] [--seed N] [--minutes N]
//!     [--device cpu|vulkan] [--patience N [--lr-decay X] [--lr-decays N]]
//!     [--cache DIR] [--data-on-device GB]
//! cargo run --locked --release -p nn-train -- evaluate MODEL DATA_DIR...
//! cargo run --locked --release -p nn-train -- rl OUT_DIR [--start MODEL]
//!     [--games N] [--pool easy=W,medium=W,strong=W,ref=W,snap=W] [--seed N]
//!     [--minutes N] [--iterations N] [--cpus N] [--workers N] (more: usage)
//! ```
//!
//! `smoke` is the stack smoke test of #69: it encodes the decisions of
//! `GAMES` seeded games of the Classic level Easy (policy target: Easy's
//! action; value target: the acting seat's Seeger-Fabian points), trains a
//! small network for `STEPS` full-batch Adam steps, and checks the
//! checkpoint round trip: the model file loads into the CPU runtime with
//! the same outputs, and a restored optimizer continues exactly like the
//! original. With `OUT_DIR` it writes `model.gcnn` and `optimizer.bpk`
//! there. `fixture` writes a tiny random-weight model for trying the
//! arena (`calibrate -- nn:<file>`). `generate` plays Strong self-play
//! and writes raw decision records into shards (#70,
//! [`nn_train::shards`]); it resumes an interrupted run in the same
//! directory and stops at its wall-time and disk caps. `verify` checks
//! every shard of a run and reports incomplete ones. `train` trains the
//! card-play network on the card-play decisions of one or more runs (#73,
//! [`nn_train::train`]); it resumes from `OUT_DIR/checkpoint`, stops at
//! its wall-time cap and writes `model.gcnn` (the epoch with the lowest
//! validation loss) and `train.json` (recipe with data hashes, coverage,
//! epoch logs, metrics per split, model hash). With `--patience N`, N
//! epochs without a lower validation loss continue from the best weights
//! with the learning rate times `--lr-decay` (default 0.1), at most
//! `--lr-decays` times (default 1); the next such plateau stops the run.
//! Examples are held with one byte per feature ([`nn_train::features`]).
//! `--cache DIR` keeps the loaded data in a binary file per data set and
//! target settings ([`nn_train::cache`]) and reads it back on the next run.
//! `--data-on-device GB` keeps the training rows in device memory if they
//! fit in GB ([`nn_train::resident`]; for `--device vulkan`), otherwise
//! batches come from RAM as without it. `evaluate` reports a
//! model file's metrics on every split of the given runs, so models
//! trained on different data compare on the same examples. `rl` refines
//! a model by self-play against a frozen opponent pool (#75,
//! [`nn_train::rl`]): it resumes from `OUT_DIR/checkpoint`, keeps learner
//! snapshots in `OUT_DIR/snapshots`, stops at its wall-time or iteration
//! cap or after the iteration during which `OUT_DIR/STOP` appeared, and
//! writes the current learner `model.gcnn` and `rl.json` (recipe, logs,
//! snapshots).

#[cfg(feature = "vulkan-search")]
mod endgame;

use std::time::Instant;

use burn::module::Module;
use burn::optim::{AdamConfig, GradientsParams, ModuleOptimizer};
use burn::store::ModuleRecord;
use burn::tensor::activation::log_softmax;
use burn::tensor::{Bytes, Device, Tensor, TensorData};
use skat_ai::{Level, Opponent};
use skat_core::{Game, RuleConfig};
use skat_nn::{
    ACTIONS, ENCODER_VERSION, FEATURES, ModelHeader, NetShape, NeuralModel, Phases, PolicyValueNet,
    action_index, encode, sha256, write_model,
};

/// Hidden width of the smoke-test network.
const HIDDEN: usize = 64;
/// Scale of the value target in the loss (points per unit).
const VALUE_SCALE: f32 = 60.0;
/// Learning rate of the smoke test.
const LEARNING_RATE: f64 = 1e-3;

/// Training examples: features, legal-action mask (0 legal, -1e9 illegal),
/// one-hot policy target and value target, each row-major.
struct Batch {
    rows: usize,
    features: Vec<f32>,
    mask: Vec<f32>,
    policy: Vec<f32>,
    value: Vec<f32>,
}

/// Encodes every decision of `games` seeded games of the Easy level.
fn dataset(games: u64) -> Batch {
    let rules = RuleConfig { kontra_re: true };
    let mut batch = Batch {
        rows: 0,
        features: Vec::new(),
        mask: Vec::new(),
        policy: Vec::new(),
        value: Vec::new(),
    };
    for deal in 0..games {
        let players = [0, 1, 2].map(|s| Opponent::new(Level::Easy, s));
        let mut game = Game::new(deal, rules);
        let mut rows = Vec::new();
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            let action = players[seat.index()]
                .decide(&view, None)
                .expect("the seat to move decides")
                .action;
            let mut mask = vec![-1e9f32; ACTIONS];
            for &a in &view.legal_actions {
                mask[action_index(a).expect("legal actions have an index")] = 0.0;
            }
            let mut policy = vec![0.0f32; ACTIONS];
            policy[action_index(action).expect("legal actions have an index")] = 1.0;
            rows.push((seat, encode(&view), mask, policy));
            game.apply(action).expect("legal action");
        }
        let points = game.result().map(|r| r.seeger_fabian());
        for (seat, features, mask, policy) in rows {
            batch.rows += 1;
            batch.features.extend(features);
            batch.mask.extend(mask);
            batch.policy.extend(policy);
            batch
                .value
                .push(points.map_or(0.0, |p| p[seat.index()] as f32));
        }
    }
    batch
}

fn tensor(values: &[f32], shape: [usize; 2], device: &Device) -> Tensor<2> {
    Tensor::from_data(TensorData::new(values.to_vec(), shape), device)
}

/// Masked cross-entropy of the policy plus the scaled squared value error.
fn loss(net: &PolicyValueNet, batch: &Batch, device: &Device) -> Tensor<1> {
    let n = batch.rows;
    let (logits, value) = net.forward(tensor(&batch.features, [n, FEATURES], device));
    let logits = logits + tensor(&batch.mask, [n, ACTIONS], device);
    let policy_loss = (log_softmax(logits, 1) * tensor(&batch.policy, [n, ACTIONS], device))
        .sum_dim(1)
        .mean()
        .neg();
    let value_loss = ((value - tensor(&batch.value, [n, 1], device)) / VALUE_SCALE)
        .powi_scalar(2)
        .mean();
    policy_loss + value_loss
}

fn scalar(t: Tensor<1>) -> f32 {
    t.into_scalar::<f32>()
}

/// One Adam step; returns the loss before it.
fn step(
    net: PolicyValueNet,
    optim: &mut ModuleOptimizer,
    batch: &Batch,
    device: &Device,
) -> (PolicyValueNet, f32) {
    let l = loss(&net, batch, device);
    let value = scalar(l.clone());
    let grads = GradientsParams::from_grads(l.backward(), &net);
    (optim.step(LEARNING_RATE, net, grads), value)
}

/// Outputs of a CPU network on the first `rows` examples.
fn outputs(net: &PolicyValueNet, batch: &Batch, rows: usize) -> Vec<f32> {
    let cpu = Device::flex();
    let net = net.clone().to_device(&cpu);
    let (logits, value) = net.forward(tensor(
        &batch.features[..rows * FEATURES],
        [rows, FEATURES],
        &cpu,
    ));
    let mut out = logits.into_data().try_to_vec::<f32>().expect("f32");
    out.extend(value.into_data().try_to_vec::<f32>().expect("f32"));
    out
}

/// What the smoke test measured.
struct Report {
    examples: usize,
    first_loss: f32,
    last_loss: f32,
    model_bytes: Vec<u8>,
    optimizer_bytes: Vec<u8>,
}

/// Trains on `device`, then checks the model file and optimizer checkpoint
/// round trips.
fn smoke(device: &Device, games: u64, steps: usize) -> Report {
    let batch = dataset(games);
    device.seed(69);
    let shape = NetShape::current(HIDDEN);
    let mut net = PolicyValueNet::new(shape, device);
    let mut optim = AdamConfig::new().init();
    let (mut first_loss, mut last_loss) = (f32::NAN, f32::NAN);
    for i in 0..steps {
        let (next, l) = step(net, &mut optim, &batch, device);
        net = next;
        if i == 0 {
            first_loss = l;
        }
        last_loss = l;
    }

    // Model file round trip: written from the training device, loaded by
    // the CPU runtime of the app.
    let header = ModelHeader {
        encoder_version: ENCODER_VERSION,
        shape,
        phases: Phases::ALL,
    };
    let model_bytes = write_model(&header, net.valid()).expect("model serialises");
    let model = NeuralModel::load(&model_bytes, &sha256(&model_bytes)).expect("model loads");
    let rows = batch.rows.min(8);
    let reloaded = PolicyValueNet::new(shape, &Device::flex()).load_record(
        ModuleRecord::from_bytes(Bytes::from_bytes_vec(
            net.clone()
                .into_record()
                .into_bytes()
                .expect("record")
                .to_vec(),
        ))
        .expect("record"),
    );
    let expected = outputs(&reloaded, &batch, rows);
    for r in 0..rows {
        let (logits, value) = model.forward(batch.features[r * FEATURES..][..FEATURES].to_vec());
        let row = &expected[r * ACTIONS..][..ACTIONS];
        assert!(
            logits.iter().zip(row).all(|(a, b)| (a - b).abs() <= 1e-4),
            "logits of example {r} differ after the round trip"
        );
        assert!((value - expected[rows * ACTIONS + r]).abs() <= 1e-3);
    }

    // Optimizer checkpoint round trip: a restored optimizer and model take
    // the same next step as the originals.
    let optimizer_bytes = optim.into_bytes().expect("optimizer serialises");
    let restored_optim = AdamConfig::new()
        .init()
        .from_bytes(optimizer_bytes.clone())
        .expect("optimizer loads");
    let restored_net = PolicyValueNet::new(shape, device).load_record(
        ModuleRecord::from_bytes(Bytes::from_bytes_vec(
            net.clone()
                .into_record()
                .into_bytes()
                .expect("record")
                .to_vec(),
        ))
        .expect("record"),
    );
    let (mut optim_a, mut optim_b) = (optim, restored_optim);
    let (a, _) = step(net, &mut optim_a, &batch, device);
    let (b, _) = step(restored_net, &mut optim_b, &batch, device);
    let (a, b) = (
        outputs(&a.valid(), &batch, rows),
        outputs(&b.valid(), &batch, rows),
    );
    assert!(
        a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= 1e-5),
        "restored optimizer takes another step"
    );

    Report {
        examples: batch.rows,
        first_loss,
        last_loss,
        model_bytes,
        optimizer_bytes: optimizer_bytes.to_vec(),
    }
}

fn arg<T: std::str::FromStr>(args: &[String], i: usize, default: T) -> T {
    args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// The engine commit for the manifest: `git describe`, `unknown` without
/// git.
fn engine_commit() -> String {
    std::process::Command::new("git")
        .args(["describe", "--always", "--dirty", "--abbrev=12"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

/// Value of `--name` in `args`, or `default`.
fn flag<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    match args.iter().position(|a| a == name) {
        Some(i) => args
            .get(i + 1)
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| panic!("{name} needs a number")),
        None => default,
    }
}

fn generate(args: &[String]) {
    use nn_train::selfplay::{DecisionKind, Table, TeacherProfile};
    use nn_train::shards::{Limits, Manifest, generate};
    use std::num::NonZeroUsize;

    let dir = std::path::Path::new(args.first().expect("generate OUT_DIR [flags]"));
    let deals: u64 = flag(args, "--deals", 100);
    let shard_deals: u64 = flag(args, "--shard-deals", 10);
    let seed: u64 = flag(args, "--seed", 0);
    let minutes: f64 = flag(args, "--minutes", 30.0);
    let max_mb: u64 = flag(args, "--max-mb", 1024);
    let all = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
    let cpus = NonZeroUsize::new(flag(args, "--cpus", all)).expect("--cpus > 0");
    let workers =
        NonZeroUsize::new(flag(args, "--workers", cpus.get().min(4))).expect("--workers > 0");
    assert!(minutes > 0.0 && max_mb > 0, "caps must be positive");
    let limits = Limits {
        wall_time: std::time::Duration::from_secs_f64(minutes * 60.0),
        disk_bytes: max_mb * 1024 * 1024,
        cpus,
        workers,
    };
    let backend = text_flag(args, "--search-backend", "cpu");
    let depth = flag(args, "--rest-tricks", 3u8);
    assert!((2..=4).contains(&depth), "--rest-tricks needs 2, 3 or 4");
    #[cfg(feature = "vulkan-search")]
    let experiment = (backend != "cpu").then(|| endgame::Run::start(backend, depth, cpus.get()));
    #[cfg(not(feature = "vulkan-search"))]
    assert_eq!(
        backend, "cpu",
        "enable vulkan-search for an endgame backend"
    );
    let teacher = TeacherProfile {
        name: match (backend, depth) {
            ("cpu" | "profile", _) => "strong",
            ("vulkan" | "cpu-endgame", 2) => "strong-endgame-2",
            ("vulkan" | "cpu-endgame", 3) => "strong-endgame-3",
            ("vulkan" | "cpu-endgame", 4) => "strong-endgame-4",
            _ => panic!("invalid search backend or rest-tricks"),
        },
        ..TeacherProfile::STRONG
    };
    let table_name = text_flag(args, "--table", "teacher");
    let table = Table::parse(table_name).unwrap_or_else(|| {
        panic!("unknown table {table_name}: teacher, easy-mix, rookie-mix, medium-mix or level-mix")
    });
    let manifest = Manifest::new(&engine_commit(), teacher, table, seed, deals, shard_deals);
    println!(
        "run: {} deals in {} shards, seed {seed}, profile {}, table {}, commit {}",
        deals,
        manifest.shards(),
        teacher.name,
        table.name(),
        manifest.engine_commit
    );
    println!(
        "budget: {} CPUs = {} workers x {} analysis threads (max), caps {minutes} min, {max_mb} MiB",
        cpus,
        workers,
        limits.threads()
    );
    let report = generate(dir, &manifest, teacher, limits).expect("generate");
    #[cfg(feature = "vulkan-search")]
    if let Some(experiment) = experiment {
        experiment.report();
    }
    let secs = report.elapsed.as_secs_f64();
    println!(
        "shards: {} skipped, {} written; stop: {:?}",
        report.skipped, report.written, report.stop
    );
    println!(
        "written: {} games, {} records, {} bytes, {:.0} s",
        report.deals, report.records, report.bytes, secs
    );
    if report.deals > 0 {
        let games_per_hour = report.deals as f64 / secs * 3600.0;
        let bytes_per_game = report.bytes as f64 / report.deals as f64;
        println!("games/hour: {games_per_hour:.0}");
        println!("bytes/game: {bytes_per_game:.0}");
        for kind in DecisionKind::ALL {
            let n = report.kinds.get(&kind).copied().unwrap_or(0);
            println!(
                "decisions {kind:?}: {n} ({:.2} per game)",
                n as f64 / report.deals as f64
            );
        }
        println!("incomplete teacher analyses: {}", report.incomplete);
        if table != Table::Teacher {
            for (roles, n) in &report.roles {
                println!(
                    "card-play roles {} (declarer|defenders, T teacher, R Rookie, E Easy, M Medium): {n} deals",
                    roles.label()
                );
            }
            println!(
                "weaker players' actions that differ from the teacher's label: {}",
                report.overruled
            );
        }
        println!(
            "projection for 10,000 games: {:.1} h, {:.0} MB",
            10_000.0 / games_per_hour,
            bytes_per_game * 10_000.0 / 1e6
        );
    }
}

/// Value of the text flag `--name` in `args`, or `default`.
fn text_flag<'a>(args: &'a [String], name: &str, default: &'a str) -> &'a str {
    match args.iter().position(|a| a == name) {
        Some(i) => args
            .get(i + 1)
            .unwrap_or_else(|| panic!("{name} needs a value")),
        None => default,
    }
}

fn device(kind: &str) -> Device {
    match kind {
        "cpu" => Device::flex(),
        #[cfg(feature = "vulkan")]
        "vulkan" => Device::vulkan(burn::tensor::DeviceKind::DefaultDevice),
        #[cfg(not(feature = "vulkan"))]
        "vulkan" => panic!("vulkan needs a build with --features vulkan"),
        other => panic!("unknown device {other}: cpu or vulkan"),
    }
}

fn train(args: &[String]) {
    use nn_train::resident::ResidentRows;
    use nn_train::train::{
        Hyper, PHASES, Plateau, Recipe, TRAIN_FORMAT, TrainLimits, TrainManifest, TrainStop,
        evaluate_with, hex_hash, load_with, read_model, train_with,
    };
    let out = std::path::PathBuf::from(args.first().expect("train OUT_DIR DATA_DIR..."));
    let dirs: Vec<std::path::PathBuf> = args[1..]
        .iter()
        .take_while(|a| !a.starts_with("--"))
        .map(std::path::PathBuf::from)
        .collect();
    assert!(!dirs.is_empty(), "train needs at least one DATA_DIR");
    let d = Hyper::default();
    let hyper = Hyper {
        hidden: flag(args, "--hidden", d.hidden),
        layers: flag(args, "--layers", d.layers),
        batch: flag(args, "--batch", d.batch),
        learning_rate: flag(args, "--lr", d.learning_rate),
        epochs: flag(args, "--epochs", d.epochs),
        value_weight: flag(args, "--value-weight", d.value_weight),
        value_mix: flag(args, "--value-mix", d.value_mix),
        soft_weight: flag(args, "--soft-weight", d.soft_weight),
        soft_temperature: flag(args, "--soft-temperature", d.soft_temperature),
        score_weight: flag(args, "--score-weight", d.score_weight),
        win_weight: flag(args, "--win-weight", d.win_weight),
        belief_weight: flag(args, "--belief-weight", d.belief_weight),
        seed: flag(args, "--seed", d.seed),
        plateau: args.iter().any(|a| a == "--patience").then(|| Plateau {
            patience: flag(args, "--patience", 2),
            decay: flag(args, "--lr-decay", 0.1),
            decays: flag(args, "--lr-decays", 1),
        }),
    };
    let minutes: f64 = flag(args, "--minutes", 60.0);
    let kind = text_flag(args, "--device", "cpu");
    // Read now: the tree may change while a long run trains.
    let commit = engine_commit();
    let start = Instant::now();
    let beliefs = hyper.belief_weight > 0.0;
    let cache = text_flag(args, "--cache", "");
    let (data, from) = if cache.is_empty() {
        let data = load_with(&dirs, &hyper, beliefs).expect("load training data");
        (data, "runs")
    } else {
        use nn_train::cache::{CacheUse, load_cached};
        let (data, used) = load_cached(&dirs, &hyper, beliefs, std::path::Path::new(cache))
            .expect("load training data");
        (
            data,
            match used {
                CacheUse::Hit => "cache",
                CacheUse::Written => "runs, cache written",
            },
        )
    };
    println!(
        "examples: {} train, {} validation, {} test ({:.0} s to load from {from})",
        data.train.rows(),
        data.validation.rows(),
        data.test.rows(),
        start.elapsed().as_secs_f64()
    );
    let autodiff = device(kind).autodiff();
    // Optional: the training split's feature rows in device memory, if
    // they fit the budget in GB; otherwise batches come from RAM.
    let budget: f64 = flag(args, "--data-on-device", 0.0);
    let inputs = NetShape::current(hyper.hidden).inputs;
    let need = ResidentRows::bytes(&data.train.features, inputs);
    let resident = if budget <= 0.0 {
        None
    } else if need as f64 > budget * 1e9 {
        println!(
            "data on device: {:.1} GB needed, budget {budget:.1} GB; batches come from RAM",
            need as f64 / 1e9
        );
        None
    } else {
        let began = Instant::now();
        let r = ResidentRows::upload(&data.train.features, inputs, &autodiff);
        println!(
            "data on device: {:.1} GB of training rows uploaded ({:.0} s)",
            need as f64 / 1e9,
            began.elapsed().as_secs_f64()
        );
        Some(r)
    };
    let recipe = Recipe {
        train_format: TRAIN_FORMAT,
        encoder_version: ENCODER_VERSION,
        phases: PHASES.bits(),
        data: data.sources.clone(),
        hyper: hyper.clone(),
    };
    println!("hyperparameters: {hyper:?}");
    let limits = TrainLimits {
        wall_time: std::time::Duration::from_secs_f64(minutes * 60.0),
    };
    let trained = train_with(
        &out,
        &recipe,
        &data,
        resident.as_ref(),
        &autodiff,
        limits,
        |log| {
            println!(
                "epoch {:3}: lr {:.1e}, train loss {:.4}, validation loss {:.4}, policy {:.4}, \
                 agreement {:.2} %, value RMSE {:.1}, score RMSE {:.1}, win loss {:.4}, \
                 score agreement {:.2} %, belief {:.3} bits, {:.0} s",
                log.epoch,
                log.learning_rate,
                log.train_loss,
                log.validation.loss,
                log.validation.policy_loss,
                100.0 * log.validation.agreement,
                log.validation.value_rmse,
                log.validation.score_rmse,
                log.validation.win_loss,
                100.0 * log.validation.score_agreement,
                log.validation.belief_bits,
                log.seconds
            );
            true
        },
    )
    .expect("train");
    let (net, head) = read_model(&trained.model, trained.belief.as_deref()).expect("read model");
    let metrics = |examples| evaluate_with(&net, head.as_ref(), examples, &hyper);
    let manifest = TrainManifest {
        recipe,
        engine_commit: commit,
        device: kind.to_string(),
        coverage: data.coverage.clone(),
        logs: trained.state.logs.clone(),
        best_epoch: trained.state.best_epoch,
        train: metrics(&data.train),
        validation: metrics(&data.validation),
        test: metrics(&data.test),
        model_bytes: trained.model.len(),
        model_sha256: hex_hash(&trained.model),
        stop: match trained.stop {
            TrainStop::Finished => "finished",
            TrainStop::WallTime => "wall-time",
            TrainStop::Stopped => "stopped",
            TrainStop::Plateau => "plateau",
        }
        .to_string(),
    };
    std::fs::write(out.join("model.gcnn"), &trained.model).expect("write model");
    if let Some(belief) = &trained.belief {
        std::fs::write(out.join("belief.bpk"), belief).expect("write belief head");
    }
    let mut json = serde_json::to_vec_pretty(&manifest).expect("manifest serialises");
    json.push(b'\n');
    std::fs::write(out.join("train.json"), json).expect("write train.json");
    println!(
        "best epoch {} of {}; stop: {}",
        manifest.best_epoch,
        manifest.logs.len(),
        manifest.stop
    );
    for (name, m) in [
        ("train", manifest.train),
        ("validation", manifest.validation),
        ("test", manifest.test),
    ] {
        println!(
            "{name}: {} examples, loss {:.4}, policy {:.4}, agreement {:.2} %, value RMSE {:.1}, \
             score RMSE {:.1}, win loss {:.4}, score agreement {:.2} %",
            m.examples,
            m.loss,
            m.policy_loss,
            100.0 * m.agreement,
            m.value_rmse,
            m.score_rmse,
            m.win_loss,
            100.0 * m.score_agreement
        );
    }
    println!(
        "wrote {} ({} bytes, sha256 {}), train.json; {:.0} s",
        out.join("model.gcnn").display(),
        manifest.model_bytes,
        manifest.model_sha256,
        start.elapsed().as_secs_f64()
    );
}

fn evaluate(args: &[String]) {
    use nn_train::train::{Hyper, evaluate_model, load};
    let model = std::fs::read(args.first().expect("evaluate MODEL DATA_DIR...")).expect("model");
    let dirs: Vec<std::path::PathBuf> = args[1..].iter().map(std::path::PathBuf::from).collect();
    assert!(!dirs.is_empty(), "evaluate needs at least one DATA_DIR");
    let hyper = Hyper::default();
    let data = load(&dirs, &hyper).expect("load data");
    let report = |name: &str, examples: &nn_train::train::Examples| {
        let m = evaluate_model(&model, examples, &hyper).expect("evaluate");
        println!(
            "{name}: {} examples, policy {:.4}, agreement {:.2} %, value RMSE {:.1}, \
             score RMSE {:.1}, win loss {:.4}, score agreement {:.2} %",
            m.examples,
            m.policy_loss,
            100.0 * m.agreement,
            m.value_rmse,
            m.score_rmse,
            m.win_loss,
            100.0 * m.score_agreement
        );
    };
    for (name, examples) in [
        ("train", &data.train),
        ("validation", &data.validation),
        ("test", &data.test),
    ] {
        report(name, examples);
    }
    // Origin of the test examples: who played the acting seat and the
    // card-play roles (labels are always the teacher's).
    let test = &data.test;
    if test.roles.iter().any(|r| r.weak() > 0) {
        use nn_train::selfplay::Player;
        for player in Player::ALL {
            let part = test.select(|r| test.player[r] == player);
            if part.rows() > 0 {
                report(&format!("  test, {player:?} seat"), &part);
            }
        }
        let lineups: std::collections::BTreeSet<_> = test.roles.iter().copied().collect();
        for roles in lineups {
            let part = test.select(|r| test.roles[r] == roles);
            if part.rows() > 0 {
                report(&format!("  test, roles {}", roles.label()), &part);
            }
        }
    }
}

/// Manifest of a self-play run, written as `rl.json`.
#[derive(serde::Serialize)]
struct RlManifest<'a> {
    recipe: &'a nn_train::rl::RlRecipe,
    trainer_commit: String,
    logs: &'a [nn_train::rl::IterationLog],
    snapshots: &'a [nn_train::rl::Snapshot],
    stop: nn_train::rl::RlStop,
    model_bytes: usize,
    model_sha256: String,
}

fn rl(args: &[String]) {
    use nn_train::rl::{RlHyper, RlLimits, parse_pool, recipe, run};
    use std::num::NonZeroUsize;

    let out = std::path::PathBuf::from(args.first().expect("rl OUT_DIR --start MODEL [flags]"));
    let start_path = text_flag(args, "--start", "assets/models/card-play.gcnn");
    let start = std::fs::read(start_path).unwrap_or_else(|e| panic!("{start_path}: {e}"));
    let d = RlHyper::default();
    let hyper = RlHyper {
        games: flag(args, "--games", d.games),
        batch: flag(args, "--batch", d.batch),
        passes: flag(args, "--passes", d.passes),
        learning_rate: flag(args, "--lr", d.learning_rate),
        clip: flag(args, "--clip", d.clip),
        value_weight: flag(args, "--value-weight", d.value_weight),
        score_weight: flag(args, "--score-weight", d.score_weight),
        win_weight: flag(args, "--win-weight", d.win_weight),
        entropy_weight: flag(args, "--entropy-weight", d.entropy_weight),
        reference_weight: flag(args, "--reference-weight", d.reference_weight),
        snapshot_every: flag(args, "--snapshot-every", d.snapshot_every),
        pool_snapshots: flag(args, "--pool-snapshots", d.pool_snapshots),
        pool: match args.iter().position(|a| a == "--pool") {
            Some(i) => parse_pool(args.get(i + 1).expect("--pool needs a value"))
                .unwrap_or_else(|e| panic!("--pool: {e}")),
            None => d.pool,
        },
        seed: flag(args, "--seed", d.seed),
    };
    let all = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
    let cpus = NonZeroUsize::new(flag(args, "--cpus", all)).expect("--cpus > 0");
    let limits = RlLimits {
        wall_time: std::time::Duration::from_secs_f64(flag(args, "--minutes", 30.0) * 60.0),
        iterations: flag(args, "--iterations", usize::MAX),
        cpus,
        workers: NonZeroUsize::new(flag(args, "--workers", cpus.get())).expect("--workers > 0"),
    };
    let recipe = recipe(&start, hyper).expect("start model");
    // Read now: the tree may change while a long run trains.
    let commit = engine_commit();
    let stop_file = out.join("STOP");
    std::fs::create_dir_all(&out).expect("create OUT_DIR");
    let _ = std::fs::remove_file(&stop_file);
    println!("recipe: {recipe:?}");
    println!(
        "budget: {} CPUs, {} parallel games x {} analysis threads; stop with `touch {}`",
        limits.cpus,
        limits.workers,
        (limits.cpus.get() / limits.workers.get()).max(1),
        stop_file.display()
    );
    let result = run(
        &out,
        &recipe,
        &start,
        &Device::flex().autodiff(),
        limits,
        |log| {
            let pool: Vec<String> = log
                .pool
                .iter()
                .map(|(k, l)| format!("{} {:+.1} ({})", k.name(), l.difference(), l.games))
                .collect();
            println!(
                "iteration {:5}: {} decisions, reward {:+.1}, vs pool [{}], value RMSE {:.1}, \
                 entropy {:.3}, KL(ref) {:.4}, clipped {:.1} %, play {:.0} s, update {:.0} s",
                log.iteration,
                log.decisions,
                log.mean_reward,
                pool.join(", "),
                log.value_rmse,
                log.entropy,
                log.reference_kl,
                100.0 * log.clipped,
                log.play_seconds,
                log.update_seconds
            );
            !stop_file.exists()
        },
    )
    .expect("self-play run");
    let manifest = RlManifest {
        recipe: &recipe,
        trainer_commit: commit,
        logs: &result.state.logs,
        snapshots: &result.state.snapshots,
        stop: result.stop,
        model_bytes: result.model.len(),
        model_sha256: nn_train::train::hex_hash(&result.model),
    };
    std::fs::write(out.join("model.gcnn"), &result.model).expect("write model");
    let mut json = serde_json::to_vec_pretty(&manifest).expect("manifest");
    json.push(b'\n');
    std::fs::write(out.join("rl.json"), json).expect("write rl.json");
    println!(
        "stopped ({:?}) after {} iterations; wrote {} and rl.json",
        result.stop,
        result.state.logs.len(),
        out.join("model.gcnn").display()
    );
}

fn probe(args: &[String]) {
    use nn_train::probe::{ProbeConfig, probe};
    use nn_train::selfplay::Split;
    let model = std::fs::read(args.first().expect("probe MODEL DATA_DIR...")).expect("read model");
    let dirs: Vec<std::path::PathBuf> = args[1..]
        .iter()
        .take_while(|a| !a.starts_with("--"))
        .map(std::path::PathBuf::from)
        .collect();
    assert!(!dirs.is_empty(), "probe needs at least one DATA_DIR");
    let belief = match text_flag(args, "--belief", "") {
        "" => None,
        path => Some(std::fs::read(path).expect("read belief head")),
    };
    let (net, head) =
        nn_train::train::read_model(&model, belief.as_deref()).expect("read model and head");
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let config = ProbeConfig {
        rows: flag(args, "--rows", 20_000),
        samples: flag(args, "--samples", 256),
        seed: flag(args, "--seed", 0),
        threads: flag(args, "--threads", threads),
        split: Split::Validation,
    };
    let start = Instant::now();
    let r = probe(&net, head.as_ref(), &dirs, config).expect("probe");
    println!(
        "{} rows from {} validation deals, {} inputs ({} views the sampler rejects), {:.0} s",
        r.rows,
        r.deals,
        r.inputs,
        r.inconsistent,
        start.elapsed().as_secs_f64()
    );
    println!("value residual ICC (rows, groups, RMSE, ICC, effective rows):");
    for (name, i) in [
        ("final points by (deal, seat)", r.label.final_by_seat),
        ("final points by deal", r.label.final_by_deal),
        ("chosen option by (deal, seat)", r.label.option_by_seat),
    ] {
        println!(
            "  {name:32} {:7} {:6} {:6.1} {:6.3} {:9.0}",
            i.rows, i.groups, i.rmse, i.icc, i.effective_rows
        );
    }
    println!(
        "policy loss {:.4}, agreement {:.2} %; by completed tricks: {}",
        r.base.policy_loss,
        100.0 * r.base.agreement,
        r.base
            .loss_by_trick
            .iter()
            .map(|l| format!("{l:.3}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("zeroed block: Δ policy loss, Δ agreement (points), Δ loss by completed tricks");
    let mut blocks = r.blocks.clone();
    blocks.sort_by(|a, b| b.policy_loss_delta.total_cmp(&a.policy_loss_delta));
    for b in &blocks {
        println!(
            "  {:16} {:+.4} {:+6.2}  {}",
            b.block,
            b.policy_loss_delta,
            100.0 * b.agreement_delta,
            b.loss_delta_by_trick
                .iter()
                .map(|l| format!("{l:+.3}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!("hidden-card bits per card: tricks, cards, constraints, + bidding, head");
    for b in &r.belief {
        let label = if b.tricks == 10 {
            "all".to_string()
        } else {
            b.tricks.to_string()
        };
        println!(
            "  {label:>3} {:7} {:.3} {:.3} {:.3}",
            b.hidden, b.uniform_bits, b.evidence_bits, b.head_bits
        );
    }
    if let Some(path) = args
        .iter()
        .position(|a| a == "--json")
        .and_then(|i| args.get(i + 1))
    {
        let mut json = serde_json::to_vec_pretty(&r).expect("report serialises");
        json.push(b'\n');
        std::fs::write(path, json).expect("write report");
        println!("wrote {path}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("generate") => generate(&args[1..]),
        Some("train") => train(&args[1..]),
        Some("evaluate") => evaluate(&args[1..]),
        Some("rl") => rl(&args[1..]),
        Some("probe") => probe(&args[1..]),
        Some("verify") => {
            let dir = args.get(1).expect("verify DIR");
            match nn_train::shards::verify(std::path::Path::new(dir)) {
                Ok(v) => println!(
                    "{} complete shards ({} records), {} incomplete",
                    v.complete, v.records, v.incomplete
                ),
                Err(e) => {
                    eprintln!("{dir}: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("smoke") => {
            let kind = args.get(1).map_or("cpu", String::as_str);
            let device = device(kind);
            // Adapter names exist only with a GPU backend compiled in.
            #[cfg(feature = "vulkan")]
            let name = device
                .identity()
                .map_or_else(|| kind.to_string(), |i| i.name);
            #[cfg(not(feature = "vulkan"))]
            let name = kind.to_string();
            let (games, steps) = (arg(&args, 2, 200u64), arg(&args, 3, 300usize));
            let start = Instant::now();
            let report = smoke(&device.autodiff(), games, steps);
            println!("device: {name}");
            println!("examples: {} from {games} games", report.examples);
            println!(
                "loss: {:.4} -> {:.4} after {steps} steps",
                report.first_loss, report.last_loss
            );
            println!(
                "model file: {} bytes, optimizer checkpoint: {} bytes",
                report.model_bytes.len(),
                report.optimizer_bytes.len()
            );
            println!("round trips: model file and optimizer ok");
            println!("time: {:.1} s", start.elapsed().as_secs_f64());
            if let Some(dir) = args.get(4) {
                std::fs::create_dir_all(dir).expect("create output directory");
                let path = std::path::Path::new(dir).join("model.gcnn");
                std::fs::write(&path, &report.model_bytes).expect("write model");
                println!("wrote {}", path.display());
                let path = std::path::Path::new(dir).join("optimizer.bpk");
                std::fs::write(&path, &report.optimizer_bytes).expect("write optimizer");
                println!("wrote {}", path.display());
            }
        }
        Some("fixture") => {
            let seed: u64 = arg(&args, 1, 0);
            let out = args.get(2).expect("fixture SEED OUT_FILE");
            let bytes = skat_nn::fixture_model(seed, Phases::ALL);
            std::fs::write(out, &bytes).expect("write fixture");
            let hash: String = sha256(&bytes).iter().map(|b| format!("{b:02x}")).collect();
            println!("wrote {out} ({} bytes, sha256 {hash})", bytes.len());
        }
        _ => {
            eprintln!("usage: nn-train smoke [cpu|vulkan] [GAMES] [STEPS] [OUT_DIR]");
            eprintln!("       nn-train fixture SEED OUT_FILE");
            eprintln!(
                "       nn-train generate OUT_DIR [--deals N] [--shard-deals N] [--seed N] \
                 [--minutes N] [--max-mb N] [--cpus N] [--workers N] [--search-backend cpu|profile|cpu-endgame|vulkan] [--rest-tricks 2|3|4] \
                 [--table teacher|easy-mix|rookie-mix|medium-mix|level-mix]"
            );
            eprintln!("       nn-train verify DIR");
            eprintln!("       nn-train evaluate MODEL DATA_DIR...");
            eprintln!(
                "       nn-train probe MODEL DATA_DIR... [--belief FILE] [--rows N] [--samples N] \
                 [--seed N] [--threads N] [--json FILE]"
            );
            eprintln!(
                "       nn-train rl OUT_DIR [--start MODEL] [--games N] [--batch N] [--passes N] \
                 [--lr X] [--clip X] [--value-weight X] [--score-weight X] [--win-weight X] \
                 [--entropy-weight X] [--reference-weight X] [--snapshot-every N] \
                 [--pool-snapshots N] [--pool easy=W,medium=W,strong=W,ref=W,snap=W] [--seed N] \
                 [--minutes N] [--iterations N] [--cpus N] [--workers N]"
            );
            eprintln!(
                "       nn-train train OUT_DIR DATA_DIR... [--hidden N] [--batch N] [--lr X] \
                 [--epochs N] [--value-weight X] [--value-mix X] [--soft-weight X] \
                 [--soft-temperature X] [--score-weight X] [--win-weight X] [--belief-weight X] \
                 [--seed N] [--minutes N] [--device cpu|vulkan] \
                 [--patience N [--lr-decay X] [--lr-decays N]] [--cache DIR] \
                 [--data-on-device GB]"
            );
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The training stack on the CPU: the loss falls and both checkpoints
    /// round-trip (GPU training is run by hand, `docs/testing.md`).
    #[test]
    fn cpu_smoke_training_round_trips() {
        let report = smoke(&Device::flex().autodiff(), 4, 30);
        assert!(report.examples > 20);
        assert!(
            report.last_loss < report.first_loss,
            "{} {}",
            report.first_loss,
            report.last_loss
        );
        assert!(!report.optimizer_bytes.is_empty());
    }
}
