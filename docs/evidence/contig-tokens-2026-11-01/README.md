# CONTIG-TOKENS evidence (2026-11-01)

Lane **CONTIG-TOKENS** ([`docs/design/engine-v2/CONTIG-TOKENS.md`](../../design/engine-v2/CONTIG-TOKENS.md),
DESIGN.md §12 P6). Main is `3f835f488`; the prototype is `agent/kabir-claude-contig/lazy-flush`.

## Machines and method

- **The NixOS PC** (AMD 7800X3D), through its self-hosted runner (`.github/workflows/contig-pc.yml` on a
  measurement branch; the PC's ssh wanted a Tailscale re-auth that day). `perf stat -e
  instructions:u,cycles:u` per compile, compiles one at a time under `nice`, engines interleaved.
  The runner is shared, so cycles are a reference only next to their run's `load` field;
  instructions do not move with load.
- **GitHub-hosted Linux** (`.github/workflows/contig-bench.yml`): valgrind cachegrind, which counts
  instructions exactly, and with `--cache-sim=yes` simulates an Apple-M1-like cache (D1 128 KiB 8-way,
  LL 16 MiB 16-way, 128-byte lines). `scripts/cgsum.py` charges each counted line to the source
  function around it, so the always-inlined `get_avail` and `flush_list` get totals of their own.
- **Documents**: *Infinite Descent ×2* (`tools/incr-bench/mkbook.py infdesc-x2`, 1,140 pages) and
  P6-HYPEROPT's 13 arXiv papers (every 12th id of `arxiv-2025-01`, sorted; `scripts/fetchdocs.py`).
  Each is settled first (at most 6 passes, until every `.aux`/`.toc`/`.ind`/... file stops changing),
  then measured as `pdftex -fmt=pdflatex -interaction=batchmode '\pdfsetrandomseed 1\relax\input{…}'`
  with `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1` (`scripts/bench.py`).
- **Engines**: `main`; `off`, the prototype's build with the switch off; `on`, the same binary with
  `FLASHTEX_LAZY_FLUSH=1`. Every run records the PDF's and the log's SHA-256 and the log's memory-usage
  lines; `scripts/summary.py` compares them with main's.

## Results

- `raw/pc-2/` (VERIFIED; run 41338025620 of `workflows/contig-pc.yml` on branch
  `agent/kabir-claude-contig/pc-2`, prototype `3cb785555` regenerated as `cb7108610`): `runs.jsonl`
  (every run: rc, user/sys time, perf instructions and cycles, load, PDF and log SHA-256, memory
  lines), `pcsum.md` (medians, min-max, the 13 papers' sums), `summary.md` (identity check), and
  the one log difference, which is the binary's own path in pdfTeX's warnings
  (`2501.07084v3-log-main-vs-proto.diff`).
- `raw/pc-1/` (VERIFIED; run 41335252513, prototype `d362f5381`): the same for the earlier build.
- Headline (pc-2, switch on against main): *Infinite Descent ×2* −0.36 % instructions, −4.23 %
  cycles; 13 papers −0.40 % instructions, −2.41 % cycles, fewer cycles on 13 of 13. Switch off:
  +1.74 % and +1.15 % instructions. pc-1: −6.68 % and −3.00 % cycles with the switch on.
- Output identity: every PDF of 420 measured compiles (210 per build; pc-1 and pc-2 measured
  different builds) equals main's; logs equal but for that path. No document sets
  `\tracingstats>1`, so the only memory line compared is the end-of-run "words of memory", which
  does not read `dyn_used`; the recomputed `dyn_used` is tested in #2016 (below). `maxrss_kb` here
  is `RUSAGE_CHILDREN`'s running maximum, not a per-compile peak: these runs carry no memory data.
- Never ran: read-side cache behaviour (`workflows/contig-bench.yml`, run 41335270525) and the
  formal gates with the switch on (`workflows/contig-gates.yml`, run 41342741326 on branch
  `agent/kabir-claude-contig/pcgates-1`, and that branch's CI run 41342741298) were all
  **cancelled** in the queue. The gates were run instead on #2016's always-on build; see
  CONTIG-TOKENS.md §5.

## Reproduce

Push a branch named `agent/kabir-claude-contig/pc-*` holding `workflows/contig-pc.yml` as
`.github/workflows/` and a `bench.json` naming the builds and engines; the run regenerates each
build's `src/generated/` with web2rust, builds it, makes its format, settles and measures every
document, and uploads `out/` (with each build's regeneration patch). `scripts/pcsum.py
raw/pc-2/runs.jsonl` prints the table above.
