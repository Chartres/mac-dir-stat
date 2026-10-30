use crate::ui::theme;
use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Ui, Vec2};

/// Filled accent button — primary call to action.
pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = theme::accent();
        v.inactive.weak_bg_fill = theme::accent();
        v.inactive.bg_stroke = Stroke::NONE;
        v.inactive.fg_stroke = Stroke::new(1.0, theme::on_accent());
        v.hovered.bg_fill = theme::accent_hover();
        v.hovered.weak_bg_fill = theme::accent_hover();
        v.hovered.bg_stroke = Stroke::NONE;
        v.hovered.fg_stroke = Stroke::new(1.0, theme::on_accent());
        v.active.bg_fill = theme::accent();
        v.active.weak_bg_fill = theme::accent();
        v.active.bg_stroke = Stroke::new(1.0, theme::highlight());
        v.active.fg_stroke = Stroke::new(1.0, theme::on_accent());
        ui.add(egui::Button::new(
            RichText::new(label).color(theme::on_accent()).strong(),
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
        v.inactive.bg_stroke = Stroke::new(1.0, theme::rule());
        v.inactive.fg_stroke = Stroke::new(1.0, theme::ink_soft());
        v.hovered.bg_fill = theme::bg_hover();
        v.hovered.weak_bg_fill = theme::bg_hover();
        v.hovered.bg_stroke = Stroke::new(1.0, theme::rule());
        v.hovered.fg_stroke = Stroke::new(1.0, theme::ink());
        v.active.bg_fill = theme::bg_hover();
        v.active.weak_bg_fill = theme::bg_hover();
        v.active.bg_stroke = Stroke::new(1.0, theme::accent());
        v.active.fg_stroke = Stroke::new(1.0, theme::ink());
        ui.add(egui::Button::new(RichText::new(label)))
    })
    .inner
}

/// Destructive action button — red fill, used in confirm dialogs.
pub fn danger_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = theme::danger();
        v.inactive.weak_bg_fill = theme::danger();
        v.inactive.bg_stroke = Stroke::NONE;
        v.inactive.fg_stroke = Stroke::new(1.0, theme::on_accent());
        v.hovered.bg_fill = theme::danger_hover();
        v.hovered.weak_bg_fill = theme::danger_hover();
        v.hovered.bg_stroke = Stroke::NONE;
        v.hovered.fg_stroke = Stroke::new(1.0, theme::on_accent());
        v.active.bg_fill = theme::danger();
        v.active.weak_bg_fill = theme::danger();
        v.active.bg_stroke = Stroke::NONE;
        v.active.fg_stroke = Stroke::new(1.0, theme::on_accent());
        ui.add(egui::Button::new(
            RichText::new(label).color(theme::on_accent()).strong(),
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
        .fill(theme::bg_base())
        .stroke(Stroke::new(1.0, theme::rule()))
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
                            theme::bg_raised()
                        } else {
                            Color32::TRANSPARENT
                        };
                        let hover_bg = if is_active {
                            theme::bg_raised()
                        } else {
                            theme::bg_hover()
                        };
                        let fg = if is_active {
                            theme::ink()
                        } else {
                            theme::ink_soft()
                        };
                        v.inactive.bg_fill = bg;
                        v.inactive.weak_bg_fill = bg;
                        v.inactive.bg_stroke = Stroke::NONE;
                        v.inactive.fg_stroke = Stroke::new(1.0, fg);
                        v.inactive.corner_radius = CornerRadius::same(theme::RADIUS_SM);
                        v.hovered.bg_fill = hover_bg;
                        v.hovered.weak_bg_fill = hover_bg;
                        v.hovered.bg_stroke = Stroke::NONE;
                        v.hovered.fg_stroke = Stroke::new(1.0, theme::ink());
                        v.hovered.corner_radius = CornerRadius::same(theme::RADIUS_SM);
                        v.active.bg_fill = theme::bg_raised();
                        v.active.weak_bg_fill = theme::bg_raised();
                        v.active.bg_stroke = Stroke::NONE;
                        v.active.fg_stroke = Stroke::new(1.0, theme::ink());
                        v.active.corner_radius = CornerRadius::same(theme::RADIUS_SM);
                        ui.spacing_mut().button_padding = Vec2::new(10.0, 4.0);
                        ui.add(egui::Button::new(
                            RichText::new(label)
                                .size(11.5)
                                .color(if is_active {
                                    theme::ink()
                                } else {
                                    theme::ink_soft()
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
            .color(theme::faint())
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
    ui.painter().rect_filled(rect, 0.0, theme::rule_faint());
}
