// SPDX-License-Identifier: EUPL-1.2

//! The two computer opponents as named figures with a portrait (issue #134,
//! D-72). Only identities live here: translated names and portrait files are
//! presentation resources keyed by [`FigureId::key`], so any frontend
//! (egui now, Godot later) can show them.
//!
//! Each language has its own pool of twelve figures, six female and six
//! male. A series draws two different figures per pool from its own
//! deterministic stream of the series seed, which no deal or opponent
//! decision reads. The figures keep their physical seats (player 1 left,
//! player 2 right) for the whole series while the roles rotate.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::session::PlayerId;

/// Stream of the series seed for the cast; streams 0-4 belong to deals,
/// opponents, coach, quiz and practice quiz.
const CAST_STREAM: u64 = 5;

/// Selection weight of a familiar name.
pub const FAMILIAR_WEIGHT: u32 = 3;
/// Selection weight of an old-fashioned name: the four of a pool together
/// get 4 × 2 / (8 × 3 + 4 × 2) = 25 % of the first draw.
pub const RARE_WEIGHT: u32 = 2;

/// A pool of figures; the frontend shows the pool of its language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FigurePool {
    /// German names.
    German,
    /// English names.
    English,
}

impl FigurePool {
    /// Both pools.
    pub const ALL: [FigurePool; 2] = [FigurePool::German, FigurePool::English];
}

/// Grammatical gender of a figure's name, for an even pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    /// Female.
    Female,
    /// Male.
    Male,
}

/// One figure of the catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Figure {
    /// Stable resource key: message `figure-<key>` and portrait
    /// `<key>.svg`.
    pub key: &'static str,
    /// Its pool.
    pub pool: FigurePool,
    /// Gender of the name.
    pub gender: Gender,
    /// `true` for an old-fashioned, rarer name ([`RARE_WEIGHT`]).
    pub rare: bool,
}

impl Figure {
    /// Selection weight.
    pub const fn weight(&self) -> u32 {
        if self.rare {
            RARE_WEIGHT
        } else {
            FAMILIAR_WEIGHT
        }
    }
}

const fn figure(key: &'static str, pool: FigurePool, gender: Gender, rare: bool) -> Figure {
    Figure {
        key,
        pool,
        gender,
        rare,
    }
}

use FigurePool::{English as EN, German as DE};
use Gender::{Female as F, Male as M};

/// All figures; the index is the [`FigureId`]. Never reorder or remove an
/// entry: a figure keeps its id, name and portrait permanently.
pub const FIGURES: [Figure; 24] = [
    figure("de-anna", DE, F, false),
    figure("de-sabine", DE, F, false),
    figure("de-monika", DE, F, false),
    figure("de-annemarie", DE, F, false),
    figure("de-adelheid", DE, F, true),
    figure("de-hildegard", DE, F, true),
    figure("de-thomas", DE, M, false),
    figure("de-michael", DE, M, false),
    figure("de-peter", DE, M, false),
    figure("de-stefan", DE, M, false),
    figure("de-kunibert", DE, M, true),
    figure("de-friedrich-wilhelm", DE, M, true),
    figure("en-emma", EN, F, false),
    figure("en-sarah", EN, F, false),
    figure("en-linda", EN, F, false),
    figure("en-elizabeth", EN, F, false),
    figure("en-ethel", EN, F, true),
    figure("en-mildred", EN, F, true),
    figure("en-james", EN, M, false),
    figure("en-david", EN, M, false),
    figure("en-tom", EN, M, false),
    figure("en-oliver", EN, M, false),
    figure("en-archibald", EN, M, true),
    figure("en-bartholomew", EN, M, true),
];

/// A figure of [`FIGURES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FigureId(pub u8);

impl FigureId {
    /// The figure.
    pub fn figure(self) -> &'static Figure {
        &FIGURES[usize::from(self.0)]
    }

    /// Stable resource key of the figure.
    pub fn key(self) -> &'static str {
        self.figure().key
    }

    /// All figures of `pool`.
    pub fn pool(pool: FigurePool) -> impl Iterator<Item = FigureId> {
        (0..FIGURES.len() as u8)
            .map(FigureId)
            .filter(move |id| id.figure().pool == pool)
    }
}

/// The opponents of a series: two different figures per pool, by
/// physical seat (index 0 is player 1 on the left, 1 is player 2 on the
/// right). A pure function of the series seed, so a replayed or practised
/// game of the series shows the same figures; switching the language and
/// back restores the previous cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeriesCast {
    /// German pool.
    pub german: [FigureId; 2],
    /// English pool.
    pub english: [FigureId; 2],
}

impl SeriesCast {
    /// The cast of the series with `seed`, from its own stream.
    pub fn for_seed(seed: u64) -> SeriesCast {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        rng.set_stream(CAST_STREAM);
        let german = draw_two(&mut rng, FigurePool::German);
        let english = draw_two(&mut rng, FigurePool::English);
        SeriesCast { german, english }
    }

    /// The two figures of `pool` by physical seat.
    pub fn pool(&self, pool: FigurePool) -> [FigureId; 2] {
        match pool {
            FigurePool::German => self.german,
            FigurePool::English => self.english,
        }
    }

    /// Figure of `player` in `pool`; `None` for the human, who stays "you"
    /// without a portrait.
    pub fn figure(&self, pool: FigurePool, player: PlayerId) -> Option<FigureId> {
        match player.0 {
            1 | 2 => Some(self.pool(pool)[player.index() - 1]),
            _ => None,
        }
    }
}

/// Two different weighted draws from `pool`.
fn draw_two(rng: &mut ChaCha8Rng, pool: FigurePool) -> [FigureId; 2] {
    let first = draw(rng, pool, None);
    [first, draw(rng, pool, Some(first))]
}

/// One weighted draw from `pool` without `except`.
fn draw(rng: &mut ChaCha8Rng, pool: FigurePool, except: Option<FigureId>) -> FigureId {
    let candidates = || FigureId::pool(pool).filter(|&id| Some(id) != except);
    let total: u32 = candidates().map(|id| id.figure().weight()).sum();
    pick(candidates(), rng.next_u32() % total)
}

/// The figure whose weight interval contains `ticket`.
fn pick(candidates: impl Iterator<Item = FigureId>, mut ticket: u32) -> FigureId {
    for id in candidates {
        let weight = id.figure().weight();
        if ticket < weight {
            return id;
        }
        ticket -= weight;
    }
    unreachable!("the ticket lies below the total weight")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pool_has_six_female_and_six_male_figures_with_unique_keys() {
        for pool in FigurePool::ALL {
            let ids: Vec<_> = FigureId::pool(pool).collect();
            assert_eq!(ids.len(), 12);
            let female = ids
                .iter()
                .filter(|id| id.figure().gender == Gender::Female)
                .count();
            assert_eq!(female, 6, "{pool:?}");
            let rare: Vec<_> = ids.iter().filter(|id| id.figure().rare).collect();
            assert_eq!(rare.len(), 4, "{pool:?}");
            assert_eq!(
                rare.iter()
                    .filter(|id| id.figure().gender == Gender::Female)
                    .count(),
                2
            );
        }
        let mut keys: Vec<_> = FIGURES.iter().map(|f| f.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), FIGURES.len());
        for f in FIGURES {
            let prefix = match f.pool {
                FigurePool::German => "de-",
                FigurePool::English => "en-",
            };
            assert!(f.key.starts_with(prefix), "{}", f.key);
        }
    }

    #[test]
    fn rare_names_get_a_quarter_of_the_weight() {
        for pool in FigurePool::ALL {
            let total: u32 = FigureId::pool(pool).map(|id| id.figure().weight()).sum();
            let rare: u32 = FigureId::pool(pool)
                .filter(|id| id.figure().rare)
                .map(|id| id.figure().weight())
                .sum();
            assert_eq!(rare * 4, total, "{pool:?}");
        }
    }

    #[test]
    fn the_ticket_walks_the_weight_intervals_exactly() {
        let pool = || FigureId::pool(FigurePool::German);
        // de-anna (3), de-sabine (3), …: the boundaries fall at 3 and 6.
        assert_eq!(pick(pool(), 0).key(), "de-anna");
        assert_eq!(pick(pool(), 2).key(), "de-anna");
        assert_eq!(pick(pool(), 3).key(), "de-sabine");
        // 4 × 3 familiar women, then the rare de-adelheid (2).
        assert_eq!(pick(pool(), 12).key(), "de-adelheid");
        assert_eq!(pick(pool(), 13).key(), "de-adelheid");
        assert_eq!(pick(pool(), 14).key(), "de-hildegard");
        // The last ticket of 32 is the last figure.
        assert_eq!(pick(pool(), 31).key(), "de-friedrich-wilhelm");
    }

    #[test]
    fn the_cast_is_deterministic_distinct_and_from_its_own_pool() {
        for seed in 0..2_000 {
            let cast = SeriesCast::for_seed(seed);
            assert_eq!(cast, SeriesCast::for_seed(seed));
            for pool in FigurePool::ALL {
                let [left, right] = cast.pool(pool);
                assert_ne!(left, right, "seed {seed}");
                assert_eq!(left.figure().pool, pool);
                assert_eq!(right.figure().pool, pool);
                assert_eq!(cast.figure(pool, PlayerId(1)), Some(left));
                assert_eq!(cast.figure(pool, PlayerId(2)), Some(right));
                assert_eq!(cast.figure(pool, PlayerId::HUMAN), None);
            }
        }
    }

    #[test]
    fn draws_follow_the_weights() {
        // 20,000 series: the rare names hold about a quarter of the first
        // draws, each familiar name 3/32 and each rare one 2/32.
        let n = 20_000;
        let mut counts = [0u32; 24];
        let mut rare_first = 0;
        for seed in 0..n {
            let cast = SeriesCast::for_seed(seed);
            for pool in FigurePool::ALL {
                let first = cast.pool(pool)[0];
                counts[usize::from(first.0)] += 1;
                rare_first += u32::from(first.figure().rare);
            }
        }
        let share = f64::from(rare_first) / f64::from(2 * n as u32);
        assert!((share - 0.25).abs() < 0.015, "rare share {share}");
        for (i, &count) in counts.iter().enumerate() {
            let expected = f64::from(FIGURES[i].weight()) / 32.0 * n as f64;
            let deviation = (f64::from(count) - expected).abs() / expected;
            assert!(deviation < 0.1, "{} drawn {count}", FIGURES[i].key);
        }
    }

    #[test]
    fn different_seeds_give_different_casts() {
        let casts: std::collections::HashSet<_> = (0..200).map(SeriesCast::for_seed).collect();
        assert!(casts.len() > 150, "{} casts", casts.len());
    }
}
