//! Move 1, the inventory: what can be rebuilt against what only you have,
//! what each costs to get back, and the closing account.

use mac_dir_stat::app::{App, AppState, PendingAction};
use mac_dir_stat::cleanup::{
    classify, find_candidates, find_only_yours, is_rebuildable, restore_cost, Account,
};
use mac_dir_stat::scanner::tree::FileTree;
use mac_dir_stat::treemap::color::ColorMode;
use mac_dir_stat::ui::cleanup_window::groups;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MB: u64 = 1024 * 1024;

fn home_tree() -> FileTree {
    let t0 = SystemTime::UNIX_EPOCH;
    let mut t = FileTree::new();
    t.rename_root(b"/Users/p");
    let r = t.root();
    let pics = t.add_dir(r, b"Pictures", false, t0, 1);
    let lib = t.add_dir(pics, b"Photos Library.photoslibrary", false, t0, 2);
    t.add_file(lib, b"a.heic", 100 * MB, Some("heic"), t0, 3);
    t.add_file(pics, b"scan.png", 5 * MB, Some("png"), t0, 2);
    let docs = t.add_dir(r, b"Documents", false, t0, 1);
    t.add_file(docs, b"tax.pdf", 20 * MB, Some("pdf"), t0, 2);
    let music = t.add_dir(r, b"Music", false, t0, 1);
    t.add_file(music, b"tiny.mp3", 1000, Some("mp3"), t0, 2);
    let code = t.add_dir(r, b"code", false, t0, 1);
    for (name, age) in [(b"a" as &[u8], 3u64), (b"b", 200)] {
        let p = t.add_dir(code, name, false, t0, 2);
        let nm = t.add_dir(p, b"node_modules", false, t0, 3);
        let when = SystemTime::now() - Duration::from_secs(age * 86_400);
        t.add_file(nm, b"x.js", 30 * MB, Some("js"), when, 4);
    }
    let caches = t.add_dir(r, b"Library", false, t0, 1);
    let c = t.add_dir(caches, b"Caches", false, t0, 2);
    t.add_file(c, b"blob", 8 * MB, None, t0, 3);
    t.compute_sizes();
    t
}

#[test]
fn only_yours_lists_personal_folders_and_splits_out_photos() {
    let t = home_tree();
    let rows = find_only_yours(&t, Some(Path::new("/Users/p")));
    let got: Vec<(&str, u64)> = rows.iter().map(|r| (r.label.as_str(), r.size)).collect();
    assert_eq!(
        got,
        vec![
            ("Photos Library", 100 * MB),
            ("Documents", 20 * MB),
            ("Pictures (the rest)", 5 * MB),
        ],
        "Music is under 1 MB and left out; Photos is not counted twice"
    );
}

#[test]
fn only_yours_needs_a_home_and_never_overlaps_the_rebuildable_side() {
    let t = home_tree();
    assert!(find_only_yours(&t, None).is_empty());
    let mine = find_only_yours(&t, Some(Path::new("/Users/p")));
    let rebuild = find_candidates(&t, t.root());
    for m in &mine {
        assert!(!is_rebuildable(&m.path), "{} offered as rebuildable", m.path.display());
        assert!(rebuild.iter().all(|c| !c.path.starts_with(&m.path) && !m.path.starts_with(&c.path)));
    }
}

#[test]
fn every_rule_says_what_it_takes_to_get_it_back() {
    let paths = [
        "/u/Library/Developer/Xcode/DerivedData",
        "/u/Library/Developer/Xcode/iOS DeviceSupport",
        "/u/Library/Developer/Xcode/watchOS DeviceSupport",
        "/u/Library/Developer/Xcode/Archives",
        "/u/Library/Developer/CoreSimulator/Caches",
        "/u/Library/Developer/CoreSimulator/Devices",
        "/u/Library/Developer/Xcode/UserData/Previews",
        "/u/Library/Caches",
        "/u/Library/Logs",
        "/u/Library/Containers/com.docker.docker",
        "/u/.cargo/registry",
        "/u/.rustup/toolchains",
        "/u/p/node_modules",
        "/u/p/__pycache__",
        "/u/p/.pytest_cache",
        "/u/.gradle",
        "/u/.m2",
        "/u/p/DerivedData",
        "/u/p/.next",
        "/u/p/.turbo",
        "/u/.cache",
    ];
    for p in paths {
        let (cat, _) = classify(Path::new(p)).unwrap_or_else(|| panic!("{p} unclassified"));
        assert_ne!(restore_cost(cat), "the tool that made it", "no restore line for {cat}");
    }
    assert_ne!(restore_cost("Rust target/"), "the tool that made it");
}

#[test]
fn groups_one_row_per_rule_with_the_newest_change() {
    let t = home_tree();
    let c = find_candidates(&t, t.root());
    let g = groups(&c);
    assert_eq!(g[0].category, "node_modules");
    assert_eq!(g[0].items.len(), 2);
    assert_eq!(g[0].size, 60 * MB);
    let days = SystemTime::now().duration_since(g[0].newest).unwrap().as_secs() / 86_400;
    assert_eq!(days, 3, "newest of the two projects");
    assert_eq!(g[1].category, "Application Caches");
}

#[test]
fn account_splits_what_left_into_rebuildable_and_only_yours() {
    let mut a = Account::default();
    a.add(&[
        (PathBuf::from("/u/p/node_modules"), 30),
        (PathBuf::from("/u/p/node_modules/x/y.js"), 5),
        (PathBuf::from("/u/Documents/tax.pdf"), 7),
    ]);
    a.add(&[(PathBuf::from("/u/Library/Caches"), 8)]);
    assert_eq!((a.items, a.moved, a.rebuildable, a.only_yours), (4, 50, 43, 7));
}

/// Journey: after a scan, Pavol opens Cleanup. Nothing is ticked for him.
/// He ticks node_modules and asks to move it to the Trash; the one
/// confirmation lists exactly those folders and none of his own.
#[test]
fn journey_open_inventory_tick_one_rule_confirm() {
    let tree = home_tree();
    let mut s = AppState::new(
        PathBuf::from("/Users/p"),
        false,
        ColorMode::Extension,
        mac_dir_stat::flywheel::disabled().clone(),
    );
    s.request_rescan = false;
    s.home = Some(PathBuf::from("/Users/p"));
    s.cleanup_candidates = find_candidates(&tree, tree.root());
    s.only_yours = find_only_yours(&tree, s.home.as_deref());
    s.view_root = Some(tree.root());
    s.tree = Some(tree);
    s.cleanup_window_open = true;
    let mut app = App::from_state(s);

    let ctx = egui::Context::default();
    mac_dir_stat::ui::theme::apply_theme(&ctx);
    let _ = ctx.run(Default::default(), |_| {});
    let mut frame = eframe::Frame::_new_kittest();
    let _ = ctx.run(Default::default(), |ctx| eframe::App::update(&mut app, ctx, &mut frame));
    assert!(app.state.cleanup_selected.is_empty(), "something was ticked for him");
    assert!(app.state.pending_action.is_none());

    // He ticks the node_modules row (what its checkbox does).
    let ids: Vec<_> = groups(&app.state.cleanup_candidates)
        .iter()
        .find(|g| g.category == "node_modules")
        .unwrap()
        .items
        .iter()
        .map(|c| c.node_id)
        .collect();
    app.state.cleanup_selected.extend(ids.iter().copied());

    // Click "Move to Trash…" in the action bar. Find it by rendering once and
    // clicking where egui laid it out.
    let pos = find_button(&ctx, &mut app, &mut frame, "Move to Trash…");
    click(&ctx, &mut app, &mut frame, pos);
    match &app.state.pending_action {
        Some(PendingAction::ConfirmBatchTrash(items, total)) => {
            let mut got: Vec<_> = items.iter().map(|(id, _)| *id).collect();
            got.sort();
            let mut want = ids.clone();
            want.sort();
            assert_eq!(got, want);
            assert_eq!(*total, 60 * MB);
            assert!(items.iter().all(|(_, p)| is_rebuildable(p)));
        }
        other => panic!("expected one batch confirmation, got {}", other.is_some()),
    }
}

fn find_button(ctx: &egui::Context, app: &mut App, frame: &mut eframe::Frame, label: &str) -> egui::Pos2 {
    let out = ctx.run(Default::default(), |ctx| eframe::App::update(app, ctx, frame));
    let mut found = None;
    for clipped in &out.shapes {
        if let egui::Shape::Text(t) = &clipped.shape {
            if t.galley.text() == label {
                found = Some(t.pos + t.galley.rect.center().to_vec2());
            }
        }
    }
    found.unwrap_or_else(|| panic!("no {label:?} on screen"))
}

fn click(ctx: &egui::Context, app: &mut App, frame: &mut eframe::Frame, pos: egui::Pos2) {
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    for events in [
        vec![egui::Event::PointerMoved(pos)],
        vec![press(true)],
        vec![press(false)],
    ] {
        let input = egui::RawInput { events, ..Default::default() };
        let _ = ctx.run(input, |ctx| eframe::App::update(app, ctx, frame));
    }
}
