//! Move 2A, the ledger in the folder list: rows ranked by size, a length bar
//! scaled to the largest row of the level, and hover synced with the map.

use mac_dir_stat::app::{App, AppState, DirSortMode};
use mac_dir_stat::scanner::tree::{FileTree, NodeId};
use mac_dir_stat::treemap::color::ColorMode;
use mac_dir_stat::ui::dir_tree::{level_max, list_root, loose_files, visible_dirs};
use std::path::PathBuf;
use std::time::SystemTime;

const MB: u64 = 1024 * 1024;

/// /r: big (60 MB in a/), small (10 MB), and 25 MB of loose files.
fn tree() -> (FileTree, NodeId, NodeId) {
    let t0 = SystemTime::UNIX_EPOCH;
    let mut t = FileTree::new();
    t.rename_root(b"/r");
    let r = t.root();
    let big = t.add_dir(r, b"big", false, t0, 1);
    let a = t.add_dir(big, b"a", false, t0, 2);
    t.add_file(a, b"x.mov", 60 * MB, Some("mov"), t0, 3);
    let small = t.add_dir(r, b"small", false, t0, 1);
    t.add_file(small, b"y.pdf", 10 * MB, Some("pdf"), t0, 2);
    t.add_file(r, b"one.zip", 15 * MB, Some("zip"), t0, 1);
    t.add_file(r, b"two.zip", 10 * MB, Some("zip"), t0, 1);
    t.compute_sizes();
    (t, big, small)
}

fn app(t: FileTree) -> App {
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
    App::from_state(s)
}

#[test]
fn a_level_scales_to_its_largest_row_including_loose_files() {
    let (t, big, small) = tree();
    assert_eq!(loose_files(&t, t.root()), (25 * MB, 2));
    assert_eq!(level_max(&t, t.root()), 60 * MB);
    assert_eq!(level_max(&t, small), 10 * MB, "a folder of only files scales to them");
    assert_eq!(level_max(&t, big), 60 * MB);
}

#[test]
fn the_list_follows_the_map_into_a_zoomed_folder() {
    let (t, big, _) = tree();
    let mut a = app(t);
    assert_eq!(list_root(&a.state), a.state.tree.as_ref().map(|t| t.root()));
    a.state.view_root = Some(big);
    assert_eq!(list_root(&a.state), Some(big));
    // The list's root row is always open, even if collapsed in the tree.
    let tree = a.state.tree.as_ref().unwrap();
    let rows = visible_dirs(tree, big, DirSortMode::Size);
    assert_eq!(rows.len(), 2, "big and its child a");
    assert_eq!(rows[0], big);
}

#[test]
fn a_dead_view_root_falls_back_to_the_tree_root() {
    let (t, big, _) = tree();
    let mut a = app(t);
    a.state.view_root = Some(big);
    a.state.tree.as_mut().unwrap().remove_node(big);
    assert_eq!(list_root(&a.state), Some(0));
}

/// Journey: Pavol moves the pointer down the folder list. Each row he
/// passes is the one the map outlines; off the list, nothing is outlined.
#[test]
fn journey_hovering_a_row_marks_that_folder_for_the_map() {
    let (t, big, small) = tree();
    let mut a = app(t);
    let ctx = egui::Context::default();
    mac_dir_stat::ui::theme::apply_theme(&ctx);
    let mut frame = eframe::Frame::_new_kittest();
    let screen = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0)));
    let mut run = |a: &mut App, events: Vec<egui::Event>| {
        let input = egui::RawInput { screen_rect: screen, events, ..Default::default() };
        ctx.run(input, |ctx| eframe::App::update(a, ctx, &mut frame))
    };
    let out = run(&mut a, vec![]);
    let pos_of = |name: &str| {
        out.shapes
            .iter()
            .find_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.text() == name => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no row {name:?}"))
    };
    let (p_big, p_small) = (pos_of("big"), pos_of("small"));
    // Rows are ranked: big above small, and the loose files are listed.
    assert!(p_big.y < p_small.y);
    let _ = pos_of("2 files");

    run(&mut a, vec![egui::Event::PointerMoved(p_big)]);
    run(&mut a, vec![egui::Event::PointerMoved(p_big)]);
    assert_eq!(a.state.ledger_hover, Some(big));
    run(&mut a, vec![egui::Event::PointerMoved(p_small)]);
    assert_eq!(a.state.ledger_hover, Some(small));
    run(&mut a, vec![egui::Event::PointerMoved(egui::pos2(640.0, 400.0))]);
    assert_eq!(a.state.ledger_hover, None);
}
