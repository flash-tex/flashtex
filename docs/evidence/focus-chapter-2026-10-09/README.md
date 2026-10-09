# Chapter focus (`\includeonly`): parity and latency, 2026-10-09

Lane FOCUS-CHAPTER (owner idea, 2026-10-04). Machine: mac-m1max-a (M1 Max), shared with
other lanes: **load average 220–330 during every run** (`uptime` recorded per run), so
absolute times are inflated and noisy; the focus/whole comparisons were taken back to back
under the same load. Engine: this branch's release `flashtex-host`, its own pdflatex format
(`tools/incr-bench/mkeng.sh`). Oracle: MacTeX 2026 (pdfTeX 1.40.29), tools only.

## What the host runs

`COMPILE` `"includeonly": ["chapters/ch22"]` → first line
`\AtBeginDocument{\includeonly{chapters/ch22}}\input main.tex` (everything else on the
command line unchanged), in the app's `out-focus` folder started from the whole document's
auxiliary files.

Why at `\begin{document}` and not in the preamble: LaTeX's `\includeonly` normalises each
name with l3's `\file_full_name:n`, which asks `\pdffilesize` of the chapter. In the
preamble that lookup puts the chapter file in S₀'s key, and **every keystroke in the
focused chapter was a cold run** (measured: `cold_reason: ./chapters/ch03.tex changed`,
edited page p50 240 ms against 38 ms for the whole document). In the `begindocument` hook
`\includeonly` runs after the `.aux` is read and before `\document` disables the preamble
commands, so its effect is the same and the lookup is the body's.

## Parity (VERIFIED, `scripts/oracle_focus.py`)

4-chapter `book` (`scripts/gen_book.py OUT 4 5`), whole document to convergence, then the
focused first line twice, pdflatex against the FlashTeX engine:

| run | pages | per-page content streams (qpdf `--normalize-content`) | every `.aux` |
|---|---|---|---|
| whole document | 17 = 17 | 17/17 identical | identical (5 files) |
| focused `chapters/ch03`, preamble form | 5 = 5 | 5/5 identical | identical |
| focused `chapters/ch03`, hook form (shipped) | 5 = 5 | 5/5 identical | identical |

Under pdflatex the hook form and the preamble form give the same pages, streams and `.aux`
(`SAVE=` comparison: equal). `host_incremental::a_focused_compile_is_includeonly_and_keeps_the_documents_pages`
checks through the socket that the focused chapter's pages equal the whole document's own
pages (`\count0` and every glyph's font key, code and position), that an edit in the focused
chapter is `incremental` and equals a from-scratch focused compile, and that dropping the
focus equals a from-scratch whole compile.

## Latency (MEASURED, `scripts/bench_focus.sh`, load ~230)

Synthetic book: 44 `\include` chapters, 986 pages (`gen_book.py OUT 44 48`); focus on
`chapters/ch22` (34 pages focused: contents + the chapter). Keystrokes: 40 letters
inserted/deleted in the chapter's middle page, 300 ms apart, through `dl3-keys` as the app
sends them.

| | whole document | focused |
|---|---|---|
| open, cold, to convergence (`DONE`) | 54.3 s (3 passes, 2,948 pages typeset) | 3.25 s (2 passes, 68 pages) |
| open, first page | 228 ms | 418 ms |
| keystroke → edited page, p50 / p95 / max | 87.7 / 184 / 294 ms | 59.5 / 157 / 316 ms |
| keystroke → `DONE`, p50 / p95 | 418 / 792 ms | 522 / 934 ms |
| switch to it from the other job (stored S₀), all pages | 23.1 s | 1.45 s |
| same, first page | 397 ms | 668 ms |

Restart points of the focused keystrokes: page 20 (the edited page) in 40 of 40 after the
restart-point fix below; without it, page 0 every time (the whole focused document
re-typeset, edited page p50 149 ms).

## Restart-point fix (engine, `incr.rs` `restart_point`)

With the chapter looked up at `\begin{document}`, the checkpoints between that lookup and the
chapter's `\input` are not restart points (a changed file read before them and not open),
while those in the chapter before the edit are. The binary search assumed the good
checkpoints are a prefix and stopped before the gap (probe: checkpoints 119–156 bad, the
search ended at checkpoint 7). For one edited file a second binary search now finds the last
checkpoint that has not read the file past the edit (not opened yet, open at or before the
edit, past it: an order that only grows) and takes it when it is later, passes the existing
per-checkpoint test (`consumed_nothing_changed`) and is restorable. Soundness is still that
test's; only a later checkpoint is found.

Engine suites on this branch: `incremental` 62/62, `checkpoint` 3/3, `host_tools` 9/9,
`host_incremental` 16/17 + the new test; the one failure,
`a_keystroke_that_changes_nothing_has_its_page_at_once`, fails the same way with the original
`incr.rs` at this load (3 of 3 runs), a timing test. Soundness sweeps (hosted runners) are
for the reviewer to run (`gh workflow run sweeps.yml -f ref=<branch>`).

## Not done / beliefs

- Switching focus drops the resident document (a job change): the way back to the whole
  document re-typesets every page from the stored S₀ (23 s here, under load). Keeping two
  resident jobs is a follow-up.
- Export while focused first brings the whole document to convergence (the export's sync
  compile is the whole job), so the preview shows the whole document until the next edit.
- *Infinite Descent* splits its chapters with `\input` (`book/*/_*.tex`), so focus is not
  offered there; it was not measured.
