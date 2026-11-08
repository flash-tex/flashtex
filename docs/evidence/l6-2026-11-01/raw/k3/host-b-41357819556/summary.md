Host: nixos (Linux x86_64), counters: perf, reps 1, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | cycles | Δ | user CPU | Δ | PDF |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| plain-100 | e0-origin_main | def | 2.274 G | +0.00 % | 0.870 G | +0.00 % | 0.187 s | +0.00 % | same |
| plain-100 | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 2.275 G | +0.01 % | 0.845 G | -2.83 % | 0.182 s | -2.30 % | same |

PDF identity: every engine and variant gave the same bytes on every document.

First opens through the socket host (coldopen.py), the engine thread's instructions over every pass, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | DONE | passes | failed |
|---|---|---|---:|---:|---:|---:|---:|
| full-100 | e0-origin_main | def | 20.398 G | +0.00 % | 1.706 s | 2.0 | 0 |
| full-100 | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 18.770 G | -7.98 % | 1.565 s | 2.0 | 0 |
| full-300 | e0-origin_main | def | 71.380 G | +0.00 % | 5.933 s | 3.0 | 0 |
| full-300 | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 68.531 G | -3.99 % | 5.807 s | 3.0 | 0 |
| beamer-30 | e0-origin_main | def | 42.550 G | +0.00 % | 3.562 s | 2.0 | 0 |
| beamer-30 | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 42.156 G | -0.93 % | 3.443 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e0-origin_main | def | 6.986 G | +0.00 % | 0.648 s | 2.0 | 0 |
| fx:beamer-blocks-columns | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 7.002 G | +0.22 % | 0.652 s | 2.0 | 0 |
| fx:beamer-madrid | e0-origin_main | def | 10.442 G | +0.00 % | 0.927 s | 2.0 | 0 |
| fx:beamer-madrid | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 10.502 G | +0.57 % | 0.904 s | 2.0 | 0 |
| fx:beamer-polish | e0-origin_main | def | 9.358 G | +0.00 % | 0.838 s | 2.0 | 0 |
| fx:beamer-polish | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 9.383 G | +0.26 % | 0.829 s | 2.0 | 0 |
| fx:framed-tikz-theorems | e0-origin_main | def | 5.026 G | +0.00 % | 0.487 s | 2.0 | 0 |
| fx:framed-tikz-theorems | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 5.043 G | +0.33 % | 0.478 s | 2.0 | 0 |
| infdesc | e0-origin_main | def | 793.726 G | +0.00 % | 72.463 s | 3.0 | 0 |
| infdesc | e1-_agent_kabir-claude-l6_bm-first-write-b | def | 796.934 G | +0.40 % | 68.367 s | 3.0 | 0 |
