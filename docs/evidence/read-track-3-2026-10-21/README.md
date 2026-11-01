# READ-TRACK-3: b4's end keystroke under 165 M instructions (2026-10-21)

Lane READ-TRACK-3 follows [READ-TRACK-2](../read-track-2-2026-10-20/README.md). Its aim was
Unicode M3's last §1.2 miss: b4's end keystroke, p95 11.6 ms on the M5 Pro
([unicode-m3-mac](../unicode-m3-mac-2026-10-21/README.md)) against ≤ 11 ms, about 195 M
instructions on the PC. The rule is in [READ-TRACK.md §13](../../design/engine-v2/READ-TRACK.md).

Where and how it was measured:
- **Machine:** the NixOS PC, one host at a time.
- **Builds:** release builds, one tree. "main" is main `8d23cf520`. "Branch" is
  `agent/kabir-claude/read-track-3`: the measured build (`r3c`) is the PR's code; later commits
  change only docs.
- **TeX Live:** TeX Live 2026. **Counts:** callgrind (valgrind 3.27.1); M is a million
  instructions.
- **Scripts:** READ-TRACK-2's (`../read-track-2-2026-10-20/scripts/`: `build.sh`, `prof.sh`,
  `measure.sh`, `tests.sh`, `gate.sh`) and [`scripts/umeasure.sh`](scripts/umeasure.sh), which
  runs every Unicode bench, b5 included, at the end and at EDITPOINT.

## Where b4's end keystroke went (`prof.sh`, keystroke 2, the dump after each page sent)

| part | main | branch |
|---|---:|---:|
| the restore, pages 1–11 sent again | 18.0 | 13.0 |
| page 12 typeset | 61.6 | 59.8 |
| page 13: the test, the jump, page 13 kept | 39.0 | 25.1 |
| pages 14–18 kept | 22.1 | 0.7 |
| page 19 typeset live | 54.1 | 53.6 |
| **to the edited page** | **194.7** | **152.3** |

The counts are of the whole process, so the forwarder's thread is in them. The branch column is build `r3b`, the same code as `r3c` less the test changes and the `same_doc` error rule.
- **A kept page cost 4.4 M**, 3.4 M of it to build the page again from its XDV. Now the jump
  takes the old run's document over where this run's document at the convergence point is made
  from the same inputs (READ-TRACK.md §13). The old pages go with their frames, so a kept page
  costs its frame's copy.
- **`set_from`**, the dormant line's words written into the kept states, cost 7.3 M: it unpacked
  and packed again every delta of each of seven logs. It now packs only the deltas it edits.
- **The forwarder decoded every page** (0.6–0.8 M) to look for items the client did not accept.
  The bench's client accepts none of them. The forwarder now reads the item opcodes without
  decoding the page.
- **The convergence test's chunk maps** hashed with SipHash (about 3 M); they use a multiply and a fold (`ChunkHasher`).

## Unicode (`unicode.py --keys 3 --callgrind`, M to the edited page, then the whole compile)

| row | main k1 / k2 / k3 | branch k1 / k2 / k3 |
|---|---|---|
| **b4 end** (the M3 row, median) | 128.7 / **194.9** / 195.0 | 122.0 / **151.7** / 152.0 |
| b4 end, the whole compile | 757.6 / 401.1 / 401.5 | 734.9 / 357.7 / 357.0 |
| b5 end (report only) | 339.0 / 339.3 / 339.2 | 135.7 / 135.8 / 135.9 |
| b1 EDITPOINT | 66.6 / 66.6 / 66.6 | 66.7 / 66.6 / 66.6 |
| b2 EDITPOINT | 18.6 / 18.6 / 18.6 | 18.5 / 18.5 / 18.6 |
| b3 EDITPOINT | 38.1 / 38.1 / 38.1 | 38.1 / 38.1 / 38.1 |
| b4 EDITPOINT | 131.3 / 82.2 / 82.1 | 130.8 / 82.0 / 81.7 |
| b5 EDITPOINT | 229.8 / 181.3 / 180.4 | 225.0 / 175.4 / 175.4 |
| b1 / b2 / b3 end | 66.6 / 18.6 / 38.1 | 66.7 / 18.6 / 38.1 |

- **b4's M3 row: 194.9 M → 151.7 M (−22 %).** Scaled by main's wall time on the M5 Pro
  (p95 11.6 ms at 195 M), that is about 9.0 ms p95. This is a belief, not a measurement on the Mac.
- **b5's end: 339.2 M → 135.8 M.** b5's end keystroke also jumps over kept pages, and they now
  cost a frame's copy. Its 34.5 ms on the Mac would scale to about 14 ms (belief; the row is not
  gated).
- The EDITPOINT rows are equal or lower. Their edited page lies before the convergence point, so
  the documents differ there and the kept pages are built again from their XDV, as on main.

Cold compiles (M, the whole compile): b1 1783.6 → 1784.3 (+0.04 %), b2 5617.6 → 5618.5
(+0.02 %), b3 14451.2 → 14451.1, b4 7898.9 → 7874.6 (−0.31 %), b5 49800.1 → 49373.4 (−0.86 %).
The decreases are the forwarder's decode.

## Classic (`measure.sh classic`, pinned checkpoints, M per keystroke)

| session | main k1 / k2 / k3 | branch k1 / k2 / k3 | 3 keys, main → branch |
|---|---|---|---|
| c1-lipsum end | 179.5 / 184.6 / 121.4 | 177.8 / 177.5 / 112.7 | 485.5 → 468.0 (−3.6 %) |
| c2-ifnextchar end | 212.7 / 133.9 / 82.8 | 211.8 / 129.4 / 79.2 | 429.3 → 420.4 (−2.1 %) |
| c1-lipsum `{Thr` | | | 1,243.7 → 1,204.8 (−3.1 %) |
| plain-100 `Section 13` | | | 4,273.9 → 4,261.2 (−0.3 %) |
| full-100 `Section 13` | | | 25,449.6 → 25,323.6 (−0.5 %) |

The settles differ by 0.015 % at most. Cold compiles (`cg.sh`, instructions; the PDFs' digests
are main's): plain-100 2,290,938,966 → 2,290,930,949; full-100 8,682,397,617 → 8,682,399,404;
c1-lipsum 933,194,985 → 933,195,114; c2-ifnextchar 612,120,817 → 612,098,058. All within
±0.004 %. Classic gains from the cheaper `set_from` and chunk maps (its dormant jump and its
convergence test use the same arena code).

**Pages re-run per keystroke** are main's in every session: c1 end 8/2/2, c2 end 13/2/2,
`{Thr` 6/17/17, plain-100 102/2/53, full-100 110/110/110.

## Soundness

`tests.sh r3c` on the PC:
- `tests/incremental.rs::a_dormant_line_is_kept_until_it_is_read` passed (Classic).
- `tests/host.rs::a_dormant_line_is_kept_until_it_is_read` passed (Unicode). It now also
  requires that the old documents were taken over, and it has a third case, `hyper`: hyperref,
  sections, labels and references on the kept pages, so the document taken over carries links,
  destinations and outline entries. Each compile's PDF, `.aux` and log equal a fresh process's.
- `rt_end.py --verify` gave `BAD 0` for c1-lipsum end (4 keys), c2-ifnextchar end (4 keys),
  c1-lipsum `{Thr` (3 keys) and plain-100 `Section 13` (3 keys).
- b4 end with `FLASHTEX_UNICODE_CONV_DEBUG`: keystrokes 2 and 3 report `6 pages taken over`, and
  the host's own check holds (`claim at page 12 holds (to page 18)`).
- New unit tests: `page::tests` (`item_ops` against the decoded items, and a truncated body
  fails); `arena::tests::set_from_rewrites_the_later_history` now writes a large value, zero and
  a small one, so the edited delta grows, shrinks or keeps its length.

**Sweeps** (GitHub-hosted, `ref=agent/kabir-claude/read-track-3` at `054017ecb`): `sweeps.yml`
run 39549899488 and `sweeps-unicode.yml` run 39549903578 both passed.
