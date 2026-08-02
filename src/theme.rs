//! Visual polish — dark-first data-tool palette.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(88, 166, 255);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(56, 110, 180);
pub const BG: Color32 = Color32::from_rgb(18, 20, 26);
pub const BG_PANEL: Color32 = Color32::from_rgb(24, 28, 36);
pub const BG_RAISED: Color32 = Color32::from_rgb(32, 38, 48);
pub const BG_SELECTED: Color32 = Color32::from_rgb(40, 58, 88);
pub const BORDER: Color32 = Color32::from_rgb(48, 56, 72);
pub const TEXT: Color32 = Color32::from_rgb(230, 234, 242);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 150, 168);
pub const OK: Color32 = Color32::from_rgb(110, 200, 140);
pub const ERR: Color32 = Color32::from_rgb(255, 120, 120);

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.indent = 16.0;
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: 10.0,
        handle_min_length: 28.0,
        bar_inner_margin: 2.0,
        bar_outer_margin: 0.0,
        floating: true,
        ..Default::default()
    };
    style.visuals = dark_visuals();
    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT;
    v.faint_bg_color = BG_PANEL;
    v.extreme_bg_color = BG;
    v.code_bg_color = BG_RAISED;
    v.window_fill = BG_PANEL;
    v.panel_fill = BG;
    v.selection.bg_fill = BG_SELECTED;
    v.selection.stroke = Stroke::new(1.0, ACCENT_DIM);
    v.widgets.noninteractive.bg_fill = BG_PANEL;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    v.widgets.inactive.bg_fill = BG_RAISED;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.hovered.bg_fill = Color32::from_rgb(48, 58, 78);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = ACCENT_DIM;
    v.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.open.bg_fill = BG_SELECTED;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
}

pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .strong()
            .color(ACCENT)
            .size(13.0),
    );
}
