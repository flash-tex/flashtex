# Test oracles

These are the crates [DESIGN.md](../../docs/design/engine-v2/DESIGN.md) §10 keeps
"as oracles and tests" after the old engine retired. Retirement stage S8
([plan](../../docs/design/engine-v2/retirement-plan.md) §2, RQ4) moved them here
from `crates/`, so that what ships and what only checks are apart.

| Crate | What it is an oracle for | Its expected data and the generator that writes it |
|---|---|---|
| `tex-expansion` | TeX/e-TeX/LaTeX macro expansion (a T6 expander oracle) | `tests/oracle/expected/`, `manifest.json`: `tests/oracle/gen/cases.py` (run from this crate) |
| `tex-boxes` | hpack/vpack, glue setting, shipout rounding (a T6 box-builder oracle) | `tests/oracle/expected.txt`: `tests/oracle/generate.py` |
| `paragraph-layout` | Knuth–Plass line breaking, hyphenation | `tests/oracle/emergency/expected/`: `tests/oracle/emergency/generate.py`; `tests/oracle/hyphenation_corpus.txt`: `tests/oracle/gen_hyphenation_oracle.py` (`tests/oracle/showhyphens.txt` is a recorded `\showhyphens` log) |
| `microtype` | pdfTeX protrusion and expansion as microtype configures them | `tests/oracle/expected/`: `tests/oracle/generate.py` |
| `document-style` | (a normal dependency of paragraph-layout, kept only for it) | — |
| `math-layout` | TeXbook Appendix G math layout; its `src/tfm.rs` is the TFM reader the T6 TFM fuzzer compares with the engine's | `src/cm_tfm.rs`, `src/ams_tfm.rs`: `tools/gen_cm_tfm.py` (`scripts/check-generated.py` tracks both) |
| `page-builder` | TeX's page builder, insertions, `\vsplit` | `oracle/expected/`: `oracle/gen.py` |

Rules:

- **Expected data comes only from the oracle.** Every generator above runs TeX
  Live 2026's `tex` or `pdflatex` (MacTeX here) and writes what TeX printed; a
  test that disagrees is fixed in the crate, never by editing expected data. TeX
  is never in the product path: `cargo test` reads the committed files and runs
  no TeX.
- **MIT, and never linked to the engine.** Each crate declares
  `license = "MIT"` (bundled TeX Live data files keep their own terms, named in
  the crate's `Cargo.toml`). Check A of `scripts/check-license-boundary.sh` keeps
  `flashtex-engine` out of their dependency graphs, and check G requires every
  package to declare a licence.
- **Nothing ships them.** No product crate, app or package depends on these
  crates; they are root workspace members (`members = [..., "tests/oracles/*"]`
  in the root `Cargo.toml`), so `cargo test --workspace` and `scripts/gate.sh`
  test them as before. `gate.sh` maps a changed `tests/oracles/<crate>/` path to
  that crate's package, exactly as it does for `crates/<crate>/`.
