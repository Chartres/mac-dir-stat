//! The inventory (docs/SIGNATURE.md Move 1). Left: what tools can rebuild,
//! grouped by rule, each with what it takes to get it back (1A + 1B).
//! Right: what only you have, shown for awareness and never offered for
//! deletion. After a trash, a short closing account replaces the lists (1C).

use crate::app::{AppState, PendingAction};
use crate::cleanup::{restore_cost, Account, CleanupCandidate};
use crate::scanner::tree::NodeId;
use crate::ui::{theme, widgets};
use egui::{Context, RichText, Ui};

pub fn show(ctx: &Context, state: &mut AppState) {
    if !state.cleanup_window_open {
        return;
    }
    let mut open = state.cleanup_window_open;
    egui::Window::new("Cleanup")
        .open(&mut open)
        .default_width(860.0)
        .default_height(560.0)
        .min_width(560.0)
        .min_height(340.0)
        .show(ctx, |ui| {
            ui.spacing_mut().interact_size.y = 18.0;
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
            ui.label(
                RichText::new("What you can rebuild, and what you cannot")
                    .color(theme::ink())
                    .size(18.0),
            );
            ui.label(
                RichText::new(
                    "Nothing is deleted until you tick items and choose Move to Trash. \
                     If you are unsure about one, Reveal shows it in Finder.",
                )
                .family(theme::italic())
                .color(theme::note())
                .size(11.5),
            );
            ui.add_space(4.0);
            widgets::ink_rule(ui);
            ui.add_space(6.0);

            if let Some(account) = state.closing_account.clone() {
                closing_account(ui, state, &account);
                return;
            }
            if state.tree.is_none() {
                ui.label(RichText::new("Run a scan first.").color(theme::note()));
                return;
            }

            egui::TopBottomPanel::bottom("inventory_actions")
                .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(0, 6)))
                .show_inside(ui, |ui| action_bar(ui, state));

            egui::CentralPanel::default()
                .frame(egui::Frame::new())
                .show_inside(ui, |ui| {
                    ui.columns(2, |cols| {
                        rebuildable_column(&mut cols[0], state);
                        only_yours_column(&mut cols[1], state);
                    });
                });
        });
    if !open {
        // The account is for the visit it closes; the next visit starts
        // from the inventory.
        state.closing_account = None;
    }
    state.cleanup_window_open = open;
}

/// A rule's candidates, as one row of the left column.
pub struct Group<'a> {
    pub category: &'static str,
    pub description: &'static str,
    pub items: Vec<&'a CleanupCandidate>,
    pub size: u64,
    pub newest: std::time::SystemTime,
}

/// Candidates grouped by rule, largest group first; items largest first.
pub fn groups(candidates: &[CleanupCandidate]) -> Vec<Group<'_>> {
    let mut out: Vec<Group> = Vec::new();
    for c in candidates {
        match out.iter_mut().find(|g| g.category == c.category) {
            Some(g) => {
                g.items.push(c);
                g.size += c.size;
                g.newest = g.newest.max(c.newest);
            }
            None => out.push(Group {
                category: c.category,
                description: c.description,
                items: vec![c],
                size: c.size,
                newest: c.newest,
            }),
        }
    }
    for g in &mut out {
        g.items.sort_by(|a, b| b.size.cmp(&a.size));
    }
    out.sort_by(|a, b| b.size.cmp(&a.size).then(a.category.cmp(b.category)));
    out
}

fn column_head(ui: &mut Ui, title: &str, note: &str) {
    ui.label(RichText::new(title).color(theme::ink()).size(15.0));
    ui.label(
        RichText::new(note)
            .family(theme::italic())
            .color(theme::note())
            .size(11.0),
    );
    ui.add_space(6.0);
}

fn rebuildable_column(ui: &mut Ui, state: &mut AppState) {
    column_head(
        ui,
        "Can be rebuilt",
        "Tools will make these again. Removing them costs time, not data.",
    );
    if state.cleanup_candidates.is_empty() {
        ui.label(
            RichText::new("Nothing a tool can rebuild was found in this scan.")
                .color(theme::note())
                .size(12.0),
        );
        return;
    }
    let candidates = state.cleanup_candidates.clone();
    let groups = groups(&candidates);
    egui::ScrollArea::vertical()
        .id_salt("rebuildable")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for g in &groups {
                group_row(ui, state, g);
            }
        });
}

fn group_row(ui: &mut Ui, state: &mut AppState, g: &Group) {
    let ids: Vec<NodeId> = g.items.iter().map(|c| c.node_id).collect();
    let picked = ids.iter().filter(|id| state.cleanup_selected.contains(id)).count();
    ui.horizontal(|ui| {
        let mut all = picked == ids.len();
        let label = if g.items.len() > 1 {
            format!("{} ({})", g.category, g.items.len())
        } else {
            g.category.to_string()
        };
        let cb = egui::Checkbox::new(&mut all, RichText::new(label).color(theme::ink()).size(13.0))
            .indeterminate(picked > 0 && picked < ids.len());
        if ui.add(cb).on_hover_text(g.description).changed() {
            set_selected(state, &ids, all);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(theme::format_size(g.size))
                    .color(theme::ink_soft())
                    .size(12.0),
            );
        });
    });
    ui.indent(("restore", g.category), |ui| {
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "To get it back: {}. Last changed {}.",
                    restore_cost(g.category),
                    theme::format_age(g.newest)
                ))
                .family(theme::italic())
                .color(theme::note())
                .size(11.0),
            )
            .wrap(),
        )
        .on_hover_text(g.description);
    });
    let id = ui.make_persistent_id(("inventory_group", g.category));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label(
                RichText::new(if g.items.len() == 1 { "where" } else { "each one" })
                    .color(theme::faint())
                    .size(10.5),
            );
        })
        .body(|ui| {
            for c in &g.items {
                item_row(ui, state, c);
            }
        });
    ui.add_space(2.0);
    widgets::subtle_divider(ui);
    ui.add_space(4.0);
}

fn item_row(ui: &mut Ui, state: &mut AppState, c: &CleanupCandidate) {
    ui.horizontal(|ui| {
        let mut on = state.cleanup_selected.contains(&c.node_id);
        let path = short_path(&c.path.display().to_string(), 48);
        if ui
            .checkbox(&mut on, RichText::new(path).color(theme::ink_soft()).size(11.0))
            .on_hover_text(c.path.display().to_string())
            .changed()
        {
            set_selected(state, &[c.node_id], on);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Locate").clicked() {
                locate(state, c.node_id);
            }
            if ui.small_button("Reveal").clicked() {
                crate::platform::finder::reveal_in_finder(&c.path);
            }
            ui.label(
                RichText::new(theme::format_size(c.size))
                    .color(theme::note())
                    .size(10.5),
            );
        });
    });
}

fn only_yours_column(ui: &mut Ui, state: &mut AppState) {
    column_head(
        ui,
        "Only you have these",
        "Never suggested for removal. Shown so you know where the space is.",
    );
    if state.only_yours.is_empty() {
        ui.label(
            RichText::new(
                "None of your usual folders (Documents, Desktop, Pictures, Movies, Music, \
                 Mail, Messages) is in this scan.",
            )
            .color(theme::note())
            .size(12.0),
        );
        return;
    }
    let rows = state.only_yours.clone();
    egui::ScrollArea::vertical()
        .id_salt("only_yours")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for o in &rows {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&o.label).color(theme::ink()).size(13.0))
                        .on_hover_text(o.path.display().to_string());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Locate").clicked() {
                            locate(state, o.node_id);
                        }
                        ui.label(
                            RichText::new(theme::format_size(o.size))
                                .color(theme::ink_soft())
                                .size(12.0),
                        );
                    });
                });
                ui.add_space(2.0);
                widgets::subtle_divider(ui);
                ui.add_space(4.0);
            }
        });
}

fn action_bar(ui: &mut Ui, state: &mut AppState) {
    widgets::subtle_divider(ui);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let picked: Vec<&CleanupCandidate> = state
            .cleanup_candidates
            .iter()
            .filter(|c| state.cleanup_selected.contains(&c.node_id))
            .collect();
        let total: u64 = picked.iter().map(|c| c.size).sum();
        let items: Vec<_> = picked.iter().map(|c| (c.node_id, c.path.clone())).collect();
        ui.label(
            RichText::new(if items.is_empty() {
                "Nothing is ticked until you tick it.".to_string()
            } else {
                format!(
                    "{} ticked, {}.",
                    items.len(),
                    theme::format_size(total)
                )
            })
            .family(theme::italic())
            .color(theme::note())
            .size(11.5),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let n = items.len();
            ui.add_enabled_ui(n > 0, |ui| {
                if widgets::danger_button(ui, "Move to Trash…").clicked() {
                    state.pending_action = Some(PendingAction::ConfirmBatchTrash(items, total));
                }
            });
            if widgets::ghost_button(ui, "Empty Trash…").clicked() {
                state.pending_action = Some(PendingAction::ConfirmEmptyTrash);
            }
            if n > 0 && widgets::ghost_button(ui, "Clear").clicked() {
                state.cleanup_selected.clear();
            }
            // Never pressed for you (SIGNATURE.md, Refused).
            if !state.cleanup_candidates.is_empty()
                && widgets::ghost_button(ui, "Select all").clicked()
            {
                state.cleanup_selected =
                    state.cleanup_candidates.iter().map(|c| c.node_id).collect();
            }
        });
    });
}

fn closing_account(ui: &mut Ui, state: &mut AppState, a: &Account) {
    let line = |ui: &mut Ui, label: &str, value: String, size: f32| {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(label).color(theme::ink()).size(size));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(value).color(theme::ink()).size(size));
            });
        });
        ui.add_space(2.0);
        widgets::subtle_divider(ui);
        ui.add_space(6.0);
    };
    ui.add_space(8.0);
    ui.label(RichText::new("After cleanup").color(theme::ink()).size(15.0));
    ui.add_space(8.0);
    let items = if a.items == 1 { "1 item".to_string() } else { format!("{} items", a.items) };
    line(ui, &format!("Moved to Trash ({items})"), theme::format_size(a.moved), 14.0);
    line(ui, "of which can be rebuilt", theme::format_size(a.rebuildable), 13.0);
    line(ui, "of which no tool can rebuild", theme::format_size(a.only_yours), 13.0);
    if let Some((free, total)) = a.disk {
        line(
            ui,
            "Free on this disk now",
            format!("{} of {}", theme::format_size(free), theme::format_size(total)),
            13.0,
        );
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "The Trash is not emptied. Until you empty it, everything above can be taken \
             back out, and the space is not free yet.",
        )
        .family(theme::italic())
        .color(theme::note())
        .size(11.5),
    );
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::ghost_button(ui, "Close").clicked() {
                state.closing_account = None;
            }
        });
    });
}

fn set_selected(state: &mut AppState, ids: &[NodeId], on: bool) {
    for id in ids {
        if on {
            state.cleanup_selected.insert(*id);
        } else {
            state.cleanup_selected.remove(id);
        }
    }
}

fn locate(state: &mut AppState, id: NodeId) {
    state.selected_node = Some(id);
    state.scroll_dir_tree_to = Some(id);
    state.expand_to_node(id);
    state.treemap_dirty = true;
}

/// Keeps the head and tail of a long path: "/Users/p…/node_modules".
fn short_path(p: &str, max: usize) -> String {
    let n = p.chars().count();
    if n <= max {
        return p.to_string();
    }
    let head: String = p.chars().take(max / 3).collect();
    let tail: String = p.chars().skip(n - (max - max / 3 - 1)).collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::short_path;

    #[test]
    fn short_path_multibyte() {
        // ASCII short path passes through unchanged.
        assert_eq!(short_path("/usr/local/bin", 48), "/usr/local/bin");
        // Multibyte chars (Founder Case 2): result is char-correct and within max+1 (the ellipsis).
        let p = "/Users/用户/Downloads/node_modules/very/long/path";
        let result = short_path(p, 20);
        assert!(result.chars().count() <= 21, "result too long: {result}");
        assert!(result.contains('…'), "missing ellipsis: {result}");
        // Pure multibyte path: head and tail are chars, not bytes.
        let p2 = "あ".repeat(30);
        let result2 = short_path(&p2, 12);
        assert!(result2.chars().count() <= 13);
        assert!(result2.contains('…'));
    }
}
