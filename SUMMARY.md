# P5 scoreboard summary

Host: nixos-7800x3d-3 (flashtex-linux; oracle pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), LaTeX 2026-06-01, tlpdb 909745461c89 (/home/kubar/texlive/2026); tools 6e66beddf).
Engines: new 6e66beddf054228104a21dcab4a329ae8aecec0d.

Old column: v1 was deleted in retirement stage S7 (#1888) and is not run; its cells are committed one-off v1 measurements: T3 from v1's last run, p5-scoreboard 41203982275 at 5a42ab270 (2026-10-31; `baselines/t3-v1-oneoff.json`); T4 from the decision-1 one-off (2026-10-01; `baselines/t4-v1-oneoff.json`).

Full table and notes: https://github.com/flash-tex/flashtex/actions/runs/42487025277

**Not all green** (2 row(s) not green). Retirement from S5 on does not start.

| tier | metric | new | old | verdict | target | gates |
|---|---|---|---|---|---|---|
| T3 fixtures | P-T1 | 88/88 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| T3 fixtures | P-T2 | 88/88 (100.0%) | 0/88 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S3, S3r, S5+ |
| T3 fixtures | L0 | 88/88 (100.0%) | 84/88 (95.5%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 fixtures | L1 | 88/88 (100.0%) | 84/88 (95.5%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 fixtures | L2 | 88/88 (100.0%) | 74/88 (84.1%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 fixtures | L3 | 88/88 (100.0%) | 64/88 (72.7%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 arXiv | P-T1 | 127/127 (100.0%) | n/a | ahead (old n/a) | >= 98% (owner bar; old n/a) | S5+ |
| T3 arXiv | P-T2 | 127/127 (100.0%) | 0/127 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 127/127 on the baseline's 127 IDs | ahead (v1 one-off) | new >= old; >= 99% (owner bar) | S5+ |
| T3 arXiv | L0 | 127/127 (100.0%) | 2/127 (1.6%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 127/127 on the baseline's 127 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 arXiv | L1 | 127/127 (100.0%) | 1/127 (0.8%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 127/127 on the baseline's 127 IDs | ahead (v1 one-off) | new >= old; >= 90% | S5+ |
| T3 arXiv | L2 | 127/127 (100.0%) | 0/127 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 127/127 on the baseline's 127 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 arXiv | L3 | 127/127 (100.0%) | 0/127 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 127/127 on the baseline's 127 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 templates | P-T1 | 17/17 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| T3 templates | P-T2 | 17/17 (100.0%) | 0/17 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 17/17 on the baseline's 17 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 templates | L0 | 17/17 (100.0%) | 2/17 (11.8%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 17/17 on the baseline's 17 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 templates | L1 | 17/17 (100.0%) | 2/17 (11.8%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 17/17 on the baseline's 17 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 templates | L2 | 17/17 (100.0%) | 0/17 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 17/17 on the baseline's 17 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 templates | L3 | 17/17 (100.0%) | 0/17 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 17/17 on the baseline's 17 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 packages (#1288) | P-T1 | 86/88 (97.7%) | n/a | **below bar (old n/a)** | 100% or baseline (old n/a) | S5+ |
| T3 packages (#1288) | P-T2 | 88/88 (100.0%) | 0/88 (0.0%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 packages (#1288) | L0 | 88/88 (100.0%) | 11/88 (12.5%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 packages (#1288) | L1 | 88/88 (100.0%) | 9/88 (10.2%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 packages (#1288) | L2 | 88/88 (100.0%) | 6/88 (6.8%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T3 packages (#1288) | L3 | 88/88 (100.0%) | 3/88 (3.4%) v1 one-off (last v1 run, p5-scoreboard 41203982275, 2026-10-31); new 88/88 on the baseline's 88 IDs | ahead (v1 one-off) | new >= old | S5+ |
| T4 5k corpus (#1276) | (any) | missing | missing | **missing** | new >= old | S5+ |
| T2 LaTeX suites | tests | 1590/1590 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a); 0 unexpected | S5+ |
| package-smoke | documents | 481/481 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |
| fonts (font census) | fonts | 441/441 (100.0%) | n/a | ahead (old n/a) | 100% or baseline (old n/a) | S5+ |

### Oracle provenance

Expected data comes from this TeX Live only. A board measured against another texlive.tlpdb (another host, or a `tlmgr update`) is a different oracle: compare boards only when this table matches.

| | |
|---|---|
| pdfTeX | `pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)` |
| platform | `Linux 6.18.58 x86_64` |
| TeX Live root | `/home/kubar/texlive/2026` |
| TeX Live release | `2026` |
| texlive.tlpdb sha256 | `909745461c89882e1afdaff0851db7277f055e2cdb35cd45630cf9b7e7d58934` |
| LaTeX format (latex.ltx) | `2026-06-01` |
| tlpdb revisions | `l3kernel r80334, latex r79618, latex-bin r80015, pdftex r79618` |
| pdftex binary sha256 | `1c5ff71156ee990c3a18402cf06d3671ecf748bd84fb3983dbd5d62b600bc40b` |
