# Unicode mode's M3 gate: harnesses

The M3 row of `docs/design/modes/PROPOSAL.md`'s milestone table ("Unicode in
the product") names the gate. This directory measures each criterion;
`.github/workflows/unicode-m3.yml` runs them on GitHub-hosted runners (Linux
with the pinned TeX Live 2026 as the oracle; macos-15, which has no TeX
Live, for the bundle-only run and Core Graphics). Expected results come
only from TeX Live's `xelatex`/`xetex`, never from FlashTeX.

| M3 criterion | harness |
|---|---|
| Unicode corpus (fontspec, unicode-math, polyglossia, xeCJK; Q2's corpus) at P-T1 100 % | `corpus.py assemble`, then `tools/xetex-lockstep/latex.py --cases-dir` (P-T1 + XDV against `xelatex -no-pdf`) |
| Export gate per Q2 | `tools/xetex-pdfparity/run.py --cases-dir --no-visual --pdf-dir` (structural, Linux), `visual.py` (Core Graphics at 2× and 3×, macOS) |
| §1.2 latency on xelatex bench documents | `latency.py` (instructions on hosted runners; wall time pending the reference Mac) |
| Incremental soundness sweeps on Unicode | `host_sweep.py` (the host's reuse between compiles: spares against cold); Unicode mode has no incremental engine to sweep yet |
| A no-TeX-Live run of the corpus from the bundle | `notex_corpus.py --mode notex` (macOS, sandboxed), compared with `--mode texlive` and with xelatex |
| `[fonts]` fixtures: P-T1 + XDV against xelatex on the effective source; precedence | `fonts-cases/` through `latex.py` (first line from `fonts_line`, expected and forbidden fonts per fixture), and `crates/flashtex-xetex/tests/host.rs` |
| Classic lockstep, P-T1/P-T2, latency, `flashtex-host` size and start-up unchanged | `host_startup.py` at each U3 merge and its first parent (the `classic` job); Classic's own gates in `ci.yml` |

## The corpus

`corpus.json`: the existing xelatex cases (`tools/xetex-lockstep/latex-cases`,
`tools/xetex-pdfparity/cases`), new ones in `cases/` (polyglossia with Latin,
Greek, Cyrillic, Hebrew and Arabic scripts; xeCJK and ctex with Chinese and
Japanese; fontspec by family name; unicode-math with four more math fonts;
beamer), TeX Live documentation sources (fontspec's and unicode-math's
examples, the unicode-math symbol list, polyglossia's manual, xeCJK's twelve
examples, the thuthesis template), fetched from tlnet's archive and pinned by
SHA-256, and the `[fonts]` fixtures of `fonts-cases/`. Fonts a document names
that TeX Live does not ship (Windows and Adobe CJK fonts) are rewritten to
TeX Live's own in the source both engines read (`cjk_subst`, `subst`).

```sh
python3 tools/unicode-m3/corpus.py list
python3 tools/unicode-m3/corpus.py assemble --out DIR --fonts-line crates/flashtex-xetex/target/release/examples/fonts_line
python3 tools/xetex-lockstep/latex.py --engine BIN --cases-dir DIR --json pt1.json
python3 tools/xetex-pdfparity/run.py --engine BIN --cases-dir DIR --no-visual --pdf-dir PDFS --json export.json
python3 tools/unicode-m3/summarize.py RESULTS-DIR
```

A case directory may set `trace` to `boxes` (box dumps at every `\shipout`,
no `\tracingall`) for documents too long to trace in full; the P-T1 results
say which cases were compared that way.

Fonts by name: on Linux xetex finds them through fontconfig and the port
through its own index, so the workflow gives both the same directories
(TeX Live's `fonts/opentype` and `fonts/truetype`, and `/usr/share/fonts`).
