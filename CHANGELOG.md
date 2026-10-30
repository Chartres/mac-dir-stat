# Changelog

All notable changes to MacDirStat are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project uses
[semantic versioning](https://semver.org/).

## [Unreleased]

### Added
- **The inventory.** The Cleanup window now has two columns. *Can be rebuilt* groups the cleanup
  candidates by rule, one tick per rule (or per folder under "each one"), with a line saying what it
  takes to get it back (`npm install`, the next Xcode build, …) and when it last changed. *Only you
  have these* lists Photos libraries, Documents, Desktop, Pictures, Movies, Music, Mail and Messages
  in your home folder with their sizes; they have no tick boxes and are never offered for deletion.
- **A closing account** after you move things to the Trash with the Cleanup window open: how much
  went, how much of it a tool can rebuild and how much no tool can, free space on the disk, and a
  reminder that the Trash is not emptied, so nothing is gone yet.

- **The ledger, in the folder list.** The directory tree is now a ranked ledger of the folder the
  map shows: each row has its exact size and a length bar scaled to the largest row at its level,
  and a folder's own files appear as one row ("12 files") so the lengths add up. Hovering a row
  outlines that folder on the map; hovering the map lights up the row that holds the block. Zooming
  the map moves the list to that folder.

### Changed
- **New look: the survey register.** Book typography on paper: Source Serif 4 (bundled, SIL Open
  Font License) for all text with a real italic for notes, ink on paper, hairline rules, and one
  oxblood accent for the main action in each window. Treemap fills are muted earth and slate tints.
  This replaces the violet, dark-first theme.
- A dark variant inverts the same ink-on-paper contrast and follows the macOS appearance setting.
- The phone web page (`serve`) uses the same colours and a serif font.

## [0.6.0] — 2026-10-17

### Added
- **Move to External Disk…** in the right-click menu. It copies the item with `ditto` to
  `<volume>/MacDirStat Offload/<original absolute path>`, flushes the copy to disk, compares it
  byte for byte, then moves the original to the Trash and, if you ask, leaves a symlink. A failed
  move deletes only its temporary copy. One move runs at a time.
- Moves refuse, with the reason shown: system folders, `/Users/Shared`, other users' home folders,
  whole volumes, your home folder and `~/Library` themselves (checked case-insensitively); a
  destination on the same disk, inside the item, already existing or without enough room; and
  exFAT/FAT limits (app bundles and packages, symlinks, `._*` files, and on FAT32 files of 4 GiB
  or more).
- Moves and approved plans trash items through NSFileManager, so they work without a window.
  Finder's Put Back does not work for those items.
- **`mac-dir-stat serve [--bind IP] [--port 7878] [--root PATH]`** runs without a window and serves
  a phone-friendly web page, a REST API under `/api/v1` (OpenAPI 3.1 at `/api/v1/openapi.json`) and
  an MCP endpoint at `/mcp`. It listens on loopback unless you pass `--bind`.
- **Plans.** Trash and move requests become a plan, and nothing runs until you approve it. Each
  operation is checked again just before it runs, and a plan with errors cannot be approved. Plans
  live in memory; the newest 200 finished ones are kept.
- Two tokens in `~/Library/Application Support/MacDirStat/` (mode 0600): `server-token` can do
  everything; `mcp-token` can read and propose plans, and gets 403 on approve, reject and the REST
  rescan. Delete a file to rotate its token. The token is printed only when stdout is a terminal,
  and the page refuses to be framed.
- MCP tools: `list_volumes`, `scan`, `get_tree`, `top_items`, `find`, `cleanup_suggestions`,
  `propose_plan`, `list_plans`, `get_plan`.
- **Copy summary for any chat** and **Create plan from JSON** in the web page, for AI chats that
  do not speak MCP.

### Changed
- The README and the website now say that release builds send anonymous usage counts unless you
  turn them off.

### Fixed
- Move to Trash refuses protected paths such as `/System`, `/usr`, `/Users/Shared` and other users'
  home folders, and the status bar says why.
- Move to Trash is no longer offered for the scan root or for the Free Space and Hidden / Skipped
  blocks, in the menu or through ⌘⌫.
- A confirmed trash acts only on the path shown in the dialog. If a rescan moved or removed that
  item, it refuses and says so. A rescan also clears Cleanup selections and open confirmations,
  and a full scan drops a subtree refresh that was still running.
- ⌘⌫ and ↩ are ignored while you type in a text field, and ⌘⌫ is ignored while a confirmation is
  open. The confirmation shows the full path instead of a shortened name.
- Folders whose metadata cannot be read, and entries whose parent is missing, are no longer
  attached to the wrong folder. That could point Trash or Reveal at the wrong path.
- A move that finishes after a rescan removes the moved item from the treemap, not whichever item
  now has its old place.
- Cleanup Suggestions: iCloud Drive is no longer suggested, because it can hold files that are not
  uploaded yet. `target` counts only next to a `Cargo.toml`. Path rules match whole folder names,
  so `MyLibrary/Caches` is not `Library/Caches`. The Trash and the scan root are never suggested.
  Gradle, Maven and Docker entries warn about saved credentials and named volumes. Each category
  header appears once.
- Hard-linked files are counted once, so folder totals no longer exceed the space used.
- Choosing "(none)" in the file-type list highlights files without an extension, and the
  file-type filter resets on a new scan.
- Trashing the folder you zoomed into resets the treemap instead of leaving it blank.
- Fixed crashes on names with non-ASCII characters and when hovering after a rescan.
- Trash, batch trash and Empty Trash errors show in the status bar and stay until the next scan.
- The spinner stops if the scan thread crashes.
- Usage-count uploads no longer leave zombie `curl` processes, and the status bar no longer walks
  the whole tree every frame to count files.
- `cargo test` no longer overwrites the saved scan folder in `~/Library/Application Support`.

## [0.5.1] — 2026-06-18

### Changed
- **First signed and notarized release.** The `.app` is signed with an Apple
  Developer ID and notarized by Apple, so it opens normally on first launch with
  Gatekeeper on — no `xattr` workaround. Removes the hard blocker for a
  `homebrew/cask` submission.
- README install section drops the unsigned/quarantine caveat.

## [0.5.0] — 2026-06-18

### Added
- **Scan on launch** — opens straight into a scan of the whole disk (or your
  last-scanned folder), no welcome screen.
- **One-time Full Disk Access prompt** — on first run, offers to open the right
  Settings pane once, then never nags again.
- **Item and file counts** — each folder shows its item count (files + subdirs);
  the file-type breakdown shows a file count per extension.
- **Sort controls** — folder list sorts by Size · Name · Items · Recent;
  file-type list by Size · Count · Name.
- **Keyboard navigation** of the directory list (arrow keys to move, expand,
  collapse).
- **More context-menu actions** — Open, Open Terminal Here, Get Info, alongside
  Reveal, Copy Path, Refresh, Zoom Into, Move to Trash.
- **Whole-volume accounting** for any volume root (not just `/`): adds free-space
  and hidden/skipped blocks so the treemap covers the entire disk.
- **Anonymous, off-by-default usage analytics** (app opens, scans run, space
  reclaimed — never file names or paths), with in-app opt-out under Help →
  Privacy.

## [0.4.3] — 2026-05-09

### Added
- Refresh-subtree without losing the current view; cleanup-suggestion
  refinements.

## [0.4.2] — 2026-05-03

### Fixed
- Treemap color-gradient and layout fixes.

## [0.4.1] — 2026-05-03

### Added
- Initial public treemap, directory tree, file-type breakdown, and cleanup
  suggestions.

[0.5.1]: https://github.com/Chartres/mac-dir-stat/releases/tag/v0.5.1
[0.5.0]: https://github.com/Chartres/mac-dir-stat/releases/tag/v0.5.0
[0.4.3]: https://github.com/Chartres/mac-dir-stat/releases/tag/v0.4.3
[0.4.2]: https://github.com/Chartres/mac-dir-stat/releases/tag/v0.4.2
[0.4.1]: https://github.com/Chartres/mac-dir-stat/releases/tag/v0.4.1
