// SPDX-License-Identifier: EUPL-1.2

//! The quiet, modern look of the window (issue #134, D-72): light
//! surfaces around a dark table, clear type, generous spacing and one
//! accent colour for every control. Presentation only.

use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, Stroke, TextStyle, Vec2};

/// Background of the side panels and bars.
pub const SURFACE: Color32 = Color32::from_rgb(246, 247, 249);
/// Cards, windows and grouped sections on a surface.
pub const RAISED: Color32 = Color32::from_rgb(255, 255, 255);
/// Hairlines between areas.
pub const LINE: Color32 = Color32::from_rgb(222, 226, 232);
/// Body text.
pub const TEXT: Color32 = Color32::from_rgb(33, 37, 43);
/// The one accent of controls and selection.
pub const ACCENT: Color32 = Color32::from_rgb(28, 110, 140);
/// Text on the dark table.
pub const ON_TABLE: Color32 = Color32::from_rgb(240, 244, 240);
/// Secondary text on the dark table.
pub const ON_TABLE_WEAK: Color32 = Color32::from_rgb(196, 214, 202);
/// Highlight on the table: whose turn it is, bids.
pub const ON_TABLE_ACCENT: Color32 = Color32::from_rgb(255, 214, 120);

/// Corner radius of controls and sections.
const ROUND: u8 = 8;

/// Installs the light look with the project's spacing and type. The
/// window stays light also when the system (or a test harness) asks for a
/// dark theme: both theme styles get it.
pub fn apply(ctx: &egui::Context) {
    ctx.options_mut(|o| o.theme_preference = egui::ThemePreference::Light);
    ctx.all_styles_mut(|style| {
        style.visuals = egui::Visuals::light();
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(13.5)),
            (TextStyle::Button, FontId::proportional(13.5)),
            (TextStyle::Heading, FontId::proportional(18.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        let spacing = &mut style.spacing;
        spacing.item_spacing = Vec2::new(8.0, 4.0);
        spacing.button_padding = Vec2::new(10.0, 4.0);
        spacing.window_margin = Margin::same(12);
        spacing.menu_margin = Margin::same(8);
        let v = &mut style.visuals;
        v.panel_fill = SURFACE;
        v.window_fill = RAISED;
        v.extreme_bg_color = RAISED;
        v.faint_bg_color = Color32::from_rgb(239, 242, 245);
        v.window_stroke = Stroke::new(1.0, LINE);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(ROUND);
        v.hyperlink_color = ACCENT;
        v.selection.bg_fill = Color32::from_rgb(200, 228, 236);
        v.selection.stroke = Stroke::new(1.5, ACCENT);
        let w = &mut v.widgets;
        for state in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            state.corner_radius = CornerRadius::same(ROUND);
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        w.noninteractive.fg_stroke.color = TEXT;
        w.inactive.weak_bg_fill = Color32::from_rgb(234, 238, 242);
        w.inactive.bg_fill = Color32::from_rgb(234, 238, 242);
        w.hovered.weak_bg_fill = Color32::from_rgb(219, 233, 239);
        w.hovered.bg_fill = Color32::from_rgb(219, 233, 239);
        w.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        w.active.weak_bg_fill = Color32::from_rgb(196, 222, 232);
        w.active.bg_fill = Color32::from_rgb(196, 222, 232);
        w.active.bg_stroke = Stroke::new(1.5, ACCENT);
    });
}

/// A grouped section on a light surface: white, rounded, hairline border.
pub fn section() -> egui::Frame {
    egui::Frame::new()
        .fill(RAISED)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(10))
}

/// A seat on the dark table: a slightly lighter, rounded plate; outlined
/// in the highlight colour while that player is to move.
pub fn seat_plate(table: Color32, acting: bool) -> egui::Frame {
    let fill = table.lerp_to_gamma(Color32::WHITE, 0.08);
    let stroke = if acting {
        Stroke::new(2.0, ON_TABLE_ACCENT)
    } else {
        Stroke::new(1.0, table.lerp_to_gamma(Color32::WHITE, 0.2))
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(10))
}
