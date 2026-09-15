# Remote branch cleanup, round 2 — the no-PR branches (2026-09-13)

Second pass, under a further explicit owner instruction: *"if the 130 are
superseded, finished, or abandoned, confirm that and then get rid of them."*
**Confirmation here is per-branch evidence, not a category guess.**

Round 1 (`branch-cleanup-2026-09-13.md`) deleted 318 branches that were
ancestors of `main`. This pass looks at branches with **no open PR** whose tip
is **older than 6 hours**, and sorts every one into exactly one bucket.

## Buckets

| bucket | meaning | safe to delete? |
|---|---|---|
| **A** | tip is an ancestor of `origin/main` | yes — content is in `main` |
| **B** | not an ancestor, but the content is provably preserved elsewhere | yes — see per-branch evidence |
| **C** | its PR was closed **without** merge — abandoned by human decision | yes |
| **D** | unique commits not in `main`, and no evidence they landed another way | **NO — left for a human** |

Counts: **A 7, B 30, C 1, D 71** (of 109 candidates).

Bucket **B** is established by one of three mechanical tests, recorded per branch:

1. `git cherry origin/main <tip>` returns no `+` line — every patch is already upstream (catches squash-merges and rebases);
2. every unique commit *subject* also appears in `main`'s history (catches a squash that changed content);
3. the tip is an **ancestor of another live branch**, so the commits survive there;
4. or `main` is byte-identical to the tip on every file the branch touched.

## Still binding

- nothing younger than 6 h; nothing matching a lane running right now;
- never `main`, `gh-pages`, a release/tag branch, `coordination-claims` (the new
  claim protocol lives there) or `__dolt_remote_info__`;
- name + tip SHA recorded **before** deletion, ancestor/identity re-verified
  immediately before each batch, deleted in batches;
- every branch below restorable with `git push origin <sha>:refs/heads/<name>`.

## Bucket A — already in `main` (7) — DELETED

| branch | tip SHA | evidence |
|---|---|---|
| `agent/kabir-claude/amsmath-inline-2` | `794c031298f8e96175da818657a8e9de2681793d` | tip is an ancestor of origin/main |
| `agent/kabir-claude/math-amssymb-braces-pipeline` | `d298254646bd08601ec985af94f78bfba4fe08c5` | tip is an ancestor of origin/main |
| `agent/kabir-claude/math-nested-grids-pipeline` | `322513f15171ffef1242d4623b8cbf7535169c7b` | tip is an ancestor of origin/main |
| `agent/kabir-claude/microtype-adoption` | `16de1ffc71a58adf62fe1f225d98b07fe43b21e0` | tip is an ancestor of origin/main |
| `agent/kabir-claude/render-pipeline-class-geometry-3r` | `3b028fc6d9b9379485773400ac23807375ee7dfc` | tip is an ancestor of origin/main |
| `agent/kabir-claude/tabular-array-pipeline` | `a6e138b1435743b4f79b4a2e9ab74607c910b425` | tip is an ancestor of origin/main |
| `agent/kabir-claude/text-logos-rule-symbols-pipeline` | `1c1318ae799cf6b9e96850510bc63aba6543c83d` | tip is an ancestor of origin/main |

## Bucket B — superseded or finished, content preserved (30) — DELETED

| branch | tip SHA | evidence |
|---|---|---|
| `agent/aarush-macbook/pdf-output` | `96f1d9681b4c3da28f1f32b8dcb6c4321bd5f2fd` | tip is an ancestor of live branch(es) `agent/aarush-macbook/incremental-reuse` -- content preserved there |
| `agent/claude-dq222/register` | `cc5c05bea33aab04ab0909c49e74d594df41dd30` | tip is an ancestor of live branch(es) `agent/daniel-parent/supervisor` -- content preserved there |
| `agent/commander/integrate-ft002-compiler` | `dee12f5dca144768d1bed1e096ef0f1e2b5db2c7` | git cherry origin/main: 0 commit(s), none marked + (all patches already upstream) |
| `agent/commander/integrate-ft011-corpus` | `9deda1df2afc08352480b8c48b82983982aaf4af` | git cherry origin/main: 0 commit(s), none marked + (all patches already upstream) |
| `agent/daniel-grok-coverage/compiler` | `252f99f471018fded83eb41b634c625dfb58f1ab` | all 2 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/daniel-parent-b/completion-vocab-sync` | `5efd4f3079c9158eb2b4ed7a9929ab3067dd965b` | tip is an ancestor of live branch(es) `agent/daniel-parent-b/lm-math-symbols` -- content preserved there |
| `agent/daniel-parent-b/sqrt-vinculum` | `62e66d75aba6339b81318cf9d849f24c0ff4916b` | git cherry origin/main: 0 commit(s), none marked + (all patches already upstream) |
| `agent/daniel-ux-cmdr-dataloss` | `9e54d8f3d410bfd623a26f022ea83fa8866d67f9` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |
| `agent/daniel-ux-editor-keys` | `31a5fa26a47e09b3fdda011822c6bd67543bca63` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/daniel-ux-find` | `db106508c31b4ef9dfd5827d83cfe93e5dae3c1d` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/daniel-ux-spellcheck` | `b2ddf771d1cdfe20de0e89bde804f8c668b4927c` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/daniel-ux-wordcount` | `6ef344d7b739283c364bf91aeec5b14cdf71c133` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/kabir-claude/amsmath-envs` | `7a816f31c5022f2e18502739846d575f9ff31445` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/kabir-claude/compiler-perf-r1` | `f4d7a5a3196f19c70b03f8310716f440cb989008` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/kabir-claude/conversion-provider` | `196ab0280f6c283a05a71f894c31eeb755f47874` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |
| `agent/kabir-claude/deterministic-font-warning` | `c59d2c66475489d51baeebe07d4b5d4d96ce998f` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |
| `agent/kabir-claude/graphics-floats` | `b2726754bfbddf8116da0340b83d4e445fdcd83a` | tip is an ancestor of live branch(es) `agent/kabir-claude/integration` -- content preserved there |
| `agent/kabir-claude/hw2-math-final` | `6492bf532000f83bb116bf61c554b8e1385d0c4f` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/kabir-claude/hyphenation` | `0820d0e2164f7be3b300489268ca83d8f499bc0e` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |
| `agent/kabir-claude/supported-latex` | `54f1e78b3502ba7bbf90c5787a72f5900ff04685` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/kabir-claude/supported-latex-r1` | `92ff59eb3e0e6b8f37b9328afde27df8f2f145be` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/kabir-claude/tikz-min` | `cf81fc41916c5d7688ebe38c33ec29c56adf39c6` | all 1 unique commit subject(s) also appear in main's history (squash/rebase landing) |
| `agent/mac-ai-review-2/recovery-applied` | `9710f07c8d4ecd7f867a967a6c8405ede7570cdd` | git cherry origin/main: 2 commit(s), none marked + (all patches already upstream) |
| `agent/mac-diagnostics-2/partial-output-applied` | `afc8adf94f75a45abeebcc03317eb4cd4e78c44f` | git cherry origin/main: 2 commit(s), none marked + (all patches already upstream) |
| `agent/mac-editor-a11y-2/large-doc-applied` | `9f7d5f518604816fcc0bbf82aa9b8283c0b7c04f` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |
| `agent/mac-export-ux/export-ux-applied` | `6aacda63333950c1ec83acaa392fabb1fdfa0722` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |
| `agent/mac-math-layout/math-boxes` | `57cbc444d208885275aaabf0df23c179fd3d71f4` | tip is an ancestor of live branch(es) `mml-rev4` -- content preserved there |
| `agent/mac-pdf/exact-export` | `bc43704f814be2de7937e6f233d9732b1749343e` | git cherry origin/main: 4 commit(s), none marked + (all patches already upstream) |
| `agent/mac-pdf/fidelity` | `a3536c2f58a5f9f4f3475bc02f48e0298e50d7e2` | tip is an ancestor of live branch(es) `agent/mac-pdf/searchable-text` -- content preserved there |
| `agent/mac-preview-anchoring/anchoring-applied` | `5d1503e741b796082ac4e62149e4541953afb7c1` | git cherry origin/main: 1 commit(s), none marked + (all patches already upstream) |

## Bucket C — abandoned by human decision (PR closed unmerged) (1) — DELETED

| branch | tip SHA | evidence |
|---|---|---|
| `agent/kabir-claude/render-pipeline-class-geometry-2` | `aaeb4449636c47f00327a6518501a298ef4637a8` | PR #102 closed WITHOUT merge (2026-09-13) |

## Bucket D (71) — NOT deleted, human decision required

Each of these holds commits that exist **nowhere else**: not in `main`, not in
another live branch, and with no PR to point at. Deleting one would lose work,
so none were touched. The "unique" column is the number of commits not upstream;
"differs" is how many of the files it touches still differ from `main`.

| branch | tip SHA | age | unique commits | files / differ | what it holds |
|---|---|--:|--:|---|---|
| `agent/aarush-macbook/companion-capture` | `98021805ada6b85b9d7ab85b30e4292bf150322c` | 27 h | 14 | 25 / 25 | companion: scaffold PencilKit and camera capture app |
| `agent/aarush-macbook/incremental-reuse` | `0236aa0a74413fd997c15b31b885bfde941c4325` | 27 h | 6 | 10 / 10 | compiler(FT-005): add PDF output via pdf-writer 0.15 |
| `agent/aarush-macbook/register` | `5db9d2ca61d29e0d4e3b60a8be2cbc3f204a6630` | 39 h | 1 | 1 / 1 | register: aarush-macbook resource inventory |
| `agent/aarush-macbook/test-coverage-v2` | `ba1ca4b9ad424c96067c631d17f7507295e50944` | 27 h | 6 | 6 / 6 | test(compiler): add 51 unit tests for Span, Diagnostic, and lexer |
| `agent/chatgpt-a/companion-project-repair` | `699e1bf6ff3759e14d4bdeec7c5e6b781b7ec7fc` | 38 h | 7 | 22 / 22 | companion: scaffold PencilKit and camera capture app |
| `agent/chatgpt-a/companion-reliability` | `439c8cc44c7e8ee2e685822418bcf5371e22edcb` | 26 h | 22 | 24 / 24 | companion: scaffold PencilKit and camera capture app |
| `agent/chatgpt-a/companion-validation` | `46c1c9d480db086308ec98ac7dcfb48816118e29` | 25 h | 14 | 5 / 5 | coordination: acknowledge FT-014 companion validation |
| `agent/chatgpt-a/register` | `af55b9e73a839718cdc4c3d41fdd43fe35f25cc6` | 38 h | 3 | 2 / 2 | coordination: register chatgpt-a |
| `agent/claude/compiler-foundation` | `49e6eb43808ac8fccb08d620b23667403fcb8667` | 26 h | 13 | 31 / 31 | compiler: burst-edit gates, wider pinned corpus, parity published sepa |
| `agent/claude/kabir-mac-max-plan` | `f1d5a7180da326c133c85a01aee9cfe49565573d` | 14 h | 1 | 2 / 2 | resources: record Claude Max plan on mac-m5pro-kabir |
| `agent/commander-corpus/font-resources` | `55bfd48ebd51e788926f767b585130bd833d55c1` | 32 h | 1 | 1 / 1 | Report verified rational Type1 matrix checkpoint |
| `agent/commander-preview-performance/preview-performance` | `ff4282300503036552d62021d07fd1275381dd58` | 25 h | 7 | 22 / 22 | Checkpoint Text integration review and unchanged ownership decision |
| `agent/commander-render-core/rendering-core` | `05418407d2b568c206aab1d0de119381d8118c71` | 25 h | 2 | 59 / 59 | Verify searchable Text exports and isolate existing whitespace extract |
| `agent/commander-runtime-display/display-runtime` | `f15edf29bf50df030f7cce0f39b929192775dc3b` | 25 h | 1 | 1 / 1 | docs(runtime): audit lazy source hashing and reject speculative reuse  |
| `agent/daniel-calc/tex-calc` | `8d9da162a230c331957bf47f8c7eea034983d939` | 24 h | 13 | 13 / 13 | Add tex-calc: bounded TeX dimension expression evaluator |
| `agent/daniel-color/color-expressions` | `171f925f8ffac63e080345b93c639412881c564e` | 26 h | 5 | 6 / 6 | FT-035 rev3: adversarial bounds and exact identity regressions |
| `agent/daniel-contents/toc-layout` | `b5534602b8adcd29c6af737d17c9ae350712d24a` | 24 h | 5 | 12 / 12 | FT-034 rev 3: adversarial bounds and exact identity regressions for to |
| `agent/daniel-corpus-sweep/report` | `c1ba48ea2c6418f2a32ffc3582e6c48e80167f44` | 23 h | 1 | 2 / 2 | coordination: add extended-tex-corpus compiler sweep report |
| `agent/daniel-floats/float-layout` | `20543da65a55959cdaca33f3f1cafc95287d80e5` | 24 h | 12 | 17 / 17 | font-engine: bounded validated TTC face/table directory layout |
| `agent/daniel-font-route-study/report` | `4d72260c543937f67e10f8d12704fd902941a866` | 23 h | 3 | 1 / 1 | coordination: partial mathbb font-route notes handed to daniel-parent- |
| `agent/daniel-footnotes/footnote-layout` | `a2cbb8793f67977e9ccad94eb2adaba8b2e22472` | 24 h | 9 | 13 / 13 | math-layout: indexed roots, delimiters, script geometry, clean-rebuild |
| `agent/daniel-hw1-packages/compiler` | `fd24147d1ce45d1cc191e76d7c1da87dc71aa442` | 23 h | 1 | 2 / 2 | compiler: document package-option matching, setlist, and enumitem labe |
| `agent/daniel-hw1-visual/audit` | `c16590542e0286575446b6689a496020e1b61457` | 23 h | 1 | 2 / 2 | coordination: HW1 visual audit — ranked rendering defects vs reference |
| `agent/daniel-images/image-assets` | `6c84d88754259ea7b09e55be8aaa6ed81b059cc7` | 25 h | 7 | 8 / 8 | Add flashtex-image-assets: rooted PNG/JPEG document asset loader |
| `agent/daniel-links/link-annotations` | `1d9028d1041721dffd3371de3f292d3a0d5b2078` | 24 h | 9 | 15 / 15 | Add link-annotations crate: typed, source-mapped hyperlink model |
| `agent/daniel-math-accents/compiler` | `67cd50cc392620bf7f2191bd04342d5ec96f2036` | 23 h | 4 | 6 / 6 | compiler: declaration-scoped \tiny..\Huge and \setlength parindent/par |
| `agent/daniel-math-access/math-accessibility` | `fa9e674b69b508478c9488e6d223411fd9c5c55c` | 24 h | 9 | 7 / 7 | math-accessibility: semantic readable/MathML adapter over math-layout |
| `agent/daniel-parent-b/lm-math-symbols` | `5a7be8bbbfadc15505844e8c70f73217e9df1a4a` | 23 h | 6 | 13 / 13 | mac: sync completion vocabulary with compiler on main after rebase |
| `agent/daniel-parent-b/mac-ui-rebase` | `490474a9ab59008e3b9eea0b1c7f77e0a7a2223b` | 23 h | 1 | 35 / 18 | mac: sync completion vocabulary with compiler on main after rebase |
| `agent/daniel-parent/compile-latency` | `24f285af461e5ea4fd1ba1179f11357ed1fe44f7` | 14 h | 2 | 6 / 4 | compiler: shift reused incremental blocks in place and add edit latenc |
| `agent/daniel-parent/corpus-quickwins-2` | `d6afc285ea2f755ccd131da1bf3a5afe8eac5905` | 15 h | 3 | 3 / 3 | compiler: add \verb and \url as literal text commands |
| `agent/daniel-parent/figures` | `59e45f95e81a9c70f17bbc9eab78031598b77811` | 15 h | 2 | 9 / 9 | compiler: figure/table floats, numbered captions, sized \includegraphi |
| `agent/daniel-parent/hw1-integration-v2` | `e00edb2503668caabb6423f6e9243e0ae3f2f73a` | 23 h | 1 | 4 / 4 | compiler: declaration-scoped \tiny..\Huge and \setlength parindent/par |
| `agent/daniel-parent/ledger` | `d2eff69146fc421dd782c50c95a10a655a135721` | 23 h | 2 | 1 / 1 | coordination(daniel-parent): lane ledger checkpoint 19:55Z for resumpt |
| `agent/daniel-parent/mac-ui-redesign` | `2362287ce0b964f6107bbb920885223a83cedf6f` | 24 h | 1 | 1 / 1 | coordination: Mac UI design spec and critique (daniel-parent lane) |
| `agent/daniel-parent/supervisor` | `876bb668d3f31484ee0b9c08d3551cf21524d1ec` | 24 h | 31 | 17 / 17 | Register mac-m5pro-dq222 offering Claude Max 20x and Cursor Pro |
| `agent/daniel-parent/unowned-crate-hardening` | `53f3058971aca44585aa70f7c43924ccbbbfb76a` | 23 h | 4 | 4 / 4 | project-files: fix TOCTOU symlink race in graph discovery (GH#45) |
| `agent/daniel-snippets/editor-snippets` | `490538db3f1e4401e1e8f9ad436b66d0c5047943` | 26 h | 8 | 20 / 20 | Add editor-snippets crate: bounded, UTF-8-safe snippet expansion |
| `agent/daniel-statistics/document-statistics` | `866a9c7c3dc95728bca8a5a6754bc887b5e0bbc8` | 24 h | 9 | 18 / 18 | Add document-statistics crate: word/math/page counts with exact revisi |
| `agent/daniel-templates/project-templates` | `940b687829198c2fd97468be417317094604280d` | 25 h | 9 | 15 / 15 | Add flashtex-project-templates: undergraduate templates + rooted insta |
| `agent/daniel-tex-ligatures/compiler` | `2d336ff88c2b83c53b4dc32321ec861238038ecb` | 23 h | 1 | 3 / 3 | compiler: convert TeX text-mode input ligatures before layout |
| `agent/daniel-text-styles/compiler` | `2477e1c0c4c0114e4ea1b58a81a93f8533a803d3` | 23 h | 3 | 8 / 8 | compiler: real text styles for \textbf, \textit, \emph and font declar |
| `agent/daniel-title/title-layout` | `f9b94af68e31bef3f566dd40ccdf4827f6c36a99` | 25 h | 6 | 12 / 12 | title-layout: adversarial bounds and exact identity regressions (FT-03 |
| `agent/daniel-ux-project-files` | `8081f239ea1862689c2f299e907cf8c6b69651af` | 14 h | 1 | 7 / 7 | mac: real filenames and automatic included-file loading |
| `agent/kabir-claude/compiler-perf` | `196d580fea78f4d373b25bedfd0ddd84625628bc` | 13 h | 4 | 43 / 13 | compiler: shift reused incremental blocks in place; cut warm reply ser |
| `agent/kabir-claude/graphics-floats-r1` | `a6bc41aaaafb5b94abdc969f1300abfcd5807f2f` | 13 h | 1 | 46 / 15 | render-pipeline: \includegraphics and figure/table float placement (FT |
| `agent/kabir-claude/graphics-floats-r2` | `357be33d38f72779f52bd87d11aa71600d455110` | 11 h | 1 | 46 / 15 | render-pipeline: \includegraphics and figure/table float placement (FT |
| `agent/kabir-claude/integration` | `6be162682904e543bf776a8eba4a86cecbfbb503` | 13 h | 6 | 304 / 151 | compiler: \mathcal from New Computer Modern Math, msbm \varnothing, HW |
| `agent/mac-bibliography-kinds/bibkinds-applied` | `6a73f6d6f14facbc430c66d4fd3d55b41c6f030c` | 29 h | 2 | 2 / 2 | mac: launch-time filter for persisted bibliography declarations + shel |
| `agent/mac-citation-rename/citation-rename-applied` | `5e4a175ce48423c305a8cad75a3a2b8437967a1e` | 29 h | 1 | 1 / 1 | mac (APPLIED, parent-retained): FlashTeXMacApp hook for Rename Citatio |
| `agent/mac-claude-a/fontmetrics` | `30a14a6c16e79bc344f381e79d8d6f78c0e6fa33` | 38 h | 1 | 8 / 8 | fontmetrics: base-14 advance tables measured via CoreText (proposal fo |
| `agent/mac-claude-a/nearby-client` | `6e1510404eb14905fd5aa89c954ac14a2a582630` | 35 h | 1 | 13 / 8 | mac: reference nearby companion client (Swift package + CLI) with e2e  |
| `agent/mac-claude-a/preview-v2` | `bc28c59a9715172052e2d8521064eeb045114875` | 35 h | 2 | 14 / 13 | mac: rendering-v2 display-list model, hash-resolved glyph-run renderer |
| `agent/mac-claude-a/register` | `431889cbb426f320e7080601eb8e9cbaeb3bfdca` | 39 h | 1 | 2 / 2 | coordination: register mac-m1max-a worker and resource inventory |
| `agent/mac-claude-a/typing-bench` | `854bf7cbada6b9e3adfabdd3ddc6382ff109518b` | 35 h | 1 | 14 / 14 | typing-bench: full 12-cell evidence (compiler + flashtex-render × 3 se |
| `agent/mac-contract-review/checker` | `6f2420d24ece49058fcdbf60a95457398b811f2d` | 37 h | 1 | 4 / 4 | test: add pinned Mac bridge contract review checker |
| `agent/mac-diagnostic-explanations/explain` | `2bf14cd9b829736168b9ebd807016ae7c11b28b1` | 37 h | 3 | 16 / 16 | coordination: register mac-diagnostic-explanations (issue #2) |
| `agent/mac-font-engine/tex-fonts` | `73422451ec367269d8ce2ba4554ba5103224a262` | 35 h | 1 | 37 / 3 | wip(mac-font-engine): preserve in-progress work after subagent termina |
| `agent/mac-helper-display/route-applied` | `7083f3e79754a845ab2432b336a085dec8abb227` | 28 h | 1 | 3 / 3 | LOCAL APPLICATION: parent-retained hook lines for the display-candidat |
| `agent/mac-large-document/large-document-applied` | `463c0c53c30c20e77e717391dd669e88e3429564` | 28 h | 1 | 4 / 4 | coord: correct the Claude-Session trailer of 9542a24b (typo: …PnmBYXg… |
| `agent/mac-pdf/searchable-text` | `b4b15136f54dd3dfe0fe48f682f07f3e7e127c8f` | 24 h | 1 | 6 / 3 | pdf docs: HW1 replay evidence (3232 glyphs on their display-list origi |
| `agent/mac-pdf/v2-adapter` | `20e5277857b2cd37f102fb07acdd164f82bb49db` | 32 h | 5 | 15 / 10 | pdf: prune CFF subroutine and string INDEXes in subsets; inlined-chars |
| `agent/mac-preferences/editor-applied` | `314d6f1167591880a4995ca27c8301f60fc79450` | 32 h | 1 | 5 / 5 | mac(LOCAL APPLICATION, not for integration): wire EditorPreferences in |
| `agent/mac-project-files/graph` | `bf462b92c0a6aceee1a3e0709b2e5fa8ad361d0d` | 36 h | 2 | 2 / 2 | coordination: mac-project-files handoff for #18 fix at d92db37 |
| `agent/mac-render-pipeline/delta-proposal` | `6965c9bef0319de5ddc6a7a1e027893926a59eef` | 24 h | 10 | 11 / 11 | render-pipeline: display-list-v2-delta proposal (schema + acceptance p |
| `agent/mac-validation/native-verification` | `51c8c776d241abbce7a2728f06f8fb515e2df09d` | 36 h | 7 | 36 / 36 | validation: native verification suite and first evidence run (FT-010) |
| `agent/mac-visual-oracle/reference-raster` | `dda820017306aecef5ce733e8eb86e7c2b51ce36` | 32 h | 16 | 2611 / 2611 | visual-corpus: reference-render and raster-diff harness with first evi |
| `agent/orchestrator-jaysen-opus/standby` | `4a462258448bfbae07326b99580e69acc3bac881` | 35 h | 1 | 7 / 7 | standby: read-only Jaysen Opus standby monitor, fail-closed revival ga |
| `commander/handover-claude` | `a48f16808d93b9ff57f264979c566a58688261d0` | 26 h | 1 | 4 / 4 | coordination: fix fetch-error visibility and branch-age lookup |
| `mml-rev4` | `ac3bf0b10a5cf348eb64de2f1316701383c682a8` | 35 h | 13 | 75 / 73 | math-layout: extensible delimiters/radicals, over/underline, text oper |
| `mpl/linebreak-rev4` | `bad0666f13e006f967834d5f0ce6c5cf25f6eb27` | 35 h | 1 | 7 / 7 | wip(mac-paragraph-layout): preserve in-progress work after subagent te |

Suggested disposition: ask each owning machine whether its lane is finished. A
bucket-D branch is only safe to delete once someone can say where the work went.

