# latency-bench: the T7 latency gate

`tools/latency-bench` measures the DESIGN.md §1.2 latency targets the way the
app experiences them, and fails when one is missed (DESIGN.md §8, T7). MIT.

| §1.2 target | what the bench times (client side of the socket) |
|---|---|
| edit in a paragraph, ≤ 16 ms p95 for the edited page, up to 1,000 pages | `local`: `COMPILE` carrying a one-letter insertion (and, separately, its revert) to the decoded `PAGE` frame showing the edit |
| reflowing edit: edited page ≤ 16 ms, later pages in the background | `reflow`: the same for a 28-word insertion (two or more lines, so every later page moves) and its revert; `done_ms` is the background pass |
| preamble edit ≤ 400 ms to the first visible page | `preamble`: a `\newcommand` added after the last `\usepackage`, and removed, to page 1 being current |
| reopen ≤ 100 ms to the first visible page | `reopen`: a new host process, the document edited on disk, the first `COMPILE` (from the S₀ the previous host persisted) to page 1 |

## Licence boundary

The engine host (`flashtex-host`, GPL-2.0-or-later) runs as a **separate
process**. The bench talks to it only through the display-list-v3 socket
([spec](../../docs/protocol/display-list-v3.md)) with the MIT client crate
`crates/display-list-v3`, the way the Mac app does. It never depends on
`crates/flashtex-engine`; `scripts/check-license-boundary.sh` checks every
workspace member, this one included.

## Running it

```sh
cargo build --release --locked -p flashtex-engine --bin flashtex-host -p latency-bench
python3 tools/latency-bench/gen.py /tmp/lb-docs          # --sizes 10,100 for fewer
target/release/latency-bench run --host target/release/flashtex-host \
    --pool crates/flashtex-engine/pdftex.pool --docs /tmp/lb-docs --out /tmp/lb
target/release/latency-bench check /tmp/lb/summary.json  # the gate: exit 1 on a miss
```

Needs TeX Live (the engine reads it; the host builds its format cache on the
first start). `run` options: `--only plain-10,full-100`, `--local N` (edit
sites per region, default 6), `--reflow N` (3), `--preamble N` (3),
`--reopen N` (6), `--label TEXT`. `check`: `--margin` (default 0.10: a p95 may
exceed its target by 10% before the gate fails), `--min-samples` (6).

The nightly job (`.github/workflows/nightly.yml`, `latency-gate`) runs exactly
this on the self-hosted Mac after the parity tiers.

## Documents

`gen.py` writes `plain-N/main.tex` and `full-N/main.tex` for N = 10, 100, 300,
1,000 pages, deterministically (fixed seeds):

- **plain**: article, geometry, amsmath: prose, inline and display math,
  sections. Byte-identical to the P4-L2-L3 evidence's documents
  (`docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py`), so the numbers compare.
- **full**: the same prose with hyperref, siunitx, cleveref, xcolor,
  footnotes, labels and cross-references (as the evidence's), plus a small
  TikZ picture every 18 paragraphs. Paragraphs per page are recalibrated for
  the pictures; the report states the page count each document actually has.

## What a run does, per document

1. Starts `flashtex-host --socket … --s0-cache …` and connects (3.1 client:
   `incremental`, a fixed `output_dir` as the app's cache directory).
2. Compiles cold, then compiles without edits until the host says `unchanged`.
3. For each region (a paragraph in the first, middle and last tenth of the
   body) and trial: sends the edit in `COMPILE.edits` with `viewport` set to
   the page showing the site, times the frame that shows the edit, waits for
   `DONE`, then reverts it the same way. The edited page is found from the
   frames themselves: the page with a glyph whose span is the edited line and
   whose column is within 12 bytes of the edit (spec §5.3).
4. After every measured compile, compiles again (unmeasured) until
   `unchanged`: the client's rerun after an `.aux` change (spec §6.3). The
   number and time of those reruns are in each sample (`settle_*`).
5. Preamble edits, then closes the host; the last full run persisted S₀.
6. Reopen: edits the file on disk, starts a new host with the same S₀ cache,
   and times the first `COMPILE` to page 1. Also recorded: spawn to
   `listening` (`host_ready_ms`: format check and warm-up) and spawn to page
   1, which the target does not cover (the app starts the host when the
   window opens).

## Output

- `samples.jsonl`: one line per compile: the client-side times
  (`watched_ms`, `first_frame_ms`, `done_ms`), the host's own `DONE` fields
  (`mode`, `restart_page`, `converged_at`, `typeset_pages`, `first_page_ms`…),
  and the machine: `load1`, `load5`, `runners`, `runners_busy`.
- `summary.json` / `summary.md`: per document and metric, n, p50, p95, max,
  the host's p95, the load range and the busiest runner count, and the status
  against the target. `summary.json` also has the machine, `uptime` at start
  and end, the host's `HELLO` and start-up line.
- `host-<doc>.log`: the host's stderr.

## Noise

The Mac that runs the nightly job also runs three CI runners, and a
millisecond target is sensitive to them. The job waits (at most 30 min) for the
1-minute load to fall below the core count, every sample records the load and
how many runners were busy (the nightly job's own runner is one), and the gate
has a 10% margin. The margin is for noise: numbers are reported as measured and
a target is not relaxed to make a run pass.
