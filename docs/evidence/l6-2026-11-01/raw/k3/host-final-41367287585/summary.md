Host: nixos (Linux x86_64), counters: perf, reps 1, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | cycles | Δ | user CPU | Δ | PDF |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| plain-100 | e0-origin_main | def | 2.274 G | +0.00 % | 0.879 G | +0.00 % | 0.184 s | +0.00 % | same |
| plain-100 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 3.299 G | +45.04 % | 1.207 G | +37.32 % | 0.257 s | +39.96 % | same |
| plain-100 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 3.297 G | +44.95 % | 1.151 G | +30.95 % | 0.255 s | +38.65 % | same |

PDF identity: every engine and variant gave the same bytes on every document.

First opens through the socket host (coldopen.py), the engine thread's instructions over every pass, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | DONE | passes | failed |
|---|---|---|---:|---:|---:|---:|---:|
| full-100 | e0-origin_main | def | 20.400 G | +0.00 % | 1.730 s | 2 | 0 |
| full-100 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 18.702 G | -8.32 % | 1.596 s | 2 | 0 |
| full-100 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 19.006 G | -6.83 % | 1.605 s | 2 | 0 |
| full-300 | e0-origin_main | def | 71.381 G | +0.00 % | 5.966 s | 3 | 0 |
| full-300 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 68.393 G | -4.19 % | 5.788 s | 3 | 0 |
| full-300 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 68.770 G | -3.66 % | 5.754 s | 3 | 0 |
| beamer-30 | e0-origin_main | def | 42.552 G | +0.00 % | 3.246 s | 2 | 0 |
| beamer-30 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 41.947 G | -1.42 % | 3.197 s | 2 | 0 |
| beamer-30 | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 42.046 G | -1.19 % | 3.221 s | 2 | 0 |
| fx:beamer-blocks-columns | e0-origin_main | def | 6.987 G | +0.00 % | 0.603 s | 2 | 0 |
| fx:beamer-blocks-columns | e1-in_agent_kabir-claude-l6_bm-first-write | def | 6.987 G | +0.00 % | 0.607 s | 2 | 0 |
| fx:beamer-blocks-columns | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 6.983 G | -0.06 % | 0.598 s | 2 | 0 |
| fx:beamer-madrid | e0-origin_main | def | 10.442 G | +0.00 % | 0.851 s | 2 | 0 |
| fx:beamer-madrid | e1-in_agent_kabir-claude-l6_bm-first-write | def | 10.453 G | +0.11 % | 0.861 s | 2 | 0 |
| fx:beamer-madrid | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 10.487 G | +0.43 % | 0.846 s | 2 | 0 |
| fx:beamer-polish | e0-origin_main | def | 9.358 G | +0.00 % | 0.759 s | 2 | 0 |
| fx:beamer-polish | e1-in_agent_kabir-claude-l6_bm-first-write | def | 9.359 G | +0.01 % | 0.770 s | 2 | 0 |
| fx:beamer-polish | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 9.356 G | -0.02 % | 0.766 s | 2 | 0 |
| fx:framed-tikz-theorems | e0-origin_main | def | 5.025 G | +0.00 % | 0.447 s | 2 | 0 |
| fx:framed-tikz-theorems | e1-in_agent_kabir-claude-l6_bm-first-write | def | 5.025 G | +0.00 % | 0.448 s | 2 | 0 |
| fx:framed-tikz-theorems | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 5.028 G | +0.06 % | 0.441 s | 2 | 0 |
| infdesc | e0-origin_main | def | 792.350 G | +0.00 % | 65.870 s | 3 | 0 |
| infdesc | e1-in_agent_kabir-claude-l6_bm-first-write | def | 792.355 G | +0.00 % | 65.706 s | 3 | 0 |
| infdesc | e2-origin_agent_kabir-claude-l6_bm-inplace | def | 793.934 G | +0.20 % | 66.179 s | 3 | 0 |
