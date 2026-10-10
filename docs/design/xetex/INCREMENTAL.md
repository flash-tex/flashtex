# Unicode mode: from cold runs to an incremental host

Lane XETEX-FAST, 2026-10-10. A short design note: what makes a Unicode-mode
keystroke slow today, what landed in this lane, and how the Unicode host
becomes incremental after the runtime extraction (R2, PLAN.md §3.3). Nothing
here is implemented beyond §2. `docs/design/engine-v2/DESIGN.md` (§5) governs.

## 1. Where a keystroke goes today

`flashtex-host-unicode` (PLAN.md §4C) runs every compile as a new process
of the engine, from the format, to the end of the document. Measured on
mac-m1max-a (instructions retired, `/usr/bin/time -l`, warm `.aux`, best of
three; the machine was loaded, so instruction counts, not times, are the
measure here):

| document | whole run | process start to format loaded | the document |
|---|---:|---:|---:|
| article + amsmath + graphicx (Latin Modern OTF) | 1,918 M | 1,138 M (59 %) | 780 M |
| fontspec, `\setmainfont{Georgia}` | 4,003 M | 1,138 M + fonts' catalog 750 M | 2,115 M |
| unicode-math + amsmath | 8,072 M | 1,138 M (14 %) | 6,934 M |
| fontspec, 11 pages | 4,483 M | 1,145 M (26 %) | 3,338 M |

The first 1.1 G instructions are the same at every keystroke: the process
start, kpathsea's start-up (`texmf.cnf`, `ls-R`), `initialize` and
`init_prim` over XeTeX's Unicode-sized `eqtb`, and the undump of the 27 MB
`xelatex.fmt`. A document that names a system font adds the fonts' catalog
(a walk of the font directories, the name cache, about 750 M). The rest is
TeX itself running the preamble and the pages: for unicode-math, 86 % of the
run is `unicode-math`'s and `expl3`'s macros at `\usepackage` time, which only
a post-preamble snapshot (S₀) removes. TeX Live's `xelatex -no-pdf` spends
about the same (1,702 M, 3,387 M, 7,793 M and 3,732 M on these documents),
and its PDF goes through xdvipdfmx after that: FlashTeX's engine is not slow,
it is cold.

## 2. What landed in this lane (cold path)

- **Hot spares** (`src/host/spare.rs`): while a document is being compiled,
  the host keeps the next run started and stopped just after the format is
  loaded, and lets it go when the next compile comes, if nothing it read has
  changed (the command, the format file, the main file's `%&` line, and the
  font directories for a prepared catalog). This is the child-per-run
  counterpart of Classic's resident engine with `fmtimage` (#1750). Output is
  byte-identical (PDF, log, display list) with and without spares.
- **The font map is indexed, not parsed** (`src/out/fontmap.rs`): 46,000
  lines of `pdftex.map` are indexed by TFM name, and a line is parsed when its
  font is used.
- **Children run without the resolver's lookup memo**, which only pays off
  in a resident engine.

A spare holds about 200 MB while it waits, for at most
`FLASHTEX_UNICODE_SPARE_TTL` seconds (default 120): at most one a document
and two in all (`FLASHTEX_UNICODE_SPARE_CAP`), none in the Low Memory
performance mode (a switch to it ends the spare; the host now has
`profile-v1`). A spare is started exactly as the run it replaces: its key is
the whole command (arguments, so shell escape and the other compile
settings; directory; the host's environment, so read confinement, Live
Share roots and `openout_any`) and the external-tools setting. It costs no CPU
while it waits (DESIGN.md §5.1, decision 10, forbids spinning, not
residency) and none before the first compile of a document (decision 8:
nothing is kept warm in advance; the first compile is a cold one). Spares
end when the incremental host below replaces runs in their own processes.

## 3. What Classic's incremental machinery gives, and what is pdfTeX's

Reusable once R2 moves it into the shared runtime crate, because it works on
web2rust's word space and on files, not on pdfTeX's data structures:

- `arena.rs`: the flat word space, the dirty bitmap, undo and redo logs,
  compaction, `diff_branch`; `checkpoint.rs`'s sealing and restoring of the
  word space and of file positions.
- `incr.rs`'s session: changed-file detection (common prefix and suffix),
  the restart search by consumed input, retention under the budget, viewport
  first, the background finish, the `.aux` pass rules (§5.5) and external
  tools; `readset.rs`, `revalidate.rs`, `lookupproof.rs`, `hashcache.rs`,
  `texlines.rs` and `midline.rs` (input consumed by whole lines).
- `fmtimage.rs` (the format image) and the resident host's protocol code
  (`host/resident.rs`, `host/server.rs`).

pdfTeX's own, each needing a XeTeX counterpart written against `xetex.web`:

- **Change files.** `checkpoint.ch` (the hook at `big_switch`),
  `fmtimage.ch`, `lineshift.ch`, `readset.ch`, `throughput.ch` and the
  intrinsics (`intrinsics.rs`) are written against `pdftex.web`'s sections.
  `xetex.web` shares TeX's core but not its numbering or its extensions.
- **`iso.rs`** (structural comparison for convergence) walks pdfTeX's node
  layouts. XeTeX adds `native_word_node`, `glyph_node`, `pic_node` and
  `pdf_node` whatsits with handles in them, and a Unicode `eqtb`.
- **The dead-word whitelist** (`positions_only` and the rules citing
  `pdftex.web`) has to be redone from `xetex.web`, each rule with its
  soundness case (DESIGN.md §5.3, adopted 2026-10-01).
- **`CState`** (pdfTeX's C parts) becomes XeTeX's state outside the word
  space, next section.

## 4. What a XeTeX checkpoint must hold

- **The handle table** (`state.rs`'s `Handles`): glyph-info arrays, picture
  paths, and as `Other` the fonts' layout engines (FreeType face and size,
  HarfBuzz face and font, the font's feature and script settings), TECkit
  converters and OpenType math assemblies. Checkpoints already copy the
  table and share the objects (PLAN.md §4.7). Shared objects must not carry
  state that changes later answers: FreeType's glyph slot and HarfBuzz's
  shape-plan and table caches are caches (belief, to be tested by a
  soundness case that shapes the same word before and after a restore);
  TECkit converters are reset per conversion (`applymapping`) (belief, same).
- **The font manager** (`Host::font_mgr`): the catalog is immutable; the
  manager's maps fill as names are looked up and change later answers, so it
  is saved copy-on-write (already in `Saved`).
- **OS font references.** A checkpoint is valid only while the files it
  loaded are: every font file a run opened (path, size, times, inode) goes
  into the read-set, and for fonts found by name the font directories'
  identities too (the rule spares use, which is fontconfig's own). A changed
  font restarts before its `\font`.
- **The output** (`Host::out`, not saved yet): the display-list document
  (defined fonts and their resource ids, images, forms, named objects and
  the outline, pending map lines), the PDF writer's per-page records, and
  the XDV bytes of a page not yet read. It plays the part of Classic's
  external-effect log.
- Not saved, as now: the normalizers, `mapping_out_length`, the FreeType
  library, the start time (the run's).

## 5. Order of work after R2

1. **R2** (owned elsewhere): the shared runtime crate and the engine trait,
   with every process-wide `static` of the runtime made per-engine, and the
   engine's exits (`final_end`, `exit_process`) returning to the host instead
   of ending the process.
2. **A resident Unicode engine, cold runs in-process** (L1 without S₀): the
   engine runs on the host's thread from a fresh `Globals` per run, with the
   format image (`fmtimage.ch` for `xetex.web`) and the kpathsea and font
   catalogs kept in the process. Gate: P-T1 lockstep, XDV and the PDF parity
   baseline unchanged; spares removed.
3. **S₀** at `\begin{document}`, keyed by the read-set (font files and
   directories included). This is the step that matters for unicode-math.
4. **L2 checkpoints** at `\shipout` with §4's state, `checkpoint.ch` for
   `xetex.web`.
5. **L3 restart and converge**: `iso.rs` for XeTeX's nodes, the dead-word
   rules from `xetex.web`, line shifting; a soundness case for every rule and
   the soundness sweep (incremental equal to cold, frame by frame).
6. **L4 viewport first and L5 passes**, as Classic has them.
