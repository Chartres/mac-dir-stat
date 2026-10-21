use crossbeam_channel::unbounded;
use mac_dir_stat::scanner::walk::walk_directory;
use mac_dir_stat::scanner::ScanProgress;
use std::fs;
use std::path::PathBuf;

/// Scaffolds a temp dir, runs walk_directory synchronously (on the calling
/// thread), and returns the finished FileTree.
fn scan_temp(root: PathBuf) -> mac_dir_stat::scanner::tree::FileTree {
    let (tx, rx) = unbounded();
    walk_directory(root, tx);
    for msg in rx {
        match msg {
            ScanProgress::Done(tree) => return tree,
            ScanProgress::Error(e) => panic!("scan error: {}", e),
            ScanProgress::Counting { .. } => {}
        }
    }
    panic!("channel closed without Done");
}

#[test]
fn walk_counts_files_and_subdirs() {
    // Build: <tmp>/root/file.txt  (1 000 bytes)
    //        <tmp>/root/sub/inner.bin  (2 000 bytes)
    let base = std::env::temp_dir().join(format!(
        "mac_dir_stat_walk_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(base.join("sub")).unwrap();
    fs::write(base.join("file.txt"), vec![0u8; 1_000]).unwrap();
    fs::write(base.join("sub").join("inner.bin"), vec![0u8; 2_000]).unwrap();

    let tree = scan_temp(base.clone());
    let root = tree.root();

    // Root size should aggregate both files.
    assert_eq!(
        tree.node(root).size,
        3_000,
        "root size should be sum of all file sizes"
    );

    // Root should have two direct children: file.txt and sub/
    assert_eq!(
        tree.node(root).children().len(),
        2,
        "root should have exactly two direct children"
    );

    // sub/ directory should have one child (inner.bin)
    let sub_id = tree
        .node(root)
        .children()
        .iter()
        .copied()
        .find(|&id| tree.node(id).is_dir())
        .expect("sub/ directory not found");
    assert_eq!(
        tree.node(sub_id).children().len(),
        1,
        "sub/ should contain exactly one file"
    );
    assert_eq!(tree.node(sub_id).size, 2_000);

    fs::remove_dir_all(&base).ok();
}

#[test]
fn walk_deduplicates_hard_links() {
    // Build: <tmp>/base/original.bin  (4 000 bytes, nlink == 2)
    //        <tmp>/base/sub/hardlink  (same inode — hard link of original.bin)
    // The scanner must count the 4 000 bytes only once.
    let base = std::env::temp_dir().join(format!(
        "mac_dir_stat_hardlink_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(base.join("sub")).unwrap();
    fs::write(base.join("original.bin"), vec![0u8; 4_000]).unwrap();
    fs::hard_link(base.join("original.bin"), base.join("sub").join("hardlink")).unwrap();

    let tree = scan_temp(base.clone());
    let root = tree.root();

    // Total bytes must be 4 000, not 8 000.
    assert_eq!(
        tree.node(root).size,
        4_000,
        "hard-linked file should be counted once"
    );

    fs::remove_dir_all(&base).ok();
}

#[test]
fn walk_records_depth_relative_to_root() {
    // Build: <tmp>/base/sub/inner.bin — sub is depth 1, inner.bin depth 2.
    let base = std::env::temp_dir().join(format!(
        "mac_dir_stat_depth_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(base.join("sub")).unwrap();
    fs::write(base.join("sub").join("inner.bin"), vec![0u8; 10]).unwrap();

    let tree = scan_temp(base.clone());
    let sub = tree.node(tree.root()).children()[0];
    let inner = tree.node(sub).children()[0];
    assert_eq!(tree.node(sub).depth, 1, "direct child of root");
    assert_eq!(tree.node(inner).depth, 2, "grandchild of root");

    fs::remove_dir_all(&base).ok();
}
