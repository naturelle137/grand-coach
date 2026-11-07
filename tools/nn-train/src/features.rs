// SPDX-License-Identifier: EUPL-1.2

//! Lossless compact storage of encoded feature rows.
//!
//! Most encoder features take few distinct values (0, 1 and fractions
//! `k/n` with a small `n`), so a value is stored as a one-byte code into a
//! per-feature table of the exact `f32` values. A feature that shows more
//! than [`MAX_VALUES`] distinct bit patterns (sums such as the Augen per
//! party, whose rounding depends on the order of the tricks) becomes
//! *wide*: its exact values are kept as `f32` in a column of their own.
//! Decoding is bit-identical either way. With two wide features a row
//! needs about 650 bytes instead of 2,536.

use std::collections::HashMap;
use std::ops::Range;

use skat_nn::FEATURES;

/// Maximum number of distinct values of a coded feature.
pub const MAX_VALUES: usize = 256;

/// Codes of one feature.
#[derive(Clone, Debug, Default)]
struct Table {
    /// The value of each code (first-seen order); `[0.0]` for a wide
    /// feature, whose codes are all 0.
    values: Vec<f32>,
    /// Code of each value, by bit pattern (empty for a wide feature).
    index: HashMap<u32, u8>,
    /// Code of 0.0 (the most frequent value), once seen.
    zero: Option<u8>,
    /// Column in [`FeatureCodes::wide`] of a wide feature.
    slot: Option<usize>,
}

impl Table {
    /// The code of `value`, added if new; `None` if the table is full.
    fn code(&mut self, value: f32) -> Option<u8> {
        let bits = value.to_bits();
        if bits == 0
            && let Some(c) = self.zero
        {
            return Some(c);
        }
        if let Some(&c) = self.index.get(&bits) {
            return Some(c);
        }
        if self.values.len() == MAX_VALUES {
            return None;
        }
        let c = self.values.len() as u8;
        self.values.push(value);
        self.index.insert(bits, c);
        if bits == 0 {
            self.zero = Some(c);
        }
        Some(c)
    }
}

/// Feature rows of [`FEATURES`] values each: codes into per-feature
/// tables, exact columns for wide features.
#[derive(Clone, Debug)]
pub struct FeatureCodes {
    /// One code per value, row-major (0 for wide features).
    codes: Vec<u8>,
    /// One table per feature.
    tables: Vec<Table>,
    /// Wide features with one exact value per row, in the order they
    /// became wide.
    wide: Vec<(usize, Vec<f32>)>,
}

impl Default for FeatureCodes {
    fn default() -> FeatureCodes {
        FeatureCodes {
            codes: Vec::new(),
            tables: vec![Table::default(); FEATURES],
            wide: Vec::new(),
        }
    }
}

impl PartialEq for FeatureCodes {
    fn eq(&self, other: &FeatureCodes) -> bool {
        self.codes == other.codes && self.tables() == other.tables() && self.wide == other.wide
    }
}

impl FeatureCodes {
    /// Empty rows with the tables and wide features of `other`, so its
    /// rows copy as they are ([`FeatureCodes::push_row_of`]).
    pub fn with_table_of(other: &FeatureCodes) -> FeatureCodes {
        FeatureCodes {
            codes: Vec::new(),
            tables: other.tables.clone(),
            wide: other.wide.iter().map(|(f, _)| (*f, Vec::new())).collect(),
        }
    }

    /// Rows from stored parts ([`FeatureCodes::codes`],
    /// [`FeatureCodes::tables`], [`FeatureCodes::wide`]), checked.
    pub fn from_parts(
        codes: Vec<u8>,
        tables: Vec<Vec<f32>>,
        wide: Vec<(usize, Vec<f32>)>,
    ) -> Result<FeatureCodes, String> {
        if !codes.len().is_multiple_of(FEATURES) {
            return Err("feature codes are not whole rows".into());
        }
        let rows = codes.len() / FEATURES;
        if tables.len() != FEATURES {
            return Err("one feature table per feature expected".into());
        }
        let mut out = FeatureCodes::default();
        for (f, values) in tables.into_iter().enumerate() {
            if values.len() > MAX_VALUES {
                return Err("feature table too large".into());
            }
            let table = &mut out.tables[f];
            for value in values {
                let code = table.values.len() as u8;
                if table.index.insert(value.to_bits(), code).is_some() {
                    return Err("feature table holds a value twice".into());
                }
                table.values.push(value);
            }
            table.zero = table.index.get(&0).copied();
        }
        for (slot, (f, column)) in wide.iter().enumerate() {
            let Some(table) = out.tables.get_mut(*f) else {
                return Err("wide feature out of range".into());
            };
            if table.slot.is_some() || table.values != [0.0] || column.len() != rows {
                return Err("malformed wide feature".into());
            }
            table.index.clear();
            table.slot = Some(slot);
        }
        for row in codes.as_chunks::<FEATURES>().0 {
            for (&c, table) in row.iter().zip(&out.tables) {
                if usize::from(c) >= table.values.len() {
                    return Err("feature code outside its table".into());
                }
            }
        }
        out.codes = codes;
        out.wide = wide;
        Ok(out)
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.codes.len() / FEATURES
    }

    /// All codes, row-major (0 for wide features).
    pub fn codes(&self) -> &[u8] {
        &self.codes
    }

    /// The value of each code, per feature (`[0.0]` for wide features).
    pub fn tables(&self) -> Vec<Vec<f32>> {
        self.tables.iter().map(|t| t.values.clone()).collect()
    }

    /// The wide features with their exact values per row.
    pub fn wide(&self) -> &[(usize, Vec<f32>)] {
        &self.wide
    }

    /// Makes feature `f` wide: moves the values of its first `rows` rows
    /// into an exact column.
    fn widen(&mut self, f: usize, rows: usize) -> usize {
        let table = &self.tables[f];
        let column = (0..rows)
            .map(|r| table.values[usize::from(self.codes[r * FEATURES + f])])
            .collect();
        for r in 0..rows {
            self.codes[r * FEATURES + f] = 0;
        }
        let slot = self.wide.len();
        self.wide.push((f, column));
        self.tables[f] = Table {
            values: vec![0.0],
            slot: Some(slot),
            ..Table::default()
        };
        slot
    }

    /// The code of `value` in feature `f` of row `r` (the row being
    /// written); a wide feature stores the value and gets code 0.
    fn put(&mut self, f: usize, r: usize, value: f32) -> u8 {
        let slot = match self.tables[f].slot {
            Some(slot) => slot,
            None => match self.tables[f].code(value) {
                Some(c) => return c,
                None => self.widen(f, self.rows()),
            },
        };
        let column = &mut self.wide[slot].1;
        if column.len() == r {
            column.push(value);
        } else {
            column[r] = value;
        }
        0
    }

    /// Appends a row of [`FEATURES`] values.
    pub fn push_row(&mut self, row: &[f32]) {
        assert_eq!(row.len(), FEATURES, "feature row width");
        let r = self.rows();
        let mut codes = [0u8; FEATURES];
        for (f, &v) in row.iter().enumerate() {
            codes[f] = self.put(f, r, v);
        }
        self.codes.extend_from_slice(&codes);
    }

    /// Appends row `r` of `other`, which must share these tables
    /// ([`FeatureCodes::with_table_of`]).
    pub fn push_row_of(&mut self, other: &FeatureCodes, r: usize) {
        self.codes
            .extend_from_slice(&other.codes[r * FEATURES..][..FEATURES]);
        for ((_, mine), (_, theirs)) in self.wide.iter_mut().zip(&other.wide) {
            mine.push(theirs[r]);
        }
    }

    /// The first `inputs` values of row `r`, appended to `out`.
    pub fn decode_into(&self, r: usize, inputs: usize, out: &mut Vec<f32>) {
        let base = out.len();
        out.extend(
            self.codes[r * FEATURES..][..inputs]
                .iter()
                .zip(&self.tables)
                .map(|(&c, t)| t.values[usize::from(c)]),
        );
        for (f, column) in &self.wide {
            if *f < inputs {
                out[base + f] = column[r];
            }
        }
    }

    /// Row `r` as values.
    pub fn row(&self, r: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(FEATURES);
        self.decode_into(r, FEATURES, &mut out);
        out
    }

    /// Sets the values `range` of row `r` to `value`.
    pub fn fill(&mut self, r: usize, range: Range<usize>, value: f32) {
        for f in range {
            let c = self.put(f, r, value);
            self.codes[r * FEATURES + f] = c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seed: u32) -> Vec<f32> {
        (0..FEATURES)
            .map(|i| match (i as u32 + seed) % 5 {
                0 => 0.0,
                1 => 1.0,
                2 => ((i as u32 + seed) % 30) as f32 / 30.0,
                3 => ((i as u32 + seed) % 120) as f32 / 120.0,
                _ => -0.0,
            })
            .collect()
    }

    #[test]
    fn rows_round_trip_bit_identical() {
        let mut f = FeatureCodes::default();
        for s in 0..20 {
            f.push_row(&row(s));
        }
        assert_eq!(f.rows(), 20);
        for s in 0..20 {
            let back = f.row(s as usize);
            let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&back), bits(&row(s)));
        }
        let parts = FeatureCodes::from_parts(f.codes().to_vec(), f.tables(), f.wide().to_vec())
            .expect("ok");
        assert_eq!(parts, f);
    }

    #[test]
    fn a_feature_with_more_than_256_values_becomes_wide_and_stays_exact() {
        let mut f = FeatureCodes::default();
        let value = |k: usize, i: usize| {
            if i == 5 {
                k as f32 / 7.0
            } else {
                (k % 3) as f32
            }
        };
        for k in 0..600 {
            let row: Vec<f32> = (0..FEATURES).map(|i| value(k, i)).collect();
            f.push_row(&row);
        }
        assert_eq!(f.wide().len(), 1);
        assert_eq!(f.wide()[0].0, 5);
        for k in 0..600 {
            let back = f.row(k);
            assert!((0..FEATURES).all(|i| back[i].to_bits() == value(k, i).to_bits()));
        }
        let parts = FeatureCodes::from_parts(f.codes().to_vec(), f.tables(), f.wide().to_vec())
            .expect("ok");
        assert_eq!(parts, f);
        let mut part = FeatureCodes::with_table_of(&f);
        part.push_row_of(&f, 599);
        assert_eq!(part.row(0), f.row(599));
        assert!(
            FeatureCodes::from_parts(vec![3; FEATURES], vec![vec![0.0, 1.0]; FEATURES], vec![])
                .is_err()
        );
    }
}
