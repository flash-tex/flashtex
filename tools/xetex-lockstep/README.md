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
* the **XDV file** byte for byte, after the two normalisations of
  docs/design/xetex/PLAN.md §3.5: the date in the preamble's comment, and
  in each `define_native_font` record the font file's path, replaced by
  its base name (the stream is walked opcode by opcode; every other byte
  of the record is compared). `run.py --test-normalise` checks the
  normalisation alone; every run checks it first.

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

## Phase S1: native fonts

* **`cases/n*.tex`** (written by `make_cases.py`): OpenType and TrueType
  fonts named by file -- Latin Modern and TeX Gyre from TeX Live, and
  macOS's Times New Roman and Helvetica.ttc by absolute path -- with
  features, the common options (`letterspace`, `color`, `extend`, `slant`,
  `embolden`), glyph and OpenType-layout queries, glyph metrics,
  hyphenation, interword-space shaping, justified boxes and several fonts
  per XDV file. A machine without one of the fonts fails the same way in
  both engines.
* **`latex.py`** and **`latex-cases/`**: `xelatex` documents with
  `fontspec`. Each engine builds its own `xelatex.fmt` as fmtutil does,
  then each case runs twice with each engine (untraced, then traced as
  tools/parity's P-T1 capture), and the traced logs, exit statuses and
  XDV files are compared as above.

```sh
python3 tools/xetex-lockstep/run.py --engine <bin> --cases 'n0*' --no-xetex-tests
python3 tools/xetex-lockstep/latex.py --engine <bin> [--cases 'l00*'] [--keep]
```
