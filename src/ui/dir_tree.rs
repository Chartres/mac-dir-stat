use crate::app::AppState;
use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use crate::ui::{theme, widgets};
use egui::{Ui, Vec2};

/// Sort a directory's live children by the active sort mode.
fn sorted_children(tree: &FileTree, parent: NodeId, sort: crate::app::DirSortMode) -> Vec<NodeId> {
    use crate::app::DirSortMode;
    let mut children: Vec<NodeId> = tree.node(parent).children().to_vec();
    children.retain(|&c| tree.is_alive(c));
    match sort {
        DirSortMode::Size => {
            children.sort_by(|&a, &b| tree.node(b).size.cmp(&tree.node(a).size))
        }
        DirSortMode::Items => children.sort_by(|&a, &b| {
            tree.node(b)
                .items()
                .cmp(&tree.node(a).items())
                .then(tree.node(b).size.cmp(&tree.node(a).size))
        }),
        DirSortMode::Recent => {
            children.sort_by(|&a, &b| tree.node(b).modified.cmp(&tree.node(a).modified))
        }
        DirSortMode::Name => children.sort_by(|&a, &b| {
            tree.name(a)
                .to_lowercase()
                .cmp(&tree.name(b).to_lowercase())
        }),
    }
    children
}

/// Directories in top-to-bottom display order (pre-order, honoring the active
/// sort and collapsed nodes). Drives keyboard navigation; mirrors what
/// `show_node` renders.
pub fn visible_dirs(tree: &FileTree, root: NodeId, sort: crate::app::DirSortMode) -> Vec<NodeId> {
    fn rec(tree: &FileTree, id: NodeId, sort: crate::app::DirSortMode, out: &mut Vec<NodeId>) {
        if !tree.is_alive(id) || !tree.node(id).is_dir() {
            return;
        }
        let is_root = out.is_empty();
        out.push(id);
        let expanded = is_root
            || matches!(
                &tree.node(id).kind,
                NodeKind::Directory { expanded: true, .. }
            );
        if expanded {
            for c in sorted_children(tree, id, sort) {
                if tree.node(c).is_dir() {
                    rec(tree, c, sort, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    rec(tree, root, sort, &mut out);
    out
}

/// The folder the list starts from: the one the map shows.
pub fn list_root(state: &AppState) -> Option<NodeId> {
    let tree = state.tree.as_ref()?;
    Some(
        state
            .view_root
            .filter(|&v| tree.is_alive(v) && tree.node(v).is_dir())
            .unwrap_or_else(|| tree.root()),
    )
}

/// Bytes held by `dir`'s own files (not its subfolders) and how many.
/// The synthetic Free Space and Hidden blocks of a volume scan are not files
/// and are left out.
pub fn loose_files(tree: &FileTree, dir: NodeId) -> (u64, usize) {
    tree.node(dir)
        .children()
        .iter()
        .filter(|&&c| tree.is_trashable(c) && !tree.node(c).is_dir())
        .fold((0, 0), |(b, n), &c| (b + tree.node(c).size, n + 1))
}

/// The scale for one level of the ledger: its largest row, folder or the
/// folder's loose files. Bars within a level compare by length.
pub fn level_max(tree: &FileTree, dir: NodeId, loose_bytes: u64) -> u64 {
    let dirs = tree
        .node(dir)
        .children()
        .iter()
        .filter(|&&c| tree.is_alive(c) && tree.node(c).is_dir())
        .map(|&c| tree.node(c).size)
        .max()
        .unwrap_or(0);
    dirs.max(loose_bytes)
}

/// The row the map's pointer is over: the deepest listed folder that holds
/// the hovered block. `None` when the pointer is off the map.
fn map_hover_chain(state: &AppState) -> Vec<NodeId> {
    let Some(tree) = &state.tree else { return vec![] };
    let Some(start) = state
        .hovered_node
        .or(state.hovered_dir)
        .filter(|&id| tree.is_alive(id))
    else {
        return vec![];
    };
    let mut chain = vec![start];
    let mut cur = tree.node(start).parent;
    while let Some(p) = cur {
        chain.push(p);
        cur = tree.node(p).parent;
    }
    chain
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            widgets::section_header(ui, "Folders");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = format!("Sort: {}", state.dir_sort.label());
                if ui
                    .small_button(egui::RichText::new(label).size(10.0))
                    .on_hover_text("Cycle sort order (Size · Name · Items · Recent)")
                    .clicked()
                {
                    state.dir_sort = state.dir_sort.next();
                }
            });
        });

        let tree = match &state.tree {
            Some(t) => t,
            None => {
                if state.scan_progress.scanning {
                    ui.spinner();
                    ui.label(egui::RichText::new("Scanning...").color(theme::note()));
                }
                return;
            }
        };

        // Locate (Cleanup window, refresh) may target a row outside the
        // zoomed folder; zoom back out so the row exists to scroll to.
        if let (Some(target), Some(lr)) = (state.scroll_dir_tree_to, list_root(state)) {
            let inside = tree.is_alive(target)
                && std::iter::successors(Some(target), |&n| tree.node(n).parent).any(|n| n == lr);
            if !inside && lr != tree.root() {
                state.view_root = Some(tree.root());
                state.zoom_stack = vec![tree.root()];
                state.treemap_dirty = true;
            }
        }
        let tree = state.tree.as_ref().unwrap();
        let root = list_root(state).unwrap_or_else(|| tree.root());
        let root_scale = tree.node(root).size;
        // Recomputed every frame, so it never holds an id from an old tree.
        state.ledger_hover = None;
        let map_chain = map_hover_chain(state);

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
                let ctx = Row { highlighted_dir, map_chain: &map_chain, list_root: root };
                show_node(ui, state, root, root_scale, &ctx);
            });
    });
}

struct Row<'a> {
    highlighted_dir: Option<NodeId>,
    /// The hovered map block and its ancestors.
    map_chain: &'a [NodeId],
    list_root: NodeId,
}

/// Width of the ledger's length bar at full scale, in points.
const BAR_W: f32 = 64.0;

/// One ledger row: name, length bar scaled to the largest row of its level,
/// exact size. `scale` is that largest row's size.
fn show_node(ui: &mut Ui, state: &mut AppState, id: NodeId, scale: u64, ctx: &Row) {
    let tree = match &state.tree {
        Some(t) => t,
        None => return,
    };
    if !tree.is_alive(id) {
        return;
    }
    let node = tree.node(id);
    let is_root = id == ctx.list_root;
    let (children, expanded) = match &node.kind {
        NodeKind::Directory { children, expanded } => (children.clone(), *expanded || is_root),
        NodeKind::File { .. } => return,
    };

    let is_selected = ctx.highlighted_dir == Some(id);
    // The map's pointer is inside this folder, and no deeper listed row
    // shows it.
    let map_here = ctx.map_chain.contains(&id)
        && !(expanded
            && children
                .iter()
                .any(|c| ctx.map_chain.contains(c) && tree.node(*c).is_dir()));
    let name = tree.name(id).to_string();
    let size = node.size;
    let depth = node.depth.saturating_sub(tree.node(ctx.list_root).depth);
    let items = node.items();
    let has_children = children.iter().any(|&c| tree.is_alive(c));

    // The row's fill depends on hover, known only after layout.
    let bg = ui.painter().add(egui::Shape::Noop);
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin { left: 3, right: 3, top: 2, bottom: 2 })
        .corner_radius(theme::RADIUS_SM);

    let frame_response = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(depth as f32 * 10.0);

            if has_children && !is_root {
                let arrow = if expanded { "▼" } else { "▶" };
                let arrow_resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(arrow).color(theme::note()).size(9.0),
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

            let name_color = if is_selected || map_here {
                theme::ink()
            } else {
                theme::ink_soft()
            };
            // Reserve room for size, bar and item count.
            let avail = (ui.available_width() - (BAR_W + 84.0)).max(30.0);
            let max_chars = (avail / 6.5) as usize;
            let display_name = if name.chars().count() > max_chars && max_chars > 3 {
                let s: String = name.chars().take(max_chars - 1).collect();
                format!("{}…", s)
            } else {
                name.clone()
            };
            ui.label(egui::RichText::new(&display_name).color(name_color).size(12.5));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                size_and_bar(ui, size, scale, theme::highlight());
                if ui.available_width() > 48.0 {
                    ui.label(
                        egui::RichText::new(format_count(items))
                            .color(theme::faint())
                            .size(9.5),
                    );
                }
            });
        });
    });

    let row_response = frame_response.response.interact(egui::Sense::click());
    if row_response.hovered() {
        state.ledger_hover = Some(id);
    }
    let fill = if is_selected {
        theme::bg_selection()
    } else if map_here || row_response.hovered() {
        theme::bg_hover()
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().set(
        bg,
        egui::Shape::rect_filled(row_response.rect, theme::RADIUS_SM, fill),
    );
    if row_response.clicked() {
        state.selected_node = Some(id);
        state.selected_extension = None;
    }
    if row_response.double_clicked() && !is_root {
        state.view_root = Some(id);
        state.zoom_stack.push(id);
        state.treemap_dirty = true;
    }

    if state.scroll_dir_tree_to == Some(id) {
        row_response.scroll_to_me(Some(egui::Align::Center));
        state.scroll_dir_tree_to = None;
    }

    row_response.context_menu(|ui| {
        crate::ui::context_menu::show(ui, state, id);
    });

    if expanded {
        let (order, child_scale, loose) = match &state.tree {
            Some(t) => {
                let loose = loose_files(t, id);
                (sorted_children(t, id, state.dir_sort), level_max(t, id, loose.0), loose)
            }
            None => return,
        };
        for child_id in order {
            show_node(ui, state, child_id, child_scale, ctx);
        }
        if loose.1 > 0 {
            loose_row(ui, depth + 1, loose, child_scale);
        }
    }
}

/// The folder's own files, as one row, so a level's lengths add up to it.
fn loose_row(ui: &mut Ui, depth: u16, (bytes, n): (u64, usize), scale: u64) {
    egui::Frame::new()
        .inner_margin(egui::Margin { left: 3, right: 3, top: 2, bottom: 2 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(depth as f32 * 10.0 + 10.0);
                let label = if n == 1 { "1 file".to_string() } else { format!("{n} files") };
                ui.label(
                    egui::RichText::new(label)
                        .family(theme::italic())
                        .color(theme::note())
                        .size(12.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    size_and_bar(ui, bytes, scale, theme::faint());
                });
            });
        });
}

fn size_and_bar(ui: &mut Ui, size: u64, scale: u64, color: egui::Color32) {
    ui.add_sized(
        Vec2::new(54.0, 14.0),
        egui::Label::new(
            egui::RichText::new(theme::format_size(size))
                .color(theme::ink_soft())
                .size(10.5),
        ),
    );
    if ui.available_width() > BAR_W + 8.0 {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(BAR_W, 5.0), egui::Sense::hover());
        let frac = if scale > 0 { (size as f32 / scale as f32).min(1.0) } else { 0.0 };
        let filled = egui::Rect::from_min_size(
            rect.left_top(),
            Vec2::new((BAR_W * frac).max(if size > 0 { 1.0 } else { 0.0 }), rect.height()),
        );
        ui.painter().rect_filled(filled, 0.0, color);
    }
}

/// Compact item-count formatting: 1234 → "1.2k", 12 → "12".
fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
