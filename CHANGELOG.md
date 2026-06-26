# Changelog

All notable changes to MacDirStat are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project uses
[semantic versioning](https://semver.org/).

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
