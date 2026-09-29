# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 4.21
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: write barrier, mechanisms interleaved within each round
All five backends live at once; the same page's op stream replayed through each,
median over 60 rounds, order reversed on odd rounds. `plain` and `memcpy` run
identical `Vec<u64>` access code, so the gap between them is the noise floor.

Checkpoint `none` isolates the standing barrier: it runs on every write but never
copies. `every page` is §5.2's `\shipout` trigger, and `every 8 pages` is its other
trigger — a checkpoint per ~20 ms of engine time, which on a 2.4 ms body page is
one per eight pages — where the same copies are amortised over more work.

The `engine` rows add dependent integer work per access, which is what a real hot
loop has and what lets a superscalar core hide a cheap barrier.

### mem768k (14 MiB), tex-freelist, 1% touched/page, 18862 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.39 ns/op | 2.05 ns/op (-0.34, -0.27%) | 3.27 ns/op (+0.88, +0.69%) | 3.17 ns/op (+0.78, +0.61%) | 2.66 ns/op (+0.27, +0.21%) | 2.40 ns/op (+0.01, +0.01%) |
| engine | none | 10.93 ns/op | 10.98 ns/op (+0.05, +0.04%) | 11.13 ns/op (+0.20, +0.16%) | 11.14 ns/op (+0.20, +0.16%) | 10.97 ns/op (+0.04, +0.03%) | 10.97 ns/op (+0.03, +0.03%) |
| replay | every page | 2.59 ns/op | 16.74 ns/op (+14.15, **+11.12%**) | 6.85 ns/op (+4.25, **+3.34%**) | 6.78 ns/op (+4.18, **+3.29%**) | 5.52 ns/op (+2.93, +2.30%) | 2.51 ns/op (-0.08, -0.06%) |
| engine | every page | 11.10 ns/op | 26.41 ns/op (+15.31, **+12.03%**) | 14.18 ns/op (+3.08, +2.42%) | 14.10 ns/op (+3.00, +2.35%) | 13.36 ns/op (+2.26, +1.77%) | 10.99 ns/op (-0.11, -0.09%) |
| replay | every 8 pages | 2.42 ns/op | 4.51 ns/op (+2.09, +1.64%) | 4.13 ns/op (+1.71, +1.34%) | 3.95 ns/op (+1.54, +1.21%) | 3.43 ns/op (+1.01, +0.80%) | 2.44 ns/op (+0.02, +0.02%) |
| engine | every 8 pages | 10.88 ns/op | 13.64 ns/op (+2.76, +2.17%) | 11.69 ns/op (+0.81, +0.64%) | 11.73 ns/op (+0.85, +0.67%) | 11.41 ns/op (+0.53, +0.41%) | 10.88 ns/op (-0.00, -0.00%) |

### mem768k (14 MiB), tex-freelist, 5% touched/page, 94310 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.30 ns/op | 1.99 ns/op (-0.31, -1.23%) | 3.11 ns/op (+0.81, **+3.20%**) | 2.99 ns/op (+0.70, +2.73%) | 2.57 ns/op (+0.27, +1.07%) | 2.30 ns/op (-0.00, -0.00%) |
| engine | none | 10.83 ns/op | 10.95 ns/op (+0.12, +0.49%) | 11.08 ns/op (+0.25, +1.00%) | 11.11 ns/op (+0.28, +1.10%) | 10.92 ns/op (+0.09, +0.36%) | 10.94 ns/op (+0.11, +0.45%) |
| replay | every page | 2.37 ns/op | 6.86 ns/op (+4.49, **+17.65%**) | 4.16 ns/op (+1.80, **+7.06%**) | 4.12 ns/op (+1.76, **+6.90%**) | 3.49 ns/op (+1.13, **+4.42%**) | 2.34 ns/op (-0.02, -0.10%) |
| engine | every page | 10.78 ns/op | 15.67 ns/op (+4.88, **+19.19%**) | 11.91 ns/op (+1.13, **+4.44%**) | 11.82 ns/op (+1.04, **+4.08%**) | 11.50 ns/op (+0.71, +2.80%) | 10.70 ns/op (-0.08, -0.32%) |
| replay | every 8 pages | 2.27 ns/op | 2.79 ns/op (+0.52, +2.05%) | 3.31 ns/op (+1.04, **+4.08%**) | 3.32 ns/op (+1.05, **+4.14%**) | 2.81 ns/op (+0.54, +2.11%) | 2.30 ns/op (+0.03, +0.13%) |
| engine | every 8 pages | 10.69 ns/op | 11.82 ns/op (+1.13, **+4.44%**) | 11.14 ns/op (+0.44, +1.75%) | 11.18 ns/op (+0.48, +1.90%) | 10.97 ns/op (+0.28, +1.10%) | 10.68 ns/op (-0.01, -0.04%) |

### mem768k (14 MiB), uniform, 1% touched/page, 18862 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.68 ns/op | 2.35 ns/op (-0.33, -0.26%) | 3.90 ns/op (+1.22, +0.96%) | 3.68 ns/op (+0.99, +0.78%) | 3.05 ns/op (+0.36, +0.29%) | 2.68 ns/op (-0.00, -0.00%) |
| engine | none | 11.44 ns/op | 11.56 ns/op (+0.12, +0.09%) | 11.78 ns/op (+0.34, +0.27%) | 11.82 ns/op (+0.39, +0.30%) | 11.59 ns/op (+0.15, +0.12%) | 11.53 ns/op (+0.09, +0.07%) |
| replay | every page | 2.98 ns/op | 44.86 ns/op (+41.87, **+32.91%**) | 16.83 ns/op (+13.85, **+10.88%**) | 16.39 ns/op (+13.41, **+10.54%**) | 13.46 ns/op (+10.48, **+8.23%**) | 2.95 ns/op (-0.04, -0.03%) |
| engine | every page | 11.86 ns/op | 54.71 ns/op (+42.85, **+33.67%**) | 23.34 ns/op (+11.48, **+9.02%**) | 23.12 ns/op (+11.26, **+8.85%**) | 20.67 ns/op (+8.81, **+6.92%**) | 11.81 ns/op (-0.05, -0.04%) |
| replay | every 8 pages | 2.89 ns/op | 2.75 ns/op (-0.14, -0.11%) | 4.34 ns/op (+1.46, +1.14%) | 4.08 ns/op (+1.20, +0.94%) | 3.40 ns/op (+0.51, +0.40%) | 2.88 ns/op (-0.01, -0.01%) |
| engine | every 8 pages | 11.63 ns/op | 12.26 ns/op (+0.63, +0.49%) | 12.42 ns/op (+0.80, +0.63%) | 12.24 ns/op (+0.61, +0.48%) | 12.05 ns/op (+0.42, +0.33%) | 11.69 ns/op (+0.06, +0.05%) |

### mem768k (14 MiB), uniform, 5% touched/page, 94310 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.67 ns/op | 2.35 ns/op (-0.32, -1.26%) | 3.96 ns/op (+1.29, **+5.07%**) | 3.64 ns/op (+0.97, **+3.80%**) | 3.01 ns/op (+0.34, +1.35%) | 2.66 ns/op (-0.01, -0.05%) |
| engine | none | 11.09 ns/op | 11.19 ns/op (+0.10, +0.39%) | 11.37 ns/op (+0.28, +1.11%) | 11.42 ns/op (+0.33, +1.29%) | 11.14 ns/op (+0.05, +0.18%) | 11.03 ns/op (-0.06, -0.23%) |
| replay | every page | 2.66 ns/op | 12.06 ns/op (+9.40, **+36.94%**) | 6.22 ns/op (+3.56, **+14.01%**) | 6.31 ns/op (+3.65, **+14.34%**) | 5.20 ns/op (+2.55, **+10.01%**) | 2.68 ns/op (+0.02, +0.07%) |
| engine | every page | 11.29 ns/op | 21.53 ns/op (+10.24, **+40.24%**) | 13.97 ns/op (+2.67, **+10.51%**) | 13.95 ns/op (+2.66, **+10.44%**) | 13.24 ns/op (+1.94, **+7.63%**) | 11.11 ns/op (-0.18, -0.73%) |
| replay | every 8 pages | 2.66 ns/op | 2.36 ns/op (-0.29, -1.15%) | 3.82 ns/op (+1.16, **+4.58%**) | 3.58 ns/op (+0.92, **+3.63%**) | 3.00 ns/op (+0.34, +1.35%) | 2.64 ns/op (-0.02, -0.07%) |
| engine | every 8 pages | 11.15 ns/op | 11.44 ns/op (+0.29, +1.14%) | 11.72 ns/op (+0.57, +2.23%) | 11.55 ns/op (+0.40, +1.56%) | 11.31 ns/op (+0.16, +0.64%) | 11.14 ns/op (-0.01, -0.03%) |

### mem5M (47 MiB), tex-freelist, 1% touched/page, 61194 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.40 ns/op | 2.08 ns/op (-0.32, -0.80%) | 3.30 ns/op (+0.90, +2.30%) | 3.18 ns/op (+0.78, +1.98%) | 2.73 ns/op (+0.33, +0.84%) | 2.41 ns/op (+0.01, +0.02%) |
| engine | none | 10.94 ns/op | 11.01 ns/op (+0.07, +0.18%) | 11.13 ns/op (+0.19, +0.48%) | 11.21 ns/op (+0.27, +0.69%) | 11.00 ns/op (+0.07, +0.17%) | 10.97 ns/op (+0.03, +0.07%) |
| replay | every page | 2.50 ns/op | 13.05 ns/op (+10.55, **+26.91%**) | 4.91 ns/op (+2.41, **+6.15%**) | 4.84 ns/op (+2.34, **+5.97%**) | 4.11 ns/op (+1.61, **+4.11%**) | 2.43 ns/op (-0.07, -0.17%) |
| engine | every page | 11.01 ns/op | 21.34 ns/op (+10.33, **+26.34%**) | 12.49 ns/op (+1.48, **+3.77%**) | 12.38 ns/op (+1.37, **+3.50%**) | 12.06 ns/op (+1.05, +2.68%) | 10.81 ns/op (-0.21, -0.53%) |
| replay | every 8 pages | 2.43 ns/op | 4.35 ns/op (+1.92, **+4.89%**) | 4.10 ns/op (+1.67, **+4.25%**) | 3.97 ns/op (+1.54, **+3.93%**) | 3.37 ns/op (+0.94, +2.40%) | 2.39 ns/op (-0.04, -0.10%) |
| engine | every 8 pages | 11.19 ns/op | 13.56 ns/op (+2.37, **+6.05%**) | 12.00 ns/op (+0.81, +2.06%) | 11.97 ns/op (+0.78, +1.99%) | 11.71 ns/op (+0.52, +1.34%) | 11.20 ns/op (+0.01, +0.04%) |

### mem5M (47 MiB), tex-freelist, 5% touched/page, 305971 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.25 ns/op | 1.97 ns/op (-0.27, **-3.50%**) | 3.07 ns/op (+0.82, **+10.50%**) | 2.99 ns/op (+0.74, **+9.43%**) | 2.51 ns/op (+0.27, **+3.42%**) | 2.26 ns/op (+0.01, +0.14%) |
| engine | none | 10.70 ns/op | 10.78 ns/op (+0.08, +1.01%) | 10.96 ns/op (+0.26, **+3.33%**) | 10.93 ns/op (+0.23, +2.94%) | 10.78 ns/op (+0.08, +1.01%) | 10.68 ns/op (-0.02, -0.22%) |
| replay | every page | 2.30 ns/op | 5.87 ns/op (+3.57, **+45.46%**) | 3.91 ns/op (+1.61, **+20.52%**) | 3.81 ns/op (+1.51, **+19.25%**) | 3.30 ns/op (+0.99, **+12.68%**) | 2.30 ns/op (-0.00, -0.00%) |
| engine | every page | 10.85 ns/op | 14.89 ns/op (+4.04, **+51.51%**) | 11.76 ns/op (+0.91, **+11.60%**) | 11.71 ns/op (+0.87, **+11.06%**) | 11.45 ns/op (+0.60, **+7.66%**) | 10.77 ns/op (-0.08, -0.99%) |
| replay | every 8 pages | 2.30 ns/op | 3.44 ns/op (+1.15, **+14.64%**) | 3.65 ns/op (+1.35, **+17.23%**) | 3.58 ns/op (+1.28, **+16.34%**) | 3.00 ns/op (+0.71, **+9.04%**) | 2.25 ns/op (-0.04, -0.53%) |
| engine | every 8 pages | 10.73 ns/op | 12.47 ns/op (+1.74, **+22.21%**) | 11.37 ns/op (+0.64, **+8.15%**) | 11.33 ns/op (+0.60, **+7.67%**) | 11.12 ns/op (+0.39, **+4.94%**) | 10.75 ns/op (+0.02, +0.20%) |

### mem5M (47 MiB), uniform, 1% touched/page, 61194 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.91 ns/op | 2.54 ns/op (-0.36, -0.93%) | 4.33 ns/op (+1.42, **+3.63%**) | 3.99 ns/op (+1.08, +2.76%) | 3.32 ns/op (+0.41, +1.04%) | 2.85 ns/op (-0.06, -0.15%) |
| engine | none | 12.17 ns/op | 12.23 ns/op (+0.07, +0.17%) | 12.75 ns/op (+0.58, +1.49%) | 12.69 ns/op (+0.52, +1.34%) | 12.25 ns/op (+0.08, +0.21%) | 12.06 ns/op (-0.10, -0.26%) |
| replay | every page | 3.06 ns/op | 68.59 ns/op (+65.53, **+167.09%**) | 22.40 ns/op (+19.33, **+49.29%**) | 21.93 ns/op (+18.86, **+48.10%**) | 21.38 ns/op (+18.32, **+46.70%**) | 3.00 ns/op (-0.06, -0.16%) |
| engine | every page | 12.45 ns/op | 82.33 ns/op (+69.88, **+178.17%**) | 30.65 ns/op (+18.20, **+46.40%**) | 30.12 ns/op (+17.67, **+45.05%**) | 29.76 ns/op (+17.30, **+44.12%**) | 12.60 ns/op (+0.14, +0.36%) |
| replay | every 8 pages | 3.38 ns/op | 7.19 ns/op (+3.81, **+9.72%**) | 7.87 ns/op (+4.49, **+11.45%**) | 7.60 ns/op (+4.23, **+10.78%**) | 6.13 ns/op (+2.76, **+7.03%**) | 3.25 ns/op (-0.13, -0.33%) |
| engine | every 8 pages | 11.96 ns/op | 14.66 ns/op (+2.70, **+6.87%**) | 13.33 ns/op (+1.36, **+3.48%**) | 13.42 ns/op (+1.46, **+3.71%**) | 13.33 ns/op (+1.36, **+3.47%**) | 11.96 ns/op (-0.00, -0.01%) |

### mem5M (47 MiB), uniform, 5% touched/page, 305971 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.83 ns/op | 2.50 ns/op (-0.34, **-4.31%**) | 4.30 ns/op (+1.46, **+18.65%**) | 4.01 ns/op (+1.18, **+15.03%**) | 3.28 ns/op (+0.45, **+5.74%**) | 2.83 ns/op (-0.01, -0.08%) |
| engine | none | 11.65 ns/op | 11.83 ns/op (+0.18, +2.27%) | 12.16 ns/op (+0.51, **+6.48%**) | 12.18 ns/op (+0.52, **+6.66%**) | 11.82 ns/op (+0.17, +2.12%) | 11.77 ns/op (+0.12, +1.52%) |
| replay | every page | 2.86 ns/op | 20.81 ns/op (+17.95, **+228.79%**) | 9.97 ns/op (+7.11, **+90.61%**) | 9.55 ns/op (+6.69, **+85.30%**) | 8.63 ns/op (+5.77, **+73.51%**) | 2.85 ns/op (-0.01, -0.09%) |
| engine | every page | 11.68 ns/op | 30.27 ns/op (+18.58, **+236.92%**) | 17.48 ns/op (+5.79, **+73.86%**) | 17.15 ns/op (+5.47, **+69.68%**) | 16.71 ns/op (+5.03, **+64.13%**) | 11.61 ns/op (-0.07, -0.94%) |
| replay | every 8 pages | 2.82 ns/op | 2.56 ns/op (-0.26, **-3.31%**) | 4.25 ns/op (+1.43, **+18.17%**) | 4.08 ns/op (+1.25, **+15.94%**) | 3.29 ns/op (+0.47, **+5.94%**) | 2.84 ns/op (+0.01, +0.19%) |
| engine | every 8 pages | 11.65 ns/op | 11.94 ns/op (+0.29, **+3.76%**) | 12.20 ns/op (+0.55, **+7.06%**) | 12.15 ns/op (+0.50, **+6.41%**) | 11.81 ns/op (+0.16, +2.03%) | 11.63 ns/op (-0.02, -0.19%) |

### total64MB (64 MiB), tex-freelist, 1% touched/page, 83886 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.48 ns/op | 2.14 ns/op (-0.34, -1.20%) | 3.25 ns/op (+0.77, +2.68%) | 3.21 ns/op (+0.73, +2.53%) | 2.79 ns/op (+0.30, +1.07%) | 2.49 ns/op (+0.01, +0.04%) |
| engine | none | 10.97 ns/op | 11.12 ns/op (+0.14, +0.50%) | 11.14 ns/op (+0.17, +0.58%) | 11.16 ns/op (+0.18, +0.64%) | 10.98 ns/op (+0.01, +0.03%) | 10.96 ns/op (-0.02, -0.05%) |
| replay | every page | 2.50 ns/op | 11.64 ns/op (+9.13, **+31.92%**) | 4.62 ns/op (+2.12, **+7.40%**) | 4.55 ns/op (+2.04, **+7.13%**) | 3.91 ns/op (+1.40, **+4.90%**) | 2.39 ns/op (-0.11, -0.40%) |
| engine | every page | 11.05 ns/op | 21.17 ns/op (+10.12, **+35.36%**) | 12.37 ns/op (+1.31, **+4.59%**) | 12.28 ns/op (+1.22, **+4.28%**) | 11.93 ns/op (+0.88, **+3.07%**) | 10.89 ns/op (-0.16, -0.56%) |
| replay | every 8 pages | 2.46 ns/op | 4.70 ns/op (+2.24, **+7.84%**) | 4.01 ns/op (+1.55, **+5.43%**) | 3.92 ns/op (+1.46, **+5.11%**) | 3.34 ns/op (+0.88, **+3.08%**) | 2.43 ns/op (-0.03, -0.11%) |
| engine | every 8 pages | 11.00 ns/op | 14.29 ns/op (+3.29, **+11.50%**) | 12.06 ns/op (+1.06, **+3.72%**) | 11.75 ns/op (+0.75, +2.63%) | 11.58 ns/op (+0.59, +2.05%) | 11.09 ns/op (+0.09, +0.32%) |

### total64MB (64 MiB), tex-freelist, 5% touched/page, 419430 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.28 ns/op | 2.01 ns/op (-0.28, **-4.82%**) | 3.18 ns/op (+0.90, **+15.69%**) | 3.08 ns/op (+0.80, **+13.97%**) | 2.57 ns/op (+0.29, **+5.07%**) | 2.27 ns/op (-0.01, -0.21%) |
| engine | none | 10.70 ns/op | 10.80 ns/op (+0.10, +1.74%) | 10.98 ns/op (+0.27, **+4.76%**) | 11.00 ns/op (+0.30, **+5.19%**) | 10.78 ns/op (+0.08, +1.33%) | 10.72 ns/op (+0.02, +0.31%) |
| replay | every page | 2.32 ns/op | 5.64 ns/op (+3.32, **+57.94%**) | 3.97 ns/op (+1.65, **+28.86%**) | 3.84 ns/op (+1.52, **+26.54%**) | 3.28 ns/op (+0.96, **+16.83%**) | 2.29 ns/op (-0.03, -0.54%) |
| engine | every page | 10.76 ns/op | 14.86 ns/op (+4.11, **+71.75%**) | 11.63 ns/op (+0.87, **+15.22%**) | 11.56 ns/op (+0.81, **+14.09%**) | 11.36 ns/op (+0.60, **+10.49%**) | 10.77 ns/op (+0.01, +0.18%) |
| replay | every 8 pages | 2.32 ns/op | 3.51 ns/op (+1.19, **+20.85%**) | 3.71 ns/op (+1.39, **+24.25%**) | 3.63 ns/op (+1.31, **+22.91%**) | 2.99 ns/op (+0.67, **+11.67%**) | 2.26 ns/op (-0.06, -1.01%) |
| engine | every 8 pages | 10.75 ns/op | 12.65 ns/op (+1.90, **+33.23%**) | 11.42 ns/op (+0.67, **+11.74%**) | 11.44 ns/op (+0.69, **+12.12%**) | 11.15 ns/op (+0.40, **+6.94%**) | 10.73 ns/op (-0.02, -0.39%) |

### total64MB (64 MiB), uniform, 1% touched/page, 83886 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.93 ns/op | 2.54 ns/op (-0.39, -1.37%) | 4.46 ns/op (+1.53, **+5.35%**) | 4.14 ns/op (+1.22, **+4.25%**) | 3.33 ns/op (+0.41, +1.42%) | 2.97 ns/op (+0.04, +0.14%) |
| engine | none | 11.94 ns/op | 12.05 ns/op (+0.11, +0.38%) | 12.57 ns/op (+0.63, +2.20%) | 12.59 ns/op (+0.65, +2.27%) | 12.07 ns/op (+0.13, +0.47%) | 11.81 ns/op (-0.13, -0.46%) |
| replay | every page | 3.08 ns/op | 80.84 ns/op (+77.77, **+271.81%**) | 23.32 ns/op (+20.24, **+70.76%**) | 22.51 ns/op (+19.43, **+67.93%**) | 21.24 ns/op (+18.16, **+63.48%**) | 3.04 ns/op (-0.03, -0.12%) |
| engine | every page | 12.12 ns/op | 81.09 ns/op (+68.97, **+241.07%**) | 29.93 ns/op (+17.81, **+62.26%**) | 29.38 ns/op (+17.26, **+60.34%**) | 28.48 ns/op (+16.36, **+57.18%**) | 11.85 ns/op (-0.27, -0.95%) |
| replay | every 8 pages | 3.03 ns/op | 5.86 ns/op (+2.83, **+9.90%**) | 5.49 ns/op (+2.46, **+8.61%**) | 5.04 ns/op (+2.02, **+7.05%**) | 4.17 ns/op (+1.14, **+3.99%**) | 2.93 ns/op (-0.09, -0.33%) |
| engine | every 8 pages | 11.88 ns/op | 15.37 ns/op (+3.48, **+12.18%**) | 13.57 ns/op (+1.69, **+5.90%**) | 13.45 ns/op (+1.57, **+5.47%**) | 12.94 ns/op (+1.05, **+3.68%**) | 11.84 ns/op (-0.05, -0.16%) |

### total64MB (64 MiB), uniform, 5% touched/page, 419430 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.87 ns/op | 2.48 ns/op (-0.39, **-6.86%**) | 4.38 ns/op (+1.50, **+26.26%**) | 4.05 ns/op (+1.18, **+20.56%**) | 3.28 ns/op (+0.41, **+7.15%**) | 2.88 ns/op (+0.01, +0.12%) |
| engine | none | 11.77 ns/op | 11.88 ns/op (+0.11, +1.97%) | 12.28 ns/op (+0.51, **+8.91%**) | 12.19 ns/op (+0.42, **+7.35%**) | 11.83 ns/op (+0.06, +1.02%) | 11.73 ns/op (-0.04, -0.72%) |
| replay | every page | 2.98 ns/op | 23.31 ns/op (+20.33, **+355.24%**) | 10.42 ns/op (+7.44, **+129.97%**) | 9.91 ns/op (+6.92, **+120.99%**) | 9.02 ns/op (+6.04, **+105.59%**) | 2.92 ns/op (-0.06, -1.10%) |
| engine | every page | 11.76 ns/op | 33.31 ns/op (+21.55, **+376.56%**) | 17.81 ns/op (+6.05, **+105.73%**) | 17.46 ns/op (+5.69, **+99.51%**) | 16.93 ns/op (+5.16, **+90.19%**) | 11.71 ns/op (-0.06, -1.03%) |
| replay | every 8 pages | 2.87 ns/op | 2.59 ns/op (-0.28, **-4.88%**) | 4.42 ns/op (+1.55, **+27.11%**) | 4.10 ns/op (+1.24, **+21.61%**) | 3.37 ns/op (+0.50, **+8.80%**) | 2.84 ns/op (-0.03, -0.55%) |
| engine | every 8 pages | 11.78 ns/op | 11.96 ns/op (+0.18, **+3.13%**) | 12.28 ns/op (+0.49, **+8.63%**) | 12.19 ns/op (+0.41, **+7.21%**) | 11.88 ns/op (+0.10, +1.73%) | 11.74 ns/op (-0.04, -0.69%) |

### total200MB (200 MiB), tex-freelist, 1% touched/page, 262144 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.61 ns/op | 2.34 ns/op (-0.27, -2.96%) | 3.53 ns/op (+0.92, **+10.03%**) | 3.55 ns/op (+0.94, **+10.24%**) | 2.93 ns/op (+0.32, **+3.48%**) | 2.60 ns/op (-0.00, -0.04%) |
| engine | none | 10.99 ns/op | 11.35 ns/op (+0.36, **+3.92%**) | 11.53 ns/op (+0.54, **+5.90%**) | 11.54 ns/op (+0.55, **+5.99%**) | 11.29 ns/op (+0.29, **+3.20%**) | 11.13 ns/op (+0.13, +1.43%) |
| replay | every page | 2.44 ns/op | 10.67 ns/op (+8.24, **+89.95%**) | 4.32 ns/op (+1.88, **+20.57%**) | 4.16 ns/op (+1.72, **+18.81%**) | 3.55 ns/op (+1.11, **+12.16%**) | 2.41 ns/op (-0.03, -0.30%) |
| engine | every page | 11.12 ns/op | 19.87 ns/op (+8.75, **+95.56%**) | 12.24 ns/op (+1.13, **+12.30%**) | 12.09 ns/op (+0.98, **+10.68%**) | 11.81 ns/op (+0.69, **+7.51%**) | 11.01 ns/op (-0.11, -1.18%) |
| replay | every 8 pages | 2.43 ns/op | 4.63 ns/op (+2.20, **+24.06%**) | 4.19 ns/op (+1.76, **+19.26%**) | 4.06 ns/op (+1.63, **+17.85%**) | 3.41 ns/op (+0.98, **+10.71%**) | 2.45 ns/op (+0.02, +0.18%) |
| engine | every 8 pages | 11.03 ns/op | 13.53 ns/op (+2.50, **+27.36%**) | 12.08 ns/op (+1.06, **+11.53%**) | 12.08 ns/op (+1.05, **+11.52%**) | 11.68 ns/op (+0.66, **+7.17%**) | 11.01 ns/op (-0.02, -0.19%) |

### total200MB (200 MiB), tex-freelist, 5% touched/page, 1310720 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.27 ns/op | 2.00 ns/op (-0.27, **-14.72%**) | 3.16 ns/op (+0.89, **+48.81%**) | 3.09 ns/op (+0.82, **+44.76%**) | 2.53 ns/op (+0.26, **+14.22%**) | 2.27 ns/op (+0.00, +0.12%) |
| engine | none | 10.76 ns/op | 10.84 ns/op (+0.08, **+4.54%**) | 11.11 ns/op (+0.36, **+19.44%**) | 11.09 ns/op (+0.33, **+18.26%**) | 10.83 ns/op (+0.08, **+4.12%**) | 10.76 ns/op (-0.00, -0.14%) |
| replay | every page | 2.29 ns/op | 5.50 ns/op (+3.21, **+175.34%**) | 3.84 ns/op (+1.55, **+84.47%**) | 3.74 ns/op (+1.45, **+79.14%**) | 3.15 ns/op (+0.86, **+47.16%**) | 2.24 ns/op (-0.05, -2.88%) |
| engine | every page | 10.73 ns/op | 14.36 ns/op (+3.63, **+198.43%**) | 11.63 ns/op (+0.90, **+49.10%**) | 11.55 ns/op (+0.82, **+44.95%**) | 11.28 ns/op (+0.55, **+30.27%**) | 10.73 ns/op (-0.00, -0.15%) |
| replay | every 8 pages | 2.22 ns/op | 3.60 ns/op (+1.37, **+75.07%**) | 3.71 ns/op (+1.48, **+80.97%**) | 3.59 ns/op (+1.36, **+74.32%**) | 2.99 ns/op (+0.77, **+41.95%**) | 2.21 ns/op (-0.02, -0.96%) |
| engine | every 8 pages | 10.76 ns/op | 12.62 ns/op (+1.86, **+101.48%**) | 11.51 ns/op (+0.74, **+40.47%**) | 11.45 ns/op (+0.68, **+37.26%**) | 11.13 ns/op (+0.37, **+20.03%**) | 10.73 ns/op (-0.03, -1.79%) |

### total200MB (200 MiB), uniform, 1% touched/page, 262144 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 3.01 ns/op | 2.71 ns/op (-0.30, **-3.22%**) | 4.80 ns/op (+1.79, **+19.56%**) | 4.41 ns/op (+1.40, **+15.33%**) | 3.51 ns/op (+0.50, **+5.45%**) | 3.05 ns/op (+0.04, +0.42%) |
| engine | none | 12.11 ns/op | 12.26 ns/op (+0.15, +1.63%) | 13.01 ns/op (+0.90, **+9.80%**) | 12.86 ns/op (+0.75, **+8.20%**) | 12.27 ns/op (+0.16, +1.78%) | 12.10 ns/op (-0.02, -0.16%) |
| replay | every page | 3.13 ns/op | 77.11 ns/op (+73.97, **+808.00%**) | 24.23 ns/op (+21.10, **+230.43%**) | 23.76 ns/op (+20.63, **+225.33%**) | 22.06 ns/op (+18.92, **+206.69%**) | 3.11 ns/op (-0.03, -0.28%) |
| engine | every page | 12.26 ns/op | 85.54 ns/op (+73.28, **+800.38%**) | 30.98 ns/op (+18.72, **+204.48%**) | 30.43 ns/op (+18.17, **+198.51%**) | 28.97 ns/op (+16.71, **+182.49%**) | 12.12 ns/op (-0.14, -1.50%) |
| replay | every 8 pages | 3.02 ns/op | 8.21 ns/op (+5.19, **+56.72%**) | 5.99 ns/op (+2.97, **+32.42%**) | 5.80 ns/op (+2.78, **+30.32%**) | 4.55 ns/op (+1.53, **+16.68%**) | 3.04 ns/op (+0.02, +0.22%) |
| engine | every 8 pages | 12.05 ns/op | 16.56 ns/op (+4.51, **+49.27%**) | 14.05 ns/op (+2.00, **+21.85%**) | 14.17 ns/op (+2.12, **+23.15%**) | 13.15 ns/op (+1.09, **+11.95%**) | 12.18 ns/op (+0.13, +1.40%) |

### total200MB (200 MiB), uniform, 5% touched/page, 1310720 ops/page
| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |
|---|---|---|---|---|---|---|---|
| replay | none | 2.96 ns/op | 2.56 ns/op (-0.40, **-21.80%**) | 4.60 ns/op (+1.64, **+89.79%**) | 4.18 ns/op (+1.22, **+66.86%**) | 3.42 ns/op (+0.46, **+25.39%**) | 2.97 ns/op (+0.01, +0.54%) |
| engine | none | 11.93 ns/op | 11.99 ns/op (+0.06, **+3.10%**) | 12.60 ns/op (+0.67, **+36.53%**) | 12.47 ns/op (+0.54, **+29.44%**) | 12.01 ns/op (+0.08, **+4.32%**) | 11.99 ns/op (+0.06, **+3.15%**) |
| replay | every page | 2.97 ns/op | 26.24 ns/op (+23.27, **+1270.59%**) | 11.11 ns/op (+8.14, **+444.53%**) | 10.52 ns/op (+7.55, **+412.27%**) | 9.42 ns/op (+6.45, **+352.40%**) | 2.99 ns/op (+0.02, +0.86%) |
| engine | every page | 11.95 ns/op | 36.33 ns/op (+24.38, **+1331.68%**) | 18.75 ns/op (+6.81, **+371.65%**) | 18.35 ns/op (+6.40, **+349.42%**) | 17.50 ns/op (+5.55, **+303.34%**) | 11.94 ns/op (-0.01, -0.48%) |
| replay | every 8 pages | 2.97 ns/op | 2.64 ns/op (-0.33, **-17.96%**) | 4.63 ns/op (+1.66, **+90.69%**) | 4.26 ns/op (+1.30, **+70.92%**) | 3.48 ns/op (+0.52, **+28.21%**) | 2.97 ns/op (+0.01, +0.48%) |
| engine | every 8 pages | 11.95 ns/op | 12.15 ns/op (+0.20, **+11.18%**) | 12.67 ns/op (+0.72, **+39.44%**) | 12.50 ns/op (+0.56, **+30.37%**) | 12.04 ns/op (+0.09, **+5.08%**) | 11.94 ns/op (-0.01, -0.45%) |

