| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 22 (22/48 / 0/18) | 0.025 / 0.100 / 0.11 | 2 / 22 | 10.1 |
| full-100 | 66 | 38 (36/48 / 2/18) | 0.030 / 0.316 / 0.34 | 2 / 111 | 5.5 |
| full-300 | 66 | 50 (40/48 / 10/18) | 0.077 / 0.792 / 0.83 | 10 / 285 | 4.4 |
| full-1000 | 66 | 50 (38/48 / 12/18) | 0.133 / 2.715 / 2.80 | 10 / 920 | 6.2 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.016 / 0.043 / 0.05 | 2 / 10 | 2.9 |
| plain-100 | 66 | 27 (18/48 / 9/18) | 0.032 / 0.096 / 0.10 | 5 / 95 | 2.8 |
| plain-300 | 66 | 54 (36/48 / 18/18) | 0.030 / 0.140 / 0.27 | 5 / 294 | 2.6 |
| plain-1000 | 66 | 56 (40/48 / 16/18) | 0.037 / 0.868 / 1.30 | 3 / 1552 | 3.6 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 28 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 8 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 8 | `page N: N differs outside what the structural comparison reads: intr_data[N]: N -> N` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| plain | 4 | `page N: N differs outside what the structural comparison reads: page_so_far[N]: N -> N` |
| plain | 4 | `page N: structures differ: glue spec orders or reference count differ (walking Some((Glue, N, N)))` |
