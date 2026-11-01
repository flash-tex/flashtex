| side | gate | status | units | compiles | ok | bad | wrong (span) | aborts | skipped | unit-minutes |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| ref `707f886fa` | sound-a | **pass** | 88/88 | 8800 | 8800 | 0 |  | 0 | 0 | 55.4 |
| ref `707f886fa` | sound-budget | **pass** | 88/88 | 3520 | 3520 | 0 |  | 0 | 0 | 23.5 |
| ref `707f886fa` | sound-budget-d | **pass** | 89/89 | 1130 | 966 | 0 |  | 0 | 0 | 7.7 |
| ref `707f886fa` | sound-timed | **pass** | 86/86 | 1720 | 1720 | 0 |  | 0 | 0 | 10.3 |
| ref `707f886fa` | sound-vol | **pass** | 6/6 | 201 | 188 | 0 |  | 0 | 0 | 0.6 |
| ref `707f886fa` | sound-lookup | **pass** | 2/2 | 90 | 79 | 0 |  | 0 | 0 | 0.2 |
| ref `707f886fa` | sound-lines | **pass** | 262/262 | 2930 | 2832 | 0 |  | 0 | 29 | 18.9 |
| ref `707f886fa` | span | **pass** | 13/13 | 0 | 0 | 0 | 0 of 31.9 M glyphs | 0 | 0 | 12.0 |
| ref `707f886fa` | readers | **pass** | 1/1 | 0 | 0 | 0 |  | 0 | 0 | 2.0 |
| ref `707f886fa` | sound-c | **pass** | 89/89 | 1974 | 1974 | 0 |  | 0 | 0 | 17.8 |
| ref `707f886fa` | sound-d | **pass** | 89/89 | 1682 | 1456 | 0 |  | 0 | 0 | 10.6 |
| ref `707f886fa` | sound-first | **pass** | 88/88 | 1600 | 1304 | 0 |  | 0 | 0 | 14.5 |
| base `c5112f4a7` | sound-a | **pass** | 88/88 | 8800 | 8800 | 0 |  | 0 | 0 | 59.9 |
| base `c5112f4a7` | sound-budget | **pass** | 88/88 | 3520 | 3520 | 0 |  | 0 | 0 | 24.4 |
| base `c5112f4a7` | sound-budget-d | **pass** | 89/89 | 1130 | 966 | 0 |  | 0 | 0 | 9.2 |
| base `c5112f4a7` | sound-timed | **pass** | 86/86 | 1720 | 1720 | 0 |  | 0 | 0 | 11.4 |
| base `c5112f4a7` | sound-vol | **pass** | 6/6 | 201 | 188 | 0 |  | 0 | 0 | 0.7 |
| base `c5112f4a7` | sound-lookup | **pass** | 2/2 | 90 | 79 | 0 |  | 0 | 0 | 0.3 |
| base `c5112f4a7` | sound-lines | **pass** | 262/262 | 2930 | 2832 | 0 |  | 0 | 29 | 19.8 |
| base `c5112f4a7` | span | **pass** | 13/13 | 0 | 0 | 0 | 0 of 31.9 M glyphs | 0 | 0 | 13.0 |
| base `c5112f4a7` | readers | **pass** | 1/1 | 0 | 0 | 0 |  | 0 | 0 | 1.4 |
| base `c5112f4a7` | sound-c | **pass** | 89/89 | 1974 | 1974 | 0 |  | 0 | 0 | 20.7 |
| base `c5112f4a7` | sound-d | **pass** | 89/89 | 1682 | 1456 | 0 |  | 0 | 0 | 11.8 |
| base `c5112f4a7` | sound-first | **FAIL** | 88/88 | 0 | 0 | 0 |  | 88 | 0 | 0.1 |

- ref `707f886fa`: 5/5 shards, slowest 13.9 min, 45 shard-minutes
- base `c5112f4a7`: 5/5 shards, slowest 11.9 min, 44 shard-minutes

**Not ok:**

- base `sound-first|first|article-twocolumn`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|beamer-default`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|beamer-polish`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|beamer-visuals`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|full-100`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|hw1`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|input-bibliography`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|listings-manual`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min-bigdelim-12pt`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min-float-table`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min-huge-fences`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min-newblock`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min-sqrt-tall`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min-toc`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min2-fence-nested`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min2-itemize`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min2-pmatrix-3row`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min3-enumerate-only`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min3-subsubsection`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
- base `sound-first|first|min4-lig-hyphen`: error -- `interleave] [--host-args HOST_ARGS] /                     [--cold] [--toggle-files TOGGLE_FILES] [--allow-no-trials] /                     engine / soundness.py: error: unrecognized arguments: --first-open`
