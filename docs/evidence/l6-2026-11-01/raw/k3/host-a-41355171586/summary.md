Host: nixos (Linux x86_64), counters: perf, reps 1, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | cycles | Δ | user CPU | Δ | PDF |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| plain-100 | e0-origin_main | def | 2.274 G | +0.00 % | 0.843 G | +0.00 % | 0.183 s | +0.00 % | same |
| plain-100 | e0-origin_main | off | 2.274 G | +0.00 % | 0.846 G | +0.33 % | 0.188 s | +2.79 % | same |
| plain-100 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 2.272 G | -0.08 % | 0.850 G | +0.83 % | 0.185 s | +0.82 % | same |
| plain-100 | e1-in_agent_kabir-claude-l6_bm-first-write | off | 2.272 G | -0.08 % | 0.858 G | +1.85 % | 0.182 s | -0.38 % | same |

PDF identity: every engine and variant gave the same bytes on every document.

First opens through the socket host (coldopen.py), the engine thread's instructions over every pass, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | DONE | passes | failed |
|---|---|---|---:|---:|---:|---:|---:|
| full-100 | e0-origin_main | def | 20.468 G | +0.00 % | 2.206 s | 2.0 | 0 |
| full-100 | e0-origin_main | off | 18.692 G | -8.68 % | 2.428 s | 2.0 | 0 |
| full-100 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 18.996 G | -7.19 % | 1.931 s | 2.0 | 0 |
| full-100 | e1-in_agent_kabir-claude-l6_bm-first-write | off | 18.617 G | -9.04 % | 1.931 s | 2.0 | 0 |
| full-300 | e0-origin_main | def | 71.504 G | +0.00 % | 7.250 s | 3.0 | 0 |
| full-300 | e0-origin_main | off | 72.708 G | +1.69 % | 7.323 s | 3.0 | 0 |
| full-300 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 68.868 G | -3.69 % | 6.914 s | 3.0 | 0 |
| full-300 | e1-in_agent_kabir-claude-l6_bm-first-write | off | 72.707 G | +1.68 % | 7.480 s | 3.0 | 0 |
| beamer-30 | e0-origin_main | def | 42.555 G | +0.00 % | 5.410 s | 2.0 | 0 |
| beamer-30 | e0-origin_main | off | 41.230 G | -3.12 % | 4.047 s | 2.0 | 0 |
| beamer-30 | e1-in_agent_kabir-claude-l6_bm-first-write | def | 42.053 G | -1.18 % | 4.227 s | 2.0 | 0 |
| beamer-30 | e1-in_agent_kabir-claude-l6_bm-first-write | off | 41.269 G | -3.02 % | 3.510 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e0-origin_main | def | 6.987 G | +0.00 % | 0.645 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e0-origin_main | off | 6.908 G | -1.12 % | 0.741 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e1-in_agent_kabir-claude-l6_bm-first-write | def | 6.984 G | -0.04 % | 0.661 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e1-in_agent_kabir-claude-l6_bm-first-write | off | 6.899 G | -1.25 % | 0.636 s | 2.0 | 0 |
| fx:beamer-madrid | e0-origin_main | def | 10.442 G | +0.00 % | 0.919 s | 2.0 | 0 |
| fx:beamer-madrid | e0-origin_main | off | 10.218 G | -2.15 % | 0.985 s | 2.0 | 0 |
| fx:beamer-madrid | e1-in_agent_kabir-claude-l6_bm-first-write | def | 10.492 G | +0.48 % | 1.033 s | 2.0 | 0 |
| fx:beamer-madrid | e1-in_agent_kabir-claude-l6_bm-first-write | off | 10.223 G | -2.10 % | 1.037 s | 2.0 | 0 |
| infdesc | e0-origin_main | def | 795.649 G | +0.00 % | 87.145 s | 3.0 | 0 |
| infdesc | e0-origin_main | off | 783.968 G | -1.47 % | 78.985 s | 3.0 | 0 |
| infdesc | e1-in_agent_kabir-claude-l6_bm-first-write | def | 795.433 G | -0.03 % | 78.700 s | 3.0 | 0 |
| infdesc | e1-in_agent_kabir-claude-l6_bm-first-write | off | 784.423 G | -1.41 % | 73.738 s | 3.0 | 0 |
