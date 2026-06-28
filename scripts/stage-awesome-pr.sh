#!/usr/bin/env bash
# One-shot: open the awesome-rust PR adding mac-dir-stat.
# YOU run this — it forks under your account, inserts the entry in alpha order,
# and opens the PR (so the public submit is your action, per the flywheel rule).
# Requires: gh (authenticated). Safe to re-run; reuses an existing fork.
set -euo pipefail

UPSTREAM="rust-unofficial/awesome-rust"
ANCHOR="https://github.com/Mobslide/mobslide"   # entry mac-dir-stat sorts just before
ENTRY='* [mac-dir-stat](https://github.com/Chartres/mac-dir-stat) - Treemap disk-usage visualizer for macOS that also clears regenerable build junk (DerivedData, node_modules, target, caches). Built with egui.'
WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT

gh repo fork "$UPSTREAM" --clone=false >/dev/null 2>&1 || true
ME="$(gh api user -q .login)"
git clone -q "https://github.com/$ME/awesome-rust.git" "$WORK/awesome-rust"
cd "$WORK/awesome-rust"
git remote add upstream "https://github.com/$UPSTREAM.git" 2>/dev/null || true
git fetch -q upstream && git checkout -q -B add-mac-dir-stat upstream/main

grep -qF "$ANCHOR" README.md || { echo "Anchor moved — open README.md and place the entry in ### Utilities by hand:"; echo "$ENTRY"; exit 1; }
grep -qF "Chartres/mac-dir-stat" README.md && { echo "Already listed. Nothing to do."; exit 0; }

# Insert ENTRY immediately before the anchor line (keeps Utilities alphabetical).
awk -v entry="$ENTRY" -v anchor="$ANCHOR" '
  index($0, anchor) && !done { print entry; done=1 } { print }
' README.md > README.tmp && mv README.tmp README.md

git add README.md
git commit -q -m "Add mac-dir-stat (treemap disk-usage tool for macOS)"
git push -q -u origin add-mac-dir-stat --force-with-lease

gh pr create --repo "$UPSTREAM" --head "$ME:add-mac-dir-stat" \
  --title "Add mac-dir-stat" \
  --body "Adds [mac-dir-stat](https://github.com/Chartres/mac-dir-stat) under Applications → Utilities — a treemap disk-usage tool for macOS (Rust + egui). Shows what's using your disk and trashes regenerable build junk (DerivedData, node_modules, target, caches) in one pass. Free, open source (MIT), signed + notarized release. Inserted in alphabetical order in the Utilities subsection."
