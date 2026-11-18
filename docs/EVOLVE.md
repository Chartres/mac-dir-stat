# mac-dir-stat — evolve ledger (`/flywheel:evolve`)

Dated sections, newest first. Each run scores the previous run's picks, then proposes new
ones. Corrections stay visible.

## 2026-11-18 — first run (overnight flywheel pass)

### §0 Gate
Kill criterion (locked): *two consecutive quarters with flat/declining GitHub stars AND no
growth in weekly scan-completers → revert to maintain.* Deadline **2026-12-19**, 31 days out.
No previous evolve run to score.

- Stars leg: 2 stars in total, one on 2026-09-06 (Q3) and one on 2026-10-31 (Q4 to date).
  That is flat at +1 per quarter. The leg reads as met. Source: GitHub stargazers API, 2026-11-18.
- Scan-completer leg: not measurable for either quarter. `scan_completed` rows exist only from
  2026-11-01 (D3). Proxy: weekly app openers rose from 8–11 (Sep–Oct) to 11, then fell to 5
  and 3 in the two weeks after 0.7.0 shipped (D2).

**Recommendation: revert to maintain on 2026-12-19** unless idea #1 below shows the
November drop is a measurement fault and idea #2 or #3 moves a number before then.
For: the stars leg is met at 2 stars after 6.5 months, and the 75-star homebrew/cask bar is
37× away. Against: the second leg was not measurable, so the criterion was not tested as
written; the app has steady low organic downloads (D1) and real users freeing tens of GB (D4).
Maintain is not deletion: the release, the cask and the site stay up. Pavol decides via `advance.md`.

### Correction
- The portfolio record's `next_gate` says "scan-completers fell 40→15 per 28d". That number
  cannot be scan-completers: no `scan_completed` row reached the table before 2026-11-01
  (D3). It was most likely visitors or `app_open` counts. Treat it as withdrawn.

### §1 Discover
Usage SQL (read-only, flywheel-core, run 2026-11-18): the 60-day row dump from
`scripts/evolve-signals.mjs` and the weekly rollup in the PR body.

- **D1** Release downloads, cumulative to 2026-11-18 (GitHub releases API): v0.7.0 21
  (since 11-07), v0.6.0 25 (10-17 → 11-07), v0.5.1 62 (since 06-18), all versions 142.
  About 1 download a day on each release.
- **D2** Weekly distinct visitors (any event), weeks starting Mon, Prague-agnostic UTC
  `date_trunc`: 09-07 8 · 09-14 9 · 09-21 6 · 09-28 8 · 10-05 8 · 10-12 11 · 10-19 8 ·
  10-26 11 · 11-02 5 · 11-09 3 · 11-16 0 (Mon–Tue only). No anomaly bursts: max 2 new
  visitors in any hour.
- **D3** `scan_completed` has been in the app since 0.5.1 (2026-06-18) but the first row is
  2026-11-01, the day signals allow-list v2 went live (flywheel #117). Before that the event
  was dropped server-side. Weekly scan-completers since: 1 (11-01 only) · 4 · 3 · 0 (partial).
- **D4** Conversion (trash/free bytes), 60 days to 2026-11-18: 10 of 54 visitors converted
  (51 conversion events). Median single trash 2.4 GB (0.5.1 duplicates included, D8); one 66.6 GB batch of 158 items (10-30). The aha moment
  works for the people who reach it.
- **D5** 0.7.0 has 21 downloads and 3 visitors sending events (11-11, 11-12, 11-14). 0.6.0
  had 25 downloads and 20 sending visitors. Telemetry code, bake and opt-out did not change
  between v0.6.0 and v0.7.0 (`git diff v0.6.0 v0.7.0 -- src/flywheel.rs` empty). Unexplained:
  either 0.7.0 installs do not launch, or most downloads are not people.
- **D6** No 0.5.1 or 0.6.0 events after 2026-11-05, including visitor a67fb668 who scanned
  daily 11-02 → 11-05. Upgrades to 0.7.0 keep the same `visitor_id` file, so a returning
  upgrader should appear under 0.7.0 with an old id. None did.
- **D7** Measurement defect: `scan_completed` fires on a failed scan. Visitor 51c65246 sent two
  with files=0, dirs=0, errors=1 on 11-14 (`src/app.rs` `ScanProgress::Done`). Counts of
  "scan-completers" must filter `files > 0` until the app stops sending these.
- **D8** 0.5.1 logged duplicate conversions: the same `freed_bytes` twice within seconds
  (e.g. 2,415,919,104 at 07:07:52 and 07:07:58 on 09-25). Not seen in 0.6.0/0.7.0 rows.
  Conversion bytes from 0.5.1 are overstated.
- **D9** Repo traffic, 14 days to 2026-11-18: 18 views / 16 uniques; referrers
  chartres.github.io 4, mac-dir-stat.pages.dev 3, Bing 1. No inbound from any list or forum.
- **D10** Homebrew: the cask lives in the personal tap `Chartres/homebrew-mac-dir-stat`, which
  has no install analytics. `formulae.brew.sh/api/cask/mac-dir-stat.json` does not exist.
- **D11** Peer Homebrew installs, 30d / 365d (formulae.brew.sh, 2026-11-18): dust 5,988 /
  36,633 · ncdu 2,350 / 36,356 · GrandPerspective 1,354 / 14,596 · DaisyDisk 494 / 8,302 ·
  OmniDiskSweeper 471 / 6,939 · Disk Inventory X **1** / 1,761. CLI tools out-install every
  GUI. Disk Inventory X has nearly stopped (90d 49).
- **D12** Answer engines: a web search for "what is using my Mac's disk space best app 2026
  treemap free" names GrandPerspective as the free treemap, then Disk Inventory X, Radix
  (open source, has scan comparisons), DaisyDisk (paid), and macOS Storage settings. Sources
  are listicles and vendor blogs (cindori.com, cleanmymac.com, nektony.com, alternativeto.net).
  mac-dir-stat appears in none.
- **D13** Voice: no feedback rows, 0 issues, 0 forks, 0 watchers. Open PRs are sweep PRs
  #19, #24, #25 (maintenance). `scripts/stage-awesome-pr.sh` (awesome-rust) is staged and has
  not been run: no launch-log channel attempt found.

They-have / we-have:

| | DaisyDisk | GrandPerspective | OmniDiskSweeper | Disk Inv. X | dust/ncdu | macOS Storage | mac-dir-stat |
|---|---|---|---|---|---|---|---|
| price | paid | free (paid on App Store) | free | free | free | built in | free, open source |
| map | sunburst | treemap | list | treemap | text bars | categories | treemap + ledger |
| "safe to delete?" | no | no | no | no | no | partly (recommendations) | yes: rebuildable vs only yours |
| history of scans | no | no | no | no | no | no | yes, small multiples |
| listed by answer engines | yes | yes | yes | yes | yes (CLI) | yes | no |

Telemetry gaps: scan-completers before 11-01 (D3); why 0.7.0 downloads do not send (D5);
upgrade path visibility (D6); Homebrew installs (D10).

No-regret fixes: none shipped in this run. D7 is a one-line guard but needs a test and the
macOS CI gate; it is idea #4 below. The run was time-boxed.

### §2 Define
- HMW find out whether 21 downloads of 0.7.0 are people who could not launch it? (D5, D6)
- HMW count scan-completers so the gate leg can be read at all? (D3, D7)
- HMW get named when someone asks an assistant what fills their Mac? (D11, D12, D9)
- HMW reach Disk Inventory X users now that it has stalled? (D11, D12)
- HMW turn "rebuildable vs only yours" into the reason someone picks this over GrandPerspective? (D4, D12)

**Binding constraint: reach.** 16 unique repo visitors in 14 days and no listing anywhere
(D9, D12). Retention is not binding at under 10 people a week. This matches the kill
criterion, which is about stars (reach). Measurement (D5) binds first in one way: until it is
known whether 0.7.0 launches, any reach test is read against a broken counter.

### §3–4 Develop and deliver

| # | Idea | Kind | HMW | Signal | Imp | Conf | Eff | Rev | First cheapest test |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Clean-account launch check of the notarized 0.7.0 DMG and the cask install; confirm `app_open` lands | adjacent | 1 | D5, D6 | 4 | M | S | 2-way | one run on a fresh macOS user, query the row |
| 2 | Stop `scan_completed` on empty/failed scans; add `version` to every event | adjacent | 2 | D3, D7 | 2 | H | S | 2-way | unit test on the guard, red → green |
| 3 | Correct the portfolio record's "40→15" line; gate SQL filters `files > 0` | adjacent | 2 | D3 | 2 | H | S | 2-way | data PR in flywheel |
| 4 | Run the staged awesome-rust PR (`scripts/stage-awesome-pr.sh`), plus an alternativeTo entry under Disk Inventory X and GrandPerspective | adjacent | 3, 4 | D9, D11, D13 | 3 | M | S | 2-way | Pavol signs off the two listings |
| 5 | `mac-dir-stat inventory --json PATH`: headless CLI that prints rebuildable vs only-yours with sizes, shipped in the same cask | mid | 3, 5 | D11, D12 | 4 | M | M | 2-way | CLI subcommand on the existing scanner; count `key_action: cli_inventory` |
| 6 | Uses the incumbent: a "Show in mac-dir-stat" handoff from macOS Storage (a Finder Quick Action on any folder) | mid | 5 | D12 | 2 | L | M | 2-way | Quick Action script in the DMG |
| 7 | Rebuild-cost recipes: for each rebuildable item, the exact command to get it back (`brew reinstall`, `npm ci`, Xcode DerivedData) | mid | 5 | D4 | 3 | M | M | 2-way | add to the inventory column for the top 5 kinds |
| 8 | Disk Inventory X migration page on the site: "Disk Inventory X stopped; here is the same map with notarization and Apple Silicon" | mid | 4 | D11, D12 | 3 | M | S | 2-way | one page + llms.txt line; watch referrers |
| 9 | Founder lens: a "before macOS upgrade" check (Tahoe needs ~25 GB free) as the seasonal peak, shown on launch when free space is low | mid | 5 | D4 | 2 | L | M | 2-way | copy-only banner when free < 30 GB |
| 10 | Post-AGI lens: an MCP server (`mac-dir-stat mcp`) that exposes scan + inventory, so Claude/other assistants answer "what's using my disk" from it | radical | 3 | D11, D12 | 4 | L | M | 2-way | stdio MCP over the CLI from #5; one sign-off'd post |
| 11 | Remove: drop the headless `serve` web UI and its token server; it has no events and doubles the surface | radical | 5 | D13 | 1 | M | S | 1-way | grep events for any serve use (none logged); deprecate in CHANGELOG |
| 12 | Fringe/radical: upstream the rebuildable/only-yours classifier as a contribution to GrandPerspective or Mole, and keep mac-dir-stat in maintain | radical | 3 | D11, D12 | 3 | L | M | 1-way | draft an upstream issue for Pavol's sign-off |

### Top 3, with kill tests
1. **Quick win — #1 launch check.** By **2026-11-25**: if a clean-account install of the
   notarized 0.7.0 DMG produces `app_open` and `scan_completed` rows, D5 is "downloads are
   not people" and closes. If it produces no row, it is a defect and blocks #2–#3 below.
2. **Mid bet — #5 `inventory --json` CLI, listed with #4.** Ship by **2026-12-05**; kill on
   **2026-12-19** if fewer than 3 distinct visitors log `cli_inventory` and stars are still ≤ 3.
3. **Radical probe — #10 MCP server.** Only after #5. Kill on **2026-12-19** if, after one
   sign-off'd post, fewer than 3 distinct visitors call it. If it dies, the gate's maintain
   default stands.

### §5 Questions for Pavol
1. Gate: do you accept "revert to maintain on 2026-12-19" as the default, given the stars leg
   is met and the scan-completer leg was not measurable? (silence = yes)
2. May the awesome-rust PR and two alternativeTo entries go out (outward; #4)? (silence = no)
3. Is a CLI/MCP shape inside this product's red lines, or does mac-dir-stat stay a GUI-only app?
