| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 14 (14/48 / 0/18) | 0.031 / 0.156 / 0.18 | 3 / 22 | 12.5 |
| full-100 | 66 | 10 (10/48 / 0/18) | 0.296 / 0.630 / 0.66 | 49 / 111 | 10.4 |
| full-300 | 66 | 8 (8/48 / 0/18) | 0.876 / 1.896 / 1.91 | 147 / 292 | 9.5 |
| full-1000 | 66 | 22 (22/48 / 0/18) | 0.389 / 5.681 / 5.88 | 64 / 920 | 14.1 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.019 / 0.050 / 0.05 | 2 / 10 | 5.0 |
| plain-100 | 66 | 25 (18/48 / 7/18) | 0.034 / 0.130 / 0.13 | 5 / 95 | 6.3 |
| plain-300 | 66 | 52 (36/48 / 16/18) | 0.041 / 0.202 / 0.39 | 5 / 294 | 3.1 |
| plain-1000 | 66 | 52 (40/48 / 12/18) | 0.040 / 1.242 / 1.92 | 3 / 1552 | 10.5 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 84 | `page N: structures differ: whatsit N word N differs` |
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 28 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 8 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| plain | 6 | `page N: N differs outside what the structural comparison reads: max_buf_stack: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| full | 2 | `page N: N differs outside what the structural comparison reads: hc[N]: N -> N` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: best_height_plus_depth: N -> N` |
