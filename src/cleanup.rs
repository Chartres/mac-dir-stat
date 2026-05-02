//! Heuristic detection of "safe to delete" directories — caches, build
//! artifacts, simulators, and similar regenerable data — to surface
//! cleanup opportunities to the user.

use crate::scanner::tree::{FileTree, NodeId};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    pub node_id: NodeId,
    pub path: PathBuf,
    pub category: &'static str,
    pub description: &'static str,
    pub size: u64,
}

/// Returns (category, description) if the path matches a known cleanup
/// pattern. Skipping into matched directories is intentional — we report
/// the top-level match, not its internals.
pub fn classify(path: &Path) -> Option<(&'static str, &'static str)> {
    let s = path.to_string_lossy();

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
            "Docker images, volumes, build cache. Prefer `docker system prune -a` for finer-grained cleanup.",
        ),
        (
            "Library/Containers/com.apple.iCloud.iCloudDrive",
            "iCloud Drive evictable cache",
            "Local cache of iCloud-stored files. Files reload from iCloud on next access.",
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
        if s.ends_with(suffix) {
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
                "Gradle redownloads dependencies on next build.",
            )),
            ".m2" => Some((
                "Maven local repo",
                "Maven local artifacts cache; rebuilt on next dependency resolve.",
            )),
            "DerivedData" => Some((
                "DerivedData (loose)",
                "Xcode build artifacts found outside the standard location.",
            )),
            "target" => Some((
                "Rust target/",
                "`cargo build` recreates these. Often the largest single dir on a Rust dev machine.",
            )),
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
            ".Trash" | ".Trashes" => Some((
                "Trash",
                "Files in the macOS Trash. Empty when you're sure you don't need them.",
            )),
            _ => None,
        };
    }

    None
}

/// Walks the tree once and returns all directories matching cleanup rules,
/// sorted by size descending. Doesn't recurse into matched directories.
pub fn find_candidates(tree: &FileTree, root: NodeId) -> Vec<CleanupCandidate> {
    let mut candidates = Vec::new();
    walk(tree, root, &mut candidates);
    candidates.sort_by(|a, b| b.size.cmp(&a.size));
    candidates
}

fn walk(tree: &FileTree, id: NodeId, out: &mut Vec<CleanupCandidate>) {
    if !tree.is_alive(id) {
        return;
    }
    let node = tree.node(id);
    if !node.is_dir() {
        return;
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
            });
        }
        // Don't recurse into a matched directory — we want the parent
        // match, not a hundred sub-matches inside Library/Caches.
        return;
    }
    for &child in node.children() {
        walk(tree, child, out);
    }
}
