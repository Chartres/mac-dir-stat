use crate::ui::theme;
use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Ui, Vec2};

/// Solid-fill button with a bold white label. `active_stroke` is the
/// pressed-state outline. Shared by `primary_button` and `danger_button`.
fn filled_button(
    ui: &mut Ui,
    label: &str,
    fill: Color32,
    hover_fill: Color32,
    active_stroke: Stroke,
) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        for (state, bg) in [
            (&mut v.inactive, fill),
            (&mut v.hovered, hover_fill),
            (&mut v.active, fill),
        ] {
            state.bg_fill = bg;
            state.weak_bg_fill = bg;
            state.bg_stroke = Stroke::NONE;
            state.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        }
        v.active.bg_stroke = active_stroke;
        ui.add(egui::Button::new(
            RichText::new(label).color(Color32::WHITE).strong(),
        ))
    })
    .inner
}

/// Filled accent button — primary call to action.
pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    filled_button(
        ui,
        label,
        theme::ACCENT,
        theme::ACCENT_HOVER,
        Stroke::new(1.0, theme::ACCENT_LIGHT),
    )
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
    filled_button(ui, label, theme::DANGER, theme::DANGER_HOVER, Stroke::NONE)
}

/// Centered, chrome-free confirmation modal: heading, one line of detail, and
/// a danger/cancel button pair. `id` is the egui window id (the title bar is
/// hidden, so it is never shown). Returns `None` while the user hasn't chosen,
/// `Some(true)` on confirm, `Some(false)` on cancel.
pub fn confirm_dialog(
    ctx: &egui::Context,
    id: &str,
    heading: &str,
    body: &str,
    confirm_label: &str,
) -> Option<bool> {
    let mut choice = None;
    egui::Window::new(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.set_max_width(360.0);
            ui.label(
                RichText::new(heading)
                    .color(theme::TEXT_PRIMARY)
                    .size(13.0)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(body)
                    .color(theme::TEXT_SECONDARY)
                    .size(11.0),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if danger_button(ui, confirm_label).clicked() {
                        choice = Some(true);
                    }
                    if ghost_button(ui, "Cancel").clicked() {
                        choice = Some(false);
                    }
                });
            });
        });
    choice
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
                            RichText::new(label).size(11.5).color(fg),
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
