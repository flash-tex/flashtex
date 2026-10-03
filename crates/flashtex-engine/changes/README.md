# `crates/flashtex-engine/changes/`: WEB change files

`third_party/pdftex/pdftex.web` is never edited. Everything this engine does
differently from it is a WEB change file, applied by `tools/web2rust` exactly
as TANGLE applies one (`@x` lines must match the master; see
`tools/web2rust/src/changes.rs`). Several change files are applied one after
another, in the order `web2rust-default.args` lists them, as `tie -c`
combines web2c's change files for TeX Live's pdfTeX.

The files here are GPL-2.0-or-later, like the rest of the crate. Each change's
`@x` line names the line of `pdftex.web` it replaces and, where it
re-specifies web2c, the part of `tex.ch` or `pdftex.ch` it stands in for
(DESIGN.md §4.1: "the system-dependent behaviour web2c adds ... is
re-specified by us"). TeX Live's own public-domain feature change files are in
`third_party/pdftex/web2c/` and are applied unmodified.

In order:

| file | what it does |
|---|---|
| `web2c.ch` | What pdftex.web takes for granted from web2c: `orig_char_info`; every character is itself, printable if ASCII (tex.ch [2]); a box's glue ratio shown as a number even when its bits look strange, never `?.?` (tex.ch [12.186]); the frozen-control-sequence layout that makes room for `\pdfprimitive`'s table (pdftex.ch); the run-time arrays web2c allocates; the date and time (`SOURCE_DATE_EPOCH`, `FORCE_SOURCE_DATE`); the expansion-depth limit; more than 256 fonts (`max_font_max`, 16-bit font numbers in char nodes, `fnt_def2`/`fnt2` in DVI); more than 255 hyphenation ops per language ("bigtrie"); a repeated `\hyphenation` word replacing the old one and giving its string back (tex.ch [42.941]); `\font` never reusing a font that has expansion parameters (pdftex.ch, `pdf_font_step[f]<>0`); a font scaled to 2048pt or more is an error that ignores the scaling factor, and a TFM name of more than 255 characters is "file name too long", the name packed without `.tfm` (tex.ch [30.560-568], [49.1260]); a DVI file longer than `"7FFFFFFF` bytes is the fatal error `dvi length exceeds "7FFFFFFF`, with no postamble (tex.ch [32.598], [32.599], [32.642]); the newline before the program ends; `hash_size+hash_extra` in the statistics. |
| `goto.ch` | The one jump that enters a sibling `case` arm (`prune_page_top`), which labelled blocks and loops cannot express, rewritten into the same statements in the same order. |
| `precedence.ch` | Four expressions whose C reading (web2c prints Pascal's `and`/`or` as C's `&&`/`\|\|` with C's precedence) differs from their Pascal reading; the parentheses the C compiler implies are made explicit. `web2rust` refuses any such expression, so this list is complete. |
| `ext.ch` | The interface to pdfTeX's C parts (`pdftex.defines`): their routines declared `external` with Pascal types, bodies in `src/pdftex/`; `pdftex.h`'s macros as Pascal; the PDF buffer pointer `pdf_buf` as an explicit choice between the two buffers; kpathsea's PK set-up as one call. |
| `filenames.ch` | File names as tex.ch [29] treats them: `/` areas, the last `.` starts the extension, `"` quoting, `\input{...}`, no forced `.tex` (the resolver tries `name.tex`, then `name`), the name found shown in the log (its full name, before `./` is taken off), `\openout` logged when texmf.cnf's `log_openout` says so; `prompt_file_name`'s help line (`(Press Enter to retry, or Control-D to exit; default file extension is ...)`) and an empty reply retrying the same name (tex.ch [29.530]); tex.ch's string recycling (`search_string`, `slow_make_string`: `end_name` reuses an equal string already in the pool, `new_font` flushes nothing), which shows in the `\tracingstats` string counts. |
| `virtex.ch` | INITEX and production runs in one program (tex.ch's `-ini`), `-etex` (pdftex.ch), and a production run's default format (`-fmt`, else the program name), which its banner names. |
| `web2c-hooks.ch` | The lines of tex.ch that TeX Live's feature change files are written against (web2c's integer parameters, which start with ML\TeX's three; one undump line). |
| *third_party/pdftex/web2c/* | `tracingstacklevels.ch`, `partoken-102.ch`, `partoken.ch`, `locnull-optimize.ch`, `showstream.ch`, unmodified. |
| `enctex.ch` | TeX Live's encTeX eqtb entries (enctex1.ch, enctex-pdftex.ch): the three region-4 locations of `\xordcode`, `\xchrcode`, `\xprncode` and the integer parameters `\mubytein`, `\mubyteout`, `\mubytelog`, `\specialout`, without the primitives (`-enc` is refused), so that eqtb has TeX Live's layout. |
| *third_party/pdftex/web2c/* | `unbalanced-braces.ch`, unmodified. |
| `synctex.ch` | SyncTeX's memory layout as TeX Live's synctex-*.ch files make it (box nodes 9 words, rules 6, glue, kern, math and penalty nodes 4; the file tag and line that `get_node` writes into every node of 4 words or more, and that copies keep), and the `\synctex` parameter. The layout shows in box dumps, not only in memory accounting: `hpack` with `cal_expand_ratio` leaves a line box's glue fields as `get_node` found them. No `.synctex` file is written. |
| *third_party/pdftex/web2c/* | `char-warning-pdftex.ch`, unmodified. |
| `web2c-run.ch` | How a run is set up and reports itself (tex.ch's part of web2c's command line and texmf.cnf; texmfmp.c's part is `src/main.rs` and `system::configure`): `error_line`, `half_error_line`, `max_print_line` and `expand_depth` read from texmf.cnf at run time; `-interaction`; `-file-line-error` messages; `-halt-on-error`; the status lines after the banner (`\write18`, file:line:error, %&-line parsing, the TCX file); a `%&format` first line; `-jobname`; the recorder's file name; `\write18` and `\eof18`; `openin_any`/`openout_any`; the TCX file's `xord`/`xchr`/`xprn`, which a format carries; `-output-format` and `-draftmode`; tex.ch's fixes for fatal errors on the terminal. |
| `checkpoint.ch` | Where the incremental engine may take a checkpoint (DESIGN.md §5.1, §5.2): `big_switch` calls the hand-written `flashtex_checkpoint_hook` when `ckpt_request` is nonzero, and a restored run re-enters `main_control` there without re-inserting `\everyjob`; expanding the control sequence the host names (`\document`) arms the begin-document snapshot S₀, which `pop_input` requests once that expansion has been consumed; `ship_out` requests a checkpoint after each page when asked. Nothing it adds changes what the program computes (`src/checkpoint.rs`, `src/host/`). |
| `readset.ch` | L5 read-sets (DESIGN.md §5.5): while `rs_on` (the host turns it on at the `.aux` point), `get_next`'s reads of a control sequence's meaning (from a file, a token list, an active character, an empty line's `\par`, a `\noexpand`ed token) and every `id_lookup` (so `\csname` and e-TeX's `\ifcsname`, found or not) call the hand-written `flashtex_cs_read`/`flashtex_id_read` the first time, and `rs_seen` (in the word space) remembers which were seen. Nothing it adds changes what the program computes (`src/readset.rs`). |
| `displaylist.ch` | Where the display-list writer (`src/displaylist/`, docs/protocol/display-list-v3.md) observes the engine, changing nothing TeX computes: each node allocated gets its source position in a side table (`get_avail`, `fast_get_avail`, `get_node`; copies keep their original's in `copy_node_list`; words `hyphenate` rebuilds keep their first letter's), and `pdf_hlist_out`/`pdf_vlist_out` note the page stream's offset as they output each node. The routines do nothing unless a display list was asked for. |
| `intrinsics.ch` | The macro-level profiler (`FLASHTEX_MACRO_PROFILE`, `src/macroprof.rs`) and the guarded intrinsics (DESIGN.md §5.6 items 4 and 6, `src/intrinsics.rs`): a registered parameterless macro that `big_switch` expands is recorded -- every state it reads, every change it makes, every command it runs -- and later replayed through `eq_define`/`eq_word_define`/`new_save_level`/`unsave` while nothing it read has changed (`eq_define`, `geq_define` and `unsave` report writes to watched `eqtb` entries). Every hook is behind a boolean that is false unless the feature is on, and none changes what the program computes; `FLASHTEX_INTRINSICS=verify` runs both paths and diffs the state (`src/intrinsics_verify.rs`). |
| `diagnostics.ch` | Where the diagnostics side channel (`src/diag.rs`, docs/protocol/display-list-v3.md §6.7 `diag-v1`) observes the engine, changing nothing TeX computes or prints: `print_err` marks where a message starts in the captured terminal; `error` hands over the message, the input stack (every level, with the column of the bottom file level's split), the help lines or `\errhelp`; `pdf_warning`; the overfull/underfull box reports of `hpack` and `vpackage` (the box's first and last character, from the display list's side table); `\def` (definition sites); `write_out`'s `\write`s to the terminal (LaTeX's warnings). The routines do nothing unless the host turned the side channel on (or `FLASHTEX_DIAGNOSTICS=1`). |
| `throughput.ch` | Faster routines that compute exactly what pdftex.web's do (lane P6-THROUGHPUT, DESIGN.md §4.2 and §5.6 L6), each with its proof of identity in the file: `get_next`'s external-file part becomes the function `get_next_file` (same statements in the same order; its `goto restart`, `return` and fall-through are its result), and `divide_scaled` is one exact 64-bit division instead of a digit loop (`tests/divide_scaled.rs` compares it with pdftex.web's loop). Evidence: `docs/evidence/p6-throughput-2026-09-30/`. |

New sections are added only at the end of part 54 ("System-dependent
changes"), as tex.web asks, so the section numbers `// §NNNN` in
`src/generated/` stay those of pdftex.web up to part 54.

## Deliberate differences from TeX Live's pdfTeX

- `\write18` follows web2c: texmf.cnf's `shell_escape` decides unless an
  option does (TeX Live ships `p`, restricted to `shell_escape_commands`;
  DESIGN.md §4.5). The bundle resolver, which has no texmf.cnf, carries TeX
  Live's two values. Every command executed
  (`\write18`, `\input|cmd`, `\openout` to `|cmd`) is recorded as an
  external effect (`system::external_effects`,
  `FLASHTEX_EXTERNAL_EFFECTS`).
- The format directory kpathsea searches is `web2c/flashtex` (`$engine`),
  because this engine's formats are not pdfTeX's; `FLASHTEX_FORMATS` (a
  list of directories) is searched after it.

## Not re-specified yet

What else TeX Live's pdfTeX does that is visible, measured on the parity
fixtures (`tools/parity`, fixtures tier) and the lockstep corpus
(`tools/lockstep`) against TeX Live 2026's `pdftex`:

- the font map, font embedding and image inclusion (`{.../x.enc}` and
  `<.../cmr10.pfb>` in the log, `\pdfximage`, `isscalable` for font
  expansion): pdfTeX's C parts, lane P3;
- SyncTeX's output, the `.synctex` file (its node layout is in
  `synctex.ch`; `-synctex` is refused);
- the string pool's constants. pdfTeX's pool holds 31 strings and 726
  characters more than this engine's, which shows in the `out of`
  figures of `\tracingstats`' string lines (the counts of strings used
  match). The difference is made of: encTeX's and MLTeX's strings
  (`-enc` and `-mltex` are refused); tex.ch messages assembled from
  different pieces that print the same text (`print_in_mode`, the
  runaway and `while scanning` texts, the format-dump counts); this
  engine's own `TeXinputs:` and `TeXfonts:`, the area names tex.ch
  [29.514] deletes; and tex.ch behaviours not ported yet, which are
  visible beyond the counts: `call_edit` ([6.84]: `E` at an error prompt opens the editor;
  this engine still holds tex.web's `You want to edit file ` message).
  Under a TCX file, pdfTeX's first 256 strings also keep `^^` for every
  character outside ASCII's printable range (enctex1.ch's `print` checks
  `xprn` instead), 390 characters more;
- tex.ch's hashed `\hyphenation` exception table (its order shows only in a format; a repeated word is handled as tex.ch does, in `web2c.ch`);
- source specials, MLTeX, encTeX, IPC (`-src-specials`, `-mltex`, `-enc`,
  `-ipc` are refused);
- the recorder lists the files the engine opens, but not the texmf.cnf files
  kpathsea reads.
