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
//! question is open; `training` opens the focused counting training.
//! `GRAND_COACH_LANGUAGE=de|en` selects the language for that smoke.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;
use gui_egui::GrandCoachApp;
use skat_app::{AssistSettings, CoachLevel, CoachSettings, OpponentEngine, SessionConfig};

/// Wraps the app to take one screenshot and close (smoke test).
struct Screenshot {
    app: GrandCoachApp,
    path: std::path::PathBuf,
    delay: f64,
    requested: bool,
    /// Smoke scenario `coach`: still playing towards a counting question.
    advancing: bool,
    done: Arc<AtomicBool>,
}

impl eframe::App for Screenshot {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        if self.advancing {
            self.advancing = !self.app.smoke_advance(now);
            if !self.advancing {
                // Let the question window finish its fade-in before the screenshot.
                self.delay = self.delay.max(now + 1.5);
            }
            if self.advancing && now > 70.0 {
                eprintln!("smoke: no counting question within 70 s");
                std::process::exit(2);
            }
        }
        if self.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.requested && !self.advancing && now >= self.delay {
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
                    engine: if package_neural {
                        OpponentEngine::Neural
                    } else {
                        OpponentEngine::Classic
                    },
                    ..SessionConfig::default()
                },
            );
            let scenario = std::env::var("GRAND_COACH_SMOKE_SCENARIO").ok();
            let smoke = std::env::var_os("GRAND_COACH_SCREENSHOT").is_some() && scenario.is_some();
            if smoke {
                match std::env::var("GRAND_COACH_LANGUAGE").as_deref() {
                    Ok("de") => app.set_language(gui_egui::Language::De),
                    Ok("en") => app.set_language(gui_egui::Language::En),
                    _ => {}
                }
                if scenario.as_deref() == Some("coach") {
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
                app.open_training();
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
                    advancing: smoke && scenario.as_deref() == Some("coach"),
                    done: Arc::default(),
                }),
                None => Box::new(app),
            })
        }),
    )
}
