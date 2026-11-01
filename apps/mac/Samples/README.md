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

`demo.tex` is a multipage document with accented words and an em dash for
UTF-8 source-navigation checks; open it in the app. `demo-request.json` and
`demo-result.json` are a runtime-v1 request wrapping it and the old engine's
response, recorded before that engine retired (S7). The script that produced
them drove the retired compiler, so it was removed in S8; the files stay as
recorded samples and are no longer refreshed.
