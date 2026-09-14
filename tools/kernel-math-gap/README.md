# Kernel math gap: the pdflatex oracle for the 21 commands still unsupported

Owner: lane `linux-kernel-math-gap` (Claude Code, machine `linux-primary`).
Created 2026-09-13. Python 3 standard library only; needs `pdflatex` on PATH.
**pdflatex is an oracle only and is never in the product path. Nothing here is
a parity claim, and nothing here changes a crate.**

```sh
python3 tools/kernel-math-gap/measure.py                    # pdflatex: slot, advance, atom class
python3 tools/kernel-math-gap/fontprobe.py                  # bundled faces: can we draw it?
python3 tools/kernel-math-gap/fontprobe.py --fonts apps/mac/Fonts
```

#180's sweep found 115 unsupported LaTeX kernel math commands. #180 closed 10,
#185/#186 closed 50, #200/#201 closed 33. The remaining **21** are the subject
of this directory: 9 `latexsym` symbols, 7 `cmex`/`cmmi`/`cmsy` assembly
pieces, `\sqsubset`/`\sqsupset`, `\dddot`/`\ddddot`, and `\mathscr`.

This is measurement, not implementation. The point is that every one of the 21
turns out to be blocked on something a symbol-table row cannot express, and the
numbers below are what say so.

## Method

`\showthe\wd` of `\hbox{$\sym$}` pins the family and slot a command is declared
with. The same box for `\hbox{$a\sym b$}`, against the `$ab$` control
(**9.57755 pt** at 10pt), pins the *atom class*: the glue TeX inserts between
two atoms is a function of the two classes alone, so the excess over control
minus the glyph's own advance is `Ord` 0, `Bin` 4mu both sides = 2.22222 pt
each, `Rel` 5mu both sides = 2.77778 pt each, `Punct` 3mu after only.

`measure.py` prints the class it *measures* beside the class the TeX source
*declares*, and flags any disagreement. On TeX Live 2025 pdfTeX
3.141592653-2.6-1.40.27 all 18 commands that get a class check agree, with zero
mismatches — the nine latexsym symbols, `\sqsubset`/`\sqsupset`, and the seven
pieces. The declarations quoted below for those 18 are confirmed, not assumed.
`\dddot`/`\ddddot` and `\mathscr` are measured by box, not by class: neither is
a `\DeclareMathSymbol`, so there is no declared class to check them against.

## The seven pieces are not extensible-assembly parts

They are plain `\DeclareMathSymbol` rows (`fontmath.ltx` 340, 374-377,
447-450), each a real glyph in a real TFM:

| command | family, slot | Computer Modern glyph | class | advance @10pt |
| --- | --- | --- | --- | ---: |
| `\lhook` | `letters` (cmmi) `"2C` | `arrowhookleft` | Rel | 2.77779 |
| `\rhook` | `letters` (cmmi) `"2D` | `arrowhookright` | Rel | 2.77779 |
| `\mapstochar` | `symbols` (cmsy) `"37` | `mapsto` | Rel | **0.00000** |
| `\braceld` | `largesymbols` (cmex) `"7A` | `bracehtipdownleft` | Ord | 4.50005 |
| `\bracerd` | `largesymbols` (cmex) `"7B` | `bracehtipdownright` | Ord | 4.50005 |
| `\bracelu` | `largesymbols` (cmex) `"7C` | `bracehtipupleft` | Ord | 4.50005 |
| `\braceru` | `largesymbols` (cmex) `"7D` | `bracehtipupright` | Ord | 4.50005 |

So PR #227's `AssemblyPart` / `vertical_assembly` machinery is **not** where
they belong: that reads a font's `MathVariants` table, and these seven are not
in anybody's `MathVariants`. Two further facts settle them:

1. **They already work as wholes, exactly.** `measure.py` builds each whole
   from its pieces the way `fontmath.ltx` does and gets the same box to the
   last unit: `\hookrightarrow` 11.11118 = `\lhook\joinrel\rightarrow`,
   `\hookleftarrow` 11.11118 = `\leftarrow\joinrel\rhook`, `\mapsto` 10.00002 =
   `\mapstochar\rightarrow`, `\longmapsto` 16.11119 =
   `\mapstochar\longrightarrow`. All four wholes, plus `\overbrace` and
   `\underbrace`, are already supported.
2. **Not one of the seven can be painted at pdflatex's advance from a bundled
   face** (`fontprobe.py`):
   - `\bracelu`, `\braceru`, `\lhook`, `\rhook`, `\mapstochar` have **no
     Unicode code point at all**, and no glyph in either bundled face.
   - `\braceld`/`\bracerd` *are* U+23B0/U+23B1 — the same two glyphs
     `\lmoustache`/`\rmoustache` use, because cmex `"7A`/`"7B` serve both roles.
     Latin Modern Math has neither character; New Computer Modern Math has both
     but at **7.52000 pt against cmex's 4.50005 — a +3.02 pt error each**,
     larger than the `\Diamond` error that decided #201 against bundling lasy.
   - The OpenType model deliberately does not expose these shapes separately.
     Latin Modern Math keeps them only inside U+23DE/U+23DF's *horizontal*
     assembly, where the two middle tips are **welded into one 2003-unit
     glyph**, and inside U+21A6's assembly, whose first part is a 499-unit
     stem-and-bar where `\mapstochar` is **zero width**.

A zero-width `\mapstochar` is the clearest case: there is no OpenType glyph
with that metric anywhere, because the whole point of the character is to be
overprinted by the arrow that follows it.

**Recommendation: diagnose all seven, do not draw them.** They are plain.tex's
private parts; exposing them would let a document build a construct the engine
cannot draw at the right metrics, when the composed command already draws
correctly. The diagnosis belongs in the gating arm #230 adds.

## latexsym: amsfonts already answers all nine, correctly

**This section replaces an earlier draft of it that was wrong.** The first
version of this file said aliasing the nine to amssymb "changes the atom class
of four of them", and recommended transcribing lasy10's metrics as constants.
That is true of the *naive* alias `\lhd` → `\vartriangleleft`, but it is not
what amsfonts does, and the naive alias is not the option on the table.

`latexsym.sty` declares eleven commands against `lasy10`:

| command | lasy slot | class | advance @10pt |
| --- | --- | --- | ---: |
| `\mho` | `"30` | Ord | 7.22223 |
| `\Join` | `"31` | Rel | 7.22223 |
| `\Box` | `"32` | Ord | 7.47224 |
| `\Diamond` | `"33` | Ord | 7.91673 |
| `\leadsto` | `"3B` | Rel | 10.00002 |
| `\lhd` `\unlhd` `\rhd` `\unrhd` | `"01`-`"04` | **Bin** | 7.77780 |
| `\sqsubset` `\sqsupset` | `"3C` `"3D` | Rel | 7.77780 |

**`amsfonts.sty` 151-162 provides all nine itself**, guarded by
`\@ifpackageloaded{latexsym}{\@tempswafalse}{\@tempswatrue}` — so a document
that loads `amssymb` (or `amsfonts`) and not `latexsym` already has every one
of them, from msam/msbm, **with the kernel classes intact**:

| command | declared | lasy wd | amssymb wd + class | delta | amsfonts declaration |
| --- | --- | ---: | --- | ---: | --- |
| `\mho` | Ord | 7.22223 | 7.22223 Ord | **+0.00000** | 101, AMSb `"66` |
| `\Join` | Rel | 7.22223 | 7.88913 Rel | +0.66690 | 161, AMSb `"6F` + `\mkern-13.8mu` + `"6E` |
| `\Box` | Ord | 7.47224 | 7.77780 Ord | +0.30556 | 152, `\let` to `\square` AMSa `"03` |
| `\Diamond` | Ord | 7.91673 | 6.66669 Ord | −1.25004 | 153, `\let` to `\lozenge` AMSa `"06` |
| `\leadsto` | Rel | 10.00002 | 10.00002 Rel | **+0.00000** | 154, `\let` to `\rightsquigarrow` AMSa `"20` |
| `\lhd` | **Bin** | 7.77780 | 7.77780 **Bin** | **+0.00000** | 159, AMSa `"43` as `\mathbin` |
| `\unlhd` | **Bin** | 7.77780 | 7.77780 **Bin** | **+0.00000** | 160, AMSa `"45` as `\mathbin` |
| `\rhd` | **Bin** | 7.77780 | 7.77780 **Bin** | **+0.00000** | 161, AMSa `"42` as `\mathbin` |
| `\unrhd` | **Bin** | 7.77780 | 7.77780 **Bin** | **+0.00000** | 162, AMSa `"44` as `\mathbin` |

**Every class is right and six of the nine are advance-exact.** The four
triangles keep `\mathbin` because amsfonts re-declares them on the AMSa slots
rather than `\let`ting them to the `\vartriangle*` relations — which is exactly
the trap the naive alias falls into:

| latexsym | naive alias | advance delta | what it breaks |
| --- | --- | ---: | --- |
| `\lhd` | `\vartriangleleft` | 0 | **Bin → Rel, +1.11111 pt of glue** |
| `\unlhd` | `\trianglelefteq` | 0 | **Bin → Rel, +1.11111 pt** |
| `\rhd` | `\vartriangleright` | 0 | **Bin → Rel, +1.11111 pt** |
| `\unrhd` | `\trianglerighteq` | 0 | **Bin → Rel, +1.11111 pt** |

Same glyph, same slot, wrong class. Use the amsfonts declarations, not the
`\vartriangle*` names.

One exception to the guard, which `measure.py` checks: with **both** packages
loaded, amsfonts leaves latexsym's design alone for every name except `\mho`
(lasy ht 6.83331 → msbm 6.88889, advance unchanged), because amsfonts 101's
`\ams@DeclareMathSymbol` is `\global\let#1\undefined` then re-declare — an
unconditional override sitting *outside* the `\if@tempswa` guard. Load order
does not change it.

### Recommendation

**Provide all nine through the amsfonts declarations the generator already
reads, gated on amssymb/amsfonts, and do not bundle lasy.**

`gen_amssymb.py` already parses `amsfonts.sty` (#212, #230), msam/msbm metrics
are already bundled — `FLASHTEX_TFM_DIRS` includes
`texmf/fonts/tfm/public/amsfonts/symbols`, and `\mathbb` already draws from
msbm — and #230's `provider` field is exactly the gate this needs. Nothing new
has to be bundled, licensed, or transcribed.

The residual, which should be *reported* rather than hidden: a document that
loads `latexsym` and not `amssymb` gets lasy10 in pdflatex, where this route
sets the msam/msbm design. Six of the nine are identical anyway; the three that
differ are `\Box` +0.30556, `\Join` +0.66690 and `\Diamond` −1.25004 pt. That
is a far smaller error than any font-substitution route, and much smaller than
the 2.33 pt that painting `\Diamond` from New Computer Modern Math's U+25C7
would cost (10.25000 against lasy's 7.91673; `fontprobe.py` reproduces #201's
figure, and Latin Modern Math has no U+25C7 at all).

This supersedes the earlier recommendation in this file to transcribe eleven
lasy10 constants: that would have been real work for a *worse* result than
using declarations the generator already reads.

## `\sqsubset` / `\sqsupset` — already closed twice, in two open PRs

Not kernel commands. `latex.ltx` leaves them as `\not@base` stubs; both
`latexsym.sty` (lasy `"3C`/`"3D`) and `amsfonts.sty` 99-100 (AMSa `"40`/`"41`)
declare them, and `amssymb.sty` 111-112 has them commented out precisely
because amsfonts already did it.

They are **the same advance (7.77780 pt) and the same `Rel` class either way,
but a different design**: under latexsym ht 5.39098 / dp 0.39098, under amssymb
ht 5.49860 / dp 0.35173. So which package a document loads changes the ink, and
package gating is a correctness requirement here, not hygiene.

PR #212 supplies them by removing `gen_amssymb.py`'s hardcoded `SKIP`; PR #230
independently reaches the same result through its `provider` field and states
that it unblocks #212. **This lane added nothing — duplicating them a third
time would be actively harmful.**

## `\dddot` / `\ddddot` — not accents

| expr | wd | ht |
| --- | ---: | ---: |
| `a` | 5.28589 | 4.30554 |
| `\dot{a}`, `\ddot{a}` | 5.28589 | 6.67859 |
| `\dddot{a}` | **10.00038** | 5.90553 |
| `\ddddot{a}` | **12.77817** | 5.90553 |

`\dot` and `\ddot` are real math accents and keep the nucleus width. `\dddot`
and `\ddddot` do not: `amsmath.sty` 744-750 builds them as
`\mathop{#1}\limits^{\vbox to-1.4\ex@{\hbox{\,\normalfont ...}}}`, so the dots
are **text-size roman periods** and the whole advance is that `\hbox`, exactly:
`\hbox{\,\normalfont...}` measures **10.00038** and `\dddot{a}` measures
**10.00038**; `\hbox{\,\normalfont....}` and `\ddddot{a}` both measure
**12.77817**; the difference between the two accents is **2.77779**, one cmr10
period, to the last unit. (The +4.71449 in the table is just the excess over
the nucleus, not the decomposition.) Setting them as a combining accent would give
roughly the right ink and a 4.7 pt-too-narrow box. They need a raised
text-size hbox over an operator nucleus, which is new layout machinery rather
than a table row.

## `\mathscr` — an alphabet from a package that is not bundled

Not a kernel command. It comes from `mathrsfs` (rsfs10) or `euscript`'s
`mathscr` option (eusm10); neither font is bundled, and the two are different
alphabets from each other:

| | `\mathscr{A}` | `\mathscr{F}` | `\mathcal{A}` |
| --- | ---: | ---: | ---: |
| mathrsfs | 10.31778 | 10.31780 | 7.98471 |
| euscript | 7.70540 | 6.76282 | 7.98471 |

Aliasing it to `\mathcal` is a 2.33 pt error on `A` under mathrsfs and picks
the wrong one of two alphabets. This is the `\mathbb` lesson again: the real
bug in `\mathbb` was pretending outside the range that was actually supported.
**Recommendation: diagnose "requires `\usepackage{mathrsfs}`" through #230's
gating.**

## Honest limits

- Every number here is TeX Live 2025 pdfTeX 3.141592653-2.6-1.40.27 at 10pt in
  `article`. Family-3 sizing differs at other sizes when amsmath redeclares
  `OMX/cmex` — the effect #204 and #229 deal with.
- `measure.py` reads `\wd`/`\ht`/`\dp` of an `\hbox`, so it measures the box
  TeX builds, not the ink. Two glyphs with the same advance and different
  outlines are indistinguishable to it; that is what `fontprobe.py` is for.
- `otfmath.py` is not a general OpenType reader: it does cmap formats 4 and 12,
  `hmtx`, and `MATH`'s `MathVariants` assemblies. It ignores `GSUB`, so a shape
  reachable only through a substitution is reported absent.
- The atom-class inference assumes text style and the default `\thinmuskip`,
  `\medmuskip`, `\thickmuskip`. It cannot separate `Ord` from `Open`/`Close`,
  which all add no glue between ordinaries.
