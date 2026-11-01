Samples for manual checks of the Mac shell. These are hand-written examples in
runtime v1 shape owned by apps/mac; the authoritative contract fixtures live in
`protocol/fixtures/` (Commander-owned). `capture-proposal.json` loads via
Edit > Open Capture Proposal… after pinning an insertion point (⌘⌥P).
`multipage-result.json` / `multipage-request.json` are a request and the old
engine's result for a two-page document with non-ASCII source text and two
diagnostics. File > Open Compile Result Fixture…, which loaded them into the
window, was removed with the old engine's route (retirement stage S6); to
check click-to-source, caret sync and diagnostics, open a `.tex` file holding
the request's `main.tex` text instead.

`recovery-demo.tex` is a short document that exercises sections, `\textbf`/`\emph`,
non-ASCII words, and two deliberately unsupported constructs (`$x^2$`, `\foo`) so
the old compiler's `recovered` status and diagnostics are visible. The app's
engine is pdfLaTeX-compatible, so it typesets `$x^2$` and reports `\foo` as
TeX's `Undefined control sequence`.

`demo.tex` is a multipage document using only the compiler's supported LaTeX
subset, with accented words and an em dash for UTF-8 source-navigation checks.
Run `python3 apps/mac/Samples/make-demo-request.py` from the repository root
to wrap that source in the runtime-v1 `demo-request.json`, invoke the compiler
at the scratchpad binary path recorded in the script, and save its response as
`demo-result.json`. The script uses only the Python 3 standard library, prints
status, page count, and diagnostic count, and fails unless compilation is `ok`
with zero diagnostics and at least two pages; rerun it after editing the
source to refresh both JSON files. The app no longer opens result files: open
`demo.tex` itself.
