# Parity scoreboard 2026-09-29: new engine vs v1 vs pdfTeX (packages)

Measured on **mac-m5pro-dq222**. Host-dependent data: it is not another host's baseline (DESIGN §8).

**Provenance:** measured with the tools/parity harness at **c16da57c4** (91 entries, oracle cache key v4, no pinned random seed). The new engine is flashtex-initex sha256 **f24eaf19a1e7**, built from origin/main 6160c284f in one cargo invocation together with flashtex-cli; `-p flashtex-engine` built on its own gives ac65cc162300 (see `_provenance` in the `.notes.json`). The oracle cache key v5, the pinned seed (`\pdfsetrandomseed 1`), the size-before-read log guard and pgfmath-qr-example's return (92 entries) **all came after this measurement**, and none of it has been re-measured here. This header was added by hand after `engines.py` wrote the file.

| engine | version | git SHA | engine sha256 | host | TeX Live | \write18 |
|---|---|---|---|---|---|---|
| new | pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine) | 6160c284f | f24eaf19a1e7 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | -shell-restricted |
| v1 | flashtex 0.1.0 (6160c284f) | 6160c284f | 6af7a9b4f4ec | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | -shell-restricted |
| pdflatex | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | — | 3ead7baeffb8 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | -shell-restricted |

## packages

| engine | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|---|---|---|---|
| new | 91 | 0 | 82/83 (98.8%) | 90/91 (98.9%) | 90/91 (98.9%) | 90/91 (98.9%) | 90/91 (98.9%) | 90/91 (98.9%) | 90/91 (98.9%) |
| v1 | 91 | 0 | n/a | 0/91 (0.0%) | 11/91 (12.1%) | 9/91 (9.9%) | 6/91 (6.6%) | 3/91 (3.3%) | 1/91 (1.1%) |
| pdflatex | 91 | 0 | 83/83 (100.0%) | 91/91 (100.0%) | 91/91 (100.0%) | 91/91 (100.0%) | 91/91 (100.0%) | 91/91 (100.0%) | 91/91 (100.0%) |

## Root causes (new): every document that is not P-T1 + P-T2 + L4

- (a) package or font missing from the user's TeX Live (§4.4 bundle fallback): **0**
- (b) engine difference: **1**
- (c) harness issue (tools/parity): **0**
- (d) pdflatex fails too (excluded): **0**
- (e) excluded by the convergence rule: pdflatex compiles it but keeps asking for a rerun: **0**

| tier | document | new level | P-T1 | P-T2 | class | cause | § | owner | issue |
|---|---|---|---|---|---|---|---|---|---|
| packages | fontenc-encguide | below L0 | fail | fail | b | font wnr10 has no pdftex.map entry: the engine aborts at shipout 20 (`bitmap (PK/Type 3) fonts are not implemented yet`) where pdfTeX embeds a PK/Type 3 font | writet3 (Type 3 from PK), via pdf_init_font | kabir-claude (engine P3 output) | #1218 |

## P-T1 not evaluated (new): 8 documents

They are in no P-T1 denominator. Their P-T2 and levels are measured as usual.

- packages/appendix-tikzpeople: not evaluated: oracle: the traced pass did not finish in the capture's 600 s limit
- packages/bm-beautynote: not evaluated: the oracle's traced log is 1083 MiB, above --pt1-max-log-mb 1024
- packages/chemfig-chemfig-en: not evaluated: oracle: the traced pass did not finish in the capture's 600 s limit
- packages/fancyvrb-verbatim-content: not evaluated: the ltx-talk class draws \int_rand (\pdfuniformdeviate), which pdfTeX seeds from the clock, so pdfTeX's own \tracingall log differs between runs
- packages/forest-milsymb: not evaluated: the oracle's traced log is 7239 MiB, above --pt1-max-log-mb 1024
- packages/lscape-cahierprof-exemple: not evaluated: the oracle's traced log is 9780 MiB, above --pt1-max-log-mb 1024
- packages/mhchem-mhchem: not evaluated: the oracle's traced log is 7168 MiB, above --pt1-max-log-mb 1024
- packages/tabu-europasscv: not evaluated: tabu times its X-column trial typesetting with \pdfelapsedtime, which \tracingall logs, so pdfTeX's own traced log differs between runs
