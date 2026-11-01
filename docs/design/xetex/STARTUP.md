# Unicode mode's start-up: where it went, what changed, what is next

Lane M3-FAST, 2026-10-12, under the Commander `kabir-claude`. The M3 gate's
latency rows failed ([`docs/evidence/unicode-m3-2026-10-11`](../../evidence/unicode-m3-2026-10-11/README.md)
§3): every Unicode compile is a full run, and the engine's start-up alone
was 848 M instructions. This note attributes that cost, records what this
lane changed (each change keeps every output byte), and proposes the path
to an incremental Unicode host on Classic's machinery.
[`INCREMENTAL.md`](INCREMENTAL.md) (lane XETEX-FAST) and PR #1779 (lane
XETEX-INCR, the resident engine) are the other two pieces; this note does
not repeat them.

**Verified** below means measured in the runs named; **belief** means not
measured.

## 1. How it is measured

[`.github/workflows/unicode-startup.yml`](../../../.github/workflows/unicode-startup.yml)
builds `flashtex-xetex` and `flashtex-host-unicode` at a commit and at its
merge base with main (release, with line tables so that profiles name
functions; debug information does not change the code generated), on
ubuntu-latest with the pinned scheme-full TeX Live 2026 as the inputs'
tree. TeX Live is never in the product path.

- [`tools/unicode-m3/profile_startup.py`](../../../tools/unicode-m3/profile_startup.py):
  callgrind (the runner's `perf` cannot count) on four runs: the engine as
  `xelatex` on `\csname @@end\endcsname` (process start to the format
  loaded, M3's "start-up" row), bench b1 (article, amsmath, graphicx) and
  b2 (fontspec, TeX Gyre Termes by name) with their PDF and a warm `.aux`,
  and the host from its start to its HELLO reply. Costs are attributed by
  inclusive cost of named functions.
- [`tools/unicode-m3/identical.py`](../../../tools/unicode-m3/identical.py):
  both builds on the M3 corpus and the bench (53 cases), each from a format
  it builds itself: the engine's PDF (run until the `.aux` settles), its
  XDV and its log, and the PDF the host writes, compared byte for byte.
  `--loaded-state` compares the word space right after the format load
  (below).

## 2. Where the start-up went (verified, run 38208817371, base 73d5be75e)

User-space instructions, M:

| | start-up | b1 | b2 | host to HELLO |
|---|---:|---:|---:|---:|
| whole run | 848.3 | 1,504.0 | 3,320.4 | 4,154.6 |
| `initialize` | 537.0 | 537.0 | 537.0 | |
| `load_fmt_file` (the undump) | 286.8 | 286.8 | 286.8 | |
| kpathsea set-up (`texmf.cnf`, `ls-R`) | 14.9 | 14.9 | 14.9 | 14.8 |
| `pdftex.map` index | | 90.9 | | |
| the fonts' catalog (lookup by name) | | | 485.8 | |
| `format::ensure` (of which SHA-256 of the program) | | | | 4,137.0 (4,069.6) |

- **`initialize` was most of the start-up**, and most of it was INITEX's
  work: setting every entry of XeTeX's Unicode-sized `eqtb` (9.6 M words),
  the hash's frozen part, `mem`'s static part. web2c's `tex.ch` [1.8] runs
  that part only when `ini_version` ("init...tini is dynamic"); the port
  ran it on every run, and then the format load overwrote all of it.
  Through the arena's write barrier each word costs about 10 instructions.
- **The undump** is 3.4 M words, each through `get_word` and the barrier.
- **The host's start** hashed its own 8.6 MB binary (more with line tables)
  once per format to name the format cache's slot: the 644 M instructions
  the M3 evidence measured for HELLO on the release binary.
- **A lookup by name** walked the font directories, opening every font
  file, then read the name cache's JSON (157 M for the parse alone).
- **The font map**: 5.5 MB of `pdftex.map` scanned a byte at a time.

kpathsea is not a cost here (15 M): `ls-R` is read lazily.

## 3. What changed (verified, run 38208817371 against its merge base)

| | start-up | b1 | b2 | host to HELLO |
|---|---:|---:|---:|---:|
| before | 848.3 | 1,504.0 | 3,320.4 | 4,154.6 |
| after | 472.7 (−44 %) | 1,089.9 (−28 %) | 2,531.2 (−24 %) | 85.1 (−98 %) |

1. **Production runs skip INITEX's table entries** (`changes/virtex.ch`):
   tex.ch's `Init`...`Tini` for "Initialize table entries" [1.8] and for the
   main program's string pool, `init_prim` and first clock reading
   [51.1332], through `initex_tables` (= `ini_version`). `initialize` 537 →
   180 M. Faithfulness: this is web2c's own behaviour, and it is checked
   directly: `FLASHTEX_INITEX_TABLES=1` makes a production run set the
   tables up anyway, `FLASHTEX_LOADED_STATE=PATH` writes the word space
   (64 KiB blocks, and each scalar global) when the format file is closed,
   and `identical.py --loaded-state` compares the two. Runs 38208817371 and
   38211022101: every block of every array is equal (5,348 blocks); the
   scalars that differ are the clock (`sys_day`, `sys_month`, `sys_year`,
   `epochseconds`, `microseconds`), which §1337 reads again right after the
   load in every run, and what `init_prim` leaves (`prim_used`, read only
   by `primitive`, which only INITEX calls, and `cur_val`). web2c's
   production runs never call `init_prim` and start with both at 0, as
   the port's now do.
2. **The format is read whole** (`system.rs`): `get_word` indexes a byte
   vector instead of a buffered reader's `read_exact`. `load_fmt_file`
   286.8 → 269.4 M.
3. **The build digest is kept** (`host/format.rs`): `formats-unicode/builds`
   maps the program file's identity (device, inode, size, modification
   and change times, path) to its SHA-256; a replaced program has a new
   inode or change time and is hashed again, a file within 2 s of now is
   not written down. `ensure` 4,137 → 67.5 M.
4. **The fonts' catalog is kept** (`fontmgr/catalog/kept.rs`): the catalog
   is written down with the directory list, each directory the walk lists
   (device, inode, modification time, taken before it is listed;
   `font-discovery`'s new `FontIndex::scan_traced`) and each font file it
   visits (device, inode, size, modification and change times). A run whose
   identities are all equal takes the catalog from the file and opens no
   font file; a scan with a time within 2 s of its start is not kept.
   `find_native_font` 491 → 79.5 M on b2.
5. **The map's lines are found eight bytes at a time** (`out/fontmap.rs`):
   `FontMap::from_vec` 90.9 → 52.4 M.

Output: 53 of 53 cases identical, engine PDF, XDV and log and host PDF
(run 38208817371, `identical`). The crate's library tests pass on the head
build (54 of 54, `line_ends_are_the_first_newline` and
`a_kept_catalog_is_the_scan_until_something_changes` among them).

## 4. What is left of a cold run (verified costs, head of run 38208817371)

- `load_fmt_file`, 269 M: the undump itself, through the barrier. A
  persisted image (§5, step B) removes it on a cold host.
- `initialize`, 180 M: what xetex.web sets for every run, chiefly the
  `hash` (8.4 M entries, §257) and `xeq_level` (1.1 M, §280). web2c does
  the same work at allocation.
- `format::ensure`'s validation, 59.5 M at HELLO: it re-runs the format's
  recorded kpathsea lookups (Classic's LOOKUP-SKIP class of work).
- `pdftex.map`'s index, 52 M per run with a TFM font: kept between runs,
  as the catalog is, it would cost a read (belief).
- The PDF's font programs are hashed at every run (`Fonts::read_program`,
  53.8 M on b1): keyable by file identity, as the build digest is (belief).
- The catalog file's parse, 73 M on b2 (`unesc_list` allocations): a
  binary form would cut it (belief).

Belief, not measured: on an M-series core a 473 M-instruction start-up is
roughly 40–60 ms. The §1.2 rows (edit ≤ 16 ms, reopen ≤ 100 ms) still need
§5: a cold run cannot meet the edit row.

## 5. The incremental Unicode path (proposal, for later)

Classic's machinery works on web2rust's word space and on files, not on
pdfTeX's structures (INCREMENTAL.md §3), so the Unicode host can take it
over in steps, each gated by `identical.py` (byte identity with cold
runs) and the host sweep:

- **A. The resident engine, in process** (PR #1779, lane XETEX-INCR): the
  first run keeps a checkpoint where the format is loaded and later runs
  restore it, skipping `initialize`, the undump and process start. This
  lane's changes make that first run cheaper and keep its catalog and
  build digest across host restarts.
- **B. The loaded image across processes.** A reopen or a host's first
  compile still pays §4's ~470 M. Classic's arena can map a persisted
  space copy-on-write (`Arena::map_pages`, `mark_loaded`, as Classic's
  persisted sessions do). Write the word space and the scalars after the
  format load, with the state outside it (`state.rs`'s `Saved`), to
  `formats-unicode/<build>/<format>/image`, keyed by the engine build, the
  format file's identity and the pre-load state's inputs (command line,
  first line, `-etex`); a cold run maps it instead of `initialize` and the
  undump. Proof of faithfulness, as #1750's `FLASHTEX_FMT_IMAGE=verify`: a
  verify mode loads as now and compares with the image, which
  `FLASHTEX_LOADED_STATE` already does block for block.
- **C. S₀ at `\begin{document}`** (INCREMENTAL.md §5 step 3):
  `checkpoint.ch` for xetex.web, keyed by the read set. The font
  identities `kept.rs` records (directories listed, font files visited) are
  the read set's font part: a changed font directory restarts before the
  first `\font` that searched it.
- **D. Body checkpoints, restart and converge, viewport first** (INCREMENTAL.md
  §5 steps 4–6), after R2 moves Classic's session code (`incr.rs`,
  `readset.rs`, `revalidate.rs`, `iso.rs` for XeTeX's node types) into the
  shared runtime.

The edit row (≤ 16 ms) needs C and D; the reopen row (≤ 100 ms) needs B,
or A with a warm host.
