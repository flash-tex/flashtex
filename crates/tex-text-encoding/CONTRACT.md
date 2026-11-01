# flashtex-tex-text-encoding — contract and adoption proposal (KC-105)

Owner: `kabir-claude` (mac-m5pro-kabir). Owned paths: `crates/tex-text-encoding/**` only.
No other crate is edited; everything below about `crates/compiler`,
`crates/paragraph-layout`, `crates/render-pipeline` and the IDE is a **proposal**.

## What this crate models

How pdfLaTeX turns *text* input into glyphs, for the default article class at 10pt:

| Area | Module | Source of truth |
|---|---|---|
| UTF-8 input → LaTeX commands | `unicode` | `omsenc/ot1enc/t1enc/ts1enc.dfu` (loaded by the pdflatex format in this order — verified in `texmf-var/web2c/pdftex/pdflatex.log`) + `utf8.def:360–370`; undeclared → `Unicode character X (U+XXXX) not set up for use with LaTeX.` |
| Text commands per encoding | `encoding` | `\DeclareTextSymbol/Accent/Command/Composite(Command)` from `ot1enc/t1enc/ts1enc/omsenc/omlenc.def`; kernel defaults + `\UndeclareTextCommand` from `latex.ltx` (file order); `Command \x unavailable in encoding E.` |
| Font selection, glyph names | `fonts` | `ot1cmr/t1cmr/ts1cmr/oms*/oml*.fd`, `lmodern.sty`, `*lmr.fd`; `pdftex.map` + `lm-rm/lm-ec/lm-ts1/lm-mathsy/lm-mathit.enc`, `cm-super-t1/ts1.enc`, builtin encodings of `cmr10/cmsy10/cmmi10.pfb` |
| TFM metrics | `tfm` | tex.web §560–575 (exact `store_scaled`; slant unscaled) |
| Ligatures/kerns | `ligkern` | tex.web §1034–1040 main loop, incl. boundary chars and `=:|>` ops |
| `\accent` | `accent` | tex.web §1123–1125: `delta = round((w−a)/2 + h·t − x·s)`, box shift `x − h` |
| Space factor / interword glue | `sfcode` | IniTeX §232, latex.ltx:552–555, 648, 22095–22106, 9422; tex.web §1041–1044 |
| One-line typesetting | `typeset`, `layout` | `\add@accent`, `\UseTextSymbol`, `\UseTextAccent`, `\@text@composite`, `\CheckEncodingSubset` (latex.ltx:10430), and the box constructs `\b \c \d \k`, OT1 `\L \l \ij \r A`, `\textellipsis`, `\textcommabelow` |

Tables live in `src/generated.rs`, produced by `tools/extract_tables.py` from the
installed TeX Live 2026 files (versions are recorded in the generated doc comments).

### Key behaviours (all verified against pdflatex)

* **OT1 vs T1.** OT1 builds every accent with `\accent` (kern, accent glyph — boxed and
  raised when the base height ≠ x-height — kern, base glyph). T1 uses a precomposed slot
  when a composite exists (`\'e` → 233) and `\accent` otherwise (`\v h`, `\~e`, `\=g`).
  A precomposed T1 glyph is an ordinary character: it ligatures/kerns with neighbours
  (`B\^Ac` shows a font kern Â–c), while `\add@accent`'s group cuts kerning in OT1.
* **Kernel defaults moved to TS1** (latex.ltx:14408ff.): in OT1 `\pounds` is TS1 163 (the old
  `cmu10` dollar hack is `\UndeclareTextCommand`-ed), `\S`/`\P`/`\dag`/`\ddag`/`\copyright`/
  `\textbullet`/`\texteuro`/`\textdegree`/`\textregistered`/`\texttrademark` are TS1 glyphs
  (`tcrm1000` or `ts1-lmr10`); `\textbackslash`, `\textbar`, `\textbraceleft` stay OMS
  (`cmsy10`/`lmsy10`), `\textless` OML. T1 has its own `\textsection` (159), `\textsterling`
  (191), `\textbackslash` (92).
* **`\t`** uses the TS1 tie accent (slot 26) for both `cmr` (subset 0) and `lmr` (subset 1),
  with the base letters in the text font; `\textcommabelow` (used for U+0218–021B) uses a
  5pt comma (`cmr5`/`ecrm0500`/`rm-lmr5`/`ec-lmr5`) lowered 0.31ex.
* **Unavailable in OT1** (error, argument still typeset): `\k`, `\th`, `\dh`, `\NG`, `\ng`,
  `\guillemotleft/right`, `\guillemetleft/right`, `\guilsinglleft`, `\quotedblbase`,
  `\quotesinglbase`, `\textquotedbl`. U+2026 `…` is declared (utf8.def) → `\textellipsis`
  = `.\kern\fontdimen3\font` ×3; U+0416 `Ж` and emoji are undeclared errors.
* **Space factor depends on the encoding, not on Unicode case.** `\sfcode` 999 applies to
  `A–Z` and the T1 uppercase ranges `"80–"9C`, `"C0–"DF`. Hence `\AE. b` and `\L. b` get
  sentence spacing in OT1 (slot 29 has sfcode 1000; OT1 `\L` is a box, which resets the
  factor to 1000) but normal spacing in T1 (slots 198/138 have 999). `\'E. b`/`É. b` get
  normal spacing everywhere (`\add@accent` restores the base's space factor; T1 slot 201
  has 999). `\ ` always gives the normal space (§1041). `A\@. B` gives sentence spacing.

## Oracle evidence (verified)

* Engine: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026). Generator:
  `python3 tools/generate_oracle.py` (typesets each fixture as `\setbox0\hbox{…}`; parses
  `\showbox` with `max_print_line=100000` and the uncompressed PDF page; origin from
  `\pdfsavepos`). `pdftotext`/`pdffonts`/`mutool`/`qpdf` are not installed on this host, so
  the PDF content stream and `/Differences`/`/Widths` are parsed directly instead.
* 177 one-line fixtures (38 accents, 57 symbols, 57 UTF-8 inputs, 25 space-factor lines)
  × 5 preambles (`ot1-cm`, `t1-cm`, `ot1-lm`, `t1-lm`, `t1-lm` + `[utf8]{inputenc}` +
  `textcomp`) = **885 cases; `cargo test` replays 885/885**:
  identical error messages; identical top-level node lists (character TFM + code, font kerns,
  `\accent` kerns and interword glue width/stretch/shrink **exact to the scaled point**);
  identical box width/height/depth (sp); identical glyph sequence (PDF base font, code,
  glyph name); glyph positions within **0.0049pt** (bound set by pdfTeX's 3-decimal bp
  output; tolerance 0.1pt). The inputenc/textcomp variant equals `t1-lm` in every compared field (errors, nodes,
  box, glyphs).
* Every glyph of a Latin Modern *roman* TFM in the oracle maps through `fonts::bundled_glyph`
  to a glyph present in `apps/mac/Fonts/lmroman{10,5}-regular.otf`
  (`tests/oracle/otf-glyph-names.json`, from `tools/otf_glyph_names.py`, fontTools).
* TFMs used by the tests are copies from TeX Live with SHA-256 in
  `tests/fixtures/tfm/MANIFEST.json`. Cargo tests never run TeX.

## Overlap with other branches (checked 2026-09-13 on origin)

* `origin/agent/daniel-tex-ligatures/compiler` (480308da) converts ``` `` '' -- --- !` ?` ```
  to Unicode in `crates/compiler/src/lexer.rs`. **Excluded here as a compiler feature.** This
  crate does not add a Unicode-level ligature pass; it runs the fonts' own TFM lig/kern
  programs, which is what selects OT1 92/34 vs T1 16/17/18 (`,,`) and the kerns. Adoption
  must use exactly one of the two layers per run, never both.
* `crates/paragraph-layout/src/items.rs` (`update_space_factor`) and
  `crates/render-pipeline/src/adapter.rs` (`space_factor`) already implement the plain
  `\sfcode` rules (merged via `daniel-parent-b/inline-spacing`). Not re-owned; differences
  found by this oracle are listed below for their owners.
* Math accents (`daniel-math-accents`, `daniel-parent/accent-skew`) and math symbol fonts are
  out of scope.

## Adoption proposal (for the owners of the named crates)

1. **Document setup (compiler preamble parsing).** Map `\usepackage[T1]{fontenc}` →
   `Encoding::T1` (last option wins; default `OT1`), `\usepackage{lmodern}` →
   `Family::LatinModern`, and build `typeset::Setup::article10`. `inputenc` with `utf8`
   and `textcomp` are no-ops (verified).
2. **Text runs.** Replace ad-hoc accent handling (`render-pipeline::adapter::accent`, a
   precomposed-Unicode table covering 7 accents; compiler base-14 accent composition) with
   either `typeset::typeset_hbox` per inline text run, or the pieces:
   `unicode::classify` → `encoding::resolve`/`composite` → `accent::make_accent` →
   `ligkern::lig_kern_run` on TFM codes. Keep source spans by running per word; the node
   list is ordered like TeX's and `layout::flatten` gives glyph positions.
3. **Space factor.** Compute it on TFM character codes with `sfcode::SfCodes` and
   `sfcode::interword_glue` (exact `xn_over_d`). Differences from the current code:
   `"` is not sfcode 0 in LaTeX (only `)` `'` `]`); Unicode uppercase outside the encoding's
   999 ranges (OT1 `Æ Œ Ø Ł`) must keep 1000; T1 accented capitals must be 999; `\@`,
   `\frenchspacing` and `\ ` (always normal space) need handling; boxes reset to 1000;
   `\add@accent` restores the base's factor.
4. **Font selection.** `fonts::text_tfm(family, encoding, size)` gives the NFSS TFM
   (`cmr10`, `ecrm1000`, `tcrm1000`, `cmsy10`, `cmmi10`, `rm-lmr10`, `ec-lmr10`,
   `ts1-lmr10`, `lmsy10`, `lmmi10`, 5pt variants). Symbol-font fallbacks (OMS/OML) are
   real in pdflatex output and must not be replaced by the text font.
5. **IDE rendering.** `fonts::bundled_glyph(tfm, code)` names the OTF file and CFF glyph
   (`.enc` name with LM OTF aliases: `ff→f_f`, `fi→f_i`, `IJ→I_J`, `Germandbls/SS→S_S`,
   `arrowleft→uni2190`, …). `same_design == false` flags Computer Modern / CM-Super
   documents drawn with the LM master (shape approximation). Not bundled: `lmsy*`/`lmmi*`
   (OMS/OML text symbols such as OT1 `\textbackslash`, `\textless`) and TS1 glyphs whose
   names have no OTF counterpart (`twelveudash`, `born`, `tildelow`, …).

## Known limits (believed, not oracle-tested)

* Only medium/upright text at the article-10pt sizes; no bold/italic/sans/typewriter
  families, no `\fontsize` changes inside the line.
* Not modelled (reported in `Typeset::unsupported`): `\textunderscore`,
  `\textvisiblespace`, `\textcircled`, `\textcommaabove` (`\c g`), `\-`, math, the fallback
  branch of `\CheckEncodingSubset` (families other than cmr/lmr), encodings loaded by other
  `fontenc` options (T2A, LGR, …) and their `.dfu` files, non-UTF-8 `inputenc`.
* Paragraph (unrestricted horizontal) mode: `RunOptions::unrestricted_hmode` inserts TeX's
  empty discretionary after `\hyphenchar`, but all oracle fixtures are `\hbox`es.
* `\add@accent` typesets its argument twice in pdflatex (measuring box); an error inside an
  accent argument would be logged twice there but once here.

## Regenerating

```sh
cd crates/tex-text-encoding
python3 tools/extract_tables.py          # src/generated.rs (needs kpsewhich)
python3 tools/generate_oracle.py         # tests/oracle/expected.json + tests/fixtures/tfm (needs pdflatex)
<python-with-fontTools> tools/otf_glyph_names.py   # tests/oracle/otf-glyph-names.json
CARGO_BUILD_JOBS=2 cargo test --offline
```
