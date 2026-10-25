# Founder cases — real situations with known answers

Every sweep and release candidate checks these. Drafted 2026-10-21 from the repo's history; none is confirmed yet. A row becomes binding (and gets a CI check) when Pavol marks it confirmed. A case changes only with a written reason.

| # | Situation | Known right answer | Source | Confirmed |
|---|---|---|---|---|
| 1 | Scan a folder holding `original.bin` (4 000 bytes) and a hard link to it in `sub/hardlink` | Folder total is 4 000 bytes, not 8 000; the (dev, ino) pair is counted once | commit a176225, PR #4; test `walk_deduplicates_hard_links` in PR #10 | no |
| 2 | A file named `Příliš.pdf` is long enough to be truncated in the treemap or folder tree | Label is cut on a character boundary; the UI thread does not panic | commit f53ad79, PR #4 (fix by inspection; no test covers a non-ASCII label) | no |
| 3 | Cleanup scan sees `/x/MyLibrary/Caches` | Not a cleanup candidate; suffix rules such as `Library/Caches` match whole path components only | commit c9e4375, PR #4 | no |
| 4 | Cleanup scan of a home folder that contains `~/.Trash`, and the scan root itself | Neither `.Trash`/`.Trashes` nor the scan root is listed as a candidate | commits 7dbc9f7, 18c35de, PR #4 | no |
| 5 | Cleanup scan sees `~/Library/Containers/com.apple.iCloud.iCloudDrive` | Not suggested; it can hold files not yet uploaded | commit 14c1a3d, PR #7; test `classify_icloud_drive_not_suggested` | no |
| 6 | Move or plan targets `/USERS`, `/applications`, `/users/<me>/library`, `/USERS/Shared` or `/Volumes/ADATA SD810` | Refused as protected; the path is normalized to its on-disk case before the rules run | PR #6 H1 and real check; tests `case_folded_paths_are_still_protected`, `protected_paths_are_refused`; `/USERS/Shared` and the ADATA volume are from the PR #6 real check only | no |
| 7 | GUI Move to Trash on an item under `/System/…` or `/Users/Shared/…` after a whole-disk scan | Refused, and the status bar shows the protected-path reason (batch: first failure's reason) | commit de6c236, PR #7; tests `gui_trash_refuses_protected_path`, `gui_batch_trash_refuses_protected_paths` | no |
| 8 | An AI client using the MCP token proposes a plan, then tries to approve or reject it | Propose works; approve and reject return 403. Only the admin token approves | PR #6 H2 and real check; test `mcp_token_can_read_and_propose_but_not_approve` | no |
| 9 | Move a 4.5 MB scratch folder (3 files) to the exFAT disk `ADATA SD810` with link=true, then approve | Plan ends `done`; copy sits at `MacDirStat Offload/<original path>`, md5 matches through the symlink, original is in `~/.Trash`, no `.mds-partial-*` left | PR #5 and PR #6 real checks, run by hand on the Mac mini against the ADATA exFAT disk; CI cannot repeat them | no |
| 10 | Move to exFAT of a symlinked folder, a folder containing a symlink, or `/Applications/Safari.app` | Refused at propose; approving that plan returns 409 | PR #5 real check (by hand), decision 1; the relocate unit tests cover the exFAT symlink and package refusals | no |
