# mac-dir-stat — Design Spec

A WinDirStat clone for macOS, built in Rust with egui. Dark & vibrant aesthetic. Shows disk usage as a treemap with all files visible simultaneously. Supports "Show in Finder" and "Move to Trash" from the UI.

## Overview

**Goal:** A fast, beautiful disk space analyzer for macOS that shows where space is going at a glance.

**Key principles:**
- All files visible at once in the treemap (no mandatory drill-down)
- Fast — parallel scanning, instant treemap relayout on zoom/delete
- Safe — deletes go to Trash, not `rm`
- Beautiful — dark theme, vibrant gradients, polished UI

## Architecture

Four modules in a single Rust binary:

```
mac-dir-stat/
├── src/
│   ├── main.rs           # Entry point, launches eframe
│   ├── app.rs            # Top-level egui App, owns state, routes input
│   ├── scanner/
│   │   ├── mod.rs        # Public API: scan(), ScanProgress, FileTree
│   │   ├── walk.rs       # jwalk-based parallel directory walker
│   │   └── tree.rs       # Arena-based FileTree data structure
│   ├── treemap/
│   │   ├── mod.rs        # Public API: layout(), TreemapRect
│   │   ├── squarify.rs   # Squarified treemap layout algorithm
│   │   └── color.rs      # Coloring modes (extension, depth, age)
│   ├── ui/
│   │   ├── mod.rs        # Panel orchestration, layout
│   │   ├── toolbar.rs    # Top toolbar (scan button, path, color mode toggle)
│   │   ├── dir_tree.rs   # Directory tree panel (top-left)
│   │   ├── ext_list.rs   # File type list panel (top-right)
│   │   ├── treemap_view.rs # Treemap canvas (bottom)
│   │   ├── context_menu.rs # Right-click menu
│   │   ├── search.rs     # Cmd+F search/filter bar
│   │   └── theme.rs      # Colors, gradients, spacing constants
│   └── platform/
│       ├── mod.rs        # Public API
│       ├── finder.rs     # Reveal in Finder (open -R)
│       ├── trash.rs      # Move to Trash (trash crate)
│       └── dialogs.rs    # Folder picker (rfd), volume enumeration
└── Cargo.toml
```

## Data Structure

Arena-based tree for cache-friendly traversal and easy mutation on delete:

```rust
type NodeId = usize; // index into FileTree.nodes

struct FileTree {
    nodes: Vec<Node>,
    root: NodeId,
}

struct Node {
    name: OsString,
    size: u64,              // file: file size on disk, dir: sum of children
    kind: NodeKind,
    modified: SystemTime,
    parent: Option<NodeId>,
    depth: u16,
}

enum NodeKind {
    File { extension: Option<String> },
    Directory { children: Vec<NodeId>, expanded: bool },
}
```

**Why arena:** Flat Vec means no Box/Rc overhead, trivial to index by NodeId, and deletion is a swap-remove or mark-as-dead without reallocating.

## Scanner

**Crate:** `jwalk` for parallel directory walking. It uses Rayon internally and respects macOS sandbox/permission boundaries.

**Flow:**
1. User selects directory (default: `/`)
2. Spawn background thread with `jwalk::WalkDir`
3. Build `FileTree` incrementally, sending progress via `crossbeam-channel`:
   - `ScanProgress::Counting { files: usize, dirs: usize, bytes: u64 }`
   - `ScanProgress::Done(FileTree)`
4. UI shows animated progress bar with files/dirs/bytes counts
5. On completion: compute extension statistics, trigger treemap layout

**Permission errors:** Skipped directories appear in the tree as "(access denied)" with size 0. No panic, no abort.

**Rescan:** Drops existing `FileTree`, runs a fresh scan. Same flow.

## Treemap Engine

**Algorithm:** Squarified treemap (Bruls, Huizing, van Wijk, 2000). Recursively partitions a rectangle into sub-rectangles proportional to file sizes, optimizing for aspect ratios close to 1.

**Input:** The current view root's `NodeId` + the target pixel rectangle. The algorithm is applied **hierarchically**: first, direct children of the view root are assigned rectangles proportional to their total size. Then, within each directory's rectangle, its children are recursively laid out. This creates visible directory groupings — files from the same folder cluster together, separated by 1-2px gaps between directory regions.

**Output:** `Vec<TreemapRect>`:
```rust
struct TreemapRect {
    node_id: NodeId,
    rect: egui::Rect,   // pixel coordinates
    color: (Color32, Color32), // gradient pair
}
```

**Coloring modes:**
- **Extension (default):** Deterministic hash of extension string maps to one of ~20 curated vibrant gradient pairs. Unknown extensions get a neutral gray gradient.
- **Depth:** Each top-level directory under the view root gets a distinct hue. Children are lighter/darker shades of the parent hue, proportional to depth.
- **Age:** File's `modified` timestamp mapped to a warm-to-cool gradient. Recent files (< 1 week) = bright magenta/pink. Old files (> 1 year) = deep navy/indigo. Linear interpolation between.

**Performance target:** Layout computation for 100k files in <50ms. The layout is cached and only recomputed on:
- Window resize
- Zoom in/out (double-click folder in tree or treemap)
- Delete (tree patch triggers relayout)
- Color mode toggle

## UI Layout

Three-panel layout inspired by WinDirStat:

```
┌─────────────────────────────────────────────┐
│ Toolbar: [Scan...] [⟳] path/info  [Ext|Dep|Age] │
├────────────────────────┬────────────────────┤
│ Directory Tree         │ File Types         │
│ (top-left)             │ (top-right)        │
│                        │                    │
│ Expandable tree with   │ Extension list     │
│ size bars per entry    │ sorted by total    │
│                        │ size, color swatch │
├────────────────────────┴────────────────────┤
│ Treemap (bottom, ~60% of height)            │
│                                             │
│ Every file = one rectangle                  │
│ Colored by selected mode                    │
│ Hover = glow border + tooltip               │
│                                             │
├─────────────────────────────────────────────┤
│ Status: 142,847 files • 23,412 dirs • 3.2s  │
└─────────────────────────────────────────────┘
```

**Panel splitters:** Draggable horizontal splitter between top panels and treemap. Draggable vertical splitter between directory tree and file types. egui's built-in panel system handles this.

### Toolbar
- **Scan Directory...** button → opens native folder picker (`rfd`)
- **Refresh** button → rescans current directory
- **Path display** — shows current scan root + total used/available
- **Color mode toggle** — three buttons: Extension (default), Depth, Age. Active mode highlighted with accent border.

### Directory Tree (top-left)
- Expandable/collapsible tree
- Each row: expand arrow + folder icon + name + size + proportional size bar
- Selected row highlighted with accent background + left border
- **Single click** → highlights folder's region in treemap (glow border around the group)
- **Double click** → zooms treemap into that folder (breadcrumb appears for navigation back)
- Sorted by size descending within each level

### File Types (top-right)
- List of extensions sorted by total size descending
- Each row: color swatch + extension name + size bar + absolute size + percentage
- Clicking an extension highlights all files of that type in the treemap
- Shows "+ N more types (X GB, Y%)" footer for long tail

### Treemap (bottom)
- Fills remaining space (~60% of window height)
- Every file is its own rectangle, sized proportionally
- Gradient fill per block, matching the active coloring mode
- **Hover:** white glow border + tooltip showing full path, size, extension, last modified
- **Single click:** selects file, highlights in tree + extension list
- **Double click on a file:** zooms into that file's parent directory
- **Right click:** context menu (Show in Finder, Move to Trash, Copy Path)
- **Breadcrumb bar** (visible when zoomed): clickable path segments to navigate back up
- Labels shown on blocks large enough to fit text (file name + size). Blocks too small get no label.

### Status Bar
- Left: file count, directory count, scan duration
- Right: hovered file's path + size + extension, keyboard shortcut hints

## Interactions

### Selection synchronization
All three panels stay in sync:
- Click in tree → highlights in treemap + scrolls to extension
- Click in treemap → expands + scrolls tree to that file's parent, highlights extension
- Click extension → highlights all matching files in treemap

### Zoom
- Double-click a folder (in tree or treemap) → treemap re-renders showing only that folder's contents
- Breadcrumb bar appears showing zoom path: `/ > Users > pavol > Library`
- Click any breadcrumb segment to zoom to that level
- Escape zooms back to root
- Zoom is instant (relayout of cached tree, no rescan)

### Delete flow
1. User selects file/folder (click in tree or treemap)
2. `Cmd+Backspace` or right-click → "Move to Trash"
3. Confirmation dialog: "Move [name] ([size]) to Trash?"
4. On confirm: `trash` crate moves to macOS Trash
5. Tree patch: remove node, subtract size from all ancestors
6. Recompute treemap layout (no rescan)
7. Selection moves to next sibling or parent

### Search
- `Cmd+F` opens search bar at top of treemap area
- Filters directory tree to matching entries
- Matching files highlighted in treemap (non-matching dimmed to 20% opacity)
- Escape closes search bar

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+O` | Open folder picker to scan new directory |
| `Cmd+R` | Rescan current directory |
| `Cmd+F` | Toggle search/filter bar |
| `Cmd+Backspace` | Move selected item to Trash (with confirmation) |
| `Enter` | Show selected item in Finder |
| `Escape` | Clear selection / close search / zoom out to root |
| `Up/Down` | Navigate directory tree |
| `Right/Left` | Expand/collapse tree node |

All shortcuts shown in context menus and as tooltips for discoverability.

## Theme

**Dark & Vibrant** — inspired by modern dev tools (Arc, Warp).

```
Background:       #0e0c18 (near-black with purple tint)
Panel background:  #161225
Border:           #2a2a4a
Text primary:     #e0e0f0
Text secondary:   #8b8bbb
Text muted:       #666
Accent:           #7c3aed → #a78bfa (purple gradient)
Selection bg:     #1a1730
Section header:   #6b5b95
```

**Treemap gradient palette** (extension mode, ~20 pairs):
```
Purple:  #7c3aed → #a78bfa  (.mov, .mkv)
Blue:    #2563eb → #60a5fa  (.dmg, .iso)
Cyan:    #0891b2 → #67e8f9  (.mp4, .avi)
Red:     #dc2626 → #f87171  (.app, .exe)
Orange:  #ea580c → #fb923c  (.zip, .tar)
Green:   #16a34a → #4ade80  (.jpg, .heic, .png)
Yellow:  #ca8a04 → #facc15  (.mp3, .wav)
Magenta: #9333ea → #c084fc  (.js, .ts)
Slate:   #475569 → #94a3b8  (unknown/other)
```

Each block renders with a subtle 135deg gradient + a light overlay in the top-right corner for depth.

**Hover state:** White 2px border + purple glow (`box-shadow` equivalent via `Painter::rect_stroke` + translucent rect behind).

## Dependencies

```toml
[dependencies]
eframe = "0.31"           # egui framework with wgpu backend
egui = "0.31"             # immediate-mode GUI
jwalk = "0.8"             # parallel directory walker
crossbeam-channel = "0.5" # scanner → UI communication
trash = "5"               # macOS Trash integration
rfd = "0.15"              # native file dialogs
chrono = "0.4"            # timestamp formatting for age mode

[profile.release]
opt-level = 3
lto = true
```

## Performance Targets

- **Scan speed:** ~3s for a typical macOS home directory (~150k files) using jwalk's parallel walking
- **Treemap layout:** <50ms for 100k files
- **UI frame rate:** 60fps during interaction (hover, scroll, resize)
- **Memory:** ~100 bytes per file node → ~15MB for 150k files
- **Binary size:** <10MB release build with LTO

## Out of Scope

- Windows/Linux support (macOS only, uses macOS-specific APIs)
- Real-time filesystem watching (manual refresh only)
- Duplicate file detection
- Cloud storage analysis
- Export/reporting features
