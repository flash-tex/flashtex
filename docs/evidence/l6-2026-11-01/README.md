# L6: E0 (main's profile) and E3 (MACRO-REPLAY's gate), 2026-11-01

Lane **L6** (kabir-claude-l6 under Commander kabir-claude). This directory holds two experiments of
[`docs/design/engine-v2/L6.md`](../../design/engine-v2/L6.md). E2's evidence is
`docs/evidence/l6-csname-2026-11-01/` (#1952).

## Machine and method

- **Machine.** The NixOS PC (Ryzen 7 7800X3D), through its self-hosted runner, `l6-ab.yml`:
  - run 41332717533, E0 and the first E3 pass, load1 4.7–7.3;
  - run 41335468538, E3, load1 5.2–7.0;
  - run 41333996048, the profiles.

  The PC's ssh needs a Tailscale re-auth that no agent can do. The runner is in `flashtex.slice`,
  and every compile ran under `nice`.
- **The measure: instructions.** They repeat to ±0.01 %. Cycles are non-reference: identical code
  run twice differs by up to 3.9 % on sub-second documents.
- **How each compile runs** (`tools/incr-bench/l6ab.py`, #1940): cold command-line compiles of fresh
  copies of documents settled with three passes, under `perf stat`. Each cell is the median of 3
  reps, with the variants interleaved.
- **Engine.** main `3f835f488`.
- **PDF identity.** Every variant gave the same PDF bytes on every document.

## E0: where a cold compile's instructions go (main, `perf record -e instructions:u`)

| function (self) | full-100 | beamer-30 | *Infinite Descent* |
|---|---:|---:|---:|
| `macro_call` | 26.8 % | 25.2 % | 25.6 % |
| `get_next_slow` (with `get_next` inlined into it) | 17.6 % | 21.2 % | 17.1 % |
| `expand` | 7.9 % | 11.2 % | 13.1 % |
| `scan_toks` | 2.8 % | 6.2 % | 5.1 % |
| `pass_text` | 1.5 % | 4.9 % | 6.7 % |
| `id_lookup` | 2.4 % | 2.4 % | 3.1 % |
| `conditional` | 1.7 % | 2.7 % | 3.7 % |
| the file scan (`get_next_file` + `input_ln`) | 2.2 % | 0.8 % | 0.4 % |

`raw/perf-main-*.txt` has the top 60 rows of each profile. What follows from them:
- **The token routines are about half of everything** (`macro_call`, `get_next`, `expand`,
  `scan_toks`, `pass_text`). They belong to lane CONTIG-TOKENS, L6 item 2.
- **`id_lookup` is E2's target.**
- **The file scan is 0.4–4.2 %** of the six documents profiled (framed-tikz-theorems is the
  highest), so NEON scanning (item 5) stays declined. Its bar is 5 %.

## E3: MACRO-REPLAY's gate, measured through BOX-MEMO

MACRO-REPLAY.md §1 was written for xcolor's `\XC@col@rlet`, which was 47 % of a beamer deck on
10-04. BOX-MEMO already registers that macro, but in the host it records only in an edit's window,
so a cold pass gains nothing from it.

**The question.** What would the host's cold passes gain if BOX-MEMO recorded `\XC@col@rlet`
anywhere, as it already records `\pdfstringdefPreHook` and `\pgf@parsefunc` (`RECORD_ANYWHERE`)?

**The variants** (the command line records anywhere):
- `host` is the host's cold-pass configuration: `FLASHTEX_BOXMEMO=on` with only those two names;
- `hostxc` is `host` plus `XC@col@rlet`.

Δ is `hostxc` against `host`. Offers and hits are `XC@col@rlet`'s, from `raw/boxmemo-stats/`.

| document | off | host (Δ against off) | host + `XC@col@rlet` | **Δ instructions** | Δ cycles | offers | hits |
|---|---:|---:|---:|---:|---:|---:|---:|
| beamer-5 | 5.122 G | 5.150 G (+0.55 %) | 5.039 G | **−2.17 %** | +3.49 % | 2,125 | 73 % |
| beamer-30 | 21.102 G | 21.415 G (+1.48 %) | 14.772 G | **−31.02 %** | −24.30 % | 33,638 | 95 % |
| beamer-madrid (fixture) | 6.648 G | 6.715 G (+1.01 %) | 6.771 G | **+0.83 %** | +5.69 % | 4,890 | 73 % |
| beamer-polish (fixture) | 6.129 G | 6.165 G (+0.58 %) | 5.894 G | **−4.40 %** | +1.68 % | 4,834 | 80 % |
| beamer-overlays (fixture) | 5.809 G | 5.844 G (+0.59 %) | 5.161 G | **−11.69 %** | −5.11 % | 4,250 | 89 % |
| beamer-blocks-columns (fixture) | 5.096 G | 5.124 G (+0.53 %) | 5.753 G | **+12.28 %** | +19.88 % | 2,156 | 44 % |
| full-100 | 8.682 G | 9.572 G (+10.25 %) | 9.572 G | **0.00 %** | −0.89 % | 0 | – |
| framed-tikz-theorems (fixture) | 3.221 G | 3.246 G (+0.76 %) | 3.345 G | **+3.05 %** | +5.77 % | 204 | 19 % |
| *Infinite Descent* (592 pp.) | 233.185 G | 234.443 G (+0.54 %) | 246.773 G | **+5.26 %** | +12.54 % | 29,959 | 37 % |

**Verdict: the gate is not met as things stand.** DESIGN's rule asks for a real-document win.
- **Where it wins.** The win is large where the hit rate is high: beamer-30 −31 % (95 % hits),
  beamer-overlays −12 % (89 %). The first E3 pass agrees: beamer-30's macro profile has
  `\XC@col@rlet` at 43.7 % inclusive with BOX-MEMO off, and it drops to 23.5 % once recorded
  (`raw/macro-*.tsv`).
- **Where it loses.** It loses where the hit rate is low, because a recording costs more than a
  call: *Infinite Descent* +5.3 % (37 % hits), beamer-blocks-columns +12.3 % (44 %).
- **Why the hits fail.** Most misses are `Meanings`: a colour the key read was defined again
  between calls (16,499 of the book's 18,920 misses).
- **What it means for MACRO-REPLAY.** Its guard (watched `eqtb` entries) would miss on the same
  redefinitions. So the measurement gives no case for its separate code (#1524–#1526) over
  BOX-MEMO's.

**What would make it a win everywhere** (for the Commander and the BOX-MEMO lanes; this lane does
not touch BOX-MEMO's recording rules):
- the name in `RECORD_ANYWHERE`, with a per-name recording budget: stop recording a name whose
  hits fall below a threshold after N offers;
- the budget is deterministic, since the counts are engine state;
- then measure in the host: cold open and keystrokes on beamer decks, and *Infinite Descent*;
  then run the sweeps with BOX-MEMO on, verify and off.

**Seen on the way (a lead, not a host measurement).** Under the host's cold-pass configuration,
full-100 (hyperref + siunitx) runs **+10.25 % instructions against BOX-MEMO off**. Every other
document is at +0.5–1.5 %, the cost of BOX-MEMO's hooks.
- **Why.** Off, `\pdfstringdefPreHook` is D9's (the intrinsics). In the host it is BOX-MEMO's
  (BOX-MEMO.md §12.2): 226 offers, 163 hits and 63 recordings, at about 25 M instructions per
  call that siunitx fills.
- **What to measure next.** If a host cold open shows the same, it is about 10 % of a
  hyperref + siunitx cold open (lanes COLD-OPEN and BOX-MEMO).

## Files

`raw/`:
- `e0-*` and `e3-*`: runs and summaries;
- `boxmemo-stats/`: `FLASHTEX_BOXMEMO_STATS` per document and variant;
- `macro-*.tsv`: macro profiles, top 80 rows by self time;
- `perf-main-*.txt`: profiles.
