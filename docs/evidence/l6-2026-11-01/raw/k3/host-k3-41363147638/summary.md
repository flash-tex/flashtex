Host: nixos (Linux x86_64), counters: perf, reps 1, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | cycles | Δ | user CPU | Δ | PDF |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| plain-100 | e0-origin_main | def | 2.274 G | +0.00 % | 0.876 G | +0.00 % | 0.192 s | +0.00 % | same |
| plain-100 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 3.299 G | +45.04 % | 1.152 G | +31.47 % | 0.255 s | +32.79 % | same |
| plain-100 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 2.272 G | -0.08 % | 0.846 G | -3.42 % | 0.181 s | -5.42 % | same |

PDF identity: every engine and variant gave the same bytes on every document.

First opens through the socket host (coldopen.py), the engine thread's instructions over every pass, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | DONE | passes | failed |
|---|---|---|---:|---:|---:|---:|---:|
| full-100 | e0-origin_main | def | 20.401 G | +0.00 % | 1.894 s | 2.0 | 0 |
| full-100 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 18.727 G | -8.20 % | 1.722 s | 2.0 | 0 |
| full-100 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 19.002 G | -6.86 % | 1.760 s | 2.0 | 0 |
| full-300 | e0-origin_main | def | 71.425 G | +0.00 % | 6.571 s | 3.0 | 0 |
| full-300 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 68.465 G | -4.14 % | 6.288 s | 3.0 | 0 |
| full-300 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 68.793 G | -3.68 % | 6.255 s | 3.0 | 0 |
| beamer-30 | e0-origin_main | def | 42.552 G | +0.00 % | 3.524 s | 2.0 | 0 |
| beamer-30 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 41.954 G | -1.41 % | 3.471 s | 2.0 | 0 |
| beamer-30 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 42.053 G | -1.17 % | 3.952 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e0-origin_main | def | 6.989 G | +0.00 % | 0.754 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e1-in_agent_kabir-claude-l6_bm-first-write | def | 6.991 G | +0.03 % | 0.733 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 6.982 G | -0.09 % | 0.732 s | 2.0 | 0 |
| fx:beamer-madrid | e0-origin_main | def | 10.445 G | +0.00 % | 1.053 s | 2.0 | 0 |
| fx:beamer-madrid | e1-in_agent_kabir-claude-l6_bm-first-write | def | 10.463 G | +0.18 % | 1.047 s | 2.0 | 0 |
| fx:beamer-madrid | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 10.493 G | +0.46 % | 1.055 s | 2.0 | 0 |
| fx:beamer-polish | e0-origin_main | def | 9.362 G | +0.00 % | 0.970 s | 2.0 | 0 |
| fx:beamer-polish | e1-in_agent_kabir-claude-l6_bm-first-write | def | 9.366 G | +0.04 % | 0.984 s | 2.0 | 0 |
| fx:beamer-polish | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 9.358 G | -0.04 % | 0.945 s | 2.0 | 0 |
| fx:framed-tikz-theorems | e0-origin_main | def | 5.025 G | +0.00 % | 0.532 s | 2.0 | 0 |
| fx:framed-tikz-theorems | e1-in_agent_kabir-claude-l6_bm-first-write | def | 5.026 G | +0.02 % | 0.543 s | 2.0 | 0 |
| fx:framed-tikz-theorems | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 5.028 G | +0.07 % | 0.544 s | 2.0 | 0 |
| infdesc | e0-origin_main | def | 795.406 G | +0.00 % | 81.290 s | 3.0 | 0 |
| infdesc | e1-in_agent_kabir-claude-l6_bm-first-write | def | 792.594 G | -0.35 % | 68.152 s | 3.0 | 0 |
| infdesc | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 792.967 G | -0.31 % | 64.202 s | 3.0 | 0 |
