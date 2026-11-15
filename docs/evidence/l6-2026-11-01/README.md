# L6: E0 (main's profile), E3 (MACRO-REPLAY's gate), lead 5 and E1, 2026-11-01

Lane **L6** (kabir-claude-l6 under Commander kabir-claude). This directory holds E0, E3 (twice),
lead 5 and E1 of [`docs/design/engine-v2/L6.md`](../../design/engine-v2/L6.md). E2's evidence is
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

**What might make it a win everywhere** (for the Commander and the BOX-MEMO lanes; this lane does
not touch BOX-MEMO's recording rules). The budget was tried after the Commander's go, and it
does not work (§ "E3 with a recording budget" below):
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
  hyperref + siunitx cold open (lanes COLD-OPEN and BOX-MEMO). Measured below (§ "Lead 5 in
  the host").

## E3 with a recording budget (the Commander's go)

**The bar.** `\XC@col@rlet` joins `RECORD_ANYWHERE` with a per-name budget. It lands only if:
- no measured document is slower by more than 0.5 % in instructions;
- the wins hold;
- the PDFs are identical.

**The change tried.** Branch `agent/kabir-claude-l6/xc-anywhere`, `fdba6e296`, no PR; it is kept
as a record.
- Outside an edit's window, the name's offers and hits are counted.
- After `OFFERS` offers, a hit rate below `RATE` stops it for the session: it is no longer
  recorded or offered outside a window.
- The counts are engine state, so the rule is deterministic. The setting is
  `FLASHTEX_BOXMEMO_BUDGET=OFFERS:RATE` or `off`, and `FLASHTEX_BOXMEMO_BUDGET_TRACE` logs each
  offer.

**The run.** Run 41343036548 on the PC.
- main `c5763d9b8` against `fdba6e296`;
- cold command-line compiles, the median of 3 interleaved reps;
- `FLASHTEX_BOXMEMO_NAMES` is the host's two names plus `XC@col@rlet`.

Δ is instructions against main with the host's two names. The PDFs were identical everywhere.

| document | no budget | 1024 : 0.70 | **2048 : 0.75** | 4096 : 0.80 |
|---|---:|---:|---:|---:|
| beamer-5 | −2.18 % | −0.22 % | −2.15 % | −2.19 % |
| beamer-30 | −31.08 % | +1.24 % | +0.97 % | −0.18 % |
| beamer-madrid (fixture) | +0.80 % | +3.86 % | +1.22 % | +0.78 % |
| beamer-polish (fixture) | −4.43 % | +3.99 % | −0.04 % | −4.87 % |
| beamer-overlays (fixture) | −11.71 % | +0.54 % | −11.65 % | −11.74 % |
| beamer-blocks-columns (fixture) | +12.26 % | +5.32 % | +10.99 % | +12.26 % |
| framed-tikz-theorems (fixture) | +3.08 % | +3.08 % | +3.08 % | +3.08 % |
| plain-100, full-100 | ≤ +0.06 % | ≤ +0.06 % | ≤ +0.06 % | ≤ +0.06 % |
| *Infinite Descent* | +5.24 % | +0.19 % | +0.30 % | +0.82 % |

The first rule tried a budget by hits per recording (64 recordings, then 2–4 hits each; run
41337596660, `raw/e3-budget/runs-41337596660.jsonl`). It stopped the name everywhere, and
beamer-30 ended at +0.13 %.

**Why no threshold works.** The source is `raw/e3-budget/traces/`, one line per offer: offers,
hits, recordings, stopped.
- **Early on, winners and losers look the same.** Hit rates over the first 1,024 offers:

  | document | hit rate |
  |---|---:|
  | beamer-30 | 35 % |
  | beamer-madrid | 35 % |
  | beamer-blocks-columns | 34 % |
  | *Infinite Descent* | 28 % |
- **beamer-30 pays off late.** Its hit rate per 512 offers reaches 82 % only after 2,048 offers,
  and 99 % after 16,384.
- **beamer-blocks-columns pays early.** It has 2,112 offers in all, and 1,132 of its 1,173
  recordings come in the first 2,048.
- **What breaks even.** A fit over these runs gives about 0.71 M instructions per recording and
  0.25 M saved per hit. That is about 2.9 hits per recording, a 74 % hit rate.
- **So no rule meets the bar.** A rule that waits long enough to see beamer-30's rate has
  already paid for blocks-columns' recordings, and one that decides earlier stops beamer-30.
- **framed-tikz-theorems' +3.1 % comes from 192 offers,** too few for any budget to judge.

**Verdict: E3 does not land.** No budget on offers and hits meets the bar.
- What could change that is a cheaper recording for small expansion-only macros: about 0.7 M per
  recording against 0.25 M per hit.
- That is BOX-MEMO's recording machinery (the BOX-MEMO lanes), or MACRO-REPLAY's §9 replay if
  its recordings cost less.
- **Seen on the way, not chased.** On the command line's default configuration, the branch's
  `c096d16b3` cost plain-100 +45 % (3.300 G against 2.274 G; the CLI rows of run 41337613821).
  `FLASHTEX_BOXMEMO=off` was +0.08 %, and so were the runs with restricted names.

  The cause was not chased because the branch does not land. Whoever revives it must find the
  cause first.

## Lead 5 in the host: `\pdfstringdefPreHook`

**The run.** Run 41337613821 on the PC, main `32b875c40`.
- **What it measures:** first opens through the socket host (`coldopen.py`, `l6ab.py
  --host-docs`), the engine thread's instructions summed over the passes, the median of 2.
- **The variants:**
  - `def` is the host's configuration;
  - `off` is `FLASHTEX_BOXMEMO=off`;
  - `d9` is BOX-MEMO with every registered name except `\pdfstringdefPreHook`, so D9 takes the
    hook.

| document | passes | def | off | d9 |
|---|---:|---:|---:|---:|
| full-100 | 2 | 20.409 G | −8.56 % | **−8.34 %** |
| full-300 | 3 | 71.487 G | +1.71 % | **+1.74 %** |
| beamer-30 | 2 | 42.552 G | −3.11 % | +0.15 % |
| beamer-blocks-columns (fixture) | 2 | 6.987 G | −1.19 % | +0.14 % |
| beamer-madrid (fixture) | 2 | 10.441 G | −2.16 % | +0.21 % |
| *Infinite Descent* | 3 | 793.887 G | −1.50 % | −0.31 % |

What it shows:
- **The lead is real on full-100.** BOX-MEMO owning the hook costs 8.6 % of the first open, and
  handing it to D9 takes 8.3 % back.
  - On the command line the gap is 10.2 % (run 41343123964): D9 replays 198 of 226 calls, while
    BOX-MEMO hits 163.
  - Each call run in full costs about 25–28 M instructions with siunitx.
- **It reverses on full-300:** there BOX-MEMO's ownership saves 1.7 %. The reason has not been
  analysed.
- **Why BOX-MEMO misses** (`raw/lead5/boxmemo-misses-full-100.txt`):
  - half the misses are a meaning the key read: `\reserved@a` and `\reserved@b`, LaTeX's scratch
    macros;
  - the other half are calls whose K2 (parameters such as `\offinterlineskip`'s three and
    `\uccode`~`) matches no recorded call. The registers in the digest are what changed between
    calls, not entries of the key (corrected below);
  - D9's guard does not depend on them.
- **Also seen.** In the host, BOX-MEMO costs 1.2–3.1 % of a first open on the decks and 1.5 % on
  the book (`off` against `def`). The keystroke wins it is there for are not measured here.

**Not changed here.** Handing the hook to D9 would fail E3's 0.5 % bar on full-300 (+1.7 %). It
would also give up BOX-MEMO.md §12.2's keystroke win, which this run does not measure. Two fixes
would keep both, and both change BOX-MEMO's recording rules, so they belong to the Commander and
the BOX-MEMO lanes:
- a key that does not depend on those scratch macros and registers;
- D9 owning the hook in cold passes and BOX-MEMO in edit windows. Today `owns()` allows one owner
  per macro per session.

## Lead 5 fixed: K3 by first access (#2041), and a hole in the token loops (#2064)

The coordinator's follow-up: check whether a recording keys entries whose first access is a write.
It did, and `docs/design/engine-v2/BOX-MEMO.md` §15 has the rule and the proof.
- **Why the hook missed.** `get_next`'s hook keyed the full meaning of every control sequence it
  delivered, including the target `get_r_token` delivers for a `\def` or `\let`, before the
  assignment.
- **The 130 `Meanings` misses of the 252** (`raw/k3/debug-41355186238/misses-main.txt`): the hook
  defines `\reserved@a`, `\reserved@b` and `\l__cmd_tmpa_tl` before it reads them.
- **The other 122** were `NotRecorded`: K2 differences, untouched.

**The change, #2041 (head `fbc0cce50`).**
- A read only as a token keys only the meaning's kind: `\outer`, `#`, `&`/`\cr`, `\if`, `\fi`.
  The token readers are `get_r_token`, `scan_toks` without expansion, `macro_call` matching, and
  `pass_text`.
- An assignment to something nothing read yet keeps the old word aside, out of the key.
- A later read that finds that word again (an `\endgroup` brought it back) keys it.

**Measured** on the PC, main `6777eff07`, instructions. PDFs were identical in every run.

| | full-100 | full-300 | beamer-30 | madrid | blocks-columns | polish | framed | *Infinite Descent* |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| host first open, #2041 | **−8.32 %** | **−4.19 %** | −1.42 % | +0.11 % | +0.00 % | +0.01 % | +0.00 % | +0.00 % |
| host first open, #2064 | −6.83 % | −3.66 % | −1.19 % | +0.43 % | −0.06 % | −0.02 % | +0.06 % | +0.20 % |
| command line, BOX-MEMO off, #2041 | −0.00 % | | +0.00 % | −0.00 % | −0.00 % | +0.00 % | −0.00 % | +0.00 % |
| command line, the host's two names, #2041 | −8.58 % | | −1.36 % | +0.08 % | −0.00 % | −0.00 % | −0.00 % | −0.00 % |

The host row is run 41367287585 (three opens per cell). The command-line rows are run
41367284395 (three reps); `raw/k3/cli-final-41367284395/table.md` has every row, the
`\XC@col@rlet` variant included.
- **Lead 5 on full-100.** The hook's hits went from 163 to 195 of 226 (D9 gets 198), and its
  recordings from 63 to 31. The first open is now within about 0.2 % of BOX-MEMO off: 18.70 G
  against 18.69 G in run 41355171586.
- **On full-300,** BOX-MEMO's ownership of the hook now saves about 5.9 % against off, up from
  1.7 %.
- **The remaining 31 misses on full-100** are all `NotRecorded`, that is K2
  (`raw/k3/debug-41355186238/misses-branch.txt`).
- **E3's recording cost** (`fit_xc.py`: each document's `\XC@col@rlet` net against its hits and
  recordings):

  | build | cost of a recording | saving per hit |
  |---|---:|---:|
  | main | 0.799 M | 0.255 M |
  | #2041 | 0.805 M | 0.254 M |
  | #2064 | 1.011 M | 0.262 M |

  An earlier #2041 head, `7181c7071`, cost 0.867 M per recording. It looked old values up in a
  hash and loaded `eqtb` before the stamp test; `80071d7ca` and `fbc0cce50` fixed both.

  So #2041 leaves E3 where it was: the break-even is 3.16 hits per recording, and E3 still does
  not land.

**The hole that predates #2041 (#2064, for the Commander).**
- `changes/throughput.ch`'s token loops [4]–[7] read in place whenever D9 is not recording, so a
  BOX-MEMO recording never saw those control sequences. For example, a body's `\x` that becomes
  `#` between calls was replayed past.
- `a_token_that_turns_into_a_parameter_is_a_miss` gives a different log with BOX-MEMO on when the
  loops are left as they were.
- **#2064's fix:** the loops go through `get_next` while a recording is in progress, as they do
  for D9. Its cost is in the rows above: about 1.0 M per recording against 0.8 M.
- **Tried and dropped:** reporting each control sequence from inside the loops. It cost every run
  0.2–0.7 %, BOX-MEMO off included (run 41357526880, `raw/k3/cli-a-b-41357526880/table.md`).

## E1: PGO, not measured

`l6-pgo.yml` on GitHub's M1 VMs (runs 41334149491 and 41339976306):
- The PGO-instrumented `flashtex-initex` (`scripts/build-engine-dist.sh`) segfaults at once while
  building the format.
- That happens with an 8 MB stack and with a 64 MB one.

**The lldb backtrace** (run 41346722990): `EXC_BAD_ACCESS` at `0x200000000` in
`__llvm_profile_instrument_target`, LLVM's value-profiling runtime.

**The cheap fix tried, which failed** (run 41365690360). `l6pgo.sh`'s `L6PGO_GEN` built the
instrumented engine with `-Cllvm-args=-disable-vp=true`; `scripts/build-engine-dist.sh` itself was
not changed. It crashed in the same function: `EXC_BAD_ACCESS` at `0x3a65646f43797274`, which is
the ASCII text "tryCode:" read as a pointer.
- **What follows:** either the flag did not turn value profiling off in rustc's pipeline, or the
  profile data is overwritten.
- **Not a cheap fix either way, so PGO stops here, as agreed.** The crash log is
  `pgo-crash.txt` in that run's artifact.

## Files

`raw/`:
- `e0-*` and `e3-*`: runs and summaries;
- `boxmemo-stats/`: `FLASHTEX_BOXMEMO_STATS` per document and variant;
- `macro-*.tsv`: macro profiles, top 80 rows by self time;
- `perf-main-*.txt`: profiles.
- `e3-budget/`: the budget runs (`runs-*.jsonl`, `summary-41343036548.md`), per-offer
  `traces/` and `boxmemo-stats/`;
- `lead5/`: the host first opens (`host-41337613821.jsonl`, `summary-41337613821.md`), and
  the command-line debug run 41343123964 (its BOX-MEMO and D9 stats, and a digest of
  `FLASHTEX_BOXMEMO_DEBUG`'s misses);
- `k3/`: the runs of #2041 and #2064, one directory per run with its summary, rows, engines and
  BOX-MEMO stats. Beside them are the scripts that made the tables:
  - `ab_table.py`, the A/B tables;
  - `fit_xc.py`, E3's recording cost;
  - `misses.py`, the miss digests.
