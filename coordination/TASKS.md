# Commander task board

Owner: Commander `orchestrator-jaysen-claude` (mac-m1max-a). Updated: 2026-09-14T01:05:00Z.
Full process: [orchestration master plan](../ORCHESTRATION.md). Claims: `python3 scripts/coord.py claims --stale`
(branch `coordination-claims`). Live machines: mac-m1max-a (this Commander), Kabir's
linux-primary / mac-m5pro-kabir (`kabir-claude`, integration lane + FT-060..067/070).

> **Staffing rule (user, 2026-09-13/14):** tasks go only to machines with a fresh live
> report on GH issue #2. Heavy models (Opus/Fable) are reserved for compiler/producer
> performance (FT-070 Kabir, FT-071 Mac). Claim before starting, one-liners included.

## Current queue (main eea975c3, v0.1.3 tagged at b613ff41)

Merges to main are review-gated on mac-m1max-a; Kabir's integration lane merges gated PRs in the order posted on GH issue #2.

| ID / revision | Task | Owner | Branch | State | Notes |
|---|---|---|---|---|---|
| FT-060 / 1 | HW2 math completion + \mathcal from NewCMMath | kabir-claude | agent/kabir-claude/hw2-math-final | integrated | HW2: 3 pages, 0 errors, 0 overfull on main |
| FT-061 / 1 | amsmath environment completeness | kabir-claude | agent/kabir-claude/amsmath-envs | in progress | #198 blocked on rebase (collides with #180); tier-1 spacing landed via #230 |
| FT-062 / 1 | TikZ minimal-but-real subset | kabir-claude | agent/kabir-claude/tikz-min | in progress | path items on main; nodes/arrows pending |
| FT-063 / 1 | Real images and floats | kabir-claude | agent/kabir-claude/graphics-floats | in progress | image items on main; float placement pending |
| FT-064 / 1 | Hyphenation (Liang patterns) | kabir-claude | agent/kabir-claude/hyphenation | integrated | unified in paragraph-layout |
| FT-065 / 1 | Compiler hyperoptimization | kabir-claude | agent/kabir-claude/compiler-perf | integrated | 500 KB warm edit 4.5 s -> 111 ms; folded into FT-070 |
| FT-066 / 1 | Conversion provider seam + on-device model design | kabir-claude | agent/kabir-claude/conversion-provider | integrated | `--conversion-provider`; Keychain `tech.jay3332.flashtex.ai.<provider>` |
| FT-067 / 1 | Supported-LaTeX truth: generated docs + drift gate | kabir-claude | agent/kabir-claude/supported-latex | integrated | every compiler lane must re-run `apps/mac/scripts/sync-supported-latex.sh` |
| FT-070 / 1 | Compiler hyperoptimization program (standing) | kabir-claude | agent/linux-primary/ft070-* | active | #232 memory (Cluster 160 -> 88 B); next: shared assembled items, page window API |
| FT-071 / 1 | Producer-side performance (standing) | mac-claude-a | agent/mac-render-pipeline/perf-* | active | perf-4 `display-list-v2-compact` running (lane mac-perf-4); shares the page-window API with #232 |

### Open PR dispositions (Kabir's 2026-09-14T00:10Z triage, agreed)

| PR(s) | Disposition | Owner |
|---|---|---|
| #233 | remove web-authored docs/examples that describe another codebase — merge first (v0.1.3 is out) | integration lane |
| #241 | render-pipeline: packages the pipeline sets are no longer reported as not implemented | integration lane |
| #242 / #243 / #244 | letter class; `\c` `\v` accents; `\maketitle` without `\author` + `Parser::unsupported` message — the last real-world corpus errors | unclaimed compiler lanes (Kabir's machine) |
| #230 -> #231 -> #232 | package gating (compiler, pipeline) then FT-070 memory; merge in that order, each gated | kabir-claude |
| #142 | compiles against a removed `long_required_group`; rebase | its lane (compiler) |
| #197 | two of its own tests fail on main (`\thm@headsep`); re-measure on main | its lane |
| #153 -> #154 -> #192 | #153 predates the `crate::expansion` refactor; rebuild on the current compiler before the chain can land | its lane (footnotes) |
| #181 | rebase onto #199 keeping its tests | kabir-claude |
| #201 | rebase onto #186 | kabir-claude |
| #173 -> #176 -> #184 | cheapest chain; rebase #173 first | kabir-claude (offered) |
| #194 -> #195 | rebase #194 onto main, then re-base #195 on it | kabir-claude |
| 30 others | conflict with main on `supported-latex.json` / `coverage.md` / `lm_math.rs` / `typeset.rs`; owners rebase; no integrator merges | owners |
| #212 | held until #230 lands (its stated blocker) | its lane |

### Deferred / owner-only

iPad on-device run needs Xcode 26.4 (iOS 26.4.1 DDI); exact-route PDF paths (crates/pdf);
`msbm` advances from TFM in the producer; whether to rewrite the five mis-attributed main
commits (97a02a26..58aa1093) is the owner's call. Historical rows follow.

| ID / revision | Task | Owner | State | Dependencies |
|---|---|---|---|---|
| ORCH-001 / 1 | Publish orchestration plan, roster, dispatch board, and discovery links | commander | integrated: 567d84b on main | Self-registration clarification follow-up |
| ORCH-002 / 1 | Executable coordination and discovery service | commander | integrated: 6d096a3; 12 tests pass | Discovery service active |
| FT-001 / 1 | Shared compile/edit/capture contracts and fixtures | commander | integrated: 6d096a3 runtime-v1 and fixtures | Native consumer verification pending |
| FT-002 / 1 | Original Rust compiler foundation | claude (Codex on Kabir Mac) | assigned; acknowledgement pending | FT-001 |
| FT-003 / 1 | Native Mac shell | mac-claude-a | assigned; acknowledgement pending | runtime-v1; use available Codex instead of protected Claude |
| FT-004 / 1 | Pencil and camera capture | aarush-macbook | assigned; acknowledgement pending | runtime-v1; confirm OpenAI tool readiness; no protected Claude |
| FT-005 / 1 | Rust layout/output and source mapping | Unassigned | unassigned | FT-002 |
| FT-006 / 1 | Incremental reuse and recovery evidence | Unassigned | unassigned | FT-005 |
| FT-007 / 1 | Rust bridge: Grok/transfer and reviewed insertion | commander | assigned; begins after loop publication | FT-001/003/004, Grok funding |
| FT-008 / 1 | Integrated demo verification | commander + future Mac worker | unassigned | FT-003/005/006/007 |
| FT-014 / 1 | Independent companion native validation and repair broker | chatgpt-a | assigned; ACK/PID pending | FT-004 branch, Xcode 26.6; no overlapping writes |
| FT-015 / 1 | Linux end-to-end demo, recovery, and regression harness | local-claude-opus | assigned; blocked on execution login | Compiler/bridge/PDF exact SHAs; offline fixtures required |
| FT-016 / 1 | Deterministic Claude worker supervisor and recovery safety | commander-supervisor | assigned; implementation via Commander OpenAI route | Local Claude FT-015 and auth recovery issue #6 |
| FT-017 / 1 | Reference TeX raster-diff visual oracle | mac-visual-oracle (Opus/Mac) | assigned; ACK/PID pending | Declared visual fixtures; test oracles only |
| FT-018 / 1 | TeX font shaping, metrics, and PDF embedding | mac-font-engine (Opus/Mac) | assigned; ACK/PID pending | Visual oracle; compiler adapter later |
| FT-019 / 1 | Paragraph line breaking, glue, kerning, and baselines | mac-paragraph-layout (Opus/Mac) | assigned; ACK/PID pending | FT-018 metric API |
| FT-020 / 1 | TeX-style math boxes, rules, delimiters, and spacing | mac-math-layout (Opus/Mac) | assigned; ACK/PID pending | FT-018 metric API |

## Dispatch record required before changing a task to assigned

```text
Task ID / assignment revision / owner / branch:
Objective and acceptance criteria:
Owned paths / protected shared paths:
Dependencies and exact input revisions:
Contract and fixture links:
Timebox / next report time / hard deadline:
Resource allocation and descendant limits:
Validation / deliverable location:
Worker acknowledgement revision:
```

Assignment rows alone do not launch agents. Worker must acknowledge the dispatch
revision in its handoff before being counted as working. Commander owns this file.
Structured FT-003/FT-004 records under `coordination/assignments/` define the exact
ownership, branch, timebox, acceptance criteria, and funding references. Workers
should migrate their legacy Markdown registration with `scripts/coord.py register`
on the assigned branch, then acknowledge the published assignment. If local Cursor
is unavailable, submit a patch plus report via a repository issue for central commit.

## Commander implementation queue (current user instruction)

1. ORCH-003/004: worker execution, completion dispatch, resource/recovery reporting,
   coauthor enforcement, and real autonomous-startup verification.
2. FT-007: original Rust capture bridge and transfer-v1 consumer contract under
   crates/bridge; deduplication, revision-safe review and context/provider tests.
3. Integration: review and combine compiler/Mac/companion checkpoints, resolve
   conflicts and validate combined behavior; do not make workers wait on review
   when an independent next stage is already approved.
4. FT-008: end-to-end test scenarios, compatibility corpus and measured responsiveness;
   distinguish actual device/native evidence from Linux or protocol-only checks.

Compiler FT-002 acknowledgement is published at 25fe5c4. Mac FT-003 revision1 reports
ready at f13979c: six native tests/builds reported; Commander inspected package,
README and ShellModel and identified stale-source navigation as a follow-up check.
Mac next queued work now prioritizes real JSONLines subprocess transport, matching
its stated next experiment. Initial iPad acknowledgement remains pending.
