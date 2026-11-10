Host: nixos (Linux x86_64), counters: perf, reps 5, medians; Δ against the first engine and first variant.

| document | engine | variant | instructions | Δ | cycles | Δ | user CPU | Δ | PDF |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| plain-100 | e0-origin_main | base | 2.274 G | +0.00 % | 0.888 G | +0.00 % | 0.189 s | +0.00 % | same |
| plain-100 | e0-origin_main | csxoff | 2.274 G | +0.00 % | 0.854 G | -3.77 % | 0.188 s | -0.85 % | same |
| plain-100 | e1-origin_agent_kabir-claude-l6_csname | base | 2.256 G | -0.81 % | 0.853 G | -3.89 % | 0.193 s | +2.06 % | same |
| plain-100 | e1-origin_agent_kabir-claude-l6_csname | csxoff | 2.275 G | +0.04 % | 0.866 G | -2.43 % | 0.190 s | +0.26 % | same |
| full-100 | e0-origin_main | base | 8.682 G | +0.00 % | 3.136 G | +0.00 % | 0.661 s | +0.00 % | same |
| full-100 | e0-origin_main | csxoff | 8.682 G | +0.00 % | 3.131 G | -0.16 % | 0.668 s | +1.07 % | same |
| full-100 | e1-origin_agent_kabir-claude-l6_csname | base | 8.614 G | -0.78 % | 3.052 G | -2.69 % | 0.647 s | -2.09 % | same |
| full-100 | e1-origin_agent_kabir-claude-l6_csname | csxoff | 8.689 G | +0.08 % | 3.097 G | -1.22 % | 0.662 s | +0.17 % | same |
| beamer-30 | e0-origin_main | base | 21.102 G | +0.00 % | 7.120 G | +0.00 % | 1.501 s | +0.00 % | same |
| beamer-30 | e0-origin_main | csxoff | 21.102 G | -0.00 % | 7.081 G | -0.55 % | 1.501 s | +0.03 % | same |
| beamer-30 | e1-origin_agent_kabir-claude-l6_csname | base | 20.906 G | -0.93 % | 6.847 G | -3.84 % | 1.449 s | -3.43 % | same |
| beamer-30 | e1-origin_agent_kabir-claude-l6_csname | csxoff | 21.111 G | +0.05 % | 7.030 G | -1.26 % | 1.497 s | -0.25 % | same |
| fx:beamer-madrid | e0-origin_main | base | 6.648 G | +0.00 % | 2.252 G | +0.00 % | 0.483 s | +0.00 % | same |
| fx:beamer-madrid | e0-origin_main | csxoff | 6.648 G | -0.00 % | 2.166 G | -3.85 % | 0.466 s | -3.52 % | same |
| fx:beamer-madrid | e1-origin_agent_kabir-claude-l6_csname | base | 6.592 G | -0.84 % | 2.149 G | -4.58 % | 0.456 s | -5.63 % | same |
| fx:beamer-madrid | e1-origin_agent_kabir-claude-l6_csname | csxoff | 6.651 G | +0.05 % | 2.235 G | -0.78 % | 0.470 s | -2.73 % | same |
| fx:framed-tikz-theorems | e0-origin_main | base | 3.221 G | +0.00 % | 1.073 G | +0.00 % | 0.234 s | +0.00 % | same |
| fx:framed-tikz-theorems | e0-origin_main | csxoff | 3.221 G | +0.00 % | 1.088 G | +1.41 % | 0.237 s | +0.94 % | same |
| fx:framed-tikz-theorems | e1-origin_agent_kabir-claude-l6_csname | base | 3.188 G | -1.02 % | 1.057 G | -1.49 % | 0.230 s | -1.92 % | same |
| fx:framed-tikz-theorems | e1-origin_agent_kabir-claude-l6_csname | csxoff | 3.223 G | +0.06 % | 1.097 G | +2.21 % | 0.236 s | +0.90 % | same |
| infdesc | e0-origin_main | base | 233.185 G | +0.00 % | 89.971 G | +0.00 % | 18.732 s | +0.00 % | same |
| infdesc | e0-origin_main | csxoff | 233.185 G | -0.00 % | 88.959 G | -1.12 % | 18.725 s | -0.04 % | same |
| infdesc | e1-origin_agent_kabir-claude-l6_csname | base | 230.171 G | -1.29 % | 85.226 G | -5.27 % | 17.951 s | -4.17 % | same |
| infdesc | e1-origin_agent_kabir-claude-l6_csname | csxoff | 233.318 G | +0.06 % | 89.139 G | -0.92 % | 18.775 s | +0.23 % | same |

PDF identity: every engine and variant gave the same bytes on every document.
