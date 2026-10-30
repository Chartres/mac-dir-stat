//! Past scans, for the small multiples (docs/SIGNATURE.md Move 3B).
//!
//! Each finished scan leaves a small summary: when, which folder, bytes
//! used, and the sizes of its largest top-level folders. No file lists and
//! no paths below the top level. Summaries live in
//! `~/Library/Application Support/mac-dir-stat/history.json` on this Mac and
//! are never sent anywhere. At most [`KEEP`] are kept per scanned folder.

use crate::scanner::tree::FileTree;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Summaries kept, and shown, per scanned folder.
pub const KEEP: usize = 6;
/// Top-level folders named in a summary; the rest are summed as "other".
pub const TOP: usize = 8;
/// Scanned folders remembered; the least recently scanned go first.
pub const ROOTS: usize = 20;
/// Name used for everything outside the top folders.
pub const OTHER: &str = "other";

#[derive(Debug, Clone, PartialEq)]
pub struct ScanSummary {
    pub root: PathBuf,
    /// Seconds since 1970.
    pub when: u64,
    /// Bytes found by the scan, not counting free space.
    pub used: u64,
    /// Largest top-level entries, largest first, then "other" if any.
    pub top: Vec<(String, u64)>,
}

/// Summarise a finished scan of `root`.
pub fn summarize(tree: &FileTree, root: &Path, when: SystemTime) -> ScanSummary {
    let r = tree.root();
    let mut kids: Vec<(String, u64)> = tree
        .node(r)
        .children()
        .iter()
        .filter(|&&c| tree.is_alive(c))
        .filter(|&&c| tree.extension(c) != Some("__free_space__"))
        .map(|&c| {
            let name = if tree.extension(c) == Some("__skipped__") {
                "hidden".to_string()
            } else {
                tree.name(c).into_owned()
            };
            (name, tree.node(c).size)
        })
        .collect();
    kids.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let used: u64 = kids.iter().map(|k| k.1).sum();
    let rest: u64 = kids.iter().skip(TOP).map(|k| k.1).sum();
    kids.truncate(TOP);
    if rest > 0 {
        kids.push((OTHER.to_string(), rest));
    }
    ScanSummary {
        root: root.to_path_buf(),
        when: when.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        used,
        top: kids,
    }
}

/// Add `s`, keeping the newest [`KEEP`] per folder and [`ROOTS`] folders.
pub fn record(all: &mut Vec<ScanSummary>, s: ScanSummary) {
    all.push(s);
    all.sort_by_key(|s| s.when);
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut keep = vec![false; all.len()];
    // Walk newest first so the counts favour recent scans.
    for i in (0..all.len()).rev() {
        let root = &all[i].root;
        let n = all[i + 1..]
            .iter()
            .zip(&keep[i + 1..])
            .filter(|(o, k)| **k && &o.root == root)
            .count();
        if !seen.contains(root) {
            if seen.len() >= ROOTS {
                continue;
            }
            seen.push(root.clone());
        }
        keep[i] = n < KEEP;
    }
    let mut k = keep.into_iter();
    all.retain(|_| k.next().unwrap_or(false));
}

/// Summaries of `root`, oldest first, at most [`KEEP`].
pub fn for_root<'a>(all: &'a [ScanSummary], root: &Path) -> Vec<&'a ScanSummary> {
    let mut v: Vec<&ScanSummary> = all.iter().filter(|s| s.root == root).collect();
    v.sort_by_key(|s| s.when);
    let skip = v.len().saturating_sub(KEEP);
    v.split_off(skip)
}

/// One fixed order for every panel: the newest scan's ranking, then names
/// only older scans had, then "other". A block keeps its place across
/// panels, so growth reads at a glance.
pub fn layout_order(scans: &[&ScanSummary]) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for s in scans.iter().rev() {
        for (name, _) in &s.top {
            if name != OTHER && !order.contains(name) {
                order.push(name.clone());
            }
        }
    }
    order.push(OTHER.to_string());
    order
}

pub fn to_json(all: &[ScanSummary]) -> String {
    let v: Vec<Value> = all
        .iter()
        .map(|s| {
            json!({
                "root": s.root.to_string_lossy(),
                "when": s.when,
                "used": s.used,
                "top": s.top.iter().map(|(n, b)| json!([n, b])).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "version": 1, "scans": v }).to_string()
}

/// Parse what [`to_json`] wrote. Anything unreadable is skipped, never fatal.
pub fn from_json(text: &str) -> Vec<ScanSummary> {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(scans) = v.get("scans").and_then(Value::as_array) else {
        return Vec::new();
    };
    scans
        .iter()
        .filter_map(|s| {
            Some(ScanSummary {
                root: PathBuf::from(s.get("root")?.as_str()?),
                when: s.get("when")?.as_u64()?,
                used: s.get("used")?.as_u64()?,
                top: s
                    .get("top")?
                    .as_array()?
                    .iter()
                    .filter_map(|p| Some((p.get(0)?.as_str()?.to_string(), p.get(1)?.as_u64()?)))
                    .collect(),
            })
        })
        .collect()
}

fn file() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h).join("Library/Application Support/mac-dir-stat/history.json")
    })
}

pub fn load() -> Vec<ScanSummary> {
    file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| from_json(&t))
        .unwrap_or_default()
}

pub fn save(all: &[ScanSummary]) {
    // Tests drive poll_scan; never touch the user's real history.
    if cfg!(test) {
        return;
    }
    let Some(path) = file() else { return };
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    // Write then rename, so a crash mid-write can't leave half a file.
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, to_json(all)).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}
