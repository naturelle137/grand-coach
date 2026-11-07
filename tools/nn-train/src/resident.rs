// SPDX-License-Identifier: EUPL-1.2

//! Training rows resident in device memory, an option for GPU training.
//!
//! By default every batch is assembled on the CPU: the feature rows are
//! decoded and five dense `[batch, ACTIONS]` target tables are filled and
//! uploaded. With [`ResidentRows`] the feature codes of the training split
//! live on the device, four codes packed into one `i32` (VRAM need: about
//! one byte per feature, plus four per row for each wide feature), in
//! blocks of at most [`BLOCK_BYTES`] so no buffer exceeds a device limit. A step then uploads only row indices and
//! the sparse per-action targets; the device selects, unpacks and decodes
//! the features ([`FeatureCodes`] tables) and scatters the targets into
//! dense tables. The batch has the same values as the CPU batch of the
//! same rows ([`ResidentRows::group`] orders a batch's rows by block, which
//! does not change the batch loss).

use burn::tensor::{Device, IndexingUpdateOp, Int, Tensor, TensorData};
use skat_nn::ACTIONS;

use crate::features::{FeatureCodes, MAX_VALUES};
use crate::train::{BELIEF_OUTPUTS, BELIEF_PLACES, Batch, Examples, NOT_HIDDEN, tensor};

/// Maximum size of one device buffer of packed rows.
pub const BLOCK_BYTES: usize = 1 << 30;

/// Rows of one block on the device.
struct Block {
    /// Packed codes, `[rows, words]`.
    packed: Tensor<2, Int>,
    /// Exact values of the wide features, `[rows, wide]`, if any.
    wide: Option<Tensor<2>>,
}

/// Packed feature rows of one split on a device.
pub struct ResidentRows {
    /// Blocks of `block_rows` rows (the last may be shorter).
    blocks: Vec<Block>,
    /// Rows per block.
    block_rows: usize,
    /// `i32` words per row.
    words: usize,
    /// Features per row (the network's inputs).
    inputs: usize,
    /// Rows in total.
    rows: usize,
    /// Value of each code, [`MAX_VALUES`] per feature (`[inputs * 256]`).
    table: Tensor<1>,
    /// Start of each feature's table, `[1, inputs]`.
    starts: Tensor<2, Int>,
    /// Wide features among the inputs, `[1, wide]`, if any.
    wide: Option<Tensor<2, Int>>,
}

/// Wide features of `features` among the first `inputs`, with their
/// columns.
fn wide(features: &FeatureCodes, inputs: usize) -> Vec<&(usize, Vec<f32>)> {
    features
        .wide()
        .iter()
        .filter(|(f, _)| *f < inputs)
        .collect()
}

/// Device bytes per row.
fn row_bytes(features: &FeatureCodes, inputs: usize) -> usize {
    4 * (inputs.div_ceil(4) + wide(features, inputs).len())
}

impl ResidentRows {
    /// Device bytes of the first `inputs` features of all rows of
    /// `features`.
    pub fn bytes(features: &FeatureCodes, inputs: usize) -> usize {
        features.rows() * row_bytes(features, inputs)
    }

    /// Uploads the first `inputs` features of every row of `features`.
    pub fn upload(features: &FeatureCodes, inputs: usize, device: &Device) -> ResidentRows {
        let block_rows = (BLOCK_BYTES / row_bytes(features, inputs)).max(1);
        ResidentRows::upload_blocks(features, inputs, block_rows, device)
    }

    /// [`ResidentRows::upload`] in blocks of `block_rows` rows.
    pub(crate) fn upload_blocks(
        features: &FeatureCodes,
        inputs: usize,
        block_rows: usize,
        device: &Device,
    ) -> ResidentRows {
        let words = inputs.div_ceil(4);
        let rows = features.rows();
        let codes = features.codes();
        let width = skat_nn::FEATURES;
        let wide = wide(features, inputs);
        let mut blocks = Vec::new();
        let mut start = 0;
        while start < rows {
            let end = (start + block_rows).min(rows);
            let mut packed = Vec::with_capacity((end - start) * words);
            for r in start..end {
                let row = &codes[r * width..][..inputs];
                for w in 0..words {
                    let mut word = 0u32;
                    for k in 0..4 {
                        if let Some(&c) = row.get(4 * w + k) {
                            word |= u32::from(c) << (8 * k);
                        }
                    }
                    packed.push(word as i32);
                }
            }
            let values = (!wide.is_empty()).then(|| {
                let values: Vec<f32> = (start..end)
                    .flat_map(|r| wide.iter().map(move |(_, column)| column[r]))
                    .collect();
                tensor(values, [end - start, wide.len()], device)
            });
            blocks.push(Block {
                packed: Tensor::<2, Int>::from_data(
                    TensorData::new(packed, [end - start, words]),
                    device,
                ),
                wide: values,
            });
            start = end;
        }
        let mut table = vec![0.0f32; inputs * MAX_VALUES];
        for (f, values) in features.tables().iter().take(inputs).enumerate() {
            table[f * MAX_VALUES..][..values.len()].copy_from_slice(values);
        }
        let starts: Vec<i32> = (0..inputs).map(|f| (f * MAX_VALUES) as i32).collect();
        ResidentRows {
            blocks,
            block_rows,
            words,
            inputs,
            rows,
            table: Tensor::from_data(TensorData::new(table, [inputs * MAX_VALUES]), device),
            starts: Tensor::from_data(TensorData::new(starts, [1, inputs]), device),
            wide: (!wide.is_empty()).then(|| {
                let features: Vec<i32> = wide.iter().map(|(f, _)| *f as i32).collect();
                let len = features.len();
                Tensor::from_data(TensorData::new(features, [1, len]), device)
            }),
        }
    }

    /// Rows on the device.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Features per row.
    pub fn inputs(&self) -> usize {
        self.inputs
    }

    /// Orders `rows` by block, keeping their order within a block, as
    /// [`ResidentRows::batch`] expects.
    pub fn group(&self, rows: &mut [usize]) {
        let b = self.block_rows;
        rows.sort_by_key(|&r| r / b);
    }

    /// The feature rows `rows` (grouped by block) as values.
    fn features(&self, rows: &[usize], device: &Device) -> Tensor<2> {
        let n = rows.len();
        let b = self.block_rows;
        let mut parts = Vec::new();
        let mut wide_parts = Vec::new();
        let mut i = 0;
        while i < n {
            let block = rows[i] / b;
            let len = rows[i..].iter().take_while(|&&r| r / b == block).count();
            let local: Vec<i32> = rows[i..i + len]
                .iter()
                .map(|&r| (r - block * b) as i32)
                .collect();
            let index = Tensor::<1, Int>::from_data(TensorData::new(local, [len]), device);
            let block = &self.blocks[block];
            if let Some(wide) = &block.wide {
                wide_parts.push(wide.clone().select(0, index.clone()));
            }
            parts.push(block.packed.clone().select(0, index));
            i += len;
        }
        let packed = Tensor::cat(parts, 0);
        let bytes: Vec<Tensor<2, Int>> = (0..4)
            .map(|k| {
                packed
                    .clone()
                    .bitwise_right_shift_scalar(8 * k)
                    .bitwise_and_scalar(0xff)
            })
            .collect();
        let codes = (Tensor::<2, Int>::stack::<3>(bytes, 2)
            .reshape([n, 4 * self.words])
            .slice([0..n, 0..self.inputs])
            + self.starts.clone())
        .reshape([n * self.inputs]);
        let values = self
            .table
            .clone()
            .select(0, codes)
            .reshape([n, self.inputs]);
        match &self.wide {
            // Wide features decode to 0 above; their exact values replace
            // them (distinct columns, so assigning is well defined).
            Some(features) => values.scatter(
                1,
                features.clone().repeat_dim(0, n),
                Tensor::cat(wide_parts, 0),
                IndexingUpdateOp::Assign,
            ),
            None => values,
        }
    }

    /// The batch of `rows` (grouped by block, [`ResidentRows::group`]) of
    /// `examples`, whose feature rows are the resident ones.
    pub(crate) fn batch(&self, examples: &Examples, rows: &[usize], device: &Device) -> Batch {
        let n = rows.len();
        let span = |r: usize| examples.offsets[r]..examples.offsets[r + 1];
        let width = rows
            .iter()
            .map(|&r| span(r).len())
            .max()
            .unwrap_or(0)
            .max(1);
        // Padding entries go to the extra column ACTIONS with value 0.
        let mut index = vec![ACTIONS as i32; n * width];
        let mut legal = vec![0.0f32; n * width];
        let mut target = vec![0.0f32; n * width];
        let mut option = vec![0.0f32; n * width];
        let mut score = vec![0.0f32; n * width];
        let mut win = vec![0.0f32; n * width];
        let mut value = Vec::with_capacity(n);
        let mut options = 0;
        let with_belief = examples.belief.len() == 32 * examples.rows();
        let mut belief = vec![0.0f32; if with_belief { n * BELIEF_OUTPUTS } else { 0 }];
        let mut hidden = 0;
        for (b, &r) in rows.iter().enumerate() {
            for (j, i) in span(r).enumerate() {
                let e = b * width + j;
                index[e] = examples.legal[i] as i32;
                legal[e] = 1.0;
                target[e] = examples.target[i];
                if examples.has_options[r] {
                    option[e] = 1.0;
                    score[e] = examples.option_score[i];
                    win[e] = examples.option_win[i];
                    options += 1;
                }
            }
            value.push(examples.value[r]);
            if with_belief {
                for (c, &place) in examples.belief[32 * r..][..32].iter().enumerate() {
                    if place != NOT_HIDDEN {
                        belief[b * BELIEF_OUTPUTS + c * BELIEF_PLACES + usize::from(place)] = 1.0;
                        hidden += 1;
                    }
                }
            }
        }
        let index = Tensor::<2, Int>::from_data(TensorData::new(index, [n, width]), device);
        let dense = |values: Vec<f32>| -> Tensor<2> {
            Tensor::<2>::zeros([n, ACTIONS + 1], device)
                .scatter(
                    1,
                    index.clone(),
                    tensor(values, [n, width], device),
                    IndexingUpdateOp::Add,
                )
                .slice([0..n, 0..ACTIONS])
        };
        Batch {
            features: self.features(rows, device),
            // 1 → 0 for legal actions, 0 → -1e9 for the others.
            mask: (dense(legal) - 1.0) * 1e9,
            policy: dense(target),
            value: tensor(value, [n, 1], device),
            option_mask: dense(option),
            score: dense(score),
            win: dense(win),
            options,
            belief: with_belief.then(|| (tensor(belief, [n, BELIEF_OUTPUTS], device), hidden)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::train::{Hyper, load_with};

    fn values(t: Tensor<2>) -> Vec<u32> {
        t.into_data()
            .try_to_vec::<f32>()
            .expect("f32")
            .iter()
            .map(|x| x.to_bits())
            .collect()
    }

    #[test]
    fn resident_batches_equal_cpu_batches() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/selfplay");
        let hyper = Hyper {
            soft_weight: 0.3,
            ..Hyper::default()
        };
        let data = load_with(&[fixture], &hyper, true).expect("fixture loads");
        let e = &data.train;
        let device = Device::flex();
        let inputs = skat_nn::FEATURES;
        // Small blocks, so a batch spans several of them.
        let resident = ResidentRows::upload_blocks(&e.features, inputs, 7, &device);
        assert_eq!(resident.rows(), e.rows());
        let mut rows: Vec<usize> = (0..e.rows()).rev().step_by(3).take(40).collect();
        resident.group(&mut rows);
        let cpu = e.batch(&rows, inputs, &device);
        let dev = resident.batch(e, &rows, &device);
        assert_eq!(values(dev.features), values(cpu.features));
        assert_eq!(values(dev.mask), values(cpu.mask));
        assert_eq!(values(dev.policy), values(cpu.policy));
        assert_eq!(values(dev.value), values(cpu.value));
        assert_eq!(values(dev.option_mask), values(cpu.option_mask));
        assert_eq!(values(dev.score), values(cpu.score));
        assert_eq!(values(dev.win), values(cpu.win));
        assert_eq!(dev.options, cpu.options);
        let (db, dh) = dev.belief.expect("belief");
        let (cb, ch) = cpu.belief.expect("belief");
        assert_eq!((values(db), dh), (values(cb), ch));
    }

    #[test]
    fn wide_features_decode_exactly_on_the_device() {
        let width = skat_nn::FEATURES;
        let value = |k: usize, i: usize| match i {
            3 => k as f32 / 7.0,
            9 if k.is_multiple_of(5) => -0.0,
            9 => k as f32 / 11.0,
            _ => (k % 4) as f32 / 3.0,
        };
        let mut f = FeatureCodes::default();
        for k in 0..700 {
            f.push_row(&(0..width).map(|i| value(k, i)).collect::<Vec<_>>());
        }
        assert_eq!(f.wide().len(), 2);
        let device = Device::flex();
        for inputs in [width, 6] {
            let resident = ResidentRows::upload_blocks(&f, inputs, 50, &device);
            let mut rows: Vec<usize> = (0..700).rev().step_by(9).collect();
            resident.group(&mut rows);
            let mut cpu = Vec::new();
            for &r in &rows {
                f.decode_into(r, inputs, &mut cpu);
            }
            let cpu: Vec<u32> = cpu.iter().map(|x| x.to_bits()).collect();
            assert_eq!(values(resident.features(&rows, &device)), cpu);
        }
    }
}
