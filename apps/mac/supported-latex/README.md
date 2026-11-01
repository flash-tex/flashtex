# supported-latex

The editors' completion vocabulary (`supported-latex.json`, schema `flashtex-supported-latex/1`), generated from the new engine's measured package support, with pdflatex as the oracle (retirement plan RQ5, ruled 2026-10-11). Every file here is MIT and app-owned. None is written by hand: `apps/mac/scripts/gen-supported-latex.py` writes all three.

| File | Step | From | Needs TeX |
|---|---|---|---|
| `package-smoke.json` | `record` | `tools/package-smoke/run.py`'s stdout from a P5 scoreboard run (`smoke-new.txt`) and its `oracle.json`: which documents the new engine typesets as pdfTeX 1.40.29 does on both passes (transcripts, box dumps, P-T2 PDFs) | no (the run did) |
| `probe.json` | `probe` | pdflatex: what the LaTeX kernel (article class) defines, and what each verified package loaded by at least 20 of the 400 papers in `tools/package-smoke/sample-400.json` adds | yes, TeX Live 2026 |
| `supported-latex.json` | `render` | the two files above and the `tools/package-smoke` manifests | no |

`render` lists every package-smoke package with its verdict, and puts the kernel's and the vocabulary packages' commands and environments in the rows both editors decode. A command is offered in math only when pdflatex defines it as a math character or its body starts with a math-only primitive; any other command is offered in text and in math, because the oracle does not rule it out.

## Checks

- CI's `checks` job runs `gen-supported-latex.py check`: the committed `supported-latex.json` must be exactly what `render` makes from the committed inputs. `scripts/check-generated.py` records the generator's and the file's SHA-256.
- The `supported-latex` workflow runs `probe --check` on the pinned TeX Live image whenever the generator or its inputs change. It uploads the probe it measured as the artifact `supported-latex-probe`. To update, commit that artifact's `probe.json` and run `render`.
- To re-measure package support, take a newer P5 board's `smoke-new.txt` and `oracle.json`, then run `record`, `probe` (or the workflow) and `render`.

## Until S7

The app still bundles the old compiler's inventory, kept byte-identical by `apps/mac/scripts/sync-supported-latex.sh`. S7 deletes that source, and `render --install` then writes this file to `apps/mac/Sources/FlashTeXMac/Resources/` and `apps/ios/FlashTeXPad/Resources/`. `check --installed` replaces `sync-supported-latex.sh --check` in CI, `gate.sh` and `make-app.sh`.
