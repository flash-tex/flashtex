# BENCH-README: the new engine against pdflatex, xelatex and latexmk (2026-10-14)

Lane **BENCH-README** (Commander kabir-claude): benchmark the engine as it is on main and publish
the numbers in README.md. Everything below was measured, except where a line says **belief** or
**estimate**.

## Where and what

| | |
|---|---|
| **Engine** | main `a8379a52b`. The hosted runs built `da1923161` and `eb9edd975`: main plus this lane's harness commits, with no change to `crates/flashtex-engine`, `crates/flashtex-build` or `crates/flashtex-xetex` (`git diff a8379a52b eb9edd975 --stat`: the workflow, `tools/incr-bench/bench_readme*.py`, `coldopen.py --keep-s0` and `dl3-keys`, which is the benchmark client) |
| **Build** | `cargo build --release --locked`, rustc 1.99.0 (b940084d7 2026-09-28) |
| **Machine A, hosted** | **GitHub-hosted M1 VM (macos-15, 3 vCPU, 7 GB)**: `VirtualMac2,1`, Apple M1 (Virtual), macOS 15.7.9. A shared virtual machine, so its figures describe this runner class and are not the P4 reference. Workflow `.github/workflows/bench-readme.yml`, runs [38634650946](https://github.com/flash-tex/flashtex/actions/runs/38634650946) (every shard) and [38639764923](https://github.com/flash-tex/flashtex/actions/runs/38639764923) (Unicode, rerun with its structural parity check) |
| **Machine B, local** | the owner's **Apple M5 Pro** MacBook Pro (Mac17,9, 15 cores, 24 GB), macOS 26.6.2. Used for T7 only (see below) |
| **Oracle** | TeX Live 2026: pdfTeX 3.141592653-2.6-1.40.29, XeTeX 0.999998, latexmk. Hosted: MacTeX 2026 (`mactex-no-gui` cask 2026.0324, whose package SHA-256 the cask pins). M5 Pro: MacTeX at `/Library/TeX/texbin`. pdflatex and xelatex are the oracle only; neither runs in the product |
| **Power, M5 Pro** | T7 ran on mains power with Low Power Mode off (`pmset -g`: `powermode 2`; battery 15 % → 40 %, charging; `t7-m5pro/power.txt`). An earlier start on battery was stopped after 12 minutes when the Mac was plugged in, and it is not used |

**Overleaf** was not measured. It needs an account and compiles on remote servers. Overleaf runs
pdflatex through latexmk, so a full Overleaf compile takes at least the pdflatex/latexmk time below,
plus upload and queueing. The local pdflatex and latexmk columns are the honest proxy.

## 1. Live preview: keystroke to updated page

### Against recompiling, on the same machine (hosted M1 VM)

A keystroke's time to its updated page comes from `modes.sh` / `mem.py` over the host's socket, as
the app types: 8 letters on each of 4 pages, 300 ms apart, Balanced mode. The rebuild times are
§2's medians on the same runner class.

| document | keystroke → updated page p50 / p95 | pdflatex, one pass | latexmk, full build | faster than one pdflatex pass | faster than a latexmk rebuild |
|---|---:|---:|---:|---:|---:|
| full-1000 (1,002 pages) | **19.6 / 53.8 ms** | 12.10 s | 37.92 s | **617×** | **1,935×** |
| plain-1000 (1,001 pages) | **11.8 / 46.9 ms** | 2.40 s | 4.91 s | **203×** | **416×** |
| b04, fontspec, 11 pages (Unicode) | **11.2 / 17.0 ms** | xelatex 1.17 s | latexmk -xelatex 2.64 s | **104×** | **236×** |

The ratios are medians over medians. The preview is the page FlashTeX re-typesets. Its PDF export
is §2's.

### T7 on the M5 Pro (`t7-m5pro/`, non-reference)

`scripts/t7-reference.sh --no-build --max-load 4 --wait 300` (all ten T7 documents plus
`infdesc`) passed every machine check: an Apple-Silicon Mac, mains power, Low Power Mode off, a clean
checkout and TeX Live. **It is a non-reference run**: load1 reached 7.3 (the owner's desktop apps
were open and another project's builds were running), above the 4.0 this run was given (the default
is 2.0). Its verdict is "31 MISSES (89 of 120 gated rows pass)". Most misses are the beamer decks,
which have no baseline row, and full-1000's typing rows (p95 11.7–13.2 ms against 11 ms). The full
table is `t7-m5pro/table.md`. The columns are the host's share, from COMPILE sent to the edited
page's PAGE frame, p50 / p95 in ms:

| doc | pages | letter (middle) | sentence | typing every 50 ms | typing every 100 ms | preamble edit (to page 1) | host peak RSS |
|---|---:|---:|---:|---:|---:|---:|---:|
| plain-10 | 10 | 4.4 / 5.1 | 4.3 / 5.6 | 4.3 / 6.5 | 4.2 / 4.6 | 36.6 / 38.6 | 154 MB |
| full-10 | 11 | 7.8 / 8.6 | 8.2 / 9.1 | 7.8 / 8.2 | 8.1 / 8.9 | 167 / 177 | 214 MB |
| plain-100 | 100 | 4.2 / 5.6 | 3.6 / 3.9 | 3.2 / 3.4 | 3.7 / 4.3 | 34.9 / 37.4 | 189 MB |
| full-100 | 101 | 6.9 / 7.9 | 6.8 / 8.5 | 5.1 / 5.9 | 6.0 / 6.6 | 169 / 193 | 232 MB |
| plain-300 | 301 | 4.0 / 5.2 | 4.1 / 4.9 | 3.3 / 4.3 | 3.7 / 3.8 | 36.7 / 37.8 | 278 MB |
| full-300 | 299 | 7.6 / 8.4 | 7.5 / 8.6 | 6.4 / 7.3 | 7.2 / 8.0 | 175 / 179 | 337 MB |
| plain-1000 | 1001 | 5.6 / 6.4 | 6.2 / 7.6 | 4.8 / 5.4 | 5.2 / 5.5 | 43.9 / 51.3 | 541 MB |
| full-1000 | 1002 | 8.4 / 9.5 | 8.4 / 9.4 | 10.9 / 13.2 | 9.7 / 12.8 | 200 / 214 | 655 MB |
| beamer-5 | 5 | 11.7 / 13.0 | 11.8 / 12.4 | 11.2 / 12.7 | 11.5 / 12.5 | 245 / 311 | 202 MB |
| beamer-30 | 58 | 24.4 / 26.3 | 24.0 / 26.9 | 26.3 / 37.9 | 23.8 / 26.5 | 267 / 304 | 217 MB |

The table has no **infdesc** rows. Its in-body and typing phases ran, but the preamble phase stopped
the run with a harness error: `dl3-keys: no prose line on one page`, because a book's main file has
only `\include` lines. This lane fixes that in `dl3-keys`, since a preamble edit needs no prose
line. The fixed build was not re-run on the M5 Pro, because the remaining measurements moved to the
hosted runners. From the run's raw records (`t7-m5pro/raw.tgz`; summarised with `t7.stats` in
`t7-m5pro/infdesc-partial.md`), infdesc's edited page: letter@start 31.6 / 34.5 ms, letter@middle
77.2 / 80.9, letter@end 80.0 / 83.5, sentence 83.3 / 90.2, newline 78.9 / 87.5, split 83.7 / 92.9.
Typing every 50 / 60 / 80 / 100 / 150 ms: p50 86 / 969 / 64 / 47 / 88 ms, p95 262 / 1,994 / 96 /
145 / 117 ms. The typing@60ms row stands out: on the book, keystrokes 60 ms apart queue up. That
is a finding for the incremental lanes; these are not gated numbers.

The M5 Pro run also tried cold compiles. Another session's build then raised load1 above 150, so
those runs were stopped and discarded, and the cold compiles moved to the hosted runners.

## 2. Cold compile: the engine, its CLI, pdflatex and latexmk (hosted M1 VM)

`tools/incr-bench/bench_readme.py`. Each run gets a fresh copy of the document (no `.aux`, no
`.toc`) and a fresh process. Its wall time runs from fork to exit. The OS file cache and the format
cache are warm. Each document gets 7 timed runs per engine, and a run of every engine is taken in
turn. The figures are the median and p95 (nearest rank). Engines:

- **`flashtex-v3 build`**: the new engine's CLI, as a user runs it. It starts `flashtex-host`, runs
  a resident compile with the external tools, then an export compile.
- **pdflatex**: one pass of `pdflatex -interaction=batchmode`.
- **latexmk**: `latexmk -pdf`, which runs pdflatex until it converges (2–3 passes here) and runs
  makeindex through `\write18` where the document asks for it.
- **engine**: `flashtex-initex` itself as pdflatex (the same arguments, through its `pdftex` link):
  the engine with no host and no checkpoints.
- **engine-mk**: `latexmk -pdf -pdflatex='pdftex -fmt=pdflatex %O %S'` with the engine first on
  `PATH`, which is the engine's own full build.

**Output parity**, from each document's warm-up. engine-mk's PDF is **byte-identical** to latexmk's
on all 11 documents. flashtex-v3's PDF is byte-identical except the trailer `/ID`, on all 11.
pdfTeX's `/ID` hashes the output file's name, and v3 writes into a private directory. Both runs use
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Every speed figure below is therefore for matching
output.

| document | flashtex-v3 build | pdflatex one pass | latexmk (passes) | engine one pass | engine-mk (latexmk with the engine) | engine vs pdflatex, one pass | engine-mk vs latexmk | v3 vs latexmk |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| hw1 (3 pages) | 791 ms / 947 ms | 657 ms / 830 ms | 1.60 s / 1.78 s (2) | 208 ms / 246 ms | 663 ms / 745 ms | 3.16× | 2.41× | 2.03× |
| hw2 (3 pages) | 641 ms / 781 ms | 539 ms / 599 ms | 1.20 s / 1.29 s (2) | 162 ms / 209 ms | 536 ms / 642 ms | 3.33× | 2.24× | 1.87× |
| plain-100 | 2.24 s / 2.59 s | 708 ms / 881 ms | 1.52 s / 2.19 s (2) | 324 ms / 485 ms | 886 ms / 1.13 s | 2.19× | 1.72× | 0.68× |
| plain-300 | 5.32 s / 5.67 s | 1.14 s / 1.31 s | 2.51 s / 2.92 s (2) | 750 ms / 914 ms | 1.75 s / 1.90 s | 1.52× | 1.43× | 0.47× |
| full-100 | 5.08 s / 5.79 s | 2.00 s / 2.42 s | 4.24 s / 4.64 s (2) | 1.06 s / 1.20 s | 3.68 s / 4.08 s | 1.89× | 1.15× | 0.83× |
| beamer-5 | 1.86 s / 2.16 s | 1.07 s / 1.18 s | 2.45 s / 3.43 s (2) | 600 ms / 665 ms | 2.26 s / 2.36 s | 1.78× | 1.08× | 1.32× |
| beamer-30 | 8.14 s / 8.92 s | 2.87 s / 3.49 s | 5.96 s / 7.66 s (2) | 2.34 s / 2.74 s | 7.25 s / 8.56 s | 1.23× | 0.82× | 0.73× |
| plain-1000 | 14.59 s / 16.84 s | 2.40 s / 2.54 s | 4.91 s / 5.60 s (2) | 1.81 s / 1.94 s | 4.09 s / 4.90 s | 1.33× | 1.20× | 0.34× |
| full-1000 | 52.98 s / 61.31 s | 12.10 s / 14.67 s | 37.92 s / 50.98 s (3) | 6.54 s / 8.20 s | 19.82 s / 23.07 s | 1.85× | 1.91× | 0.72× |
| *Infinite Descent* (592 pages) | 118.59 s / 131.19 s † | 23.39 s / 28.06 s | 71.90 s / 83.99 s (3) | 19.39 s / 24.77 s | 63.46 s / 71.48 s | 1.21× | 1.13× | 0.61× |
| *Infinite Descent* ×2 (1,142 pages) | 300.32 s / 346.41 s † | 56.44 s / 63.84 s | 162.82 s / 183.13 s (3) | 47.90 s / 66.34 s | 152.38 s / 177.15 s | 1.18× | 1.07× | 0.54× |

Each cell is p50 / p95. A ratio is the other engine's median over the first one's, so above 1× is
faster.

**Known issue: the CLI's cold build is slow (lane CLI-COLD).** The engine alone is 1.2–3.3× faster
than pdflatex per pass. `flashtex-v3 build`, though, is slower than latexmk on every document above
100 pages: 0.34× on plain-1000, 0.72× on full-1000, 0.61× on *Infinite Descent*. Its first compile
is the resident one, which records the checkpoints that live preview needs. An export compile
follows it, so a cold `build` pays for the preview machinery that a one-shot build does not need.
(**Belief**, from the architecture and the engine/engine-mk columns; not profiled here.)
**† `flashtex-v3 build` exits 1 on both books** ("makeindex: errors", "Input index file main.idx not
found"), although it writes the PDF, and that PDF matches latexmk's byte for byte apart from `/ID`.
Those times count, and the table marks them. Both items are for CLI-COLD.

Peak resident memory of each run's whole process tree (ps every 100 ms, warm-up run):
`tables/cold-classic.md`. pdflatex peaks at 47–82 MB, the engine at 48–127 MB, and flashtex-v3 at
147 MB–1.16 GB, most of which is the host's checkpoints.

## 3. Unicode mode against xelatex (hosted M1 VM)

`bench_readme.py --set unicode`, b01–b04 (`tools/incr-bench/unicode-corpus`), 9 timed runs each.
**Parity:** FlashTeX writes Unicode mode's PDF itself, where xelatex runs xdvipdfmx, so the check is
the M3 export gate's structural one (`tools/xetex-pdfparity/compare.py --no-visual`). On all four
documents it finds **0 structural differences** and every glyph matched (227, 196, 83 and 24,284
glyphs), at most 0.051 bp from xelatex's position.

| document | flashtex-v3 build --mode unicode | xelatex one pass | latexmk -xelatex (passes) | v3 vs latexmk |
|---|---:|---:|---:|---:|
| b01 article | 608 ms / 811 ms | 954 ms / 1.10 s | 1.50 s / 1.68 s (2) | 2.47× |
| b02 Georgia | 1.21 s / 1.28 s | 1.19 s / 1.41 s | 1.92 s / 2.10 s (2) | 1.59× |
| b03 unicode-math | 2.98 s / 3.12 s | 1.71 s / 2.02 s | 3.05 s / 3.32 s (2) | 1.02× |
| b04 fontspec, 11 pages | 1.86 s / 2.11 s | 1.17 s / 1.35 s | 2.64 s / 2.78 s (3) | 1.42× |

Keystrokes (`unicode.py --keys 40`, one letter at a time, the edited page): b01 8.9 / 16.5 ms, b02
6.4 / 10.3, b03 16.4 / 21.8, b04 11.2 / 17.0 (p50 / p95). Host peak 204–214 MB.
`tables/unicode-keys.md`.

## 4. First page after open, and reopen (hosted M1 VM)

`coldopen.py`, 7 runs each. **Open** uses a fresh copy and a fresh host and S0 cache, as a first
open does. **Reopen** uses `--keep-aux --keep-s0`: the previous run's output and its persisted S0,
as the app reopens a project. The previous run's output is the record from the run before, and the
first run, which is cold, is left out.

| document | pages | open: first page | open: all pages | open: DONE (passes) | reopen: first page | reopen: all pages | host peak RSS (open / reopen) |
|---|---:|---:|---:|---:|---:|---:|---:|
| hw1 | 3 | 122 / 149 ms | 131 / 158 ms | 208 / 247 ms (2) | 122 / 138 ms | 131 / 149 ms | 125 / 77 MB |
| plain-100 | 100 | 59 / 70 ms | 448 / 453 ms | 743 / 773 ms (2) | 61 / 63 ms | 434 / 452 ms | 121 / 111 MB |
| plain-1000 | 1001 | 62 / 64 ms | 4.43 / 4.54 s | 7.62 / 7.80 s (2) | 65 / 95 ms | 4.36 / 4.52 s | 487 / 478 MB |
| full-1000 | 1002 | 285 / 483 ms | 10.34 / 19.73 s | 31.60 / 56.22 s (3) | 470 / 655 ms | 15.04 / 17.35 s | 672 / 544 MB |
| beamer-30 | 58 | 670 / 833 ms | 2.75 / 3.17 s | 4.83 / 6.72 s (2) | 578 / 686 ms | 2.43 / 2.52 s | 127 / 115 MB |
| *Infinite Descent* | 580 | 1.87 / 2.32 s | 77.52 / 87.77 s | 116.35 / 132.91 s (3) | 1.73 / 2.01 s | 38.34 / 44.19 s | 614 / 449 MB |
| *Infinite Descent* ×2 | 1140 | 1.58 / 2.64 s | 137.56 / 146.40 s | 208.78 / 224.84 s (3) | 1.65 / 1.87 s | 67.92 / 78.24 s | 1,087 / 906 MB |

The page counts are the host's at the open's last frame (`dl3-coldopen`). A reopen takes one pass
where an open takes two or three. The S0 cache brings the first page back about as fast as an open,
and the whole book back in half the time. The T7 run's `reopen` rows measure the same thing on the
M5 Pro: 57–104 ms p50 to page 1, including the host's spawn.

## 5. Memory by performance mode (hosted M1 VM)

`modes.sh ref` reports the host's peak resident memory while typing (`mem.py`, 32 keystrokes on 4
pages) and the edited page's latency:

| document | low-memory | balanced (default) | high-performance |
|---|---:|---:|---:|
| plain-1000 | 485 MB, 16.4 / 338 ms | 569 MB, 11.8 / 46.9 ms | 658 MB, 12.2 / 34.6 ms |
| full-1000 | 543 MB, 24.1 / 870 ms | 667 MB, 19.6 / 53.8 ms | 832 MB, 21.4 / 76.5 ms |
| *Infinite Descent* ×2 | 445 MB | 1,119 MB | 1,497 MB |

infdesc-x2 fits in the 7 GB runner in every mode, peaking at 1.5 GB in high-performance mode. Its
typing rows have no latency: `mem.py --edit` on the doubled book recorded 3 keystrokes and no edited
page, so only its peak RSS is reported. The M5 Pro T7 run's Balanced peaks per document are the
last column of §1's T7 table (154–655 MB).

## Commands

```sh
# hosted: every shard (a new workflow is dispatchable once it is on main; before that, push to bench-readme/**)
gh workflow run bench-readme.yml -f ref=<sha> [-f shards=cold-small,unicode] [-f reps=7]
git push origin <sha>:refs/heads/bench-readme/only-unicode     # one shard by branch name

# what a shard runs (INCR_BENCH_DIR=/tmp/ib, MacTeX 2026 first on PATH, release binaries in target/release)
tools/incr-bench/mkeng.sh ref && python3 tools/incr-bench/mkdocs.py && python3 tools/incr-bench/mkbook.py infdesc infdesc-x2
python3 tools/incr-bench/bench_readme.py OUT.jsonl --docs hw1,hw2,plain-100,... --reps 7
python3 tools/incr-bench/bench_readme.py OUT.jsonl --set unicode --reps 9
python3 tools/incr-bench/unicode.py target/release/flashtex-host-unicode tools/incr-bench/unicode-corpus/b04-fontspec-11pages.tex --keys 40
python3 tools/incr-bench/coldopen.py /tmp/ib/ref DOCDIR main.tex open-DOC.jsonl --reps 7 --tag open
python3 tools/incr-bench/coldopen.py /tmp/ib/ref DOCDIR main.tex reopen-DOC.jsonl --reps 8 --keep-aux --keep-s0 --tag reopen
MODES_INFDESC=/tmp/ib/docs/infdesc-x2 MODES_LIMIT_GB=6 tools/incr-bench/modes.sh ref
python3 tools/incr-bench/bench_readme_table.py cold OUT.jsonl...   # the tables

# M5 Pro: T7
scripts/t7-reference.sh --no-build --max-load 4 --wait 300 --out DIR \
  --docs plain-10,full-10,plain-100,full-100,plain-300,full-300,plain-1000,full-1000,beamer-5,beamer-30,infdesc
```

## Files

- `tables/`: every table, as `bench_readme_table.py` and `modes_table.py` print them: `cold-classic.md`
  (with the memory table), `cold-unicode.md`, `unicode-keys.md`, `open-reopen.md`, `modes.md`.
- `hosted-raw.tgz`: every shard's artifact. That is the JSONL (one line per run, with its UTC time
  and load1), the per-shard table and `machine-*.txt` (runner, toolchain and TeX versions, disk,
  load), for both runs.
- `t7-m5pro/`: the T7 evidence directory `t7-reference.sh` wrote (`README.md`, `table.md`,
  `summary.json.gz`, `raw.tgz`, `environment.txt`, `t7-output.txt`), `power.txt` (`pmset` at start
  and end), and `infdesc-partial.md`.
