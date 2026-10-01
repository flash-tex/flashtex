# Track 4 review 2026-09-30: new engine vs v1 vs pdfTeX on the NixOS PC (arxiv, templates)

Measured on **nixos-7800x3d**. Host-dependent data: it is not another host's baseline (DESIGN §8).

| engine | version | git SHA | engine sha256 | host | TeX Live | \write18 |
|---|---|---|---|---|---|---|
| new | pdfTeX 3.141592653-2.6-1.40.29 (FlashTeX engine) | 02dcf9d07 | 48d374fcbf47 | nixos-7800x3d (Linux 6.18.45 x86_64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | default |
| v1 | flashtex 0.1.0 (02dcf9d07102) | 02dcf9d07 | 4e47874d99aa | nixos-7800x3d (Linux 6.18.45 x86_64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | default |
| pdflatex | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | — | 1c5ff71156ee | nixos-7800x3d (Linux 6.18.45 x86_64) | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) | default |

## arxiv

| engine | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|---|---|---|---|
| new | 149 | oracle 18 | 125/129 (96.9%) | 128/131 (97.7%) | 129/131 (98.5%) | 129/131 (98.5%) | 129/131 (98.5%) | 129/131 (98.5%) | 129/131 (98.5%) |
| v1 | 149 | oracle 18 | n/a | 0/131 (0.0%) | 2/131 (1.5%) | 1/131 (0.8%) | 0/131 (0.0%) | 0/131 (0.0%) | 0/131 (0.0%) |
| pdflatex | 149 | oracle 18 | n/a | 131/131 (100.0%) | 131/131 (100.0%) | 131/131 (100.0%) | 131/131 (100.0%) | 131/131 (100.0%) | 131/131 (100.0%) |

## templates

| engine | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |
|---|---|---|---|---|---|---|---|---|---|
| new | 20 | fetch 7, oracle 2 | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) |
| v1 | 20 | fetch 7, oracle 2 | n/a | 0/11 (0.0%) | 2/11 (18.2%) | 2/11 (18.2%) | 0/11 (0.0%) | 0/11 (0.0%) | 0/11 (0.0%) |
| pdflatex | 20 | fetch 7, oracle 2 | n/a | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) | 11/11 (100.0%) |

## Root causes (new): every document that is not P-T1 + P-T2 + L4

- (a) package or font missing from the user's TeX Live (§4.4 bundle fallback): **0**
- (b) engine difference: **6**
- (c) harness issue (tools/parity): **6**
- (d) pdflatex fails too (excluded): **18**
- (e) excluded by the convergence rule: pdflatex compiles it but keeps asking for a rerun: **1**

| tier | document | new level | P-T1 | P-T2 | class | cause | § | owner | issue |
|---|---|---|---|---|---|---|---|---|---|
| arxiv | 2501.06999v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Missing \begin{document}. |  |  |  |
| arxiv | 2501.07023v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Command \c@corollary already defined. |  |  |  |
| arxiv | 2501.07032v4 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Missing $ inserted. |  |  |  |
| arxiv | 2501.07072v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Package pdftex.def Error: File `Round_1/eks4.pdf' not found: using draft sett |  |  |  |
| arxiv | 2501.07077v1 | L4 | fail | pass | b | shipout 3: `\hbox(6.88995+2.16492)x397.48499, glue set 0.0foul` where pdfTeX has no glue set; hpack(cal_expand_ratio) returns before setting glue fields (uninitialised in pdfTeX too) | hpack with m=cal_expand_ratio (tex §649 + pdfTeX expansion) | kabir-claude (engine) | #1220 |
| arxiv | 2501.07098v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Command \c@corollary already defined. |  |  |  |
| arxiv | 2501.07105v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Package babel Error: Unknown option 'francais'. |  |  |  |
| arxiv | 2501.07184v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Extra \or. |  |  |  |
| arxiv | 2501.07190v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Extra \or. |  |  |  |
| arxiv | 2501.07457v1 | excluded | n/a | n/a | b (auto d) | panic, index out of bounds (f=27744 > font_max) under microtype spacing: violates the §4.5 no-panic contract | adjust_interword_glue -> get_kn_bs_code (port labels §705, §1874) | kabir-claude (engine) | #1219 |
| arxiv | 2501.07482v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Package pdftex.def Error: File `images/new_version/world_maps/Design sem nome |  |  |  |
| arxiv | 2501.07495v1 | excluded | n/a | n/a | e | pdflatex exits 0 and its PDF is stable from pass 2, but natbib prints `Rerun to get citations correct.` on every pass (multiply defined citations), so the convergence rule records no reference. Hand check by review 58961 |  |  |  |
| arxiv | 2501.07497v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Command \c@clm already defined. |  |  |  |
| arxiv | 2501.08371v3 | below L0 | fail | fail | b | font bbm12 has no pdftex.map entry: the engine aborts where pdfTeX embeds a PK/Type 3 font | writet3 (Type 3 from PK), via pdf_init_font | kabir-claude (engine P3 output) | #1218 |
| arxiv | 2501.08663v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Command \c@lemma already defined. |  |  |  |
| arxiv | 2501.08713v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Command \c@cor already defined. |  |  |  |
| arxiv | 2501.08775v2 | below L0 | fail | fail | b | font bbm10 has no pdftex.map entry: the engine aborts where pdfTeX embeds a PK/Type 3 font | writet3 (Type 3 from PK), via pdf_init_font | kabir-claude (engine P3 output) | #1218 |
| arxiv | 2501.08928v2 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Missing $ inserted. |  |  |  |
| arxiv | 2501.09091v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! File ended while scanning use of \next. |  |  |  |
| arxiv | 2501.09225v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! Extra }, or forgotten $. |  |  |  |
| arxiv | 2501.10183v1 | excluded | n/a | n/a | d | oracle: pdflatex exit 1: ! LaTeX Error: Command \c@lemma already defined. |  |  |  |
| arxiv | 2501.10230v1 | L4 | fail | fail | b | microtype expansion: pdfTeX loads 295 fonts, the engine 293; first log line `{changing current font=OT1/cmr/bx/n/14.4}` vs `{reassigning current font=OT1/cmr/bx/sc/14.4}` | font expansion: hpack cal_expand_ratio / get_expand_font | kabir-claude (engine) | #1220 |
| templates | acmart-sample-acmsmall | excluded | n/a | n/a | c | fetch: missing in TeX Live: <TEXLIVE>/texmf-dist/doc/latex/acmart/samples/sample-acmsmall.tex |  |  |  |
| templates | acmart-sample-sigconf | excluded | n/a | n/a | c | fetch: missing in TeX Live: <TEXLIVE>/texmf-dist/doc/latex/acmart/samples/sample-sigconf.tex |  |  |  |
| templates | amscls-amsbook-template | excluded | n/a | n/a | d | the template's `\include{}` placeholders are empty, so pdflatex cannot write `.aux`: it fails under pdfTeX too, excluded |  |  |  |
| templates | amsmath-amsldoc | excluded | n/a | n/a | c | fetch: sha256 mismatch for doc/latex/amsmath/amsldoc.tex: manifest d21fe77b7588, local c842a86ff194 |  |  |  |
| templates | ieeetran-bare-conf | excluded | n/a | n/a | c | fetch: missing in TeX Live: <TEXLIVE>/texmf-dist/doc/latex/IEEEtran/bare_conf.tex |  |  |  |
| templates | ieeetran-bare-conf-compsoc | excluded | n/a | n/a | c | fetch: missing in TeX Live: <TEXLIVE>/texmf-dist/doc/latex/IEEEtran/bare_conf_compsoc.tex |  |  |  |
| templates | ieeetran-bare-jrnl | excluded | n/a | n/a | c | fetch: missing in TeX Live: <TEXLIVE>/texmf-dist/doc/latex/IEEEtran/bare_jrnl.tex |  |  |  |
| templates | llncs-llncsdoc | excluded | n/a | n/a | d | llncsdoc.sty is not in TeX Live 2026 (LNCS ships it separately): it fails under pdfTeX too, excluded |  |  |  |
| templates | moderncv-template | excluded | n/a | n/a | b (auto c) | microtype expansion: one fewer internal font, so /F95 becomes /F94 (P-T2); log `{changing current font=T1/lmss/m/sl/10}` vs `{reassigning current font=T1/lmss/m/it/10}` | font expansion: hpack cal_expand_ratio / get_expand_font | kabir-claude (engine) | #1220 |

## P-T1 not evaluated (new): 2 documents

They are in no P-T1 denominator. Their P-T2 and levels are measured as usual.

- arxiv/2501.07413v3: not evaluated: listed in --pt1-skip
- arxiv/2501.08950v2: not evaluated: listed in --pt1-skip
