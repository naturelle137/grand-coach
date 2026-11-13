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
//! overrides it).
//!
//! Smoke test: with `GRAND_COACH_SCREENSHOT=<file.ppm>` the window saves a
//! screenshot after `GRAND_COACH_SCREENSHOT_DELAY` seconds (default 3) and
//! closes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;
use gui_egui::GrandCoachApp;
use skat_app::SessionConfig;

/// Wraps the app to take one screenshot and close (smoke test).
struct Screenshot {
    app: GrandCoachApp,
    path: std::path::PathBuf,
    delay: f64,
    requested: bool,
    done: Arc<AtomicBool>,
}

impl eframe::App for Screenshot {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
        let ctx = ui.ctx().clone();
        if self.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.requested && ctx.input(|i| i.time) >= self.delay {
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
                    ..SessionConfig::default()
                },
            );
            app.set_show_seed(fixed_seed.is_some());
            if let Some(dir) = gui_egui::learning::data_dir() {
                app.use_learning_file(dir.join(gui_egui::learning::HISTORY_FILE));
            }
            app.show_new_series();
            Ok(match std::env::var_os("GRAND_COACH_SCREENSHOT") {
                Some(path) => Box::new(Screenshot {
                    app,
                    path: path.into(),
                    delay: std::env::var("GRAND_COACH_SCREENSHOT_DELAY")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(3.0),
                    requested: false,
                    done: Arc::default(),
                }),
                None => Box::new(app),
            })
        }),
    )
}
