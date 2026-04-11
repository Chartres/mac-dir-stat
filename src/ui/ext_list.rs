use crate::app::AppState;
use crate::treemap::color::extension_color;
use crate::ui::theme;
use egui::{Color32, Ui, Vec2};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("FILE TYPES")
                .color(theme::SECTION_HEADER)
                .size(9.0)
                .strong(),
        );
        ui.add_space(4.0);

        if state.tree.is_none() {
            if state.scan_progress.scanning {
                ui.spinner();
            }
            return;
        }

        let total_size: u64 = state.extension_stats.iter().map(|(_, b, _)| *b).sum();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let max_to_show = 50;
                let stats = &state.extension_stats;
                let show_count = stats.len().min(max_to_show);

                for i in 0..show_count {
                    let (ext, bytes, _count) = &stats[i];
                    let is_selected = state.selected_extension.as_deref() == Some(ext.as_str());
                    let colors = extension_color(if ext.is_empty() { "" } else { ext });

                    let ext_id = ui.make_persistent_id(("ext_row", i));

                    let response = ui.horizontal(|ui| {
                        let (swatch_rect, _) =
                            ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            swatch_rect,
                            3.0,
                            Color32::from_rgba_premultiplied(
                                colors.0[0],
                                colors.0[1],
                                colors.0[2],
                                colors.0[3],
                            ),
                        );

                        let ext_display = if ext.is_empty() {
                            "(no ext)".to_string()
                        } else {
                            format!(".{}", ext)
                        };
                        let name_color =
                            if is_selected { theme::ACCENT_LIGHT } else { theme::TEXT_PRIMARY };
                        ui.label(
                            egui::RichText::new(&ext_display).color(name_color).size(12.0),
                        );

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

                                if total_size > 0 {
                                    let fraction = *bytes as f32 / total_size as f32;
                                    let bar_width = 50.0;
                                    let (bar_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(bar_width, 5.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(bar_rect, 3.0, theme::BAR_BG);
                                    let filled = egui::Rect::from_min_size(
                                        bar_rect.left_top(),
                                        Vec2::new(bar_width * fraction, 5.0),
                                    );
                                    ui.painter().rect_filled(
                                        filled,
                                        3.0,
                                        Color32::from_rgba_premultiplied(
                                            colors.0[0],
                                            colors.0[1],
                                            colors.0[2],
                                            colors.0[3],
                                        ),
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
