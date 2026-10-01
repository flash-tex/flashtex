| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 14 (14/48 / 0/18) | 0.026 / 0.102 / 0.11 | 3 / 22 | 9.3 |
| full-100 | 66 | 26 (26/48 / 0/18) | 0.029 / 0.321 / 0.41 | 4 / 111 | 5.5 |
| full-300 | 66 | 10 (10/48 / 0/18) | 0.453 / 0.915 / 0.93 | 147 / 292 | 5.6 |
| full-1000 | 66 | 26 (26/48 / 0/18) | 0.177 / 2.845 / 3.27 | 25 / 920 | 7.1 |
| plain-10 | 66 | 36 (32/48 / 4/18) | 0.016 / 0.045 / 0.05 | 2 / 10 | 3.1 |
| plain-100 | 66 | 37 (28/48 / 9/18) | 0.023 / 0.081 / 0.10 | 4 / 95 | 2.7 |
| plain-300 | 66 | 66 (48/48 / 18/18) | 0.022 / 0.043 / 0.08 | 2 / 65 | 2.4 |
| plain-1000 | 66 | 64 (48/48 / 16/18) | 0.037 / 0.202 / 1.34 | 3 / 1552 | 3.4 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 69 | `page N: structures differ: action word N differs (N vs N) (walking Some((Action, N, N)))` |
| full | 16 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| plain | 15 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 15 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 11 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 8 | `page N: N differs outside what the structural comparison reads: intr_pre[N]: N -> N` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| plain | 4 | `page N: N differs outside what the structural comparison reads: page_so_far[N]: N -> N` |
| plain | 4 | `page N: structures differ: glue spec orders or reference count differ (walking Some((Glue, N, N)))` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
