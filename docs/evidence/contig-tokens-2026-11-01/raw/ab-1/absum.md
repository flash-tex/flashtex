## Cold compiles (perf stat, random order, medians)

| document | engine | n | instructions | Δ vs main | cycles | Δ vs main [95 % CI] | peak RSS | same PDF, log |
|---|---|---:|---:|---:|---:|---:|---:|---|
| 2501.06980v1 | main | 5 | 6.79 G |  | 4.23 G |  | 65 MB | yes |
| 2501.06980v1 | mainB | 5 | 6.79 G | -0.00 % | 4.10 G | -3.11 % [-9.65, +17.01] | 65 MB | yes |
| 2501.06980v1 | lazy | 5 | 6.74 G | -0.82 % | 3.82 G | -9.70 % [-15.81, +13.86] | 65 MB | yes |
| 2501.07014v3 | main | 5 | 10.35 G |  | 5.67 G |  | 63 MB | yes |
| 2501.07014v3 | mainB | 5 | 10.35 G | +0.00 % | 5.67 G | +0.06 % [-3.79, +3.21] | 63 MB | yes |
| 2501.07014v3 | lazy | 5 | 10.32 G | -0.29 % | 5.59 G | -1.39 % [-6.35, +2.82] | 63 MB | yes |
| 2501.07044v1 | main | 5 | 9.10 G |  | 4.89 G |  | 66 MB | yes |
| 2501.07044v1 | mainB | 5 | 9.10 G | -0.00 % | 4.90 G | +0.18 % [-3.44, +1.77] | 66 MB | yes |
| 2501.07044v1 | lazy | 5 | 9.06 G | -0.43 % | 4.87 G | -0.49 % [-3.14, +0.18] | 66 MB | yes |
| 2501.07084v3 | main | 5 | 5.52 G |  | 2.96 G |  | 65 MB | yes |
| 2501.07084v3 | mainB | 5 | 5.52 G | +0.00 % | 2.96 G | -0.01 % [-3.00, +5.53] | 65 MB | NO |
| 2501.07084v3 | lazy | 5 | 5.47 G | -0.91 % | 2.86 G | -3.33 % [-4.80, -0.69] | 65 MB | NO |
| 2501.07124v3 | main | 5 | 55.25 G |  | 32.16 G |  | 85 MB | yes |
| 2501.07124v3 | mainB | 5 | 55.25 G | -0.00 % | 31.64 G | -1.62 % [-4.31, -0.24] | 85 MB | yes |
| 2501.07124v3 | lazy | 5 | 55.13 G | -0.22 % | 32.06 G | -0.33 % [-3.95, +1.43] | 85 MB | yes |
| 2501.07198v2 | main | 5 | 6.92 G |  | 3.77 G |  | 61 MB | yes |
| 2501.07198v2 | mainB | 5 | 6.92 G | +0.00 % | 3.70 G | -1.88 % [-4.60, +4.04] | 61 MB | yes |
| 2501.07198v2 | lazy | 5 | 6.89 G | -0.44 % | 3.76 G | -0.12 % [-5.00, +5.90] | 61 MB | yes |
| 2501.07301v2 | main | 5 | 10.03 G |  | 5.48 G |  | 68 MB | yes |
| 2501.07301v2 | mainB | 5 | 10.03 G | -0.00 % | 5.56 G | +1.50 % [-5.87, +6.04] | 68 MB | yes |
| 2501.07301v2 | lazy | 5 | 9.92 G | -1.03 % | 5.29 G | -3.42 % [-10.44, -1.11] | 68 MB | yes |
| 2501.07372v2 | main | 5 | 4.26 G |  | 2.82 G |  | 61 MB | yes |
| 2501.07372v2 | mainB | 5 | 4.26 G | -0.01 % | 2.57 G | -8.85 % [-17.89, +15.02] | 61 MB | yes |
| 2501.07372v2 | lazy | 5 | 4.22 G | -0.93 % | 2.47 G | -12.30 % [-18.42, +16.73] | 61 MB | yes |
| 2501.07497v1 | main | 5 | 17.37 G |  | 10.64 G |  | 72 MB | yes |
| 2501.07497v1 | mainB | 5 | 17.37 G | -0.00 % | 10.64 G | -0.04 % [-8.20, +12.30] | 72 MB | yes |
| 2501.07497v1 | lazy | 5 | 17.15 G | -1.28 % | 9.40 G | -11.70 % [-18.38, +1.12] | 72 MB | yes |
| 2501.07623v3 | main | 5 | 3.73 G |  | 2.17 G |  | 61 MB | yes |
| 2501.07623v3 | mainB | 5 | 3.73 G | +0.00 % | 2.07 G | -4.27 % [-12.36, +5.86] | 62 MB | yes |
| 2501.07623v3 | lazy | 5 | 3.70 G | -0.72 % | 2.08 G | -3.98 % [-12.10, +1.35] | 62 MB | yes |
| 2501.07840v2 | main | 5 | 5.10 G |  | 3.36 G |  | 60 MB | yes |
| 2501.07840v2 | mainB | 5 | 5.10 G | +0.00 % | 3.41 G | +1.56 % [-11.90, +29.24] | 60 MB | yes |
| 2501.07840v2 | lazy | 5 | 5.07 G | -0.65 % | 3.57 G | +6.14 % [-12.47, +33.05] | 60 MB | yes |
| 2501.08157v1 | main | 5 | 12.31 G |  | 7.06 G |  | 69 MB | yes |
| 2501.08157v1 | mainB | 5 | 12.31 G | -0.00 % | 7.13 G | +0.95 % [-2.54, +8.91] | 69 MB | yes |
| 2501.08157v1 | lazy | 5 | 12.20 G | -0.91 % | 6.76 G | -4.24 % [-9.54, -1.95] | 69 MB | yes |
| 2501.09225v1 | main | 5 | 8.35 G |  | 4.53 G |  | 67 MB | yes |
| 2501.09225v1 | mainB | 5 | 8.35 G | +0.00 % | 4.49 G | -0.85 % [-8.46, +6.78] | 67 MB | yes |
| 2501.09225v1 | lazy | 5 | 8.28 G | -0.82 % | 4.35 G | -3.96 % [-11.34, +1.40] | 67 MB | yes |
| infdesc-x2-ts2 | main | 1 | 454.81 G |  | 280.65 G |  | 77 MB | yes |
| infdesc-x2-ts2 | mainB | 1 | 454.81 G | -0.00 % | 280.28 G | -0.13 % [-0.13, -0.13] | 76 MB | yes |
| infdesc-x2-ts2 | lazy | 1 | 452.80 G | -0.44 % | 248.85 G | -11.33 % [-11.33, -11.33] | 76 MB | yes |

The 13 papers, sums of medians:
- main: instructions 155.09 G (+0.00 %), cycles 89.74 G (+0.00 %)
- mainB: instructions 155.09 G (-0.00 %), cycles 88.85 G (-1.00 %)
- lazy: instructions 154.15 G (-0.60 %), cycles 86.88 G (-3.19 %)
- mainB: fewer cycles than main on 8 of 13 papers
- lazy: fewer cycles than main on 12 of 13 papers

Order effect (cycles of a run against its own median, all documents):
- lazy after another: n=58, mean +0.58 %
- lazy after itself: n=8, mean +0.99 %
- main after another: n=58, mean -0.14 %
- main after itself: n=8, mean -1.32 %
- mainB after another: n=60, mean +0.52 %
- mainB after itself: n=6, mean +4.76 %

## Host: keystrokes through the socket (checkpoints on), p50/p95 pooled over rounds; CPU is the host thread's CPU time, which CPU-quota throttling does not inflate, wall is shown but is not comparable on a throttled PC

| page | engine | rounds | keys | edited page CPU ms | request CPU ms | convergence test ms | restore ms | edited page wall ms | session instructions | Δ | session cycles | Δ |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 900 | main | 1 | 20 | 25.8 / 28.0 | 313.1 / 357.2 | 202.2 / 265.9 | 3.7 / 11.0 | 95.9 / 162.3 | 1634.18 G |  | 929.79 G |  |
| 900 | mainB | 2 | 40 | 27.7 / 34.8 | 358.0 / 394.6 | 304.8 / 417.1 | 7.8 / 191.8 | 196.0 / 977.5 | 1632.65 G | -0.09 % | 971.78 G | +4.52 % |
| 900 | lazy | 2 | 40 | 31.6 / 41.2 | 361.4 / 458.8 | 399.2 / 3900.7 | 89.3 / 799.4 | 602.6 / 3202.1 | 1626.56 G | -0.47 % | 1012.67 G | +8.91 % |
