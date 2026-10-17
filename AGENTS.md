# AGENTS.md — mac-dir-stat

The build/test/release contract for this repo. An agent (or the overnight ralph loop) should be
able to read only this file and ship correctly. Keep every command copy-pasteable and current.
Taste rules that apply to every flywheel product live in the hub: `flywheel/docs/standards/taste.md`.

> One-liner: Treemap directory-size visualizer for macOS — find what's eating your disk and reclaim it.
> Stack/template: Rust + egui/eframe, `cli-homebrew`  ·  Track: community  ·  Portfolio record: `flywheel/data/products/mac-dir-stat.json`

## Build
```bash
# no install step — cargo fetches deps on first build
cargo build --release        # binary at target/release/mac-dir-stat
```

## Test (TDD required; persona-journey test per primary journey)
```bash
cargo check                  # typecheck
cargo test                   # unit + integration (tests/) — the CI gate
# no E2E harness
```
Gate: typecheck · test · build must pass (CI is `.github/workflows/ci.yml`; gates on `cargo test`).
`cargo fmt`/`cargo clippy` are not clean repo-wide yet, so CI does not block on them — run them
locally before touching a file you can leave clean.

## Run / verify a change in the real app
```bash
cargo run --release          # launches the egui app; scans your disk on open
```
```bash
cargo run --release -- serve --root ~/Downloads   # headless server; prints the web UI link with token
```
Look at: treemap renders, directory tree syncs on click, file-type breakdown, cleanup window.
Press `?` in-app for the shortcut list.

## Release (the finish line — produces a storefront link)
Desktop app, distributed as a signed + notarized universal DMG on a GitHub Release plus a
Homebrew cask. Tag-driven:
```bash
git tag vX.Y.Z && git push origin vX.Y.Z   # triggers .github/workflows/release.yml
```
The tag must be `v` + the `version` in `Cargo.toml`; the workflow fails before building otherwise.
The workflow builds the universal `.app` (`scripts/bundle.sh`), makes the DMG (`scripts/dmg.sh`),
signs + notarizes (`scripts/notarize.sh`), publishes the release, and bumps the Homebrew cask at
`Chartres/homebrew-mac-dir-stat` (tap clone at `~/my-git/homebrew-mac-dir-stat`, `Casks/mac-dir-stat.rb`).
Repo secrets the workflow reads (all set as of 2026-10-17):
- signing: `MACOS_CERT_P12_BASE64`, `MACOS_CERT_PASSWORD`
- notarization: `MACOS_NOTARY_APPLE_ID`, `MACOS_NOTARY_PASSWORD`, `MACOS_NOTARY_TEAM_ID`
- cask bump: `HOMEBREW_TAP_DEPLOY_KEY`, the private half of a write deploy key on
  `Chartres/homebrew-mac-dir-stat` (rotation steps in `docs/GROWTH.md`)
- analytics bake: `FLYWHEEL_SUPABASE_URL`, `FLYWHEEL_SUPABASE_KEY`

The `APPLE_*` secrets in the repo are not read by any workflow. Missing signing/tap secrets degrade
gracefully (unsigned build, skipped cask). Adoption is read from GitHub release **download counts** + GitHub
stars (north star) — no extra telemetry needed for KPIs.

## Analytics (Common Platform)
Vendored client: `src/flywheel.rs` (Rust port of `flywheel-client.ts`). Fires the shared taxonomy
(`page_view`, `signup_*`, `conversion`, `key_action`, `feedback_given`, `error`) to the shared
flywheel-core Supabase `events` table via a detached `curl`. Cookieless. Release builds bake in
`FLYWHEEL_SUPABASE_URL`/`FLYWHEEL_SUPABASE_KEY` (`option_env!`) and send unless the user opts out
(`MACDIRSTAT_NO_TELEMETRY=1`, or "Don't send anonymous usage data" under Privacy in the `?` window).
Source builds have no endpoint and send nothing. The aha moment (bytes freed) fires
`conversion`; `activation_event: "conversion"` is set in the portfolio record.

## Done means
Green CI · released (notarized DMG on a GitHub Release + cask bumped) · portfolio record updated
(stage/gate/links) · storefront link live · (outward promotion only after Pavol's sign-off).
