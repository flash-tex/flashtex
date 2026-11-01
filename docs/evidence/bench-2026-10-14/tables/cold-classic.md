| document | n | flashtex-v3 build p50 / p95 | pdflatex one pass p50 / p95 | latexmk p50 / p95 (passes) | v3 vs one pass | v3 vs latexmk | v3 output vs latexmk | engine one pass p50 / p95 | engine latexmk p50 / p95 | engine-mk output |
|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---|
| hw1 | 7 | 791 ms / 947 ms | 657 ms / 830 ms | 1.60 s / 1.78 s (2) | 0.83× | 2.03× | byte-identical but the trailer /ID | 208 ms / 246 ms | 663 ms / 745 ms | byte-identical |
| hw2 | 7 | 641 ms / 781 ms | 539 ms / 599 ms | 1.20 s / 1.29 s (2) | 0.84× | 1.87× | byte-identical but the trailer /ID | 162 ms / 209 ms | 536 ms / 642 ms | byte-identical |
| plain-100 | 7 | 2.24 s / 2.59 s | 708 ms / 881 ms | 1.52 s / 2.19 s (2) | 0.32× | 0.68× | byte-identical but the trailer /ID | 324 ms / 485 ms | 886 ms / 1.13 s | byte-identical |
| plain-300 | 7 | 5.32 s / 5.67 s | 1.14 s / 1.31 s | 2.51 s / 2.92 s (2) | 0.21× | 0.47× | byte-identical but the trailer /ID | 750 ms / 914 ms | 1.75 s / 1.90 s | byte-identical |
| full-100 | 7 | 5.08 s / 5.79 s | 2.00 s / 2.42 s | 4.24 s / 4.64 s (2) | 0.39× | 0.83× | byte-identical but the trailer /ID | 1.06 s / 1.20 s | 3.68 s / 4.08 s | byte-identical |
| beamer-5 | 7 | 1.86 s / 2.16 s | 1.07 s / 1.18 s | 2.45 s / 3.43 s (2) | 0.57× | 1.32× | byte-identical but the trailer /ID | 600 ms / 665 ms | 2.26 s / 2.36 s | byte-identical |
| beamer-30 | 7 | 8.14 s / 8.92 s | 2.87 s / 3.49 s | 5.96 s / 7.66 s (2) | 0.35× | 0.73× | byte-identical but the trailer /ID | 2.34 s / 2.74 s | 7.25 s / 8.56 s | byte-identical |
| plain-1000 | 7 | 14.59 s / 16.84 s | 2.40 s / 2.54 s | 4.91 s / 5.60 s (2) | 0.16× | 0.34× | byte-identical but the trailer /ID | 1.81 s / 1.94 s | 4.09 s / 4.90 s | byte-identical |
| full-1000 | 7 | 52.98 s / 61.31 s | 12.10 s / 14.67 s | 37.92 s / 50.98 s (3) | 0.23× | 0.72× | byte-identical but the trailer /ID | 6.54 s / 8.20 s | 19.82 s / 23.07 s | byte-identical |
| infdesc | 7 | 118.59 s / 131.19 s † | 23.39 s / 28.06 s | 71.90 s / 83.99 s (3) | 0.20× | 0.61× | byte-identical but the trailer /ID | 19.39 s / 24.77 s | 63.46 s / 71.48 s | byte-identical |
| infdesc-x2 | 7 | 300.32 s / 346.41 s † | 56.44 s / 63.84 s | 162.82 s / 183.13 s (3) | 0.19× | 0.54× | byte-identical but the trailer /ID | 47.90 s / 66.34 s | 152.38 s / 177.15 s | byte-identical |

Ratios are the other engine's median over v3's: above 1× v3 is faster, below 1× slower.

Peak resident memory of the whole process tree, warm-up run (ps every 100 ms):

| document | v3 | pdflatex | latexmk | engine | engine-mk | v3 passes (--progress) |
|---|---:|---:|---:|---:|---:|---|
| hw1 | 204 MB | 47 MB | 68 MB | 48 MB | 73 MB | flashtex-v3: wrote v3-out.pdf (936 ms) |
| hw2 | 147 MB | 53 MB | 64 MB | - | 74 MB | flashtex-v3: wrote v3-out.pdf (689 ms) |
| plain-100 | 200 MB | 51 MB | 67 MB | 55 MB | 71 MB | flashtex-v3: wrote v3-out.pdf (1823 ms) |
| plain-300 | 305 MB | 51 MB | 67 MB | 55 MB | 71 MB | flashtex-v3: wrote v3-out.pdf (5706 ms) |
| full-100 | 244 MB | 54 MB | 70 MB | 60 MB | 76 MB | flashtex-v3: wrote v3-out.pdf (4893 ms) |
| beamer-5 | 170 MB | 55 MB | 72 MB | 62 MB | 78 MB | flashtex-v3: wrote v3-out.pdf (1680 ms) |
| beamer-30 | 202 MB | 56 MB | 72 MB | 62 MB | 78 MB | flashtex-v3: wrote v3-out.pdf (7178 ms) |
| plain-1000 | 672 MB | 51 MB | 68 MB | 109 MB | 71 MB | flashtex-v3: wrote v3-out.pdf (24368 ms) |
| full-1000 | 780 MB | 55 MB | 73 MB | 61 MB | 79 MB | flashtex-v3: wrote v3-out.pdf (53322 ms) |
| infdesc | 660 MB | 79 MB | 96 MB | 124 MB | 94 MB | flashtex-v3: wrote v3-out.pdf (116899 ms) |
| infdesc-x2 | 1158 MB | 82 MB | 99 MB | 127 MB | 98 MB | flashtex-v3: wrote v3-out.pdf (346323 ms) |

† flashtex-v3 exited 1 (an error reported, the PDF written) in {('infdesc', 'v3'): 7, ('infdesc-x2', 'v3'): 7} timed runs; the warm-up's PDF of the same build matched latexmk's, so the times count.
