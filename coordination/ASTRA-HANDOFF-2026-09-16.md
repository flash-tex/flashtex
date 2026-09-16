# FlashTeX — session handoff, 2026-09-16 ~04:00Z

Written by orchestrator-astra (linux-primary) near the end of its usage budget.
The owner explicitly authorised working past the 96% weekly ramp-down line.

## Headline

- **main = `18eb1892`**, 114 commits merged in the last 26 hours, **76 PRs open** (from 219 at session start).
- **Rust side verified green in release**: compiler 910, render-pipeline 478, pdf 122,
  math-layout 74, tex-expansion 92, flashtex-cli 58 — **1734 tests, 0 failures**,
  compile-checked (`--no-run`) first.
- **NOT released.** `v0.1.5` is the latest release and points at `fe9bc430`
  (2026-09-15 15:07 EDT). **36 commits have landed on main since**, and none of
  this session's work is in any release — verified: the `\ne` fix, the vim
  dispatch fix, the zoom-HUD removal and the page window are all absent from
  `v0.1.5`. 0.1.6 was not cut. See "Why not released".

## Owner's feedback list — status

Landed: `\ne` rendering (#295/#296), floating zoom HUD removed (#669), syntax-highlight
recovery after select-all/delete/type (#673), autosave on by default (#675), preview
pinch-zoom (#674), entry document keeps its real name (#655), the whole appearance
overhaul (#653) plus 8 of 9 review follow-ups (#666), export routes 4 → 1 with Print
sharing Export's bytes (#671).

Vim: registers incl. the `"_` black-hole data-loss bug (#670), case ops + visual
`r u U D X C S R Y O` + `ip`/`ap` + `zt`/`zb` + `:N` (#672), insert-mode chords (#677),
marks with edit-tracking + jumplist (#685). Remaining, written and gated but unlanded:
text objects (#684), `gq` reformat, regex search/`:s`, visual block.

Not started: Emmet-for-LaTeX, user snippets/macros, multidirectory + `.sty` support,
`.flashtex` project format, choosing which file to preview, the deferred accent work
(outline row colours, current-line band, gutter severity).

## Why not released

1. **#681 — the Mac suite aborts.** `IMECompositionTests.testCancelledCompositionLeaves
   TheHelperAndLedgerUntouched` kills the suite with signal 6 (`swift_task_dealloc_specific`
   → `swift_Concurrency_fatalError`, inside `XCTSwiftErrorObservation`). **It is not a
   code regression**: a full-suite bisect showed `f0a38b73^` aborting identically, the
   crash signature first appears 45 min before that merge window, and #677's own
   known-good tree (which passed 1395 tests at 22:57) now fails. The change was never in
   the repository. Leading theory: machine state — two long-lived FlashTeX instances each
   with their own edit-ledger, plus **50 orphaned test helpers** from dead agent worktrees
   that were spawning *during* lock-held runs. I killed all 50; only the owner's app
   (pid 40530, unsaved buffer) remains. **A fresh full-suite run on main, in its own
   worktree, may now simply pass — do that first.**
2. Two live bugs found but unfixed: **⌘⇧J "Reveal Caret in Preview" fails on every
   compile** (`Navigation.swift` has no v2 branch; `result.pages` is empty under v2-only),
   and **the page number is invisible on defaults** (readout gated `if !model.previewV2`
   while `previewV2` defaults true; `toolbarPageCount` reads v1 pages = 0). Fixes are
   drafted as A1/A2 by the v1-removal agent.

## Blocked on the owner

- **`workflow` token scope.** `gh auth refresh -s workflow` unblocks **#659** (CI artifact
  upload), **#660** (snapshot env gating) and the CI job-count reduction. Current token:
  `gist, read:org, repo`.
- **0.1.6 release decision.** Tag `v0.1.5` exists at `fe9bc430` and is inert.

## Landmines — read before touching anything

- **One worktree per test run.** Two concurrent runs in `/private/tmp/ft-ime` raced and
  produced a false result I reported to the owner before catching it. This same class of
  error produced two false "green" reports earlier in the session.
- **`cargo test --no-run` first, always.** A Rust target that fails to compile emits no
  `test result:` line, so summing them reads green. This broke main twice (#664, #638).
- **Run Rust in `--release`.** Issue #667: render-pipeline output differs between debug
  and release (colorbox path count 2 vs 1). Debug is not a valid oracle.
- **Splitting a PR: apply a diff, never `git checkout <branch> -- <files>`.** That copies
  files wholesale and silently discards main's later changes — it is why #683 does not compile.
- **Squash-merge + stacked PRs.** Merging a parent with `--delete-branch` auto-closes any
  PR based on it (this happened to #656). Retarget children to main *before* merging the parent.
- **The Mac gate lock** (`/tmp/ftgate.lock`, atomic `mkdir`, 75-min stale breaker) exists
  because three concurrent suites drove load to 196 and manufactured failures (8/20/38
  across runs of the *same commit*, differing subsets). Use it.
- **CI truth**: the Actions log is truncated by `grep | tail`; the authoritative output is
  the `mac-swift-test-log` artifact. Main's `mac app` check is red for ~19 pre-existing
  environment reasons — diff failing test names against the baseline before blaming a branch.

## Open issues filed this session

#667 debug/release output divergence · #678 marks don't track plain insert-mode typing ·
#680 RuntimeTranscriptTests contention flake · #681 the Mac suite abort · #650 blank_comments ·
#662 main's CI health.

## Agents

Two live at handoff, both with merge authority under strict gates: the vim-parity agent
(4 branches gated, visual block written) and the v1-removal agent (A1/A2 drafted, owns
#681 and #683). Both were told: green locked gate, failures *explained* not assumed,
squash only, no `--delete-branch` on anything stacked.

**No live Commander successor exists.** Jaysen is dead per the owner; Daniel has been
silent since 2026-09-15T13:01Z; all Remote Control peers offline. The seat is published
as OPEN — see `docs/commander-failover.md`.
