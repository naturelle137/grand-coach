// SPDX-License-Identifier: EUPL-1.2

//! Diagnostics of the dependencies between the decisions of one game
//! (`nn-train probe`), on a deal-sampled part of one split (validation
//! for a trained model).
//!
//! - **Label correlation**: every card-play row of a seat in one game
//!   shares the final Seeger-Fabian points. The intra-class correlation
//!   (ICC) of the model's value residuals within a (deal, seat) group and
//!   within a deal measures how much of the target is per-game noise
//!   rather than per-decision signal, and gives the effective number of
//!   independent examples. The same for the residual against the teacher's
//!   expected points of the chosen card (a per-decision target).
//! - **Block importance**: each encoder block ([`skat_nn::BLOCKS`]) zeroed
//!   in turn; the change of policy loss and agreement, overall and by the
//!   number of completed tricks, shows which parts of the history the
//!   model uses.
//! - **Hidden-card information**: for every card the seat cannot see, the
//!   cross-entropy in bits of its true place (next seat, seat after, Skat,
//!   from a replay of the deal) under three beliefs: the hard constraints
//!   alone (uniform samples of `skat_analysis::sampling`, as Classic),
//!   the same samples weighted by the bidding evidence
//!   (`skat_analysis::evidence`, as Classic), and the learned belief head
//!   if given. The gap between the first two and the head is the evidence
//!   about hidden hands that earlier decisions carry and that Classic does
//!   not use.
//!
//! Final points, teacher values and the replayed deal are targets of the
//! diagnostics only; the model sees nothing but the encoded view.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

use burn::nn::Linear;
use burn::tensor::Device;
use burn::tensor::activation::log_softmax;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::Serialize;
use skat_ai::Status;
use skat_analysis::evidence;
use skat_analysis::sampling::HiddenCards;
use skat_core::{Phase, PlayerView, Seat};
use skat_nn::{ACTIONS, BLOCKS, PolicyValueNet};

use crate::selfplay::{DecisionRecord, Split};
use crate::shards::{read_manifest, read_shard};
use crate::train::{
    BELIEF_PLACES, Examples, Hyper, NOT_HIDDEN, SplitMix, belief_log_probs, belief_targets,
};

/// Rows per forward pass.
const BATCH: usize = 4096;

/// Settings of a probe.
#[derive(Clone, Copy, Debug)]
pub struct ProbeConfig {
    /// Validation rows to use at least (whole deals are taken).
    pub rows: usize,
    /// Sampled worlds per row for the Classic beliefs (0 skips them).
    pub samples: usize,
    /// Seed of the deal choice and the samples.
    pub seed: u64,
    /// Worker threads for the sampling.
    pub threads: usize,
    /// Split to probe (validation for a trained model).
    pub split: Split,
}

/// Intra-class correlation of a residual.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Icc {
    /// Rows.
    pub rows: usize,
    /// Groups.
    pub groups: usize,
    /// Root mean square of the residual, in points.
    pub rmse: f64,
    /// ICC (one-way random effects, unbalanced groups).
    pub icc: f64,
    /// Rows / (1 + (mean group size − 1) · ICC).
    pub effective_rows: f64,
}

/// Correlation of the value residuals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct LabelCorrelation {
    /// Residual against the final points, grouped by (deal, seat).
    pub final_by_seat: Icc,
    /// … grouped by deal.
    pub final_by_deal: Icc,
    /// Residual against the teacher's expected points of the chosen card
    /// (rows with a complete analysis), grouped by (deal, seat).
    pub option_by_seat: Icc,
}

/// Policy quality of one input variant.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PolicyScore {
    /// Mean policy cross-entropy.
    pub policy_loss: f64,
    /// Share of rows where the most probable card is the teacher's.
    pub agreement: f64,
    /// Policy loss by completed tricks (index 0..10).
    pub loss_by_trick: Vec<f64>,
}

/// The effect of zeroing one encoder block.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BlockEffect {
    /// Block name ([`skat_nn::BLOCKS`]).
    pub block: String,
    /// Policy loss with the block zeroed minus without.
    pub policy_loss_delta: f64,
    /// Agreement with the block zeroed minus without.
    pub agreement_delta: f64,
    /// Policy loss delta by completed tricks.
    pub loss_delta_by_trick: Vec<f64>,
}

/// Hidden-card cross-entropies after a number of completed tricks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct BeliefRow {
    /// Completed tricks.
    pub tricks: usize,
    /// Hidden cards scored.
    pub hidden: usize,
    /// Bits per hidden card, hard constraints only.
    pub uniform_bits: f64,
    /// Bits per hidden card, weighted by the bidding evidence.
    pub evidence_bits: f64,
    /// Bits per hidden card of the belief head (0 without one).
    pub head_bits: f64,
}

/// The result of [`probe`].
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ProbeReport {
    /// Input width of the model.
    pub inputs: usize,
    /// Rows used.
    pub rows: usize,
    /// Deals used.
    pub deals: usize,
    /// Rows the sampler could not explain (skipped in the beliefs).
    pub inconsistent: usize,
    /// Value residual correlation.
    pub label: LabelCorrelation,
    /// Policy quality with every input.
    pub base: PolicyScore,
    /// Effect of zeroing each block the model reads.
    pub blocks: Vec<BlockEffect>,
    /// Hidden-card information by completed tricks, then all together.
    pub belief: Vec<BeliefRow>,
}

/// A deal: index of its data directory and deal index.
type DealKey = (usize, u64);

/// One probed row.
struct Row {
    group: DealKey,
    seat: Seat,
    tricks: usize,
    final_points: f32,
    option_value: Option<f32>,
    view: PlayerView,
    belief: [u8; 32],
}

/// The validation card-play rows of whole deals of `dirs`, at least
/// `rows` of them (all if fewer), chosen by `seed`.
fn select(
    dirs: &[PathBuf],
    split: Split,
    rows: usize,
    seed: u64,
) -> io::Result<(Examples, Vec<Row>)> {
    let hyper = Hyper::default();
    let mut deals: BTreeMap<DealKey, Vec<(DecisionRecord, [u8; 32])>> = BTreeMap::new();
    for (d, dir) in dirs.iter().enumerate() {
        let manifest = read_manifest(dir)?;
        for index in 0..manifest.shards() {
            let records = match read_shard(dir, index) {
                Ok(r) => r,
                Err(crate::shards::ShardError::Incomplete) => continue,
                Err(e) => return Err(io::Error::other(format!("{}: {e}", dir.display()))),
            };
            let targets = belief_targets(&manifest, &records).map_err(io::Error::other)?;
            for (r, t) in records.into_iter().zip(targets) {
                if r.split == split && r.view.phase == Phase::Spiel {
                    deals.entry((d, r.deal)).or_default().push((r, t));
                }
            }
        }
    }
    let mut keys: Vec<_> = deals.keys().copied().collect();
    let mut rng = SplitMix(seed ^ 0x7072_6f62);
    for i in (1..keys.len()).rev() {
        keys.swap(i, (rng.next() % (i as u64 + 1)) as usize);
    }
    let mut examples = Examples::new();
    let mut out = Vec::new();
    for key in keys {
        if out.len() >= rows {
            break;
        }
        for (r, belief) in deals.remove(&key).expect("key of the map") {
            let option_value = (r.teacher.status == Some(Status::Complete))
                .then(|| {
                    let chosen = r.view.legal_actions.iter().position(|&a| a == r.action)?;
                    r.teacher.options.get(chosen)?.estimate
                })
                .flatten()
                .map(|e| e.expected_score as f32);
            if examples.push(&r, &hyper, Some(belief)) {
                out.push(Row {
                    group: key,
                    seat: r.view.seat,
                    tricks: r.view.tricks.len(),
                    final_points: r.final_points as f32,
                    option_value,
                    view: r.view,
                    belief,
                });
            }
        }
    }
    Ok((examples, out))
}

/// Per row: policy cross-entropy, agreement, value, and the belief head's
/// bits for each hidden card (empty without a head).
struct Outputs {
    loss: Vec<f64>,
    agree: Vec<bool>,
    value: Vec<f32>,
    head_bits: Vec<Vec<f64>>,
}

fn forward(net: &PolicyValueNet, head: Option<&Linear>, examples: &Examples) -> Outputs {
    let device = Device::flex();
    let inputs = net.inputs();
    let mut out = Outputs {
        loss: Vec::new(),
        agree: Vec::new(),
        value: Vec::new(),
        head_bits: Vec::new(),
    };
    let all: Vec<usize> = (0..examples.rows()).collect();
    for chunk in all.chunks(BATCH) {
        let b = examples.batch(chunk, inputs, &device);
        let x = net.trunk(b.features);
        if let Some(head) = head {
            let lp = belief_log_probs(head, x.clone())
                .into_data()
                .try_to_vec::<f32>()
                .expect("f32");
            for (i, &r) in chunk.iter().enumerate() {
                let bits = examples.belief[32 * r..][..32]
                    .iter()
                    .enumerate()
                    .filter(|&(_, &p)| p != NOT_HIDDEN)
                    .map(|(c, &p)| {
                        let l = lp[(i * 32 + c) * BELIEF_PLACES + usize::from(p)];
                        -f64::from(l) / std::f64::consts::LN_2
                    })
                    .collect();
                out.head_bits.push(bits);
            }
        }
        let o = net.heads(x);
        let logits = o.policy + b.mask;
        let ce = (log_softmax(logits.clone(), 1) * b.policy)
            .sum_dim(1)
            .neg()
            .into_data()
            .try_to_vec::<f32>()
            .expect("f32");
        let logits = logits.into_data().try_to_vec::<f32>().expect("f32");
        let value = o.value.into_data().try_to_vec::<f32>().expect("f32");
        for (i, &r) in chunk.iter().enumerate() {
            let legal = &examples.legal[examples.offsets[r]..examples.offsets[r + 1]];
            let row = &logits[i * ACTIONS..][..ACTIONS];
            let best = (0..legal.len())
                .max_by(|&a, &b| {
                    row[legal[a] as usize]
                        .partial_cmp(&row[legal[b] as usize])
                        .expect("finite")
                        .then(b.cmp(&a))
                })
                .expect("a legal action");
            out.agree.push(best == examples.chosen[r] as usize);
            out.loss.push(f64::from(ce[i]));
            out.value.push(value[i]);
        }
    }
    out
}

fn score(o: &Outputs, rows: &[Row]) -> PolicyScore {
    let n = rows.len().max(1) as f64;
    let mut by = [(0.0, 0usize); 10];
    for (row, &l) in rows.iter().zip(&o.loss) {
        let t = row.tricks.min(9);
        by[t].0 += l;
        by[t].1 += 1;
    }
    PolicyScore {
        policy_loss: o.loss.iter().sum::<f64>() / n,
        agreement: o.agree.iter().filter(|&&a| a).count() as f64 / n,
        loss_by_trick: by.iter().map(|&(s, c)| s / c.max(1) as f64).collect(),
    }
}

/// ICC of `values` grouped by `groups`.
pub fn icc<K: Ord>(values: &[(K, f64)]) -> Icc {
    let n = values.len();
    let mut groups: BTreeMap<&K, (f64, usize)> = BTreeMap::new();
    for (k, v) in values {
        let g = groups.entry(k).or_default();
        g.0 += v;
        g.1 += 1;
    }
    let g = groups.len();
    let rmse = (values.iter().map(|(_, v)| v * v).sum::<f64>() / n.max(1) as f64).sqrt();
    if g < 2 || n <= g {
        return Icc {
            rows: n,
            groups: g,
            rmse,
            ..Icc::default()
        };
    }
    let mean = values.iter().map(|(_, v)| v).sum::<f64>() / n as f64;
    let ssb: f64 = groups
        .values()
        .map(|&(s, c)| c as f64 * (s / c as f64 - mean).powi(2))
        .sum();
    let ssw: f64 = values
        .iter()
        .map(|(k, v)| {
            let (s, c) = groups[k];
            (v - s / c as f64).powi(2)
        })
        .sum();
    let msb = ssb / (g - 1) as f64;
    let msw = ssw / (n - g) as f64;
    let sq: f64 = groups.values().map(|&(_, c)| (c * c) as f64).sum();
    let n0 = (n as f64 - sq / n as f64) / (g - 1) as f64;
    let icc = (msb - msw) / (msb + (n0 - 1.0) * msw);
    let mean_size = n as f64 / g as f64;
    Icc {
        rows: n,
        groups: g,
        rmse,
        icc,
        effective_rows: n as f64 / (1.0 + (mean_size - 1.0) * icc.max(0.0)),
    }
}

/// Uniform and evidence-weighted bits of the hidden cards of one row, or
/// `None` if the sampler cannot explain the view.
fn classic_bits(row: &Row, samples: usize, seed: u64) -> Option<(Vec<f64>, Vec<f64>)> {
    let hidden = HiddenCards::from_view(&row.view).ok()?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let me = row.seat.index();
    let mut uniform = [[0.0f64; BELIEF_PLACES]; 32];
    let mut weighted = [[0.0f64; BELIEF_PLACES]; 32];
    let mut total = 0.0;
    for _ in 0..samples {
        let world = hidden.sample(&mut rng);
        let w = evidence::weight(&row.view, &world);
        total += w;
        for (c, &p) in row.belief.iter().enumerate() {
            if p == NOT_HIDDEN {
                continue;
            }
            let card = skat_core::Card::from_index(c as u8).expect("card index");
            let place = if world.skat.contains(card) {
                2
            } else if world.hands[(me + 1) % 3].contains(card) {
                0
            } else {
                1
            };
            uniform[c][place] += 1.0;
            weighted[c][place] += w;
        }
    }
    // Additive smoothing of half a sample per place keeps every estimate
    // finite; a place the constraints exclude never holds the true card.
    const EPS: f64 = 0.5;
    let s = samples as f64;
    let mean_w = total / s;
    let mut u_bits = Vec::new();
    let mut e_bits = Vec::new();
    for (c, &p) in row.belief.iter().enumerate() {
        if p == NOT_HIDDEN {
            continue;
        }
        let p = usize::from(p);
        let pu = (uniform[c][p] + EPS) / (s + 3.0 * EPS);
        let pe = (weighted[c][p] + EPS * mean_w) / (total + 3.0 * EPS * mean_w);
        u_bits.push(-pu.log2());
        e_bits.push(-pe.log2());
    }
    Some((u_bits, e_bits))
}

/// Probes `net` (with its belief head, if given) on `dirs`.
pub fn probe(
    net: &PolicyValueNet,
    head: Option<&Linear>,
    dirs: &[PathBuf],
    config: ProbeConfig,
) -> io::Result<ProbeReport> {
    let (examples, rows) = select(dirs, config.split, config.rows, config.seed)?;
    let base = forward(net, head, &examples);

    // Label correlation.
    let resid = |row: &Row, v: f32, target: f32| (row.group, f64::from(target - v));
    let by_seat: Vec<_> = rows
        .iter()
        .zip(&base.value)
        .map(|(r, &v)| {
            let (g, x) = resid(r, v, r.final_points);
            ((g, r.seat.index()), x)
        })
        .collect();
    let by_deal: Vec<_> = rows
        .iter()
        .zip(&base.value)
        .map(|(r, &v)| resid(r, v, r.final_points))
        .collect();
    let option: Vec<_> = rows
        .iter()
        .zip(&base.value)
        .filter_map(|(r, &v)| {
            let (g, x) = resid(r, v, r.option_value?);
            Some(((g, r.seat.index()), x))
        })
        .collect();
    let label = LabelCorrelation {
        final_by_seat: icc(&by_seat),
        final_by_deal: icc(&by_deal),
        option_by_seat: icc(&option),
    };

    // Block importance.
    let base_score = score(&base, &rows);
    let inputs = net.inputs();
    let mut blocks = Vec::new();
    for (name, range) in BLOCKS {
        if range.end > inputs {
            continue;
        }
        let mut zeroed = examples.clone();
        for r in 0..zeroed.rows() {
            zeroed.features.fill(r, range.clone(), 0.0);
        }
        let s = score(&forward(net, None, &zeroed), &rows);
        blocks.push(BlockEffect {
            block: name.to_string(),
            policy_loss_delta: s.policy_loss - base_score.policy_loss,
            agreement_delta: s.agreement - base_score.agreement,
            loss_delta_by_trick: s
                .loss_by_trick
                .iter()
                .zip(&base_score.loss_by_trick)
                .map(|(a, b)| a - b)
                .collect(),
        });
    }

    // Hidden-card information.
    let classic: Vec<Option<(Vec<f64>, Vec<f64>)>> = if config.samples == 0 {
        Vec::new()
    } else {
        let threads = config.threads.max(1);
        let chunk = rows.len().div_ceil(threads).max(1);
        std::thread::scope(|scope| {
            let handles: Vec<_> = rows
                .chunks(chunk)
                .enumerate()
                .map(|(t, part)| {
                    scope.spawn(move || {
                        part.iter()
                            .enumerate()
                            .map(|(i, row)| {
                                let index = (t * chunk + i) as u64;
                                let seed = SplitMix(config.seed ^ index).next();
                                classic_bits(row, config.samples, seed)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("sampling thread"))
                .collect()
        })
    };
    let mut sums = [(0usize, 0.0, 0.0, 0.0); 11];
    let mut inconsistent = 0;
    for (i, row) in rows.iter().enumerate() {
        let classic = match classic.get(i) {
            Some(None) => {
                inconsistent += 1;
                continue;
            }
            Some(Some(c)) => Some(c),
            None => None,
        };
        let head_bits = base.head_bits.get(i);
        let hidden = row.belief.iter().filter(|&&p| p != NOT_HIDDEN).count();
        for t in [row.tricks.min(9), 10] {
            let s = &mut sums[t];
            s.0 += hidden;
            if let Some((u, e)) = classic {
                s.1 += u.iter().sum::<f64>();
                s.2 += e.iter().sum::<f64>();
            }
            if let Some(h) = head_bits {
                s.3 += h.iter().sum::<f64>();
            }
        }
    }
    let belief = sums
        .iter()
        .enumerate()
        .map(|(t, &(n, u, e, h))| {
            let per = |x: f64| x / n.max(1) as f64;
            BeliefRow {
                tricks: t,
                hidden: n,
                uniform_bits: per(u),
                evidence_bits: per(e),
                head_bits: per(h),
            }
        })
        .collect();

    Ok(ProbeReport {
        inputs,
        rows: rows.len(),
        deals: rows
            .iter()
            .map(|r| r.group)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        inconsistent,
        label,
        base: base_score,
        blocks,
        belief,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_runs_on_the_fixture_and_the_beliefs_order_as_expected() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/selfplay");
        let shape = skat_nn::NetShape::current(16);
        let net = crate::train::initial(shape, 3);
        let head = crate::train::initial_belief(16, 3, &Device::flex());
        let config = ProbeConfig {
            rows: 1,
            samples: 64,
            seed: 0,
            threads: 2,
            split: Split::Train,
        };
        let r = probe(&net, Some(&head), std::slice::from_ref(&dir), config).unwrap();
        // One whole deal: 30 card plays and two Kontra/Re decisions.
        assert_eq!((r.rows, r.deals, r.inconsistent), (32, 1, 0));
        assert_eq!(r.blocks.len(), BLOCKS.len());
        let all = r.belief[10];
        assert!(all.hidden > 0);
        // Constraints alone know less than log2(3) bits per card; an
        // untrained head is near log2(3).
        assert!(all.uniform_bits > 0.0 && all.uniform_bits < 3f64.log2());
        assert!(all.head_bits > 1.0);
        // Same seed, same report.
        assert_eq!(probe(&net, Some(&head), &[dir], config).unwrap(), r);
    }

    #[test]
    fn icc_of_constant_groups_is_one_and_of_noise_near_zero() {
        let same: Vec<_> = (0..40).map(|i| (i / 4, f64::from(i / 4))).collect();
        assert!((icc(&same).icc - 1.0).abs() < 1e-9);
        assert!((icc(&same).effective_rows - 10.0).abs() < 1e-9);
        // Alternating values within every group: no group effect.
        let noise: Vec<_> = (0..40)
            .map(|i| (i / 4, if i % 2 == 0 { 1.0 } else { -1.0 }))
            .collect();
        assert!(icc(&noise).icc < 0.0);
        assert_eq!(icc(&noise).effective_rows, 40.0);
    }
}
