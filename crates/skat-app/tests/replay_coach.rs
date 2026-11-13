// SPDX-License-Identifier: EUPL-1.2

//! M7 historical versus hindsight comments on one recorded human decision
//! (D-67). Expected cards, seats and bidding follow the documented deal of
//! `fixtures/recordings/README.md` (seed 7), not the code under test.

use skat_app::coach_model::{Decision, Judgement, Role, Status};
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
/// (player 0) sits in Hinterhand and the Vorhand cards belong to player 1.
fn late(name: &str) -> GameRecording {
    let mut r = fixture(name);
    r.context.number = 2;
    r.context.dealer = PlayerId(0);
    r
}

const SMALL: Budget = Budget {
    samples: 4,
    nodes_per_task: 2_000_000,
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

fn set(cards: &[Card]) -> CardSet {
    cards.iter().copied().collect()
}

fn index_of(coach: &ReplayCoach, prefix: usize) -> usize {
    coach
        .decisions()
        .iter()
        .position(|d| d.prefix == prefix)
        .unwrap()
}

fn ready(coach: &ReplayCoach) -> ReplayComment {
    match coach.view().status {
        ReplayStatus::Ready(c) => *c,
        other => panic!("not ready: {other:?}"),
    }
}

#[test]
fn historical_view_is_exactly_the_humans_table_before_the_recorded_action() {
    // Herz fixture, game 1 (dealer player 2): the human is Vorhand and the
    // declarer; it discarded C9/CT. Trick 4: Mittelhand (player 1) won
    // trick 3 and led S8, Hinterhand (player 2) followed S7.
    let r = fixture("herz");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    assert_eq!(coach.view().status, ReplayStatus::NotSelected);
    let i = index_of(&coach, 17);
    coach.select(Some(i));
    let d = coach.decisions()[i];
    assert_eq!(d.action, Action::Play(card("H9")));
    assert_eq!((d.phase, d.trick, d.options), (Phase::Spiel, Some(4), 7));
    let view = coach.view();
    assert_eq!(view.perspective, ReplayPerspective::Historical);
    assert_eq!(view.status, ReplayStatus::Idle);
    let c = view.context.unwrap();
    assert_eq!((c.game_number, c.actions, c.dealer), (1, 17, PlayerId(2)));
    assert_eq!(c.seat(PlayerId::HUMAN), Seat::Vorhand);
    assert_eq!(c.declarer, Some(PlayerId::HUMAN));
    assert_eq!(c.action, Action::Play(card("H9")));
    // VH dealt C9 CT CJ CQ H9 HT HQ HK D9 DK, picked up S9 DA, discarded
    // C9 CT and played CJ, CQ and S9 in tricks 1–3.
    assert_eq!(set(&c.hand), cards("HT HK HQ H9 DA DK D9"));
    assert_eq!(set(c.own_skat.as_ref().unwrap()), cards("C9 CT"));
    assert_eq!(c.declarer_cards, None);
    assert_eq!(
        c.bidding,
        vec![
            (PlayerId(1), Call::Pass),
            (PlayerId(2), Call::Pass),
            (PlayerId::HUMAN, Call::Bid(18)),
        ]
    );
    assert_eq!(c.tricks.len(), 3, "no future trick");
    let current: Vec<_> = c.current_trick.iter().map(|p| (p.player, p.card)).collect();
    assert_eq!(
        current,
        vec![(PlayerId(1), card("S8")), (PlayerId(2), card("S7"))]
    );
    // Nothing played later is visible: every card shown is in the hand,
    // the own Skat, the three completed tricks or the current trick.
    let mut shown = set(&c.hand) | set(c.own_skat.as_ref().unwrap());
    for t in &c.tricks {
        shown |= t.cards.iter().map(|p| p.card).collect::<CardSet>();
    }
    shown |= c.current_trick.iter().map(|p| p.card).collect::<CardSet>();
    assert_eq!(shown.len(), 7 + 2 + 9 + 2);
    let later = r.reconstruct_privileged(r.actions.len()).unwrap();
    let after: CardSet = later.state.seats[0].tricks[4..]
        .iter()
        .flat_map(|t| t.cards)
        .collect();
    assert!(
        (shown - set(&c.hand)).is_disjoint(after),
        "later tricks absent"
    );
}

#[test]
fn late_series_dealer_moves_the_human_and_ouvert_cards_appear_only_once_revealed() {
    // Grand Hand Ouvert as game 2: the human (player 0) is Hinterhand and a
    // defender holding the original HH cards C7 C8 S7 SK SA H8 HJ HA DT DQ.
    let r = late("grand");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    let first = coach.decisions()[0];
    assert_eq!((first.prefix, first.action), (3, Action::Bid(20)));
    coach.select(Some(0));
    let c = coach.view().context.unwrap();
    assert_eq!(c.dealer, PlayerId(0));
    assert_eq!(c.seat(PlayerId::HUMAN), Seat::Hinterhand);
    assert_eq!(c.seat(PlayerId(1)), Seat::Vorhand);
    assert_eq!(set(&c.hand), cards("C7 C8 S7 SK SA H8 HJ HA DT DQ"));
    assert_eq!((c.own_skat.clone(), c.declarer_cards.clone()), (None, None));
    // First card decision: the declarer (player 1, Vorhand) has revealed
    // and led one card; nine open cards as they lay then, not the final four.
    let i = index_of(&coach, 12);
    coach.select(Some(i));
    let c = coach.view().context.unwrap();
    assert_eq!(c.declarer, Some(PlayerId(1)));
    let open = set(c.declarer_cards.as_ref().unwrap());
    assert_eq!(open.len(), 9);
    assert_eq!(c.own_skat, None, "a defender never sees the Hand Skat");
    // Game 1 of the same record: the human is Vorhand and the declarer.
    // Re before the reveal shows no open cards; the first card does.
    let mut first = ReplayCoach::open(&fixture("grand"), SMALL).unwrap();
    let re = index_of(&first, 9);
    first.select(Some(re));
    let c = first.view().context.unwrap();
    assert_eq!(c.action, Action::Re);
    assert_eq!(c.declarer_cards, None);
    first.select(Some(re + 1));
    let c = first.view().context.unwrap();
    assert_eq!(
        set(c.declarer_cards.as_ref().unwrap()),
        cards("C9 CT CJ CQ H9 HT HQ HK D9 DK")
    );
}

#[test]
fn chosen_action_and_legal_alternative_with_role_seat_evidence_and_perspective() {
    let r = fixture("herz");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    let i = index_of(&coach, 17);
    coach.select(Some(i));
    let job = coach.request().unwrap();
    assert_eq!(coach.view().status, ReplayStatus::Computing);
    assert!(!job.sees_all_cards());
    assert_eq!(job.perspective(), ReplayPerspective::Historical);
    assert_eq!(job.view().seat, Seat::Vorhand);
    assert_eq!(job.view().hand, cards("HT HK HQ H9 DA DK D9"));
    // Void in Pik: every card in the hand is legal.
    assert_eq!(job.view().legal_actions.len(), 7);
    coach.finish(job.run(None)).unwrap();
    let comment = ready(&coach);
    assert_eq!(comment.perspective, ReplayPerspective::Historical);
    let review = &comment.review;
    assert_eq!(
        (review.decision, review.role),
        (Decision::Card, Role::Declarer)
    );
    assert_eq!(review.trick, Some(4));
    assert!(!review.forced && !review.basis.known);
    assert_eq!(review.chosen.action, Action::Play(card("H9")));
    let alternative = comment.alternative.clone().unwrap();
    assert_ne!(alternative.action, review.chosen.action);
    assert!(job.view().legal_actions.contains(&alternative.action));
    assert_eq!(review.chosen.compared_with, Some(alternative.action));
    // A gap is decisive only beyond the noise margin of a complete analysis.
    if review.chosen.judgement == Judgement::Weak {
        assert_eq!(review.status, Status::Complete);
        assert!(review.chosen.gap > review.chosen.margin);
    }
    // Explicit hindsight: the real deal, marked as such.
    coach.set_perspective(ReplayPerspective::Hindsight);
    assert_eq!(coach.view().status, ReplayStatus::Idle);
    let known = coach.request().unwrap();
    assert!(known.sees_all_cards());
    assert_eq!(
        known.view(),
        job.view(),
        "same permitted view, more evidence"
    );
    coach.finish(known.run(None)).unwrap();
    let hindsight = ready(&coach);
    assert_eq!(hindsight.perspective, ReplayPerspective::Hindsight);
    assert!(hindsight.review.basis.known);
    assert_eq!(hindsight.review.chosen.action, Action::Play(card("H9")));
    // Selecting another decision restores the historical perspective.
    coach.select(Some(i - 1));
    assert_eq!(coach.view().perspective, ReplayPerspective::Historical);
    assert_eq!(coach.view().status, ReplayStatus::Idle);
}

#[test]
fn forced_cancelled_failed_all_pass_and_short_files_report_honest_status() {
    let r = fixture("herz");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    // The last card: only one legal option, no alternative, no rating.
    let last = coach.decisions().len() - 1;
    assert_eq!(coach.decisions()[last].options, 1);
    coach.select(Some(last));
    let job = coach.request().unwrap();
    coach.finish(job.run(None)).unwrap();
    let comment = ready(&coach);
    assert!(comment.review.forced);
    assert_eq!(comment.alternative, None);
    // A cancelled analysis is shown as cancelled, never as a comment.
    let job = coach.request().unwrap();
    let cancel = Cancel::new();
    cancel.cancel();
    coach.finish(job.run(Some(&cancel))).unwrap();
    assert_eq!(
        coach.view().status,
        ReplayStatus::Unavailable(Unavailable::Cancelled)
    );
    // Retry after it: a new request computes again.
    let job = coach.request().unwrap();
    coach.finish(job.run(None)).unwrap();
    assert!(matches!(coach.view().status, ReplayStatus::Ready(_)));
    // Cancelling from the frontend rejects the late result.
    let job = coach.request().unwrap();
    coach.cancel();
    assert_eq!(
        coach.view().status,
        ReplayStatus::Unavailable(Unavailable::Cancelled)
    );
    assert_eq!(coach.finish(job.run(None)), Err(ReplayStale::Stale));

    // All pass: the human's single pass is a real bidding decision with a
    // legal alternative; no card play, no invented trick.
    let mut all_pass = Session::new(SessionConfig::default())
        .export_current_recording_privileged()
        .unwrap();
    all_pass.actions = vec![Action::Pass; 3];
    all_pass.end = RecordingEnd::Eingepasst;
    let mut coach = ReplayCoach::open(&all_pass, SMALL).unwrap();
    let d = coach.decisions().to_vec();
    assert_eq!(d.len(), 1);
    assert_eq!(
        (d[0].prefix, d[0].action, d[0].trick),
        (2, Action::Pass, None)
    );
    coach.select(Some(0));
    let job = coach.request().unwrap();
    coach.finish(job.run(None)).unwrap();
    let comment = ready(&coach);
    assert_eq!(comment.review.trick, None);
    assert!(matches!(
        comment.alternative.map(|a| a.action),
        Some(Action::Bid(18))
    ));

    // In-progress files before the human's first action: nothing to select.
    let mut live = Session::new(SessionConfig::default());
    for file in [live.export_current_recording_privileged().unwrap(), {
        live.run_opponents();
        live.export_current_recording_privileged().unwrap()
    }] {
        assert_eq!(file.end, RecordingEnd::InProgress);
        let mut coach = ReplayCoach::open(&file, SMALL).unwrap();
        assert!(coach.decisions().is_empty());
        assert_eq!(coach.view().status, ReplayStatus::NoDecisions);
        coach.select(Some(0));
        assert!(coach.request().is_none());
        assert_eq!(coach.view().status, ReplayStatus::NoDecisions);
    }
    // An invalid file opens no replay coach.
    let mut broken = fixture("herz");
    broken.actions.truncate(10);
    assert!(ReplayCoach::open(&broken, SMALL).is_err());
}

#[test]
fn stale_results_are_rejected_after_changes_and_across_identical_recordings() {
    let herz = fixture("herz");
    // A different file with the same game number and actions.
    let mut twin = fixture("herz");
    twin.context.application_version = "0.0.9-other".into();
    let mut a = ReplayCoach::open(&herz, SMALL).unwrap();
    let mut b = ReplayCoach::open(&twin, SMALL).unwrap();
    let mut same = ReplayCoach::open(&herz, SMALL).unwrap();
    let i = index_of(&a, 17);
    for c in [&mut a, &mut b, &mut same] {
        c.select(Some(i));
    }
    let job_a = a.request().unwrap();
    let job_b = b.request().unwrap();
    let job_same = same.request().unwrap();
    assert_eq!(job_a.view(), job_b.view());
    assert_eq!(job_a.seed(), job_b.seed(), "seed from the view only");
    let (res_a, res_b, res_same) = (job_a.run(None), job_b.run(None), job_same.run(None));
    assert_eq!(res_a.comment(), res_b.comment(), "same view, same comment");
    assert_eq!(b.finish(res_a.clone()), Err(ReplayStale::Stale));
    assert_eq!(same.finish(res_a.clone()), Err(ReplayStale::Stale));
    assert_eq!(a.finish(res_b.clone()), Err(ReplayStale::Stale));
    assert_eq!(a.finish(res_same), Err(ReplayStale::Stale));
    assert_eq!(b.view().status, ReplayStatus::Computing);
    b.finish(res_b).unwrap();

    // Rapid changes: each makes the outstanding result stale.
    let job = a.request().unwrap();
    a.select(Some(i - 1));
    assert_eq!(a.finish(job.run(None)), Err(ReplayStale::Stale));
    assert_eq!(a.view().status, ReplayStatus::Idle);
    let job = a.request().unwrap();
    a.set_perspective(ReplayPerspective::Hindsight);
    assert_eq!(a.finish(job.run(None)), Err(ReplayStale::Stale));
    let job = a.request().unwrap();
    a.set_perspective(ReplayPerspective::Historical);
    assert_eq!(a.finish(job.run(None)), Err(ReplayStale::Stale));
    let job = a.request().unwrap();
    let retry = a.request().unwrap();
    assert_eq!(a.finish(job.run(None)), Err(ReplayStale::Stale));
    a.finish(retry.run(None)).unwrap();
    let job = a.request().unwrap();
    a.set_budget(Budget {
        samples: 2,
        ..SMALL
    });
    assert_eq!(a.finish(job.run(None)), Err(ReplayStale::Stale));
    // Close and reopen: the old result never reaches the new viewer.
    let job = a.request().unwrap();
    drop(a);
    let mut reopened = ReplayCoach::open(&herz, SMALL).unwrap();
    assert_eq!(reopened.view().status, ReplayStatus::NotSelected);
    reopened.select(Some(i - 1));
    let _ = reopened.request().unwrap();
    assert_eq!(reopened.finish(job.run(None)), Err(ReplayStale::Stale));
    // A result is accepted once only.
    let job = reopened.request().unwrap();
    let result = job.run(None);
    reopened.finish(result.clone()).unwrap();
    assert_eq!(reopened.finish(result), Err(ReplayStale::Stale));
}

#[test]
fn replay_coaching_leaves_the_live_session_its_pending_job_and_the_file_untouched() {
    let mut live = Session::new(SessionConfig {
        seed: 3,
        series: SeriesLength::Fixed(3),
        kontra_re: true,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    live.set_coach_budget(SMALL);
    live.run_opponents();
    live.execute(Command::RequestAdvice).unwrap();
    let pending = live.coach_job().expect("live coach job pending");
    let before = live.view();
    let recording = live.export_current_recording_privileged().unwrap();
    let herz = fixture("herz");
    let bytes = serde_json::to_vec(&herz).unwrap();
    let mut coach = ReplayCoach::open(&herz, SMALL).unwrap();
    let late = late("herz");
    let mut late_coach = ReplayCoach::open(&late, SMALL).unwrap();
    for c in [&mut coach, &mut late_coach] {
        for perspective in [ReplayPerspective::Historical, ReplayPerspective::Hindsight] {
            c.select(Some(1));
            c.set_perspective(perspective);
            let job = c.request().unwrap();
            c.finish(job.run(None)).unwrap();
        }
    }
    assert_eq!(live.view(), before);
    assert!(live.coach_job().unwrap().same_decision(&pending));
    assert_eq!(
        live.export_current_recording_privileged().unwrap(),
        recording
    );
    assert_eq!(serde_json::to_vec(coach.recording()).unwrap(), bytes);
    assert_eq!(coach.recording(), &herz);
    live.finish_coach(pending.run(None)).unwrap();
    assert!(live.coach_result().is_some());
}

#[test]
fn late_series_human_is_found_by_player_and_dealer_not_by_seat_zero() {
    // Herz as game 2: the human is Hinterhand, passed and defends; the
    // declarer (player 1, Vorhand) holds the Skat. VH and HH cards differ.
    let r = late("herz");
    let mut coach = ReplayCoach::open(&r, SMALL).unwrap();
    let first = coach.decisions()[0];
    assert_eq!((first.prefix, first.action), (1, Action::Pass));
    assert!(
        coach.decisions()[1..]
            .iter()
            .all(|d| d.phase == Phase::Spiel)
    );
    coach.select(Some(1));
    let job = coach.request().unwrap();
    assert_eq!(job.view().seat, Seat::Hinterhand);
    coach.finish(job.run(None)).unwrap();
    let comment = ready(&coach);
    assert_eq!(comment.review.role, Role::Defender);
    let c = coach.view().context.unwrap();
    assert_eq!(c.declarer, Some(PlayerId(1)));
    assert_eq!(c.own_skat, None);
    assert!(set(&c.hand).is_subset(cards("C7 C8 S7 SK SA H8 HJ HA DT DQ")));
    assert_ne!(comment.review.decision, Decision::Bid);
    assert_ne!(comment.review.status, Status::NoDecision);
}
