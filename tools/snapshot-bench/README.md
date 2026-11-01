# snapshot-bench

Standalone benchmark that chooses the checkpoint mechanism for FlashTeX engine v2,
[DESIGN §5.2](../../docs/design/engine-v2/DESIGN.md). Licensed MIT; it links nothing
but `libSystem` and depends on no other crate in this repository.

The gate from §5.2: **a snapshot under 1 ms, and a write barrier that adds at most 3%
to the engine's hot loop.**

## What it models

One flat word space split into TeX's arenas (`mem`, `eqtb`, `hash`, `save_stack`,
`str_pool`, `font_info`), driven by a synthetic "typesetting" op stream whose locality
is taken from Appendix B.1 of the design (`mem` 768k words used, about 58 KB freed per
shipout, 2.4 ms per body page).

Three mechanisms, all over the same state and the same replayed op stream:

| id | mechanism |
|---|---|
| `plain` | `Vec<u64>`, no snapshot support — the slowdown denominator |
| `kernel` | (a) `mach_vm_allocate` + `mach_vm_remap(copy=TRUE)` / `vm_copy` |
| `arc` | (b1) chunk table of `Arc<[u64; 2048]>`, barrier = `Arc::make_mut` |
| `bitmap` | (b2) chunk table of `Arc<[Cell<u64>; 2048]>`, barrier = dirty bitmap |
| `flat-undo-log` | (b3) flat state + dirty bitmap + one-level undo log (hot-loop phases) |
| `flat-undo-chain` | (b3) chained sealed undo logs, redo capture, convergence jump (`chain` phase) |
| `memcpy` | (c) full `memcpy` of all arenas |

`arc` is the shape named in the P0-SNAPSHOT-BENCH assignment; `bitmap` is the shape
named in DESIGN §5.2 ("a software 16 KB-chunk copy-on-write with a dirty bitmap").
Both are measured because they do not cost the same.

## Running

This crate is **standalone**, not a member of the root workspace: it has its own
`[workspace]`, `Cargo.lock` and `target/`, so run Cargo from this directory. Mechanism
(a) uses Mach VM calls and exists only on macOS; elsewhere the crate still builds and the
kernel phases print "unsupported". `SNAPSHOT_BENCH_NO_KERNEL=1 cargo test --release`
compiles and tests that non-macOS path on a Mac.

```sh
set -o pipefail
./run.sh                      # every phase, JSON to out/, tables to stdout
cargo run --release -- help   # individual phases, including `chain` (DESIGN §5.3 restores)
cargo test --release          # correctness self-tests for every mechanism
```

`run.sh` honours `CARGO_BUILD_JOBS` (default 4).

## Reading the numbers

- Every duration is the **median of at least 20 repetitions**; p90 and max are printed
  next to it because the gate is about worst-case keystroke latency.
- The hot-loop columns come in two flavours. `replay` is a bare index-replay loop, which
  over-states a barrier's relative cost because a real engine does arithmetic and token
  dispatch between accesses. `engine` adds a measured amount of dependent integer work
  per access. Judge the 3% gate on `engine`, and read `replay` as the pessimistic bound.
- Chunk/page granularity is 16 KB on this host (`getconf PAGESIZE` = 16384), which is
  also the software chunk size, so (a) and (b) have the *same* copy granularity here.
