//! Heuristic detection of "safe to delete" directories — caches, build
//! artifacts, simulators, and similar regenerable data — to surface
//! cleanup opportunities to the user.

use crate::scanner::tree::{FileTree, NodeId};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    pub node_id: NodeId,
    pub path: PathBuf,
    pub category: &'static str,
    pub description: &'static str,
    pub size: u64,
    /// Latest modification time anywhere inside: when it was last in use.
    pub newest: SystemTime,
}

/// What it takes to get a rebuildable category back, in plain words. These
/// are fixed estimates, never promises; the rule behind each one is the
/// category's description.
pub fn restore_cost(category: &str) -> &'static str {
    match category {
        "Xcode DerivedData" | "DerivedData (loose)" => "the next Xcode build, minutes per project",
        "iOS DeviceSupport" | "watchOS DeviceSupport" => {
            "Xcode downloads it again when a device connects"
        }
        "Xcode Archives" => "rebuild from source at that version; not byte-identical",
        "Simulator Caches" => "the next simulator launch",
        "iOS / watchOS Simulators" => "Xcode recreates them; apps and data inside are lost",
        "SwiftUI Previews" => "the next preview render",
        "Application Caches" => "apps refill them as you use them",
        "Application Logs" => "nothing; new logs start straight away",
        "Docker Desktop Data" => "docker pull and rebuild; named volumes are lost",
        "Cargo registry cache" => "the next cargo build downloads them",
        "Rustup toolchains" => "rustup toolchain install",
        "node_modules" => "npm install (or pnpm, yarn), about a minute each",
        "Python __pycache__" => "the next import",
        ".pytest_cache" => "the next pytest run",
        "Gradle caches" => "the next Gradle build downloads them",
        "Maven local repo" => "the next Maven build downloads them",
        "Rust target/" => "cargo build, minutes per crate",
        ".next/" => "the next next build",
        ".turbo/" => "the next turbo run",
        ".cache directories" => "the tool that made it fills it again",
        _ => "the tool that made it",
    }
}

/// True if `path` is, or sits inside, something a cleanup rule says a tool
/// can rebuild.
pub fn is_rebuildable(path: &Path) -> bool {
    path.ancestors().any(|a| classify(a).is_some())
}

/// A folder the app never offers for deletion: it holds things only the
/// user has. Shown so they know where the space is.
#[derive(Debug, Clone)]
pub struct OnlyYours {
    pub node_id: NodeId,
    pub path: PathBuf,
    pub label: String,
    pub size: u64,
}

/// The usual places personal data lives under `home`, largest first.
/// Photos libraries are listed on their own and their bytes taken out of
/// the folder that holds them, and so are rebuildable folders inside (a
/// `node_modules` under Documents is on the other side of the inventory).
/// Entries under 1 MB are dropped.
pub fn find_only_yours(
    tree: &FileTree,
    home: Option<&Path>,
    rebuildable: &[CleanupCandidate],
) -> Vec<OnlyYours> {
    const PLACES: &[(&str, &str)] = &[
        ("Documents", "Documents"),
        ("Desktop", "Desktop"),
        ("Pictures", "Pictures"),
        ("Movies", "Movies"),
        ("Music", "Music"),
        ("Library/Mail", "Mail"),
        ("Library/Messages", "Messages"),
    ];
    let Some(home) = home else { return Vec::new() };
    let mut out: Vec<OnlyYours> = Vec::new();
    for &(rel, label) in PLACES {
        let path = home.join(rel);
        let Some(id) = tree.find_path(&path) else { continue };
        let mut size = tree.node(id).size;
        // Photos keeps its library in ~/Pictures by default; an extra
        // library may sit one folder deeper.
        if rel == "Pictures" {
            for lib in photos_libraries(tree, id, 2) {
                let n = tree.node(lib).size;
                size = size.saturating_sub(n);
                let name = tree.name(lib);
                let label = match name.strip_suffix(".photoslibrary") {
                    Some("Photos Library") | None => "Photos Library".to_string(),
                    Some(other) => format!("Photos: {other}"),
                };
                out.push(OnlyYours { node_id: lib, path: tree.full_path(lib), label, size: n });
            }
        }
        let label = if size < tree.node(id).size { format!("{label} (the rest)") } else { label.into() };
        out.push(OnlyYours { node_id: id, path, label, size });
    }
    for o in &mut out {
        let inside: u64 = rebuildable
            .iter()
            .filter(|c| c.path.starts_with(&o.path))
            .map(|c| c.size)
            .sum();
        o.size = o.size.saturating_sub(inside);
    }
    out.retain(|o| o.size > 1024 * 1024);
    out.sort_by(|a, b| b.size.cmp(&a.size));
    out
}

fn photos_libraries(tree: &FileTree, dir: NodeId, depth: u8) -> Vec<NodeId> {
    let mut out = Vec::new();
    for &c in tree.node(dir).children() {
        if !tree.is_alive(c) || !tree.node(c).is_dir() {
            continue;
        }
        if tree.name_bytes(c).ends_with(b".photoslibrary") {
            out.push(c);
        } else if depth > 1 {
            out.extend(photos_libraries(tree, c, depth - 1));
        }
    }
    out
}

/// The short account shown after a trash from the Cleanup window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Account {
    pub items: usize,
    pub moved: u64,
    pub rebuildable: u64,
    pub only_yours: u64,
    /// (free, total) bytes on the scanned volume, if it could be read.
    pub disk: Option<(u64, u64)>,
}

impl Account {
    /// Adds what one trash action moved: (path, bytes) per item.
    pub fn add(&mut self, moved: &[(PathBuf, u64)]) {
        for (path, size) in moved {
            self.items += 1;
            self.moved += size;
            if is_rebuildable(path) {
                self.rebuildable += size;
            } else {
                self.only_yours += size;
            }
        }
    }
}

/// Returns (category, description) if the path matches a known cleanup
/// pattern. Skipping into matched directories is intentional — we report
/// the top-level match, not its internals.
pub fn classify(path: &Path) -> Option<(&'static str, &'static str)> {
    // Path-suffix matches (location-specific signals — high confidence)
    const SUFFIX_RULES: &[(&str, &str, &str)] = &[
        (
            "Library/Developer/Xcode/DerivedData",
            "Xcode DerivedData",
            "Xcode rebuilds these on demand. Safe to delete.",
        ),
        (
            "Library/Developer/Xcode/iOS DeviceSupport",
            "iOS DeviceSupport",
            "Symbols for past iOS device versions. Xcode re-downloads on demand.",
        ),
        (
            "Library/Developer/Xcode/watchOS DeviceSupport",
            "watchOS DeviceSupport",
            "Symbols for past watchOS device versions. Xcode re-downloads on demand.",
        ),
        (
            "Library/Developer/Xcode/Archives",
            "Xcode Archives",
            "Past app build archives. Useful only if you still need symbolication for shipped builds.",
        ),
        (
            "Library/Developer/CoreSimulator/Caches",
            "Simulator Caches",
            "Cached simulator runtimes. Recreated as needed.",
        ),
        (
            "Library/Developer/CoreSimulator/Devices",
            "iOS / watchOS Simulators",
            "Simulators recreate on first launch. Drops local sim data and installed sim apps.",
        ),
        (
            "Library/Developer/Xcode/UserData/Previews",
            "SwiftUI Previews",
            "Cached preview renderings. Xcode regenerates them.",
        ),
        (
            "Library/Caches",
            "Application Caches",
            "Apps regenerate caches as needed. Only obviously safe ones; don't worry — your data isn't here.",
        ),
        (
            "Library/Logs",
            "Application Logs",
            "Historical log files. Removing them clears app diagnostics history.",
        ),
        (
            "Library/Containers/com.docker.docker",
            "Docker Desktop Data",
            "Docker images, build cache, and named volumes. Named volumes may contain database data that cannot be regenerated. Prefer `docker system prune` for finer-grained cleanup.",
        ),
        (
            ".cargo/registry",
            "Cargo registry cache",
            "Downloaded Rust crate sources. Re-fetched on next `cargo build`.",
        ),
        (
            ".rustup/toolchains",
            "Rustup toolchains",
            "Previously-installed Rust toolchains. Reinstall via `rustup` if needed.",
        ),
    ];
    for &(suffix, label, desc) in SUFFIX_RULES {
        // Path::ends_with checks component boundaries, so "MyLibrary/Caches"
        // does not match the "Library/Caches" rule.
        if path.ends_with(suffix) {
            return Some((label, desc));
        }
    }

    // Bare-name matches (low-level signals — match wherever they appear)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        return match name {
            "node_modules" => Some((
                "node_modules",
                "Run `npm install` (or pnpm/yarn) to restore. Heavy on small files.",
            )),
            "__pycache__" => Some((
                "Python __pycache__",
                "Compiled bytecode caches; Python regenerates on next import.",
            )),
            ".pytest_cache" => Some((
                ".pytest_cache",
                "pytest cache dir; safe to delete.",
            )),
            ".gradle" => Some((
                "Gradle caches",
                "Gradle build cache and local dependency store. May contain gradle.properties with credentials — review before deleting.",
            )),
            ".m2" => Some((
                "Maven local repo",
                "Maven local artifact cache. May contain settings.xml with credentials — review before deleting.",
            )),
            "DerivedData" => Some((
                "DerivedData (loose)",
                "Xcode build artifacts found outside the standard location.",
            )),
            "target" if path.parent().map(|p| p.join("Cargo.toml").exists()).unwrap_or(false) => {
                Some((
                    "Rust target/",
                    "`cargo build` recreates these. Often the largest single dir on a Rust dev machine.",
                ))
            }
            ".next" => Some((
                ".next/",
                "Next.js build output. Regenerated on `next build`.",
            )),
            ".turbo" => Some((
                ".turbo/",
                "Turborepo cache. Refilled on next build.",
            )),
            ".cache" => Some((
                ".cache directories",
                "App- and tool-specific caches. Generally regenerable.",
            )),
            // .Trash / .Trashes are intentionally excluded: moving them to the
            // Trash via the cleanup button would incorrectly nest the system
            // Trash inside itself. Use the dedicated "Empty Trash…" button.
            _ => None,
        };
    }

    None
}

/// Walks the tree once and returns all directories matching cleanup rules,
/// sorted by size descending. Doesn't recurse into matched directories.
/// The root node itself is never a candidate — classifying the scan root
/// (e.g. ~/Library) would be catastrophic.
pub fn find_candidates(tree: &FileTree, root: NodeId) -> Vec<CleanupCandidate> {
    let mut candidates = Vec::new();
    if tree.is_alive(root) {
        // Skip root itself; walk children directly.
        for &child in tree.node(root).children() {
            walk(tree, child, &mut candidates);
        }
    }
    candidates.sort_by(|a, b| b.size.cmp(&a.size));
    candidates
}

fn walk(tree: &FileTree, start: NodeId, out: &mut Vec<CleanupCandidate>) {
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        if !tree.is_alive(id) {
            continue;
        }
        let node = tree.node(id);
        if !node.is_dir() {
            continue;
        }
        let path = tree.full_path(id);
        if let Some((category, description)) = classify(&path) {
            let size = node.size;
            // Threshold: ignore tiny matches — they're noise.
            if size > 1024 * 1024 {
                out.push(CleanupCandidate {
                    node_id: id,
                    path,
                    category,
                    description,
                    size,
                    newest: newest(tree, id),
                });
            }
            // Don't recurse into a matched directory — we want the parent
            // match, not a hundred sub-matches inside Library/Caches.
            continue;
        }
        stack.extend_from_slice(node.children());
    }
}

fn newest(tree: &FileTree, id: NodeId) -> SystemTime {
    let mut t = tree.node(id).modified;
    let mut stack = vec![id];
    while let Some(n) = stack.pop() {
        for &c in tree.node(n).children() {
            if tree.is_alive(c) {
                t = t.max(tree.node(c).modified);
                stack.push(c);
            }
        }
    }
    t
}
