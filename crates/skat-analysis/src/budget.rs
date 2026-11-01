// SPDX-License-Identifier: EUPL-1.2

//! Deterministic work budgets and cancellation.
//!
//! Work is counted in search nodes, not time, so the same inputs, seed and
//! budget always give the same result. Time limits are layered on top by
//! cancelling from outside ([`Cancel`]); a cancelled analysis reports this
//! in its status and never passes as complete.

use std::cell::Cell;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// How much work one analysis may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Budget {
    /// Hypothetical distributions of the hidden cards to evaluate. If there
    /// are no more distributions than this, all are enumerated instead.
    pub samples: u32,
    /// Search nodes allowed per task. A task evaluates every option on one
    /// distribution (for Drücken: one option on one distribution). A task
    /// that runs out drops its distribution and the analysis reports
    /// [`Status::BudgetExhausted`](crate::Status::BudgetExhausted).
    pub nodes_per_task: u64,
}

impl Budget {
    /// Budget for interactive use (see `docs/analysis.md`).
    pub const INTERACTIVE: Budget = Budget {
        samples: 24,
        nodes_per_task: 100_000_000,
    };
}

impl Default for Budget {
    fn default() -> Budget {
        Budget::INTERACTIVE
    }
}

/// Shared cancellation flag, optionally with a deadline. Clone it, hand one
/// copy to the analysis and call [`Cancel::cancel`] on the other, e.g. from
/// the UI thread. A deadline makes the result depend on timing; only the
/// status tells whether the analysis finished.
#[derive(Clone, Debug, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl Cancel {
    /// A flag that is not set.
    pub fn new() -> Cancel {
        Cancel::default()
    }

    /// A flag that also counts as set from `deadline` on.
    pub fn with_deadline(deadline: Instant) -> Cancel {
        Cancel {
            flag: Arc::default(),
            deadline: Some(deadline),
        }
    }

    /// Asks the analysis to stop as soon as possible.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// `true` once [`Cancel::cancel`] was called or the deadline passed.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed) || self.deadline.is_some_and(|d| Instant::now() >= d)
    }
}

thread_local! {
    /// Thread limit of analyses started on this thread; `None`: all cores.
    static MAX_THREADS: Cell<Option<NonZeroUsize>> = const { Cell::new(None) };
}

/// Runs `f` with every analysis it starts on the calling thread limited to
/// `threads` worker threads (by default an analysis uses all cores). Tools
/// that run several analyses side by side use this to stay within a CPU
/// budget. Results do not depend on the thread count. The previous limit
/// is restored afterwards, also on panic.
pub fn with_max_threads<T>(threads: NonZeroUsize, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<NonZeroUsize>);
    impl Drop for Restore {
        fn drop(&mut self) {
            MAX_THREADS.with(|m| m.set(self.0));
        }
    }
    let _restore = Restore(MAX_THREADS.with(|m| m.replace(Some(threads))));
    f()
}

/// Worker threads an analysis started on the calling thread may use: the
/// limit of [`with_max_threads`], or all cores.
pub fn max_threads() -> usize {
    MAX_THREADS
        .with(Cell::get)
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, NonZeroUsize::get)
}

/// Why a computation stopped before finishing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Interrupted {
    /// The node budget is used up.
    BudgetExhausted,
    /// [`Cancel::cancel`] was called.
    Cancelled,
}

/// Counts search nodes against a limit and watches a [`Cancel`] flag.
#[derive(Clone, Debug)]
pub struct Meter {
    nodes: u64,
    limit: u64,
    cancel: Option<Cancel>,
}

/// Nodes between two looks at the cancellation flag.
const CANCEL_CHECK: u64 = 1024;

impl Meter {
    /// A meter allowing `limit` nodes, optionally watching `cancel`.
    pub fn new(limit: u64, cancel: Option<Cancel>) -> Meter {
        Meter {
            nodes: 0,
            limit,
            cancel,
        }
    }

    /// A meter without limit or cancellation (tests, small positions).
    pub fn unlimited() -> Meter {
        Meter::new(u64::MAX, None)
    }

    /// Nodes counted so far.
    pub fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Counts one node.
    pub fn tick(&mut self) -> Result<(), Interrupted> {
        self.nodes += 1;
        if self.nodes > self.limit {
            return Err(Interrupted::BudgetExhausted);
        }
        if self.nodes.is_multiple_of(CANCEL_CHECK) {
            self.check_cancel()?;
        }
        Ok(())
    }

    /// Fails if cancellation was requested.
    pub fn check_cancel(&self) -> Result<(), Interrupted> {
        match &self.cancel {
            Some(cancel) if cancel.is_cancelled() => Err(Interrupted::Cancelled),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_stops_at_limit_and_on_cancel() {
        let mut meter = Meter::new(3, None);
        assert!(meter.tick().is_ok());
        assert!(meter.tick().is_ok());
        assert!(meter.tick().is_ok());
        assert_eq!(meter.tick(), Err(Interrupted::BudgetExhausted));

        let cancel = Cancel::new();
        let mut meter = Meter::new(u64::MAX, Some(cancel.clone()));
        assert!(meter.check_cancel().is_ok());
        cancel.cancel();
        assert_eq!(meter.check_cancel(), Err(Interrupted::Cancelled));
        let failed = (0..CANCEL_CHECK).map(|_| meter.tick()).any(|r| r.is_err());
        assert!(failed);

        let past = Cancel::with_deadline(Instant::now());
        assert!(past.is_cancelled());
        let later = Cancel::with_deadline(Instant::now() + std::time::Duration::from_secs(3600));
        assert!(!later.is_cancelled());
    }

    #[test]
    fn thread_limit_is_scoped() {
        let all = max_threads();
        let two = NonZeroUsize::new(2).unwrap();
        let one = NonZeroUsize::MIN;
        with_max_threads(two, || {
            assert_eq!(max_threads(), 2);
            with_max_threads(one, || assert_eq!(max_threads(), 1));
            assert_eq!(max_threads(), 2);
            // Other threads keep their own limit.
            std::thread::scope(|s| {
                s.spawn(|| assert_eq!(max_threads(), all));
            });
        });
        assert_eq!(max_threads(), all);
    }
}
