# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 5.65
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: 1000 checkpoints, log-spaced retention (dense 16, 4 per octave)
Retention keeps **40 of 1000** checkpoints: [0, 128, 256, 384, 512, 576]... and the newest 16.
`measured` rows are real: the software mechanisms through a counting global
allocator, the kernel mechanism through the host's free-memory drop, because its
         pages are invisible to the task's own ledger. `modelled`
rows would have needed more than 5.00 GiB live at once, so they are the measured
dirty-chunk rate times the retained count instead.

| state | locality | touched/page | mechanism | retained | total memory | per checkpoint | how |
|---|---|---|---|---|---|---|---|
| mem768k | tex-freelist | 1% | `chunk-bitmap` | 40 | 146.1 MiB | 3.7 MiB | measured (counting allocator, heap peak 164.6 MiB); chunk-version accounting 137.9 MiB; 105 barrier slow paths/page |
| mem768k | tex-freelist | 1% | `arc-make-mut` | 40 | 146.1 MiB | 3.7 MiB | measured (counting allocator) |
| mem768k | tex-freelist | 1% | `kernel-remap` | 40 | 183.5 MiB | 4.6 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem768k | tex-freelist | 1% | `memcpy` | 40 | 575.6 MiB | 14.4 MiB | analytic: retained x full state |
| mem768k | tex-freelist | 5% | `chunk-bitmap` | 40 | 187.7 MiB | 4.7 MiB | measured (counting allocator, heap peak 207.6 MiB); chunk-version accounting 178.2 MiB; 165 barrier slow paths/page |
| mem768k | tex-freelist | 5% | `arc-make-mut` | 40 | 187.7 MiB | 4.7 MiB | measured (counting allocator) |
| mem768k | tex-freelist | 5% | `kernel-remap` | 40 | 189.8 MiB | 4.7 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem768k | tex-freelist | 5% | `memcpy` | 40 | 575.6 MiB | 14.4 MiB | analytic: retained x full state |
| mem768k | uniform | 1% | `chunk-bitmap` | 40 | 246.3 MiB | 6.2 MiB | measured (counting allocator, heap peak 266.8 MiB); chunk-version accounting 251.2 MiB; 367 barrier slow paths/page |
| mem768k | uniform | 1% | `arc-make-mut` | 40 | 246.3 MiB | 6.2 MiB | measured (counting allocator) |
| mem768k | uniform | 1% | `kernel-remap` | 40 | 409.5 MiB | 10.2 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem768k | uniform | 1% | `memcpy` | 40 | 575.6 MiB | 14.4 MiB | analytic: retained x full state |
| mem768k | uniform | 5% | `chunk-bitmap` | 40 | 259.5 MiB | 6.5 MiB | measured (counting allocator, heap peak 281.3 MiB); chunk-version accounting 269.4 MiB; 407 barrier slow paths/page |
| mem768k | uniform | 5% | `arc-make-mut` | 40 | 259.5 MiB | 6.5 MiB | measured (counting allocator) |
| mem768k | uniform | 5% | `kernel-remap` | 40 | 366.9 MiB | 9.2 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem768k | uniform | 5% | `memcpy` | 40 | 575.6 MiB | 14.4 MiB | analytic: retained x full state |
| mem5M | tex-freelist | 1% | `chunk-bitmap` | 40 | 502.2 MiB | 12.6 MiB | measured (counting allocator, heap peak 558.6 MiB); chunk-version accounting 414.8 MiB; 156 barrier slow paths/page |
| mem5M | tex-freelist | 1% | `arc-make-mut` | 40 | 502.2 MiB | 12.6 MiB | measured (counting allocator) |
| mem5M | tex-freelist | 1% | `kernel-remap` | 40 | 494.4 MiB | 12.4 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem5M | tex-freelist | 1% | `memcpy` | 40 | 1.82 GiB | 46.7 MiB | analytic: retained x full state |
| mem5M | tex-freelist | 5% | `chunk-bitmap` | 40 | 836.6 MiB | 20.9 MiB | measured (counting allocator, heap peak 908.0 MiB); chunk-version accounting 728.6 MiB; 354 barrier slow paths/page |
| mem5M | tex-freelist | 5% | `arc-make-mut` | 40 | 836.6 MiB | 20.9 MiB | measured (counting allocator) |
| mem5M | tex-freelist | 5% | `kernel-remap` | 40 | 922.9 MiB | 23.1 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem5M | tex-freelist | 5% | `memcpy` | 40 | 1.82 GiB | 46.7 MiB | analytic: retained x full state |
| mem5M | uniform | 1% | `chunk-bitmap` | 40 | 1.32 GiB | 33.7 MiB | measured (counting allocator, heap peak 1.39 GiB); chunk-version accounting 1.23 GiB; 1668 barrier slow paths/page |
| mem5M | uniform | 1% | `arc-make-mut` | 40 | 1.32 GiB | 33.7 MiB | measured (counting allocator) |
| mem5M | uniform | 1% | `kernel-remap` | 40 | 1.33 GiB | 33.9 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem5M | uniform | 1% | `memcpy` | 40 | 1.82 GiB | 46.7 MiB | analytic: retained x full state |
| mem5M | uniform | 5% | `chunk-bitmap` | 40 | 1.52 GiB | 39.0 MiB | measured (counting allocator, heap peak 1.61 GiB); chunk-version accounting 1.56 GiB; 2465 barrier slow paths/page |
| mem5M | uniform | 5% | `arc-make-mut` | 40 | 1.52 GiB | 39.0 MiB | measured (counting allocator) |
| mem5M | uniform | 5% | `kernel-remap` | 40 | 1.77 GiB | 45.4 MiB | measured (host free-memory drop; see the ledger caveat) |
| mem5M | uniform | 5% | `memcpy` | 40 | 1.82 GiB | 46.7 MiB | analytic: retained x full state |
| total64MB | tex-freelist | 1% | `chunk-bitmap` | 40 | 680.7 MiB | 17.0 MiB | measured (counting allocator, heap peak 1.61 GiB); chunk-version accounting 549.3 MiB; 177 barrier slow paths/page |
| total64MB | tex-freelist | 1% | `arc-make-mut` | 40 | 680.6 MiB | 17.0 MiB | measured (counting allocator) |
| total64MB | tex-freelist | 1% | `kernel-remap` | 40 | 676.0 MiB | 16.9 MiB | measured (host free-memory drop; see the ledger caveat) |
| total64MB | tex-freelist | 1% | `memcpy` | 40 | 2.50 GiB | 64.0 MiB | analytic: retained x full state |
| total64MB | tex-freelist | 5% | `chunk-bitmap` | 40 | 1.15 GiB | 29.4 MiB | measured (counting allocator, heap peak 1.61 GiB); chunk-version accounting 1017.6 MiB; 451 barrier slow paths/page |
| total64MB | tex-freelist | 5% | `arc-make-mut` | 40 | 1.15 GiB | 29.4 MiB | measured (counting allocator) |
| total64MB | tex-freelist | 5% | `kernel-remap` | 40 | 1.24 GiB | 31.8 MiB | measured (host free-memory drop; see the ledger caveat) |
| total64MB | tex-freelist | 5% | `memcpy` | 40 | 2.50 GiB | 64.0 MiB | analytic: retained x full state |
| total64MB | uniform | 1% | `chunk-bitmap` | 40 | 1.89 GiB | 48.3 MiB | measured (counting allocator, heap peak 1.99 GiB); chunk-version accounting 1.75 GiB; 2334 barrier slow paths/page |
| total64MB | uniform | 1% | `arc-make-mut` | 40 | 1.89 GiB | 48.3 MiB | measured (counting allocator) |
| total64MB | uniform | 1% | `kernel-remap` | 40 | 1.89 GiB | 48.5 MiB | measured (host free-memory drop; see the ledger caveat) |
| total64MB | uniform | 1% | `memcpy` | 40 | 2.50 GiB | 64.0 MiB | analytic: retained x full state |
| total64MB | uniform | 5% | `chunk-bitmap` | 40 | 2.20 GiB | 56.4 MiB | measured (counting allocator, heap peak 2.32 GiB); chunk-version accounting 2.25 GiB; 3564 barrier slow paths/page |
| total64MB | uniform | 5% | `arc-make-mut` | 40 | 2.20 GiB | 56.4 MiB | measured (counting allocator) |
| total64MB | uniform | 5% | `kernel-remap` | 40 | 2.39 GiB | 61.1 MiB | measured (host free-memory drop; see the ledger caveat) |
| total64MB | uniform | 5% | `memcpy` | 40 | 2.50 GiB | 64.0 MiB | analytic: retained x full state |
| total200MB | tex-freelist | 1% | `chunk-bitmap` | 40 | 2.00 GiB | 51.3 MiB | measured (counting allocator, heap peak 2.32 GiB); chunk-version accounting 1.54 GiB; 335 barrier slow paths/page |
| total200MB | tex-freelist | 1% | `arc-make-mut` | 40 | 2.00 GiB | 51.3 MiB | measured (counting allocator) |
| total200MB | tex-freelist | 1% | `kernel-remap` | 40 | 1.87 GiB | 47.9 MiB | measured (host free-memory drop; see the ledger caveat) |
| total200MB | tex-freelist | 1% | `memcpy` | 40 | 7.81 GiB | 200.0 MiB | analytic: retained x full state |
| total200MB | tex-freelist | 5% | `chunk-bitmap` | 40 | 3.69 GiB | 94.6 MiB | measured (counting allocator, heap peak 4.00 GiB); chunk-version accounting 3.16 GiB; 1209 barrier slow paths/page |
| total200MB | tex-freelist | 5% | `arc-make-mut` | 40 | 3.69 GiB | 94.6 MiB | measured (counting allocator) |
| total200MB | tex-freelist | 5% | `kernel-remap` | 40 | 3.34 GiB | 85.5 MiB | measured (host free-memory drop; see the ledger caveat) |
| total200MB | tex-freelist | 5% | `memcpy` | 40 | 7.81 GiB | 200.0 MiB | analytic: retained x full state |
| total200MB | uniform | 1% | `chunk-bitmap` | 40 | 6.34 GiB | 162.4 MiB | measured (counting allocator, heap peak 6.66 GiB); chunk-version accounting 5.84 GiB; 7526 barrier slow paths/page |
| total200MB | uniform | 1% | `arc-make-mut` | 40 | 6.34 GiB | 162.4 MiB | measured (counting allocator) |
| total200MB | uniform | 1% | `kernel-remap` | 40 | 0 B | 0 B | measured (host free-memory drop; see the ledger caveat) |
| total200MB | uniform | 1% | `memcpy` | 40 | 7.81 GiB | 200.0 MiB | analytic: retained x full state |
| total200MB | uniform | 5% | `chunk-bitmap` | 40 | 7.43 GiB | 190.3 MiB | modelled: 12181 dirty chunks/page x 40 retained |
| total200MB | uniform | 5% | `arc-make-mut` | 40 | 7.43 GiB | 190.3 MiB | modelled: 12181 dirty chunks/page x 40 retained |
| total200MB | uniform | 5% | `kernel-remap` | 40 | 7.43 GiB | 190.3 MiB | modelled: 12181 dirty chunks/page x 40 retained |
| total200MB | uniform | 5% | `memcpy` | 40 | 7.81 GiB | 200.0 MiB | analytic: retained x full state |

