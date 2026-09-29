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

- microtype option combinations (expansion, protrusion, spacing,
  tracking, letterspace, final), font packages (lmodern, mathpazo,
  newtxtext, libertine), fontenc T1/OT1
- `\pagestyle` (plain/empty/headings) or a custom `\ps@` header
- floats (figure, table with tabular), lists, footnotes,
  `\textsl \textit \textsc \emph` switches
- hyperref and geometry option sets, paragraphs with ligature
  (`ffi ffl fi fl`) and kern (`VA To AV`) pairs

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
