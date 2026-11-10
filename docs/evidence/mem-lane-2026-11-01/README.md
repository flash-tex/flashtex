# MEMORY: where the resident host's memory goes, and the page-body fix (2026-11-01)

Lane **MEMORY** (kabir-claude-memory, for the Commander; DESIGN.md §1.2, "memory everywhere",
2026-10-06). **VERIFIED** means measured here, with the script and raw file named. **BELIEF** means
inferred, not measured.

## Method (VERIFIED)

The measurements ran on the NixOS PC's self-hosted runners (`nixos-7800x3d*`), inside
`flashtex.slice` with MemoryMax=10G and `nice -n 10`. They were dispatched by pushes to
`agent/kabir-claude-memory/measure*` branches, whose workflow (`mem-lane.yml`) is not on main.
`scripts/run.sh OUT` builds the release host with `mem-stats`, so that DONE carries the heap by tag
(`crate::memstat`). For each document, `scripts/prof.py`:

1. starts `flashtex-host --socket --s0-cache` on a fresh copy;
2. opens it (`dl3-keys --keys 0`) and types 8 letters 300 ms apart on each of 4 pages;
3. idles 8 s, past Balanced's 2 s idle trim;
4. opens it once more, unchanged.

It records `/proc` (VmRSS, VmHWM, smaps grouped by mapping), DONE's `mem` and the size of the
S₀/S₁ cache on disk.

Units:

- **steady** is VmRSS after the idle;
- **peak** is VmHWM;
- **open** is VmRSS right after the first open.

All values are in MB. With `BASE_REF`, `run.sh` builds main the same way and interleaves the two
engines by document. `scripts/ab.py` prints the A/B. `scripts/hosts.py` sums Rss and Pss over
every `flashtex-host` while four `full-1000` hosts are open at once.

The documents:

- plain-1000 and full-1000 (`tools/incr-bench/mkdocs.py`);
- infdesc-x2 (`mkbook.py`, 1,140 pages);
- the metropolis deck (`fixtures/beamer-v3/metropolis`);
- the 13 arXiv papers of P6-HYPEROPT (every 12th of `arxiv-2025-01`, `scripts/arxiv13.py`).

## Main `3f835f488` (engine code as at `320b360ad`), Balanced

From `raw/main-all.jsonl` and `raw/main-4hosts*.json*`:

| doc | open | steady | peak | S₀ on disk |
|---|---|---|---|---|
| plain-1000 | 482 | 439 | 509 | 18 |
| full-1000 | 686 | 608 | 692 | 24 |
| infdesc-x2 | 1043 | 939 | 1080 | 43 |
| metropolis | 161 | 107 | 183 | 30 |
| 13 arXiv papers | 158–299 | 74–222 | 161–332 | 19–37 |
| 4 × full-1000 at once | | 4 × 606 | sum of peaks 2,482 Rss / 2,397 Pss | |

Four hosts share only about 85 MB (Pss against Rss). Each host's word space holds 27 MB resident,
but the hosts do not share it, so sharing the format would gain at most about 20 MB a host.

Where full-1000's steady 608 MB goes (DONE.mem after the idle):

| part | MB | note |
|---|---|---|
| undo logs (`log`) | 278 | 1,028 checkpoints; 2.30 M deltas (24-byte headers: 55 MB), 51 M words |
| display lists (`dl`) | 108 | the page bodies (97.6), shared by the host's page cache and the writer's memo |
| engine heap (`engine`) | 85 | about 30 of it BOX-MEMO's recordings (heaptrack, `boxmemo.rs`) |
| restore branch, COW, prepare | 29 | |
| word space resident | 27 | eqtb 4.8, mem 6.3, str_pool 4.7, dl_side 4.2, trie 3.3 |
| slab, spare tails | 25 | |

On infdesc-x2 the logs are 585 of 939 MB (62 %), the page bodies 48, and the definition sites
(`diag.rs`, up to four snapshots of about 7 MB) about 45.

By array, the logs' words (`raw/main-logwords-lowmem.jsonl`, measurement build):

| doc | mem | dl_side | pdf_op_buf |
|---|---|---|---|
| full-1000 | 54 % | 31 % | 12 % |
| infdesc-x2 | 68 % | 22 % | 7 % |

A `mem` word in a log that was dead in the state the log restores (inside a free variable-size block
or in the unused zones, the free lists' own words excluded) is 25–28 % of the mem words and 14–28 %
of all the words sealed. That is an upper bound: such a word can be left out of a log only if it
was dead at every checkpoint since it was last written. BELIEF: the shipped-page nodes freed just
before each shipout checkpoint meet that condition.

## The fix: the page body packed in place (`displaylist::Body`)

The host's page cache gzip-packed its body and dropped its reference, but the writer's memo
(`State::memo`) kept the same `Arc`. Nothing was freed, and the packed copy came on top. This was
Low Memory's packing since it landed. On full-1000 in Low Memory, `dl` was 108 MB and the packed
cache 49 MB more, under `other`.

`Emitted.body` is now an `Arc<displaylist::Body>`, packed in place by the packer thread for every
holder. It reads back as the bytes made, compares by SHA-256 when packed, and a memo hit is the same
body (`ptr_eq`). Balanced now packs too (`Profile::pack_pages`; High Performance does not).

Results (`raw/ab-page-body.jsonl`, main and the branch interleaved):

| doc, mode | steady | peak | open | edited-page instructions p50 | keystroke instructions p50 |
|---|---|---|---|---|---|
| plain-1000, Balanced | 440 → **389** | 510 → **456** | 480 → 390 | 31.5 → 31.5 M | 404 → 408 M |
| full-1000, Balanced | 608 → **558** | 691 → **659** | 684 → 622 | 97.0 → 98.3 M | 744 → 747 M |
| infdesc-x2, Balanced | 939 → **915** | 1080 → **1045** | 1057 → 998 | | |
| full-1000, Low Memory | 455 → **339** | 522 → **429** | 477 → 383 | 94.5 → 94.5 M | 557 → 559 M |
| infdesc-x2, Low Memory | 392 → **327** | 519 → **455** | 440 → 386 | | |
| 4 × full-1000, Balanced | 4 × 606 → 4 × 556 | | | | |

Edited-page wall times on the shared PC vary between runs by more than the change (p95 14–37 ms on
main itself), so no latency claim is made. The instruction counts are the comparison.

## Measured and not landed: compact log headers

Branch `agent/kabir-claude-memory/log-headers` codes each delta's 24-byte header as varints and mask
runs, with an index every 16 deltas. It cut the logs by 19, 27 and 51 MB on plain-1000, full-1000
and infdesc-x2. Keystroke instructions rose by 65–83 % (`raw/ab-log-headers.jsonl`: full-1000
744 → 1,224 M), even after the walks that skip most deltas stopped decoding their masks. The
edited page rose 1 %. Not proposed. The cost is the per-delta decode in the walks that run every
keystroke.

## Reproducing

```
# on a measure* branch: scripts/ci.env sets DOCS, MULTI, HEAPTRACK, BASE_REF, KEYS
bash docs/evidence/mem-lane-2026-11-01/scripts/run.sh OUT
python3 docs/evidence/mem-lane-2026-11-01/scripts/ab.py OUT/prof.jsonl
```
