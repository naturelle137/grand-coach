// SPDX-License-Identifier: EUPL-1.2

//! Starts the Grand-Coach window.
//!
//! The series seed defaults to the current time; `GRAND_COACH_SEED` fixes it
//! to replay a series and also turns on the developer option that shows the
//! seed (otherwise off, see Settings). The window opens with the "new
//! series" dialog showing the series settings.
//!
//! The learning history is kept in `learning-history.json` in the data
//! directory ([`gui_egui::learning::data_dir`]; `GRAND_COACH_DATA`
//! overrides it). `GRAND_COACH_VALIDATION=1` opens validation confirmation
//! instead of new-series setup; no reserved task is shown on launch.
//!
//! Smoke test: with `GRAND_COACH_SCREENSHOT=<file.ppm>` the window saves a
//! screenshot after `GRAND_COACH_SCREENSHOT_DELAY` seconds (default 3) and
//! closes. `GRAND_COACH_SMOKE_SCENARIO=coach|training` (with a screenshot)
//! additionally prepares one packaged feature view: `coach` plays first
//! legal actions with coach, counting aid and quiz on until a counting
//! question is open; `quiz-answer` answers it through the app command;
//! `training` opens the focused counting training;
//! `series` plays a complete 36-game series with first legal actions;
//! `open` plays an open-table game for a few moves; `replay` plays that
//! series, saves its first game to the data directory and shows it in the
//! recording viewer; `review` does the same and waits until the requested
//! recording review has ended with its summary; `practice` starts an isolated
//! attempt from the reopened recording; `cards` shows all 32 faces
//! of every deck in a window.
//! `GRAND_COACH_LANGUAGE=de|en` selects the language for that smoke.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;
use gui_egui::{GrandCoachApp, SmokeGoal};
use skat_app::{
    AssistSettings, CoachLevel, CoachSettings, OpponentEngine, SeriesLength, SessionConfig,
    TableDisplay,
};

/// Wraps the app to take one screenshot and close (smoke test).
struct Screenshot {
    app: GrandCoachApp,
    path: std::path::PathBuf,
    delay: f64,
    requested: bool,
    /// Smoke scenario: still playing towards its goal.
    advancing: Option<SmokeGoal>,
    /// Smoke scenario `replay`: open the first game in the viewer at the goal.
    replay_dir: Option<std::path::PathBuf>,
    /// Smoke scenario `review`: the opened recording is reviewed before the shot.
    review_wanted: bool,
    reviewing: bool,
    practice_wanted: bool,
    quiz_answer_wanted: bool,
    /// Smoke scenario `cards`: show every deck's faces over the app.
    faces: bool,
    done: Arc<AtomicBool>,
}

impl eframe::App for Screenshot {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
        let ctx = ui.ctx().clone();
        if self.faces {
            self.app.smoke_faces(&ctx);
        }
        let now = ctx.input(|i| i.time);
        if let Some(goal) = self.advancing {
            match self.app.smoke_advance(now, goal) {
                Ok(true) => {
                    self.advancing = None;
                    if self.quiz_answer_wanted
                        && let Err(reason) = self.app.smoke_answer_quiz(now)
                    {
                        eprintln!("smoke: quiz answer: {reason}");
                        std::process::exit(2);
                    }
                    if let Some(dir) = &self.replay_dir
                        && let Err(reason) = self.app.smoke_open_replay(dir)
                    {
                        eprintln!("smoke: replay: {reason}");
                        std::process::exit(2);
                    }
                    if self.practice_wanted
                        && let Err(reason) = self.app.smoke_start_practice()
                    {
                        eprintln!("smoke: practice: {reason}");
                        std::process::exit(2);
                    }
                    self.reviewing = self.review_wanted;
                    // Let windows finish their fade-in before the screenshot.
                    self.delay = self.delay.max(now + 1.5);
                }
                Ok(false) => {}
                Err(reason) => {
                    eprintln!("smoke: {reason}");
                    std::process::exit(2);
                }
            }
            if self.advancing.is_some() && now > 900.0 {
                eprintln!("smoke: goal not reached within 900 s");
                std::process::exit(2);
            }
        }
        if self.reviewing {
            match self.app.smoke_review(&ctx) {
                Ok(true) => {
                    self.reviewing = false;
                    self.delay = self.delay.max(now + 1.5);
                }
                Ok(false) => {}
                Err(reason) => {
                    eprintln!("smoke: {reason}");
                    std::process::exit(2);
                }
            }
            if now > 900.0 {
                eprintln!("smoke: review not finished within 900 s");
                std::process::exit(2);
            }
        }
        if self.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.requested
            && self.advancing.is_none()
            && !self.reviewing
            && now >= self.delay
        {
            self.requested = true;
            let (path, done, repaint) = (self.path.clone(), self.done.clone(), ctx.clone());
            ctx.request_screenshot(move |image| {
                let [w, h] = image.size;
                let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
                for pixel in &image.pixels {
                    ppm.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b()]);
                }
                if let Err(e) = std::fs::write(&path, ppm) {
                    eprintln!("screenshot {}: {e}", path.display());
                }
                done.store(true, Ordering::SeqCst);
                repaint.request_repaint();
            });
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn main() -> eframe::Result {
    let fixed_seed = std::env::var("GRAND_COACH_SEED")
        .ok()
        .and_then(|s| s.parse().ok());
    let seed = fixed_seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    });
    let package_neural = std::env::var_os("GRAND_COACH_SCREENSHOT").is_some()
        && std::env::var("GRAND_COACH_PACKAGE_NEURAL").as_deref() == Ok("1");
    let scenario_name = std::env::var("GRAND_COACH_SMOKE_SCENARIO").ok();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Grand-Coach")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Grand-Coach",
        options,
        Box::new(move |cc| {
            let mut app = GrandCoachApp::new(
                &cc.egui_ctx,
                SessionConfig {
                    seed,
                    series: match scenario_name.as_deref() {
                        Some("series") => SeriesLength::Fixed(36),
                        Some("replay" | "review" | "practice") => SeriesLength::Fixed(2),
                        _ => SessionConfig::default().series,
                    },
                    display: if scenario_name.as_deref() == Some("open") {
                        TableDisplay::Open
                    } else {
                        TableDisplay::Closed
                    },
                    engine: if package_neural {
                        OpponentEngine::Neural
                    } else {
                        OpponentEngine::Classic
                    },
                    ..SessionConfig::default()
                },
            );
            let scenario = scenario_name.clone();
            let smoke = std::env::var_os("GRAND_COACH_SCREENSHOT").is_some() && scenario.is_some();
            if smoke {
                app.opponent_delay = 0.0;
                match std::env::var("GRAND_COACH_LANGUAGE").as_deref() {
                    Ok("de") => app.set_language(gui_egui::Language::De),
                    Ok("en") => app.set_language(gui_egui::Language::En),
                    _ => {}
                }
                if matches!(scenario.as_deref(), Some("coach" | "quiz-answer")) {
                    app.set_coach(CoachSettings {
                        level: CoachLevel::EveryMove,
                        ..CoachSettings::default()
                    });
                    app.set_assist(AssistSettings {
                        counting_aid: true,
                        quiz: true,
                        ..AssistSettings::default()
                    });
                }
            }
            app.set_show_seed(fixed_seed.is_some());
            if let Some(dir) = gui_egui::learning::data_dir() {
                app.use_learning_file(dir.join(gui_egui::learning::HISTORY_FILE));
            }
            if std::env::var("GRAND_COACH_VALIDATION").as_deref() == Ok("1") {
                app.prepare_validation();
            } else if smoke && scenario.as_deref() == Some("training") {
                app.smoke_training().map_err(|e| e.to_string())?;
            } else if !package_neural && !smoke {
                app.show_new_series();
            }
            Ok(match std::env::var_os("GRAND_COACH_SCREENSHOT") {
                Some(path) => Box::new(Screenshot {
                    app,
                    path: path.into(),
                    delay: std::env::var("GRAND_COACH_SCREENSHOT_DELAY")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(3.0),
                    requested: false,
                    advancing: if smoke {
                        match scenario.as_deref() {
                            Some("coach" | "quiz-answer") => Some(SmokeGoal::Question),
                            Some("series") => Some(SmokeGoal::Series),
                            Some("replay" | "review" | "practice") => Some(SmokeGoal::Game),
                            Some("open") => Some(SmokeGoal::Moves(12)),
                            _ => None,
                        }
                    } else {
                        None
                    },
                    replay_dir: (smoke
                        && matches!(scenario.as_deref(), Some("replay" | "review" | "practice")))
                    .then(gui_egui::learning::data_dir)
                    .flatten(),
                    review_wanted: smoke && scenario.as_deref() == Some("review"),
                    reviewing: false,
                    practice_wanted: smoke && scenario.as_deref() == Some("practice"),
                    quiz_answer_wanted: smoke && scenario.as_deref() == Some("quiz-answer"),
                    faces: smoke && scenario.as_deref() == Some("cards"),
                    done: Arc::default(),
                }),
                None => Box::new(app),
            })
        }),
    )
}
