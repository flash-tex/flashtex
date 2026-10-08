# COLD-OPEN: a first compile's anchor at `\document` (2026-10-06)

The fix for the cause in `../README.md`: on a first open there is no `.aux`, so a keystroke during
the open restarted from the format. NixOS PC, wall times non-reference (CPU-capped shared slice; load
3–36). The measure is the engine thread's instructions.

## Edits during the open, owner's book.tex (VERIFIED, `final-edit2.jsonl`)

A letter typed 6 s into the open (`coldopen.py --edit-at-ms 6000 --edit-line N --edit-viewport`),
main `b76f057f1` against the branch at `cb5ec001c`:

| edit on | the open was at | main | branch |
|---|---|---|---|
| page 61 (behind the open) | page 72–79 | **cold**, "looking up main.aux finds another file now": 5,754 M instructions to the edited page (5.1 s) | **incremental** from page 61's checkpoint: 44 M (123 ms) |
| page 928 (ahead of the open) | page 0–53 | cold: 5,755 M, then every page to 928 | incremental: continues from the open's last checkpoint to page 928 (viewport first) |

The edit ahead of the open still has to typeset up to its page; that is the goal's "show it right after
reaching it". A keystroke that arrives before S₀ waits for S₀ by design (`preempt_after_s0`): the
`queue` instructions of the page-928 row on the branch (1,987 M) are the rest of the preamble.

## The open itself (VERIFIED, `final-anchor.jsonl`, `book-mem.jsonl`)

| book.tex open | passes | engine instructions | peak RSS |
|---|---|---:|---:|
| main | cold, cold | 134.0 G | 1.13 GB |
| branch | cold, incremental (from the `.aux` point) | 129.2–129.4 G (−3.5 %) | 1.12 GB |

Pass 2 restarts at the anchor, not the format, and skips the convergence tests it cannot pass. It keeps
none of pass 1's future. In an intermediate version that kept it, peak RSS was 1.43 GB and `Arena::retain`
cost +10 G instructions (`perf` diff).

## Soundness (VERIFIED, hosted `sweeps.yml`, `sweeps-summary.md`)

At `707f886fa`, every gate passes with 0 bad:
- sound-a 8,800 compiles;
- budget 3,520;
- budget-d 1,130;
- timed 1,720;
- vol 201;
- lookup 90;
- lines 2,930;
- span 31.9 M glyphs with 0 wrong;
- readers;
- sound-c 1,974;
- sound-d 1,682;
- the new **sound-first**: 1,600 compiles, 1,304 verified. These are interleaved edits that interrupt a first compile, with no `.aux`, in its first pass.

main's sound-first rows fail only because main's `soundness.py` lacks `--first-open`.
