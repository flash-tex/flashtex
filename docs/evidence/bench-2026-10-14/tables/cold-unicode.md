| document | n | flashtex-v3 build p50 / p95 | xelatex one pass p50 / p95 | latexmk -xelatex p50 / p95 (passes) | v3 vs one pass | v3 vs latexmk | v3 output vs latexmk |
|---|---:|---:|---:|---:|---:|---:|---|
| b01 | 9 | 608 ms / 811 ms | 954 ms / 1.10 s | 1.50 s / 1.68 s (2) | 1.57× | 2.47× | structurally equal (xetex-pdfparity) |
| b02 | 9 | 1.21 s / 1.28 s | 1.19 s / 1.41 s | 1.92 s / 2.10 s (2) | 0.99× | 1.59× | structurally equal (xetex-pdfparity) |
| b03 | 9 | 2.98 s / 3.12 s | 1.71 s / 2.02 s | 3.05 s / 3.32 s (2) | 0.57× | 1.02× | structurally equal (xetex-pdfparity) |
| b04 | 9 | 1.86 s / 2.11 s | 1.17 s / 1.35 s | 2.64 s / 2.78 s (3) | 0.63× | 1.42× | structurally equal (xetex-pdfparity) |

Ratios are the other engine's median over v3's: above 1× v3 is faster, below 1× slower.

Peak resident memory of the whole process tree, warm-up run (ps every 100 ms):

| document | v3 | xelatex | latexmk | v3 passes (--progress) |
|---|---:|---:|---:|---|
| b01 | 220 MB | 210 MB | 194 MB | flashtex-v3: wrote v3-out.pdf (380 ms) |
| b02 | 223 MB | 212 MB | 194 MB | flashtex-v3: wrote v3-out.pdf (1433 ms) |
| b03 | 226 MB | 212 MB | 198 MB | flashtex-v3: wrote v3-out.pdf (2662 ms) |
| b04 | 220 MB | 203 MB | 196 MB | flashtex-v3: wrote v3-out.pdf (1587 ms) |
