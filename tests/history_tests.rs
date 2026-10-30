//! Move 3B, small multiples of past scans: what a summary keeps, how many
//! are kept, and the History window.

use mac_dir_stat::app::{App, AppState};
use mac_dir_stat::history::{
    for_root, from_json, layout_order, record, summarize, to_json, ScanSummary, FILES, HIDDEN,
    KEEP, OTHER, ROOTS, TOP,
};
use mac_dir_stat::scanner::tree::FileTree;
use mac_dir_stat::treemap::color::ColorMode;
use mac_dir_stat::ui::history_window::ymd;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MB: u64 = 1024 * 1024;

fn tree() -> FileTree {
    let t0 = UNIX_EPOCH;
    let mut t = FileTree::new();
    t.rename_root(b"/");
    let r = t.root();
    for i in 0..10u64 {
        let d = t.add_dir(r, format!("d{i}").as_bytes(), false, t0, 1);
        let deep = t.add_dir(d, b"secret-project", false, t0, 2);
        t.add_file(deep, b"diary.txt", (10 - i) * MB, Some("txt"), t0, 3);
    }
    t.add_file(r, b"<Free Space>", 500 * MB, Some("__free_space__"), t0, 1);
    t.add_file(r, b"<Hidden>", 30 * MB, Some("__skipped__"), t0, 1);
    t.add_file(r, b"divorce-settlement.pdf", 7 * MB, Some("pdf"), t0, 1);
    let other = t.add_dir(r, b"other", false, t0, 1);
    t.add_file(other, b"o.bin", 20 * MB, None, t0, 2);
    t.compute_sizes();
    t
}

fn at(days: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_790_000_000 + days * 86_400)
}

fn summary(root: &str, day: u64) -> ScanSummary {
    ScanSummary { root: PathBuf::from(root), when: at(day).duration_since(UNIX_EPOCH).unwrap().as_secs(), used: day, top: vec![] }
}

#[test]
fn a_summary_is_totals_and_top_folders_only() {
    let s = summarize(&tree(), Path::new("/"), at(0));
    // Free space is not used space; hidden and loose files count, unnamed.
    assert_eq!(s.used, (55 + 30 + 7 + 20) * MB);
    assert_eq!(s.top.iter().map(|t| t.1).sum::<u64>(), s.used);
    assert_eq!(s.top[0], ("other".to_string(), 20 * MB), "a real folder named other");
    assert_eq!(s.top.iter().filter(|t| !t.0.starts_with('/')).count(), TOP);
    let bucket = |k: &str| s.top.iter().find(|t| t.0 == k).map(|t| t.1);
    assert_eq!(bucket(FILES), Some(7 * MB));
    assert_eq!(bucket(HIDDEN), Some(30 * MB));
    assert_eq!(bucket(OTHER), Some(6 * MB), "d7 to d9");
    // Nothing below the top level, and no file names, are written down.
    let json = to_json(&[s]);
    for secret in ["secret-project", "diary", "divorce"] {
        assert!(!json.contains(secret), "{secret} in {json}");
    }
}

#[test]
fn a_new_scan_is_kept_even_if_the_clock_went_back() {
    let mut all = Vec::new();
    for day in 10..16 {
        record(&mut all, summary("/a", day));
    }
    let mut late = summary("/a", 0);
    late.used = 999;
    record(&mut all, late);
    let a = for_root(&all, Path::new("/a"));
    assert_eq!(a.len(), KEEP);
    assert_eq!(a.last().unwrap().used, 999);
}

#[test]
fn six_scans_are_kept_per_folder_and_twenty_folders_in_all() {
    let mut all = Vec::new();
    for day in 0..10 {
        record(&mut all, summary("/a", day));
    }
    record(&mut all, summary("/b", 3));
    let a = for_root(&all, Path::new("/a"));
    assert_eq!(a.len(), KEEP);
    assert_eq!(a.first().unwrap().used, 4, "oldest kept is the 5th scan");
    assert_eq!(a.last().unwrap().used, 9);
    assert_eq!(for_root(&all, Path::new("/b")).len(), 1);

    for i in 0..ROOTS + 5 {
        record(&mut all, summary(&format!("/r{i}"), 20 + i as u64));
    }
    let mut roots: Vec<_> = all.iter().map(|s| s.root.clone()).collect();
    roots.dedup();
    roots.sort();
    roots.dedup();
    assert_eq!(roots.len(), ROOTS);
    assert!(all.iter().all(|s| s.root != Path::new("/a")), "least recent folder dropped");
}

#[test]
fn every_panel_shares_the_newest_scans_order() {
    let mut old = summary("/a", 0);
    old.top = vec![("Movies".into(), 9), ("Code".into(), 5), (OTHER.into(), 1)];
    let mut new = summary("/a", 1);
    new.top = vec![("Code".into(), 20), ("Mail".into(), 8), (OTHER.into(), 1)];
    assert_eq!(
        layout_order(&[&old, &new]),
        vec!["Code", "Mail", "Movies", FILES, HIDDEN, OTHER]
    );
}

#[test]
fn history_round_trips_and_bad_files_are_ignored() {
    let mut s = summarize(&tree(), Path::new("/"), at(5));
    s.top.truncate(2);
    let back = from_json(&to_json(&[s.clone()]));
    assert_eq!(back, vec![s]);
    assert!(from_json("not json").is_empty());
    assert!(from_json(r#"{"scans":[{"root":1}]}"#).is_empty());
}

#[test]
fn dates_are_civil_dates() {
    assert_eq!(ymd(0), (1970, 1, 1));
    assert_eq!(ymd(1_793_318_400), (2026, 10, 30));
    assert_eq!(ymd(951_782_400), (2000, 2, 29));
}

/// Journey: Pavol has scanned his home folder three times. He opens
/// History and sees three panels, one per scan, dated, oldest first.
#[test]
fn journey_open_history_sees_one_panel_per_scan() {
    let mut s = AppState::new(
        PathBuf::from("/Users/p"),
        false,
        ColorMode::Extension,
        mac_dir_stat::flywheel::disabled().clone(),
    );
    s.request_rescan = false;
    for (i, day) in [0u64, 31, 61].iter().enumerate() {
        let mut x = summary("/Users/p", *day);
        x.used = (i as u64 + 1) * 100 * MB;
        x.top = vec![("Documents".into(), x.used)];
        record(&mut s.history, x);
    }
    record(&mut s.history, summary("/elsewhere", 70));
    s.history_window_open = true;
    let mut app = App::from_state(s);
    let ctx = egui::Context::default();
    mac_dir_stat::ui::theme::apply_theme(&ctx);
    let _ = ctx.run(Default::default(), |_| {});
    let mut frame = eframe::Frame::_new_kittest();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let out = ctx.run(Default::default(), |ctx| eframe::App::update(&mut app, ctx, &mut frame));
        texts = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => Some((t.pos.x, t.galley.text().to_string())),
                _ => None,
            })
            .collect();
    }
    let has = |s: &str| texts.iter().any(|t| t.1 == s);
    assert!(has("The same folder, 3 scans"), "{texts:?}");
    let dates: Vec<_> = texts
        .iter()
        .filter(|t| ["Sep", "Oct", "Nov", "Dec", "Jan", "Aug"].iter().any(|m| t.1.ends_with(m)))
        .collect();
    assert_eq!(dates.len(), 3, "{texts:?}");
    assert!(dates.windows(2).all(|w| w[0].0 < w[1].0), "oldest first, left to right");
    assert!(has("300.0 MB"));
}
