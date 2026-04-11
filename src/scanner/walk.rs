use super::ScanProgress;
use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use crossbeam_channel::Sender;
use jwalk::WalkDir;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn walk_directory(root: PathBuf, tx: Sender<ScanProgress>) {
    let mut tree = FileTree::new();
    tree.node_mut(tree.root()).name = OsString::from(root.to_string_lossy().as_ref());

    let mut dir_map: HashMap<PathBuf, NodeId> = HashMap::new();
    dir_map.insert(root.clone(), tree.root());

    let mut file_count: usize = 0;
    let mut dir_count: usize = 0;
    let mut byte_count: u64 = 0;
    let mut progress_counter: usize = 0;

    for entry in WalkDir::new(&root).skip_hidden(false).sort(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
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
            None => {
                match find_closest_ancestor(&dir_map, &parent_path) {
                    Some(id) => id,
                    None => continue,
                }
            }
        };

        let file_name = match path.file_name() {
            Some(n) => OsString::from(n),
            None => continue,
        };

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let depth = path.components().count().saturating_sub(root.components().count()) as u16;

        if metadata.is_dir() {
            let node_id = tree.add_node(
                parent_id,
                file_name,
                0,
                NodeKind::Directory {
                    children: vec![],
                    expanded: depth < 2,
                },
                modified,
                depth,
            );
            dir_map.insert(path.to_path_buf(), node_id);
            dir_count += 1;
        } else {
            let size = metadata.len();
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase());
            tree.add_node(
                parent_id,
                file_name,
                size,
                NodeKind::File { extension },
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
            });
        }
    }

    tree.compute_sizes();
    let _ = tx.send(ScanProgress::Done(tree));
}

fn find_closest_ancestor(dir_map: &HashMap<PathBuf, NodeId>, path: &Path) -> Option<NodeId> {
    let mut current = path.parent();
    while let Some(p) = current {
        if let Some(&id) = dir_map.get(p) {
            return Some(id);
        }
        current = p.parent();
    }
    None
}
