# L6: raw-speed candidates, measured (2026-09-29)

Lane L6-OPTIMIZATIONS (DESIGN.md §5.6 L6: "profile first; every item needs a measured
win"; §1.2 targets; P-T1 unchanged). Branch `agent/kabir-claude/l6-optimizations`, from
`origin/agent/kabir-claude/l6-hyperref-intrinsics` (184949c69), so every number includes
the hyperref intrinsics. Raw records are in [`raw/`](raw/), the drivers in
[`scripts/`](scripts/) (see *Reproducing*).

**Hosts.** Primary: mac-m5pro-kabir, Apple M5 Pro (15 cores), macOS 26.6.2, rustc 1.98.0,
TeX Live 2026 pdflatex as the oracle. The machine was shared with other sessions all
along: the 1-minute load average was 6 to 20 (up to 85 while other builds ran); every
table gives it. Secondary: the NixOS PC (AMD Ryzen 7 7800X3D, 16 threads, TeX Live 2026
x86_64-linux), runs pinned to one core, also shared (its load is given too).

**Method.** `scripts/bench.py`: every engine compiles each document to a settled `.aux`
in its own directory, then each repetition runs every engine once in a rotating order;
medians. On macOS each run is wrapped in `/usr/bin/time -l`, which also reports the
Apple PMU's *instructions retired* and *cycles elapsed* for the process. Instructions do
not move with load at all and cycles move far less than CPU seconds, so small effects are
judged on those two; CPU seconds are given for comparison with pdflatex. Linux reports
CPU seconds only (no PMU access there). Engines are release builds of one source state
(`scripts/snap.sh`) with one option changed (`scripts/variants.sh`). Documents: P4-L2-L3's
generator (`plain-*`: article, amsmath; `full-*`: plus hyperref, siunitx, cleveref,
xcolor, footnotes, cross-references) and a one-line `hello.tex`.

## Result

| document | pdflatex | before (184949c69) | **after, default build** | **after, PGO build** |
|---|---|---|---|---|
| plain-1000 | 1.17 s | 1.51 s (1.29× pdflatex) | **1.08 s (0.92×)** | **0.94 s (0.80×)** |
| full-1000 | 5.58 s | 3.79 s (0.68×) | **3.18 s (0.57×)** | **2.71 s (0.49×)** |
| plain-100 | 0.33 s | 0.30 s | 0.25 s | 0.23 s |
| full-100 | 0.94 s | 0.66 s | 0.62 s | 0.51 s |
| hello (1 page) | 0.21 s | 0.14 s | 0.13 s | 0.12 s |

CPU seconds (user+sys), median of 7 interleaved runs, macOS, load 8.1-10.0
(`raw/bench-mac/bench-final.jsonl`). In cycles: plain-1000 6.44G → 4.62G → 4.04G
(pdflatex 5.01G); full-1000 16.10G → 13.50G → 11.52G (pdflatex 23.70G). *Default build*
= `cargo build --release` of this branch (commits 8a5299f9f, 8ebb242c9, b3d223431);
*PGO build* = `scripts/build-engine-dist.sh` (f84bd7eae). Both write byte-identical PDFs
and logs to the starting point's on 89 documents (*Gates*).

## Every candidate

Gains are the median change of cycles (instructions) against the reference named, macOS
unless marked. "Noise" means within the run-to-run spread of that measurement (about ±3%
in cycles on this loaded machine; instructions ±0.5%).

| # | candidate | measured effect | verdict | reason |
|---|---|---|---|---|
| 8a | **`with_fonts` rebuilt the font state per character** (found by the profile) | plain-1000 **−20% cycles, −29% instructions**; full-100 −6% (load 17-20, `bench-fix.jsonl`) | **adopt** (8a5299f9f) | `isscalable` → `with_fonts` → `std::mem::take` made and dropped a `Fonts::default()` (three `Arc`s) for every character shipped out: 20% of plain-1000 (`raw/profiles/plain-1000-base.txt`) |
| 8b | `isscalable`/`hasfmentry` fast path once the map entry is known | plain-1000 −2.7% instructions, −5% cycles (noisy); full-100 −0.5% instructions | **adopt** (same commit) | per-character `with_state` + swap avoided; same result by construction |
| 3 | **bounds checks on array reads** (generator + `src/ix.rs`) | reads unchecked vs checked, same source: plain-1000 **−5.2% cycles (−11.0% instr.)**, full-1000 **−7.7% (−10.8%)** (load 7-9, `bench-u1.jsonl`); unchecked writes too: a further 1-4% of instructions, no measurable cycles; Linux x86: plain-1000 −3.2%, full-1000 −1.3% CPU (`raw/bench-linux/bench-linux-3.txt`) | **adopt** (8ebb242c9) | see *Bounds checks* below; writes stay checked (they are the write barrier) |
| 2 | **PGO** (trained on the 83 parity fixtures + 10/100-page documents; tested on 300/1,000 pages) | on top of the default build: plain-1000 **−12.6% cycles (−12.7% instr.)**, full-1000 **−14.7%**, full-100 −17%, hello −7% (`bench-final.jsonl`); an earlier independent training: −10.6% / −12.0%. Linux x86 (Zen 4): **0%** (the profile applies: 0 engine functions without profile data; `perf-plain-1000.txt`) | **adopt for the macOS product build** as `scripts/build-engine-dist.sh` | output byte-identical; see *PGO* below for reproducibility and CI cost |
| 1a | `lto = "fat"` + `codegen-units = 1` | plain-1000 +0.5% (+1.4% instr.), full-300 +2.7% (+2.0%) (`bench-u1prof.jsonl`, load 6-7); with PGO it *lost* PGO's whole gain (u1fatpgo = u1fat, `bench-u1.jsonl`); Linux −0.9% / +0.1% | **reject** | no win; the build takes 48-53 s instead of 30-35 s |
| 1b | `lto = "thin"` | −1.7% / −0.6% (instr. −0.1%); Linux +0.2% / −0.6% | **reject** | noise |
| 1c | `codegen-units = 1` alone | −1.0% / +3.2%; Linux +0.4% / 0.0% | **reject** | noise |
| 1d | `panic = "abort"` | fat+abort vs fat: −0.8% plain-1000, +1.8% full-100 (load 9-18) | **reject** | noise, and the resident host needs unwinding: `checkpoint.rs` stops a run with `resume_unwind`/`catch_unwind` |
| 1e | `target-cpu = apple-m1` | −1.1% / +0.7% (instr. 0.0%); Linux `x86-64-v3`: −0.3% / −1.2% (`bench-linux-4.jsonl`) | **reject** | noise; nothing in the engine vectorises |
| 1f | `opt-level = 3` | already the release profile's value | – | – |
| 4 | mimalloc instead of the system allocator | after 8a, allocation is **0.2-0.7%** of samples (plain-1000 0.7%, full-1000 0.2%); that bounds any allocator's gain | **reject** (not built) | cannot reach 2%; the word space is `mmap`ed, not `malloc`ed |
| 5a | parallel compression of one stream (pigz-style) | – | **reject** | splitting a deflate stream changes its bytes; across streams it is item 6 |
| 5b | the system's zlib 1.2.12 instead of TeX Live's 1.3.2 | bytes identical on 352/352 streams; 60.3 vs 60.5 ms per pass (`raw/zlib.txt`) | **reject** | no gain |
| 5c | zlib-rs 0.6.7 (the zlib-ng port; flate2's fast backend) | 22% faster, but **336 of 352 streams differ** | **reject** | not byte-identical |
| 5d | miniz_oxide 0.8.9 | 350 of 352 differ | **reject** | not byte-identical |
| 5e | libdeflate, zlib-ng proper | not built | **reject** | libdeflate is a different compressor with no streaming API; zlib-ng is what zlib-rs ports (5c) |
| 6 | shipout pipelining (display list + PDF on a second thread) | upper bound, measured: without compression (`\pdfcompresslevel=0`, PGO build, separate runs) plain-1000 **−20% cycles**, full-1000 **−14%**; all of `pdf_ship_out` is 36% / 14% of samples | **later lane** (host/unify) | design sketch below; the engine needs the compressed length and last byte synchronously today |
| 7 | start-up: font map, kpathsea `ls-R` | hello: map parse 44 ms, `ls-R` hash 23 ms, format load 5 ms of ~90 ms; one line buffer for the map parser: hello **−8% cycles (−13% instr.)** (`bench-hello.jsonl`) | **buffer: adopt** (8a5299f9f); **rest: later lane** | reopen already restores the parsed map from the snapshot; `ls-R` is kpathsea's C state, see below |
| 8c | `get_next`, `macro_call`, `end_token_list`, `id_lookup` | self time plain-1000 14.1 / 5.6 / 2.6 / 2.3%, full-1000 26.5 / 11.0 / 5.9 / 2.5% (`raw/profiles/`) | **later lane** (§5.6 items 2, 3, 5) | TeX's own inner loops; the fixes are contiguous token lists, a fast `\csname`, NEON scanning, not a local tweak |
| 8d | intrinsics guard (`flashtex_intr_touch` → `holds` → `same_tokens`) | 2.3% of full-1000 | **later lane** (intrinsics) | compares whole token lists on every write to a watched macro |
| 8e | wrapping arithmetic in the generated code | none: release has no overflow checks, so `wrapping_add` is `add` | – | – |

### Bounds checks (item 3)

`web2rust --index-type crate::ix::U` wraps every subscript of `src/generated/`;
`src/ix.rs` gives `U` an `Index` impl that reads without a check in optimised builds and
an `IndexMut` impl that goes through the arrays' own checked write barrier. Debug builds
(`cargo test` without `--release`, which is how `scripts/gate.sh` runs the engine's tests)
and the new `checked-arrays` feature keep every check. The trip and e-trip configurations
do not pass the option and are unchanged.

What makes an unchecked read acceptable, stated plainly: the subscripts are the ones
pdfTeX computes, and web2c's C, which has no checks at all, relies on pdfTeX keeping them
in range (`mem`/`eqtb` indices are pointers TeX allocated or codes it range-checked when
it scanned them). That is an invariant of TeX's program, not a proof. An out-of-range read
in an unchecked build reads, it never writes; the arrays are regions of one mapping, so a
small overshoot reads a neighbouring array. On M5 the checks were 11% of instructions and
5-8% of cycles (the compiler must reload each array's length after every write through
`&mut Globals`); on x86 2%. Recommended for CI (not done here: the workflows belong to
the CI lane): a nightly lockstep/parity run with `--features checked-arrays`.

### PGO (item 2)

`scripts/build-engine-dist.sh`: instrumented build (`-Cprofile-generate`), the pdflatex
format, every parity fixture and the 10- and 100-page documents compiled twice,
`llvm-profdata merge`, optimised build (`-Cprofile-use`). The release profile is
unchanged. On mac-m5pro-kabir the whole script takes about 2 minutes (36 s + 50 s + 35 s);
a CI runner would take several times that, once per release.

- **Reproducibility.** Two training runs give different profiles (hash seeds, file-system
  state), hence different binaries. Given one profile the build is bit-for-bit
  reproducible: `--profile FILE` rebuilt the same `flashtex-initex` twice (sha256
  cc4376de…) after a forced recompile. The profile path is part of `RUSTFLAGS`, which cargo
  hashes into symbols, so the script always builds from `target/pgo-use/`. A release pins
  its `.profdata` (the script copies it next to the binaries with its sha256).
- **Tooling.** `llvm-profdata` of rustc's LLVM major version: `rustup component add
  llvm-tools`; Xcode's (Apple LLVM 21) read rustc 1.98's (LLVM 22) raw profiles here.
- **Not wired into CI or releases:** the new engine is not shipped by `release.yml` yet.
  Whoever adds it there runs this script (or `--profile` with a pinned profile).
- **Linux x86 (Zen 4): no gain.** plain-1000 17.30G vs 17.28G instructions, 6.63G vs 6.64G
  cycles (`raw/bench-linux/perf-plain-1000.txt`); a build with
  `-pgo-warn-missing-function` in both steps reports no engine function without profile
  data, so the profile is applied and simply does not help there. Linux builds need not
  use the script.

### Item 6: shipout pipelining, design sketch (for the host/unify lane)

Measured bound: zlib is 18.7% of plain-1000's samples and 7.3% of full-1000's
(`write_zip`); compiling with `\pdfcompresslevel=0` removes 20% / 14% of the PGO build's
cycles (`bench-z0.jsonl`; that also drops object-stream compression). All of
`pdf_ship_out` is 36% / 14%. On the §1.2 keystroke path this does not matter: the
host's preview mode already stores streams uncompressed. It matters for exports and cold
full runs.

Why it cannot simply be a thread today: pdfTeX reads three things back from the file
synchronously. (1) `pdf_offset` (`pdf_gone + pdf_ptr`) at every `pdf_begin_obj`, for the
xref, and for `/Length`'s position; (2) `pdf_last_byte` after a compressed stream, to
decide whether `pdf_end_stream` writes a newline before `endstream` (and the host counts
those reads for convergence, `LAST_BYTE_READS`); (3) `write_stream_length` seeks back to
patch `/Length`.

Sketch that keeps the bytes: the engine hands a writer thread an ordered queue of
operations instead of writing: `Raw(bytes)`, `Deflate{bytes, level, finish}` (the same
`deflateInit`/`deflateReset`/`deflate` calls in the same order), `ObjStart(k)` (the
writer records the offset into its own table), and "end stream" (the writer applies the
newline rule itself, since it has the last byte, and patches `/Length`). The engine keeps
no file offsets; the xref, `Output written … (N bytes)` and the end of the run wait for
the queue to drain. Engine-visible state that depends on the compressed bytes must be
audited first (only `pdf_last_byte` and `pdf_gone` were found here). Expected wall-clock
gain: up to the zlib share (0.15-0.2 s of plain-1000, 0.3-0.4 s of full-1000), CPU
unchanged.

### Item 7: start-up (for P4-L5-RESTART)

`raw/profiles/hello-u1.txt` (10 runs of `hello.tex`, ~88 ms each under the sampler):
reading `pdftex.map` (46,380 lines, 5.5 MB) 44 ms; kpathsea's `ls-R` databases into its
hash table (`kpathsea_init_db`, `hash_insert_normalized`) 23 ms; the format 5 ms. The
line buffer (above) took 8% of `hello`'s cycles. A reopen from a persisted snapshot
does not parse the map again (the parsed map is part of the snapshot's `Fonts`), and the
resident host parses it once per process (`MapCache`), so what remains for the ≤100 ms
reopen target is kpathsea's 23 ms per new process. kpathsea is vendored unmodified;
options for that lane: keep the process resident (already the design), or serialise
kpathsea's hash next to the format (a kpathsea change), or build it on a thread while the
host reads the snapshot. Not measured further here.

## Gates

The default build is the same binary at f84bd7eae and b3d223431 (`flashtex-initex`
sha256 096a2112…; b3d223431 only moves a `mod` line). *PGO build*: `build-engine-dist.sh`
at f84bd7eae (cc4376de…) for parity and lockstep, and the script's fresh run at
b3d223431 (0dab041c…) for the identity sweep and the timings.

- **P-T1 83/83, P-T2 83/83** for the default and the PGO build
  (`raw/parity-fixtures-cur/`, `raw/parity-fixtures-dist/`); no fixture below its
  baseline level.
- **Lockstep 260/260** for both (accounting: 0 cases differ).
- **trip** (`raw/trip.txt`, on b3d223431: at f84bd7eae the tex82 scratch build failed on
  the new `mod ix`, which b3d223431 leaves out of it), **etrip** (`raw/etrip.txt`),
  web2rust **drift** (`raw/drift.txt`): pass.
- **Identity:** 80 parity fixtures + 9 benchmark documents, each compiled to a settled
  `.aux`: PDF and log byte-identical to the starting point's for the default build, the
  checked build (`checked-arrays`) and the PGO build: **0 of 89 differ**
  (`raw/identity.txt`, `scripts/identity.py`).
- `scripts/gate.sh pr` (base `origin/agent/kabir-claude/l6-hyperref-intrinsics`):
  `raw/gate-pr-b3d223431.txt` (see the summary at its end). On f84bd7eae it failed only
  on rustfmt of `lib.rs` (fixed in b3d223431) and an evidence script.
- Linux: the same sources build and run on the NixOS PC (x86-64 Linux, gcc 15, rustc
  1.98.1; `raw/bench-linux/`). GitHub's Ubuntu and macOS CI jobs were not run from here
  (the branch has no PR checks yet); nothing in the change is platform-specific.

## Linux x86 (secondary)

NixOS PC, runs pinned to core 3, load 1.3-3.3 (`raw/bench-linux/bench-linux-3.txt`):

| document | pdflatex | before | after (default build) | after (PGO) |
|---|---|---|---|---|
| plain-1000 | 1.21 s | 1.76 s | 1.41 s | 1.41 s |
| full-1000 | 5.79 s | 4.94 s | 4.58 s | 4.58 s |
| hello | 0.14 s | 0.13 s | 0.12 s | 0.12 s |

On x86 FlashTeX still loses plain-1000 by 16%: 17.3G instructions and 6.64G cycles
against pdflatex's 12.1G and 5.72G (`perf stat`). TeX Live's gcc build of pdfTeX is far
denser on x86 than on ARM (19.4G instructions on the M5), where FlashTeX now wins. The x86
profile has the same shape as the Mac's, plus `divide_scaled` at 4.6% (integer division)
and a libm `round` call (0.8%, inlined on ARM). Not pursued: macOS is the product
platform.

## Items handed to other lanes

- **Shipout pipelining / zlib off the engine thread** (item 6): host/unify lane
  (P3P4-HOST-UNIFY), with the sketch and bound above.
- **Cold-start kpathsea `ls-R`** (item 7): P4-L5-RESTART.
- **Intrinsics guard cost** (8d, 2.3% of full-1000): the intrinsics lane.
- **`get_next`/`macro_call`/token lists/`\csname`** (8c): later L6 lanes, DESIGN §5.6
  items 2, 3 and 5.
- **Nightly `checked-arrays` run** and **PGO in the release workflow**: CI/release lane.

## Reproducing

```sh
S=docs/evidence/l6-optimizations-2026-09-29/scripts
python3 docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py /tmp/l6o/docs
bash $S/snap.sh u2                                   # freeze the sources
SRC=/tmp/l6o/src-u2 bash $S/variants.sh u2 u2chk u2pgogen
bash $S/pgo-train.sh u2pgogen && SRC=/tmp/l6o/src-u2 bash $S/variants.sh u2pgo
python3 $S/bench.py plain-1000 full-1000 --engines u2chk,u2,u2pgo,pdflatex --reps 7
bash $S/prof.sh u2 plain-1000 3 && python3 $S/sampletop.py /tmp/l6o/sample-u2-plain-1000-*.txt
bash $S/zbuild.sh                                    # item 5
python3 $S/identity.py base u2 u2chk u2pgo           # outputs vs the starting point
bash $S/gates.sh u2 u2pgo                            # parity, lockstep, trip, etrip, drift, gate pr
```

`variants.sh` names: `base`/`fix`/`fix2`/`u1`/`u2` are source states (the start; the
font fix; plus the `isscalable` fast path; plus unchecked reads; plus the map buffer), the
rest are build options on one of them. The `unchk`/`unchkr` rows in `bench-prof1`,
`bench-profiles` and `bench-pgo` were built with a since-removed measurement feature in
`arena.rs` (reads, or reads and writes, without checks); `src/ix.rs` replaced it and `u1`
vs `u1chk` measure the real thing.
