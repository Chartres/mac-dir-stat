# mac-dir-stat

Treemap directory-size visualizer for macOS. Rust + [egui](https://github.com/emilk/egui).

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
