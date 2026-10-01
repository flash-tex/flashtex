# Hybrid conceal — evidence, 2026-09-30

Rendered by `HybridConcealEditorTests.testWritesEvidenceScreenshotsWhenRequested`
(`FLASHTEX_CONCEAL_EVIDENCE=<this folder> swift test --filter HybridConcealEditorTests`)
on mac-m1max-a: a real `SourceEditorView` in a hosted window that is never
activated and never on a display (HostedWindowSupport, `orderFrontRegardless`).
The test asks `screencapture -l <window id>` first; for a window parked off
every display it exits 1 ("could not create image from window"), so each
image is the window's own drawing (`cacheDisplay`), which is what AppKit
draws on screen. The storage is asserted unchanged after every step.

| File | What it shows |
|---|---|
| `1-concealed-caret-on-last-line-light.png`, `…-dark.png` | Caret on `\end{itemize}`: every other line concealed — § heading, α ≤ β², xᵢ ∈ ℝ, ∑ₙ, → ∞, bold and italic with the commands hidden, “quotes”, – and —, • items, the comment dimmed. |
| `2-caret-line-revealed-light.png`, `…-dark.png` | Caret moved onto line 2: that line shows its source, the rest stay concealed. |
| `3-construct-mode-caret-after-alpha.png` | Reveal "At the caret": only `\alpha`, which the caret touches, shows its source; `\leq` on the same line stays ≤. |
| `4-deny-textbf-and-leq.png` | Never conceal `\textbf, \phi, \leq`: those stay as typed, the rest conceal. |
| `5-off-raw-source.png` | Master switch off: the plain source. |
