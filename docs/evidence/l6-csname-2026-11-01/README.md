# L6 E2: `id_lookup` asks an index first (2026-11-01)

Lane **L6** (kabir-claude-l6 under Commander kabir-claude), experiment E2 of
[`docs/design/engine-v2/L6.md`](../../design/engine-v2/L6.md): DESIGN.md §5.6 item 3, "a fast
`\csname` lookup". The change is `changes/throughput.ch` [9] and `src/csindex.rs`. Its proof that
the index answers what the chain walk answers is in the change file.

## Machine and method

- **Machine.** The NixOS PC (Ryzen 7 7800X3D), through its self-hosted runner: `l6-ab.yml` run
  41333996048 (the PC's ssh needs a Tailscale re-auth that no agent can do). The runner is in
  `flashtex.slice`, and everything ran under `nice`. Load1 was 2.9–6.2.
- **The measure: instructions.** They repeat to ±0.01 % run to run.
- **Cycles are non-reference.** The "noise" column below runs main's identical code twice, and the
  two differ by up to 3.9 % on the sub-second documents and by 1.1 % on *Infinite Descent*.
- **How each compile runs** (`tools/incr-bench/l6ab.py`, #1940): one cold command-line compile of
  a fresh copy of the settled document, under `perf stat -e instructions:u,cycles:u`. Each cell
  is the median of 5 reps, with the engines and variants interleaved.
- **The engines.** The engines are main `320b360ad` and this branch `16173088a`. Each runs
  as-is and with `FLASHTEX_CSX=off`, the ablation's other side in the same binary.
- **PDF identity.** Every engine and variant gave the same PDF bytes on every document.
- **Files.** `raw/runs.jsonl` holds every run, `raw/summary.md` the tool's table, and `raw/perf-main-*.txt`
  main's `perf record` profiles (instructions).

## Result (VERIFIED; cycles non-reference)

| document | main | index on | Δ instr | Δ cycles | index off, same binary | on vs off: instr | on vs off: cycles | noise: main twice, cycles |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| plain-100 | 2.274 G | 2.256 G | −0.81 % | −3.89 % | 2.275 G | −0.85 % | −1.49 % | −3.77 % |
| full-100 | 8.682 G | 8.614 G | −0.78 % | −2.69 % | 8.689 G | −0.86 % | −1.48 % | −0.16 % |
| beamer-30 | 21.102 G | 20.906 G | −0.93 % | −3.84 % | 21.111 G | −0.97 % | −2.61 % | −0.55 % |
| beamer-madrid (fixture) | 6.648 G | 6.592 G | −0.84 % | −4.58 % | 6.651 G | −0.89 % | −3.82 % | −3.85 % |
| framed-tikz-theorems (fixture) | 3.221 G | 3.188 G | −1.02 % | −1.49 % | 3.223 G | −1.09 % | −3.61 % | +1.41 % |
| *Infinite Descent* (592 pp.) | 233.185 G | 230.171 G | **−1.29 %** | **−5.27 %** | 233.318 G | −1.35 % | −4.39 % | −1.12 % |

- **Every row improves**, in instructions and in cycles.
  - **Instructions:** −0.78 % to −1.29 %. The index takes about two-fifths of what `id_lookup`
    costs: that routine is 2.4–3.1 % of main's instructions on these documents (`raw/perf-main-*.txt`).
  - **Cycles:** more than instructions, because the walk's misses were dependent cache misses.
    The clearest case is *Infinite Descent*, where its 18 s compiles keep the noise near 1 %: −5.3 %
    against main, and −4.4 % against the same binary with the index off.
- **The mode check costs +0.04–0.08 % instructions** (index off against main). That check is the
  only thing on the path while the index is off.
- **L6.md's bar for E2** is ≥ 1 % instructions or cycles with no row worse. The cycles meet it on
  every row. The instructions reach it on two of six.

## Soundness (VERIFIED where marked)

- `tests/csindex.rs` runs 400,000 random lookups with names entered and not entered. It includes
  restores, with the index kept from a later state, and names taken out as `readset.rs` takes
  them. It runs against an engine without the index: the same answer at every step, and the same
  tables throughout. It also runs with `FLASHTEX_CSX=verify`, which walks the chain for every answer
  the index gives and panics on a difference.
- Byte-identical PDFs across main, index on and index off on all six documents above (VERIFIED).
- The incremental gates (soundness sweeps with BOX-MEMO on, verify and off) are in the PR.
