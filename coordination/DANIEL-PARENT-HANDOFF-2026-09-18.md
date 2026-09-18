# FlashTeX — session handoff, 2026-09-18 ~07:00Z

Commander: **daniel-parent** on mac-m5pro-dq222. Session began as a real-user corpus test
and became a bug-fixing run. `main` advanced from `931d7009` to `0f3f86508` (9 PRs),
with a 4-PR batch staged and tested but not yet pushed (see **Immediate next steps**).

## Headline

The owner supplied `/Users/dqi26/ross` — 20 of his own LaTeX homework files — and asked
whether FlashTeX renders them. **It does: 20/20 produce a PDF in 0.0–0.2 s each, against
pdflatex's seconds, with no crash, no hang and no blocker.** A separately collected corpus
of 30 real-world documents (CTAN canon, arXiv, university homework, olympiad camp handouts)
also renders 30/30.

So the engine is healthy. What the corpus exposed is **wrong output**, and the worst of it
is wrong output with **no diagnostic at all**: a 50-frame beamer deck silently became 2
pages; every page of every document silently lost its running header.

**Crash-free is verified, not assumed:** 50 real documents plus 72 adversarial cases
(32-deep nesting, self-recursive macros, catcode abuse, invalid UTF-8, embedded NULs,
1000-deep braces) — zero panics, zero hangs. FlashTeX actually *out-recovers* pdflatex on
unmatched braces, crossed environments and a missing `\end{document}`.

## Merged to main this session (9)

| PR | what |
|---|---|
| #840 | TAB catcode 10 — root cause of #839 and #836 |
| #853 | re-pin `vendor/tex-expansion` — made #840 real for users |
| #850 | vendor pin integrity + drift check |
| #858 | KOMA-Script typearea (superseded #847) |
| #838 | `\0`–`\9` control-symbol diagnostics |
| #845 | `list` environment and its lengths |
| #844 | `\text`/`\boxed` in text mode |
| #865 | unclosed env nesting reports only the innermost, at any depth |
| #867 | class-scope data model for completion |

Measured effect of the #853 re-pin on the owner's corpus: page counts matching pdflatex
went **16/20 → 18/20**, and `U+0009` warnings went **45 → 0**.

## Immediate next steps

1. **Push the staged batch.** `~/flashtex-wt/batch-ag` at `743d125ea` has #849 (fancyhdr),
   #866 (tikz `\parindent`), #860 (glyphless body) and #862 (proof QED span) merged, with
   generated artifacts regenerated from the fully merged source and Mac sync passing. Debug,
   release and render-pipeline suites were running at handoff — **confirm all three green,
   then fast-forward `main`**. It contains one hand-written integration fix (below).
2. **Then re-pin `vendor/compiler`.** This is the last action and must stay last: drift went
   15 → 30 commits as tonight's merges landed, so any earlier re-pin is immediately stale.
   #864 exists but targets `f189d339` and needs retargeting to final main.
3. #869 needs the multi-file gating decision (below) before it can ship.

## Landmines — read before touching anything

- **`vendor/compiler` is 30 commits behind.** `crates/render-pipeline` and `flashtex-cli`
  build against the frozen snapshot, so a fix merged into `crates/compiler` reaches **no
  user** until a re-pin. Do not close an issue on the strength of a merge — verify through
  the built CLI. `scripts/check-vendor-pins.sh` (merged as #850) reports this automatically.
- **Verify BOTH debug and release.** `cargo test` alone is insufficient: #863 passes debug
  and fails `cargo test --release` on the *core feature of the PR*, deterministically. I
  published it green off a debug-only run. Every lane brief now requires both.
- **`timeout` does not exist on macOS.** Use `perl -e 'alarm shift; exec @ARGV' 60 <cmd>`;
  the hang exit code is **142**, not 124. A `timeout`-based harness returns 127 for
  everything and looks like a total failure.
- **zsh eats `:` modifiers.** `"$SHA:crates/x"` silently becomes `<sha>rates/x`. Always
  brace: `"${SHA}:crates/x"`. This corrupted a `git archive` revspec and, combined with a
  `cd` that failed without aborting, made a re-pin attempt run `find -delete` in the main
  repo (448 tracked files; restored from HEAD, nothing pushed).
- **Never `mdls -name kMDItemNumberOfPages`** — returns null in scratch dirs and fabricates
  a page-count mismatch for every file. Use `pdfinfo`.
- **Corpus `.log` files use `-file-line-error`**, so grepping `^!` undercounts pdflatex's
  own errors to zero. pdflatex itself reports **75 errors** on the owner's corpus; most
  "FlashTeX errors" are FlashTeX correctly flagging the author's broken LaTeX.
- **`pdftotext` reports `\mathbf{U}` as U+1D400**, not ASCII `U`. A naive word-diff reads
  that as content loss. Likewise it tokenises differently when a picture shifts, so
  `node` → `n od e` looks like missing text when the glyph *widths* are identical.
- **LaTeX manuals legitimately print `\command` names.** Always subtract pdflatex's own
  count before claiming a leak — it took a "19 of 30 documents leak" finding down to 11.
- **CI truncates the Mac log** (`ci.yml:171` pipes through `tail -n 400`), so a 1,483-test
  run shows zero `error:` lines. The full log is the **`mac-swift-test-log` artifact**:
  `gh run download <run-id> --repo flash-tex/flashtex -n mac-swift-test-log`. The **iPad job
  uploads no artifact at all** — its failures are unrecoverable from GitHub. Worth fixing.
- **`-halt-on-error` silently shrinks the oracle denominator.** It kills pdflatex on ~10 of
  the owner's documents (their own undefined macros), which is how I reported an
  unreproducible "15/19" instead of the true 16/20.
- **A failing `engine performance` check means rendered output changed, not that anything
  got slower.** Timing checks only fire on the pinned reference host, which no CI runner is.
  Re-record with `crates/perf-bench/scripts/merge_digests.py --case <id>` — a diff tool, not
  an accept-all — from a `--release` build (#667).
- **Merge through an integration worktree with a compile check between each PR.** #849 adds
  `Inline::PageStyle`; #860 adds a `match` over `Inline`. Both individually green; together
  a non-exhaustive-match compile error. GitHub's "mergeable" only checks text conflicts.
  The batch carries a hand-written fix adding `PageStyle` to `inline_span`'s whatsit arm.
- **Regenerate, never side-pick, on a `docs/user/compiler.md` conflict.** It is the
  generated command-count line; post-merge neither side is correct. Run
  `crates/compiler/scripts/render_supported_latex.sh`, then
  `apps/mac/scripts/sync-supported-latex.sh`.
- **Lanes cannot touch `apps/**`.** When a lane regenerates the inventory, the supervisor
  must run the Mac sync or CI's byte-identity gate fails. This broke #844/#845/#849.

## Muse harness — fixed this session, read before dispatching

- **Lanes are now in-sandbox clones, not worktrees.** `~/flashtex-muse-tools/new-lane.sh`.
  A worktree's `.git` file points outside the sandbox, so every git command failed with
  `fatal: not a git repository: (null)` and **no lane could commit** — four lanes did fully
  tested work that I had to hand-commit. Do **not** use `--shared`: `muse.sb` grants Muse
  write access to `~/flashtex-muse`, so a shared object store lets one lane corrupt every
  other lane's history.
- **`git clone` copies the mirror's LOCAL branches**, and the mirror's `main` was four months
  stale. `new-lane.sh` fetches `refs/remotes/origin/*` first. **Always verify a new lane's
  base equals `origin/main` before launching** — one lane silently branched from `f25a1ff1`.
- **Publish only through `publish-lane.sh`.** It refuses to push without `--push`, refuses
  trees carrying sandbox-workaround artifacts, and **refuses a lane whose base is behind
  `origin/main`** (exit 4), printing the files that would be reverted. That gate caught
  **six** lanes that would each have silently reverted merged fixes.
- **Disk: `run-muse.sh` sets `CARGO_TARGET_DIR` per lane.** This reached **184 GB** and took
  the volume to 1.7 GB free, killing two lanes silently. `new-lane.sh` now reaps dead lanes'
  targets, and lanes are capped at `CARGO_BUILD_JOBS=2` (5 lanes drove load to 14.9 with
  224 MB RAM free). Lane clones also grow nested `crates/*/target` dirs — sweep those too.
- **Never trust a lane's own check-in narrative.** Verify against git. Lanes have overstated
  ("all 21 Ross files" when there are 20) and three independently claimed work not done.

## Open issues filed this session (13)

**#833** fancyhdr headers silently missing, 20/20 documents · **#834** `\text`/`\boxed`
false error, 20/20 · **#835** `list` env, `\llap`, font-relative dimensions · **#836**
vertical fill drift, +1 page · **#837** ranked real-user package backlog from two corpora ·
**#839** tab-only line doesn't end a paragraph, 14/20 · **#841** beamer `frame` doesn't
paginate — 50 frames → 2 pages, **no diagnostic** · **#842** KOMA paper size + type area ·
**#843** `\includegraphics` silently dropped; glyphless body produces no PDF; 58.9 s on a
4.5 MB line · **#846** math mode typesets unresolved control sequences literally, 11/30
documents, up to 912 per doc · **#852** under-spacing exposed by the TAB fix · **#857**
`tikzpicture` misses `\parindent` · **#876** marker carried past `\pagebreak`.

**#837 is the product-direction one.** Ranked by how many real documents load each
unsupported package: `microtype` 9, `fancyhdr` 8, `babel` 8, `fontspec` 8, `mathtools` 7,
`mathrsfs` 7, `textcomp` 7, `tcolorbox` 7. `fancyhdr` is the only package that tops **both**
corpora. Caveat recorded there: counts of 5–8 are inflated by `evan.sty` being reused
verbatim across 5 of 30 documents — one preamble, not five decisions.

## Blocked on the owner

- **#869 (Swift completion gating).** The agent made the gate strict —
  `(Some(_), None) => false` — so class-scoped commands are hidden when the class is
  unknown. That regresses multi-file projects: **24 of 66** real corpus `.tex` files have no
  `\documentclass` (they are included chapters), so a user editing a section of a beamer
  deck would stop being offered `\frametitle`. I asked for project-root class resolution with
  a permissive fallback. Needs a ruling if that is not reachable from `commandSuggestions`.
- **Corpus licensing.** Of the 30 collected documents, 15 are `UNCLEAR` (personal GitHub
  homework with no LICENSE). None is committed and none should be until ruled on. The corpus
  lives outside the repo, in the session scratchpad.

## Agents

- **daniel-muse-lead** (lead-1): active through the session, published #838 and others.
- **daniel-muse-lead-2**: **went idle ~02:23Z and has not returned.** Four open PRs: #819
  and #851 mergeable, #816 and #820 now CONFLICTING from stale bases. #851 has CHANGES
  REQUIRED — it fixes spurious diagnostics at 2-level nesting and reintroduces them at 3+;
  the fix is already published as **#865**, so coordinate rather than duplicate.
- **Independent review earned its cost: four of six reviews returned something material, and
  every one was invisible to a green CI run.** #840 had five tests that would all stay green
  if paragraph breaking broke for both tab and space. #850 — my own drift gate — was a
  silently-passing no-op under zsh, exiting 0 having verified nothing. #851 reintroduced its
  own bug one nesting level deeper. #849 shipped `[LE,RO]` rendering twice (the canonical
  fancyhdr idiom), a header on the `\maketitle` title page, a **full recompile on every
  keystroke for documents that never load fancyhdr**, and `\setlength{\headrulewidth}` —
  invalid LaTeX — pinned by a test and shipped as the documented interface.

  The pattern is consistent: **tests that encode the implementation rather than the
  specification**. `positions_edges_and_empty_cases` asserted only "no diagnostic" while the
  position was wrong; `tikz_pipeline.rs` asserted the left-margin placement that was the bug.

## Standing constraints (unchanged)

Muse runs on the Meta API key, never anyone's Claude quota. Muse is public-repo-only, gets
no owner fixture PDFs, and no `vendor`/font-engine/font-resources/paragraph-layout/
math-layout/microtype/tex-boxes/`crates/pdf`/`apps` work. Launch via `run-muse.sh` with
`--disable-sandbox`; never loosen `muse.sb`. **A sandbox denial is a stop condition** — no
renaming, hiding or recreating files to get around one. No force-push, no `git stash`, no
git config changes. Commander owns merging and releases. Tier-1 decisions need a visible flag.

## Update ~09:00Z — corpus, visual sweep, and a 182 GiB storage recovery

### `main` is now `743d125ea` — 13 PRs merged
Added since the first write-up: **#849 fancyhdr** (the 20/20 bug), **#866** tikz `\parindent`
(lands in live `render-pipeline`, so it reached users on merge), **#860** glyphless body,
**#862** proof QED span, **#865** env nesting at any depth, **#867** class-scope model.

### Visual sweep — six NEW defects no numeric check could see
A lane rendered all 20 corpus documents with both engines and **looked at the pages**. 18/20
now match on page count. Every one of these is silent:

| issue | defect |
|---|---|
| **#892** | `\vec` draws its arrow a **glyph-width left** of the letter — glyph 1817 (U+20D7) is a *zero-advance combining mark* `[159.12,159.12]`; pdflatex uses CMMI10 glyph 7, a *spacing* accent `[159.12,164.10]`. Character bboxes and advances are **identical**, which is why text and page-count checks passed. Specific to `\vec`; ten other accents match within 0.02 pt. |
| **#893** | math `\dots` never becomes `\cdots` |
| **#894** | mid-formula `\displaystyle` ignored; `\cfrac` renders as `\frac` |
| **#895** | `\DeclareMathOperator*` uses display limits inline |
| **#897** | a `proof` nested in a list loses its italic head and −7.97 pt of `\topsep` |
| **#898** | TikZ `\coordinate` with a braced expression drops **every later path** using it |

**Method note:** I called #897 a false positive earlier because a document-wide `pdffonts`
showed an italic face present — masked by the *top-level* proof being correct. **Per-span
font checks, not per-document.** Equally, checked-and-clean: tables with single/double rules
at 300 dpi, `array`, aligned/numbered equations across five documents, integrals,
`\underbrace`, nested displays, floor/ceiling, itemize bullets, QED boxes.

### Corpus is now 63 documents, and the headline finding is structural
> **Coverage is validated against the `article` class ONLY. 41 of 63 real documents use
> something else** — beamer, scrartcl/scrbook/scrreprt, amsart, exam, IEEEtran, revtex4-2,
> acmart, elsarticle, llncs, the thesis classes — all with **zero measured class-macro
> coverage**. `coverage.md` reports **tikz at 0/43 = 0%** while tikz appears in 21 of 63.

Ranked unsupported backlog (63 docs): `microtype` 22 · `babel` 20 · **`listings` 18 (new)** ·
`fancyhdr` 17 · `mathtools` 14 · `textcomp` 13 · `fontspec` 11 · `url`/`etoolbox` 10.
Caveat: `acmart`/`exam`/`IEEEtran`/`elsarticle`/thesis classes `\RequirePackage` internally,
so some counts are a class requiring a package, not an author choosing it.

Staged for Muse at `~/flashtex-muse-home/corpus/real` (64 dirs, 264 `.tex`, no PDFs) plus
`corpus/ross`. **Every lane must now render the relevant slice before and after its change**
and report any NEW diagnostic anywhere in the corpus. Two more corpora in collection: up to
100 well-known papers, up to 100 student-facing templates and package manuals.

### Storage: 182 GiB recovered, and why it went undetected
Volume went 554 GiB used → 372 GiB; free 345 → 534 GiB.

| what | size |
|---|---|
| `~/flashtex-wt/merge-target` — one cargo cache whose `deps/` had hit the **65535-entry directory cap** | **93 G** |
| 100 stale cargo targets in **`/private/tmp`** | **77 G** |
| Homebrew cache, `~/.npm/_cacache`, orphaned lane target, 2 merged worktrees, Xcode DerivedData | ~20 G |

**Why it hid:** I was measuring the places I expected growth (`~`, the project dirs) and
never the whole volume. `/private/tmp` held 101 G and never appears in a `du` of `~`.
**There are NO APFS local snapshots** — `tmutil` and `diskutil` both report zero, so that
suspect is ruled out.

Also cleared: **36.5 GB of orphaned iOS simulators across 648 processes** (left booted by
`xcodebuild test`; more than every other consumer combined, ~90x what six Muse lanes use),
and four orphaned processes running against directories my own worktree sweep had deleted.

### Guards, now structural rather than remembered
- `resourceguard.sh` (replaces `diskguard.sh`): reaps dead lanes' targets, shuts down
  orphaned simulators when no `xcodebuild`/`xctest` runs, and **skips any path with an open
  file handle**. The old guard watched `/Users/dqi26/flashtex/target-*` — **a glob matching
  nothing.** It ran all night, fired its threshold, found no candidates and guarded nothing.
- `new-lane.sh` reaps dead lanes' target dirs before creating a new one.
- `run-muse.sh` caps lanes at `CARGO_BUILD_JOBS=2` (5 lanes had driven load to 14.9 with
  224 MB RAM free).
- Muse task template: **`kpsewhich`, never `find /`** — a lane burned 67% of a core walking
  the whole volume for `textcomp.sty`; `kpsewhich` answers in 0.1 s.
- `scratchpad/PROTECTED.md` records the do-not-delete set and how to re-derive it.

### Standing rule added tonight
**Verify BOTH `cargo test` AND `cargo test --release`.** #863 passes debug and fails release
*on the core feature of the PR*, deterministically 3-for-3. I published it green off a
debug-only run. #667 is the filed form of this.

## Update ~15:30Z — overnight results, a 263-document corpus, and the architectural finding

### `main` is now `3aaf5b414`
Overnight the leads merged **31 PRs / 38 commits** and closed **9 issues** (#884, #885,
#870, #831, #811, #729, #717, #676) — including this session's #849 fancyhdr, #860, #862,
#866. **The Commander idled ~6 hours during this.** The fleet kept working, which says the
harness rules held unattended, but **uptake decayed to zero and nothing restarted it.**
That is a structural gap: lanes finish, publish and stop, and there is no refill trigger.
A future Commander should treat "fleet idle" as an alarm condition, not something to notice.

### THE finding — #905, and it reframes the whole backlog
**FlashTeX never reads a `.cls` or `.sty` file at all.**
- `\input`/`\include` resolve **`.tex` only** — `crates/compiler/src/parser.rs:4261`
- `\documentclass` has built-in knowledge for **article/report/book/letter only** — `parser.rs:4330-4380`
- `\def`, `\let`, `\newif`, `\expandafter`, `\csname`, `\makeatletter`, `\RequirePackage`,
  `\LoadClass`, `\DeclareOption`, `\ProcessOptions` **do not exist**

So package support is a **hardcoded allow-list of 24 packages and 4 classes**, not a partial
implementation. Measured on 163 documents: **60% open with a class we cannot read**, and
**only 6% (10/163) load nothing outside the allow-list**; the median document loads **12**
unsupported packages.

Day one for a student: **Metropolis** (6.9k★) renders as *boxed paragraphs* because `frame`
is implemented as "draw a box" while `\usetheme`/`\setbeamercolor`/`\frametitle` are not —
silent-wrong, the worst outcome. **Cambridge PhDThesis** (433 forks) dies at line 1.
**Awesome-CV** (28.5k★) renders nothing. **Jake's Resume** (705 forks) is the nearest miss
and the cheapest win: `titlesec` + `tabularx` + an `\input` system-file fallback.

**This is a Tier-1 architectural decision (read `.sty`/`.cls` and interpret the kernel, vs
keep growing the allow-list) — flagged to the owner, NOT dispatched.**

### Our own instrumentation is wrong in both directions
`supported-latex.json` is **not a support oracle**: `\hline`, `\cline`, `\multicolumn`,
booktabs rules, `multirow`, `longtable`, `colortbl` are **implemented** in
`crates/compiler/src/parser/tabular.rs` but absent from the inventory, and `vocabulary.rs`'s
`KNOWN_UNIMPLEMENTED_*` lists are stale and overlap implemented names. **Any ranking built
from `coverage.md` alone is unreliable** — including ones I dispatched. Needs a reconcile lane.

### Corpus is now 263 documents, staged in-sandbox
```
~/flashtex-muse-home/corpus/real/   263 dirs · 2,351 .tex · 453 .cls/.sty · 67 .bbl · 78 MB
~/flashtex-muse-home/corpus/ross/   the owner's own 20
```
100 famous arXiv papers (Maldacena, Shor, Perelman, AlexNet, LLaMA), 100 student-facing
templates/guides/package manuals (Metropolis, Awesome-CV, Cambridge thesis, the 204k-line
pgf/TikZ manual, l2tabu), plus the earlier 63. **No PDFs.** Every lane must render the
relevant slice before and after and report any NEW diagnostic anywhere in the corpus.

**Licensing:** ~121 of 163 are UNCLEAR/not-commit-safe; the papers are arXiv
`nonexclusive-distrib` (arXiv gets distribution rights, third parties do not). Only **8 are
CC-BY**. Recommendation: keep local, regenerate from the 100-ID list, commit nothing.

### Two cheap wins filed from the corpus
- **#906 — consume a pre-built `.bbl`.** 60 of 100 arXiv papers ship one; only 13 ship a
  `.bib`. A `.bbl` is just a `thebibliography` block we already render correctly, so
  resolving `\bibliography{name}` to a sibling `.bbl` recovers the **entire reference list
  for 60% of real papers with no BibTeX implementation**. Lane running.
- **#907 — name plain TeX / LaTeX 2.09.** 10 of 100 famous papers are neither LaTeX2e
  (Maldacena, Shor, Witten, Seiberg-Witten). Wants *recognition* — one early diagnostic that
  names it and suppresses the downstream flood. Lane running.

### Six silent defects from the visual sweep (#892–#898)
Found by rendering all 20 corpus documents in both engines and **looking at the pages**:
`\vec` draws its arrow a glyph-width LEFT of the letter (identical bboxes/advances, so every
numeric check passed); `\dots` never becomes `\cdots`; mid-formula `\displaystyle` ignored
and `\cfrac` renders as `\frac`; `\operatorname*` uses display limits inline; a nested
`proof` loses its italic head and −7.97 pt of `\topsep`; TikZ `\coordinate` with a braced
expression drops every later path.

**Method note:** I called #897 a false positive because a document-wide `pdffonts` showed an
italic face — masked by the *top-level* proof being correct. **Per-span checks, not per-document.**

### #869 blocked — measured, not opinion
The strict class gate `(Some(_), None) => false` hides class-scoped commands when the class
is unknown. **189 of 287 real corpus files (two-thirds) have no `\documentclass`** — they are
included chapters. Five beamer section files use `\frametitle` with no class declared, so
strict gating breaks exactly the case the PR fixes, by ~2:1. Needs project-root class
resolution with a permissive fallback.

### Machine — 182 GiB recovered, guards now structural
Volume 554 → ~370 GiB used; free 345 → 473 GiB. The bulk was **`/private/tmp` at 101 G of
stale cargo targets** (invisible to a `du` of `~`) plus one `merge-target` at **93 G** whose
`deps/` had hit macOS's **65535-entry directory cap**. **No APFS snapshots exist** — that
suspect is ruled out. Also cleared: 36.5 GB of orphaned iOS simulators across **648
processes** (left booted by `xcodebuild test`; ~90x what six Muse lanes use), 28 stale
worktrees (~15 GB), and 4 processes running against directories a sweep had deleted.

**Kept deliberately — 12 `ft-wt-*` worktrees:** 4 with uncommitted files, **8 whose commits
exist on NO remote** (`grok-coverage`, `sqrt`, `ux-cmdr`, `ux-diagnostics`, `ux-editor-keys`,
`ux-find`, `ux-spellcheck`, `ux-wordcount`). Those 8 are *clean* and look abandoned — only
`git branch -r --contains <head>` distinguishes them. **They are worth pushing.**

Guards: `resourceguard.sh` reaps dead lanes' targets, shuts down orphaned simulators when no
`xcodebuild` runs, and **skips any path with an open file handle**. The old guard watched
`/Users/dqi26/flashtex/target-*` — **a glob matching nothing** — and guarded nothing all
night. `scratchpad/PROTECTED.md` records the do-not-delete set.

### Operational
**The `vpc` Codex slot is dead** — out of weekly usage AND the plan is cancelled. Never route
`cx` work to it; it will not come back. The `plus` slot keeps its ≥70% weekly pause floor.

**The Muse mirror corrupts periodically** (`failed to copy file to .../objects/...`). Fix:
`rm -rf ~/flashtex-muse/.git/objects/info/commit-graphs`, then `git gc --prune=now`, then
`fetch`. Seen twice.

## Update ~17:00Z — a 12-PR merge batch, and three self-inflicted traps worth inheriting

### Merged

Integration branch `integration/0918b` off `origin/main`, ten Muse PRs merged with a
per-crate `cargo check` between each: **#909 #912 #903 #911 #913 #915 #929 #917 #901 #902**.
`#901`/`#902` conflicted only on the generated `docs/user/compiler.md` and were resolved by
**regenerating**, never by side-picking. `#914` and `#920` conflicted against main itself
(stale lane base) and were resolved by hand — see the conflict-shape note below.

`#910` no longer fails: the `engine performance` gate was correct to fire, because the PR
deliberately moves the `\vec` arrow. Re-recorded `twelvept-plain` only, with
`merge_digests.py --case twelvept-plain` from a `--release` build. All ten digest keys moved
and they match the ten CI named exactly; `git diff --stat` shows the baseline file as the
only file touched, 25 other cases byte-identical. Pushed as `80d9d6016`.

### Three traps I walked into today — all mine, all cheap to avoid

**1. There is no cargo workspace.** No root `Cargo.toml` exists; every crate under `crates/`
is standalone. `cargo check --workspace` from the repo root fails with *"could not find
Cargo.toml"* — a structural failure, not a code failure. My first merge script read that as
"this PR breaks the build" and **rejected all 12 PRs identically**, which looks exactly like
a real signal. Derive the crates from the diff and check each:
`git diff --name-only HEAD~1 HEAD -- 'crates/*' | cut -d/ -f2 | sort -u`.

**2. zsh eats `:` after a bare `$var` in a double-quoted string.** `"$n:conflict"` becomes
`901onflict` — `:c` is parsed as a modifier. This is the **third** time this bug has cost
real work here (previously `"$pin:crates/$name"` in `check-vendor-pins.sh`, which made it
exit 0 having verified nothing, and `"$SHA:crates/tex-expansion"`, which contributed to a
`find -delete` wiping 448 tracked files). Always brace: `"${n}:conflict"`. bash does not do
this, so a script tested under bash breaks under zsh — and macOS defaults to zsh.

**3. The "keep both sides" conflict that does not compile.** Hit twice today, in
`compiler/src/supported.rs` (#920) and `render-pipeline/src/typeset.rs` (#914), and once
earlier in `parser.rs`. Git picks the conflict region by line similarity, not syntax, so when
both branches append to the same array or `impl` block it anchors on the *fields* and leaves
the shared opening delimiter above the region and the shared closing delimiter below it.
Each side is then a **fragment**, not a complete item, and concatenating them fuses the last
item of one into the first of the other — in `supported.rs` that made one 6-field tuple where
three fields were expected.

> **The tell:** look at the line immediately above `<<<<<<<`. If it opens a delimiter that
> neither side closes, the sides share it, and the resolution is a **bridge** (close the
> first, reopen for the second), not a join.

### Verification gate — restated because it keeps slipping

The lane harness reported `debug ok=94 failed=0` for both `bbl-consume` and
`plaintex-diagnose` and printed **nothing for release**. Absence of a release line is not a
release pass. #863 was published green off exactly this and fails `cargo test --release`
deterministically on its own core feature. Release is being re-run explicitly before either
lane is published.

### Still open

- **#914 / #920** — resolved and compiling locally, not yet pushed.
- **#905** — Tier-1, owner's call, **not dispatched**: read `.cls`/`.sty` vs grow the allow-list.
- **#694** (mac-claude-a) — release workflow signs ad-hoc; `MAC_CERT_P12` / `MAC_CERT_PASSWORD`
  are not set. **Owner item** — secrets, nothing an agent can resolve.
- Seven verified-free lanes dispatched to the Muse leads on #2: **#833 #928 #925 #832 #828
  #924 #857**. #833 (fancyhdr missing on 20/20 ross documents) is the highest real-user impact.
- Eight worktrees still hold commits that exist on no remote; worth pushing before any cleanup.

## Update ~17:45Z — batch on main, Muse refilled by the Commander, and two integrity findings

### Landed

**`main` is at `5f3e49000`** — ten Muse PRs (#909 #912 #903 #911 #913 #915 #929 #917 #901 #902),
verified before push with both profiles across all three touched crates:

```
compiler        debug 39 / 0    release 39 / 0
render-pipeline debug 34 / 0    release 34 / 0
vector-graphics debug  4 / 0    release  4 / 0
```

**#934** (`.bbl` consumption, #906) and **#935** (plain TeX / LaTeX 2.09 diagnosis, #907) published.
**#910** pushed with `twelvept-plain` re-recorded — ten keys, exactly the ten CI named.

### Muse went to ZERO and the leads did not refill it

All three lanes checked in between 17:00Z and 17:07Z; live `muse-bin` hit 0. I posted a refill
trigger on #2 and, when nothing started, **launched three lanes directly** rather than wait:
`fancyhdr-headers` (#833), `newenvironment` (#928), `uline-bullet` (#924). Recipe, since it is
not obvious from the tooling alone:

```sh
MUSE_LANE=<lane> ~/flashtex-muse-tools/run-muse.sh exec \
  --api-key-stdin --model muse-spark-1.3-contributor --reasoning-effort high \
  --disable-sandbox --disable-approval \
  --prompt-file ~/flashtex-muse-home/prompts/<lane>-1.md
```

`--disable-sandbox` refers to **Muse's own** sandbox; the outer `muse.sb` still applies and must
never be loosened. Run `~/flashtex-muse-tools/selftest.sh` first — it must print `SELFTEST OK`
and costs nothing. Prompts are built from `muse-task-template.md` by substituting `{{CONTEXT}}`,
`{{TASK}}`, `{{SCOPE}}`, `{{OUT_OF_SCOPE}}`, `{{ACCEPTANCE}}`.

**The Commander can and should launch lanes when the leads stall.** Waiting cost six hours once
already.

### Two integrity findings

**#936 filed — `vendor/vector-graphics` is pinned to an unreachable commit.** `3593209be` is on
no branch, no tag, no remote ref; `git fsck --unreachable` lists it. It survives only in this
machine's object store and is one `git gc` from being gone. In a fresh clone the pin resolves to
nothing, so the vendored snapshot's provenance **cannot be verified by anyone**. That is worse in
kind than the nine crates that merely lag, whose pins are at least ancestors of main.

**The nine lagging pins are themselves overdue.** 41 compiler commits are inert for users. #896
does the re-pin and fails only `engine performance`, which for a re-pin is expected; I gave
mac-claude-a the exact digest list (`lecture-notes`, 9 keys). Note what that narrowness means:
**one case out of 26 moving is a statement about the perf corpus's coverage, not a reassurance
about the re-pin.** None of those 26 documents loads fancyhdr, soul or csquotes, so most of what
the re-pin makes live is invisible to the gate by construction.

### Visual-oracle methodology — three traps, all of which produced a convincing false reading

1. **Compare baselines, not bbox tops.** A bbox top is `baseline − ascent`, and ascent is a font
   property. FlashTeX's bullet is `LatinModernMath-Regular` where pdflatex's is `SFRM1000`, so
   bbox tops differ ~3.4 pt for *identical* output. Use PyMuPDF `span['origin']`. Measured that
   way, a tab-indented `itemize` matches pdflatex to **0.03 pt on every span**.
2. **Never match on extracted PDF text for math.** FlashTeX emits U+1D4AE `𝒮`, U+2124 `ℤ`,
   U+2223 `∣` where pdflatex emits ASCII `S`, `Z`, `|`. Ours is the more truthful encoding —
   pdflatex's ToUnicode map claims `S` for a CMSY10 glyph that is a script S — so this will not
   change. Every math-bearing line therefore fails a text comparison for reasons unrelated to
   layout.
3. **Use two extractors before believing an extraction result.** PyMuPDF reported FlashTeX losing
   the space in `𝒮be a set`, which would have been a real copy-paste defect. `pdftotext` on the
   same file gives `𝒮 be a set`. It was PyMuPDF's word-boundary heuristic reacting to the Unicode
   codepoints. I nearly filed it.

Against real ross documents: **page counts match 6/6**, median |dx| is **0.00**, but three of six
show median |dy| of 21–36 pt. Recorded as a **lead, not a measurement** — after trap 2 the aligned
subset is biased toward math-free lines, so line-breaking divergence cannot yet be separated from
spacing divergence. Whoever takes it should align on geometry, not text.

### A merge conflict shape that appears constantly here and must not be resolved by "keep both"

Three conflicts today landed on the same `supported.rs` table and needed **three different**
resolutions. The distinguishing question: *does the line above `<<<<<<<` open a delimiter that
neither side closes?*

- #920 — each side a tuple **body** sharing one `(` above and `),` below → bridge `),\n    (`.
- #914 `typeset.rs` — HEAD's fn body unclosed, borrowing the `}` past the conflict → bridge `}`.
- `inventory-reconcile` — both sides **complete** single-line tuples → plain union; a bridge here
  would have produced an empty tuple.

Guessing wrong yields code that reads correctly and does not compile. That is exactly how conflict
markers got committed into `parser.rs` earlier in this session.

### Invariant tests earn their keep — a worked example

`inventory-reconcile` purges 275 stale `KNOWN_UNIMPLEMENTED` entries and inventories 27 real ones,
but its **durable** contribution is two assertions. Merging main introduced one drift (main gained
`tabularx` via #901; the lane's inventory predated it) and it failed both, for different reasons:

- `environment_and_package_inventory_equals_the_parser_arms` — inventory missing an environment
  the parser accepts.
- `unimplemented_lists_name_nothing_implemented` — once fixed, `tabularx` was both implemented
  *and* listed unimplemented.

Note `vocabulary.rs` names `tabularx` twice: the stale unimplemented list (remove) and the
environment→package map (**keep** — `\usepackage{tabularx}` really is required). Deleting both
would stop FlashTeX telling users which package they are missing.

### Open

- **#914 / #920** — resolved, in test, not yet pushed.
- **#905** — Tier-1, owner's call, **not dispatched**. I commented that the allow-list branch is
  being built anyway (titlesec, etoolbox, calc, iftex, csquotes, cancel); each raises the
  switching cost of the read-the-`.sty` option and none lowers it. Not blocking that work.
- **#936**, **#896** — vendor pins.
- `calc-package` and `jakes-resume` await review; `proof-in-list` slice 2 is approved and idle.
