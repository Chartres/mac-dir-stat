//! Measures scan time and per-frame UI cost on a real folder.
//!     cargo run --release --example bench -- ~/Library
use egui::{Pos2, ThemePreference};
use mac_dir_stat::app::{App, AppState};
use mac_dir_stat::scanner::{tree::{NodeId, NodeKind}, ScanProgress};
use mac_dir_stat::treemap::color::ColorMode;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("path"));
    let (tx, rx) = crossbeam_channel::unbounded();
    let t0 = Instant::now();
    mac_dir_stat::scanner::scan(root.clone(), tx);
    let tree = loop {
        match rx.recv().unwrap() {
            ScanProgress::Done(t) => break t,
            ScanProgress::Error(e) => panic!("{e}"),
            _ => {}
        }
    };
    let scan = t0.elapsed();
    let r = tree.root();
    println!("scan: {:.2}s  files={} dirs={}", scan.as_secs_f64(), tree.node(r).file_count, tree.node(r).subdir_count);

    let mut s = AppState::new(root, true, ColorMode::Extension, mac_dir_stat::flywheel::disabled().clone());
    s.request_rescan = false;
    s.extension_stats = tree.collect_extensions(r);
    s.view_root = Some(r);
    s.zoom_stack = vec![r];
    // expand every directory: worst case for the list
    s.tree = Some(tree);
    let n_expand = expand_all(&mut s, 3);
    println!("expanded dirs to depth 3: {n_expand}");
    {
        let t = s.tree.as_ref().unwrap();
        let r = t.root();
        let t0 = Instant::now();
        let rows = mac_dir_stat::ui::dir_tree::flatten(t, r, s.dir_sort);
        println!("flatten: {:.2}ms ({} rows)", t0.elapsed().as_secs_f64() * 1000.0, rows.len());
        let b = mac_dir_stat::treemap::squarify::Rect { x: 0.0, y: 0.0, w: 700.0, h: 700.0 };
        let t0 = Instant::now();
        let l = mac_dir_stat::treemap::compute_treemap(t, r, b, ColorMode::Extension);
        println!("treemap layout: {:.2}ms ({} blocks)", t0.elapsed().as_secs_f64() * 1000.0, l.file_rects.len());
    }
    let mut app = App::from_state(s);
    let ctx = egui::Context::default();
    ctx.set_theme(ThemePreference::Light);
    mac_dir_stat::ui::theme::apply_theme(&ctx);
    let _ = ctx.run(Default::default(), |_| {});
    let mut frame = eframe::Frame::_new_kittest();
    let mut run = |label: &str, n: usize, ptr: Option<Pos2>, app: &mut App| {
        let mut times = vec![];
        for i in 0..n {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
                time: Some(i as f64 * 0.016),
                ..Default::default()
            };
            if let Some(p) = ptr {
                input.events.push(egui::Event::PointerMoved(Pos2::new(p.x + (i % 7) as f32, p.y)));
            }
            let t = Instant::now();
            let out = ctx.run(input, |c| eframe::App::update(app, c, &mut frame));
            let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let first = times[0];
        let mut rest: Vec<f64> = times[1..].to_vec();
        rest.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("{label}: first {first:.1}ms  median {:.2}ms  p95 {:.2}ms", rest[rest.len() / 2], rest[rest.len() * 95 / 100]);
    };
    run("idle frame", 60, None, &mut app);
    run("hover list", 60, Some(Pos2::new(120.0, 200.0)), &mut app);
    run("hover map", 60, Some(Pos2::new(700.0, 400.0)), &mut app);
}

fn expand_all(s: &mut AppState, max_depth: u16) -> usize {
    let t = s.tree.as_mut().unwrap();
    let mut n = 0;
    let mut stack = vec![t.root()];
    while let Some(id) = stack.pop() {
        if t.node(id).depth > max_depth { continue; }
        let kids: Vec<NodeId> = t.node(id).children().to_vec();
        if let NodeKind::Directory { expanded, .. } = &mut t.node_mut(id).kind { *expanded = true; n += 1; }
        stack.extend(kids.into_iter().filter(|&k| t.node(k).is_dir()));
    }
    n
}
