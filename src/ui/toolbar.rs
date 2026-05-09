use crate::app::AppState;
use crate::treemap::color::ColorMode;
use crate::ui::{theme, widgets};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if widgets::primary_button(ui, "Scan Directory…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select directory to scan")
                .set_directory(&state.scan_root)
                .pick_folder()
            {
                state.scan_root = path;
                state.request_rescan = true;
            }
        }

        if widgets::ghost_button(ui, "⟳  Refresh").clicked() {
            state.request_rescan = true;
        }

        if state.tree.is_some() {
            let count = state.cleanup_candidates.len();
            let total: u64 = state.cleanup_candidates.iter().map(|c| c.size).sum();
            let label = if count > 0 {
                format!("Cleanup…  ·  {}", theme::format_size(total))
            } else {
                "Cleanup…".to_string()
            };
            let resp = widgets::ghost_button(ui, &label);
            let resp = resp.on_hover_text(
                "Opens a review window listing safe-to-delete folders \
                 (caches, Xcode build artifacts, node_modules, simulators, …). \
                 Nothing is deleted until you tick items and confirm a Trash action.",
            );
            if resp.clicked() {
                state.cleanup_window_open = !state.cleanup_window_open;
            }
        }

        ui.add_space(8.0);

        let path_text = if state.scan_progress.scanning {
            format!("Scanning  {}", state.scan_root.display())
        } else if let Some(tree) = &state.tree {
            format!(
                "{}   {}",
                state.scan_root.display(),
                theme::format_size(tree.node(tree.root()).size),
            )
        } else {
            format!("{}", state.scan_root.display())
        };
        ui.label(
            RichText::new(path_text)
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::ghost_button(ui, "?").clicked() {
                state.help_window_open = !state.help_window_open;
            }
            ui.add_space(4.0);
            let mut current = state.color_mode;
            if widgets::segmented_control(
                ui,
                &mut current,
                &[
                    (ColorMode::Extension, "Extension"),
                    (ColorMode::Depth, "Depth"),
                    (ColorMode::Age, "Age"),
                ],
            ) {
                state.color_mode = current;
                state.treemap_dirty = true;
                crate::state::save(&state.scan_root, state.color_mode);
            }

            ui.label(
                RichText::new("Color by")
                    .color(theme::TEXT_MUTED)
                    .size(11.0),
            );
        });
    });
}
