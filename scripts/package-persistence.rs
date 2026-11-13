// SPDX-License-Identifier: EUPL-1.2

//! RC test helper, linked against the same skat-app build as the package.
//! Uses actual training events; never included inside the application archive.

use skat_app::{AssistanceLevel, LearningHistory, TrainingCommand, TrainingSession, TrainingTopic};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("create or check");
    let path = PathBuf::from(args.next().expect("history path"));
    if mode == "create" {
        assert!(!path.exists(), "probe only creates a new scratch history");
        let mut session = TrainingSession::new(
            TrainingTopic::TrumpfOutside,
            AssistanceLevel::AnswerBeforeReveal,
        );
        let mut events = session.take_events();
        session.execute(TrainingCommand::Skip).unwrap();
        events.extend(session.take_events());
        let mut history = LearningHistory::new();
        let attempt = history.begin_attempt(session.attempt_kind()).unwrap();
        history.record(attempt, &events).unwrap();
        assert!(history.summary().totals.opportunities > 0);
        history.save_local(&path).unwrap();
        assert_eq!(LearningHistory::open_local(&path).unwrap(), history);
    } else {
        assert_eq!(mode, "check");
        let history = LearningHistory::open_local(&path).unwrap();
        assert!(history.summary().totals.opportunities > 0);
        assert!(history.summary().totals.skips > 0);
    }
}
