# SHARING.md — launch plan for MacDirStat

> Generated 2026-06-18 from the flywheel launch playbook. **Pavol drafts are
> below; Pavol signs and posts every one.** Claude does not post. Disclosure is
> non-negotiable on every post: *"I made this, it's free, open-source, and
> unofficial — feedback welcome."*

Positioning, one line: **the free, open-source WinDirStat/DaisyDisk for macOS —
a treemap that shows what's eating your disk, and trashes it in two clicks.**

Audience reality: this is a global English-first developer/power-user tool.
EN copy leads; a Slovak/Czech variant is a small secondary (local Mac/dev
communities), not the spine of the launch.

---

## 0. Pre-launch QA gate (do before Wave 0)

- [ ] **Real screenshot + a short demo GIF in the README** (today it's a
      placeholder). This is the single biggest lever on both stars and launch
      conversion — a treemap tool with no picture converts almost no one.
- [ ] First-run on a clean Mac: scan `~`, scan `/`, trash something, empty
      trash. No crash, no permission-prompt wall (TCC filter holds).
- [ ] The Gatekeeper story in the README is accurate for the build you ship.
- [ ] `brew install --cask chartres/mac-dir-stat/mac-dir-stat` works end to end.

---

## 1. Community-discovery checklist

Scored persona-fit × activity × promo-tolerance (0–3 each, max 9). Keep top 5;
friendliest is Wave 0. Read each venue's self-promo rules **and the last ~10
"I built…" posts' reception** before posting.

| Channel | Fit | Activity | Promo-tol | Score | Notes |
|---|---|---|---|---|---|
| **r/macapps** | 3 | 3 | 3 | **9** | Built for "I made a free Mac app" posts. **Wave 0.** Lead with the GIF + "free & open-source". |
| **alternativeto.net** | 3 | 3 | 3 | **9** | List as alternative to WinDirStat, DaisyDisk, GrandPerspective. Not a "post" — an evergreen listing that captures "WinDirStat for Mac" searchers forever. Do this in week 1 regardless of waves. |
| **r/DataHoarder** | 3 | 3 | 2 | **8** | People whose hobby is reclaiming disk. Frame: audit where TBs went. |
| **Hacker News (Show HN)** | 2 | 3 | 3 | **8** | Tryable without signup (it's a download) → Show HN-eligible. Rust + treemap + macOS is on-topic. Wave 2. |
| **r/rust** | 3 | 2 | 3 | **8** | "I built a WinDirStat for macOS in Rust + egui." Engineering angle; they reward the writeup, not the pitch. |
| **lobste.rs** | 2 | 2 | 2 | 6 | `show` tag; needs an invite. Good signal-to-noise. |
| **r/macOS** | 2 | 3 | 1 | 6 | Large but strict on self-promo; only after Wave 1 feedback has hardened the app. |

Evergreen, non-wave (do once, early):
- **AlternativeTo** listing (above).
- PRs to **awesome-lists**: `awesome-macos`, `awesome-rust`,
  `awesome-egui`, `awesome-macos-command-line` (if a CLI mode ever lands).
- Add **topic tags** on the GitHub repo: `macos`, `treemap`, `disk-usage`,
  `windirstat`, `daisydisk-alternative`, `egui`, `rust` (drives GitHub's own
  topic discovery).

---

## 2. Post drafts (Pavol's voice — edit before posting; one bespoke post per venue)

### Wave 0 — r/macapps

> **MacDirStat — a free, open-source treemap to see what's eating your disk**
>
> I kept hitting "your disk is almost full" and clicking around Finder trying to
> find the culprit. On Windows I'd have reached for WinDirStat; on Mac the good
> option (DaisyDisk) is paid, so I built my own.
>
> It scans a folder or a whole volume and draws a treemap — every rectangle is a
> file, area is its size — so the big stuff is literally the big stuff on screen.
> You can sort the folder list by size/items/recent, right-click → Open / Reveal
> / Trash, and there's a one-click cleanup for the usual regenerable junk (Xcode
> DerivedData, node_modules, target/, caches).
>
> Free, open-source (MIT), no ads, no account. It's unofficial and very much a
> personal project — I'd love feedback, especially on the first-run experience.
>
> Install: `brew install --cask chartres/mac-dir-stat/mac-dir-stat` · source +
> DMG: github.com/Chartres/mac-dir-stat
>
> [GIF]

### Wave 1 — r/rust (engineering angle)

> **I built a WinDirStat-style disk visualizer for macOS in Rust + egui**
>
> Treemap directory-size tool: parallel scan with `jwalk`, a flat arena-backed
> tree (NodeId = index, names interned in one `Vec<u8>`), squarified treemap
> rendered as a single batched `egui` mesh. The fun parts were keeping the UI at
> 60fps while a background thread streams scan progress over a crossbeam channel,
> and dodging macOS TCC permission prompts by pruning protected paths *before*
> traversal.
>
> Source is MIT. Happy to go into any of the internals — and PRs/criticism
> welcome.

### Wave 2 — Show HN

> **Show HN: MacDirStat – free, open-source treemap disk visualizer for macOS**
>
> A WinDirStat/DaisyDisk-style treemap for the Mac, in Rust. Scans a volume,
> shows where the space went, and trashes regenerable junk in two clicks. Free,
> MIT, no signup — download and run. Built it because every good Mac option was
> paid. Feedback very welcome, especially on correctness of the size accounting.

### Secondary — Slovak/Czech (e.g. local Mac/dev Discord, r/Slovakia tech threads)

> *Native-language variant of the Wave 0 post, same honesty block. Post only if a
> genuinely relevant local venue exists — don't force it.*

---

## 3. Timing

- One community per day, never simultaneous. Post only when 2–3h of reply
  availability follows.
- **Show HN:** Tue–Thu, 09:00–12:00 ET.
- **Reddit:** weekday evenings or Sunday; r/macapps and r/DataHoarder tolerate
  weekends well.
- AlternativeTo + awesome-list PRs: anytime in week 1, independent of waves.

---

## 4. Feedback-harvest plan

Within 24h of each post, capture every comment/DM/issue verbatim (quote, handle,
URL, date, persona guess) and classify → route:

| Class | Routes to |
|---|---|
| bug / confusion | GitHub issues — fix before the next wave |
| feature request | PRD / roadmap backlog, with source link |
| persona evidence | flywheel calibration inbox |
| channel evidence | launch-log verdict for that channel |
| objection | positioning notes (likely: "why not just DaisyDisk/GrandPerspective?") |
| praise with specifics | testimonial candidates (ask permission) |

Reply to every substantive comment. When a launch-sourced request ships, close
the loop in the original thread.

---

## 5. Launch-log plan

Append one entry per channel to `~/my-git/flywheel/data/launch-log/mac-dir-stat.jsonl`
(wave, language, reach, outcomes, verdict `repeat|maybe|dead`). For a desktop app
without web referrers, the measurable funnel is:

- **Reach:** post views / upvotes / comments (channel-reported).
- **Interest:** GitHub stars + repo unique visitors (GitHub Insights) in the 72h
  after the post.
- **Install proxy:** Homebrew tap analytics (`brew install` counts) + GitHub
  Release DMG download counts.
- **Activation proxy:** flywheel `scan_completed` events, and `conversion`
  (space reclaimed) events — the in-app SDK now emits these (see
  `src/flywheel.rs`); needs the analytics endpoint baked into the release build.
- **Retention proxy:** returning `app_open` visitors week-over-week.

The star-growth and Homebrew-acceptance roadmap lives in
[GROWTH.md](GROWTH.md).
