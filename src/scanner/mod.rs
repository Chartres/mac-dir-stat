pub mod tree;
pub mod walk;

use crossbeam_channel::Sender;
use std::path::PathBuf;
use tree::FileTree;

pub enum ScanProgress {
    Counting {
        files: usize,
        dirs: usize,
        bytes: u64,
        errors: usize,
        current_path: Option<String>,
    },
    Done(FileTree),
    Error(String),
}

pub fn scan(root: PathBuf, tx: Sender<ScanProgress>) {
    scan_impl(root, tx, true);
}

/// Like `scan` but omits synthetic Free Space / Hidden nodes — used for
/// partial subtree refreshes where the target is not the tree root.
pub fn scan_subtree(root: PathBuf, tx: Sender<ScanProgress>) {
    scan_impl(root, tx, false);
}

fn scan_impl(root: PathBuf, tx: Sender<ScanProgress>, inject_synthetics: bool) {
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            walk::walk_directory(root, tx.clone(), inject_synthetics);
        }));
        if result.is_err() {
            let _ = tx.send(ScanProgress::Error("Scanner panicked".into()));
        }
    });
}
