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
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            walk::walk_directory(root, tx.clone());
        }));
        if result.is_err() {
            let _ = tx.send(ScanProgress::Error("Scanner panicked".into()));
        }
    });
}
