use crate::app::AppState;
use crate::treemap::color::ColorMode;
use crate::ui::theme;
use egui::Ui;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let scan_btn = ui.add(
            egui::Button::new(
                egui::RichText::new("Scan Directory...")
                    .color(egui::Color32::WHITE)
                    .size(12.0),
            )
            .fill(theme::ACCENT),
        );
        if scan_btn.clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select directory to scan")
                .set_directory(&state.scan_root)
                .pick_folder()
            {
                state.scan_root = path;
                state.request_rescan = true;
            }
        }

        let refresh_btn = ui.add(
            egui::Button::new(
                egui::RichText::new("⟳ Refresh")
                    .color(theme::ACCENT_LIGHT)
                    .size(12.0),
            )
            .fill(theme::BUTTON_BG),
        );
        if refresh_btn.clicked() {
            state.request_rescan = true;
        }

        ui.add_space(8.0);
        let path_text = if state.scan_progress.scanning {
            format!("Scanning {}...", state.scan_root.display())
        } else if let Some(tree) = &state.tree {
            format!(
                "{} — {}",
                state.scan_root.display(),
                theme::format_size(tree.node(tree.root()).size),
            )
        } else {
            format!("{}", state.scan_root.display())
        };
        ui.label(egui::RichText::new(path_text).color(theme::TEXT_SECONDARY).size(12.0));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (mode, label) in [
                (ColorMode::Age, "Age"),
                (ColorMode::Depth, "Depth"),
                (ColorMode::Extension, "Extension"),
            ] {
                let is_active = state.color_mode == mode;
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(label)
                            .color(if is_active { theme::ACCENT_LIGHT } else { theme::TEXT_SECONDARY })
                            .size(11.0),
                    )
                    .fill(if is_active { theme::BUTTON_BG } else { theme::BG_PANEL })
                    .stroke(if is_active {
                        egui::Stroke::new(1.0, theme::BUTTON_ACTIVE_BORDER)
                    } else {
                        egui::Stroke::NONE
                    }),
                );
                if btn.clicked() && !is_active {
                    state.color_mode = mode;
                    state.treemap_dirty = true;
                }
            }
        });
    });
}
