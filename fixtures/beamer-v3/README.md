# Beamer corpus for the new engine (lane BEAMER-V3, 2026-10-03)

Small decks written for the `beamer` parity tier (`tools/parity/corpus/beamer.json`).
Each directory holds one document, `main.tex`. The pdflatex reference is made by the
local pdflatex when the tier runs (the oracle); nothing generated is committed.
Together with the ten `fixtures/real-world/beamer-*` fixtures (the gated fixtures tier)
and the TeX Live beamer examples the manifest names, they cover:

| directory | covers |
|---|---|
| `warsaw-beaver` | `\usetheme{Warsaw}` + `\usecolortheme{beaver}`, sections, navigation bars, TOC, blocks, columns |
| `madrid-dolphin` | `\usetheme{Madrid}` + `\usecolortheme{dolphin}`, `\logo`, theorem environments, `\againframe` |
| `metropolis` | `\usetheme{metropolis}` under pdflatex, progress bar, standout frame, appendix |
| `overlays-advanced` | `\pause`, `\only`, `\uncover`, `\visible`, `\invisible`, `\alt`, `\temporal`, `<+->`, `[<+->]`, `\alert<>`, `overlayarea`, `actionenv`, transitions, buttons |
| `handout-2on1` | `handout` mode with `| handout:` overlay specs, `pgfpages` 2 on 1 |
| `graphics` | `\includegraphics` (PDF, PNG, JPEG) with overlays, a background image, a `\logo` image |
| `tikz-overlays` | TikZ with `\only`, `\visible`, `\pause` and an `onslide` style inside the picture; `remember picture, overlay` |
| `notes` | `\note`, `show notes on second screen=right` (pgfpages) |
| `bibliography` | `thebibliography` with beamer's bibitem templates, `\cite` |
| `allowframebreaks` | `[allowframebreaks]` on a long list and a long bibliography |
| `widescreen-169` | `aspectratio=169` with columns and blocks |
| `article-mode` | `beamerarticle`: frames typeset as an article |
| `long-deck` | 60 frames, many with overlays, for the incremental benchmark |
