use crate::app::AppState;
use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use crate::ui::{theme, widgets};
use egui::{CursorIcon, Pos2, Rect, Sense, Ui, Vec2};

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
/// sort and collapsed nodes). Drives keyboard navigation; the folder rows of
/// `flatten`, so it always matches what the list renders.
pub fn visible_dirs(tree: &FileTree, root: NodeId, sort: crate::app::DirSortMode) -> Vec<NodeId> {
    flatten(tree, root, sort)
        .into_iter()
        .filter(|r| r.kind == RowKind::Dir)
        .map(|r| r.id)
        .collect()
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

/// Height of one ledger row, points. Fixed, so only visible rows are laid out.
pub const ROW_H: f32 = 20.0;
/// Most ancestor rows pinned at the top while scrolling.
pub const MAX_STICKY: usize = 4;
/// Width of the ledger's length bar at full scale, in points.
const BAR_W: f32 = 64.0;
const INDENT: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowKind {
    Dir,
    /// The folder's own files as one row; `id` is the folder.
    Loose { bytes: u64, n: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgerRow {
    pub id: NodeId,
    /// Levels below the list's top row.
    pub depth: u16,
    /// Size of the largest row at this level, for the length bar.
    pub scale: u64,
    pub kind: RowKind,
    /// A folder with something inside: gets a triangle.
    pub has_kids: bool,
}

/// Every row the list shows, top to bottom. Only rows in view are drawn.
pub fn flatten(tree: &FileTree, root: NodeId, sort: crate::app::DirSortMode) -> Vec<LedgerRow> {
    fn rec(
        tree: &FileTree,
        id: NodeId,
        depth: u16,
        scale: u64,
        sort: crate::app::DirSortMode,
        out: &mut Vec<LedgerRow>,
    ) {
        if !tree.is_alive(id) || !tree.node(id).is_dir() {
            return;
        }
        let node = tree.node(id);
        out.push(LedgerRow {
            id,
            depth,
            scale,
            kind: RowKind::Dir,
            has_kids: node.children().iter().any(|&c| tree.is_alive(c)),
        });
        let expanded = depth == 0
            || matches!(&node.kind, NodeKind::Directory { expanded: true, .. });
        if !expanded {
            return;
        }
        let loose = loose_files(tree, id);
        let child_scale = level_max(tree, id, loose.0);
        for c in sorted_children(tree, id, sort) {
            rec(tree, c, depth + 1, child_scale, sort, out);
        }
        if loose.1 > 0 {
            out.push(LedgerRow {
                id,
                depth: depth + 1,
                scale: child_scale,
                kind: RowKind::Loose { bytes: loose.0, n: loose.1 },
                has_kids: false,
            });
        }
    }
    let mut out = Vec::new();
    rec(tree, root, 0, tree.node(root).size, sort, &mut out);
    out
}

/// Flip a folder open or shut. Returns the new state, `None` for a file.
pub fn toggle_expanded(tree: &mut FileTree, id: NodeId) -> Option<bool> {
    match &mut tree.node_mut(id).kind {
        NodeKind::Directory { expanded, .. } => {
            *expanded = !*expanded;
            Some(*expanded)
        }
        NodeKind::File { .. } => None,
    }
}

/// Row indices of the ancestors of `rows[i]`, outermost first.
fn ancestors(rows: &[LedgerRow], i: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut d = rows[i].depth;
    for j in (0..i).rev() {
        if d == 0 {
            break;
        }
        if rows[j].depth < d && rows[j].kind == RowKind::Dir {
            out.push(j);
            d = rows[j].depth;
        }
    }
    out.reverse();
    out
}

/// The ancestor rows to pin above the viewport when row `first` is its top
/// edge. The pinned rows hide the next ones, so the chain is taken for the
/// first row they leave visible.
pub fn sticky_chain(rows: &[LedgerRow], first: usize) -> Vec<usize> {
    if rows.is_empty() {
        return vec![];
    }
    let last = rows.len() - 1;
    for n in 0..=MAX_STICKY {
        let c = ancestors(rows, (first + n).min(last));
        if c.len() == n {
            return c;
        }
    }
    let mut c = ancestors(rows, (first + MAX_STICKY).min(last));
    c.drain(..c.len() - MAX_STICKY.min(c.len()));
    c
}

/// Where the triangle of a row sits.
pub fn arrow_rect(row: Rect, depth: u16) -> Rect {
    let x = row.left() + 3.0 + depth as f32 * INDENT;
    Rect::from_min_size(Pos2::new(x - 2.0, row.top()), Vec2::new(16.0, row.height()))
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    state.ledger_arrows.clear();
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            widgets::section_header(ui, "Folders");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = format!("Sort: {}", state.dir_sort.label());
                if ui
                    .small_button(egui::RichText::new(label).size(10.0))
                    .on_hover_text("Cycle sort order (Size · Name · Items · Recent)")
                    .on_hover_cursor(CursorIcon::PointingHand)
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
                let r = tree.root();
                crate::nav::navigate_to(state, r);
            }
        }
        let tree = state.tree.as_ref().unwrap();
        let root = list_root(state).unwrap_or_else(|| tree.root());
        // Recomputed every frame, so it never holds an id from an old tree.
        state.ledger_hover = None;
        let map_chain = map_hover_chain(state);
        let rows = flatten(tree, root, state.dir_sort);

        // The selected file's folder is the one to highlight.
        let highlighted_dir = state
            .selected_node
            .filter(|&s| tree.is_alive(s))
            .and_then(|sel| if tree.node(sel).is_dir() { Some(sel) } else { tree.node(sel).parent });

        let index_of = |id: NodeId| rows.iter().position(|r| r.id == id && r.kind == RowKind::Dir);
        let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if let Some(i) = state.scroll_dir_tree_top.take().and_then(index_of) {
            let pinned = ancestors(&rows, i).len().min(MAX_STICKY);
            area = area.vertical_scroll_offset(((i - pinned.min(i)) as f32 * ROW_H).max(0.0));
        } else if let Some(i) = state.scroll_dir_tree_to.take().and_then(index_of) {
            let off = i as f32 * ROW_H - (ui.available_height() - ROW_H) / 2.0;
            area = area.vertical_scroll_offset(off.max(0.0));
        }

        let cx = Ctx { highlighted_dir, map_chain: &map_chain };
        let mut clicked: Vec<(NodeId, Click)> = Vec::new();
        // show_rows derives its row stride from this, so set it before.
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.visuals_mut().clip_rect_margin = 0.0;
        let out = area.show_rows(ui, ROW_H, rows.len(), |ui, range| {
            for i in range {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::hover());
                if let Some(c) = row_ui(ui, state, &rows[i], rect, i as u64, &cx) {
                    clicked.push(c);
                }
            }
        });

        // Pinned ancestors: drawn over the top of the viewport.
        let first = (out.state.offset.y / ROW_H).floor().max(0.0) as usize;
        let pinned = sticky_chain(&rows, first);
        if !pinned.is_empty() {
            let top = out.inner_rect.top();
            let layer = ui.painter().with_clip_rect(out.inner_rect);
            let bottom = top + pinned.len() as f32 * ROW_H;
            layer.rect_filled(
                Rect::from_min_max(out.inner_rect.left_top(), Pos2::new(out.inner_rect.right(), bottom)),
                0.0,
                theme::bg_raised(),
            );
            layer.hline(out.inner_rect.x_range(), bottom, egui::Stroke::new(1.0, theme::rule()));
            for (k, &ri) in pinned.iter().enumerate() {
                let rect = Rect::from_min_size(
                    Pos2::new(out.inner_rect.left(), top + k as f32 * ROW_H),
                    Vec2::new(out.inner_rect.width(), ROW_H),
                );
                if let Some(mut c) = row_ui(ui, state, &rows[ri], rect, (1 << 40) + ri as u64, &cx) {
                    // A pinned row's click takes you to the folder.
                    if matches!(c.1, Click::Row) {
                        c.1 = Click::RowScroll;
                    }
                    clicked.push(c);
                }
            }
        }

        for (id, click) in clicked {
            match click {
                Click::Arrow => {
                    if let Some(t) = &mut state.tree {
                        toggle_expanded(t, id);
                    }
                    ui.ctx().request_repaint();
                }
                Click::Row | Click::RowScroll => {
                    state.selected_node = Some(id);
                    state.selected_extension = None;
                    if matches!(click, Click::RowScroll) {
                        state.scroll_dir_tree_top = Some(id);
                        ui.ctx().request_repaint();
                    }
                }
                Click::Open => {
                    crate::nav::navigate_to(state, id);
                    ui.ctx().request_repaint();
                }
            }
        }
    });
}

struct Ctx<'a> {
    highlighted_dir: Option<NodeId>,
    /// The hovered map block and its ancestors.
    map_chain: &'a [NodeId],
}

enum Click {
    Arrow,
    Row,
    /// Click on a pinned row: select and scroll the list to it.
    RowScroll,
    Open,
}

/// Paint one ledger row into `rect` and read its clicks. `salt` keeps the
/// widget ids of a pinned copy apart from the row it repeats.
fn row_ui(
    ui: &mut Ui,
    state: &mut AppState,
    row: &LedgerRow,
    rect: Rect,
    salt: u64,
    cx: &Ctx,
) -> Option<(NodeId, Click)> {
    let tree = state.tree.as_ref()?;
    let painter = ui.painter();
    let node = tree.node(row.id);
    let indent = row.depth as f32 * INDENT;

    if let RowKind::Loose { bytes, n } = row.kind {
        let label = if n == 1 { "1 file".to_string() } else { format!("{n} files") };
        painter.text(
            Pos2::new(rect.left() + 3.0 + indent + 14.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::new(12.0, theme::italic()),
            theme::note(),
        );
        size_and_bar(painter, rect, bytes, row.scale, theme::faint());
        return None;
    }

    let is_root = row.depth == 0;
    let expanded = is_root || matches!(&node.kind, NodeKind::Directory { expanded: true, .. });
    let resp = ui.interact(rect, ui.id().with(("row", salt)), Sense::click());
    let arrow = (row.has_kids && !is_root).then(|| arrow_rect(rect, row.depth));
    // Added after the row so it wins the click where they overlap.
    let arrow_resp = arrow.map(|a| ui.interact(a, ui.id().with(("arrow", salt)), Sense::click()));
    if let Some(a) = arrow {
        if salt < (1 << 40) {
            state.ledger_arrows.push((row.id, a));
        }
    }
    let over_arrow = arrow_resp.as_ref().is_some_and(|r| r.hovered());
    if resp.hovered() || over_arrow {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        state.ledger_hover = Some(row.id);
    }

    let tree = state.tree.as_ref()?;
    let painter = ui.painter();
    let is_selected = cx.highlighted_dir == Some(row.id);
    // The map's pointer is inside this folder, and no deeper listed row
    // shows it.
    let map_here = cx.map_chain.contains(&row.id)
        && !(expanded
            && node
                .children()
                .iter()
                .any(|c| cx.map_chain.contains(c) && tree.node(*c).is_dir()));
    let fill = if is_selected {
        theme::bg_selection()
    } else if map_here || resp.hovered() {
        theme::bg_hover()
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, theme::RADIUS_SM, fill);

    if let Some(a) = arrow {
        let glyph = if expanded { "▼" } else { "▶" };
        let color = if over_arrow { theme::ink() } else { theme::note() };
        painter.text(a.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(9.0), color);
    }

    let name_color = if is_selected || map_here { theme::ink() } else { theme::ink_soft() };
    let name_x = rect.left() + 3.0 + indent + 14.0;
    let right = size_and_bar(painter, rect, node.size, row.scale, theme::highlight());
    let mut text_right = right;
    if right - name_x > 90.0 {
        let c = painter.text(
            Pos2::new(right - 4.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format_count(node.items()),
            egui::FontId::proportional(9.5),
            theme::faint(),
        );
        text_right = c.left();
    }
    let name = tree.name(row.id);
    let max_chars = (((text_right - name_x) - 6.0) / 6.5).max(0.0) as usize;
    let shown = truncate_label(&name, max_chars);
    painter.text(
        Pos2::new(name_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        shown,
        egui::FontId::proportional(12.5),
        name_color,
    );

    let id = row.id;
    resp.context_menu(|ui| {
        crate::ui::context_menu::show(ui, state, id);
    });
    if arrow_resp.as_ref().is_some_and(|r| r.clicked()) {
        return Some((id, Click::Arrow));
    }
    if resp.double_clicked() && !is_root {
        return Some((id, Click::Open));
    }
    if resp.clicked() {
        return Some((id, Click::Row));
    }
    None
}

/// Paints size and bar at the right edge of `rect`; returns the x where the
/// next element to the left must stop.
fn size_and_bar(painter: &egui::Painter, rect: Rect, size: u64, scale: u64, color: egui::Color32) -> f32 {
    let mut x = rect.right() - 6.0;
    painter.text(
        Pos2::new(x, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        theme::format_size(size),
        egui::FontId::proportional(10.5),
        theme::ink_soft(),
    );
    x -= 54.0 + 6.0;
    if x - BAR_W > rect.left() + 60.0 {
        let frac = if scale > 0 { (size as f32 / scale as f32).min(1.0) } else { 0.0 };
        let w = (BAR_W * frac).max(if size > 0 { 1.0 } else { 0.0 });
        let bar = Rect::from_min_size(Pos2::new(x - BAR_W, rect.center().y - 2.5), Vec2::new(w, 5.0));
        painter.rect_filled(bar, 0.0, color);
        x -= BAR_W + 6.0;
    }
    x
}

/// Truncate `name` to `max_chars` characters, appending `…` if truncated.
/// Uses char boundaries so multi-byte Unicode names never panic or split.
fn truncate_label(name: &str, max_chars: usize) -> String {
    if name.chars().count() > max_chars && max_chars > 3 {
        let s: String = name.chars().take(max_chars - 1).collect();
        format!("{s}…")
    } else {
        name.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_label_non_ascii_cuts_on_char_boundary() {
        // Founder case 2: a non-ASCII name must truncate on a char boundary
        // without panicking and must produce valid UTF-8.
        let result = truncate_label("Příliš.pdf", 5);
        assert!(result.ends_with('…'), "expected ellipsis: {result}");
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
        // No truncation when name fits.
        assert_eq!(truncate_label("short", 10), "short");
        // No truncation when max_chars <= 3 (guard against tiny boxes).
        assert_eq!(truncate_label("Příliš.pdf", 3), "Příliš.pdf");
    }
}
