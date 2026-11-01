# Nightly design review 2026-09-30, track 3: process, CI, merge queue, coordination

Scope: DESIGN.md §9 (development process and CI), §12 (P0 exit gate items), §14 step 1
(audit). Read-only analysis; no PR, runner or queue state was changed.

- **Window:** 2026-09-28T05:36Z to 2026-09-30T05:55Z (48 h). The Actions API returned
  no CI runs before 2026-09-29T01:50Z, so the CI figures cover about 28 h.
- **Main at the time of writing:** `02dcf9d07` (#1243, merged 03:55Z). Nothing has
  merged since then (2 h).
- **Labels:** VERIFIED means measured with the command or API named. BELIEF means
  inferred and not proven.

## Method

| Data | Source |
|---|---|
| Merged PRs | `gh pr list --state merged --search "merged:>=2026-09-28T05:36:00Z"` (43 PRs) |
| Queue events | GraphQL `timelineItems` (`ADDED_TO_MERGE_QUEUE_EVENT`, `REMOVED_FROM_MERGE_QUEUE_EVENT`, force-push, ready-for-review), for every merged PR and every open PR ≥ #1183 |
| CI runs | `GET /actions/runs?created>=…` (741 runs) |
| CI jobs | `GET /actions/runs/{id}/jobs?filter=latest` (9,934 jobs) |
| Failure causes | Check-run annotations and job logs (`/actions/jobs/{id}/logs`) for a sample of failed jobs |
| Live state | GraphQL `mergeQueue(branch:"main")`, `GET /actions/runners`, ruleset 24166787 |
| Open PRs | `gh pr list --state open` (145 PRs), plus `/pulls/{n}` and `/pulls/{n}/files` for each |
| Coordination | Issue #2 comments since the window start (264) |
| This Mac | `git worktree list`, `du -sh`, `df -h`, launchd `com.flashtex.clean-worktrees` and its log, runner service logs |

## 1. Throughput

**Merged to main (VERIFIED):** 43 PRs.

- By lane: `agent/kabir-claude` 29, `agent/flashtex-2a` 8, `agent/daniel-muse-lead` 4,
  `agent/mac-claude-a` 2.
- By author: GoKubar 30, d-q222 11, jay3332 2.
- 34 merged after the ruleset `main: CI required + merge queue` became active
  (2026-09-29T08:14Z).
- 12 merged without a queue event:
  - 9 were merged directly before the ruleset: #1060, #1106, #1155, #1161, #1167,
    #1185, #1186, #1187, #1188;
  - 2 used the **admin bypass after the ruleset**: #1212 (merged 3.5 min after it was
    opened) and #1222 (opened and merged 5 s apart). Neither is mentioned on #2;
  - #1122 reached main inside #1183's merge group.
- A third bypass, #1206 (manual queue removal, then a merge 10 s later), *was* logged on
  #2 at 15:37Z.

**Latency (minutes, VERIFIED):**

| Measure | n | median | p75 | max |
|---|---|---|---|---|
| PR open → merged (all 43) | 43 | 141 | 469 | 8,354 |
| PR open → merged (after the ruleset) | 34 | 142 | 410 | 4,460 |
| First queued → merged | 31 | 75 | 121 | 466 (#1183) |
| Last queued → merged | 31 | 40 | 88 | 186 |

**Merge-queue removals (VERIFIED, merged PRs plus open PRs ≥ #1183):**

| Reason | Count |
|---|---|
| merged | 30 |
| `failed_checks` | 17 |
| `invalid_merge_commit` | 3 (#1230, at 02:00, 02:10 and 02:20Z; cause not determined) |
| manual | 2 (#1183, #1206) |
| `checks_timed_out` | 1 (#1199) |
| `merge_conflict` | 1 (#1245, on DESIGN.md after #1243 landed) |

- **Re-queues:** 23 manual re-adds across 47 queued PRs.
- **Evicted more than once:** #1183 ×3, #1198 ×3, #1209 ×3, #1230 ×3, #1226 ×2.
- **Force-pushes:** none recorded on any of these PRs (VERIFIED, `HeadRefForcePushedEvent`).

**Causes of the 40 failed `merge_group` runs** (VERIFIED, by failed job and runner):

- **31 of 40 (78%) failed only on self-hosted macOS jobs.** 28 of those ran on this Mac's
  runners (`Kabir's MacBook Pro`, `Kabir-MBP-2`, `Kabir-MBP-3`). The 3 others were the
  first jobs on `mac-m1max-a` and `mac-m5pro-dq222`.
- **9 involved a GitHub-hosted job:**
  - 5 on hosted "mac app";
  - 3 on hosted Linux `quick`/`rust workspace`;
  - 2 on hosted iPad jobs.

  (Some runs had more than one failing job, so these add up to more than 9.)
- Examples of hosted failures unrelated to the PR (VERIFIED from logs):
  - #1244 (one docs file) failed "mac app" on `Failed to CreateArtifact … Request timeout`;
  - #1232's "mac app" exited 1 after `1701 tests … 2 failures (0 unexpected)`.

  So flaky or infrastructure failures, not code, dominate evictions.

**Live queue now (VERIFIED, GraphQL at about 05:50Z):**

- 16 entries. The head (#1239) has been `AWAITING_CHECKS` since 03:41Z, and GitHub
  estimates 5.9 h until the last entry merges.
- **One group is already known to fail but still waits.** In the head group's current run
  (`36672253457`), `rust workspace (ubuntu-latest)` failed at 05:32Z: 3 targets of
  `flashtex-rendering-core`, on a PR that only touches `tools/lockstep`. The run still
  waits on three self-hosted Mac jobs that have been `queued` since 05:11Z.
- Ruleset settings:
  - `max_entries_to_build` 3;
  - `grouping_strategy` ALLGREEN;
  - `check_response_timeout_minutes` 240;
  - `required_approving_review_count` 0;
  - one required check (`CI required`).

## 2. CI

**Wall time per run** (VERIFIED; from run creation to the last job's completion; CI
workflow only):

| Tier | Period (UTC) | n | median | p75 | p90 | max | Outcomes |
|---|---|---|---|---|---|---|---|
| PR (`pull_request`) | 09-29 06:49–18:00 | 64 | 25 | 37 | 55 | 89 | 42 ok / 4 fail / 18 cancelled |
| PR | 09-29 18:00 – 09-30 06:00 | 71 | **7** | 10 | 18 | 34 | 53 / 7 / 11 |
| `merge_group` | 09-29 06:49–18:00 | 40 | 62 | 80 | 106 | 141 | 21 / 13 / 6 |
| `merge_group` | 09-29 18:00 – 09-30 06:00 | 53 | **26** | 38 | 42 | 43 | 25 / 27 / 1 |

- **PR tier:** its median now meets the §9.2 target (≤ 10 min), but p90 is 18 min.
- **Pass rates (VERIFIED):** `merge_group` 46 of 93 completed runs (49%; 53% of
  success+failure). `pull_request` 120 of 173 (69%; 31 were cancelled by newer pushes).
  Main `push` 275 of 319.
- **Slowest jobs (median run time):**
  - `mac app` 16.6 min;
  - `rust workspace` macOS 12.9 min and Linux 11.8 min;
  - `rust render-pipeline` 6 min;
  - iPad 5.8 min.

  Everything else takes ≤ 3.6 min. The merge_group critical path is the Mac app plus
  the wait for a Mac runner.

**Queue wait against run time by runner kind** (VERIFIED, all CI jobs, minutes):

| Runner kind | Event | Busy minutes | Wait median | Wait p90 | Wait max |
|---|---|---|---|---|---|
| GitHub-hosted Linux | PR / merge_group / push | 3,424 / 3,503 / 3,034 | 0 | 1–3 | 13 |
| **GitHub-hosted macOS** | PR | 1,820 | 14 | **75** | 144 |
| GitHub-hosted macOS | merge_group | 2,657 | 7 | 53 | 101 |
| GitHub-hosted macOS | push | 1,974 | 16 | 71 | 123 |
| Self-hosted macOS | merge_group | 1,592 | 2 | 22 | 105 |
| Self-hosted Linux (nixos) | merge_group | 356 | 3 | 12 | 22 |

- **GitHub-hosted macOS:** 6,451 runner-minutes in about 28 h (VERIFIED).
- **Capacity notice:** GitHub now annotates runs with "Due to capacity constraints, jobs
  targeting macOS arm64 runners may experience longer queue times."
- **In the PR tier,** the GitHub-hosted macOS `parity fixtures` job waits p90 34.5 min.
  That wait alone keeps p90 above 10 min.

**Self-hosted runner reliability and use** (VERIFIED; `merge_group` jobs; busy time from
job start and end):

| Runner | Failed / total | Busy h | Active span h | Utilisation | Status now |
|---|---|---|---|---|---|
| Kabir's MacBook Pro | 46 / 134 (34%) | 8.0 | 12.9 | 62% | **offline** |
| Kabir-MBP-2 | 16 / 100 (16%) | 6.1 | 11.2 | 54% | **offline** |
| Kabir-MBP-3 | 17 / 100 (17%) | 5.5 | 11.1 | 50% | **offline** |
| mac-m1max-a (single, retired) | 2 / 26 | 2.1 | 4.6 | 47% | n/a |
| mac-m1max-a-1/-2/-3 | 0 / 47 | 2.5 / 2.4 / 2.2 | about 4.2 | 51–62% | online, all busy |
| mac-m5pro-dq222 (+-2, -3) | 4 / 4 (first run) | small | n/a | n/a | **offline (all 3)** |
| nixos-7800x3d (+-2, -3) | 0 / 176 | 3.8 / 2.4 / 2.2 | 9.1 / 4.4 / 4.3 | 41–55% | online, **idle** |

- **Failures by job, this Mac's runners only (merge_group):**
  - mac app 10/10 and iPad 8/10;
  - `rust workspace` 20/64, `rust flashtex-cli` 17/65, `rust render-pipeline` 10/64;
  - parity 12/67.

  The same Rust jobs on GitHub-hosted macOS: **0 of 76 failed.**
- **Failure signatures** (annotations on the failed jobs):
  - `An error occurred trying to start process …/externals/node24/bin/node with working directory …` (about 34);
  - `Process completed with exit code 101` (30);
  - `The self-hosted runner lost communication with the server` (4);
  - `…start process '/opt/homebrew/bin/bash' with working directory…` (4).

  BELIEF: this Mac also runs the Commander's agent fleet and its Cargo builds, so its
  runners are starved and hit races on the ephemeral working directory.
- **This Mac's runners are offline now.** Their service logs show SIGINT/SIGKILL at
  about 04:43–04:52Z, and the launchd agents are not loaded. That leaves 3 online Mac
  runners for the full tier, and those 3 are busy.
- **Flaky tests** (VERIFIED, the same head SHA and job with both a failure and a pass):
  - `quick (touched crates)` 3, `parity fixtures (self-hosted)` 3, `mac app` 2;
  - `trip`, `etrip`, `rust workspace`, `rust flashtex-cli` and `rust render-pipeline` 1 each;
  - `CI required` 47 (the aggregator, as expected).

  Failures unrelated to the PR's files: #1244 (a docs file) on mac app; #1239
  (`tools/lockstep`) on the `flashtex-rendering-core` targets. The latter is either a
  flake or a latent main regression; the same PR passed its 04:06Z group (BELIEF: flake).

## 3. Stale work

**Open PRs (VERIFIED):** 145 open, 116 of them older than 24 h.

- 27 of the 116 are drafts. By GitHub's mergeable state: 70 `dirty` (conflicts), 40
  `blocked`, 3 `unstable`, 3 unknown.
- None of the 116 was updated in the last 24 h.
- Authors: d-q222 91, GoKubar 24, jay3332 1.
- Lanes: `agent/daniel-muse-lead` 83, `agent/daniel-parent` 8, `agent/kabir-claude` 8,
  others 17.

**Close under D13** (old-engine feature or hand-port work; each touches `crates/compiler`,
`render-pipeline`, `tex-expansion`, `vector-graphics`, `math-layout`, `paragraph-layout`,
`pdf`, `docstrip`, `project-files`, `microtype`, `rendering-core` or `perf-bench`). 108 PRs:

> 175, 194, 257, 261, 262, 266, 268, 270, 271, 273, 283, 286, 288, 289, 292, 293, 298,
> 299, 325, 332, 340, 363, 365, 373, 569, 582, 584, 599, 617, 627, 731, 962, 964, 966,
> 967, 968, 970, 971, 975, 977, 978, 979, 980, 981, 982, 983, 984, 985, 986, 987, 988,
> 989, 990, 994, 998, 999, 1000, 1001, 1018, 1021, 1024, 1027, 1028, 1029, 1030, 1031,
> 1034, 1035, 1036, 1038, 1040, 1041, 1042, 1043, 1044, 1045, 1046, 1050, 1051, 1053,
> 1080, 1082, 1083, 1085, 1086, 1087, 1091, 1097, 1102, 1103, 1104, 1111, 1112, 1120,
> 1133, 1136, 1140, 1145, 1148, 1150, 1151, 1152, 1157, 1158, 1159, 1164, 1173, 1181

This includes the `agent/daniel-muse-lead/*` run the prompt names (#1148–#1181).

- Keep the branches.
- Check two before closing:
  - **#1120** (rendering-core goldens re-gate): `flashtex-rendering-core` tests are
    failing in today's queue;
  - **#627:** its multicols panic fix was re-landed as **#1227**, which is in the queue
    as the approved D13 fix and must stay open.

**For the Commander to review** (not old-engine code, but stale):

- #1019 (old-engine visual-oracle evidence);
- #287 (old release gate);
- #84 and #101 (Beads coordination drafts from 2026-09-13);
- the Mac/iOS app PRs #1095, #1096, #1107, #1113, #1115.

**Stacked PRs** against §9.5 (VERIFIED, `base.ref` ≠ main):

- #1241 on #1230;
- #1254 on #1247;
- #268, #271 (bases are old branches);
- #373;
- #1133 and #1136 (on #1103);
- #1140 (its base, #1122, is already merged).

**Branches with no PR,** updated in the window (VERIFIED, `for-each-ref` against every
PR's head): 47. Excluding 4 queue refs and `coordination-claims`, they are:

- 10 M1 package batches (`agent/daniel-muse-lead/m1-packages-01..10`, handled on #2
  as data);
- 3 `typst-design-{a,b,c}` research branches;
- `p4-finish`, `p4-l1-image-state`, `p3-fonts-2`, `review-2026-09-30-t5`;
- `flashtex-2a/nightly-corpus`, `flashtex-2a/packages-tier`;
- 20 flashtex-2a old-engine fix branches from 2026-09-28, stopped by D13.

## 4. Coordination

**#2 volume (VERIFIED):** 264 comments and 381,234 characters in 48 h, about 5.5
comments an hour.

- By author: d-q222 228 (86%), GoKubar 28, jay3332 8.
- 82 are review verdicts; the longest is 9.9k characters.
- 7 are "START NOW" assignments between Daniel's two sessions, and 5 are self-corrections.

**MERGE-READY → queued wait** (VERIFIED; the #2 verdict time against the next
`AddedToMergeQueueEvent`):

| PR | Wait |
|---|---|
| #1188 | 0.6 h (merged directly) |
| #1191 | 0.6 h |
| #1204 | 0.8 h |
| #1239 / #1240 | 1.2 h |
| #1233 | 2.2 h |
| #1196 | 2.7 h |
| #1227 (critical panic fix) | 4.9 h |
| #1210 | 7.6 h |
| #1224 | 9.1 h |
| #1213 | 9.2 h |
| #1211 | 9.5 h |

- **Median 2.4 h,** with 5 of 12 waiting more than 4.5 h.
- Landing was **delegated to flashtex-2a at 03:41Z**, and 6 PRs were queued within
  30 seconds. The waits were a Commander-attention bottleneck, not a CI one.

**Repeated failure patterns (VERIFIED on #2 and in the timelines):**

1. **Heads moved after MERGE-READY.** #1248 at 04:17Z and #1251 at 05:45Z and 05:50Z.
   Each move cancels a queued landing and costs a re-review round. flashtex-2a
   announced a "no push after MERGE-READY" rule at 05:45Z.
2. **Duplicate test cases across parallel waves.** #1251, #1253 and #1256 each got
   NEEDS-FIX for duplicates of #1233's cases. #1258 (a near-duplicate checker) followed.
3. **Parallel edits to DESIGN.md.**
   - 5 merged in the window: #1203, #1209, #1229, #1231, #1243.
   - 3 are open now: #1245, #1250, #1257.
   - #1245 was evicted with `merge_conflict` minutes after #1243 landed.
4. **More than 3 branches per machine in CI** (§9.5). This machine has 10 of the 16
   queue entries: #1230, 1232, 1244, 1245, 1246, 1247, 1249, 1252, 1255, 1257.
5. **Overlaps and duplicates:**
   - a second daniel-muse-lead instance (09-29 02:23Z);
   - lane clones with unpublished commits deleted twice (01:58Z and 02:21Z);
   - a duplicate enumitem gate (B4);
   - #1200/#1201 closed as duplicates;
   - P3-APP-V3 and P3-SOURCE-MAP re-scoped from mac-claude-a to this machine
     (03:10Z and 05:33Z).
6. **Lockfile conflicts:** none observed in the window.

## 5. Disk and machine health (this Mac, VERIFIED)

**Worktrees:** `git worktree list` shows 58 for this repository.

| Location | Count / size |
|---|---|
| `.claude/worktrees/` | 41 worktrees, 128 GB (`du -sh`); 13 locked |
| `/private/tmp/p2pt1` | 4 detached review worktrees at merged SHAs, 23 GB |
| `flashtex-cmd` | 36 GB |
| Volume | 519 GiB used of 926, 382 GiB free (58%) |

- **Five worktrees of already-merged lanes hold 93 GB:**
  - `retire-vendor` 43 GB (locked);
  - `gate-nested-worktree` 14 GB (locked);
  - `p1-tex82-core` 13 GB;
  - `p2-pdftex-core` 12 GB;
  - `p2-pt1-fixtures` 11 GB.
- **Other merged lanes that still have worktrees:**
  - `p3-images`, `p3-fonts`, `p3-displaylist`, `p3-distribution`;
  - `host-unify`, `p4-l5-restart`, `snapshot-bench`;
  - `ipad-deflake`, `spellcheck-deflake`, `quick-root-manifest`.

**The clean-worktrees schedule is installed but reclaims nothing.**

- launchd `com.flashtex.clean-worktrees` runs daily at 05:00 local (09:00Z).
- It has run twice (`runs = 2`, last exit 0). Both runs logged "worktrees removed: 0;
  build dirs deleted: 0; freed: 0 GB".
- Every candidate was skipped, as "active" (locked, an open cwd, or a file modified in
  the last 6 h) or as "uncommitted changes".
- The harness keeps finished agents' worktrees locked, and §9.7's "a lane removes its
  own worktree when it lands" is not being done.

## 6. Recommendations

C = a Commander refinement (§14 step 5). O = an owner decision.

| # | Change | Expected effect | Class |
|---|---|---|---|
| 1 | Take this Mac's three runners out of the full tier until they pass a burn-in (for example 20 consecutive green jobs), or put them on a separate label. Route Mac full-tier jobs to `mac-m1max-a-*`, the dq222 runners once they are back online, and the hosted fallback. | Removes 28 of the 40 failed `merge_group` runs in the window. The pass rate goes from 53% to about 85–90%, and most re-queues disappear. | C |
| 2 | Decide whether the Commander's own working Mac should host CI runners at all while it runs the agent fleet. | Settles the source of 79 of 85 self-hosted Mac failures. | O |
| 3 | Fail fast in `merge_group`: a small job that cancels the run as soon as any `needs` job fails. Also lower `check_response_timeout_minutes` from 240 to about 60. | A failing group leaves the queue in minutes, not after the slowest Mac job. Today's head group has waited 20+ min after a known failure. | C |
| 4 | Auto-queue: any lane owner may queue its own PR once MERGE-READY is posted on the exact head SHA and the PR checks are green. This extends the 03:41Z delegation to every machine, within the existing scope rules. | MERGE-READY → queued goes from a 2.4 h median (9.5 h max) to minutes. | C |
| 5 | Freeze the head after MERGE-READY: any push invalidates the verdict. Enforce it with the ruleset (`required_approving_review_count` 1, `dismiss_stale_reviews_on_push`, `require_last_push_approval`), or with a queue-time check that compares the head against the verdict SHA. | Ends the #1248/#1251 re-review loops. | C |
| 6 | Enforce §9.5's "≤ 3 branches per machine in CI" at queue time (coord.py or the plan job). | This machine has 10 of 16 entries now; one machine cannot starve the others' landings. | C |
| 7 | Once Mac capacity is back (≥ 6 healthy Mac runners), raise `max_entries_to_build` from 3 to 5. | About 1.6× queue throughput. It costs little while ALLGREEN is kept. | C |
| 8 | Close the 108 D13 PRs listed in §3 with a one-line D13 reason; keep the branches. Review the 9 "Commander review" PRs. | The open list drops from 145 to about 30 real PRs. Stale-PR noise stops hiding MERGE-READY work. | C (policy already approved: D13) |
| 9 | Serialise DESIGN.md: one design-edit PR in flight at a time, rebased by its author after each landing. Alternatively, batch design edits into the review commit (§14 step 6). | Ends `merge_conflict` evictions on DESIGN.md: 3 PRs are open on it now, and 1 has been evicted. | C |
| 10 | Log every admin bypass on #2 (#1212 and #1222 were not logged), or stop using the bypass for CI-only changes. | An audit trail for main. | C |
| 11 | Post review verdicts on the PR itself, and only a one-line pointer on #2. | #2 drops by about a third (82 of 264 are verdicts, many 4–10k characters). Every machine re-reads less. | C |
| 12 | Quarantine flaky tests: a test that fails and then passes on the same SHA twice in 7 days is listed in a quarantine file with an issue. It runs non-blocking in the merge queue and blocking in the nightly until fixed. Start with `mac app` (unexplained exit 1 with 0 unexpected failures), artifact-upload timeouts (retry), and `flashtex-rendering-core` on Linux. | Removes the 9 hosted-involved evictions that were not caused by the PR. | C |
| 13 | Take the PR-tier macOS `parity fixtures` job off hosted macOS (it waits p90 34.5 min): run it on hosted Linux if the fixtures tier is host-independent (BELIEF, to verify). Otherwise keep it hosted but non-blocking for the ≤ 10 min target. | PR-tier p90 goes from 18 min to about 10 min. | C |
| 14 | Allow self-hosted runners for `pull_request` from in-repo branches (never forks). | Removes the hosted-macOS wait from the PR tier. Changes an owner-approved §9.3 security rule. | O |
| 15 | Move more Linux full-tier jobs to the nixos runners (#1232, now in the queue); they sat at 41–55% utilisation and were idle while Mac jobs queued. | Less hosted-Linux dependence. Frees GitHub capacity. | C |
| 16 | Disk: have `clean-worktrees.sh` treat a worktree whose branch is merged into `origin/main` as idle regardless of mtime. Clear a stale harness lock only when no process holds the directory. Add `/private/tmp/*` review worktrees at merged SHAs. Enforce "a lane removes its own worktree when it lands". | Reclaims about 93 GB (merged-lane worktrees) plus 23 GB (`/private/tmp/p2pt1`) on this Mac now. | C |
| 17 | Parallel test-case waves (lockstep corpus): run the near-duplicate checker (#1258) in `gate.sh pr` for `tools/lockstep/cases/`. | Ends duplicate-driven NEEDS-FIX rounds (3 in the last 2 h). | C |

**Carry-over checks for the next review:**

- merge_group pass rate (target ≥ 85%);
- PR-tier p90 (target ≤ 10 min);
- MERGE-READY → queued (target median < 15 min);
- queue depth at checkpoints;
- open old-engine PRs (target 0);
- GB reclaimed by the daily clean.
