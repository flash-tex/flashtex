# Track A prototype (evidence only)

Licence: **MIT** (FlashTeX's own throwaway code). Its dependencies are the Apache-2.0
`typst` crates at `=0.15.1`. It is a standalone Cargo workspace. It is not part of the
repository's root workspace and nothing links it. Only source is committed, with no build
outputs.

| Path | What |
|---|---|
| `proto/` | `tbench`: a 1,184-line Typst embedder. Modes: `gen`, `bench`, `seeded`, `mem`, `trace`, `census`, `startup`, `dump`, `jumps` |
| `run_bench.sh`, `run_seeded.sh`, `run_mem.sh` | Wrappers that write one JSON row per run with the load average (`raw-a/*.jsonl`) |
| `variants.py` | Makes the d300 ablations (`nooutline`, `nocite`, `plain`, `bare`) |
| `trace_sum.py` | Summarises a `typst-timing` Chrome trace (`raw-a/trace-*.summary.txt`) |
| `px/pxdiff.swift` | Core Graphics rendering of the typst-pdf page vs Core Text drawing of the display list |
| `px/pdfpos.py` | Re-derives glyph origins from the PDF content stream (`pdfpos`; add `sp` to round to sp) |
| `px/parity_all.sh`, `px/variants.sh` | Pixel-parity matrix (`raw-a/pixel-parity.jsonl`) |
| `px/cffcheck.py`, `px/runshift.py` | CFF hint comparison; per-run sub-pixel shift analysis |

## Build and reproduce (macOS, Rust ≥ 1.92, Python 3 with `pikepdf`, `fontTools`, `numpy`, `Pillow`)

```sh
cd prototype
cargo build --release --manifest-path proto/Cargo.toml   # binary: proto/target/release/tbench
T=proto/target/release/tbench

# Documents (A4, 11 pt, outline, bibliography, numbered equations, refs, figures)
$T gen docs/d10 10;  $T gen docs/d100 100;  $T gen docs/d300 300;  $T gen docs/d1000 1000
$T gen docs/c300 300 14          # a #pagebreak() every 14 sections
python3 variants.py docs/d300    # ablations

# Latency (40 keystrokes per location; evict(10) after every compile)
./run_bench.sh bench.jsonl 40 10 d10 d100 d300
./run_seeded.sh seeded.jsonl 40 time d10 d100 d300 c300 d1000
./run_seeded.sh validate.jsonl 16 validate d10 d100 d300     # seeded == standard page hashes
./run_mem.sh mem.jsonl d300 1000 10 seeded                    # 0 = no eviction

# Pixel parity (d10 page 2)
$T dump docs/d10 1 px/p2.json    # page index is 0-based here; writes p2.pdf + p2.single.pdf + font blobs
python3 px/pdfpos.py px/p2.json px/p2.pdf 2 px/p2-pdfpos.json
python3 px/pdfpos.py px/p2.json px/p2.pdf 2 px/p2-pdfpos-sp.json sp
swiftc -O px/pxdiff.swift -o px/pxdiff
./px/parity_all.sh               # also expects the d300 page-150 set (dump docs/d300 149 px/d300p150.json)
```

The census needs Typst's test suite and assets: `$T census <typst checkout>/tests`, with
typst-dev-assets v0.15.1. Override the binary or the document directory with `TBENCH=` and
`DOCS=`. Sources are copied unchanged from the original job directory. The only changes are
relative paths in the shell wrappers, plus `[workspace]` and `license` in `Cargo.toml`.
