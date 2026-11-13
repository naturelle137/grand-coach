// SPDX-License-Identifier: EUPL-1.2

//! On-demand coaching of one earlier human decision in a recording (M7,
//! D-67): the historical comment uses only what that player could know
//! then; a separate, explicitly privileged hindsight comment evaluates the
//! options in the real deal. Both reuse `skat_coach::advise` /
//! `advise_known` and `review`; nothing here searches or ranks on its own.
//!
//! Replay jobs and results are their own types: they can never be passed to
//! [`crate::Session::finish_coach`], never enter the live coach's cache or
//! queue, and never reach opponents or practice attempts.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use skat_ai::{Budget, Cancel};
use skat_coach::{ComparedOption, MoveReview, Status, advise, advise_known, decision_seed, review};
use skat_core::{Action, Game, OmniscientView, Phase, PlayerView};

use crate::coach::{DecisionKey, Unavailable};
use crate::context::DecisionContext;
use crate::recording::{GameRecording, RecordingError};
use crate::replay_review::{ReviewState, ReviewStatus};
use crate::session::PlayerId;

/// Sampling base of every historical replay analysis. The seed of a job is
/// [`decision_seed`] of this constant and the permitted view: it uses no
/// recording, series or game seed and no concealed card, so the same view
/// and budget give the same historical comment whatever the hidden deal.
pub const REPLAY_COACH_SEED: u64 = 0x4D37_5245_504C_4159;

/// Wall-clock limit a frontend should give one replay analysis on top of its
/// sample and node budget ([`Cancel::with_deadline`]). Reaching it shows as
/// cancelled, never as a judgement.
pub const REPLAY_COACH_TIME_LIMIT: Duration = Duration::from_secs(60);

/// Whose knowledge a replay comment uses. Part of every request, result and
/// rendered comment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplayPerspective {
    /// Only what the human could know before the recorded action (default).
    #[default]
    Historical,
    /// Hindsight: every card of the recorded deal is known. A model
    /// comparison in the real deal, never proof that the decision was a
    /// mistake from the player's view.
    Hindsight,
}

/// One recorded action of the human, in game order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedDecision {
    /// Actions applied before it (the reconstruction prefix).
    pub prefix: usize,
    /// The recorded action, legal in the view before it.
    pub action: Action,
    /// Phase of the decision.
    pub phase: Phase,
    /// The trick, from 1, during card play.
    pub trick: Option<u8>,
    /// Number of legal actions then (1: forced).
    pub options: u8,
    /// Bid value reached then (to name "hold").
    pub bid: u16,
    /// Hand game, once decided then (to name announcements).
    pub hand_game: Option<bool>,
}

/// Identifies one replay request: the opened recording, the user's request
/// generation, the decision, the perspective, budget and seed. A result is
/// accepted only for exactly the outstanding key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ReplayKey {
    /// Unique per opened recording, even for identical files.
    source: u64,
    generation: u64,
    prefix: usize,
    perspective: ReplayPerspective,
    budget: Budget,
    seed: u64,
}

/// One replay analysis, possibly run on another thread. A historical job
/// holds only the human's permitted view; only a hindsight job, built
/// through the explicitly privileged path, also holds the real deal.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayCoachJob {
    key: ReplayKey,
    view: Box<PlayerView>,
    /// The real position; only for [`ReplayPerspective::Hindsight`].
    all: Option<Box<OmniscientView>>,
    chosen: Action,
}

impl ReplayCoachJob {
    /// The historical job: built from `view` alone; its seed depends only on
    /// the view ([`REPLAY_COACH_SEED`]).
    fn historical(
        source: u64,
        generation: u64,
        prefix: usize,
        view: PlayerView,
        chosen: Action,
        budget: Budget,
    ) -> ReplayCoachJob {
        ReplayCoachJob {
            key: ReplayKey {
                source,
                generation,
                prefix,
                perspective: ReplayPerspective::Historical,
                budget,
                seed: decision_seed(REPLAY_COACH_SEED, &view),
            },
            view: Box::new(view),
            all: None,
            chosen,
        }
    }

    /// The hindsight job: explicitly privileged, it also carries the real
    /// position `all` at that prefix.
    fn hindsight_privileged(
        source: u64,
        generation: u64,
        prefix: usize,
        view: PlayerView,
        all: OmniscientView,
        chosen: Action,
        budget: Budget,
    ) -> ReplayCoachJob {
        let mut job = ReplayCoachJob::historical(source, generation, prefix, view, chosen, budget);
        job.key.perspective = ReplayPerspective::Hindsight;
        job.all = Some(Box::new(all));
        job
    }

    /// Analyses and explains the decision with the shared coach. `cancel`
    /// (with [`REPLAY_COACH_TIME_LIMIT`]) stops it; a cancelled analysis is
    /// never shown as a comment.
    pub fn run(&self, cancel: Option<&Cancel>) -> ReplayCoachResult {
        ReplayCoachResult {
            key: self.key,
            outcome: comment(
                &self.view,
                self.all.as_deref(),
                self.chosen,
                self.key.budget,
                self.key.seed,
                cancel,
            ),
        }
    }

    /// Perspective of this analysis.
    pub fn perspective(&self) -> ReplayPerspective {
        self.key.perspective
    }

    /// `true` only for the privileged hindsight job.
    pub fn sees_all_cards(&self) -> bool {
        self.all.is_some()
    }

    /// Sampling seed (independent of every recorded seed and hidden card).
    pub fn seed(&self) -> u64 {
        self.key.seed
    }

    /// Work budget.
    pub fn budget(&self) -> Budget {
        self.key.budget
    }

    /// The permitted view analysed (the human's, before the action).
    pub fn view(&self) -> &PlayerView {
        &self.view
    }
}

/// The shared comment on `chosen` in `view`: `advise` with `seed` from the
/// permitted view alone, or `advise_known` with the real position `all`
/// (hindsight only); then `review`. Used by single comments and the
/// recording review (D-69).
pub(crate) fn comment(
    view: &PlayerView,
    all: Option<&OmniscientView>,
    chosen: Action,
    budget: Budget,
    seed: u64,
    cancel: Option<&Cancel>,
) -> Result<Box<ReplayComment>, Unavailable> {
    let explanation = match all {
        Some(all) => advise_known(view, all, budget, cancel),
        None => advise(view, budget, seed, cancel),
    };
    match explanation {
        Some(e) if e.status == Status::Cancelled => Err(Unavailable::Cancelled),
        Some(e) => match review(&e, chosen) {
            Some(review) => Ok(Box::new(ReplayComment {
                perspective: if all.is_some() {
                    ReplayPerspective::Hindsight
                } else {
                    ReplayPerspective::Historical
                },
                alternative: review
                    .chosen
                    .compared_with
                    .and_then(|a| e.option(a))
                    .cloned(),
                review,
            })),
            None => Err(Unavailable::Failed),
        },
        None => Err(Unavailable::Failed),
    }
}

/// A finished replay analysis for [`ReplayCoach::finish`]. Not a
/// [`crate::CoachResult`]: the live session cannot accept it.
///
/// ```compile_fail
/// fn leak(live: &mut skat_app::Session, replay: skat_app::ReplayCoachResult) {
///     let _ = live.finish_coach(replay);
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayCoachResult {
    key: ReplayKey,
    outcome: Result<Box<ReplayComment>, Unavailable>,
}

impl ReplayCoachResult {
    /// The comment, if the analysis finished (completely or within budget).
    pub fn comment(&self) -> Option<&ReplayComment> {
        self.outcome.as_deref().ok()
    }

    /// Perspective the result was computed with.
    pub fn perspective(&self) -> ReplayPerspective {
        self.key.perspective
    }
}

/// The structured comment on one recorded decision (M5 keys, D-13).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayComment {
    /// The perspective it was computed with; frontends label it.
    pub perspective: ReplayPerspective,
    /// The recorded choice judged by the shared coach: role, chosen and
    /// best option, comparison, evidence, lesson or fallback, basis.
    pub review: MoveReview,
    /// The legal alternative the chosen option is compared with: the best
    /// option, or the runner-up when the choice was rated best.
    pub alternative: Option<ComparedOption>,
}

/// Why a replay result was not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayStale {
    /// Computed for another recording, decision, perspective, budget or an
    /// earlier request.
    Stale,
}

/// What the replay coach shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReplayStatus {
    /// The recording has no recorded human action (e.g. an in-progress file
    /// before the human's first decision).
    NoDecisions,
    /// No decision selected.
    NotSelected,
    /// A decision is selected; nothing requested for this perspective.
    Idle,
    /// The requested analysis is running.
    Computing,
    /// The comment. Provisional unless its review status is complete.
    Ready(Box<ReplayComment>),
    /// Cancelled or failed; requesting again recomputes.
    Unavailable(Unavailable),
}

/// The replay coach's part of the recording window, as plain data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayCoachView {
    /// Every recorded human action, in game order.
    pub decisions: Vec<RecordedDecision>,
    /// Index into `decisions`.
    pub selected: Option<usize>,
    /// Current perspective; historical after every selection.
    pub perspective: ReplayPerspective,
    /// The human's permitted table before the selected action.
    pub context: Option<DecisionContext>,
    /// What to show.
    pub status: ReplayStatus,
    /// The recording review (D-69).
    pub review: ReviewStatus,
}

/// On-demand coaching of one opened recording. Holds the privileged
/// recording; hands out at most one outstanding job and accepts only its
/// result. Never touches a live session.
#[derive(Clone, Debug)]
pub struct ReplayCoach {
    pub(crate) source: u64,
    pub(crate) recording: GameRecording,
    pub(crate) decisions: Vec<RecordedDecision>,
    selected: Option<usize>,
    context: Option<DecisionContext>,
    perspective: ReplayPerspective,
    pub(crate) budget: Budget,
    generation: u64,
    outstanding: Option<ReplayKey>,
    status: ReplayStatus,
    /// The bounded recording review (D-69); its own requests and cache.
    pub(crate) review: ReviewState,
}

impl ReplayCoach {
    /// Validates the whole recording and lists the human's recorded actions.
    /// The human is [`PlayerId::HUMAN`] at the recorded dealer's table.
    pub fn open(recording: &GameRecording, budget: Budget) -> Result<ReplayCoach, RecordingError> {
        recording.reconstruct_privileged(recording.actions.len())?;
        static SOURCE: AtomicU64 = AtomicU64::new(1);
        let dealer = recording.context.dealer;
        let mut game = Game::from_deal(recording.deal, recording.rules);
        let mut decisions = Vec::new();
        for (prefix, &action) in recording.actions.iter().enumerate() {
            let seat = game.current_seat().expect("entire history validated");
            if PlayerId::on_seat(seat, dealer).is_human() {
                let view = game.view(seat);
                debug_assert!(view.legal_actions.contains(&action));
                decisions.push(RecordedDecision {
                    prefix,
                    action,
                    phase: view.phase,
                    trick: (view.phase == Phase::Spiel).then_some(view.tricks.len() as u8 + 1),
                    options: view.legal_actions.len().min(usize::from(u8::MAX)) as u8,
                    bid: view.bid,
                    hand_game: view.hand_game,
                });
            }
            game.apply(action).expect("entire history validated");
        }
        let status = if decisions.is_empty() {
            ReplayStatus::NoDecisions
        } else {
            ReplayStatus::NotSelected
        };
        Ok(ReplayCoach {
            source: SOURCE.fetch_add(1, Ordering::Relaxed),
            recording: recording.clone(),
            decisions,
            selected: None,
            context: None,
            perspective: ReplayPerspective::Historical,
            budget,
            generation: 0,
            outstanding: None,
            status,
            review: ReviewState::default(),
        })
    }

    /// Every recorded human action.
    pub fn decisions(&self) -> &[RecordedDecision] {
        &self.decisions
    }

    /// The opened recording (unchanged).
    pub fn recording(&self) -> &GameRecording {
        &self.recording
    }

    /// Work budget of new requests.
    pub fn budget(&self) -> Budget {
        self.budget
    }

    /// Selects decision `index` (or none). Restores the historical
    /// perspective and drops any outstanding or shown comment; nothing is
    /// computed until requested. Out-of-range indices select nothing.
    pub fn select(&mut self, index: Option<usize>) {
        let index = index.filter(|&i| i < self.decisions.len());
        self.invalidate();
        self.selected = index;
        self.perspective = ReplayPerspective::Historical;
        self.context = index.map(|i| self.context_of(i));
        self.status = self.resting();
    }

    /// Switches the perspective; drops any outstanding or shown comment.
    pub fn set_perspective(&mut self, perspective: ReplayPerspective) {
        self.invalidate();
        self.perspective = perspective;
        self.status = self.resting();
    }

    /// Sets the budget of later requests (tests, screenshots); drops any
    /// outstanding or shown comment and the recording review with its
    /// cached results, which were computed with the old budget.
    pub fn set_budget(&mut self, budget: Budget) {
        self.invalidate();
        self.review.reset();
        self.budget = budget;
        self.status = self.resting();
    }

    /// Requests the comment for the selected decision in the current
    /// perspective (also a retry). Any earlier request becomes stale.
    /// `None` without a selected decision, and while the recording review
    /// runs: at most one replay analysis at a time.
    pub fn request(&mut self) -> Option<ReplayCoachJob> {
        if self.review_running() {
            return None;
        }
        let d = self.decisions[self.selected?];
        self.invalidate();
        let job = match self.perspective {
            ReplayPerspective::Historical => ReplayCoachJob::historical(
                self.source,
                self.generation,
                d.prefix,
                self.human_view(d.prefix),
                d.action,
                self.budget,
            ),
            ReplayPerspective::Hindsight => self.hindsight_job_privileged(d),
        };
        self.outstanding = Some(job.key);
        self.status = ReplayStatus::Computing;
        Some(job)
    }

    /// Cancels the outstanding request: its result will be rejected.
    pub fn cancel(&mut self) {
        if self.outstanding.is_some() {
            self.invalidate();
            self.status = ReplayStatus::Unavailable(Unavailable::Cancelled);
        }
    }

    /// Accepts the result of the outstanding request only.
    pub fn finish(&mut self, result: ReplayCoachResult) -> Result<(), ReplayStale> {
        if self.outstanding != Some(result.key) {
            return Err(ReplayStale::Stale);
        }
        self.outstanding = None;
        self.status = match result.outcome {
            Ok(comment) => ReplayStatus::Ready(comment),
            Err(reason) => ReplayStatus::Unavailable(reason),
        };
        Ok(())
    }

    /// `true` while a request is outstanding.
    pub fn computing(&self) -> bool {
        self.outstanding.is_some()
    }

    /// The replay coach view.
    pub fn view(&self) -> ReplayCoachView {
        ReplayCoachView {
            decisions: self.decisions.clone(),
            selected: self.selected,
            perspective: self.perspective,
            context: self.context.clone(),
            status: self.status.clone(),
            review: self.review_status(),
        }
    }

    /// The explicitly privileged path: the real position at the decision.
    fn hindsight_job_privileged(&self, d: RecordedDecision) -> ReplayCoachJob {
        let position = self
            .recording
            .reconstruct_privileged(d.prefix)
            .expect("validated when opened");
        let view = position.state.seats[self.human_seat().index()].clone();
        ReplayCoachJob::hindsight_privileged(
            self.source,
            self.generation,
            d.prefix,
            view,
            position.state,
            d.action,
            self.budget,
        )
    }

    /// The permitted table before decision `index` (D-59 snapshot).
    pub(crate) fn context_of(&self, index: usize) -> DecisionContext {
        let d = self.decisions[index];
        let view = self.human_view(d.prefix);
        let key = DecisionKey {
            lineage: 0,
            number: self.recording.context.number,
            actions: d.prefix,
        };
        DecisionContext::new(key, self.recording.context.dealer, &view, d.action)
    }

    /// The human's historical permitted view before action `prefix`.
    pub(crate) fn human_view(&self, prefix: usize) -> PlayerView {
        let position = self
            .recording
            .reconstruct_privileged(prefix)
            .expect("validated when opened");
        position.state.seats[self.human_seat().index()].clone()
    }

    fn human_seat(&self) -> skat_core::Seat {
        PlayerId::HUMAN.seat(self.recording.context.dealer)
    }

    /// Rejects every result handed out so far.
    fn invalidate(&mut self) {
        self.generation += 1;
        self.outstanding = None;
    }

    /// The status without a request: a historical comment the recording
    /// review already computed for the selected decision is shown as is
    /// (same recording, view, seed and budget, D-69); hindsight is never
    /// cached.
    fn resting(&self) -> ReplayStatus {
        match (self.decisions.is_empty(), self.selected) {
            (true, _) => ReplayStatus::NoDecisions,
            (false, None) => ReplayStatus::NotSelected,
            (false, Some(i)) => match (self.perspective, self.review.cached(i)) {
                (ReplayPerspective::Historical, Some(comment)) => {
                    ReplayStatus::Ready(Box::new(comment.clone()))
                }
                _ => ReplayStatus::Idle,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{CardSet, Deal, Seat};

    fn herz() -> GameRecording {
        serde_json::from_str(include_str!("../tests/fixtures/recordings/herz.json")).unwrap()
    }

    const SMALL: Budget = Budget {
        samples: 4,
        nodes_per_task: 2_000_000,
    };

    /// Swaps cards `a` and `b` between the places of the deal.
    fn swapped(deal: Deal, a: skat_core::Card, b: skat_core::Card) -> Deal {
        let swap = |set: CardSet| {
            let mut out = set;
            if set.contains(a) != set.contains(b) {
                for c in [a, b] {
                    if out.contains(c) {
                        out.remove(c);
                    } else {
                        out.insert(c);
                    }
                }
            }
            out
        };
        Deal::new(Seat::ALL.map(|s| swap(deal.hand(s))), swap(deal.skat())).unwrap()
    }

    /// Cards the human may not see at `game` (seat `human`).
    fn concealed(game: &Game, human: Seat) -> CardSet {
        let all = game.omniscient_view();
        let mut hidden = CardSet::EMPTY;
        for seat in Seat::ALL.into_iter().filter(|&s| s != human) {
            if !(game.is_revealed() && game.declarer() == Some(seat)) {
                hidden |= all.hands[seat.index()];
            }
        }
        if all.seats[human.index()].own_skat.is_none() {
            hidden |= all.skat;
        }
        hidden
    }

    #[test]
    fn substituted_hidden_deal_with_the_same_view_gives_the_same_historical_comment() {
        let r = herz();
        let human = PlayerId::HUMAN.seat(r.context.dealer);
        let coach = ReplayCoach::open(&r, SMALL).unwrap();
        let mut compared = 0;
        // Bidding, Skat and card-play decisions.
        for d in coach.decisions().iter().copied() {
            let mut game = Game::from_deal(r.deal, r.rules);
            for &a in &r.actions[..d.prefix] {
                game.apply(a).unwrap();
            }
            let view = game.view(human);
            let hidden: Vec<_> = concealed(&game, human).iter().collect();
            let place = |card| {
                Seat::ALL
                    .into_iter()
                    .position(|s| r.deal.hand(s).contains(card))
            };
            let mut found = None;
            'pairs: for (i, &a) in hidden.iter().enumerate() {
                for &b in &hidden[i + 1..] {
                    if place(a) == place(b) {
                        continue;
                    }
                    let other = swapped(r.deal, a, b);
                    let mut replay = Game::from_deal(other, r.rules);
                    if r.actions[..d.prefix]
                        .iter()
                        .all(|&x| replay.apply(x).is_ok())
                        && replay.view(human) == view
                    {
                        found = Some(replay);
                        break 'pairs;
                    }
                }
            }
            let Some(replay) = found else { continue };
            assert_ne!(replay.omniscient_view(), game.omniscient_view());
            let a = ReplayCoachJob::historical(1, 1, d.prefix, view.clone(), d.action, SMALL);
            let b = ReplayCoachJob::historical(1, 1, d.prefix, replay.view(human), d.action, SMALL);
            assert_eq!(a, b, "the historical job carries only the permitted view");
            assert!(!a.sees_all_cards());
            // Run a few to keep the test quick; the jobs are equal anyway.
            if compared < 2 {
                assert_eq!(a.run(None), b.run(None));
                let known_a = ReplayCoachJob::hindsight_privileged(
                    1,
                    1,
                    d.prefix,
                    view.clone(),
                    game.omniscient_view(),
                    d.action,
                    SMALL,
                );
                let known_b = ReplayCoachJob::hindsight_privileged(
                    1,
                    1,
                    d.prefix,
                    replay.view(human),
                    replay.omniscient_view(),
                    d.action,
                    SMALL,
                );
                assert!(known_a.sees_all_cards());
                assert_ne!(known_a, known_b, "only hindsight sees the actual deal");
                let comment = known_a.run(None);
                let comment = comment.comment().unwrap();
                assert!(comment.review.basis.known);
                assert_eq!(comment.perspective, ReplayPerspective::Hindsight);
            }
            compared += 1;
        }
        assert!(compared >= 5, "{compared}");
    }
}
