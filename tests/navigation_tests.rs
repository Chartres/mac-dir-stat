//! Folder list clicks, going up the hierarchy, and the pinned parent rows.

use egui::{Event, PointerButton, Pos2};
use mac_dir_stat::app::{App, AppState, DirSortMode};
use mac_dir_stat::nav;
use mac_dir_stat::scanner::tree::{FileTree, NodeId, NodeKind};
use mac_dir_stat::treemap::color::ColorMode;
use mac_dir_stat::ui::dir_tree::{flatten, sticky_chain, toggle_expanded, RowKind, MAX_STICKY};
use std::path::PathBuf;
use std::time::SystemTime;

const MB: u64 = 1024 * 1024;

/// /r/a/b/c/d/e, each level holding a file, plus a sibling /r/z.
fn deep() -> (FileTree, Vec<NodeId>) {
    let t0 = SystemTime::UNIX_EPOCH;
    let mut t = FileTree::new();
    t.rename_root(b"/r");
    let mut ids = vec![t.root()];
    for (i, n) in ["a", "b", "c", "d", "e"].iter().enumerate() {
        let d = t.add_dir(*ids.last().unwrap(), n.as_bytes(), false, t0, i as u16 + 1);
        t.add_file(d, b"f.bin", 10 * MB, Some("bin"), t0, i as u16 + 2);
        ids.push(d);
    }
    let z = t.add_dir(t.root(), b"z", false, t0, 1);
    t.add_file(z, b"g.bin", MB, Some("bin"), t0, 2);
    t.compute_sizes();
    (t, ids)
}

fn state(t: FileTree) -> AppState {
    let mut s = AppState::new(
        PathBuf::from("/r"),
        false,
        ColorMode::Extension,
        mac_dir_stat::flywheel::disabled().clone(),
    );
    s.request_rescan = false;
    s.view_root = Some(t.root());
    s.zoom_stack = vec![t.root()];
    s.tree = Some(t);
    s
}

fn open_all(s: &mut AppState, dirs: &[NodeId]) {
    let t = s.tree.as_mut().unwrap();
    for &id in dirs {
        if let NodeKind::Directory { expanded, .. } = &mut t.node_mut(id).kind {
            *expanded = true;
        }
    }
}

#[test]
fn up_goes_to_the_parent_and_back_returns() {
    let (t, ids) = deep();
    let mut s = state(t);
    nav::navigate_to(&mut s, ids[3]); // /r/a/b/c
    assert_eq!(s.view_root, Some(ids[3]));
    assert_eq!(s.zoom_stack, vec![ids[0], ids[1], ids[2], ids[3]], "breadcrumb is the whole path");
    assert!(nav::go_up(&mut s));
    assert_eq!(s.view_root, Some(ids[2]));
    assert!(nav::go_back(&mut s));
    assert_eq!(s.view_root, Some(ids[3]), "back to where Up was pressed");
}

#[test]
fn back_returns_to_the_folder_left_and_skips_dead_ones() {
    let (t, ids) = deep();
    let mut s = state(t);
    nav::navigate_to(&mut s, ids[2]);
    nav::navigate_to(&mut s, ids[4]);
    assert_eq!(s.nav_back, vec![ids[0], ids[2]]);
    s.tree.as_mut().unwrap().remove_node(ids[2]);
    assert!(nav::go_back(&mut s), "falls through the dead folder to the root");
    assert_eq!(s.view_root, Some(ids[0]));
    assert!(!nav::go_back(&mut s), "nothing left");
}

#[test]
fn up_at_the_scan_root_does_nothing() {
    let (t, ids) = deep();
    let mut s = state(t);
    assert!(!nav::go_up(&mut s));
    assert_eq!(s.view_root, Some(ids[0]));
    assert!(s.nav_back.is_empty());
}

#[test]
fn flatten_lists_open_folders_depth_first_with_loose_files_last() {
    let (t, ids) = deep();
    let mut s = state(t);
    let t = s.tree.as_mut().unwrap();
    toggle_expanded(t, ids[1]);
    let rows = flatten(t, ids[0], DirSortMode::Size);
    let shape: Vec<(NodeId, u16, bool)> =
        rows.iter().map(|r| (r.id, r.depth, matches!(r.kind, RowKind::Loose { .. }))).collect();
    // r, a (open), b, a's loose file, z, (r has no loose files)
    assert_eq!(shape[0], (ids[0], 0, false));
    assert_eq!(shape[1], (ids[1], 1, false));
    assert_eq!(shape[2], (ids[2], 2, false));
    assert_eq!(shape[3], (ids[1], 2, true));
    let z = t.find_path(std::path::Path::new("/r/z")).unwrap();
    assert!(shape.contains(&(z, 1, false)), "z listed at level 1");
    assert_eq!(toggle_expanded(t, ids[1]), Some(false));
    assert_eq!(flatten(t, ids[0], DirSortMode::Size).len(), 3, "r, a, z");
}

#[test]
fn pinned_rows_are_the_ancestors_of_the_first_visible_row() {
    let (t, ids) = deep();
    let mut s = state(t);
    open_all(&mut s, &ids);
    let rows = flatten(s.tree.as_ref().unwrap(), ids[0], DirSortMode::Size);
    let at = |id: NodeId| rows.iter().position(|r| r.id == id && r.kind == RowKind::Dir).unwrap();
    assert!(sticky_chain(&rows, 0).is_empty(), "scrolled to the top: nothing pinned");
    // Top edge is "d": r, a, b, c are above it; c's chain is r > a > b > c.
    let chain = sticky_chain(&rows, at(ids[4]));
    let pinned: Vec<NodeId> = chain.iter().map(|&i| rows[i].id).collect();
    assert!(pinned.len() <= MAX_STICKY);
    assert_eq!(pinned.last(), Some(&ids[3]), "the nearest parent is always pinned");
    assert!(pinned.windows(2).all(|w| w[0] < w[1]), "outermost first");
    assert!(!pinned.contains(&ids[4]), "a row is not its own parent");
}

fn frame(a: &mut App, ctx: &egui::Context, events: Vec<Event>) {
    let mut f = eframe::Frame::_new_kittest();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| eframe::App::update(a, ctx, &mut f));
}

fn click(a: &mut App, ctx: &egui::Context, p: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(p)]);
    let btn = |pressed| Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    frame(a, ctx, vec![btn(true)]);
    frame(a, ctx, vec![btn(false)]);
    frame(a, ctx, vec![]);
}

fn is_open(a: &App, id: NodeId) -> bool {
    matches!(a.state.tree.as_ref().unwrap().node(id).kind, NodeKind::Directory { expanded: true, .. })
}

/// The bug: the row's own click area sat on top of the triangle, so a click
/// on the triangle only selected the row and the folder never opened.
#[test]
fn journey_clicking_the_triangle_opens_and_closes_the_folder() {
    let (t, ids) = deep();
    let mut a = App::from_state(state(t));
    let ctx = egui::Context::default();
    mac_dir_stat::ui::theme::apply_theme(&ctx);
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    let arrow = |a: &App, id: NodeId| a.state.ledger_arrows.iter().find(|x| x.0 == id).map(|x| x.1.center());
    let p = arrow(&a, ids[1]).expect("row a has a triangle");
    assert!(!is_open(&a, ids[1]));

    click(&mut a, &ctx, p);
    assert!(is_open(&a, ids[1]), "triangle opens the folder");
    assert_ne!(a.state.selected_node, Some(ids[1]), "and does not just select it");
    click(&mut a, &ctx, p);
    assert!(!is_open(&a, ids[1]), "second click closes it");

    // The name is not the triangle: it selects, and leaves the folder shut.
    let p_name = Pos2::new(p.x + 60.0, p.y);
    click(&mut a, &ctx, p_name);
    assert_eq!(a.state.selected_node, Some(ids[1]));
    assert!(!is_open(&a, ids[1]));
}
