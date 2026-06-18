# GROWTH.md — GitHub stars → Homebrew acceptance

> The goal: enough genuine traction to land MacDirStat in the **official
> `homebrew/cask` tap**, so `brew install --cask mac-dir-stat` works with no
> third-party tap. Stars are not the end — they're the gate Homebrew uses to
> judge whether a project is "notable" rather than "too obscure."

## The acceptance bar (from Homebrew's *Acceptable Casks*, checked 2026-06-18)

| Submission type | Forks | Watchers | Stars |
|---|---:|---:|---:|
| **Standard** (someone other than the author submits, or general submission) | 30 | 30 | 75 |
| **Self-submitted** (PR author owns the repo) | 90 | 90 | 225 |

Plus two hard rules that matter here:

1. **Unsigned apps are rejected.** Verbatim: an app that *"fails with Gatekeeper
   enabled … (e.g. unsigned apps will not launch on Apple Silicon Macs)"* is not
   acceptable. MacDirStat is **currently unsigned** (see README). **This is a
   blocker for `homebrew/cask` regardless of stars** and must be fixed first.
2. **Notability can be waived** for apps with their **own website** (even if the
   binary is GitHub-hosted) or **newly released software getting significant
   social-media attention** — a realistic shortcut if a launch post takes off.

### What this means for sequencing

```
notarize the app  ──►  reach standard notability (≥75★/30 forks/30 watchers)
   (hard blocker)         via the SHARING.md launch + the tactics below
        │                              │
        └──────────────►  submit to homebrew/cask (prefer a non-author submitter
                          to use the 75★ bar, not the 225★ self-submit bar)
```

Until both hold, the **personal tap stays the install path**
(`brew install --cask chartres/mac-dir-stat/mac-dir-stat`) — that has no
notability or signing gate and works today.

---

## Track A — unblock signing (prerequisite, do first)

- [ ] Enroll / use an **Apple Developer ID** certificate; sign the `.app` in
      `scripts/bundle.sh` (`codesign --deep --options runtime`).
- [ ] **Notarize** the DMG (`notarytool submit … --wait`) and **staple** it.
- [ ] Verify on an Apple-Silicon Mac with Gatekeeper on: download → open, **no**
      `xattr` workaround needed.
- [ ] Update the README install section: drop the "unsigned / strip quarantine"
      caveat once notarized.
- [ ] Wire signing secrets into `.github/workflows/release.yml` so every tagged
      release ships notarized.

This also *improves* the launch: "unsigned, run `xattr -dr`…" is a real
conversion-killer on r/macapps and HN.

### The CI is already wired — you just add secrets

`scripts/bundle.sh`, `scripts/notarize.sh`, and `.github/workflows/release.yml`
already sign + notarize + staple **when these secrets exist**, and fall back to
the current unsigned build when they don't. One-time setup:

```sh
# Export your Developer ID Application cert from Keychain as cert.p12, then:
base64 -i cert.p12 | gh secret set MACOS_CERT_P12_BASE64 -R Chartres/mac-dir-stat
gh secret set MACOS_CERT_PASSWORD    -R Chartres/mac-dir-stat   # the .p12 password
gh secret set MACOS_NOTARY_APPLE_ID  -R Chartres/mac-dir-stat   # your Apple ID email
gh secret set MACOS_NOTARY_PASSWORD  -R Chartres/mac-dir-stat   # app-specific password
gh secret set MACOS_NOTARY_TEAM_ID   -R Chartres/mac-dir-stat   # 10-char Team ID
```

To turn on the in-app analytics (otherwise the SDK ships dark), also:

```sh
gh secret set FLYWHEEL_SUPABASE_URL -R Chartres/mac-dir-stat -b "https://<project>.supabase.co"
gh secret set FLYWHEEL_SUPABASE_KEY -R Chartres/mac-dir-stat -b "<publishable-key>"   # sb_publishable_…
```

After the secrets are set, the next `git tag vX.Y.Z && git push --tags` ships a
notarized DMG and bumps the cask automatically.

---

## Track B — earn the stars (genuine traction, not gaming)

Ordered by leverage. Homebrew's metrics reward sustained interest, so favor
evergreen discovery over one-day spikes.

1. **A real screenshot + demo GIF in the README.** Non-negotiable. A visual tool
   with a placeholder image is invisible. This gates everything else.
2. **AlternativeTo listing** as a WinDirStat / DaisyDisk / GrandPerspective
   alternative. Captures the exact search "WinDirStat for Mac" indefinitely.
3. **Awesome-list PRs:** `awesome-macos`, `awesome-rust`, `awesome-egui`. Each is
   a durable referral and a small star drip.
4. **GitHub repo topics** (`macos`, `treemap`, `disk-usage`, `windirstat`,
   `rust`, `egui`) → surfaces in GitHub topic browsing and search.
5. **The SHARING.md launch waves** — r/macapps, r/DataHoarder, r/rust, Show HN.
   One bespoke post per venue, Pavol signs each.
6. **Comparison content:** a short "MacDirStat vs DaisyDisk vs GrandPerspective"
   table in the README (honest: DaisyDisk is more polished/paid; GrandPerspective
   is older/unmaintained-ish; MacDirStat is free + actively developed + does
   one-click cleanup). Comparison pages are what people link to.
7. **Ship visibly:** a tight CHANGELOG and a steady release cadence signal an
   alive project (Homebrew and humans both weight "maintained").
8. **A tiny product page** (GitHub Pages) — also unlocks Homebrew's
   "has its own website" notability waiver.

### What NOT to do
Buying stars, star-for-star rings, asking for upvotes, or sock-puppet posts —
all violate the flywheel disclosure ethics, and Homebrew/Reddit/HN actively
detect and penalize them. Notability earned this way is worthless to the cask
reviewers anyway.

---

## Milestones / definition of done

| Milestone | Signal | Gate it unlocks |
|---|---|---|
| M1 | Notarized release, opens clean on Apple Silicon | Removes the hard cask blocker |
| M2 | README has GIF + screenshot; AlternativeTo + 3 awesome-lists live | Evergreen discovery on |
| M3 | ≥75★ / ≥30 forks / ≥30 watchers, sustained | Meets **standard** cask notability |
| M4 | `homebrew/cask` PR merged (ideally a non-author submitter) | `brew install --cask mac-dir-stat` |

Track stars/forks/watchers over time in the flywheel KPI pull; the in-app
flywheel SDK (`src/flywheel.rs`) covers the *usage* side (opens, scans,
space reclaimed) once the analytics endpoint is baked into the release.

---

## Honest read

The fastest realistic path is **M1 (notarize) → one launch post that lands →
M2 evergreen listings → coast to 75★ over a few weeks → M4 third-party cask
submission.** If a Show HN or r/DataHoarder post goes big, the "significant
social-media attention" waiver can collapse M3 entirely. If launches fizzle, the
personal tap remains a perfectly good distribution channel — homebrew/cask is the
prize, not the only door, and the kill-criterion on the flywheel record
(zero install growth for two quarters → sunset) is the honest backstop.
