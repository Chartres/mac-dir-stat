use super::ScanProgress;
use crate::scanner::tree::FileTree;
use crossbeam_channel::Sender;
use jwalk::WalkDir;
use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Get total and available bytes for the volume containing `path`.
fn volume_space(path: &Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let total = stat.f_blocks as u64 * stat.f_frsize as u64;
            let avail = stat.f_bavail as u64 * stat.f_frsize as u64;
            Some((total, avail))
        } else {
            None
        }
    }
}

/// Builds the list of paths to skip preemptively. Skipping is done at the
/// directory-listing stage (via jwalk's process_read_dir) so the walker never
/// even tries to enter these paths — which is what avoids triggering macOS
/// TCC permission prompts in the first place.
fn build_skip_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = vec![
        // APFS firmlinks (duplicate the entire fs)
        PathBuf::from("/System/Volumes"),
        // Time Machine local snapshots and installer sandboxes
        PathBuf::from("/.MobileBackups"),
        PathBuf::from("/.PKInstallSandboxManager"),
        PathBuf::from("/.PKInstallSandboxManager-SystemSoftware"),
    ];

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        // TCC-protected user dirs — accessing any of these triggers a system
        // permission prompt. Skip preemptively.
        for sub in [
            "Library/Mail",
            "Library/Messages",
            "Library/Calendars",
            "Library/Reminders",
            "Library/Safari",
            "Library/Cookies",
            "Library/HomeKit",
            "Library/IdentityServices",
            "Library/Suggestions",
            "Library/PersonalizationPortrait",
            "Library/Sharing",
            "Library/Metadata/CoreSpotlight",
            "Library/CoreFollowUp",
            "Library/Application Support/AddressBook",
            "Library/Application Support/CallHistoryDB",
            "Library/Application Support/CallHistoryTransactions",
            "Library/Application Support/com.apple.TCC",
            "Library/Caches/com.apple.Safari",
            "Library/Containers/com.apple.mail",
        ] {
            paths.push(home.join(sub));
        }
    }
    paths
}

fn is_photos_library(path: &Path) -> bool {
    path.extension()
        .map_or(false, |e| e == "photoslibrary" || e == "photolibrary")
}

fn should_skip(path: &Path, scan_root: &Path, skip_paths: &[PathBuf]) -> bool {
    // APFS firmlinks
    if path.starts_with("/System/Volumes") {
        return true;
    }
    // When scanning from /, skip /Volumes entries (mount points / firmlinks
    // to root, plus the "Removable Volumes" TCC trigger).
    if scan_root == Path::new("/") && path.starts_with("/Volumes") {
        return true;
    }
    // TCC-protected paths
    for p in skip_paths {
        if path.starts_with(p) {
            return true;
        }
    }
    if is_photos_library(path) {
        return true;
    }
    false
}

pub fn walk_directory(root: PathBuf, tx: Sender<ScanProgress>) {
    let mut tree = FileTree::new();
    tree.rename_root(root.as_os_str().as_bytes());

    let mut dir_map: HashMap<PathBuf, usize> = HashMap::new();
    dir_map.insert(root.clone(), tree.root());

    let mut file_count: usize = 0;
    let mut dir_count: usize = 0;
    let mut byte_count: u64 = 0;
    let mut error_count: usize = 0;
    let mut progress_counter: usize = 0;

    let skip_paths = Arc::new(build_skip_paths());
    let walker_skip_paths = Arc::clone(&skip_paths);
    let walker_root = root.clone();

    for entry in WalkDir::new(&root)
        .skip_hidden(false)
        .sort(true)
        .follow_links(false)
        .process_read_dir(move |_depth, _dir_path, _state, children| {
            children.retain(|res| match res {
                Ok(entry) => !should_skip(&entry.path(), &walker_root, &walker_skip_paths),
                Err(_) => true,
            });
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                error_count += 1;
                continue;
            }
        };

        let path = entry.path();

        if path == root {
            continue;
        }

        // Defensive — process_read_dir already filters most cases, but keep
        // this for the rare entry that slips through.
        if should_skip(&path, &root, &skip_paths) {
            continue;
        }

        let parent_path = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };

        let parent_id = match dir_map.get(&parent_path) {
            Some(&id) => id,
            None => match find_closest_ancestor(&dir_map, &parent_path) {
                Some(id) => id,
                None => continue,
            },
        };

        let file_name_os = match path.file_name() {
            Some(n) => n,
            None => continue,
        };
        let file_name_bytes = file_name_os.as_bytes();

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                error_count += 1;
                continue;
            }
        };

        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let depth = path.components().count().saturating_sub(root.components().count()) as u16;

        if metadata.is_dir() {
            let node_id = tree.add_dir(
                parent_id,
                file_name_bytes,
                depth < 2,
                modified,
                depth,
            );
            dir_map.insert(path.to_path_buf(), node_id);
            dir_count += 1;
        } else {
            let size = metadata.len();
            let extension_owned = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase());
            tree.add_file(
                parent_id,
                file_name_bytes,
                size,
                extension_owned.as_deref(),
                modified,
                depth,
            );
            file_count += 1;
            byte_count += size;
        }

        progress_counter += 1;
        if progress_counter % 500 == 0 {
            let _ = tx.send(ScanProgress::Counting {
                files: file_count,
                dirs: dir_count,
                bytes: byte_count,
                errors: error_count,
                current_path: Some(path.display().to_string()),
            });
        }
    }

    // Free the path → NodeId map before doing post-processing. With deep
    // trees this can hold hundreds of MB of full PathBufs.
    drop(dir_map);

    tree.compute_sizes();

    // Free Space + Skipped synthetic nodes — only meaningful when scanning a
    // volume root. For sub-roots (e.g. ~/Downloads), the disk's free space
    // doesn't belong inside the scanned tree.
    if root == Path::new("/") {
        if let Some((total, avail)) = volume_space(&root) {
            let used = tree.node(tree.root()).size;

            // Real free space — what the filesystem reports as available.
            // Replaces the previous (total - used) heuristic, which over-
            // counted whenever any path was unreadable or TCC-skipped.
            if avail > 0 {
                tree.add_file(
                    tree.root(),
                    b"<Free Space>",
                    avail,
                    Some("__free_space__"),
                    SystemTime::now(),
                    1,
                );
            }

            // The gap between (used + avail) and total is what we couldn't
            // see — TCC-protected paths, /System/Volumes firmlinks already
            // skipped, etc. Surface it as its own block so it's obvious
            // there's data we didn't account for.
            let counted = used.saturating_add(avail);
            let skipped = total.saturating_sub(counted);
            if skipped > 16 * 1024 * 1024 {
                tree.add_file(
                    tree.root(),
                    b"<Hidden / Skipped>",
                    skipped,
                    Some("__skipped__"),
                    SystemTime::now(),
                    1,
                );
            }
            tree.compute_sizes();
        }
    }

    // Final progress snapshot so the receiver sees the post-loop counts
    // (in particular the total error count, which may have grown during
    // the last < 500 entries that didn't trigger a periodic send).
    let _ = tx.send(ScanProgress::Counting {
        files: file_count,
        dirs: dir_count,
        bytes: byte_count,
        errors: error_count,
        current_path: None,
    });
    let _ = tx.send(ScanProgress::Done(tree));
}

fn find_closest_ancestor(dir_map: &HashMap<PathBuf, usize>, path: &Path) -> Option<usize> {
    let mut current = path.parent();
    while let Some(p) = current {
        if let Some(&id) = dir_map.get(p) {
            return Some(id);
        }
        current = p.parent();
    }
    None
}
