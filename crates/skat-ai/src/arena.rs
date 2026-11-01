// SPDX-License-Identifier: EUPL-1.2

//! Headless AI-vs-AI games for calibrating the strength levels (M4,
//! `docs/ai.md`). Uses the real engine, analysis and opponents; no
//! frontend.
//!
//! Comparison schedule: one deal is played six times, the two compared
//! profiles swapping seats so that each profile holds each seat (and so
//! each dealer position) equally often, both alone against two copies of
//! the other and in pairs. Duplicate play on the same cards removes most of
//! the luck of the deal.

use skat_core::{Game, GameResult, RuleConfig, Seat};

use crate::{Opponent, Player, Profile};

/// Plays one deal to the end; `players` are indexed by seat. Every action
/// comes from the deciding seat's own view and goes through the engine.
///
/// # Panics
///
/// If an opponent chooses no action or an illegal one.
pub fn play(deal_seed: u64, rules: RuleConfig, players: &[Opponent; 3]) -> Game {
    play_with(deal_seed, rules, [&players[0], &players[1], &players[2]])
}

/// [`play`] with any [`Player`]s, e.g. a
/// [`NeuralOpponent`](crate::NeuralOpponent) against Classic opponents.
///
/// # Panics
///
/// If a player chooses no action or an illegal one.
pub fn play_with(deal_seed: u64, rules: RuleConfig, players: [&dyn Player; 3]) -> Game {
    let mut game = Game::new(deal_seed, rules);
    while let Some(seat) = game.current_seat() {
        let view = game.view(seat);
        let decision = players[seat.index()]
            .decide(&view, None)
            .expect("an opponent always finds an action");
        game.apply(decision.action)
            .expect("an opponent only chooses legal actions");
    }
    game
}

/// Which profile sits on which seat in the six games of one deal: `true`
/// for profile A. Games 0–2 seat A alone on Vorhand, Mittelhand and
/// Hinterhand, games 3–5 seat B alone.
pub const SEATINGS: [[bool; 3]; 6] = [
    [true, false, false],
    [false, true, false],
    [false, false, true],
    [false, true, true],
    [true, false, true],
    [true, true, false],
];

/// Totals of one profile over some seat-games.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tally {
    /// Seat-games played (every seat of every game, thrown-in deals
    /// included).
    pub seat_games: u32,
    /// Seeger-Fabian points over those seat-games.
    pub points: i64,
    /// Games played as declarer.
    pub declarer_games: u32,
    /// Games won as declarer.
    pub declarer_won: u32,
    /// Seeger-Fabian points as declarer.
    pub declarer_points: i64,
    /// Seat-games as defender (a game was played, someone else declared).
    pub defender_games: u32,
    /// Seat-games as defender in which the declarer lost.
    pub defender_won: u32,
    /// Seeger-Fabian points as defender.
    pub defender_points: i64,
}

impl Tally {
    fn add(&mut self, seat: Seat, result: Option<&GameResult>) {
        self.seat_games += 1;
        let Some(result) = result else {
            return;
        };
        let points = i64::from(result.seeger_fabian()[seat.index()]);
        self.points += points;
        if result.outcome.declarer == seat {
            self.declarer_games += 1;
            self.declarer_won += u32::from(result.won);
            self.declarer_points += points;
        } else {
            self.defender_games += 1;
            self.defender_won += u32::from(!result.won);
            self.defender_points += points;
        }
    }

    /// Adds another tally.
    pub fn merge(&mut self, other: &Tally) {
        self.seat_games += other.seat_games;
        self.points += other.points;
        self.declarer_games += other.declarer_games;
        self.declarer_won += other.declarer_won;
        self.declarer_points += other.declarer_points;
        self.defender_games += other.defender_games;
        self.defender_won += other.defender_won;
        self.defender_points += other.defender_points;
    }

    /// Mean Seeger-Fabian points per seat-game.
    pub fn mean(&self) -> f64 {
        self.points as f64 / f64::from(self.seat_games.max(1))
    }
}

/// Result of one deal played with all six seatings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DealComparison {
    /// Profile A.
    pub a: Tally,
    /// Profile B.
    pub b: Tally,
    /// Thrown-in games among the six.
    pub thrown_in: u32,
}

impl DealComparison {
    /// Mean points per seat-game of A minus those of B: the statistic whose
    /// mean over deals measures A against B.
    pub fn difference(&self) -> f64 {
        self.a.mean() - self.b.mean()
    }
}

/// Plays `deal_seed` with all [`SEATINGS`] of profiles `a` and `b`.
/// `ai_seeds(game, seat)` gives each opponent's seed; it must not depend
/// on the deal seed.
pub fn compare(
    deal_seed: u64,
    rules: RuleConfig,
    a: Profile,
    b: Profile,
    ai_seeds: impl Fn(usize, Seat) -> u64,
) -> DealComparison {
    let opponent = |profile: Profile| {
        let ai_seeds = &ai_seeds;
        move |g: usize, seat: Seat| Opponent {
            profile,
            seed: ai_seeds(g, seat),
        }
    };
    compare_with(deal_seed, rules, opponent(a), opponent(b))
}

/// [`compare`] for any [`Player`]s: `a(game, seat)` and `b(game, seat)`
/// build the player of each side for one of the six games. Their seeds must
/// not depend on the deal seed.
pub fn compare_with<A: Player, B: Player>(
    deal_seed: u64,
    rules: RuleConfig,
    a: impl Fn(usize, Seat) -> A,
    b: impl Fn(usize, Seat) -> B,
) -> DealComparison {
    let mut out = DealComparison::default();
    for (g, seating) in SEATINGS.iter().enumerate() {
        let players: [Box<dyn Player>; 3] = Seat::ALL.map(|seat| -> Box<dyn Player> {
            if seating[seat.index()] {
                Box::new(a(g, seat))
            } else {
                Box::new(b(g, seat))
            }
        });
        let game = play_with(deal_seed, rules, [&*players[0], &*players[1], &*players[2]]);
        let result = game.result();
        out.thrown_in += u32::from(result.is_none());
        for seat in Seat::ALL {
            let tally = if seating[seat.index()] {
                &mut out.a
            } else {
                &mut out.b
            };
            tally.add(seat, result);
        }
    }
    out
}

/// Mean, standard error and a 95 % confidence interval of per-deal values
/// (normal approximation; deals are independent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    /// Number of values.
    pub n: usize,
    /// Mean.
    pub mean: f64,
    /// Standard error of the mean.
    pub std_error: f64,
}

impl Interval {
    /// Summarises `values`.
    pub fn of(values: &[f64]) -> Interval {
        let n = values.len();
        let mean = values.iter().sum::<f64>() / n.max(1) as f64;
        let variance = if n > 1 {
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        Interval {
            n,
            mean,
            std_error: (variance / n.max(1) as f64).sqrt(),
        }
    }

    /// Lower end of the two-sided 95 % interval.
    pub fn low(&self) -> f64 {
        self.mean - 1.96 * self.std_error
    }

    /// Upper end of the two-sided 95 % interval.
    pub fn high(&self) -> f64 {
        self.mean + 1.96 * self.std_error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    #[test]
    fn seatings_are_balanced() {
        for seat in 0..3 {
            let a = SEATINGS.iter().filter(|s| s[seat]).count();
            assert_eq!(a, 3, "seat {seat}");
        }
        let a_alone = SEATINGS
            .iter()
            .filter(|s| s.iter().filter(|&&x| x).count() == 1);
        assert_eq!(a_alone.count(), 3);
        let total: usize = SEATINGS.iter().flatten().filter(|&&x| x).count();
        assert_eq!(total, 9);
    }

    #[test]
    fn identical_profiles_compare_as_equal_and_reproducibly() {
        let easy = Level::Easy.profile();
        let rules = RuleConfig { kontra_re: true };
        for deal in 0..20 {
            let c = compare(deal, rules, easy, easy, |g, s| (g * 3 + s.index()) as u64);
            assert_eq!(c.a.seat_games, 9);
            assert_eq!(c.b.seat_games, 9);
            // The baseline ignores seeds: each seating plays the same game.
            assert!(c.thrown_in == 0 || c.thrown_in == 6);
            assert!(c.difference().abs() < 1e-9, "deal {deal}: {c:?}");
            let again = compare(deal, rules, easy, easy, |g, s| (g * 3 + s.index()) as u64);
            assert_eq!(c, again);
        }
    }

    #[test]
    fn interval_matches_hand_computation() {
        let i = Interval::of(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(i.n, 4);
        assert!((i.mean - 2.5).abs() < 1e-12);
        // Sample variance 5/3, standard error sqrt(5/12).
        assert!((i.std_error - (5.0f64 / 12.0).sqrt()).abs() < 1e-12);
        assert!(i.low() < i.mean && i.mean < i.high());
    }
}
