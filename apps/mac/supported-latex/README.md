# supported-latex

The editors' completion vocabulary (`supported-latex.json`, schema `flashtex-supported-latex/1`), generated from the new engine's measured package support, with pdflatex as the oracle (retirement plan RQ5, ruled 2026-10-11). Every file here is MIT and app-owned. None is written by hand: `apps/mac/scripts/gen-supported-latex.py` writes all four.

| File | Step | From | Needs TeX |
|---|---|---|---|
| `package-smoke.json` | `record` | `tools/package-smoke/run.py`'s stdout from a P5 scoreboard run (`smoke-new.txt`) and its `oracle.json`: which documents the new engine typesets as pdfTeX 1.40.29 does on both passes (transcripts, box dumps, P-T2 PDFs) | no (the run did) |
| `probe.json` | `probe` | pdflatex: what the LaTeX kernel (article class) defines, and what each verified package loaded by at least 20 of the 400 papers in `tools/package-smoke/sample-400.json` adds | yes, TeX Live 2026 |
| `annotations.json` | `annotate` (a one-off) | the retired compiler's generated inventory as of f16430793 (`apps/mac/Sources/FlashTeXMac/Resources/supported-latex.json` there): each row's mode, origin, argument shape and glyph, in its order; no descriptions (they described the deleted compiler) and no class-scoped rows | no |
| `supported-latex.json` | `render` | the three files above and the `tools/package-smoke` manifests | no |

`render` lists every package-smoke package with its verdict and writes the rows both editors decode. Presence is measured, presentation is annotated:

1. Every `annotations.json` row, in its order, kept only when the probe measured its name (the kernel or a vocabulary package defines it, either mode). The row takes the annotation's mode, origin, argument shape and glyph, and the first measuring source's `source` and description ("LaTeX kernel", "\usepackage{amsmath}"; "accepted" for the math twin of a text row).
2. Then each vocabulary package's measured commands that no annotation names, marked `"annotated": false`, except a package's `\X`/`\endX` for its own environment `X`. The kernel's unannotated names are not added: they are mostly internals (`\secdef`, `\sectionmark`, `\setbox`). Such a command is offered in math only when pdflatex defines it as a math character or its body starts with a math-only primitive; otherwise in text and in math, because the oracle does not rule it out. Both editors offer an unannotated command only when no annotated one matches what was typed, so babel's `\frq` never buries `\frac`.
3. Environments follow the same rule.

Control symbols (`\\`, `\,`) have no row: the probe measures letter-named commands only. Nor has a kernel command the annotations do not present (`\def`, `\newline`).

## Checks

- CI's `checks` job runs `gen-supported-latex.py check`: the committed `supported-latex.json` must be exactly what `render` makes from the committed inputs. `scripts/check-generated.py` records the generator's and the file's SHA-256.
- The `supported-latex` workflow runs `probe --check` on the pinned TeX Live image whenever the generator or its inputs change. It uploads the probe it measured as the artifact `supported-latex-probe`. To update, commit that artifact's `probe.json` and run `render`.
- To re-measure package support, take a newer P5 board's `smoke-new.txt` and `oracle.json`, then run `record`, `probe` (or the workflow) and `render`.

## The bundled copies

Both editors bundle this file: `render --install` writes it to `apps/mac/Sources/FlashTeXMac/Resources/` and `apps/ios/FlashTeXPad/Resources/`, and `check --installed` (CI's `checks` and `mac-app` jobs, `gate.sh`, `make-app.sh`) refuses a stale copy. S7 retired the old compiler's inventory and `sync-supported-latex.sh`.

The vocabulary is what completion offers, not a list of what the engine runs. A command, class or package with no row is unknown to completion and is never marked unsupported: the editors show a document's other commands as "used in this document", and a package package-smoke measured equal to pdfLaTeX as "typeset as pdfLaTeX does".
