# Real-world corpus gate: main c40d2a6c vs agent/kabir-claude/hw-residuals-2 93d55735

`tools/real-world-corpus/run.py --no-oracle --no-pixels`: render status, page count and
diagnostic count are unchanged for all 9 fixtures.

`residuals.py . - <name>=<render.v2.json>=<reference.pdf> ...` per page (words with dx >0.1pt /
lines with median dy >0.1pt) against each fixture's committed reference PDF (hw1:
`reference-mactex2026.pdf`, hw2: `HW2-reference.pdf`, others: `reference.pdf`):

| doc | page | base dx | base dy | new dx | new dy |
|---|---|---|---|---|---|
| article-twocolumn | 1 | 0 | 0 | 0 | 0 |
| cv | 1 | 15 | 2 | 15 | 2 |
| input-bibliography | 1 | 0 | 0 | 0 | 0 |
| input-bibliography | 2 | 0 | 0 | 0 | 0 |
| lecture-notes | 1 | 32 | 9 | 32 | 9 |
| lecture-notes | 2 | 27 | 0 | 27 | 0 |
| letter | 1 | 0 | 0 | 0 | 0 |
| math-sheet | 1 | 2 | 2 | 2 | 2 |
| math-sheet | 2 | 68 | 24 | 68 | 24 |
| unicode-accents | 1 | 25 | 0 | 25 | 0 |
| hw1 | 1 | 4 | 25 | 4 | 0 |
| hw1 | 2 | 2 | 16 | 2 | 0 |
| hw1 | 3 | 12 | 9 | 2 | 0 |
| hw2 | 1 | 24 | 25 | 24 | 0 |
| hw2 | 2 | 48 | 19 | 38 | 0 |
| hw2 | 3 | 41 | 8 | 30 | 0 |

(HW2 p1 counts 24 against the committed `HW2-reference.pdf` and 11 against a fresh MacTeX 2026
build, `residuals-3-ec-em.md`; the comparison is relative.)
