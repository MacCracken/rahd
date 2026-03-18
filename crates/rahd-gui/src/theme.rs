//! AGNOS dark theme — shared palette across AGNOS desktop apps.

use egui::{Color32, CornerRadius, Stroke, Style, Visuals};

pub const BG_DARK: Color32 = Color32::from_rgb(24, 24, 28);
pub const BG_PANEL: Color32 = Color32::from_rgb(32, 32, 38);
pub const BG_WIDGET: Color32 = Color32::from_rgb(44, 44, 52);
pub const ACCENT: Color32 = Color32::from_rgb(0, 188, 212);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0, 131, 148);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 235);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 170);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 110);
pub const SUCCESS: Color32 = Color32::from_rgb(76, 175, 80);
pub const ERROR: Color32 = Color32::from_rgb(220, 60, 60);
pub const WARNING: Color32 = Color32::from_rgb(255, 183, 77);

/// Apply the AGNOS dark theme to the given context.
pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut visuals = Visuals::dark();

    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_DARK;
    visuals.extreme_bg_color = BG_DARK;
    visuals.faint_bg_color = BG_WIDGET;
    visuals.override_text_color = Some(TEXT_PRIMARY);

    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.widgets.noninteractive.bg_fill = BG_WIDGET;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = BG_WIDGET;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 65);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    style.visuals = visuals;
    ctx.set_style(style);
}
