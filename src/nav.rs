//! Moving the map's root up and down the hierarchy: breadcrumb, Up, Back.

use crate::app::AppState;
use crate::scanner::tree::{FileTree, NodeId};

/// Scan root down to `node`, inclusive.
pub fn chain(tree: &FileTree, node: NodeId) -> Vec<NodeId> {
    let mut c: Vec<NodeId> = std::iter::successors(Some(node), |&n| tree.node(n).parent).collect();
    c.reverse();
    c
}

fn live_dir(state: &AppState, id: NodeId) -> bool {
    state.tree.as_ref().is_some_and(|t| t.is_alive(id) && t.node(id).is_dir())
}

/// Show `node` (a folder) as the map's root. The folder left is remembered
/// for Back. `zoom_stack` mirrors the breadcrumb so Esc steps up one level.
pub fn navigate_to(state: &mut AppState, node: NodeId) {
    if !live_dir(state, node) || state.view_root == Some(node) {
        return;
    }
    if let Some(prev) = state.view_root {
        state.nav_back.push(prev);
    }
    set_root(state, node);
}

fn set_root(state: &mut AppState, node: NodeId) {
    let Some(tree) = &state.tree else { return };
    state.zoom_stack = chain(tree, node);
    state.view_root = Some(node);
    state.treemap_dirty = true;
}

/// The folder above the current root, if any.
pub fn parent_of_root(state: &AppState) -> Option<NodeId> {
    let tree = state.tree.as_ref()?;
    let vr = state.view_root?;
    tree.node(vr).parent.filter(|&p| tree.is_alive(p))
}

pub fn go_up(state: &mut AppState) -> bool {
    match parent_of_root(state) {
        Some(p) => {
            navigate_to(state, p);
            true
        }
        None => false,
    }
}

/// Return to the folder shown before the last jump; skips folders that no
/// longer exist.
pub fn go_back(state: &mut AppState) -> bool {
    while let Some(prev) = state.nav_back.pop() {
        if live_dir(state, prev) {
            set_root(state, prev);
            return true;
        }
    }
    false
}
