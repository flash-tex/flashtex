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

## Font lookup by name (`fontmatch.py`)

`fontmatch.py` measures the port's font lookup
(`crates/flashtex-xetex/src/fontmgr`: `splitFontName`, `XeTeXFontMgr`'s
matching rules over the platform-free font index, PLAN.md §3.1 and §5)
against TeX Live 2026's `xetex -no-pdf`. Each name in `fontnames.txt` (one
`\font` name per line, optionally a tab and `12` or `scaled 1200`) runs in a
fresh xetex, and the face xetex chose is read from the XDV's
`define_native_font` record (path and face index); the port answers through
`examples/find_font`, with a fresh font manager per name.

```sh
(cd crates/flashtex-xetex && cargo build --release --example find_font)
python3 tools/xetex-lockstep/fontmatch.py [--names FILE] [--jobs N] [--json OUT]
```

It reports, separately: faces equal (path and index), the `name_of_file`
XeTeX leaves (`\fontname`), and the size the font is loaded at (the XDV
record's; macOS's xetex records a font it shapes with Core Text, an
AAT-only font such as Helvetica, in big points). A name xetex's lookup
finds but whose face then fails to load (`\XeTeXtracingfonts` shows the
lookup succeeded) is unverifiable and counted apart. Every difference is
printed with both faces.

On macOS xetex asks Core Text; the port follows XeTeX's fontconfig build
over its own index, so a difference is either the index (which files are
seen) or the platform search (which faces get into the maps). The module
documentation of `fontmgr` lists what is known.

**Known differences** (`fontmatch-known.txt`, measured on macOS): 2 faces
and 7 `name_of_file`s out of 180 lookups, all from Core Text: variable
fonts' named instances (`STIX Two Text Bold` is found by xetex only; Core
Text's instance names for `STIX Two Text` and `Inter`), and two installed
families both named `SF Mono` (`SFMono-Regular` is found by the port
only). They print as `KNOWN`; the run fails on a difference not listed or
a listed one that no longer differs.

The port's catalog keeps each face's names in a cache
(`$FLASHTEX_CACHE_DIR`, else the user's cache directory,
`xetex-font-names.json`), keyed by path, face index, size and modification
time, so only the directory scan is repeated on a later run.
