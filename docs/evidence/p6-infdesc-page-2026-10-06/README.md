# P6-INFDESC-PAGE: where *Infinite Descent*'s edited page goes (2026-10-06)

This is lane **P6-INFDESC-PAGE**, Step 1. It is P4-critical: the owner's target is ≤ 20 ms from key to commit while typing at 50 ms on *Infinite Descent ×2* (DESIGN.md §1.2, #1644).

**Starting point.** P6-ENGINE-SPEED's profile (main `f8e671dca`) found that an edit in `equivalence-relations.tex` restarts 112 bytes before the edit. Even so, the edited page costs about 800 M instructions, against about 90 M for a full-1000 page. 63 % of that is macro expansion. This directory answers which TeX-level work that is.

**Machine and engine.**
- The NixOS PC, shared and loaded (load 13–68 during the runs). **Every number here is instructions retired by the engine thread**, which do not move with load.
- Main `dc078d7a5`, plus this lane's profiler change (the commit that adds this directory).
- The document is the pinned `infdesc-48825c5` (`tools/parity/corpus/books.json`): `infdesc.tex`, 592 pages. *Infinite Descent ×2* repeats the same pages, so it has the same per-page costs.

## Method

`flashtex-host --socket` with `--s0-cache`, driven by `dl3-keys`, as the app drives it. `scripts/sites.sh` runs one resident host and, for each site, types 3 letter keystrokes at a fixed line of a chapter (`--edit FILE --line N --page P`, 300 ms after each `DONE`).

The host's macro profiler (`src/macroprof.rs`, extended here) does the measuring:
- `FLASHTEX_MACRO_PROFILE=FILE` in the host profiles each edit's window, from the engine's resumption after the restore to the edited page's shipout (the `FLASHTEX_PERF_MARKS` interval). Each window is written to `FILE.N`.
- `FLASHTEX_MACRO_PROFILE_CLOCK=instr` makes the clock the thread's retired-instruction counter. An earlier run with the time clock charged descheduled time on this loaded machine to whatever macro was running (`pgfsysprotocol@invokecurrentprotocol` 48 % "self" for 31 calls).
- `FLASHTEX_MACRO_PROFILE_ROOTS=...` charges every instruction to the innermost listed construct then active: a breakdown with no double counting.

The window's `instr=` header comes from the thread's own counter. It includes the profiler's cost: about 1–2 % (`total` against `instr`).

```sh
# on the PC (ftx-run: flashtex-agents.slice); IB=~/ib-infpage, engine prof2 built by tools/incr-bench/mkeng.sh
FLASHTEX_MACRO_PROFILE_CLOCK=instr PROF=1 ENG=prof2 \
FLASHTEX_MACRO_PROFILE_ROOTS="fb@sizeofframe,fb@putboxa,centerline,fb@put@frame,MakeFramed,@outputpage,tikzcd,endtikzcd,tikzpicture,endtikzpicture,process@envbody,@thm,@xfloat,@endfloatbox,index,label,cref,Cref,hyper@anchorstart,refstepcounter,@sect,@ssect,@makechapterhead,item,@item" \
  ~/.local/bin/ftx-run 7200 scripts/sites.sh inf3 ~/ib-infpage/src-inf infdesc.tex 3 \
  book/relations/equivalence-relations.tex:430:208 book/relations/equivalence-relations.tex:426:207 \
  book/relations/equivalence-relations.tex:436:208 book/sets/sets.tex:195:100 \
  book/functions/functions.tex:124:126 book/probability-theory/discrete-probability-spaces.tex:13:415
python3 scripts/roots.py ~/ib-infpage/out/inf3          # the table below
python3 scripts/proftop.py ~/ib-infpage/out/inf3/prof.1 50 --incl
```

`raw.tgz` holds `inf3/`, with every window's profile (`prof.N`), the `dl3-keys` lines (`siteN.jsonl`), the host's stderr and the perf marks. It also holds the pdflatex trace of one `\fb@sizeofframe` call (`t.tex`, `m.txt`: `\tracingassigns`; `cm.txt`: `\tracingcommands=3`).

## Results (VERIFIED: instruction counts, these runs)

Each row is a keystroke's window (the warm-up keystroke is left out where its restart differed). Columns are the innermost construct; M = 10⁶ instructions.

| site (line, 0-based page) | window | `\fb@sizeofframe` | frame draws (`\fb@putboxa`) | `tikzcd` + other pictures | `\@outputpage` | rest |
|---|---:|---:|---:|---:|---:|---:|
| equivalence-relations.tex:430 (theorem body), p. 208 | 846–852 M | 115 (14 %) | 77 (9 %) | 206 + 16 (26 %) | 112 (13 %, 2 pages) | align 72, other 271–278 |
| :426 (prose just before that theorem), p. 207 | 125–126 M | 40 (32 %) | 38 (30 %) | 2 | 58 (46 %) | 12 |
| :436 (the proof that follows), p. 208 | 639–681 M | 37–74 | 36 | 205 + 8–11 (33 %) | 54 | align 72, other 221–264 |
| sets.tex:195, p. 100 | 656–746 M | 306–343 (44 %) | 152–188 (22–24 %) | 33–39 | 56–95 | 73–103 |
| functions.tex:124, p. 126 | 116 M (432 M first) | 40–203 | 0–80 | 2–19 | 57–96 | 44–49 |
| discrete-probability-spaces.tex:13, p. 415 | 240–243 M | 82 (34 %) | 40 (17 %) | 8 | 60 (25 %) | 71–86 |

**The first keystroke at a new place** cost 6.07 G (`prof.0`). It restarted at page 200, not mid-page 207, and typeset 9 pages: the open's checkpoints there had been thinned. Over those 9 pages, `\fb@sizeofframe` was 2.47 G (40 %) and frame draws 1.16 G (19 %).

### Why the window is long (VERIFIED from the profiles and the source)

The restart is close to the edit. The cost is everything TeX must do before the edited page can ship:
- **(a)** A framed theorem's body (ntheorem `[framed]` on framed.sty, with a TikZ `\thmbox` frame, `book/includes/theorems.tex`) is held in `\@tempboxa` until `\end{theorem}` runs `\fb@put@frame`.
- **(b)** The page builder needs the item that overflows the page. At line 430 that is the whole proof after the theorem (lines 433–464, its own framed box) and then the `tikzcd` diagram after it (lines 466–478, 205 M by itself).
- **(c)** A restart in mid-page re-ships the page before when the box does not fit there (`\eject` in `\fb@put@frame`), so two output routines run.

### `\fb@sizeofframe` (VERIFIED: framed.sty, pdflatex trace)

framed.sty measures how much a frame adds by **drawing it**, the whole `\FrameCommand` (here a TikZ picture with a node and a rule), around a 5 in rule box. It keeps two `\global` dimensions and throws the box away:

```tex
\def\fb@sizeofframe#1{\begingroup
 \setbox\z@\vbox{\vskip-5in \hbox{\hskip-5in
   #1{\hbox{\vrule \@height 4.7in \@depth.3in \@width 5in}}}%
   \vskip\z@skip}%
 \global\fb@frw\wd\z@ \global\fb@frh\ht\z@
 \endgroup}
```

- **How often.** Once in `\MakeFramed` and again in `\fb@put@frame` (once more per split piece). Each call is 37–40 M instructions, about the cost of drawing the real frame.
- **Over a cold pass** it is **40.5 % of all macro time** (COLD-SPEED's Mac profile, `docs/evidence/cold-speed-2026-10-04/raw/macroprof-top300.tsv`: 3,386 calls, 25.4 of 62.8 s).
- **What one call changes (pdflatex, `m.txt`).** 623 global assignments, almost all of them pgf's scratch registers and macros (`\pgf@x`, the soft-path buffers, the colour macros, the node `box`'s anchors). **No node escapes.** Only two of them depend on where the call is: the picture serial `\count367` (N → N+1) and `\pgf@sh@pi@box` = `pgfid`N+1.
- **Consequence.** While a user types, the same call at the same place sees the same N, so a memo keyed on exact reads hits. Across the calls of a cold pass N differs, so a cold pass would gain only once the serial is handled.

### The output routine

`\@outputpage` costs 54–60 M per page here (95 M on a page with more marks). Within it, LaTeX's shipout hooks run (`\use_ii:nn` self time, 21 M on the prose site), as do fancyhdr's head and foot (`\f@nch@head`, `\f@nch@foot`, `\f@nch@saveclr@parhook`) and geometry's crop marks (`showcrop`).

## Step 2: proposed fixes, ranked (PROPOSED, not measured)

1. **Measure memo** (DESIGN.md §12 P6 "picture memoisation", §5.6 guarded intrinsics) for macro calls that build only boxes they discard, such as `\fb@sizeofframe`.
   - What qualifies: the call is group-balanced, appends no node to any list outside itself, and has no external effect.
   - How it works: it is recorded once. Afterwards only its net global writes are replayed, provided every read still holds, and it fails closed.
   - Gain: −14 to −44 % of these windows; up to −40 % of a cold pass once the serial is handled.
2. **Draw memo** for pictures that append nodes. These are the frame draws (`\X@framecommand{\box\@tempboxa}`, where the content box is an opaque hole keyed by its dimensions) and `tikzcd`/`tikzpicture` bodies. Gain: −9 to −33 % more.
3. **Output routine.** Its cost is mostly expansion, which is MACRO-REPLAY's domain. The header and footer boxes are draw-memo candidates. Gain: 8–46 %.
4. **Restart points inside long environments.** Not the bottleneck on these sites; see (a)–(c).

Upper bound with 1 and 2: the theorem site goes from 846 to about 430 M, the prose site from 125 to about 45 M, and sets.tex:195 from 700 to about 170 M. The rest is prose, `align` and lists that genuinely need typesetting again (P6-ENGINE-SPEED, MACRO-REPLAY).

**Boundary with MACRO-REPLAY** (#1319, agreed with its proposal):
- Macro replay covers expansion-only macros: eqtb and token state, no nodes.
- This lane covers anything that builds nodes.
- While a box memo records, macro replay is suspended inside it, and a box-memo hit skips the inner calls.
