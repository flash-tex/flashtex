# Issue #710: Latin Modern versus Computer Modern glyph metrics

Status: measurement only. No layout or rendering code was changed.

This sweep compares the actual 10pt Latin Modern faces loaded by FlashTeX with the local pdfTeX Computer Modern TFM files. Every reported delta is `Latin Modern - Computer Modern`. Values are points at 10pt design size. Relative values use the Computer Modern advance as the denominator; a zero Computer Modern advance is reported as not applicable rather than divided or estimated.

## Result at a glance

- Compared glyph rows: **251**.
- Rows by family: **cmr10 42**, **cmmi10 94**, **cmsy10 82**, **cmex10 33**.
- Unmeasured or out-of-scope entries: **42**.
- Rule gate: **0.1 bp**; position gate: **0.5 bp**.
- “Approaches” means at least **80%** of a gate: 0.40 bp for position and 0.08 bp for rules.
- `\ell` is present: raw advance CM 4.16670 pt, LM 4.17000 pt, delta +0.00330 pt; italic correction CM 0.00000 pt, LM 0.09000 pt; effective math char-box delta **+0.09330 pt = +0.09365 bp**.

## Sources and method
Scope: CMR rows are the math family-0 inventory. OT1 text-only slots and ligature programs are not included because this engine's normal text path uses T1 Latin Modern metrics, not cmr10.

The OTF faces, advances, CFF bounds, and MATH italic corrections are loaded through the existing `render-pipeline` `FontSet`, `LoadedFace`, and `TexMathMetrics::otf_glyph` path. The CM metric numbers are parsed by the existing `render-pipeline::tfm::Tfm` wrapper, which delegates to the shared `font-resources` TFM reader. `fontmath.ltx` supplies TeX's family/slot declarations, including `\ell`; it is not used to obtain metric numbers.
Font directory supplied to `FontSet`: `apps/mac/Fonts`.

| side | source | SHA-256 | notes |
| --- | --- | --- | --- |
| Latin Modern CMR path | `apps/mac/Fonts/lmroman10-regular.otf` | `1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852` | `lmroman10-regular.otf`, used for cmr10 math-family rows |
| Latin Modern math | `apps/mac/Fonts/latinmodern-math.otf` | `6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f` | `latinmodern-math.otf`, used for cmmi10/cmsy10/cmex10 rows |
| Computer Modern | `/usr/local/texlive/2026/texmf-dist/fonts/tfm/public/cm/cmr10.tfm` | `87f2d8981927644cbecaf3d639e96e348ea4e7be49d8804468bd8ba9ff3f5244` | `cmr10.tfm, design size 10.00000 pt; TFM bytes are read through the shared parser |
| Computer Modern | `/usr/local/texlive/2026/texmf-dist/fonts/tfm/public/cm/cmmi10.tfm` | `e442c5487f84df70218ff37f775c87060856f5b6e04c011b6cadbbadfcf46645` | `cmmi10.tfm, design size 10.00000 pt; TFM bytes are read through the shared parser |
| Computer Modern | `/usr/local/texlive/2026/texmf-dist/fonts/tfm/public/cm/cmsy10.tfm` | `0ca13d421ac7133271aed7c935099ecf3d1d08ac9e15f81acb34a16564ab8a46` | `cmsy10.tfm, design size 10.00000 pt; TFM bytes are read through the shared parser |
| Computer Modern | `/usr/local/texlive/2026/texmf-dist/fonts/tfm/public/cm/cmex10.tfm` | `0890bccea1dd4d27f001ac30e86c63af35bc803e0557c35aafb1903c8d208e92` | `cmex10.tfm, design size 10.00000 pt; TFM bytes are read through the shared parser |
| TeX declarations | `/usr/local/texlive/2026/texmf-dist/tex/latex/base/fontmath.ltx` | n/a | `kpsewhich fontmath.ltx`; declarations only |

The LM height is the loaded face's existing CFF glyph bound above the baseline; depth is the magnitude of its bound below the baseline. LM italic correction comes from the MATH table. The roman face has no MATH table, so its LM italic correction is zero; the CM value is still read from `cmr10.tfm`. Empty assembly placeholders are listed as unmeasured rather than treating `.notdef` as a glyph.

## Largest divergences
Assembly-piece note: private cmex brace-piece rows are source-piece comparisons; Latin Modern Math's horizontal assemblies are not independent glyph metrics for those pieces.

Raw advance width is ranked first, followed by the effective math char-box width (advance plus italic correction), because that is the width used by clean character boxes such as fraction numerators and denominators. The component tables then rank height, depth, and italic correction too.

### Largest absolute advance differences

| rank | metric | family/slot | glyph | label | delta | denominator fraction |
| ---: | --- | --- | --- | --- | ---: | ---: |
| 1 | advance width | cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 pt (+15.58819 bp) | +3.45107 |
| 2 | advance width | cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 pt (+15.58819 bp) | +3.45107 |
| 3 | advance width | cmex10 0x7A | U+23DE `⏞` | private cmex brace piece | +5.51995 pt (+5.54065 bp) | +1.22664 |
| 4 | advance width | cmex10 0x7C | U+23DF `⏟` | private cmex brace piece | +5.51995 pt (+5.54065 bp) | +1.22664 |
| 5 | advance width | cmex10 0x7B | U+23DE `⏞` | private cmex brace piece | +5.50995 pt (+5.53062 bp) | +1.22442 |
| 6 | advance width | cmex10 0x7D | U+23DF `⏟` | private cmex brace piece | +5.50995 pt (+5.53062 bp) | +1.22442 |
| 7 | advance width | cmmi10 0x7E | U+20D7 `⃗` | \vec (fontmath.ltx) | -5.00002 pt (-5.01877 bp) | -1.00000 |
| 8 | advance width | cmex10 0x48 | U+222E `∮` | \oint | +1.92777 pt (+1.93500 bp) | +0.40823 |
| 9 | advance width | cmex10 0x52 | U+222B `∫` | \int | +1.92777 pt (+1.93500 bp) | +0.40823 |
| 10 | advance width | cmsy10 0x3D | U+2111 `ℑ` | \Im (fontmath.ltx) | -1.68224 pt (-1.68855 bp) | -0.23293 |
| 11 | advance width | cmex10 0x78 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | -1.66669 pt (-1.67294 bp) | -0.25000 |
| 12 | advance width | cmex10 0x79 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | -1.66669 pt (-1.67294 bp) | -0.25000 |
| 13 | advance width | cmsy10 0x30 | U+2032 `′` | \prime, \prime (fontmath.ltx) | +1.32000 pt (+1.32495 bp) | +0.48000 |
| 14 | advance width | cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | -1.31557 pt (-1.32051 bp) | -0.23680 |
| 15 | advance width | cmex10 0x0D | U+2225 `∥` | \parallel | -1.31557 pt (-1.32051 bp) | -0.23680 |
| 16 | advance width | cmsy10 0x38 | U+2200 `∀` | \forall, \forall (fontmath.ltx) | +1.10443 pt (+1.10857 bp) | +0.19880 |
| 17 | advance width | cmsy10 0x3C | U+211C `ℜ` | \Re (fontmath.ltx) | +1.05776 pt (+1.06172 bp) | +0.14646 |
| 18 | advance width | cmsy10 0x6B | U+2016 `‖` | \Vert, \lVert, \rVert | -1.02002 pt (-1.02384 bp) | -0.20400 |
| 19 | advance width | cmex10 0x65 | U+0303 COMBINING TILDE | \widetilde (fontmath.ltx) | +0.96443 pt (+0.96804 bp) | +0.17360 |
| 20 | advance width | cmex10 0x62 | U+0302 COMBINING CIRCUMFLEX | \widehat (fontmath.ltx) | +0.88443 pt (+0.88774 bp) | +0.15920 |

### Largest relative advance differences

| rank | metric | family/slot | glyph | label | delta | denominator fraction |
| ---: | --- | --- | --- | --- | ---: | ---: |
| 1 | advance width | cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 pt | +3.45107 |
| 2 | advance width | cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 pt | +3.45107 |
| 3 | advance width | cmex10 0x7A | U+23DE `⏞` | private cmex brace piece | +5.51995 pt | +1.22664 |
| 4 | advance width | cmex10 0x7C | U+23DF `⏟` | private cmex brace piece | +5.51995 pt | +1.22664 |
| 5 | advance width | cmex10 0x7B | U+23DE `⏞` | private cmex brace piece | +5.50995 pt | +1.22442 |
| 6 | advance width | cmex10 0x7D | U+23DF `⏟` | private cmex brace piece | +5.50995 pt | +1.22442 |
| 7 | advance width | cmmi10 0x7E | U+20D7 `⃗` | \vec (fontmath.ltx) | -5.00002 pt | -1.00000 |
| 8 | advance width | cmsy10 0x30 | U+2032 `′` | \prime, \prime (fontmath.ltx) | +1.32000 pt | +0.48000 |
| 9 | advance width | cmex10 0x48 | U+222E `∮` | \oint | +1.92777 pt | +0.40823 |
| 10 | advance width | cmex10 0x52 | U+222B `∫` | \int | +1.92777 pt | +0.40823 |
| 11 | advance width | cmex10 0x78 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | -1.66669 pt | -0.25000 |
| 12 | advance width | cmex10 0x79 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | -1.66669 pt | -0.25000 |
| 13 | advance width | cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | -1.31557 pt | -0.23680 |
| 14 | advance width | cmex10 0x0D | U+2225 `∥` | \parallel | -1.31557 pt | -0.23680 |
| 15 | advance width | cmsy10 0x3D | U+2111 `ℑ` | \Im (fontmath.ltx) | -1.68224 pt | -0.23293 |
| 16 | advance width | cmsy10 0x6B | U+2016 `‖` | \Vert, \lVert, \rVert | -1.02002 pt | -0.20400 |
| 17 | advance width | cmmi10 0x7C | U+0237 `ȷ` | \jmath (fontmath.ltx) | -0.78030 pt | -0.20319 |
| 18 | advance width | cmsy10 0x38 | U+2200 `∀` | \forall, \forall (fontmath.ltx) | +1.10443 pt | +0.19880 |
| 19 | advance width | cmsy10 0x0E | U+2218 `∘` | \circ, \circ (fontmath.ltx) | -0.88002 pt | -0.17600 |
| 20 | advance width | cmex10 0x65 | U+0303 COMBINING TILDE | \widetilde (fontmath.ltx) | +0.96443 pt | +0.17360 |

### Largest absolute effective char-box width differences

| rank | metric | family/slot | glyph | label | delta | denominator fraction |
| ---: | --- | --- | --- | --- | ---: | ---: |
| 1 | effective char-box width | cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 pt (+15.58819 bp) | +3.45107 |
| 2 | effective char-box width | cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 pt (+15.58819 bp) | +3.45107 |
| 3 | effective char-box width | cmmi10 0x7E | U+20D7 `⃗` | \vec (fontmath.ltx) | -6.53821 pt (-6.56273 bp) | -1.30764 |
| 4 | effective char-box width | cmex10 0x7A | U+23DE `⏞` | private cmex brace piece | +5.51995 pt (+5.54065 bp) | +1.22664 |
| 5 | effective char-box width | cmex10 0x7C | U+23DF `⏟` | private cmex brace piece | +5.51995 pt (+5.54065 bp) | +1.22664 |
| 6 | effective char-box width | cmex10 0x7B | U+23DE `⏞` | private cmex brace piece | +5.50995 pt (+5.53062 bp) | +1.22442 |
| 7 | effective char-box width | cmex10 0x7D | U+23DF `⏟` | private cmex brace piece | +5.50995 pt (+5.53062 bp) | +1.22442 |
| 8 | effective char-box width | cmex10 0x48 | U+222E `∮` | \oint | +3.30331 pt (+3.31570 bp) | +0.69952 |
| 9 | effective char-box width | cmex10 0x52 | U+222B `∫` | \int | +3.30331 pt (+3.31570 bp) | +0.69952 |
| 10 | effective char-box width | cmex10 0x78 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | -1.66669 pt (-1.67294 bp) | -0.25000 |
| 11 | effective char-box width | cmex10 0x79 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | -1.66669 pt (-1.67294 bp) | -0.25000 |
| 12 | effective char-box width | cmsy10 0x3D | U+2111 `ℑ` | \Im (fontmath.ltx) | -1.61224 pt (-1.61829 bp) | -0.22323 |
| 13 | effective char-box width | cmsy10 0x30 | U+2032 `′` | \prime, \prime (fontmath.ltx) | +1.32000 pt (+1.32495 bp) | +0.48000 |
| 14 | effective char-box width | cmsy10 0x3C | U+211C `ℜ` | \Re (fontmath.ltx) | +1.31776 pt (+1.32270 bp) | +0.18246 |
| 15 | effective char-box width | cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | -1.31557 pt (-1.32051 bp) | -0.23680 |
| 16 | effective char-box width | cmex10 0x0D | U+2225 `∥` | \parallel | -1.31557 pt (-1.32051 bp) | -0.23680 |
| 17 | effective char-box width | cmsy10 0x38 | U+2200 `∀` | \forall, \forall (fontmath.ltx) | +1.10443 pt (+1.10857 bp) | +0.19880 |
| 18 | effective char-box width | cmsy10 0x6B | U+2016 `‖` | \Vert, \lVert, \rVert | -1.02002 pt (-1.02384 bp) | -0.20400 |
| 19 | effective char-box width | cmex10 0x65 | U+0303 COMBINING TILDE | \widetilde (fontmath.ltx) | +0.96443 pt (+0.96804 bp) | +0.17360 |
| 20 | effective char-box width | cmex10 0x62 | U+0302 COMBINING CIRCUMFLEX | \widehat (fontmath.ltx) | +0.88443 pt (+0.88774 bp) | +0.15920 |

### Largest relative effective char-box width differences

| rank | metric | family/slot | glyph | label | delta | denominator fraction |
| ---: | --- | --- | --- | --- | ---: | ---: |
| 1 | effective char-box width | cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 pt | +3.45107 |
| 2 | effective char-box width | cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 pt | +3.45107 |
| 3 | effective char-box width | cmmi10 0x7E | U+20D7 `⃗` | \vec (fontmath.ltx) | -6.53821 pt | -1.30764 |
| 4 | effective char-box width | cmex10 0x7A | U+23DE `⏞` | private cmex brace piece | +5.51995 pt | +1.22664 |
| 5 | effective char-box width | cmex10 0x7C | U+23DF `⏟` | private cmex brace piece | +5.51995 pt | +1.22664 |
| 6 | effective char-box width | cmex10 0x7B | U+23DE `⏞` | private cmex brace piece | +5.50995 pt | +1.22442 |
| 7 | effective char-box width | cmex10 0x7D | U+23DF `⏟` | private cmex brace piece | +5.50995 pt | +1.22442 |
| 8 | effective char-box width | cmex10 0x48 | U+222E `∮` | \oint | +3.30331 pt | +0.69952 |
| 9 | effective char-box width | cmex10 0x52 | U+222B `∫` | \int | +3.30331 pt | +0.69952 |
| 10 | effective char-box width | cmsy10 0x30 | U+2032 `′` | \prime, \prime (fontmath.ltx) | +1.32000 pt | +0.48000 |
| 11 | effective char-box width | cmex10 0x78 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | -1.66669 pt | -0.25000 |
| 12 | effective char-box width | cmex10 0x79 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | -1.66669 pt | -0.25000 |
| 13 | effective char-box width | cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | -1.31557 pt | -0.23680 |
| 14 | effective char-box width | cmex10 0x0D | U+2225 `∥` | \parallel | -1.31557 pt | -0.23680 |
| 15 | effective char-box width | cmsy10 0x3D | U+2111 `ℑ` | \Im (fontmath.ltx) | -1.61224 pt | -0.22323 |
| 16 | effective char-box width | cmsy10 0x6B | U+2016 `‖` | \Vert, \lVert, \rVert | -1.02002 pt | -0.20400 |
| 17 | effective char-box width | cmmi10 0x7C | U+0237 `ȷ` | \jmath (fontmath.ltx) | -0.78030 pt | -0.20319 |
| 18 | effective char-box width | cmsy10 0x38 | U+2200 `∀` | \forall, \forall (fontmath.ltx) | +1.10443 pt | +0.19880 |
| 19 | effective char-box width | cmsy10 0x3C | U+211C `ℜ` | \Re (fontmath.ltx) | +1.31776 pt | +0.18246 |
| 20 | effective char-box width | cmsy10 0x0E | U+2218 `∘` | \circ, \circ (fontmath.ltx) | -0.88002 pt | -0.17600 |

### Largest absolute component differences

| rank | metric | family/slot | glyph | label | delta | denominator fraction |
| ---: | --- | --- | --- | --- | ---: | ---: |
| 1 | advance width | cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 pt (+15.58819 bp) | +3.45107 |
| 2 | advance width | cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 pt (+15.58819 bp) | +3.45107 |
| 3 | height | cmex10 0x0C | U+007C `|` | ASCII math, \vert | +12.02000 pt (+12.06507 bp) | +3.60599 |
| 4 | height | cmex10 0x0C | U+2223 `∣` | \lvert, \mid, \rvert | +12.02000 pt (+12.06507 bp) | +3.60599 |
| 5 | height | cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | +12.02000 pt (+12.06507 bp) | +2.16359 |
| 6 | height | cmex10 0x0D | U+2225 `∥` | \parallel | +12.02000 pt (+12.06507 bp) | +2.16359 |
| 7 | height | cmex10 0x0E | U+002F `/` | ASCII math | +8.65001 pt (+8.68245 bp) | +1.49711 |
| 8 | height | cmex10 0x0F | U+005C `\` | ASCII math, \backslash, \backslash (fontmath.ltx) | +8.65001 pt (+8.68245 bp) | +1.49711 |
| 9 | depth | cmex10 0x00 | U+0028 `(` | ASCII math | -8.13013 pt (-8.16062 bp) | -1.77384 |
| 10 | depth | cmex10 0x01 | U+0029 `)` | ASCII math | -8.13013 pt (-8.16062 bp) | -1.77384 |
| 11 | depth | cmex10 0x02 | U+005B `[` | ASCII math | -8.10013 pt (-8.13051 bp) | -1.94402 |
| 12 | depth | cmex10 0x03 | U+005D `]` | ASCII math | -8.10013 pt (-8.13051 bp) | -1.94402 |
| 13 | depth | cmex10 0x04 | U+230A `⌊` | \lfloor, \lfloor (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.71531 |
| 14 | depth | cmex10 0x05 | U+230B `⌋` | \rfloor, \rfloor (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.71531 |
| 15 | depth | cmex10 0x06 | U+2308 `⌈` | \lceil, \lceil (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.71531 |
| 16 | depth | cmex10 0x07 | U+2309 `⌉` | \rceil, \rceil (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.71531 |
| 17 | depth | cmex10 0x08 | U+007B `{` | \lbrace, \lbrace (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.38859 |
| 18 | depth | cmex10 0x09 | U+007D `}` | \rbrace, \rbrace (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.38859 |
| 19 | depth | cmex10 0x0A | U+27E8 `⟨` | \langle, \langle (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.71531 |
| 20 | depth | cmex10 0x0B | U+27E9 `⟩` | \rangle, \rangle (fontmath.ltx) | -8.10013 pt (-8.13051 bp) | -1.71531 |

### Largest relative component differences

| rank | metric | family/slot | glyph | label | delta | denominator fraction |
| ---: | --- | --- | --- | --- | ---: | ---: |
| 1 | height | cmex10 0x0C | U+007C `|` | ASCII math, \vert | +12.02000 pt | +3.60599 |
| 2 | height | cmex10 0x0C | U+2223 `∣` | \lvert, \mid, \rvert | +12.02000 pt | +3.60599 |
| 3 | advance width | cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 pt | +3.45107 |
| 4 | advance width | cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 pt | +3.45107 |
| 5 | height | cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | +12.02000 pt | +2.16359 |
| 6 | height | cmex10 0x0D | U+2225 `∥` | \parallel | +12.02000 pt | +2.16359 |
| 7 | depth | cmex10 0x02 | U+005B `[` | ASCII math | -8.10013 pt | -1.94402 |
| 8 | depth | cmex10 0x03 | U+005D `]` | ASCII math | -8.10013 pt | -1.94402 |
| 9 | height | cmex10 0x02 | U+005B `[` | ASCII math | +8.10001 pt | +1.94399 |
| 10 | height | cmex10 0x03 | U+005D `]` | ASCII math | +8.10001 pt | +1.94399 |
| 11 | depth | cmex10 0x0C | U+007C `|` | ASCII math, \vert | -6.00006 pt | -1.80001 |
| 12 | depth | cmex10 0x0C | U+2223 `∣` | \lvert, \mid, \rvert | -6.00006 pt | -1.80001 |
| 13 | depth | cmex10 0x00 | U+0028 `(` | ASCII math | -8.13013 pt | -1.77384 |
| 14 | depth | cmex10 0x01 | U+0029 `)` | ASCII math | -8.13013 pt | -1.77384 |
| 15 | height | cmex10 0x00 | U+0028 `(` | ASCII math | +8.07001 pt | +1.76072 |
| 16 | height | cmex10 0x01 | U+0029 `)` | ASCII math | +8.07001 pt | +1.76072 |
| 17 | depth | cmex10 0x04 | U+230A `⌊` | \lfloor, \lfloor (fontmath.ltx) | -8.10013 pt | -1.71531 |
| 18 | depth | cmex10 0x05 | U+230B `⌋` | \rfloor, \rfloor (fontmath.ltx) | -8.10013 pt | -1.71531 |
| 19 | depth | cmex10 0x06 | U+2308 `⌈` | \lceil, \lceil (fontmath.ltx) | -8.10013 pt | -1.71531 |
| 20 | depth | cmex10 0x07 | U+2309 `⌉` | \rceil, \rceil (fontmath.ltx) | -8.10013 pt | -1.71531 |

## Gate watch list
Private cmex brace-piece rows are source-piece evidence, not a claim that an LM assembly piece is painted as an independent glyph.

This list is based on the effective math char-box width (advance plus italic correction), because that is the width used for clean character boxes such as fraction numerators and denominators. Raw advance and italic correction remain separate in the complete table. `approaches` is the sweep's explicit 80% convention; it is not a project gate. A row can be below the position threshold while already exceeding the stricter rule threshold.

**What `EXCEEDS` means here.** The 0.1 bp / 0.5 bp thresholds are the same generic acceptance tolerances this codebase already applies when comparing rendered rule widths and box positions against pdfTeX output (see `crates/render-pipeline/tests/*_oracle.rs`); this sweep reuses them as a screening cutoff on each glyph's raw metric delta, in isolation. An `EXCEEDS` mark is not a claim that this glyph draws a mis-sized TeX `\rule`, and not a claim that any test currently fails — it means this glyph's own LM-versus-CM divergence, by itself, is larger than the margin those tests require. Whether that divergence ever becomes a visible or gate-breaking difference depends on whether, and how, this specific glyph's metric feeds a rendered rule or box width in some construct, the way `\ell`'s italic correction feeds the vinculum width in `\sqrt{\ell/\ell}` (issue #710). Most rows below are ordinary letters and symbols (`\forall`, `\Im`, `\prime`, the cmex10 arrow and brace-piece slots) that are never used to draw a rule; for those, `EXCEEDS` flags a candidate worth checking against a specific construct, not a known rendering regression. Of the 69 rows below, 62 exceed the rule threshold (7 more approach it) and 27 exceed the position threshold (2 more approach it); those counts describe this screening pass, not confirmed defects.

| family/slot | glyph | labels | raw Δw pt | raw Δw bp | effective Δ char-box pt | effective Δ bp | effective fraction of CM advance | position 0.5 bp | rule 0.1 bp |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | +15.52995 | +15.58819 | +15.52995 | +15.58819 | +3.45107 | EXCEEDS | EXCEEDS |
| cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | +15.52995 | +15.58819 | +15.52995 | +15.58819 | +3.45107 | EXCEEDS | EXCEEDS |
| cmmi10 0x7E | U+20D7 `⃗` | \vec (fontmath.ltx) | -5.00002 | -5.01877 | -6.53821 | -6.56273 | -1.30764 | EXCEEDS | EXCEEDS |
| cmex10 0x7A | U+23DE `⏞` | private cmex brace piece | +5.51995 | +5.54065 | +5.51995 | +5.54065 | +1.22664 | EXCEEDS | EXCEEDS |
| cmex10 0x7C | U+23DF `⏟` | private cmex brace piece | +5.51995 | +5.54065 | +5.51995 | +5.54065 | +1.22664 | EXCEEDS | EXCEEDS |
| cmex10 0x7B | U+23DE `⏞` | private cmex brace piece | +5.50995 | +5.53062 | +5.50995 | +5.53062 | +1.22442 | EXCEEDS | EXCEEDS |
| cmex10 0x7D | U+23DF `⏟` | private cmex brace piece | +5.50995 | +5.53062 | +5.50995 | +5.53062 | +1.22442 | EXCEEDS | EXCEEDS |
| cmex10 0x48 | U+222E `∮` | \oint | +1.92777 | +1.93500 | +3.30331 | +3.31570 | +0.69952 | EXCEEDS | EXCEEDS |
| cmex10 0x52 | U+222B `∫` | \int | +1.92777 | +1.93500 | +3.30331 | +3.31570 | +0.69952 | EXCEEDS | EXCEEDS |
| cmex10 0x78 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | -1.66669 | -1.67294 | -1.66669 | -1.67294 | -0.25000 | EXCEEDS | EXCEEDS |
| cmex10 0x79 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | -1.66669 | -1.67294 | -1.66669 | -1.67294 | -0.25000 | EXCEEDS | EXCEEDS |
| cmsy10 0x3D | U+2111 `ℑ` | \Im (fontmath.ltx) | -1.68224 | -1.68855 | -1.61224 | -1.61829 | -0.22323 | EXCEEDS | EXCEEDS |
| cmsy10 0x30 | U+2032 `′` | \prime, \prime (fontmath.ltx) | +1.32000 | +1.32495 | +1.32000 | +1.32495 | +0.48000 | EXCEEDS | EXCEEDS |
| cmsy10 0x3C | U+211C `ℜ` | \Re (fontmath.ltx) | +1.05776 | +1.06172 | +1.31776 | +1.32270 | +0.18246 | EXCEEDS | EXCEEDS |
| cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | -1.31557 | -1.32051 | -1.31557 | -1.32051 | -0.23680 | EXCEEDS | EXCEEDS |
| cmex10 0x0D | U+2225 `∥` | \parallel | -1.31557 | -1.32051 | -1.31557 | -1.32051 | -0.23680 | EXCEEDS | EXCEEDS |
| cmsy10 0x38 | U+2200 `∀` | \forall, \forall (fontmath.ltx) | +1.10443 | +1.10857 | +1.10443 | +1.10857 | +0.19880 | EXCEEDS | EXCEEDS |
| cmsy10 0x6B | U+2016 `‖` | \Vert, \lVert, \rVert | -1.02002 | -1.02384 | -1.02002 | -1.02384 | -0.20400 | EXCEEDS | EXCEEDS |
| cmex10 0x65 | U+0303 COMBINING TILDE | \widetilde (fontmath.ltx) | +0.96443 | +0.96804 | +0.96443 | +0.96804 | +0.17360 | EXCEEDS | EXCEEDS |
| cmex10 0x62 | U+0302 COMBINING CIRCUMFLEX | \widehat (fontmath.ltx) | +0.88443 | +0.88774 | +0.88443 | +0.88774 | +0.15920 | EXCEEDS | EXCEEDS |
| cmsy10 0x0E | U+2218 `∘` | \circ, \circ (fontmath.ltx) | -0.88002 | -0.88332 | -0.88002 | -0.88332 | -0.17600 | EXCEEDS | EXCEEDS |
| cmsy10 0x34 | U+25B3 `△` | \triangle (fontmath.ltx) | +0.79109 | +0.79405 | +0.79109 | +0.79405 | +0.08900 | EXCEEDS | EXCEEDS |
| cmsy10 0x35 | U+25BD `▽` | \bigtriangledown (fontmath.ltx) | +0.79109 | +0.79405 | +0.79109 | +0.79405 | +0.08900 | EXCEEDS | EXCEEDS |
| cmmi10 0x7C | U+0237 `ȷ` | \jmath (fontmath.ltx) | -0.78030 | -0.78323 | -0.78030 | -0.78323 | -0.20319 | EXCEEDS | EXCEEDS |
| cmsy10 0x6E | U+2216 `∖` | \setminus, \setminus (fontmath.ltx) | +0.67998 | +0.68253 | +0.67998 | +0.68253 | +0.13600 | EXCEEDS | EXCEEDS |
| cmex10 0x0C | U+007C `|` | ASCII math, \vert | -0.55334 | -0.55541 | -0.55334 | -0.55541 | -0.16600 | EXCEEDS | EXCEEDS |
| cmex10 0x0C | U+2223 `∣` | \lvert, \mid, \rvert | -0.55334 | -0.55541 | -0.55334 | -0.55541 | -0.16600 | EXCEEDS | EXCEEDS |
| cmmi10 0x7B | U+0131 `ı` | \imath (fontmath.ltx) | -0.44456 | -0.44623 | -0.44456 | -0.44623 | -0.13787 | approaches | EXCEEDS |
| cmmi10 0x6A | U+006A `j` | ASCII math | +0.00193 | +0.00194 | -0.44050 | -0.44216 | -0.10697 | approaches | EXCEEDS |
| cmex10 0x0E | U+002F `/` | ASCII math | +0.39221 | +0.39368 | +0.39221 | +0.39368 | +0.06788 | below | EXCEEDS |
| cmex10 0x0F | U+005C `\` | ASCII math, \backslash, \backslash (fontmath.ltx) | +0.39221 | +0.39368 | +0.39221 | +0.39368 | +0.06788 | below | EXCEEDS |
| cmmi10 0x58 | U+0058 `X` | ASCII math | -0.00474 | -0.00476 | -0.27945 | -0.28050 | -0.03373 | below | EXCEEDS |
| cmmi10 0x21 | U+03C9 `ω` | \omega, \omega (fontmath.ltx) | -0.00455 | -0.00456 | -0.26334 | -0.26433 | -0.04231 | below | EXCEEDS |
| cmmi10 0x63 | U+0063 `c` | ASCII math | +0.00244 | +0.00244 | +0.25244 | +0.25338 | +0.05833 | below | EXCEEDS |
| cmmi10 0x42 | U+0042 `B` | ASCII math | +0.00490 | +0.00492 | -0.24682 | -0.24775 | -0.03254 | below | EXCEEDS |
| cmmi10 0x76 | U+0076 `v` | ASCII math | +0.00276 | +0.00277 | -0.24603 | -0.24695 | -0.05076 | below | EXCEEDS |
| cmmi10 0x20 | U+03C8 `ψ` | \psi, \psi (fontmath.ltx) | -0.00392 | -0.00393 | -0.24271 | -0.24362 | -0.03726 | below | EXCEEDS |
| cmmi10 0x1D | U+03C5 `υ` | \upsilon, \upsilon (fontmath.ltx) | -0.00280 | -0.00281 | -0.24159 | -0.24250 | -0.04472 | below | EXCEEDS |
| cmmi10 0x77 | U+0077 `w` | ASCII math | +0.00082 | +0.00083 | -0.23826 | -0.23916 | -0.03328 | below | EXCEEDS |
| cmmi10 0x44 | U+0044 `D` | ASCII math | +0.00083 | +0.00083 | -0.23696 | -0.23785 | -0.02862 | below | EXCEEDS |
| cmmi10 0x64 | U+0064 `d` | ASCII math | -0.00488 | -0.00490 | +0.23512 | +0.23600 | +0.04517 | below | EXCEEDS |
| cmmi10 0x4F | U+004F `O` | ASCII math | +0.00224 | +0.00224 | -0.22555 | -0.22640 | -0.02957 | below | EXCEEDS |
| cmmi10 0x6C | U+006C `l` | ASCII math | -0.00380 | -0.00381 | -0.20058 | -0.20133 | -0.06722 | below | EXCEEDS |
| cmmi10 0x66 | U+0066 `f` | ASCII math | +0.00414 | +0.00416 | -0.17226 | -0.17291 | -0.03519 | below | EXCEEDS |
| cmmi10 0x0C | U+03B2 `β` | \beta, \beta (fontmath.ltx) | +0.00374 | +0.00375 | -0.16404 | -0.16466 | -0.02900 | below | EXCEEDS |
| cmmi10 0x6B | U+006B `k` | ASCII math | +0.00396 | +0.00397 | -0.16085 | -0.16145 | -0.03090 | below | EXCEEDS |
| cmmi10 0x52 | U+0052 `R` | ASCII math | -0.00290 | -0.00291 | +0.15985 | +0.16044 | +0.02105 | below | EXCEEDS |
| cmmi10 0x72 | U+0072 `r` | ASCII math | -0.00159 | -0.00160 | -0.14938 | -0.14994 | -0.03311 | below | EXCEEDS |
| cmmi10 0x70 | U+0070 `p` | ASCII math | -0.00126 | -0.00127 | +0.14874 | +0.14930 | +0.02956 | below | EXCEEDS |
| cmmi10 0x12 | U+03B8 `θ` | \theta, \theta (fontmath.ltx) | -0.00444 | -0.00446 | -0.14223 | -0.14276 | -0.03030 | below | EXCEEDS |
| cmmi10 0x7A | U+007A `z` | ASCII math | -0.00050 | -0.00050 | -0.14030 | -0.14083 | -0.03017 | below | EXCEEDS |
| cmmi10 0x62 | U+0062 `b` | ASCII math | -0.00167 | -0.00167 | +0.13833 | +0.13885 | +0.03223 | below | EXCEEDS |
| cmsy10 0x0D | U+25EF `◯` | \bigcirc, \bigcirc (fontmath.ltx) | +0.12997 | +0.13046 | +0.12997 | +0.13046 | +0.01300 | below | EXCEEDS |
| cmmi10 0x1A | U+03C1 `ρ` | \rho, \rho (fontmath.ltx) | -0.00015 | -0.00016 | +0.12985 | +0.13033 | +0.02511 | below | EXCEEDS |
| cmmi10 0x59 | U+0059 `Y` | ASCII math | +0.00443 | +0.00445 | -0.12780 | -0.12828 | -0.02201 | below | EXCEEDS |
| cmmi10 0x1B | U+03C3 `σ` | \sigma, \sigma (fontmath.ltx) | -0.00414 | -0.00416 | -0.12293 | -0.12339 | -0.02151 | below | EXCEEDS |
| cmmi10 0x6F | U+006F `o` | ASCII math | +0.00277 | +0.00278 | +0.12277 | +0.12323 | +0.02533 | below | EXCEEDS |
| cmmi10 0x1C | U+03C4 `τ` | \tau, \tau (fontmath.ltx) | -0.00155 | -0.00155 | -0.11350 | -0.11393 | -0.02596 | below | EXCEEDS |
| cmmi10 0x24 | U+03D6 `ϖ` | \varpi, \varpi (fontmath.ltx) | -0.00130 | -0.00130 | -0.10908 | -0.10949 | -0.01317 | below | EXCEEDS |
| cmmi10 0x19 | U+03C0 `π` | \pi, \pi (fontmath.ltx) | -0.00027 | -0.00027 | -0.10906 | -0.10947 | -0.01913 | below | EXCEEDS |
| cmmi10 0x67 | U+0067 `g` | ASCII math | +0.00031 | +0.00031 | -0.10848 | -0.10889 | -0.02274 | below | EXCEEDS |
| cmmi10 0x4A | U+004A `J` | ASCII math | +0.00486 | +0.00488 | +0.10305 | +0.10344 | +0.01858 | below | EXCEEDS |
| cmmi10 0x18 | U+03BE `ξ` | \xi, \xi (fontmath.ltx) | +0.00498 | +0.00500 | -0.09509 | -0.09545 | -0.02173 | below | approaches |
| cmmi10 0x60 | U+2113 `ℓ` | \ell (fontmath.ltx) | +0.00330 | +0.00331 | +0.09330 | +0.09365 | +0.02239 | below | approaches |
| cmmi10 0x10 | U+03B6 `ζ` | \zeta, \zeta (fontmath.ltx) | +0.00498 | +0.00500 | -0.09286 | -0.09321 | -0.02122 | below | approaches |
| cmmi10 0x54 | U+0054 `T` | ASCII math | -0.00376 | -0.00378 | +0.08733 | +0.08766 | +0.01494 | below | approaches |
| cmmi10 0x56 | U+0056 `V` | ASCII math | -0.00334 | -0.00335 | -0.08557 | -0.08589 | -0.01467 | below | approaches |
| cmmi10 0x11 | U+03B7 `η` | \eta, \eta (fontmath.ltx) | +0.00469 | +0.00470 | -0.08411 | -0.08442 | -0.01694 | below | approaches |
| cmmi10 0x79 | U+0079 `y` | ASCII math | -0.00282 | -0.00283 | -0.08161 | -0.08192 | -0.01665 | below | approaches |

## Complete measured table

All compared rows are included below, including rows with zero or near-zero deltas. `CM → LM` columns are points; delta fractions use the raw CM advance. Effective char-box width is raw advance plus italic correction.

| family/slot | glyph | labels | gid/face | width CM → LM pt | Δw pt | Δw bp | Δw/adv | effective char-box CM → LM pt | Δbox bp | Δbox/adv | height CM → LM pt | Δh pt | Δh/adv | depth CM → LM pt | Δd pt | Δd/adv | italic CM → LM pt | Δic pt | Δic/adv |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cmr10 0x00 | U+0393 `Γ` | \Gamma, \Gamma (fontmath.ltx) | 306 / lmroman10-regular | 6.25002 → 6.25000 | -0.00002 | -0.00002 | -0.00000 | 6.25002 → 6.25000 | -0.00002 | -0.00000 | 6.83332 → 6.80000 | -0.03332 | -0.00533 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x01 | U+0394 `Δ` | \Delta, \Delta (fontmath.ltx) | 235 / lmroman10-regular | 8.33336 → 8.33000 | -0.00336 | -0.00337 | -0.00040 | 8.33336 → 8.33000 | -0.00337 | -0.00040 | 6.83332 → 7.16000 | +0.32668 | +0.03920 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x02 | U+0398 `Θ` | \Theta, \Theta (fontmath.ltx) | 555 / lmroman10-regular | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.83332 → 7.05000 | +0.21668 | +0.02786 | 0.00000 → 0.22000 | +0.22000 | +0.02829 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x03 | U+039B `Λ` | \Lambda, \Lambda (fontmath.ltx) | 376 / lmroman10-regular | 6.94446 → 6.94000 | -0.00446 | -0.00447 | -0.00064 | 6.94446 → 6.94000 | -0.00447 | -0.00064 | 6.83332 → 7.16000 | +0.32668 | +0.04704 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x04 | U+039E `Ξ` | \Xi, \Xi (fontmath.ltx) | 629 / lmroman10-regular | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 6.83332 → 6.77000 | -0.06332 | -0.00950 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x05 | U+03A0 `Π` | \Pi, \Pi (fontmath.ltx) | 491 / lmroman10-regular | 7.50002 → 7.50000 | -0.00002 | -0.00002 | -0.00000 | 7.50002 → 7.50000 | -0.00002 | -0.00000 | 6.83332 → 6.80000 | -0.03332 | -0.00444 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x06 | U+03A3 `Σ` | \Sigma, \Sigma (fontmath.ltx) | 543 / lmroman10-regular | 7.22224 → 7.22000 | -0.00224 | -0.00225 | -0.00031 | 7.22224 → 7.22000 | -0.00225 | -0.00031 | 6.83332 → 6.83000 | -0.00332 | -0.00046 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x07 | U+03A5 `Υ` | \Upsilon, \Upsilon (fontmath.ltx) | 615 / lmroman10-regular | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.83332 → 7.05000 | +0.21668 | +0.02786 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x08 | U+03A6 `Φ` | \Phi, \Phi (fontmath.ltx) | 490 / lmroman10-regular | 7.22224 → 7.22000 | -0.00224 | -0.00225 | -0.00031 | 7.22224 → 7.22000 | -0.00225 | -0.00031 | 6.83332 → 6.83000 | -0.00332 | -0.00046 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x09 | U+03A8 `Ψ` | \Psi, \Psi (fontmath.ltx) | 493 / lmroman10-regular | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.83332 → 6.83000 | -0.00332 | -0.00043 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x0A | U+03A9 `Ω` | \Omega, \Omega (fontmath.ltx) | 469 / lmroman10-regular | 7.22224 → 7.22000 | -0.00224 | -0.00225 | -0.00031 | 7.22224 → 7.22000 | -0.00225 | -0.00031 | 6.83332 → 7.05000 | +0.21668 | +0.03000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x12 | U+0060 GRAVE ACCENT | \grave (fontmath.ltx) | 60 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.94445 → 6.98000 | +0.03555 | +0.00711 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x13 | U+00B4 `´` | \acute (fontmath.ltx) | 155 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.94445 → 6.98000 | +0.03555 | +0.00711 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x14 | U+02C7 `ˇ` | \check (fontmath.ltx) | 202 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.28473 → 6.92000 | +0.63527 | +0.12705 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x15 | U+02D8 `˘` | \breve (fontmath.ltx) | 193 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.94445 → 6.90000 | -0.04445 | -0.00889 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x16 | U+00AF `¯` | \bar (fontmath.ltx) | 393 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 5.67777 → 6.20000 | +0.52223 | +0.10445 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x21 | U+0021 `!` | ASCII math | 53 / lmroman10-regular | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 6.94445 → 7.16000 | +0.21555 | +0.07760 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x28 | U+0028 `(` | ASCII math | 85 / lmroman10-regular | 3.88890 → 3.89000 | +0.00110 | +0.00110 | +0.00028 | 3.88890 → 3.89000 | +0.00110 | +0.00028 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x29 | U+0029 `)` | ASCII math | 86 / lmroman10-regular | 3.88890 → 3.89000 | +0.00110 | +0.00110 | +0.00028 | 3.88890 → 3.89000 | +0.00110 | +0.00028 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x2B | U+002B `+` | ASCII math | 89 / lmroman10-regular | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.83000 | -0.00334 | -0.00043 | 0.83334 → 0.83000 | -0.00334 | -0.00043 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x2F | U+002F `/` | ASCII math | 102 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x30 | U+0030 `0` | ASCII math | 121 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x31 | U+0031 `1` | ASCII math | 82 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x32 | U+0032 `2` | ASCII math | 107 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x33 | U+0033 `3` | ASCII math | 106 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x34 | U+0034 `4` | ASCII math | 57 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.77000 | +0.32556 | +0.06511 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x35 | U+0035 `5` | ASCII math | 56 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x36 | U+0036 `6` | ASCII math | 101 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x37 | U+0037 `7` | ASCII math | 100 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.76000 | +0.31556 | +0.06311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x38 | U+0038 `8` | ASCII math | 51 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x39 | U+0039 `9` | ASCII math | 78 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.44444 → 6.66000 | +0.21556 | +0.04311 | 0.00000 → 0.22000 | +0.22000 | +0.04400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x3A | U+003A `:` | ASCII math | 44 / lmroman10-regular | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 4.30555 → 4.31000 | +0.00445 | +0.00160 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x3B | U+003B `;` | ASCII math | 99 / lmroman10-regular | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 4.30555 → 4.31000 | +0.00445 | +0.00160 | 1.94445 → 1.93000 | -0.01445 | -0.00520 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x3D | U+003D `=` | ASCII math | 52 / lmroman10-regular | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 3.66875 → 3.67000 | +0.00125 | +0.00016 | -1.33125 → 0.00000 | +1.33125 | +0.17116 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x3F | U+003F `?` | ASCII math | 92 / lmroman10-regular | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 6.94445 → 7.05000 | +0.10555 | +0.02235 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x40 | U+0040 `@` | ASCII math | 33 / lmroman10-regular | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 7.05000 | +0.10555 | +0.01357 | 0.00000 → 0.11000 | +0.11000 | +0.01414 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x5B | U+005B `[` | ASCII math | 40 / lmroman10-regular | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x5D | U+005D `]` | ASCII math | 41 / lmroman10-regular | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x5E | U+02C6 `ˆ` | \hat (fontmath.ltx) | 216 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.94445 → 6.92000 | -0.02445 | -0.00489 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x5F | U+02D9 `˙` | \dot (fontmath.ltx) | 245 / lmroman10-regular | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 6.67859 → 6.57000 | -0.10859 | -0.03909 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x7E | U+02DC `˜` | \tilde (fontmath.ltx) | 561 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.67859 → 6.51000 | -0.16859 | -0.03372 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmr10 0x7F | U+00A8 `¨` | \ddot (fontmath.ltx) | 237 / lmroman10-regular | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.67859 → 6.52000 | -0.15859 | -0.03172 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x0B | U+03B1 `α` | \alpha, \alpha (fontmath.ltx) | 4459 / latinmodern-math | 6.39702 → 6.40000 | +0.00298 | +0.00299 | +0.00047 | 6.43404 → 6.40000 | -0.03417 | -0.00532 | 4.30555 → 4.42000 | +0.11445 | +0.01789 | 0.00000 → 0.11000 | +0.11000 | +0.01720 | 0.03702 → 0.00000 | -0.03702 | -0.00579 |
| cmmi10 0x0C | U+03B2 `β` | \beta, \beta (fontmath.ltx) | 4460 / latinmodern-math | 5.65626 → 5.66000 | +0.00374 | +0.00375 | +0.00066 | 6.18404 → 6.02000 | -0.16466 | -0.02900 | 6.94445 → 7.06000 | +0.11555 | +0.02043 | 1.94445 → 1.94000 | -0.00445 | -0.00079 | 0.52778 → 0.36000 | -0.16778 | -0.02966 |
| cmmi10 0x0D | U+03B3 `γ` | \gamma, \gamma (fontmath.ltx) | 4461 / latinmodern-math | 5.17731 → 5.18000 | +0.00269 | +0.00270 | +0.00052 | 5.73286 → 5.71000 | -0.02295 | -0.00442 | 4.30555 → 4.42000 | +0.11445 | +0.02211 | 1.94445 → 2.15000 | +0.20555 | +0.03970 | 0.55555 → 0.53000 | -0.02555 | -0.00494 |
| cmmi10 0x0E | U+03B4 `δ` | \delta, \delta (fontmath.ltx) | 4462 / latinmodern-math | 4.44445 → 4.44000 | -0.00445 | -0.00446 | -0.00100 | 4.82291 → 4.80000 | -0.02300 | -0.00516 | 6.94445 → 7.12000 | +0.17555 | +0.03950 | 0.00000 → 0.11000 | +0.11000 | +0.02475 | 0.37847 → 0.36000 | -0.01847 | -0.00415 |
| cmmi10 0x0F | U+03F5 `ϵ` | \epsilon, \epsilon (fontmath.ltx) | 4463 / latinmodern-math | 4.05904 → 4.06000 | +0.00096 | +0.00097 | +0.00024 | 4.05904 → 4.06000 | +0.00097 | +0.00024 | 4.30555 → 4.31000 | +0.00445 | +0.00110 | 0.00000 → 0.11000 | +0.11000 | +0.02710 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x10 | U+03B6 `ζ` | \zeta, \zeta (fontmath.ltx) | 4464 / latinmodern-math | 4.37502 → 4.38000 | +0.00498 | +0.00500 | +0.00114 | 5.11286 → 5.02000 | -0.09321 | -0.02122 | 6.94445 → 6.97000 | +0.02555 | +0.00584 | 1.94445 → 2.05000 | +0.10555 | +0.02413 | 0.73784 → 0.64000 | -0.09784 | -0.02236 |
| cmmi10 0x11 | U+03B7 `η` | \eta, \eta (fontmath.ltx) | 4465 / latinmodern-math | 4.96531 → 4.97000 | +0.00469 | +0.00470 | +0.00094 | 5.32411 → 5.24000 | -0.08442 | -0.01694 | 4.30555 → 4.42000 | +0.11445 | +0.02305 | 1.94445 → 2.16000 | +0.21555 | +0.04341 | 0.35879 → 0.27000 | -0.08879 | -0.01788 |
| cmmi10 0x12 | U+03B8 `θ` | \theta, \theta (fontmath.ltx) | 4466 / latinmodern-math | 4.69444 → 4.69000 | -0.00444 | -0.00446 | -0.00095 | 4.97223 → 4.83000 | -0.14276 | -0.03030 | 6.94445 → 7.05000 | +0.10555 | +0.02248 | 0.00000 → 0.11000 | +0.11000 | +0.02343 | 0.27779 → 0.14000 | -0.13779 | -0.02935 |
| cmmi10 0x13 | U+03B9 `ι` | \iota, \iota (fontmath.ltx) | 4467 / latinmodern-math | 3.53937 → 3.54000 | +0.00063 | +0.00063 | +0.00018 | 3.53937 → 3.54000 | +0.00063 | +0.00018 | 4.30555 → 4.42000 | +0.11445 | +0.03234 | 0.00000 → 0.11000 | +0.11000 | +0.03108 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x14 | U+03BA `κ` | \kappa, \kappa (fontmath.ltx) | 4468 / latinmodern-math | 5.76159 → 5.76000 | -0.00159 | -0.00160 | -0.00028 | 5.76159 → 5.76000 | -0.00160 | -0.00028 | 4.30555 → 4.42000 | +0.11445 | +0.01986 | 0.00000 → 0.11000 | +0.11000 | +0.01909 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x15 | U+03BB `λ` | \lambda, \lambda (fontmath.ltx) | 4470 / latinmodern-math | 5.83336 → 5.83000 | -0.00336 | -0.00337 | -0.00058 | 5.83336 → 5.83000 | -0.00337 | -0.00058 | 6.94445 → 6.94000 | -0.00445 | -0.00076 | 0.00000 → 0.13000 | +0.13000 | +0.02229 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x16 | U+03BC `μ` | \mu, \mu (fontmath.ltx) | 4471 / latinmodern-math | 6.02550 → 6.03000 | +0.00450 | +0.00452 | +0.00075 | 6.02550 → 6.03000 | +0.00452 | +0.00075 | 4.30555 → 4.42000 | +0.11445 | +0.01899 | 1.94445 → 2.16000 | +0.21555 | +0.03577 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x17 | U+03BD `ν` | \nu, \nu (fontmath.ltx) | 4472 / latinmodern-math | 4.93983 → 4.94000 | +0.00017 | +0.00017 | +0.00003 | 5.57641 → 5.52000 | -0.05662 | -0.01142 | 4.30555 → 4.42000 | +0.11445 | +0.02317 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.63658 → 0.58000 | -0.05658 | -0.01145 |
| cmmi10 0x18 | U+03BE `ξ` | \xi, \xi (fontmath.ltx) | 4473 / latinmodern-math | 4.37502 → 4.38000 | +0.00498 | +0.00500 | +0.00114 | 4.83509 → 4.74000 | -0.09545 | -0.02173 | 6.94445 → 6.97000 | +0.02555 | +0.00584 | 1.94445 → 2.05000 | +0.10555 | +0.02413 | 0.46007 → 0.36000 | -0.10007 | -0.02287 |
| cmmi10 0x19 | U+03C0 `π` | \pi, \pi (fontmath.ltx) | 4474 / latinmodern-math | 5.70027 → 5.70000 | -0.00027 | -0.00027 | -0.00005 | 6.05906 → 5.95000 | -0.10947 | -0.01913 | 4.30555 → 4.31000 | +0.00445 | +0.00078 | 0.00000 → 0.11000 | +0.11000 | +0.01930 | 0.35879 → 0.25000 | -0.10879 | -0.01909 |
| cmmi10 0x1A | U+03C1 `ρ` | \rho, \rho (fontmath.ltx) | 4475 / latinmodern-math | 5.17015 → 5.17000 | -0.00015 | -0.00016 | -0.00003 | 5.17015 → 5.30000 | +0.13033 | +0.02511 | 4.30555 → 4.42000 | +0.11445 | +0.02214 | 1.94445 → 2.16000 | +0.21555 | +0.04169 | 0.00000 → 0.13000 | +0.13000 | +0.02514 |
| cmmi10 0x1B | U+03C3 `σ` | \sigma, \sigma (fontmath.ltx) | 4476 / latinmodern-math | 5.71414 → 5.71000 | -0.00414 | -0.00416 | -0.00072 | 6.07293 → 5.95000 | -0.12339 | -0.02151 | 4.30555 → 4.31000 | +0.00445 | +0.00078 | 0.00000 → 0.11000 | +0.11000 | +0.01925 | 0.35879 → 0.24000 | -0.11879 | -0.02079 |
| cmmi10 0x1C | U+03C4 `τ` | \tau, \tau (fontmath.ltx) | 4477 / latinmodern-math | 4.37155 → 4.37000 | -0.00155 | -0.00155 | -0.00035 | 5.50350 → 5.39000 | -0.11393 | -0.02596 | 4.30555 → 4.31000 | +0.00445 | +0.00102 | 0.00000 → 0.12000 | +0.12000 | +0.02745 | 1.13195 → 1.02000 | -0.11195 | -0.02561 |
| cmmi10 0x1D | U+03C5 `υ` | \upsilon, \upsilon (fontmath.ltx) | 4478 / latinmodern-math | 5.40280 → 5.40000 | -0.00280 | -0.00281 | -0.00052 | 5.76159 → 5.52000 | -0.24250 | -0.04472 | 4.30555 → 4.42000 | +0.11445 | +0.02118 | 0.00000 → 0.11000 | +0.11000 | +0.02036 | 0.35879 → 0.12000 | -0.23879 | -0.04420 |
| cmmi10 0x1E | U+03C6 `φ` | \phi, \phi (fontmath.ltx) | 4479 / latinmodern-math | 5.95835 → 5.96000 | +0.00165 | +0.00166 | +0.00028 | 5.95835 → 6.01000 | +0.05185 | +0.00867 | 6.94445 → 6.94000 | -0.00445 | -0.00075 | 1.94445 → 2.05000 | +0.10555 | +0.01772 | 0.00000 → 0.05000 | +0.05000 | +0.00839 |
| cmmi10 0x1F | U+03C7 `χ` | \chi, \chi (fontmath.ltx) | 4480 / latinmodern-math | 6.25692 → 6.26000 | +0.00308 | +0.00309 | +0.00049 | 6.25692 → 6.26000 | +0.00309 | +0.00049 | 4.30555 → 4.42000 | +0.11445 | +0.01829 | 1.94445 → 2.05000 | +0.10555 | +0.01687 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x20 | U+03C8 `ψ` | \psi, \psi (fontmath.ltx) | 4481 / latinmodern-math | 6.51392 → 6.51000 | -0.00392 | -0.00393 | -0.00060 | 6.87271 → 6.63000 | -0.24362 | -0.03726 | 6.94445 → 6.94000 | -0.00445 | -0.00068 | 1.94445 → 2.05000 | +0.10555 | +0.01620 | 0.35879 → 0.12000 | -0.23879 | -0.03666 |
| cmmi10 0x21 | U+03C9 `ω` | \omega, \omega (fontmath.ltx) | 4482 / latinmodern-math | 6.22455 → 6.22000 | -0.00455 | -0.00456 | -0.00073 | 6.58334 → 6.32000 | -0.26433 | -0.04231 | 4.30555 → 4.42000 | +0.11445 | +0.01839 | 0.00000 → 0.11000 | +0.11000 | +0.01767 | 0.35879 → 0.10000 | -0.25879 | -0.04158 |
| cmmi10 0x22 | U+03B5 `ε` | \varepsilon, \varepsilon (fontmath.ltx) | 4483 / latinmodern-math | 4.66318 → 4.66000 | -0.00318 | -0.00319 | -0.00068 | 4.66318 → 4.66000 | -0.00319 | -0.00068 | 4.30555 → 4.53000 | +0.22445 | +0.04813 | 0.00000 → 0.22000 | +0.22000 | +0.04718 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x23 | U+03D1 `ϑ` | \vartheta, \vartheta (fontmath.ltx) | 4484 / latinmodern-math | 5.91440 → 5.91000 | -0.00440 | -0.00442 | -0.00074 | 5.91440 → 5.91000 | -0.00442 | -0.00074 | 6.94445 → 7.05000 | +0.10555 | +0.01785 | 0.00000 → 0.11000 | +0.11000 | +0.01860 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x24 | U+03D6 `ϖ` | \varpi, \varpi (fontmath.ltx) | 4485 / latinmodern-math | 8.28130 → 8.28000 | -0.00130 | -0.00130 | -0.00016 | 8.55908 → 8.45000 | -0.10949 | -0.01317 | 4.30555 → 4.31000 | +0.00445 | +0.00054 | 0.00000 → 0.11000 | +0.11000 | +0.01328 | 0.27779 → 0.17000 | -0.10779 | -0.01302 |
| cmmi10 0x26 | U+03C2 `ς` | \varsigma, \varsigma (fontmath.ltx) | 4487 / latinmodern-math | 3.62848 → 3.63000 | +0.00152 | +0.00152 | +0.00042 | 4.42708 → 4.37000 | -0.05729 | -0.01573 | 4.30555 → 4.42000 | +0.11445 | +0.03154 | 0.97223 → 1.08000 | +0.10777 | +0.02970 | 0.79860 → 0.74000 | -0.05860 | -0.01615 |
| cmmi10 0x27 | U+03D5 `ϕ` | \varphi, \varphi (fontmath.ltx) | 4488 / latinmodern-math | 6.54167 → 6.54000 | -0.00167 | -0.00168 | -0.00026 | 6.54167 → 6.54000 | -0.00168 | -0.00026 | 4.30555 → 4.42000 | +0.11445 | +0.01750 | 1.94445 → 2.18000 | +0.23555 | +0.03601 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x3A | U+002E `.` | ASCII math | 15 / latinmodern-math | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 1.05556 → 1.06000 | +0.00444 | +0.00160 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x3B | U+002C `,` | ASCII math | 13 / latinmodern-math | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 1.05556 → 1.06000 | +0.00444 | +0.00160 | 1.94445 → 1.93000 | -0.01445 | -0.00520 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x3C | U+003C `<` | ASCII math | 29 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.39098 → 5.56000 | +0.16902 | +0.02173 | 0.39098 → 0.56000 | +0.16902 | +0.02173 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x3D | U+002F `/` | ASCII math | 16 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x3E | U+003E `>` | ASCII math | 31 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.39098 → 5.56000 | +0.16902 | +0.02173 | 0.39098 → 0.56000 | +0.16902 | +0.02173 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x40 | U+2202 `∂` | \partial, \partial (fontmath.ltx) | 4489 / latinmodern-math | 5.30904 → 5.31000 | +0.00096 | +0.00097 | +0.00018 | 5.86459 → 5.94000 | +0.07569 | +0.01420 | 6.94445 → 7.16000 | +0.21555 | +0.04060 | 0.00000 → 0.22000 | +0.22000 | +0.04144 | 0.55555 → 0.63000 | +0.07445 | +0.01402 |
| cmmi10 0x41 | U+0041 `A` | ASCII math | 1270 / latinmodern-math | 7.50002 → 7.50000 | -0.00002 | -0.00002 | -0.00000 | 7.50002 → 7.50000 | -0.00002 | -0.00000 | 6.83332 → 7.16000 | +0.32668 | +0.04356 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x42 | U+0042 `B` | ASCII math | 1271 / latinmodern-math | 7.58510 → 7.59000 | +0.00490 | +0.00492 | +0.00065 | 8.08682 → 7.84000 | -0.24775 | -0.03254 | 6.83332 → 6.83000 | -0.00332 | -0.00044 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.50173 → 0.25000 | -0.25173 | -0.03319 |
| cmmi10 0x43 | U+0043 `C` | ASCII math | 1272 / latinmodern-math | 7.14722 → 7.15000 | +0.00278 | +0.00279 | +0.00039 | 7.86249 → 7.88000 | +0.01757 | +0.00245 | 6.83332 → 7.05000 | +0.21668 | +0.03032 | 0.00000 → 0.22000 | +0.22000 | +0.03078 | 0.71527 → 0.73000 | +0.01473 | +0.00206 |
| cmmi10 0x44 | U+0044 `D` | ASCII math | 1273 / latinmodern-math | 8.27917 → 8.28000 | +0.00083 | +0.00083 | +0.00010 | 8.55696 → 8.32000 | -0.23785 | -0.02862 | 6.83332 → 6.83000 | -0.00332 | -0.00040 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.27779 → 0.04000 | -0.23779 | -0.02872 |
| cmmi10 0x45 | U+0045 `E` | ASCII math | 1274 / latinmodern-math | 7.38195 → 7.38000 | -0.00195 | -0.00196 | -0.00026 | 7.95834 → 7.92000 | -0.03848 | -0.00519 | 6.83332 → 6.80000 | -0.03332 | -0.00451 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.57638 → 0.54000 | -0.03638 | -0.00493 |
| cmmi10 0x46 | U+0046 `F` | ASCII math | 1275 / latinmodern-math | 6.43057 → 6.43000 | -0.00057 | -0.00057 | -0.00009 | 7.81947 → 7.77000 | -0.04966 | -0.00769 | 6.83332 → 6.80000 | -0.03332 | -0.00518 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 1.38890 → 1.34000 | -0.04890 | -0.00760 |
| cmmi10 0x47 | U+0047 `G` | ASCII math | 1276 / latinmodern-math | 7.86249 → 7.86000 | -0.00249 | -0.00250 | -0.00032 | 7.86249 → 7.86000 | -0.00250 | -0.00032 | 6.83332 → 7.05000 | +0.21668 | +0.02756 | 0.00000 → 0.22000 | +0.22000 | +0.02798 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x48 | U+0048 `H` | ASCII math | 1277 / latinmodern-math | 8.31251 → 8.31000 | -0.00251 | -0.00252 | -0.00030 | 9.12499 → 9.09000 | -0.03513 | -0.00421 | 6.83332 → 6.83000 | -0.00332 | -0.00040 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.81248 → 0.78000 | -0.03248 | -0.00391 |
| cmmi10 0x49 | U+0049 `I` | ASCII math | 1278 / latinmodern-math | 4.39585 → 4.40000 | +0.00415 | +0.00417 | +0.00094 | 5.18056 → 5.25000 | +0.06970 | +0.01580 | 6.83332 → 6.83000 | -0.00332 | -0.00076 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.78471 → 0.85000 | +0.06529 | +0.01485 |
| cmmi10 0x4A | U+004A `J` | ASCII math | 1279 / latinmodern-math | 5.54514 → 5.55000 | +0.00486 | +0.00488 | +0.00088 | 6.50695 → 6.61000 | +0.10344 | +0.01858 | 6.83332 → 6.83000 | -0.00332 | -0.00060 | 0.00000 → 0.22000 | +0.22000 | +0.03967 | 0.96181 → 1.06000 | +0.09819 | +0.01771 |
| cmmi10 0x4B | U+004B `K` | ASCII math | 1280 / latinmodern-math | 8.49307 → 8.49000 | -0.00307 | -0.00308 | -0.00036 | 9.20835 → 9.17000 | -0.03849 | -0.00451 | 6.83332 → 6.83000 | -0.00332 | -0.00039 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.71527 → 0.68000 | -0.03527 | -0.00415 |
| cmmi10 0x4C | U+004C `L` | ASCII math | 1281 / latinmodern-math | 6.80557 → 6.81000 | +0.00443 | +0.00444 | +0.00065 | 6.80557 → 6.81000 | +0.00444 | +0.00065 | 6.83332 → 6.83000 | -0.00332 | -0.00049 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x4D | U+004D `M` | ASCII math | 1282 / latinmodern-math | 9.70140 → 9.70000 | -0.00140 | -0.00141 | -0.00014 | 10.79167 → 10.72000 | -0.07194 | -0.00739 | 6.83332 → 6.83000 | -0.00332 | -0.00034 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 1.09027 → 1.02000 | -0.07027 | -0.00724 |
| cmmi10 0x4E | U+004E `N` | ASCII math | 1283 / latinmodern-math | 8.03473 → 8.03000 | -0.00473 | -0.00474 | -0.00059 | 9.12499 → 9.09000 | -0.03513 | -0.00436 | 6.83332 → 6.83000 | -0.00332 | -0.00041 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 1.09027 → 1.06000 | -0.03027 | -0.00377 |
| cmmi10 0x4F | U+004F `O` | ASCII math | 1284 / latinmodern-math | 7.62776 → 7.63000 | +0.00224 | +0.00224 | +0.00029 | 7.90555 → 7.68000 | -0.22640 | -0.02957 | 6.83332 → 7.05000 | +0.21668 | +0.02841 | 0.00000 → 0.22000 | +0.22000 | +0.02884 | 0.27779 → 0.05000 | -0.22779 | -0.02986 |
| cmmi10 0x50 | U+0050 `P` | ASCII math | 1285 / latinmodern-math | 6.42014 → 6.42000 | -0.00014 | -0.00014 | -0.00002 | 7.80904 → 7.82000 | +0.01100 | +0.00171 | 6.83332 → 6.83000 | -0.00332 | -0.00052 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 1.38890 → 1.40000 | +0.01110 | +0.00173 |
| cmmi10 0x51 | U+0051 `Q` | ASCII math | 1286 / latinmodern-math | 7.90555 → 7.91000 | +0.00445 | +0.00447 | +0.00056 | 7.90555 → 7.91000 | +0.00447 | +0.00056 | 6.83332 → 7.05000 | +0.21668 | +0.02741 | 1.94445 → 1.94000 | -0.00445 | -0.00056 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x52 | U+0052 `R` | ASCII math | 1287 / latinmodern-math | 7.59290 → 7.59000 | -0.00290 | -0.00291 | -0.00038 | 7.67015 → 7.83000 | +0.16044 | +0.02105 | 6.83332 → 6.83000 | -0.00332 | -0.00044 | 0.00000 → 0.22000 | +0.22000 | +0.02897 | 0.07726 → 0.24000 | +0.16274 | +0.02143 |
| cmmi10 0x53 | U+0053 `S` | ASCII math | 1288 / latinmodern-math | 6.13195 → 6.13000 | -0.00195 | -0.00196 | -0.00032 | 6.70834 → 6.73000 | +0.02175 | +0.00353 | 6.83332 → 7.05000 | +0.21668 | +0.03534 | 0.00000 → 0.22000 | +0.22000 | +0.03588 | 0.57638 → 0.60000 | +0.02362 | +0.00385 |
| cmmi10 0x54 | U+0054 `T` | ASCII math | 1289 / latinmodern-math | 5.84376 → 5.84000 | -0.00376 | -0.00378 | -0.00064 | 7.23267 → 7.32000 | +0.08766 | +0.01494 | 6.83332 → 6.77000 | -0.06332 | -0.01084 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 1.38890 → 1.48000 | +0.09110 | +0.01559 |
| cmmi10 0x55 | U+0055 `U` | ASCII math | 1290 / latinmodern-math | 6.82777 → 6.83000 | +0.00223 | +0.00223 | +0.00033 | 7.91804 → 7.88000 | -0.03819 | -0.00557 | 6.83332 → 6.83000 | -0.00332 | -0.00049 | 0.00000 → 0.22000 | +0.22000 | +0.03222 | 1.09027 → 1.05000 | -0.04027 | -0.00590 |
| cmmi10 0x56 | U+0056 `V` | ASCII math | 1291 / latinmodern-math | 5.83334 → 5.83000 | -0.00334 | -0.00335 | -0.00057 | 8.05557 → 7.97000 | -0.08589 | -0.01467 | 6.83332 → 6.83000 | -0.00332 | -0.00057 | 0.00000 → 0.22000 | +0.22000 | +0.03771 | 2.22223 → 2.14000 | -0.08223 | -0.01410 |
| cmmi10 0x57 | U+0057 `W` | ASCII math | 1292 / latinmodern-math | 9.44446 → 9.44000 | -0.00446 | -0.00447 | -0.00047 | 10.83336 → 10.76000 | -0.07363 | -0.00777 | 6.83332 → 6.83000 | -0.00332 | -0.00035 | 0.00000 → 0.22000 | +0.22000 | +0.02329 | 1.38890 → 1.32000 | -0.06890 | -0.00730 |
| cmmi10 0x58 | U+0058 `X` | ASCII math | 1293 / latinmodern-math | 8.28474 → 8.28000 | -0.00474 | -0.00476 | -0.00057 | 9.06945 → 8.79000 | -0.28050 | -0.03373 | 6.83332 → 6.83000 | -0.00332 | -0.00040 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.78471 → 0.51000 | -0.27471 | -0.03316 |
| cmmi10 0x59 | U+0059 `Y` | ASCII math | 1294 / latinmodern-math | 5.80557 → 5.81000 | +0.00443 | +0.00445 | +0.00076 | 8.02780 → 7.90000 | -0.12828 | -0.02201 | 6.83332 → 6.83000 | -0.00332 | -0.00057 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 2.22223 → 2.09000 | -0.13223 | -0.02278 |
| cmmi10 0x5A | U+005A `Z` | ASCII math | 1295 / latinmodern-math | 6.82640 → 6.83000 | +0.00360 | +0.00361 | +0.00053 | 7.54168 → 7.51000 | -0.03179 | -0.00464 | 6.83332 → 6.83000 | -0.00332 | -0.00049 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.71527 → 0.68000 | -0.03527 | -0.00517 |
| cmmi10 0x5B | U+266D `♭` | \flat (fontmath.ltx) | 3045 / latinmodern-math | 3.88890 → 3.88000 | -0.00890 | -0.00894 | -0.00229 | 3.88890 → 3.88000 | -0.00894 | -0.00229 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 0.00000 → 0.22000 | +0.22000 | +0.05657 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x5C | U+266E `♮` | \natural (fontmath.ltx) | 3044 / latinmodern-math | 3.88890 → 3.88000 | -0.00890 | -0.00894 | -0.00229 | 3.88890 → 3.88000 | -0.00894 | -0.00229 | 6.94445 → 7.28000 | +0.33555 | +0.08628 | 1.94445 → 2.17000 | +0.22555 | +0.05800 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x5D | U+266F `♯` | \sharp (fontmath.ltx) | 3043 / latinmodern-math | 3.88890 → 3.88000 | -0.00890 | -0.00894 | -0.00229 | 3.88890 → 3.88000 | -0.00894 | -0.00229 | 6.94445 → 7.16000 | +0.21555 | +0.05543 | 1.94445 → 2.16000 | +0.21555 | +0.05543 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x60 | U+2113 `ℓ` | \ell (fontmath.ltx) | 1263 / latinmodern-math | 4.16670 → 4.17000 | +0.00330 | +0.00331 | +0.00079 | 4.16670 → 4.26000 | +0.09365 | +0.02239 | 6.94445 → 7.05000 | +0.10555 | +0.02533 | 0.00000 → 0.12000 | +0.12000 | +0.02880 | 0.00000 → 0.09000 | +0.09000 | +0.02160 |
| cmmi10 0x61 | U+0061 `a` | ASCII math | 1296 / latinmodern-math | 5.28590 → 5.29000 | +0.00410 | +0.00411 | +0.00078 | 5.28590 → 5.29000 | +0.00411 | +0.00078 | 4.30555 → 4.42000 | +0.11445 | +0.02165 | 0.00000 → 0.11000 | +0.11000 | +0.02081 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x62 | U+0062 `b` | ASCII math | 1297 / latinmodern-math | 4.29167 → 4.29000 | -0.00167 | -0.00167 | -0.00039 | 4.29167 → 4.43000 | +0.13885 | +0.03223 | 6.94445 → 6.94000 | -0.00445 | -0.00104 | 0.00000 → 0.11000 | +0.11000 | +0.02563 | 0.00000 → 0.14000 | +0.14000 | +0.03262 |
| cmmi10 0x63 | U+0063 `c` | ASCII math | 1298 / latinmodern-math | 4.32756 → 4.33000 | +0.00244 | +0.00244 | +0.00056 | 4.32756 → 4.58000 | +0.25338 | +0.05833 | 4.30555 → 4.42000 | +0.11445 | +0.02645 | 0.00000 → 0.11000 | +0.11000 | +0.02542 | 0.00000 → 0.25000 | +0.25000 | +0.05777 |
| cmmi10 0x64 | U+0064 `d` | ASCII math | 1299 / latinmodern-math | 5.20488 → 5.20000 | -0.00488 | -0.00490 | -0.00094 | 5.20488 → 5.44000 | +0.23600 | +0.04517 | 6.94445 → 6.94000 | -0.00445 | -0.00085 | 0.00000 → 0.11000 | +0.11000 | +0.02113 | 0.00000 → 0.24000 | +0.24000 | +0.04611 |
| cmmi10 0x65 | U+0065 `e` | ASCII math | 1300 / latinmodern-math | 4.65627 → 4.66000 | +0.00373 | +0.00375 | +0.00080 | 4.65627 → 4.66000 | +0.00375 | +0.00080 | 4.30555 → 4.42000 | +0.11445 | +0.02458 | 0.00000 → 0.11000 | +0.11000 | +0.02362 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x66 | U+0066 `f` | ASCII math | 1301 / latinmodern-math | 4.89586 → 4.90000 | +0.00414 | +0.00416 | +0.00085 | 5.97226 → 5.80000 | -0.17291 | -0.03519 | 6.94445 → 7.05000 | +0.10555 | +0.02156 | 1.94445 → 2.05000 | +0.10555 | +0.02156 | 1.07640 → 0.90000 | -0.17640 | -0.03603 |
| cmmi10 0x67 | U+0067 `g` | ASCII math | 1302 / latinmodern-math | 4.76969 → 4.77000 | +0.00031 | +0.00031 | +0.00007 | 5.12848 → 5.02000 | -0.10889 | -0.02274 | 4.30555 → 4.42000 | +0.11445 | +0.02399 | 1.94445 → 2.05000 | +0.10555 | +0.02213 | 0.35879 → 0.25000 | -0.10879 | -0.02281 |
| cmmi10 0x68 | U+0068 `h` | ASCII math | 1303 / latinmodern-math | 5.76159 → 5.76000 | -0.00159 | -0.00160 | -0.00028 | 5.76159 → 5.76000 | -0.00160 | -0.00028 | 6.94445 → 6.94000 | -0.00445 | -0.00077 | 0.00000 → 0.11000 | +0.11000 | +0.01909 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x69 | U+0069 `i` | ASCII math | 1304 / latinmodern-math | 3.44513 → 3.45000 | +0.00487 | +0.00489 | +0.00141 | 3.44513 → 3.45000 | +0.00489 | +0.00141 | 6.59525 → 6.61000 | +0.01475 | +0.00428 | 0.00000 → 0.11000 | +0.11000 | +0.03193 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x6A | U+006A `j` | ASCII math | 1305 / latinmodern-math | 4.11807 → 4.12000 | +0.00193 | +0.00194 | +0.00047 | 4.69050 → 4.25000 | -0.44216 | -0.10697 | 6.59525 → 6.61000 | +0.01475 | +0.00358 | 1.94445 → 2.05000 | +0.10555 | +0.02563 | 0.57243 → 0.13000 | -0.44243 | -0.10744 |
| cmmi10 0x6B | U+006B `k` | ASCII math | 1306 / latinmodern-math | 5.20604 → 5.21000 | +0.00396 | +0.00397 | +0.00076 | 5.52085 → 5.36000 | -0.16145 | -0.03090 | 6.94445 → 6.94000 | -0.00445 | -0.00085 | 0.00000 → 0.11000 | +0.11000 | +0.02113 | 0.31481 → 0.15000 | -0.16481 | -0.03166 |
| cmmi10 0x6C | U+006C `l` | ASCII math | 1307 / latinmodern-math | 2.98380 → 2.98000 | -0.00380 | -0.00381 | -0.00127 | 3.18058 → 2.98000 | -0.20133 | -0.06722 | 6.94445 → 6.94000 | -0.00445 | -0.00149 | 0.00000 → 0.11000 | +0.11000 | +0.03687 | 0.19678 → 0.00000 | -0.19678 | -0.06595 |
| cmmi10 0x6D | U+006D `m` | ASCII math | 1308 / latinmodern-math | 8.78014 → 8.78000 | -0.00014 | -0.00014 | -0.00002 | 8.78014 → 8.78000 | -0.00014 | -0.00002 | 4.30555 → 4.42000 | +0.11445 | +0.01303 | 0.00000 → 0.11000 | +0.11000 | +0.01253 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x6E | U+006E `n` | ASCII math | 1309 / latinmodern-math | 6.00235 → 6.00000 | -0.00235 | -0.00236 | -0.00039 | 6.00235 → 6.00000 | -0.00236 | -0.00039 | 4.30555 → 4.42000 | +0.11445 | +0.01907 | 0.00000 → 0.11000 | +0.11000 | +0.01833 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x6F | U+006F `o` | ASCII math | 1310 / latinmodern-math | 4.84723 → 4.85000 | +0.00277 | +0.00278 | +0.00057 | 4.84723 → 4.97000 | +0.12323 | +0.02533 | 4.30555 → 4.42000 | +0.11445 | +0.02361 | 0.00000 → 0.11000 | +0.11000 | +0.02269 | 0.00000 → 0.12000 | +0.12000 | +0.02476 |
| cmmi10 0x70 | U+0070 `p` | ASCII math | 1311 / latinmodern-math | 5.03126 → 5.03000 | -0.00126 | -0.00127 | -0.00025 | 5.03126 → 5.18000 | +0.14930 | +0.02956 | 4.30555 → 4.42000 | +0.11445 | +0.02275 | 1.94445 → 1.94000 | -0.00445 | -0.00088 | 0.00000 → 0.15000 | +0.15000 | +0.02981 |
| cmmi10 0x71 | U+0071 `q` | ASCII math | 1312 / latinmodern-math | 4.46414 → 4.46000 | -0.00414 | -0.00416 | -0.00093 | 4.82293 → 4.80000 | -0.02302 | -0.00514 | 4.30555 → 4.42000 | +0.11445 | +0.02564 | 1.94445 → 1.94000 | -0.00445 | -0.00100 | 0.35879 → 0.34000 | -0.01879 | -0.00421 |
| cmmi10 0x72 | U+0072 `r` | ASCII math | 1313 / latinmodern-math | 4.51159 → 4.51000 | -0.00159 | -0.00160 | -0.00035 | 4.78938 → 4.64000 | -0.14994 | -0.03311 | 4.30555 → 4.42000 | +0.11445 | +0.02537 | 0.00000 → 0.11000 | +0.11000 | +0.02438 | 0.27779 → 0.13000 | -0.14779 | -0.03276 |
| cmmi10 0x73 | U+0073 `s` | ASCII math | 1314 / latinmodern-math | 4.68750 → 4.69000 | +0.00250 | +0.00251 | +0.00053 | 4.68750 → 4.69000 | +0.00251 | +0.00053 | 4.30555 → 4.42000 | +0.11445 | +0.02442 | 0.00000 → 0.11000 | +0.11000 | +0.02347 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x74 | U+0074 `t` | ASCII math | 1315 / latinmodern-math | 3.61113 → 3.61000 | -0.00113 | -0.00113 | -0.00031 | 3.61113 → 3.61000 | -0.00113 | -0.00031 | 6.15080 → 6.26000 | +0.10920 | +0.03024 | 0.00000 → 0.11000 | +0.11000 | +0.03046 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x75 | U+0075 `u` | ASCII math | 1316 / latinmodern-math | 5.72458 → 5.72000 | -0.00458 | -0.00460 | -0.00080 | 5.72458 → 5.72000 | -0.00460 | -0.00080 | 4.30555 → 4.42000 | +0.11445 | +0.01999 | 0.00000 → 0.11000 | +0.11000 | +0.01922 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x76 | U+0076 `v` | ASCII math | 1317 / latinmodern-math | 4.84724 → 4.85000 | +0.00276 | +0.00277 | +0.00057 | 5.20603 → 4.96000 | -0.24695 | -0.05076 | 4.30555 → 4.42000 | +0.11445 | +0.02361 | 0.00000 → 0.11000 | +0.11000 | +0.02269 | 0.35879 → 0.11000 | -0.24879 | -0.05133 |
| cmmi10 0x77 | U+0077 `w` | ASCII math | 1318 / latinmodern-math | 7.15918 → 7.16000 | +0.00082 | +0.00083 | +0.00012 | 7.42826 → 7.19000 | -0.23916 | -0.03328 | 4.30555 → 4.42000 | +0.11445 | +0.01599 | 0.00000 → 0.11000 | +0.11000 | +0.01536 | 0.26909 → 0.03000 | -0.23909 | -0.03340 |
| cmmi10 0x78 | U+0078 `x` | ASCII math | 1319 / latinmodern-math | 5.71528 → 5.72000 | +0.00472 | +0.00473 | +0.00083 | 5.71528 → 5.72000 | +0.00473 | +0.00083 | 4.30555 → 4.42000 | +0.11445 | +0.02002 | 0.00000 → 0.11000 | +0.11000 | +0.01925 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x79 | U+0079 `y` | ASCII math | 1320 / latinmodern-math | 4.90282 → 4.90000 | -0.00282 | -0.00283 | -0.00058 | 5.26161 → 5.18000 | -0.08192 | -0.01665 | 4.30555 → 4.42000 | +0.11445 | +0.02334 | 1.94445 → 2.05000 | +0.10555 | +0.02153 | 0.35879 → 0.28000 | -0.07879 | -0.01607 |
| cmmi10 0x7A | U+007A `z` | ASCII math | 1321 / latinmodern-math | 4.65050 → 4.65000 | -0.00050 | -0.00050 | -0.00011 | 5.09030 → 4.95000 | -0.14083 | -0.03017 | 4.30555 → 4.42000 | +0.11445 | +0.02461 | 0.00000 → 0.11000 | +0.11000 | +0.02365 | 0.43981 → 0.30000 | -0.13981 | -0.03006 |
| cmmi10 0x7B | U+0131 `ı` | \imath (fontmath.ltx) | 143 / latinmodern-math | 3.22456 → 2.78000 | -0.44456 | -0.44623 | -0.13787 | 3.22456 → 2.78000 | -0.44623 | -0.13787 | 4.30555 → 4.42000 | +0.11445 | +0.03549 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x7C | U+0237 `ȷ` | \jmath (fontmath.ltx) | 180 / latinmodern-math | 3.84030 → 3.06000 | -0.78030 | -0.78323 | -0.20319 | 3.84030 → 3.06000 | -0.78323 | -0.20319 | 4.30555 → 4.42000 | +0.11445 | +0.02980 | 1.94445 → 2.05000 | +0.10555 | +0.02749 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x7D | U+2118 `℘` | \wp (fontmath.ltx) | 2996 / latinmodern-math | 6.36459 → 6.36000 | -0.00459 | -0.00461 | -0.00072 | 6.36459 → 6.36000 | -0.00461 | -0.00072 | 4.30555 → 4.53000 | +0.22445 | +0.03526 | 1.94445 → 2.16000 | +0.21555 | +0.03387 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmmi10 0x7E | U+20D7 `⃗` | \vec (fontmath.ltx) | 1817 / latinmodern-math | 5.00002 → 0.00000 | -5.00002 | -5.01877 | -1.00000 | 6.53821 → 0.00000 | -6.56273 | -1.30764 | 7.14444 → 7.11000 | -0.03444 | -0.00689 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 1.53819 → 0.00000 | -1.53819 | -0.30764 |
| cmsy10 0x00 | U+002D `-` | ASCII math | 2615 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 2.70000 | -3.13334 | -0.40286 | 0.83334 → 0.00000 | -0.83334 | -0.10714 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x01 | U+22C5 `⋅` | \cdot, \cdot (fontmath.ltx) | 2625 / latinmodern-math | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 4.44446 → 3.03000 | -1.41446 | -0.50920 | -0.55554 → 0.00000 | +0.55554 | +0.20000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x02 | U+00D7 `×` | \times, \times (fontmath.ltx) | 2638 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 4.93000 | -0.90334 | -0.11614 | 0.83334 → 0.00000 | -0.83334 | -0.10714 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x03 | U+2217 `∗` | \ast, \ast (fontmath.ltx) | 2655 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 4.65279 → 4.62000 | -0.03279 | -0.00656 | -0.34721 → 0.00000 | +0.34721 | +0.06944 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x04 | U+00F7 `÷` | \div, \div (fontmath.ltx) | 2623 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.04000 | -0.79334 | -0.10200 | 0.83334 → 0.04000 | -0.79334 | -0.10200 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x06 | U+00B1 `±` | \pm, \pm (fontmath.ltx) | 2619 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.83000 | -0.00334 | -0.00043 | 0.83334 → 0.84000 | +0.00666 | +0.00086 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x07 | U+2213 `∓` | \mp, \mp (fontmath.ltx) | 2620 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.84000 | +0.00666 | +0.00086 | 0.83334 → 0.83000 | -0.00334 | -0.00043 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x08 | U+2295 `⊕` | \oplus, \oplus (fontmath.ltx) | 2805 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.92000 | +0.08666 | +0.01114 | 0.83334 → 0.92000 | +0.08666 | +0.01114 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x09 | U+2296 `⊖` | \ominus, \ominus (fontmath.ltx) | 2808 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.92000 | +0.08666 | +0.01114 | 0.83334 → 0.92000 | +0.08666 | +0.01114 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x0A | U+2297 `⊗` | \otimes, \otimes (fontmath.ltx) | 2810 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.92000 | +0.08666 | +0.01114 | 0.83334 → 0.92000 | +0.08666 | +0.01114 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x0B | U+2298 `⊘` | \oslash, \oslash (fontmath.ltx) | 2809 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.92000 | +0.08666 | +0.01114 | 0.83334 → 0.92000 | +0.08666 | +0.01114 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x0C | U+2299 `⊙` | \odot, \odot (fontmath.ltx) | 2801 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.83334 → 5.92000 | +0.08666 | +0.01114 | 0.83334 → 0.92000 | +0.08666 | +0.01114 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x0D | U+25EF `◯` | \bigcirc, \bigcirc (fontmath.ltx) | 3019 / latinmodern-math | 10.00003 → 10.13000 | +0.12997 | +0.13046 | +0.01300 | 10.00003 → 10.13000 | +0.13046 | +0.01300 | 6.94445 → 7.01000 | +0.06555 | +0.00656 | 1.94445 → 2.01000 | +0.06555 | +0.00656 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x0E | U+2218 `∘` | \circ, \circ (fontmath.ltx) | 2635 / latinmodern-math | 5.00002 → 4.12000 | -0.88002 | -0.88332 | -0.17600 | 5.00002 → 4.12000 | -0.88332 | -0.17600 | 4.44446 → 4.00000 | -0.44446 | -0.08889 | -0.55554 → 0.00000 | +0.55554 | +0.11111 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x11 | U+2261 `≡` | \equiv, \equiv (fontmath.ltx) | 2825 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 4.63748 → 4.64000 | +0.00252 | +0.00032 | -0.36252 → 0.00000 | +0.36252 | +0.04661 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x12 | U+2286 `⊆` | \subseteq, \subseteq (fontmath.ltx) | 2918 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.35971 → 6.27000 | -0.08971 | -0.01153 | 1.35971 → 1.27000 | -0.08971 | -0.01153 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x13 | U+2287 `⊇` | \supseteq, \supseteq (fontmath.ltx) | 2919 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.35971 → 6.27000 | -0.08971 | -0.01153 | 1.35971 → 1.27000 | -0.08971 | -0.01153 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x14 | U+2264 `≤` | \le, \le (fontmath.ltx), \leq, \leq (fontmath.ltx) | 2862 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.35971 → 6.40000 | +0.04029 | +0.00518 | 1.35971 → 1.19000 | -0.16971 | -0.02182 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x15 | U+2265 `≥` | \ge, \ge (fontmath.ltx), \geq, \geq (fontmath.ltx) | 2863 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.35971 → 6.40000 | +0.04029 | +0.00518 | 1.35971 → 1.19000 | -0.16971 | -0.02182 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x18 | U+223C `∼` | \sim, \sim (fontmath.ltx) | 2941 / latinmodern-math | 7.77781 → 7.73000 | -0.04781 | -0.04798 | -0.00615 | 7.77781 → 7.73000 | -0.04798 | -0.00615 | 3.66875 → 3.66000 | -0.00875 | -0.00112 | -1.33125 → 0.00000 | +1.33125 | +0.17116 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x19 | U+2248 `≈` | \approx, \approx (fontmath.ltx) | 2952 / latinmodern-math | 7.77781 → 7.73000 | -0.04781 | -0.04798 | -0.00615 | 7.77781 → 7.73000 | -0.04798 | -0.00615 | 4.83122 → 4.57000 | -0.26122 | -0.03359 | -0.16878 → 0.00000 | +0.16878 | +0.02170 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x1A | U+2282 `⊂` | \subset, \subset (fontmath.ltx) | 2914 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.39098 → 5.43000 | +0.03902 | +0.00502 | 0.39098 → 0.43000 | +0.03902 | +0.00502 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x1B | U+2283 `⊃` | \supset, \supset (fontmath.ltx) | 2915 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 5.39098 → 5.43000 | +0.03902 | +0.00502 | 0.39098 → 0.43000 | +0.03902 | +0.00502 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x1C | U+226A `≪` | \ll (fontmath.ltx) | 2886 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 5.39098 → 11.46000 | +6.06902 | +0.60690 | 0.39098 → 0.00000 | -0.39098 | -0.03910 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x1D | U+226B `≫` | \gg (fontmath.ltx) | 2887 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 5.39098 → 12.37000 | +6.97902 | +0.69790 | 0.39098 → 0.00000 | -0.39098 | -0.03910 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x20 | U+2190 `←` | \gets, \gets (fontmath.ltx), \leftarrow, \leftarrow (fontmath.ltx) | 1857 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 3.66875 → 5.10000 | +1.43125 | +0.14312 | -1.33125 → 0.10000 | +1.43125 | +0.14312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x21 | U+2192 `→` | \rightarrow, \rightarrow (fontmath.ltx), \to, \to (fontmath.ltx) | 1858 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 3.66875 → 5.10000 | +1.43125 | +0.14312 | -1.33125 → 0.10000 | +1.43125 | +0.14312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x22 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | 1867 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.94445 → 6.79000 | -0.15445 | -0.03089 | 1.94443 → 2.03000 | +0.08557 | +0.01711 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x23 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | 1868 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 6.94445 → 7.03000 | +0.08555 | +0.01711 | 1.94443 → 1.79000 | -0.15443 | -0.03089 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x24 | U+2194 `↔` | \leftrightarrow, \leftrightarrow (fontmath.ltx) | 1889 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 3.66875 → 5.10000 | +1.43125 | +0.14312 | -1.33125 → 0.10000 | +1.43125 | +0.14312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x27 | U+2243 `≃` | \simeq (fontmath.ltx) | 2946 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 4.63748 → 4.68000 | +0.04252 | +0.00547 | -0.36252 → 0.00000 | +0.36252 | +0.04661 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x28 | U+21D0 `⇐` | \Leftarrow, \Leftarrow (fontmath.ltx) | 2099 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 3.66875 → 5.20000 | +1.53125 | +0.15312 | -1.33125 → 0.20000 | +1.53125 | +0.15312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x29 | U+21D2 `⇒` | \Rightarrow, \Rightarrow (fontmath.ltx) | 2100 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 3.66875 → 5.20000 | +1.53125 | +0.15312 | -1.33125 → 0.20000 | +1.53125 | +0.15312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x2A | U+21D1 `⇑` | \Uparrow (fontmath.ltx) | 2109 / latinmodern-math | 6.11113 → 6.11000 | -0.00113 | -0.00113 | -0.00018 | 6.11113 → 6.11000 | -0.00113 | -0.00018 | 6.94445 → 6.76000 | -0.18445 | -0.03018 | 1.94443 → 2.03000 | +0.08557 | +0.01400 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x2B | U+21D3 `⇓` | \Downarrow (fontmath.ltx) | 2110 / latinmodern-math | 6.11113 → 6.11000 | -0.00113 | -0.00113 | -0.00018 | 6.11113 → 6.11000 | -0.00113 | -0.00018 | 6.94445 → 7.03000 | +0.08555 | +0.01400 | 1.94443 → 1.76000 | -0.18443 | -0.03018 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x2C | U+21D4 `⇔` | \Leftrightarrow, \Leftrightarrow (fontmath.ltx) | 2119 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 3.66875 → 5.20000 | +1.53125 | +0.15312 | -1.33125 → 0.20000 | +1.53125 | +0.15312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x2F | U+221D `∝` | \propto (fontmath.ltx) | 209 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 4.30555 → 4.42000 | +0.11445 | +0.01471 | 0.00000 → 0.11000 | +0.11000 | +0.01414 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x30 | U+2032 `′` | \prime, \prime (fontmath.ltx) | 2981 / latinmodern-math | 2.75000 → 4.07000 | +1.32000 | +1.32495 | +0.48000 | 2.75000 → 4.07000 | +1.32495 | +0.48000 | 5.55557 → 5.49000 | -0.06557 | -0.02384 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x31 | U+221E `∞` | \infty, \infty (fontmath.ltx) | 152 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 4.30555 → 4.42000 | +0.11445 | +0.01144 | 0.00000 → 0.11000 | +0.11000 | +0.01100 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x32 | U+2208 `∈` | \in, \in (fontmath.ltx) | 2926 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.39098 → 5.43000 | +0.03902 | +0.00585 | 0.39098 → 0.43000 | +0.03902 | +0.00585 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x33 | U+220B `∋` | \ni, \ni (fontmath.ltx) | 2927 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.39098 → 5.43000 | +0.03902 | +0.00585 | 0.39098 → 0.43000 | +0.03902 | +0.00585 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x34 | U+25B3 `△` | \triangle (fontmath.ltx) | 3009 / latinmodern-math | 8.88891 → 9.68000 | +0.79109 | +0.79405 | +0.08900 | 8.88891 → 9.68000 | +0.79405 | +0.08900 | 6.94445 → 7.41000 | +0.46555 | +0.05237 | 1.94445 → 0.05000 | -1.89445 | -0.21312 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x35 | U+25BD `▽` | \bigtriangledown (fontmath.ltx) | 3013 / latinmodern-math | 8.88891 → 9.68000 | +0.79109 | +0.79405 | +0.08900 | 8.88891 → 9.68000 | +0.79405 | +0.08900 | 6.94445 → 5.05000 | -1.89445 | -0.21312 | 1.94445 → 2.41000 | +0.46555 | +0.05237 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x38 | U+2200 `∀` | \forall, \forall (fontmath.ltx) | 2782 / latinmodern-math | 5.55557 → 6.66000 | +1.10443 | +1.10857 | +0.19880 | 5.55557 → 6.66000 | +1.10857 | +0.19880 | 6.94445 → 6.96000 | +0.01555 | +0.00280 | 0.00000 → 0.02000 | +0.02000 | +0.00360 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x39 | U+2203 `∃` | \exists, \exists (fontmath.ltx) | 2784 / latinmodern-math | 5.55557 → 5.56000 | +0.00443 | +0.00444 | +0.00080 | 5.55557 → 5.56000 | +0.00444 | +0.00080 | 6.94445 → 6.84000 | -0.10445 | -0.01880 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x3A | U+00AC `¬` | \lnot (fontmath.ltx), \neg (fontmath.ltx) | 2734 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 4.30555 → 3.67000 | -0.63555 | -0.09533 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x3B | U+2205 `∅` | \emptyset, \emptyset (fontmath.ltx), \varnothing | 2786 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.72000 | +0.22000 | +0.04400 | 0.55555 → 0.78000 | +0.22445 | +0.04489 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x3C | U+211C `ℜ` | \Re (fontmath.ltx) | 3725 / latinmodern-math | 7.22224 → 8.28000 | +1.05776 | +1.06172 | +0.14646 | 7.22224 → 8.54000 | +1.32270 | +0.18246 | 6.94445 → 6.86000 | -0.08445 | -0.01169 | 0.00000 → 0.27000 | +0.27000 | +0.03738 | 0.00000 → 0.26000 | +0.26000 | +0.03600 |
| cmsy10 0x3D | U+2111 `ℑ` | \Im (fontmath.ltx) | 3716 / latinmodern-math | 7.22224 → 5.54000 | -1.68224 | -1.68855 | -0.23293 | 7.22224 → 5.61000 | -1.61829 | -0.22323 | 6.94445 → 6.86000 | -0.08445 | -0.01169 | 0.00000 → 0.27000 | +0.27000 | +0.03738 | 0.00000 → 0.07000 | +0.07000 | +0.00969 |
| cmsy10 0x3E | U+22A4 `⊤` | \top (fontmath.ltx) | 2744 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 6.64000 | -0.30445 | -0.03914 | 0.00000 → 0.20000 | +0.20000 | +0.02571 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x3F | U+22A5 `⊥` | \perp (fontmath.ltx) | 2741 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 6.84000 | -0.10445 | -0.01343 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x40 | U+2135 `ℵ` | \aleph (fontmath.ltx) | 3114 / latinmodern-math | 6.11113 → 6.11000 | -0.00113 | -0.00113 | -0.00018 | 6.11113 → 6.11000 | -0.00113 | -0.00018 | 6.94445 → 6.93000 | -0.01445 | -0.00236 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x5B | U+222A `∪` | \cup, \cup (fontmath.ltx) | 2764 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.55557 → 6.04000 | +0.48443 | +0.07266 | 0.00000 → 0.20000 | +0.20000 | +0.03000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x5C | U+2229 `∩` | \cap, \cap (fontmath.ltx) | 2763 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.55557 → 6.04000 | +0.48443 | +0.07266 | 0.00000 → 0.20000 | +0.20000 | +0.03000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x5E | U+2227 `∧` | \land, \land (fontmath.ltx), \wedge, \wedge (fontmath.ltx) | 2769 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.55557 → 11.95000 | +6.39443 | +0.95916 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x5F | U+2228 `∨` | \lor, \lor (fontmath.ltx), \vee, \vee (fontmath.ltx) | 2770 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.55557 → 6.37000 | +0.81443 | +0.12216 | 0.00000 → 0.01000 | +0.01000 | +0.00150 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x60 | U+22A2 `⊢` | \vdash (fontmath.ltx) | 2747 / latinmodern-math | 6.11113 → 6.11000 | -0.00113 | -0.00113 | -0.00018 | 6.11113 → 6.11000 | -0.00113 | -0.00018 | 6.94445 → 6.84000 | -0.10445 | -0.01709 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x61 | U+22A3 `⊣` | \dashv (fontmath.ltx) | 2749 / latinmodern-math | 6.11113 → 6.11000 | -0.00113 | -0.00113 | -0.00018 | 6.11113 → 6.11000 | -0.00113 | -0.00018 | 6.94445 → 6.84000 | -0.10445 | -0.01709 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x62 | U+230A `⌊` | \lfloor, \lfloor (fontmath.ltx) | 2355 / latinmodern-math | 4.44446 → 4.44000 | -0.00446 | -0.00447 | -0.00100 | 4.44446 → 4.44000 | -0.00447 | -0.00100 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x63 | U+230B `⌋` | \rfloor, \rfloor (fontmath.ltx) | 2356 / latinmodern-math | 4.44446 → 4.44000 | -0.00446 | -0.00447 | -0.00100 | 4.44446 → 4.44000 | -0.00447 | -0.00100 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x64 | U+2308 `⌈` | \lceil, \lceil (fontmath.ltx) | 2353 / latinmodern-math | 4.44446 → 4.44000 | -0.00446 | -0.00447 | -0.00100 | 4.44446 → 4.44000 | -0.00447 | -0.00100 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x65 | U+2309 `⌉` | \rceil, \rceil (fontmath.ltx) | 2354 / latinmodern-math | 4.44446 → 4.44000 | -0.00446 | -0.00447 | -0.00100 | 4.44446 → 4.44000 | -0.00447 | -0.00100 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x66 | U+007B `{` | \lbrace, \lbrace (fontmath.ltx) | 92 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x67 | U+007D `}` | \rbrace, \rbrace (fontmath.ltx) | 94 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x68 | U+27E8 `⟨` | \langle, \langle (fontmath.ltx) | 2579 / latinmodern-math | 3.88890 → 3.89000 | +0.00110 | +0.00110 | +0.00028 | 3.88890 → 3.89000 | +0.00110 | +0.00028 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x69 | U+27E9 `⟩` | \rangle, \rangle (fontmath.ltx) | 2580 / latinmodern-math | 3.88890 → 3.89000 | +0.00110 | +0.00110 | +0.00028 | 3.88890 → 3.89000 | +0.00110 | +0.00028 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x6A | U+007C `|` | ASCII math, \vert | 93 / latinmodern-math | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x6A | U+2223 `∣` | \lvert, \mid, \mid (fontmath.ltx), \rvert | 2670 / latinmodern-math | 2.77779 → 2.78000 | +0.00221 | +0.00222 | +0.00080 | 2.77779 → 2.78000 | +0.00222 | +0.00080 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x6B | U+2016 `‖` | \Vert, \lVert, \rVert | 2727 / latinmodern-math | 5.00002 → 3.98000 | -1.02002 | -1.02384 | -0.20400 | 5.00002 → 3.98000 | -1.02384 | -0.20400 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x6B | U+2225 `∥` | \parallel, \parallel (fontmath.ltx) | 2671 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x6E | U+005C `\` | ASCII math, \backslash, \backslash (fontmath.ltx) | 61 / latinmodern-math | 5.00002 → 5.00000 | -0.00002 | -0.00002 | -0.00000 | 5.00002 → 5.00000 | -0.00002 | -0.00000 | 7.50000 → 7.50000 | +0.00000 | +0.00000 | 2.50000 → 2.50000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x6E | U+2216 `∖` | \setminus, \setminus (fontmath.ltx) | 2669 / latinmodern-math | 5.00002 → 5.68000 | +0.67998 | +0.68253 | +0.13600 | 5.00002 → 5.68000 | +0.68253 | +0.13600 | 7.50000 → 11.98000 | +4.48000 | +0.89600 | 2.50000 → 0.00000 | -2.50000 | -0.50000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x70 | U+221A `√` | fontmath.ltx radical | 3077 / latinmodern-math | 8.33336 → 8.33000 | -0.00336 | -0.00337 | -0.00040 | 8.33336 → 8.33000 | -0.00337 | -0.00040 | 0.39999 → 0.40000 | +0.00001 | +0.00000 | 9.60001 → 9.60000 | -0.00001 | -0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x72 | U+2207 `∇` | \nabla, \nabla (fontmath.ltx) | 4097 / latinmodern-math | 8.33336 → 8.33000 | -0.00336 | -0.00337 | -0.00040 | 8.33336 → 8.33000 | -0.00337 | -0.00040 | 6.83332 → 6.83000 | -0.00332 | -0.00040 | 0.00000 → 0.33000 | +0.33000 | +0.03960 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x74 | U+2294 `⊔` | \sqcup, \sqcup (fontmath.ltx) | 2789 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.55557 → 6.04000 | +0.48443 | +0.07266 | 0.00000 → 0.20000 | +0.20000 | +0.03000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x75 | U+2293 `⊓` | \sqcap, \sqcap (fontmath.ltx) | 2788 / latinmodern-math | 6.66669 → 6.67000 | +0.00331 | +0.00332 | +0.00050 | 6.66669 → 6.67000 | +0.00332 | +0.00050 | 5.55557 → 6.04000 | +0.48443 | +0.07266 | 0.00000 → 0.20000 | +0.20000 | +0.03000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x76 | U+2291 `⊑` | \sqsubseteq, \sqsubseteq (fontmath.ltx) | 2932 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.35971 → 6.27000 | -0.08971 | -0.01153 | 1.35971 → 1.27000 | -0.08971 | -0.01153 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x77 | U+2292 `⊒` | \sqsupseteq, \sqsupseteq (fontmath.ltx) | 2933 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.35971 → 6.27000 | -0.08971 | -0.01153 | 1.35971 → 1.27000 | -0.08971 | -0.01153 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x7C | U+2663 `♣` | \clubsuit (fontmath.ltx) | 3040 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 7.27000 | +0.32555 | +0.04186 | 1.29629 → 1.30000 | +0.00371 | +0.00048 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x7D | U+2662 `♢` | \diamondsuit (fontmath.ltx) | 3037 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 7.27000 | +0.32555 | +0.04186 | 1.29629 → 1.63000 | +0.33371 | +0.04291 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x7E | U+2661 `♡` | \heartsuit (fontmath.ltx) | 3035 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 7.16000 | +0.21555 | +0.02771 | 1.29629 → 0.33000 | -0.96629 | -0.12424 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmsy10 0x7F | U+2660 `♠` | \spadesuit (fontmath.ltx) | 3034 / latinmodern-math | 7.77781 → 7.78000 | +0.00219 | +0.00220 | +0.00028 | 7.77781 → 7.78000 | +0.00220 | +0.00028 | 6.94445 → 7.27000 | +0.32555 | +0.04186 | 1.29629 → 1.30000 | +0.00371 | +0.00048 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x00 | U+0028 `(` | ASCII math | 2389 / latinmodern-math | 4.58336 → 4.58000 | -0.00336 | -0.00337 | -0.00073 | 4.58336 → 4.58000 | -0.00337 | -0.00073 | 0.39999 → 8.47000 | +8.07001 | +1.76072 | 11.60013 → 3.47000 | -8.13013 | -1.77384 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x01 | U+0029 `)` | ASCII math | 2390 / latinmodern-math | 4.58336 → 4.58000 | -0.00336 | -0.00337 | -0.00073 | 4.58336 → 4.58000 | -0.00337 | -0.00073 | 0.39999 → 8.47000 | +8.07001 | +1.76072 | 11.60013 → 3.47000 | -8.13013 | -1.77384 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x02 | U+005B `[` | ASCII math | 2395 / latinmodern-math | 4.16669 → 4.17000 | +0.00331 | +0.00332 | +0.00079 | 4.16669 → 4.17000 | +0.00332 | +0.00079 | 0.39999 → 8.50000 | +8.10001 | +1.94399 | 11.60013 → 3.50000 | -8.10013 | -1.94402 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x03 | U+005D `]` | ASCII math | 2396 / latinmodern-math | 4.16669 → 4.17000 | +0.00331 | +0.00332 | +0.00079 | 4.16669 → 4.17000 | +0.00332 | +0.00079 | 0.39999 → 8.50000 | +8.10001 | +1.94399 | 11.60013 → 3.50000 | -8.10013 | -1.94402 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x04 | U+230A `⌊` | \lfloor, \lfloor (fontmath.ltx) | 2399 / latinmodern-math | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 0.39999 → 8.50000 | +8.10001 | +1.71529 | 11.60013 → 3.50000 | -8.10013 | -1.71531 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x05 | U+230B `⌋` | \rfloor, \rfloor (fontmath.ltx) | 2400 / latinmodern-math | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 0.39999 → 8.50000 | +8.10001 | +1.71529 | 11.60013 → 3.50000 | -8.10013 | -1.71531 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x06 | U+2308 `⌈` | \lceil, \lceil (fontmath.ltx) | 2397 / latinmodern-math | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 0.39999 → 8.50000 | +8.10001 | +1.71529 | 11.60013 → 3.50000 | -8.10013 | -1.71531 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x07 | U+2309 `⌉` | \rceil, \rceil (fontmath.ltx) | 2398 / latinmodern-math | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 0.39999 → 8.50000 | +8.10001 | +1.71529 | 11.60013 → 3.50000 | -8.10013 | -1.71531 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x08 | U+007B `{` | \lbrace, \lbrace (fontmath.ltx) | 2393 / latinmodern-math | 5.83336 → 5.83000 | -0.00336 | -0.00337 | -0.00058 | 5.83336 → 5.83000 | -0.00337 | -0.00058 | 0.39999 → 8.50000 | +8.10001 | +1.38857 | 11.60013 → 3.50000 | -8.10013 | -1.38859 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x09 | U+007D `}` | \rbrace, \rbrace (fontmath.ltx) | 2394 / latinmodern-math | 5.83336 → 5.83000 | -0.00336 | -0.00337 | -0.00058 | 5.83336 → 5.83000 | -0.00337 | -0.00058 | 0.39999 → 8.50000 | +8.10001 | +1.38857 | 11.60013 → 3.50000 | -8.10013 | -1.38859 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0A | U+27E8 `⟨` | \langle, \langle (fontmath.ltx) | 2587 / latinmodern-math | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 0.39999 → 8.50000 | +8.10001 | +1.71529 | 11.60013 → 3.50000 | -8.10013 | -1.71531 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0B | U+27E9 `⟩` | \rangle, \rangle (fontmath.ltx) | 2588 / latinmodern-math | 4.72224 → 4.72000 | -0.00224 | -0.00225 | -0.00047 | 4.72224 → 4.72000 | -0.00225 | -0.00047 | 0.39999 → 8.50000 | +8.10001 | +1.71529 | 11.60013 → 3.50000 | -8.10013 | -1.71531 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0C | U+007C `|` | ASCII math, \vert | 2722 / latinmodern-math | 3.33334 → 2.78000 | -0.55334 | -0.55541 | -0.16600 | 3.33334 → 2.78000 | -0.55541 | -0.16600 | 0.00000 → 12.02000 | +12.02000 | +3.60599 | 6.00006 → 0.00000 | -6.00006 | -1.80001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0C | U+2223 `∣` | \lvert, \mid, \rvert | 2722 / latinmodern-math | 3.33334 → 2.78000 | -0.55334 | -0.55541 | -0.16600 | 3.33334 → 2.78000 | -0.55541 | -0.16600 | 0.00000 → 12.02000 | +12.02000 | +3.60599 | 6.00006 → 0.00000 | -6.00006 | -1.80001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0D | U+2016 `‖` | \Vert, \lVert, \rVert | 2725 / latinmodern-math | 5.55557 → 4.24000 | -1.31557 | -1.32051 | -0.23680 | 5.55557 → 4.24000 | -1.32051 | -0.23680 | 0.00000 → 12.02000 | +12.02000 | +2.16359 | 6.00006 → 0.00000 | -6.00006 | -1.08001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0D | U+2225 `∥` | \parallel | 2725 / latinmodern-math | 5.55557 → 4.24000 | -1.31557 | -1.32051 | -0.23680 | 5.55557 → 4.24000 | -1.32051 | -0.23680 | 0.00000 → 12.02000 | +12.02000 | +2.16359 | 6.00006 → 0.00000 | -6.00006 | -1.08001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0E | U+002F `/` | ASCII math | 2672 / latinmodern-math | 5.77779 → 6.17000 | +0.39221 | +0.39368 | +0.06788 | 5.77779 → 6.17000 | +0.39368 | +0.06788 | 0.39999 → 9.05000 | +8.65001 | +1.49711 | 11.60013 → 4.05000 | -7.55013 | -1.30675 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x0F | U+005C `\` | ASCII math, \backslash, \backslash (fontmath.ltx) | 2673 / latinmodern-math | 5.77779 → 6.17000 | +0.39221 | +0.39368 | +0.06788 | 5.77779 → 6.17000 | +0.39368 | +0.06788 | 0.39999 → 9.05000 | +8.65001 | +1.49711 | 11.60013 → 4.05000 | -7.55013 | -1.30675 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x48 | U+222E `∮` | \oint | 3053 / latinmodern-math | 4.72223 → 6.65000 | +1.92777 | +1.93500 | +0.40823 | 6.66669 → 9.97000 | +3.31570 | +0.69952 | 0.00000 → 8.05000 | +8.05000 | +1.70470 | 11.11122 → 3.06000 | -8.05122 | -1.70496 | 1.94446 → 3.32000 | +1.37554 | +0.29129 |
| cmex10 0x50 | U+2211 `∑` | \sum, \sum (fontmath.ltx) | 3060 / latinmodern-math | 10.55559 → 10.56000 | +0.00441 | +0.00442 | +0.00042 | 10.55559 → 10.56000 | +0.00442 | +0.00042 | 0.00000 → 7.50000 | +7.50000 | +0.71052 | 10.00013 → 2.50000 | -7.50013 | -0.71054 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x51 | U+220F `∏` | \prod, \prod (fontmath.ltx) | 3061 / latinmodern-math | 9.44448 → 9.44000 | -0.00448 | -0.00449 | -0.00047 | 9.44448 → 9.44000 | -0.00449 | -0.00047 | 0.00000 → 7.50000 | +7.50000 | +0.79412 | 10.00013 → 2.50000 | -7.50013 | -0.79413 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x52 | U+222B `∫` | \int | 3049 / latinmodern-math | 4.72223 → 6.65000 | +1.92777 | +1.93500 | +0.40823 | 6.66669 → 9.97000 | +3.31570 | +0.69952 | 0.00000 → 8.05000 | +8.05000 | +1.70470 | 11.11122 → 3.06000 | -8.05122 | -1.70496 | 1.94446 → 3.32000 | +1.37554 | +0.29129 |
| cmex10 0x62 | U+0302 COMBINING CIRCUMFLEX | \widehat (fontmath.ltx) | 2280 / latinmodern-math | 5.55557 → 6.44000 | +0.88443 | +0.88774 | +0.15920 | 5.55557 → 6.44000 | +0.88774 | +0.15920 | 7.22223 → 7.46000 | +0.23777 | +0.04280 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x65 | U+0303 COMBINING TILDE | \widetilde (fontmath.ltx) | 2282 / latinmodern-math | 5.55557 → 6.52000 | +0.96443 | +0.96804 | +0.17360 | 5.55557 → 6.52000 | +0.96804 | +0.17360 | 7.22223 → 7.51000 | +0.28777 | +0.05180 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x70 | U+221A `√` | fontmath.ltx radical | 3081 / latinmodern-math | 10.00003 → 10.00000 | -0.00003 | -0.00003 | -0.00000 | 10.00003 → 10.00000 | -0.00003 | -0.00000 | 0.39999 → 8.50000 | +8.10001 | +0.81000 | 11.60013 → 3.50000 | -8.10013 | -0.81001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x78 | U+2191 `↑` | \uparrow, \uparrow (fontmath.ltx) | 1873 / latinmodern-math | 6.66669 → 5.00000 | -1.66669 | -1.67294 | -0.25000 | 6.66669 → 5.00000 | -1.67294 | -0.25000 | 0.00000 → 5.05000 | +5.05000 | +0.75750 | 6.00006 → 0.00000 | -6.00006 | -0.90001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x79 | U+2193 `↓` | \downarrow, \downarrow (fontmath.ltx) | 1874 / latinmodern-math | 6.66669 → 5.00000 | -1.66669 | -1.67294 | -0.25000 | 6.66669 → 5.00000 | -1.67294 | -0.25000 | 0.00000 → 5.05000 | +5.05000 | +0.75750 | 6.00006 → 0.00000 | -6.00006 | -0.90001 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x7A | U+23DE `⏞` | private cmex brace piece | 2547 / latinmodern-math | 4.50005 → 10.02000 | +5.51995 | +5.54065 | +1.22664 | 4.50005 → 10.02000 | +5.54065 | +1.22664 | 1.19998 → 7.24000 | +6.04002 | +1.34221 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x7B | U+23DE `⏞` | private cmex brace piece | 2550 / latinmodern-math | 4.50005 → 10.01000 | +5.50995 | +5.53062 | +1.22442 | 4.50005 → 10.01000 | +5.53062 | +1.22442 | 1.19998 → 7.24000 | +6.04002 | +1.34221 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x7B | U+23DF `⏟` | private cmex brace piece | 2553 / latinmodern-math | 4.50005 → 20.03000 | +15.52995 | +15.58819 | +3.45107 | 4.50005 → 20.03000 | +15.58819 | +3.45107 | 1.19998 → 0.00000 | -1.19998 | -0.26666 | 0.00000 → 4.23000 | +4.23000 | +0.93999 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x7C | U+23DF `⏟` | private cmex brace piece | 2551 / latinmodern-math | 4.50005 → 10.02000 | +5.51995 | +5.54065 | +1.22664 | 4.50005 → 10.02000 | +5.54065 | +1.22664 | 1.19998 → 0.00000 | -1.19998 | -0.26666 | 0.00000 → 2.94000 | +2.94000 | +0.65333 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x7D | U+23DE `⏞` | private cmex brace piece | 2549 / latinmodern-math | 4.50005 → 20.03000 | +15.52995 | +15.58819 | +3.45107 | 4.50005 → 20.03000 | +15.58819 | +3.45107 | 1.19998 → 8.54000 | +7.34002 | +1.63110 | 0.00000 → 0.00000 | +0.00000 | +0.00000 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |
| cmex10 0x7D | U+23DF `⏟` | private cmex brace piece | 2554 / latinmodern-math | 4.50005 → 10.01000 | +5.50995 | +5.53062 | +1.22442 | 4.50005 → 10.01000 | +5.53062 | +1.22442 | 1.19998 → 0.00000 | -1.19998 | -0.26666 | 0.00000 → 2.94000 | +2.94000 | +0.65333 | 0.00000 → 0.00000 | +0.00000 | +0.00000 |

## Unmeasured or out-of-scope rows

These are not estimates. They are listed because a source or mapping needed for a four-family comparison was absent, or because the engine uses a composite/secondary-face path. No number was substituted.

| subject | reason |
| --- | --- |
| \Longleftarrow ⟸ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \Longleftrightarrow ⟺ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \Longrightarrow ⟹ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \angle ∠ | fontmath.ltx defines a constructed box, not one CM TFM glyph |
| \blacksquare ■ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \checkmark ✓ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \coloneqq ≔ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \complement ∁ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \cong ≅ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \ddots ⋱ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \gtrsim ≳ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \hbar ℏ | fontmath.ltx defines a composite overprint, not one CM TFM glyph |
| \hookrightarrow ↪ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \leftrightarrows ⇆ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \lesssim ≲ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \longleftarrow ⟵ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \longleftrightarrow ⟷ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \longrightarrow ⟶ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \lozenge ◊ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \mapsto ↦ | plain TeX composes a zero-width mapstochar with an arrow; no single four-family CM glyph is declared |
| \measuredangle ∡ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \models ⊨ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \neq ≠ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \nexists ∄ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \ngeq ≱ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \nleq ≰ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \nmid ∤ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \not U+0338 | the engine uses a zero-width overprint before the following relation |
| \notin ∉ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \rightsquigarrow ⇝ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \square □ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \subsetneq ⊊ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \supsetneq ⊋ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \therefore ∴ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \triangleq ≜ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| \varnothing ∅ | the engine gives this command the msbm/New Computer Modern Math sentinel path, not one of the four CM families |
| \vdots ⋮ | no comparable declaration in cmr10/cmmi10/cmsy10/cmex10 was found; the current engine uses an OTF fallback or a composite path |
| cmex10 0x0F U+2216 `∖` | the existing engine slot-to-Latin-Modern mapping returned no glyph |
| cmex10 0x7A U+23DF `⏟` | the existing engine maps this brace assembly position to its empty placeholder (gid 0), not a font glyph |
| cmex10 0x7C U+23DE `⏞` | the existing engine maps this brace assembly position to its empty placeholder (gid 0), not a font glyph |
| cmex10 0x7E U+21D1 `⇑` | the existing engine slot-to-Latin-Modern mapping returned no glyph |
| cmex10 0x7F U+21D3 `⇓` | the existing engine slot-to-Latin-Modern mapping returned no glyph |

