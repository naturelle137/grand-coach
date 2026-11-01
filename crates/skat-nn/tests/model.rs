// SPDX-License-Identifier: EUPL-1.2

//! Model loading and inference safety (#69): hash, version and shape
//! checks, corrupt weights, the legal mask, non-finite outputs, uncovered
//! phases and reproducibility.

use burn::tensor::Device;
use skat_core::{Action, Game, Phase, PlayerView, RuleConfig, Seat};
use skat_nn::{
    ACTIONS, ENCODER_VERSION, FEATURES, FIXTURE_HIDDEN, Fallback, ModelError, ModelHeader,
    NetShape, NeuralModel, Phases, PolicyValueNet, action_index, fixture_model, read_header,
    sha256, write_model,
};

const RULES: RuleConfig = RuleConfig { kontra_re: true };

fn load(bytes: &[u8]) -> Result<NeuralModel, ModelError> {
    NeuralModel::load(bytes, &sha256(bytes))
}

/// A model whose parameters are given by their position in
/// [`PolicyValueNet::from_weights`] order.
fn model_with(phases: Phases, weight: impl Fn(usize) -> f32) -> NeuralModel {
    let shape = NetShape::current(FIXTURE_HIDDEN);
    let mut k = 0;
    let net = PolicyValueNet::from_weights(shape, &Device::flex(), || {
        k += 1;
        weight(k - 1)
    });
    let header = ModelHeader {
        encoder_version: ENCODER_VERSION,
        shape,
        phases,
    };
    load(&write_model(&header, net).unwrap()).unwrap()
}

/// Offset of the policy bias in [`PolicyValueNet::from_weights`] order.
fn policy_bias() -> usize {
    let h = FIXTURE_HIDDEN;
    FEATURES * h + h + h * h + h + h * ACTIONS
}

/// Views of every seat to move along seeded games: bidding up to 24, then
/// the last legal action (a Hand game, the highest announcement, the last
/// card, Kontra and Re where offered).
fn decision_views(games: u64) -> Vec<PlayerView> {
    let mut out = Vec::new();
    for seed in 0..games {
        let mut game = Game::new(seed, RULES);
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            let legal = &view.legal_actions;
            let action = match view.phase {
                Phase::Reizen => legal
                    .iter()
                    .copied()
                    .find(|a| matches!(a, Action::Bid(v) if *v <= 24) || *a == Action::Hold)
                    .unwrap_or(Action::Pass),
                _ => *legal.last().unwrap(),
            };
            out.push(view);
            game.apply(action).unwrap();
        }
    }
    out
}

#[test]
fn fixture_loads_and_reports_its_header() {
    let bytes = fixture_model(1, Phases::ALL);
    let model = load(&bytes).unwrap();
    assert_eq!(model.sha256(), sha256(&bytes));
    assert_eq!(model.header().encoder_version, ENCODER_VERSION);
    assert_eq!(model.header().shape, NetShape::current(FIXTURE_HIDDEN));
    assert_eq!(model.header().phases, Phases::ALL);
    assert_eq!(read_header(&bytes).unwrap(), *model.header());
}

#[test]
fn fixture_bytes_are_reproducible() {
    assert_eq!(fixture_model(3, Phases::ALL), fixture_model(3, Phases::ALL));
    assert_ne!(fixture_model(3, Phases::ALL), fixture_model(4, Phases::ALL));
}

#[test]
fn wrong_hash_is_rejected() {
    let bytes = fixture_model(1, Phases::ALL);
    let mut other = sha256(&bytes);
    other[0] ^= 1;
    assert_eq!(
        NeuralModel::load(&bytes, &other).unwrap_err(),
        ModelError::HashMismatch
    );
    let mut flipped = bytes.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 0xff;
    assert_eq!(
        NeuralModel::load(&flipped, &sha256(&bytes)).unwrap_err(),
        ModelError::HashMismatch
    );
}

#[test]
fn incompatible_headers_are_rejected() {
    let bytes = fixture_model(1, Phases::ALL);
    let patched = |at: usize, value: &[u8]| {
        let mut b = bytes.clone();
        b[at..at + value.len()].copy_from_slice(value);
        load(&b).unwrap_err()
    };
    assert_eq!(load(&bytes[..10]).unwrap_err(), ModelError::Truncated);
    assert_eq!(patched(0, b"XXXX"), ModelError::Magic);
    assert_eq!(
        patched(4, &2u16.to_le_bytes()),
        ModelError::Format { found: 2 }
    );
    let next = ENCODER_VERSION + 1;
    assert_eq!(
        patched(6, &next.to_le_bytes()),
        ModelError::Encoder { found: next }
    );
    assert!(matches!(
        patched(8, &(FEATURES as u32 + 1).to_le_bytes()),
        ModelError::Shape { .. }
    ));
    assert!(matches!(
        patched(12, &0u32.to_le_bytes()),
        ModelError::Shape { hidden: 0, .. }
    ));
    assert!(matches!(
        patched(16, &1u32.to_le_bytes()),
        ModelError::Shape { actions: 1, .. }
    ));
    assert_eq!(patched(20, &[0]), ModelError::Phases { bits: 0 });
    assert_eq!(patched(20, &[0x40]), ModelError::Phases { bits: 0x40 });
}

#[test]
fn corrupt_or_mismatched_weights_are_rejected() {
    let bytes = fixture_model(1, Phases::ALL);
    // Weights cut short.
    let cut = &bytes[..bytes.len() - 100];
    assert!(matches!(load(cut), Err(ModelError::Weights(_))));
    // No weights at all.
    assert!(matches!(load(&bytes[..24]), Err(ModelError::Weights(_))));
    // Header claims another hidden width than the weights have.
    let mut wide = bytes.clone();
    wide[12..16].copy_from_slice(&(FIXTURE_HIDDEN as u32 * 2).to_le_bytes());
    assert!(matches!(load(&wide), Err(ModelError::Weights(_))));
}

#[test]
fn chosen_actions_are_legal_and_the_policy_is_normalised() {
    let model = load(&fixture_model(2, Phases::ALL)).unwrap();
    let views = decision_views(30);
    let mut phases = std::collections::BTreeSet::new();
    for view in &views {
        let e = model.evaluate(view).unwrap();
        assert!(view.legal_actions.contains(&e.action));
        assert_eq!(e.policy.len(), view.legal_actions.len());
        let total: f32 = e.policy.iter().map(|&(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-4, "{total}");
        assert!(e.value.is_finite());
        phases.insert(format!("{:?}", view.phase));
    }
    assert_eq!(phases.len(), 5, "{phases:?}");
}

#[test]
fn illegal_actions_are_masked_before_normalisation() {
    // Huge logits for every card and zero elsewhere: outside card play all
    // cards are illegal and must get no probability.
    let start = policy_bias();
    let card = action_index(Action::Play(skat_core::Card::from_index(0).unwrap())).unwrap();
    let model = model_with(Phases::ALL, |k| {
        if (start + card..start + card + 32).contains(&k) {
            1000.0
        } else {
            0.0
        }
    });
    for view in decision_views(10) {
        let e = model.evaluate(&view).unwrap();
        assert!(view.legal_actions.contains(&e.action), "{e:?}");
        let total: f32 = e.policy.iter().map(|&(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-4);
        if view.phase != Phase::Spiel {
            // Uniform over the legal actions: ties go to the first one.
            assert_eq!(e.action, view.legal_actions[0]);
        }
    }
}

#[test]
fn non_finite_outputs_fall_back() {
    let view = decision_views(1).remove(0);
    for bad in [f32::NAN, f32::INFINITY] {
        let model = model_with(Phases::ALL, |_| bad);
        assert_eq!(model.evaluate(&view).unwrap_err(), Fallback::NonFinite);
    }
    // Only the value head broken.
    let model = model_with(Phases::ALL, |k| {
        if k >= policy_bias() + ACTIONS {
            f32::NAN
        } else {
            0.0
        }
    });
    assert_eq!(model.evaluate(&view).unwrap_err(), Fallback::NonFinite);
}

#[test]
fn uncovered_phases_and_other_seats_fall_back() {
    let model = load(&fixture_model(1, Phases::SPIEL)).unwrap();
    let game = Game::new(5, RULES);
    let seat = game.current_seat().unwrap();
    assert_eq!(
        model.evaluate(&game.view(seat)).unwrap_err(),
        Fallback::UnsupportedPhase(Phase::Reizen)
    );
    let other = Seat::ALL.into_iter().find(|&s| s != seat).unwrap();
    assert_eq!(
        model.evaluate(&game.view(other)).unwrap_err(),
        Fallback::NoDecision
    );
    let spiel = decision_views(3)
        .into_iter()
        .find(|v| v.phase == Phase::Spiel)
        .unwrap();
    assert!(model.evaluate(&spiel).is_ok());
}

#[test]
fn fixed_model_and_input_reproduce_the_decision() {
    let bytes = fixture_model(7, Phases::ALL);
    let (a, b) = (load(&bytes).unwrap(), load(&bytes).unwrap());
    for view in decision_views(5) {
        let first = a.evaluate(&view).unwrap();
        for e in [a.evaluate(&view).unwrap(), b.evaluate(&view).unwrap()] {
            assert_eq!(e.action, first.action);
            assert_eq!(e.value.to_bits(), first.value.to_bits());
            let bits = |p: &[(Action, f32)]| p.iter().map(|x| x.1.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&e.policy), bits(&first.policy));
        }
    }
}
