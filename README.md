# mac-dir-stat

Treemap directory-size visualizer for macOS. Rust + [egui](https://github.com/emilk/egui).

![treemap directory size visualizer](docs/screenshot-placeholder.png)

## Features

- **Treemap rendering** of directory contents — area = file size — with cushioned gradient colors per file type, depth, or modification age.
- **Directory tree panel** synced to the treemap; click anywhere in the treemap to reveal the location.
- **File-type breakdown** showing total bytes per extension.
- **Cleanup Suggestions**: scans for known-regenerable directories (Xcode DerivedData, iOS Simulators, `node_modules`, `target`, Application Caches, Docker data, …) and lets you trash them with one click.
- **Refresh subtree**: right-click a folder to re-scan only that subtree.
- **Right-click menu** on every node — Reveal in Finder, Copy Path, Refresh, Zoom Into, Move to Trash.
- **Drag-and-drop** a folder onto the window to scan it.
- **Hover tooltip** with name, size, type, and relative modified time.
- **Persistent state** — last scan path and color mode are restored on launch.
- **TCC-aware**: known protected paths (Photos library, Mail, Calendar, Reminders, removable volumes, …) are filtered out before traversal so you don't get a wall of macOS permission prompts on first scan.

## Install

### Homebrew (recommended)

```sh
brew install --cask chartres/mac-dir-stat/mac-dir-stat
```

`brew` strips the quarantine attribute on install, so the app opens normally.

### Manual DMG

Grab the latest `MacDirStat-<version>.dmg` from [Releases](https://github.com/Chartres/mac-dir-stat/releases), open it, drag **MacDirStat** into `/Applications`.

The app is currently **unsigned**, so on first launch macOS Gatekeeper will block it. Either:

- **Right-click → Open** in Finder (one-time "Open anyway" prompt), or
- Strip the quarantine flag once: `xattr -dr com.apple.quarantine /Applications/MacDirStat.app`

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `⌘O` | Pick directory to scan |
| `⌘R` | Re-scan current root |
| `⌘F` | Search files in scanned tree |
| `⌘⌫` | Move selected node to Trash |
| `Enter` | Reveal selected node in Finder |
| `Esc` | Close search / pop zoom / clear selection |

## Build from source

```sh
cargo run --release
```

Build a universal `.app` + DMG:

```sh
./scripts/bundle.sh   # produces dist/MacDirStat.app
./scripts/dmg.sh      # produces dist/MacDirStat-<version>.dmg
```

Tag-driven release: pushing `vX.Y.Z` triggers `.github/workflows/release.yml`, which builds a universal DMG and publishes it to GitHub Releases.

## Architecture

- `src/scanner/` — parallel filesystem walk via [jwalk](https://github.com/byron/jwalk), TCC-protected paths filtered up front.
- `src/treemap/` — squarified treemap layout, color gradients, palette.
- `src/ui/` — egui chrome: toolbar, side panels, treemap viewport, search, cleanup window, context menu.
- `src/cleanup.rs` — heuristic detection of regenerable directories.
- `src/state.rs` — persisted UI state (scan root, color mode).
