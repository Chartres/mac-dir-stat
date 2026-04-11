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

/// Compute the full treemap layout with colors for all files under view_root.
pub fn compute_treemap(
    tree: &FileTree,
    view_root: NodeId,
    bounds: Rect,
    color_mode: ColorMode,
) -> Vec<ColoredRect> {
    let mut result = Vec::new();
    layout_recursive(tree, view_root, &bounds, color_mode, 0, &mut result);
    result
}

fn layout_recursive(
    tree: &FileTree,
    node_id: NodeId,
    bounds: &Rect,
    color_mode: ColorMode,
    hue_index: usize,
    result: &mut Vec<ColoredRect>,
) {
    if bounds.w < 1.0 || bounds.h < 1.0 {
        return;
    }
    let node = tree.node(node_id);
    if !tree.is_alive(node_id) {
        return;
    }

    match &node.kind {
        NodeKind::File { extension } => {
            let (cs, ce) = match color_mode {
                ColorMode::Extension => {
                    extension_color(extension.as_deref().unwrap_or(""))
                }
                ColorMode::Depth => depth_color(hue_index, node.depth),
                ColorMode::Age => age_color(node.modified),
            };
            result.push(ColoredRect {
                node_id,
                rect: *bounds,
                color_start: cs,
                color_end: ce,
            });
        }
        NodeKind::Directory { children, .. } => {
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
            let gap = 1.0;

            for p in &placed {
                let child_id = kids[p.id];
                let mut child_bounds = p.rect;
                child_bounds.x += gap;
                child_bounds.y += gap;
                child_bounds.w -= gap * 2.0;
                child_bounds.h -= gap * 2.0;
                if child_bounds.w < 0.5 || child_bounds.h < 0.5 {
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
                    result,
                );
            }
        }
    }
}
