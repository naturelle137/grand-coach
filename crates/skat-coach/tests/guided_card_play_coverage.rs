// SPDX-License-Identifier: EUPL-1.2

//! Sampled completeness of the guided card-play dialogue: in every contract
//! family (Grand, all Farbspiele, Null; Hand, Ouvert and pickup games), each
//! core-reached card-play decision with at least two legal cards has a lesson.
//! A new Easy branch that the dialogue does not cover fails here by name.

#[path = "support/guided.rs"]
mod support;
use skat_coach::guided::{kontra_lesson, lesson};
use skat_core::*;
use support::*;

/// Replays bidding to the card play: Hand, or pickup with a fixed discard.
fn spiel(
    deal: Deal,
    declarer: Seat,
    ann: Announcement,
    pickup: bool,
    kontra_re: bool,
) -> Option<Game> {
    let cfg = RuleConfig { kontra_re };
    let mut g = Game::from_deal(deal, cfg);
    let mut spoke = [false; 3];
    while g.phase() != Phase::Spiel {
        let a = match g.phase() {
            Phase::Reizen if g.current_seat() == Some(declarer) => {
                let acts = g.legal_actions();
                // With Kontra/Re the declarer outbids the defenders' 18 (D-26.1).
                if kontra_re && acts.contains(&Action::Bid(20)) {
                    Action::Bid(20)
                } else if acts.contains(&Action::Bid(18)) {
                    Action::Bid(18)
                } else {
                    Action::Hold
                }
            }
            // A defender that bids or holds once is Kontra-eligible (D-26.1).
            Phase::Reizen if kontra_re && !spoke[g.current_seat().unwrap().index()] => {
                spoke[g.current_seat().unwrap().index()] = true;
                let acts = g.legal_actions();
                if acts.contains(&Action::Bid(18)) {
                    Action::Bid(18)
                } else if acts.contains(&Action::Hold) {
                    Action::Hold
                } else {
                    Action::Pass
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
    (g.view(declarer).declarer == Some(declarer)).then_some(g)
}

#[test]
fn every_multi_choice_card_play_decision_has_a_lesson() {
    let mut rng = 12345u64;
    let (mut seen, mut kontra_seen, mut missing) = (0u32, 0u32, Vec::new());
    let games = [
        GameType::Grand,
        GameType::Farbe(Suit::Kreuz),
        GameType::Farbe(Suit::Herz),
        GameType::Farbe(Suit::Pik),
        GameType::Farbe(Suit::Karo),
        GameType::Null,
    ];
    // The optional Kontra/Re setting runs on fewer deals: random play then
    // takes Kontra/Re at random, so the card decisions after it are sampled too.
    for (kontra_re, seeds) in [(false, 200u64), (true, 60)] {
        for seed in 0..seeds {
            for declarer in Seat::ALL {
                for game in games {
                    // (ouvert, pickup): Hand, Hand Ouvert, pickup game, pickup Null Ouvert.
                    for (ouvert, pickup) in
                        [(false, false), (true, false), (false, true), (true, true)]
                    {
                        if pickup && ouvert && game != GameType::Null {
                            continue; // Ouvert Farbspiel/Grand is Hand only
                        }
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
                            Announcement {
                                ouvert,
                                ..Announcement::plain(game)
                            }
                        } else {
                            ann
                        };
                        let Some(mut g) =
                            spiel(Deal::from_seed(seed), declarer, ann, pickup, kontra_re)
                        else {
                            continue;
                        };
                        while g.phase() == Phase::Spiel {
                            let seat = g.current_seat().unwrap();
                            let v = g.view(seat);
                            // A Kontra/Re decision belongs to `kontra_lesson`, not to the card lesson.
                            let call = v.legal_actions.iter().any(|a| {
                                matches!(
                                    a,
                                    Action::Kontra
                                        | Action::KeinKontra
                                        | Action::Re
                                        | Action::KeinRe
                                )
                            });
                            if call {
                                kontra_seen += 1;
                                if kontra_lesson(&v).is_none() {
                                    missing.push(format!(
                                    "kontra seed {seed} {game:?} ouvert={ouvert} pickup={pickup} \
                                     declarer={declarer:?} seat={seat:?}"
                                ));
                                }
                            }
                            let cards = v
                                .legal_actions
                                .iter()
                                .filter(|a| matches!(a, Action::Play(_)))
                                .count();
                            if cards >= 2 && !call {
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
    }
    assert!(
        kontra_seen > 100,
        "no Kontra/Re decisions sampled: {kontra_seen}"
    );
    assert!(seen > 50_000, "sample too small: {seen}");
    assert!(
        missing.is_empty(),
        "{} unsupported, first: {:?}",
        missing.len(),
        &missing[..missing.len().min(5)]
    );
}
