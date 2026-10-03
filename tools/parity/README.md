# Parity scoreboard: how close is FlashTeX to pdfLaTeX, measured

Owner: lane PARITY-SCOREBOARD (kabir-claude, mac-m5pro-kabir). Created
2026-09-21; DESIGN §1.1 tiers added by lane P0-PARITY-TIERS (flashtex-2a).
Python 3 standard library, plus `qpdf` (Apache-2.0) run as an external
program for P-T2. **pdflatex (MacTeX) is the oracle and never runs in the
product path.**

The engine under test is `--engine <bin>` (`--flashtex` still works):

- **the shipped command line** (default), `flashtex build`, which makes the
  same exact-route PDF the Mac app exports. It runs on the untouched source
  tree;
- **any pdfTeX-compatible command line**: pdfTeX itself for the self-test,
  later the new engine. It runs as `<bin> -fmt=pdflatex` on a copy of the
  tree, to convergence, like the oracle.

The kind is detected from the first line of `<bin> --version`;
`--engine-kind` overrides it.

The question this answers: "can we measurably know that we have full parity
with pdflatex?" The headline per corpus tier is the two **DESIGN §1.1 gating
tiers, P-T1 and P-T2**. Below them are the older levels L0–L4 (the share of
documents at L3 among them), plus the breakdown that says what stops the rest.

## Gating tiers (headline; DESIGN §1.1)

| tier | passes when | how it is measured |
|---|---|---|
| **P-T1** | every `\shipout` box dump and the whole `\tracingall` log are identical to the pinned pdfTeX 1.40.29's, in PDF mode | `capture.py` runs one traced pass after convergence. The pass sets `\tracingall` (which includes `\tracingoutput`), `\tracingonline=1`, `\showboxdepth=\showboxbreadth=2147483647` and `\nonstopmode`, with `max_print_line=10000`, `error_line=254` and `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Normalised: the banner (lines before the `**` echo) and the work-directory path. Then, per the DESIGN §1.1 ruling, only end-of-run accounting is removed: `\\tracingstats` `Memory usage before: …` lines (whole-line match), the `Here is how much of TeX's memory you used:` and `PDF statistics:` blocks, and the byte count in `Output written on … (N pages, B bytes)`. The page count stays compared. Each removed line must have one of the exact shapes pdfTeX 1.40.29 prints, in pdfTeX's order, each at most once (`capture.ACCOUNTING_BLOCKS`, taken from the fixture logs). The headers and `Output written` count only once, in the trailer after the last shipout. A `Memory usage` line counts only when a shipout still owes it. Anything else is compared, including ` junk`, ` Overfull …` and a header in the middle of the log. The removed lines form a separate **non-gating accounting check** in the report. Box dumps are split at `Completed box being shipped out` |
| **P-T2** | identical embedded font subsets and identical page content streams | Both PDFs go through `qpdf --qdf --normalize-content=y --object-streams=disable` and are parsed with `tools/visual-oracle/pdftext.py`. Fonts: the multiset of (name without subset tag, subtype, hash of the decoded font program with subset tags normalised). Pages: content-stream bytes, resources and media box. Every indirect object is compared by a hash of its content with references resolved, so object numbers never matter |

**P-T1 is n/a for the flashtex CLI.** It isn't a TeX engine and writes no box
dumps or `\tracingall` log, and the report says so instead of printing 0.
Its P-T2 is measured.

**How both TeX engines run** (the oracle, and a TeX `--engine`), identically:

- **One `\write18` setting for both.** It is `capture.SHELL_ESCAPE`. By
  default there is no flag, so each engine runs in its own default mode:
  restricted, as in TeX Live's pdflatex (owner decision #1209, DESIGN §4.5).
  That gives ` restricted \write18 enabled.` and `\pdfshellescape` = 2,
  which l3kernel's `\sys_if_shell` reads. `--shell-escape-flag` takes
  `default`, `-shell-restricted`, `-no-shell-escape` or `-shell-escape`. The
  setting is part of the oracle cache key.
  Give the value with `=`, as in `--shell-escape-flag=-shell-restricted`,
  because argparse reads a separate `-shell-restricted` as an option.
- **Converted figures are the oracle's.** Under restricted `\write18`,
  epstopdf converts `x.eps` to `x-eps-converted-to.pdf` during the run.
  Ghostscript stamps each conversion with the time and a fresh id. pdfTeX
  copies those into the including PDF and prints the file's date in the log,
  so two engines that each convert differ in P-T1 and P-T2 for no
  typesetting reason. The oracle's conversions (`tiers.GENERATED`) are kept
  in its cache entry and copied, with their times, into the candidate's tree
  before its first pass, so both see the same files and neither converts
  again. The templates tier copies TeX Live files with their times for the
  same reason.
  A conversion's input that the run wrote itself (grfguide's `filecontents`
  `a.eps`) is kept with it, because epstopdf logs the input's date.
  So is a conversion the source ships but the run redid: epstopdf converts
  again when the EPS is a second newer than the shipped PDF, and an unpacked
  e-print's file times are its unpack times. A cache entry made before this
  rule (`tiers.GENERATED_V`) is made again, but only for a tree that ships
  a conversion. `corpus.unpack` gives each file its archive time (a time
  the OS can't set keeps the unpack time), so a re-unpacked e-print never
  makes its EPS newer than a cached seed. Trees unpacked before this rule
  are unpacked again (`corpus.UNPACK_V`), and the oracle entries of those
  that ship a conversion are made again (`tiers.GENERATED_V` 3).
  A seeded candidate finds the conversions up to date, so it can pass even
  if its own conversion would fail (no shell escape, no Ghostscript). Each document's
  P-T record gives `seeded_conversions`, and the report and summary line
  count the seeded documents.
- **The random seed is pinned** (DESIGN §4.5). pdfTeX seeds
  `\pdfuniformdeviate` from the clock, so l3kernel's `\int_rand` and pgf's
  random numbers differ from run to run. Every pass of every TeX engine the
  harness runs (both oracles and a TeX `--engine`) starts its command line
  with `capture.SEED` (`\pdfsetrandomseed 1\relax`), then `\input{<entry>}`.
  Both logs echo that line equally. The P-T oracle cache key includes the
  seed and `tiers.ORACLE_CACHE_V`, which is bumped whenever what an entry
  holds changes.
- **argv[0] is exactly `pdftex`.** Each engine runs through a `pdftex`
  symlink in its own bin directory, and that directory goes first on `PATH`
  so kpathsea resolves the real binary. pdfTeX prints argv[0] in warnings,
  e.g. `pdfTeX warning: pdftex (file ./fig.pdf): …`. Setting the name
  before the run means no program-name token is ever normalised, so nothing
  can hide behind a normaliser.
- **Environment.** `--engine-env KEY=VALUE` (repeatable) goes to the TeX
  `--engine` only, never the oracle; for example
  `FLASHTEX_FORMATS=<dir with pdflatex.fmt>` and `FLASHTEX_POOL=<pdftex.pool>`.
  No `FLASHTEX_*` variable is inherited from the calling shell.

Expected data comes only from the oracle pdfTeX (`--oracle-pdftex`, default
`/Library/TeX/texbin/pdftex`; the run warns if it isn't 1.40.29). It is never
compared against the committed references, which include TeX Live 2025
builds. The oracle run is cached under `<cache>/pt-oracle/`, keyed by the
source-tree hash, the pdfTeX version and the capture settings. The
normalised log is stored gzipped. `--pt pt2` skips the traced pass;
`--pt off` skips both tiers.

**Run paths in the logs** (`capture.workdir_subs`). Before P-T1 compares two
logs, each run's work directory becomes `<WORKDIR>` and its TEXMFVAR becomes
`<TEXMFVAR>`. TEXMFVAR is where mktexpk writes PK fonts, and pdfTeX names each
PK font it embeds by that path. If TEXMFVAR is set, the harness uses its
value; if it is unset, it uses kpathsea's default, from the `kpsewhich` beside
`--oracle-pdftex`. Either way, the same directory normalises the same way.
A cached oracle log is also normalised for both the current TEXMFVAR and
kpathsea's default when it is read (`capture.cached_log_subs`), so an older
entry cached with TEXMFVAR unset still matches a run that sets it elsewhere,
such as `scoreboard-run.sh`.

**`--regenerate`** re-runs only the fixtures tier's L oracle (pdflatex with
the local TeX, instead of the committed PDF). It never re-makes a P-T oracle
entry. Those entries are invalidated by `tiers.ORACLE_CACHE_V`, which is part
of the key, and by `tiers.stale_entry`, which also covers a streamed entry's
`pt1stream.V`. To re-make one entry, delete its `<cache>/pt-oracle/<k[:2]>/<key>`
directory.

**Traced logs too big to hold: streamed P-T1.** Some e-prints trace to
tens of gigabytes (arXiv 2501.08663v2: 23.7 GiB from pdfTeX). A traced log
over `--pt1-max-log-mb N` (default 256; 0: no budget) is never read whole.
It is read once, as a stream, into its P-T1 *fingerprint* (`pt1stream.py`,
constant memory: about 20 MiB whatever the log's size), and
`tiers.compare_pt1_streamed` compares two fingerprints:
- **The same verdict as the in-memory compare, on every input.** The rules
  are `capture.py`'s own code (`workdir_subs`, `banner_end`,
  `Accounting.step`, `BoxSplitter`), not a second normaliser. Only the
  driver differs: a stream can't look ahead to the last shipout or the `**`
  line, so it runs those as hypotheses that the stream resolves (see
  `pt1stream.py`). The strict log and each box dump are compared by SHA-256
  of exactly the text `split_accounting` and `split_boxes` produce; the
  accounting lines are kept whole, so the non-gating accounting report is
  unchanged. A failure gives the first differing shipout exactly and the
  strict-log lines (a 4 MiB segment) that hold the first log difference,
  not the line's text.
- **The fast path.** A run of lines where no rule can fire (no shipout, no
  owed `Memory usage`, no block header or `Output written`, no blank line in
  an open box) goes to the hashes as one text. Every line still feeds them.
- **No log reaches the disk.** The oracle's traced pass always writes its
  log into a named pipe the harness reads while pdfTeX runs. The oracle
  cache keeps a small log's normalised text (`log.gz`, as before) or a big
  log's fingerprint (`fingerprint.json`); an entry cached before streaming,
  over the budget with neither, is made again. A candidate's traced pass
  uses a pipe too: streamed from the first byte when the oracle's log is
  over the budget, else held in memory up to the budget and streamed past
  it, so a candidate that traces far more than the oracle (a runaway loop
  until the time limit) writes nothing to the disk. A candidate over the
  budget that differs fails P-T1.
- **The budget** is 256 MiB (it was 1024) because the in-memory compare of
  two *differing* logs holds about 13 times one log (5.4 GiB measured for
  the 436 MB beamer-visuals fixture log), and processes must stay under
  6 GB. The budget no longer decides whether a document is evaluated, only
  whether a failure quotes the first differing line.
- **Time limit** (`--pt1-timeout S`, default 1800): a traced pass gets
  `max(S, oracle log bytes / 8 MiB/s)`. A pass it stops is a **harness
  error**, never a pass: a candidate's fails P-T1, an oracle's leaves the
  document not evaluated, and both are counted (`harness_errors`) and
  reported. A cached oracle entry that a shorter limit stopped is made again.

`--pt1-skip [tier/]ID` still reports a document's P-T1 as not evaluated
(its oracle is never traced), but size is no longer a reason to use it.
The 2026-09-29 arXiv scoreboard skipped 2501.07413v3, 2501.08663v2,
2501.08950v2 and 2501.10183v1 for size; they need no `--pt1-skip` now.
The summary counts the documents P-T1 compared as streams (`streamed`).

**Oracle work directories.** `tiers.oracle` runs pdfTeX in
`<cache>/pt-oracle/<key>/work-<pid>` and removes it on every exit path:
success, an exception, the time limit, and SIGTERM (a worker's handler
unwinds the document it is scoring, which kills its engine and runs each
cleanup; parity.py's own SIGTERM handler passes the signal to its workers).
A worker killed with SIGKILL runs no cleanup, so `parity.py` sweeps, at
startup, every `work-<pid>` and `*.<pid>.tmp` whose process is no longer
alive.

**A worker that dies** (killed for memory, say) breaks the pool, and every
unfinished document fails with it. Those documents run again on a fresh
pool. If that breaks too, the rest run one per pool. A document whose own
worker dies is recorded as failed at every level and tier (`worker_died`),
never excluded, so no denominator shrinks. The report is written, but
`parity.py` exits 3 and makes or checks no baseline, so no gate can use the
run.

**A traced pass without the end of its log** is reported as a timeout when
the capture's time limit (`--pt1-timeout`) stopped it (a harness limit, class c in
`engines.py`). Otherwise it is reported as a crash (the engine's, class b).

**Capture adapter.** `capture.py` exposes
`capture(tex_path, engine_bin, workdir, *, fmt=None) -> Capture(log, boxes, pdf_path)`
(plus the harness's `stream=` and `timeout=`, and a Capture's `size`, `complete`,
`fingerprint` and `timed_out` for a streamed log).
That is the signature `tools/lockstep/run.py` will expose (P0-LOCKSTEP-HARNESS;
#2 comments 5885107001 and 5885140240). Until lockstep lands, `capture.py` is a
minimal stand-in marked `TODO(lockstep)`. Then it becomes an import, so the
repository has one capture, not two.

**Self-test:** `--engine /Library/TeX/texbin/pdftex` must score 100% on P-T1
and P-T2 across the fixtures tier.

## Levels L0–L4 (per document, cumulative, strictest last)

| level | passes when | how it is measured |
|---|---|---|
| L0 | it compiles | pdflatex compiles it with `-halt-on-error` (only then is it in the denominator) **and** `flashtex build --json` exits 0 with `summary.errors == 0` and writes a PDF and a display list |
| L1 | same page count | reference PDF page tree vs display-list pages |
| L2 | same glyphs | per page, the multiset of glyph characters is equal. Reference glyph names (from `/Differences`, or the embedded Type 1 program's built-in encoding for `cm*`/`msbm`/`cmex`) and candidate cluster text are both reduced to NFKD Unicode (`glyphkeys.py`) |
| L3 | every glyph in place | a one-to-one matching of same-character glyphs with \|dx\| ≤ 0.5 bp and \|dy\| ≤ 0.5 bp covers every glyph on every page. Math-extension glyphs (`cmex`, `lmex`) are held on x only, because a cmex glyph hangs from its origin and an OpenType MATH variant sits on the axis. Their vertical allowance is 2 em, plus the stack height for extensible delimiters |
| L4 | pixels match | both PDFs rasterised at 144 dpi grey (`pdftoppm`, else Ghostscript). A pixel differs when \|Δ\| > 64 grey levels, and each page may have at most 0.02 % of its pixels differing |

For a TeX engine, L0 means exit 0 with a PDF. Its glyphs are read from that
PDF exactly as the reference's are.

A document "at L3" also passed L0–L2. The report also gives each check's
independent pass rate, the glyph-position error distribution (p50/p95/max of
max(\|dx\|,\|dy\|) over glyphs that `rank.aligned_pairs`, the word alignment
of `tools/visual-oracle/rank.py`, pairs), the first diverging page, and
**blockers**: what stands between each document and its next level.

## Tiers

- **fixtures**: `fixtures/real-world` + `fixtures/divergence-probes` against
  their committed reference PDFs. Deterministic, needs no TeX, and takes about
  3 s. CI runs it on macOS against `baseline-fixtures.json` (no document may
  drop below its recorded level).
- **arxiv**: `corpus/arxiv-2025-01.json`, 149 version-pinned e-prints (10 per
  primary category across math, cs, hep-th, quant-ph, cond-mat, astro-ph and
  gr-qc, submitted 13–17 Jan 2025), each with its SHA-256. `corpus.py
  select-arxiv` records how they were drawn.
- **templates**: `corpus/templates-texlive-2026.json`, 20 sample documents
  shipped in TeX Live 2026 (IEEEtran, acmart, amsart/amsproc/amsbook,
  revtex4-2, elsarticle, llncs, tufte, moderncv, beamer, `sample2e`,
  `testmath`, `amsldoc`), pinned by hash.
- **packages**: `corpus/packages-texlive-2026.json`, 92 documents under
  TeX Live 2026's `texmf-dist/doc`, one per package for 92 of the 98
  packages on the M1 list (amsmath, xcolor, geometry, pgfplots, hyperref,
  siunitx, microtype, beamer, minted, …), pinned by hash. Each file loads
  its package directly and compiles with pdflatex alone under TeX Live's
  restricted `\write18`, which is the harness's `default` mode (at most 3
  passes, at most 100 pages). Don't run it with `-no-shell-escape`: minted
  needs `\write18`.
  `copy_dir` copies the file's whole directory and `files` names the
  neighbours it needs. The `skipped` list gives the 6 packages with no such
  file and why (biblatex needs biber, background's only loader is too large,
  …). An entry's `pt1_skip` says why pdfTeX's own `\tracingall` log differs
  between runs, even with the pinned seed (tabu's `\pdfelapsedtime`), so
  its P-T1 is reported as not evaluated, never as passed. P-T2 and L0–L4
  are still measured. The Muse M1 lanes (daniel-muse-lead) drew the tier
  and #2 reviewed it.

- **nightly-5k** (DESIGN §8 T4): `corpus/nightly-5k.json`, about 5,000
  version-pinned e-prints, see below.

- **books**: `corpus/books.json`, whole books from their authors' public
  repositories, each a forge's commit archive pinned by commit and SHA-256.
  An entry's `root` is the archive's top directory, which becomes the tree,
  so relative `\input`s resolve as in the checkout. The first is Clive
  Newstead's *An Infinite Descent into Pure Mathematics* (592 pages; source
  LPPL 1.3c, book CC BY-SA 4.0; owner priority 2026-10-03,
  `docs/evidence/infdesc-2026-10-03`). It is `on_demand`, so it runs only
  when named (`--tier books`). Its traced pass writes a 20 GB log and takes
  pdfTeX about 2,300 s, so give it `--pt1-timeout 7200`: the default 1,800 s
  leaves its P-T1 not evaluated.

- **beamer**: `corpus/beamer.json`, 24 slide decks (lane BEAMER-V3,
  `docs/evidence/beamer-v3-2026-10-03`). Two kinds of entry:
  - eleven `tl-*` examples shipped in TeX Live 2026 (metropolis's
    `demo.tex`, the conference talk, the lecture in beamer and article mode,
    three ornate `solutions` talks, four emulations), copied from
    `texmf-dist/doc` and pinned by SHA-256 like the templates tier. Beamer's
    user guide is left out: pdflatex cannot compile it from the installation
    (its theme pictures are not shipped).
  - thirteen `v3-*` decks committed under `fixtures/beamer-v3` (themes,
    overlays, handout, notes, graphics, TikZ, bibliography,
    `allowframebreaks`, 16:9, `beamerarticle`, a 118-page deck). A `repo`
    entry names the deck's file in this repository instead of a TeX Live
    path; `corpus.py` copies its whole directory into the cache and copies
    it again when the directory's content changes. Git pins it, so it has
    no `sha256`. These decks are outside `fixtures/real-world`, so they are
    not in the gated fixtures tier and have no committed reference: the
    reference is made by the local pdflatex, as for every non-fixture tier.

  It is `on_demand`: it runs only when named (`--tier beamer`), with
  `--pt1-timeout 3600` (the 118-page deck's traced pass is the longest).
  The ten `fixtures/real-world/beamer-*` decks stay in the fixtures tier.

Third-party sources are **never committed**. `corpus.py fetch` downloads them
into `$FLASHTEX_PARITY_CACHE` (default `~/.cache/flashtex-parity`), verifies
the hashes and unpacks them. It sends at most one arXiv request every 3 s.

## Nightly corpus (T4)

DESIGN §8 T4 runs every night on the owner's NixOS PC (`.github/workflows/nightly.yml`,
job `corpus-t4`). `nightly.py` drives `parity.py`; it is not a second harness.

- **Corpus.** `corpus/nightly-5k.json` holds 20 primary categories × 10 years
  (2016–2025) × 25 e-prints. It was drawn by `corpus.py select-arxiv-grid`.
  Each (category, year) cell draws from a 14-day window that starts on a day
  of that year chosen by SHA-256 of the seed, category and year. Within the
  window it takes the oldest e-prints that are TeX source with a top-level
  file. The rule, the seed and every cell's query and skip counts are in the
  manifest's `selection`. Each entry is pinned by versioned id and SHA-256.
  The draw on 2026-09-29/30 filled all 200 cells: 5,000 e-prints, 9.3 GB.
  It skipped 246 PDF-only e-prints and 47 others (no top-level file, or no
  source served, a 404).
  The manifest is `on_demand`, so a bare `corpus.py fetch` skips it and does
  not start hours of polite downloading. Each shard fetches only its own
  documents, at most one arXiv request every 3 s.
- **Tiers.** `nightly-5k` plus the T3 tiers `arxiv`, `templates` and
  `packages`. A tier with no manifest yet is skipped with a notice.
- **Shards.** `nightly.py run --shards 50` runs `parity.py --shard K/50` one
  shard after another. Shard K takes every 50th document from the K-th, so
  each shard is a cross-section of the corpus. The runner wipes the job's
  work directory every job, so state lives in `$FLASHTEX_NIGHTLY_HOME`
  (default `~/.cache/flashtex-nightly`). A finished shard's `scoreboard.json`
  and `documents.json` are kept under a run key. The key is the SHA-256 of:
  - the engine binary;
  - every module the scoring imports (`tools/parity`, `tools/real-world-corpus`,
    `tools/visual-oracle`);
  - the contents of what `--engine-env` names (the formats directory, the pool);
  - the oracle's identity: the pdfTeX binary, the TeX Live root and its
    `tlpkg/texlive.tlpdb`;
  - the manifests and the settings.

  A stopped job therefore resumes at its first unfinished shard. A shard with
  an unmeasured document (a failed fetch, a harness error) is scored again.
  `--deadline-minutes` stops starting shards in time to upload the artifact.
- **Cost.** P-T2 and L0–L4 run on every document. P-T1 needs a traced pass
  whose log can run to GBs, so it runs on a fixed pseudo-random sample:
  `--pt1-sample nightly-5k=0.05`, which is about 250 documents. A document is
  in the sample when SHA-256 of `flashtex-pt1-sample/1/<tier>/<id>`, read as a
  fraction, is below 0.05 (`parity.in_pt1_sample`). The sample is the same
  every night, so the traced oracle logs stay cached. The other tiers get
  P-T1 on every document, as in T3.
- **Memory bound.** An in-memory P-T1 comparison holds both traced logs. Its
  peak resident size was measured at 7.5 and 7.6 times the log size, on
  logs of 210 and 252 MiB (`parity.PT1_MEMORY_FACTOR` = 9). Two limits bound
  the total:
  - `--pt1-max-log-mb 64`: logs up to 64 MiB are compared in memory. A
    larger one (e-prints trace to 25 GB) is compared as a stream, in
    constant memory (`pt1stream.py`, the same verdict). So the budget bounds
    memory without leaving any document's P-T1 not evaluated.
  - `nightly.py run` refuses to start when the bound is over
    `--memory-budget-gib`: `-j` × (1 GiB, an allowance per worker, not a
    measurement, + 9 × the budget). The workflow sets the budget to 70% of
    the PC's `MemAvailable` at the start of the job. Its bound is
    8 × (1 + 9 × 64/1024) = 12.5 GiB.
  - A run without a budget is refused: one e-print's log would be read into
    memory whole.
- **Oracle on the same host.** The references are made by the runner's own
  pdfTeX 1.40.29 and pdflatex (TeX Live 2026) and cached by source hash.
  Nothing expected is committed.
- **Ratchet.** `nightly.py ratchet` fails the job when any of these happens:
  - a document drops below its recorded level;
  - its P-T2 or P-T1 goes from pass to fail, or from pass to not evaluated;
  - a tier has more documents excluded by the oracle than the baseline
    recorded.
- **Fixed denominator.** Every document of the run's tiers (after `--spread`)
  must come back, measured or excluded by the oracle. A document that is not
  returned fails the ratchet. So does one that could not be fetched (a
  failed download, a SHA-256 mismatch) or scored (a harness error), and so
  does a baseline document that has left the corpus.
- **Host identity.** The baseline is
  `$FLASHTEX_NIGHTLY_HOME/baseline/<host label>.json`. The label is not an
  argument. `host_identity` derives it from the machine (`/etc/machine-id`,
  or the Mac's IOPlatformUUID) and the oracle's TeX Live root. The oracle's
  identity is in the fingerprint: the pdfTeX binary's SHA-256 and the
  `texlive.tlpdb` SHA-256, which changes with every `tlmgr update`. The check
  refuses results measured on another machine, a baseline of another
  machine, a Mac-recorded baseline on Linux, and a changed TeX Live.
- **Recording.** `--record` is accepted in CI only in a `workflow_dispatch`
  of `main`; nightly.py checks `GITHUB_EVENT_NAME` and `GITHUB_REF` itself,
  as well as the workflow's `corpus_record_baseline`. Outside CI it needs
  `--local-proof`, and CI refuses such a baseline. A run with unfinished
  shards, unreturned or unmeasured documents, or shards measured under
  different settings is never recorded. Improvements are reported but never
  recorded automatically.
- **Artifact** `corpus-t4`: `summary.md` has the per-tier P-T1/P-T2/L0–L4
  table, the classification (a)–(e) of every document that is not a full
  pass (as in `reports/arxiv-scoreboard-*`) and the top root causes, with
  numbers folded so that one difference in many documents ranks once.
  `summary.json`, and `documents.json` with one small record per document.
  `ratchet.json`. Never logs.

```sh
python3 tools/parity/nightly.py make-formats --engine target/release/flashtex-initex \
    --pool crates/flashtex-engine/pdftex.pool --out "$FMT"
python3 tools/parity/nightly.py run --texbin "$TEXBIN" --shards 50 --tier nightly-5k --tier arxiv \
    --tier templates --tier packages --pt1-sample nightly-5k=0.05 --pt1-max-log-mb 64 -j 8 \
    --memory-budget-gib 24 --engine target/release/flashtex-initex \
    --engine-env FLASHTEX_FORMATS="$FMT" --engine-env FLASHTEX_POOL="$PWD/crates/flashtex-engine/pdftex.pool" \
    --out "$OUT" -- --texmf "$TEXMF"
python3 tools/parity/nightly.py ratchet --results "$OUT"                          # check
python3 tools/parity/nightly.py ratchet --results "$OUT" --record --local-proof   # outside CI only
```

## Running it

```sh
cargo build --release -p flashtex-cli --bin flashtex   # -> target/release/flashtex
python3 tools/parity/parity.py --tier fixtures                      # P-T2 + L0-L4 for the CLI
python3 tools/parity/parity.py --tier fixtures --engine /Library/TeX/texbin/pdftex --raster none   # self-test
python3 tools/parity/corpus.py fetch                                # once; ~450 MB of e-prints
python3 tools/parity/parity.py --tier fixtures --tier arxiv --tier templates -j 10
python3 tools/parity/parity.py --tier packages -j 2                   # traced logs to GBs: keep -j low
#   -> docs/evidence/parity-<UTC date>/{report.md,scoreboard.json,documents.json}
python3 tools/parity/parity.py --tier fixtures --raster none --check-baseline tools/parity/baseline-fixtures.json
python3 -m unittest discover -s tools/parity -p 'test_*.py' -v
```

pdflatex references for the fetched tiers are cached under
`<cache>/oracle/`, keyed by the SHA-256 of the source tree, the entry file,
the pdflatex version and argv. Measured on a 15-core M-series Mac: a warm
run of all three tiers takes about 3 min at `-j 10`. A cold run adds 789 s of
serial pdflatex (the longest document takes 79 s), which is about 1.5 min
more at `-j 10`. A document pdflatex cannot compile is cached as such
and excluded.

### Engines side by side (`engines.py`)

`engines.py` puts several runs side by side: the new engine, v1 (the
`flashtex` CLI) and the pdfTeX self-test. For each tier it gives P-T1, P-T2
and L0–L4, and it gives every document of the `--subject` engine that is
not a full pass (P-T1, P-T2 and L4) a root-cause class:

| class | meaning |
|---|---|
| a | a package or font missing from the user's TeX Live (the DESIGN §4.4 bundle fallback) |
| b | an engine difference, with the first differing log, box or content line |
| c | a harness issue in tools/parity |
| d | pdflatex fails too, so the document is excluded |
| e | excluded by the convergence rule: pdflatex compiles it, but its log asks for a rerun on every pass (natbib's `Rerun to get citations correct.`) |

The class comes from the run's records. A notes file adds what a person
found by reading the logs: the pdftex.web section, the owner and the issue.
A note's class overrides the automatic one, and the report shows both. The
output is small and deterministic, and it carries the measuring host, so it
can be committed as evidence (`reports/`):

```sh
python3 tools/parity/engines.py --run new=<out> --run v1=<out> --run pdflatex=<out> --subject new \
    --sha new=<git sha> --sha v1=<git sha> --harness-sha <tools/parity git sha> \
    --notes tools/parity/reports/<name>.notes.json \
    --out tools/parity/reports/<name>
```

### The P5 scoreboard (`scoreboard.py`, `scoreboard-run.sh`)

DESIGN §12 P5's gate is "new engine ≥ old on every tier; arXiv L1 ≥ 90%;
retirement complete". `scoreboard.py` puts every tier in one table, new
engine against v1, from what the existing harnesses already wrote. It
measures nothing itself, so there is no second harness:

| tier | harness | metric per engine |
|---|---|---|
| fixtures, arxiv, templates, packages | `parity.py` (`scoreboard.json`, `documents.json`) | P-T1, P-T2, L0–L3 (L4 when rasterised) |
| nightly-5k (T4) | `nightly.py` (`summary.json`, #1276) | P-T1 (its 5% sample), P-T2, L0–L3 |
| T2 LaTeX suites | `tools/latex-suites/run.py` (transcript) | tests agreeing with pdfTeX; target 0 unexpected |
| package-smoke | `tools/package-smoke/run.py` (transcript) | documents equal to pdfTeX |
| fonts | `tools/font-census/census.py` (`census.json`) | fonts identical to pdfTeX |

Each row gets a verdict. **ahead**, **equal** and **ahead (old n/a)** are
green. The others are:
- **behind**;
- **below target**: arXiv L1 < 90%, or an unexpected T2 failure;
- **below bar (old n/a)**: see below;
- **denominators differ**;
- **host mismatch**: two hosts or two oracles in one row (DESIGN §8);
- **invalid**: a worker died, a transcript without its summary line, a zero
  denominator, or a run that is not this board's engine (below);
- **missing**: a tier nobody ran is still a row.

**Fail closed.** A harness output with a missing or renamed field raises
`FormatError` (exit 2); nothing defaults to 0. Every row needs a denominator
above 0. Denominators are the harnesses' own. Exclusions are printed with
their reasons.

A run is **partial** when:
- it was limited (`--limit`, `--only`, `--shard`, `--spread`);
- it saw fewer documents than its tier's manifest lists, or the tier has no
  manifest to count against (fixtures excepted);
- a nightly run has missing shards, documents not returned or unmeasured, or
  mixed fingerprints;
- T2 ran fewer tests than `run.py --suite all --list` counts
  (`--latex-suites-list`; without it T2 is always partial);
- package-smoke ran fewer documents than `tools/package-smoke` holds.

A font census is a **sample** when:
- it ran below its default `--per-family`;
- it ran with `--only` or `--kind`;
- it tested fewer families than it found, or tested no fonts of some kind;
- it does not record its `selection`.

A board with any partial row, any sample, or `--sample-note` is never
all-green.

**One engine, one commit, one oracle.** A run that records its commit
(`nightly.py`) must be at the board's `--sha`. A run that records none, or a
board without `--sha`, is invalid. Every run of one engine that records the
binary's sha256 must match that engine's `parity.py` run. Every run that
records its oracle must name the same one. Any mismatch marks that run's
cells INVALID: a stale T4 cannot count.

Only the T4 tiers are read from a nightly summary. #1276's corpus-t4 job
also runs the T3 tiers, which come from this board's own `parity.py` runs.

**Rows v1 cannot run.** v1 cannot run T2, package-smoke, the font census or
P-T1, because they need a pdfTeX-compatible binary. Its cell there reads
**n/a** with that reason. new >= old is then vacuous, so these rows have a
bar: green only when new passed 100% of what it measured with nothing
skipped, or at least the row's recorded baseline (`--na-baseline FILE`,
`{"rows": {"tier:metric": {"passed": P, "of": N}}}`). Below it the verdict
is **below bar (old n/a)**.

On a TeX Live newer than the suites' pins, pass
`--latex-suites-reference` with the same suites run through that host's
pdfTeX. The T2 baseline is then pdfTeX on the same host, as in
`scripts/engine-parity.sh`.

**T2 failures are (directory, test) pairs.** The same test name can run in two
directories: l3kernel's testfiles-backend runs under etex-dvips and under
etex-dvisvgm. So pdfTeX failing `m3backend01` in one directory while the
engine fails it in the other is a difference. `run.py` prints each
directory's failures (`LABEL: FAILED t1 t2 ...`), and "unexpected" is the set
of pairs the engine fails and pdfTeX passes.

A transcript is INVALID when:
- its per-directory failures do not account for every FAIL count;
- its `failing tests:` block does not name them;
- its `UNEXPECTED failures:` line names a test no directory failed.

The same checks apply to the reference transcript.

**Retirement stages.** The stages of #1236 (`retirement-stages.json`) form a
column and a table. Each row lists the stages it gates: fixtures P-T2 gates
S3 (the P3 exit), and every row gates S5 onward. An all-green board meets
only the scoreboard part of S5's precondition. The status line names what S5
still needs, including "T1 (lockstep) has 0 new differences", which this
board does not measure.

**Issues.** `--issues apply` opens or updates one issue per tier where
new < old, on complete runs only. The body carries the marker
`<!-- p5-scoreboard:tier=NAME -->`. The run closes that issue only when every
row of the tier is green and no cell is partial, a sample or invalid, and the
board has no `--sample-note`. `dry-run` prints the plan.

**T4 v1.** #1276 runs T4 for the new engine only. T4's old column therefore
reads **missing (no v1 leg in nightly: decision 1)** until corpus-t4 gains a
v1 leg that uploads `corpus-t4-v1`.

`scoreboard-run.sh` runs everything but T4 end to end: it builds both
engines and the new engine's formats, then runs each harness for each
engine and aggregates. `.github/workflows/p5-scoreboard.yml` runs it nightly
on the NixOS runners. The T4 rows come from the `corpus-t4` and
`corpus-t4-v1` artifacts of the newest nightly run from the last 36 h, and the
whole board is measured at that run's commit.

The engine is built alone, in its own `cargo build -p flashtex-engine`, as
corpus-t4 builds it, so the two binaries' sha256 can match. Built together
with `flashtex-cli`, shared dependencies unify features and the binary
differs.

Disk. The first board on a Mac (run 37113092020) died after 5 h with "No
space left on device". The script now bounds what it writes:
- every harness writes under `--work` (default `OUT/work`), its `TMPDIR`
  too, and the script removes what it made there on exit (`--keep-work`
  keeps it). parity.py removes each document's work directory once the
  document is scored, and no traced log reaches the disk (`pt1_plan`).
  On a local `--tiers fixtures --limit 11` board (P-T1 on, T2 sample), the
  peak under `OUT` fell from 533 MB to 42 MB, and 200 KB is left after the
  run instead of 41 MB;
- a guard reads the free space of `OUT`, `WORK` and the parity cache every
  `FLASHTEX_BOARD_DISK_POLL_S` seconds (default 30) while a stage runs.
  Under `FLASHTEX_BOARD_MIN_FREE_GB` (default 10) it stops that stage
  (SIGTERM to its process tree, SIGKILL after 60 s) and starts no other.
  The board is still written; the rows of the stages that did not finish
  read missing or invalid, a `::error::` names the disk and its free
  space, and the script exits 1. It stops rather than pauses, because a
  pause would run into the harnesses' wall-clock time limits.

The workflow keeps write tokens away from third-party TeX sources:
- the measuring job has read-only permissions and a checkout without
  credentials. Its token is only in the step that downloads the T4
  artifacts, before any TeX runs;
- a separate job that runs no TeX files the issues with
  `scoreboard.py --from-board`, and pushes the summary.

```sh
tools/parity/scoreboard-run.sh --out /tmp/p5 --jobs 2            # every tier but T4
tools/parity/scoreboard-run.sh --out /tmp/p5 --limit 12 --sample-note "local sample"
python3 tools/parity/scoreboard.py --parity new=<dir> --parity old=<dir> \
    --nightly new=<dir> --nightly old=<dir> --latex-suites new=<file> \
    --latex-suites-reference <file> --package-smoke new=<file> --fonts new=<dir> \
    --out <dir> [--summary FILE] [--issues dry-run|apply] [--require-green]
```

## Root causes

A blocker is one of:

- **L0**: every error diagnostic, keyed as `code: cs \name (context) <- file`.
  The context is the reason family (in math mode, in the preamble, dimension
  argument, …). `<- file` is the `.sty`/`.cls` FlashTeX was reading when the
  error fired.
- **L1–L3**: warnings that name an unimplemented construct, plus the source
  construct at the first divergence (`pagination:`, `content:` or
  `placement:`).
- **L3→L4**: font/outline notes.

`definers.py` then **groups constructs by who defines them**, because a
package is what a lane implements. The index is built once (~5 s) from TeX
Live and cached. Precedence:

1. TeX primitive
2. the project's own `.sty`/`.cls`
3. the document class
4. kernel register
5. kernel command, which stays one key per command
6. the document's own macro
7. the loaded package whose `\RequirePackage` closure reaches a definer first

Each cause is ranked by the number of documents whose next level it holds.
`sole` counts the documents that cause alone blocks. `share` is Σ 1/|blockers|.

## Honest limits

- Identity is Unicode, not glyph id. An unmapped reference glyph name is
  reported (`unmapped_reference_glyphs`) and counts as a mismatch.
- Text inside Form XObjects (included PDF figures) is read on neither side
  by L2/L3. P-T2 compares them through the page resources.
- P-T2 does not compare annotations (links), outlines or the document
  catalogue. P-T3 (byte identity) is optional and not implemented.
- A cold P-T1 oracle run is heavy: `\tracingall` logs run from 10 MB to
  430 MB per fixture (beamer is the largest).
- L1–L3 blockers are heuristics: the first-divergence label comes from the
  nearest candidate glyph's source span.
- The fixtures tier uses the committed references. Twelve real-world fixtures
  and all divergence probes were made with TeX Live 2025, the rest with
  MacTeX 2026.
- `LaTeX kernel` attribution uses a regex index of definitions, not
  expansion, so a name built with `\csname` is not found and keeps its own
  key.
