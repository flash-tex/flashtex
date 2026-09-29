# Document-level differential fuzzer (t6-docgen)

Builds small complete LaTeX documents (article class) from real TeX Live
packages and runs each on the candidate and the reference through the
lockstep `capture()` with `fmt='pdflatex'`. MIT, Python 3 stdlib only.

## Run

```sh
FLASHTEX_POOL=$HOME/engine/pdftex.pool FLASHTEX_FORMATS=$HOME/engine/fmt \
python3 tools/fuzz/docgen.py --candidate BIN --oracle BIN \
  --out DIR --iterations N --seed S --timeout SEC
```

`FLASHTEX_POOL`/`FLASHTEX_FORMATS` pass through to the candidate only
(`run.candidate_env`), never the oracle. Each engine runs in its own
temp dir: a pdflatex run writes `doc.aux` next to the source, and
sharing one dir would hand the second engine the first one's aux file
(a spurious `(./doc.aux)` log difference).

## Generator

`generate_doc(rng)` (deterministic per seed) picks from a menu, all
choices through `rng`; package presence is checked with `kpsewhich`
at run time and missing packages are never selected:

- microtype option sets: half the draws use a spacing set
  (`spacing=true`, `spacing`, `spacing=true,expansion=true`,
  `spacing=true,protrusion=true`), the rest cover expansion,
  protrusion, tracking, letterspace (`letterspace=120`), final;
  font packages (lmodern, mathpazo, newtxtext, libertine), fontenc T1/OT1
- header block (`header=<shape>:<box>/<style>` in the options string):
  `\ps@headings` oddhead/oddfoot/both, custom `\ps@mine`, or fancyhdr,
  each with a lone-space box (`\hbox{ }`, `\hbox to 0pt{ }`,
  `\hbox{\ }`); otherwise a plain `\pagestyle` (plain/empty/headings)
  or a text-only `\ps@mine`
- title block (`\maketitle` or a hand-built centering block),
  empty `\null` page + `\newpage`, mid-document `\pagestyle` switch
- floats holding an empty box (`\hbox{}` figure) or an empty tabular,
  lists, footnotes, `\textsl \textit \textsc \emph` switches
- hyperref and geometry option sets, paragraphs with ligature
  (`ffi ffl fi fl`) and kern (`VA To AV`) pairs

## Known divergence: microtype spacing + lone-space header

This 12-line document panics the candidate (observed:
`thread 'main' panicked at crates/flashtex-engine/src/generated/
body_0.rs:1033:37: index out of bounds: the len is 9001 but the
index is 61867`, i.e. in `adjust_interword_glue`) while reference
pdfTeX succeeds:

```tex
\documentclass{article}
\usepackage[spacing=true]{microtype}
\makeatletter
\def\ps@headings{\def\@oddhead{\hbox{ }}}
\pagestyle{headings}
\makeatother
\begin{document}
\null
\newpage
x
\end{document}
```

Probed trigger (candidate `rc=101`): microtype with `spacing` (bare
or `spacing=true`, alone or combined) AND a shipped header/footer
box holding a lone space (`\hbox{ }`, `\hbox{\ }`, head or foot,
`\ps@` or fancyhdr). Either half alone is harmless, and
`\hbox to 0pt{ }` does not trigger. The generator pairs the two
halves on purpose (~1 in 8 docs), so a 300-iteration run against the
real candidate is expected to file several `candidate-crash` cases
for this bug.

## Classes

Same as `run.py` (`equal`, `diverge`, `candidate-crash`,
`oracle-crash`, `both-fail`, `timeout`), plus:

- `fontcount-diff`: otherwise equal runs whose raw log line
  `N words of font info for M fonts` names different M. The lockstep
  `compared_lines` normalises that line away, so it is compared
  directly from the raw logs; classification and signatures reuse
  `run.classify`/`run.signature` by import.

Findings save like `run.py`: `OUT/<class>/<sha256-prefix>.tex` plus a
`.json` sidecar (`options` string instead of seed/mutation), signature
dedupe in `OUT/signatures.json`, per-100 and final counts.

## Tests

```sh
python3 -m unittest discover -s tools/fuzz
```

Fake shell-script engines only; the real candidate is never used.
