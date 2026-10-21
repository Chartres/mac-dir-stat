use mac_dir_stat::scanner::tree::FileTree;
use std::time::SystemTime;

#[test]
fn test_add_file_and_directory() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_dir(root, b"Documents", false, SystemTime::now(), 1);

    let file = tree.add_file(dir, b"readme.txt", 1024, Some("txt"), SystemTime::now(), 2);

    assert_eq!(tree.node(root).children().len(), 1);
    assert_eq!(tree.node(dir).children().len(), 1);
    assert_eq!(tree.node(file).size, 1024);
    assert_eq!(tree.node(file).parent, Some(dir));
}

#[test]
fn test_compute_sizes() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_dir(root, b"src", false, SystemTime::now(), 1);

    tree.add_file(dir, b"a.rs", 500, Some("rs"), SystemTime::now(), 2);
    tree.add_file(dir, b"b.rs", 300, Some("rs"), SystemTime::now(), 2);

    tree.compute_sizes();

    assert_eq!(tree.node(dir).size, 800);
    assert_eq!(tree.node(root).size, 800);
}

#[test]
fn test_remove_node() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_dir(root, b"tmp", false, SystemTime::now(), 1);

    let file = tree.add_file(dir, b"big.zip", 5000, Some("zip"), SystemTime::now(), 2);

    tree.compute_sizes();
    assert_eq!(tree.node(root).size, 5000);

    tree.remove_node(file);

    assert_eq!(tree.node(dir).children().len(), 0);
    assert_eq!(tree.node(dir).size, 0);
    assert_eq!(tree.node(root).size, 0);
}

#[test]
fn test_collect_extensions() {
    let mut tree = FileTree::new();
    let root = tree.root();

    tree.add_file(root, b"a.rs", 500, Some("rs"), SystemTime::now(), 1);
    tree.add_file(root, b"b.rs", 300, Some("rs"), SystemTime::now(), 1);
    tree.add_file(root, b"c.txt", 200, Some("txt"), SystemTime::now(), 1);

    let exts = tree.collect_extensions(root);
    assert_eq!(exts[0], ("rs".to_string(), 800, 2));
    assert_eq!(exts[1], ("txt".to_string(), 200, 1));
}

#[test]
fn test_compute_counts() {
    // root
    //   src/        (dir)
    //     a.rs, b.rs
    //     nested/   (dir)
    //       c.rs
    //   top.txt
    let mut tree = FileTree::new();
    let root = tree.root();
    let src = tree.add_dir(root, b"src", false, SystemTime::now(), 1);
    tree.add_file(src, b"a.rs", 1, Some("rs"), SystemTime::now(), 2);
    tree.add_file(src, b"b.rs", 1, Some("rs"), SystemTime::now(), 2);
    let nested = tree.add_dir(src, b"nested", false, SystemTime::now(), 2);
    tree.add_file(nested, b"c.rs", 1, Some("rs"), SystemTime::now(), 3);
    tree.add_file(root, b"top.txt", 1, Some("txt"), SystemTime::now(), 1);

    tree.compute_sizes();

    assert_eq!(tree.node(nested).file_count, 1);
    assert_eq!(tree.node(nested).subdir_count, 0);
    assert_eq!(tree.node(src).file_count, 3); // a, b, c
    assert_eq!(tree.node(src).subdir_count, 1); // nested
    assert_eq!(tree.node(root).file_count, 4); // a, b, c, top
    assert_eq!(tree.node(root).subdir_count, 2); // src, nested
    assert_eq!(tree.node(root).items(), 6);
}

#[test]
fn test_counts_decrement_on_remove() {
    let mut tree = FileTree::new();
    let root = tree.root();
    let src = tree.add_dir(root, b"src", false, SystemTime::now(), 1);
    tree.add_file(src, b"a.rs", 1, Some("rs"), SystemTime::now(), 2);
    let nested = tree.add_dir(src, b"nested", false, SystemTime::now(), 2);
    tree.add_file(nested, b"c.rs", 1, Some("rs"), SystemTime::now(), 3);
    tree.compute_sizes();
    assert_eq!(tree.node(root).file_count, 2);
    assert_eq!(tree.node(root).subdir_count, 2);

    // Removing the nested dir drops its file (c.rs) and itself from totals.
    tree.remove_node(nested);
    assert_eq!(tree.node(src).file_count, 1); // only a.rs remains
    assert_eq!(tree.node(src).subdir_count, 0);
    assert_eq!(tree.node(root).file_count, 1);
    assert_eq!(tree.node(root).subdir_count, 1); // only src
}

#[test]
fn test_full_path() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let users = tree.add_dir(root, b"Users", false, SystemTime::now(), 1);
    let pavol = tree.add_dir(users, b"pavol", false, SystemTime::now(), 2);
    let file = tree.add_file(pavol, b"test.txt", 100, Some("txt"), SystemTime::now(), 3);

    let path = tree.full_path(file);
    assert_eq!(path.to_str().unwrap(), "/Users/pavol/test.txt");
}

#[test]
fn test_find_path() {
    let mut tree = FileTree::new();
    tree.rename_root(b"/Users/me");
    let root = tree.root();
    let dir = tree.add_dir(root, b"proj", false, SystemTime::now(), 1);
    let nm = tree.add_dir(dir, b"node_modules", false, SystemTime::now(), 2);
    assert_eq!(
        tree.find_path(std::path::Path::new("/Users/me")),
        Some(root)
    );
    assert_eq!(
        tree.find_path(std::path::Path::new("/Users/me/proj/node_modules")),
        Some(nm)
    );
    assert_eq!(tree.find_path(std::path::Path::new("/Users/me/nope")), None);
    assert_eq!(tree.find_path(std::path::Path::new("/Users/other")), None);
    tree.remove_node(nm);
    assert_eq!(
        tree.find_path(std::path::Path::new("/Users/me/proj/node_modules")),
        None
    );
}

#[test]
fn test_remove_node_double_dead_is_noop() {
    // Calling remove_node twice on the same node must not double-subtract
    // sizes or panic. The second call is silently ignored.
    let mut tree = FileTree::new();
    let root = tree.root();
    let dir = tree.add_dir(root, b"cache", false, SystemTime::now(), 1);
    tree.add_file(dir, b"big.bin", 4096, Some("bin"), SystemTime::now(), 2);
    tree.compute_sizes();

    assert_eq!(tree.node(root).size, 4096);

    tree.remove_node(dir);
    assert_eq!(tree.node(root).size, 0);

    // Second remove — must not underflow or panic.
    tree.remove_node(dir);
    assert_eq!(tree.node(root).size, 0, "double remove must not underflow");
    assert!(!tree.is_alive(dir));
}

#[test]
fn test_is_trashable_excludes_scan_root_and_synthetic_nodes() {
    // The context menu (Move to Trash, Move to External Disk) and Cmd+Backspace
    // offer actions only for trashable nodes.
    let mut tree = FileTree::new();
    let root = tree.root();
    let dir = tree.add_dir(root, b"proj", false, SystemTime::now(), 1);
    let file = tree.add_file(dir, b"a.txt", 10, Some("txt"), SystemTime::now(), 2);
    let free = tree.add_file(
        root,
        b"<Free Space>",
        100,
        Some("__free_space__"),
        SystemTime::now(),
        1,
    );
    let hidden = tree.add_file(
        root,
        b"<Hidden / Skipped>",
        100,
        Some("__skipped__"),
        SystemTime::now(),
        1,
    );
    tree.compute_sizes();
    assert!(!tree.is_trashable(root));
    assert!(!tree.is_trashable(free));
    assert!(!tree.is_trashable(hidden));
    assert!(tree.is_trashable(dir));
    assert!(tree.is_trashable(file));
    tree.remove_node(file);
    assert!(!tree.is_trashable(file));
}

#[test]
fn test_graft_under_sizes_counts_depths() {
    // Live tree: root / target_dir / old_file (1 000 bytes)
    let mut live = FileTree::new();
    let root = live.root();
    let target = live.add_dir(root, b"target", false, SystemTime::now(), 1);
    live.add_file(target, b"old_file.bin", 1_000, Some("bin"), SystemTime::now(), 2);
    live.compute_sizes();
    assert_eq!(live.node(root).size, 1_000);

    // Fresh subtree: root / a.rs (500 bytes) / sub / b.rs (300 bytes)
    let mut fresh = FileTree::new();
    let fr = fresh.root();
    fresh.add_file(fr, b"a.rs", 500, Some("rs"), SystemTime::now(), 1);
    let sub = fresh.add_dir(fr, b"sub", false, SystemTime::now(), 1);
    fresh.add_file(sub, b"b.rs", 300, Some("rs"), SystemTime::now(), 2);
    fresh.compute_sizes();

    live.graft_under(target, fresh);

    // old_file must be gone; sizes/counts must reflect the fresh subtree.
    assert_eq!(live.node(target).size, 800, "graft size");
    assert_eq!(live.node(target).file_count, 2, "graft file_count");
    assert_eq!(live.node(target).subdir_count, 1, "graft subdir_count");
    assert_eq!(live.node(root).size, 800, "root propagated");

    // Depths of grafted nodes must be relative to the live tree, not fresh.
    let grafted_a = live
        .node(target)
        .children()
        .iter()
        .copied()
        .find(|&id| !live.node(id).is_dir())
        .expect("a.rs");
    assert_eq!(live.node(grafted_a).depth, 2, "depth of a.rs under target (depth 1)");
}
