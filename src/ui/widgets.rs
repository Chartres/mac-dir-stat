use crate::ui::theme;
use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Ui, Vec2};

/// Filled accent button — primary call to action.
pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = theme::ACCENT;
        v.inactive.weak_bg_fill = theme::ACCENT;
        v.inactive.bg_stroke = Stroke::NONE;
        v.inactive.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        v.hovered.bg_fill = theme::ACCENT_HOVER;
        v.hovered.weak_bg_fill = theme::ACCENT_HOVER;
        v.hovered.bg_stroke = Stroke::NONE;
        v.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        v.active.bg_fill = theme::ACCENT;
        v.active.weak_bg_fill = theme::ACCENT;
        v.active.bg_stroke = Stroke::new(1.0, theme::ACCENT_LIGHT);
        v.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        ui.add(egui::Button::new(
            RichText::new(label).color(Color32::WHITE).strong(),
        ))
    })
    .inner
}

/// Outlined / subtle button — secondary actions.
pub fn ghost_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = Color32::TRANSPARENT;
        v.inactive.weak_bg_fill = Color32::TRANSPARENT;
        v.inactive.bg_stroke = Stroke::new(1.0, theme::BORDER);
        v.inactive.fg_stroke = Stroke::new(1.0, theme::TEXT_SECONDARY);
        v.hovered.bg_fill = theme::BG_HOVER;
        v.hovered.weak_bg_fill = theme::BG_HOVER;
        v.hovered.bg_stroke = Stroke::new(1.0, theme::BORDER);
        v.hovered.fg_stroke = Stroke::new(1.0, theme::TEXT_PRIMARY);
        v.active.bg_fill = theme::BG_HOVER;
        v.active.weak_bg_fill = theme::BG_HOVER;
        v.active.bg_stroke = Stroke::new(1.0, theme::ACCENT);
        v.active.fg_stroke = Stroke::new(1.0, theme::TEXT_PRIMARY);
        ui.add(egui::Button::new(RichText::new(label)))
    })
    .inner
}

/// Destructive action button — red fill, used in confirm dialogs.
pub fn danger_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = theme::DANGER;
        v.inactive.weak_bg_fill = theme::DANGER;
        v.inactive.bg_stroke = Stroke::NONE;
        v.inactive.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        v.hovered.bg_fill = theme::DANGER_HOVER;
        v.hovered.weak_bg_fill = theme::DANGER_HOVER;
        v.hovered.bg_stroke = Stroke::NONE;
        v.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        v.active.bg_fill = theme::DANGER;
        v.active.weak_bg_fill = theme::DANGER;
        v.active.bg_stroke = Stroke::NONE;
        v.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        ui.add(egui::Button::new(
            RichText::new(label).color(Color32::WHITE).strong(),
        ))
    })
    .inner
}

/// Pill-style segmented control. Returns true if the selection changed.
pub fn segmented_control<T: Copy + PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    let frame = egui::Frame::new()
        .fill(theme::BG_DARK)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .inner_margin(Margin::same(2));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
            for &(value, label) in options {
                let is_active = *current == value;
                let response = ui
                    .scope(|ui| {
                        let v = &mut ui.style_mut().visuals.widgets;
                        let bg = if is_active {
                            theme::BG_ELEVATED
                        } else {
                            Color32::TRANSPARENT
                        };
                        let hover_bg = if is_active {
                            theme::BG_ELEVATED
                        } else {
                            theme::BG_HOVER
                        };
                        let fg = if is_active {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        };
                        v.inactive.bg_fill = bg;
                        v.inactive.weak_bg_fill = bg;
                        v.inactive.bg_stroke = Stroke::NONE;
                        v.inactive.fg_stroke = Stroke::new(1.0, fg);
                        v.inactive.corner_radius = CornerRadius::same(theme::RADIUS_SM);
                        v.hovered.bg_fill = hover_bg;
                        v.hovered.weak_bg_fill = hover_bg;
                        v.hovered.bg_stroke = Stroke::NONE;
                        v.hovered.fg_stroke = Stroke::new(1.0, theme::TEXT_PRIMARY);
                        v.hovered.corner_radius = CornerRadius::same(theme::RADIUS_SM);
                        v.active.bg_fill = theme::BG_ELEVATED;
                        v.active.weak_bg_fill = theme::BG_ELEVATED;
                        v.active.bg_stroke = Stroke::NONE;
                        v.active.fg_stroke = Stroke::new(1.0, theme::TEXT_PRIMARY);
                        v.active.corner_radius = CornerRadius::same(theme::RADIUS_SM);
                        ui.spacing_mut().button_padding = Vec2::new(10.0, 4.0);
                        ui.add(egui::Button::new(
                            RichText::new(label)
                                .size(11.5)
                                .color(if is_active {
                                    theme::TEXT_PRIMARY
                                } else {
                                    theme::TEXT_SECONDARY
                                }),
                        ))
                    })
                    .inner;
                if response.clicked() && !is_active {
                    *current = value;
                    changed = true;
                }
            }
        });
    });
    changed
}

/// Uppercased, letter-spaced section label — sits above panel content.
pub fn section_header(ui: &mut Ui, label: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(format!("  {}  ", label.to_uppercase()))
            .color(theme::TEXT_DIM)
            .size(10.0)
            .strong(),
    );
    ui.add_space(6.0);
}

/// Thin, low-contrast horizontal divider.
pub fn subtle_divider(ui: &mut Ui) {
    let height = 1.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, theme::BORDER_SUBTLE);
}
