# Rewriting the engine into idiomatic Rust

**Status:** plan, with its first step done (the pilot, §6). Lane IDIOMATIC-REWRITE,
2026-10-04. Governed by [DESIGN.md](DESIGN.md), which wins where the two differ.

**Owner decision (2026-10-04).** The owner asked for the archaic, slow and sloppy code
inherited from pdfTeX and TeX Live to be translated and rewritten into clean, idiomatic,
safe Rust. FlashTeX may therefore **stop regenerating `src/generated/` from `pdftex.web`
and evolve the Rust directly**; a later pdfTeX release is then ported by hand. The record
of the decision and of the licensing constraints that still apply (§2 here) for DESIGN.md
§3, §4.1 and §13 is on branch `agent/mac-claude-a/rewrite-design-record`, held as a PR
until the DESIGN.md PRs already open land (DESIGN.md §9.5: one open DESIGN.md PR at a time).

The bar does not move: every step keeps P-T1 and P-T2 (DESIGN.md §1.1), passes every gate
of DESIGN.md §8 that applies, and costs no speed (§5).

---

## 1. Inventory (main `f2012b591`, 2026-10-04; VERIFIED by `wc`/`grep`)

### 1.1 What there is

| Part | Where | Lines | Origin |
|---|---|---:|---|
| Generated engine | `crates/flashtex-engine/src/generated/` | 62,697 | `web2rust` from `pdftex.web` + change files; never edited (drift test) |
| — procedure bodies | `body_0.rs` … `body_7.rs`, `main_body.rs` | 57,152 | 602 functions, split by count (76 per file), not by WEB part |
| — globals, constants, types | `globals.rs`, `consts.rs`, `types.rs` | 5,420 | 996 named `@d` constants (#1302) |
| WEB change files | `crates/flashtex-engine/changes/*.ch` | 3,852 | ours; every behaviour beyond `pdftex.web` |
| pdfTeX's C parts, ported | `crates/flashtex-engine/src/pdftex/` | 15,427 | hand ports of `writet1.c`, `writettf.c`, `pdftoepdf.cc`, … (one module per C file) |
| C/C++ shims | `crates/flashtex-engine/csrc/` | 620 | libpng, xpdf and regex glue |
| Runtime | `src/*.rs` (arena, checkpoint, incr, iso, system, resolver, …) | 24,435 | hand-written Rust |
| Host, display list, bundle | `src/host/`, `src/displaylist/`, `src/bundle/` | 12,416 | hand-written Rust |
| Translator | `tools/web2rust/` | 6,042 | MIT |

The generated code by `pdftex.web` part (approximate: each function is counted under the
part of its first `§`):

| Parts | Lines | | Parts | Lines |
|---|---:|---|---|---:|
| 1–7b printing, errors, arithmetic | 2,870 | | 32a–32f pdfTeX output | 9,075 |
| 9–20 memory, boxes, eqtb, hash, save stack | 4,232 | | 33–37 packaging, math, alignment | 4,393 |
| 21–29 input, scanning, expansion, file names | 9,140 | | 38–45 line and page breaking, hyphenation | 5,814 |
| 30–32 font metrics, `ship_out` | 2,802 | | 46–49 chief executive, building lists | 6,783 |
| 50–51 dump/undump, main program | 4,812 | | 53–53a extensions, e-TeX | 6,787 |
| sections the change files add (after part 53a) | 380 | | **total** | 57,088 |

What makes the generated code hard to read or change (counts over the procedure bodies):

| Pattern | Count |
|---|---:|
| `crate::ix::U(...)` index wrappers | 10,457 |
| `as` casts | 10,620 |
| integer literals of three or more digits (string-pool numbers, `eqtb` locations) | 5,441 |
| raw `self.mem[...]` accesses (node fields by offset) | 5,047 |
| `let __vNNN` temporaries (Pascal assignment order) | 1,587 |
| labelled `break`/`continue` (`goto`s) | 534 |

### 1.2 `unsafe`

198 occurrences of `unsafe` in the crate (172 blocks, 16 `unsafe fn`, 8 `unsafe impl`):

| Area | Count | Where |
|---|---:|---|
| Generated code | 0 | (`ix.rs` has 1, behind the opt-in `unchecked-reads` feature, DESIGN.md §4.2) |
| Runtime | 118 | `arena.rs` 49, `os.rs` 23, `resolver.rs` 16 (kpathsea FFI), `memstat.rs` 9, `host/crash.rs` 7, 14 elsewhere |
| C parts | 80 | `xpdf.rs` 43, `cfmt.rs` 13 (C's `printf`/`scanf`), `writepng.rs` 11, `zlib.rs` 10, `writet3.rs` 2, `utils.rs` 1 |

Almost all of it is FFI (kpathsea, libpng, zlib, xpdf, libc) or the arena's raw chunk
handling. Removing FFI `unsafe` is lane MEMORY-SAFETY's work; this plan's rule is only that
no step adds any, and that a rewritten module without FFI is `#![forbid(unsafe_code)]`.

## 2. Licensing: what still applies once the Rust is edited by hand

DESIGN.md §3 stays as it is; hand evolution adds these rules.

- **`third_party/pdftex/pdftex.web` is never edited, and stays in the repository.** The
  e-TeX notice in it reads: "Copying of this file is authorized only if (1) you are
  P. Breitenlohner, or if (2) you make absolutely no changes to your copy. (Programs such
  as TIE allow the application of several change files to tex.web; the master files
  tex.web and etex.ch should stay intact.)" Like Knuth's notice on `tex.web`, it governs
  copies of *that file*: the master must stay intact, and changes are made outside it.
  Change files did that; so does hand-written Rust derived from it. Neither the notice
  nor pdfTeX's GPL-2.0-or-later forbids modifying the derived program, and nothing here
  modifies the master. *Belief, not legal advice:* this is the reading the owner's §3
  legal review should confirm before a public release.
- **The fork point stays reproducible.** The last generated tree is tagged
  (`engine-fork-base`), and `tools/web2rust`, the change files and the `*.args` files stay
  in the repository at that tag, so anyone can regenerate exactly what was derived
  mechanically and see what was changed by hand since.
- **Derived code stays identifiable.** Every hand-evolved file keeps a header naming what
  it is derived from (`pdftex.web` §§, or the C file), its copyright holders
  (Han The Thanh and the pdfTeX team; P. Breitenlohner for e-TeX; D. E. Knuth for TeX)
  and GPL-2.0-or-later, and states that it was modified and when. GPL-2 §2(a) requires
  exactly this ("prominent notices stating that you changed the files and the date of any
  change"); git history carries the per-change dates. Identifiers and `// §NNNN`
  references stay wherever the code still corresponds to a section.
- **Names.** Product text never says TeX, e-TeX or pdfTeX (DESIGN.md §3 already). Open for
  the legal review: the engine's log banner and `\pdftexbanner` say "This is pdfTeX,
  Version 3.141592653-2.6-1.40.29 (TeX Live 2026)", because P-T1 compares logs line by
  line. A hand-modified program printing another program's name is a stronger claim than
  a mechanical translation doing so; P-T1 could normalise the banner line instead.
- **The C parts** (`writet1.c` and the others) are GPL-2.0-or-later; their rewrites carry
  the same header rule. Linked libraries (kpathsea, zlib, libpng, xpdf) are unaffected:
  they stay linked, unmodified, with the licences DESIGN.md §3 lists.

## 3. Strategy: a strangler, module by module

The engine keeps running and passing every gate after each step. A step replaces one
module behind the interface it already has, proves the new one identical, and deletes the
old one in the same PR. Nothing is rewritten speculatively; each step has a measured
reason (readability, safety, speed) and a gate.

### 3.1 Stage A: pdfTeX's C parts (`src/pdftex/`), now

These were never generated, so no freeze is needed. Each C file's port is a module with a
narrow interface to the engine (the `external` routines of `changes/ext.ch`, called from
the generated code). The target shape, set by the pilot (§6):

- parsing and writing are pure functions of their input, with **no `&mut Globals`
  inside**: what they need from the engine (warnings, the subset tag, file lookup) is a
  small trait, so each part is unit-tested without TeX Live;
- **errors are values** (`Result<_, Fail>`); the one place that calls `pdftex_fail` is the
  glue, so a parse cannot end the run halfway through a buffer;
- C idioms become types: `-1`/EOF sentinels become `Option`, mode integers become `enum`s,
  NUL-terminated buffers become a type with the two views C uses (up to the NUL; up to
  the pointer), command numbers become an `enum` with their table;
- no panics on malformed input where the port panicked (DESIGN.md §4.5);
- the module's state that checkpoints copy (`CState`, `codec_struct!`) keeps its encoding
  unless the step bumps the persist version.

### 3.2 Stage B: the generated engine, after a freeze

**Gated on the DESIGN.md record.** Until the PR of branch
`agent/mac-claude-a/rewrite-design-record` lands (it records the owner decision and its
licensing constraints in DESIGN.md §3, §4.1 and §13; the Commander opens it once the open
DESIGN.md PRs have landed, §9.5), DESIGN.md §4.1's rule stands: `src/generated/` is
regenerated and never edited, and no step of Stage B starts, its prerequisites included.
Stage A does not depend on it: the C ports were never generated.

1. **Prerequisites** (each its own PR, while regeneration still works):
   - the trip and etrip capacities become a build configuration of the engine crate (a
     cargo feature selecting a constants module) instead of a separate generation:
     today `scripts/flashtex-trip.sh` regenerates `tex.web` and `scripts/flashtex-etrip.sh`
     regenerates `pdftex.web` with other `--const`/`--macro` values, so after a freeze they
     would no longer test the code that ships;
   - web2rust's last improvements that make the frozen output better to start from:
     string-pool references emitted symbolically and the 212 hand-copied numeric
     constants of `iso.rs`/`intrinsics.rs` imported (DESIGN.md §4.1, decision (e));
     functions grouped into files by WEB part instead of by count;
   - every open PR that edits a change file has landed or been rebased (P6 lanes first).
2. **Freeze** (one PR, Commander-approved): regenerate one last time; tag
   `engine-fork-base`; the drift test becomes a test that the tag still regenerates its own
   tree; the change files stay as the record of what the fork point contains; new
   behaviour is edited into the Rust from then on.
3. **Module by module**, in the order of §4, each behind the `Globals` methods the rest of
   the engine calls:
   - typed views first: `NodeRef`/`TokenRef` newtypes and field accessors over the same
     `mem` words, `eqtb` locations by name, `?`-style error paths instead of
     `goto`-shaped loops; then
   - idiomatic bodies, one WEB part at a time.

### 3.3 The `mem` layout

Checkpoints (`arena.rs`: one word space with a dirty bitmap and per-chunk undo logs), the
convergence test (`iso.rs`'s lock-step walk of both states, `statediff.rs`), read-sets and
the intrinsics all read `mem`, `eqtb`, `hash` and the save stack **by word**. DESIGN.md D4
keeps those as flat arenas, and so does this plan:

- **Stage B keeps the word layout.** Typed accessors change how code reads a word, not
  where the word is; `iso.rs`, the persisted formats and the checkpoint logs see identical
  memory, which the both-paths gates below confirm.
- **A layout change** (e.g. contiguous token lists, DESIGN.md §4.2 P6) is a separate step
  with its own proof: done only after the typed accessors make every access go through a
  few functions; the format and persist versions bumped in the same PR; `iso.rs` and
  `statediff.rs` updated in the same PR; incremental soundness sweeps A, C, D and the
  fixture-wide soundness test green; and, while it is in review, a both-paths mode (old
  and new layout run side by side, states diffed at every checkpoint) as the intrinsics
  verify mode does today (`FLASHTEX_INTRINSICS=verify`).

## 4. Order, by value and risk

| # | Step | Value | Risk | Gate beyond §5's |
|---|---|---|---|---|
| A1 | **`writet1` (Type 1 subsetting): the pilot, done** | every document embeds Type 1 fonts; the clearest C shape | medium | the Type 1 corpus sweep (§6) |
| A2 | `writeenc`, `tounicode`, `mapfile`, `subfont` | parsed once per process; map files are user input | low | a map/enc sweep over TeX Live's map and `.enc` files |
| A3 | `writettf` (TrueType, OpenType CFF) | fontspec-free documents rarely use it; T6 fuzzers exist | medium | TrueType/OTF corpus sweep; after #1383 lands |
| A4 | `writepng`, `writejpg`, `writejbig2`, `images` | every image | medium | an image corpus sweep (`tests/images/generate.py` plus real images) |
| A5 | `writet3`/PK | rare | low | `pk_malformed.rs`, PK corpus |
| A6 | `pdftoepdf`, `epdf` | PDF inclusion, xpdf FFI (43 `unsafe`) | high | after MEMORY-SAFETY; PDF-inclusion corpus |
| A7 | `utils`, `output`, `vfpacket`, `avlstuff` | small; touch the checkpointed `CState` | medium | soundness sweeps |
| B0 | prerequisites and freeze (§3.2) | enables B | low | drift test of the tag; trip/etrip from the feature |
| B1 | typed node/token/eqtb accessors | readability everywhere; enables B2+ | low per PR, wide | lockstep, T2 |
| B2 | printing, arithmetic, string pool (parts 1–7b) | leaf code, trip covers it | low | trip, etrip |
| B3 | file names, input stack, font metrics (parts 22–30) | | medium | T2 |
| B4 | pdfTeX output (parts 32a–32f) | talks to the C parts | medium | P-T2 fixtures, arXiv |
| B5 | packaging, math, alignment (33–37) | | medium | lockstep, T2 |
| B6 | line breaking, hyphenation, page builder (38–45) | hot paths | high | instruction counts per bench document; coordinated with P6 lanes |
| B7 | chief executive, extensions, e-TeX (46–53a) | the dispatcher; every hook | high | everything, soundness sweeps |
| B8 | the `mem` layout, if ever (§3.3) | speed (P6) | high | §3.3 |

Hot-path steps (B6, B7, and anything in `get_next`, `expand`, `hpack`, `line_break`) go
last and are scheduled with the P6 lanes (P6-HYPEROPT and its successors), never in
parallel with them on the same files.

## 5. Proof obligation per step

A step lands only with all of these, measured on the PR's head, reported VERIFIED or
REPORTED as DESIGN.md §14 asks:

1. **T0**: `scripts/flashtex-trip.sh` and `scripts/flashtex-etrip.sh` (18/18) pass;
   `scripts/pdftex-regression.sh` 7/7.
2. **T1**: lockstep against pdfTeX 1.40.29, every case (1,435 on 2026-10-02), 0 new
   differences.
3. **P-T1 and P-T2** on the parity fixtures (`scripts/engine-parity.sh parity`, 86/86 each),
   and the arXiv sample (`tools/parity --tier arxiv`) not below its baseline.
4. **Incremental soundness** when the step touches checkpointed state (`CState`, the word
   space, anything `iso.rs` walks): `scripts/engine-parity.sh soundness` and sweeps A, C, D.
5. **A module corpus sweep, old against new, byte-identical**: the widest real input set
   for the module (the pilot's is every Type 1 font TeX Live maps), the old module kept
   until it passes and deleted in the same PR; and the same sweep against pdfTeX itself,
   where the old module's agreement with pdfTeX is the bar the new one must not lower.
   Real inputs are not enough: **differential fuzzing** of malformed inputs, old against
   new against pdfTeX, finds what no real input has (the pilot's review found a regression
   in PFB segment headers this way, §6). A difference between old and new is allowed only
   where the new module then agrees with pdfTeX.
6. **No performance regression, in instructions**: instructions retired (macOS
   `/usr/bin/time -l`, Linux `perf stat -e instructions:u`), median of at least five runs,
   on whole compiles of the bench documents the step can reach, and **amplified**: the
   module's work repeated in-process (the pilot runs each font's embedding 20 more times
   through an environment switch in a scratch build of each side) so that its own cost
   stands well above the run-to-run noise of a whole compile (0.4–2 % on a loaded Mac,
   where macOS counts kernel instructions too). Measure the module's cheap and expensive
   inputs both: the pilot's first cut was faster on whole fonts and 9.5 % slower on
   five-glyph subsets, because moving per-byte functions into other modules lost their
   inlining across codegen units. Wall time is reported, not gated.

   **Waivers.** A measured regression on one path lands only as a waiver recorded here:
   the path, its size, how many inputs reach it, the suspect, and the follow-up that owns it.

   | # | Path | Size | Reach | Suspect | Follow-up |
   |---|---|---|---|---|---|
   | W1 (pilot, 2026-10-04, reviewed on #1501) | `writet1` whole-font embedding (`<<`) | +3.9 % of its embedding instructions, about 0.1 M per font (24.0 → 24.9 M for 10 fonts, amplified) | 52 of TeX Live's 45,857 Type 1 map entries; subsets, the default, are 4–11 % cheaper | per-byte reading: `Reader::read_line` and `Reader::next_byte` (PFB segment bookkeeping, `Result` per byte); `next_byte` stays out of line | the next step on this module (§4 A1 follow-up): profile the include loop and bring it to ≤ the port before any other change to `reader.rs` |
7. **No new `unsafe`**, no new panic sites; the count before and after is in the PR.
8. `scripts/gate.sh pr` green.

## 6. The pilot: `writet1` (lane IDIOMATIC-REWRITE, 2026-10-04)

**Why this module first.** Of the C ports it is the one every document reaches (each Type 1
font of each compile, previews included), it has the narrowest interface (`writet1`,
`load_enc_file`, `standard_glyph_name`), it has no FFI and no `unsafe`, and its output is
exactly what P-T2 compares (embedded font subsets). It touches no hot path of the
typesetting engine and no file that P6-HYPEROPT (engine hot paths) or MEMORY-SAFETY (FFI
`unsafe`) works on.

**What changed.** `src/pdftex/writet1.rs` (2,396 lines, one 60-field struct holding
`&mut Globals`, every error a diverging `pdftex_fail` call) became `src/pdftex/writet1/`:

| Module | Contents |
|---|---|
| `cipher.rs` | the Type 1 cipher as a type (`Cipher::EEXEC`, `Cipher::CHARSTRING`) |
| `reader.rs` | PFA/PFB reading a line at a time; `Line` with C's two views; `Eexec` state `enum`; `Option<u8>` for EOF |
| `encoding.rs` | StandardEncoding, the shared `/name` array reader, `.enc` files as a pure function |
| `charstring.rs` | `Op` `enum` with its operand table, the operand stack, the non-recursive `cs_mark` walk |
| `subset.rs` | whole-font copy and subsetting, a `Writer` for the font buffer |
| `mod.rs` | the engine glue: the only place that prints or calls `pdftex_fail`; `Host` trait |

The output for the same input is the same bytes, the same log, the same warnings in the
same order, and the same font-descriptor values.

**Results** (mac-m1max-a, M1 Max, release builds, load 19–160 from other lanes; all
VERIFIED on this machine):

- **Parity.** The Type 1 sweep (`tools/rewrite/type1_sweep.py`; 12,818 `pdftex.map`
  entries: every font file once, every slanted or extended entry, every tenth re-encoding;
  each as eight glyphs, all 256 codes, and whole-font `<<`): 2,703 jobs, 161 of them
  ending in a font error, 26,588 embedded font streams. New against old: **every PDF and
  every log byte-identical** (final commit). New and old against TeX Live's pdfTeX 1.40.29:
  **26,588 of 26,588 font streams identical**, the same 161 fatal jobs. Lockstep
  **1,464/1,464** (7 non-gating accounting differences), parity fixtures **P-T1 86/86,
  P-T2 86/86** (both on the first commit; the second changes only inlining and an error
  type), etrip **18/18**, trip pass, pdfTeX regression **7/7**, the Type 1 integration
  tests (`type1_subr_nesting`, `pdf_fonts2`, `pdf_backend`) pass, 21 new unit tests and one
  integration test.
- **Review of #1501** (independent reviewer: a 50-entry sweep, 400 PFB and 200 PFA fuzz
  mutations, 12 crafted fonts) found one regression the map sweep could not see, because
  no real font has it: the rewrite read a PFB segment header when the bytes left were
  `<= 0` where C tests `== 0`, so a header of length 0 failed with "invalid marker" where
  pdfTeX prints "-N bytes more than expected" (5 of 400 fuzz cases). Fixed, and the count
  is now C's `int`, as pdfTeX's is: the port had read `ff ff ff ff` as 4294967295 and did
  not wrap -2^31, so it already differed from pdfTeX there; now 90 of 90 crafted segment
  lengths (cmr10, cmti10, cmbx10; each segment; 0, 1, -1, 2^31 - 1, -2^31; subset and
  whole) give pdfTeX's message (`tests/type1_pfb_segments.rs` compares them with TeX
  Live's pdftex; three unit tests pin the arithmetic). The map sweep cannot find this
  class, so `type1_sweep.py fuzz` (old, new and pdftex on fuzzed PFB and PFA fonts) is
  part of this module's proof from now on, nightly and on pull requests that touch it
  (`.github/workflows/writet1-sweep.yml`). Also from the review: a charstring announcing
  more bytes than the file has stops at the end of the file instead of decrypting up to
  2^31 times (the same "unexpected end of file" follows); `callothersubr`'s count wraps
  as C's does; the derived-file header on every file of the module. After the fixes
  (VERIFIED): the map sweep at every tenth entry (315 jobs, 3,466 font streams) is
  byte-identical to the port and equal to pdftex; two fuzz runs of 600 (400 PFB, 200 PFA;
  seeds 1501 and 15012) give 0 crashes and 0 regressions; every difference from the port
  (14 runs, 7 fonts) is a segment length the port read as unsigned, where the rewrite now
  prints pdfTeX's message; the only differences from pdftex (28 runs) are fonts on which
  pdfTeX itself crashes (SIGSEGV, a subr that calls itself), which are font errors here by
  design (#1237).
- **`unsafe`: 0 before, 0 after**, now `#![forbid(unsafe_code)]`. Shape (non-test code):
  `as` casts 129 → 37; `unwrap`/`expect` 12 → 1 (the glue's existing map-entry lookup);
  `.to_vec()` 34 → 21; `.clone()` 19 → 9; code lines 2,078 → 1,873. Two inputs on which
  the port panicked (undefined behaviour in pdfTeX) now keep what the line has.
- **Performance** (instructions retired, minimum of 5–9 interleaved runs):

  | Measurement | Before | After | Change |
  |---|---:|---:|---:|
  | per document's embedding, amplified: plain-10 | 38.0 M | 36.3 M | −4.6 % |
  | full-10 | 51.3 M | 49.0 M | −4.4 % |
  | lmodern-report | 187.8 M | 168.3 M | −10.4 % |
  | beamer-default | 21.9 M | 19.4 M | −11 % |
  | math-sheet | 90.9 M | 84.2 M | −7.4 % |
  | whole fonts (`<<`, 10 fonts) | 24.0 M | 24.9 M | **+3.9 %** |
  | ini job, 200 fonts × 256 glyphs, uncompressed (whole run) | 4,592 M | 4,122 M | −10.2 % |
  | ini job, 200 fonts × 5 glyphs (whole run) | 3,056 M | 3,042 M | −0.5 % |
  | the same jobs without embedding (controls) | | | 0.0 %, −0.1 % |
  | whole compiles: plain-10, full-10, full-100, lmodern-report, beamer-default, math-sheet | | | +0.16 … +0.32 % |
  | an empty LaTeX document | | | 0.0 % |

  The whole-compile differences are inside this machine's noise: the same comparison
  between two other builds of the two sides gave −0.7 … +0.7 % on the same documents, and
  the amplified per-embedding numbers above are the module's own cost. *Open:* the
  whole-font path (+3.9 %, about 0.1 M instructions per font) reads every byte through
  `Reader::read_line` and is slower there; only 52 of TeX Live's 45,857 Type 1 map entries
  embed whole. A second attempt (`#[inline(always)]` and a cold PFB-header path) made every
  path slower and was dropped; it is a follow-up for the next step on this module.

## 7. Coordination

- **P6-HYPEROPT** (engine hot paths) and later P6 lanes own `get_next`, `expand`, the
  intrinsics and the throughput change files; rewrite steps B6/B7 wait for them and are
  scheduled with them.
- **MEMORY-SAFETY** owns the FFI `unsafe` (`cfmt.rs`, `xpdf.rs`, `writepng.rs`, `zlib.rs`,
  `arena.rs`, `os.rs`, `resolver.rs`); this lane does not edit those files until it lands,
  and the pilot still calls `cfmt` for C's `sscanf`/`printf`.
- One PR per step, base `main`, lane branch `agent/<id>/rewrite-<topic>`.
