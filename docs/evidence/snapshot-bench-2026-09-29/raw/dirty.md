# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2
host page size 16384 B; software chunk 16384 B (2048 words); reps 50; pages 60; 1-minute load average at start 4.35
  mem768k: 14.4 MiB total, 921 chunks of 16 KB; mem 768528 words (5.9 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  mem5M: 46.7 MiB total, 2988 chunks of 16 KB; mem 5001744 words (38.2 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total64MB: 64.0 MiB total, 4096 chunks of 16 KB; mem 7270928 words (55.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000
  total200MB: 200.0 MiB total, 12800 chunks of 16 KB; mem 25096720 words (191.5 MiB), eqtb 60000, hash 65536, save_stack 80000, str_pool 262144, font_info 650000

## Phase: 16 KB chunks dirtied per page, and memory per checkpoint
Both copy-on-write mechanisms copy at 16 KB on this host, so one table covers
both: memory per checkpoint is the dirty-chunk count times 16 KB either way.
Chunks, not words, are what a checkpoint costs, so the gap between the two
columns is the whole question.

### Budget: % of state words *touched* (read or written) per page

| state | locality | touched/page | distinct words written | chunks dirty | % of state | memory per checkpoint |
|---|---|---|---|---|---|---|
| mem768k | tex-freelist | 1% | 4403 (0.23% of state) | 105 / 921 | 11.4% | 1.6 MiB |
| mem768k | tex-freelist | 5% | 16934 (0.90% of state) | 166 / 921 | 18.0% | 2.6 MiB |
| mem768k | uniform | 1% | 4603 (0.24% of state) | 368 / 921 | 39.9% | 5.7 MiB |
| mem768k | uniform | 5% | 22949 (1.22% of state) | 407 / 921 | 44.2% | 6.4 MiB |
| mem5M | tex-freelist | 1% | 12153 (0.20% of state) | 158 / 2988 | 5.3% | 2.5 MiB |
| mem5M | tex-freelist | 5% | 35303 (0.58% of state) | 356 / 2988 | 11.9% | 5.6 MiB |
| mem5M | uniform | 1% | 14988 (0.24% of state) | 1667 / 2988 | 55.8% | 26.0 MiB |
| mem5M | uniform | 5% | 74364 (1.22% of state) | 2463 / 2988 | 82.4% | 38.5 MiB |
| total64MB | tex-freelist | 1% | 15574 (0.19% of state) | 176 / 4096 | 4.3% | 2.8 MiB |
| total64MB | tex-freelist | 5% | 41994 (0.50% of state) | 454 / 4096 | 11.1% | 7.1 MiB |
| total64MB | uniform | 1% | 20628 (0.25% of state) | 2332 / 4096 | 56.9% | 36.4 MiB |
| total64MB | uniform | 5% | 101643 (1.21% of state) | 3565 / 4096 | 87.0% | 55.7 MiB |
| total200MB | tex-freelist | 1% | 32477 (0.12% of state) | 339 / 12800 | 2.6% | 5.3 MiB |
| total200MB | tex-freelist | 5% | 83148 (0.32% of state) | 1213 / 12800 | 9.5% | 18.9 MiB |
| total200MB | uniform | 1% | 64032 (0.24% of state) | 7496 / 12800 | 58.6% | 117.1 MiB |
| total200MB | uniform | 5% | 315312 (1.20% of state) | 12181 / 12800 | 95.2% | 190.3 MiB |

### Budget: % of state words *written* per page, calibrated

This is the row the §5.2 question asks for: memory per checkpoint at 1% and 5%
**dirty** per page. Every `mem` access is made a write and the op budget is then
searched until the *distinct* words written land on the target, because a page
rewrites many words more than once. The achieved fraction is shown, not the
target, and a row is marked `(capped)` where the arena cannot be dirtied that
far — `hash` and `font_info` are never written mid-page.

| state | locality | dirty target | distinct words written | chunks dirty | % of state | memory per checkpoint |
|---|---|---|---|---|---|---|
| mem768k | tex-freelist | 1% (32065 ops/page) | 18895 (1.00% of state) | 178 / 921 | 19.4% | 2.8 MiB |
| mem768k | tex-freelist | 5% (490414 ops/page) | 94189 (4.99% of state) | 406 / 921 | 44.1% | 6.3 MiB |
| mem768k | uniform | 1% (22634 ops/page) | 20324 (1.08% of state) | 407 / 921 | 44.2% | 6.4 MiB |
| mem768k | uniform | 5% (111286 ops/page) | 95018 (5.04% of state) | 407 / 921 | 44.2% | 6.4 MiB |
| mem5M | tex-freelist | 1% (250896 ops/page) | 62465 (1.02% of state) | 835 / 2988 | 27.9% | 13.0 MiB |
| mem5M | tex-freelist | 5% (2447769 ops/page) | 207810 (3.40% of state) | 2416 / 2988 | 80.8% | 37.7 MiB |
| mem5M | uniform | 1% (73433 ops/page) | 66211 (1.08% of state) | 2461 / 2988 | 82.4% | 38.5 MiB |
| mem5M | uniform | 5% (348807 ops/page) | 306957 (5.02% of state) | 2475 / 2988 | 82.8% | 38.7 MiB |
| total64MB | tex-freelist | 1% (402653 ops/page) | 84051 (1.00% of state) | 1240 / 4096 | 30.3% | 19.4 MiB |
| total64MB | tex-freelist | 5% (3355443 ops/page) | 235010 (2.80% of state) | 3466 / 4096 | 84.6% | 54.2 MiB |
| total64MB | uniform | 1% (100663 ops/page) | 90842 (1.08% of state) | 3560 / 4096 | 86.9% | 55.6 MiB |
| total64MB | uniform | 5% (478150 ops/page) | 421148 (5.02% of state) | 3584 / 4096 | 87.5% | 56.0 MiB |
| total200MB | tex-freelist | 1% (4377804 ops/page) | 262789 (1.00% of state) | 8842 / 12800 | 69.1% | 138.2 MiB |
| total200MB | tex-freelist | 5% (10485760 ops/page) | 416980 (1.59% of state) | 11735 / 12800 | 91.7% | 183.4 MiB |
| total200MB | uniform | 1% (314572 ops/page) | 283788 (1.08% of state) | 12158 / 12800 | 95.0% | 190.0 MiB |
| total200MB | uniform | 5% (1494220 ops/page) | 1316464 (5.02% of state) | 12291 / 12800 | 96.0% | 192.1 MiB |

### Budget: % of state words *written* per page, uncalibrated

The same shape with the op budget set straight to the target, for reference: the
achieved figure is below the target because a
page rewrites some words more than once.

| state | locality | dirty target | distinct words written | chunks dirty | % of state | memory per checkpoint |
|---|---|---|---|---|---|---|
| mem768k | tex-freelist | 1% | 13116 (0.70% of state) | 147 / 921 | 15.9% | 2.3 MiB |
| mem768k | tex-freelist | 5% | 35114 (1.86% of state) | 284 / 921 | 30.8% | 4.4 MiB |
| mem768k | uniform | 1% | 16963 (0.90% of state) | 407 / 921 | 44.2% | 6.4 MiB |
| mem768k | uniform | 5% | 81186 (4.30% of state) | 407 / 921 | 44.2% | 6.4 MiB |
| mem5M | tex-freelist | 1% | 27794 (0.45% of state) | 318 / 2988 | 10.6% | 5.0 MiB |
| mem5M | tex-freelist | 5% | 70735 (1.16% of state) | 945 / 2988 | 31.6% | 14.8 MiB |
| mem5M | uniform | 1% | 55276 (0.90% of state) | 2446 / 2988 | 81.9% | 38.2 MiB |
| mem5M | uniform | 5% | 270253 (4.42% of state) | 2475 / 2988 | 82.8% | 38.7 MiB |
| total64MB | tex-freelist | 1% | 33182 (0.40% of state) | 384 / 4096 | 9.4% | 6.0 MiB |
| total64MB | tex-freelist | 5% | 86284 (1.03% of state) | 1280 / 4096 | 31.2% | 20.0 MiB |
| total64MB | uniform | 1% | 75874 (0.90% of state) | 3524 / 4096 | 86.0% | 55.1 MiB |
| total64MB | uniform | 5% | 370852 (4.42% of state) | 3583 / 4096 | 87.5% | 56.0 MiB |
| total200MB | tex-freelist | 1% | 64464 (0.25% of state) | 997 / 12800 | 7.8% | 15.6 MiB |
| total200MB | tex-freelist | 5% | 162221 (0.62% of state) | 3943 / 12800 | 30.8% | 61.6 MiB |
| total200MB | uniform | 1% | 236608 (0.90% of state) | 11997 / 12800 | 93.7% | 187.5 MiB |
| total200MB | uniform | 5% | 1158752 (4.42% of state) | 12290 / 12800 | 96.0% | 192.0 MiB |

### Sensitivity to `mem` write scatter (total64MB, 1% touched/page)

| uniform-write fraction of `mem` writes | chunks dirty | % of state | memory per checkpoint |
|---|---|---|---|
| 0.0% | 105 | 2.6% | 1.6 MiB |
| 2.0% | 177 | 4.3% | 2.8 MiB |
| 5.0% | 285 | 7.0% | 4.5 MiB |
| 10.0% | 456 | 11.1% | 7.1 MiB |
| 20.0% | 757 | 18.5% | 11.8 MiB |
| 50.0% | 1524 | 37.2% | 23.8 MiB |
| 100.0% | 2345 | 57.3% | 36.6 MiB |

