# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 5.96
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: hot loop
`plain` is a `Vec<u64>` with no snapshot support: the denominator. Each mechanism
is then measured twice.

* **barrier** — no checkpoint outstanding, so the barrier runs on every write but
never copies. This is the standing tax on the hot loop, the thing DESIGN §5.2
caps at 3%.
* **+copies** — a checkpoint at every page boundary, so the barrier's slow path
copies every chunk the page dirties, once. This is per-checkpoint work, not a
per-access tax, and it belongs with the snapshot cost rather than the barrier.

`extra/page` is absolute, so it can be read against the DESIGN B.1 page budgets
(2.4 ms body page, 90 ms heavy pgfplots page) whatever this synthetic loop's own
speed is. `replay x` is the ratio against `plain` on the bare replay loop: the
pessimistic bound, since a real engine does far more work per access.

The snapshot itself is **not** in these numbers; it is in the snapshot phase.

### mem768k (14 MiB), tex-freelist, 1% touched/page, 18862 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 1.90 | 0.036 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.73 | 0.033 | -0.003 ms | -0.14% | -0.00% | 0.908x |
| `kernel-remap` | +copies | replay | 13.51 | 0.255 | +0.219 ms | **+9.13%** | +0.24% | 7.113x |
| `arc-make-mut` | barrier | replay | 2.44 | 0.046 | +0.010 ms | +0.42% | +0.01% | 1.283x |
| `arc-make-mut` | +copies | replay | 5.09 | 0.096 | +0.060 ms | +2.51% | +0.07% | 2.679x |
| `chunk-bitmap` | barrier | replay | 2.26 | 0.043 | +0.007 ms | +0.28% | +0.01% | 1.190x |
| `chunk-bitmap` | +copies | replay | 4.97 | 0.094 | +0.058 ms | +2.41% | +0.06% | 2.616x |
| `flat-undo-log` | barrier | replay | 2.03 | 0.038 | +0.002 ms | +0.10% | +0.00% | 1.067x |
| `flat-undo-log` | +copies | replay | 3.42 | 0.065 | +0.029 ms | +1.19% | +0.03% | 1.800x |
| `memcpy` | barrier | replay | 1.68 | 0.032 | -0.004 ms | -0.17% | -0.00% | 0.884x |
| `memcpy` | +copies | replay | 1.93 | 0.036 | +0.001 ms | +0.03% | +0.00% | 1.017x |
| `plain (no snapshot)` | — | engine | 10.22 | 0.193 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.32 | 0.195 | +0.002 ms | +0.07% | +0.00% | 1.009x |
| `kernel-remap` | +copies | engine | 21.48 | 0.405 | +0.212 ms | **+8.84%** | +0.24% | 2.101x |
| `arc-make-mut` | barrier | engine | 10.46 | 0.197 | +0.005 ms | +0.19% | +0.01% | 1.024x |
| `arc-make-mut` | +copies | engine | 12.49 | 0.236 | +0.043 ms | +1.78% | +0.05% | 1.221x |
| `chunk-bitmap` | barrier | engine | 10.34 | 0.195 | +0.002 ms | +0.09% | +0.00% | 1.011x |
| `chunk-bitmap` | +copies | engine | 12.56 | 0.237 | +0.044 ms | +1.84% | +0.05% | 1.229x |
| `flat-undo-log` | barrier | engine | 10.55 | 0.199 | +0.006 ms | +0.25% | +0.01% | 1.032x |
| `flat-undo-log` | +copies | engine | 11.47 | 0.216 | +0.024 ms | +0.98% | +0.03% | 1.122x |
| `memcpy` | barrier | engine | 10.27 | 0.194 | +0.001 ms | +0.03% | +0.00% | 1.004x |
| `memcpy` | +copies | engine | 10.49 | 0.198 | +0.005 ms | +0.21% | +0.01% | 1.026x |

### mem768k (14 MiB), tex-freelist, 5% touched/page, 94310 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 1.79 | 0.168 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.62 | 0.153 | -0.016 ms | -0.66% | -0.02% | 0.906x |
| `kernel-remap` | +copies | replay | 5.19 | 0.489 | +0.321 ms | **+13.37%** | +0.36% | 2.905x |
| `arc-make-mut` | barrier | replay | 2.39 | 0.225 | +0.057 ms | +2.36% | +0.06% | 1.336x |
| `arc-make-mut` | +copies | replay | 3.94 | 0.372 | +0.204 ms | **+8.48%** | +0.23% | 2.209x |
| `chunk-bitmap` | barrier | replay | 2.36 | 0.223 | +0.054 ms | +2.26% | +0.06% | 1.322x |
| `chunk-bitmap` | +copies | replay | 3.71 | 0.349 | +0.181 ms | **+7.55%** | +0.20% | 2.076x |
| `flat-undo-log` | barrier | replay | 2.21 | 0.208 | +0.040 ms | +1.67% | +0.04% | 1.238x |
| `flat-undo-log` | +copies | replay | 2.87 | 0.271 | +0.103 ms | **+4.27%** | +0.11% | 1.609x |
| `memcpy` | barrier | replay | 1.82 | 0.172 | +0.003 ms | +0.13% | +0.00% | 1.019x |
| `memcpy` | +copies | replay | 1.94 | 0.183 | +0.015 ms | +0.62% | +0.02% | 1.089x |
| `plain (no snapshot)` | — | engine | 10.47 | 0.988 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.39 | 0.980 | -0.008 ms | -0.33% | -0.01% | 0.992x |
| `kernel-remap` | +copies | engine | 14.41 | 1.359 | +0.371 ms | **+15.46%** | +0.41% | 1.376x |
| `arc-make-mut` | barrier | engine | 10.47 | 0.987 | -0.000 ms | -0.02% | -0.00% | 1.000x |
| `arc-make-mut` | +copies | engine | 11.21 | 1.058 | +0.070 ms | +2.92% | +0.08% | 1.071x |
| `chunk-bitmap` | barrier | engine | 10.29 | 0.970 | -0.018 ms | -0.73% | -0.02% | 0.982x |
| `chunk-bitmap` | +copies | engine | 11.28 | 1.064 | +0.076 ms | **+3.18%** | +0.08% | 1.077x |
| `flat-undo-log` | barrier | engine | 10.31 | 0.972 | -0.015 ms | -0.63% | -0.02% | 0.985x |
| `flat-undo-log` | +copies | engine | 11.02 | 1.039 | +0.052 ms | +2.16% | +0.06% | 1.052x |
| `memcpy` | barrier | engine | 10.44 | 0.984 | -0.003 ms | -0.14% | -0.00% | 0.997x |
| `memcpy` | +copies | engine | 10.42 | 0.982 | -0.005 ms | -0.22% | -0.01% | 0.995x |

### mem768k (14 MiB), uniform, 1% touched/page, 18862 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 1.87 | 0.035 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.98 | 0.037 | +0.002 ms | +0.09% | +0.00% | 1.059x |
| `kernel-remap` | +copies | replay | 39.11 | 0.738 | +0.702 ms | **+29.27%** | +0.78% | 20.904x |
| `arc-make-mut` | barrier | replay | 3.05 | 0.057 | +0.022 ms | +0.92% | +0.02% | 1.628x |
| `arc-make-mut` | +copies | replay | 13.34 | 0.252 | +0.216 ms | **+9.01%** | +0.24% | 7.130x |
| `chunk-bitmap` | barrier | replay | 2.33 | 0.044 | +0.009 ms | +0.36% | +0.01% | 1.246x |
| `chunk-bitmap` | +copies | replay | 12.86 | 0.243 | +0.207 ms | **+8.64%** | +0.23% | 6.872x |
| `flat-undo-log` | barrier | replay | 2.20 | 0.042 | +0.006 ms | +0.26% | +0.01% | 1.177x |
| `flat-undo-log` | +copies | replay | 7.84 | 0.148 | +0.113 ms | **+4.69%** | +0.13% | 4.191x |
| `memcpy` | barrier | replay | 1.66 | 0.031 | -0.004 ms | -0.17% | -0.00% | 0.885x |
| `memcpy` | +copies | replay | 1.82 | 0.034 | -0.001 ms | -0.04% | -0.00% | 0.972x |
| `plain (no snapshot)` | — | engine | 10.28 | 0.194 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.37 | 0.196 | +0.002 ms | +0.07% | +0.00% | 1.009x |
| `kernel-remap` | +copies | engine | 47.46 | 0.895 | +0.701 ms | **+29.22%** | +0.78% | 4.616x |
| `arc-make-mut` | barrier | engine | 10.65 | 0.201 | +0.007 ms | +0.29% | +0.01% | 1.036x |
| `arc-make-mut` | +copies | engine | 19.62 | 0.370 | +0.176 ms | **+7.34%** | +0.20% | 1.908x |
| `chunk-bitmap` | barrier | engine | 10.47 | 0.197 | +0.004 ms | +0.15% | +0.00% | 1.018x |
| `chunk-bitmap` | +copies | engine | 18.62 | 0.351 | +0.157 ms | **+6.55%** | +0.17% | 1.811x |
| `flat-undo-log` | barrier | engine | 10.31 | 0.195 | +0.001 ms | +0.02% | +0.00% | 1.003x |
| `flat-undo-log` | +copies | engine | 14.49 | 0.273 | +0.079 ms | **+3.31%** | +0.09% | 1.409x |
| `memcpy` | barrier | engine | 10.14 | 0.191 | -0.003 ms | -0.11% | -0.00% | 0.986x |
| `memcpy` | +copies | engine | 10.35 | 0.195 | +0.001 ms | +0.06% | +0.00% | 1.007x |

### mem768k (14 MiB), uniform, 5% touched/page, 94310 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 1.52 | 0.143 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.82 | 0.172 | +0.028 ms | +1.19% | +0.03% | 1.199x |
| `kernel-remap` | +copies | replay | 11.20 | 1.057 | +0.913 ms | **+38.05%** | +1.01% | 7.365x |
| `arc-make-mut` | barrier | replay | 3.12 | 0.295 | +0.151 ms | **+6.30%** | +0.17% | 2.054x |
| `arc-make-mut` | +copies | replay | 5.70 | 0.537 | +0.394 ms | **+16.41%** | +0.44% | 3.746x |
| `chunk-bitmap` | barrier | replay | 2.11 | 0.199 | +0.056 ms | +2.31% | +0.06% | 1.387x |
| `chunk-bitmap` | +copies | replay | 4.80 | 0.453 | +0.309 ms | **+12.88%** | +0.34% | 3.155x |
| `flat-undo-log` | barrier | replay | 2.20 | 0.207 | +0.064 ms | +2.66% | +0.07% | 1.444x |
| `flat-undo-log` | +copies | replay | 3.80 | 0.359 | +0.215 ms | **+8.96%** | +0.24% | 2.500x |
| `memcpy` | barrier | replay | 2.05 | 0.193 | +0.050 ms | +2.08% | +0.06% | 1.348x |
| `memcpy` | +copies | replay | 2.07 | 0.195 | +0.052 ms | +2.16% | +0.06% | 1.361x |
| `plain (no snapshot)` | — | engine | 10.61 | 1.001 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.98 | 1.035 | +0.034 ms | +1.43% | +0.04% | 1.034x |
| `kernel-remap` | +copies | engine | 21.80 | 2.056 | +1.055 ms | **+43.97%** | +1.17% | 2.054x |
| `arc-make-mut` | barrier | engine | 10.62 | 1.001 | +0.000 ms | +0.01% | +0.00% | 1.000x |
| `arc-make-mut` | +copies | engine | 13.16 | 1.241 | +0.240 ms | **+10.00%** | +0.27% | 1.240x |
| `chunk-bitmap` | barrier | engine | 10.34 | 0.976 | -0.025 ms | -1.06% | -0.03% | 0.975x |
| `chunk-bitmap` | +copies | engine | 12.96 | 1.222 | +0.221 ms | **+9.21%** | +0.25% | 1.221x |
| `flat-undo-log` | barrier | engine | 10.58 | 0.998 | -0.003 ms | -0.13% | -0.00% | 0.997x |
| `flat-undo-log` | +copies | engine | 11.35 | 1.070 | +0.069 ms | +2.88% | +0.08% | 1.069x |
| `memcpy` | barrier | engine | 10.23 | 0.965 | -0.036 ms | -1.50% | -0.04% | 0.964x |
| `memcpy` | +copies | engine | 10.45 | 0.986 | -0.015 ms | -0.62% | -0.02% | 0.985x |

### mem5M (47 MiB), tex-freelist, 1% touched/page, 61194 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.12 | 0.130 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.82 | 0.111 | -0.018 ms | -0.77% | -0.02% | 0.857x |
| `kernel-remap` | +copies | replay | 9.43 | 0.577 | +0.448 ms | **+18.65%** | +0.50% | 4.451x |
| `arc-make-mut` | barrier | replay | 2.76 | 0.169 | +0.039 ms | +1.64% | +0.04% | 1.303x |
| `arc-make-mut` | +copies | replay | 4.44 | 0.272 | +0.142 ms | **+5.91%** | +0.16% | 2.094x |
| `chunk-bitmap` | barrier | replay | 2.72 | 0.167 | +0.037 ms | +1.54% | +0.04% | 1.285x |
| `chunk-bitmap` | +copies | replay | 4.29 | 0.262 | +0.133 ms | **+5.53%** | +0.15% | 2.023x |
| `flat-undo-log` | barrier | replay | 2.28 | 0.139 | +0.010 ms | +0.40% | +0.01% | 1.074x |
| `flat-undo-log` | +copies | replay | 3.45 | 0.211 | +0.081 ms | **+3.38%** | +0.09% | 1.626x |
| `memcpy` | barrier | replay | 2.11 | 0.129 | -0.001 ms | -0.03% | -0.00% | 0.995x |
| `memcpy` | +copies | replay | 2.23 | 0.137 | +0.007 ms | +0.29% | +0.01% | 1.053x |
| `plain (no snapshot)` | — | engine | 10.45 | 0.640 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.70 | 0.655 | +0.015 ms | +0.62% | +0.02% | 1.023x |
| `kernel-remap` | +copies | engine | 19.02 | 1.164 | +0.524 ms | **+21.83%** | +0.58% | 1.819x |
| `arc-make-mut` | barrier | engine | 10.69 | 0.654 | +0.015 ms | +0.60% | +0.02% | 1.023x |
| `arc-make-mut` | +copies | engine | 12.06 | 0.738 | +0.098 ms | **+4.09%** | +0.11% | 1.153x |
| `chunk-bitmap` | barrier | engine | 10.70 | 0.655 | +0.015 ms | +0.62% | +0.02% | 1.023x |
| `chunk-bitmap` | +copies | engine | 12.02 | 0.736 | +0.096 ms | **+4.00%** | +0.11% | 1.150x |
| `flat-undo-log` | barrier | engine | 10.49 | 0.642 | +0.002 ms | +0.10% | +0.00% | 1.004x |
| `flat-undo-log` | +copies | engine | 11.64 | 0.713 | +0.073 ms | **+3.04%** | +0.08% | 1.114x |
| `memcpy` | barrier | engine | 10.47 | 0.641 | +0.001 ms | +0.05% | +0.00% | 1.002x |
| `memcpy` | +copies | engine | 10.68 | 0.654 | +0.014 ms | +0.59% | +0.02% | 1.022x |

### mem5M (47 MiB), tex-freelist, 5% touched/page, 305971 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.06 | 0.630 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.79 | 0.547 | -0.083 ms | **-3.45%** | -0.09% | 0.869x |
| `kernel-remap` | +copies | replay | 5.04 | 1.542 | +0.912 ms | **+38.01%** | +1.01% | 2.448x |
| `arc-make-mut` | barrier | replay | 2.70 | 0.827 | +0.197 ms | **+8.22%** | +0.22% | 1.313x |
| `arc-make-mut` | +copies | replay | 3.66 | 1.119 | +0.489 ms | **+20.38%** | +0.54% | 1.776x |
| `chunk-bitmap` | barrier | replay | 2.81 | 0.861 | +0.231 ms | **+9.63%** | +0.26% | 1.367x |
| `chunk-bitmap` | +copies | replay | 3.66 | 1.119 | +0.489 ms | **+20.36%** | +0.54% | 1.776x |
| `flat-undo-log` | barrier | replay | 2.26 | 0.691 | +0.061 ms | +2.55% | +0.07% | 1.097x |
| `flat-undo-log` | +copies | replay | 3.03 | 0.927 | +0.297 ms | **+12.36%** | +0.33% | 1.471x |
| `memcpy` | barrier | replay | 2.13 | 0.652 | +0.022 ms | +0.92% | +0.02% | 1.035x |
| `memcpy` | +copies | replay | 2.19 | 0.671 | +0.041 ms | +1.71% | +0.05% | 1.065x |
| `plain (no snapshot)` | — | engine | 10.67 | 3.266 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.55 | 3.228 | -0.038 ms | -1.57% | -0.04% | 0.988x |
| `kernel-remap` | +copies | engine | 14.02 | 4.290 | +1.024 ms | **+42.65%** | +1.14% | 1.313x |
| `arc-make-mut` | barrier | engine | 10.93 | 3.345 | +0.079 ms | **+3.30%** | +0.09% | 1.024x |
| `arc-make-mut` | +copies | engine | 11.74 | 3.591 | +0.325 ms | **+13.54%** | +0.36% | 1.100x |
| `chunk-bitmap` | barrier | engine | 10.71 | 3.278 | +0.012 ms | +0.51% | +0.01% | 1.004x |
| `chunk-bitmap` | +copies | engine | 11.45 | 3.503 | +0.237 ms | **+9.89%** | +0.26% | 1.073x |
| `flat-undo-log` | barrier | engine | 10.59 | 3.239 | -0.027 ms | -1.13% | -0.03% | 0.992x |
| `flat-undo-log` | +copies | engine | 11.22 | 3.432 | +0.166 ms | **+6.90%** | +0.18% | 1.051x |
| `memcpy` | barrier | engine | 10.77 | 3.296 | +0.030 ms | +1.26% | +0.03% | 1.009x |
| `memcpy` | +copies | engine | 10.68 | 3.267 | +0.001 ms | +0.05% | +0.00% | 1.000x |

### mem5M (47 MiB), uniform, 1% touched/page, 61194 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.74 | 0.168 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.29 | 0.140 | -0.028 ms | -1.16% | -0.03% | 0.835x |
| `kernel-remap` | +copies | replay | 54.79 | 3.353 | +3.185 ms | **+132.71%** | **+3.54%** | 19.968x |
| `arc-make-mut` | barrier | replay | 3.99 | 0.244 | +0.076 ms | **+3.18%** | +0.08% | 1.454x |
| `arc-make-mut` | +copies | replay | 22.40 | 1.371 | +1.203 ms | **+50.12%** | +1.34% | 8.163x |
| `chunk-bitmap` | barrier | replay | 3.69 | 0.226 | +0.058 ms | +2.41% | +0.06% | 1.345x |
| `chunk-bitmap` | +copies | replay | 21.82 | 1.335 | +1.167 ms | **+48.63%** | +1.30% | 7.951x |
| `flat-undo-log` | barrier | replay | 3.05 | 0.187 | +0.019 ms | +0.78% | +0.02% | 1.112x |
| `flat-undo-log` | +copies | replay | 22.20 | 1.359 | +1.191 ms | **+49.61%** | +1.32% | 8.091x |
| `memcpy` | barrier | replay | 2.65 | 0.162 | -0.006 ms | -0.24% | -0.01% | 0.966x |
| `memcpy` | +copies | replay | 2.77 | 0.170 | +0.002 ms | +0.07% | +0.00% | 1.010x |
| `plain (no snapshot)` | — | engine | 11.26 | 0.689 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 11.26 | 0.689 | -0.000 ms | -0.01% | -0.00% | 1.000x |
| `kernel-remap` | +copies | engine | 71.16 | 4.355 | +3.666 ms | **+152.74%** | **+4.07%** | 6.321x |
| `arc-make-mut` | barrier | engine | 11.82 | 0.723 | +0.035 ms | +1.44% | +0.04% | 1.050x |
| `arc-make-mut` | +copies | engine | 29.38 | 1.798 | +1.109 ms | **+46.20%** | +1.23% | 2.609x |
| `chunk-bitmap` | barrier | engine | 11.73 | 0.718 | +0.029 ms | +1.20% | +0.03% | 1.042x |
| `chunk-bitmap` | +copies | engine | 28.70 | 1.756 | +1.067 ms | **+44.46%** | +1.19% | 2.549x |
| `flat-undo-log` | barrier | engine | 11.23 | 0.687 | -0.002 ms | -0.07% | -0.00% | 0.998x |
| `flat-undo-log` | +copies | engine | 28.97 | 1.773 | +1.084 ms | **+45.16%** | +1.20% | 2.573x |
| `memcpy` | barrier | engine | 11.44 | 0.700 | +0.011 ms | +0.47% | +0.01% | 1.016x |
| `memcpy` | +copies | engine | 11.50 | 0.704 | +0.015 ms | +0.62% | +0.02% | 1.022x |

### mem5M (47 MiB), uniform, 5% touched/page, 305971 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.70 | 0.825 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.41 | 0.739 | -0.086 ms | **-3.59%** | -0.10% | 0.896x |
| `kernel-remap` | +copies | replay | 22.09 | 6.758 | +5.933 ms | **+247.20%** | **+6.59%** | 8.190x |
| `arc-make-mut` | barrier | replay | 4.00 | 1.223 | +0.398 ms | **+16.60%** | +0.44% | 1.483x |
| `arc-make-mut` | +copies | replay | 10.26 | 3.140 | +2.315 ms | **+96.45%** | +2.57% | 3.805x |
| `chunk-bitmap` | barrier | replay | 3.75 | 1.147 | +0.322 ms | **+13.40%** | +0.36% | 1.390x |
| `chunk-bitmap` | +copies | replay | 9.77 | 2.989 | +2.164 ms | **+90.16%** | +2.40% | 3.623x |
| `flat-undo-log` | barrier | replay | 3.13 | 0.958 | +0.133 ms | **+5.52%** | +0.15% | 1.161x |
| `flat-undo-log` | +copies | replay | 9.42 | 2.881 | +2.056 ms | **+85.67%** | +2.28% | 3.492x |
| `memcpy` | barrier | replay | 2.79 | 0.853 | +0.028 ms | +1.18% | +0.03% | 1.034x |
| `memcpy` | +copies | replay | 2.85 | 0.874 | +0.048 ms | +2.02% | +0.05% | 1.059x |
| `plain (no snapshot)` | — | engine | 11.58 | 3.543 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 11.63 | 3.558 | +0.015 ms | +0.62% | +0.02% | 1.004x |
| `kernel-remap` | +copies | engine | 29.25 | 8.949 | +5.406 ms | **+225.25%** | **+6.01%** | 2.526x |
| `arc-make-mut` | barrier | engine | 11.78 | 3.606 | +0.063 ms | +2.61% | +0.07% | 1.018x |
| `arc-make-mut` | +copies | engine | 17.61 | 5.387 | +1.844 ms | **+76.84%** | +2.05% | 1.521x |
| `chunk-bitmap` | barrier | engine | 11.81 | 3.615 | +0.072 ms | **+3.00%** | +0.08% | 1.020x |
| `chunk-bitmap` | +copies | engine | 17.41 | 5.328 | +1.785 ms | **+74.39%** | +1.98% | 1.504x |
| `flat-undo-log` | barrier | engine | 11.52 | 3.524 | -0.019 ms | -0.80% | -0.02% | 0.995x |
| `flat-undo-log` | +copies | engine | 17.37 | 5.315 | +1.772 ms | **+73.82%** | +1.97% | 1.500x |
| `memcpy` | barrier | engine | 11.65 | 3.563 | +0.020 ms | +0.85% | +0.02% | 1.006x |
| `memcpy` | +copies | engine | 11.79 | 3.607 | +0.064 ms | +2.68% | +0.07% | 1.018x |

### total64MB (64 MiB), tex-freelist, 1% touched/page, 83886 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.21 | 0.185 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.07 | 0.173 | -0.012 ms | -0.49% | -0.01% | 0.936x |
| `kernel-remap` | +copies | replay | 10.89 | 0.914 | +0.728 ms | **+30.35%** | +0.81% | 4.931x |
| `arc-make-mut` | barrier | replay | 2.80 | 0.235 | +0.049 ms | +2.06% | +0.05% | 1.266x |
| `arc-make-mut` | +copies | replay | 4.11 | 0.345 | +0.160 ms | **+6.65%** | +0.18% | 1.861x |
| `chunk-bitmap` | barrier | replay | 2.65 | 0.222 | +0.037 ms | +1.54% | +0.04% | 1.200x |
| `chunk-bitmap` | +copies | replay | 4.13 | 0.346 | +0.161 ms | **+6.71%** | +0.18% | 1.870x |
| `flat-undo-log` | barrier | replay | 2.31 | 0.194 | +0.009 ms | +0.36% | +0.01% | 1.047x |
| `flat-undo-log` | +copies | replay | 3.22 | 0.270 | +0.085 ms | **+3.53%** | +0.09% | 1.457x |
| `memcpy` | barrier | replay | 2.03 | 0.170 | -0.015 ms | -0.62% | -0.02% | 0.919x |
| `memcpy` | +copies | replay | 2.23 | 0.187 | +0.002 ms | +0.07% | +0.00% | 1.009x |
| `plain (no snapshot)` | — | engine | 10.54 | 0.884 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.61 | 0.890 | +0.006 ms | +0.24% | +0.01% | 1.006x |
| `kernel-remap` | +copies | engine | 18.50 | 1.552 | +0.667 ms | **+27.81%** | +0.74% | 1.755x |
| `arc-make-mut` | barrier | engine | 10.98 | 0.921 | +0.037 ms | +1.53% | +0.04% | 1.042x |
| `arc-make-mut` | +copies | engine | 12.40 | 1.040 | +0.156 ms | **+6.51%** | +0.17% | 1.177x |
| `chunk-bitmap` | barrier | engine | 10.88 | 0.913 | +0.029 ms | +1.20% | +0.03% | 1.032x |
| `chunk-bitmap` | +copies | engine | 12.06 | 1.011 | +0.127 ms | **+5.30%** | +0.14% | 1.144x |
| `flat-undo-log` | barrier | engine | 10.99 | 0.922 | +0.037 ms | +1.56% | +0.04% | 1.042x |
| `flat-undo-log` | +copies | engine | 12.13 | 1.018 | +0.133 ms | **+5.56%** | +0.15% | 1.151x |
| `memcpy` | barrier | engine | 10.90 | 0.914 | +0.030 ms | +1.26% | +0.03% | 1.034x |
| `memcpy` | +copies | engine | 11.08 | 0.930 | +0.045 ms | +1.89% | +0.05% | 1.051x |

### total64MB (64 MiB), tex-freelist, 5% touched/page, 419430 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.26 | 0.946 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.88 | 0.788 | -0.158 ms | **-6.56%** | -0.17% | 0.834x |
| `kernel-remap` | +copies | replay | 5.37 | 2.254 | +1.308 ms | **+54.49%** | +1.45% | 2.382x |
| `arc-make-mut` | barrier | replay | 2.90 | 1.214 | +0.268 ms | **+11.18%** | +0.30% | 1.284x |
| `arc-make-mut` | +copies | replay | 3.67 | 1.538 | +0.592 ms | **+24.68%** | +0.66% | 1.626x |
| `chunk-bitmap` | barrier | replay | 2.84 | 1.190 | +0.244 ms | **+10.18%** | +0.27% | 1.258x |
| `chunk-bitmap` | +copies | replay | 4.37 | 1.833 | +0.887 ms | **+36.98%** | +0.99% | 1.938x |
| `flat-undo-log` | barrier | replay | 2.88 | 1.206 | +0.260 ms | **+10.83%** | +0.29% | 1.275x |
| `flat-undo-log` | +copies | replay | 3.34 | 1.403 | +0.457 ms | **+19.04%** | +0.51% | 1.483x |
| `memcpy` | barrier | replay | 2.23 | 0.933 | -0.013 ms | -0.52% | -0.01% | 0.987x |
| `memcpy` | +copies | replay | 2.19 | 0.919 | -0.027 ms | -1.12% | -0.03% | 0.972x |
| `plain (no snapshot)` | — | engine | 10.69 | 4.483 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.73 | 4.499 | +0.016 ms | +0.66% | +0.02% | 1.004x |
| `kernel-remap` | +copies | engine | 14.29 | 5.994 | +1.511 ms | **+62.95%** | +1.68% | 1.337x |
| `arc-make-mut` | barrier | engine | 10.79 | 4.525 | +0.042 ms | +1.75% | +0.05% | 1.009x |
| `arc-make-mut` | +copies | engine | 11.43 | 4.794 | +0.311 ms | **+12.96%** | +0.35% | 1.069x |
| `chunk-bitmap` | barrier | engine | 10.72 | 4.498 | +0.015 ms | +0.62% | +0.02% | 1.003x |
| `chunk-bitmap` | +copies | engine | 11.56 | 4.848 | +0.365 ms | **+15.22%** | +0.41% | 1.081x |
| `flat-undo-log` | barrier | engine | 10.77 | 4.516 | +0.033 ms | +1.38% | +0.04% | 1.007x |
| `flat-undo-log` | +copies | engine | 11.42 | 4.788 | +0.305 ms | **+12.73%** | +0.34% | 1.068x |
| `memcpy` | barrier | engine | 10.84 | 4.549 | +0.066 ms | +2.75% | +0.07% | 1.015x |
| `memcpy` | +copies | engine | 11.01 | 4.617 | +0.134 ms | **+5.60%** | +0.15% | 1.030x |

### total64MB (64 MiB), uniform, 1% touched/page, 83886 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.77 | 0.232 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.40 | 0.201 | -0.031 ms | -1.29% | -0.03% | 0.866x |
| `kernel-remap` | +copies | replay | 76.25 | 6.397 | +6.164 ms | **+256.84%** | **+6.85%** | 27.522x |
| `arc-make-mut` | barrier | replay | 4.26 | 0.357 | +0.125 ms | **+5.19%** | +0.14% | 1.536x |
| `arc-make-mut` | +copies | replay | 24.51 | 2.056 | +1.824 ms | **+75.98%** | +2.03% | 8.846x |
| `chunk-bitmap` | barrier | replay | 4.18 | 0.350 | +0.118 ms | **+4.91%** | +0.13% | 1.507x |
| `chunk-bitmap` | +copies | replay | 22.90 | 1.921 | +1.689 ms | **+70.36%** | +1.88% | 8.266x |
| `flat-undo-log` | barrier | replay | 3.18 | 0.267 | +0.034 ms | +1.43% | +0.04% | 1.148x |
| `flat-undo-log` | +copies | replay | 22.07 | 1.851 | +1.619 ms | **+67.45%** | +1.80% | 7.965x |
| `memcpy` | barrier | replay | 2.84 | 0.238 | +0.006 ms | +0.24% | +0.01% | 1.024x |
| `memcpy` | +copies | replay | 2.97 | 0.249 | +0.017 ms | +0.70% | +0.02% | 1.073x |
| `plain (no snapshot)` | — | engine | 11.71 | 0.983 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 11.96 | 1.004 | +0.021 ms | +0.88% | +0.02% | 1.022x |
| `kernel-remap` | +copies | engine | 88.25 | 7.403 | +6.420 ms | **+267.52%** | **+7.13%** | 7.535x |
| `arc-make-mut` | barrier | engine | 12.62 | 1.059 | +0.076 ms | **+3.18%** | +0.08% | 1.078x |
| `arc-make-mut` | +copies | engine | 30.52 | 2.560 | +1.577 ms | **+65.73%** | +1.75% | 2.605x |
| `chunk-bitmap` | barrier | engine | 12.10 | 1.015 | +0.033 ms | +1.36% | +0.04% | 1.033x |
| `chunk-bitmap` | +copies | engine | 29.47 | 2.472 | +1.489 ms | **+62.06%** | +1.65% | 2.516x |
| `flat-undo-log` | barrier | engine | 11.62 | 0.975 | -0.008 ms | -0.32% | -0.01% | 0.992x |
| `flat-undo-log` | +copies | engine | 28.82 | 2.417 | +1.435 ms | **+59.78%** | +1.59% | 2.460x |
| `memcpy` | barrier | engine | 11.59 | 0.972 | -0.010 ms | -0.43% | -0.01% | 0.989x |
| `memcpy` | +copies | engine | 11.70 | 0.982 | -0.001 ms | -0.04% | -0.00% | 0.999x |

### total64MB (64 MiB), uniform, 5% touched/page, 419430 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.87 | 1.205 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.48 | 1.041 | -0.164 ms | **-6.82%** | -0.18% | 0.864x |
| `kernel-remap` | +copies | replay | 22.96 | 9.632 | +8.427 ms | **+351.12%** | **+9.36%** | 7.995x |
| `arc-make-mut` | barrier | replay | 4.30 | 1.805 | +0.600 ms | **+25.02%** | +0.67% | 1.498x |
| `arc-make-mut` | +copies | replay | 10.38 | 4.352 | +3.148 ms | **+131.15%** | **+3.50%** | 3.613x |
| `chunk-bitmap` | barrier | replay | 3.99 | 1.675 | +0.471 ms | **+19.61%** | +0.52% | 1.391x |
| `chunk-bitmap` | +copies | replay | 10.00 | 4.196 | +2.992 ms | **+124.65%** | **+3.32%** | 3.483x |
| `flat-undo-log` | barrier | replay | 3.23 | 1.355 | +0.150 ms | **+6.25%** | +0.17% | 1.125x |
| `flat-undo-log` | +copies | replay | 9.27 | 3.889 | +2.685 ms | **+111.85%** | +2.98% | 3.228x |
| `memcpy` | barrier | replay | 2.86 | 1.198 | -0.007 ms | -0.29% | -0.01% | 0.994x |
| `memcpy` | +copies | replay | 2.98 | 1.250 | +0.045 ms | +1.89% | +0.05% | 1.038x |
| `plain (no snapshot)` | — | engine | 11.75 | 4.927 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 11.80 | 4.949 | +0.021 ms | +0.89% | +0.02% | 1.004x |
| `kernel-remap` | +copies | engine | 31.49 | 13.208 | +8.281 ms | **+345.04%** | **+9.20%** | 2.681x |
| `arc-make-mut` | barrier | engine | 11.94 | 5.010 | +0.082 ms | **+3.44%** | +0.09% | 1.017x |
| `arc-make-mut` | +copies | engine | 17.78 | 7.458 | +2.531 ms | **+105.45%** | +2.81% | 1.514x |
| `chunk-bitmap` | barrier | engine | 11.94 | 5.010 | +0.082 ms | **+3.43%** | +0.09% | 1.017x |
| `chunk-bitmap` | +copies | engine | 17.51 | 7.345 | +2.418 ms | **+100.74%** | +2.69% | 1.491x |
| `flat-undo-log` | barrier | engine | 11.62 | 4.875 | -0.053 ms | -2.19% | -0.06% | 0.989x |
| `flat-undo-log` | +copies | engine | 17.22 | 7.223 | +2.295 ms | **+95.64%** | +2.55% | 1.466x |
| `memcpy` | barrier | engine | 11.81 | 4.953 | +0.026 ms | +1.06% | +0.03% | 1.005x |
| `memcpy` | +copies | engine | 11.73 | 4.919 | -0.008 ms | -0.34% | -0.01% | 0.998x |

### total200MB (200 MiB), tex-freelist, 1% touched/page, 262144 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.20 | 0.578 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.99 | 0.521 | -0.057 ms | -2.36% | -0.06% | 0.902x |
| `kernel-remap` | +copies | replay | 10.54 | 2.764 | +2.186 ms | **+91.08%** | +2.43% | 4.782x |
| `arc-make-mut` | barrier | replay | 3.27 | 0.858 | +0.281 ms | **+11.69%** | +0.31% | 1.485x |
| `arc-make-mut` | +copies | replay | 4.04 | 1.059 | +0.481 ms | **+20.05%** | +0.53% | 1.832x |
| `chunk-bitmap` | barrier | replay | 2.83 | 0.741 | +0.163 ms | **+6.79%** | +0.18% | 1.282x |
| `chunk-bitmap` | +copies | replay | 3.86 | 1.011 | +0.433 ms | **+18.04%** | +0.48% | 1.749x |
| `flat-undo-log` | barrier | replay | 2.40 | 0.630 | +0.052 ms | +2.16% | +0.06% | 1.090x |
| `flat-undo-log` | +copies | replay | 3.18 | 0.834 | +0.256 ms | **+10.66%** | +0.28% | 1.443x |
| `memcpy` | barrier | replay | 2.10 | 0.550 | -0.028 ms | -1.15% | -0.03% | 0.952x |
| `memcpy` | +copies | replay | 2.31 | 0.605 | +0.027 ms | +1.14% | +0.03% | 1.048x |
| `plain (no snapshot)` | — | engine | 10.66 | 2.794 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.83 | 2.839 | +0.045 ms | +1.88% | +0.05% | 1.016x |
| `kernel-remap` | +copies | engine | 18.59 | 4.873 | +2.079 ms | **+86.64%** | +2.31% | 1.744x |
| `arc-make-mut` | barrier | engine | 10.84 | 2.841 | +0.047 ms | +1.97% | +0.05% | 1.017x |
| `arc-make-mut` | +copies | engine | 11.80 | 3.092 | +0.298 ms | **+12.44%** | +0.33% | 1.107x |
| `chunk-bitmap` | barrier | engine | 10.81 | 2.834 | +0.041 ms | +1.70% | +0.05% | 1.015x |
| `chunk-bitmap` | +copies | engine | 11.80 | 3.094 | +0.301 ms | **+12.53%** | +0.33% | 1.108x |
| `flat-undo-log` | barrier | engine | 10.76 | 2.820 | +0.026 ms | +1.09% | +0.03% | 1.009x |
| `flat-undo-log` | +copies | engine | 11.61 | 3.043 | +0.249 ms | **+10.38%** | +0.28% | 1.089x |
| `memcpy` | barrier | engine | 10.71 | 2.807 | +0.013 ms | +0.56% | +0.01% | 1.005x |
| `memcpy` | +copies | engine | 10.88 | 2.852 | +0.059 ms | +2.44% | +0.07% | 1.021x |

### total200MB (200 MiB), tex-freelist, 5% touched/page, 1310720 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.22 | 2.907 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 1.93 | 2.530 | -0.377 ms | **-15.69%** | -0.42% | 0.870x |
| `kernel-remap` | +copies | replay | 5.53 | 7.254 | +4.348 ms | **+181.15%** | **+4.83%** | 2.496x |
| `arc-make-mut` | barrier | replay | 3.19 | 4.175 | +1.268 ms | **+52.84%** | +1.41% | 1.436x |
| `arc-make-mut` | +copies | replay | 3.79 | 4.968 | +2.061 ms | **+85.88%** | +2.29% | 1.709x |
| `chunk-bitmap` | barrier | replay | 2.97 | 3.898 | +0.991 ms | **+41.30%** | +1.10% | 1.341x |
| `chunk-bitmap` | +copies | replay | 3.61 | 4.726 | +1.819 ms | **+75.80%** | +2.02% | 1.626x |
| `flat-undo-log` | barrier | replay | 2.40 | 3.151 | +0.244 ms | **+10.18%** | +0.27% | 1.084x |
| `flat-undo-log` | +copies | replay | 3.04 | 3.990 | +1.083 ms | **+45.12%** | +1.20% | 1.373x |
| `memcpy` | barrier | replay | 2.21 | 2.891 | -0.016 ms | -0.66% | -0.02% | 0.995x |
| `memcpy` | +copies | replay | 2.20 | 2.886 | -0.021 ms | -0.86% | -0.02% | 0.993x |
| `plain (no snapshot)` | — | engine | 10.65 | 13.964 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 10.74 | 14.075 | +0.111 ms | **+4.64%** | +0.12% | 1.008x |
| `kernel-remap` | +copies | engine | 14.30 | 18.746 | +4.782 ms | **+199.27%** | **+5.31%** | 1.342x |
| `arc-make-mut` | barrier | engine | 10.84 | 14.205 | +0.241 ms | **+10.03%** | +0.27% | 1.017x |
| `arc-make-mut` | +copies | engine | 11.46 | 15.020 | +1.056 ms | **+44.02%** | +1.17% | 1.076x |
| `chunk-bitmap` | barrier | engine | 10.93 | 14.329 | +0.365 ms | **+15.21%** | +0.41% | 1.026x |
| `chunk-bitmap` | +copies | engine | 11.58 | 15.180 | +1.216 ms | **+50.67%** | +1.35% | 1.087x |
| `flat-undo-log` | barrier | engine | 10.74 | 14.079 | +0.115 ms | **+4.79%** | +0.13% | 1.008x |
| `flat-undo-log` | +copies | engine | 11.17 | 14.647 | +0.683 ms | **+28.45%** | +0.76% | 1.049x |
| `memcpy` | barrier | engine | 10.63 | 13.931 | -0.033 ms | -1.36% | -0.04% | 0.998x |
| `memcpy` | +copies | engine | 10.79 | 14.146 | +0.182 ms | **+7.60%** | +0.20% | 1.013x |

### total200MB (200 MiB), uniform, 1% touched/page, 262144 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.93 | 0.767 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.56 | 0.670 | -0.096 ms | **-4.02%** | -0.11% | 0.874x |
| `kernel-remap` | +copies | replay | 72.57 | 19.024 | +18.257 ms | **+760.71%** | **+20.29%** | 24.808x |
| `arc-make-mut` | barrier | replay | 4.39 | 1.152 | +0.385 ms | **+16.04%** | +0.43% | 1.502x |
| `arc-make-mut` | +copies | replay | 23.67 | 6.206 | +5.439 ms | **+226.63%** | **+6.04%** | 8.093x |
| `chunk-bitmap` | barrier | replay | 4.11 | 1.078 | +0.311 ms | **+12.98%** | +0.35% | 1.406x |
| `chunk-bitmap` | +copies | replay | 23.09 | 6.054 | +5.287 ms | **+220.29%** | **+5.87%** | 7.895x |
| `flat-undo-log` | barrier | replay | 3.32 | 0.869 | +0.102 ms | **+4.26%** | +0.11% | 1.133x |
| `flat-undo-log` | +copies | replay | 21.51 | 5.639 | +4.872 ms | **+202.99%** | **+5.41%** | 7.353x |
| `memcpy` | barrier | replay | 2.92 | 0.766 | -0.001 ms | -0.05% | -0.00% | 0.998x |
| `memcpy` | +copies | replay | 2.98 | 0.781 | +0.014 ms | +0.58% | +0.02% | 1.018x |
| `plain (no snapshot)` | — | engine | 11.92 | 3.125 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 11.94 | 3.129 | +0.004 ms | +0.18% | +0.00% | 1.001x |
| `kernel-remap` | +copies | engine | 88.56 | 23.216 | +20.091 ms | **+837.14%** | **+22.32%** | 7.430x |
| `arc-make-mut` | barrier | engine | 12.51 | 3.278 | +0.153 ms | **+6.40%** | +0.17% | 1.049x |
| `arc-make-mut` | +copies | engine | 30.44 | 7.980 | +4.855 ms | **+202.28%** | **+5.39%** | 2.554x |
| `chunk-bitmap` | barrier | engine | 12.40 | 3.250 | +0.125 ms | **+5.22%** | +0.14% | 1.040x |
| `chunk-bitmap` | +copies | engine | 29.78 | 7.807 | +4.683 ms | **+195.11%** | **+5.20%** | 2.499x |
| `flat-undo-log` | barrier | engine | 12.02 | 3.152 | +0.027 ms | +1.14% | +0.03% | 1.009x |
| `flat-undo-log` | +copies | engine | 28.70 | 7.525 | +4.400 ms | **+183.33%** | **+4.89%** | 2.408x |
| `memcpy` | barrier | engine | 12.02 | 3.151 | +0.026 ms | +1.08% | +0.03% | 1.008x |
| `memcpy` | +copies | engine | 12.02 | 3.151 | +0.027 ms | +1.11% | +0.03% | 1.008x |

### total200MB (200 MiB), uniform, 5% touched/page, 1310720 ops/page
| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |
|---|---|---|---|---|---|---|---|---|
| `plain (no snapshot)` | — | replay | 2.99 | 3.915 | — | — | — | 1.000x |
| `kernel-remap` | barrier | replay | 2.56 | 3.352 | -0.564 ms | **-23.48%** | -0.63% | 0.856x |
| `kernel-remap` | +copies | replay | 25.34 | 33.216 | +29.301 ms | **+1220.88%** | **+32.56%** | 8.484x |
| `arc-make-mut` | barrier | replay | 4.51 | 5.915 | +2.000 ms | **+83.33%** | +2.22% | 1.511x |
| `arc-make-mut` | +copies | replay | 10.93 | 14.322 | +10.407 ms | **+433.61%** | **+11.56%** | 3.658x |
| `chunk-bitmap` | barrier | replay | 4.23 | 5.550 | +1.635 ms | **+68.10%** | +1.82% | 1.417x |
| `chunk-bitmap` | +copies | replay | 10.56 | 13.844 | +9.928 ms | **+413.68%** | **+11.03%** | 3.536x |
| `flat-undo-log` | barrier | replay | 3.35 | 4.394 | +0.479 ms | **+19.95%** | +0.53% | 1.122x |
| `flat-undo-log` | +copies | replay | 9.42 | 12.352 | +8.437 ms | **+351.53%** | **+9.37%** | 3.155x |
| `memcpy` | barrier | replay | 2.95 | 3.865 | -0.050 ms | -2.07% | -0.06% | 0.987x |
| `memcpy` | +copies | replay | 3.00 | 3.936 | +0.021 ms | +0.86% | +0.02% | 1.005x |
| `plain (no snapshot)` | — | engine | 11.91 | 15.607 | — | — | — | 1.000x |
| `kernel-remap` | barrier | engine | 12.00 | 15.731 | +0.124 ms | **+5.15%** | +0.14% | 1.008x |
| `kernel-remap` | +copies | engine | 33.73 | 44.208 | +28.601 ms | **+1191.70%** | **+31.78%** | 2.833x |
| `arc-make-mut` | barrier | engine | 12.51 | 16.393 | +0.786 ms | **+32.75%** | +0.87% | 1.050x |
| `arc-make-mut` | +copies | engine | 18.48 | 24.218 | +8.611 ms | **+358.79%** | **+9.57%** | 1.552x |
| `chunk-bitmap` | barrier | engine | 12.28 | 16.093 | +0.486 ms | **+20.25%** | +0.54% | 1.031x |
| `chunk-bitmap` | +copies | engine | 18.16 | 23.808 | +8.201 ms | **+341.70%** | **+9.11%** | 1.525x |
| `flat-undo-log` | barrier | engine | 11.87 | 15.564 | -0.043 ms | -1.78% | -0.05% | 0.997x |
| `flat-undo-log` | +copies | engine | 17.45 | 22.878 | +7.271 ms | **+302.95%** | **+8.08%** | 1.466x |
| `memcpy` | barrier | engine | 11.90 | 15.604 | -0.003 ms | -0.13% | -0.00% | 1.000x |
| `memcpy` | +copies | engine | 12.00 | 15.725 | +0.118 ms | **+4.93%** | +0.13% | 1.008x |

