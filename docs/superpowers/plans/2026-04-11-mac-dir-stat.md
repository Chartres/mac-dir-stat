# mac-dir-stat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a WinDirStat clone for macOS — a fast, beautiful disk space analyzer with treemap visualization, built in Rust with egui.

**Architecture:** Four modules — `scanner` (parallel fs walk → arena tree), `treemap` (squarified layout + coloring), `ui` (egui three-panel layout), `platform` (Finder/Trash/dialogs). The scanner runs on a background thread and streams progress to the UI via crossbeam channels. The treemap algorithm is pure computation, cached and only recomputed on resize/zoom/delete.

**Tech Stack:** Rust, eframe/egui (GUI), jwalk (parallel directory walking), crossbeam-channel (threading), trash (macOS Trash), rfd (native dialogs)

**Spec:** `docs/superpowers/specs/2026-04-11-mac-dir-stat-design.md`

---

## File Map

```
mac-dir-stat/
├── Cargo.toml
├── src/
│   ├── main.rs                # Entry point, launches eframe
│   ├── app.rs                 # Top-level App, owns all state, routes input
│   ├── scanner/
│   │   ├── mod.rs             # Re-exports, ScanProgress enum, scan() fn
│   │   ├── tree.rs            # FileTree, Node, NodeId, NodeKind
│   │   └── walk.rs            # jwalk-based parallel walker, builds FileTree
│   ├── treemap/
│   │   ├── mod.rs             # Re-exports, TreemapRect
│   │   ├── squarify.rs        # Squarified treemap layout algorithm
│   │   └── color.rs           # Extension/depth/age coloring modes
│   ├── ui/
│   │   ├── mod.rs             # Panel orchestration
│   │   ├── theme.rs           # Colors, gradients, spacing constants
│   │   ├── toolbar.rs         # Top bar: scan, refresh, path, color toggle
│   │   ├── dir_tree.rs        # Directory tree panel
│   │   ├── ext_list.rs        # File extension list panel
│   │   ├── treemap_view.rs    # Treemap canvas rendering
│   │   ├── context_menu.rs    # Right-click menu
│   │   └── search.rs          # Cmd+F search/filter bar
│   └── platform/
│       ├── mod.rs             # Re-exports
│       ├── finder.rs          # Reveal in Finder
│       ├── trash.rs           # Move to Trash + tree patching
│       └── dialogs.rs         # Folder picker, volume enumeration
└── tests/
    ├── tree_tests.rs          # FileTree unit tests
    ├── squarify_tests.rs      # Treemap layout tests
    └── color_tests.rs         # Coloring mode tests
```

---

### Task 1: Project Scaffolding + Empty Window

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/app.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "mac-dir-stat"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.31"
egui = "0.31"
jwalk = "0.8"
crossbeam-channel = "0.5"
trash = "5"
rfd = "0.15"

[profile.release]
opt-level = 3
lto = true
```

- [ ] **Step 2: Create src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("mac-dir-stat"),
        ..Default::default()
    };
    eframe::run_native(
        "mac-dir-stat",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
```

- [ ] **Step 3: Create src/app.rs**

```rust
pub struct App;

impl App {
    pub fn new() -> Self {
        Self
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("mac-dir-stat");
            ui.label("Scanning not started yet.");
        });
    }
}
```

- [ ] **Step 4: Build and run**

Run: `cargo run`
Expected: A window appears with "mac-dir-stat" heading and placeholder text. Close it.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: project scaffolding with empty egui window"
```

---

### Task 2: FileTree Data Structure

**Files:**
- Create: `src/scanner/mod.rs`
- Create: `src/scanner/tree.rs`
- Create: `tests/tree_tests.rs`
- Modify: `src/main.rs` (add `mod scanner`)

- [ ] **Step 1: Write failing tests for FileTree**

Create `tests/tree_tests.rs`:

```rust
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
    // Returns Vec<(String, u64, usize)> -> (extension, total_bytes, file_count)
    // Sorted by total_bytes descending
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test tree_tests 2>&1 | head -20`
Expected: Compilation error — module `scanner::tree` doesn't exist yet.

- [ ] **Step 3: Create src/scanner/tree.rs**

```rust
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

    /// Recompute directory sizes bottom-up.
    pub fn compute_sizes(&mut self) {
        // Process nodes in reverse order (children before parents since we append children after parents)
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

    /// Remove a node and subtract its size from all ancestors.
    pub fn remove_node(&mut self, id: NodeId) {
        let size = self.nodes[id].size;
        let parent = self.nodes[id].parent;

        // Mark as dead
        self.nodes[id].alive = false;

        // Remove from parent's children
        if let Some(pid) = parent {
            if let NodeKind::Directory { children, .. } = &mut self.nodes[pid].kind {
                children.retain(|&c| c != id);
            }
        }

        // Subtract size from all ancestors
        let mut current = parent;
        while let Some(pid) = current {
            self.nodes[pid].size = self.nodes[pid].size.saturating_sub(size);
            current = self.nodes[pid].parent;
        }

        // Mark all descendants as dead too
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

    /// Build full path from root to this node.
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
        // Root is "/", join the rest
        let mut path = PathBuf::from(&parts[0]);
        for part in &parts[1..] {
            path.push(part);
        }
        path
    }

    /// Collect extension statistics under a given root.
    /// Returns Vec<(extension, total_bytes, file_count)> sorted by total_bytes desc.
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

    /// Collect all alive leaf files under a node.
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

    /// Iterate direct children of a directory, sorted by size descending.
    pub fn children_sorted(&self, id: NodeId) -> Vec<NodeId> {
        let mut children: Vec<NodeId> = self.node(id).children().to_vec();
        children.retain(|&c| self.nodes[c].alive);
        children.sort_by(|&a, &b| self.nodes[b].size.cmp(&self.nodes[a].size));
        children
    }

    /// Check if a node is alive.
    pub fn is_alive(&self, id: NodeId) -> bool {
        self.nodes[id].alive
    }
}
```

- [ ] **Step 4: Create src/scanner/mod.rs**

```rust
pub mod tree;
```

- [ ] **Step 5: Add module declaration to src/main.rs**

Add `pub mod scanner;` to `src/main.rs` after `mod app;`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod scanner;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("mac-dir-stat"),
        ..Default::default()
    };
    eframe::run_native(
        "mac-dir-stat",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test tree_tests`
Expected: All 5 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/scanner/ tests/tree_tests.rs src/main.rs
git commit -m "feat: FileTree arena data structure with tests"
```

---

### Task 3: Squarified Treemap Algorithm

**Files:**
- Create: `src/treemap/mod.rs`
- Create: `src/treemap/squarify.rs`
- Create: `tests/squarify_tests.rs`
- Modify: `src/main.rs` (add `mod treemap`)

- [ ] **Step 1: Write failing tests for squarify**

Create `tests/squarify_tests.rs`:

```rust
use mac_dir_stat::treemap::squarify::{layout, LayoutItem, Rect};

#[test]
fn test_single_item() {
    let items = vec![LayoutItem { id: 0, size: 100.0 }];
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let rects = layout(&items, &bounds);
    assert_eq!(rects.len(), 1);
    assert!((rects[0].rect.w - 800.0).abs() < 0.01);
    assert!((rects[0].rect.h - 600.0).abs() < 0.01);
}

#[test]
fn test_two_equal_items() {
    let items = vec![
        LayoutItem { id: 0, size: 100.0 },
        LayoutItem { id: 1, size: 100.0 },
    ];
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let rects = layout(&items, &bounds);
    assert_eq!(rects.len(), 2);

    // Total area should equal bounds area
    let total_area: f64 = rects.iter().map(|r| r.rect.w * r.rect.h).sum();
    assert!((total_area - 800.0 * 600.0).abs() < 1.0);

    // Each rect should have roughly equal area
    let area0 = rects[0].rect.w * rects[0].rect.h;
    let area1 = rects[1].rect.w * rects[1].rect.h;
    assert!((area0 - area1).abs() < 1.0);
}

#[test]
fn test_areas_proportional_to_sizes() {
    let items = vec![
        LayoutItem { id: 0, size: 600.0 },
        LayoutItem { id: 1, size: 300.0 },
        LayoutItem { id: 2, size: 100.0 },
    ];
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 1000.0,
    };
    let rects = layout(&items, &bounds);
    assert_eq!(rects.len(), 3);

    let areas: Vec<f64> = rects.iter().map(|r| r.rect.w * r.rect.h).collect();
    let total: f64 = areas.iter().sum();

    // Area ratios should match size ratios (within 1%)
    assert!((areas[0] / total - 0.6).abs() < 0.01);
    assert!((areas[1] / total - 0.3).abs() < 0.01);
    assert!((areas[2] / total - 0.1).abs() < 0.01);
}

#[test]
fn test_no_overlap() {
    let items: Vec<LayoutItem> = (0..20)
        .map(|i| LayoutItem {
            id: i,
            size: (20 - i) as f64 * 100.0,
        })
        .collect();
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let rects = layout(&items, &bounds);

    // Check no two rects overlap (using center points — each center should be inside only its own rect)
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate() {
            if i >= j {
                continue;
            }
            let overlap_x = a.rect.x < b.rect.x + b.rect.w && a.rect.x + a.rect.w > b.rect.x;
            let overlap_y = a.rect.y < b.rect.y + b.rect.h && a.rect.y + a.rect.h > b.rect.y;
            // Allow 0.1px overlap for floating point imprecision
            if overlap_x && overlap_y {
                let overlap_w = (a.rect.x + a.rect.w).min(b.rect.x + b.rect.w)
                    - a.rect.x.max(b.rect.x);
                let overlap_h = (a.rect.y + a.rect.h).min(b.rect.y + b.rect.h)
                    - a.rect.y.max(b.rect.y);
                assert!(
                    overlap_w < 0.1 || overlap_h < 0.1,
                    "Rects {} and {} overlap by {}x{}",
                    i,
                    j,
                    overlap_w,
                    overlap_h
                );
            }
        }
    }
}

#[test]
fn test_all_within_bounds() {
    let items: Vec<LayoutItem> = (0..50)
        .map(|i| LayoutItem {
            id: i,
            size: (50 - i) as f64 * 10.0 + 1.0,
        })
        .collect();
    let bounds = Rect {
        x: 10.0,
        y: 20.0,
        w: 780.0,
        h: 560.0,
    };
    let rects = layout(&items, &bounds);

    for r in &rects {
        assert!(
            r.rect.x >= bounds.x - 0.01,
            "x={} < bounds.x={}",
            r.rect.x,
            bounds.x
        );
        assert!(
            r.rect.y >= bounds.y - 0.01,
            "y={} < bounds.y={}",
            r.rect.y,
            bounds.y
        );
        assert!(
            r.rect.x + r.rect.w <= bounds.x + bounds.w + 0.01,
            "right edge out of bounds"
        );
        assert!(
            r.rect.y + r.rect.h <= bounds.y + bounds.h + 0.01,
            "bottom edge out of bounds"
        );
    }
}

#[test]
fn test_aspect_ratios_reasonable() {
    let items: Vec<LayoutItem> = (0..10)
        .map(|i| LayoutItem {
            id: i,
            size: (10 - i) as f64 * 50.0 + 10.0,
        })
        .collect();
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let rects = layout(&items, &bounds);

    // Squarified algorithm should produce aspect ratios < 5 for all rects
    for r in &rects {
        let aspect = if r.rect.w > r.rect.h {
            r.rect.w / r.rect.h
        } else {
            r.rect.h / r.rect.w
        };
        assert!(
            aspect < 5.0,
            "Bad aspect ratio {} for rect {:?}",
            aspect,
            r
        );
    }
}

#[test]
fn test_empty_input() {
    let items: Vec<LayoutItem> = vec![];
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let rects = layout(&items, &bounds);
    assert!(rects.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test squarify_tests 2>&1 | head -10`
Expected: Compilation error — module doesn't exist.

- [ ] **Step 3: Create src/treemap/squarify.rs**

```rust
/// A simple axis-aligned rectangle.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Input item for layout.
#[derive(Debug, Clone, Copy)]
pub struct LayoutItem {
    pub id: usize,
    pub size: f64,
}

/// Output: an item placed at a specific rect.
#[derive(Debug, Clone)]
pub struct PlacedRect {
    pub id: usize,
    pub rect: Rect,
}

/// Squarified treemap layout.
/// Items should have positive sizes. Returns one PlacedRect per item.
pub fn layout(items: &[LayoutItem], bounds: &Rect) -> Vec<PlacedRect> {
    if items.is_empty() || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return vec![];
    }

    let total_size: f64 = items.iter().map(|i| i.size).sum();
    if total_size <= 0.0 {
        return vec![];
    }

    // Sort by size descending
    let mut sorted: Vec<LayoutItem> = items.to_vec();
    sorted.sort_by(|a, b| b.size.partial_cmp(&a.size).unwrap());

    let mut result = Vec::with_capacity(sorted.len());
    let mut remaining = *bounds;
    let area = bounds.w * bounds.h;

    // Scale items so their sizes sum to the total pixel area
    let scale = area / total_size;
    let scaled: Vec<LayoutItem> = sorted
        .iter()
        .map(|item| LayoutItem {
            id: item.id,
            size: item.size * scale,
        })
        .collect();

    squarify_recursive(&scaled, &mut remaining, &mut result);
    result
}

fn squarify_recursive(items: &[LayoutItem], bounds: &mut Rect, result: &mut Vec<PlacedRect>) {
    if items.is_empty() || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return;
    }

    if items.len() == 1 {
        result.push(PlacedRect {
            id: items[0].id,
            rect: *bounds,
        });
        return;
    }

    // Find the best split: add items to a row along the shorter side,
    // stopping when the worst aspect ratio starts increasing.
    let short_side = bounds.w.min(bounds.h);

    let mut row: Vec<&LayoutItem> = vec![];
    let mut row_area = 0.0;
    let mut best_worst_aspect = f64::MAX;
    let mut best_row_len = 1;

    for item in items {
        row.push(item);
        row_area += item.size;

        let worst = worst_aspect_ratio(&row, row_area, short_side);
        if worst <= best_worst_aspect {
            best_worst_aspect = worst;
            best_row_len = row.len();
        } else {
            break;
        }
    }

    // Layout the chosen row
    let row_items = &items[..best_row_len];
    let row_total: f64 = row_items.iter().map(|i| i.size).sum();

    layout_row(row_items, row_total, bounds, result);

    // Recurse on remaining items with the reduced bounds
    let remaining = &items[best_row_len..];
    squarify_recursive(remaining, bounds, result);
}

fn worst_aspect_ratio(row: &[&LayoutItem], row_area: f64, short_side: f64) -> f64 {
    if short_side <= 0.0 || row_area <= 0.0 {
        return f64::MAX;
    }
    let s2 = short_side * short_side;
    let mut worst = 0.0_f64;
    for item in row {
        let r = if item.size > 0.0 {
            let a = (s2 * item.size) / (row_area * row_area);
            let b = (row_area * row_area) / (s2 * item.size);
            a.max(b)
        } else {
            f64::MAX
        };
        worst = worst.max(r);
    }
    worst
}

fn layout_row(
    items: &[LayoutItem],
    row_total: f64,
    bounds: &mut Rect,
    result: &mut Vec<PlacedRect>,
) {
    if row_total <= 0.0 {
        return;
    }

    let horizontal = bounds.w >= bounds.h;

    if horizontal {
        // Row occupies a vertical strip on the left
        let row_width = row_total / bounds.h;
        let mut y = bounds.y;

        for item in items {
            let h = if row_width > 0.0 {
                item.size / row_width
            } else {
                0.0
            };
            result.push(PlacedRect {
                id: item.id,
                rect: Rect {
                    x: bounds.x,
                    y,
                    w: row_width,
                    h,
                },
            });
            y += h;
        }

        // Shrink bounds: remove the left strip
        bounds.x += row_width;
        bounds.w -= row_width;
    } else {
        // Row occupies a horizontal strip on the top
        let row_height = row_total / bounds.w;
        let mut x = bounds.x;

        for item in items {
            let w = if row_height > 0.0 {
                item.size / row_height
            } else {
                0.0
            };
            result.push(PlacedRect {
                id: item.id,
                rect: Rect {
                    x,
                    y: bounds.y,
                    w,
                    h: row_height,
                },
            });
            x += w;
        }

        // Shrink bounds: remove the top strip
        bounds.y += row_height;
        bounds.h -= row_height;
    }
}
```

- [ ] **Step 4: Create src/treemap/mod.rs**

```rust
pub mod squarify;
pub mod color;
```

Create a placeholder `src/treemap/color.rs`:

```rust
// Coloring modes — implemented in Task 4.
```

- [ ] **Step 5: Add module to src/main.rs**

Add `pub mod treemap;` after `pub mod scanner;` in `src/main.rs`.

- [ ] **Step 6: Run tests**

Run: `cargo test --test squarify_tests`
Expected: All 7 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/treemap/ tests/squarify_tests.rs src/main.rs
git commit -m "feat: squarified treemap layout algorithm with tests"
```

---

### Task 4: Treemap Coloring

**Files:**
- Modify: `src/treemap/color.rs`
- Create: `tests/color_tests.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/color_tests.rs`:

```rust
use mac_dir_stat::treemap::color::{extension_color, depth_color, age_color, ColorMode, GradientPair};
use std::time::{SystemTime, Duration};

#[test]
fn test_extension_color_deterministic() {
    let c1 = extension_color("rs");
    let c2 = extension_color("rs");
    assert_eq!(c1.0, c2.0);
    assert_eq!(c1.1, c2.1);
}

#[test]
fn test_extension_color_different_extensions() {
    let rs = extension_color("rs");
    let txt = extension_color("txt");
    // Different extensions should usually produce different colors
    // (hash collisions possible but unlikely for these two)
    assert_ne!(rs.0, txt.0);
}

#[test]
fn test_extension_color_unknown() {
    let c = extension_color("");
    // Empty extension should return the slate/gray pair
    assert_eq!(c.0, [71, 85, 105, 255]);
    assert_eq!(c.1, [148, 163, 184, 255]);
}

#[test]
fn test_depth_color_varies_by_depth() {
    let c0 = depth_color(0, 0);
    let c1 = depth_color(0, 1);
    let c2 = depth_color(0, 2);
    // Same hue index, different depth → different lightness
    assert_ne!(c0, c1);
    assert_ne!(c1, c2);
}

#[test]
fn test_depth_color_varies_by_hue_index() {
    let c0 = depth_color(0, 1);
    let c1 = depth_color(1, 1);
    // Different hue index, same depth → different colors
    assert_ne!(c0, c1);
}

#[test]
fn test_age_color_recent_is_warm() {
    let now = SystemTime::now();
    let c = age_color(now);
    // Recent = warm (high red channel)
    assert!(c.0[0] > 150, "Recent file should be warm-toned, got r={}", c.0[0]);
}

#[test]
fn test_age_color_old_is_cool() {
    let old = SystemTime::now() - Duration::from_secs(365 * 24 * 3600 * 2); // 2 years ago
    let c = age_color(old);
    // Old = cool (high blue channel relative to red)
    assert!(c.0[2] > c.0[0], "Old file should be cool-toned");
}

#[test]
fn test_palette_has_enough_colors() {
    // Ensure we have at least 10 distinct gradient pairs
    let exts = ["rs", "txt", "mov", "mp4", "dmg", "zip", "jpg", "png", "app", "pdf"];
    let colors: Vec<GradientPair> = exts.iter().map(|e| extension_color(e)).collect();
    let unique: std::collections::HashSet<[u8; 8]> = colors
        .iter()
        .map(|c| {
            let mut key = [0u8; 8];
            key[..4].copy_from_slice(&c.0);
            key[4..].copy_from_slice(&c.1);
            key
        })
        .collect();
    assert!(unique.len() >= 6, "Expected at least 6 unique colors, got {}", unique.len());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test color_tests 2>&1 | head -10`
Expected: Compilation error.

- [ ] **Step 3: Implement src/treemap/color.rs**

```rust
use std::time::SystemTime;

/// RGBA color as [r, g, b, a].
pub type Color = [u8; 4];

/// A gradient pair (start_color, end_color) for rendering blocks.
pub type GradientPair = (Color, Color);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Extension,
    Depth,
    Age,
}

/// Curated vibrant gradient pairs.
const PALETTE: &[GradientPair] = &[
    ([124, 58, 237, 255], [167, 139, 250, 255]),   // Purple
    ([37, 99, 235, 255], [96, 165, 250, 255]),      // Blue
    ([8, 145, 178, 255], [103, 232, 249, 255]),     // Cyan
    ([220, 38, 38, 255], [248, 113, 113, 255]),     // Red
    ([234, 88, 12, 255], [251, 146, 60, 255]),      // Orange
    ([22, 163, 74, 255], [74, 222, 128, 255]),      // Green
    ([202, 138, 4, 255], [250, 204, 21, 255]),      // Yellow
    ([147, 51, 234, 255], [192, 132, 252, 255]),    // Magenta
    ([219, 39, 119, 255], [244, 114, 182, 255]),    // Pink
    ([13, 148, 136, 255], [94, 234, 212, 255]),     // Teal
    ([101, 163, 13, 255], [163, 230, 53, 255]),     // Lime
    ([79, 70, 229, 255], [129, 140, 248, 255]),     // Indigo
    ([217, 70, 239, 255], [232, 121, 249, 255]),    // Fuchsia
    ([245, 158, 11, 255], [252, 211, 77, 255]),     // Amber
    ([6, 182, 212, 255], [34, 211, 238, 255]),      // Sky
    ([16, 185, 129, 255], [52, 211, 153, 255]),     // Emerald
    ([239, 68, 68, 255], [252, 165, 165, 255]),     // Rose
    ([168, 85, 247, 255], [196, 181, 253, 255]),    // Violet
    ([14, 165, 233, 255], [125, 211, 252, 255]),    // Light Blue
    ([132, 204, 22, 255], [190, 242, 100, 255]),    // Yellow-Green
];

const SLATE_PAIR: GradientPair = ([71, 85, 105, 255], [148, 163, 184, 255]);

/// Get a gradient pair for a file extension. Deterministic hash-based mapping.
pub fn extension_color(ext: &str) -> GradientPair {
    if ext.is_empty() {
        return SLATE_PAIR;
    }
    let hash = simple_hash(ext);
    PALETTE[hash % PALETTE.len()]
}

/// Get a gradient pair based on directory depth. `hue_index` identifies the top-level dir branch.
pub fn depth_color(hue_index: usize, depth: u16) -> GradientPair {
    let base = &PALETTE[hue_index % PALETTE.len()];
    let factor = 1.0 - (depth as f32 * 0.12).min(0.6);
    let lighten = |c: Color| -> Color {
        [
            (c[0] as f32 * factor) as u8,
            (c[1] as f32 * factor) as u8,
            (c[2] as f32 * factor) as u8,
            c[3],
        ]
    };
    (lighten(base.0), lighten(base.1))
}

/// Get a gradient pair based on file age. Recent = warm, old = cool.
pub fn age_color(modified: SystemTime) -> GradientPair {
    let age_secs = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        .as_secs();

    let one_year = 365 * 24 * 3600;
    let t = (age_secs as f64 / one_year as f64).min(1.0);

    // Warm (magenta/pink) to cool (indigo/navy)
    let warm_start: [f64; 3] = [219.0, 39.0, 119.0];
    let warm_end: [f64; 3] = [244.0, 114.0, 182.0];
    let cool_start: [f64; 3] = [49.0, 46.0, 129.0];
    let cool_end: [f64; 3] = [79.0, 70.0, 229.0];

    let lerp = |a: f64, b: f64| -> u8 { (a + (b - a) * t).clamp(0.0, 255.0) as u8 };

    let start = [
        lerp(warm_start[0], cool_start[0]),
        lerp(warm_start[1], cool_start[1]),
        lerp(warm_start[2], cool_start[2]),
        255,
    ];
    let end = [
        lerp(warm_end[0], cool_end[0]),
        lerp(warm_end[1], cool_end[1]),
        lerp(warm_end[2], cool_end[2]),
        255,
    ];

    (start, end)
}

fn simple_hash(s: &str) -> usize {
    let mut h: usize = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as usize);
    }
    h
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test color_tests`
Expected: All 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/treemap/color.rs tests/color_tests.rs
git commit -m "feat: treemap coloring modes (extension, depth, age)"
```

---

### Task 5: Scanner (Background Walking)

**Files:**
- Create: `src/scanner/walk.rs`
- Modify: `src/scanner/mod.rs`

- [ ] **Step 1: Implement ScanProgress and scan() in src/scanner/mod.rs**

```rust
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
    },
    Done(FileTree),
    Error(String),
}

/// Launch a background scan. Returns immediately.
/// Progress is sent over the channel.
pub fn scan(root: PathBuf, tx: Sender<ScanProgress>) {
    std::thread::spawn(move || {
        walk::walk_directory(root, tx);
    });
}
```

- [ ] **Step 2: Implement src/scanner/walk.rs**

```rust
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
    // Set root name to the scanned path
    tree.node_mut(tree.root()).name = OsString::from(root.to_string_lossy().as_ref());

    // Map from directory path → NodeId so we can find parents
    let mut dir_map: HashMap<PathBuf, NodeId> = HashMap::new();
    dir_map.insert(root.clone(), tree.root());

    let mut file_count: usize = 0;
    let mut dir_count: usize = 0;
    let mut byte_count: u64 = 0;
    let mut progress_counter: usize = 0;

    for entry in WalkDir::new(&root).skip_hidden(false).sort(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // Permission denied or other error — skip
        };

        let path = entry.path();

        // Skip the root itself
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
                // Parent wasn't recorded (e.g. permission denied on a higher dir).
                // Try to find the closest known ancestor.
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

        // Send progress every 500 items
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
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 4: Quick smoke test**

Run: `cargo test --test tree_tests && cargo test --test squarify_tests && cargo test --test color_tests`
Expected: All existing tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/scanner/
git commit -m "feat: background filesystem scanner with jwalk"
```

---

### Task 6: Theme + App State

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/theme.rs`
- Modify: `src/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/ui/theme.rs**

```rust
use egui::Color32;

// Background colors
pub const BG_DARK: Color32 = Color32::from_rgb(14, 12, 24);
pub const BG_PANEL: Color32 = Color32::from_rgb(22, 18, 37);
pub const BG_SELECTION: Color32 = Color32::from_rgb(26, 23, 48);
pub const BG_HOVER: Color32 = Color32::from_rgb(30, 26, 55);

// Border
pub const BORDER: Color32 = Color32::from_rgb(42, 42, 74);

// Text
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(224, 224, 240);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(139, 139, 187);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(102, 102, 102);

// Accent
pub const ACCENT: Color32 = Color32::from_rgb(124, 58, 237);
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(167, 139, 250);

// Section headers
pub const SECTION_HEADER: Color32 = Color32::from_rgb(107, 91, 149);

// Size bar background
pub const BAR_BG: Color32 = Color32::from_rgb(26, 23, 48);

// Button
pub const BUTTON_BG: Color32 = Color32::from_rgb(42, 42, 74);
pub const BUTTON_ACTIVE_BORDER: Color32 = Color32::from_rgb(124, 58, 237);

/// Apply the dark & vibrant theme to an egui context.
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;

    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_DARK;

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT_PRIMARY;

    visuals.widgets.inactive.bg_fill = BUTTON_BG;
    visuals.widgets.inactive.fg_stroke.color = TEXT_SECONDARY;

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;

    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;

    visuals.selection.bg_fill = BG_SELECTION;
    visuals.selection.stroke.color = ACCENT_LIGHT;

    visuals.extreme_bg_color = BG_DARK;

    ctx.set_style(style);
}

/// Format bytes as human-readable string.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
```

- [ ] **Step 2: Create src/ui/mod.rs**

```rust
pub mod theme;
pub mod toolbar;
pub mod dir_tree;
pub mod ext_list;
pub mod treemap_view;
pub mod context_menu;
pub mod search;
```

Create empty placeholder files for each submodule (we'll implement them in subsequent tasks):

`src/ui/toolbar.rs`:
```rust
use egui::Ui;
use crate::app::AppState;

pub fn show(_ui: &mut Ui, _state: &mut AppState) {}
```

`src/ui/dir_tree.rs`:
```rust
use egui::Ui;
use crate::app::AppState;

pub fn show(_ui: &mut Ui, _state: &mut AppState) {}
```

`src/ui/ext_list.rs`:
```rust
use egui::Ui;
use crate::app::AppState;

pub fn show(_ui: &mut Ui, _state: &mut AppState) {}
```

`src/ui/treemap_view.rs`:
```rust
use egui::Ui;
use crate::app::AppState;

pub fn show(_ui: &mut Ui, _state: &mut AppState) {}
```

`src/ui/context_menu.rs`:
```rust
use crate::app::AppState;
use crate::scanner::tree::NodeId;

pub fn show(_ui: &mut egui::Ui, _state: &mut AppState, _node: NodeId) {}
```

`src/ui/search.rs`:
```rust
use egui::Ui;
use crate::app::AppState;

pub fn show(_ui: &mut Ui, _state: &mut AppState) {}
```

- [ ] **Step 3: Rewrite src/app.rs with full state management**

```rust
use crate::scanner::tree::{FileTree, NodeId};
use crate::scanner::ScanProgress;
use crate::treemap::color::ColorMode;
use crate::treemap::squarify::PlacedRect;
use crate::ui;
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::time::Instant;

pub struct App {
    pub state: AppState,
    theme_applied: bool,
}

pub struct AppState {
    // Scan state
    pub tree: Option<FileTree>,
    pub scan_root: PathBuf,
    pub scan_receiver: Option<Receiver<ScanProgress>>,
    pub scan_progress: ScanProgressInfo,
    pub scan_start: Option<Instant>,
    pub scan_duration_secs: f32,

    // Extension stats: (extension, total_bytes, file_count)
    pub extension_stats: Vec<(String, u64, usize)>,

    // Treemap
    pub colored_rects: Vec<crate::treemap::ColoredRect>,
    pub treemap_dirty: bool,
    pub color_mode: ColorMode,
    pub view_root: Option<NodeId>,
    pub zoom_stack: Vec<NodeId>,

    // Selection
    pub selected_node: Option<NodeId>,
    pub hovered_node: Option<NodeId>,
    pub selected_extension: Option<String>,

    // Search
    pub search_active: bool,
    pub search_query: String,

    // UI
    pub pending_action: Option<PendingAction>,
    pub request_rescan: bool,
    pub last_screen_size: egui::Vec2,
}

pub struct ScanProgressInfo {
    pub files: usize,
    pub dirs: usize,
    pub bytes: u64,
    pub scanning: bool,
}

pub enum PendingAction {
    RevealInFinder(PathBuf),
    MoveToTrash(NodeId),
    ConfirmTrash(NodeId, String, u64), // node, name, size
}

impl App {
    pub fn new() -> Self {
        let scan_root = PathBuf::from("/");
        App {
            state: AppState {
                tree: None,
                scan_root: scan_root.clone(),
                scan_receiver: None,
                scan_progress: ScanProgressInfo {
                    files: 0,
                    dirs: 0,
                    bytes: 0,
                    scanning: false,
                },
                scan_start: None,
                scan_duration_secs: 0.0,
                extension_stats: vec![],
                colored_rects: vec![],
                treemap_dirty: true,
                color_mode: ColorMode::Extension,
                view_root: None,
                zoom_stack: vec![],
                selected_node: None,
                hovered_node: None,
                selected_extension: None,
                search_active: false,
                search_query: String::new(),
                pending_action: None,
                request_rescan: false,
                last_screen_size: egui::Vec2::ZERO,
            },
            theme_applied: false,
        }
    }

    pub fn start_scan(&mut self) {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.state.scan_receiver = Some(rx);
        self.state.scan_progress = ScanProgressInfo {
            files: 0,
            dirs: 0,
            bytes: 0,
            scanning: true,
        };
        self.state.scan_start = Some(Instant::now());
        self.state.tree = None;
        self.state.colored_rects.clear();
        self.state.extension_stats.clear();
        self.state.selected_node = None;
        self.state.hovered_node = None;
        self.state.view_root = None;
        self.state.zoom_stack.clear();
        crate::scanner::scan(self.state.scan_root.clone(), tx);
    }

    fn poll_scan(&mut self) {
        if let Some(rx) = &self.state.scan_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanProgress::Counting { files, dirs, bytes } => {
                        self.state.scan_progress.files = files;
                        self.state.scan_progress.dirs = dirs;
                        self.state.scan_progress.bytes = bytes;
                    }
                    ScanProgress::Done(tree) => {
                        if let Some(start) = self.state.scan_start {
                            self.state.scan_duration_secs = start.elapsed().as_secs_f32();
                        }
                        let root = tree.root();
                        self.state.extension_stats = tree.collect_extensions(root);
                        self.state.view_root = Some(root);
                        self.state.tree = Some(tree);
                        self.state.scan_progress.scanning = false;
                        self.state.treemap_dirty = true;
                    }
                    ScanProgress::Error(msg) => {
                        eprintln!("Scan error: {}", msg);
                        self.state.scan_progress.scanning = false;
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            ui::theme::apply_theme(ctx);
            self.theme_applied = true;
            // Auto-start scan on launch
            self.start_scan();
        }

        self.poll_scan();

        // Request repaint while scanning for progress updates
        if self.state.scan_progress.scanning {
            ctx.request_repaint();
        }

        // Top toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui::toolbar::show(ui, &mut self.state);
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(tree) = &self.state.tree {
                    let root = tree.root();
                    let file_count = tree.collect_files(root).len();
                    ui.label(
                        egui::RichText::new(format!(
                            "{} files  •  {} dirs  •  {:.1}s",
                            file_count,
                            self.state.scan_progress.dirs,
                            self.state.scan_duration_secs,
                        ))
                        .color(ui::theme::TEXT_MUTED)
                        .size(11.0),
                    );
                } else if self.state.scan_progress.scanning {
                    ui.label(
                        egui::RichText::new(format!(
                            "Scanning... {} files  •  {} dirs  •  {}",
                            self.state.scan_progress.files,
                            self.state.scan_progress.dirs,
                            ui::theme::format_size(self.state.scan_progress.bytes),
                        ))
                        .color(ui::theme::ACCENT_LIGHT)
                        .size(11.0),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let (Some(tree), Some(hovered)) =
                        (&self.state.tree, self.state.hovered_node)
                    {
                        if tree.is_alive(hovered) {
                            let path = tree.full_path(hovered);
                            let node = tree.node(hovered);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}  •  {}",
                                    path.display(),
                                    ui::theme::format_size(node.size),
                                ))
                                .color(ui::theme::ACCENT_LIGHT)
                                .size(11.0),
                            );
                        }
                    }
                });
            });
        });

        // Left panel: directory tree
        egui::SidePanel::left("dir_tree")
            .default_width(300.0)
            .min_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui::dir_tree::show(ui, &mut self.state);
            });

        // Right panel: extension list
        egui::SidePanel::right("ext_list")
            .default_width(280.0)
            .min_width(180.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui::ext_list::show(ui, &mut self.state);
            });

        // Central panel: treemap
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.search_active {
                ui::search::show(ui, &mut self.state);
            }
            ui::treemap_view::show(ui, &mut self.state);
        });
    }
}
```

- [ ] **Step 4: Update src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod scanner;
pub mod treemap;
pub mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("mac-dir-stat"),
        ..Default::default()
    };
    eframe::run_native(
        "mac-dir-stat",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
```

- [ ] **Step 5: Build and run**

Run: `cargo run`
Expected: Dark-themed window appears, auto-starts scanning `/`, shows scanning progress in status bar. Side panels and center are empty. Close it.

- [ ] **Step 6: Commit**

```bash
git add src/
git commit -m "feat: theme, app state, panel layout shell with auto-scan"
```

---

### Task 7: Toolbar

**Files:**
- Modify: `src/ui/toolbar.rs`
- Modify: `src/app.rs` (add helper methods)

- [ ] **Step 1: Implement src/ui/toolbar.rs**

```rust
use crate::app::AppState;
use crate::treemap::color::ColorMode;
use crate::ui::theme;
use egui::Ui;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        // Scan button
        let scan_btn = ui.add(
            egui::Button::new(
                egui::RichText::new("Scan Directory...")
                    .color(egui::Color32::WHITE)
                    .size(12.0),
            )
            .fill(theme::ACCENT),
        );
        if scan_btn.clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select directory to scan")
                .set_directory(&state.scan_root)
                .pick_folder()
            {
                state.scan_root = path;
                state.request_rescan = true;
            }
        }

        // Refresh button
        let refresh_btn = ui.add(
            egui::Button::new(
                egui::RichText::new("⟳ Refresh")
                    .color(theme::ACCENT_LIGHT)
                    .size(12.0),
            )
            .fill(theme::BUTTON_BG),
        );
        if refresh_btn.clicked() {
            state.request_rescan = true;
        }

        // Path display
        ui.add_space(8.0);
        let path_text = if state.scan_progress.scanning {
            format!("Scanning {}...", state.scan_root.display())
        } else if let Some(tree) = &state.tree {
            format!(
                "{} — {}",
                state.scan_root.display(),
                theme::format_size(tree.node(tree.root()).size),
            )
        } else {
            format!("{}", state.scan_root.display())
        };
        ui.label(
            egui::RichText::new(path_text)
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );

        // Color mode toggle (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (mode, label) in [
                (ColorMode::Age, "Age"),
                (ColorMode::Depth, "Depth"),
                (ColorMode::Extension, "Extension"),
            ] {
                let is_active = state.color_mode == mode;
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(label)
                            .color(if is_active {
                                theme::ACCENT_LIGHT
                            } else {
                                theme::TEXT_SECONDARY
                            })
                            .size(11.0),
                    )
                    .fill(if is_active {
                        theme::BUTTON_BG
                    } else {
                        theme::BG_PANEL
                    })
                    .stroke(if is_active {
                        egui::Stroke::new(1.0, theme::BUTTON_ACTIVE_BORDER)
                    } else {
                        egui::Stroke::NONE
                    }),
                );
                if btn.clicked() && !is_active {
                    state.color_mode = mode;
                    state.treemap_dirty = true;
                }
            }
        });
    });
}
```

- [ ] **Step 2: Wire up `request_rescan` in App::update()**

In `App::update()`, add this right after `self.poll_scan()`:

```rust
if self.state.request_rescan {
    self.state.request_rescan = false;
    self.start_scan();
}
```

- [ ] **Step 3: Build and run**

Run: `cargo run`
Expected: Toolbar shows at top with Scan button, Refresh, path display, and color mode toggle. Clicking "Scan Directory..." opens a native folder picker. Color mode buttons toggle visually.

- [ ] **Step 4: Commit**

```bash
git add src/ui/toolbar.rs src/app.rs
git commit -m "feat: toolbar with scan, refresh, path display, color mode toggle"
```

---

### Task 8: Directory Tree Panel

**Files:**
- Modify: `src/ui/dir_tree.rs`

- [ ] **Step 1: Implement src/ui/dir_tree.rs**

```rust
use crate::app::AppState;
use crate::scanner::tree::{NodeId, NodeKind};
use crate::ui::theme;
use egui::{Ui, Vec2};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("DIRECTORY TREE")
                .color(theme::SECTION_HEADER)
                .size(9.0)
                .strong(),
        );
        ui.add_space(4.0);

        let tree = match &state.tree {
            Some(t) => t,
            None => {
                if state.scan_progress.scanning {
                    ui.spinner();
                    ui.label(
                        egui::RichText::new("Scanning...")
                            .color(theme::TEXT_MUTED),
                    );
                }
                return;
            }
        };

        let root = tree.root();
        let root_size = tree.node(root).size;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                show_node(ui, state, root, root_size);
            });
    });
}

fn show_node(ui: &mut Ui, state: &mut AppState, id: NodeId, root_size: u64) {
    let tree = match &state.tree {
        Some(t) => t,
        None => return,
    };

    let node = tree.node(id);
    if !tree.is_alive(id) {
        return;
    }

    // Only show directories in the tree
    let (children, expanded) = match &node.kind {
        NodeKind::Directory { children, expanded } => (children.clone(), *expanded),
        NodeKind::File { .. } => return,
    };

    let is_selected = state.selected_node == Some(id);
    let name = node.name.to_string_lossy().to_string();
    let size = node.size;
    let depth = node.depth;

    // Indent
    let indent = depth as f32 * 16.0;

    let response = ui.horizontal(|ui| {
        ui.add_space(indent);

        // Background for selected row
        let rect = ui.available_rect_before_wrap();
        if is_selected {
            ui.painter().rect_filled(
                rect,
                4.0,
                theme::BG_SELECTION,
            );
            // Left accent border
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
                0.0,
                theme::ACCENT_LIGHT,
            );
        }

        // Expand arrow
        let has_children = !children.is_empty();
        if has_children {
            let arrow = if expanded { "▼" } else { "▶" };
            let arrow_resp = ui.add(
                egui::Label::new(
                    egui::RichText::new(arrow)
                        .color(theme::TEXT_MUTED)
                        .size(10.0),
                )
                .sense(egui::Sense::click()),
            );
            if arrow_resp.clicked() {
                if let Some(tree) = &mut state.tree {
                    if let NodeKind::Directory {
                        expanded: ref mut exp,
                        ..
                    } = tree.node_mut(id).kind
                    {
                        *exp = !*exp;
                    }
                }
            }
        } else {
            ui.add_space(14.0);
        }

        // Folder icon
        ui.label(
            egui::RichText::new("📁")
                .color(theme::ACCENT_LIGHT)
                .size(12.0),
        );

        // Name
        let name_color = if is_selected {
            theme::ACCENT_LIGHT
        } else {
            theme::TEXT_PRIMARY
        };
        ui.label(
            egui::RichText::new(&name)
                .color(name_color)
                .size(12.0),
        );

        // Size (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Size bar
            if root_size > 0 {
                let fraction = size as f32 / root_size as f32;
                let bar_width = 50.0;
                let (bar_rect, _) = ui.allocate_exact_size(
                    Vec2::new(bar_width, 4.0),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .rect_filled(bar_rect, 2.0, theme::BAR_BG);
                let filled = egui::Rect::from_min_size(
                    bar_rect.left_top(),
                    Vec2::new(bar_width * fraction, 4.0),
                );
                ui.painter().rect_filled(filled, 2.0, theme::ACCENT);
            }

            // Size text
            ui.label(
                egui::RichText::new(theme::format_size(size))
                    .color(theme::TEXT_MUTED)
                    .size(10.0),
            );
        });
    });

    // Click handling on the row
    let row_response = response.response.interact(egui::Sense::click());
    if row_response.clicked() {
        state.selected_node = Some(id);
        state.selected_extension = None;
    }
    if row_response.double_clicked() {
        // Zoom treemap into this directory
        state.view_root = Some(id);
        state.zoom_stack.push(id);
        state.treemap_dirty = true;
    }

    // Show children if expanded
    if expanded {
        let tree = match &state.tree {
            Some(t) => t,
            None => return,
        };
        let mut sorted_children = children;
        sorted_children.sort_by(|&a, &b| {
            tree.node(b).size.cmp(&tree.node(a).size)
        });
        for child_id in sorted_children {
            show_node(ui, state, child_id, root_size);
        }
    }
}
```

- [ ] **Step 2: Build and run**

Run: `cargo run`
Expected: Left panel shows expanding/collapsing directory tree with folder icons, sizes, and size bars. Clicking a row selects it. Double-clicking zooms (though treemap isn't rendered yet).

- [ ] **Step 3: Commit**

```bash
git add src/ui/dir_tree.rs
git commit -m "feat: directory tree panel with expand/collapse and size bars"
```

---

### Task 9: Extension List Panel

**Files:**
- Modify: `src/ui/ext_list.rs`

- [ ] **Step 1: Implement src/ui/ext_list.rs**

```rust
use crate::app::AppState;
use crate::treemap::color::{extension_color, ColorMode};
use crate::ui::theme;
use egui::{Ui, Vec2, Color32};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("FILE TYPES")
                .color(theme::SECTION_HEADER)
                .size(9.0)
                .strong(),
        );
        ui.add_space(4.0);

        if state.tree.is_none() {
            if state.scan_progress.scanning {
                ui.spinner();
            }
            return;
        }

        let total_size: u64 = state.extension_stats.iter().map(|(_, b, _)| *b).sum();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let max_to_show = 50;
                let stats = &state.extension_stats;
                let show_count = stats.len().min(max_to_show);

                for (ext, bytes, count) in &stats[..show_count] {
                    let is_selected = state.selected_extension.as_deref() == Some(ext.as_str());
                    let colors = extension_color(if ext.is_empty() { "" } else { ext });

                    let response = ui.horizontal(|ui| {
                        // Color swatch
                        let (swatch_rect, _) = ui.allocate_exact_size(
                            Vec2::new(12.0, 12.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            swatch_rect,
                            3.0,
                            Color32::from_rgba_premultiplied(
                                colors.0[0], colors.0[1], colors.0[2], colors.0[3],
                            ),
                        );

                        // Extension name
                        let ext_display = if ext.is_empty() {
                            "(no ext)".to_string()
                        } else {
                            format!(".{}", ext)
                        };
                        let name_color = if is_selected {
                            theme::ACCENT_LIGHT
                        } else {
                            theme::TEXT_PRIMARY
                        };
                        ui.label(
                            egui::RichText::new(&ext_display)
                                .color(name_color)
                                .size(12.0),
                        );

                        // Right side: bar + size + percentage
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                // Percentage
                                let pct = if total_size > 0 {
                                    *bytes as f64 / total_size as f64 * 100.0
                                } else {
                                    0.0
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", pct))
                                        .color(theme::TEXT_MUTED)
                                        .size(10.0),
                                );

                                // Size
                                ui.label(
                                    egui::RichText::new(theme::format_size(*bytes))
                                        .color(theme::TEXT_SECONDARY)
                                        .size(10.0),
                                );

                                // Bar
                                if total_size > 0 {
                                    let fraction = *bytes as f32 / total_size as f32;
                                    let bar_width = 50.0;
                                    let (bar_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(bar_width, 5.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter()
                                        .rect_filled(bar_rect, 3.0, theme::BAR_BG);
                                    let filled = egui::Rect::from_min_size(
                                        bar_rect.left_top(),
                                        Vec2::new(bar_width * fraction, 5.0),
                                    );
                                    ui.painter().rect_filled(
                                        filled,
                                        3.0,
                                        Color32::from_rgba_premultiplied(
                                            colors.0[0],
                                            colors.0[1],
                                            colors.0[2],
                                            colors.0[3],
                                        ),
                                    );
                                }
                            },
                        );
                    });

                    // Click to select extension
                    if response.response.interact(egui::Sense::click()).clicked() {
                        if is_selected {
                            state.selected_extension = None;
                        } else {
                            state.selected_extension = Some(ext.clone());
                            state.selected_node = None;
                        }
                    }
                }

                // Footer
                if stats.len() > max_to_show {
                    let remaining: u64 = stats[max_to_show..].iter().map(|(_, b, _)| *b).sum();
                    let remaining_count = stats.len() - max_to_show;
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "+ {} more types ({})",
                            remaining_count,
                            theme::format_size(remaining),
                        ))
                        .color(theme::TEXT_MUTED)
                        .size(10.0)
                        .italics(),
                    );
                }
            });
    });
}
```

- [ ] **Step 2: Build and run**

Run: `cargo run`
Expected: Right panel shows file extensions sorted by size with color swatches, bars, sizes, and percentages. Clicking an extension toggles selection.

- [ ] **Step 3: Commit**

```bash
git add src/ui/ext_list.rs
git commit -m "feat: extension list panel with color swatches and size bars"
```

---

### Task 10: Treemap Rendering

**Files:**
- Modify: `src/ui/treemap_view.rs`
- Modify: `src/treemap/mod.rs`

This is the core visual feature. The treemap view renders all files as colored rectangles using egui's `Painter`.

- [ ] **Step 1: Add hierarchical layout helper to src/treemap/mod.rs**

```rust
pub mod color;
pub mod squarify;

use crate::scanner::tree::{FileTree, NodeId, NodeKind};
use color::{age_color, depth_color, extension_color, Color, ColorMode};
use squarify::{layout, LayoutItem, PlacedRect, Rect};

/// A treemap rectangle with color info, ready for rendering.
#[derive(Debug, Clone)]
pub struct ColoredRect {
    pub node_id: NodeId,
    pub rect: Rect,
    pub color_start: Color,
    pub color_end: Color,
}

/// Compute the full treemap layout with colors for all files under `view_root`.
pub fn compute_treemap(
    tree: &FileTree,
    view_root: NodeId,
    bounds: Rect,
    color_mode: ColorMode,
) -> Vec<ColoredRect> {
    let mut result = Vec::new();
    layout_recursive(tree, view_root, &bounds, color_mode, 0, &mut result);
    result
}

fn layout_recursive(
    tree: &FileTree,
    node_id: NodeId,
    bounds: &Rect,
    color_mode: ColorMode,
    hue_index: usize,
    result: &mut Vec<ColoredRect>,
) {
    if bounds.w < 1.0 || bounds.h < 1.0 {
        return;
    }

    let node = tree.node(node_id);
    if !tree.is_alive(node_id) {
        return;
    }

    match &node.kind {
        NodeKind::File { extension } => {
            let (cs, ce) = match color_mode {
                ColorMode::Extension => {
                    extension_color(extension.as_deref().unwrap_or(""))
                }
                ColorMode::Depth => depth_color(hue_index, node.depth),
                ColorMode::Age => age_color(node.modified),
            };
            result.push(ColoredRect {
                node_id,
                rect: *bounds,
                color_start: cs,
                color_end: ce,
            });
        }
        NodeKind::Directory { children, .. } => {
            // Get alive children sorted by size desc
            let mut kids: Vec<NodeId> = children
                .iter()
                .copied()
                .filter(|&c| tree.is_alive(c) && tree.node(c).size > 0)
                .collect();
            kids.sort_by(|&a, &b| tree.node(b).size.cmp(&tree.node(a).size));

            if kids.is_empty() {
                return;
            }

            // Layout children proportionally in the bounds
            let items: Vec<LayoutItem> = kids
                .iter()
                .enumerate()
                .map(|(i, &c)| LayoutItem {
                    id: i,
                    size: tree.node(c).size as f64,
                })
                .collect();

            let placed = layout(&items, bounds);

            // Add small gap between directory children for visual grouping
            let gap = 1.0;

            for p in &placed {
                let child_id = kids[p.id];
                let mut child_bounds = p.rect;
                // Inset slightly for gap effect
                child_bounds.x += gap;
                child_bounds.y += gap;
                child_bounds.w -= gap * 2.0;
                child_bounds.h -= gap * 2.0;
                if child_bounds.w < 0.5 || child_bounds.h < 0.5 {
                    continue;
                }

                // Use child's index as hue_index for depth coloring at the top level
                let child_hue = if tree.node(child_id).depth <= 1 {
                    p.id
                } else {
                    hue_index
                };

                layout_recursive(tree, child_id, &child_bounds, color_mode, child_hue, result);
            }
        }
    }
}
```

- [ ] **Step 2: Implement src/ui/treemap_view.rs**

```rust
use crate::app::AppState;
use crate::scanner::tree::NodeKind;
use crate::treemap;
use crate::treemap::squarify::Rect as TRect;
use crate::ui::theme;
use egui::{Color32, Pos2, Rect, Ui, Vec2};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let available = ui.available_rect_before_wrap();

    // Breadcrumb bar if zoomed
    if state.zoom_stack.len() > 1 {
        ui.horizontal(|ui| {
            if let Some(tree) = &state.tree {
                for (i, &node_id) in state.zoom_stack.iter().enumerate() {
                    if i > 0 {
                        ui.label(
                            egui::RichText::new(" › ")
                                .color(theme::TEXT_MUTED)
                                .size(11.0),
                        );
                    }
                    let name = tree.node(node_id).name.to_string_lossy().to_string();
                    let label = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&name)
                                .color(theme::ACCENT_LIGHT)
                                .size(11.0),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if label.clicked() {
                        state.view_root = Some(node_id);
                        state.zoom_stack.truncate(i + 1);
                        state.treemap_dirty = true;
                    }
                }
            }
        });
    }

    let treemap_rect = ui.available_rect_before_wrap();

    // Allocate the full remaining space for the treemap
    let (response, painter) = ui.allocate_painter(
        treemap_rect.size(),
        egui::Sense::click_and_drag().union(egui::Sense::hover()),
    );
    let canvas_rect = response.rect;

    // Recompute treemap if dirty
    let tree_exists = state.tree.is_some();
    if state.treemap_dirty && tree_exists {
        if let (Some(tree), Some(view_root)) = (&state.tree, state.view_root) {
            let bounds = TRect {
                x: canvas_rect.min.x as f64,
                y: canvas_rect.min.y as f64,
                w: canvas_rect.width() as f64,
                h: canvas_rect.height() as f64,
            };
            state.colored_rects = treemap::compute_treemap(tree, view_root, bounds, state.color_mode);
            state.treemap_dirty = false;
        }
    }

    // Draw background
    painter.rect_filled(canvas_rect, 0.0, theme::BG_DARK);

    if state.colored_rects.is_empty() {
        if state.scan_progress.scanning {
            // Show scanning indicator
            painter.text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Scanning...",
                egui::FontId::proportional(18.0),
                theme::ACCENT_LIGHT,
            );
        }
        return;
    }

    // Detect hovered block
    let pointer_pos = response.hover_pos();
    state.hovered_node = None;

    // Draw all blocks
    for cr in &state.colored_rects {
        let rect = Rect::from_min_size(
            Pos2::new(cr.rect.x as f32, cr.rect.y as f32),
            Vec2::new(cr.rect.w as f32, cr.rect.h as f32),
        );

        if rect.width() < 0.5 || rect.height() < 0.5 {
            continue;
        }

        let is_hovered = pointer_pos.map_or(false, |p| rect.contains(p));
        let is_selected = state.selected_node == Some(cr.node_id);

        // Check if this extension is highlighted
        let is_ext_highlighted = if let Some(ref sel_ext) = state.selected_extension {
            if let Some(tree) = &state.tree {
                tree.node(cr.node_id).extension() == Some(sel_ext.as_str())
            } else {
                false
            }
        } else {
            true // No extension filter → all highlighted
        };

        // Dimming for extension filter
        let alpha = if state.selected_extension.is_some() && !is_ext_highlighted {
            60
        } else {
            255
        };

        // Base gradient colors
        let c_start = Color32::from_rgba_unmultiplied(
            cr.color_start[0],
            cr.color_start[1],
            cr.color_start[2],
            alpha,
        );
        let c_end = Color32::from_rgba_unmultiplied(
            cr.color_end[0],
            cr.color_end[1],
            cr.color_end[2],
            alpha,
        );

        // Draw gradient (approximate with two triangles)
        // Top-left = c_start, bottom-right = c_end
        let mesh = gradient_rect(rect, c_start, c_end);
        painter.add(mesh);

        // Subtle light overlay in top-right for depth
        if rect.width() > 4.0 && rect.height() > 4.0 {
            let overlay_rect = Rect::from_min_size(
                Pos2::new(
                    rect.right() - rect.width() * 0.4,
                    rect.top(),
                ),
                Vec2::new(rect.width() * 0.4, rect.height() * 0.5),
            );
            painter.rect_filled(
                overlay_rect,
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 8),
            );
        }

        // Hover glow
        if is_hovered {
            state.hovered_node = Some(cr.node_id);
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, Color32::WHITE),
            );
            // Glow effect
            let glow_rect = rect.expand(2.0);
            painter.rect_stroke(
                glow_rect,
                3.0,
                egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(167, 139, 250, 100)),
            );
        }

        // Selection border
        if is_selected {
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0, theme::ACCENT_LIGHT),
            );
        }

        // Labels for blocks large enough
        if rect.width() > 40.0 && rect.height() > 16.0 {
            if let Some(tree) = &state.tree {
                let node = tree.node(cr.node_id);
                let name = node.name.to_string_lossy();
                // Truncate if needed
                let max_chars = (rect.width() / 7.0) as usize;
                let display_name = if name.len() > max_chars && max_chars > 3 {
                    format!("{}...", &name[..max_chars - 3])
                } else {
                    name.to_string()
                };

                let text_pos = Pos2::new(rect.left() + 3.0, rect.top() + 2.0);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &display_name,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );

                // Size label if tall enough
                if rect.height() > 28.0 {
                    let size_pos = Pos2::new(rect.left() + 3.0, rect.top() + 14.0);
                    painter.text(
                        size_pos,
                        egui::Align2::LEFT_TOP,
                        theme::format_size(node.size),
                        egui::FontId::proportional(9.0),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                    );
                }
            }
        }
    }

    // Handle clicks
    if response.clicked() {
        if let Some(hovered) = state.hovered_node {
            state.selected_node = Some(hovered);
            state.selected_extension = None;
        }
    }
    if response.double_clicked() {
        if let Some(hovered) = state.hovered_node {
            if let Some(tree) = &state.tree {
                // Zoom into the parent directory of the clicked file
                let parent = tree.node(hovered).parent;
                if let Some(pid) = parent {
                    if tree.node(pid).is_dir() {
                        state.view_root = Some(pid);
                        state.zoom_stack.push(pid);
                        state.treemap_dirty = true;
                    }
                }
            }
        }
    }

    // Context menu
    response.context_menu(|ui| {
        if let Some(node_id) = state.hovered_node.or(state.selected_node) {
            crate::ui::context_menu::show(ui, state, node_id);
        }
    });
}

/// Create a mesh that approximates a diagonal gradient on a rect.
fn gradient_rect(rect: Rect, c_start: Color32, c_end: Color32) -> egui::Shape {
    let mut mesh = egui::Mesh::default();

    let tl = rect.left_top();
    let tr = rect.right_top();
    let bl = rect.left_bottom();
    let br = rect.right_bottom();

    // Interpolate colors for a 135deg gradient (top-left to bottom-right)
    let c_tr = lerp_color(&c_start, &c_end, 0.5);
    let c_bl = lerp_color(&c_start, &c_end, 0.5);

    mesh.colored_vertex(tl, c_start);   // 0
    mesh.colored_vertex(tr, c_tr);       // 1
    mesh.colored_vertex(br, c_end);      // 2
    mesh.colored_vertex(bl, c_bl);       // 3

    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);

    egui::Shape::mesh(mesh)
}

fn lerp_color(a: &Color32, b: &Color32, t: f32) -> Color32 {
    let lerp = |a: u8, b: u8| -> u8 {
        (a as f32 + (b as f32 - a as f32) * t).clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}
```

- [ ] **Step 3: Add resize detection to App::update()**

In `src/app.rs`, add to `App::update()` after `self.poll_scan()` and the rescan check:

```rust
// Mark treemap dirty if window was resized
let current_size = ctx.screen_rect().size();
if (current_size - self.state.last_screen_size).length() > 1.0 {
    self.state.treemap_dirty = true;
    self.state.last_screen_size = current_size;
}
```

Also clear `colored_rects` in `start_scan()`:
```rust
self.state.colored_rects.clear();
```

- [ ] **Step 4: Build and run**

Run: `cargo run`
Expected: After scan completes, the central panel renders a colorful treemap showing all files. Hovering shows white glow borders. Clicking selects. Labels appear on larger blocks. Extension filter from right panel dims non-matching files.

- [ ] **Step 5: Commit**

```bash
git add src/treemap/mod.rs src/ui/treemap_view.rs src/app.rs
git commit -m "feat: treemap rendering with gradients, hover, labels, and extension filter"
```

---

### Task 11: Platform Operations + Context Menu

**Files:**
- Create: `src/platform/mod.rs`
- Create: `src/platform/finder.rs`
- Create: `src/platform/trash.rs`
- Create: `src/platform/dialogs.rs`
- Modify: `src/ui/context_menu.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/platform/finder.rs**

```rust
use std::path::Path;
use std::process::Command;

/// Reveal a file or folder in Finder (highlights it).
pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("open").arg("-R").arg(path).spawn();
}
```

- [ ] **Step 2: Create src/platform/trash.rs**

```rust
use std::path::Path;

/// Move a file or folder to macOS Trash. Returns Ok(()) on success.
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("Failed to move to trash: {}", e))
}
```

- [ ] **Step 3: Create src/platform/dialogs.rs**

```rust
use std::path::{Path, PathBuf};

/// Show a native folder picker dialog.
pub fn pick_folder(start_dir: &Path) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Select directory to scan")
        .set_directory(start_dir)
        .pick_folder()
}
```

- [ ] **Step 4: Create src/platform/mod.rs**

```rust
pub mod finder;
pub mod trash;
pub mod dialogs;
```

- [ ] **Step 5: Implement src/ui/context_menu.rs**

```rust
use crate::app::{AppState, PendingAction};
use crate::scanner::tree::NodeId;
use crate::ui::theme;
use egui::Ui;

pub fn show(ui: &mut Ui, state: &mut AppState, node: NodeId) {
    let (name, size, path) = if let Some(tree) = &state.tree {
        let n = tree.node(node);
        (
            n.name.to_string_lossy().to_string(),
            n.size,
            tree.full_path(node),
        )
    } else {
        return;
    };

    ui.label(
        egui::RichText::new(&name)
            .color(theme::TEXT_PRIMARY)
            .strong()
            .size(12.0),
    );
    ui.label(
        egui::RichText::new(theme::format_size(size))
            .color(theme::TEXT_MUTED)
            .size(11.0),
    );
    ui.separator();

    if ui.button("📂 Show in Finder").clicked() {
        crate::platform::finder::reveal_in_finder(&path);
        ui.close_menu();
    }

    if ui.button("📋 Copy Path").clicked() {
        ui.ctx().copy_text(path.to_string_lossy().to_string());
        ui.close_menu();
    }

    ui.separator();

    let trash_btn = ui.button(
        egui::RichText::new("🗑 Move to Trash").color(Color32::from_rgb(248, 113, 113)),
    );
    if trash_btn.clicked() {
        state.pending_action = Some(PendingAction::ConfirmTrash(
            node,
            name.clone(),
            size,
        ));
        ui.close_menu();
    }
}

use egui::Color32;
```

- [ ] **Step 6: Add `pub mod platform;` to src/main.rs**

Add after `pub mod ui;`:
```rust
pub mod platform;
```

- [ ] **Step 7: Build and verify**

Run: `cargo build`
Expected: Compiles. Right-clicking treemap blocks shows context menu with Show in Finder, Copy Path, Move to Trash.

- [ ] **Step 8: Commit**

```bash
git add src/platform/ src/ui/context_menu.rs src/main.rs
git commit -m "feat: platform operations (Finder, Trash) and context menu"
```

---

### Task 12: Delete Flow with Confirmation + Tree Patch

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add confirmation dialog handling to App::update()**

Add this at the end of `App::update()`, after the `CentralPanel`:

```rust
// Handle pending actions
let mut action_to_process = None;
if let Some(action) = &self.state.pending_action {
    match action {
        PendingAction::ConfirmTrash(node_id, name, size) => {
            let node_id = *node_id;
            let name = name.clone();
            let size = *size;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Move \"{}\" ({}) to Trash?",
                        name,
                        ui::theme::format_size(size),
                    ));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new("Cancel")
                                    .fill(ui::theme::BUTTON_BG),
                            )
                            .clicked()
                        {
                            action_to_process = Some(None); // dismiss
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Move to Trash")
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(220, 38, 38)),
                            )
                            .clicked()
                        {
                            action_to_process = Some(Some(node_id));
                        }
                    });
                });
        }
        _ => {}
    }
}
if let Some(result) = action_to_process {
    self.state.pending_action = None;
    if let Some(node_id) = result {
        self.perform_delete(node_id);
    }
}
```

- [ ] **Step 2: Add perform_delete method to App**

```rust
impl App {
    fn perform_delete(&mut self, node_id: NodeId) {
        if let Some(tree) = &self.state.tree {
            let path = tree.full_path(node_id);

            match crate::platform::trash::move_to_trash(&path) {
                Ok(()) => {
                    // Patch the tree
                    if let Some(tree) = &mut self.state.tree {
                        tree.remove_node(node_id);
                        // Recompute extension stats
                        let root = tree.root();
                        self.state.extension_stats = tree.collect_extensions(root);
                    }
                    // Clear selection if it was the deleted node
                    if self.state.selected_node == Some(node_id) {
                        self.state.selected_node = None;
                    }
                    if self.state.hovered_node == Some(node_id) {
                        self.state.hovered_node = None;
                    }
                    self.state.treemap_dirty = true;
                }
                Err(e) => {
                    eprintln!("Delete failed: {}", e);
                }
            }
        }
    }
}
```

Add `use crate::scanner::tree::NodeId;` to app.rs imports.

- [ ] **Step 3: Build and run**

Run: `cargo run`
Expected: Right-click a file → Move to Trash → confirmation dialog → file disappears from treemap and tree. Treemap recomputes instantly without rescanning.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: delete flow with confirmation dialog and tree patching"
```

---

### Task 13: Keyboard Shortcuts

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add keyboard handling to App::update()**

Add this at the start of `App::update()`, after theme application and poll_scan:

```rust
// Keyboard shortcuts
if ctx.input(|i| i.key_pressed(egui::Key::R) && i.modifiers.command) {
    self.state.request_rescan = true;
}
if ctx.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command) {
    if let Some(path) = crate::platform::dialogs::pick_folder(&self.state.scan_root) {
        self.state.scan_root = path;
        self.state.request_rescan = true;
    }
}
if ctx.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.command) {
    self.state.search_active = !self.state.search_active;
    if !self.state.search_active {
        self.state.search_query.clear();
    }
}
if ctx.input(|i| i.key_pressed(egui::Key::Backspace) && i.modifiers.command) {
    if let Some(node_id) = self.state.selected_node {
        if let Some(tree) = &self.state.tree {
            let name = tree.node(node_id).name.to_string_lossy().to_string();
            let size = tree.node(node_id).size;
            self.state.pending_action = Some(PendingAction::ConfirmTrash(node_id, name, size));
        }
    }
}
if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
    if let Some(node_id) = self.state.selected_node {
        if let Some(tree) = &self.state.tree {
            let path = tree.full_path(node_id);
            crate::platform::finder::reveal_in_finder(&path);
        }
    }
}
if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
    if self.state.search_active {
        self.state.search_active = false;
        self.state.search_query.clear();
    } else if self.state.zoom_stack.len() > 1 {
        self.state.zoom_stack.pop();
        self.state.view_root = self.state.zoom_stack.last().copied();
        self.state.treemap_dirty = true;
    } else {
        self.state.selected_node = None;
        self.state.selected_extension = None;
    }
}
```

- [ ] **Step 2: Build and run**

Run: `cargo run`
Expected: Cmd+R refreshes, Cmd+O opens folder picker, Cmd+F toggles search, Cmd+Backspace triggers delete on selected item, Enter reveals in Finder, Escape clears selection/search/zoom.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: keyboard shortcuts (Cmd+R/O/F/Backspace, Enter, Escape)"
```

---

### Task 14: Search/Filter Bar

**Files:**
- Modify: `src/ui/search.rs`

- [ ] **Step 1: Implement src/ui/search.rs**

```rust
use crate::app::AppState;
use crate::ui::theme;
use egui::Ui;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🔍")
                .color(theme::ACCENT_LIGHT)
                .size(14.0),
        );

        let response = ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Search files...")
                .desired_width(ui.available_width() - 40.0)
                .font(egui::FontId::proportional(13.0)),
        );

        // Auto-focus the search box when it first appears
        if response.gained_focus() || state.search_query.is_empty() {
            response.request_focus();
        }

        // Close button
        if ui
            .add(
                egui::Label::new(
                    egui::RichText::new("✕")
                        .color(theme::TEXT_MUTED)
                        .size(14.0),
                )
                .sense(egui::Sense::click()),
            )
            .clicked()
        {
            state.search_active = false;
            state.search_query.clear();
        }
    });
    ui.add_space(4.0);
}

/// Check if a node name matches the current search query.
pub fn matches_search(name: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&query.to_lowercase())
}
```

- [ ] **Step 2: Apply search filter to treemap rendering**

In `src/ui/treemap_view.rs`, update the alpha/dimming logic to also account for search. Replace the existing dimming block with:

```rust
// Dimming for extension filter and search
let is_search_match = if state.search_active && !state.search_query.is_empty() {
    if let Some(tree) = &state.tree {
        let name = tree.node(cr.node_id).name.to_string_lossy();
        crate::ui::search::matches_search(&name, &state.search_query)
    } else {
        true
    }
} else {
    true
};

let alpha = if (!is_ext_highlighted && state.selected_extension.is_some())
    || (!is_search_match)
{
    40
} else {
    255
};
```

- [ ] **Step 3: Build and run**

Run: `cargo run`
Expected: Cmd+F shows search bar. Typing filters/dims treemap blocks that don't match. X button or Escape closes search.

- [ ] **Step 4: Commit**

```bash
git add src/ui/search.rs src/ui/treemap_view.rs
git commit -m "feat: search/filter bar with treemap dimming"
```

---

### Task 15: Progress Bar During Scan

**Files:**
- Modify: `src/ui/treemap_view.rs`

- [ ] **Step 1: Improve the scanning state display**

In `src/ui/treemap_view.rs`, replace the simple "Scanning..." text with an animated progress indicator. Update the scanning block:

```rust
if state.colored_rects.is_empty() {
    if state.scan_progress.scanning {
        let center = canvas_rect.center();

        // Animated dots
        let time = ui.input(|i| i.time);
        let dots = match ((time * 2.0) as usize) % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        };

        painter.text(
            Pos2::new(center.x, center.y - 20.0),
            egui::Align2::CENTER_CENTER,
            format!("Scanning{}", dots),
            egui::FontId::proportional(20.0),
            theme::ACCENT_LIGHT,
        );

        painter.text(
            Pos2::new(center.x, center.y + 10.0),
            egui::Align2::CENTER_CENTER,
            format!(
                "{} files  •  {} dirs  •  {}",
                state.scan_progress.files,
                state.scan_progress.dirs,
                theme::format_size(state.scan_progress.bytes),
            ),
            egui::FontId::proportional(14.0),
            theme::TEXT_SECONDARY,
        );

        // Animated bar
        let bar_width = 200.0;
        let bar_height = 4.0;
        let bar_rect = Rect::from_min_size(
            Pos2::new(center.x - bar_width / 2.0, center.y + 35.0),
            Vec2::new(bar_width, bar_height),
        );
        painter.rect_filled(bar_rect, 2.0, theme::BAR_BG);

        let progress_x = ((time * 0.5).sin() * 0.5 + 0.5) as f32;
        let indicator_width = 60.0;
        let indicator_x = bar_rect.left() + (bar_width - indicator_width) * progress_x;
        let indicator = Rect::from_min_size(
            Pos2::new(indicator_x, bar_rect.top()),
            Vec2::new(indicator_width, bar_height),
        );
        painter.rect_filled(indicator, 2.0, theme::ACCENT);

        ui.ctx().request_repaint();
    }
    return;
}
```

- [ ] **Step 2: Build and run**

Run: `cargo run`
Expected: While scanning, the center of the treemap area shows an animated "Scanning..." with live file/dir/byte counts and a bouncing progress indicator.

- [ ] **Step 3: Commit**

```bash
git add src/ui/treemap_view.rs
git commit -m "feat: animated scanning progress indicator"
```

---

### Task 16: Polish and Final Integration

**Files:**
- Modify: `src/app.rs` (scroll-to-selected in tree)
- Modify: `src/ui/dir_tree.rs` (highlight when treemap selection changes)
- Modify: `src/ui/toolbar.rs` (use platform::dialogs)

- [ ] **Step 1: Wire toolbar to use platform::dialogs**

In `src/ui/toolbar.rs`, replace the `rfd::FileDialog` call with:

```rust
if scan_btn.clicked() {
    if let Some(path) = crate::platform::dialogs::pick_folder(&state.scan_root) {
        state.scan_root = path;
        state.request_rescan = true;
    }
}
```

Remove the `use rfd` import if present.

- [ ] **Step 2: Add tree auto-expand on treemap selection**

In `src/app.rs`, add a method to expand the tree path to a selected node:

```rust
impl AppState {
    /// Expand all ancestor directories of the given node so it's visible in the tree.
    pub fn expand_to_node(&mut self, node_id: NodeId) {
        if let Some(tree) = &mut self.tree {
            let mut current = tree.node(node_id).parent;
            while let Some(pid) = current {
                if let NodeKind::Directory { expanded, .. } = &mut tree.node_mut(pid).kind {
                    *expanded = true;
                }
                current = tree.node(pid).parent;
            }
        }
    }
}
```

Add `use crate::scanner::tree::NodeKind;` to imports in `app.rs`.

Call `expand_to_node` when a treemap block is clicked. In `treemap_view.rs`, after setting `selected_node`:

```rust
if response.clicked() {
    if let Some(hovered) = state.hovered_node {
        state.selected_node = Some(hovered);
        state.selected_extension = None;
        state.expand_to_node(hovered);
    }
}
```

- [ ] **Step 3: Add .gitignore**

Create `.gitignore`:

```
/target
.superpowers/
.DS_Store
```

- [ ] **Step 4: Full build + run test**

Run: `cargo build --release && cargo test`
Expected: Release build succeeds. All tests pass. App runs smoothly.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: polish — tree auto-expand on selection, platform dialogs, gitignore"
```

---

### Task 17: Final Smoke Test + Release Build

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass (tree, squarify, color).

- [ ] **Step 2: Release build**

Run: `cargo build --release`
Expected: Compiles with LTO. Binary at `target/release/mac-dir-stat`.

- [ ] **Step 3: Check binary size**

Run: `ls -lh target/release/mac-dir-stat`
Expected: Under 15MB.

- [ ] **Step 4: Manual smoke test**

Run: `cargo run --release`

Verify:
1. App opens with dark & vibrant theme
2. Auto-scans `/` with progress indicator
3. Treemap renders all files with colored gradient blocks
4. Directory tree shows expandable folders with size bars
5. Extension list shows sorted file types with swatches
6. Hover on treemap block → white glow + status bar shows path
7. Click treemap block → highlights in tree + extension list
8. Click extension → dims non-matching files in treemap
9. Right-click → context menu with Show in Finder, Copy Path, Move to Trash
10. Double-click folder in tree → zooms treemap, breadcrumb appears
11. Escape → zooms back out
12. Cmd+F → search bar filters treemap
13. Cmd+R → rescans
14. Cmd+O → folder picker

- [ ] **Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore: release build verified, smoke test passed"
```
