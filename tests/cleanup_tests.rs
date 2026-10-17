use mac_dir_stat::cleanup::{classify, find_candidates};
use mac_dir_stat::scanner::tree::FileTree;
use std::path::Path;
use std::time::SystemTime;

// classify ──────────────────────────────────────────────────────────────────

#[test]
fn classify_trash_excluded() {
    // .Trash and .Trashes must never be offered as cleanup candidates —
    // "trashing" them would nest the system Trash inside itself.
    assert!(classify(Path::new("/Users/pavol/.Trash")).is_none());
    assert!(classify(Path::new("/Volumes/Ext/.Trashes")).is_none());
}

#[test]
fn classify_rustup_toolchains() {
    let path = Path::new("/Users/pavol/.rustup/toolchains");
    let (cat, _desc) = classify(path).expect(".rustup/toolchains should be classified");
    assert_eq!(cat, "Rustup toolchains");
}

#[test]
fn classify_icloud_drive_not_suggested() {
    // iCloud Drive's container can hold files not yet uploaded; suggesting
    // it for deletion risks data loss.
    let path = Path::new("/Users/pavol/Library/Containers/com.apple.iCloud.iCloudDrive");
    assert!(classify(path).is_none());
}

#[test]
fn classify_node_modules() {
    let path = Path::new("/Users/pavol/projects/app/node_modules");
    let (cat, _) = classify(path).expect("node_modules should be classified");
    assert_eq!(cat, "node_modules");
}

// find_candidates ──────────────────────────────────────────────────────────

/// Build a small tree that contains a .Trash dir and a node_modules dir
/// (both > 1 MB), then verify find_candidates excludes .Trash and includes
/// node_modules.
#[test]
fn find_candidates_excludes_trash_includes_node_modules() {
    let mut tree = FileTree::new();
    let root = tree.root();

    // node_modules — should be found (2 MB)
    let nm = tree.add_dir(root, b"node_modules", false, SystemTime::UNIX_EPOCH, 1);
    tree.add_file(
        nm,
        b"big.js",
        2 * 1024 * 1024,
        Some("js"),
        SystemTime::UNIX_EPOCH,
        2,
    );

    // .Trash — must NOT be in candidates
    let trash = tree.add_dir(root, b".Trash", false, SystemTime::UNIX_EPOCH, 1);
    tree.add_file(
        trash,
        b"old.zip",
        5 * 1024 * 1024,
        Some("zip"),
        SystemTime::UNIX_EPOCH,
        2,
    );

    tree.compute_sizes();

    let candidates = find_candidates(&tree, root);
    assert!(
        candidates.iter().any(|c| c.category == "node_modules"),
        "node_modules should be a candidate"
    );
    assert!(
        !candidates.iter().any(|c| c.path.ends_with(".Trash")),
        ".Trash must not appear as a candidate"
    );
}
