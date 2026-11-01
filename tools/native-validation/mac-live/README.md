# mac-live: native typing-to-paint, helper crash/restart and packaged capture-cycle acceptance

Owner: `mac-validation` (Claude Code subagent, parent `mac-claude-a`, machine
`mac-m1max-a`). Status: runnable, first evidence committed under `reports/`.
Created 2026-09-12. Independent of the product sources: nothing under
`apps/mac`, `crates/` or `tools/typing-bench` is modified; the runner only
reads them from git and executes the shipped scripts.

`run.sh` is one repeatable acceptance run (bash + python3, no new dependencies):

1. **Helpers from integrated main.** A shared clone of the repository pinned
   (detached) at the exact `<main-ref>` commit — a clean tree with no local
   edits and a truthful `git rev-parse HEAD`, the same isolation
   `tools/typing-bench/run.sh` gets from `git archive` — then `cargo build
   --release` for `flashtex-compiler`, `flashtex-pdf`, `flashtex-bridge`,
   `flashtex-edit-ledger` and `flashtex-preview-controller` (offline first,
   crates.io only as a fallback). The git SHA, as-shipped sha256 and
   signature-removed sha256 of each binary are recorded.
2. **App from the branch under test.** A second pinned clone at the branch
   commit; `swift build -c release`. Default branch
   `origin/agent/mac-claude-a/mac-shell`, default main `origin/main`.
3. **Typing bench.** The unmodified `tools/typing-bench/run.sh` from that branch
   runs `fixture`, `demo`, `body60k` at 30 ms and 0 ms with the step-1 compiler,
   recording keystroke -> paint p50/p95/p99/max, paints, coalesced and unpainted
   keystrokes, compile and render-pass times — on the direct worker route
   (producer `compiler`), with `flashtex-render` as a second direct producer
   (the bench's own `render` producer, `FLASHTEX_RENDER` pointing at the
   scratch build below), and on the durable helper route (producer
   `controller`: `FLASHTEX_PREVIEW_CONTROLLER=<flashtex-preview-controller>`
   exported, private `FLASHTEX_CONTROLLER_LEDGER_ROOT`), all in one bench pass
   (`--producers "compiler render controller"`), then the helper route again
   with `FLASHTEX_COMPLETED_SNAPSHOTS=1` (historical previews), classified with
   the branch's `docs/evidence/historical-preview-2026-09-12T1010Z/analyze.py`
   (current vs historical paints, historical lag, keystroke -> current paint).
   Every cell first waits for a quiet machine (bench `--quiet-load 8
   --quiet-wait 300`, overridable with `--quiet-load/--quiet-wait`); the bench
   records the 1-minute load before/after each cell and marks cells above its
   load limit (10) load-affected — latency gates are applied only to unaffected
   cells, the others are reported. `uptime` is recorded at start, before every
   pass and at the end.
3b. **Optional bundled routes.** `flashtex-render` from
   `origin/agent/mac-render-pipeline/unified` (`--render-ref`) and
   `flashtex-pdf-exact` from `origin/agent/mac-pdf/v2-adapter`
   (`--pdf-exact-ref`), each built in its own pinned shared clone, are handed
   to `make-app.sh --render … --pdf-exact …`. `--skip-extras` leaves them out.
4. **Package + launch check.** `apps/mac/scripts/make-app.sh` with the four
   step-1 helpers, then the unmodified `apps/mac/scripts/launch-check.sh`
   (compiler and bridge children attached, each killed in turn, app survives,
   both exits logged, clean quit). `launch-check.sh` calls `open` without
   `FLASHTEX_NO_ACTIVATE`, so `lib/open-shim/open` is put first on `PATH` and
   adds `--env FLASHTEX_NO_ACTIVATE=1` (window ordered back, never activated),
   a private `FLASHTEX_BRIDGE_STORE` and a `FLASHTEX_TRANSCRIPT`. The run
   refuses this step while another `FlashTeX.app` is running, because
   `launch-check.sh` would `pkill` it (`--force` overrides).
5. **Packaged capture cycle.** `lib/capture_cycle.py` drives the helpers
   *inside the produced bundle* over their real JSON Lines protocols in the
   shell's order: `document_open` -> `destination_pin` -> `capture_submit`
   (`protocol/fixtures/capture-submission.json`) -> duplicate and conflicting
   retries -> `capture_convert` (refused `provider_disabled`: no
   `--enable-grok`, no key — the expected refusal) -> `capture_status` ->
   `capture_prepare_insert` (refused `proposal_missing`) -> SIGKILL + restart on
   the same journal -> `capture_reject` (terminal). Offline proposal review of
   `fixtures/capture-proposal.json` (bounds checked, explicitly not a provider
   result), durable insertion through `flashtex-edit-ledger` (`initialize`,
   `apply`, identical retry, `status`, SIGKILL + restart), then `compile` of the
   post-insertion document with `flashtex-compiler` and export via
   `flashtex-pdf --out … --verify`.
5b. **Packaged render pipeline + exact export.** `lib/app_features.py
   render-attach` executes the bundle's `FlashTeX` directly with
   `FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_COMPILER=<bundled
   flashtex-render>` (what File > Attach Render Pipeline resolves to) and asserts
   `launched … (preview face: latin-modern)`, `status: attached:` and
   `revision 1: ok` in `FLASHTEX_LOG` plus a `flashtex-render` child under the
   app pid; `exact-export` runs the bundled `flashtex-pdf-exact from-v2` on
   `apps/mac/Tests/FlashTeXMacTests/Fixtures/display-list-v2-text.json` with
   `--font-dir apps/mac/Fonts` and reads the PDF back through PDFKit
   (`lib/pdfkit_probe.swift`, compiled on the fly): page count and text.
5c. **Windows, worker relaunch, multi-file project.** `open-window` launches the
   bundle with `FLASHTEX_OPEN_WINDOW=a11y-help` / `nearby`, finds the window
   through a CGWindowList probe (`lib/window_probe.swift`) and captures it by
   id with `screencapture -x -l` (never activated). `worker-relaunch` kills the
   bundled compiler child with SIGKILL four times while the typing bench keeps
   editing and asserts the shell's bounded auto-relaunch (scheduled, relaunched
   with a new child, a later keystroke recompiled — within 5 s — for three
   kills, then `worker relaunch limit reached`). `multifile` opens a temp
   project (`main.tex` + `\input{chapter}`) on the helper route with
   `FLASHTEX_OPEN_INCLUDES=1 FLASHTEX_ACTIVE_PATH=chapter.tex`, types into the
   active editor and asserts from the log, the helper ledger and the disk that
   `chapter.tex` was opened through the helper and received the edit durably
   while both files on disk stayed untouched. `flashtex-project-files` is built
   from the app branch (its JSON Lines host is not on main yet).
5d. **Branch XCTests against the real helpers.** Save routing to a member (⌘S),
   quit-save of members, reviewed reload after an external edit and the on-disk
   conflict refusal need the Save menu / quit alert / reload sheet, which cannot
   be driven without Accessibility; `swift test --filter 'ProjectDocumentsTests|
   DocumentFilesTests|HistoricalPreviewTests|ShellModelTests/
   testCrashedWorkerIsRelaunchedWithBoundedBackoff'` runs those paths with
   `FLASHTEX_*` pointing at the built helpers; per-test results (including skip
   reasons) are gated (`--skip-tests` to leave it out).
6. **Report.** `lib/report.py` writes `reports/<UTC>.md` with every number,
   binary hash, source SHA, machine/OS/Xcode/Swift/cargo version, the driving
   agent session and the exact commands (`reports/<UTC>/commands.log`), and
   asserts `thresholds.json`. Exit 0 only when every gate passed.

## Usage

```sh
tools/native-validation/mac-live/run.sh                      # full run, defaults
tools/native-validation/mac-live/run.sh --branch origin/agent/x/y --main-ref origin/main
tools/native-validation/mac-live/run.sh --skip-bench --skip-launch   # cycle only
tools/native-validation/mac-live/run.sh --skip-controller            # direct worker route only
tools/native-validation/mac-live/run.sh --intervals "30" --seeds "demo"
FLASHTEX_MAC_LIVE_SESSION=<session url> FLASHTEX_MAC_LIVE_AGENT=<agent id> tools/native-validation/mac-live/run.sh
```

Options: `--no-fetch`, `--rebuild` (discard the scratch helper/app builds),
`--force` (run launch-check even if a FlashTeX.app is running), `--work <dir>`
(scratch root; default `build/`, gitignored), `--session`, `--agent`.
Scratch builds are keyed by SHA (`build/helpers/<main sha>`, `build/app/<branch
sha>`, shared `build/cargo-target`) and reused across runs.

## Thresholds (`thresholds.json`)

Gates fail the run; targets are reported only.

| section | gate |
|---|---|
| build | helpers built from main (pinned clone, clean); app built; bundle has `FlashTeX`, `flashtex-compiler`, `flashtex-pdf`, `flashtex-bridge`, `flashtex-edit-ledger`; bundled helpers and app carry the freshly built object code (signature-masked Mach-O content sha256 equal; make-app.sh re-signs the bundle ad hoc); `components.json` SHAs equal the main/branch SHAs |
| typing-bench/compiler | all six cells present; per cell: `unpainted == 0`, typing budget not exhausted, `typed == keystrokes == script_keystrokes` (200), `paints >= 1`, producer reported as `flashtex-compiler`; keystroke -> paint p50 <= 40 ms (`fixture`), <= 60 ms (`demo`), <= 400 ms (`body60k`) |
| typing-bench/render | same correctness gates, producer reported as `flashtex-render`; latency reported (not gated) |
| typing-bench/controller | same correctness gates, producer reported as `flashtex-preview-controller`; latency is reported (not gated) against the project target |
| typing-bench (all) | p50 gates are applied only when the 1-minute load average before the pass is <= 10; above that the gate row says "NOT applied" and the numbers are reported only |
| render-attach | bundled `flashtex-render` present; `preview face: latin-modern`, `status: attached:`, `revision 1: ok` in `FLASHTEX_LOG`; `flashtex-render` child under the app pid; app alive after the wait; launched with `FLASHTEX_NO_ACTIVATE=1` |
| exact-export | `flashtex-pdf-exact from-v2` exit 0; PDF written; PDFKit page count == fixture pages; PDFKit text contains "Office fixtures", "office", "bold", "caf" |
| open-window/* | app running; an on-screen window titled "Accessibility Help" / "Nearby Companion" owned by the app; `screencapture -l <id>` wrote a PNG > 4 KB and > 100x100 px |
| worker-relaunch | compiler attached + bench typing; kills 1–3: relaunch scheduled, relaunched with a new child pid, a later keystroke recompiled, all within 5 s, app alive; kill 4: `worker relaunch limit reached`, no relaunch, app alive; app exits cleanly with the bench summary |
| multifile | bench summary written; `project: opened chapter.tex`; helper attached; every keystroke painted; `main.tex` and `chapter.tex` on disk untouched; helper ledger: chapter.tex contains the typed text, main.tex does not |
| app-tests | `swift test` exit 0 with 0 failures; the listed ProjectDocuments/DocumentFiles/ShellModel tests passed (not skipped) |
| historical | analyze.py produced rows for the `FLASHTEX_COMPLETED_SNAPSHOTS=1` pass; >= 1 historical frame painted across it |
| build (extras) | `flashtex-render` / `flashtex-pdf-exact` built from their refs (pinned clone HEAD == SHA); bundled copies carry the built object code; `components.json` `render` / `pdf_exact` SHAs == their branch SHAs |
| typing-bench target (not a gate) | project target typing-to-visible p50 and p95 <= 200 ms, reported per cell as met / not met |
| launch-check | zero `FAIL:` lines; compiler attached, `attached:` and `revision 1: ok` logged, app survives compiler kill, `worker exited (` logged; bridge attached, `bridge: attached:` logged, app survives bridge kill, `bridge exited (` logged; clean quit; launched with `FLASHTEX_NO_ACTIVATE=1` |
| launch-check informational | on-screen window confirmed (CGWindowList); child re-attached after kill (the shell has no auto-relaunch) |
| capture-cycle | every driver check (durable receipt, duplicate/conflict, `provider_disabled`, `proposal_missing`, journal survives SIGKILL, reject terminal, `capture_missing`, ledger receipt + identical retry + pending receipt + survives SIGKILL, compile ok/recovered with >= 1 page, `flashtex-pdf --verify` exit 0, PDF header/trailer, PDF page objects == compiled pages); no provider enablement |

## Typing latency on the packaged app, attributed per stage (mac-validation-4)

`lib/typing_attribution.py` executes the bundle's `Contents/MacOS/FlashTeX`
directly (never `open`; `FLASHTEX_NO_ACTIVATE=1`) through the shell's own
typing bench on four routes — `v1` (bundled `flashtex-compiler`, PreviewView),
`v1-render` (bundled `flashtex-render` painting v1), `v2` (bundled
`flashtex-render` + `FLASHTEX_PREVIEW_V2=1`, PreviewV2View) and `controller`
(bundled `flashtex-preview-controller` owning the ledger + bundled compiler) —
for seeds `fixture`, `demo`, `hw1` (`fixtures/real-world/hw1/HW1.tex`),
`render27` (the render-pipeline lane's 27-page `tests/incremental.rs
document(40)` fixture, reproduced) and `body60k`. Every painted revision's
keystroke -> paint time is split over the bench timeline log lines (emitted by
the shell under `TypingBench.isBenchActive`): key->send, producer wall (send ->
last decoded line, minus decode), reader decode, main-queue hop, apply, the
`preview-v2:` prepare / preraster / deliver / publish stages, and paint; the
helper route's client logs no decode/hop lines, so its answer is one
`helper` stage. Decoded lines carry no revision and the next request is sent
before the v2 sibling is decoded, so lines are claimed by the consumer event
that follows them (`compile: applied revision r` -> v1 line, `preview-v2:
preparing/coalesced (revision r)` -> sibling). Producer CPU is sampled from
the child processes (`ps -o cputime`, twice a second; last sample / results).
Each cell waits for a quiet machine, records the 1-minute load before/after
and `uptime`, and is marked load-affected above `--load-limit` (never hidden).
`--report <dir>` renders `attribution.md`; `run.sh` now packages the preview
controller too (`make-app.sh --controller`) so the bundle carries the helper
route. `quiet-window.sh --app <FlashTeX.app>` runs every cell unattended, one
analyzer invocation per cell, each after the load fell below `--quiet-load`
(default 8, up to `--quiet-wait` s) with no other FlashTeX process alive,
appending `uptime` before/after each cell to `<out>/uptime.log`; re-running
with the same `--out` resumes (cells with a summary are skipped).

## Layout

- `lib/typing_attribution.py` — packaged-app per-stage typing attribution
  (cells, log parsing, Markdown report); `quiet-window.sh` — unattended
  quiet-window driver for it; `reports/attribution-<UTC>/` — its evidence
  (`attribution.md/json`, per cell `.json` summary, `.attribution.json` per
  painted revision, `.log` timeline, `uptime.log`).
- `run.sh` — orchestrator; `lib/report.py` — report + gates; `lib/launch_summary.py`
  — parses launch-check evidence; `lib/capture_cycle.py` — packaged capture
  cycle; `lib/app_features.py` — headless render-pipeline attach and exact
  export checks; `lib/pdfkit_probe.swift` — PDFKit page count/text probe;
  `lib/hashes.py` — as-shipped, signature-removed and signature-masked sha256;
  `lib/open-shim/open` — adds `--env` values to `open`.
- `lib/window_probe.swift` — CGWindowList window list for a pid;
  `lib/xctest_summary.py` — `swift test` log to JSON.
- `fixtures/capture-proposal.json` — offline `capture_proposal` fixture.
- `reports/<UTC>.md` + `reports/<UTC>/` — committed evidence: `env.json`,
  `helpers.json`, `app.json`, `bundle.json`, `steps.jsonl`, `commands.log`,
  `logs/`, `typing-bench/` and `typing-bench-controller/` (bench evidence + raw JSON), `launch-check.md/json`,
  `launch-transcript.jsonl`, `capture-cycle.json` + `capture-cycle/`
  (transcript, compile result, exported PDF).

## Limitations

- The typing bench inserts text programmatically; the OS keyboard hop and
  display scan-out are not measured (see the bench's own methodology). One run
  per cell; other agents' builds/tests on the same machine are recorded (load
  average, process list) but not controlled.
- The shell has no automatic child relaunch: "restart" in the launch check means
  the app keeps running and reports the exit; reattachment is a user action.
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
- Latency numbers depend on machine load: other agents build and test on this
  Mac concurrently. `uptime` is recorded at start, before every bench pass and
  at the end; latency gates are not applied when the 1-minute load average
  before a pass exceeds 10 (the report says so), and are never hidden otherwise.
- The render-attach check proves the menu item's resolution path and the
  producer face switch through the app's own log, not the menu click itself.
