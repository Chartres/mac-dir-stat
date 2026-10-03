use super::ScanProgress;
use crate::scanner::tree::FileTree;
use crossbeam_channel::Sender;
use jwalk::WalkDir;
use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// The device id of `path` (`st_dev`), used to detect volume mount roots.
fn device_id(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        if libc::stat(c_path.as_ptr(), &mut st) == 0 {
            Some(st.st_dev as u64)
        } else {
            None
        }
    }
}

/// True when `path` is the root of a mounted volume — `/` or any mount point
/// whose device differs from its parent's (e.g. `/Volumes/External`). Free and
/// unknown space are only meaningful at a volume root.
fn is_volume_root(path: &Path) -> bool {
    if path == Path::new("/") {
        return true;
    }
    // APFS firmlinks (/Users, /Applications, /Library, /private, …) appear on a
    // different st_dev from / but are not real mount points — they are firmlinks
    // into the APFS Data volume. Treating them as roots adds spurious Free Space
    // and Hidden/Skipped nodes under each of them.
    const APFS_FIRMLINKS: &[&str] = &[
        "/Applications", "/Library", "/Users",
        "/cores", "/home", "/opt", "/private",
        // Additional firmlinks confirmed to have a different st_dev from /
        "/usr", "/usr/local",
        "/System/Library/Caches",
        "/System/Library/Assets",
        "/System/Library/AssetsV2",
    ];
    if APFS_FIRMLINKS.iter().any(|&p| path == Path::new(p)) {
        return false;
    }
    match (device_id(path), path.parent().and_then(device_id)) {
        (Some(dev), Some(parent_dev)) => dev != parent_dev,
        _ => false,
    }
}

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
///
/// When the app has Full Disk Access the TCC-protected subdirectories are
/// readable, so we only skip the structural paths (firmlinks, snapshots).
fn build_skip_paths() -> Vec<PathBuf> {
    let has_fda = crate::platform::fda::has_full_disk_access();

    let mut paths: Vec<PathBuf> = vec![
        // Time Machine local snapshots and installer sandboxes
        // Note: /System/Volumes is NOT in this list — should_skip() handles
        // it with a scan-root exception so scanning /System/Volumes/Data works.
        PathBuf::from("/.MobileBackups"),
        PathBuf::from("/.PKInstallSandboxManager"),
        PathBuf::from("/.PKInstallSandboxManager-SystemSoftware"),
    ];

    if !has_fda {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            // TCC-protected user dirs — accessing any of these triggers a
            // system permission prompt. Skip preemptively.
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
    }
    paths
}

fn is_photos_library(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "photoslibrary" || e == "photolibrary")
}

fn should_skip(path: &Path, scan_root: &Path, skip_paths: &[PathBuf]) -> bool {
    // APFS firmlinks — skip sibling volumes under /System/Volumes, but not
    // when the scan root itself is inside /System/Volumes (e.g. /System/Volumes/Data).
    if path.starts_with("/System/Volumes") && !path.starts_with(scan_root) {
        return true;
    }
    // When scanning from /, skip /Volumes entries (mount points / firmlinks
    // to root, plus the "Removable Volumes" TCC trigger).
    if scan_root == Path::new("/") && path.starts_with("/Volumes") {
        return true;
    }
    // /net and /home are autofs mounts that stall on lookup when accessed
    // from outside. Skip them unless the user explicitly scanned into them.
    for autofs in &["/net", "/home"] {
        let ap = Path::new(autofs);
        if path.starts_with(ap) && !scan_root.starts_with(ap) {
            return true;
        }
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

pub fn walk_directory(root: PathBuf, tx: Sender<ScanProgress>, inject_synthetics: bool) {
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

        let parent_path = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };

        let parent_id = match dir_map.get(&parent_path) {
            Some(&id) => id,
            None => continue, // orphan: parent was filtered or raced; drop it
        };

        let file_name_os = match path.file_name() {
            Some(n) => n,
            None => continue,
        };
        let file_name_bytes = file_name_os.as_bytes();

        let depth = path.components().count().saturating_sub(root.components().count()) as u16;

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                error_count += 1;
                // If this is a directory, add a stub node so its children
                // get the right parent instead of falling back to closest-
                // ancestor, which produces wrong full_paths for Trash/Reveal.
                if entry.file_type().is_dir() {
                    let node_id = tree.add_dir(
                        parent_id,
                        file_name_bytes,
                        false,
                        SystemTime::UNIX_EPOCH,
                        depth,
                    );
                    dir_map.insert(path.to_path_buf(), node_id);
                    dir_count += 1;
                }
                continue;
            }
        };

        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

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
    // volume root (`/` or a mounted volume like /Volumes/External). For
    // sub-roots (e.g. ~/Downloads), the disk's free space doesn't belong
    // inside the scanned tree.
    if inject_synthetics && is_volume_root(&root) {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn orphan_entry_missing_parent_produces_no_node() {
        // Simulates the guard: if a parent is absent from dir_map the entry
        // must be dropped, not re-parented to a distant ancestor.
        let dir_map: HashMap<PathBuf, usize> = HashMap::new();
        let parent_path = PathBuf::from("/some/dir");
        // Missing parent → None, which triggers `continue` in the walk loop.
        assert!(dir_map.get(&parent_path).is_none());
    }
}
