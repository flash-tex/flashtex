# Grammar-based parity fuzzer (fuzz-parity)

Differential fuzzing of FlashTeX's engine (`flashtex-initex`) against
pdfTeX on generated plain-TeX documents. MIT, Python 3 stdlib only (plus
`qpdf` for the P-T2 check when two PDFs differ in bytes). pdfTeX from TeX
Live is the oracle and only the oracle (DESIGN §13); nothing here is in
the product path.

`tools/fuzz` mutates seed cases; this tool instead *generates* whole
documents from a grammar aimed at corners the arXiv corpus rarely
reaches.

## Run

```sh
python3 tools/fuzz-parity/fuzz.py --candidate target/release/flashtex-initex \
    --seeds 0:500 --workers 2 --out /tmp/fzp --timeout 60
```

- `--seeds START:COUNT`: each seed is one document; `gen.generate(seed)`
  is a pure function of the seed (one `random.Random(seed)`), so a seed
  number plus the generator's revision is the whole test case.
- `--workers` is capped at 2.
- Exit 0 when every seed was equal (or `both-hang`/`both-signal`). Each non-equal seed is written to
  `OUT/<class>/<seed>/` (`doc.tex`, both normalised logs, `result.json`
  with the first difference); `OUT/summary.json` groups seeds by
  signature.

Each engine's `doc.tex` gets the same modification time (2001-09-09),
so `\pdffilemoddate{doc.tex}` is comparable.

Print one document: `python3 tools/fuzz-parity/gen.py 42`.

## What runs

Both engines run each document through the lockstep harness's
`capture()` (`tools/lockstep/run.py`): `-ini -etex`, nonstopmode without
`-halt-on-error` (so error recovery is compared too), max_print_line 1000,
error_line 254, `SOURCE_DATE_EPOCH=0`, `FORCE_SOURCE_DATE=1`, the same
`pdftex` argv[0], and shell escape off (`-cnf-line=shell_escape=f`). Each
engine gets its own directory. A document starts with
`\input plain` in INITEX (so `\patterns` are still allowed), sets
`\pdfoutput` (80% PDF, 20% DVI) and a random subset of the tracing
switches, `\showboxdepth`/`\showboxbreadth` and the interaction mode.

## What is compared (P-T1/P-T2, DESIGN §1.1)

In order; the first difference names the class:

| class | difference |
|---|---|
| `candidate-crash` | candidate died by a signal or exited 101 (panic) |
| `oracle-crash` | pdfTeX died by a signal and the candidate did not: nothing to match, but a pdfTeX bug to look at |
| `log` | the log after the lockstep normalisation (`compared_lines`: banner, paths, dates and the §1.1 accounting lines replaced by placeholders). It holds every `\showbox`, `\showlists`, `\show...` and tracing line and every shipped box |
| `rc` | exit codes |
| `output` | only one engine wrote a PDF/DVI |
| `pdf` | PDF bytes differ and `tools/parity` `compare_pt2` fails (qpdf-normalised content streams, fonts, resources, media boxes) |
| `dvi` | DVI bytes differ |
| `terminal` | stdout and stderr differ (after the same normalisation). kpathsea's `Not writing to`/`Not reading from` refusals are taken out first and compared as a list of their own (`kpse-msg`): the capture merges stderr into stdout, so where a refusal lands depends on when the engine last flushed stdout |
| `timeout` | one engine exceeded `--timeout` |

Two outcomes are not differences and do not fail a run: `both-hang`
(both engines exceeded `--timeout`) and `both-signal` (both died by the
same signal, in practice SIGXFSZ when a runaway loop fills the 64 MiB
output cap).

The signature is the class plus the first differing pair of lines with
digits masked, so one cause found at many seeds counts once.

## Grammar

`gen.py`, one method per area. A document is a preamble plus 4–14
independent chunks, each balanced on its own:

- paragraphs: `\parshape`, `\hangindent`/`\hangafter`, `\looseness`,
  `\emergencystretch`, `\lastlinefit`, tolerances, skips, the demerits and
  penalties, `\interlinepenalties`/`\clubpenalties`, `\pdfadjustspacing`,
  `\pdfprotrudechars` with `\rpcode`/`\lpcode`, `\pdfeachlinedepth`;
- hyphenation: `\patterns` for languages 1 and 2 in the preamble,
  `\hyphenation`, `\uchyph`, `\lefthyphenmin`/`\righthyphenmin`,
  `\hyphenchar`, `\exhyphenchar`, `\savinghyphcodes`, `\discretionary`,
  `\-`;
- math: `\mathchoice`, `\left`/`\middle`/`\right`, `\radical`, `\sqrt`,
  generalised fractions with and without delimiters, limits, `\nonscript`,
  `\mkern`/`\mskip`, style changes, the math spacing and delimiter
  parameters, `\fontdimen` changes of families 2 and 3, displays with
  `\eqno`/`\leqno`;
- vertical: `\vsplit` with `\splittopskip`/`\splitmaxdepth`, split marks,
  `\savingvdiscards`/`\pagediscards`/`\splitdiscards`, inserts,
  `\vadjust`, `\mark` and e-TeX `\marks`, `\leaders`/`\cleaders`/`\xleaders`
  (horizontal and vertical), output routines that show and ship boxes;
- alignments: `\halign`/`\valign` with `\span`, `\omit`, `\noalign`,
  `\tabskip`, `to`/`spread`, `\everycr`;
- characters and fonts: `\accent`, ligature/kern programs with
  `\noboundary` and `\/`, `\char` edge values, `\fontdimen` changes and
  extension, `\font ... at/scaled` edge sizes, `\pdffontexpand`,
  `\letterspacefont`, `\pdfcopyfont`, `\pdfnoligatures`, `\tagcode`,
  `\efcode`;
- expressions: `\numexpr`/`\dimexpr`/`\glueexpr`/`\muexpr` rounding and
  overflow, `\multiply`/`\divide`/`\advance` overflow, out-of-range
  dimensions and integers, `\gluestretch` and friends;
- strings: `\pdfstrcmp`, `\pdfescapestring`/`name`/`hex`, `\pdfunescapehex`,
  `\pdfmdfivesum`, `\detokenize`, `\unexpanded`;
- `\showbox`, `\showlists`, `\showthe`, `\showgroups`, `\showifs`,
  `\showtokens`, the tracing parameters, `\batchmode`/`\scrollmode`/
  `\nonstopmode`/`\interactionmode`, and a list of error-recovery inputs;
- pdfTeX objects and specials (`\pdfliteral`, `\pdfobj`, `\pdfxform`,
  links, destinations, outlines, `\pdfsavepos`, `\pdfcolorstack`), and
  file I/O (`\openout`/`\write`/`\openin`/`\read`/`\readline`,
  `\scantokens`, `\everyeof`);
- 8-bit and control characters through `\message`, `\write`, `\show`
  and `\tracinglostchars`, `\escapechar`/`\newlinechar`/`\endlinechar`
  edge values, lines longer than `max_print_line`;
- runs that end in a capacity overflow or a fatal error (rare, and last);
- pdfTeX's C parts and web2c (`web2cstuff`): `\pdfmatch` and
  `\pdflastmatch` (POSIX regex, invalid patterns), `\pdffilesize`,
  `\pdffiledump`, `\pdfmdfivesum file`, `\pdffilemoddate`, quoted and
  braced file names, `\tracingstacklevels`, `\showstream`,
  `\partokenname`/`\partokencontext`, `\lastnodetype`, `\pdfinsertht`,
  `\pdfcolorstackinit`, the interword-glue and kerning codes
  (`\knbscode` & co.), `\pdftracingfonts`, map files and lines;
- images and fonts at shipout (`media`): `\pdfximage` of TeX Live's
  mwe PDF, PNG and JPEG examples with sizes, pages, page boxes and the
  image parameters; virtual fonts (Times, Palatino, Helvetica); font
  expansion with and without `autoexpand`; `\letterspacefont`,
  `\pdfcopyfont`, protrusion; the ToUnicode and omit switches;
- e-TeX's less used parts (`etex`): TeX--XeT (`\TeXXeTstate`,
  `\beginL`/`\endR`, unmatched ends, displays, leaders and breaks in
  reversed text), `\fontcharwd` & co., `\parshapelength`/`indent`/`dimen`,
  `\currentgrouptype`/`\currentiftype`/`\currentifbranch`, the glue
  queries, `\lastlinefit`, discards, `\tracingnesting` across
  `\scantokens`, `\protected\edef`;
- the input file's own bytes (`rawbytes`): CR and CR LF line ends,
  trailing blanks, form feeds, NUL/DEL/8-bit bytes, `\endlinechar`,
  `\obeylines`, long lines;
- the PDF writer (`pdfwriter`): object streams and files, reserved and
  reused object numbers, annotations, links of every kind, destinations,
  outlines, threads, the info/trailer/version keys, compression levels,
  page boxes and sizes, missing images, forms, colour stacks, and
  `openout_any`/`openin_any` (absolute, `..` and dot-file names).

A document never replaces the font map (`\pdfmapfile` without `+`):
pdfTeX would then run kpathsea's `mktexpk` for every font, which writes
into the TeX tree and is not something the two engines share.

Random numbers: every `\pdfuniformdeviate`/`\pdfnormaldeviate` the grammar
emits follows a `\pdfsetrandomseed`, and the preamble sets one too (pdfTeX seeds from the clock, and a deviate in an argument or a `\write` can be expanded before the `\pdfsetrandomseed` beside it runs).

## Minimise

```sh
python3 tools/fuzz-parity/minimize.py --candidate BIN --seed 123 --out min.tex
python3 tools/fuzz-parity/minimize.py --candidate BIN --tex OUT/log/123/doc.tex
```

ddmin over chunks, then lines, then space-separated tokens, keeping an
input only while it differs with the same class and signature (`--any`:
any difference).

## Smoke test

```sh
python3 -m unittest tools/fuzz-parity/test_fuzz_parity.py          # < 30 s
FUZZ_PARITY_CANDIDATE=BIN python3 tools/fuzz-parity/test_fuzz_parity.py
```

Generator determinism and shape, ddmin, and (with `pdftex` on PATH) the
oracle against itself on fixed seeds 0, 1, 2 and 7: the harness must call
two runs of one engine equal. With `FUZZ_PARITY_CANDIDATE` the candidate
is compared with the oracle on the same seeds.
