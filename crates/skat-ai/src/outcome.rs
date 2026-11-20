// SPDX-License-Identifier: EUPL-1.2

//! Outcome-label pilot for the Drücken decision (#74, step 2).
//!
//! For one declarer view in the Drücken phase every legal discard pair is
//! tried in hypothetical games: the opponents' hidden cards are sampled
//! from the acting view's constraints (never copied from a real deal), the
//! public history is replayed, and the game is played out by frozen
//! [`Player`]s. The mean Seeger-Fabian score of the acting seat per
//! candidate is a *target* only. It is a game outcome against the given
//! frozen opponents, not a double-dummy estimate (D-40), and never sets
//! `exact` or a reliability flag. All samples are shared by all candidates
//! (paired comparison), so differences are less noisy than the levels.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use skat_analysis::sampling::HiddenCards;
use skat_core::{Action, Call, Deal, Game, Phase, PlayerView};

use crate::Player;

/// Mean outcome of one legal action over the sampled games.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateOutcome {
    /// The candidate discard.
    pub action: Action,
    /// Mean Seeger-Fabian points of the acting seat (thrown-in games cannot
    /// occur after a pickup).
    pub mean: f64,
}

/// Plays every legal discard of `view` out `samples` times against `players`
/// (indexed by seat, including the acting seat's own continuation).
///
/// Returns `None` if `view` is not a declarer's Drücken decision or its
/// hidden cards or public history are inconsistent. Deterministic for equal
/// `seed` and deterministic `players`.
pub fn discard_outcomes(
    view: &PlayerView,
    players: [&dyn Player; 3],
    samples: u32,
    seed: u64,
) -> Option<Vec<CandidateOutcome>> {
    if view.phase != Phase::Druecken || view.current_seat != Some(view.seat) {
        return None;
    }
    let hidden = HiddenCards::from_view(view).ok()?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut sums = vec![0.0; view.legal_actions.len()];
    for _ in 0..samples {
        let world = hidden.sample(&mut rng);
        for (sum, &option) in sums.iter_mut().zip(&view.legal_actions) {
            let mut game = replay(view, &world)?;
            game.apply(option).ok()?;
            while let Some(seat) = game.current_seat() {
                let decision = players[seat.index()].decide(&game.view(seat), None)?;
                game.apply(decision.action).ok()?;
            }
            *sum += f64::from(game.result()?.seeger_fabian()[view.seat.index()]);
        }
    }
    let n = f64::from(samples.max(1));
    Some(
        view.legal_actions
            .iter()
            .zip(sums)
            .map(|(&action, sum)| CandidateOutcome {
                action,
                mean: sum / n,
            })
            .collect(),
    )
}

/// The hypothetical game in `world` with the public history up to the
/// declarer's Drücken replayed. Once the declarer holds twelve cards, any
/// two of them rebuild the same position.
fn replay(view: &PlayerView, world: &skat_analysis::sampling::World) -> Option<Game> {
    let skat = view.hand.iter().take(2).collect();
    let mut hands = world.hands;
    hands[view.seat.index()] = view.hand - skat;
    let mut game = Game::from_deal(Deal::new(hands, skat).ok()?, view.config);
    let history = view
        .bidding
        .iter()
        .map(|r| match r.call {
            Call::Bid(v) => Action::Bid(v),
            Call::Hold(_) => Action::Hold,
            Call::Pass => Action::Pass,
        })
        .chain([Action::TakeSkat]);
    for action in history {
        game.apply(action).ok()?;
    }
    (game.phase() == Phase::Druecken && game.current_seat() == Some(view.seat)).then_some(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Opponent};
    use skat_core::{RuleConfig, Seat};

    /// A deal's first Drücken view under Classic play, if any.
    fn druecken_view(seed: u64) -> Option<PlayerView> {
        let classic = Opponent {
            profile: Level::Easy.profile(),
            seed: 0,
        };
        let mut game = Game::new(seed, RuleConfig::default());
        while let Some(seat) = game.current_seat() {
            let view = game.view(seat);
            if view.phase == Phase::Druecken {
                return Some(view);
            }
            game.apply(classic.decide(&view, None)?.action).ok()?;
        }
        None
    }

    fn easy() -> [Opponent; 3] {
        [0, 1, 2].map(|seed| Opponent {
            profile: Level::Easy.profile(),
            seed,
        })
    }

    #[test]
    fn scores_all_sixty_six_pairs_reproducibly() {
        let view = (0..50)
            .find_map(druecken_view)
            .expect("some deal is played");
        let players = easy();
        let p = [&players[0] as &dyn Player, &players[1], &players[2]];
        let a = discard_outcomes(&view, p, 2, 7).expect("declarer Drücken view");
        assert_eq!(a.len(), 66);
        assert!(a.iter().all(|c| c.mean.is_finite()));
        assert!(a.iter().any(|c| c.mean != a[0].mean), "pairs differ");
        assert_eq!(a, discard_outcomes(&view, p, 2, 7).unwrap());
    }

    #[test]
    fn other_phases_and_seats_give_no_labels() {
        let players = easy();
        let p = [&players[0] as &dyn Player, &players[1], &players[2]];
        let game = Game::new(1, RuleConfig::default());
        let seat = game.current_seat().unwrap();
        assert!(discard_outcomes(&game.view(seat), p, 1, 0).is_none());
        let view = (0..50).find_map(druecken_view).unwrap();
        let mut other = view.clone();
        other.seat = Seat::ALL[(view.seat.index() + 1) % 3];
        assert!(discard_outcomes(&other, p, 1, 0).is_none());
    }
}
