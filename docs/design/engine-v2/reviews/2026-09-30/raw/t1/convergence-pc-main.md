| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 16 (16/48 / 0/18) | 0.043 / 0.211 / 0.23 | 3 / 22 | 16.6 |
| full-100 | 66 | 10 (10/48 / 0/18) | 0.438 / 0.926 / 0.99 | 49 / 111 | 13.7 |
| full-300 | 50 | 6 (6/32 / 0/18) | 1.274 / 3.111 / 3.96 | 147 / 292 | 18.2 |
| full-1000 | 50 | 12 (12/32 / 0/18) | 5.538 / 13.376 / 13.56 | 493 / 920 | 34.5 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.042 / 0.126 / 0.15 | 2 / 10 | 11.8 |
| plain-100 | 66 | 25 (18/48 / 7/18) | 0.087 / 0.316 / 0.34 | 5 / 95 | 14.3 |
| plain-300 | 66 | 52 (36/48 / 16/18) | 0.106 / 0.469 / 0.96 | 5 / 294 | 8.9 |
| plain-1000 | 66 | 52 (40/48 / 12/18) | 0.124 / 3.107 / 4.68 | 3 / 1552 | 44.0 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 66 | `page N: structures differ: whatsit N word N differs` |
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 26 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 10 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| plain | 6 | `page N: N differs outside what the structural comparison reads: max_buf_stack: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| full | 2 | `page N: N differs outside what the structural comparison reads: hc[N]: N -> N` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: best_height_plus_depth: N -> N` |
