# mac-live: native engine host and helper crash/restart, packaged capture-cycle acceptance

Owner: `mac-validation` (Claude Code subagent, parent `mac-claude-a`, machine
`mac-m1max-a`). Status: runnable, first evidence committed under `reports/`.
Created 2026-09-12. Independent of the product sources: nothing under
`apps/mac` or `crates/` is modified; the runner only reads them from git and
executes the shipped scripts.

Retirement stage S6 (#1236) removed the old engine's routes from the app, and
with them this runner's typing bench (`tools/typing-bench`, which drove only
the old engine's routes), the packaged old-pipeline attach, the exact
export, the worker relaunch and multi-file helper-route cycles, and the
per-stage typing attribution (`lib/typing_attribution.py`, `quiet-window.sh`).
Their evidence stays under `reports/`. The engine's own keystroke-to-page bench is
`EngineV3Bench` (`FLASHTEX_V3_BENCH`, `apps/mac/Sources/FlashTeXMac/EngineV3Bench.swift`).

`run.sh` is one repeatable acceptance run (bash + python3, no new dependencies):

1. **Helpers from integrated main.** A shared clone of the repository pinned
   (detached) at the exact `<main-ref>` commit — a clean tree with no local
   edits and a truthful `git rev-parse HEAD` — then `cargo build --release`
   for `flashtex-bridge` and `flashtex-edit-ledger` (offline first, crates.io
   only as a fallback). The git SHA, as-shipped sha256 and signature-removed
   sha256 of each binary are recorded.
2. **Engine host, project files and app from the branch under test.** The
   engine host (`flashtex-host`, `crates/flashtex-engine`) and
   `flashtex-project-files` are each built in their own clone pinned at the
   branch commit (`make-app.sh` stages the host's string pool and licence
   from the app's clone, so the host is built from the same commit); a second
   pinned clone at the branch commit runs `swift build -c release`. Default
   branch `origin/agent/mac-claude-a/mac-shell`, default main `origin/main`.
3. **Package + launch check.** `apps/mac/scripts/make-app.sh --bridge …
   --ledger … --project-files … --engine-host … --require-engine-host`, then
   the unmodified `apps/mac/scripts/launch-check.sh` (engine host and bridge
   children started, each killed in turn, app survives, the host's exit and
   restart and the bridge's exit logged, clean quit). `lib/open-shim/open` is
   put first on `PATH` and adds `--env FLASHTEX_NO_ACTIVATE=1` (window ordered
   back, never activated) and a private `FLASHTEX_BRIDGE_STORE`. The run
   refuses this step while another `FlashTeX.app` is running, because
   `launch-check.sh` would `pkill` it (`--force` overrides).
4. **Packaged capture cycle.** `lib/capture_cycle.py` drives the helpers
   *inside the produced bundle* over their real JSON Lines protocols in the
   shell's order: `document_open` -> `destination_pin` -> `capture_submit`
   (`protocol/fixtures/capture-submission.json`) -> duplicate and conflicting
   retries -> `capture_convert` (refused `provider_disabled`: no
   `--enable-grok`, no key — the expected refusal) -> `capture_status` ->
   `capture_prepare_insert` (refused `proposal_missing`) -> SIGKILL + restart on
   the same journal -> `capture_reject` (terminal). Offline proposal review of
   `fixtures/capture-proposal.json` (bounds checked, explicitly not a provider
   result), then durable insertion through `flashtex-edit-ledger`
   (`initialize`, `apply`, identical retry, `status`, SIGKILL + restart).
4b. **Windows.** `open-window` launches the bundle with
   `FLASHTEX_OPEN_WINDOW=a11y-help` / `nearby`, finds the window through a
   CGWindowList probe (`lib/window_probe.swift`) and captures it by id with
   `screencapture -x -l` (never activated).
4c. **Branch XCTests against the real helpers.** Save routing to a member (⌘S),
   reviewed reload after an external edit and the on-disk conflict refusal
   need the Save menu / reload sheet, which cannot be driven without
   Accessibility; `swift test --filter 'ProjectDocumentsTests|DocumentFilesTests'`
   runs those paths with `FLASHTEX_*` pointing at the built helpers; per-test
   results (including skip reasons) are gated (`--skip-tests` to leave it out).
5. **Report.** `lib/report.py` writes `reports/<UTC>.md` with every number,
   binary hash, source SHA, machine/OS/Xcode/Swift/cargo version, the driving
   agent session and the exact commands (`reports/<UTC>/commands.log`), and
   asserts `thresholds.json`. Exit 0 only when every gate passed.

## Usage

```sh
tools/native-validation/mac-live/run.sh                      # full run, defaults
tools/native-validation/mac-live/run.sh --branch origin/agent/x/y --main-ref origin/main
tools/native-validation/mac-live/run.sh --skip-launch                # cycle, windows and tests only
FLASHTEX_MAC_LIVE_SESSION=<session url> FLASHTEX_MAC_LIVE_AGENT=<agent id> tools/native-validation/mac-live/run.sh
```

Options: `--no-fetch`, `--skip-cycle`, `--skip-features`, `--skip-tests`,
`--rebuild` (discard the scratch helper/app builds), `--force` (run
launch-check even if a FlashTeX.app is running), `--work <dir>` (scratch root;
default `build/`, gitignored), `--session`, `--agent`.
Scratch builds are keyed by SHA (`build/helpers/<main sha>`, `build/app/<branch
sha>`, `build/engine-host/<branch sha>`, `build/project-files/<branch sha>`,
shared `build/cargo-target`) and reused across runs.

## Thresholds (`thresholds.json`)

Gates fail the run; informational lines are reported only.

| section | gate |
|---|---|
| build | helpers built from main (pinned clone, clean); `flashtex-host` built from the branch (pinned clone HEAD == SHA); app built; bundle has `FlashTeX`, `flashtex-bridge`, `flashtex-edit-ledger`, `flashtex-host` (`Contents/Helpers`); bundled helpers, host and app carry the freshly built object code (signature-masked Mach-O content sha256 equal; make-app.sh re-signs the bundle ad hoc); `components.json` SHAs equal the main/branch SHAs |
| launch-check | zero `FAIL:` lines; engine host started (log and child process), app survives the host's kill, its exit and restart logged; bridge attached, `bridge: attached:` logged, app survives bridge kill, `bridge exited (` logged; clean quit; launched with `FLASHTEX_NO_ACTIVATE=1` |
| launch-check informational | on-screen window confirmed (CGWindowList); bridge re-attached after kill (the shell has no bridge auto-relaunch) |
| capture-cycle | every driver check (durable receipt, duplicate/conflict, `provider_disabled`, `proposal_missing`, journal survives SIGKILL, reject terminal, `capture_missing`, ledger receipt + identical retry + pending receipt + survives SIGKILL); no provider enablement |
| open-window/* | app running; an on-screen window titled "Accessibility Help" / "Nearby Companion" owned by the app; `screencapture -l <id>` wrote a PNG > 4 KB and > 100x100 px |
| app-tests | `swift test` exit 0 with 0 failures; the listed ProjectDocuments/DocumentFiles tests passed (not skipped) |

## Layout

- `run.sh` — orchestrator; `lib/report.py` — report + gates; `lib/launch_summary.py`
  — parses launch-check evidence; `lib/capture_cycle.py` — packaged capture
  cycle; `lib/app_features.py` — headless secondary-window check;
  `lib/hashes.py` — as-shipped, signature-removed and signature-masked sha256;
  `lib/open-shim/open` — adds `--env` values to `open`.
- `lib/window_probe.swift` — CGWindowList window list for a pid;
  `lib/xctest_summary.py` — `swift test` log to JSON;
  `lib/test_engine_labels.py` — unit tests of the engine label in
  `launch_summary.py`.
- `fixtures/capture-proposal.json` — offline `capture_proposal` fixture.
- `reports/<UTC>.md` + `reports/<UTC>/` — committed evidence: `env.json`,
  `helpers.json`, `extras.json`, `app.json`, `bundle.json`, `steps.jsonl`,
  `commands.log`, `logs/`, `launch-check.md/json`, `capture-cycle.json` +
  `capture-cycle/` (transcript). Reports before S6 also carry the old
  engine's typing bench, render, export and attribution evidence
  (`reports/attribution-<UTC>/`).

## Limitations

- The launch check exercises child crash and app survival: the app restarts
  the engine host itself, but has no automatic bridge relaunch, so for the
  bridge "restart" means the app keeps running and reports the exit;
  reattachment is a user action. On a Mac without TeX Live the engine waits
  for the TeX files' download consent instead of starting its host, so the
  host notes are absent and their gates fail.
- The capture cycle exercises the bundled helpers and the shell's protocol
  sequence, not the SwiftUI review sheet (no Accessibility, no activation). The
  shell's review/insertion code is covered by `swift test` on the branch.
- No provider call is made; the reviewed proposal is a fixture. Real conversion
  needs `--enable-grok` and the Mac credential adapter, outside this runner.
- `make-app.sh` re-signs the bundle ad hoc, so bundled binaries are compared to
  the built ones by a signature-masked Mach-O content hash (`lib/hashes.py`:
  bytes before the `LC_CODE_SIGNATURE` blob with the two header fields a
  re-sign rewrites zeroed); `codesign --remove-signature` output is recorded
  too but is not stable across re-signs. `components.json` SHAs come from the
  pinned clones and are gated against the main/branch SHAs.
- Other agents build and test on this Mac concurrently; the load average and
  `uptime` are recorded at start and end, not controlled.
