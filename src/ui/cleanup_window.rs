use crate::app::{AppState, PendingAction};
use crate::cleanup::CleanupCandidate;
use crate::ui::{theme, widgets};
use egui::{Context, CornerRadius, Margin, RichText, Stroke};

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.cleanup_window_open {
        return;
    }

    let total_savings: u64 = state.cleanup_candidates.iter().map(|c| c.size).sum();
    let count = state.cleanup_candidates.len();

    let mut open = state.cleanup_window_open;
    egui::Window::new("Cleanup Suggestions")
        .open(&mut open)
        .default_width(620.0)
        .default_height(480.0)
        .min_width(380.0)
        .min_height(300.0)
        .show(ctx, |ui| {
            // Header
            ui.label(
                RichText::new("Cleanup Suggestions")
                    .color(theme::TEXT_PRIMARY)
                    .strong()
                    .size(15.0),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if state.tree.is_none() {
                        "Run a scan first to see suggestions.".to_string()
                    } else if count == 0 {
                        "No safe-to-clean directories found in this scan.".to_string()
                    } else {
                        format!(
                            "{} candidates · up to {} freeable",
                            count,
                            theme::format_size(total_savings),
                        )
                    })
                    .color(theme::TEXT_SECONDARY)
                    .size(11.0),
                );
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if widgets::ghost_button(ui, "Empty Trash…").clicked() {
                            state.pending_action = Some(
                                crate::app::PendingAction::ConfirmEmptyTrash,
                            );
                        }
                    },
                );
            });
            ui.label(
                RichText::new(
                    "Nothing is deleted until you tick items and click a Trash button. \
                     Each candidate has a description — if you're unsure, click Reveal \
                     to see it in Finder.",
                )
                .color(theme::TEXT_MUTED)
                .size(10.5)
                .italics(),
            );
            ui.add_space(6.0);

            // Batch action bar
            if !state.cleanup_candidates.is_empty() {
                ui.horizontal(|ui| {
                    let selected_count = state.cleanup_selected.len();
                    let selected_total: u64 = state
                        .cleanup_candidates
                        .iter()
                        .filter(|c| state.cleanup_selected.contains(&c.node_id))
                        .map(|c| c.size)
                        .sum();

                    if widgets::ghost_button(ui, "Select all").clicked() {
                        state.cleanup_selected = state
                            .cleanup_candidates
                            .iter()
                            .map(|c| c.node_id)
                            .collect();
                    }
                    if widgets::ghost_button(ui, "Clear").clicked() {
                        state.cleanup_selected.clear();
                    }
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let label = if selected_count == 0 {
                                "Trash selected".to_string()
                            } else {
                                format!(
                                    "Trash {} ({})",
                                    selected_count,
                                    theme::format_size(selected_total),
                                )
                            };
                            let enabled = selected_count > 0;
                            ui.add_enabled_ui(enabled, |ui| {
                                if widgets::danger_button(ui, &label).clicked() {
                                    let ids: Vec<crate::scanner::tree::NodeId> = state
                                        .cleanup_candidates
                                        .iter()
                                        .filter(|c| state.cleanup_selected.contains(&c.node_id))
                                        .map(|c| c.node_id)
                                        .collect();
                                    if !ids.is_empty() {
                                        state.pending_action = Some(
                                            crate::app::PendingAction::ConfirmBatchTrash(
                                                ids,
                                                selected_total,
                                            ),
                                        );
                                    }
                                }
                            });
                        },
                    );
                });
            }

            ui.add_space(4.0);
            widgets::subtle_divider(ui);
            ui.add_space(4.0);

            if state.cleanup_candidates.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("Nothing obvious to clean.")
                            .color(theme::TEXT_MUTED)
                            .size(12.0),
                    );
                });
                return;
            }

            // List
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let candidates = state.cleanup_candidates.clone();
                    let mut current_category = "";

                    // Group totals
                    let mut category_totals: std::collections::BTreeMap<&str, u64> =
                        std::collections::BTreeMap::new();
                    for c in &candidates {
                        *category_totals.entry(c.category).or_insert(0) += c.size;
                    }

                    for cand in &candidates {
                        if cand.category != current_category {
                            current_category = cand.category;
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(cand.category)
                                        .color(theme::TEXT_PRIMARY)
                                        .strong()
                                        .size(12.0),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let total = category_totals
                                            .get(cand.category)
                                            .copied()
                                            .unwrap_or(0);
                                        ui.label(
                                            RichText::new(theme::format_size(total))
                                                .color(theme::TEXT_SECONDARY)
                                                .size(11.0),
                                        );
                                    },
                                );
                            });
                            ui.label(
                                RichText::new(cand.description)
                                    .color(theme::TEXT_MUTED)
                                    .size(10.0)
                                    .italics(),
                            );
                            ui.add_space(2.0);
                        }

                        candidate_row(ui, state, cand);
                    }
                });
        });

    state.cleanup_window_open = open;
}

fn candidate_row(ui: &mut egui::Ui, state: &mut AppState, cand: &CleanupCandidate) {
    let is_selected = state.cleanup_selected.contains(&cand.node_id);
    let (fill, stroke) = if is_selected {
        (theme::BG_SELECTION, theme::ACCENT)
    } else {
        (theme::BG_DARK, theme::BORDER_SUBTLE)
    };
    let frame = egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(theme::RADIUS_SM))
        .inner_margin(Margin::symmetric(8, 6));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Selection checkbox
            let mut selected = state.cleanup_selected.contains(&cand.node_id);
            if ui.checkbox(&mut selected, "").changed() {
                if selected {
                    state.cleanup_selected.insert(cand.node_id);
                } else {
                    state.cleanup_selected.remove(&cand.node_id);
                }
            }
            // Path on the left, truncated
            ui.vertical(|ui| {
                let path_str = cand.path.display().to_string();
                let display_path = if path_str.chars().count() > 64 {
                    let mut iter = path_str.chars();
                    let head: String = iter.by_ref().take(28).collect();
                    let tail: String = iter
                        .skip(path_str.chars().count().saturating_sub(64).saturating_sub(28))
                        .collect();
                    format!("{}…{}", head, tail)
                } else {
                    path_str
                };
                ui.label(
                    RichText::new(display_path)
                        .color(theme::TEXT_PRIMARY)
                        .size(11.5),
                );
                ui.label(
                    RichText::new(theme::format_size(cand.size))
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
            });

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if widgets::ghost_button(ui, "Trash").clicked() {
                        let name = cand
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| cand.path.display().to_string());
                        state.pending_action = Some(PendingAction::ConfirmTrash(
                            cand.node_id,
                            name,
                            cand.size,
                        ));
                    }
                    if widgets::ghost_button(ui, "Reveal").clicked() {
                        crate::platform::finder::reveal_in_finder(&cand.path);
                    }
                    if widgets::ghost_button(ui, "Locate").clicked() {
                        state.selected_node = Some(cand.node_id);
                        state.scroll_dir_tree_to = Some(cand.node_id);
                        state.expand_to_node(cand.node_id);
                        state.treemap_dirty = true;
                    }
                },
            );
        });
    });
    ui.add_space(2.0);
}
