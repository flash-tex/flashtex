# XeTeX lockstep harness

The P-T1 and XDV gate of the XeTeX port (`crates/flashtex-xetex`,
`docs/design/xetex/PLAN.md`), the XeTeX counterpart of `tools/lockstep`. It
runs each case through TeX Live 2026's `xetex -no-pdf` (an oracle only, never
in the product path) and through the port, with identical arguments and
environment, each in its own temp dir, both through a symlink named `xetex`,
and requires

* **P-T1** (DESIGN.md §1.1): the normalised transcript log -- every
  `\tracingall`-style trace line and every box dump at `\shipout` -- equal
  line for line, with `tools/lockstep`'s normalisation and accounting rule
  (its functions are imported, so the two harnesses cannot drift apart);
* the same exit status, and a case that runs as it says it does (to its
  `\end`, or into an error for a `% lockstep: no-halt` case);
* the **XDV file** byte for byte, after two normalisations: the date in the
  preamble's comment, and the path of a native font (S0 has none).

```sh
scripts/xetex-lockstep.sh                     # build the port, run every case
scripts/xetex-lockstep.sh --cases 'x0*'       # a subset (globs)
python3 tools/xetex-lockstep/run.py --engine <bin> [--jobs N] [--keep]
python3 tools/xetex-lockstep/run.py --self-test   # xetex against itself
python3 tools/xetex-lockstep/run.py --select      # rewrite suite.txt
```

The invocation is `tools/lockstep`'s (`-cnf-line=max_print_line = 1000
-cnf-line=error_line = 254 -ini -etex -interaction=nonstopmode
-halt-on-error`) plus `-no-pdf`, with `SOURCE_DATE_EPOCH=0`,
`FORCE_SOURCE_DATE=1`, `TZ=UTC`. The reference must report `XeTeX
3.141592653-2.6-0.999998 (TeX Live 2026)`.

## Cases

* **`suite.txt`**: the `tools/lockstep/cases` that use no pdfTeX-only
  primitive (the primitives of `pdftex.web` that `xetex.web` does not have)
  and that TeX Live's xetex runs as the case says (exit 0, or an error for a
  no-halt case) without an undefined control sequence. `--select` writes it.
* **`cases/`**: XeTeX-specific cases -- UTF-8, UTF-16 and byte input, bad
  UTF-8, `^^^^`, `\Uchar`, `\Ucharcat`, the `\Umath...` and `\XeTeXmath...`
  primitives, interchar tokens, `\XeTeXupwardsmode`, Unicode control
  sequences and case codes, lost characters, the native-font primitives
  applied to TFM fonts, `\strcmp`, `\mdfivesum`, `\filedump`, ... They are
  written by `make_cases.py` (some hold bytes an editor would change).
* **XeTeX's own tests** (`third_party/xetex/tests`, from TeX Live's
  `xetexdir/xetex-*.test`): run as the `.test` scripts run them (a format
  first, for `bug73` and `ctrlsym`), and compared with xetex's log and with
  the committed `.log`.

`prelude.tex` is `tools/lockstep/prelude.tex` without `\pdfoutput`, which
XeTeX does not have.
