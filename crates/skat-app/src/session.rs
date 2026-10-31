// SPDX-License-Identifier: EUPL-1.2

//! The session: one series of games between the human and two opponents.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use skat_ai::{Budget, Cancel, Level, Mephisto, Opponent, Profile};
use skat_core::{Action, Event, Game, Phase, RuleConfig, RuleError, Seat};

use crate::coach::{
    CoachEvent, CoachJob, CoachResult, CoachSettings, CoachState, DecisionKey, Pending,
};
use crate::context::DecisionContext;
use crate::series::{GameRecord, ScoreSheet, SeriesResult};
use crate::view::{TableView, build_view};

/// A player of the series. Player 0 is the human; players 1 and 2 are the
/// opponents, 1 sitting to the human's left (plays next), 2 to the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u8);

impl PlayerId {
    /// The human player.
    pub const HUMAN: PlayerId = PlayerId(0);
    /// All players in clockwise order.
    pub const ALL: [PlayerId; 3] = [PlayerId(0), PlayerId(1), PlayerId(2)];

    /// Index `0..3`.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The next player clockwise (to the left).
    pub const fn next(self) -> PlayerId {
        PlayerId((self.0 + 1) % 3)
    }

    /// `true` for the human player.
    pub const fn is_human(self) -> bool {
        self.0 == 0
    }

    /// Seat of this player in a deal by `dealer` (the dealer is Hinterhand,
    /// the player to the dealer's left Vorhand).
    pub const fn seat(self, dealer: PlayerId) -> Seat {
        Seat::from_index((self.0 as usize + 5 - dealer.0 as usize) % 3)
    }

    /// Player on `seat` in a deal by `dealer`.
    pub const fn on_seat(seat: Seat, dealer: PlayerId) -> PlayerId {
        PlayerId(((dealer.0 as usize + 1 + seat.index()) % 3) as u8)
    }
}

/// Length of a series (D-05).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeriesLength {
    /// A fixed number of games, thrown-in deals included (D-31).
    Fixed(u16),
    /// Free play: the human ends the series between games.
    Free,
}

/// Settings of a new session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Seed of the series; it fixes every deal (D-06, D-22).
    pub seed: u64,
    /// Series length.
    pub series: SeriesLength,
    /// Kontra/Re (D-03).
    pub kontra_re: bool,
    /// The opponents.
    pub opponents: Opponents,
}

/// Who the opponents are (D-40, D-41).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opponents {
    /// Each opponent with its own level: index 0 is player 1 (left),
    /// index 1 player 2 (right).
    Levels([Level; 2]),
    /// One opponent, drawn per series, plays Strong but forgets played
    /// cards ([`Profile::GOLDFISH`]); the other plays Medium.
    Goldfish,
    /// Both opponents see every card before card play. In card play a
    /// computer declarer keeps seeing every card while its partner plays
    /// Strong; against the human declarer both defenders see every card.
    Mephisto,
}

impl Default for SessionConfig {
    fn default() -> SessionConfig {
        SessionConfig {
            seed: 0,
            series: SeriesLength::Fixed(36),
            kontra_re: false,
            opponents: Opponents::Levels([Level::Medium; 2]),
        }
    }
}

impl SessionConfig {
    /// Level of `player` when every opponent has one; `None` for the
    /// human and in the Goldfish and Mephisto modes.
    pub fn level(&self, player: PlayerId) -> Option<Level> {
        match (self.opponents, player.0) {
            (Opponents::Levels(levels), 1 | 2) => Some(levels[player.index() - 1]),
            _ => None,
        }
    }
}

/// What a frontend asks the session to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// A game action of the human player.
    Act(Action),
    /// Let the opponent whose turn it is make one move. Frontends call this
    /// at their own pace so the human can follow the opponents' moves.
    AdvanceOpponent,
    /// Deal the next game of the series.
    NextGame,
    /// End a free-play series between games.
    EndSeries,
    /// Change the coach settings; the game and the opponents are not
    /// affected. Allowed at any time.
    SetCoach(CoachSettings),
    /// Ask the coach about the current decision of the human.
    RequestAdvice,
    /// Put the coach's advice for the current decision away until the next
    /// decision or a new request; the game, the settings and the analyses
    /// are unchanged. Nothing is ever forced on the player.
    SkipAdvice,
    /// Play the action the warning held back, exactly once.
    ProceedAnyway,
    /// Withdraw the held action and decide again; the game is unchanged.
    Reconsider,
}

/// What happened. Contains only information the human may see.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppEvent {
    /// A new game was dealt.
    GameStarted {
        /// Game number in the series, from 1.
        number: u32,
        /// Dealer.
        dealer: PlayerId,
    },
    /// A game event from the rule engine (public information only). Its
    /// seats map to players through [`TableView::players`].
    Game {
        /// The event.
        event: Event,
        /// Player who acted or is named in the event, if any.
        player: Option<PlayerId>,
    },
    /// The game was finished or thrown in and entered on the score sheet.
    GameRecorded {
        /// The new line of the score sheet.
        record: GameRecord,
    },
    /// The series is over.
    SeriesFinished {
        /// Number of games played.
        games: u32,
    },
    /// The coach held back or released a human action.
    Coach(CoachEvent),
}

/// Why a command was rejected. The session is unchanged after an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppError {
    /// The rule engine rejected the action.
    Rule(RuleError),
    /// It is an opponent's turn.
    NotYourTurn,
    /// It is not an opponent's turn.
    NoOpponentToMove,
    /// The opponent move was computed for an earlier position.
    StaleOpponentMove,
    /// The current game is not over yet.
    GameInProgress,
    /// The series is over; start a new session.
    SeriesFinished,
    /// Only a free-play series can be ended by the player.
    NotFreePlay,
    /// The human has no decision to take right now.
    NoDecision,
    /// The coach is off.
    CoachOff,
    /// No action is held back by the coach.
    NothingPending,
    /// An action is held back by the coach: proceed or reconsider first.
    ActionPending,
    /// The coach result was computed for an earlier position or budget.
    StaleCoachResult,
}

/// An opponent decision to compute, possibly on another thread so the
/// frontend stays responsive. It holds the opponent's own view, which the
/// frontend cannot read.
#[derive(Clone, Debug)]
pub struct OpponentJob {
    level: Option<Level>,
    brain: Brain,
    position: Position,
}

/// How an opponent decides, with exactly the information it may use.
#[derive(Clone, Debug)]
enum Brain {
    /// From its own seat's view (D-23).
    Fair(Opponent, Box<skat_core::PlayerView>),
    /// With every card known (Mephisto, D-41).
    Mephisto(Box<Game>),
}

/// Identifies the position an opponent move was computed for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Position {
    number: u32,
    actions: usize,
    player: u8,
}

impl OpponentJob {
    /// Player who decides.
    pub fn player(&self) -> PlayerId {
        PlayerId(self.position.player)
    }

    /// Level of that player; `None` in the Goldfish and Mephisto modes.
    pub fn level(&self) -> Option<Level> {
        self.level
    }

    /// `true` if this decision is taken with every card known.
    pub fn sees_all_cards(&self) -> bool {
        matches!(self.brain, Brain::Mephisto(_))
    }

    /// Computes the decision. `cancel` stops the analysis early; the move
    /// is then chosen from what was finished, or by the rules of thumb,
    /// and is always legal.
    pub fn run(&self, cancel: Option<&Cancel>) -> OpponentMove {
        let decision = match &self.brain {
            Brain::Fair(opponent, view) => opponent.decide(view, cancel),
            Brain::Mephisto(game) => Mephisto.decide(game, cancel),
        }
        .expect("an opponent always finds a legal action");
        OpponentMove {
            position: self.position,
            action: decision.action,
        }
    }
}

/// A computed opponent move, to be passed to [`Session::finish_opponent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpponentMove {
    position: Position,
    action: Action,
}

/// A running series. All randomness comes from the configured seed, so the
/// same seed and the same human commands give the same series.
#[derive(Clone, Debug)]
pub struct Session {
    config: SessionConfig,
    /// Source of the per-deal seeds.
    rng: ChaCha8Rng,
    /// Source of the opponents' seeds: another stream, so they reveal
    /// nothing about the deals.
    ai_rng: ChaCha8Rng,
    /// Seeds of players 1 and 2 for the current game.
    ai_seeds: [u64; 2],
    /// The forgetful opponent in the Goldfish mode, drawn per series.
    goldfish: PlayerId,
    /// Actions applied in the current game.
    actions: usize,
    pub(crate) game: Game,
    game_seed: u64,
    number: u32,
    pub(crate) dealer: PlayerId,
    sheet: ScoreSheet,
    finished: bool,
    /// The live coach; it never influences deals or opponents.
    pub(crate) coach: CoachState,
}

impl Session {
    /// Starts a series and deals its first game. In the first game the
    /// human is Vorhand (player 2 deals).
    pub fn new(config: SessionConfig) -> Session {
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
        let game_seed = rng.next_u64();
        let mut ai_rng = ChaCha8Rng::seed_from_u64(config.seed);
        ai_rng.set_stream(1);
        let goldfish = PlayerId(1 + (ai_rng.next_u64() % 2) as u8);
        let ai_seeds = [ai_rng.next_u64(), ai_rng.next_u64()];
        // The coach's own stream: its seed reveals nothing about the deals
        // and leaves the opponents' seeds untouched.
        let mut coach_rng = ChaCha8Rng::seed_from_u64(config.seed);
        coach_rng.set_stream(2);
        Session {
            config,
            rng,
            ai_rng,
            ai_seeds,
            goldfish,
            actions: 0,
            game: Game::new(game_seed, rule_config(config)),
            game_seed,
            number: 1,
            dealer: PlayerId(2),
            sheet: ScoreSheet::default(),
            finished: matches!(config.series, SeriesLength::Fixed(0)),
            coach: CoachState::new(coach_rng.next_u64()),
        }
    }

    /// Settings of this session.
    pub fn config(&self) -> SessionConfig {
        self.config
    }

    /// Seed of the current deal (for "same deal again", D-06).
    pub fn game_seed(&self) -> u64 {
        self.game_seed
    }

    /// The human's view of the table.
    pub fn view(&self) -> TableView {
        let human_seat = PlayerId::HUMAN.seat(self.dealer);
        build_view(
            &self.config,
            self.number,
            self.dealer,
            &self.sheet,
            self.finished,
            self.opponent_to_move(),
            &self.game.view(human_seat),
            self.coach
                .view(self.human_decision(), self.game.current_seat().is_none()),
        )
    }

    /// Coach settings.
    pub fn coach_settings(&self) -> CoachSettings {
        self.coach.settings
    }

    /// Sets the work budget of coach analyses (default
    /// [`Budget::INTERACTIVE`]); for tests and benchmarks. Results computed
    /// with another budget are not used.
    pub fn set_coach_budget(&mut self, budget: Budget) {
        self.coach.budget = budget;
    }

    /// The current decision of the human, if any.
    fn human_decision(&self) -> Option<DecisionKey> {
        let seat = self.game.current_seat()?;
        (!self.finished && PlayerId::on_seat(seat, self.dealer).is_human()).then_some(DecisionKey {
            number: self.number,
            actions: self.actions,
        })
    }

    /// The coach analysis due now: for the human's current decision if the
    /// coach needs one and has none yet, otherwise for the oldest own
    /// decision played before its analysis finished (note and debriefing,
    /// [`CoachJob::is_review`]). Compute it with [`CoachJob::run`] (on any
    /// thread) and pass the result to [`Session::finish_coach`]. It holds
    /// only the human's view.
    pub fn coach_job(&self) -> Option<CoachJob> {
        if let Some(key) = self.human_decision()
            && self.coach.wanted(key)
            && self.coach.result(key).is_none()
        {
            let seat = PlayerId::HUMAN.seat(self.dealer);
            return Some(self.coach.job(key, self.game.view(seat)));
        }
        self.coach.review_job()
    }

    /// The finished coach analysis of the human's current decision, with
    /// the full ranking. For tools and tests only: frontends show the
    /// [`crate::CoachView`], which never names the best option before the
    /// move (D-49).
    pub fn coach_result(&self) -> Option<&CoachResult> {
        self.coach.result(self.human_decision()?)
    }

    /// Stores a coach result for the decision it was computed for. If the
    /// warning held an action back waiting for it, the action is now either
    /// played (not weak, or no finished analysis to judge it) or held with
    /// a warning. Fails without changes if the decision changed meanwhile.
    pub fn finish_coach(&mut self, result: CoachResult) -> Result<Vec<AppEvent>, AppError> {
        if self.coach.accept_review(&result) {
            return Ok(Vec::new());
        }
        let key = self.human_decision().ok_or(AppError::StaleCoachResult)?;
        if !self.coach.accept(key, result) {
            return Err(AppError::StaleCoachResult);
        }
        match self.coach.pending.clone() {
            Some(Pending::Checking { action }) => self.check(key, action),
            _ => Ok(Vec::new()),
        }
    }

    /// The warning check of a human action at `key`: plays it, holds it
    /// until the analysis is finished, or holds it with a warning.
    fn check(&mut self, key: DecisionKey, action: Action) -> Result<Vec<AppEvent>, AppError> {
        if !self.coach.settings.warnings {
            return self.play_own(key, action);
        }
        // Reject illegal actions before holding them back.
        self.game.validate(action).map_err(AppError::Rule)?;
        if self.coach.result(key).is_none() {
            self.coach.pending = Some(Pending::Checking { action });
            return Ok(vec![AppEvent::Coach(CoachEvent::Checking { action })]);
        }
        match self.coach.warning(key, action) {
            Some(warning) => {
                self.coach.pending = Some(Pending::Warning {
                    action,
                    warning: Box::new(warning),
                });
                Ok(vec![AppEvent::Coach(CoachEvent::Warned { action })])
            }
            None => self.play_own(key, action),
        }
    }

    /// Plays the human's own action at `key`; the coach keeps it for the
    /// note after the move and the debriefing (D-49).
    fn play_own(&mut self, key: DecisionKey, action: Action) -> Result<Vec<AppEvent>, AppError> {
        self.game.validate(action).map_err(AppError::Rule)?;
        let view = self.game.view(PlayerId::HUMAN.seat(self.dealer));
        let context = DecisionContext::new(key, self.dealer, &view, action);
        self.coach.record(key, &view, context);
        self.apply(action)
    }

    /// `true` if an opponent has to act.
    pub fn opponent_to_move(&self) -> bool {
        !self.finished
            && self
                .game
                .current_seat()
                .is_some_and(|s| !PlayerId::on_seat(s, self.dealer).is_human())
    }

    /// The opponent decision due now, if an opponent has to act. Compute it
    /// with [`OpponentJob::run`] (on any thread) and apply the result with
    /// [`Session::finish_opponent`].
    pub fn opponent_job(&self) -> Option<OpponentJob> {
        if !self.opponent_to_move() {
            return None;
        }
        let seat = self.game.current_seat()?;
        let player = PlayerId::on_seat(seat, self.dealer);
        let seed = self.ai_seeds[player.index() - 1];
        // A fair opponent sees only its own seat's view (D-23).
        let fair = |opponent: Opponent| Brain::Fair(opponent, Box::new(self.game.view(seat)));
        let brain = match self.config.opponents {
            Opponents::Levels(levels) => fair(Opponent::new(levels[player.index() - 1], seed)),
            Opponents::Goldfish if player == self.goldfish => fair(Opponent {
                profile: Profile::GOLDFISH,
                seed,
            }),
            Opponents::Goldfish => fair(Opponent::new(Level::Medium, seed)),
            Opponents::Mephisto => {
                let declarer = self.game.declarer();
                let partner = self.game.phase() == Phase::Spiel
                    && declarer.is_some_and(|d| {
                        d != seat && !PlayerId::on_seat(d, self.dealer).is_human()
                    });
                if partner {
                    fair(Opponent::new(Level::Strong, seed))
                } else {
                    Brain::Mephisto(Box::new(self.game.clone()))
                }
            }
        };
        Some(OpponentJob {
            level: self.config.level(player),
            brain,
            position: Position {
                number: self.number,
                actions: self.actions,
                player: player.0,
            },
        })
    }

    /// Applies an opponent move computed by [`OpponentJob::run`]. Fails
    /// without changes if the position changed in the meantime.
    pub fn finish_opponent(&mut self, mv: OpponentMove) -> Result<Vec<AppEvent>, AppError> {
        if self.finished {
            return Err(AppError::SeriesFinished);
        }
        match self.opponent_job() {
            Some(job) if job.position == mv.position => self.apply(mv.action),
            Some(_) => Err(AppError::StaleOpponentMove),
            None => Err(AppError::NoOpponentToMove),
        }
    }

    /// Runs opponent moves until the human has to act or the game is over.
    pub fn run_opponents(&mut self) -> Vec<AppEvent> {
        let mut events = Vec::new();
        while self.opponent_to_move() {
            events.extend(
                self.execute(Command::AdvanceOpponent)
                    .expect("opponent can move"),
            );
        }
        events
    }

    /// Executes a command. On error nothing changes.
    pub fn execute(&mut self, command: Command) -> Result<Vec<AppEvent>, AppError> {
        if let Command::SetCoach(settings) = command {
            self.coach.settings = settings;
            if !settings.warnings {
                // Nothing to check any more: the held action is withdrawn,
                // never played behind the human's back.
                self.coach.pending = None;
            }
            return Ok(Vec::new());
        }
        if self.finished {
            return Err(AppError::SeriesFinished);
        }
        match command {
            Command::Act(action) => {
                let seat = self.game.current_seat();
                if seat.is_some_and(|s| !PlayerId::on_seat(s, self.dealer).is_human()) {
                    return Err(AppError::NotYourTurn);
                }
                if self.coach.pending.is_some() {
                    return Err(AppError::ActionPending);
                }
                match self.human_decision() {
                    Some(key) => self.check(key, action),
                    None => self.apply(action),
                }
            }
            Command::SetCoach(_) => unreachable!("handled above"),
            Command::RequestAdvice => {
                let key = self.human_decision().ok_or(AppError::NoDecision)?;
                if self.coach.settings.level == crate::CoachLevel::Off {
                    return Err(AppError::CoachOff);
                }
                // A cancelled or failed analysis is computed again.
                if self.coach.result(key).is_some_and(|r| r.is_unavailable()) {
                    self.coach.cache = None;
                }
                // One more stage each time (D-49); nothing more once all
                // are shown.
                if let Some(stage) = self.coach.next_stage(key) {
                    self.coach.requested = Some((key, stage));
                }
                Ok(Vec::new())
            }
            Command::SkipAdvice => {
                let key = self.human_decision().ok_or(AppError::NoDecision)?;
                if self.coach.settings.level == crate::CoachLevel::Off {
                    return Err(AppError::CoachOff);
                }
                self.coach.skipped = Some(key);
                self.coach.requested = None;
                Ok(Vec::new())
            }
            Command::ProceedAnyway => {
                let key = self.human_decision().ok_or(AppError::NothingPending)?;
                let pending = self.coach.pending.take().ok_or(AppError::NothingPending)?;
                let action = pending.action();
                self.coach.acknowledged.push((key, action));
                self.play_own(key, action)
                    .inspect_err(|_| self.coach.pending = Some(pending))
            }
            Command::Reconsider => {
                let pending = self.coach.pending.take().ok_or(AppError::NothingPending)?;
                let action = pending.action();
                if let (Some(key), Pending::Warning { .. }) = (self.human_decision(), &pending) {
                    // Choosing it again counts as confirmed (no second
                    // warning for the same action and position).
                    self.coach.acknowledged.push((key, action));
                }
                Ok(vec![AppEvent::Coach(CoachEvent::Withdrawn { action })])
            }
            Command::AdvanceOpponent => {
                let job = self.opponent_job().ok_or(AppError::NoOpponentToMove)?;
                self.finish_opponent(job.run(None))
            }
            Command::NextGame => {
                self.ensure_game_over()?;
                Ok(self.next_game())
            }
            Command::EndSeries => {
                if self.config.series != SeriesLength::Free {
                    return Err(AppError::NotFreePlay);
                }
                self.ensure_game_over()?;
                self.finished = true;
                Ok(vec![AppEvent::SeriesFinished {
                    games: self.sheet.records.len() as u32,
                }])
            }
        }
    }

    fn ensure_game_over(&self) -> Result<(), AppError> {
        if self.game.current_seat().is_some() {
            Err(AppError::GameInProgress)
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, action: Action) -> Result<Vec<AppEvent>, AppError> {
        let core_events = self.game.apply(action).map_err(AppError::Rule)?;
        self.actions += 1;
        self.coach.decision_changed();
        let mut events: Vec<AppEvent> = core_events
            .into_iter()
            .map(|event| AppEvent::Game {
                event,
                player: event_seat(&event).map(|s| PlayerId::on_seat(s, self.dealer)),
            })
            .collect();
        if matches!(self.game.phase(), Phase::Beendet | Phase::Eingepasst) {
            events.extend(self.record_game());
        }
        Ok(events)
    }

    fn record_game(&mut self) -> Vec<AppEvent> {
        let dealer = self.dealer;
        let result = self.game.result().copied();
        let mut points = [0; 3];
        if let Some(result) = result {
            let by_seat = result.seeger_fabian();
            for seat in Seat::ALL {
                points[PlayerId::on_seat(seat, dealer).index()] = by_seat[seat.index()];
            }
        }
        let record = self.sheet.push(GameRecord {
            number: self.number,
            dealer,
            declarer: self.game.declarer().map(|s| PlayerId::on_seat(s, dealer)),
            contract: self.game.contract(),
            bid: if result.is_some() { self.game.bid() } else { 0 },
            result,
            points,
            totals: [0; 3],
        });
        let mut events = vec![AppEvent::GameRecorded { record }];
        if let SeriesLength::Fixed(n) = self.config.series
            && self.sheet.records.len() >= usize::from(n)
        {
            self.finished = true;
            events.push(AppEvent::SeriesFinished {
                games: self.sheet.records.len() as u32,
            });
        }
        events
    }

    fn next_game(&mut self) -> Vec<AppEvent> {
        self.number += 1;
        self.dealer = self.dealer.next();
        self.game_seed = self.rng.next_u64();
        self.ai_seeds = [self.ai_rng.next_u64(), self.ai_rng.next_u64()];
        self.actions = 0;
        self.coach.game_changed();
        self.game = Game::new(self.game_seed, rule_config(self.config));
        vec![AppEvent::GameStarted {
            number: self.number,
            dealer: self.dealer,
        }]
    }

    /// The forgetful opponent if the session plays the Goldfish mode.
    pub fn goldfish(&self) -> Option<PlayerId> {
        (self.config.opponents == Opponents::Goldfish).then_some(self.goldfish)
    }

    /// Score sheet so far.
    pub fn sheet(&self) -> &ScoreSheet {
        &self.sheet
    }

    /// Result of the series once it is over.
    pub fn series_result(&self) -> Option<SeriesResult> {
        self.finished.then(|| self.sheet.result())
    }
}

fn rule_config(config: SessionConfig) -> RuleConfig {
    RuleConfig {
        kontra_re: config.kontra_re,
    }
}

/// Seat named in a core event, if any.
fn event_seat(event: &Event) -> Option<Seat> {
    match *event {
        Event::Bid { seat, .. }
        | Event::Held { seat, .. }
        | Event::Passed { seat }
        | Event::Kontra { seat }
        | Event::KeinKontra { seat }
        | Event::Re { seat }
        | Event::KeinRe { seat }
        | Event::CardPlayed { seat, .. } => Some(seat),
        Event::BiddingWon { declarer, .. }
        | Event::HandChosen { declarer }
        | Event::SkatTaken { declarer }
        | Event::Discarded { declarer }
        | Event::Announced { declarer, .. }
        | Event::Revealed { declarer, .. } => Some(declarer),
        Event::TrickWon { trick } => Some(trick.winner),
        Event::GameFinished { result } => Some(result.outcome.declarer),
        Event::Eingepasst => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seats_rotate_with_the_dealer() {
        for dealer in PlayerId::ALL {
            assert_eq!(dealer.seat(dealer), Seat::Hinterhand);
            assert_eq!(dealer.next().seat(dealer), Seat::Vorhand);
            for player in PlayerId::ALL {
                assert_eq!(PlayerId::on_seat(player.seat(dealer), dealer), player);
            }
        }
        assert_eq!(PlayerId::HUMAN.seat(PlayerId(2)), Seat::Vorhand);
    }
}
