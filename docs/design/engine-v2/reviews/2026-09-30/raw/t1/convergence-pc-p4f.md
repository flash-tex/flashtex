| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-1000 | 66 | 18 (18/48 / 0/18) | 4.350 / 8.281 / 8.53 | 493 / 920 | 24.6 |
| plain-1000 | 66 | 54 (40/48 / 14/18) | 0.076 / 2.029 / 3.73 | 3 / 1552 | 30.6 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 10 | `page N: N differs outside what the structural comparison reads: intr_state[N]: N -> N` |
| full | 8 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 8 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 6 | `page N: structures differ: whatsit N word N differs` |
