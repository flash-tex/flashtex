# Bundled fonts

Latin Modern (OpenType CFF) from CTAN `fonts/lm` (Latin Modern 2.005 / LM Math
1.959), redistributed under the GUST Font License v1.0 (`GUST-FONT-LICENSE.TXT`).
These are the LaTeX default faces (Computer Modern design); the preview, the
CoreGraphics export, and `flashtex-pdf --default-face lm` use them so the app does
not depend on a TeX installation. `scripts/make-app.sh` copies this directory to
`Contents/Resources/Fonts`; `PreviewFonts` registers it (`FLASHTEX_LM_DIR` overrides).
Included: lmroman 7/10/12/17 masters (regular; 10/12 also bold/italic/bolditalic)
and `latinmodern-math.otf` (LM Math 1.959, 733,736 bytes, sha256
6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f — the file
MacTeX 2026 ships at texmf-dist/fonts/opentype/public/lm-math/). Nothing else in
the repository embeds proprietary fonts.

## Rooted TeX metrics (`texmf/`)

`texmf/fonts/tfm/public/lm/{ec-lmr10,ec-lmr12,rm-lmr12,rm-lmr8,rm-lmr6}.tfm` and
`texmf/doc/fonts/lm/GUST-FONT-LICENSE.TXT` are the official Latin Modern 2.004
metrics (GUST `lm2.004bas.zip`, archive sha256
97a725ea012d41367bf44fec1a2f4ccf4fe134c016715522133594e347115a7c) that
`flashtex-render` needs for the 10 pt / 12 pt regular text and 12 pt roman-math
fixtures, laid out exactly as a `texmf-dist` root so the producer can derive the
root and the license. They were copied byte-for-byte from MacTeX 2026
(`/usr/local/texlive/2026/texmf-dist`) only after their SHA-256 matched the
pinned manifest in `crates/rendering-core/docs/handoffs/native-assets/manifest.json`
(ec-lmr10 cd13479f…, ec-lmr12 29902112…, rm-lmr12 9d4e3d8e…, rm-lmr6 eb0bfdf8…,
rm-lmr8 80bcbfd8…, license 49ea6cb9…); `scripts/bundle-texmf.py check Fonts/texmf`
re-verifies them and `make-app.sh` refuses to package on any mismatch. Bold,
italic and the other design sizes have no metrics here yet.

`texmf/SUPPLEMENTARY-METRICS.json` pins 23 further text TFMs in the same
directory (`ec-lmr{5,6,7,8,9,17}`, `ec-lmbx{5,6,7,8,9,10,12}`,
`ec-lmri{7,8,9,10,12}`, `ec-lmbxi10`, `rm-lmr{5,7,9,10}`) for the other design
sizes and the bold/italic faces. They are not in the Commander's manifest: copied
from the same MacTeX 2026 tree (TeX Live `lm` rev 77682, catalogue 2.005,
MANIFEST 2.004) and byte-identical to the CTAN `lm.zip` copy on the build
machine, but not verified against the pinned 2.004 archive hash; see the
`provenance` block in that file. `make-app.sh` refuses packaging if any of them
drifts from the pinned hash.

## EC (`jknappen/ec`) and AMS symbol metrics (T1 `cmr` and `\mathbb`)

`texmf/SUPPLEMENTARY-METRICS.json` also pins two further metric sets, both
copied byte-for-byte from MacTeX 2026 (`/usr/local/texlive/2026/texmf-dist`;
see the `ec_provenance` / `ams_symbols_provenance` blocks in that file):

* **`texmf/fonts/tfm/jknappen/ec/{ecrm,ecbx,ecti,ecbi,ecsl}{0500,...,3583}.tfm`**
  (70 files: the 5 shapes `t1cmr.fd` selects — medium/bold roman, medium/bold
  italic, slanted — at the 14 sizes it declares) plus
  `texmf/doc/fonts/ec/copyrite.txt` (the ec-fonts copyright notice; free
  redistribution of the unchanged files). PR #111 (GH111): the render
  pipeline (`crates/render-pipeline/src/fonts.rs`, `EC_TFM_DIR`/`ec_tfm_file`)
  lays a `[T1]{fontenc}` document out with these metrics whenever `lmodern`
  is not also loaded, instead of falling back to the Latin Modern OTF metrics
  and warning `ec_metrics_unavailable`. `fonts.rs`'s own
  `Discovery::bundle_texmf_roots` derives `Contents/Resources/texmf` from the
  running executable and probes this directory automatically inside a real
  `.app` bundle; `BundledMetrics.ecTfmDirectory`/`producerEnvironment` also
  advertise it explicitly in `FLASHTEX_TFM_DIRS` so a bare (non-bundled)
  producer binary gets the same metrics.
* **`texmf/fonts/tfm/public/amsfonts/symbols/{msbm,msam}{5,7,10}.tfm`**
  (6 files) plus `texmf/doc/fonts/amsfonts/README` (LPPL 1.3c or later,
  American Mathematical Society). pdfLaTeX draws `\mathbb` glyph advances
  from `msbm10.tfm`, not from an OpenType face's metrics (measured
  ‑0.91 pt/glyph on the double-struck ℝ against pdflatex); these are bundled
  so a producer can read the real TFM advances for `\mathbb`/`\mathfrak`-style
  math alphabets. **Not yet consumed**: as of this change no producer reads
  these files (`BundledMetrics.amsSymbolsDirectory` and the `FLASHTEX_TFM_DIRS`
  entry it adds exist, ready for that follow-up); today's double-struck
  glyphs still come from `NewCMMath-Regular.otf`, described above, whose
  advances already track msbm's design.

Both sets are supplementary (not in the Commander's pinned manifest):
`bundle-texmf.py check`/`stage` verify them against the in-repo pin like the
Latin Modern supplementary metrics, and `make-app.sh` refuses packaging on any
drift. `BundledMetricsTests` checks the pin against the vendored bytes and
that every vendored TFM under all three metric subdirectories (Latin Modern,
EC, AMS symbols) is accounted for by exactly one tier.

All **58 producer-requestable faces** are now vendored — every file
`latin_modern_outline` can return: eight Roman regular masters
(5/6/7/8/9/10/12/17), seven bold (5/6/7/8/9/10/12), five italic
(7/8/9/10/12), Roman10 bold-italic, Latin Modern Math, New Computer Modern
Math, the 10 typewriter designs (typewriter section below), the 11 non-upright
roman designs and the 14 sans designs (sans/slanted/caps section below).
`SUPPLEMENTARY-FACES.json` pins the 55 faces outside the
Commander's three-OTF manifest. Each of the original roman/math entries is
byte-identical to its member in the local CTAN `lm.zip`
(SHA-256 `71c48809cb50fbfe09c8eddaa251398957c7b243acdf69f7f807268f0d42c939`,
LM 2.004) and the installed MacTeX copy; overlapping Commander-pinned face/license
hashes agree. This archive is distinct from the Commander's pinned baseline ZIP;
its hash equivalence is not claimed. See the sidecar's provenance and
`docs/evidence/opus-fonts-takeover-20260912T1800Z/archive-verification.json`.

## Typewriter: `\texttt`, `\ttfamily`, `verbatim` (added by GH2 / `bundle-typewriter-fonts`)

Until this change the tree carried **no typewriter design at all**: no
`ectt`/`ecst`/`ecit`/`ectc` under `jknappen/ec`, no `ec-lmtt*` under
`public/lm`, and no `lmmono*.otf`. The font tables in
`crates/render-pipeline/src/fonts.rs` ask for those files
(`ec_tfm_file` → `ectt`/`ecst`/`ecit`/`ectc`, `latin_modern_outline` →
`lmmono*`, `latin_modern_tfm` → `ec-lmtt*`), so every `\texttt` run against
this bundle fell back: `ec_metrics_unavailable` (roman `ec-lmr*` widths used
for typewriter text, which changes the line breaks) plus
`font_outline_substituted` (roman outlines drawn). The fallback is reported,
but it is easy to miss, and two lanes measured `\texttt` geometry on it
before anyone noticed. On the real-world corpus it was **six diagnostics**
(`article-twocolumn` 4, `input-bibliography` 2); after this change it is
**zero**, with no verdict, status or page count changed.

**64 files added**, all from the same two upstream packages already vendored
here, so no new license applies:

* `texmf/fonts/tfm/jknappen/ec/{ectt,ecst,ecit,ectc}{0800…3583}.tfm` — 44
  files, the 4 shapes `t1cmtt.fd` declares (`m/n` `ectt`, `m/sl` `ecst`,
  `m/it` `ecit`, `m/sc` `ectc`) at its 11 distinct sizes. `t1cmtt.fd` writes
  `<5><6><7><8>#50800`, so 5/6/7 pt load the 8 pt file, and `bx/n`/`bx/it`
  are `ssub*cmtt/m/n`/`m/it` — there is no bold EC typewriter file to add.
  Covered by the `doc/fonts/ec/copyrite.txt` already in this tree.
* `texmf/fonts/tfm/public/lm/ec-lm{tt8,tt9,tt10,tt12,tti10,tto10,tcsc10,tcso10,tk10,tko10}.tfm`
  — 10 files, the metrics `t1lmtt.fd` loads. Covered by the GUST Font
  License already in this tree.
* `lmmono{8,9,10,12}-regular.otf`, `lmmono10-italic.otf`,
  `lmmonoslant10-regular.otf`, `lmmonocaps10-{regular,oblique}.otf`,
  `lmmonolt10-{bold,boldoblique}.otf` — 10 outlines, 669 KB, the files
  `latin_modern_outline` returns for `FamilyKind::Tt`. GUST Font License,
  the same Latin Modern release as the 21 `lmroman*` faces above.

Both in-repo pins were extended (`SUPPLEMENTARY-METRICS.json` 101 → 155
entries with a `tt_provenance` block, `SUPPLEMENTARY-FACES.json` 20 → 30 with
a `mono_provenance` block), so `bundle-texmf.py check` and `make-app.sh`
verify and refuse drift exactly as they do for the roman files.

### Provenance and why it is the same release

The earlier tiers were copied from MacTeX 2026 on `mac-m1max-a`. These files
were copied on `linux-primary`, which has **TeX Live 2025 r78234** and no
MacTeX, so release identity was established by byte-comparison rather than
asserted: every file already vendored from these two packages is
byte-identical (`cmp`) between the two distributions — all **70** vendored
`jknappen/ec` TFMs, all **28** vendored `public/lm` TFMs, all **21** vendored
Latin Modern OTFs and all **6** `amsfonts/symbols` TFMs. The typewriter
members were then copied out of the same two directories in the same run and
hashed before and after the copy. No CTAN archive is present on this host, so
no archive hash is claimed; the sidecar `limitation` fields say so.

### What was still not vendored (closed by the sans/slanted/caps tier below)

Sans (`ecss`/`ecsi`/`ecsx`/`ecso`, `lmsans*`, `lmsansdemicond*`) and the
roman caps/slanted/demi/unslanted designs (`eccc`/`ecui`/`ecsc`/`ecrb`/
`ecxc`/`ecoc`, `lmromanslant*`, `lmromancaps*`, `lmromandemi*`,
`lmromanunsl*`) were still absent, so `\textsf`/`\textsc`/`\textsl` on this
bundle substituted and warned. That is the same silent-substitution shape the
typewriter change removed; it was a known gap, not a decision. The next
section closes it.

`crates/render-pipeline/tests/bundled_typewriter.rs` is the guard: it walks
every `\ttfamily` shape through `nfss::select`/`nfss::terminal` in all four
schemes (OT1/T1 × cm/lm), asserts every file the tables can then name is on
disk and pinned, and renders four `\texttt` documents asserting **zero**
`font_unavailable` / `required_metrics_unavailable` / `ec_metrics_unavailable`
/ `font_outline_substituted` diagnostics. It reads no `FLASHTEX_*` variable.

## Sans, slanted and small caps: `\textsf`, `\textsl`, `\textsc`, and every running head

Until this change the tree carried **no Latin Modern sans, slanted, small-caps,
demi or unslanted design** — 33 outlines, all of them `lmroman*`, `lmmono*` or
math. `article.cls` and `book.cls` set every `headings`/`myheadings` running
head in **`\slshape`** (`article.cls:134-135,152,163-164`), so this was not a
corner case: **every running head in the bundle** was drawn from a substituted
roman outline with a `font_outline_substituted` warning, and `\textsf`,
`\textsl` and `\textsc` substituted roman throughout the body.

Three oracle tests measured it against pdfLaTeX, and all three now pass:

| test | before | after |
|---|---|---|
| `page_frame_against_pdflatex` | 16 fixtures differ, all in "chrome" (running heads); body text already matched 82/82 | **pass** |
| `footnotes_against_pdflatex` | `21-book-chapter` running head off 0.512 bp | **pass** |
| `font_family_fixtures_match_pdflatex` | 32 of 50 fixtures differ | **pass** |

### What was added

* 25 outlines, the files `latin_modern_outline` returns for `FamilyKind::Sf`
  and for the roman shapes outside `m/n`, `bx/n`, `m/it`, `bx/it`:
  `lmromanslant{8,9,10,12,17}-regular.otf`, `lmromanslant10-bold.otf`,
  `lmromancaps10-{regular,oblique}.otf`, `lmromanunsl10-regular.otf`,
  `lmromandemi10-{regular,oblique}.otf`, `lmsans{8,9,10,12,17}-regular.otf`,
  `lmsans{8,9,10,12,17}-oblique.otf`, `lmsans10-{bold,boldoblique}.otf`,
  `lmsansdemicond10-{regular,oblique}.otf`. GUST Font License, the same Latin
  Modern release as the 33 faces already here.
* 25 `ec-lm*` TFMs under `public/lm` — exactly what `latin_modern_tfm` maps
  those outlines to (`ec-lmro` 8/9/10/12/17, `ec-lmbxo10`, `ec-lmcsc10`,
  `ec-lmcsco10`, `ec-lmu10`, `ec-lmb10`, `ec-lmbo10`, `ec-lmss` and
  `ec-lmsso` 8/9/10/12/17, `ec-lmssbx10`, `ec-lmssbo10`, `ec-lmssdc10`,
  `ec-lmssdo10`), so each new face lays out on its own TeX metrics.
* 142 `jknappen/ec` TFMs — every remaining file `ec_tfm_file` can name for a
  `[T1]{fontenc}` document without `lmodern`: `eccc`, `ecsc`, `ecoc`, `ecui`,
  `ecbl`, `ecrb`, `ecxc` at the 14 `t1cmr.fd` sizes, and `ecss`, `ecsi`,
  `ecsx`, `ecso` at the 11 distinct sizes `t1cmss.fd` reaches. Without
  `ecxc1000.tfm` the `20-t1-rm-bfsc` fixture measured **11.533 bp** from
  pdfLaTeX, which sets it from `SFXC1000` (= `ecxc1000.tfm`). After this tier
  `ec_tfm_file` can no longer name a metric the bundle lacks. Covered by the
  `ec` copyright notice already vendored at `texmf/doc/fonts/ec/copyrite.txt`.

No new package is involved — `lm` and `jknappen` were both already vendored —
so no new licence file was added.

Both in-repo pins were extended (`SUPPLEMENTARY-METRICS.json` 155 → 322
entries with `sans_slant_caps_provenance` and `ec_shapes_provenance` blocks,
`SUPPLEMENTARY-FACES.json` 30 → 55 with a `sans_slant_caps_provenance`
block), so `bundle-texmf.py check` and `make-app.sh` verify and refuse drift
exactly as they do for every earlier tier: `check` reports **386 entries, all
`verified`** (9 `pinned`, 55 `supplementary-face`, 322 `supplementary`).
`MAX_SUPPLEMENTARY` in `bundle-texmf.py` was raised 65536 → 262144: it is a
sanity bound on parsing an in-repo sidecar, not a trust boundary, and every
entry inside is still checked individually while unpinned files are still
refused.

### Provenance and why it is the same release

Same method as the typewriter tier, and the same host: `linux-primary` has
**TeX Live 2025 r78234** and no MacTeX, so release identity was established by
byte-comparison rather than asserted. Every file already vendored from the
packages involved is byte-identical (`cmp`) between the two distributions —
all **33** Latin Modern OTFs, all **38** `public/lm` TFMs, all **114**
`jknappen/ec` TFMs and all **6** `amsfonts/symbols` TFMs: **191 files
compared, 0 differences**. The new members were copied out of the same
directories in the same run and hashed after the copy. No CTAN archive is
present on this host, so no archive hash is claimed.


## Secondary math face: New Computer Modern Math (`NewCMMath-Regular.otf`)

pdfLaTeX draws `\mathbb` from AMS `msbm10`, a serifed double-struck design;
Latin Modern Math's double-struck block (U+2102 ℂ … U+2124 ℤ, U+1D538–U+1D56B)
is the sans-like "open face" design, which is why `\mathbb{Z}` looked wrong next
to Overleaf. New Computer Modern Math (Antonis Tsolomitis) reproduces the msbm
design and its advances track msbm's, so `flashtex-render` draws every
double-struck code point from this face as a secondary math face
(`crates/render-pipeline/src/mathfont.rs`, `BB_FONT`) whenever
`NewCMMath-Regular.otf` is in a font directory; without it Latin Modern Math
draws `\mathbb` and one `math_resource_profile` note (`msbm10: …`) says so.
Everything else in math stays Latin Modern Math.

* File: `NewCMMath-Regular.otf`, 1,187,476 bytes, sha256
  `60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2`,
  PostScript name `NewCMMath-Regular`, name-table version `4.0`
  (`head.fontRevision` 3.00), package `newcomputermodern` 7.1.1.
* Source: copied byte-for-byte from MacTeX 2026
  `/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/newcomputermodern/NewCMMath-Regular.otf`
  (TeX Live file dated 2026-01-07); the hash above was checked before and after
  the copy. Nothing downloaded.
* License: GUST Font License v1.0 or later — the font's own name table (nameID 0)
  says "This work is released under the GUST Font License", and the package
  README (`texmf-dist/doc/fonts/newcomputermodern/README`) says "GustFLv1 or
  later". It is therefore covered by the `GUST-FONT-LICENSE.TXT` already in this
  directory (sha256 `49ea6cb9…`), the same licence as Latin Modern. (An earlier
  brief described it as GPL-3.0 with the font exception; that is not what this
  release declares.)
* Pinned in `SUPPLEMENTARY-FACES.json` (tier `supplementary-face`, 20 entries
  now); `bundle-texmf.py check`/`make-app.sh` verify it like every other face and
  stage it into `Contents/Resources/Fonts`, where the producer's
  `<exe>/../Resources/Fonts` discovery finds it. `V2FontStore` loads it by raw
  bytes (GH31) like the rest; `PreviewFonts.latinModernFaceFiles` lists it so a
  missing copy is reported, never silently substituted.

`bundle-texmf.py check Fonts/texmf Fonts` checks every face. `make-app.sh` stages
faces through verified copies and refuses missing, altered, symlinked or unpinned
OTFs before building/signing. `FLASHTEX_BUNDLE_FONTS_DIR` selects an explicit
verified source directory; no download or host TeX lookup occurs during packaging.
Preview Roman master selection follows the pinned producer's style-dependent
boundaries, including Roman6 and Roman10 as the sole bold-italic master. The
registered source directory's missing-face list remains visible to consumers.

Acceptance evidence and app-only export reproduction commands are in
[the temporary continuation report](../../../docs/evidence/opus-fonts-takeover-20260912T1800Z/README.md).
`apps/mac/scripts/faces-acceptance.py` drives the actual app producer and exact
exporter with host TeX and repository font reads denied. Use `--require-optical`
with an optical-capable producer to require Roman8/Roman6 and refusal after
Roman8 removal. A producer lacking these emitted faces is not optical coverage.
