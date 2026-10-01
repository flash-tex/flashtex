| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 14 (14/48 / 0/18) | 0.025 / 0.101 / 0.11 | 3 / 22 | 9.4 |
| full-100 | 66 | 10 (10/48 / 0/18) | 0.159 / 0.326 / 0.41 | 49 / 111 | 5.2 |
| full-300 | 66 | 8 (8/48 / 0/18) | 0.441 / 0.880 / 0.90 | 147 / 292 | 4.6 |
| full-1000 | 66 | 22 (22/48 / 0/18) | 0.218 / 2.932 / 3.43 | 64 / 920 | 6.8 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.016 / 0.041 / 0.04 | 2 / 10 | 2.8 |
| plain-100 | 66 | 27 (18/48 / 9/18) | 0.033 / 0.091 / 0.09 | 5 / 95 | 2.8 |
| plain-300 | 66 | 54 (36/48 / 18/18) | 0.031 / 0.139 / 0.27 | 5 / 294 | 2.4 |
| plain-1000 | 66 | 54 (40/48 / 14/18) | 0.038 / 0.846 / 1.31 | 3 / 1552 | 3.5 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 76 | `page N: structures differ: whatsit N word N differs` |
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 28 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 8 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 8 | `page N: N differs outside what the structural comparison reads: intr_state[N]: N -> N` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| plain | 4 | `page N: N differs outside what the structural comparison reads: best_height_plus_depth: N -> N` |
| plain | 4 | `page N: structures differ: glue spec orders or reference count differ (walking Some((Glue, N, N)))` |
| full | 2 | `page N: N differs outside what the structural comparison reads: hc[N]: N -> N` |
