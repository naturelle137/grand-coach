// SPDX-License-Identifier: EUPL-1.2

//! M7 closed-evidence acceptance (issue #12): what the recorded human could
//! deduce at a decision, and that historical comments, their lessons and the
//! recording review (D-67, D-69) teach exactly that, never more.
//!
//! The expected facts are specified independently from the documented deal
//! of `fixtures/recordings/README.md` and the ISkO follow rule implemented
//! below in this file, not from the analysis under test.

use skat_app::coach_model::{
    Claim, Follow, MOMENT_POINTS, MomentKind, Observation, Role, Source, Status,
};
use skat_app::*;

fn fixture(name: &str) -> GameRecording {
    let json = match name {
        "herz" => include_str!("fixtures/recordings/herz.json"),
        "grand" => include_str!("fixtures/recordings/grand.json"),
        "null" => include_str!("fixtures/recordings/null.json"),
        _ => panic!("unknown fixture"),
    };
    serde_json::from_str(json).unwrap()
}

/// The same record as game 2 of its series: dealer player 0, so the human
/// (player 0) holds the Hinterhand cards and defends against player 1.
fn late(name: &str) -> GameRecording {
    let mut r = fixture(name);
    r.context.number = 2;
    r.context.dealer = PlayerId(0);
    r
}

/// Every fixed fixture, with the human as declarer (game 1) and defender.
fn all() -> Vec<(&'static str, GameRecording)> {
    vec![
        ("herz", fixture("herz")),
        ("late herz", late("herz")),
        ("grand", fixture("grand")),
        ("late grand", late("grand")),
        ("null", fixture("null")),
        ("late null", late("null")),
    ]
}

const SMALL: Budget = Budget {
    samples: 2,
    nodes_per_task: 1_000_000,
};

fn card(text: &str) -> Card {
    let suit = match text.as_bytes()[0] {
        b'C' => Suit::Kreuz,
        b'S' => Suit::Pik,
        b'H' => Suit::Herz,
        b'D' => Suit::Karo,
        _ => panic!("suit"),
    };
    let rank = match text.as_bytes()[1] {
        b'7' => Rank::Sieben,
        b'8' => Rank::Acht,
        b'9' => Rank::Neun,
        b'T' => Rank::Zehn,
        b'J' => Rank::Bube,
        b'Q' => Rank::Dame,
        b'K' => Rank::Koenig,
        b'A' => Rank::Ass,
        _ => panic!("rank"),
    };
    Card::new(suit, rank)
}

fn cards(text: &str) -> CardSet {
    text.split_whitespace().map(card).collect()
}

/// ISkO: Buben are Trumpf except in Null; in a Farbspiel its suit too.
fn trumpf(game: GameType, c: Card) -> bool {
    match game {
        GameType::Null => false,
        GameType::Grand => c.rank() == Rank::Bube,
        GameType::Farbe(s) => c.rank() == Rank::Bube || c.suit() == s,
    }
}

fn class(game: GameType, c: Card) -> Follow {
    if trumpf(game, c) {
        Follow::Trumpf
    } else {
        Follow::Farbe(c.suit())
    }
}

/// Every card shown on the table before the decision, trick by trick, in
/// play order (the leader first); the trick in progress last.
fn public_tricks(c: &DecisionContext) -> Vec<Vec<PlayedCard>> {
    let mut tricks: Vec<Vec<PlayedCard>> = c.tricks.iter().map(|t| t.cards.to_vec()).collect();
    if !c.current_trick.is_empty() {
        tricks.push(c.current_trick.clone());
    }
    tricks
}

/// The voids anyone at the table can prove: a player who did not follow
/// the led class, with the trick (from 1), led card and card played. The
/// human's own voids are no deduction.
fn expected_voids(c: &DecisionContext, game: GameType) -> Vec<(Seat, Follow, u8, Card, Card)> {
    let mut out: Vec<(Seat, Follow, u8, Card, Card)> = Vec::new();
    for (n, trick) in public_tricks(c).iter().enumerate() {
        let led = trick[0].card;
        for p in &trick[1..] {
            let follow = class(game, led);
            let seat = c.seat(p.player);
            if p.player != PlayerId::HUMAN
                && class(game, p.card) != follow
                && !out.iter().any(|v| (v.0, v.1) == (seat, follow))
            {
                out.push((seat, follow, n as u8 + 1, led, p.card));
            }
        }
    }
    out
}

fn played(c: &DecisionContext) -> CardSet {
    public_tricks(c).iter().flatten().map(|p| p.card).collect()
}

/// The voids the evidence states, with the trick that proves each.
fn stated_voids(review: &coach_model::MoveReview) -> Vec<(Seat, Follow, u8, Card, Card)> {
    let mut out: Vec<_> = review
        .evidence
        .items
        .iter()
        .filter_map(|item| {
            let Claim::Fact(Observation::Void { seat, follow }) = item.claim else {
                return None;
            };
            let source = item.sources.iter().find_map(|s| match *s {
                Source::Trick {
                    index,
                    seat: s,
                    led,
                    card,
                } if s == seat => Some((index + 1, led, card)),
                _ => None,
            });
            let (trick, led, card) = source.expect("a void cites its trick");
            Some((seat, follow, trick, led, card))
        })
        .collect();
    out.sort_by_key(|v| v.2);
    out
}

fn trumps_out(review: &coach_model::MoveReview) -> Option<u8> {
    review
        .evidence
        .items
        .iter()
        .find_map(|item| match item.claim {
            Claim::Fact(Observation::TrumpsOut { count }) => Some(count),
            _ => None,
        })
}

fn comment_at(coach: &mut ReplayCoach, prefix: usize) -> ReplayComment {
    let i = coach
        .decisions()
        .iter()
        .position(|d| d.prefix == prefix)
        .unwrap();
    coach.select(Some(i));
    let job = coach.request().unwrap();
    assert!(!job.sees_all_cards());
    coach.finish(job.run(None)).unwrap();
    match coach.view().status {
        ReplayStatus::Ready(c) => *c,
        other => panic!("not ready: {other:?}"),
    }
}

/// Late Herz (dealer player 0): the human holds the documented HH cards
/// C7 C8 S7 SK SA H8 HJ HA DT DQ and defends against player 1 (Vorhand,
/// dealt C9 CT CJ CQ H9 HT HQ HK D9 DK, took S9 DA, discarded C9 CT).
///
/// Tricks 1–3 as recorded: CJ SJ H8; CQ CK C7; then Mittelhand leads CA,
/// the human follows C8 and the declarer plays S9: from that public card on,
/// everyone knows the declarer holds no Kreuz. In trick 4 Mittelhand leads
/// S8 and the human decides BEFORE the declarer trumps with H9, so the Pik
/// void is not yet deducible there; it is at the human's lead in trick 6.
#[test]
fn proven_void_is_taught_from_the_trick_that_proves_it_never_earlier() {
    let r = late("herz");
    let game = GameType::Farbe(Suit::Herz);
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    let declarer = Seat::Vorhand;
    let kreuz = (
        declarer,
        Follow::Farbe(Suit::Kreuz),
        3,
        card("CA"),
        card("S9"),
    );
    let pik = (
        declarer,
        Follow::Farbe(Suit::Pik),
        4,
        card("S8"),
        card("H9"),
    );

    // Trick 2 (prefix 11): everybody followed so far, no void exists.
    let before = comment_at(&mut coach, 11);
    assert_eq!(before.review.role, Role::Defender);
    assert_eq!(stated_voids(&before.review), vec![]);
    // Trumpf: 11 = four Buben and seven Herz. Own HJ HA; played CJ SJ H8.
    assert_eq!(trumps_out(&before.review), Some(6));
    assert!(!matches!(
        before.review.lesson,
        Some(coach_model::LessonContent::ProvenVoid { .. })
    ));

    // Trick 4 (prefix 16): exactly the Kreuz void of trick 3.
    let at = comment_at(&mut coach, 16);
    assert_eq!(at.review.chosen.action, Action::Play(card("S7")));
    assert_eq!(stated_voids(&at.review), vec![kreuz]);
    assert_eq!(trumps_out(&at.review), Some(6));
    let Some(coach_model::LessonContent::ProvenVoid { game: g, voids }) = &at.review.lesson else {
        panic!("proven-void lesson expected: {:?}", at.review.lesson);
    };
    assert_eq!(*g, game);
    let taught: Vec<_> = voids
        .iter()
        .map(|v| (v.seat, v.follow, v.trick, v.led, v.card))
        .collect();
    assert_eq!(taught, vec![kreuz], "no Pik void before the declarer plays");
    // The legal alternative: another Pik (the human must follow Pik).
    let alternative = at.alternative.as_ref().unwrap().action;
    assert!([Action::Play(card("SK")), Action::Play(card("SA"))].contains(&alternative));

    // Trick 6 lead (prefix 21): both voids; trumps out HQ HK DJ (own HA;
    // played CJ SJ H8, H9, HT H7 HJ).
    let lead = comment_at(&mut coach, 21);
    assert_eq!(stated_voids(&lead.review), vec![kreuz, pik]);
    assert_eq!(trumps_out(&lead.review), Some(3));
    if let Some(coach_model::LessonContent::RemainingTrumpf { sets, .. }) = &lead.review.lesson {
        assert_eq!(sets.unseen, cards("HQ HK DJ"));
        assert_eq!(sets.own, cards("HA"));
    }

    // The recording review teaches the same void at the same decision, as
    // a lesson, not as blame: the choice is no "Bad" moment.
    coach.select(None);
    assert!(coach.request_review());
    while let Some(job) = coach.review_job() {
        coach.finish_review(job.run(None)).unwrap();
    }
    let ReviewStatus::Done(summary) = coach.review_status() else {
        panic!("review not done");
    };
    assert!(summary.moments.len() <= 3);
    let moment = summary
        .moments
        .iter()
        .find(|m| coach.decisions()[m.decision].prefix == 16)
        .expect("trick 4 is a moment");
    assert_eq!(moment.moment.concept, coach_model::Concept::ProvenVoid);
    assert_ne!(moment.moment.kind, MomentKind::Bad);
    assert_eq!(moment.moment.lesson, at.review.lesson.clone().unwrap());
}

/// Every analysed card decision of every fixture, as declarer and as
/// defender: stated voids and the Trumpf count equal the independent
/// public-trick deduction at that prefix, and a taught void or Trumpf
/// lesson assigns no concealed card.
#[test]
fn every_stated_void_and_trumpf_fact_is_deducible_from_the_public_tricks_then() {
    let mut checked = 0;
    let mut voids_seen = 0;
    for (name, r) in all() {
        let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
        let decisions = coach.decisions().to_vec();
        let before = checked;
        for d in decisions
            .iter()
            .filter(|d| d.phase == Phase::Spiel && d.options > 1)
        {
            let comment = comment_at(&mut coach, d.prefix);
            let c = coach.view().context.unwrap();
            let review = &comment.review;
            let game = c.contract.unwrap().game;
            let expected = expected_voids(&c, game);
            assert_eq!(stated_voids(review), expected, "{name} prefix {}", d.prefix);
            voids_seen += expected.len();
            let own: CardSet = c.hand.iter().copied().collect();
            let trumps: CardSet = CardSet::ALL.iter().filter(|&x| trumpf(game, x)).collect();
            if game == GameType::Null {
                assert_eq!(trumps_out(review), None, "{name}: Null has no Trumpf");
            } else {
                let out = trumps - own - played(&c);
                assert_eq!(trumps_out(review), Some(out.len() as u8), "{name}");
            }
            match &review.lesson {
                Some(coach_model::LessonContent::ProvenVoid { voids, .. }) => {
                    for v in voids {
                        assert!(
                            expected.contains(&(v.seat, v.follow, v.trick, v.led, v.card)),
                            "{name}: taught void not deducible then"
                        );
                    }
                }
                Some(coach_model::LessonContent::RemainingTrumpf { sets, .. }) => {
                    let revealed: CardSet = c
                        .declarer_cards
                        .iter()
                        .flatten()
                        .copied()
                        .filter(|&x| trumpf(game, x))
                        .collect();
                    let skat: CardSet = c
                        .own_skat
                        .iter()
                        .flatten()
                        .copied()
                        .filter(|&x| trumpf(game, x))
                        .collect();
                    assert_eq!(sets.own, trumps & own, "{name}");
                    assert_eq!(sets.played, trumps & played(&c), "{name}");
                    assert_eq!(
                        sets.unseen,
                        trumps - own - played(&c) - revealed - skat,
                        "{name}: no concealed Trumpf assigned"
                    );
                }
                _ => {}
            }
            checked += 1;
        }
        assert!(checked > before, "{name}: no analysed card decision");
    }
    // 42 analysed card decisions over the six recordings (counted when
    // written); every recording contributes, several state voids.
    assert!(checked >= 6 * 2, "{checked}");
    assert!(voids_seen > 0);
}

/// A recording of an open-display game (D-68) teaches exactly what its
/// closed twin does: the display is no analysis input, so the review and
/// every historical comment and job are identical.
#[test]
fn open_display_recording_teaches_exactly_like_its_closed_twin() {
    // Declarer and defender, Farbspiel and Null Ouvert.
    for (name, closed) in all()
        .into_iter()
        .filter(|(n, _)| ["herz", "late herz", "late null"].contains(n))
    {
        let mut open = closed.clone();
        open.context.session.display = TableDisplay::Open;
        assert_ne!(open, closed);
        let run = |r: &GameRecording| {
            let mut coach = ReplayCoach::open(r, SMALL).unwrap();
            assert!(coach.request_review());
            let mut jobs = Vec::new();
            while let Some(job) = coach.review_job() {
                jobs.push((job.view().clone(), job.seed()));
                coach.finish_review(job.run(None)).unwrap();
            }
            (coach.review_status(), jobs)
        };
        assert_eq!(run(&open), run(&closed), "{name}");
    }
}

/// D-58 moments over the historical comments, as declarer and defender:
/// at most three, in game order, one per concept, never forced; the role
/// follows the decision's game; win chance and game points keep the
/// deciding party's and the declarer's sign; "Bad" and "Good" need a
/// complete analysis and a gap beyond the noise margin; a provisional
/// analysis is only "Neutral". Losing the game forces no "Bad" moment.
#[test]
fn moments_keep_role_metric_and_noise_semantics_as_declarer_and_defender() {
    let mut roles = Vec::new();
    for (name, r) in all() {
        let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
        assert!(coach.request_review());
        while let Some(job) = coach.review_job() {
            coach.finish_review(job.run(None)).unwrap();
        }
        let ReviewStatus::Done(s) = coach.review_status() else {
            panic!("{name}: review not done");
        };
        assert!(s.exhaustive(), "{name}");
        assert!(s.moments.len() <= 3, "{name}");
        assert!(s.moments.windows(2).all(|w| w[0].decision < w[1].decision));
        for (i, m) in s.moments.iter().enumerate() {
            assert!(
                s.moments[..i]
                    .iter()
                    .all(|o| o.moment.concept != m.moment.concept)
            );
            let d = coach.decisions()[m.decision];
            assert!(d.options > 1, "{name}: forced decision chosen");
            let review = &m.moment.review;
            assert_eq!(review.chosen.action, d.action);
            let role = match m.context.declarer {
                None => Role::Bidder,
                Some(p) if p == PlayerId::HUMAN => Role::Declarer,
                Some(_) => Role::Defender,
            };
            assert_eq!(review.role, role, "{name}");
            roles.push(role);
            if let Some(alt) = &m.alternative {
                assert_ne!(alt.action, d.action);
                assert_eq!(review.chosen.compared_with, Some(alt.action));
            }
            match m.moment.kind {
                MomentKind::Bad => {
                    assert_eq!(review.status, Status::Complete, "{name}");
                    assert!(review.chosen.gap >= MOMENT_POINTS);
                    assert!(review.chosen.gap > review.chosen.margin, "{name}: noise");
                }
                MomentKind::Good => {
                    assert_eq!(review.status, Status::Complete, "{name}");
                    assert!(review.important && review.trap.is_some());
                }
                MomentKind::Neutral => {}
            }
            if review.status != Status::Complete {
                assert_eq!(m.moment.kind, MomentKind::Neutral, "{name}");
            }
            if let Some(e) = review.chosen.estimate.filter(|e| e.thrown_in == 0.0) {
                // The deciding party's win chance, its own score and (once
                // the declarer is known) the declarer's entry agree in sign.
                if e.win_chance == 1.0 {
                    assert!(e.expected_score > 0.0, "{name} {e:?}");
                } else if e.win_chance == 0.0 {
                    assert!(e.expected_score <= 0.0, "{name} {e:?}");
                }
                let sign = match role {
                    Role::Declarer => 1.0,
                    Role::Defender => -1.0,
                    Role::Bidder => continue,
                };
                if e.win_chance == 1.0 {
                    assert!(sign * e.expected_game_points > 0.0, "{name} {e:?}");
                } else if e.win_chance == 0.0 {
                    assert!(sign * e.expected_game_points < 0.0, "{name} {e:?}");
                }
            }
        }
    }
    assert!(roles.contains(&Role::Declarer) && roles.contains(&Role::Defender));
}
