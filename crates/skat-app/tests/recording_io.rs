// SPDX-License-Identifier: EUPL-1.2

//! Actual local files and untrusted readers, not serialization-only round trips.

use skat_app::*;
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "skat-recording-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(name: &str) -> GameRecording {
    let json = match name {
        "herz" => include_str!("fixtures/recordings/herz.json"),
        "grand" => include_str!("fixtures/recordings/grand.json"),
        "null" => include_str!("fixtures/recordings/null.json"),
        _ => unreachable!(),
    };
    serde_json::from_str(json).unwrap()
}

#[test]
fn local_files_survive_restart_with_identical_states_events_and_visibility() {
    let temp = Temp::new();
    for name in ["herz", "grand", "null"] {
        let recording = fixture(name);
        let path = temp.file(name);
        recording.save_local_new(&path).unwrap();
        let reopened = GameRecording::open_local(&path).unwrap();
        assert_eq!(recording, reopened);
        let boundaries = reopened.trick_boundaries().unwrap();
        assert_eq!(boundaries[0], 0);
        assert_eq!(boundaries.last(), Some(&recording.actions.len()));
        for _ in 0..2 {
            for &n in boundaries.iter().chain(boundaries.iter().rev()) {
                assert_eq!(
                    recording.reconstruct_privileged(n),
                    reopened.reconstruct_privileged(n)
                );
            }
        }
    }
    let mut session = Session::new(SessionConfig {
        seed: 7,
        series: SeriesLength::Fixed(2),
        kontra_re: false,
        opponents: Opponents::Levels([Level::Easy; 2]),
        ..SessionConfig::default()
    });
    let current = session.export_current_recording_privileged().unwrap();
    current.save_local_new(temp.file("current")).unwrap();
    drop(current);
    let reopened = GameRecording::open_local(temp.file("current")).unwrap();
    assert_eq!(reopened.end, RecordingEnd::InProgress);
    assert_eq!(
        reopened,
        session.export_current_recording_privileged().unwrap()
    );
    while session.completed_recordings_privileged().is_empty() {
        if session.opponent_to_move() {
            session.execute(Command::AdvanceOpponent).unwrap();
        } else {
            let view = session.view();
            let a = if view.legal_actions.contains(&Action::Pass) {
                Action::Pass
            } else {
                view.legal_actions[0]
            };
            session.execute(Command::Act(a)).unwrap();
        }
    }
    let completed = session.completed_recordings_privileged()[0].clone();
    completed.save_local_new(temp.file("completed")).unwrap();
    drop(session);
    assert_eq!(
        GameRecording::open_local(temp.file("completed")).unwrap(),
        completed
    );
}

#[test]
fn corrupt_unsupported_oversized_and_illegal_files_are_rejected() {
    let temp = Temp::new();
    let path = temp.file("input");
    fs::write(&path, b"{").unwrap();
    assert_eq!(
        GameRecording::open_local(&path),
        Err(RecordingFileError::InvalidJson)
    );
    let base = serde_json::to_value(fixture("herz")).unwrap();
    for (field, value, expected) in [
        (
            "format",
            serde_json::json!(999999),
            RecordingFileError::UnsupportedFormat,
        ),
        (
            "engine",
            serde_json::json!("future"),
            RecordingFileError::UnsupportedEngine,
        ),
        (
            "actions",
            serde_json::json!(["TakeSkat"]),
            RecordingFileError::InvalidRecording(RecordingError::EndMismatch),
        ),
        (
            "actions",
            serde_json::json!(vec!["Pass"; MAX_RECORDING_ACTIONS + 1]),
            RecordingFileError::TooLarge,
        ),
    ] {
        let mut value_record = base.clone();
        value_record[field] = value;
        fs::write(&path, serde_json::to_vec(&value_record).unwrap()).unwrap();
        let error = GameRecording::open_local(&path).unwrap_err();
        if matches!(expected, RecordingFileError::InvalidRecording(_)) {
            assert!(matches!(error, RecordingFileError::InvalidRecording(_)));
        } else {
            assert_eq!(error, expected);
        }
    }
    fs::write(&path, vec![b' '; MAX_RECORDING_BYTES + 1]).unwrap();
    assert_eq!(
        GameRecording::open_local(&path),
        Err(RecordingFileError::TooLarge)
    );
}

#[test]
fn capped_reader_handles_growth_and_read_failure_before_parsing() {
    struct Growing(usize);
    impl Read for Growing {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = buf.len().min(31);
            self.0 += n;
            assert!(self.0 <= MAX_RECORDING_BYTES + 1);
            buf[..n].fill(b' ');
            Ok(n)
        }
    }
    assert_eq!(
        GameRecording::read_local(Growing(0)),
        Err(RecordingFileError::TooLarge)
    );
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("failed"))
        }
    }
    assert_eq!(
        GameRecording::read_local(Broken),
        Err(RecordingFileError::Io)
    );
}

#[test]
fn failed_saves_never_modify_original_bytes_and_validate_before_creation() {
    let temp = Temp::new();
    let path = temp.file("original");
    let recording = fixture("null");
    recording.save_local_new(&path).unwrap();
    let original = fs::read(&path).unwrap();
    assert_eq!(
        recording.save_local_new(&path),
        Err(RecordingFileError::AlreadyExists)
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    let mut invalid = recording.clone();
    invalid.actions = vec![Action::Pass; MAX_RECORDING_ACTIONS + 1];
    assert_eq!(
        invalid.save_local_new(&path),
        Err(RecordingFileError::TooLarge)
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    invalid.actions = vec![Action::TakeSkat];
    assert!(matches!(
        invalid.save_local_new(&path),
        Err(RecordingFileError::InvalidRecording(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(invalid.save_local_new(temp.file("new-invalid")).is_err());
    assert!(!temp.file("new-invalid").exists());
    assert_eq!(
        GameRecording::open_local(temp.file("missing")),
        Err(RecordingFileError::Missing)
    );
    assert_eq!(
        recording.save_local_new(temp.file("absent/child")),
        Err(RecordingFileError::Missing)
    );
    // A file cannot be used as a parent directory on either platform.
    assert_eq!(
        recording.save_local_new(path.join("child")),
        Err(RecordingFileError::Io)
    );
    assert_eq!(
        GameRecording::open_local(&temp.0),
        Err(RecordingFileError::Io)
    );
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn symlinks_and_unwritable_directory_are_not_overwritten() {
    use std::os::unix::{
        ffi::OsStrExt,
        fs::{PermissionsExt, symlink},
    };
    let temp = Temp::new();
    let recording = fixture("herz");
    let path = temp.file("original");
    fs::write(&path, b"preserve").unwrap();
    let link = temp.file("link");
    symlink(&path, &link).unwrap();
    assert_eq!(
        recording.save_local_new(&link),
        Err(RecordingFileError::AlreadyExists)
    );
    assert_eq!(fs::read(&path).unwrap(), b"preserve");
    let dir = temp.file("readonly");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let result = recording.save_local_new(dir.join("new"));
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    // Tests run as the normal developer; root can bypass directory modes.
    if std::env::var_os("USER").is_none_or(|user| user.as_os_str().as_bytes() != b"root") {
        assert_eq!(result, Err(RecordingFileError::Io));
    }
}

#[test]
fn all_pass_and_live_prefix_files_preserve_honest_end_state() {
    let temp = Temp::new();
    let mut record = fixture("herz");
    record.actions = vec![Action::Pass; 3];
    record.end = RecordingEnd::Eingepasst;
    record.save_local_new(temp.file("all-pass")).unwrap();
    let all_pass = GameRecording::open_local(temp.file("all-pass")).unwrap();
    assert_eq!(
        all_pass.reconstruct_privileged(3).unwrap().state.seats[0].phase,
        Phase::Eingepasst
    );
    assert_eq!(all_pass.trick_boundaries().unwrap(), vec![0, 3]);
    let mut record = fixture("grand");
    record.actions.truncate(8);
    record.end = RecordingEnd::InProgress;
    record.save_local_new(temp.file("prefix")).unwrap();
    assert_eq!(
        GameRecording::open_local(temp.file("prefix")).unwrap(),
        record
    );
}

#[test]
fn trick_boundaries_follow_core_events_with_historical_permitted_views() {
    for name in ["herz", "grand", "null"] {
        let record = fixture(name);
        let mut game = skat_core::Game::from_deal(record.deal, record.rules);
        let mut expected = vec![0];
        for (n, &action) in record.actions.iter().enumerate() {
            let events = game.apply(action).unwrap();
            if events
                .iter()
                .any(|event| matches!(event, Event::TrickWon { .. }))
            {
                expected.push(n + 1);
            }
            let position = record.reconstruct_privileged(n + 1).unwrap();
            for seat in Seat::ALL {
                assert_eq!(position.state.seats[seat.index()], game.view(seat));
            }
        }
        if expected.last() != Some(&record.actions.len()) {
            expected.push(record.actions.len());
        }
        assert_eq!(record.trick_boundaries().unwrap(), expected);
    }
}

/// Open display (D-68) is stored with the series configuration: open files
/// carry it and survive a restart, closed files keep their format-1 bytes
/// without the field, existing files load as closed, and a same-deal
/// practice attempt keeps the open display with the closed-view coach.
#[test]
fn open_display_round_trips_through_files_and_practice() {
    for name in ["herz", "grand", "null"] {
        let recording = fixture(name);
        assert_eq!(recording.context.session.display, TableDisplay::Closed);
        assert!(
            !serde_json::to_string(&recording)
                .unwrap()
                .contains("display")
        );
    }
    let temp = Temp::new();
    let mut files = Vec::new();
    for display in [TableDisplay::Closed, TableDisplay::Open] {
        let mut session = Session::new(SessionConfig {
            seed: 9,
            series: SeriesLength::Fixed(2),
            kontra_re: true,
            opponents: Opponents::Levels([Level::Easy; 2]),
            display,
        });
        while session.completed_recordings_privileged().is_empty() {
            if session.opponent_to_move() {
                session.execute(Command::AdvanceOpponent).unwrap();
            } else {
                let action = session.view().legal_actions[0];
                session.execute(Command::Act(action)).unwrap();
            }
        }
        let completed = session.completed_recordings_privileged()[0].clone();
        assert_eq!(completed.context.session.display, display);
        let path = temp.file(&format!("{display:?}"));
        completed.save_local_new(&path).unwrap();
        let bytes = fs::read_to_string(&path).unwrap();
        assert_eq!(bytes.contains("display"), display == TableDisplay::Open);
        let reopened = GameRecording::open_local(&path).unwrap();
        assert_eq!(reopened, completed);
        files.push((session, reopened, bytes));
    }
    let (closed_session, closed, closed_bytes) = &files[0];
    let (open_session, open, open_bytes) = &files[1];
    // Display only: the same deal, actions, seeds and result.
    assert_eq!(closed.deal, open.deal);
    assert_eq!(closed.actions, open.actions);
    assert_eq!(closed.end, open.end);
    assert_eq!(closed.context.opponent_seeds, open.context.opponent_seeds);
    assert_eq!(closed_session.sheet(), open_session.sheet());
    // The open file differs from the closed one only by the field; without
    // it, it reads as an older, closed file with identical bytes.
    let stripped = open_bytes.replace(",\n      \"display\": \"Open\"", "");
    assert_eq!(&stripped, closed_bytes);
    let older = GameRecording::read_local(stripped.as_bytes()).unwrap();
    assert_eq!(&older, closed);
    let mut practice = open_session.start_practice(open).unwrap();
    assert!(practice.is_practice());
    assert_eq!(practice.config().display, TableDisplay::Open);
    practice.run_opponents();
    let view = practice.view();
    assert!(view.open_cards.is_some());
    assert_eq!(view.coach_basis, CoachBasis::OwnView);
    let job = practice
        .coach_job()
        .expect("on-request coach analyses the decision");
    assert!(!job.sees_all_cards());
    let closed_practice = closed_session.start_practice(closed).unwrap();
    assert_eq!(closed_practice.view().open_cards, None);
}
