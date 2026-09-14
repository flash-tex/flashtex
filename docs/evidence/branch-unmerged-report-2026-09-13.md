# Unmerged remote branches — report for a human decision (2026-09-13)

Companion to `branch-cleanup-2026-09-13.md`. **Nothing in this file has been
deleted or modified.** These 153 branches are *not* ancestors of `main`, so
deleting any of them would lose commits. This is a recommendation only.

Surveyed against `origin/main` = `31be6f9e`, 2026-09-13 ~18:20Z.
"Ancestor of" means the branch tip is fully contained in another live branch,
i.e. someone already carried the work forward and this name is a leftover.

## Summary

| owner prefix | branches | with open PR | no PR ever | oldest tip | abandonment candidates |
|---|--:|--:|--:|--:|--:|
| `agent/kabir-claude` | 60 | 44 | 15 | 13 h | 7 |
| `agent/daniel-parent` | 8 | 0 | 8 | 24 h | 5 |
| `agent` | 6 | 0 | 6 | 14 h | 0 |
| `agent/aarush-macbook` | 5 | 0 | 5 | 39 h | 5 |
| `agent/mac-claude-a` | 5 | 0 | 5 | 39 h | 5 |
| `agent/chatgpt-a` | 4 | 0 | 4 | 38 h | 4 |
| `agent/daniel-parent-b` | 4 | 0 | 4 | 23 h | 3 |
| `agent/mac-pdf` | 4 | 0 | 4 | 33 h | 4 |
| `(top level)` | 3 | 0 | 3 | 35 h | 1 |
| `agent/claude` | 2 | 0 | 2 | 26 h | 1 |
| `agent/commander` | 2 | 0 | 2 | 38 h | 2 |
| `agent/mac-reference-corpus` | 2 | 2 | 0 | 26 h | 0 |
| `agent/claude-dq222` | 1 | 0 | 1 | 26 h | 1 |
| `agent/commander-corpus` | 1 | 0 | 1 | 31 h | 1 |
| `agent/commander-preview-performance` | 1 | 0 | 1 | 24 h | 1 |
| `agent/commander-render-core` | 1 | 0 | 1 | 24 h | 1 |
| `agent/commander-runtime-display` | 1 | 0 | 1 | 24 h | 1 |
| `agent/daniel-calc` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-color` | 1 | 0 | 1 | 25 h | 1 |
| `agent/daniel-contents` | 1 | 0 | 1 | 24 h | 1 |
| `agent/daniel-corpus-sweep` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-floats` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-font-route-study` | 1 | 0 | 1 | 22 h | 1 |
| `agent/daniel-footnotes` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-grok-coverage` | 1 | 0 | 1 | 22 h | 1 |
| `agent/daniel-hw1-packages` | 1 | 0 | 1 | 22 h | 1 |
| `agent/daniel-hw1-visual` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-images` | 1 | 0 | 1 | 24 h | 1 |
| `agent/daniel-links` | 1 | 0 | 1 | 24 h | 1 |
| `agent/daniel-math-accents` | 1 | 0 | 1 | 22 h | 1 |
| `agent/daniel-math-access` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-snippets` | 1 | 0 | 1 | 25 h | 1 |
| `agent/daniel-statistics` | 1 | 0 | 1 | 23 h | 1 |
| `agent/daniel-templates` | 1 | 0 | 1 | 24 h | 1 |
| `agent/daniel-tex-ligatures` | 1 | 0 | 1 | 22 h | 1 |
| `agent/daniel-text-styles` | 1 | 0 | 1 | 22 h | 1 |
| `agent/daniel-title` | 1 | 0 | 1 | 24 h | 1 |
| `agent/linux-engine` | 1 | 1 | 0 | 1 h | 0 |
| `agent/linux-primary` | 1 | 0 | 1 | 0 h | 0 |
| `agent/mac-ai-review-2` | 1 | 0 | 1 | 26 h | 1 |
| `agent/mac-bibliography-kinds` | 1 | 0 | 1 | 28 h | 1 |
| `agent/mac-citation-rename` | 1 | 0 | 1 | 28 h | 1 |
| `agent/mac-contract-review` | 1 | 0 | 1 | 37 h | 1 |
| `agent/mac-diagnostic-explanations` | 1 | 0 | 1 | 36 h | 1 |
| `agent/mac-diagnostics-2` | 1 | 0 | 1 | 26 h | 1 |
| `agent/mac-editor-a11y-2` | 1 | 0 | 1 | 26 h | 1 |
| `agent/mac-export-ux` | 1 | 0 | 1 | 29 h | 1 |
| `agent/mac-font-engine` | 1 | 0 | 1 | 35 h | 1 |
| `agent/mac-helper-display` | 1 | 0 | 1 | 28 h | 1 |
| `agent/mac-large-document` | 1 | 0 | 1 | 28 h | 1 |
| `agent/mac-math-layout` | 1 | 0 | 1 | 35 h | 1 |
| `agent/mac-preferences` | 1 | 0 | 1 | 31 h | 1 |
| `agent/mac-preview-anchoring` | 1 | 0 | 1 | 28 h | 1 |
| `agent/mac-project-files` | 1 | 0 | 1 | 36 h | 1 |
| `agent/mac-render-pipeline` | 1 | 0 | 1 | 24 h | 1 |
| `agent/mac-validation` | 1 | 0 | 1 | 36 h | 1 |
| `agent/mac-visual-oracle` | 1 | 0 | 1 | 32 h | 1 |
| `agent/orchestrator-jaysen-opus` | 1 | 0 | 1 | 35 h | 1 |
| `commander` | 1 | 0 | 1 | 25 h | 1 |
| `mpl` | 1 | 0 | 1 | 35 h | 1 |

**83 of 153 are flagged as abandonment candidates.** They are listed in full at
the end. The remaining 70 are either backed by an open PR (the live review
queue) or too recent to judge.

## By owner prefix

### `agent/kabir-claude` — 60 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/kabir-claude/hw2-math-final` | `6492bf532000` | 13 h | none | `agent/kabir-claude/amsmath-envs`, `agent/kabir-claude/integration` | **candidate:** superseded — fully contained in `agent/kabir-claude/amsmath-envs`, `agent/kabir-claude/integration` |
| `agent/kabir-claude/conversion-provider` | `196ab0280f6c` | 13 h | none | `agent/kabir-claude/integration` | **candidate:** superseded — fully contained in `agent/kabir-claude/integration` |
| `agent/kabir-claude/hyphenation` | `0820d0e2164f` | 13 h | none | `agent/kabir-claude/integration` | **candidate:** superseded — fully contained in `agent/kabir-claude/integration` |
| `agent/kabir-claude/supported-latex` | `54f1e78b3502` | 13 h | none | `agent/kabir-claude/integration` | **candidate:** superseded — fully contained in `agent/kabir-claude/integration` |
| `agent/kabir-claude/amsmath-envs` | `7a816f31c502` | 13 h | none | `agent/kabir-claude/integration` | **candidate:** superseded — fully contained in `agent/kabir-claude/integration` |
| `agent/kabir-claude/graphics-floats` | `b2726754bfbd` | 13 h | none | `agent/kabir-claude/integration` | **candidate:** superseded — fully contained in `agent/kabir-claude/integration` |
| `agent/kabir-claude/tikz-min` | `cf81fc41916c` | 13 h | none | — | keep for now |
| `agent/kabir-claude/compiler-perf` | `196d580fea78` | 12 h | none | — | keep for now |
| `agent/kabir-claude/integration` | `6be162682904` | 12 h | none | — | keep for now |
| `agent/kabir-claude/graphics-floats-r1` | `a6bc41aaaafb` | 12 h | none | — | keep for now |
| `agent/kabir-claude/compiler-perf-r1` | `f4d7a5a3196f` | 12 h | none | — | keep for now |
| `agent/kabir-claude/supported-latex-r1` | `92ff59eb3e0e` | 12 h | none | — | keep for now |
| `agent/kabir-claude/makeindex` | `b8dc94cebe58` | 11 h | #94 open/draft | — | live — open PR |
| `agent/kabir-claude/tex-align` | `1650408a5752` | 11 h | #95 open/draft | — | live — open PR |
| `agent/kabir-claude/engine-poc` | `8d243135f045` | 11 h | #96 open/draft | — | live — open PR |
| `agent/kabir-claude/graphics-floats-r2` | `357be33d38f7` | 10 h | none | — | keep for now |
| `agent/kabir-claude/beads-init` | `7cb37c4c43ba` | 10 h | #101 open/draft | — | live — open PR |
| `agent/kabir-claude/beads-cutover` | `473e0f733b1e` | 10 h | #84 open/draft | — | live — open PR |
| `agent/kabir-claude/render-pipeline-class-geometry-2` | `aaeb4449636c` | 10 h | #102 closed | `agent/kabir-claude/render-pipeline-class-geometry-3` | **candidate:** superseded — fully contained in `agent/kabir-claude/render-pipeline-class-geometry-3` |
| `agent/kabir-claude/deterministic-font-warning` | `c59d2c664754` | 9 h | none | — | keep for now |
| `agent/kabir-claude/render-pipeline-class-geometry-3` | `09134d1d775a` | 9 h | #104 open/draft | — | live — open PR |
| `agent/kabir-claude/maketitle-layout` | `309e73f32f67` | 8 h | #114 open/draft | — | live — open PR |
| `agent/kabir-claude/math-nested-grids` | `030f5b2d089d` | 8 h | #115 open/draft | — | live — open PR |
| `agent/kabir-claude/realworld-pagecounts` | `95bd4cd79a80` | 8 h | #117 open/draft | `agent/kabir-claude/float-bodies-r` +2 more | live — open PR |
| `agent/kabir-claude/float-tables` | `2c15cabed9fd` | 8 h | #123 open/draft | `agent/kabir-claude/float-bodies-r`, `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/hyperref-coverage` | `3b131b88bac0` | 7 h | #131 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/box-commands` | `54ff53d8cbb0` | 7 h | #137 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/float-bodies-r` | `f125c4afda65` | 7 h | #135 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/box-commands-pipeline` | `a2ec24289d26` | 7 h | #138 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/footnote-long-argument` | `d7470191ce16` | 6 h | #142 open/draft | `agent/kabir-claude/footnotes-2-compiler`, `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/verbatim-fidelity` | `5a8d626da7af` | 6 h | #145 open/draft | — | live — open PR |
| `agent/kabir-claude/list-fidelity` | `d2a59a29cacf` | 6 h | #143 open/draft | `agent/kabir-claude/integration-2026-09-13b`, `agent/kabir-claude/list-structure-pipeline` | live — open PR |
| `agent/kabir-claude/verbatim-fidelity-compiler` | `439dc1e6d542` | 6 h | #144 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/footnotes-2-compiler` | `3f227bbac59c` | 6 h | #153 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/theorem-fidelity-compiler` | `ce7797fdd234` | 6 h | #149 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/heading-fidelity` | `d1d4facb1672` | 5 h | #151 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/list-structure-pipeline` | `d98279ef7c58` | 5 h | #152 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/footnotes-2` | `8217e984bcd0` | 5 h | #154 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/theorem-fidelity` | `c287fb35f96b` | 5 h | #156 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/algorithmic` | `5336e9a9c893` | 4 h | #162 open/draft | — | live — open PR |
| `agent/kabir-claude/algorithmic-compiler` | `1c78cd7396ed` | 4 h | #159 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/citations-bibliography` | `40b4070fff27` | 4 h | #166 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/citations-bibliography-pipeline` | `83e8df372efc` | 4 h | #167 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/inline-graphics` | `da3fb1bf9408` | 4 h | #170 open/draft | `agent/kabir-claude/integration-2026-09-13b` | live — open PR |
| `agent/kabir-claude/fancyhdr` | `8cbc23161a7d` | 3 h | none | — | keep for now |
| `agent/kabir-claude/math-font-metrics-pipeline` | `769aac693956` | 3 h | #177 open/draft | — | live — open PR |
| `agent/kabir-claude/table-packages-compiler` | `e84b98851236` | 3 h | #173 open/draft | — | live — open PR |
| `agent/kabir-claude/mac-test-fixes` | `e138653e3c3b` | 2 h | #179 open/draft | — | live — open PR |
| `agent/kabir-claude/math-inline-expansion` | `df1bbae6cc36` | 2 h | #175 open/draft | — | live — open PR |
| `agent/kabir-claude/math-symbols-compiler` | `002425083bfe` | 2 h | #180 open/draft | — | live — open PR |
| `agent/kabir-claude/table-packages` | `18e442f77b4b` | 2 h | #176 open/draft | `agent/kabir-claude/longtable-inserts` | live — open PR |
| `agent/kabir-claude/math-symbols-pipeline` | `ab4dd82a0265` | 1 h | #181 open/draft | — | live — open PR |
| `agent/kabir-claude/math-kernel-symbols-layout` | `c170235323e2` | 1 h | #186 open/draft | — | live — open PR |
| `agent/kabir-claude/longtable-inserts` | `46dae865a139` | 1 h | #184 open/draft | — | live — open PR |
| `agent/kabir-claude/math-kernel-symbols-compiler` | `6fb5cb0337a4` | 1 h | #185 open/draft | — | live — open PR |
| `agent/kabir-claude/bundle-typewriter-fonts` | `c5cfd9b555ee` | 1 h | #183 open/draft | — | live — open PR |
| `agent/kabir-claude/literal-verbatim-nfss` | `158974961c4f` | 0 h | #187 open/draft | — | live — open PR |
| `agent/kabir-claude/integration-2026-09-13b` | `407df7842425` | 0 h | #178 open/draft | — | live — open PR |
| `agent/kabir-claude/lstinline-compiler` | `05c44533960d` | 0 h | #188 open/draft | — | live — open PR |
| `agent/kabir-claude/branch-hygiene-2026-09-13` | `de874d624b7e` | 0 h | #189 open | — | live — open PR |

### `agent/daniel-parent` — 8 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-parent/supervisor` | `876bb668d3f3` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |
| `agent/daniel-parent/mac-ui-redesign` | `2362287ce0b9` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |
| `agent/daniel-parent/unowned-crate-hardening` | `53f3058971ac` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |
| `agent/daniel-parent/hw1-integration-v2` | `e00edb250366` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |
| `agent/daniel-parent/ledger` | `d2eff69146fc` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |
| `agent/daniel-parent/corpus-quickwins-2` | `d6afc285ea2f` | 14 h | none | — | keep for now |
| `agent/daniel-parent/figures` | `59e45f95e81a` | 14 h | none | — | keep for now |
| `agent/daniel-parent/compile-latency` | `24f285af461e` | 14 h | none | — | keep for now |

### `agent` — 6 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-ux-spellcheck` | `b2ddf771d1cd` | 14 h | none | — | keep for now |
| `agent/daniel-ux-cmdr-dataloss` | `9e54d8f3d410` | 14 h | none | — | keep for now |
| `agent/daniel-ux-wordcount` | `6ef344d7b739` | 14 h | none | — | keep for now |
| `agent/daniel-ux-find` | `db106508c31b` | 14 h | none | — | keep for now |
| `agent/daniel-ux-project-files` | `8081f239ea18` | 14 h | none | — | keep for now |
| `agent/daniel-ux-editor-keys` | `31a5fa26a47e` | 14 h | none | — | keep for now |

### `agent/aarush-macbook` — 5 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/aarush-macbook/register` | `5db9d2ca61d2` | 39 h | none | — | **candidate:** no PR ever opened, 39 h stale |
| `agent/aarush-macbook/pdf-output` | `96f1d9681b4c` | 37 h | none | `agent/aarush-macbook/incremental-reuse` | **candidate:** superseded — fully contained in `agent/aarush-macbook/incremental-reuse` |
| `agent/aarush-macbook/incremental-reuse` | `0236aa0a7441` | 27 h | none | — | **candidate:** no PR ever opened, 27 h stale |
| `agent/aarush-macbook/test-coverage-v2` | `ba1ca4b9ad42` | 27 h | none | — | **candidate:** no PR ever opened, 27 h stale |
| `agent/aarush-macbook/companion-capture` | `98021805ada6` | 26 h | none | — | **candidate:** no PR ever opened, 26 h stale |

### `agent/mac-claude-a` — 5 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-claude-a/register` | `431889cbb426` | 39 h | none | — | **candidate:** no PR ever opened, 39 h stale |
| `agent/mac-claude-a/fontmetrics` | `30a14a6c16e7` | 37 h | none | — | **candidate:** no PR ever opened, 37 h stale |
| `agent/mac-claude-a/nearby-client` | `6e1510404eb1` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |
| `agent/mac-claude-a/preview-v2` | `bc28c59a9715` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |
| `agent/mac-claude-a/typing-bench` | `854bf7cbada6` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |

### `agent/chatgpt-a` — 4 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/chatgpt-a/register` | `af55b9e73a83` | 38 h | none | — | **candidate:** no PR ever opened, 38 h stale |
| `agent/chatgpt-a/companion-project-repair` | `699e1bf6ff37` | 37 h | none | — | **candidate:** no PR ever opened, 37 h stale |
| `agent/chatgpt-a/companion-reliability` | `439c8cc44c7e` | 25 h | none | — | **candidate:** no PR ever opened, 25 h stale |
| `agent/chatgpt-a/companion-validation` | `46c1c9d480db` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/daniel-parent-b` — 4 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-parent-b/mac-ui-rebase` | `490474a9ab59` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |
| `agent/daniel-parent-b/completion-vocab-sync` | `5efd4f3079c9` | 22 h | none | `agent/daniel-parent-b/lm-math-symbols` | **candidate:** superseded — fully contained in `agent/daniel-parent-b/lm-math-symbols` |
| `agent/daniel-parent-b/lm-math-symbols` | `5a7be8bbbfad` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |
| `agent/daniel-parent-b/sqrt-vinculum` | `62e66d75aba6` | 20 h | none | — | keep for now |

### `agent/mac-pdf` — 4 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-pdf/exact-export` | `bc43704f814b` | 33 h | none | — | **candidate:** no PR ever opened, 33 h stale |
| `agent/mac-pdf/v2-adapter` | `20e5277857b2` | 31 h | none | — | **candidate:** no PR ever opened, 31 h stale |
| `agent/mac-pdf/fidelity` | `a3536c2f58a5` | 26 h | none | `agent/mac-pdf/searchable-text` | **candidate:** superseded — fully contained in `agent/mac-pdf/searchable-text` |
| `agent/mac-pdf/searchable-text` | `b4b15136f54d` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `(top level)` — 3 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `mml-rev4` | `ac3bf0b10a5c` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |
| `gh-pages` | `10f250ada452` | 0 h | none | — | keep for now |
| `__dolt_remote_info__` | `938d6dfcd596` | 0 h | none | — | keep for now |

### `agent/claude` — 2 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/claude/compiler-foundation` | `49e6eb43808a` | 26 h | none | — | **candidate:** no PR ever opened, 26 h stale |
| `agent/claude/kabir-mac-max-plan` | `f1d5a7180da3` | 14 h | none | — | keep for now |

### `agent/commander` — 2 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/commander/integrate-ft011-corpus` | `9deda1df2afc` | 38 h | none | — | **candidate:** no PR ever opened, 38 h stale |
| `agent/commander/integrate-ft002-compiler` | `dee12f5dca14` | 38 h | none | — | **candidate:** no PR ever opened, 38 h stale |

### `agent/mac-reference-corpus` — 2 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-reference-corpus/extended-suite` | `dfe4756f52b6` | 26 h | #42 open/draft | `agent/mac-reference-corpus/hw1-probes` | live — open PR |
| `agent/mac-reference-corpus/hw1-probes` | `fc8cab68f1fb` | 21 h | #53 open/draft | — | live — open PR |

### `agent/claude-dq222` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/claude-dq222/register` | `cc5c05bea33a` | 26 h | none | `agent/daniel-parent/supervisor` | **candidate:** superseded — fully contained in `agent/daniel-parent/supervisor` |

### `agent/commander-corpus` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/commander-corpus/font-resources` | `55bfd48ebd51` | 31 h | none | — | **candidate:** no PR ever opened, 31 h stale |

### `agent/commander-preview-performance` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/commander-preview-performance/preview-performance` | `ff4282300503` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/commander-render-core` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/commander-render-core/rendering-core` | `05418407d2b5` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/commander-runtime-display` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/commander-runtime-display/display-runtime` | `f15edf29bf50` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/daniel-calc` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-calc/tex-calc` | `8d9da162a230` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-color` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-color/color-expressions` | `171f925f8ffa` | 25 h | none | — | **candidate:** no PR ever opened, 25 h stale |

### `agent/daniel-contents` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-contents/toc-layout` | `b5534602b8ad` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/daniel-corpus-sweep` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-corpus-sweep/report` | `c1ba48ea2c64` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-floats` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-floats/float-layout` | `20543da65a55` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-font-route-study` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-font-route-study/report` | `4d72260c5439` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |

### `agent/daniel-footnotes` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-footnotes/footnote-layout` | `a2cbb8793f67` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-grok-coverage` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-grok-coverage/compiler` | `252f99f47101` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |

### `agent/daniel-hw1-packages` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-hw1-packages/compiler` | `fd24147d1ce4` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |

### `agent/daniel-hw1-visual` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-hw1-visual/audit` | `c16590542e02` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-images` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-images/image-assets` | `6c84d8875425` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/daniel-links` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-links/link-annotations` | `1d9028d10417` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/daniel-math-accents` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-math-accents/compiler` | `67cd50cc3926` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |

### `agent/daniel-math-access` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-math-access/math-accessibility` | `fa9e674b69b5` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-snippets` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-snippets/editor-snippets` | `490538db3f1e` | 25 h | none | — | **candidate:** no PR ever opened, 25 h stale |

### `agent/daniel-statistics` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-statistics/document-statistics` | `866a9c7c3dc9` | 23 h | none | — | **candidate:** no PR ever opened, 23 h stale |

### `agent/daniel-templates` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-templates/project-templates` | `940b68782919` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/daniel-tex-ligatures` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-tex-ligatures/compiler` | `2d336ff88c2b` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |

### `agent/daniel-text-styles` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-text-styles/compiler` | `2477e1c0c4c0` | 22 h | none | — | **candidate:** no PR ever opened, 22 h stale |

### `agent/daniel-title` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/daniel-title/title-layout` | `f9b94af68e31` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/linux-engine` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/linux-engine/layout-cache-discriminants` | `f05bb23b9930` | 1 h | #182 open/draft | — | live — open PR |

### `agent/linux-primary` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/linux-primary/lecture-notes-trivlist-breaks` | `2945f61ae130` | 0 h | none | — | keep for now |

### `agent/mac-ai-review-2` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-ai-review-2/recovery-applied` | `9710f07c8d4e` | 26 h | none | — | **candidate:** no PR ever opened, 26 h stale |

### `agent/mac-bibliography-kinds` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-bibliography-kinds/bibkinds-applied` | `6a73f6d6f14f` | 28 h | none | — | **candidate:** no PR ever opened, 28 h stale |

### `agent/mac-citation-rename` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-citation-rename/citation-rename-applied` | `5e4a175ce484` | 28 h | none | — | **candidate:** no PR ever opened, 28 h stale |

### `agent/mac-contract-review` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-contract-review/checker` | `6f2420d24ece` | 37 h | none | — | **candidate:** no PR ever opened, 37 h stale |

### `agent/mac-diagnostic-explanations` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-diagnostic-explanations/explain` | `2bf14cd9b829` | 36 h | none | — | **candidate:** no PR ever opened, 36 h stale |

### `agent/mac-diagnostics-2` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-diagnostics-2/partial-output-applied` | `afc8adf94f75` | 26 h | none | — | **candidate:** no PR ever opened, 26 h stale |

### `agent/mac-editor-a11y-2` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-editor-a11y-2/large-doc-applied` | `9f7d5f518604` | 26 h | none | — | **candidate:** no PR ever opened, 26 h stale |

### `agent/mac-export-ux` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-export-ux/export-ux-applied` | `6aacda633339` | 29 h | none | — | **candidate:** no PR ever opened, 29 h stale |

### `agent/mac-font-engine` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-font-engine/tex-fonts` | `73422451ec36` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |

### `agent/mac-helper-display` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-helper-display/route-applied` | `7083f3e79754` | 28 h | none | — | **candidate:** no PR ever opened, 28 h stale |

### `agent/mac-large-document` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-large-document/large-document-applied` | `463c0c53c30c` | 28 h | none | — | **candidate:** no PR ever opened, 28 h stale |

### `agent/mac-math-layout` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-math-layout/math-boxes` | `57cbc444d208` | 35 h | none | `mml-rev4` | **candidate:** superseded — fully contained in `mml-rev4` |

### `agent/mac-preferences` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-preferences/editor-applied` | `314d6f116759` | 31 h | none | — | **candidate:** no PR ever opened, 31 h stale |

### `agent/mac-preview-anchoring` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-preview-anchoring/anchoring-applied` | `5d1503e741b7` | 28 h | none | — | **candidate:** no PR ever opened, 28 h stale |

### `agent/mac-project-files` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-project-files/graph` | `bf462b92c0a6` | 36 h | none | — | **candidate:** no PR ever opened, 36 h stale |

### `agent/mac-render-pipeline` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-render-pipeline/delta-proposal` | `6965c9bef031` | 24 h | none | — | **candidate:** no PR ever opened, 24 h stale |

### `agent/mac-validation` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-validation/native-verification` | `51c8c776d241` | 36 h | none | — | **candidate:** no PR ever opened, 36 h stale |

### `agent/mac-visual-oracle` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/mac-visual-oracle/reference-raster` | `dda820017306` | 32 h | none | — | **candidate:** no PR ever opened, 32 h stale |

### `agent/orchestrator-jaysen-opus` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `agent/orchestrator-jaysen-opus/standby` | `4a462258448b` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |

### `commander` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `commander/handover-claude` | `a48f16808d93` | 25 h | none | — | **candidate:** no PR ever opened, 25 h stale |

### `mpl` — 1 branch(es)

| branch | tip | age | PR | ancestor of | verdict |
|---|---|--:|---|---|---|
| `mpl/linebreak-rev4` | `bad0666f13e0` | 35 h | none | — | **candidate:** no PR ever opened, 35 h stale |

## Abandonment candidates in full (83) — human decision required

Recommended disposition for each: confirm with the owning machine, then either
open a PR for the work or delete the branch. **Do not bulk-delete these** — unlike
the 318 in the cleanup file, these carry commits that exist nowhere else, so a
deletion is only recoverable while the reflog/this file survives.

| branch | tip SHA | age | why flagged | subject |
|---|---|--:|---|---|
| `mml-rev4` | `ac3bf0b10a5cf348eb64de2f1316701383c682a8` | 35 h | no PR ever opened, 35 h stale | wip(mac-math-layout): preserve in-progress work after subagent termina |
| `agent/aarush-macbook/register` | `5db9d2ca61d29e0d4e3b60a8be2cbc3f204a6630` | 39 h | no PR ever opened, 39 h stale | register: aarush-macbook resource inventory |
| `agent/aarush-macbook/pdf-output` | `96f1d9681b4c3da28f1f32b8dcb6c4321bd5f2fd` | 37 h | superseded — fully contained in `agent/aarush-macbook/incremental-reuse` | compiler(FT-005): add PDF output via pdf-writer 0.15 |
| `agent/aarush-macbook/incremental-reuse` | `0236aa0a74413fd997c15b31b885bfde941c4325` | 27 h | no PR ever opened, 27 h stale | test(lib): add 8 unit tests for Span type |
| `agent/aarush-macbook/test-coverage-v2` | `ba1ca4b9ad424c96067c631d17f7507295e50944` | 27 h | no PR ever opened, 27 h stale | chore(coord): ready_for_integration — 72 tests across 5 modules (171b8 |
| `agent/aarush-macbook/companion-capture` | `98021805ada6b85b9d7ab85b30e4292bf150322c` | 26 h | no PR ever opened, 26 h stale | coord(aarush-macbook): ready_for_integration — FT-004 rev 4 companion  |
| `agent/chatgpt-a/register` | `af55b9e73a839718cdc4c3d41fdd43fe35f25cc6` | 38 h | no PR ever opened, 38 h stale | coordination: request task and reproduce companion build failure |
| `agent/chatgpt-a/companion-project-repair` | `699e1bf6ff3759e14d4bdeec7c5e6b781b7ec7fc` | 37 h | no PR ever opened, 37 h stale | fix(companion): repair corrupted Xcode project references |
| `agent/chatgpt-a/companion-reliability` | `439c8cc44c7e8ee2e685822418bcf5371e22edcb` | 25 h | no PR ever opened, 25 h stale | fix(companion): avoid repeated Bonjour reconnects |
| `agent/chatgpt-a/companion-validation` | `46c1c9d480db086308ec98ac7dcfb48816118e29` | 24 h | no PR ever opened, 24 h stale | test(companion): make native validation fail closed |
| `agent/claude/compiler-foundation` | `49e6eb43808ac8fccb08d620b23667403fcb8667` | 26 h | no PR ever opened, 26 h stale | coord: FT-007 verified on main, correcting stale bookkeeping |
| `agent/claude-dq222/register` | `cc5c05bea33aab04ab0909c49e74d594df41dd30` | 26 h | superseded — fully contained in `agent/daniel-parent/supervisor` | Report round 3 status: 12 of 16 lanes current |
| `agent/commander/integrate-ft011-corpus` | `9deda1df2afc08352480b8c48b82983982aaf4af` | 38 h | no PR ever opened, 38 h stale | integration: combine reviewed worker changes |
| `agent/commander/integrate-ft002-compiler` | `dee12f5dca144768d1bed1e096ef0f1e2b5db2c7` | 38 h | no PR ever opened, 38 h stale | integration: combine reviewed worker changes |
| `agent/commander-corpus/font-resources` | `55bfd48ebd51e788926f767b585130bd833d55c1` | 31 h | no PR ever opened, 31 h stale | Report verified rational Type1 matrix checkpoint |
| `agent/commander-preview-performance/preview-performance` | `ff4282300503036552d62021d07fd1275381dd58` | 24 h | no PR ever opened, 24 h stale | Checkpoint continued probe improvement and PDF whitespace investigatio |
| `agent/commander-render-core/rendering-core` | `05418407d2b568c206aab1d0de119381d8118c71` | 24 h | no PR ever opened, 24 h stale | Document semantic whitespace boundary for existing producer and PDF ow |
| `agent/commander-runtime-display/display-runtime` | `f15edf29bf50df030f7cce0f39b929192775dc3b` | 24 h | no PR ever opened, 24 h stale | docs(runtime): audit lazy source hashing and reject speculative reuse  |
| `agent/daniel-calc/tex-calc` | `8d9da162a230c331957bf47f8c7eea034983d939` | 23 h | no PR ever opened, 23 h stale | tex-calc: fix Scalar / Scalar division silently returning Ok(0) |
| `agent/daniel-color/color-expressions` | `171f925f8ffac63e080345b93c639412881c564e` | 25 h | no PR ever opened, 25 h stale | color-expressions: satisfy cargo fmt in identity_regressions |
| `agent/daniel-contents/toc-layout` | `b5534602b8adcd29c6af737d17c9ae350712d24a` | 24 h | no PR ever opened, 24 h stale | toc-layout: lock LineBox and RelativeEntry construction against field  |
| `agent/daniel-corpus-sweep/report` | `c1ba48ea2c6418f2a32ffc3582e6c48e80167f44` | 23 h | no PR ever opened, 23 h stale | coordination: add extended-tex-corpus compiler sweep report |
| `agent/daniel-floats/float-layout` | `20543da65a55959cdaca33f3f1cafc95287d80e5` | 23 h | no PR ever opened, 23 h stale | font-engine: lock OpenTypeMathFace construction against field bypass |
| `agent/daniel-font-route-study/report` | `4d72260c543937f67e10f8d12704fd902941a866` | 22 h | no PR ever opened, 22 h stale | coordination: correct bundled LM asset counts in font route study |
| `agent/daniel-footnotes/footnote-layout` | `a2cbb8793f67977e9ccad94eb2adaba8b2e22472` | 23 h | no PR ever opened, 23 h stale | math-layout: replace two font-metrics panics with typed errors |
| `agent/daniel-grok-coverage/compiler` | `252f99f471018fded83eb41b634c625dfb58f1ab` | 22 h | no PR ever opened, 22 h stale | Merge remote-tracking branch 'origin/main' into agent/daniel-grok-cove |
| `agent/daniel-hw1-packages/compiler` | `fd24147d1ce45d1cc191e76d7c1da87dc71aa442` | 22 h | no PR ever opened, 22 h stale | Merge remote-tracking branch 'origin/main' into agent/daniel-hw1-packa |
| `agent/daniel-hw1-visual/audit` | `c16590542e0286575446b6689a496020e1b61457` | 23 h | no PR ever opened, 23 h stale | coordination: HW1 visual audit — ranked rendering defects vs reference |
| `agent/daniel-images/image-assets` | `6c84d88754259ea7b09e55be8aaa6ed81b059cc7` | 24 h | no PR ever opened, 24 h stale | image-assets: satisfy cargo fmt |
| `agent/daniel-links/link-annotations` | `1d9028d1041721dffd3371de3f292d3a0d5b2078` | 24 h | no PR ever opened, 24 h stale | Fix Rect/Point validation bypass via public struct fields |
| `agent/daniel-math-accents/compiler` | `67cd50cc392620bf7f2191bd04342d5ec96f2036` | 22 h | no PR ever opened, 22 h stale | lanes: final checkpoint for daniel-math-accents, stopping on quota not |
| `agent/daniel-math-access/math-accessibility` | `fa9e674b69b508478c9488e6d223411fd9c5c55c` | 23 h | no PR ever opened, 23 h stale | math-accessibility: empirically justify and pin the default depth boun |
| `agent/daniel-parent/supervisor` | `876bb668d3f31484ee0b9c08d3551cf21524d1ec` | 24 h | no PR ever opened, 24 h stale | coordination: publish the invariant-bypass and documentation audits |
| `agent/daniel-parent/mac-ui-redesign` | `2362287ce0b964f6107bbb920885223a83cedf6f` | 23 h | no PR ever opened, 23 h stale | coordination: Mac UI design spec and critique (daniel-parent lane) |
| `agent/daniel-parent/unowned-crate-hardening` | `53f3058971aca44585aa70f7c43924ccbbbfb76a` | 23 h | no PR ever opened, 23 h stale | spellcheck: add bounded random-input fuzz sweep (no defect found) |
| `agent/daniel-parent/hw1-integration-v2` | `e00edb2503668caabb6423f6e9243e0ae3f2f73a` | 22 h | no PR ever opened, 22 h stale | compiler: declaration-scoped \tiny..\Huge and \setlength parindent/par |
| `agent/daniel-parent/ledger` | `d2eff69146fc421dd782c50c95a10a655a135721` | 22 h | no PR ever opened, 22 h stale | coordination(daniel-parent): final ledger 20:03Z, deliverables and che |
| `agent/daniel-parent-b/mac-ui-rebase` | `490474a9ab59008e3b9eea0b1c7f77e0a7a2223b` | 23 h | no PR ever opened, 23 h stale | mac: sync completion vocabulary with compiler on main after rebase |
| `agent/daniel-parent-b/completion-vocab-sync` | `5efd4f3079c9158eb2b4ed7a9929ab3067dd965b` | 22 h | superseded — fully contained in `agent/daniel-parent-b/lm-math-symbols` | mac+compiler: classify \setlist as a diagnostic-only parser arm |
| `agent/daniel-parent-b/lm-math-symbols` | `5a7be8bbbfadc15505844e8c70f73217e9df1a4a` | 22 h | no PR ever opened, 22 h stale | Merge remote-tracking branch 'origin/main' into agent/daniel-parent-b/ |
| `agent/daniel-snippets/editor-snippets` | `490538db3f1e4401e1e8f9ad436b66d0c5047943` | 25 h | no PR ever opened, 25 h stale | Merge remote-tracking branch 'origin/main' into agent/daniel-snippets/ |
| `agent/daniel-statistics/document-statistics` | `866a9c7c3dc95728bca8a5a6754bc887b5e0bbc8` | 23 h | no PR ever opened, 23 h stale | Fix Statistics forgery via pub fields bypassing compute()'s invariant |
| `agent/daniel-templates/project-templates` | `940b687829198c2fd97468be417317094604280d` | 24 h | no PR ever opened, 24 h stale | daniel-templates: correct an updated_utc set in the future |
| `agent/daniel-tex-ligatures/compiler` | `2d336ff88c2b83c53b4dc32321ec861238038ecb` | 22 h | no PR ever opened, 22 h stale | Merge remote-tracking branch 'origin/main' into agent/daniel-tex-ligat |
| `agent/daniel-text-styles/compiler` | `2477e1c0c4c0114e4ea1b58a81a93f8533a803d3` | 22 h | no PR ever opened, 22 h stale | lanes: daniel-text-styles progress log |
| `agent/daniel-title/title-layout` | `f9b94af68e31bef3f566dd40ccdf4827f6c36a99` | 24 h | no PR ever opened, 24 h stale | title-layout: reject non-finite parskip instead of returning a NaN gap |
| `agent/kabir-claude/hw2-math-final` | `6492bf532000f83bb116bf61c554b8e1385d0c4f` | 13 h | superseded — fully contained in `agent/kabir-claude/amsmath-envs`, `agent/kabir-claude/integration` | compiler: \mathcal from New Computer Modern Math, msbm \varnothing, HW |
| `agent/kabir-claude/conversion-provider` | `196ab0280f6c283a05a71f894c31eeb755f47874` | 13 h | superseded — fully contained in `agent/kabir-claude/integration` | bridge: --conversion-provider seam, openai-compatible provider, provid |
| `agent/kabir-claude/hyphenation` | `0820d0e2164f7be3b300489268ca83d8f499bc0e` | 13 h | superseded — fully contained in `agent/kabir-claude/integration` | paragraph-layout: Liang hyphenation, \discretionary and a pdflatex bre |
| `agent/kabir-claude/supported-latex` | `54f1e78b3502ba7bbf90c5787a72f5900ff04685` | 13 h | superseded — fully contained in `agent/kabir-claude/integration` | compiler: generate supported-LaTeX docs, JSON and coverage from the pa |
| `agent/kabir-claude/amsmath-envs` | `7a816f31c5022f2e18502739846d575f9ff31445` | 13 h | superseded — fully contained in `agent/kabir-claude/integration` | compiler: FT-061 amsmath oracle corpus (42 fixtures), baseline 4/42 wi |
| `agent/kabir-claude/graphics-floats` | `b2726754bfbddf8116da0340b83d4e445fdcd83a` | 13 h | superseded — fully contained in `agent/kabir-claude/integration` | render-pipeline: \includegraphics and figure/table float placement (FT |
| `agent/kabir-claude/render-pipeline-class-geometry-2` | `aaeb4449636c47f00327a6518501a298ef4637a8` | 10 h | superseded — fully contained in `agent/kabir-claude/render-pipeline-class-geometry-3` | render-pipeline: twoside, twocolumn, page styles and marks (CONTRACT s |
| `agent/mac-ai-review-2/recovery-applied` | `9710f07c8d4ecd7f867a967a6c8405ede7570cdd` | 26 h | no PR ever opened, 26 h stale | APPLIED (parent-retained, do not merge): ShellModel hooks for review h |
| `agent/mac-bibliography-kinds/bibkinds-applied` | `6a73f6d6f14facbc430c66d4fd3d55b41c6f030c` | 28 h | no PR ever opened, 28 h stale | Merge branch 'agent/mac-bibliography-kinds/bibkinds' into agent/mac-bi |
| `agent/mac-citation-rename/citation-rename-applied` | `5e4a175ce48423c305a8cad75a3a2b8437967a1e` | 28 h | no PR ever opened, 28 h stale | mac (APPLIED, parent-retained): FlashTeXMacApp hook for Rename Citatio |
| `agent/mac-claude-a/register` | `431889cbb426f320e7080601eb8e9cbaeb3bfdca` | 39 h | no PR ever opened, 39 h stale | coordination: register mac-m1max-a worker and resource inventory |
| `agent/mac-claude-a/fontmetrics` | `30a14a6c16e79bc344f381e79d8d6f78c0e6fa33` | 37 h | no PR ever opened, 37 h stale | fontmetrics: base-14 advance tables measured via CoreText (proposal fo |
| `agent/mac-claude-a/nearby-client` | `6e1510404eb14905fd5aa89c954ac14a2a582630` | 35 h | no PR ever opened, 35 h stale | Merge remote-tracking branch 'origin/agent/mac-claude-a/mac-shell' int |
| `agent/mac-claude-a/preview-v2` | `bc28c59a9715172052e2d8521064eeb045114875` | 35 h | no PR ever opened, 35 h stale | mac: experimental v2 preview pane — glyph runs by original GID, cluste |
| `agent/mac-claude-a/typing-bench` | `854bf7cbada6b9e3adfabdd3ddc6382ff109518b` | 35 h | no PR ever opened, 35 h stale | typing-bench: full 12-cell evidence (compiler + flashtex-render × 3 se |
| `agent/mac-contract-review/checker` | `6f2420d24ece49058fcdbf60a95457398b811f2d` | 37 h | no PR ever opened, 37 h stale | test: add pinned Mac bridge contract review checker |
| `agent/mac-diagnostic-explanations/explain` | `2bf14cd9b829736168b9ebd807016ae7c11b28b1` | 36 h | no PR ever opened, 36 h stale | diagnostics: README with catalog provenance, heuristics, native integr |
| `agent/mac-diagnostics-2/partial-output-applied` | `afc8adf94f75a45abeebcc03317eb4cd4e78c44f` | 26 h | no PR ever opened, 26 h stale | mac (APPLIED, parent-retained files — for compile/test only): diagnost |
| `agent/mac-editor-a11y-2/large-doc-applied` | `9f7d5f518604816fcc0bbf82aa9b8283c0b7c04f` | 26 h | no PR ever opened, 26 h stale | mac(applied, parent-retained): SourceEditorView large-document hooks — |
| `agent/mac-export-ux/export-ux-applied` | `6aacda63333950c1ec83acaa392fabb1fdfa0722` | 29 h | no PR ever opened, 29 h stale | LOCAL APPLICATION (not for integration): ContentView capture-bar expor |
| `agent/mac-font-engine/tex-fonts` | `73422451ec367269d8ce2ba4554ba5103224a262` | 35 h | no PR ever opened, 35 h stale | wip(mac-font-engine): preserve in-progress work after subagent termina |
| `agent/mac-helper-display/route-applied` | `7083f3e79754a845ab2432b336a085dec8abb227` | 28 h | no PR ever opened, 28 h stale | LOCAL APPLICATION: parent-retained hook lines for the display-candidat |
| `agent/mac-large-document/large-document-applied` | `463c0c53c30c20e77e717391dd669e88e3429564` | 28 h | no PR ever opened, 28 h stale | coord: correct the Claude-Session trailer of 9542a24b (typo: …PnmBYXg… |
| `agent/mac-math-layout/math-boxes` | `57cbc444d208885275aaabf0df23c179fd3d71f4` | 35 h | superseded — fully contained in `mml-rev4` | coord: mac-math-layout acknowledges FT-020 rev 4 |
| `agent/mac-pdf/exact-export` | `bc43704f814be2de7937e6f233d9732b1749343e` | 33 h | no PR ever opened, 33 h stale | pdf docs: contrast the runtime-v1 route against the reference in the c |
| `agent/mac-pdf/v2-adapter` | `20e5277857b2cd37f102fb07acdd164f82bb49db` | 31 h | no PR ever opened, 31 h stale | coord: mac-pdf handoff for refill 3 (Type 1 subsetting, math-reference |
| `agent/mac-pdf/fidelity` | `a3536c2f58a5f9f4f3475bc02f48e0298e50d7e2` | 26 h | superseded — fully contained in `agent/mac-pdf/searchable-text` | pdf docs: HW1 replay evidence (3232 glyphs on their display-list origi |
| `agent/mac-pdf/searchable-text` | `b4b15136f54dd3dfe0fe48f682f07f3e7e127c8f` | 23 h | no PR ever opened, 23 h stale | pdf: report word and ambiguous inter-glyph gaps for the searchable-tex |
| `agent/mac-preferences/editor-applied` | `314d6f1167591880a4995ca27c8301f60fc79450` | 31 h | no PR ever opened, 31 h stale | Merge agent/mac-preferences/editor into editor-applied (LOCAL APPLICAT |
| `agent/mac-preview-anchoring/anchoring-applied` | `5d1503e741b796082ac4e62149e4541953afb7c1` | 28 h | no PR ever opened, 28 h stale | APPLIED (local measurement only, parent-retained file): PreviewView an |
| `agent/mac-project-files/graph` | `bf462b92c0a6aceee1a3e0709b2e5fa8ad361d0d` | 36 h | no PR ever opened, 36 h stale | coordination: mac-project-files structured report (#18 fix, main 1654e |
| `agent/mac-render-pipeline/delta-proposal` | `6965c9bef0319de5ddc6a7a1e027893926a59eef` | 24 h | no PR ever opened, 24 h stale | coord: mac-display-delta-proposal — issue #50 fix recorded (consumer 6 |
| `agent/mac-validation/native-verification` | `51c8c776d241abbce7a2728f06f8fb515e2df09d` | 36 h | no PR ever opened, 36 h stale | validation: FT-003 rev 5 packaged recovery and responsiveness evidence |
| `agent/mac-visual-oracle/reference-raster` | `dda820017306aecef5ce733e8eb86e7c2b51ce36` | 32 h | no PR ever opened, 32 h stale | visual-oracle: exact-route closure matrix evidence (render --v2 -> fro |
| `agent/orchestrator-jaysen-opus/standby` | `4a462258448bfbae07326b99580e69acc3bac881` | 35 h | no PR ever opened, 35 h stale | standby: read-only Jaysen Opus standby monitor, fail-closed revival ga |
| `commander/handover-claude` | `a48f16808d93b9ff57f264979c566a58688261d0` | 25 h | no PR ever opened, 25 h stale | coordination: fix fetch-error visibility and branch-age lookup |
| `mpl/linebreak-rev4` | `bad0666f13e006f967834d5f0ce6c5cf25f6eb27` | 35 h | no PR ever opened, 35 h stale | wip(mac-paragraph-layout): preserve in-progress work after subagent te |

