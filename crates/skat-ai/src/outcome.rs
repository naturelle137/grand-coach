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
use skat_core::{Action, Call, Deal, Game, KontraLevel, Phase, PlayerView, Trick};

use crate::Player;

/// Mean outcome of one legal action over the sampled games.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
/// Returns `None` if `samples` is zero, `view` is not a declarer's Drücken
/// decision, or its hidden cards or public history are inconsistent. Deterministic for equal
/// `seed` and deterministic `players`.
pub fn discard_outcomes(
    view: &PlayerView,
    players: [&dyn Player; 3],
    samples: u32,
    seed: u64,
) -> Option<Vec<CandidateOutcome>> {
    if samples == 0 || view.phase != Phase::Druecken || view.current_seat != Some(view.seat) {
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
    let n = f64::from(samples);
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

/// One Drücken decision with its outcome labels.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DrueckenRow {
    /// Deal seed of the table game that reached this decision (reproduction
    /// only; never a model input).
    pub deal: u64,
    /// The declarer's permitted view.
    pub view: PlayerView,
    /// Mean outcome per legal pair, in `view.legal_actions` order.
    pub outcomes: Vec<CandidateOutcome>,
}

/// The final bid is at least 18, no call exceeds it and the declarer made it.
fn valid_winning_bid(view: &PlayerView) -> bool {
    let value = |call: Call| match call {
        Call::Bid(v) | Call::Hold(v) => Some(v),
        Call::Pass => None,
    };
    view.bid >= 18
        && view
            .bidding
            .iter()
            .filter_map(|r| value(r.call))
            .all(|v| v <= view.bid)
        && view
            .bidding
            .iter()
            .any(|r| Some(r.seat) == view.declarer && value(r.call) == Some(view.bid))
}

/// Checks that a row contains exactly the 66 distinct legal pairs and finite labels.
/// This checks structure, not label quality or provenance.
pub fn valid_druecken_row(row: &DrueckenRow) -> bool {
    let view = &row.view;
    if view.phase != Phase::Druecken
        || view.current_seat != Some(view.seat)
        || view.declarer != Some(view.seat)
        || view.hand.len() != 12
        || view
            .hand_sizes
            .iter()
            .enumerate()
            .any(|(seat, &size)| size != if seat == view.seat.index() { 12 } else { 10 })
        || view.hand_game != Some(false)
        || view.contract.is_some()
        || view.own_skat.is_some()
        || view.declarer_cards.is_some()
        || !view.tricks.is_empty()
        || view.current_trick != Trick::default()
        || view.kontra != KontraLevel::None
        || view.result.is_some()
        || view.legal_actions.len() != 66
        || row.outcomes.len() != 66
        || !valid_winning_bid(view)
    {
        return false;
    }
    for (i, outcome) in row.outcomes.iter().enumerate() {
        let Action::Discard(cards) = outcome.action else {
            return false;
        };
        if cards.len() != 2
            || !cards.is_subset(view.hand)
            || !outcome.mean.is_finite()
            || !view.legal_actions.contains(&outcome.action)
            || row.outcomes[..i]
                .iter()
                .any(|other| other.action == outcome.action)
        {
            return false;
        }
    }
    true
}

/// Plays the deals `first_deal..first_deal + deals` with `table` at all
/// seats up to the declarer's Drücken and labels each such decision by
/// [`discard_outcomes`] against `frozen`. Deals without a declarer (all
/// pass) yield no row.
pub fn discard_dataset(
    table: &dyn Player,
    frozen: [&dyn Player; 3],
    first_deal: u64,
    deals: u64,
    samples: u32,
    seed: u64,
) -> Vec<DrueckenRow> {
    (first_deal..first_deal + deals)
        .filter_map(|deal| {
            let mut game = Game::new(deal, skat_core::RuleConfig::default());
            while let Some(seat) = game.current_seat() {
                let view = game.view(seat);
                if view.phase == Phase::Druecken {
                    let outcomes = discard_outcomes(&view, frozen, samples, seed ^ deal)?;
                    return Some(DrueckenRow {
                        deal,
                        view,
                        outcomes,
                    });
                }
                game.apply(table.decide(&view, None)?.action).ok()?;
            }
            None
        })
        .collect()
}

/// How a discard policy fares against the outcome labels of a dataset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscardComparison {
    /// Rows the policy answered with a legal pair.
    pub rows: u32,
    /// Mean label of the chosen pair.
    pub mean_chosen: f64,
    /// Mean label of the best pair (the target's own optimum).
    pub mean_best: f64,
    /// Share of rows where the policy chose a pair within `tie` of the best.
    pub best_share: f64,
}

/// Scores `policy` on `rows`. The labels are noisy estimates, so a choice
/// counts as best when it is within `tie` points of the best label. Rows the
/// policy cannot answer legally are left out of every figure.
pub fn compare_discards(rows: &[DrueckenRow], policy: &dyn Player, tie: f64) -> DiscardComparison {
    let (mut n, mut chosen, mut best, mut hits) = (0u32, 0.0, 0.0, 0u32);
    for row in rows {
        let top = row.outcomes.iter().map(|c| c.mean).fold(f64::MIN, f64::max);
        let pick = policy
            .decide(&row.view, None)
            .and_then(|d| row.outcomes.iter().find(|c| c.action == d.action));
        let Some(pick) = pick else { continue };
        n += 1;
        chosen += pick.mean;
        best += top;
        hits += u32::from(top - pick.mean <= tie);
    }
    let d = f64::from(n.max(1));
    DiscardComparison {
        rows: n,
        mean_chosen: chosen / d,
        mean_best: best / d,
        best_share: f64::from(hits) / d,
    }
}

/// Mean fresh-sample outcome of the pair selected as best on each original row.
/// Rows without matching views or labels are excluded. Use independently seeded
/// samples for `holdout`; this measures selection rather than a policy's strength.
pub fn holdout_best(rows: &[DrueckenRow], holdout: &[DrueckenRow]) -> Option<(usize, f64)> {
    let mut count = 0;
    let mut sum = 0.0;
    for row in rows {
        let Some(best) = row.outcomes.iter().max_by(|a, b| a.mean.total_cmp(&b.mean)) else {
            continue;
        };
        let Some(label) = holdout
            .iter()
            .find(|r| r.deal == row.deal && r.view == row.view)
            .and_then(|r| r.outcomes.iter().find(|c| c.action == best.action))
        else {
            continue;
        };
        count += 1;
        sum += label.mean;
    }
    (count > 0).then(|| (count, sum / count as f64))
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
    use skat_core::{
        Announcement, BidRecord, CompletedTrick, Contract, GameType, RuleConfig, Seat,
    };

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
        assert!(discard_outcomes(&view, p, 0, 7).is_none());
        let a = discard_outcomes(&view, p, 2, 7).expect("declarer Drücken view");
        assert_eq!(a.len(), 66);
        assert!(a.iter().all(|c| c.mean.is_finite()));
        assert!(a.iter().any(|c| c.mean != a[0].mean), "pairs differ");
        assert_eq!(a, discard_outcomes(&view, p, 2, 7).unwrap());
    }

    #[test]
    fn dataset_rows_are_labelled_and_compared() {
        let players = easy();
        let p = [&players[0] as &dyn Player, &players[1], &players[2]];
        let rows = discard_dataset(&players[0], p, 0, 12, 1, 3);
        assert!(!rows.is_empty() && rows.len() <= 12);
        assert!(rows.iter().all(|r| r.outcomes.len() == 66));
        assert_eq!(rows, discard_dataset(&players[0], p, 0, 12, 1, 3));
        let c = compare_discards(&rows, &players[0], 0.0);
        assert_eq!(c.rows as usize, rows.len());
        assert!(c.mean_chosen <= c.mean_best + 1e-9);
        assert!((0.0..=1.0).contains(&c.best_share));
        // A tolerance above the label range counts every choice as best.
        assert_eq!(compare_discards(&rows, &players[0], 1e9).best_share, 1.0);
    }

    #[test]
    fn validates_complete_distinct_finite_discard_labels() {
        let players = easy();
        let p = [&players[0] as &dyn Player, &players[1], &players[2]];
        let row = discard_dataset(&players[0], p, 0, 1, 1, 3).pop().unwrap();
        assert!(valid_druecken_row(&row));
        let mut bad = row.clone();
        bad.outcomes[1] = bad.outcomes[0];
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.outcomes[0].mean = f64::NAN;
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.outcomes.pop();
        assert!(!valid_druecken_row(&bad));
        let mut bad = row;
        bad.view.declarer = None;
        assert!(!valid_druecken_row(&bad));
    }

    #[test]
    fn rejects_contradictory_druecken_views() {
        let players = easy();
        let p = [&players[0] as &dyn Player, &players[1], &players[2]];
        let rows = discard_dataset(&players[0], p, 0, 12, 1, 3);
        assert!(!rows.is_empty());
        assert!(rows.iter().all(valid_druecken_row));
        let row = &rows[0];
        let mut bad = row.clone();
        bad.view.hand_game = Some(true);
        assert!(!valid_druecken_row(&bad));
        bad.view.hand_game = None;
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.contract = Some(Contract::new(Announcement::plain(GameType::Grand), false));
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        let cards: Vec<_> = row.view.hand.iter().take(3).collect();
        bad.view.tricks.push(CompletedTrick {
            leader: Seat::Vorhand,
            cards: [cards[0], cards[1], cards[2]],
            winner: Seat::Vorhand,
        });
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.current_trick.cards[0] = Some(cards[0]);
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.hand_sizes[row.view.seat.index()] = 10;
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.own_skat = Some(row.view.hand);
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.declarer_cards = Some(row.view.hand);
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.kontra = KontraLevel::Kontra;
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.bid = 0;
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.bid += 2;
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.bidding.clear();
        assert!(!valid_druecken_row(&bad));
        let mut bad = row.clone();
        bad.view.bidding.push(BidRecord {
            seat: row.view.seat,
            call: Call::Bid(row.view.bid + 2),
        });
        assert!(!valid_druecken_row(&bad));
    }

    #[test]
    fn holdout_scores_selected_pair_not_holdout_maximum() {
        let players = easy();
        let p = [&players[0] as &dyn Player, &players[1], &players[2]];
        let mut rows = discard_dataset(&players[0], p, 0, 1, 1, 3);
        let row = rows.first_mut().expect("played deal");
        row.outcomes.truncate(2);
        row.outcomes[0].mean = 10.0;
        row.outcomes[1].mean = 5.0;
        let mut fresh = rows.clone();
        fresh[0].outcomes[0].mean = -3.0;
        fresh[0].outcomes[1].mean = 20.0;
        assert_eq!(holdout_best(&rows, &fresh), Some((1, -3.0)));
        fresh[0].deal += 1;
        assert_eq!(holdout_best(&rows, &fresh), None);
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
