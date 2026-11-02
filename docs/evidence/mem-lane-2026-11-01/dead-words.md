# Dead mem words left out of the undo logs: dropped (Commander, 2026-11-01)

- Rule: a log leaves out a `mem` word dead at every checkpoint since its last change (free-ring insides, lo_mem_max..hi_mem_min, above mem_end); sound, mutation-checked, release checkpoint and incremental tests 87/87 (NixOS runs 41352616041, 41359408966; branch `agent/kabir-claude-memory/dead-words`, 730830ac9, not proposed).
- Steady RSS, main 6777eff07 vs it (Balanced, 8 keys a page): full-1000 608 → 589 MB, infdesc-x2 929 → 841, plain-1000 440 → 425.
- Edited-page instructions, p50: full-1000 98M → 100M (+2%), infdesc-x2 185M → 199M (+7.6%), plain-1000 32M → 32M; speed beats memory, so it is dropped.
- Sparse sets and ranges instead of `mem`-sized bitmaps did not lower the cost (first version: +2.7% on infdesc-x2, the sparse one +7.6%), so the bitmaps were not it.
- Hypothesis, not measured: the walk of `mem`'s free ring from `rover` at every checkpoint (a pointer chase through `mem`, long on a fragmented book), plus sorting its blocks.
