pub mod squarify;
pub mod color;

use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use color::{age_color, depth_color, extension_color, Color, ColorMode};
use squarify::{layout, LayoutItem, Rect};

#[derive(Debug, Clone)]
pub struct ColoredRect {
    pub node_id: NodeId,
    pub rect: Rect,
    pub color_start: Color,
    pub color_end: Color,
}

/// Bounding rectangle for a directory in the treemap.
#[derive(Debug, Clone)]
pub struct DirRect {
    pub node_id: NodeId,
    pub rect: Rect,
    pub depth: u16,
}

pub struct TreemapLayout {
    pub file_rects: Vec<ColoredRect>,
    pub dir_rects: Vec<DirRect>,
}

/// Compute the full treemap layout with colors for all files under view_root.
pub fn compute_treemap(
    tree: &FileTree,
    view_root: NodeId,
    bounds: Rect,
    color_mode: ColorMode,
) -> TreemapLayout {
    let mut file_rects = Vec::new();
    let mut dir_rects = Vec::new();
    layout_recursive(tree, view_root, &bounds, color_mode, 0, &mut file_rects, &mut dir_rects);
    TreemapLayout { file_rects, dir_rects }
}

fn layout_recursive(
    tree: &FileTree,
    node_id: NodeId,
    bounds: &Rect,
    color_mode: ColorMode,
    hue_index: usize,
    file_rects: &mut Vec<ColoredRect>,
    dir_rects: &mut Vec<DirRect>,
) {
    if bounds.w <= 0.0 || bounds.h <= 0.0 {
        return;
    }
    let node = tree.node(node_id);
    if !tree.is_alive(node_id) {
        return;
    }

    match &node.kind {
        NodeKind::File { .. } => {
            let (cs, ce) = match color_mode {
                ColorMode::Extension => {
                    extension_color(tree.extension(node_id).unwrap_or(""))
                }
                ColorMode::Depth => depth_color(hue_index, node.depth),
                ColorMode::Age => age_color(node.modified),
            };
            file_rects.push(ColoredRect {
                node_id,
                rect: *bounds,
                color_start: cs,
                color_end: ce,
            });
        }
        NodeKind::Directory { children, .. } => {
            // Store this directory's bounding rect
            dir_rects.push(DirRect {
                node_id,
                rect: *bounds,
                depth: node.depth,
            });

            let mut kids: Vec<NodeId> = children
                .iter()
                .copied()
                .filter(|&c| tree.is_alive(c) && tree.node(c).size > 0)
                .collect();
            kids.sort_by(|&a, &b| tree.node(b).size.cmp(&tree.node(a).size));
            if kids.is_empty() {
                return;
            }

            let items: Vec<LayoutItem> = kids
                .iter()
                .enumerate()
                .map(|(i, &c)| LayoutItem {
                    id: i,
                    size: tree.node(c).size as f64,
                })
                .collect();
            let placed = layout(&items, bounds);
            let depth = node.depth;
            let gap = if depth <= 1 { 1.5 } else if depth <= 3 { 0.5 } else { 0.0 };

            for p in &placed {
                let child_id = kids[p.id];
                let mut child_bounds = p.rect;
                if gap > 0.0 {
                    child_bounds.x += gap;
                    child_bounds.y += gap;
                    child_bounds.w -= gap * 2.0;
                    child_bounds.h -= gap * 2.0;
                }
                if child_bounds.w <= 0.0 || child_bounds.h <= 0.0 {
                    continue;
                }
                let child_hue = if tree.node(child_id).depth <= 1 {
                    p.id
                } else {
                    hue_index
                };
                layout_recursive(
                    tree,
                    child_id,
                    &child_bounds,
                    color_mode,
                    child_hue,
                    file_rects,
                    dir_rects,
                );
            }
        }
    }
}
