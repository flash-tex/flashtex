# raw/ab-all.out and raw/ab-sh.out: columns

One JSON object per (engine, document, phase): `scripts/abkeys.py`, the p50 over the keystrokes
(the first, a warm-up, left out) of the host's `DONE.stages`.

| key | unit | what | reference? |
|---|---|---|---|
| `instr_k` | thousands of instructions | the engine thread, the whole compile | yes: load-independent |
| `edited_instr_k` | thousands of instructions | from just before the restore to the edited page's shipout | yes |
| `restore_instr_k` | thousands of instructions | the restore | yes |
| `test_instr_k` | thousands of instructions | the convergence tests | yes |
| `first_page_instr_k` | thousands of instructions | to the first page shipped | yes |
| `first_page_dl` | ms, **wall** | the first page's display list | no: load 40-190, Low Power Mode |
| `edited_wall` | ms, **wall** | to the edited page's shipout | no |
| `restore` | ms, **wall** | the restore | no |
| `test` | ms, **wall** | the convergence tests | no |
| `pages` | pages | median pages re-typeset per keystroke | yes |
| `conv`, `n` | keystrokes | converged, measured | yes |
