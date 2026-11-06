//! Saturn's warm, planetary Material-inspired color system for egui.

use eframe::egui::{self, Color32};

pub const BACKGROUND: Color32 = Color32::from_rgb(19, 18, 16);
pub const SURFACE: Color32 = Color32::from_rgb(28, 26, 23);
pub const SURFACE_CONTAINER: Color32 = Color32::from_rgb(39, 36, 31);
pub const SURFACE_HIGHEST: Color32 = Color32::from_rgb(55, 50, 43);
pub const OUTLINE: Color32 = Color32::from_rgb(91, 82, 69);
pub const TEXT: Color32 = Color32::from_rgb(238, 230, 216);
pub const MUTED: Color32 = Color32::from_rgb(183, 171, 151);
pub const PRIMARY: Color32 = Color32::from_rgb(238, 190, 112);
pub const ON_PRIMARY: Color32 = Color32::from_rgb(58, 37, 7);
pub const SECONDARY: Color32 = Color32::from_rgb(211, 192, 160);
pub const SUCCESS: Color32 = Color32::from_rgb(167, 207, 159);
pub const ERROR: Color32 = Color32::from_rgb(242, 178, 164);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(14, 13, 12);
    visuals.faint_bg_color = SURFACE_CONTAINER;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(102, 73, 34);
    visuals.selection.stroke = egui::Stroke::new(1.0, PRIMARY);

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, OUTLINE);

    visuals.widgets.inactive.bg_fill = SURFACE_CONTAINER;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_CONTAINER;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, OUTLINE);

    visuals.widgets.hovered.bg_fill = SURFACE_HIGHEST;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_HIGHEST;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, PRIMARY);

    visuals.widgets.active.bg_fill = Color32::from_rgb(94, 67, 34);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(94, 67, 34);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, PRIMARY);

    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.button_padding = egui::vec2(16.0, 9.0);
        style.spacing.interact_size.y = 38.0;
    });
}

pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(SURFACE_CONTAINER)
        .stroke(egui::Stroke::new(1.0, OUTLINE))
        .corner_radius(egui::CornerRadius::same(20))
        .inner_margin(egui::Margin::same(18))
        .show(ui, add_contents)
}

pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).strong().color(ON_PRIMARY))
            .fill(PRIMARY)
            .stroke(egui::Stroke::NONE)
            .corner_radius(egui::CornerRadius::same(18)),
    )
}

pub fn secondary_button(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .fill(SURFACE_HIGHEST)
            .stroke(egui::Stroke::new(1.0, OUTLINE))
            .corner_radius(egui::CornerRadius::same(18)),
    )
}

/// Paint a small Saturn mark using egui primitives, avoiding an external image asset.
pub fn paint_saturn(painter: &egui::Painter, center: egui::Pos2, radius: f32) {
    let ring_points = |start: f32, end: f32| {
        (0..=48)
            .map(|step| {
                let angle = start + (end - start) * step as f32 / 48.0;
                let x = radius * 1.65 * angle.cos();
                let y = radius * 0.55 * angle.sin();
                let tilt = -0.28_f32;
                egui::pos2(
                    center.x + x * tilt.cos() - y * tilt.sin(),
                    center.y + x * tilt.sin() + y * tilt.cos(),
                )
            })
            .collect::<Vec<_>>()
    };

    painter.add(egui::Shape::line(
        ring_points(0.0, std::f32::consts::TAU),
        egui::Stroke::new((radius * 0.28).max(1.5), Color32::from_rgb(174, 130, 75)),
    ));
    painter.circle_filled(center, radius, Color32::from_rgb(205, 157, 97));
    painter.circle_filled(
        center + egui::vec2(-radius * 0.22, -radius * 0.24),
        radius * 0.28,
        Color32::from_rgb(226, 187, 132),
    );
    painter.add(egui::Shape::line(
        ring_points(0.10, std::f32::consts::PI - 0.10),
        egui::Stroke::new((radius * 0.18).max(1.2), Color32::from_rgb(235, 195, 139)),
    ));
}
