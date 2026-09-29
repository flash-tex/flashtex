# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 4.35
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: kernel copy fault after `mach_vm_remap(copy=TRUE)`
| state | pages stored to | no snapshot | snapshot outstanding | copy fault per page |
|---|---|---|---|---|
| mem768k | 230 | 1.04 µs (min 958 ns, p90 1.08 µs) | 381.50 µs (min 288.92 µs, p90 469.50 µs) | **1654 ns/page** (min-based 1252 ns/page) |
| mem5M | 747 | 3.38 µs (min 3.00 µs, p90 3.42 µs) | 1.131 ms (min 1.020 ms, p90 1.240 ms) | **1509 ns/page** (min-based 1362 ns/page) |
| total64MB | 1024 | 4.58 µs (min 4.25 µs, p90 4.71 µs) | 1.623 ms (min 1.475 ms, p90 1.778 ms) | **1581 ns/page** (min-based 1436 ns/page) |
| total200MB | 3200 | 14.75 µs (min 14.25 µs, p90 15.00 µs) | 5.526 ms (min 5.104 ms, p90 5.816 ms) | **1722 ns/page** (min-based 1590 ns/page) |

