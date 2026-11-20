// SPDX-License-Identifier: EUPL-1.2
use super::*;

fn ordinary() -> Session {
    let mut s = Session::new(SessionConfig {
        seed: 4,
        series: SeriesLength::Fixed(3),
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    let route: Vec<Command> = serde_json::from_str(include_str!(
        "../../gui-egui/tests/fixtures/suit-early-feed-route.json"
    ))
    .unwrap();
    for cmd in route {
        s.run_opponents();
        s.execute(cmd).unwrap();
    }
    s.run_opponents();
    s
}

#[test]
fn suit_early_feed_ordinary_every_card_actual_reply_consent_stale_and_return() {
    let base = ordinary();
    let v = base.view();
    assert_eq!(
        (v.game_number, v.tricks_played, v.players[0].seat),
        (1, 1, Seat::Vorhand)
    );
    assert!(matches!(
        v.contract.map(|c| c.game),
        Some(skat_core::GameType::Farbe(_))
    ));
    assert_eq!(v.current_trick.len(), 1);
    let led = v.current_trick[0].card;
    assert_eq!(v.legal_actions.len(), 2, "{:?}", v.legal_actions);
    for action in v.legal_actions.clone() {
        let mut s = base.clone();
        let before = s.export_current_recording_privileged();
        s.execute(Command::StartGuided).unwrap();
        assert!(s.guided().unwrap().rule.is_none());
        assert!(s.guided().unwrap().task.is_none());
        assert_eq!(respond(&mut s, G::Memory(true)), Err(AppError::NoQuestion));
        assert_eq!(respond(&mut s, G::Figures), Err(AppError::NoQuestion));
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Vorhand)))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Hinterhand)))).unwrap();
        respond(&mut s, G::Answer(A::Seat(None))).unwrap();
        assert_eq!(s.guided().unwrap().correct, Some(false));
        respond(&mut s, G::Retry).unwrap();
        respond(&mut s, G::Answer(A::Seat(Some(Seat::Mittelhand)))).unwrap();
        assert_eq!(
            s.guided().unwrap().task,
            Some(crate::GuidedBranch::FeedPartner)
        );
        let Action::Play(chosen) = action else {
            unreachable!()
        };
        respond(&mut s, G::Answer(A::Card(chosen))).unwrap();
        let rule = s.guided().unwrap().rule.unwrap();
        assert_eq!(rule.branch, crate::GuidedBranch::FeedPartner);
        assert!(!rule.last && rule.table_master);
        respond(&mut s, G::Memory(true)).unwrap();
        assert_eq!(
            s.guided().unwrap().memory.unwrap().own,
            base.game.view(Seat::Vorhand).hand
        );
        respond(&mut s, G::Return).unwrap();
        assert_eq!(s.export_current_recording_privileged(), before);
        s.execute(Command::StartGuided).unwrap();
        let old = s.guided().unwrap().id;
        respond(&mut s, G::Return).unwrap();
        s.execute(Command::StartGuided).unwrap();
        assert_eq!(
            s.execute(Command::Guided(old, G::Reveal)),
            Err(AppError::StaleGuided)
        );
        respond(&mut s, G::Reveal).unwrap();
        s.execute(Command::Act(action)).unwrap();
        assert_eq!(s.guided().unwrap().step, S::Waiting);
        s.run_opponents();
        assert_eq!(s.guided().unwrap().step, S::Observe);
        let actual = s.game.tricks()[1];
        assert_eq!(actual.cards[..2], [led, chosen]);
        assert_eq!(format!("{:?}", actual.cards[2]), "Karo-Sieben");
        assert_eq!(actual.winner, Seat::Hinterhand);
        assert_eq!(
            s.guided().unwrap().observed_source,
            Some(skat_coach::Source::Trick {
                index: 1,
                seat: Seat::Mittelhand,
                led,
                card: actual.cards[2]
            })
        );
        assert!(s.guided().unwrap().observation.is_none());
        respond(&mut s, G::Reveal).unwrap();
        let o = s.guided().unwrap().observation.unwrap();
        assert_eq!(o.winner, Some(Seat::Hinterhand));
        assert!(!o.proves_void);
        respond(&mut s, G::Return).unwrap();
        assert!(s.guided().is_none());
    }
}
