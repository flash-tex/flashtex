| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 24 (24/48 / 0/18) | 0.025 / 0.105 / 0.11 | 2 / 22 | 10.1 |
| full-100 | 66 | 50 (48/48 / 2/18) | 0.032 / 0.314 / 0.35 | 2 / 111 | 5.6 |
| full-300 | 66 | 58 (48/48 / 10/18) | 0.078 / 0.795 / 0.82 | 9 / 285 | 4.6 |
| full-1000 | 66 | 60 (48/48 / 12/18) | 0.119 / 1.407 / 2.83 | 10 / 911 | 6.4 |
| plain-10 | 66 | 36 (32/48 / 4/18) | 0.016 / 0.041 / 0.06 | 2 / 10 | 3.1 |
| plain-100 | 66 | 37 (28/48 / 9/18) | 0.024 / 0.078 / 0.09 | 4 / 95 | 2.7 |
| plain-300 | 66 | 66 (48/48 / 18/18) | 0.021 / 0.042 / 0.09 | 2 / 65 | 2.3 |
| plain-1000 | 66 | 64 (48/48 / 16/18) | 0.036 / 0.199 / 1.30 | 3 / 1552 | 3.4 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| plain | 15 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 15 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 9 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 8 | `page N: N differs outside what the structural comparison reads: intr_data[N]: N -> N` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| plain | 4 | `page N: N differs outside what the structural comparison reads: page_so_far[N]: N -> N` |
| plain | 4 | `page N: structures differ: glue spec orders or reference count differ (walking Some((Glue, N, N)))` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 1 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
