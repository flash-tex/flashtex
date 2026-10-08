# P5 scoreboard summary

Host: nixos-7800x3d (flashtex-linux; oracle pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), LaTeX 2026-06-01, tlpdb 909745461c89 (/home/kubar/texlive/2026); tools 0258d79d0).
Engines: new 7abe49ad3bba7d6da4c4f14e8903b5fbee100ba1, old 7abe49ad3bba7d6da4c4f14e8903b5fbee100ba1.

Full table and notes: https://github.com/flash-tex/flashtex/actions/runs/37789250180

**All green**: new >= old on every tier, targets and bars met, every run complete. That is only the scoreboard part of S5's precondition; S5 also needs: decision 3 gate as confirmed (Q3): P-T2 >= 99% and P-T1 >= 98% on arXiv and T4, zero crashes on T4, new >= old on every tier; T2 clean; no-TeX-Live gate green; the S5 app-parity rows of docs/evidence/app-parity-2026-10-05 (plan §4.4; app_parity_gate: tools/parity/app-parity-rows.json, its tests passing, not skipped, on mac-app and the host leg); T1 (lockstep) has 0 new differences, which this board does not measure; §1.2 latency targets met on a T7 reference run; one shipped release carries #1421/#1427 records; the modes lane's fontspec fallback; S3, S3r and S4 landed.

| tier | metric | new | old | verdict | target | gates |
|---|---|---|---|---|---|---|
| T3 fixtures | P-T1 | 86/86 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| T3 fixtures | P-T2 | 86/86 (100.0%) | 0/86 (0.0%) | ahead | new >= old | S3, S3r, S5+ |
| T3 fixtures | L0 | 86/86 (100.0%) | 84/86 (97.7%) | ahead | new >= old | S5+ |
| T3 fixtures | L1 | 86/86 (100.0%) | 84/86 (97.7%) | ahead | new >= old | S5+ |
| T3 fixtures | L2 | 86/86 (100.0%) | 74/86 (86.0%) | ahead | new >= old | S5+ |
| T3 fixtures | L3 | 86/86 (100.0%) | 64/86 (74.4%) | ahead | new >= old | S5+ |
| T3 arXiv | P-T1 | 127/127 (100.0%) | n/a | ahead (old n/a) | >= 98% (owner bar; old n/a) | S5+ |
| T3 arXiv | P-T2 | 127/127 (100.0%) | 0/127 (0.0%) | ahead | new >= old; >= 99% (owner bar) | S5+ |
| T3 arXiv | L0 | 127/127 (100.0%) | 2/127 (1.6%) | ahead | new >= old | S5+ |
| T3 arXiv | L1 | 127/127 (100.0%) | 1/127 (0.8%) | ahead | new >= old; >= 90% | S5+ |
| T3 arXiv | L2 | 127/127 (100.0%) | 0/127 (0.0%) | ahead | new >= old | S5+ |
| T3 arXiv | L3 | 127/127 (100.0%) | 0/127 (0.0%) | ahead | new >= old | S5+ |
| T3 templates | P-T1 | 17/17 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| T3 templates | P-T2 | 17/17 (100.0%) | 0/17 (0.0%) | ahead | new >= old | S5+ |
| T3 templates | L0 | 17/17 (100.0%) | 2/17 (11.8%) | ahead | new >= old | S5+ |
| T3 templates | L1 | 17/17 (100.0%) | 2/17 (11.8%) | ahead | new >= old | S5+ |
| T3 templates | L2 | 17/17 (100.0%) | 0/17 (0.0%) | ahead | new >= old | S5+ |
| T3 templates | L3 | 17/17 (100.0%) | 0/17 (0.0%) | ahead | new >= old | S5+ |
| T3 packages (#1288) | P-T1 | 88/88 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| T3 packages (#1288) | P-T2 | 88/88 (100.0%) | 0/88 (0.0%) | ahead | new >= old | S5+ |
| T3 packages (#1288) | L0 | 88/88 (100.0%) | 11/88 (12.5%) | ahead | new >= old | S5+ |
| T3 packages (#1288) | L1 | 88/88 (100.0%) | 9/88 (10.2%) | ahead | new >= old | S5+ |
| T3 packages (#1288) | L2 | 88/88 (100.0%) | 6/88 (6.8%) | ahead | new >= old | S5+ |
| T3 packages (#1288) | L3 | 88/88 (100.0%) | 3/88 (3.4%) | ahead | new >= old | S5+ |
| T4 5k corpus (#1276) | P-T1 | 193/193 (100.0%) | n/a | ahead (old n/a) | >= 98% (owner bar; old n/a) | S5+ |
| T4 5k corpus (#1276) | P-T2 | 3789/3789 (100.0%) | 0/440 (0.0%) v1 one-off (decision 1, 2026-10-01); CROSS-ORACLE, v1 measured against texlive.tlpdb ca39e6791582 (/usr/local/texlive/2026), the board's 909745461c89 (/home/kubar/texlive/2026); new 381/381 on the baseline's 440 IDs | ahead (v1 one-off) | new >= old; >= 99% (owner bar) | S5+ |
| T4 5k corpus (#1276) | L0 | 3790/3790 (100.0%) | 7/440 (1.6%) v1 one-off (decision 1, 2026-10-01); CROSS-ORACLE, v1 measured against texlive.tlpdb ca39e6791582 (/usr/local/texlive/2026), the board's 909745461c89 (/home/kubar/texlive/2026); new 381/381 on the baseline's 440 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T4 5k corpus (#1276) | L1 | 3790/3790 (100.0%) | 5/440 (1.1%) v1 one-off (decision 1, 2026-10-01); CROSS-ORACLE, v1 measured against texlive.tlpdb ca39e6791582 (/usr/local/texlive/2026), the board's 909745461c89 (/home/kubar/texlive/2026); new 381/381 on the baseline's 440 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T4 5k corpus (#1276) | L2 | 3790/3790 (100.0%) | 0/440 (0.0%) v1 one-off (decision 1, 2026-10-01); CROSS-ORACLE, v1 measured against texlive.tlpdb ca39e6791582 (/usr/local/texlive/2026), the board's 909745461c89 (/home/kubar/texlive/2026); new 381/381 on the baseline's 440 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T4 5k corpus (#1276) | L3 | 3790/3790 (100.0%) | 0/440 (0.0%) v1 one-off (decision 1, 2026-10-01); CROSS-ORACLE, v1 measured against texlive.tlpdb ca39e6791582 (/usr/local/texlive/2026), the board's 909745461c89 (/home/kubar/texlive/2026); new 381/381 on the baseline's 440 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T4 5k corpus (#1276) | crashes | 3790/3790 (100.0%) | n/a | ahead (old n/a) | 0 crashes (owner bar; old n/a) | S5+ |
| T2 LaTeX suites | tests | 1590/1590 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a); 0 unexpected | S5+ |
| package-smoke | documents | 481/481 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| fonts (font census) | fonts | 441/441 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |

### Oracle provenance

Expected data comes from this TeX Live only. A board measured against another texlive.tlpdb (another host, or a `tlmgr update`) is a different oracle: compare boards only when this table matches.

| | |
|---|---|
| pdfTeX | `pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)` |
| platform | `Linux 6.18.45 x86_64` |
| TeX Live root | `/home/kubar/texlive/2026` |
| TeX Live release | `2026` |
| texlive.tlpdb sha256 | `909745461c89882e1afdaff0851db7277f055e2cdb35cd45630cf9b7e7d58934` |
| LaTeX format (latex.ltx) | `2026-06-01` |
| tlpdb revisions | `l3kernel r80334, latex r79618, latex-bin r80015, pdftex r79618` |
| pdftex binary sha256 | `1c5ff71156ee990c3a18402cf06d3671ecf748bd84fb3983dbd5d62b600bc40b` |
