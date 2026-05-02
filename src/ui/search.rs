use crate::app::AppState;
use crate::ui::{theme, widgets};
use egui::{CornerRadius, Margin, Stroke, Ui};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let frame = egui::Frame::new()
        .fill(theme::BG_PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .inner_margin(Margin::symmetric(10, 6));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("🔍")
                    .color(theme::ACCENT_LIGHT)
                    .size(13.0),
            );

            let response = ui.add(
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search files…")
                    .desired_width(ui.available_width() - 24.0)
                    .frame(false)
                    .font(egui::FontId::proportional(13.0))
                    .text_color(theme::TEXT_PRIMARY),
            );
            response.request_focus();

            if ui
                .add(
                    egui::Label::new(
                        egui::RichText::new("✕")
                            .color(theme::TEXT_MUTED)
                            .size(13.0),
                    )
                    .sense(egui::Sense::click()),
                )
                .clicked()
            {
                state.search_active = false;
                state.search_query.clear();
            }
        });
    });
    ui.add_space(8.0);
    widgets::subtle_divider(ui);
    ui.add_space(6.0);
}
