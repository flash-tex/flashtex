# Parity scoreboard 2026-09-30: new engine vs v1 vs pdfTeX (packages)

Measured on **mac-m5pro-dq222**. Host-dependent data: it is not another host's baseline (DESIGN §8).

Measured with the tools/parity harness at **1cf4f4e39**.

| engine | version | git SHA | engine sha256 | host | TeX Live | \write18 |
|---|---|---|---|---|---|---|
| new | pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine) | 9713f29b3 | 484ad943e6a6 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | default |
| v1 | flashtex 0.1.0 (9713f29b3) | 9713f29b3 | 41106a01a647 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | default |
| pdflatex | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | — | 3ead7baeffb8 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | default |

## packages

| engine | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|---|---|---|---|
| new | 92 | 0 | 84/84 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) |
| v1 | 92 | 0 | n/a | 0/92 (0.0%) | 11/92 (12.0%) | 9/92 (9.8%) | 6/92 (6.5%) | 3/92 (3.3%) | 1/92 (1.1%) |
| pdflatex | 92 | 0 | 84/84 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) | 92/92 (100.0%) |

## Root causes (new): every document that is not P-T1 + P-T2 + L4

- (a) package or font missing from the user's TeX Live (§4.4 bundle fallback): **0**
- (b) engine difference: **0**
- (c) harness issue (tools/parity): **0**
- (d) pdflatex fails too (excluded): **0**
- (e) excluded by the convergence rule: pdflatex compiles it but keeps asking for a rerun: **0**

| tier | document | new level | P-T1 | P-T2 | class | cause | § | owner | issue |
|---|---|---|---|---|---|---|---|---|---|

## P-T1 not evaluated (new): 8 documents

They are in no P-T1 denominator. Their P-T2 and levels are measured as usual.

- packages/appendix-tikzpeople: not evaluated: the oracle's traced log is 12533 MiB, above --pt1-max-log-mb 1024
- packages/bm-beautynote: not evaluated: the oracle's traced log is 1083 MiB, above --pt1-max-log-mb 1024
- packages/chemfig-chemfig-en: not evaluated: oracle: the traced pass did not finish in the capture's 600 s limit
- packages/forest-milsymb: not evaluated: the oracle's traced log is 7239 MiB, above --pt1-max-log-mb 1024
- packages/lscape-cahierprof-exemple: not evaluated: the oracle's traced log is 9780 MiB, above --pt1-max-log-mb 1024
- packages/mhchem-mhchem: not evaluated: the oracle's traced log is 7168 MiB, above --pt1-max-log-mb 1024
- packages/pgfmath-qr-example: not evaluated: the oracle's traced log is 4605 MiB, above --pt1-max-log-mb 1024
- packages/tabu-europasscv: not evaluated: tabu times its X-column trial typesetting with \pdfelapsedtime, which \tracingall logs, so pdfTeX's own traced log differs between runs
