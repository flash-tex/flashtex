# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 4.20
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: snapshot / restore cost (median of 50+)
| state | mechanism | snapshot | restore | note |
|---|---|---|---|---|
| mem768k (14 MiB) | `kernel-remap` | 16.25 µs (min 13.54 µs, p90 19.88 µs) | 32.54 µs (min 29.00 µs, p90 37.46 µs) | cur_prot 0x3, max_prot 0x7 |
| mem768k (14 MiB) | `kernel-vm_copy` | 35.25 µs (min 21.08 µs, p90 49.62 µs) | — | dest region pre-allocated and reused |
| mem768k (14 MiB) | `arc-make-mut` | 10.46 µs (min 10.12 µs, p90 10.96 µs) | 23.17 µs (min 21.75 µs, p90 24.42 µs) | snapshot = chunk-table clone (one atomic increment per chunk) |
| mem768k (14 MiB) | `chunk-bitmap` | 10.29 µs (min 10.04 µs, p90 11.29 µs) | 22.88 µs (min 21.50 µs, p90 23.75 µs) | snapshot = chunk-table clone + bitmap clear |
| mem768k (14 MiB) | `flat-undo-log` | 1.29 µs (min 0 ns, p90 1.42 µs) | 34.04 µs (min 29.38 µs, p90 38.83 µs) | snapshot = drop the log + clear the bitmap; restore = replay one log |
| mem768k (14 MiB) | `memcpy` | 334.46 µs (min 295.67 µs, p90 381.96 µs) | 231.88 µs (min 217.17 µs, p90 268.62 µs) | Vec::clone: alloc + memcpy |
| mem768k (14 MiB) | `memcpy-pooled` | 218.54 µs (min 205.58 µs, p90 254.67 µs) | — | copy_from_slice into a reused buffer |
| mem5M (47 MiB) | `kernel-remap` | 59.21 µs (min 49.00 µs, p90 71.79 µs) | 110.33 µs (min 90.00 µs, p90 142.92 µs) | cur_prot 0x3, max_prot 0x7 |
| mem5M (47 MiB) | `kernel-vm_copy` | 57.33 µs (min 45.50 µs, p90 82.79 µs) | — | dest region pre-allocated and reused |
| mem5M (47 MiB) | `arc-make-mut` | 107.00 µs (min 96.67 µs, p90 143.17 µs) | 202.54 µs (min 183.25 µs, p90 237.42 µs) | snapshot = chunk-table clone (one atomic increment per chunk) |
| mem5M (47 MiB) | `chunk-bitmap` | 110.88 µs (min 99.67 µs, p90 130.62 µs) | 171.62 µs (min 149.00 µs, p90 205.42 µs) | snapshot = chunk-table clone + bitmap clear |
| mem5M (47 MiB) | `flat-undo-log` | 2.08 µs (min 42 ns, p90 2.42 µs) | 62.08 µs (min 53.58 µs, p90 75.92 µs) | snapshot = drop the log + clear the bitmap; restore = replay one log |
| mem5M (47 MiB) | `memcpy` | 922.58 µs (min 859.25 µs, p90 972.17 µs) | 765.83 µs (min 742.58 µs, p90 830.96 µs) | Vec::clone: alloc + memcpy |
| mem5M (47 MiB) | `memcpy-pooled` | 772.46 µs (min 737.62 µs, p90 876.58 µs) | — | copy_from_slice into a reused buffer |
| total64MB (64 MiB) | `kernel-remap` | 77.71 µs (min 70.79 µs, p90 104.88 µs) | 140.08 µs (min 124.79 µs, p90 169.17 µs) | cur_prot 0x3, max_prot 0x7 |
| total64MB (64 MiB) | `kernel-vm_copy` | 62.04 µs (min 42.42 µs, p90 88.88 µs) | — | dest region pre-allocated and reused |
| total64MB (64 MiB) | `arc-make-mut` | 183.25 µs (min 163.00 µs, p90 239.71 µs) | 310.04 µs (min 255.08 µs, p90 352.75 µs) | snapshot = chunk-table clone (one atomic increment per chunk) |
| total64MB (64 MiB) | `chunk-bitmap` | 188.92 µs (min 165.21 µs, p90 246.83 µs) | 278.08 µs (min 250.79 µs, p90 333.96 µs) | snapshot = chunk-table clone + bitmap clear |
| total64MB (64 MiB) | `flat-undo-log` | 2.29 µs (min 42 ns, p90 2.71 µs) | 72.54 µs (min 59.29 µs, p90 84.88 µs) | snapshot = drop the log + clear the bitmap; restore = replay one log |
| total64MB (64 MiB) | `memcpy` | 1.240 ms (min 1.166 ms, p90 1.429 ms) | 1.066 ms (min 1.023 ms, p90 1.163 ms) | Vec::clone: alloc + memcpy |
| total64MB (64 MiB) | `memcpy-pooled` | 1.068 ms (min 1.018 ms, p90 1.148 ms) | — | copy_from_slice into a reused buffer |
| total200MB (200 MiB) | `kernel-remap` | 236.67 µs (min 207.21 µs, p90 283.42 µs) | 485.83 µs (min 416.08 µs, p90 558.33 µs) | cur_prot 0x3, max_prot 0x7 |
| total200MB (200 MiB) | `kernel-vm_copy` | 125.71 µs (min 97.83 µs, p90 208.58 µs) | — | dest region pre-allocated and reused |
| total200MB (200 MiB) | `arc-make-mut` | 1.247 ms (min 754.04 µs, p90 1.355 ms) | 1.612 ms (min 1.432 ms, p90 1.807 ms) | snapshot = chunk-table clone (one atomic increment per chunk) |
| total200MB (200 MiB) | `chunk-bitmap` | 1.211 ms (min 648.17 µs, p90 1.385 ms) | 1.576 ms (min 1.057 ms, p90 1.761 ms) | snapshot = chunk-table clone + bitmap clear |
| total200MB (200 MiB) | `flat-undo-log` | 7.54 µs (min 0 ns, p90 9.33 µs) | 166.38 µs (min 127.50 µs, p90 185.25 µs) | snapshot = drop the log + clear the bitmap; restore = replay one log |
| total200MB (200 MiB) | `memcpy` | 3.584 ms (min 3.409 ms, p90 3.727 ms) | 3.384 ms (min 3.235 ms, p90 3.584 ms) | Vec::clone: alloc + memcpy |
| total200MB (200 MiB) | `memcpy-pooled` | 3.462 ms (min 3.342 ms, p90 3.611 ms) | — | copy_from_slice into a reused buffer |

