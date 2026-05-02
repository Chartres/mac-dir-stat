use mac_dir_stat::scanner::tree::FileTree;
use std::time::SystemTime;

#[test]
fn test_add_file_and_directory() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let dir = tree.add_dir(root, b"Documents", false, SystemTime::now(), 1);

    let file = tree.add_file(
        dir,
        b"readme.txt",
        1024,
        Some("txt"),
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
fn test_full_path() {
    let mut tree = FileTree::new();
    let root = tree.root();

    let users = tree.add_dir(root, b"Users", false, SystemTime::now(), 1);
    let pavol = tree.add_dir(users, b"pavol", false, SystemTime::now(), 2);
    let file = tree.add_file(pavol, b"test.txt", 100, Some("txt"), SystemTime::now(), 3);

    let path = tree.full_path(file);
    assert_eq!(path.to_str().unwrap(), "/Users/pavol/test.txt");
}
