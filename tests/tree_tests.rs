use mac_dir_stat::scanner::tree::{FileTree, NodeKind};
use std::ffi::OsString;
use std::time::SystemTime;

#[test]
fn test_add_file_and_directory() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_node(
        root,
        OsString::from("Documents"),
        0,
        NodeKind::Directory {
            children: vec![],
            expanded: false,
        },
        SystemTime::now(),
        1,
    );

    let file = tree.add_node(
        dir,
        OsString::from("readme.txt"),
        1024,
        NodeKind::File {
            extension: Some("txt".to_string()),
        },
        SystemTime::now(),
        2,
    );

    assert_eq!(tree.node(root).children().len(), 1);
    assert_eq!(tree.node(dir).children().len(), 1);
    assert_eq!(tree.node(file).size, 1024);
    assert_eq!(tree.node(file).parent, Some(dir));
}

#[test]
fn test_compute_sizes() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_node(
        root,
        OsString::from("src"),
        0,
        NodeKind::Directory {
            children: vec![],
            expanded: false,
        },
        SystemTime::now(),
        1,
    );

    tree.add_node(
        dir,
        OsString::from("a.rs"),
        500,
        NodeKind::File {
            extension: Some("rs".to_string()),
        },
        SystemTime::now(),
        2,
    );

    tree.add_node(
        dir,
        OsString::from("b.rs"),
        300,
        NodeKind::File {
            extension: Some("rs".to_string()),
        },
        SystemTime::now(),
        2,
    );

    tree.compute_sizes();

    assert_eq!(tree.node(dir).size, 800);
    assert_eq!(tree.node(root).size, 800);
}

#[test]
fn test_remove_node() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_node(
        root,
        OsString::from("tmp"),
        0,
        NodeKind::Directory {
            children: vec![],
            expanded: false,
        },
        SystemTime::now(),
        1,
    );

    let file = tree.add_node(
        dir,
        OsString::from("big.zip"),
        5000,
        NodeKind::File {
            extension: Some("zip".to_string()),
        },
        SystemTime::now(),
        2,
    );

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

    tree.add_node(
        root,
        OsString::from("a.rs"),
        500,
        NodeKind::File {
            extension: Some("rs".to_string()),
        },
        SystemTime::now(),
        1,
    );

    tree.add_node(
        root,
        OsString::from("b.rs"),
        300,
        NodeKind::File {
            extension: Some("rs".to_string()),
        },
        SystemTime::now(),
        1,
    );

    tree.add_node(
        root,
        OsString::from("c.txt"),
        200,
        NodeKind::File {
            extension: Some("txt".to_string()),
        },
        SystemTime::now(),
        1,
    );

    let exts = tree.collect_extensions(root);
    assert_eq!(exts[0], ("rs".to_string(), 800, 2));
    assert_eq!(exts[1], ("txt".to_string(), 200, 1));
}

#[test]
fn test_full_path() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let users = tree.add_node(
        root,
        OsString::from("Users"),
        0,
        NodeKind::Directory {
            children: vec![],
            expanded: false,
        },
        SystemTime::now(),
        1,
    );

    let pavol = tree.add_node(
        users,
        OsString::from("pavol"),
        0,
        NodeKind::Directory {
            children: vec![],
            expanded: false,
        },
        SystemTime::now(),
        2,
    );

    let file = tree.add_node(
        pavol,
        OsString::from("test.txt"),
        100,
        NodeKind::File {
            extension: Some("txt".to_string()),
        },
        SystemTime::now(),
        3,
    );

    let path = tree.full_path(file);
    assert_eq!(path.to_str().unwrap(), "/Users/pavol/test.txt");
}
