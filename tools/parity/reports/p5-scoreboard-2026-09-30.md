# P5 scoreboard: new engine vs v1, pdflatex as the oracle

**Provenance.** A first local sample, not a gate result. Both engines were built from main 296c90197 (`flashtex-initex` with its own pdflatex.fmt/pdftex.fmt; v1 = `flashtex`). The harnesses were main's `parity.py` (fixtures, arxiv, templates), #1288's `parity.py` at 0478786c4 (packages) and #1276's `nightly.py` at 60ce5a81 (nightly-5k, `--spread 6`, `--pt1-sample nightly-5k=1`), plus main's latex-suites, package-smoke and font-census, with pdfTeX 1.40.29 (TeX Live 2026, MacTeX) as the oracle. Parity ran at `-j 2` with `--raster none`, so L4 was not measured. T2's baseline was the same 20 tests through pdfTeX on this host.

DESIGN §12 P5 gate: new engine >= old on every tier; arXiv L1 >= 90%; retirement complete. Expected data is only the oracle's (pdfTeX 1.40.29 / pdflatex).

Measured on **mac-m5pro-dq222**. Host-dependent data: not another host's baseline (DESIGN §8).

| engine | version | git SHA | host | sources |
|---|---|---|---|---|
| new | pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine) | 296c90197 | DN0a1f24ee.SUNet | fonts, latex-suites, nightly, package-smoke, parity |
| old | flashtex 0.1.0 (296c901972a8) | 296c90197 | DN0a1f24ee.SUNet | nightly, parity |

**Not all green** (partial or sampled runs). Retirement from S5 on does not start.

Sample: local sample on mac-m5pro-dq222: first 12 documents of fixtures/arxiv/templates/packages, 6 of nightly-5k (--spread), the first 20 latex2e/base tests, all 59 package-smoke documents, the font census at its default 6 per family

| tier | metric | new | old | verdict | target | gates |
|---|---|---|---|---|---|---|
| T3 fixtures | P-T1 | 12/12 (100.0%) [partial] | n/a | ahead (old n/a) | new >= old | S5+ |
| T3 fixtures | P-T2 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old | S3, S5+ |
| T3 fixtures | L0 | 12/12 (100.0%) [partial] | 12/12 (100.0%) [partial] | equal | new >= old | S5+ |
| T3 fixtures | L1 | 12/12 (100.0%) [partial] | 12/12 (100.0%) [partial] | equal | new >= old | S5+ |
| T3 fixtures | L2 | 12/12 (100.0%) [partial] | 7/12 (58.3%) [partial] | ahead | new >= old | S5+ |
| T3 fixtures | L3 | 12/12 (100.0%) [partial] | 5/12 (41.7%) [partial] | ahead | new >= old | S5+ |
| T3 arXiv | P-T1 | 11/11 (100.0%) [skipped 1, partial] | n/a | ahead (old n/a) | new >= old | S5+ |
| T3 arXiv | P-T2 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 arXiv | L0 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 arXiv | L1 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old; >= 90% | S5+ |
| T3 arXiv | L2 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 arXiv | L3 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 templates | P-T1 | 11/11 (100.0%) [partial] | n/a | ahead (old n/a) | new >= old | S5+ |
| T3 templates | P-T2 | 11/11 (100.0%) [partial] | 0/11 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 templates | L0 | 11/11 (100.0%) [partial] | 2/11 (18.2%) [partial] | ahead | new >= old | S5+ |
| T3 templates | L1 | 11/11 (100.0%) [partial] | 2/11 (18.2%) [partial] | ahead | new >= old | S5+ |
| T3 templates | L2 | 11/11 (100.0%) [partial] | 0/11 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 templates | L3 | 11/11 (100.0%) [partial] | 0/11 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 packages (#1288) | P-T1 | 12/12 (100.0%) [partial] | n/a | ahead (old n/a) | new >= old | S5+ |
| T3 packages (#1288) | P-T2 | 12/12 (100.0%) [partial] | 0/12 (0.0%) [partial] | ahead | new >= old | S5+ |
| T3 packages (#1288) | L0 | 12/12 (100.0%) [partial] | 5/12 (41.7%) [partial] | ahead | new >= old | S5+ |
| T3 packages (#1288) | L1 | 12/12 (100.0%) [partial] | 5/12 (41.7%) [partial] | ahead | new >= old | S5+ |
| T3 packages (#1288) | L2 | 12/12 (100.0%) [partial] | 3/12 (25.0%) [partial] | ahead | new >= old | S5+ |
| T3 packages (#1288) | L3 | 12/12 (100.0%) [partial] | 2/12 (16.7%) [partial] | ahead | new >= old | S5+ |
| T4 5k corpus (#1276) | P-T1 | 3/4 (75.0%) [skipped 1, partial] | n/a | ahead (old n/a) | new >= old | S5+ |
| T4 5k corpus (#1276) | P-T2 | 4/5 (80.0%) [partial] | 0/5 (0.0%) [partial] | ahead | new >= old | S5+ |
| T4 5k corpus (#1276) | L0 | 5/5 (100.0%) [partial] | 0/5 (0.0%) [partial] | ahead | new >= old | S5+ |
| T4 5k corpus (#1276) | L1 | 5/5 (100.0%) [partial] | 0/5 (0.0%) [partial] | ahead | new >= old | S5+ |
| T4 5k corpus (#1276) | L2 | 5/5 (100.0%) [partial] | 0/5 (0.0%) [partial] | ahead | new >= old | S5+ |
| T4 5k corpus (#1276) | L3 | 5/5 (100.0%) [partial] | 0/5 (0.0%) [partial] | ahead | new >= old | S5+ |
| T2 LaTeX suites | tests | 20/20 (100.0%) | n/a | ahead (old n/a) | new >= old; 0 unexpected | S5+ |
| package-smoke | documents | 59/59 (100.0%) | n/a | ahead (old n/a) | new >= old | S5+ |
| fonts (font census) | fonts | 439/439 (100.0%) | n/a | ahead (old n/a) | new >= old | S5+ |

## Denominators and notes

- T3 fixtures (P-T1, P-T2, L0, L1, L2, L3), new: partial: subset: --limit
- T3 fixtures (P-T1), old: note: n/a: the flashtex CLI is not a TeX engine and writes no box dumps or \tracingall log; P-T1 applies to a pdfTeX-compatible --engine
- T3 fixtures (P-T2, L0, L1, L2, L3), old: partial: subset: --limit
- T3 arXiv (P-T1), new: excluded P-T1 not evaluated (--pt1-skip / log cap) 1
- T3 arXiv (P-T1, P-T2, L0, L1, L2, L3), new: partial: subset: --limit; 12 of 149 manifest entries
- T3 arXiv (P-T1), old: note: n/a: the flashtex CLI is not a TeX engine and writes no box dumps or \tracingall log; P-T1 applies to a pdfTeX-compatible --engine
- T3 arXiv (P-T2, L0, L1, L2, L3), old: partial: subset: --limit; 12 of 149 manifest entries
- T3 templates (P-T1, P-T2, L0, L1, L2, L3), new: excluded oracle 1
- T3 templates (P-T1, P-T2, L0, L1, L2, L3), new: partial: subset: --limit; 12 of 20 manifest entries
- T3 templates (P-T1), old: note: n/a: the flashtex CLI is not a TeX engine and writes no box dumps or \tracingall log; P-T1 applies to a pdfTeX-compatible --engine
- T3 templates (P-T2, L0, L1, L2, L3), old: excluded oracle 1
- T3 templates (P-T2, L0, L1, L2, L3), old: partial: subset: --limit; 12 of 20 manifest entries
- T3 packages (#1288) (P-T1, P-T2, L0, L1, L2, L3), new: partial: subset: --limit; 12 of 92 manifest entries
- T3 packages (#1288) (P-T1), old: note: n/a: the flashtex CLI is not a TeX engine and writes no box dumps or \tracingall log; P-T1 applies to a pdfTeX-compatible --engine
- T3 packages (#1288) (P-T2, L0, L1, L2, L3), old: partial: subset: --limit; 12 of 92 manifest entries
- T4 5k corpus (#1276) (P-T1), new: excluded P-T1 not evaluated (outside the --pt1-sample, or over the log cap: 1) 1, oracle 1
- T4 5k corpus (#1276) (P-T1, P-T2, L0, L1, L2, L3), new: partial: 6 of 5000 manifest entries
- T4 5k corpus (#1276) (P-T1), old: note: n/a: the flashtex CLI is not a TeX engine and writes no box dumps or \tracingall log
- T4 5k corpus (#1276) (P-T2, L0, L1, L2, L3), new: excluded oracle 1
- T4 5k corpus (#1276) (P-T2, L0, L1, L2, L3), old: excluded oracle 1
- T4 5k corpus (#1276) (P-T2, L0, L1, L2, L3), old: partial: 6 of 5000 manifest entries
- T2 LaTeX suites (tests), new: note: 0 failed, 0 unexpected against this host's pdfTeX (reference run); dirs: latex2e/base 20/20
- T2 LaTeX suites (tests), old: note: the harness drives a pdfTeX-compatible binary; the v1 flashtex CLI is not one, so it cannot be measured here
- package-smoke (documents), old: note: the harness drives a pdfTeX-compatible binary; the v1 flashtex CLI is not one, so it cannot be measured here
- fonts (font census) (fonts), new: excluded oracle fails too 17
- fonts (font census) (fonts), new: note: per kind: opentype 0/0, pk 70/70, truetype 2/2, type1 293/293, vf 74/74
- fonts (font census) (fonts), old: note: the harness drives a pdfTeX-compatible binary; the v1 flashtex CLI is not one, so it cannot be measured here

## Retirement stages (#1236)

Only the scoreboard part of each precondition is evaluated here; the rest is listed.

| stage | name | recorded status | scoreboard gate | gate | other preconditions |
|---|---|---|---|---|---|
| S0 | vendor/ retired | done | — | none | none (#1183) |
| S1 | Decouple (no behaviour change) | not started | — | none | Q5 decided |
| S2 | Ten unused crates deleted | not started | — | none | Commander confirms the list (Q1); S1(d) landed |
| S3 | App flag, default legacy | not started | fixtures:P-T2 | not met | P3 exit: zero-tolerance preview parity check |
| S4 | Dual-run gates (T1, T2, T3, T7 in ci.yml) | not started | — | none | S3; a T7 harness exists |
| S5 | Flip: per-document default legacy -> new-for-new-documents -> new | not started | every row | not met | §1.2 latency targets; S4 |
| S6 | App old route out | not started | every row | not met | S5 soak passed (14 days) |
| S7a | Delete render-pipeline, flashtex-cli, perf-bench | not started | every row | not met | S6; T7 gating for N runs; Q6 ruled |
| S7b | Delete crates/compiler | not started | every row | not met | S1(a-c) and S7a landed; the 88 open old-path PRs closed or landed (Q10) |
| S7c | Orphans: extract TFM readers, delete orphan crates | not started | every row | not met | Q2 ruled; S7b landed |
| S8 | Oracles to tests/, final sweep | not started | every row | not met | S7c; Q4 ruled |
