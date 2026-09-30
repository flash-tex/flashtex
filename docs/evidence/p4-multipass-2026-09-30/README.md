# D1 P4-MULTIPASS: tools before the `.aux` passes, two L5 proof fixes (2026-09-30)

Lane **D1 P4-MULTIPASS** (#2 comment 5916223765; DESIGN.md §5.5, §12 P4), claimed by flashtex-2a.
Branch `agent/flashtex-2a/p4-multipass`, based on #1296 (`agent/kabir-claude/p5-external-tools`,
fdc3cf723, which contains main 296c90197). Raw output in [`raw/`](raw/), the drivers in
[`scripts/`](scripts/). Commander ruling on the plan: #2 5916446427 ((A)–(C) approved, (B) first,
(D) only if the gate is still missed).

**Host:** Daniel's Mac (arm64, 18 cores, 48 GiB, macOS 26), shared with other lanes; release builds
with `CARGO_BUILD_JOBS=3`. MacTeX 2026 (pdfTeX 1.40.29) is the oracle. MacTeX's universal `biber`
does not start on this Mac ("extracting arm64 binary with lipo failed", as on mac-m5pro-kabir),
so both latexmk and the host use its arm64 slice (`lipo -thin arm64`) through a shim `bin`
directory (`scripts/mkshim.sh`; the host finds it via `FLASHTEX_TEXLIVE_BIN`). Same biber 2.21 for
both sides.

## What changed

### (A) The tools before the `.aux` passes (`incr::Session::set_defer`, `host::external::Job::due`)

Before: a compile ran its `.aux` passes to the end (DESIGN §5.5), then the host ran the tools
(#1296), then a follow-up compile restarted before the `.bbl` read and ran its own `.aux` passes.
On a biblatex document every one of those passes re-typesets the whole document, and the passes
before the tools are wasted: the `.bbl` the tools make changes what they typeset.

Now, before each pass after a compile's first, the session asks the host whether a tool is due
on what the last pass left (latexmk's rules, the same decision `Job::run` makes, without running
anything). If so, the passes stop there (`Report::deferred`, `DONE.deferred`); the tools run after
`DONE` as before, and the follow-up compile takes up the `.aux` and the tools' outputs in one pass
(latexmk's order: a compile, the tools, a compile). A compile whose passes stopped is owed a
follow-up even when the tools change nothing, time out or are skipped (`DocTools::deferred`).
No deferral when the policy is off, a `.bib` is missing, the program is absent, or at
`MAX_ROUNDS`.

### (B) Two L5 proof failures (`readset.rs`, `incr::dead_word`)

L5's proof (re-read the `.aux` alone, `aux_delta`, put the old meanings back, `same_words`)
rejected these, and the pass re-read the `.aux` from the `.aux` point (the whole document again).
Both were on every biblatex compile:

1. **A body other control sequences share** (`\let`, an etoolbox toggle is `\@firstoftwo` /
   `\@secondoftwo`): putting the old meaning back built a copy, so the shared list's reference
   count was one less than the old run's ("token list reference count differs (205 vs 204)").
   `Patch::share` records where the run's list was when others shared it; `apply` shares the
   list at that place (`add_token_ref`) when it holds the same body and a macro still points at
   it, else builds a copy as before.
2. **Stack high-water marks** (`max_save_stack`, `max_buf_stack`, `max_in_stack`,
   `max_nest_stack`, `max_param_stack`): statistics, read only where a push raises them (the
   overflow test there is on the pointer, which exceeds the mark whenever it reaches the stack's
   size) and by the log's capacity block, which DESIGN §1.1 reports and does not compare. Dead
   words now, for the proof and the convergence test.

Soundness cases **2030** and **2031** (`tests/incremental.rs`,
`l5_shared_bodies_and_deeper_aux_reads_equal_scratch_runs`): each compile equals from-scratch
runs byte for byte (PDF, log, aux) and the `.aux` pass restarts at the entry's first read. On the
#1296 engine the same edits re-read from the `.aux` point (`scripts/t31.py`, `raw/l5-cases.txt`).

### Page versions (host)

A page a later pass typesets again with the same display list (body, hash, fonts, images, forms,
spans) keeps its version, so a client holding it is not sent it again.

## Results

Engines: **new** = this branch at 4a5e792f5 (the host binaries in `v4`); **base** = #1296 at fdc3cf723.
Every wall-time run started at 1-min load < 4 (`scripts/gatebench.sh`, `raw/gate/load.txt`); the
load at the start of each cycle is in `raw/gate/*.result.json`. Two of the new engine's
biblatex-120 edits started at 5.8 and 5.3 (another lane's job); their times match the first's.

### Against the lane's gate

| Gate | Measured (verified) | Status |
|---|---|---|
| Output byte-identical to latexmk + pdflatex | bench documents: the exported PDF's sha256 equals latexmk's after the open and after each of 3 edits, on both documents (8/8, `raw/gate-summary.txt`); tools corpus (122 documents: 60 arXiv, 52 TeX Live biblatex examples, 10 testidx): **113/113 P-T2-identical** to `latexmk -pdf` with every `.bbl`/`.ind` byte-identical, 9 excluded because latexmk itself fails (`raw/xparity.txt`); PDFs byte-identical for 111/113. The two others (2501.07021v2, 2501.07542v1) differ by one object-stream length, and they differ identically on #1296's engine (`raw/xparity-bytes-base-2docs.txt`). | met on the bench; the 2 one-byte differences predate this branch |
| Total wall ≤ 0.5× latexmk, 120-page biber document and book tier | see the next table: **0.74×** (biblatex-120) and **0.72×** (book-300) after a `\cite` edit; 0.87× / 0.88× on a cold open | **not met** (analysis below) |
| Soundness 0 mismatches | `xtools.py sound`, 122 documents, 2 trials each: `\cite` added **155/155**; `.bib` title edited **114/120**; the 6 failures (2501.06980v1, 2501.07039v1, 2501.07069v1, both trials each) are #1295's image/form resources after a restore and are **identical on #1296's engine** (`raw/xs3h-base.jsonl` vs `raw/xs3h-v4.jsonl`). `tests/incremental.rs` 9/9 (with 2030/2031), `host_tools` 7/7 (with the new deferral test), `host_incremental` 4/4 (+1 ignored), `display_list_host` 1/1; clippy `-D warnings` clean | met for this change; #1295 open (Commander's lane) |

### Wall time (median of 3 edits; each edit is a `\cite` of a key not cited yet, mid-document)

| document | latexmk (runs) | new | base (#1296) | one pdflatex run |
|---|---|---|---|---|
| biblatex-120 (126 pp), edit | 9.07 s (pdflatex ×4 + biber; CPU 9.0 s) | **6.72 s (0.74×)**, host CPU 5.6 s | 9.05 s (1.00×), host CPU 7.7 s | 2.04 s |
| biblatex-120, open | 8.38 s | 7.28 s (0.87×) | 9.46 s (1.13×) | |
| book-300 (348 pp), edit | 13.86 s (pdflatex ×4 + biber; CPU 13.8 s) | **9.96 s (0.72×)**, host CPU 8.6 s | 14.37 s (1.04×), host CPU 13.0 s | 3.16 s |
| book-300, open | 13.15 s | 11.56 s (0.88×) | 18.03 s (1.37×) | |

Host CPU is the host process's CPU time (engine and worker threads). It does not include the
tools' child processes (biber takes 1.15–1.3 s). The book tier is `mpbench.py --style book
--pages 300`: book class, hyperref, a table of contents, chapters and sections with labels,
`\ref`/`\pageref`, footnotes, makeidx, and biblatex + biber with a 400-entry `.bib`. The repository
had no book-sized tier.

### Why 0.5× is not reached (verified with `iserve`, `raw/gate/iprobe-v4.log`)

After a `\cite` edit on biblatex-120 the new engine runs, in order:

1. the edited compile from the edit's page to the end (72 pp, 1.13 s), so that biblatex writes
   the `.bcf`;
2. biber (1.2–1.3 s);
3. the follow-up: one pass from before the `.bbl` read, taking up the `.aux` too (126 pp,
   2.0–2.2 s);
4. one `.aux` pass over the whole document (1.8–1.9 s);
5. a final `.aux` pass whose L5 patch touches no page (0.015 s).

Step 4 is needed. The from-scratch sequence (and latexmk, which runs pdflatex three more times
after biber) sees biblatex's bookkeeping in the `.aux` change after the new `.bbl`: the entry
`\blx@segm@0@0` (the refsegment's list of cited keys) gains the new key, and so do the rerun
toggle and the `.bbl` checksum. biblatex reads `\blx@segm@0@0` and appends to it at every citation
(`biblatex.sty` 11479–11481). Every page after the first citation therefore reads a changed
entry, and no read-set scheme can skip those pages. Its L5 restart is at page 0 either way: the
toggle is read in `\begin{document}`. Its L5 proof also still falls back ("hash[…] differs"):
a name the new `.aux` makes in mid-read takes a hash slot that shifts every later name's slot.
This is a third proof limitation, not fixed here. With it fixed, the restart would still be at
page 0.

So the floor is about 0.57 P + B + 2 P, against latexmk's 4 P′ + B, where P is a full pass
(1.8–2.2 s here, which matches pdflatex's P′ of 2.04 s including start-up) and B is biber
(1.2 s): about 6.1 s against 9.1 s, or 0.67×. Reaching 0.5× needs P ≈ 1.2 s, a per-page
engine about 1.6× faster than pdfTeX (lane D2 P6-THROUGHPUT). The multi-pass structure cannot
supply that. The book tier has the same structure.

Measured before this branch (checkpoint 1, load 4.1–4.6, `raw/checkpoint1-bench-base.jsonl`,
`raw/iprobe-base-biblatex120.log`): the edited compile's own `.aux` pass (1.86 s) and the
follow-up's second full pass (1.8 s) both came from the two L5 proof failures (B), and the
passes before biber were wasted (A).

## Harness fix found on the way (`tools/external-tools/xtools.py`)

`sound` compared page digests taken when each page arrived, with the forms the client held
then. A form the engine writes after its page (display-list-v3 §6.4) replaces the one the page was
digested with, so a correct client could compare unequal. Whether it did depended on how many
passes re-sent the page. With (A), 2501.07069v1's `\cite` trials failed that way on this branch
and passed on #1296's. The final page bytes and forms were equal (`scripts/d1pair.py`).
`Host.held_pages()` now digests what the client holds at the end. With it, #1296's engine and
this branch give identical results on the three failing documents (`raw/xs3h-*.jsonl`).

## Open

1. **Gate ≤ 0.5×**: not reachable by pass structure (above). It needs raw engine speed (D2)
   or a decision to measure the gate differently. For the Commander.
2. **L5 proof, third limitation**: a name made in mid-read (a new cite key's
   `\blx@assignedrefcontextbib@…`) shifts later names' hash slots, and the proof compares by
   slot ("heads a chain of the hash", "hash[…] differs"). Fixing it helps documents whose changed
   entries are read late. It does not help biblatex, whose readers are on every page.
3. **Another class seen while building 2031**: "char node differs (walking List …)" through
   `TEMP_HEAD`'s link after some `.aux` re-reads (depth 20/60/120 nestings,
   `scripts/t31.py`). It is sound (it falls back) and was not diagnosed.
4. **#1295** (IMAGE without a file after a restore) and the 2 one-byte PDF differences predate
   this branch.

## Reproducing

```
scripts/mkshim.sh                    # arm64 biber + TeX Live shim bin dir
cargo build --release -p flashtex-engine && scripts/mkeng.sh NAME
scripts/mkcorpus.sh                  # the 122-document tools corpus list
LIST=corpus.txt scripts/xrun3.sh NAME parity OUT -j 3
LIST=corpus-sound.txt scripts/xrun3.sh NAME sound OUT -j 3 --trials 2
NEW=NAME scripts/gatebench.sh && python3 scripts/gatesum.py gate NAME
ENG=... python3 scripts/t31.py '{' 5,5,6,5,6     # soundness cases 2031 (and 2030 with a 3rd arg)
cargo test --release -p flashtex-engine --test incremental --test host_tools --test host_incremental
```
