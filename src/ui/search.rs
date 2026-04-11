use crate::app::AppState;
use crate::ui::theme;
use egui::Ui;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🔍").color(theme::ACCENT_LIGHT).size(14.0));

        let response = ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Search files...")
                .desired_width(ui.available_width() - 40.0)
                .font(egui::FontId::proportional(13.0)),
        );

        // Auto-focus
        response.request_focus();

        if ui.add(
            egui::Label::new(egui::RichText::new("✕").color(theme::TEXT_MUTED).size(14.0))
                .sense(egui::Sense::click()),
        ).clicked() {
            state.search_active = false;
            state.search_query.clear();
        }
    });
    ui.add_space(4.0);
}
