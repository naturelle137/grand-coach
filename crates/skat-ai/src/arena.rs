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

use skat_core::{Call, Game, GameType, KontraLevel, RuleConfig, Seat, Suit};

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

/// Contract bins in [`Tally::contracts`], in display order.
pub const CONTRACT_TYPES: [GameType; 6] = [
    GameType::Farbe(Suit::Kreuz),
    GameType::Farbe(Suit::Pik),
    GameType::Farbe(Suit::Herz),
    GameType::Farbe(Suit::Karo),
    GameType::Grand,
    GameType::Null,
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
    /// Bid calls, including Vorhand's acceptance at 18; not Hold calls.
    pub bids: u32,
    /// Hold calls during Reizen.
    pub holds: u32,
    /// Pass calls during Reizen.
    pub passes: u32,
    /// Declarer games by [`CONTRACT_TYPES`]; excludes defender seat-games
    /// and Einpassen. The sum equals `declarer_games`.
    pub contracts: [u32; 6],
    /// Declarer Hand games (overlaps the contract bins).
    pub hand: u32,
    /// Declarer games with Schneider announced, not merely achieved.
    pub schneider_announced: u32,
    /// Declarer games with Schwarz announced, not merely achieved.
    pub schwarz_announced: u32,
    /// Declarer Ouvert games.
    pub ouvert: u32,
    /// Played games in which this seat, as defender, said Kontra.
    pub kontra_said: u32,
    /// Declarer games in which a defender said Kontra against this seat.
    pub kontra_faced: u32,
    /// Declarer games in which this seat answered Kontra with Re.
    pub re_said: u32,
}

impl Tally {
    fn add(&mut self, seat: Seat, game: &Game) {
        self.seat_games += 1;
        for record in game.bidding_history().iter().filter(|r| r.seat == seat) {
            match record.call {
                Call::Bid(_) => self.bids += 1,
                Call::Hold(_) => self.holds += 1,
                Call::Pass => self.passes += 1,
            }
        }
        let Some(result) = game.result() else {
            return;
        };
        let points = i64::from(result.seeger_fabian()[seat.index()]);
        self.points += points;
        if result.outcome.declarer == seat {
            let contract = result.outcome.contract;
            let bin = CONTRACT_TYPES
                .iter()
                .position(|&g| g == contract.game)
                .unwrap();
            self.contracts[bin] += 1;
            self.hand += u32::from(contract.hand);
            self.schneider_announced += u32::from(contract.schneider);
            self.schwarz_announced += u32::from(contract.schwarz);
            self.ouvert += u32::from(contract.ouvert);
            self.declarer_games += 1;
            self.declarer_won += u32::from(result.won);
            self.declarer_points += points;
            self.kontra_faced += u32::from(game.kontra_by().is_some());
            self.re_said += u32::from(game.kontra() == KontraLevel::Re);
        } else {
            self.kontra_said += u32::from(game.kontra_by() == Some(seat));
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
        self.bids += other.bids;
        self.holds += other.holds;
        self.passes += other.passes;
        for (count, other) in self.contracts.iter_mut().zip(other.contracts) {
            *count += other;
        }
        self.hand += other.hand;
        self.schneider_announced += other.schneider_announced;
        self.schwarz_announced += other.schwarz_announced;
        self.ouvert += other.ouvert;
        self.kontra_said += other.kontra_said;
        self.kontra_faced += other.kontra_faced;
        self.re_said += other.re_said;
    }

    /// Reizen actions: denominator for bid, hold and pass action rates.
    pub fn bidding_actions(&self) -> u32 {
        self.bids + self.holds + self.passes
    }

    /// Seat-games ending in Einpassen, including participation in mixed
    /// tables. This is not a count of games caused by this profile alone.
    pub fn thrown_in(&self) -> u32 {
        self.seat_games - self.declarer_games - self.defender_games
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
            tally.add(seat, &game);
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
            assert_eq!(c.a, c.b);
            // The baseline ignores seeds: each seating plays the same game.
            assert!(c.thrown_in == 0 || c.thrown_in == 6);
            assert!(c.difference().abs() < 1e-9, "deal {deal}: {c:?}");
            let again = compare(deal, rules, easy, easy, |g, s| (g * 3 + s.index()) as u64);
            assert_eq!(c, again);
        }
    }

    #[test]
    fn passive_players_expose_einpassen_in_each_profiles_tally() {
        struct Passer;
        impl Player for Passer {
            fn decide(
                &self,
                view: &skat_core::PlayerView,
                _: Option<&crate::Cancel>,
            ) -> Option<crate::Decision> {
                assert!(view.legal_actions.contains(&skat_core::Action::Pass));
                Some(crate::Decision {
                    action: skat_core::Action::Pass,
                    status: None,
                    fallback: false,
                })
            }
        }
        let c = compare_with(
            42,
            RuleConfig { kontra_re: true },
            |_, _| Passer,
            |_, _| Passer,
        );
        assert_eq!(c.thrown_in, 6);
        assert_eq!(c.a, c.b);
        assert_eq!(c.a.seat_games, 9);
        assert_eq!(c.a.thrown_in(), 9);
        assert_eq!(c.a.bidding_actions(), 9);
        assert_eq!((c.a.bids, c.a.holds, c.a.passes), (0, 0, 9));
        assert_eq!(c.a.declarer_games + c.a.defender_games, 0);
        assert_eq!(c.a.contracts, [0; 6]);
        assert_eq!(c.a.mean(), 0.0);
        let mut merged = c.a;
        merged.merge(&c.b);
        assert_eq!(merged.thrown_in(), 18);
        assert_eq!(merged.bidding_actions(), 18);
    }

    #[test]
    fn kontra_and_re_are_attributed_to_the_acting_seats() {
        use skat_core::Action;
        for (say_re, re_count) in [(false, 0), (true, 1)] {
            let mut game = Game::new(42, RuleConfig { kontra_re: true });
            for action in [Action::Bid(18), Action::Hold, Action::Pass, Action::Pass] {
                game.apply(action).unwrap();
            }
            game.apply(Action::TakeSkat).unwrap();
            let mut saying = true;
            while let Some(seat) = game.current_seat() {
                let actions = game.view(seat).legal_actions;
                let wanted = if saying && actions.contains(&Action::Kontra) {
                    saying = false;
                    Some(Action::Kontra)
                } else if say_re && actions.contains(&Action::Re) {
                    Some(Action::Re)
                } else {
                    actions
                        .iter()
                        .copied()
                        .find(|a| matches!(a, Action::Discard(_) | Action::Play(_)))
                        .or_else(|| actions.contains(&Action::KeinRe).then_some(Action::KeinRe))
                        .or_else(|| {
                            actions
                                .contains(&Action::Announce(skat_core::Announcement::plain(
                                    GameType::Grand,
                                )))
                                .then_some(Action::Announce(skat_core::Announcement::plain(
                                    GameType::Grand,
                                )))
                        })
                };
                game.apply(wanted.unwrap_or(actions[0])).unwrap();
            }
            let by = game.kontra_by().expect("a defender said Kontra");
            let declarer = game.result().unwrap().outcome.declarer;
            assert_ne!(by, declarer);
            for seat in Seat::ALL {
                let mut t = Tally::default();
                t.add(seat, &game);
                let (said, faced, re) = if seat == declarer {
                    (0, 1, re_count)
                } else {
                    (u32::from(seat == by), 0, 0)
                };
                assert_eq!(
                    (t.kontra_said, t.kontra_faced, t.re_said),
                    (said, faced, re)
                );
            }
        }
    }

    #[test]
    fn contract_and_bidding_counts_follow_the_actual_seat_and_announcement() {
        use skat_core::{Action, Announcement, Phase};
        let mut combined = Tally::default();
        for (bin, game_type) in CONTRACT_TYPES.into_iter().enumerate() {
            // Include plain pickup games and Hand/Ouvert announcements.
            for special in [false, true] {
                let mut game = Game::new(42, RuleConfig { kontra_re: true });
                // Mittelhand bids, Vorhand holds, then both other seats pass.
                for action in [Action::Bid(18), Action::Hold, Action::Pass, Action::Pass] {
                    game.apply(action).unwrap();
                }
                game.apply(if special {
                    Action::Hand
                } else {
                    Action::TakeSkat
                })
                .unwrap();
                let announcement = Announcement {
                    game: game_type,
                    schneider: special && game_type != GameType::Null,
                    schwarz: special && game_type != GameType::Null,
                    ouvert: special,
                };
                while game.current_seat().is_some() {
                    let actions = game.legal_actions();
                    let action = if game.phase() == Phase::Ansage {
                        Action::Announce(announcement)
                    } else {
                        *actions
                            .iter()
                            .find(|a| {
                                matches!(
                                    a,
                                    Action::Play(_)
                                        | Action::Discard(_)
                                        | Action::KeinKontra
                                        | Action::KeinRe
                                )
                            })
                            .unwrap()
                    };
                    game.apply(action).unwrap();
                }
                let result = game.result().unwrap();
                assert_eq!(result.outcome.declarer, Seat::Vorhand);
                let mut tallies = [Tally::default(); 3];
                for seat in Seat::ALL {
                    let t = &mut tallies[seat.index()];
                    t.add(seat, &game);
                    assert_eq!(t.thrown_in(), 0);
                    assert_eq!(t.points, i64::from(result.seeger_fabian()[seat.index()]));
                    if seat == Seat::Vorhand {
                        let mut bins = [0; 6];
                        bins[bin] = 1;
                        assert_eq!(t.contracts, bins);
                        assert_eq!((t.bids, t.holds, t.passes), (0, 1, 0));
                        assert_eq!(t.declarer_games, 1);
                        assert_eq!(t.declarer_won, u32::from(result.won));
                        assert_eq!(t.declarer_points, t.points);
                        assert_eq!(t.hand, u32::from(special));
                        assert_eq!(t.ouvert, u32::from(special));
                        assert_eq!(t.schneider_announced, u32::from(announcement.schneider));
                        assert_eq!(t.schwarz_announced, u32::from(announcement.schwarz));
                    } else {
                        assert_eq!(t.contracts, [0; 6]);
                        assert_eq!(t.defender_games, 1);
                        assert_eq!(t.defender_won, u32::from(!result.won));
                        assert_eq!(t.defender_points, t.points);
                        assert_eq!(
                            (t.hand, t.ouvert, t.schneider_announced, t.schwarz_announced),
                            (0, 0, 0, 0)
                        );
                    }
                    combined.merge(t);
                }
                assert_eq!(
                    (tallies[1].bids, tallies[1].holds, tallies[1].passes),
                    (1, 0, 1)
                );
                assert_eq!(
                    (tallies[2].bids, tallies[2].holds, tallies[2].passes),
                    (0, 0, 1)
                );
            }
        }
        assert_eq!(combined.seat_games, 36);
        assert_eq!(combined.contracts, [2; 6]);
        assert_eq!(
            (combined.bids, combined.holds, combined.passes),
            (12, 12, 24)
        );
        assert_eq!(combined.bidding_actions(), 48);
        assert_eq!(
            (
                combined.hand,
                combined.ouvert,
                combined.schneider_announced,
                combined.schwarz_announced
            ),
            (6, 6, 5, 5)
        );
        assert_eq!(
            (
                combined.declarer_games,
                combined.defender_games,
                combined.thrown_in()
            ),
            (12, 24, 0)
        );
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
