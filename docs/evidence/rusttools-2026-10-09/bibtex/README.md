# BibTeX in Rust, in-process, from the bundle alone (lane RUST-TOOLS, 2026-10-09)

Lane **RUST-TOOLS** (mac-claude-a, mac-m1max-a), Commander lanes of 2026-10-09 on #1319: no
external programs at user time. The NO-TEXLIVE audit (#1319, 2026-10-07, gap 4) found bibtex,
biber and makeindex unavailable in bundle mode. makeindex is the stacked PR below this one
([its README](../makeindex/README.md)). MacTeX's `bibtex` is the oracle only.

## What was built

| piece | where |
|---|---|
| the source of record, unmodified: `bibtex.web` (BibTeX 0.99e) and TeX Live's `bibtex.ch`, TeX Live pin `6a300188` | `third_party/bibtex/` |
| the translation: `tools/web2rust` over `bibtex.web` + `bibtex.ch` + `changes/flashtex.ch` (committed, drift-tested) | `crates/bibtex/src/generated/` |
| `changes/flashtex.ch`: bibtex.ch's C in web2rust's Pascal (XTALLOC, BIB_XRETALLOC, cvtbib.sed's setjmp/longjmp, kpathsea, getopt, fixwrites' `%c` for `xchr[]`) | `crates/bibtex/changes/` |
| the C runtime's behaviour: `eof`/`eoln`/`getc` (lib/eofeoln.c: CR and LF both end a line), `open_input`/`open_output`/`open_input_with_dirname` (lib/openclose.c, `$TEXMFOUTPUT`), kpathsea's `getopt_long_only`, `setupboundvariable`, BIB_XRETALLOC's log lines, stdio's buffering | `crates/bibtex/src/system.rs` |
| two translator features: `--translate NAME`, `const_cstring` | `tools/web2rust` |
| the engine's glue: kpathsea lookups (`Format::Bib`/`Bst`), `max_strings.bibtex` as bibtex sees it (`config_var_for`), in-process for the host's bibtex rule and restricted `\write18`, `FLASHTEX_BIBTEX=external` | `crates/flashtex-engine/src/bibtex.rs`, `host/external.rs`, `system.rs` |
| `flashtex-bibtex`: the port as a command, exactly the `\write18` path | engine bin |
| tests without TeX Live: 13 golden cases with the oracle's output; `\write18` in-process with no bibtex on PATH; the no-TeX-Live host check | `crates/bibtex/tests/`, `crates/flashtex-engine/tests/write18.rs`, `scripts/notex-host-check.sh` |

**Why web2rust.** The task asked to try it first because it is the most faithful route. It
worked: bibtex.web needed two small translator features and one change file. Every routine
is the WEB's, so no hand port can drift from it. The first comparison on `xampl.bib` was
byte-identical.

## Identity with TeX Live 2026's bibtex (VERIFIED)

Byte for byte on **exit status, standard output, standard error and every file left in the
directory** (`.bbl`, `.blg`), each case in a fresh directory (`harness/btcmp.py`). The oracle
runs as latexmk runs it: `bibtex` along PATH, in the directory of the `.aux`. Port: the
engine's `flashtex-bibtex` (release, this branch), with the engine's kpathsea over TeX Live's
tree, as in the app with a TeX Live.

| run | cases | mismatches | raw |
|---|---|---|---|
| **corpus** (`harness/corpus.py`): every one of TeX Live 2026's **397 `.bst`** on 4 database sets (`xampl`; `IEEEabrv`+`IEEEexample`; `biblatex-examples`; `tugboat`, 5,196 entries), and every one of its **79 `.bib`** with 20 common styles (plain, alpha, abbrv, unsrt, ieeetr, acm, apalike, siam, amsplain, amsalpha, plainnat, abbrvnat, unsrtnat, IEEEtran, apsrev4-2, chicago, named, elsarticle-num, agsm, jss); `\citation{*}` plus one cited key | **3,168** (oracle exit 0: 2,386, exit 2: 782) | **0** | `raw/corpus*` |
| **fuzzer** (`harness/fuzz.py`, seeds 0-4999): random `.aux` (present, missing, duplicate and differently-cased keys, `*`, 1-22 databases, missing or repeated `\bibstyle`, nested and too-deep `\@input`, malformed lines, CR LF), random `.bib` (every entry type, `@string`/`#`, `@preamble`, `@comment`, crossrefs, von/Jr. names, `and others`, accents, 8-bit bytes, syntax errors, 25,000-character fields), styles: TeX Live's, TeX Live's damaged at random (the style-file error paths, `bst_done`), or small random programs over all 37 built-ins (many globals, long functions, deep stacks); options `-terse`, `-min-crossrefs=N` | **5,000** (oracle exit 0: 105, 1: 100, 2: 4,640, 3: 148, SIGSEGV: 6, never ends: 1) | **0** | `raw/fuzz-0-5000*` |
| golden (in `cargo test`, no TeX Live) | 13 | 0 | `crates/bibtex/tests/golden/` |
| host, with TeX Live (`tests/host_tools.rs`): the host's bibtex rule against TeX Live's bibtex run by hand | 9 tests | pass | |
| **host, no TeX Live** (`scripts/notex-host-check.sh`, TeX Live hidden by `sandbox-exec`, the published bundle `9f434472…`): a document with a bibliography and an index; `.bbl`, `.blg`, `.ind`, `.ilg` against TeX Live's | 1 | 0 | |

What the fuzz cases reached, by the oracle's `.blg` (cases): a reallocation 2,167; `.bib`
syntax errors 1,994; style-file errors 4,524; crossrefs 705; "you can't pop an empty literal
stack" 148; fatal errors (`overflow`, aux files nested too deep) 148; entry-string-size
exceeded 19; files that could not be opened 620; repeated entries 973; warnings 2,667
(`raw/fuzz-0-5000-summary.txt`).

Before these final runs, the fuzzer found two things. Both are fixed, and no mismatch remains:

- **Undefined behaviour that crashes the C program.** A damaged style assigns to an entry
  string outside `ITERATE`. TeX Live's binary writes through a NULL `entry_strs` and dies of
  SIGSEGV; the port's checked index stops at the same statement. To match the oracle, a run
  that stops there (`CRASHED`) leaves what C's stdio had passed on: whole buffers of
  `st_blksize`, 4096 for a file and 16384 for a pipe. The command then dies of SIGSEGV
  itself. Seeds 1475, 2113, 2134 and 3533 are now byte-identical, including the truncated
  `.blg` and stdout.
- **A style that never ends** (seed 2400, a `while$` that never stops) is killed at the
  timeout on both sides. The comparison accepts one output as a prefix of the other, since the
  kill time differs. The two outputs agreed up to the kill, more than 2.5 GB in. The host gives
  the in-process run a deadline (`--tool-timeout`). It is checked as each style function
  starts, so a run that never ends is reported as `timeout`, as a child process would be.

**Reallocations.** TeX Live's bibtex grows its arrays and logs each growth in the `.blg`
("Reallocated wiz_functions (elt_size=4) to 6000 items from 3000."). The element sizes the
port prints are C's `sizeof`: 1 for `ASCII_code` and `stk_type`; 4 for `integer`,
`str_number`, `hash_ptr2`, `buf_pointer` and `boolean`; 8 for `alpha_file`. The fuzz cases
with a reallocation check them.

### What the C code leaves undefined, and what the port does

- An array access past its end (C: through a NULL or freed pointer): the run stops with
  `CRASHED` and the C program's partial output (above). The C program does not always crash
  there; it depends on where the bad pointer lands.
- `int_to_ASCII` with `ex_buf` passed while `buffer_overflow` grows it: C writes into the
  freed buffer. The port's `var` argument moves the array out for the call, as web2rust does
  for every `var`. Not reproducible, and not generated.

## Measurements (VERIFIED, `/usr/bin/time -l`, instructions retired; load1 250-300)

| case | TeX Live `bibtex` | `flashtex-bibtex` (own kpathsea start-up included) |
|---|---|---|
| `xampl.bib`, plain, `\citation{*}` | 580 M | 240 M |
| `tugboat.bib` (5,196 entries), plain, `\citation{*}` | 6.88 G (0.53 s user) | 1.40 G (0.10 s user) |

In the host the engine's kpathsea is already running, so an in-process run also skips the
start-up. That start-up is about 0.5 G instructions of the TeX Live figures (reading
texmf.cnf). Belief, not measured: the rest of the gap is C stdio's locked `getc`/`putc` per
character.

## Bundle mode (VERIFIED) and a gap in the published bundle

With TeX Live hidden and the published bundle `texbundle-tl2026-1`, the host's bibtex finds
`.bst` files from the bundle. `plainnat` from natbib gave TeX Live's `.bbl` and `.blg` byte for
byte. **The bundle lacks TeX Live's `bibtex` package**: `plain.bst`, `alpha.bst`, `abbrv.bst`,
`unsrt.bst` and `xampl.bib`. The bundle packs the packages that compiled documents read, and
bibtex's reads were never recorded. The bundle does carry the styles of natbib, IEEEtran,
amscls, elsarticle, revtex, splncs04 and ACM (37 `.bst` in all). So
`\bibliographystyle{plain}` fails in bundle mode until a bundle carries the package. The
no-TeX-Live check puts `plain.bst` beside its document for that reason
(`fixtures/notex-tools/README`).

**Fixed in `texbundle-tl2026-2`** (lane RUST-TOOLS, follow-up PR): `tools/bundle/tl2026/extra.txt`
adds TeX Live's `bibtex` and `makeindex` packages, common BibTeX styles a document names only in
`\bibliographystyle` (urlbst, apacite, chicago, harvard), common biblatex styles
(biblatex-ieee, biblatex-apa, biblatex-ext) and nomencl. The no-TeX-Live check then runs with
no `plain.bst` of its own.

## Licence (recorded)

- `bibtex.web`: "available under the same terms as Donald Knuth's TeX program", and the
  master file should stay intact, with changes in change files. It is unmodified in
  `third_party/bibtex/`, and the changes are change files. `bibtex.ch` is TeX Live's and in
  the public domain.
- `crates/bibtex/LICENSE`: the translation is under BibTeX's terms, and FlashTeX's own code in
  the crate is MIT. The engine (GPL) links it, and nothing else does.
- The `.blg` says "This is BibTeX, Version 0.99e (TeX Live 2026)", as TeX Live's does, because
  the output is byte-identical. Whether a product may print that name is for the owner's §3
  legal review.

**Open for the owner's §3 legal review before any public release:** the `.blg` banner above
(TeX Live's program name and distribution in FlashTeX's output), and, for the makeindex port
this PR stacks on, whether the MakeIndex Distribution Notice may be combined with GPL-2
distribution of the engine that links it ([its README](../makeindex/README.md)).

## Hardening after review (2026-10-09)

The port reads an input whole where the C program streams it, so `refs.bib -> /dev/zero` (or an
`.aux` linked there, through `\write18{bibtex ...}`) would fill a host's memory in seconds.
Now every input (`.aux` and `\@input` files, `.bib`, `.bst`) must be a regular file after
following links and at most `MAX_INPUT` (64 MiB); otherwise `bibtex: refusing to read NAME:
not a regular file` (or the size) on standard error, and the open fails as for a file that
does not exist ("I couldn't open ..."). A directory still opens as empty, as with `fopen`.
This differs from TeX Live only where its program would hang or exhaust memory. kpathsea's
answers in the engine's lookup are taken only when they are regular files, as the host rule's
are, and the host rule's own reads and hashes refuse the same files (`host/external.rs`,
`read`). With `FLASHTEX_CONFINE_READS` (Live Share) every read also goes through the
engine's confinement (`system::tool_read_ok`; the document's names obey the name rule).
Tests: `crates/bibtex/tests/limits.rs` (`/dev/zero` as `.bib` and as `.aux`, a FIFO, a sparse
64 MiB + 1 `.bib`), `tests/write18.rs` (`bibtex_refuses_dev_zero`). A panic in the port was
already caught (`CRASHED`); the host's tools thread now also catches any panic of a rule's run,
so `ToolsDone` is always sent.

## biber: options for the owner

biber is the backend of biblatex: about 25k lines of Perl. It relies on Text::BibTeX (C
btparse), Unicode::Collate (UCA with CLDR tailorings), XML::LibXML and others. TeX Live ships
it as an 83 MB self-contained executable (PAR::Packer); biber 2.21 pairs with biblatex 3.21
through the `.bcf` format version. In the arXiv sample of lane P5-EXTERNAL-TOOLS
(`docs/evidence/p5-external-tools-2026-09-30/raw/linux/xparity-sepout.txt`), 47 of 122
documents ran biber, against 56 that ran bibtex.

1. **As now: TeX Live's biber, external.** Without TeX Live, a biblatex document keeps the
   `.bbl` it ships (arXiv sources do), or gets none. The host now says why.
2. **Fetch biber with the bundle** (recommended next step). Fetch the pinned TeX Live 2026
   biber binary per platform, on demand, with digest and consent like the bundle, and run it as
   a separate process under the same trust rules. Byte-identical by construction, and little
   engineering. But it is a separate process and not Rust, so it is not "no external
   programs". Its licence is Artistic 2.0 / the Perl licence, separate process only.
3. **Port biber to Rust.** This is the only route to fully independent biblatex.
   Byte-identical `.bbl` output needs biber's sorting to be exact, and biber's sorting is
   Unicode::Collate with tailorings. ICU4X's collator is not the same algorithm byte for
   byte. It would take months and carries high risk. Port the data model and `.bbl` writer
   first, and gate it on an arXiv corpus.
4. **Embed a Perl interpreter** (libperl, or perl compiled to WASM) to run biber's own source
   in-process. Byte-identical, but its XS modules (btparse, libxml2, Unicode::Collate) must
   also be built for every platform. It adds a large runtime and does not suit the
   iPad/WASM targets.

Suggested order: 2 now (it closes the bundle-mode gap), then 3 only if the owner wants biblatex
with no external program at all.

## Reproduce

```sh
cargo build --release -p flashtex-engine --bin flashtex-bibtex
P=$PWD/target/release/flashtex-bibtex
H=docs/evidence/rusttools-2026-10-09/bibtex/harness
python3 $H/corpus.py --port $P --tier all --jobs 4 --out corpus.jsonl
python3 $H/fuzz.py --port $P --seeds 0:5000 --jobs 4 --out fuzz.jsonl
python3 $H/golden.py .        # rewrites crates/bibtex/tests/golden from the oracle
scripts/bibtex-regenerate.sh  # src/generated/ from bibtex.web
```
