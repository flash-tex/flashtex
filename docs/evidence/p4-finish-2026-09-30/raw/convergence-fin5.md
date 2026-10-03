| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 23 (23/48 / 0/18) | 0.026 / 0.103 / 0.12 | 2 / 22 | 9.6 |
| full-100 | 66 | 44 (42/48 / 2/18) | 0.035 / 0.330 / 0.45 | 2 / 111 | 6.3 |
| full-300 | 66 | 54 (44/48 / 10/18) | 0.076 / 0.836 / 0.91 | 9 / 285 | 5.2 |
| full-1000 | 66 | 55 (43/48 / 12/18) | 0.127 / 2.688 / 4.53 | 10 / 920 | 7.0 |
| plain-10 | 66 | 35 (31/48 / 4/18) | 0.016 / 0.040 / 0.04 | 2 / 10 | 2.7 |
| plain-100 | 66 | 32 (23/48 / 9/18) | 0.030 / 0.088 / 0.09 | 5 / 95 | 2.6 |
| plain-300 | 66 | 60 (42/48 / 18/18) | 0.021 / 0.082 / 0.27 | 2 / 294 | 2.3 |
| plain-1000 | 66 | 60 (44/48 / 16/18) | 0.038 / 0.460 / 1.40 | 3 / 1552 | 3.8 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| plain | 18 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 15 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 15 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 14 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 9 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 8 | `page N: N differs outside what the structural comparison reads: intr_data[N]: N -> N` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| plain | 4 | `page N: N differs outside what the structural comparison reads: page_so_far[N]: N -> N` |
| plain | 4 | `page N: structures differ: glue spec orders or reference count differ (walking Some((Glue, N, N)))` |
