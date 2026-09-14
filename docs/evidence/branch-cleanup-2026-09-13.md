# Remote branch cleanup — 2026-09-13

Lane: integration-and-hygiene, `kabir-claude` on linux-primary (NixOS PC).
Owner asked for merged, unused branches to be cleaned up.

**This file is the restore record. It is committed and its PR opened BEFORE any
deletion.** To bring any branch below back exactly as it was:

```sh
git push origin <tip-sha>:refs/heads/<branch-name>
```

Every tip SHA listed is an ancestor of `main` at `0530666d`, so the commits
themselves are still reachable from `main` and cannot be lost by the deletion.

## Deletion rule set

A remote branch is deleted only if it satisfies **all four**:

1. `git merge-base --is-ancestor <tip> origin/main` succeeds — nothing is lost;
2. it has **no open PR** (checked against `gh pr list --state open`, 53 open PRs at the time);
3. its tip commit is **older than 3 hours** (younger branches may belong to lanes running right now);
4. it is not `main`, `gh-pages`, a release/tag-tracking branch, or `__dolt_remote_info__`.

The ancestor check is re-run immediately before each individual deletion, because
branches can move between the survey and the delete.

Additional conservative exclusion applied beyond the four rules: any branch checked
out in a live local worktree on this machine was excluded from the candidate set
(none of the candidates turned out to be, so this excluded nothing).

Survey taken 2026-09-13 ~18:10Z against `origin/main` = `0530666d`.
Totals: 488 remote branches, 335 merged, 153 unmerged, **318 deleted**, 16 merged-but-kept.

## Merged branches KEPT (not deleted) and why

| branch | tip | reason kept |
|---|---|---|
| `agent/kabir-claude/amsmath-inline-2` | `794c031298f8` | open PR #109 |
| `agent/kabir-claude/math-amssymb-braces-pipeline` | `d298254646bd` | open PR #127 |
| `agent/kabir-claude/math-nested-grids-pipeline` | `322513f15171` | open PR #125 |
| `agent/kabir-claude/microtype-adoption` | `16de1ffc71a5` | open PR #119 |
| `agent/kabir-claude/render-pipeline-class-geometry-3r` | `3b028fc6d9b9` | open PR #106 |
| `agent/kabir-claude/tabular-array-pipeline` | `a6e138b14357` | open PR #121 |
| `agent/kabir-claude/text-logos-rule-symbols-pipeline` | `1c1318ae799c` | open PR #129 |
| `agent/mac-claude-a/mac-shell` | `d46a0f63de30` | tip only 0.1 h old — lane may be live |
| `agent/mac-cli/flashtex` | `faa319d1847c` | tip only 2.2 h old — lane may be live |
| `agent/mac-includes-auto/compile-closure` | `adc59850defe` | tip only 2.5 h old — lane may be live |
| `agent/mac-new-project/scaffold` | `d4d7b8e74e26` | tip only 2.5 h old — lane may be live |
| `agent/mac-paths/v2-consumer` | `12c17a5adbb6` | tip only 2.5 h old — lane may be live |
| `agent/mac-render-pipeline/corpus-fidelity` | `ec10bc618717` | tip only 2.2 h old — lane may be live |
| `agent/mac-render-pipeline/inline-math-breaks` | `a2f85bfa3433` | tip only 2.5 h old — lane may be live |
| `agent/mac-render-pipeline/perf-1` | `9d5aa4ba666b` | tip only 0.2 h old — lane may be live |
| `integration/2026-09-13l` | `5618f5437921` | tip only 0.5 h old — lane may be live |

## Branches deleted (318) — restore records

### (top level) (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `site-ui-redesign` | `c3c1cb20a4e36acd5e3c09a8ff63dad66dba79ca` | 13.4 h |

### agent/bridge-context (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/bridge-context/conversion-jobs` | `406642e678e26b929f8d277a85f74a2d8aa1e597` | 33.7 h |
| `agent/bridge-context/project-context` | `89198703020976a3ed656410697ca85bd6fd3bdb` | 36.5 h |

### agent/claude (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/claude/machine-resource-inventory` | `ec0dac7c535d3de55dbc0b51694f30608717f5f0` | 38.4 h |

### agent/codex (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/codex/collaboration-rules` | `92e9595d9afeff5a62230ad02c7cb0db5889b617` | 39.4 h |
| `agent/codex/resource-context-policy` | `b37237b6987c316f60f47d1f8bbb57b07741947d` | 38.9 h |

### agent/commander (15)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander/assistant-context` | `c93bf0d7fd6c44bc656fdcb905e9a102e6cfad40` | 33.7 h |
| `agent/commander/capture-bridge` | `b5ca96bdaca634166a01023cd3321955f2bc5f70` | 37.3 h |
| `agent/commander/coordination-tools` | `25a92a9c591a92a3d49071e536ecf8f129112c24` | 37.8 h |
| `agent/commander/dispatch-service` | `c5279d0a2a333b60d932221bae3b2b55f126d0ac` | 25.4 h |
| `agent/commander/document-runtime` | `90b6b46ecbd206a4db77847c42b8004c417f55b5` | 36.2 h |
| `agent/commander/integrate-ft002-compiler-r2` | `60ceb5c3caedad2cb06d2862836d3f97dc04067b` | 37.4 h |
| `agent/commander/integrate-ft007-context` | `9da7e48148d91872dc0514119e1a04568ebfcb51` | 36.8 h |
| `agent/commander/integrate-ft011-corpus-r2` | `2e619e41b5698f64bb2c4559e8826d546a03b3f2` | 37.6 h |
| `agent/commander/integrate-ft012-runtime` | `a528ef9b0478033b77c158a7d7946ab2acc558c4` | 37.6 h |
| `agent/commander/integrate-ft016-dispatch` | `342e1e029f80e7a5b472ad202ae87d597274127b` | 37.3 h |
| `agent/commander/integrate-kabir-inventory` | `d432341e19abdc69aac2aa285237dcc57860d153` | 38.2 h |
| `agent/commander/orchestration-plan` | `276bb1c8586aa84fd548520c84423bc1812f9713` | 38.8 h |
| `agent/commander/orchestrator-sol` | `984fa28f2537feadb9f848eb90f69de0327d1fa5` | 36.8 h |
| `agent/commander/preview-controller` | `41602c020f78d68ae94ded1be0b9b461e5acb93c` | 34.8 h |
| `agent/commander/proposal-validation` | `137a8327a15f6cd85fbfbdbbf2c2ca69f753e1eb` | 36.5 h |

### agent/commander-corpus (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-corpus/compatibility-fixtures` | `b64472f4bdbeee50c69835f2b2b7e2d8f5d2f9e5` | 36.2 h |
| `agent/commander-corpus/visual-fixtures` | `d5014665e64bb051010637f876e35070dc035e58` | 36.6 h |

### agent/commander-fleet (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-fleet/fleet-health` | `b3707fe0268ca0a862166bad1178d71c064eb199` | 36.2 h |

### agent/commander-project-index (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-project-index/source-navigation` | `2d88f860b929d60b4722076191f2a4f1bbcc0467` | 33.7 h |

### agent/commander-protocol (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-protocol/runtime-validation` | `f596c4c359e435c1453eaff95d369cbc2f83d587` | 36.4 h |

### agent/commander-raster (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-raster/exact-comparison` | `e3b3ddfb7f85042c50ffe51e449e8bd8b691ba20` | 36.6 h |

### agent/commander-render-schema (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-render-schema/rendering-v2-schema` | `41cacfc5f6728fb8443172aa62adbe1caefcdcbb` | 36.6 h |

### agent/commander-rendering (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-rendering/rendering-contract` | `65e265ead8f331aabecbf343305035e0beb44a11` | 36.4 h |

### agent/commander-runtime-performance (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-runtime-performance/runtime-performance` | `a47bd3848073bbf146afa850fe3bd8504e994137` | 33.2 h |

### agent/commander-supervisor (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/commander-supervisor/claude-supervisor` | `056170d4c1997a47373b7089e15dcbe555dffbba` | 36.7 h |

### agent/daniel-bundle (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-bundle/project-bundle` | `0dcfc6b8f2a97c967181e545a4fe9ed15f27d1e2` | 23.2 h |

### agent/daniel-collaboration (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-collaboration/collaboration-core` | `dbb9bcb818baf06752ab2b4d2f399b31e52bfe47` | 23.5 h |

### agent/daniel-corpus-quickwins (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-corpus-quickwins/compiler` | `5d857b902be3753c9d05fc694e0a91feb3dbf1f9` | 22.2 h |

### agent/daniel-fable-ui-qa (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-fable-ui-qa/mac` | `f46558669d8735bf64703fa6d9a9670196407782` | 22.2 h |

### agent/daniel-grok-corpus (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-corpus/bridge` | `d483c01beb0e52c152bb97aefbe25828c2c012fc` | 23.0 h |

### agent/daniel-grok-coverage (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-coverage/compiler-clean` | `8ca903e5363a5d46c6f1dbd23ee584c1a9116b4d` | 22.2 h |

### agent/daniel-grok-durability (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-durability/bridge` | `0a9c5eef682bcb5279f68a029061803060f3f2ee` | 23.1 h |

### agent/daniel-grok-insert (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-insert/bridge` | `f643ddf8f099f8fe5e44e79055f58e06c0ffccd7` | 23.1 h |

### agent/daniel-grok-ipad (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-ipad/bridge` | `2d18c777e7e3f98a3064986762203a6f5896f174` | 23.2 h |

### agent/daniel-grok-live (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-live/bridge` | `479764d929e97a660b0c6bdef37cf80cf8172654` | 23.2 h |

### agent/daniel-grok-macapp (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-macapp/bridge` | `f9a630422f011f2fd39949718f32cb593dd981c3` | 23.1 h |

### agent/daniel-grok-photo (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-photo/bridge` | `a87c31871d4d095cbdf341e1bf494fec62fdf0f5` | 23.1 h |

### agent/daniel-grok-reliability (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-reliability/bridge` | `ff7e24a9dcc1bbc5849d3a149e26a7b3badc821c` | 22.8 h |

### agent/daniel-grok-safety (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-grok-safety/bridge` | `503645d0d40b0bc3c0bbfc8ce82f289b5d986500` | 23.0 h |

### agent/daniel-hfill-recovery (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-hfill-recovery/compiler` | `128ec1eda739c3eb8a3ddce08cc2f7c0f43d53f3` | 22.0 h |

### agent/daniel-hw1-preamble (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-hw1-preamble/compiler` | `70dc5dd55ae2cb2a814d107382b3209402432419` | 22.6 h |

### agent/daniel-math-amsmath (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-math-amsmath/compiler` | `a434caecb50eaeab08576a3e4c379fbe06dce753` | 22.6 h |

### agent/daniel-math-feat (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-math-feat/compiler` | `a4e20cb933e705980b204f2082c17f282797b7e8` | 22.7 h |

### agent/daniel-math-symbols (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-math-symbols/compiler` | `6f1f6b98ee1a04842fcb538516192a5d8d0d08a1` | 22.6 h |

### agent/daniel-parent (25)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-parent/accent-skew` | `1c186dc35b62ab211faed64679212cc7c7e3d896` | 14.4 h |
| `agent/daniel-parent/amsthm` | `50e3dd57235ee112c66433cdf240a37ec1c8e331` | 14.0 h |
| `agent/daniel-parent/big-delimiters` | `7fea55fa5f961d46b0ecac329838bd967154d322` | 14.3 h |
| `agent/daniel-parent/bounded-frame` | `3ffdabe402bd883ed3a568471d86900784d3e242` | 14.1 h |
| `agent/daniel-parent/cite` | `e204ff439e134d0b152e27f6158fa927e68b8d3f` | 13.9 h |
| `agent/daniel-parent/diagnostic-codes` | `4e44fb2bfadd82be51d8593d95b2e9d4996d9594` | 13.9 h |
| `agent/daniel-parent/export-lm-math` | `ce78ad8ffb9ab718f829931d2982bd0cbdb1ed51` | 14.6 h |
| `agent/daniel-parent/footnotes` | `2473a3a7d16ae69c973881a83192f8dfa72abb17` | 14.1 h |
| `agent/daniel-parent/hw1-integration` | `8fb22a43e1f6ff92ca885fc8ddd7ca220fa66ea0` | 22.4 h |
| `agent/daniel-parent/hw2-gate` | `9ea15e32fc8cad056995da839cfae80b6bcd762b` | 13.8 h |
| `agent/daniel-parent/hw2-math` | `b3d9664c8d12eba246dba66b9ec5a7c1ebbfaa3b` | 13.8 h |
| `agent/daniel-parent/hyperref-text` | `8992fd8b0168f2a9f183cd816f765b7980195542` | 14.1 h |
| `agent/daniel-parent/integration` | `fa4ef7481f826dff155cbe1e96df1b6dfac1d2bd` | 14.0 h |
| `agent/daniel-parent/justify` | `16cd61910398f8d99183b6599913fecbf73ffdef` | 14.4 h |
| `agent/daniel-parent/leftmargin-star` | `3e59ded638a5b0f54b242f1c17301831ab4da0fb` | 14.2 h |
| `agent/daniel-parent/lm-math-symbols-2` | `cb4f1dc51b2b6cad7530da894da810b73379a9f7` | 14.1 h |
| `agent/daniel-parent/local-recovery` | `776c21d73e85574007fe2758d23bc43d0213aa69` | 13.9 h |
| `agent/daniel-parent/maketitle` | `6abf269e2677c7cde7a1ccaf7fff09deff5672cf` | 13.9 h |
| `agent/daniel-parent/page-control` | `98d8a3f98c32d3b2d7a0bbaa5bb8644813522850` | 14.0 h |
| `agent/daniel-parent/project-files-fix` | `e7f301b1b2a4c575586343ec8211651250553c66` | 23.5 h |
| `agent/daniel-parent/py39-coord-tests` | `3ab7048fecb9c30e0096d33f830710c18cf06b1b` | 23.8 h |
| `agent/daniel-parent/tabular` | `45a6dcf034aaccc4222d01082ce673d1f249002c` | 13.9 h |
| `agent/daniel-parent/unbraced-args` | `302043440d08d5fbb345daaa5f1f61559ab50f58` | 13.8 h |
| `agent/daniel-parent/verbatim` | `8ee996084702208f3ebaa0aa0818b39116d32e4e` | 13.9 h |
| `agent/daniel-parent/xref-toc` | `00876672e0c4be428bf81d343b2f6cf0df8db40c` | 14.1 h |

### agent/daniel-parent-b (10)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-parent-b/inline-spacing` | `42557b09cddc28e2c22aeb372458b4ce8d72fe27` | 18.9 h |
| `agent/daniel-parent-b/list-indent` | `56b5280cf8557e7833303614eb85ff2daa006f60` | 19.1 h |
| `agent/daniel-parent-b/math-italic` | `391de812b41e598e148169239af27adf0af72392` | 19.6 h |
| `agent/daniel-parent-b/math-spacing` | `26c65b3121a937ce11f64b924244f7ccfaaea973` | 19.4 h |
| `agent/daniel-parent-b/paragraph-layout-forced-break` | `b8994294543cb27b32927843439e7569c3145fdc` | 19.7 h |
| `agent/daniel-parent-b/pdf-lm-math` | `026616dfd03ff0417109186345155dc8ff200a5f` | 19.4 h |
| `agent/daniel-parent-b/preamble-lengths` | `4b11df28839474a68fc1b2d9738d0d11263feb9a` | 19.8 h |
| `agent/daniel-parent-b/rule-gap` | `dc3dccd9473c88e18104dde92e08115b3de5b53e` | 19.5 h |
| `agent/daniel-parent-b/setlist-spacing` | `5d555c52829caf6cd7eeef59cd97f627cfca9bf9` | 19.4 h |
| `agent/daniel-parent-b/size-commands` | `b38e1884e4b775ce594c4b859ba0afe40719b5fc` | 19.3 h |

### agent/daniel-spelling (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-spelling/spellcheck` | `45477faad74d8058e6c5f1e440c3da22f5ffa886` | 23.2 h |

### agent/daniel-symbol-font (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-symbol-font/compiler` | `38fcbeee45b9ea26bbb1f8eda3090087a50967a6` | 22.2 h |

### agent/daniel-tables (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/daniel-tables/table-layout` | `a4c65eec703ae8dfe70777f1504932288a010537` | 23.2 h |

### agent/fable-hfill-corpus-merge (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/fable-hfill-corpus-merge/compiler` | `4d46f7a40702af2f8d9ae7e6107a7e430bf8e3d1` | 21.1 h |

### agent/kabir-claude (65)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/kabir-claude/amsmath-envs-r1` | `6e105864217c6ac180f0b4dfc67a04f43e3cd58b` | 12.2 h |
| `agent/kabir-claude/amsmath-inline` | `9cc8419fe6e69f78b13b9092f893e6dae18b0af5` | 9.2 h |
| `agent/kabir-claude/bibtex` | `08baeac89ac6fb29792f456f9cbcead34d1adc28` | 11.8 h |
| `agent/kabir-claude/bibtex-r2` | `315714788aa1a7d23d06324dcbe511d648598d3b` | 10.6 h |
| `agent/kabir-claude/bibtex-r3` | `5b94f8a812c5f3f1939598bdbe771185a9e57302` | 10.0 h |
| `agent/kabir-claude/class-geometry` | `2db2ffc8c1b78b35ec95c2b2b2648523add34902` | 12.3 h |
| `agent/kabir-claude/compiler-perf-r2` | `c2a61fb96c669c918c9aa077b2b25fd50722c9df` | 10.5 h |
| `agent/kabir-claude/compiler-tex-expansion` | `1634db848d11294b482f6e3a58b870aec01837d4` | 9.2 h |
| `agent/kabir-claude/compiler-tex-expansion-2` | `a2bff12ba84effb4a3681e9a1f8e50adc4734b17` | 7.2 h |
| `agent/kabir-claude/conversion-provider-r1` | `8e24d43255e24a2ca8f370bf64be81eaafa3e3fb` | 11.9 h |
| `agent/kabir-claude/corpus-refresh-crash` | `716489c3b2c62d3fe315e6b81f35a2c73be6ae84` | 8.2 h |
| `agent/kabir-claude/deterministic-font-warning-r` | `9a3cc2d09a3c25559bcb783a4f8fb937f2844b52` | 9.1 h |
| `agent/kabir-claude/display-math-placement` | `192affeda4c14ad2e74837d9058946139eec540b` | 5.8 h |
| `agent/kabir-claude/display-too-wide` | `6b9f7f70a9767919eac7840319a63c25c2aa6a5a` | 5.0 h |
| `agent/kabir-claude/emergencystretch` | `ab2a4e7e21de6e56c22f832eb145970378e96a7c` | 6.2 h |
| `agent/kabir-claude/encodings-accents` | `78d60b59a5f773d0fdfc541ee37d7addef65eb02` | 12.1 h |
| `agent/kabir-claude/equation-numbering` | `7cec0c3f52700e4cff4b85735012f87c0f42aec4` | 5.4 h |
| `agent/kabir-claude/font-families` | `d736c088c25c46b63461086ac35f8b26b2c9c335` | 6.2 h |
| `agent/kabir-claude/font-families-compiler` | `496027b35a7607e9f611a044173123c43ca5f257` | 6.4 h |
| `agent/kabir-claude/footnotes-render` | `d7ddf641325d5a09b052ad92681476be71ddfe52` | 6.2 h |
| `agent/kabir-claude/graphics-floats-r3` | `3937161ee9c55253efb48c087e6d893867ae1131` | 9.8 h |
| `agent/kabir-claude/hw-residuals-2` | `e5ca5f36ac80204f181d7dc85d0044fc79d8411d` | 3.5 h |
| `agent/kabir-claude/hw-text-widths` | `18f1348a3569536860e7919c5e09885f77d8a5b5` | 8.3 h |
| `agent/kabir-claude/hw2-math-final-r1` | `a64191bed4daa743b853e9e96ee8ed68f3cbfc76` | 12.2 h |
| `agent/kabir-claude/hyperref-pdf` | `cc261f6d18d25f1c87e876d94e320750ef600d3f` | 6.8 h |
| `agent/kabir-claude/hyphenation-r1` | `1c3ff974971d8cd4e33ec6aac2e7bb2851adaa8f` | 12.0 h |
| `agent/kabir-claude/inline-graphics-compiler` | `5e425dcfe56363b63bed0f66246d81f3eb7916e7` | 4.4 h |
| `agent/kabir-claude/list-structure-compiler` | `a86fad2e8068458002d34e786b25e8f26c53b5fd` | 5.7 h |
| `agent/kabir-claude/maketitle-layout-r` | `7185035e29a3423fbcc703d4aa70c541f21dd67f` | 7.5 h |
| `agent/kabir-claude/math-amssymb-braces` | `bcbc5ac9c6178ddeca1ed57581021698334cd162` | 7.3 h |
| `agent/kabir-claude/math-font-metrics` | `db11ac4b6141348234c8e350dc89fa520cb1fa17` | 3.3 h |
| `agent/kabir-claude/math-font-metrics-compiler` | `d3ee5d883de6094c5d741030a5a6aebc19e2b8dc` | 3.2 h |
| `agent/kabir-claude/math-glue-shrink` | `1dc371bc5a7e371130d8a570c9b76dc5d4ff6b4e` | 5.6 h |
| `agent/kabir-claude/math-glyph-spans` | `093d9c716a4a300ab72360b8cf298b9c1d138f82` | 4.3 h |
| `agent/kabir-claude/math-glyph-spans-pipeline` | `707e0e33e7aff70430387a0a1bc6b038340817dc` | 4.1 h |
| `agent/kabir-claude/math-nested-grids-compiler` | `59e50421e270b502bc5dc14269ac81b2ea0a4b8d` | 7.4 h |
| `agent/kabir-claude/mathatom-width-pub` | `343e42bf050c5972b72830bd081197b4b301740e` | 10.4 h |
| `agent/kabir-claude/microtype` | `545cd9fb9706ef1abc1cfdcfcba93f6926dba4d4` | 12.2 h |
| `agent/kabir-claude/microtype-paragraph-layout` | `c0304dc372a7273fd4204bd6ec952101a21b296f` | 7.5 h |
| `agent/kabir-claude/model-usage-policy` | `e13005e37590814ffddf6ef0d1725c4768e45d1e` | 13.2 h |
| `agent/kabir-claude/multicol` | `ab9fe6a7df660c75e49be9cee6b2f76d664b4e08` | 4.4 h |
| `agent/kabir-claude/multicol-compiler` | `ff0345881e2a4633c8dd44a08d27570ff1a1bfef` | 4.2 h |
| `agent/kabir-claude/page-builder` | `341353b6f177c9775566b5fb6a796320d98fda4d` | 12.0 h |
| `agent/kabir-claude/pdf-image-xobjects` | `210a8e55cb445fdb138ebad10f428b2f54d4b1ee` | 8.2 h |
| `agent/kabir-claude/preview-project-root` | `2f3e804137f95ded4a50b24d74751d64cc203246` | 8.4 h |
| `agent/kabir-claude/render-pipeline-class-geometry` | `29aad13196cecc23b1fc81e95b96b9678b57b3f0` | 10.1 h |
| `agent/kabir-claude/render-pipeline-class-geometry-2r` | `6c858cd503def02ba0fac99f59405f70034829f9` | 9.2 h |
| `agent/kabir-claude/rp-hyphen-align` | `77cbabb66304fb33a141ac48c19218750c15e806` | 11.8 h |
| `agent/kabir-claude/siunitx` | `c93e58ca6baaac1b586ac62eef719485415b3b75` | 4.0 h |
| `agent/kabir-claude/siunitx-pipeline` | `dc90a363b3c6520b48df1374cdd4578c2f4e1639` | 4.0 h |
| `agent/kabir-claude/supported-latex-r2` | `dbd54804436dd82402cb25be53c9d6acd85f683f` | 10.5 h |
| `agent/kabir-claude/tabular-array-compiler` | `05f13769316373d5ba1b0f6ef922032b5b270361` | 7.5 h |
| `agent/kabir-claude/tabular-fidelity` | `d8246433c93b2eaae8cd44fc5d9f3394acf1e805` | 7.9 h |
| `agent/kabir-claude/tex-boxes` | `4afd56f9f744dc0ed8ea4f48181a6295300400ce` | 11.8 h |
| `agent/kabir-claude/tex-boxes-r2` | `e68100a85d28e315a4a475647c3bde1e7ccefd4c` | 11.2 h |
| `agent/kabir-claude/tex-expansion` | `a087d143dcb43d8ca7effd88aa468ca1490f6f11` | 12.2 h |
| `agent/kabir-claude/tex-expansion-fable-r2` | `7dae933d3b3e761a77d3a1e6cf8df33448465c0e` | 11.5 h |
| `agent/kabir-claude/tex-expansion-r2-opus` | `e476dbab43b4f2d4a6b65e42645ef9f83d50fe49` | 10.2 h |
| `agent/kabir-claude/text-logos-rule-symbols` | `ad2d4178cbcc9997cdf3836e120a677b873ce3a7` | 7.3 h |
| `agent/kabir-claude/tikz-min-r1` | `18d1f0783d7f98823352258cada17d1bea3d2ead` | 11.9 h |
| `agent/kabir-claude/toc-followups` | `d7995c8436fa0a7aabd40703075a06a3da6ae4e4` | 6.1 h |
| `agent/kabir-claude/toc-layout-fidelity` | `f99a920750cf2488fedee69f4288eea77a029938` | 6.7 h |
| `agent/kabir-claude/xcolor-pipeline` | `ff5f2c9ce7e95c8ab0231eb3326b453488ea5a46` | 4.8 h |
| `agent/kabir-claude/xcolor-support` | `dc79f2e77cd3da55db93c1ac5d5d6360d4a6f202` | 4.9 h |
| `agent/kabir-claude/xetex-compat` | `a01b854d32635972abe4500f9d6e46a83a5ac573` | 12.2 h |

### agent/mac-a11y-panels (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-a11y-panels/a11y-panels` | `318d48ae220160497586f822d5f10a5dbb0efaab` | 28.4 h |

### agent/mac-accessibility (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-accessibility/help-and-rotor` | `dfe8f09e6e6d82ad7b7de092647d7c7db3b13b2e` | 31.7 h |

### agent/mac-admission-correlation (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-admission-correlation/compile-id` | `82376ac9b028e3895fb98b7739703859cbd2eddb` | 26.9 h |

### agent/mac-admission-groups (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-admission-groups/groups-and-gate` | `e03d14338d21e756f2be0a94b587bf0c8b094b57` | 26.1 h |

### agent/mac-ai-review (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ai-review/assistant-context` | `327239982e66291bb88c755f2e5fe8032038da04` | 33.1 h |
| `agent/mac-ai-review/bundled` | `0499a3b3739c08517ef674dc640847e898935927` | 31.4 h |

### agent/mac-ai-review-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ai-review-2/recovery` | `be4e203d8b551dad76a16bf6a9aa75159221d138` | 25.6 h |

### agent/mac-assistant-context (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-assistant-context/relocate-edits` | `d17952675c70c0931ea083890208606570d861ed` | 20.3 h |

### agent/mac-assistant-recovery (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-assistant-recovery/assistant-recovery` | `34d73b4f968d36ee4a2044ff19cefbf71d42385b` | 28.4 h |

### agent/mac-bibliography (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-bibliography/bibtex` | `0f7c23aeef5690b4d70927b74f02963dc83e6ddc` | 36.0 h |

### agent/mac-bibliography-kinds (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-bibliography-kinds/bibkinds` | `50810dcca6babef4781f6ab145b760f9787491a1` | 28.4 h |

### agent/mac-bridge-recovery (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-bridge-recovery/relaunch` | `faca5a4a6e27a81c252fac7f02cbb3abce5bc330` | 32.2 h |

### agent/mac-capture-acceptance (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-capture-acceptance/capture-acceptance` | `6fbfc6a39179ca05f7ec87d6bf8ad20d320d2758` | 28.5 h |

### agent/mac-capture-fluid (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-capture-fluid/flow` | `0bb8f6a8371555acee3e3f8de340509fe7c30386` | 3.8 h |

### agent/mac-capture-list-handoff (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-capture-list-handoff/capture-list` | `66f7b53bb31d3814a88a24c6ee29a88ad276bd69` | 27.9 h |

### agent/mac-ci-release (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ci-release/workflows` | `3349610b420f6e9a7a6309671951160b7edb4b85` | 14.6 h |

### agent/mac-citation-rename (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-citation-rename/citation-rename` | `1c87558fa03f42c61bb9869dc602c889b63c1fe8` | 27.9 h |

### agent/mac-claude-a (16)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-claude-a/accessibility` | `d585d0b7d3609234d6784ebe3a81cf04ccb0870f` | 36.2 h |
| `agent/mac-claude-a/app-bundle` | `ed1b20110ce7329e710b07a26bc2714428af37e8` | 37.3 h |
| `agent/mac-claude-a/bridge-integration` | `b18ed2f8d311ee92820e99d042f8966bbcfc6b50` | 35.9 h |
| `agent/mac-claude-a/caret-sync` | `b4cd9f4b85a8f21285f8423a97c440560a38b1d2` | 37.8 h |
| `agent/mac-claude-a/commit-identity` | `d6858792737c0980cccaec5db835075e94a1ebc9` | 38.2 h |
| `agent/mac-claude-a/completion` | `7df6fda0fe6b215757e278934f93f5b515064643` | 36.7 h |
| `agent/mac-claude-a/demo-sample` | `633563b1fa58bab22bb2a71f9fb2215bbcc9e820` | 37.5 h |
| `agent/mac-claude-a/editor-diagnostics` | `72a892e1c3811b21dce0f256da0a62d0fcd2389a` | 37.6 h |
| `agent/mac-claude-a/layout-capabilities` | `4234127fafacfbf5836539823e5c214ad6990616` | 35.4 h |
| `agent/mac-claude-a/layout-capabilities-3` | `be7ca7b25ac217323bc616b36d4c6b707e10550c` | 34.9 h |
| `agent/mac-claude-a/nearby-listener` | `ab835e0cf5d805c9b33191aa1128c1141d06b9a6` | 36.5 h |
| `agent/mac-claude-a/packaging` | `d7fb4fa49d37622b1b9b24b4a25759f13c78f4ef` | 35.6 h |
| `agent/mac-claude-a/paragraph-layout-handoff` | `b3238897e32f951fb56ecec34f7fc61b7d99684e` | 25.4 h |
| `agent/mac-claude-a/pdf-export` | `cc48e2967d7f58e8efd05790777043b6f23de3eb` | 37.8 h |
| `agent/mac-claude-a/proposal-preview` | `5e48334fc8a5f007fd310c5fb1bad03986b47b08` | 36.3 h |
| `agent/mac-claude-a/typing-bench-2` | `c5a8aca6f9bf50886e7fafb35cd0d3f9c1b52e9d` | 31.7 h |

### agent/mac-command-table (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-command-table/drift` | `22f28e9f9d012d4d8e9075dd27cf53c826bcd4f4` | 11.6 h |

### agent/mac-completion (4)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-completion/live-helper` | `4e71e8cc562cdd07a7325bb06e84104fadc93d62` | 32.9 h |
| `agent/mac-completion/revision-bound` | `8ff12420e9936ddd25a1a0df68e8bdbb8c226a8a` | 33.3 h |
| `agent/mac-completion/snippets` | `1a0ffbb730ffa804c8d5d1b5c03269808eb07fe6` | 31.4 h |
| `agent/mac-completion/vocabulary` | `3b80eabf3317e81055bac164ac7fea9e050b391a` | 32.1 h |

### agent/mac-completion-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-completion-2/latency` | `62bc6c93a2af1b2269b9aa88ae80f0897f751735` | 25.4 h |

### agent/mac-completion-sync (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-completion-sync/tests` | `0426d10fa8804e5b6ed357d5368521f694d77917` | 20.7 h |

### agent/mac-completion-sync-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-completion-sync-2/inventory` | `cdf9bdfdd625027ee9f939bdfc48dccada53c5f7` | 5.6 h |

### agent/mac-contract-review (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-contract-review/edit-ledger` | `8fbef4dad0b40f3b046026562a62db5eebdd32dd` | 33.6 h |
| `agent/mac-contract-review/lock-release` | `239ab97f7ee6498f161ec07efced59f8b9d57f39` | 36.1 h |

### agent/mac-core-review (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-core-review/pipeline` | `2a0662155bad16d81dfb2e96ea26b9ff98091a1b` | 27.2 h |

### agent/mac-deslop (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-deslop/remove-assistant` | `5ad6f8b1d7214e8714db6494e5785229da683a52` | 13.9 h |

### agent/mac-diagnostics-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-diagnostics-2/partial-output` | `3f722dac0c70b118b03827f23a3d988a66c40029` | 25.8 h |

### agent/mac-diagnostics-3 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-diagnostics-3/a11y-grouping` | `7f4c05fbe11c86518d61edf3424279634640e78a` | 23.7 h |

### agent/mac-display-delta-impl (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-display-delta-impl/consumer` | `65f06169f95b83f2f68c5878753f4a05d8567875` | 23.7 h |

### agent/mac-document-files (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-document-files/reload` | `de3a23923fafb7b6e33bebd03d2b9dca5ef34a1f` | 32.1 h |
| `agent/mac-document-files/rooted-helper` | `ab7286ef604c1a5066389ed336686b6eb786853e` | 33.4 h |

### agent/mac-document-files-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-document-files-2/dirty-preserve` | `7077d2678eaed9a5a717d9aaa077b4bb67785d60` | 25.6 h |

### agent/mac-document-style (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-document-style/style-model` | `bfc980d88002e125048805c5b9bdb1a29e9d4216` | 36.2 h |

### agent/mac-ec-bundle (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ec-bundle/tfms` | `b702a401c7ce41189008e20971f3f730789edf0d` | 7.5 h |

### agent/mac-editor-a11y-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-editor-a11y-2/large-doc` | `3635c3bfd8805f32ef2c7e8eda9816ddd0b3754f` | 25.7 h |

### agent/mac-editor-a11y-3 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-editor-a11y-3/rotor-motion` | `c41707087008fc7703cf3dec34152af92d4156de` | 23.6 h |

### agent/mac-editor-accessibility (3)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-editor-accessibility/braces` | `ea7f1c61ec9664ab30c08a1da1ccda2ff65b758a` | 31.2 h |
| `agent/mac-editor-accessibility/ime` | `f5455198c3dea584d092b9c429cc25eb140594bb` | 32.0 h |
| `agent/mac-editor-accessibility/responsive` | `1643f86682da7ff68d0465ffe7f546b2be2ff0a3` | 33.0 h |

### agent/mac-editor-diagnostics (3)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-editor-diagnostics/exact-marks` | `ef4ea4c9346b7cb116e473f97693ac517ea6ab88` | 33.6 h |
| `agent/mac-editor-diagnostics/explanations` | `f252f927bd9e43ba644a9a9afafbf9f1ba6782c6` | 32.0 h |
| `agent/mac-editor-diagnostics/quick-fix` | `95da847e3ab96e7f114ad2aef264072bf4e6ac92` | 31.4 h |

### agent/mac-export-ux (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-export-ux/export-ux` | `88857be0e0240b8174a556af62e2307dafbf0b6e` | 28.5 h |

### agent/mac-grok-assistant (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-grok-assistant/ask` | `ecbe55f80eb09f1e632f72b75aae4c701115428c` | 19.9 h |

### agent/mac-grok-demo (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-grok-demo/mode` | `17ce6dd790b35ad97f7a74a4ac8f10fee64982cf` | 22.5 h |

### agent/mac-grok-live (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-grok-live/wiring` | `a7193ae5e450655c123a4a2ee9697752e5eac0e7` | 22.7 h |

### agent/mac-grok-polish (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-grok-polish/fast-default` | `f87cf5b4fff9328578d54efaee5301e8783cc245` | 21.0 h |

### agent/mac-helper-display (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-helper-display/route` | `b53ffada8bd491aa84676f7982013fea1668c05d` | 27.6 h |

### agent/mac-historical-preview (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-historical-preview/consumer` | `12faf78820aa01b9d5084fd96c244d308f6dc7be` | 31.8 h |
| `agent/mac-historical-preview/hybrid` | `90af4c1271ed0eee66814a2a2e09b6f5ce6a10c2` | 31.2 h |

### agent/mac-history (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-history/panel` | `1e3ba7880404808cc6b956c9be113b2e8fffa273` | 30.8 h |

### agent/mac-hw2-1 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-hw2-1/items-displays` | `9d6e0a77fd7e300885c78f9f13b156e32c4c54ce` | 13.7 h |

### agent/mac-hyphenation (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-hyphenation/unify` | `8e853098f67c615a39d6698f8ea5fcbf96fb8404` | 10.6 h |

### agent/mac-images (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-images/v2-consumer` | `10df1a64f5ca3ac8bfc2dcf928f47b76a5ec98db` | 9.0 h |

### agent/mac-ime-composition (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ime-composition/ime` | `48a2ceaaa83296e975953bd36ddf24b03692611d` | 28.5 h |

### agent/mac-intellisense-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-intellisense-2/depth` | `f7f9195099da7e9c088da59ff7b405c6b4ab5b7f` | 13.7 h |

### agent/mac-ios-app (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ios-app/acceptance-slice` | `0f9ed794b3a222f2ec7bd52231cc194399a8b299` | 22.9 h |

### agent/mac-ios-app-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ios-app-2/finish` | `237de3ff9f04f7dee457057a2af9f4354e22e99d` | 20.5 h |

### agent/mac-large-document (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-large-document/large-document` | `59e81012f51cf4d8b797be5d4c24f7e0e21575ba` | 27.9 h |

### agent/mac-mathbb-font (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-mathbb-font/newcm-bb` | `e84bf3ae42bf1432c82bfd60bd21d54c95661781` | 14.6 h |

### agent/mac-multifile (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-multifile/project-documents` | `13769b8332e451d98fa96349eb783e5c7e907dac` | 32.1 h |
| `agent/mac-multifile/transitive` | `a442a952c8a6df0dcef89c4695606340dd7e897c` | 31.4 h |

### agent/mac-navigation (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-navigation/exact` | `990c8ff33cd45d842e317b8dd59d459cc33ba50b` | 33.2 h |
| `agent/mac-navigation/helper-navigate` | `80b883221f1c5c5265331fd45d1794635ba6da5c` | 32.1 h |

### agent/mac-navigation-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-navigation-2/multifile-nav` | `58757e0d904853e0be04f240cffb49986109b32f` | 25.6 h |

### agent/mac-navigation-3 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-navigation-3/unopened-math` | `2b18cfed9a53ad1fc0e90acc5ac4d32b448b0a1d` | 23.5 h |

### agent/mac-nearby-client (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-nearby-client/reconnect` | `0f51ef8f1bcb4b8808c42922acf0eab1056332ee` | 32.6 h |

### agent/mac-nearby-client-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-nearby-client-2/fixtures` | `96f530a91b5947ed235e4d7bd901fe36dd4799bd` | 25.8 h |

### agent/mac-nearby-errors (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-nearby-errors/nearby-errors` | `166f70c4c0505ecd1d185217b19e51c92ad1859a` | 27.9 h |

### agent/mac-nearby-transport (3)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-nearby-transport/bounded` | `b3baa25344652ede037775bbcc4bc409ec0dca5b` | 33.2 h |
| `agent/mac-nearby-transport/events` | `0d77baa36841d71203c95673cf0b39736af41903` | 32.8 h |
| `agent/mac-nearby-transport/generation-api` | `723f6e93307197cdf1580e10ff170c81db2a1ccb` | 31.2 h |

### agent/mac-nearby-transport-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-nearby-transport-2/reconnect` | `3bbac8fbef10c31ede1a0959546b532f5fa88455` | 25.6 h |

### agent/mac-nearby-transport-3 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-nearby-transport-3/lan-advertise` | `41a64f169d30bb4e5c5a9f07c18778a077f814d8` | 23.7 h |

### agent/mac-packaging (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-packaging/signing` | `af155e59f218cc3e88e19d15b1d042223d0ee298` | 31.1 h |

### agent/mac-packaging-tfm (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-packaging-tfm/texmf` | `4dba480952b942fec33e786ea8f4c2bf0e5ccbd6` | 28.2 h |

### agent/mac-pairing-ui (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-pairing-ui/events-consumer` | `f991625e97f87dbf4ca878d972984c02d5a96fc1` | 32.1 h |
| `agent/mac-pairing-ui/recovery` | `fbc3832660b7e5ed99524807c6cec9780aa8ace5` | 33.3 h |

### agent/mac-pairing-ui-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-pairing-ui-2/pairing-gaps` | `bf95d747c49d45795252302c310f2dbd1c0e794e` | 25.8 h |

### agent/mac-pairing-ui-3 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-pairing-ui-3/qr-permissions` | `63241458f8a6cc3023f1f9ffd3c2bad54c219f3d` | 23.6 h |

### agent/mac-paragraph-layout (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-paragraph-layout/linebreak` | `0a1ba87df9c99e72645cb3b9269556c67b51eb45` | 35.0 h |

### agent/mac-paste-recovery (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-paste-recovery/paste` | `d6982d377868eaaac244f5caba0cf81dd10d9c8d` | 28.5 h |

### agent/mac-pdf (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-pdf/pdf-output` | `4bd8c2e79f66c161b7fb6438f3262f8d980eb8c9` | 35.5 h |

### agent/mac-pdf-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-pdf-2/export-fidelity` | `9689384e7e8c8af115f27c4ba2e0c04227b08bd5` | 25.8 h |

### agent/mac-pdf-3 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-pdf-3/searchable-text` | `03262d65133902f85c8f4e6eacb387630edb24d8` | 23.4 h |

### agent/mac-preferences (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-preferences/editor` | `911f39301363951e57f3a7dc388508a49d5cf3e1` | 31.1 h |

### agent/mac-preview-anchoring (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-preview-anchoring/anchoring` | `9c1500bce461e544104d0098737d4cd5c2d4993a` | 28.4 h |

### agent/mac-preview-cache (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-preview-cache/invalidation` | `e4d3d8bffc48c6f5487421fbfb2c0a56b0bc2edd` | 33.5 h |

### agent/mac-preview-latency (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-preview-latency/profile` | `5918282d79887c3b5d2c09abbc2d38a06e34e12f` | 23.3 h |

### agent/mac-preview-v2 (2)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-preview-v2/exact-glyphs` | `aaa4c4bac8004f04afe69550df4250a461c8a766` | 32.8 h |
| `agent/mac-preview-v2/live` | `9dbdb01f8d7e64268c9be46cc88f95adf211f2ee` | 31.6 h |

### agent/mac-readme (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-readme/rewrite` | `d98aaf386bac41e640dddf3715bbfc789c3b24f5` | 14.0 h |

### agent/mac-realworld-corpus (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-realworld-corpus/corpus` | `74d679714944ce1de8f017543fc9adccb7cd6a69` | 27.4 h |

### agent/mac-reference-corpus (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-reference-corpus/opus-fonts-takeover` | `9ba9851cefb3ba263ffe1d25f42cb7fdd27371c4` | 23.9 h |

### agent/mac-reland-1 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-reland-1/editor-patches` | `fc36d6da64b963cabf6327e014cbf145d243ca19` | 11.9 h |

### agent/mac-render-pipeline (19)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-render-pipeline/delim-oracle` | `de0f3a80ad261a41b1dd4888fe0ccfa12756d33b` | 21.4 h |
| `agent/mac-render-pipeline/display-math` | `9fe8b8234788ac2e40a826ecdb91a861925227f2` | 22.4 h |
| `agent/mac-render-pipeline/hw1-math-2` | `51c2b9b1adc394bce37cec5319f5a70a5bb93666` | 22.6 h |
| `agent/mac-render-pipeline/hw1-math-3` | `c0f0a2a00a77ccc912143b881db283651263420a` | 22.1 h |
| `agent/mac-render-pipeline/hw1-math-4` | `2e9b52739dc0fa10af24f537c6a20a377271bcef` | 21.0 h |
| `agent/mac-render-pipeline/hw1-math-5` | `eba68dadbb30755b8eba1620303258834118279e` | 20.4 h |
| `agent/mac-render-pipeline/hw1-math-6` | `41cb33dbada955dbb94b4117718de2298ef3c8ef` | 19.7 h |
| `agent/mac-render-pipeline/hw1-math-7` | `13697f9bb3dc6150e936769003886b965b06f934` | 19.4 h |
| `agent/mac-render-pipeline/hw1-math-8` | `f5796a62fc30824bdcc967792f76ce254c9a4596` | 19.0 h |
| `agent/mac-render-pipeline/hw1-math-9` | `9fea27d8cbf7163e3d02996016210d45c6b1b155` | 17.8 h |
| `agent/mac-render-pipeline/math-symbols` | `4b9c1df53354205cdc7ad3515e33e7519cda6c55` | 23.5 h |
| `agent/mac-render-pipeline/otf-fallback` | `3c524d9e6cfbdd701a484b1ab1e952940ba483b2` | 20.6 h |
| `agent/mac-render-pipeline/repin-a9952df3` | `a63086310e724d89dad092d1534d7e5fc163759f` | 8.5 h |
| `agent/mac-render-pipeline/repin-ad720575` | `3f7fd9b127478d4ac1043dbdf32dce26a753d017` | 8.1 h |
| `agent/mac-render-pipeline/repin-c583d6d4` | `5a72a16f56b63a255251f5971864cbe01b2370d0` | 9.6 h |
| `agent/mac-render-pipeline/repin-d416472a` | `aea1a8957c2aa89959a3385b2d4e204292e2a147` | 12.2 h |
| `agent/mac-render-pipeline/repin-dbf6ec78` | `e1dddc47c1242a9b3614510ead205e61ee812382` | 11.1 h |
| `agent/mac-render-pipeline/text-gaps` | `732175268ab6eeb5c2d0d7d8bb212b2b60a1393c` | 25.4 h |
| `agent/mac-render-pipeline/unified` | `9aaec57a019c6a0073419eeb3ec90f922f5b367c` | 28.2 h |

### agent/mac-search (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-search/panel` | `afa30c2f946b3a1747c6e54e138b19ebbf475c45` | 28.6 h |

### agent/mac-search-reconcile (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-search-reconcile/gh39` | `0fcac4855b84e4f0692b5fde09e4d6522cf99885` | 26.7 h |

### agent/mac-suite-repair (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-suite-repair/full-suite` | `fbf2158dcc36d533d5ca15e63f23cb760ec1cb9a` | 21.6 h |

### agent/mac-suite-repair-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-suite-repair-2/compiler-drift` | `11ef11d754814369ea314a71f510ee61a7b1690c` | 14.1 h |

### agent/mac-syntax-highlight (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-syntax-highlight/editor` | `e066abe157f89fa9fdff2d069b1784a0605a35d2` | 22.7 h |

### agent/mac-ui-redesign (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-ui-redesign/shell` | `d41d10674448188184c9bf4c5bb92f922067866f` | 22.9 h |

### agent/mac-user-docs (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-user-docs/guides` | `288d72e887642906611f7c3c0f1831c1e06bee4c` | 14.7 h |

### agent/mac-v2-conformance (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-v2-conformance/contract` | `f5c19cb492afdb23158b2a42fc1b79a4c38eae56` | 27.1 h |

### agent/mac-validation (3)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-validation/mac-live` | `fb901c2c431dae5cf0f55ea62e76557829ca9269` | 33.0 h |
| `agent/mac-validation/mac-live-2` | `c99398c3b4d468bf610af7deb8ded4b70dd0ce7f` | 32.1 h |
| `agent/mac-validation/mac-live-3` | `5a13599c346aaa184b00f8da7a79fcefcc0ca19c` | 30.8 h |

### agent/mac-validation-4 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-validation-4/mac-live-4` | `a4da7bdbadc59311b3fed7c4956de4e37ed0355b` | 25.3 h |

### agent/mac-vector-graphics (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-vector-graphics/primitives` | `6103087b533220b2308bb06a41ba0585822ca0cb` | 36.2 h |

### agent/mac-vector-graphics-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-vector-graphics-2/json-depth` | `cbe1a1139d3ad893953a1a42df0d011a45c4e1e5` | 23.6 h |

### agent/mac-visual-oracle-2 (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-visual-oracle-2/corpus-rank` | `864e4d394f1f8cbb63433899d9b2f1cfcdb329d5` | 25.8 h |

### agent/mac-vocab-json (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-vocab-json/completion` | `0029a819d84fbf0e4879d055d7e68899742e7d95` | 9.6 h |

### agent/mac-zoom (1)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/mac-zoom/preview-editor` | `4480d00208ffbc7596b8af91adb22d48f899b7f8` | 22.4 h |

### agent/orchestrator-astra (3)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `agent/orchestrator-astra/authority-and-dispatch` | `fd717b576cfa0b63bb46e36d6800f1768ea2e89e` | 35.6 h |
| `agent/orchestrator-astra/compiler-integration` | `ae3f4de96282e7ea2c60aa268ed5f0b09c664e06` | 36.1 h |
| `agent/orchestrator-astra/product-integration` | `c89ca869781bd80128ba322b924feeebc694b4b4` | 36.3 h |

### integration (11)

| branch | tip SHA | tip age at survey |
|---|---|---|
| `integration/2026-09-13a` | `5da4b4649f01d125b18648e659b4c3335f644d26` | 12.6 h |
| `integration/2026-09-13b` | `7371ca4081809e98fcd7fd5e98ca98fbdca0e1b6` | 11.4 h |
| `integration/2026-09-13c` | `3b2703aa50e4204a634aac21caef396b82a7d3a0` | 10.3 h |
| `integration/2026-09-13d` | `c583d6d4e4a827a6721d0181735014a429ceceda` | 10.1 h |
| `integration/2026-09-13e` | `fe864d909233947feecd9b7efac4d35bb19db853` | 9.4 h |
| `integration/2026-09-13f` | `a9952df36a4111f6d4003509c3c7fc6a5f465384` | 8.8 h |
| `integration/2026-09-13g` | `ad720575c217994254c32745ba799ec525e2593b` | 8.4 h |
| `integration/2026-09-13h` | `1d4604d422896adf8076ebf4afeb15718d819045` | 7.9 h |
| `integration/2026-09-13i` | `85ec30c70ca241302c56c2d19ff6640b5236c9b7` | 7.0 h |
| `integration/2026-09-13j` | `354ed74fbc882dffa5a937039ea7caa2a839a911` | 6.0 h |
| `integration/2026-09-13k` | `eee6fd55393ae24f2b324211bc3b38c7cb69f8d0` | 5.0 h |

