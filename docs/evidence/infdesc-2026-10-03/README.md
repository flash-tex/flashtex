# *An Infinite Descent into Pure Mathematics* on the new engine (2026-10-03)

Lane **INFDESC-ENGINE** (mac-claude-a, mac-m1max-a). Owner priority 2026-10-03:
FlashTeX must compile the whole of Clive Newstead's textbook *An Infinite Descent
into Pure Mathematics* (<https://infinitedescent.org>). DESIGN.md §1.1 (P-T1,
P-T2), §4.5 (restricted `\write18`), §8 (T3 corpus tiers).

**Engine under test:** `flashtex-initex` from main **`8aee5e3be`**, built by
`scripts/engine-parity.sh build` (release, `pdflatex.fmt` made by the engine
from TeX Live's `pdflatex.ini`). **Oracle:** TeX Live 2026 pdfTeX 1.40.29
(`/Library/TeX/texbin/pdftex`), MacTeX on mac-m1max-a, oracle only.

**Source:** Codeberg `cnewstead/infdesc` at commit
`48825c5e50bf311b818d666cece6e731d8a0191d` (the owner's local checkout, run
from a copy; the commit archive's tree was checked identical to it with
`diff -rq`). `infdesc.tex` plus 95 `.tex` files under `book/`. Class `book`
10pt with mathptmx, T1, babel british, tikz, tikz-cd, mdframed (TikZ),
ntheorem (thmmarks, framed), imakeidx (four indices), cleveref, hyperref,
bookmark, listings, pdfpages, textpos, textgreek, titlesec, fancyhdr, bbm,
bussproofs, pifont, lastpage, datetime, footmisc, enumitem, ulem, environ,
clipboard, type1cm and more.

## Headline (VERIFIED on mac-m1max-a)

| check | result |
|---|---|
| Completes | yes, every pass exits 0, no `!` error |
| Pages | **592**, equal to the oracle and to the shipped `infdesc.pdf` |
| Every pass's log (P-T1 normalisation, untraced) | **equal** on all 4 passes |
| Every pass's PDF | **byte-identical** to the oracle's on all 4 passes |
| P-T2 (`tiers.compare_pt2`, converged PDFs) | **pass**: 592/592 page content streams, 41/41 font programs |
| Aux files (`.aux`, `.toc`, 4× `.idx`/`.ind`/`.ilg`) | identical after every pass |
| Index generation | the engine runs `makeindex` 4× per pass itself, via restricted `\write18` (`runsystem(makeindex infdesc.idx)...executed safely (allowed).`), as pdflatex does |
| P-T1 (traced pass, `\tracingall` + box dumps) | **pass**: 592/592 box dumps equal, the 20.3 GB strict log equal ([P-T1](#p-t1-traced-pass)) |
| The parity harness, `--tier books` (new) | **P-T1 pass, P-T2 pass, L0–L3 100 %**, `--require-pt` exit 0 ([below](#parity-corpus-the-books-tier)) |
| Typesetting divergences | **none**: no engine change needed for parity |
| The app's resident host (`flashtex-host iserve`) | right output, but every compile cold and 5 passes; **fixed in #1414** (S₀'s key held the body's `\write18`s, [below](#divergences-found-and-fixed)); a PDF image's box in sp, **fixed in #1419** |

## Run sequence

The document needs no external step. imakeidx (`makeindex` option) runs
`makeindex` from inside each pass through restricted `\write18`, which is
TeX Live's default (`shell_escape=p`, `makeindex` is in
`shell_escape_commands`) and the engine's default (DESIGN.md §4.5). So the
sequence is `pdflatex` until the log asks for no rerun:

| pass | oracle (pdflatex) | engine | pages | rerun? |
|---|---|---|---|---|
| 1 | exit 0 | exit 0 | 588 | yes (1,808 warnings: undefined references on the first pass) |
| 2 | exit 0 | exit 0 | 592 | yes ("Label(s) may have changed") |
| 3 | exit 0 | exit 0 | 592 | no: converged |
| 4 | exit 0 | exit 0 | 592 | (check) PDF byte-identical to pass 3 |

Both engines ran as the parity harness runs them (`tools/parity/capture.py`):
argv[0] `pdftex` through a link, `-fmt=pdflatex -interaction=nonstopmode`, the
first line `\pdfsetrandomseed 1\relax\input{infdesc.tex}`,
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, `max_print_line=10000`,
`error_line=254`, `half_error_line=238`, default (restricted) `\write18`, each
in its own copy of the tree.

Logs were compared after `capture.normalise_log` (banner and work directory)
and `capture.split_accounting` (the DESIGN §1.1 accounting ruling). The only
accounting difference, which does not gate, is in the end-of-run capacity
totals:

```
- 57362 strings out of 467525          (oracle)
+ 57362 strings out of 467558          (engine)
- 1173333 string characters out of 5418982
+ 1173333 string characters out of 5420156
```

The strings used are equal. The totals are `max_strings` minus the format's
own strings, and the two `pdflatex.fmt` files were built separately (TeX
Live's by fmtutil on 2026-03-22; the engine's by `engine-parity.sh build`).

### The oracle's final log

The converged log (pass 3 and pass 4) has 0 errors, 4 warnings, 16 overfull
and 19 underfull boxes; the engine's log is identical:
- 2× `LaTeX Font Warning: Font shape 'T1/cmtt/m/n' in size <126.47249> not available`;
- `Package mathptmx Warning: There are no bold math fonts`;
- `LaTeX Font Warning: Size substitutions with differences`.

### The shipped `infdesc.pdf`

The PDF in the checkout has **592 pages**, the same as our oracle. It was made
on 2026-09-02 by pdfTeX 1.40.26 (`TeX Live 2025/dev/Debian`), so its fonts,
package versions, `/CreationDate` and `PTEX.Fullbanner` differ from a TeX Live
2026 run. The parity reference is our own pdflatex run (DESIGN: the oracle is
the local pdflatex), not the shipped file.

## P-T1 (traced pass)

One traced pass per engine on its converged tree (after pass 4), through
`tools/parity/capture.py`'s own `capture()` (the harness's `\tracingall`,
`\tracingonline=1`, `\showboxdepth=\showboxbreadth=2147483647`), with the
log streamed through a named pipe into its `pt1stream` fingerprint, so no
log reached the disk. Then `tiers.compare_pt1_streamed`:

| | oracle (pdfTeX 1.40.29) | engine (`8aee5e3be`) |
|---|---|---|
| traced log | 20,318,349,077 bytes, complete | 20,318,349,076 bytes, complete |
| wall time (loaded host, both at once) | 2,317 s | 2,057 s |
| shipouts | 592 | 592 |

**P-T1: pass.** All 592 shipout box dumps are equal (`boxes_equal`) and the
strict log is equal (`log_equal`). The one-byte difference in size is in the
accounting lines, which do not gate: the capacity totals above, in the
end-of-run block (606 accounting lines each).

## Parity corpus: the `books` tier

**Licence** (`LICENCES.md` in the source): dual. The source code (all `.tex`
files) is under **LPPL 1.3c** (status "maintained", maintainer Clive
Newstead); the compiled book, its text and figures are under **CC BY-SA 4.0**.
Some code fragments come from TeX - LaTeX Stack Exchange with their own
licences, each linked in place. Both licences would allow redistribution
with attribution. Even so, the project rule is that third-party sources are
never committed (`tools/parity/README.md`), so this PR commits only a
manifest that pins the source:

- `tools/parity/corpus/books.json`: tier `books`, one entry, the Codeberg
  commit archive of `48825c5`, pinned by SHA-256
  (`40570a5c…b929e`, 3,070,297 bytes; two fetches 3 s apart returned the same
  bytes; the harness run below fetched it again). `on_demand`, so a bare `corpus.py fetch` still fetches only the
  T3 tiers. The nightly workflow names its tiers, so it does not run `books`
  until the Commander adds it.
- `tools/parity/corpus.py`: an archive tier (`ARCHIVE_TIERS`) is fetched and
  verified as an e-print is (the cache's `archives/<id>`). An entry's `root`
  names the archive's top directory, which becomes the tree, so
  `\input{book/includes/_includes.tex}` resolves as in the checkout. A changed
  hash is reported, never accepted. A forge may regenerate an archive with
  other bytes; then the fetch fails loudly and the entry needs a new pin.
- Tests: `test_archive_tier_tree_is_its_root_directory` and
  `test_books_manifest_pins_a_commit_archive` (`tools/parity/test_parity.py`;
  the whole file passes, 132 tests).

Run it with:

```
python3 tools/parity/parity.py --tier books --engine <work>/eng/flashtex-initex \
  --engine-env FLASHTEX_FORMATS=<work>/fmt --engine-env FLASHTEX_POOL=<work>/eng/pdftex.pool \
  --pt on --raster none --pt1-timeout 7200 --require-pt
```

`--pt1-timeout` matters. On the first run of the tier (engine `8aee5e3be`,
default limit 1,800 s), the harness measured **P-T2 pass and L0–L3 100 %**,
and left P-T1 **not evaluated** (a harness error, never counted as a pass).
The cause was the oracle's own traced pass: pdfTeX needs about 2,300 s for the
20 GB `\tracingall` log on this loaded host, and the limit stopped it.
P-T1 itself passes ([above](#p-t1-traced-pass), measured with the harness's own
`capture` and `compare_pt1_streamed`).

**The run with `--pt1-timeout 7200` (VERIFIED):**
`parity.py --tier books ... --require-pt` exits 0, with
`books/infdesc-48825c5: P-T1=pass P-T2=pass L3 (3854 s)`. That is P-T1 1/1,
P-T2 1/1 and L0–L3 100 % (L4 not run: `--raster none`, as `engine-parity.sh`
runs the fixtures). The only difference is the non-gating accounting, as
above. Engine `8aee5e3be`; oracle pdfTeX 1.40.29 (TeX Live 2026); converged in
3 passes, each with makeindex run through restricted `\write18`.

## Divergences found and fixed

**Typesetting: none.** The engine at `8aee5e3be` matches pdfTeX 1.40.29 on
this book with no change (P-T1, P-T2, every pass's log and PDF), so there is
no lockstep case to add.

**The app's path (`flashtex-host iserve`, the resident incremental engine):
one root cause, fixed in #1414.**
Its output was right, but every compile of the book was cold and ran the
maximum number of passes:

- *Symptom* (main `8aee5e3be`): the first compile ran 5 passes, all cold
  (`pass_modes` `["cold" ×5]`, `rerun_pages` 2,956, 350–625 s on the loaded
  host), where pdflatex converges in 3. A one-word edit on page 254 ran 5
  cold passes again (421 s), with `cold_reason` "the preamble ran an external
  command (write18)".
- *Cause:* the book's preamble runs no command. The engine's own effect list
  (`FLASHTEX_EXTERNAL_EFFECTS`) holds exactly the four `makeindex` runs of the
  `\printindex`es at the end. But S₀'s key (`host::make_key`) cuts the
  journal's files, lookups and outputs to what was read before S₀
  (`rec.reads`) and copied its `barriers` whole. When the key is made from a
  finished run's journal (`Session::after_run`), it therefore held every
  `\write18` of the run, and `Key::check` rejected S₀ on every compile. A
  rejected key also makes `more_passes` skip its repeated-state test (a key
  failure counts as a change), so the passes ran to `MAX_PASSES`.
- *Fix:* the key keeps the barriers before S₀ only, `rec.effects_len` of
  them (the journal's barriers are the run's effects in order, and a cold run
  starts both empty). A `\write18` in the preamble still makes S₀ unusable.
- *After* (same host, same book): the first compile runs 3 passes (`cold`,
  `incremental`, `incremental`; 174 s); the edit compiles incrementally in one
  pass, 17.7 s, with the edited page out after 0.069 s. It re-runs from page
  253 to the end (339 pages) and does not converge earlier, because the
  makeindex runs at the end are barriers (DESIGN.md §5.3), as intended.
- *Correctness, VERIFIED:* after the edit, the host's log and every auxiliary
  file (`.aux`, `.toc`, 4× `.idx`/`.ind`/`.ilg`, `.out`) equal pdflatex's
  from-scratch converged run on the edited tree (strict log equal), and the
  host's PDFs after the cold compile and after the edit are byte-identical to
  `flashtex-initex` preview-mode runs (`FLASHTEX_PREVIEW=1`, the host's mode)
  on the converged oracle trees.
- *Regression case:* `a_write18_in_the_body_keeps_s0`
  (`crates/flashtex-engine/tests/incremental.rs`): a 4-page imakeidx document.
  Without the fix it fails (5 cold passes); with it the passes stop early,
  the edit is incremental, every compile equals a scratch run, and the control
  (a `\write18` in the preamble) still reports the preamble barrier.

**The display list: a PDF image's box in scaled points, fixed in #1419.**
INFDESC-APP found the title page's logo drawn about a point wide. The host's
`IMAGE` resource sent an included PDF page's `width`, `height`, `orig_x` and
`orig_y` as pdfTeX keeps them (`bp2int`, scaled points), where protocol §5.2
says bp. The host now sends the box as the PDF gives it, in bp. Regression
case: `a_pdf_image_box_is_sent_in_bp` (`tests/host_incremental.rs`).

### The app lane's report (INFDESC-APP, via the Commander)

| item | status |
|---|---|
| 1 HIGH: every edit cold, "the preamble ran an external command (write18)" | **fixed, #1414.** The barrier was not in the preamble: S₀'s key held the four makeindex runs at the end of the book |
| 2 HIGH: a superseded compile finishes all its passes before the newer one starts | **mostly removed by #1414; one part remains.** Incremental passes already stop at the next page or segment checkpoint when a newer COMPILE waits (`set_preempt`, `host/resident.rs`). Before #1414 every pass of this book was cold, and `Session::cold` builds its observer with `preempt: None`, so nothing could stop it (5 cold passes, minutes). After #1414 only a cold pass is not preemptible: the first open, or an edit that invalidates S₀ (a preamble edit), about 100 s on this book. Making it preemptible needs a settled cold run to keep S₀ (today a preempted cold run would start cold again on the next keystroke), so it is a P4 design question, **reported, not changed** |
| 3 MEDIUM: 5 passes where pdflatex needs 3; `converged_at` always null | **5 passes: fixed, #1414** (3 now; the key failure skipped `more_passes`' repeated-state test). `converged_at` null after an edit is **by design**: convergence may not skip an external command (DESIGN.md §5.3), and makeindex runs at the very end of the book |
| 4 LOW: `dl_image_info` sends a PDF image's box in sp | **fixed, #1419** |
| makeindex and the output directory | **reported, not changed.** The resident engine runs with cwd = the project root and `-output-directory` = the output directory, so imakeidx's `\write18{makeindex infdesc.idx}` does not find the `.idx`. pdflatex with `-output-directory` fails the same way (TeX Live 2026's makeindex ignores `TEXMF_OUTPUT_DIRECTORY`), so making the engine's `runsystem` change directory would depart from pdflatex. The host's external-tools pass (`host/external.rs`, latexmk's rules, protocol 3.2, a trusted project) is the existing remedy; whether `runsystem` should run in the output directory is a decision-9 question for the Commander |

## What remains

- **Timing** was not measured on a quiet host. Both engines ran at the same
  time on a loaded machine (other agents' builds); the wall times per pass
  (engine 34/84/45/60 s, oracle 35/89/40/61 s) are equal within noise, so they
  are no benchmark.
- **The app itself** was not driven: the host was measured through
  `flashtex-host iserve` (the resident engine the app's socket server wraps),
  not through the Mac app or its socket and display list.
- **A command's stdout shares the iserve protocol stream.** A `\write18` whose
  command prints to stdout (`kpsewhich article.cls`) puts that line where
  `iserve`'s client reads its JSON answer (found writing the regression test;
  makeindex writes to stderr, so this book is not affected). Whether the
  socket server (`--socket`) has the same problem was not checked. It is
  reported here, not fixed.
- **An edit re-runs to the end of the book** (339 pages after a page-254 edit),
  because convergence may not skip the makeindex barriers at the end. That is
  DESIGN.md §5.3 as written; making a makeindex run replayable would be a
  design change.
- **The `books` tier in nightly** is the Commander's call (`nightly.yml`). It
  needs `--pt1-timeout` of at least 3,600 s.
- **`scripts/gate.sh pr`'s fixtures step shares fixed directories**
  (`$TMPDIR/flashtex-gate-parity{,-work}`, `rm -rf` first) across every
  worktree of a machine. Two sessions' gates at once made 14 fixtures "now
  None" on #1414's first gate. A private, short `TMPDIR` avoids it: the socket
  tests fail on a path longer than `SUN_LEN`. Reported, not fixed.

## Reproduce

```
CARGO_BUILD_JOBS=4 scripts/engine-parity.sh --work <work> build
# a copy of the source per engine; then per pass, in its directory:
pdftex -fmt=pdflatex -interaction=nonstopmode -jobname=infdesc '\pdfsetrandomseed 1\relax\input{infdesc.tex}'
# (argv[0] `pdftex` from a link to flashtex-initex, with FLASHTEX_FORMATS=<work>/fmt
#  FLASHTEX_POOL=<work>/eng/pdftex.pool; or to TeX Live's pdftex for the oracle)
```

or the `books` tier of `tools/parity/parity.py` above.
