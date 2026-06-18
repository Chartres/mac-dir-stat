use crate::app::AppState;
use crate::treemap::color::extension_color;
use crate::ui::{theme, widgets};
use egui::{Color32, CornerRadius, Ui, Vec2};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    use crate::app::ExtSortMode;
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            widgets::section_header(ui, "File types");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = format!("Sort: {}", state.ext_sort.label());
                if ui
                    .small_button(egui::RichText::new(label).size(10.0))
                    .on_hover_text("Cycle sort order (Size · Count · Name)")
                    .clicked()
                {
                    state.ext_sort = state.ext_sort.next();
                }
            });
        });

        if state.tree.is_none() {
            if state.scan_progress.scanning {
                ui.spinner();
            }
            return;
        }

        let total_size: u64 = state.extension_stats.iter().map(|(_, b, _)| *b).sum();

        // Sort a display copy by the active mode (stats are size-sorted at
        // collection time).
        let mut stats: Vec<(String, u64, usize)> = state.extension_stats.clone();
        match state.ext_sort {
            ExtSortMode::Size => stats.sort_by(|a, b| b.1.cmp(&a.1)),
            ExtSortMode::Count => stats.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1))),
            ExtSortMode::Name => stats.sort_by(|a, b| a.0.cmp(&b.0)),
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let max_to_show = 50;
                let show_count = stats.len().min(max_to_show);

                for i in 0..show_count {
                    let (ext, bytes, count) = &stats[i];
                    let is_selected = state.selected_extension.as_deref() == Some(ext.as_str());
                    let colors = extension_color(if ext.is_empty() { "" } else { ext });
                    let swatch_color = Color32::from_rgba_premultiplied(
                        colors.0[0], colors.0[1], colors.0[2], colors.0[3],
                    );

                    let ext_id = ui.make_persistent_id(("ext_row", i));

                    let response = ui.horizontal(|ui| {
                        // Color swatch — slightly larger pill
                        let (swatch_rect, _) =
                            ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(swatch_rect, CornerRadius::same(3), swatch_color);

                        // Extension name — fixed width so it doesn't get clipped
                        let ext_display = if ext.is_empty() {
                            "(none)".to_string()
                        } else {
                            format!(".{}", ext)
                        };
                        let name_color = if is_selected {
                            theme::ACCENT_LIGHT
                        } else {
                            theme::TEXT_PRIMARY
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&ext_display).color(name_color).size(11.0),
                            )
                            .truncate(),
                        );

                        // Right-aligned: size and percentage only (no bar to save space)
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let pct = if total_size > 0 {
                                    *bytes as f64 / total_size as f64 * 100.0
                                } else {
                                    0.0
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", pct))
                                        .color(theme::TEXT_MUTED)
                                        .size(10.0),
                                );
                                ui.label(
                                    egui::RichText::new(theme::format_size(*bytes))
                                        .color(theme::TEXT_SECONDARY)
                                        .size(10.0),
                                );
                                // File count for this type (WinDirStat "Files").
                                if ui.available_width() > 36.0 {
                                    ui.label(
                                        egui::RichText::new(format!("{}×", count))
                                            .color(theme::TEXT_MUTED)
                                            .size(9.0),
                                    );
                                }
                            },
                        );
                    });

                    let row_rect = response.response.rect;
                    let row_sense = ui.interact(row_rect, ext_id, egui::Sense::click());
                    if row_sense.clicked() {
                        if is_selected {
                            state.selected_extension = None;
                        } else {
                            state.selected_extension = Some(ext.clone());
                            state.selected_node = None;
                        }
                    }
                }

                if stats.len() > max_to_show {
                    let remaining: u64 =
                        stats[max_to_show..].iter().map(|(_, b, _)| *b).sum();
                    let remaining_count = stats.len() - max_to_show;
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "+ {} more types ({})",
                            remaining_count,
                            theme::format_size(remaining)
                        ))
                        .color(theme::TEXT_MUTED)
                        .size(10.0)
                        .italics(),
                    );
                }
            });
    });
}
