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
    let base = tree.node(view_root).depth;
    layout_recursive(tree, view_root, base, &bounds, color_mode, 0, &mut file_rects, &mut dir_rects);
    TreemapLayout { file_rects, dir_rects }
}

/// Directories smaller than this many square points are drawn as one block.
pub const TINY_AREA: f64 = 4.0;

/// `base` is the depth of the folder the map shows: levels count from it.
fn block_color(tree: &FileTree, id: NodeId, mode: ColorMode, hue_index: usize, base: u16) -> color::GradientPair {
    let node = tree.node(id);
    match mode {
        ColorMode::Extension => extension_color(tree.extension(id).unwrap_or("")),
        ColorMode::Depth => depth_color(hue_index, node.depth.saturating_sub(base)),
        ColorMode::Age => age_color(node.modified),
    }
}

/// The largest file under `id` (found by following the largest child), or
/// `id` itself when it holds none.
fn biggest_leaf(tree: &FileTree, id: NodeId) -> NodeId {
    let mut leaf = id;
    while let Some(&big) = tree
        .node(leaf)
        .children()
        .iter()
        .filter(|&&c| tree.is_alive(c) && tree.node(c).size > 0)
        .max_by_key(|&&c| tree.node(c).size)
    {
        leaf = big;
        if !tree.node(leaf).is_dir() {
            break;
        }
    }
    leaf
}

/// Blocks below this many square points are merged into one.
pub const TINY_BLOCK: f64 = 1.0;
/// Fewer sub-pixel blocks than this are drawn as they are.
const MIN_MERGE: usize = 8;

fn layout_recursive(
    tree: &FileTree,
    node_id: NodeId,
    base: u16,
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
            let (cs, ce) = block_color(tree, node_id, color_mode, hue_index, base);
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

            // Too small to show its contents: one block in the colour of its
            // largest file. A disk has hundreds of thousands of files and a
            // screen a few hundred thousand pixels; sub-pixel blocks only
            // cost layout, mesh and hover time.
            if bounds.w * bounds.h < TINY_AREA {
                let leaf = biggest_leaf(tree, node_id);
                let (cs, ce) = block_color(tree, leaf, color_mode, hue_index, base);
                file_rects.push(ColoredRect { node_id: leaf, rect: *bounds, color_start: cs, color_end: ce });
                return;
            }

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

            // Sub-pixel blocks (a folder with thousands of small files) are
            // merged into one block under the rest, in the colour of the
            // largest of them.
            let tiny = |r: &Rect| r.w * r.h < TINY_BLOCK;
            let n_tiny = placed.iter().filter(|p| tiny(&p.rect)).count();
            let merge = n_tiny >= MIN_MERGE;
            if merge {
                let tail: Vec<_> = placed.iter().filter(|p| tiny(&p.rect)).collect();
                let x0 = tail.iter().map(|p| p.rect.x).fold(f64::MAX, f64::min);
                let y0 = tail.iter().map(|p| p.rect.y).fold(f64::MAX, f64::min);
                let x1 = tail.iter().map(|p| p.rect.x + p.rect.w).fold(f64::MIN, f64::max);
                let y1 = tail.iter().map(|p| p.rect.y + p.rect.h).fold(f64::MIN, f64::max);
                let big = tail.iter().map(|p| p.id).min().unwrap();
                let leaf = biggest_leaf(tree, kids[big]);
                let (cs, ce) = block_color(tree, leaf, color_mode, hue_index, base);
                file_rects.push(ColoredRect {
                    node_id: leaf,
                    rect: Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 },
                    color_start: cs,
                    color_end: ce,
                });
            }

            for p in &placed {
                if merge && tiny(&p.rect) {
                    continue;
                }
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
                let child_hue = if tree.node(child_id).depth <= base + 1 {
                    p.id
                } else {
                    hue_index
                };
                layout_recursive(
                    tree,
                    child_id,
                    base,
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
