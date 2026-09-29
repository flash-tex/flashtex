# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 4.35
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: snapshot bookkeeping, `Arc` table vs side reference counts
| state | chunks | `Arc` table clone | id-table `memcpy` | + side refcount pass |
|---|---|---|---|---|
| mem768k | 921 | 10.25 µs (min 9.83 µs, p90 10.54 µs) | 42 ns (min 41 ns, p90 42 ns) | 292 ns (min 250 ns, p90 334 ns) |
| mem5M | 2988 | 99.46 µs (min 86.17 µs, p90 114.29 µs) | 166 ns (min 125 ns, p90 167 ns) | 875 ns (min 833 ns, p90 958 ns) |
| total64MB | 4096 | 175.42 µs (min 139.75 µs, p90 184.00 µs) | 167 ns (min 166 ns, p90 209 ns) | 1.17 µs (min 1.12 µs, p90 1.25 µs) |
| total200MB | 12800 | 666.67 µs (min 523.71 µs, p90 949.46 µs) | 750 ns (min 708 ns, p90 791 ns) | 3.96 µs (min 3.83 µs, p90 4.04 µs) |

