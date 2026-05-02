use crate::app::AppState;
use crate::scanner::tree::{NodeId, NodeKind};
use crate::ui::{theme, widgets};
use egui::{Ui, Vec2};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        widgets::section_header(ui, "Directory tree");

        let tree = match &state.tree {
            Some(t) => t,
            None => {
                if state.scan_progress.scanning {
                    ui.spinner();
                    ui.label(egui::RichText::new("Scanning...").color(theme::TEXT_MUTED));
                }
                return;
            }
        };

        let root = tree.root();
        let root_size = tree.node(root).size;

        // Determine which directory to highlight:
        // If selected node is a file, highlight its parent directory
        let highlighted_dir = state.selected_node.and_then(|sel| {
            let tree = state.tree.as_ref()?;
            if tree.node(sel).is_dir() {
                Some(sel)
            } else {
                tree.node(sel).parent
            }
        });

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.spacing_mut().interact_size.y = 16.0;
                show_node(ui, state, root, root_size, highlighted_dir);
            });
    });
}

fn show_node(
    ui: &mut Ui,
    state: &mut AppState,
    id: NodeId,
    root_size: u64,
    highlighted_dir: Option<NodeId>,
) {
    let tree = match &state.tree {
        Some(t) => t,
        None => return,
    };

    let node = tree.node(id);
    if !tree.is_alive(id) {
        return;
    }

    let (children, expanded) = match &node.kind {
        NodeKind::Directory { children, expanded } => (children.clone(), *expanded),
        NodeKind::File { .. } => return,
    };

    let is_selected = highlighted_dir == Some(id);
    let name = tree.name(id).to_string();
    let size = node.size;
    let depth = node.depth;

    let indent = depth as f32 * 10.0;
    let has_children = !children.is_empty();

    let frame = if is_selected {
        egui::Frame::new()
            .fill(theme::BG_SELECTION)
            .inner_margin(egui::Margin { left: 3, right: 3, top: 1, bottom: 1 })
            .corner_radius(theme::RADIUS_SM)
    } else {
        egui::Frame::new()
            .inner_margin(egui::Margin { left: 3, right: 3, top: 1, bottom: 1 })
    };

    let frame_response = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(indent);

            if has_children {
                let arrow = if expanded { "▼" } else { "▶" };
                let arrow_resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(arrow).color(theme::TEXT_MUTED).size(9.0),
                    )
                    .sense(egui::Sense::click()),
                );
                if arrow_resp.clicked() {
                    if let Some(tree) = &mut state.tree {
                        if let NodeKind::Directory {
                            expanded: ref mut exp,
                            ..
                        } = tree.node_mut(id).kind
                        {
                            *exp = !*exp;
                        }
                    }
                    return;
                }
            } else {
                ui.add_space(10.0);
            }

            let name_color = if is_selected {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            };

            // Truncate name to avoid overflow. Reserve space for the size text
            // (and bar, when there's room).
            let avail = (ui.available_width() - 70.0).max(30.0);
            let max_chars = (avail / 6.5) as usize;
            let display_name = if name.len() > max_chars && max_chars > 3 {
                format!("{}…", &name[..max_chars - 1])
            } else {
                name.clone()
            };
            ui.label(egui::RichText::new(&display_name).color(name_color).size(12.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    egui::RichText::new(theme::format_size(size))
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                );
                if root_size > 0 && ui.available_width() > 32.0 {
                    let fraction = size as f32 / root_size as f32;
                    let bar_width = 28.0;
                    let (bar_rect, _) =
                        ui.allocate_exact_size(Vec2::new(bar_width, 3.0), egui::Sense::hover());
                    ui.painter().rect_filled(bar_rect, 1.5, theme::BAR_BG);
                    let filled = egui::Rect::from_min_size(
                        bar_rect.left_top(),
                        Vec2::new(bar_width * fraction, 3.0),
                    );
                    ui.painter().rect_filled(filled, 1.5, theme::BAR_FILL);
                }
            });
        });
    });

    let row_response = frame_response.response.interact(egui::Sense::click());
    if row_response.clicked() {
        state.selected_node = Some(id);
        state.selected_extension = None;
    }
    if row_response.double_clicked() {
        state.view_root = Some(id);
        state.zoom_stack.push(id);
        state.treemap_dirty = true;
    }

    // Reveal-in-tree: scroll this row into view if the treemap requested it.
    if state.scroll_dir_tree_to == Some(id) {
        row_response.scroll_to_me(Some(egui::Align::Center));
        state.scroll_dir_tree_to = None;
    }

    // Right-click → same context menu as the treemap. id is fixed for the
    // row, so no sticky capture is needed.
    row_response.context_menu(|ui| {
        crate::ui::context_menu::show(ui, state, id);
    });

    if expanded {
        let tree = match &state.tree {
            Some(t) => t,
            None => return,
        };
        let mut sorted_children = children;
        sorted_children.sort_by(|&a, &b| tree.node(b).size.cmp(&tree.node(a).size));
        for child_id in sorted_children {
            show_node(ui, state, child_id, root_size, highlighted_dir);
        }
    }
}
