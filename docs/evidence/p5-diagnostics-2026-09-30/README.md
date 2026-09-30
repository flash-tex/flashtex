# P5-DIAGNOSTICS: structured diagnostics from the engine host (`diag-v1`), 2026-09-30

Lane P5-DIAGNOSTICS (DESIGN.md §12 P5 "new engine ≥ old on every tier", §6.1; §1.1 P-T1).
Branch `agent/kabir-claude/p5-diagnostics` from `origin/main` 02dcf9d07. The finding it closes
(`docs/evidence/typst-design-2026-09-30/track-b-ux.md`, branch `agent/kabir-claude/typst-design-b`):
the new engine's diagnostics reached the app as `{severity, message, file?, line?}` read off the
terminal, less precise than the old engine's byte spans, so switching engines would have regressed
error reporting.

**Hosts.** macOS: mac-m5pro-kabir (Apple M5 Pro, macOS 26.6.2, MacTeX 2026 pdflatex 1.40.29), shared
with other agents (load 40+ at times). Linux: the NixOS PC (16 threads, TeX Live 2026 in
`~/texlive/2026`), where the Commander moved the heavy sweeps on 2026-09-30; its checkout is this
branch merged locally with PR #1232 (the engine's rpath on NixOS), never pushed. Every number below
names its host. pdflatex is the oracle only.

## Results

| Target | Measured (verified) | Status |
|---|---|---|
| Every error/warning with file, line, **column/byte range**, macro trace, help text, severity, stable code | `DIAG` (§6.7 of docs/protocol/display-list-v3.md): `file`, `line`, `col` (TeX's context split), `range`, `offset`, `span`, `trace` (every input level, macro names, `\name #1->body` split where TeX was, definition sites), `help` (or `\errhelp`), `severity`, `code`, `origin`, `exact` | met |
| Positions equal pdflatex's error context, on ≥ 60 cases, expected data from pdflatex only | 80 cases, 109 reports; **93/93** positions with a pdflatex context: same file, line **and column**; **9/9** box line ranges; 2 more positions where pdflatex prints none (LaTeX's missing-file message). macOS and Linux identical | met |
| At least as precise as v1, better where TeX allows | v1 (`flashtex check --json`, old engine): same line 54/93, **exact column 0/93**, span covering TeX's column 36/93, box lines 1/9 (table below) | met |
| Side channel, logs untouched (P-T1) | P-T1 **83/83** and P-T2 83/83 with the hooks off, with a display list, and **with the hooks on** (`FLASHTEX_DIAGNOSTICS=1`); lockstep **260/260** with the hooks on (accounting 0) | met (Linux) |
| trip / etrip / drift | pass | met (Linux) |
| Incremental soundness, diagnostics re-emitted for reused pages | `tests/diagnostics.rs`: 16 compiles of an editing session + a reopened S₀ equal scratch compiles; fixture sweep below; `tests/incremental.rs` (8) and `tests/host_incremental.rs` (4) pass with the side channel on (logs, PDFs, `.aux` equal scratch runs) | see below |
| Protocol: versioned, additive, capability-gated; decoder for the app lane | `diag-v1` in `HELLO.capabilities`, client `HELLO.accept`; kind `0x60` in a range of its own (no page-item or 3.x change); `crates/display-list-v3/src/diag.rs` (MIT) + `client::Event::Diag` + a Swift `Decodable` sketch in §6.7; old clients get `DIAGNOSTIC` exactly as before (test) | met |

### Precision against pdflatex and against v1 (macOS, `raw/compare-macos.{txt,json}`)

`tools/diag-oracle/compare.py`, engine d769332b1's predecessor 9585ff2e2 (the notes' content is the
same), v1 = `flashtex check --json` built from this branch (runtime-v1 untouched):

| kind | reports | with a pdflatex position | engine: same line | engine: same column | v1: same line | v1: span covers the column |
|---|---:|---:|---:|---:|---:|---:|
| error | 75 | 69 | 69 | 69 | 48 | 32 |
| warning | 23 | 23 | 23 | 23 | 6 | 4 |
| `\show` | 1 | 1 | 1 | 1 | 0 | 0 |
| box | 9 | 9 (line range) | 9 | first character: 8 | 1 | — |
| pdfTeX warning | 1 | 0 | — | — | — | — |

v1's exact column never equals TeX's (0/93): v1 points at the start of a command, TeX at what it
has read. Its span covers TeX's column in 36 of 93. v1 reports 95 diagnostics on the corpus, of its
own making (`unknown_command`, `unsupported_feature`), which do not always correspond to TeX's.

How the expected positions are made (`tools/diag-oracle/oracle.py`, `expected.json`; never by hand):
pdflatex runs each case as the host compiles (nonstopmode, file:line:error, again while a file it
reads changed, up to five runs); for each error its log's bottom context line `l.<n>` gives the
line, and the column is the byte offset where the source line splits into what the two context
lines show (after a leading `...`, before a trailing `...`). A LaTeX or package warning has no
context in pdflatex's log, so a second run of pdflatex (the *probe*) makes `\GenericWarning ` an
`\errmessage`, and the warning's column is that error's split: the same input stack as the
`\immediate\write` in the real macro. Box reports: the line range TeX prints. Seven reports have
no position in pdflatex's log (EOF runaways end at the terminal level `<*>`; LaTeX's missing-file
message is a `\typeout`); the engine gives two of them one (the `\input` line).

`cargo test -p flashtex-engine --test diagnostics` (`corpus_positions_equal_pdflatex`) requires all
of this on every run where TeX Live is installed: 102 positions (93 + 9).

## What was built

- **`changes/diagnostics.ch`** (applied last, in `web2rust-default.args` and `-etrip.args`): calls
  into `src/diag.rs` at `print_err` (where a message starts in the captured terminal), `error`
  (message printed; input stack, help lines, `use_err_help` as the error left them),
  `pdf_warning`, the overfull/underfull/tight/loose reports of `hpack` and `vpackage`, `\def`
  (definition sites) and `write_out` (a `\write` to the terminal: LaTeX's and packages' warnings,
  and LaTeX's own `! LaTeX Error: File ... not found.` `\typeout`). The routines only read TeX's
  variables and never print; they do nothing unless the host enables them.
- **`src/diag.rs`**: the notes. The place is the innermost *file* level of the input stack — the
  level TeX's context display ends with — with the column of the split (`loc - start`, clamped as
  `show_context` clamps it) and the start of the command before it (a control word or symbol, with
  the brace groups and optional arguments that follow it: `\mycmd{x}`, `\ref{a}`, `\section*[a]{b}`;
  else one token, a UTF-8 character whole). The trace walks the input stack as `show_context` does
  but keeps every level (up to 24; LaTeX's `\errorcontextlines=-1` shows two), printing token lists
  as `show_token_list` does, read-only. A box report's first and last characters come from the
  display list's side table (§5.3). Definition sites: a side table keyed by the macro's token list
  and a fingerprint of its first 64 tokens, checked when read (a restore that frees and reuses a
  list cannot give a macro a wrong site); macros of the format have none.
- **Checkpoints**: `ExtRecord::notes` counts the notes a checkpoint precedes; a restore truncates,
  the restored branch keeps the old run's notes (shared, `Arc`), a convergence splices the old run's
  later notes in at their shifted terminal offsets, `reattach` puts them back, S₀ persists them and
  the definition sites. An incremental compile therefore reports the whole document's diagnostics,
  those of kept pages included, exactly as a scratch compile.
- **Host**: `src/host/diag.rs` turns notes into `DIAG`s (codes, origins, file offsets, spans declared
  in `SOURCES` first) and adds what the terminal shows that no note covers (`exact: false`, the 3.1
  rules: pdfTeX's C-part messages, `==> Fatal error occurred`). `resident.rs` sends them to a client
  that accepted `diag-v1`, `DIAGNOSTIC`s to any other; `export` compiles (another process) send
  terminal-only `DIAG`s. `flashtex-host iserve` has a `diagnostics` command.
- **Protocol**: docs/protocol/display-list-v3.md §6.7 (and §2's kind table, §6.2 `accept`, §6.4,
  §9 checklist item 9). `crates/display-list-v3/src/diag.rs`: `Diag`, `Frame`, `Loc`, `Severity`,
  `decode`/`to_json`, `CAPABILITY`; `Client::connect_accepting`; `dl3-client --diag FILE`;
  `dl3-dump` prints `DIAG`s.

## Gates (Linux, `raw/`, engine = this branch at the commit named in `raw/environment.txt`)

| gate | result |
|---|---|
| P-T1 / P-T2, parity fixtures, hooks off | **83/83, 83/83** (`raw/parity-fixtures.txt`) |
| the same with a display list | **83/83, 83/83** (`raw/parity-fixtures-display-list.txt`) |
| the same with the diagnostics hooks on | **83/83, 83/83** (`raw/parity-fixtures-diagnostics.txt`) |
| lockstep, hooks on | **260/260**, accounting 0 differ (`raw/lockstep.txt`) |
| trip, etrip, web2rust drift | pass (`raw/trip.txt`, `raw/etrip.txt`, `raw/drift.txt`) |
| `tests/diagnostics.rs`, `tests/incremental.rs`, `tests/host_incremental.rs` | 3 + 8 + 4 pass (`raw/tests.txt`) |
| `scripts/gate.sh pr` | SEE-BELOW |

(The parity "accounting (non-gating): 83/83 documents differ" line is as on `main`: DESIGN §1.1's
end-of-run capacity block.)

## Soundness (Linux, `scripts/diag_soundness.py`, `raw/diag-soundness.*`)

SOUNDNESS

## Cost of the side channel (Linux, `scripts/perf.py`, `raw/perf.txt`)

PERF

## Limits and open items

- `\batchmode` (or `\scrollmode` with the terminal closed) keeps messages off the terminal, which
  is where a note reads its text: the note keeps its place, trace and help, and its message is
  empty. The host runs nonstopmode, and a document that switches to batch mode is rare.
- Macros defined in the format (latex.ltx and what it loads) have no definition site; definitions
  made while the run reads a package do (the package file and line). The site is the line where
  TeX had read the macro's name (`\def\x`), or, for `\newcommand`, where its arguments ended.
- `range` is a reading of the source line (the command and its brace groups before the split),
  not something TeX records; `col` is exactly TeX's.
- Columns are bytes (as `SOURCES`' `col`); the app converts to its string index.
- pdfTeX's C parts' messages and every `DIAG` of an `export` compile are terminal-only
  (`exact: false`: file and line at best).
- A persisted S₀ written by an earlier build cannot be read by this one (the build id changes with
  every source change; the notes add two fields to the file).

## Reproducing

```
cargo build --release -p flashtex-engine -p flashtex-display-list -p flashtex-cli
python3 tools/diag-oracle/oracle.py                     # expected.json from pdflatex (oracle only)
cargo test --release -p flashtex-engine --test diagnostics
python3 tools/diag-oracle/compare.py --host target/release/flashtex-host --formats FMTDIR \
    --pool crates/flashtex-engine/pdftex.pool --v1 $PWD/target/release/flashtex --json out.json
python3 tools/diag-oracle/table.py out.json
bash docs/evidence/p5-diagnostics-2026-09-30/scripts/gates.sh           # the Linux gates
python3 docs/evidence/p5-diagnostics-2026-09-30/scripts/diag_soundness.py ENG FMT REPO -j 12
python3 docs/evidence/p5-diagnostics-2026-09-30/scripts/perf.py ENG FMT
```
