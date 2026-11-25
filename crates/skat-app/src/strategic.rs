// SPDX-License-Identifier: EUPL-1.2

//! Descriptive strategic classification of reviewed moments and their
//! trend (#13). Input is what the shared coach already decided
//! ([`MomentKind`] and [`Concept`] of a [`ReviewMoment`]); nothing is
//! recomputed here. Only the moments a review selected (at most three per
//! recording) are counted, so the counts are never a prevalence or error
//! rate of ordinary play, and no mastery or learning effect follows.

use serde::{Deserialize, Serialize};
use skat_coach::{Concept, MomentKind};

use crate::replay_review::ReviewMoment;

/// Moments per window of the trend; a window pair needs twice this many.
pub const STRATEGIC_WINDOW: usize = 10;

/// One classified review moment in chronological order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicObservation {
    /// The lesson concept of the moment.
    pub concept: Concept,
    /// Weaker choice, avoided trap or neither.
    pub kind: MomentKind,
}

impl From<&ReviewMoment> for StrategicObservation {
    fn from(m: &ReviewMoment) -> Self {
        StrategicObservation {
            concept: m.moment.concept,
            kind: m.moment.kind,
        }
    }
}

/// Counts per [`MomentKind`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindCounts {
    /// Important decisions decided well (costly option avoided).
    pub good: u32,
    /// A weaker option chosen with a measurable loss.
    pub bad: u32,
    /// Neither.
    pub neutral: u32,
}

impl KindCounts {
    fn add(&mut self, kind: MomentKind) {
        match kind {
            MomentKind::Good => self.good += 1,
            MomentKind::Bad => self.bad += 1,
            MomentKind::Neutral => self.neutral += 1,
        }
    }

    /// All counted moments.
    pub fn total(&self) -> u32 {
        self.good + self.bad + self.neutral
    }
}

/// The 10 moments before the last 10 beside the last 10, per concept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicTrend {
    /// The earlier window.
    pub earlier: KindCounts,
    /// The most recent window.
    pub recent: KindCounts,
}

/// One concept's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicRow {
    /// The concept.
    pub concept: Concept,
    /// All its moments.
    pub counts: KindCounts,
    /// Present from twice [`STRATEGIC_WINDOW`] moments on.
    pub trend: Option<StrategicTrend>,
}

/// Classification of a chronological moment list; rows in
/// [`Concept::ALL`] order, concepts without moments omitted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicSummary {
    /// Per-concept rows.
    pub rows: Vec<StrategicRow>,
    /// All moments.
    pub total: KindCounts,
}

impl StrategicSummary {
    /// Classify `observations` (oldest first).
    pub fn of(observations: &[StrategicObservation]) -> StrategicSummary {
        let mut total = KindCounts::default();
        for o in observations {
            total.add(o.kind);
        }
        let rows = Concept::ALL
            .into_iter()
            .filter_map(|concept| {
                let kinds: Vec<MomentKind> = observations
                    .iter()
                    .filter(|o| o.concept == concept)
                    .map(|o| o.kind)
                    .collect();
                if kinds.is_empty() {
                    return None;
                }
                let count = |ks: &[MomentKind]| {
                    let mut c = KindCounts::default();
                    ks.iter().for_each(|k| c.add(*k));
                    c
                };
                let n = kinds.len();
                let trend = (n >= 2 * STRATEGIC_WINDOW).then(|| StrategicTrend {
                    earlier: count(&kinds[n - 2 * STRATEGIC_WINDOW..n - STRATEGIC_WINDOW]),
                    recent: count(&kinds[n - STRATEGIC_WINDOW..]),
                });
                Some(StrategicRow {
                    concept,
                    counts: count(&kinds),
                    trend,
                })
            })
            .collect();
        StrategicSummary { rows, total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(concept: Concept, kind: MomentKind) -> StrategicObservation {
        StrategicObservation { concept, kind }
    }

    #[test]
    fn empty_has_no_rows() {
        assert_eq!(StrategicSummary::of(&[]), StrategicSummary::default());
    }

    #[test]
    fn counts_per_concept_in_catalogue_order() {
        let s = StrategicSummary::of(&[
            obs(Concept::ProvenVoid, MomentKind::Bad),
            obs(Concept::HandStructure, MomentKind::Good),
            obs(Concept::ProvenVoid, MomentKind::Neutral),
            obs(Concept::ProvenVoid, MomentKind::Bad),
        ]);
        let concepts: Vec<_> = s.rows.iter().map(|r| r.concept).collect();
        assert_eq!(concepts, [Concept::HandStructure, Concept::ProvenVoid]);
        assert_eq!(
            s.rows[1].counts,
            KindCounts {
                good: 0,
                bad: 2,
                neutral: 1
            }
        );
        assert_eq!(
            s.total,
            KindCounts {
                good: 1,
                bad: 2,
                neutral: 1
            }
        );
        assert!(s.rows.iter().all(|r| r.trend.is_none()));
    }

    #[test]
    fn trend_needs_two_full_windows_and_keeps_order() {
        let mut list = Vec::new();
        // 10 early Bad, 10 recent Good: exact windows.
        list.extend((0..10).map(|_| obs(Concept::GameValue, MomentKind::Bad)));
        list.extend((0..9).map(|_| obs(Concept::GameValue, MomentKind::Good)));
        assert!(StrategicSummary::of(&list).rows[0].trend.is_none());
        list.push(obs(Concept::GameValue, MomentKind::Good));
        let t = StrategicSummary::of(&list).rows[0].trend.unwrap();
        assert_eq!(
            t.earlier,
            KindCounts {
                good: 0,
                bad: 10,
                neutral: 0
            }
        );
        assert_eq!(
            t.recent,
            KindCounts {
                good: 10,
                bad: 0,
                neutral: 0
            }
        );
        // A 21st moment shifts both windows; the oldest drops out.
        list.push(obs(Concept::GameValue, MomentKind::Neutral));
        let t = StrategicSummary::of(&list).rows[0].trend.unwrap();
        assert_eq!(
            t.earlier,
            KindCounts {
                good: 1,
                bad: 9,
                neutral: 0
            }
        );
        assert_eq!(
            t.recent,
            KindCounts {
                good: 9,
                bad: 0,
                neutral: 1
            }
        );
    }

    #[test]
    fn windows_are_per_concept() {
        let mut list: Vec<_> = (0..20)
            .map(|_| obs(Concept::RemainingTrumpf, MomentKind::Neutral))
            .collect();
        list.extend((0..19).map(|_| obs(Concept::ProvenVoid, MomentKind::Bad)));
        let s = StrategicSummary::of(&list);
        let by = |c| s.rows.iter().find(|r| r.concept == c).unwrap();
        assert!(by(Concept::RemainingTrumpf).trend.is_some());
        assert!(by(Concept::ProvenVoid).trend.is_none());
    }
}
