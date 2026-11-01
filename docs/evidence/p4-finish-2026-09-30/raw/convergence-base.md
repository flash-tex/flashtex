| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 14 (14/48 / 0/18) | 0.025 / 0.102 / 0.11 | 3 / 22 | 9.4 |
| full-100 | 66 | 10 (10/48 / 0/18) | 0.161 / 0.350 / 0.36 | 49 / 111 | 5.4 |
| full-300 | 66 | 8 (8/48 / 0/18) | 0.444 / 0.900 / 1.00 | 147 / 292 | 6.0 |
| full-1000 | 66 | 22 (22/48 / 0/18) | 0.220 / 3.217 / 3.83 | 64 / 920 | 8.1 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.016 / 0.042 / 0.04 | 2 / 10 | 4.1 |
| plain-100 | 66 | 25 (18/48 / 7/18) | 0.030 / 0.092 / 0.10 | 5 / 95 | 5.3 |
| plain-300 | 66 | 52 (36/48 / 16/18) | 0.036 / 0.142 / 0.28 | 5 / 294 | 2.3 |
| plain-1000 | 66 | 52 (40/48 / 12/18) | 0.039 / 0.866 / 1.35 | 3 / 1552 | 9.5 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 80 | `page N: structures differ: whatsit N word N differs` |
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 28 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 8 | `page N: N differs outside what the structural comparison reads: intr_state[N]: N -> N` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| plain | 6 | `page N: N differs outside what the structural comparison reads: max_buf_stack: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| full | 4 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 2 | `page N: N differs outside what the structural comparison reads: hc[N]: N -> N` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: best_height_plus_depth: N -> N` |
