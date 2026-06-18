//! One-time Full Disk Access prompt, shown on the first launch that lacks
//! access. Explains why, offers to open the settings pane, and never reappears.

use crate::app::AppState;
use crate::ui::{theme, widgets};
use egui::{Context, RichText};

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.show_fda_prompt {
        return;
    }
    egui::Window::new("Full Disk Access")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.set_max_width(400.0);
            ui.label(
                RichText::new("Scan your whole disk")
                    .color(theme::TEXT_PRIMARY)
                    .strong()
                    .size(14.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "macOS hides some folders (Mail, Messages, Photos, and other \
                     protected data) from apps unless you grant Full Disk Access. \
                     Without it MacDirStat still scans everything else — those \
                     areas just show up as “Hidden / Skipped.”\n\n\
                     To include them, add MacDirStat under Full Disk Access, then \
                     re-scan. You'll only be asked this once.",
                )
                .color(theme::TEXT_SECONDARY)
                .size(11.0),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::ghost_button(ui, "Open Settings…").clicked() {
                        crate::platform::fda::open_settings();
                        state.show_fda_prompt = false;
                    }
                    if widgets::ghost_button(ui, "Not now").clicked() {
                        state.show_fda_prompt = false;
                    }
                });
            });
        });
}
