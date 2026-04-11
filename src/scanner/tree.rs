use std::ffi::OsString;
use std::path::PathBuf;
use std::time::SystemTime;

pub type NodeId = usize;

#[derive(Debug)]
pub struct FileTree {
    nodes: Vec<Node>,
    root: NodeId,
}

#[derive(Debug)]
pub struct Node {
    pub name: OsString,
    pub size: u64,
    pub kind: NodeKind,
    pub modified: SystemTime,
    pub parent: Option<NodeId>,
    pub depth: u16,
    alive: bool,
}

#[derive(Debug)]
pub enum NodeKind {
    File { extension: Option<String> },
    Directory { children: Vec<NodeId>, expanded: bool },
}

impl Node {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, NodeKind::Directory { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self.kind, NodeKind::File { .. })
    }

    pub fn children(&self) -> &[NodeId] {
        match &self.kind {
            NodeKind::Directory { children, .. } => children,
            NodeKind::File { .. } => &[],
        }
    }

    pub fn extension(&self) -> Option<&str> {
        match &self.kind {
            NodeKind::File { extension } => extension.as_deref(),
            NodeKind::Directory { .. } => None,
        }
    }
}

impl FileTree {
    pub fn new() -> Self {
        let root = Node {
            name: OsString::from("/"),
            size: 0,
            kind: NodeKind::Directory {
                children: vec![],
                expanded: true,
            },
            modified: SystemTime::now(),
            parent: None,
            depth: 0,
            alive: true,
        };
        FileTree {
            nodes: vec![root],
            root: 0,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id]
    }

    pub fn add_node(
        &mut self,
        parent: NodeId,
        name: OsString,
        size: u64,
        kind: NodeKind,
        modified: SystemTime,
        depth: u16,
    ) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            name,
            size,
            kind,
            modified,
            parent: Some(parent),
            depth,
            alive: true,
        });
        if let NodeKind::Directory { children, .. } = &mut self.nodes[parent].kind {
            children.push(id);
        }
        id
    }

    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    pub fn compute_sizes(&mut self) {
        for i in (0..self.nodes.len()).rev() {
            if !self.nodes[i].alive {
                continue;
            }
            if let NodeKind::Directory { ref children, .. } = self.nodes[i].kind {
                let child_ids: Vec<NodeId> = children.clone();
                let total: u64 = child_ids
                    .iter()
                    .filter(|&&c| self.nodes[c].alive)
                    .map(|&c| self.nodes[c].size)
                    .sum();
                self.nodes[i].size = total;
            }
        }
    }

    pub fn remove_node(&mut self, id: NodeId) {
        let size = self.nodes[id].size;
        let parent = self.nodes[id].parent;

        self.nodes[id].alive = false;

        if let Some(pid) = parent {
            if let NodeKind::Directory { children, .. } = &mut self.nodes[pid].kind {
                children.retain(|&c| c != id);
            }
        }

        let mut current = parent;
        while let Some(pid) = current {
            self.nodes[pid].size = self.nodes[pid].size.saturating_sub(size);
            current = self.nodes[pid].parent;
        }

        self.mark_dead_recursive(id);
    }

    fn mark_dead_recursive(&mut self, id: NodeId) {
        if let NodeKind::Directory { ref children, .. } = self.nodes[id].kind {
            let child_ids: Vec<NodeId> = children.clone();
            for child in child_ids {
                self.nodes[child].alive = false;
                self.mark_dead_recursive(child);
            }
        }
    }

    pub fn full_path(&self, id: NodeId) -> PathBuf {
        let mut parts = vec![];
        let mut current = id;
        loop {
            parts.push(self.nodes[current].name.clone());
            if let Some(pid) = self.nodes[current].parent {
                current = pid;
            } else {
                break;
            }
        }
        parts.reverse();
        let mut path = PathBuf::from(&parts[0]);
        for part in &parts[1..] {
            path.push(part);
        }
        path
    }

    pub fn collect_extensions(&self, root: NodeId) -> Vec<(String, u64, usize)> {
        let mut map: std::collections::HashMap<String, (u64, usize)> =
            std::collections::HashMap::new();
        self.collect_extensions_recursive(root, &mut map);
        let mut result: Vec<(String, u64, usize)> = map
            .into_iter()
            .map(|(ext, (bytes, count))| (ext, bytes, count))
            .collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    fn collect_extensions_recursive(
        &self,
        id: NodeId,
        map: &mut std::collections::HashMap<String, (u64, usize)>,
    ) {
        let node = &self.nodes[id];
        if !node.alive {
            return;
        }
        match &node.kind {
            NodeKind::File { extension } => {
                let ext = extension.clone().unwrap_or_default();
                let entry = map.entry(ext).or_insert((0, 0));
                entry.0 += node.size;
                entry.1 += 1;
            }
            NodeKind::Directory { children, .. } => {
                for &child in children {
                    self.collect_extensions_recursive(child, map);
                }
            }
        }
    }

    pub fn collect_files(&self, root: NodeId) -> Vec<NodeId> {
        let mut files = vec![];
        self.collect_files_recursive(root, &mut files);
        files
    }

    fn collect_files_recursive(&self, id: NodeId, files: &mut Vec<NodeId>) {
        let node = &self.nodes[id];
        if !node.alive {
            return;
        }
        match &node.kind {
            NodeKind::File { .. } => files.push(id),
            NodeKind::Directory { children, .. } => {
                for &child in children {
                    self.collect_files_recursive(child, files);
                }
            }
        }
    }

    pub fn children_sorted(&self, id: NodeId) -> Vec<NodeId> {
        let mut children: Vec<NodeId> = self.node(id).children().to_vec();
        children.retain(|&c| self.nodes[c].alive);
        children.sort_by(|&a, &b| self.nodes[b].size.cmp(&self.nodes[a].size));
        children
    }

    pub fn is_alive(&self, id: NodeId) -> bool {
        self.nodes[id].alive
    }
}
