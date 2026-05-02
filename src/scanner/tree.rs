use std::borrow::Cow;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::time::SystemTime;

pub type NodeId = usize;

/// Compact reference into a `StringArena`. 8 bytes vs 24 for `OsString`.
#[derive(Copy, Clone, Debug)]
pub struct StrRef {
    offset: u32,
    len: u32,
}

impl StrRef {
    pub const EMPTY: StrRef = StrRef { offset: 0, len: 0 };
}

/// Bump arena holding raw bytes for all node names + extensions.
/// One contiguous `Vec<u8>` instead of millions of small heap allocs.
#[derive(Debug, Default)]
pub struct StringArena {
    buf: Vec<u8>,
}

impl StringArena {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn intern_bytes(&mut self, b: &[u8]) -> StrRef {
        let offset = self.buf.len() as u32;
        self.buf.extend_from_slice(b);
        StrRef { offset, len: b.len() as u32 }
    }

    pub fn intern_str(&mut self, s: &str) -> StrRef {
        self.intern_bytes(s.as_bytes())
    }

    pub fn get(&self, r: StrRef) -> &[u8] {
        let start = r.offset as usize;
        &self.buf[start..start + r.len as usize]
    }
}

#[derive(Debug)]
pub struct FileTree {
    nodes: Vec<Node>,
    root: NodeId,
    strings: StringArena,
}

#[derive(Debug)]
pub struct Node {
    pub name: StrRef,
    pub size: u64,
    pub kind: NodeKind,
    pub modified: SystemTime,
    pub parent: Option<NodeId>,
    pub depth: u16,
    alive: bool,
}

#[derive(Debug)]
pub enum NodeKind {
    File { extension: Option<StrRef> },
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
}

impl FileTree {
    pub fn new() -> Self {
        let mut strings = StringArena::new();
        let root_name = strings.intern_str("/");
        let root = Node {
            name: root_name,
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
            strings,
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

    pub fn name_bytes(&self, id: NodeId) -> &[u8] {
        self.strings.get(self.nodes[id].name)
    }

    pub fn name(&self, id: NodeId) -> Cow<'_, str> {
        String::from_utf8_lossy(self.name_bytes(id))
    }

    pub fn extension(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id].kind {
            NodeKind::File { extension: Some(r) } => {
                std::str::from_utf8(self.strings.get(*r)).ok()
            }
            _ => None,
        }
    }

    pub fn rename_root(&mut self, name: &[u8]) {
        let r = self.strings.intern_bytes(name);
        self.nodes[self.root].name = r;
    }

    pub fn add_dir(
        &mut self,
        parent: NodeId,
        name: &[u8],
        expanded: bool,
        modified: SystemTime,
        depth: u16,
    ) -> NodeId {
        let name_ref = self.strings.intern_bytes(name);
        self.push_node(
            parent,
            Node {
                name: name_ref,
                size: 0,
                kind: NodeKind::Directory {
                    children: vec![],
                    expanded,
                },
                modified,
                parent: Some(parent),
                depth,
                alive: true,
            },
        )
    }

    pub fn add_file(
        &mut self,
        parent: NodeId,
        name: &[u8],
        size: u64,
        extension: Option<&str>,
        modified: SystemTime,
        depth: u16,
    ) -> NodeId {
        let name_ref = self.strings.intern_bytes(name);
        let ext_ref = extension.map(|s| self.strings.intern_str(s));
        self.push_node(
            parent,
            Node {
                name: name_ref,
                size,
                kind: NodeKind::File { extension: ext_ref },
                modified,
                parent: Some(parent),
                depth,
                alive: true,
            },
        )
    }

    fn push_node(&mut self, parent: NodeId, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
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
        let mut parts: Vec<&[u8]> = vec![];
        let mut current = id;
        loop {
            parts.push(self.name_bytes(current));
            if let Some(pid) = self.nodes[current].parent {
                current = pid;
            } else {
                break;
            }
        }
        parts.reverse();
        let mut path = PathBuf::from(OsStr::from_bytes(parts[0]));
        for part in &parts[1..] {
            path.push(OsStr::from_bytes(part));
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
                let ext = extension
                    .map(|r| String::from_utf8_lossy(self.strings.get(r)).into_owned())
                    .unwrap_or_default();
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

    /// Marks all descendants of `target` as dead and empties its children
    /// list. The node itself stays. Used as the first step of grafting a
    /// freshly-scanned subtree onto an existing tree.
    pub fn clear_descendants(&mut self, target: NodeId) {
        let mut stack: Vec<NodeId> = Vec::new();
        if let NodeKind::Directory { children, .. } = &self.nodes[target].kind {
            stack.extend_from_slice(children);
        }
        while let Some(id) = stack.pop() {
            self.nodes[id].alive = false;
            if let NodeKind::Directory { children, .. } = &self.nodes[id].kind {
                stack.extend_from_slice(children);
            }
        }
        if let NodeKind::Directory { children, .. } = &mut self.nodes[target].kind {
            children.clear();
        }
    }

    /// Recomputes `target`'s size and propagates the change upward to root.
    /// Cheaper than the full bottom-up `compute_sizes` when only one subtree
    /// changed.
    pub fn recompute_sizes_upward(&mut self, target: NodeId) {
        let mut current = Some(target);
        while let Some(id) = current {
            if let NodeKind::Directory { children, .. } = &self.nodes[id].kind {
                let total: u64 = children
                    .iter()
                    .filter(|&&c| self.nodes[c].alive)
                    .map(|&c| self.nodes[c].size)
                    .sum();
                self.nodes[id].size = total;
            }
            current = self.nodes[id].parent;
        }
    }

    /// Replaces all descendants of `target` with `source`'s root subtree
    /// (children, grandchildren, …). Source's root node itself is dropped —
    /// only its content is grafted on.
    ///
    /// Used by the "Refresh this folder" action: scan the folder fresh into
    /// a new FileTree, then graft it back into the live tree to produce the
    /// updated state.
    pub fn graft_under(&mut self, target: NodeId, source: FileTree) {
        self.clear_descendants(target);

        let target_depth = self.nodes[target].depth;
        // (source_node_id, destination_parent_in_self, depth_in_self)
        let mut stack: Vec<(NodeId, NodeId, u16)> = Vec::new();
        if let NodeKind::Directory { children, .. } = &source.nodes[source.root].kind {
            for &c in children {
                stack.push((c, target, target_depth.saturating_add(1)));
            }
        }

        while let Some((src_id, dst_parent, depth)) = stack.pop() {
            let src_node = &source.nodes[src_id];
            if !src_node.alive {
                continue;
            }
            let name_bytes = source.strings.get(src_node.name).to_vec();
            match &src_node.kind {
                NodeKind::File { extension } => {
                    let ext_owned: Option<String> = extension.map(|r| {
                        String::from_utf8_lossy(source.strings.get(r)).into_owned()
                    });
                    let new_id = self.add_file(
                        dst_parent,
                        &name_bytes,
                        src_node.size,
                        ext_owned.as_deref(),
                        src_node.modified,
                        depth,
                    );
                    let _ = new_id;
                }
                NodeKind::Directory { children, .. } => {
                    let new_id = self.add_dir(
                        dst_parent,
                        &name_bytes,
                        depth < 2,
                        src_node.modified,
                        depth,
                    );
                    for &c in children {
                        stack.push((c, new_id, depth.saturating_add(1)));
                    }
                }
            }
        }

        self.recompute_sizes_upward(target);
    }
}
