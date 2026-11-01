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

«TBD: results»
