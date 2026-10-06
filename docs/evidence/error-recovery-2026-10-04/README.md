# Error recovery, old engine vs new (2026-10-04)

Lane **ERROR-RECOVERY** (mac-claude-a, mac-m1max-a). The owner reported that "in the new
engine, error recovery seems significantly worse" than in the old one. This directory
holds the corpus, the measurements and the fix for the largest gap.

## Corpus and method

- **`gen.py` → `corpus.json`.** It builds 27 cases on two correct base documents:
  - `short` is 5 pages and `long` is 75 (article, amsmath, graphicx);
  - each case is a list of edits that turns the base into the erroneous text;
  - undoing the edits is the "fix".
- **`engines.py`** runs every case one shot through three engines:
  - pdflatex (MacTeX 2026, the oracle only);
  - the new engine, as `flashtex-host` invoked as `pdftex`;
  - the old `flashtex-compiler`.

  pdflatex and the new engine both run `-interaction=nonstopmode -file-line-error`, as the
  app's host does. Results are in `raw/engines.json`.
- **`ErrorRecoveryCorpusTests`** (`apps/mac/Tests/FlashTeXMacTests/`) is the app itself.
  - It uses `ShellModel` and `EngineV3Session` over a real host.
  - Each case starts with the good document fully on screen. The error is then typed
    (`updateActiveText`, as the editor does).
  - The test records what the pane holds, the Problems rows and DONE. It then fixes the
    error and records the recovery.
  - Run it with `FLASHTEX_ERRREC_OUT=<dir> swift test --filter ErrorRecoveryCorpusTests`.
  - Results: `raw/v3-before.json` (main `e9d6bbaff`) and `raw/v3-after.json` (this branch).
- **`shot.sh`** opens a case in the real app (`FLASHTEX_V3_CAPTURE_EDIT`,
  `FLASHTEX_NO_ACTIVATE=1`, env hooks only), screenshots the window by id and ends only
  its own pid. Output: `before-unclosed-brace-v3.png`.
- **Old engine in the app:** the old path's preview behaviour was read in the code, not run.
  - The default producer is `flashtex-render`. A `failed` result keeps the last frame,
    unmarked; `recovered` replaces the pages.
  - There is no grace period before errors show, on either path.

**Machine load.** The load average was 300+ during the first runs and 6–20 during the
second, so the absolute times below are load-sensitive.

## Verified: the new engine's error output is pdfTeX's

On all 27 cases, `flashtex-host` as pdftex and pdflatex agree on three things: the page
count, the error count and messages, and whether the run is fatal. So the errors are not
the problem; what the app does with them is.

**Of the 27 cases, 9 are fatal in pdflatex: it stops, and writes no PDF.** These include
the commonest mid-typing states:
- `\textbf{` before the brace is closed;
- `\frac{a}{`;
- `\section{`;
- a `\usepackage` typo;
- a missing `\input`;
- `\end{document}` deleted;
- capacity exceeded.

The old engine is a lenient re-implementation. It returned `recovered` with every page for
all 27 cases, but its pages are not pdfTeX's: 74 pages on `long` against pdfTeX's 75.

## Phase 1 gap table (old vs new on main `e9d6bbaff`)

| # | Gap | Old engine | New engine on main | Cases |
|---|---|---|---|---|
| 1 | **Fatal error blanks the preview** | Every page shown, recovered leniently | The pane drops every page past the ones TeX shipped before the stop. `unclosed-brace`, `frac-incomplete`, `section-incomplete`, `usepackage-typo`, `input-missing` and `capacity-exceeded` all went **0 / 5 pages**. `long-fatal-page50` lost 13 of 75 | 9 / 27 |
| 2 | **Recoverable errors are red "errors"** although the preview is complete | An error row, pages updated | Every TeX error is an error row, the pane's first error and a red count. The pages *are* complete, as pdflatex's nonstopmode makes them | 17 / 27 |
| 3 | **Fixing a fatal error after page 1 compiles cold** | about 20–90 ms per full compile on `short` | `end-document-deleted` and `file-ended-in-argument`: cold, 2.0–2.2 s. `long-fatal-page50`: **cold, 9.7 s, 150 pages typeset** (2 passes). The cause is `cannot restore: main.pdf is gone (a run that fails removes its PDF)` | 3 / 27 |
| 4 | **"File ended while scanning use of \X" has no location** | `argument to \textbf is missing its closing brace`, on the brace | No file or line, so no underline. TeX names none | 4 / 27 |
| 5 | Explanations and quick fixes | Few fixes, but every message is a plain sentence. Explanations only with the `flashtex-explain` helper, which is absent here | TeX's help text as notes; the fix is "did you mean" only (`EngineV3Fixes`). Panel gap B9/B10 | all |
| 6 | Mid-typing grace | none | none | all |
| — | Recoverable errors: do later pages still update? | yes | **yes**: nonstopmode, every page typeset, as pdflatex does | — |
| — | Fixing a recoverable error | — | Incremental, 40–700 ms on `short`. `long-error-page1` re-runs all 75 pages (2.8–3.0 s); `long-error-page50` re-runs 13 | — |

## Phase 2, PR 1: best effort (gaps 1 and 2, the owner's requirement)

The owner's requirement (2026-10-04): a recoverable error still warns where pdfTeX would
report an error, but the preview is best effort. Strict mode is opt-in.

- **Recoverable errors** (TeX goes on, as nonstopmode does) are now **warnings**, marked
  "(pdfLaTeX would report an error here)". The pane counts them as warnings, and the
  status reads `recovered`.
- **Fatal errors stay errors**, together with their cause: the error just before the stop,
  at the same line, such as ``File `amsmth.sty' not found`` or
  "File ended while scanning use of \textbf".
  - **The pane keeps every page.** The pages TeX made before the stop are current. The
    last good pages after them stay on screen, stale (dimmed, orange border).
  - The status reads "stopped: pdfLaTeX gives up here · N new pages, M kept from the last
    compile".
- **Strict mode:** Settings › Compile › "Stop at the first error", off by default.
  - The host gets `"halt_on_error": true`, which is pdflatex's `-halt-on-error` (protocol
    `COMPILE.halt_on_error`, `Job::halt`).
  - Every error is then an error, and the pages before the stop stay current.
  - Exports always run nonstopmode.
- **No engine output changed.** The engine still runs nonstopmode. `-halt-on-error` is
  pdfTeX's own flag and is only passed when strict mode is on.

After (`raw/v3-after.json`, same harness):
- **0 of the 27 cases lose a page.**
- The fatal cases keep 5 / 5 (stale), or N new plus the rest stale (`long-fatal-page50`:
  62 new + 13 stale).
- The 17 recoverable cases show 0 errors and 1–3 marked warnings each.

Tests: `EngineV3BestEffortTests` (6 tests: the rule, plus the hosted pane end to end for
fatal, recovered and strict), and the Rust test `host::server::job_tests`. Updated:
`EngineV3DiagMappingTests`, `EngineV3EditorMarksTests`, `EngineV3OpenTests` (best effort)
and `EngineV3PackagesTests` (strict).

## Phase 2, PR 2: a fixed fatal error restarts warm (gap 3)

Branch `agent/mac-claude-a/errrec-fatal-warm`.

**The cause.** After a fatal error, pdfTeX deletes its unfinished PDF (`removepdffile`).
When the user then fixes the error, the restore of a checkpoint taken while that PDF was
open finds it gone (`changed_outside`), so the compile runs cold.

**The fix, for the resident session only.**
- The file is renamed aside (`main.pdf.flashtex-removed`), not deleted. The rename keeps
  the length, mtime and inode that the stamp compares.
- Only a restore whose checkpoint had the file open puts it back.
- On disk, the PDF is gone after a fatal run, as pdfTeX leaves it.
- The one-shot engine (`pdftex`, lockstep, export) still deletes it.

`raw/v3-after-both.json` was measured with both PRs:

| Case | Fix before | Fix after |
|---|---|---|
| `end-document-deleted` | cold, 1.0–2.2 s, 10 pages typeset | incremental, 157 ms, 6 pages |
| `file-ended-in-argument` | cold, 1.4–2.1 s, 10 pages | incremental, 278 ms, 6 pages |
| `long-fatal-page50` | cold, 5.8–9.7 s, 150 pages | incremental, 908 ms, 88 pages |

The fixed document's pages are identical to the good run's in every case (`same_as_before`).

## Phase 2, PR 3: locations, explanations and fixes (gaps 4 and 5), plus two follow-ups

Branch `agent/mac-claude-a/errrec-locate`, stacked on PR 1.

**Gap 4: "File ended while scanning use of \X" now has a location.**
- TeX names no place for it, because the file level has already ended when TeX reports it.
- It does print the start of the argument, on the line after "Runaway argument?" (§306).
- The host (`host/diag.rs`, `locate_runaways`) looks for that text in the project files
  TeX opened, comparing tokens without blanks or comments, with a blank line read as `\par`.
  It then places the report on the argument's opening brace, the one never closed.
- The stop reports right after it that name no place go there too.
- In the corpus, all four such cases are now underlined on the `{`, for example `\textbf{`
  at 13:13. Before, they had no file and no line.
- This only reads what TeX printed, so the engine's output does not change.

**Gap 5: a plain-language explanation for every error code in the corpus** (`EngineV3Explain`).
It is the row's first note; TeX's own help stays a note too. A fix is attached only where
it is mechanical and the row's range is exactly the text it replaces:

| Code | Fix |
|---|---|
| `tex/too-many-right-braces` | remove the `}` |
| `tex/misplaced-alignment-tab` | `&` → `\&` |
| `latex/environment-mismatch` | `\end{B}` → `\end{A}`, or add `\end{A}` before `\end{document}` |
| `latex/file-not-found` (`.sty`) | the one close package name, as "did you mean amsmath?" |

- **Environment fixes** are offered only when the line LaTeX names contains `\begin{A}`.
  So there is none for `\[` (amsmath's `equation*`), and none for an unknown environment
  that closed `document`.
- **A missing package** used to be placed on the *next* line's `\usepackage`, because
  LaTeX looks ahead for an optional date argument. It is now placed on the name itself.
- **The old engine's "did you mean \X?"** for undefined commands (`EngineV3Fixes`) is
  unchanged.

**Follow-ups to PR 1.**
- A DONE without a `pages` field no longer clears the stale marks.
- The host announces `halt-on-error` in its HELLO capabilities. When strict mode is on and
  the host lacks that capability, Settings says that strict mode is not honoured.

## Not done here (next)

- **Gap 6:** the mid-typing grace period. It is partly moot now that recoverable errors are
  warnings and fatal ones keep the pages.
