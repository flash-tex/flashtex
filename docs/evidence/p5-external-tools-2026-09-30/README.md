# P5-EXTERNAL-TOOLS: bibtex, biber and makeindex in the engine host, 2026-09-30

Lane P5-EXTERNAL-TOOLS (owner decision 2026-09-30 "4A": FlashTeX orchestrates the user's
installed TeX Live bibtex/biber/makeindex now; ports for bundles without TeX Live come later).
DESIGN.md §4.4 (the user's TeX Live), §4.5 (restricted `\write18`, project trust), §5.3–§5.5
(restart and converge, `.aux` passes). Branch `agent/kabir-claude/p5-external-tools` from
`origin/main` fa4c662c3, merged with main 296c90197.

**Hosts.** Linux: the NixOS PC (16 threads, TeX Live 2026 in `~/texlive/2026`, shared: load
5–45 during these runs), where every number below was measured unless it says otherwise.
macOS: mac-m5pro-kabir (Apple M5 Pro, macOS 26.6.2, MacTeX 2026), for unit tests, clippy and
rustfmt only. latexmk 4.87 and pdflatex 1.40.29 are the oracle only; nothing in the product
path runs them.

## What was built

* `crates/flashtex-engine/src/host/external.rs`: latexmk's rules for bibtex, biber and
  makeindex, the runs, and what the host tells the client. After a compile's `DONE` (never
  before: the edited page is not delayed) the engine thread copies what a rule reads (the
  `.aux` chain, the `.bcf`, the `.idx`: a consistent snapshot, since the next compile
  rewrites them) and a worker thread does the rest:
  * **Detection** (latexmk 4.87 `rdb_set_latex_deps`, `parse_aux`, `parse_bcf`,
    `rdb_rerun_needed`): biber when the run wrote `JOB.bcf`; otherwise bibtex for each
    `.bbl` the run read or looked for whose `.aux` the run wrote and names `\bibdata`
    (bibunits and chapterbib included); makeindex for each `.idx` the run wrote. A rule runs
    when its sources differ from its last run's (the `.aux` lines bibtex reads: `\citation`,
    `\bibdata`, `\bibstyle`, `\@input` and the files it inputs; the `.bib` files, found as
    bibtex finds them; the `.bst`; the `.bcf`; the `.idx`), or its output is missing or not
    what it made. As latexmk's default `$bibtex_use = 1`: no bibtex/biber while a `.bib` is
    missing (an arXiv source keeps the `.bbl` it ships).
  * **Runs**: the user's TeX Live programs (`resolver::discover_texlive`, as the engine finds
    TeX Live without a shell environment), in a scratch directory, bibtex and makeindex as
    latexmk runs them (`$bibtex_fudge`/`$makeindex_fudge`: in the `.aux`'s directory,
    `BIBINPUTS`/`BSTINPUTS` starting with the project and output directories), biber with
    `--input-directory` the project; stdout/stderr captured; a timeout (`--tool-timeout`,
    120 s default) that kills only that child. Outputs (`.bbl`/`.blg`, `.ind`/`.ilg`) are
    installed by an atomic rename, and only when they changed.
  * **Folding in**: when a `.bbl` or `.ind` changed, the engine thread compiles again (a
    follow-up with the same id and `"cause": "tools"`); the resident engine restarts before
    the first read of the changed file (L3) and runs its `.aux` passes (L5); then the tools
    are asked again, at most 5 rounds (latexmk's `$max_repeat`). A newer client `COMPILE`
    supersedes the cycle (it reads what the tools made and asks again).
  * **Trust** (DESIGN §4.5): off unless the `COMPILE` says `"external_tools": "auto"` (the app
    does so for a trusted project; lane P3-APP-V3 adds the trust state) or the host runs with
    `--external-tools auto`. When off, the host says once per state what it would have run.
* Protocol **3.2** (`docs/protocol/display-list-v3.md` §3, §6.2–§6.4, §10): `COMPILE.external_tools`,
  the `TOOL` message (`run`, `done` with status/ms/changed/log, `skip` with the reason,
  `settled` once per cycle), `STARTED`/`DONE` `cause`, capability `external-tools`,
  `HELLO.texmf.tools`. Tool warnings and errors are `DIAGNOSTIC`s with `"source"` and, for a
  `.bib` syntax error, the `.bib` file and line (diag-v1, PR #1255, is not on main yet).
* `tools/external-tools/xtools.py`: the parity, soundness and bench harness (below);
  `debug_open.py`, `debug_pair.py` keep one failing case for inspection.
* `crates/flashtex-engine/tests/host_tools.rs` (6 tests): bibtex then the follow-up; an
  unchanged compile runs nothing; a new `\cite` and an edited `.bib` entry; `off` skips and
  says so once; a missing `.bib` is not run; makeindex equals makeindex run by hand; later
  `.aux` passes reach the client; a tool out of time is reported and its output unused.

**Reuse before building (§1).** Evaluated: running latexmk itself (with
`-pdflatex="flashtex-host …"`): it would run full engine passes in child processes, not the
resident incremental engine, so every tool round would cost a full compile (seconds on long
documents) instead of a restart near the `.bbl` read; and it needs Perl in the product path.
Its decision logic is small, so it is re-expressed (with references to the latexmk
subroutines) and the programs themselves (bibtex, biber, makeindex) are reused unchanged.

## Results

| Target | Measured (verified, Linux) | Status |
|---|---|---|
| Detection per latexmk | 112 corpus documents: the host ran exactly the programs latexmk ran (bibtex 55, biber 44, makeindex 10, biber + makeindex 3); latexmk re-runs bibtex after every `.aux` change, the host only when the lines bibtex reads change, with the same `.bbl` | met |
| P-T2 = `latexmk -pdf`, output directory = project | **112/112** P-T2-identical and every `.bbl`/`.ind` byte-identical; 10 excluded (latexmk itself stops: pdflatex errors) — `raw/linux/xparity-samedir.*` | met |
| … output directory elsewhere (as the app's cache) | **112/112** (one first-run report was a harness artifact, a `.bbl` the arXiv source ships; fixed and re-run) — `raw/linux/xparity-sepout*` | met |
| Incremental soundness (`\cite` added, `.bib` title edited, `\index` added) | see below | see below |
| Measurements | see below | — |
| P-T1/P-T2 fixtures | **83/83, 83/83** (`raw/linux/parity-fixtures.txt`) | met |
| lockstep | **1145/1145** (main now has 1,145 cases, was 260; accounting 1, non-gating) | met |
| trip / etrip / drift | pass (`raw/linux/{trip,etrip,drift}.txt`) | met |
| Tests | host unit 7/7, display-list 9+6+1, host_tools 6/6, host_incremental 4/4 (+1 ignored), display_list_host 1/1, incremental 8/8 (`raw/linux/tests-*.txt`); clippy `-D warnings` and rustfmt clean on macOS | met |

The corpus (`raw/linux/p5x-corpus.txt`, 122 documents): the 60 arXiv e-prints of the parity
tier's arXiv manifest whose sources use `\bibliography` with their `.bib` or biblatex
(57 bibtex, 3 biber; `scripts/survey.py`), TeX Live's 52 biblatex examples
(`doc/latex/biblatex/examples`, biber; 3 of them also makeindex) and 10 testidx samples
(makeindex; `scripts/mkcorpus.sh`). The parity fixtures have no bibliography that needs a tool
(their bibliographies are `thebibliography`), so the corpus is where the tools are measured.
Each document: two copies of its sources; `latexmk -pdf -interaction=nonstopmode` on one; a
fresh `flashtex-host` with `external_tools: auto` on the other until `settled`, then an
`export` compile (pdflatex's compressed PDF from the files the resident runs left); P-T2 by
`tools/parity/tiers.compare_pt2`. `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1` for both.

Tool run times in that run (6 documents at once on a shared machine; indicative): bibtex
median 61 ms (p90 123 ms), biber 1.17 s (p90 1.68 s), makeindex 52 ms (p90 64 ms). Opening a
document until the bibliography is settled took a median 4.0 s (latexmk 4.8 s for the same
document, which also runs pdflatex 2–4 times from scratch).

## Engine and host defects found on the way

Fixed on this branch:

1. **Later `.aux` passes never reached the client** (`host/resident.rs`, `Live::emit`): a
   page a later pass of the same compile typeset again (DESIGN §5.5) was cached but not sent
   when the compile had already delivered that page index, so the client kept the first
   pass's page: unresolved `??` references after a document's first compile, and stale
   citation labels after the tools' follow-up. Now sent again; spec §6.4 says a page may
   arrive again in one compile and replaces the earlier one; `display_list_host` and
   `host_incremental` assert that instead of "each page once"; `host_tools` has a
   regression test.
2. **`\pdfmdfivesum file` / `\pdffiledump` did not consume the file in the incremental
   journal** (`system::note_whole_read`): `restart_point` could restart after the digest and
   before a later `\input` of the same file whose consumed part was unchanged, keeping the
   old digest. biblatex takes the `.bbl`'s MD5 just before inputting it, so after biber
   changed the `.bbl` the `.aux` kept the old MD5 (arXiv 2501.07512v3, `.bib` edit;
   `scripts/iserve_repro.sh` reproduces it without the socket layer).
3. **`-output-directory` equal to the project** changed `/PTEX.FileName` of included
   figures to absolute paths (pdfTeX finds them through the output directory), so P-T2
   differed from latexmk on 39 arXiv documents with figures. The host now leaves the option
   out when `output_dir` is the project, as a plain `pdflatex main.tex` runs.

Found, not fixed here (outside this lane; for the P4/P3 owners):

4. **An incremental restart can leave an output file zero-filled** (intermittent):
   arXiv 2501.07356v3 (biblatex, `\tableofcontents`), opened with tools: the follow-up
   compile after biber sometimes ends `error` with `NodalMoments.toc` all NUL bytes (same
   size as the correct file) and a truncated `.bcf` ("Fatal error occurred, no output PDF").
   `scripts/flaky.sh` (8 opens at once): 2 of 8 with the build before this branch's changes
   (so it predates them), 0 of 8 with this branch's; it depends on where the timed checkpoints
   fall, so load matters. Suspect: the restore of an output file open at the restart point
   (`incr.rs` restore / external-write splicing).
5. **`IMAGE` messages without a file after a restore** (`displaylist::image_key` →
   `dl_image_info`): once pdfTeX has written an image XObject, `delete_image` clears its entry
   in the C parts' image table; a later pass or compile restored before that point asks the
   display list for the image and gets `file: null` (key `7d583b3e…`, the same for every such
   image), so the client may draw nothing for a figure. Seen on arXiv 2501.06980v1,
   2501.07039v1, 2501.07069v1 (pages with figures after a `.bib` edit), and in cold
   multi-pass compiles; the exported PDF is right (P-T2 equal to latexmk's).

**macOS note.** MacTeX 2026's `biber` does not start on mac-m5pro-kabir ("extracting arm64
binary with lipo failed"), so biber documents cannot get a bibliography there with or without
FlashTeX; the host reports it as `TOOL` `done` `failed`/`error` with biber's message. On NixOS,
TeX Live's biber needs `libcrypt.so.1` (`libxcrypt-legacy`, `LD_LIBRARY_PATH`), an environment
fix for the test machine only.

## Not done (later)

* makeglossaries, xindy, splitindex, custom latexmk rules (latexmk runs none of them by
  default either, so the `latexmk -pdf` parity target does not need them).
* Tool diagnostics in `diag-v1` (after PR #1255 lands).
* Ports of bibtex/makeindex for bundles without TeX Live (owner decision 4A: later).
* Tool state is per host process: a new host runs each needed tool once more (an unchanged
  output is not re-installed, so no recompile follows).

## Reproduce

`scripts/p5x-gates.sh` (on the PC: `bash ~/p5x-gates.sh build unit parity lockstep trip etrip
drift xparity xsound xbench`), `tools/external-tools/xtools.py --help`.
