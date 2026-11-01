# TFM fixture: `ec-lmr10` bound to Latin Modern Roman 10

One real, licensed TFM with matching font bytes and independently obtained
glyph identities, in the shape `crates/font-resources` `encoding::EncodingManifest`
(`bea66c7` on `origin/agent/commander-corpus/font-resources`) expects for
`BoundTfmFont`. Requested by the Commander (issue #2, 06:12Z).

| File | What | SHA-256 |
| --- | --- | --- |
| `ec-lmr10.tfm` | TFM, T1/EC-encoded Latin Modern Roman 10 (12 056 bytes), copied from TeX Live `texmf-dist/fonts/tfm/public/lm/ec-lmr10.tfm` | `cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5` |
| `lm-ec.enc` | Latin Modern's EC encoding vector (slot → glyph name), copied from `texmf-dist/fonts/enc/dvips/lm/lm-ec.enc` | `7f9932c402d22a937b853406cfdf4166b80260e3ff21a03fe9a4c05105a2918c` |
| `GUST-FONT-LICENSE.txt` | GUST Font License v1.0 (22 June 2009), from http://tug.org/fonts/licenses/GUST-FONT-LICENSE.txt | `49ea6cb9257bbee0a3979c48a774cd221550ac1c20c95549efe45fc99cc18050` |
| `ec-lmr10.encoding.json` | `EncodingManifest`: `tfm_sha256`, `font_sha256`, `face_index`, `encoding[]` (253 slots), `declared_glyphs[]` (254 names incl. `.notdef`) | generated |
| `ec-lmr10.metrics.json` | Reference metrics quoted below, machine-readable | generated |
| (not committed) `lmroman10-regular.otf` | The font bytes: `texmf-dist/fonts/opentype/public/lm/lmroman10-regular.otf`, OpenType CFF, face 0, 111 536 bytes | `1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852` |

Licence: TFM, encoding vector and font are all Latin Modern, GUST Font License
(redistribution permitted; the licence text is included). The `.otf` is
referenced by path and hash only (not copied) as requested. Computer Modern
(`cmr10.tfm` + Type 1 `cmr10.pfb`) is **not** included: this crate has no
Type 1 parser, so it cannot bind names → GIDs for a `.pfb` independently, and
guessing is not evidence.

## How the glyph identities were obtained (two independent readers, must agree)

1. **CFF charset of the OTF**, read with fontTools 4.53.1 (`TTFont.getGlyphOrder()`),
   which is the font's own name → original GID order. Not this crate's code.
2. **This crate's cmap parser** (`cargo run --example gid_for`), reached from
   each glyph name through the Adobe Glyph List (`uniXXXX` names decoded
   directly). 250 of the 253 declared names were reachable this way and every
   one agreed with reader 1; the generator exits non-zero on any disagreement.

Name aliasing (recorded in `ec-lmr10.metrics.json` → `charset_name_aliases`):
the encoding vector uses AFM-style ligature names, the CFF charset uses
AGL-style names for the same glyphs: `ff`→`f_f`, `fi`→`f_i`, `fl`→`f_l`,
`ffi`→`f_f_i`, `ffl`→`f_f_l`. The manifest declares them under the
encoding's names with the charset's GIDs. Three encoding names have no glyph
in this OTF (`Germandbls` slot 223, `IJ` slot 156, `ij` slot 188); those slots
are left **undeclared** so the adapter rejects them explicitly rather than
mapping them to `.notdef`.

## Reference metrics (read from the TFM by `tools/gen_tfm_fixture.py`'s own reader)

Design size: fix_word `10485760` = 10.0 pt. Checksum `0xae811a07`.
`fix_word` values are 32-bit, scaled by 2^20; `pt = fix_word / 2^20 × design size`.

| Slot | name | original GID | width fix_word | height | depth | italic | width pt | height pt | depth pt |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 65 | `A` | 27 | 786432 | 722338 | 0 | 0 | 7.5 | 6.88875 | 0 |
| 86 | `V` | 111 | 786432 | 722338 | 0 | 8155 | 7.5 | 6.88875 | 0 |
| 97 | `a` | 28 | 524288 | 451464 | 0 | 11301 | 5.0 | 4.30550 | 0 |
| 233 | `eacute` | 251 | 466040 | 722338 | 0 | 0 | 4.44450 | 6.88875 | 0 |
| 102 | `f` | 55 | 320392 | 722338 | 0 | 83070 | 3.05550 | 6.88875 | 0 |
| 105 | `i` | 66 | 291269 | 660314 | 0 | 0 | 2.77776 | 6.29725 | 0 |
| 28 | `fi` | 125 | 582536 | 722338 | 0 | 0 | 5.55550 | 6.88875 | 0 |

Ligature: slot 102 (`f`) followed by slot 105 (`i`) → lig/kern instruction
`op_byte 0, remainder 28` = plain ligature replacing both by slot 28 (`fi`).

Kern: slot 65 (`A`) followed by slot 86 (`V`) → `op_byte ≥ 128`, kern index
resolving to fix_word `-116509` = −1.11112 pt at 10 pt (−0.1111 em, the
Computer Modern value).

Font parameters (`params`, fix_word): slant 0, space 349525 (3.3333 pt),
stretch 174763, shrink 116509, x-height 451464, quad 1048576 (10 pt),
extra space 116509, … (21 parameters, listed in the metrics JSON).

Cross-check against the OTF: `A` advance in `lmroman10-regular.otf` is 750
units = 7.5 pt at 10 pt, `a` 500 = 5.0 pt, `fi` 556 ≈ 5.5555 pt, `eacute`
444 ≈ 4.4445 pt (see `tests/latin_modern.rs`); the TFM widths agree within half a
font unit (0.005 pt at 10 pt; the TFM keeps 444.5 for eacute where the OTF has 444).

## Regenerate

```sh
cd crates/font-engine
python3 tools/gen_tfm_fixture.py \
  --tfm  /usr/local/texlive/2026basic/texmf-dist/fonts/tfm/public/lm/ec-lmr10.tfm \
  --font /usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm/lmroman10-regular.otf \
  --enc  /usr/local/texlive/2026basic/texmf-dist/fonts/enc/dvips/lm/lm-ec.enc \
  --glyphlist /usr/local/texlive/2026basic/texmf-dist/fonts/map/glyphlist/glyphlist.txt \
  --license fonts/GUST-FONT-LICENSE.txt \
  --python-with-fonttools <python with fontTools installed> \
  --out fixtures/tfm --crate "$PWD"
```

Output is deterministic for fixed inputs (sorted names, fixed key order);
`tests/pinned.rs::tfm_fixture_matches_the_pinned_font` re-verifies the hashes,
re-derives every declared GID through this crate's cmap and checks the quoted
metrics against the OTF advances.
