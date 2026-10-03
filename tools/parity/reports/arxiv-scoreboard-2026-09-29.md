# Parity scoreboard 2026-09-29: new engine vs v1 vs pdfTeX (arxiv, templates)

Measured on **mac-m5pro-dq222**. Host-dependent data: it is not another host's baseline (DESIGN §8).

| engine | version | git SHA | engine sha256 | host | TeX Live | \write18 |
|---|---|---|---|---|---|---|
| new | pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine) | 3a5ba1cbf (#1198) + 572cc3388 (#1202) | aeb1baca41b3 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | -shell-restricted |
| v1 | flashtex 0.1.0 (92e6788d3a00) | 92e6788d3 | 5aa3fd72e603 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | -shell-restricted |
| pdflatex | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | — | 3ead7baeffb8 | mac-m5pro-dq222 (Darwin 25.6.0 arm64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | -shell-restricted |

## arxiv

| engine | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|---|---|---|---|
| new | 149 | oracle 9 | 131/136 (96.3%) | 136/140 (97.1%) | 137/140 (97.9%) | 137/140 (97.9%) | 137/140 (97.9%) | 137/140 (97.9%) | 137/140 (97.9%) |
| v1 | 149 | oracle 9 | n/a | 0/140 (0.0%) | 2/140 (1.4%) | 1/140 (0.7%) | 0/140 (0.0%) | 0/140 (0.0%) | 0/140 (0.0%) |
| pdflatex | 149 | oracle 9 | 136/136 (100.0%) | 140/140 (100.0%) | 140/140 (100.0%) | 140/140 (100.0%) | 140/140 (100.0%) | 140/140 (100.0%) | 140/140 (100.0%) |

## templates

| engine | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|---|---|---|---|
| new | 20 | oracle 2 | 17/18 (94.4%) | 17/18 (94.4%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) |
| v1 | 20 | oracle 2 | n/a | 0/18 (0.0%) | 2/18 (11.1%) | 2/18 (11.1%) | 0/18 (0.0%) | 0/18 (0.0%) | 0/18 (0.0%) |
| pdflatex | 20 | oracle 2 | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) | 18/18 (100.0%) |

## Root causes (new): every document that is not P-T1 + P-T2 + L4

- (a) package or font missing from the user's TeX Live (§4.4 bundle fallback): **0**
- (b) engine difference: **6**
- (c) harness issue (tools/parity): **0**
- (d) pdflatex fails too (excluded): **10**
- (e) excluded by the convergence rule: pdflatex compiles it but keeps asking for a rerun: **1**

| tier | document | new level | P-T1 | P-T2 | class | cause | § | owner | issue |
|---|---|---|---|---|---|---|---|---|---|
| arxiv | 2501.06999v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Missing \begin{document}. |  |  |  |
| arxiv | 2501.07032v4 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Missing $ inserted. |  |  |  |
| arxiv | 2501.07072v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Package pdftex.def Error: File `Round_1/eks4.pdf' not found: using draft sett |  |  |  |
| arxiv | 2501.07077v1 | L4 | fail | pass | b | shipout 3: `\hbox(6.88995+2.16492)x397.48499, glue set 0.0foul` where pdfTeX has no glue set; hpack(cal_expand_ratio) returns before setting glue fields (uninitialised in pdfTeX too) | hpack with m=cal_expand_ratio (tex §649 + pdfTeX expansion) | kabir-claude (engine) | #1220 |
| arxiv | 2501.07184v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Extra \or. |  |  |  |
| arxiv | 2501.07190v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Extra \or. |  |  |  |
| arxiv | 2501.07457v1 | below L0 | fail | fail | b | panic, index out of bounds (f=27744 > font_max) under microtype spacing: violates the §4.5 no-panic contract | adjust_interword_glue -> get_kn_bs_code (port labels §705, §1874) | kabir-claude (engine) | #1219 |
| arxiv | 2501.07482v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Package pdftex.def Error: File `images/new_version/world_maps/Design sem nome |  |  |  |
| arxiv | 2501.07495v1 | excluded | n/a | n/a | e | pdflatex exits 0 and its PDF is stable from pass 2, but natbib prints `Rerun to get citations correct.` on every pass (multiply defined citations), so the convergence rule records no reference. Hand check by review 58961 |  |  |  |
| arxiv | 2501.08371v3 | below L0 | fail | fail | b | font bbm12 has no pdftex.map entry: the engine aborts where pdfTeX embeds a PK/Type 3 font | writet3 (Type 3 from PK), via pdf_init_font | kabir-claude (engine P3 output) | #1218 |
| arxiv | 2501.08775v2 | below L0 | fail | fail | b | font bbm10 has no pdftex.map entry: the engine aborts where pdfTeX embeds a PK/Type 3 font | writet3 (Type 3 from PK), via pdf_init_font | kabir-claude (engine P3 output) | #1218 |
| arxiv | 2501.08928v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Missing $ inserted. |  |  |  |
| arxiv | 2501.09091v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! File ended while scanning use of \next. |  |  |  |
| arxiv | 2501.10230v1 | L4 | fail | fail | b | microtype expansion: pdfTeX loads 295 fonts, the engine 293; first log line `{changing current font=OT1/cmr/bx/n/14.4}` vs `{reassigning current font=OT1/cmr/bx/sc/14.4}` | font expansion: hpack cal_expand_ratio / get_expand_font | kabir-claude (engine) | #1220 |
| templates | amscls-amsbook-template | excluded | n/a | n/a | d | the template's `\include{}` placeholders are empty, so pdflatex cannot write `.aux`: it fails under pdfTeX too, excluded |  |  |  |
| templates | llncs-llncsdoc | excluded | n/a | n/a | d | llncsdoc.sty is not in TeX Live 2026 (LNCS ships it separately): it fails under pdfTeX too, excluded |  |  |  |
| templates | moderncv-template | L4 | fail | fail | b | microtype expansion: one fewer internal font, so /F95 becomes /F94 (P-T2); log `{changing current font=T1/lmss/m/sl/10}` vs `{reassigning current font=T1/lmss/m/it/10}` | font expansion: hpack cal_expand_ratio / get_expand_font | kabir-claude (engine) | #1220 |

## P-T1 not evaluated (new): 4 documents

They are in no P-T1 denominator. Their P-T2 and levels are measured as usual.

- arxiv/2501.07413v3: not evaluated: listed in --pt1-skip
- arxiv/2501.08663v2: not evaluated: listed in --pt1-skip
- arxiv/2501.08950v2: not evaluated: listed in --pt1-skip
- arxiv/2501.10183v1: not evaluated: listed in --pt1-skip
