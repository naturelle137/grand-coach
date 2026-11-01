// SPDX-License-Identifier: EUPL-1.2

//! Training tool for the learned engines (D-07, D-11). Not part of the
//! app; it trains the network definition of `skat-nn` with Burn's autodiff
//! on the CPU (Flex) or on a GPU via wgpu/Vulkan.
//!
//! ```sh
//! cargo run --locked --release -p nn-train -- smoke [cpu|vulkan] [GAMES] [STEPS] [OUT_DIR]
//! cargo run --locked --release -p nn-train -- fixture SEED OUT_FILE
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
//! arena (`calibrate -- nn:<file>`).

use std::time::Instant;

use burn::module::Module;
use burn::optim::{AdamConfig, GradientsParams, ModuleOptimizer};
use burn::store::ModuleRecord;
use burn::tensor::activation::log_softmax;
use burn::tensor::{Bytes, Device, DeviceKind, Tensor, TensorData};
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("smoke") => {
            let kind = args.get(1).map_or("cpu", String::as_str);
            let device = match kind {
                "cpu" => Device::flex(),
                "vulkan" => Device::vulkan(DeviceKind::DefaultDevice),
                other => panic!("unknown device {other}: cpu or vulkan"),
            };
            let name = device
                .identity()
                .map_or_else(|| kind.to_string(), |i| i.name);
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
