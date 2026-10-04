# XeTeX port: plan and phase S0 results

Owner request (2026-10-04): full custom font support -- any system font,
OpenType, and OpenType math. The route, from the feasibility spike of the same
day, is a port of TeX Live's XeTeX the way the pdfTeX engine was ported
(`docs/design/engine-v2/DESIGN.md` D1, D3, §4.1): `tools/web2rust` translates
the unmodified `xetex.web` with WEB change files, and XeTeX's C/C++ layer is
ported or linked behind the interface those change files declare. The preview
carries the result as `FONT.format: "opentype"` (DESIGN.md §15.4, E1).
DESIGN.md governs; this plan does not change any of its decisions.

Lane: XETEX-S0 (mac-claude-a). Crate: `crates/flashtex-xetex`. Branch
`agent/mac-claude-a/xetex-s0`.

## 1. What the spike found (2026-10-04)

- `xetex.web` is 34,428 lines (TeX Live 2026, XeTeX 0.999998, e-TeX 2.6).
- `web2rust` tangled it and stopped at §744 (a local `const` section).
- 84% of the pdfTeX engine's change-file hunks match `xetex.web` as written.
- The C/C++ layer is about 10.7k lines, over HarfBuzz 12.3.2, FreeType
  2.14.1, ICU, TECkit and Graphite2.
- Risks: glyph metrics through FreeType; the shaping version lock (output
  depends on the exact HarfBuzz/FreeType); C pointers kept in `mem` (native
  word nodes' glyph arrays, `font_layout_engine`); XDV output.

## 2. Architecture and licensing

- `crates/flashtex-xetex` (GPL-2.0-or-later, its own `LICENSE` with XeTeX's
  MIT notice): `src/generated/` (74,253 lines, never edited) is web2rust's
  translation of `third_party/xetex/xetex.web` (unmodified, pinned at
  `tags/texlive-2026.1`, `third_party/xetex/README.md`) with the change files
  of `changes/` (3,136 lines, `changes/README.md`).
- It **shares the pdfTeX engine's runtime through its public interface**,
  changing nothing in `crates/flashtex-engine`: the word space (`arena`), the
  checked array index (`ix`), the command-line parser (`cli`), the run's
  configuration and kpathsea (`system::configure`, `find_input`,
  `find_file`, `runsystem`), dates and MD5. XeTeX's own system layer is
  `src/system.rs` (Pascal's file model, XeTeX's Unicode input files and
  `input_line`, the routines tex.ch and xetex.ch supply) and
  `src/xetex_ext.rs` (XeTeX's C parts; MIT code, ported).
- It is **excluded from the root workspace** (`exclude` in `Cargo.toml`), with
  its own `Cargo.lock`, so its build never slows the workspace's builds and
  gates; `scripts/gate.sh` treats it as a standalone crate (clippy and tests
  only when its files change). Its translation is still checked on every run
  of web2rust's tests (`tests/drift.rs`, second test).
- DESIGN.md §3: change files only on `xetex.web` (its e-TeX notice forbids
  modified copies); XeTeX's MIT code may be ported; nothing MIT links the
  crate. `scripts/license-boundary-allow.txt` lists `flashtex-xetex` with the
  reason (a GPL executable and workspace of its own); the boundary check is
  clean. HarfBuzz (MIT), FreeType (FTL or GPL-2), ICU (Unicode licence),
  TECkit (LGPL-2.1 or CPL) and Graphite2 (LGPL-2.1 or MPL) are all usable on
  the GPL side of §3 when S1 links them.

## 3. Phases and acceptance gates

| Phase | Scope | Acceptance gate |
|---|---|---|
| **S0** (this lane) | web2rust translates `xetex.web` into `crates/flashtex-xetex`; TFM fonts only; native fonts, TECkit, ICU, Graphite and pictures stubbed as "not found"; XDV for TFM text | P-T1 (box dumps + `\tracingall` log) and exit status equal to TeX Live 2026's `xetex -no-pdf` on the plain-TeX lockstep cases that need no pdfTeX primitive plus XeTeX-specific ones (target about 200); XeTeX's own tests (`xetex-*.test`) pass; XDV byte-identical after the two normalisations; pdftex.web's translation byte-identical (drift test) |
| **S1** | Native fonts: font discovery (CoreText on macOS, fontconfig elsewhere, `[file]` names through kpathsea), FreeType metrics and HarfBuzz shaping vendored at TeX Live's exact versions (HarfBuzz 12.3.2, FreeType 2.14.1, sha256-pinned like `third_party/zlib`), native word and glyph nodes with glyph-info arrays as handles (S0's model), `define_native_font` and glyph arrays in XDV, TECkit mappings (`tex-text`), ICU encodings and line breaking, input normalisation | P-T1 + XDV equal (the native font's path normalised) on a native-font corpus: plain-TeX cases with Latin Modern/TeX Gyre OTF and macOS system fonts, then `xelatex` documents with `fontspec`; the version lock checked by a test that fails on a library upgrade |
| **S2** | OpenType math (`XeTeXOTMath.cpp`: the MATH table, variants, assemblies, kerns, `\Umath...` with OT fonts), pictures (`XeTeX_pic.c`: PNG, JPEG, BMP, PDF bounds), Graphite2 (and AAT on macOS, if the owner wants AAT-only fonts) | P-T1 + XDV equal on `unicode-math` documents (amsmath + unicode-math with Latin Modern Math, STIX Two, Libertinus Math) and on `\XeTeXpicfile`/`graphicx` documents |
| **S3** | Product integration: the `xelatex` format from the user's TeX Live (as for pdflatex, DESIGN.md §4.4), the resident host and incremental system (§5) for XeTeX documents, the preview through display-list-v3 with `FONT.format: "opentype"` (§15.4 E1), PDF through the PDF backend (an xdvipdfmx-equivalent path), the XeTeX choice in the app | The fontspec/unicode-math corpus at P-T1 in the host; P-T2 (fonts and content streams, §1.1) against `xelatex`; the latency targets of §1.2 for XeTeX documents |

## 4. Phase S0 results (2026-10-04)

All numbers below are **verified** (measured on mac-m1max-a, MacTeX 2026 as
the oracle) unless marked otherwise.

### 4.1 What compiles

- `web2rust` translates the whole of `xetex.web` with the 17 change files:
  1,721 sections, 203,498 tokens, 1,423 pool strings, 449 routines into
  `src/generated/` (`scripts/xetex-regenerate.sh`). The crate builds in
  release and debug, clippy-clean (`-D warnings`) and rustfmt-clean.
- Native fonts, TECkit, ICU, Graphite and pictures are stubs that answer as
  TeX Live's XeTeX does when nothing is found (`src/xetex_ext.rs`); the C
  pointers `xetex.web` keeps (glyph-info arrays in `mem`, layout engines,
  mappings, OpenType assemblies, picture paths) are integer handles
  (`changes/ext.ch`), which is the model S1 fills in.

### 4.2 P-T1 and XDV against `xetex -no-pdf`

`scripts/xetex-lockstep.sh` (tools/xetex-lockstep):

| set | cases | P-T1 log + exit status equal | XDV identical after normalisation |
|---|---|---|---|
| tools/lockstep cases with no pdfTeX-only primitive (`suite.txt`, selected by `--select`: 1308 of 1464; 156 use a pdfTeX-only primitive) | 1308 | 1308 | 1308 of 1308 |
| XeTeX-specific cases (`tools/xetex-lockstep/cases`) | 53 | 53 | 53 of 53 |
| XeTeX's own tests (bug73, ctrlsym, filedump; format dump and load included) | 3 | 3 (also equal to the committed `.log`) | n/a |
| **total** | **1364** | **1364** | **1361 of 1361** |

The XDV normalisations are the two of the spike (the preamble's date comment,
a native font's path); with `SOURCE_DATE_EPOCH` pinned and no native fonts
neither changes a byte in S0. The XDV is written by `xetex.web`'s own DVI
code (S0 needed no separate writer); byte identity holds through the
preamble, `set`/`fnt_def` records, the postamble and the 223 padding.

### 4.3 web2rust changes, and pdftex.web unchanged

Each is something pdftex.web never does (`tools/web2rust/README.md`, "What
xetex.web needs beyond pdftex.web"): a local `const` section; C's bare
`break`; `addressof(x)` as a `var` argument; `--first-string 65536` (the
numbering of Omega's `otangle`, which TeX Live tangles xetex.web with); and
the unary-minus-before-`and` clash, refused like the other C/Pascal
precedence clashes. Proof that pdfTeX is unchanged: `cargo test --release -p
web2rust` passes, including `tests/drift.rs`, which regenerates
`crates/flashtex-engine/src/generated/` and `pdftex.pool` from pdftex.web and
compares them byte for byte with the committed ones.

### 4.4 Review against TeX Live's own program

The WEB the change files produce was diffed against TeX Live's merged XeTeX
WEB (`tie` of xetex.web, tex.ch0, tex.ch, the feature files, SyncTeX's,
xetex.ch, char-warning-xetex.ch and tex-binpool.ch). It found, beyond what
the lockstep cases had caught: XeTeX's one-word SyncTeX layout (box nodes 8
words, not pdfTeX's 9); web2c's identity `hi`/`ho` (XeTeX's `min_halfword` is
negative); the string-pool dump of `str_start` (only the pool strings'
starts); the empty trie's size `max_hyph_char`; and two tex.ch bug fixes,
[25.369] (`\noexpand\endwrite`) and [26.449] (`\mkern` with a non-mu
dimension). All are in the change files now.

**Finding for the pdfTeX engine (not changed by this lane):** neither tex.ch
fix [25.369] nor [26.449] is re-specified in `crates/flashtex-engine/changes/`
(pdftex.web has the pre-fix text, TeX Live's pdftex gets the fixes from
tex.ch). **Measured** for [26.449]: on the input of
`tools/xetex-lockstep/cases/x053-mkern-nonmu.tex` with tools/lockstep's
prelude and invocation, TeX Live's pdftex and the engine (main at
f2012b591) differ: the engine reports `Illegal unit of measure (mu
inserted)` and `Dimension too large` after `\mkern\dimen0`, and its box is
9127.08324pt wide where pdftex's is 26.74988pt. For [25.369] the input of
`x052-noexpand-endwrite.tex` gives equal logs, so a difference there is
still only a belief. The fix is a one-hunk change each in the engine's
`web2c.ch` (the XeTeX port's `web2c.ch` has both), plus a lockstep case.

### 4.5 Speed (measured, load-sensitive)

INITEX with every tracing switch on (the lockstep invocation), median of 11
runs: `001-edef-basic` xetex 1.10 s, port 1.31 s; `x001-utf8-text` xetex
0.99 s, port 1.65 s; `089-mathcode-class` xetex 1.56 s, port 1.70 s. INITEX
is not the product path, and S0 adds none of the engine's optimisations; the
gap is a belief to be profiled in S1 (first suspect: the terminal written a
byte at a time while `\tracingonline` copies the whole trace to it).

### 4.6 Not done in S0 (by design)

`crates/flashtex-xetex/changes/README.md`, "Not re-specified": native
fonts, TECkit, ICU encodings, pictures, the output driver (`-no-pdf` is the
only mode), tex.ch's hashed `\hyphenation` table, ML\TeX, encTeX, source
specials, SyncTeX output, `\XeTeXinputencoding` on the terminal level. None
can show in a TFM-only document that does not name them.

## 5. Risks carried into S1

| risk | mitigation |
|---|---|
| Glyph metrics through FreeType (heights, depths, italic corrections, bounding boxes) | Port `XeTeXFontInst.cpp`/`XeTeXLayoutInterface.cpp` against the vendored FreeType; compare every `\XeTeXglyphbounds`, `\fontcharht` etc. with xetex in lockstep cases |
| Shaping version lock: shaped output depends on HarfBuzz's exact version | Vendor HarfBuzz 12.3.2 and FreeType 2.14.1 as TeX Live has them (sha256 in `third_party/<lib>/README.md`); a test pins the versions; an upgrade is a lane of its own with a lockstep run |
| C pointers in `mem` | Handles (S0's model): the Rust side owns the glyph arrays; `copy_native_glyph_info`/`free_native_glyph_info` map to copy/free of a handle; checkpoints (DESIGN.md §5.2) must snapshot the handle table with the word space |
| XDV for native fonts (`define_native_font`, glyph arrays, `set_text_and_glyphs`) | `make_font_def` and `make_xdv_glyph_array_data` ported from XeTeX_ext.c, compared byte for byte with the path normalised |
| System font discovery differs by machine | Lockstep cases use TeX Live's own OpenType fonts by file name for P-T1; system fonts are tested on named macOS fonts only |
