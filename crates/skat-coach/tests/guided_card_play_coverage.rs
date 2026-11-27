// SPDX-License-Identifier: EUPL-1.2

//! Sampled completeness of the guided card-play dialogue: in every contract
//! family (Grand, all Farbspiele, Null; Hand, Ouvert and pickup games), each
//! core-reached card-play decision with at least two legal cards has a lesson.
//! A new Easy branch that the dialogue does not cover fails here by name.

#[path = "support/guided.rs"]
mod support;
use skat_coach::guided::lesson;
use skat_core::*;
use support::*;

/// Replays bidding to the card play: Hand, or pickup with a fixed discard.
fn spiel(deal: Deal, declarer: Seat, ann: Announcement, pickup: bool) -> Option<Game> {
    let mut g = Game::from_deal(deal, RuleConfig::default());
    while g.phase() != Phase::Spiel {
        let a = match g.phase() {
            Phase::Reizen if g.current_seat() == Some(declarer) => {
                if g.legal_actions().contains(&Action::Bid(18)) {
                    Action::Bid(18)
                } else {
                    Action::Hold
                }
            }
            Phase::Reizen => Action::Pass,
            Phase::SkatAufnahme if pickup => Action::TakeSkat,
            Phase::SkatAufnahme => Action::Hand,
            Phase::Druecken => Action::Discard(g.view(declarer).hand.iter().take(2).collect()),
            Phase::Ansage => Action::Announce(ann),
            _ => return None,
        };
        g.apply(a).ok()?;
    }
    Some(g)
}

#[test]
fn every_multi_choice_card_play_decision_has_a_lesson() {
    let mut rng = 12345u64;
    let (mut seen, mut missing) = (0u32, Vec::new());
    let games = [
        GameType::Grand,
        GameType::Farbe(Suit::Kreuz),
        GameType::Farbe(Suit::Herz),
        GameType::Farbe(Suit::Pik),
        GameType::Farbe(Suit::Karo),
        GameType::Null,
    ];
    for seed in 0..200u64 {
        for declarer in Seat::ALL {
            for game in games {
                // (ouvert, pickup): Hand, Hand Ouvert, pickup game.
                for (ouvert, pickup) in [(false, false), (true, false), (false, true)] {
                    let ann = if game == GameType::Null {
                        Announcement {
                            game,
                            schneider: false,
                            schwarz: false,
                            ouvert,
                        }
                    } else {
                        hand_game(game, ouvert)
                    };
                    let ann = if pickup {
                        Announcement::plain(game)
                    } else {
                        ann
                    };
                    let Some(mut g) = spiel(Deal::from_seed(seed), declarer, ann, pickup) else {
                        continue;
                    };
                    while g.phase() == Phase::Spiel {
                        let seat = g.current_seat().unwrap();
                        let v = g.view(seat);
                        if v.legal_actions.len() >= 2 {
                            seen += 1;
                            if lesson(&v).is_none() {
                                missing.push(format!(
                                    "seed {seed} {game:?} ouvert={ouvert} pickup={pickup} \
                                     declarer={declarer:?} seat={seat:?} trick={:?}",
                                    v.current_trick.cards
                                ));
                            }
                        }
                        let acts = g.legal_actions();
                        rng = rng
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1442695040888963407);
                        g.apply(acts[(rng >> 33) as usize % acts.len()]).unwrap();
                    }
                }
            }
        }
    }
    assert!(seen > 50_000, "sample too small: {seen}");
    assert!(
        missing.is_empty(),
        "{} unsupported, first: {:?}",
        missing.len(),
        &missing[..missing.len().min(5)]
    );
}
