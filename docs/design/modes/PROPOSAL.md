# One core, three modes: design proposal (MODES-DESIGN)

Status: **proposal**, for the owner's decision. Author: mac-claude-a (mac-m1max-a), 2026-10-04.
Requested by the owner (2026-10-04, [#1319](https://github.com/flash-tex/flashtex/issues/1319),
"owner decisions, 2026-10-04" item 3): one modern FlashTeX core with three per-document modes.

`docs/design/engine-v2/DESIGN.md` remains the single source of truth and is owned by the
Commander. This document does not change it; where they differ, DESIGN.md wins. §9 lists what
DESIGN.md would need to say if the owner adopts this. Every number below names its source;
**VERIFIED** means re-read from that source today, and anything else is a belief and says so.

---

## 0. Recommendation in brief

| Mode | Language | Matches | Engine | Product label |
|---|---|---|---|---|
| **Classic** | LaTeX (`.tex`) | `pdflatex`, P-T1/P-T2 (DESIGN.md §1.1) | `pdftex.web` → Rust (D1–D3), today's engine | "LaTeX (pdfLaTeX-compatible)" |
| **Unicode** | LaTeX (`.tex`) | `xelatex`, P-T1 + XDV (§4.3) | `xetex.web` → Rust in the **same host binary** ([XeTeX PLAN](https://github.com/flash-tex/flashtex/pull/1489), S0–S3) | "LaTeX, Unicode (XeLaTeX-compatible)" |
| **Modern** | Typst (`.typ`) | `typst` at the pinned version (TY4) | `flashtex-typst-host`, a separate MIT process (DESIGN.md §15.2) | "Typst" |

1. **Three modes, each faithful to a real, external reference.** No mode lays out a `.tex`
   file in a way no real engine does (§1.2).
2. **One core** means one app, one protocol (`display-list-v3.x`), one renderer, one
   incremental runtime, one font index, one package manager, one diagnostics model and one
   switch. It does **not** mean one process for Typst: DESIGN.md §15.2 (TY2) keeps Typst out of
   the GPL host, and this proposal keeps that.
3. **Classic and Unicode share one GPL host binary** (`flashtex-host`) with two engine cores
   side by side, both translated by `web2rust` from unmodified WEB sources, behind one engine
   interface (§5). That is the owner's "no separate TeX" (2026-10-04): no `xetex` or
   `xdvipdfmx` executable is shipped or run; native fonts, shaping, OpenType math and the PDF
   writer live in FlashTeX's runtime.
4. **Classic modernises invisibly**: memoisation, macro replay, persisted state, pipeline
   parallelism, structured diagnostics and the idiomatic rewrite, each gated on P-T1/P-T2
   unchanged (§3). Unicode inherits them through the shared runtime.
5. **Mode is per document**, stored in `flashtex.toml` (`[project] mode`), suggested by
   detection, never switched silently, and shown in the status bar (§4).

---

## 1. Goals and non-goals

### 1.1 Goals

- **G1. Faithfulness per mode.** Classic: P-T1 and P-T2 against pdfTeX 1.40.29 (DESIGN.md
  §1.1). Unicode: P-T1 and XDV identity against TeX Live 2026's XeTeX (PLAN §3), and an export
  gate the owner chooses (Q2). Modern: pixel identity with typst-pdf (DESIGN.md §15.5).
- **G2. One user experience.** Every shared feature (preview, Live Share, diagnostics,
  packages, fonts, stored pages, the switch) behaves the same in all modes, or the UI hides it
  by capability (DESIGN.md §15.6, `HELLO.capabilities`).
- **G3. Shared speed.** The §1.2 latency targets apply to Classic and Unicode alike; Modern
  keeps its owner-accepted relaxed target above about 100 pages (DESIGN.md §15.3).
- **G4. No regression to Classic.** A Unicode or Modern change never changes a Classic box
  dump, PDF content stream or latency gate (the pattern of DESIGN.md §15.9).
- **G5. No separate TeX.** No TeX Live engine binary or DVI driver runs at user time. TeX Live
  *content* (`latex.ltx`, packages, fonts) is still used, from the user's install or the bundle
  (D12).

### 1.2 Non-goals

- **No "modern-but-different LaTeX" mode.** A mode that sets `.tex` files with, say, better
  line breaking, different float placement or different defaults is rejected:
  - **A paper is typeset by other people's engines too.** arXiv recompiles submitted sources
    with its own TeX Live; journal production recompiles them; coauthors open them in
    Overleaf or TeX Live. If FlashTeX's pages differ, the author proofreads a document nobody
    else will see: page limits, figure placement, the line a referee cites, and widow and orphan
    fixes all stop holding.
  - **It breaks Live Share's guarantee.** "Same state, same pins, same read set gives the same
    pages" ([live-collab §5.3](../live-collab/PROPOSAL.md)) would need a fourth reference
    engine that no one else has.
  - **It is the road D1 left.** Hand-written LaTeX semantics reached arXiv L0 = 1.4 % with 70
    hand-ported packages (DESIGN.md §2, D1). A different-by-design dialect has no oracle, so no
    parity gate can catch its bugs.
  - **The need it answers is already met honestly.** A user who wants modern fonts and Unicode
    gets Unicode mode, which matches `xelatex`. A user who wants a modern language gets Modern
    mode, which is openly a different language (Typst) and does not pretend to be LaTeX.
- **No LuaTeX mode now** (Q6).
- **No change to what Classic outputs,** ever, except to fix a difference from pdfTeX.
- **No Typst inside the GPL host** (DESIGN.md §15.1, §15.2).
- **No platform work beyond DESIGN.md §16.**

---

## 2. Architecture

### 2.1 What is shared and what is per mode

```
┌──────────────────────────────── Mac app (MIT, Swift) ─────────────────────────────────┐
│ SHARED  editor (FlashTeXEditorCore; typst-syntax lib for .typ, §15.6)                  │
│         mode resolver + detection + status-bar switch          (§4)                   │
│         preview renderer: CG/CT from display list, stored pages (D6, §5.1)            │
│         Problems + explanations · package/font consent UI · Live Share (collab-v1)    │
└────────────┬───────────── display-list-v3.x + lang-v1 (MIT spec) ──────────┬─────────┘
             │ Unix socket, one host process per open document               │
┌────────────┴──────────── flashtex-host (GPL-2+, one binary) ─────┐ ┌──────┴──────────────────┐
│ SHARED RUNTIME                                                   │ │ flashtex-typst-host     │
│  arena + checkpoints (§5.2) · convergence driver (§5.3)          │ │ (MIT; typst =pinned,    │
│  read-sets, multi-pass, external tools (§5.5)                    │ │  Apache-2.0)            │
│  intrinsics + macro replay (§5.6, MACRO-REPLAY)                  │ │ World: confinement,     │
│  resolver: TeX Live or bundle, format cache (§4.4)               │ │  package lock, fonts    │
│  font index client · display-list writer · PDF object writer     │ │ seeded loop, evict,     │
│  diagnostics side channel · host protocol, watchdog, crash isol. │ │  watchdog (§15.2–15.3)  │
│ ┌─ CLASSIC core ──────────────┐  ┌─ UNICODE core ───────────────┐ │ │ v3.3 writer (E1–E8)     │
│ │ pdftex.web → web2rust       │  │ xetex.web → web2rust         │ │ │                         │
│ │ pdfTeX change files         │  │ change files (84 % shared)   │ │ │ MODERN core             │
│ │ pdfTeX C parts (writet1 …)  │  │ HarfBuzz, FreeType, ICU,     │ │ └─────────────────────────┘
│ │ pdfTeX node iso (§5.3)      │  │ TECkit, Graphite2; OT math;  │ │
│ │ PDF: pdfTeX backend         │  │ XeTeX node iso; PDF: native  │ │
│ │                             │  │ fonts + dvipdfmx specials    │ │
│ └─────────────────────────────┘  └──────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────┘
SHARED MIT crates (linked by both hosts and the CLI): flashtex-display-list, project-manifest,
package-resolver, flashtex-font-discovery.  SHARED DATA: the bundle (GitHub Release assets).
```

### 2.2 Per-mode table

| Concern | Classic | Unicode | Modern |
|---|---|---|---|
| Reference oracle | pdfTeX 1.40.29 (TeX Live 2026) | XeTeX 0.999998 (TeX Live 2026; PLAN §1) | `typst` `=0.15.1`, per-project pin (§15.2) |
| Engine source | `pdftex.web` + change files (§4.1); REWRITE.md may freeze it | `xetex.web` + change files (PLAN §2) | upstream crates, unmodified |
| Format | `pdflatex.fmt` from the user's `latex.ltx` (D12) | `xelatex.fmt`, same rule | none |
| Fonts in the engine | TFM + Type 1 via `pdftex.map` | TFM, plus OpenType/TrueType by name or `[file]` (PLAN S1) | Typst's font book |
| Font in the protocol | `FONT.format` Type 1 / Type 3 / TrueType (§15.1) | `"opentype"` (E1), f64 origins (E2) | `"opentype"` (E1–E8) |
| PDF | ported pdfTeX backend (§6.3) | FlashTeX's own writer: CID/OpenType subsets, dvipdfmx specials (§5.4) | typst-pdf |
| Incremental | L1–L5, L6 (§5) | the same runtime; mode-specific `iso` and handle table (§5.3) | comemo + seeded loop (§15.3) |
| Process | `flashtex-host --mode classic` | `flashtex-host --mode unicode` | `flashtex-typst-host` |
| Licence of the core | GPL-2+; binary GPL v2 or v3 (xpdf) | GPL-2+ in the same binary | MIT + Apache-2.0 |

---

## 3. Invisible modernisations in Classic

**Rule.** A modernisation is admissible in Classic only if pdfTeX could not tell the
difference: P-T1 and P-T2 stay identical on every tier that applies, and terminal and log
output stay identical (DESIGN.md §15.7). Anything that changes a byte P-T1 compares is not a
modernisation. It would be a different mode, and §1.2 rules that mode out.

| Modernisation | What it is | Why the output cannot change | Gate | Where it stands |
|---|---|---|---|---|
| Dependency-tracked memoisation | Read-sets per checkpoint (every control sequence read; §5.5); `.aux` reruns only when a read entry changed; segment memo for large reflows (§5.7); picture memo (P6) | A memo is reused only when every input in its read-set is identical; otherwise TeX runs | P-T1 unchanged; soundness sweeps A, C, D (R5) | read-sets built (#1242); §5.7 and pictures not started (§12 P6) |
| Macro replay | Guarded intrinsics (D9) extended to macros with arguments | "A replay must leave the engine in a state no later computation can tell from the real expansion" (MACRO-REPLAY.md, the governing rule); both paths diffed in CI | MACRO-REPLAY §6.4 gates (a)–(g) with faults | design approved 2026-10-04 (§13); `\XC@col@rlet`: 69,743 calls, 104 distinct argument lists (MACRO-REPLAY §1) |
| Persisted, versioned state | S₀ on disk keyed by read-set (§5.1); format cache keyed by content hash (§4.4); stored pages (#1332); checkpointed C state with a persist version (REWRITE §3.1) | State is restored only under an exact key and version; a mismatch discards and recomputes, never migrates | P-T1 of a restored run equals a cold run; version bump → discard test | stored pages landed; cold reopen 101–365 ms at launch, not met (§5.1) |
| Pipeline parallelism | Encode the display list and the PDF page on a second thread; parallel stream compression and font embedding at export | Typesetting stays single-threaded (D10); zlib is deterministic | P-T2 byte-identical, export | measured candidate (§5.6 item 7) |
| Structured diagnostics | Column, macro trace, help text and warning ranges as a side channel (TY6 item 1) | The side channel never writes to the terminal or log | P-T1 unchanged; diagnostics fixtures | **P5 prerequisite** (§15.7) |
| Idiomatic Rust | Strangler rewrite: C parts now (stage A), the generated engine after a freeze (stage B) | Each module is replaced behind its existing interface and proven identical | REWRITE.md §5 (T0, lockstep, P-T1/P-T2, corpus sweep old/new/pdfTeX, instruction counts, no new `unsafe`) | pilot `writet1` done (REWRITE §6); stage B gated on the DESIGN.md record |
| Safety | Bounds-checked reads in every shipped build (§4.2); no-panic contract (§4.5); sanitizers | A caught error becomes a TeX error, never different output | T6 panic oracle; MEMORY-SAFETY gates | lane MEMORY-SAFETY (#1493 open) |

**Unicode gets these by sharing the runtime**, not by copying them: checkpoints, read-sets,
the convergence driver, the resolver, the format cache and stored pages work unchanged. Three
items need per-core work: the `iso` walk (XeTeX's node layout differs: box nodes are 8 words,
not 9; PLAN §4.4), the intrinsics hooks (in change files; the 84 % shared hunks make most of
them apply), and the native-font handle table, which must be checkpointed with the word space
(PLAN §4.7).

---

## 4. Mode selection UX

### 4.1 Resolution order (highest first)

| # | Source | Example | Notes |
|---|---|---|---|
| 1 | File type | `.typ` → Modern | Fixed; a `.tex` file is never Modern |
| 2 | `FLASHTEX_MODE` / `flashtex build --mode` | CI, tests | Like `FLASHTEX_ENGINE_V3` today (`EngineChoice.swift`, rule 1) |
| 3 | Manifest `[project] mode` | `mode = "unicode"` | Shared through git and Live Share |
| 4 | Magic comment `% !TEX program = xelatex` | TeXShop/TeXstudio convention | Read only when no manifest key; `pdflatex` → Classic, `xelatex` → Unicode, `lualatex` → Q6 |
| 5 | Default | Classic | |

Detection (§4.2) **suggests**; it never decides. A mode change changes the output, so it
always takes one click by the user.

### 4.2 Detection

| Signal | Where it is read | Suggestion |
|---|---|---|
| `\usepackage{fontspec}`, `unicode-math`, `polyglossia`, `xeCJK`, `xltxtra`; `\setmainfont`, `\setmathfont` | Lexical preamble scan in the app (< 1 ms tier, §15.6), including local `.sty`/`.cls` the include scan finds | Unicode, banner: "This document uses fontspec, which needs Unicode mode. Switch?" |
| **Engine truth:** a Classic run stops with fontspec's fatal error that it needs XeTeX or LuaTeX, or with `\RequireXeTeX` | The structured diagnostic (TY6 item 1) | Unicode, as an action on the error's explanation |
| Characters with no `inputenc`/`fontenc` coverage (CJK, Devanagari, Arabic …) and no CJK package | Engine truth: `\tracinglostchars` "Missing character" lines | Unicode, on the warning's explanation |
| In Unicode: pdfTeX-only primitives (`\pdfliteral`, `\pdfximage`), `\ifpdf` branches only, `inputenc` with a non-UTF-8 encoding | Diagnostic / scan | Classic |
| `.typ` file, or `typst.toml` | File type | Modern (opens in Modern, no question) |

Precision is a gate (M5, §7): no Unicode suggestion on the Classic corpora (fixtures, arXiv,
T4), and a suggestion on every document of the Unicode corpus that needs it.

### 4.3 Manifest

```toml
[project]
entry = "main.tex"
mode = "unicode"     # "classic" (default) | "unicode"; .typ entries are always Modern
```

- An unknown value is a warning and Classic, like every unknown key today
  ([project-manifest.md](../../user/project-manifest.md)).
- The Typst version pin stays where §15.2 puts it.
- Today's per-document engine choice (old vs new engine, `EngineChoice.swift`) is app-local on
  purpose: it is a fallback choice, not document semantics. **Mode is document semantics**, so
  it goes in the project, where git, the CLI and Live Share see it (Q4). The old/new choice
  disappears at P5.

### 4.4 Status bar and other surfaces

- The **status-bar engine item** (today `EngineChoiceStatusItem`) becomes the **mode item**:
  "Classic", "Unicode" or "Typst". Its menu shows the source of the mode ("set in
  flashtex.toml", "from `% !TEX program`", "default"), lets the user switch Classic ↔ Unicode
  (writes `[project] mode`, creating `flashtex.toml` if needed), and shows the detection
  suggestion if there is one. View › Mode for This Document mirrors it; VoiceOver announces a
  change.
- **Switching** restarts the document's host in the other mode: a cold compile from the
  other format (§1.2 preamble row, ≤ 400 ms to first page, is the target). Stored pages are
  kept per mode, so switching back shows the old pages at once, stale until confirmed.
- **New Project** gets three entries (extending §15.6's LaTeX | Typst picker): LaTeX, LaTeX
  (Unicode fonts) and Typst, each over its own template list.
- **Live Share:** the hub pins the mode in `join_ack` with the other session pins
  (live-collab §5.3); a peer whose manifest says otherwise is told, never silently compiled in
  another mode.

---

## 5. Unicode and Classic behind one engine interface

### 5.1 One binary, mode chosen per document

- `flashtex-host` links both cores. The mode arrives once per process (`--mode`, and an
  additive `mode` key in `OPEN`/`COMPILE` that the protocol owner assigns, R3). A process
  never changes mode: switching starts a new process. Processes stay one per document
  (§15.2's isolation rule applies to LaTeX hosts as well).
- Only the selected core's globals are allocated. **Binary size and start-up time are
  measured at U3a; they are not estimated here.**
- Build: `crates/flashtex-xetex` lives in its own workspace today so that its build never
  slows the root workspace's gates (PLAN §2). At U3a it joins `flashtex-host` behind a cargo
  feature `unicode`, on in release builds, with its CI jobs path-filtered as Typst's are
  (§15.9). Classic's required checks do not build it unless its paths change.

### 5.2 The engine interface

One Rust trait in the host, implemented by each core:

| Method family | Classic implementation | Unicode implementation |
|---|---|---|
| `init(format, pins)` / `undump` | `pdflatex.fmt` | `xelatex.fmt` |
| `run_until(shipout or restart point)` | `main_control` | `main_control` |
| `word_space()` (arena for checkpoints, §5.2) | `mem`, `eqtb`, … | the same, plus the native-font handle table (PLAN §4.7) |
| `iso(a, b)` (convergence, §5.3) | pdfTeX node types | XeTeX node types (native word, glyph, 8-word boxes) |
| `read_set()` | files, `.aux`, cs reads | the same, plus **font files** (path, SHA-256) and the font-index generation |
| `ship_page() -> display list` | Type 1 / TFM glyphs | E1 OpenType glyph ids + E2 f64 origins |
| `export_pdf()` | pdfTeX backend | FlashTeX PDF writer (§5.4) |
| `diagnostics()` | `print_err` hooks (TY6) | the same hooks in xetex change files |

The shared runtime (arena, checkpoint logs, convergence driver, resolver, format cache,
multi-pass and external tools, host protocol, watchdog) calls only the trait.

### 5.3 Two WEB translations, side by side

- Both cores are `web2rust` output from **unmodified** masters (`third_party/pdftex/pdftex.web`,
  `third_party/xetex/xetex.web`) plus change files. S0 already proves the arrangement:
  `web2rust` translates all 1,721 sections of `xetex.web` with 17 change files, and P-T1 holds
  on 1,364/1,364 cases with XDV identical on 1,361/1,361, while pdfTeX's translation stays
  byte-identical (PLAN §4.1–§4.3, drift test).
- **Side by side, not merged.** One core with `if unicode` branches would put Classic's
  P-T1 at risk for every Unicode change. Two cores keep each one's gates independent. Sharing
  happens below them (the runtime) and in the change files, where a hunk is shared only when
  it applies verbatim to both.
- The arrangement pays already: reviewing the XeTeX translation found two TeX Live `tex.ch`
  fixes ([25.369], [26.449]) that Classic lacks; [26.449] gives a measured difference from
  pdfTeX (PLAN §4.4). That is a Classic fix, made in Classic's change files under the D13/P-T1
  rules.
- **IDIOMATIC-REWRITE:** stage B freezes `pdftex.web`'s translation and edits the Rust
  directly (REWRITE.md §3.2). The XeTeX core keeps regenerating until U3 passes (Q8), so a
  TeX Live XeTeX fix still arrives as a regeneration while its parity is unproven.

### 5.4 What "no xdvipdfmx" requires

`xelatex` writes XDV and `xdvipdfmx` turns it into PDF. Owner ruling: FlashTeX does the
second step itself. Two consequences, both of them S3 scope:

1. **Fonts:** CID-keyed OpenType/CFF and TrueType subsets with ToUnicode, from the same font
   bytes HarfBuzz shaped with. The Classic backend already writes TrueType and OpenType
   (#1265); Typst's E1 records already carry OpenType faces to the renderer.
2. **Specials:** LaTeX's `xetex.def`, hyperref's `hxetex` and xcolor's xetex driver talk to
   dvipdfmx through `\special{pdf:…}`, `x:` and `color` specials. FlashTeX's writer must
   interpret dvipdfmx's special language (links, annotations, outlines, colour stacks,
   transformations, `pdf:image`, literal code). This is a port of dvipdfmx's special handlers
   (GPL-2+, so on the GPL side of §3), checked in lockstep against TeX Live's `xdvipdfmx` as
   an **oracle in tests only**. It is the largest unknown in the Unicode track (§8, risk R2).

Display-list pages are built from the same shaped glyphs, so preview and export share one
source of truth, as in Classic (D6).

### 5.5 What is shared with Modern

Modern mode is a separate process (TY2). It still shares, through the app and the MIT crates:

- the display-list protocol (v3.3 E1–E8; Unicode reuses E1/E2) and the renderer;
- the font index (`flashtex-font-discovery`: system fonts, the project's `fonts/`, bundle fonts;
  per-project font list family → file SHA-256, as §15.2 sets for Typst);
- the package UI: one consent sheet and one lock per project, with two back ends (the TeX
  bundle/CTAN and Typst Universe; §15.2);
- Problems rows, `lang-v1`, stored pages, Live Share, the mode item.

**Name matching stays per mode.** XeTeX finds fonts through CoreText on macOS and fontconfig
elsewhere (PLAN S1); Typst matches its own way. The shared index supplies the files, and each
mode resolves names exactly as its reference does, otherwise parity breaks.

---

## 6. Licensing

| Component | Licence | Side of §3 | Note |
|---|---|---|---|
| Classic core (`flashtex-engine`) | GPL-2.0-or-later | GPL host | Binary GPL v2 or v3 only, because xpdf is linked (§3) |
| Unicode core (`flashtex-xetex`) | GPL-2.0-or-later (PLAN §2) | same binary | XeTeX's own code is MIT; `xetex.web` changed only by change files (its e-TeX notice) |
| HarfBuzz / FreeType / ICU / TECkit / Graphite2 | MIT / FTL or GPL-2 / Unicode / LGPL-2.1 or CPL / LGPL-2.1 or MPL (PLAN §2) | linked into the host | *Belief, for the legal review:* the binary stays distributable under GPL v2 or v3, with FreeType taken under its GPL-2 option for v2 (the FSF lists FTL as GPLv3-compatible only) |
| Ported dvipdfmx special handlers | GPL-2.0-or-later | GPL host | Header rule of REWRITE.md §2 (derived-from, copyright, modified) |
| Typst host | MIT, linked to Apache-2.0 | separate process | Unchanged (§15.2); linking it into the host would make that binary GPLv3-only (§15.1) |
| `flashtex-display-list` | MIT | shared | Must not take GPL or Apache-only code (§15.9); E1 records are written by MIT code in each host |
| App, iPad | MIT | MIT | Never link either host (§3, D14) |

- **Product names:** "pdfLaTeX-compatible", "XeLaTeX-compatible", "Typst support". Never
  "XeTeX", "pdfTeX" or "TeX engine" in product text (§3; §15.8 for Typst). The open banner
  question of REWRITE.md §2 ("This is pdfTeX, …" printed by a hand-modified program) applies
  equally to "This is XeTeX, …".
- **Bundle redistribution (BUNDLE-PUBLISH, GitHub Release assets, owner 2026-10-04).** Hosting
  makes FlashTeX the distributor, with the same duties as the DMG:
  - TeX Live content ships **unmodified only** (§4.4; LPPL clause 6 is never needed), and only
    content TeX Live itself redistributes. Each package's licence is recorded in the bundle's
    manifest from TeX Live's catalogue data, with its licence text.
  - Fonts added for Unicode (Latin Modern and TeX Gyre OpenType, Latin Modern Math; GUST
    licence; any OFL font) ship with their licence files, as files, never compiled into a
    binary (the §15.2 rule for Typst's fonts).
  - GPL engine binaries come with their corresponding source: the tagged repository,
    including `third_party/` sources, linked from the same release.
  - No Typst Universe packages in the bundle (§15.8).
- The owner's §3 legal review covers Unicode mode, the special-handler port and the bundle.

---

## 7. Roadmap

Modes phases are **M0–M5**. They reference existing lanes and add no gate to them; each runs
under DESIGN.md's guard-rails (Classic first, path-filtered CI for the others).

| Phase | Scope | Lanes | Exit gate |
|---|---|---|---|
| **M0 Adopt** | Owner decision on this proposal and §8; DESIGN.md amendment (§9); `[project] mode` documented and read (Classic only) | MODES-DESIGN (this) | DESIGN.md PR merged; manifest tests for the key, unknown value → warning |
| **M1 Classic is the core** | P5 switch-over; invisible modernisations (§3) continue | P5 lanes (P5-ENGINE-CHOICE, P5-T4-MAC, retirement #1236); MACRO-REPLAY; IDIOMATIC-REWRITE; P6-HYPEROPT; MEMORY-SAFETY | **P5 gate as DESIGN.md §12 states it** (decision 3; thresholds pending Q3 there), with structured diagnostics (TY6 item 1) in; each modernisation's own gate |
| **M2 Unicode engine** | XeTeX S0 → S1 (native fonts, shaping) → S2 (OT math, pictures, Graphite) | XETEX-S0 (#1489), then S1, S2 | PLAN §3 gates for S0, S1, S2 (P-T1 + XDV vs `xetex -no-pdf`; version-lock test); Classic drift test byte-identical |
| **M3 Unicode in the product** | **U3a** one host binary, mode in protocol, shared runtime + per-core `iso`, handles checkpointed, E1/E2 pages; **U3b** FlashTeX PDF writer with dvipdfmx specials (§5.4); **U3c** bundle for Unicode | XeTeX S3 (split as U3a–c); BUNDLE-PUBLISH | Unicode corpus (fontspec, unicode-math, polyglossia, xeCJK; Q2's corpus) at P-T1 100 %; export gate per Q2; §1.2 latency on xelatex bench documents; incremental soundness sweeps on Unicode; a no-TeX-Live run of the corpus from the bundle; **Classic lockstep, P-T1/P-T2 fixtures and latency unchanged**. Behind a flag until met |
| **M4 Modern** | Typst T0–T3 | TYPST-T0T1 (#1481, #1484, #1487, #1511, #1513), then T2, T3 | DESIGN.md §15.10 gates, unchanged |
| **M5 One switch** | Mode item, detection, manifest writes, three-way New Project, Live Share pins the mode, explanations that offer a switch | MODES-UX (new, app); LIVE-SHARE (P1+ `join_ack`) | Detection precision (§4.2) on the Classic corpora (0 Unicode suggestions) and the Unicode corpus (all flagged); switch → first page within §1.2's preamble row; TypingBench and preview gates unchanged; VoiceOver announces the mode |

**Order and dependencies.**
- M1 comes first in staffing; M2 runs alongside it (it touches no Classic path).
- M3 needs M2's S2 gate and the P3 client (`P3-APP-V3`).
- M4 continues on its own track (R6 lifted, §13).
- M5 needs M3 (or M4's T2) to have something to switch to. The manifest key and Classic ↔
  Typst by extension can ship earlier.
- **Q1 links M1 and M3:** the old engine emulates fontspec today
  (`crates/render-pipeline/README.md`), so deleting it at P5 before M3 drops working
  fontspec documents.

---

## 8. Open questions for the owner

| # | Question | Recommended default |
|---|---|---|
| Q1 | The old engine renders fontspec documents and `[fonts]` today; DESIGN.md decision 3 deletes it at P5. Delete it before Unicode mode exists? | **Flip the default at P5 as planned. Keep the old engine only as the explicit fallback for documents that detection routes to Unicode, until M3's gate, then delete it.** This deviates from "deletion at once", so it needs your ruling |
| Q2 | Unicode export gate. `xelatex`'s PDF comes from xdvipdfmx; matching its content streams byte-for-byte (P-T2) means re-creating xdvipdfmx's writer exactly | **P-T1 + XDV identity (normalised, PLAN §3) as the primary gate; export: same embedded glyph sets and pixel-identical pages against xelatex's PDF at 2×/3× (§15.5's wording); content-stream identity not required.** Corpus: fontspec/unicode-math documents from TeX Live's documentation sources and public thesis templates, assembled in U3 |
| Q3 | Manifest key name and values | **`[project] mode = "classic" \| "unicode"`**; `.typ` implies Modern; `% !TEX program` honoured as a hint |
| Q4 | Where the status-bar choice is stored | **In `flashtex.toml`** (shared with git, CLI and Live Share), unlike today's app-local old/new choice |
| Q5 | What `[fonts]` means after the old engine | **Ignored with a warning in Classic and Unicode (no engine reads it, so output would differ from `xelatex`), with a one-click "insert `\setmainfont{…}`" fix that puts the choice in the source; deprecated after M3** |
| Q6 | LuaLaTeX (`luacode`, `\directlua`, `luaotfload`-only features) | **No LuaTeX mode now.** Such documents get a clear diagnostic; revisit with measured demand after M3 |
| Q7 | The same Unicode document can pick different fonts on different machines, as `xelatex` does | **Match XeTeX's lookup per platform; record the per-project font list (family → file SHA-256) and flag missing or changed fonts; Live Share compares it in the environment digest** |
| Q8 | Freeze the XeTeX translation as REWRITE.md does for pdfTeX? | **Not before M3's gate;** decide then, using what pdfTeX's freeze has shown |
| Q9 | UI name of Modern mode | **"Typst"** (Typst's brand guidance, §15.8); "Modern" only in design text |
| Q10 | One host binary for both LaTeX modes, or two | **One** (your "no separate TeX"); split only if U3a measures a start-up or size cost that §1.2 notices |

### Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | Shaping depends on exact HarfBuzz/FreeType versions | Vendor TeX Live's versions with sha256; a test fails on upgrade (PLAN §5) |
| R2 | dvipdfmx's special language is large and underspecified | Port its handlers per file under a lockstep against `xdvipdfmx` as a test oracle; start with what LaTeX's drivers emit (corpus census first) |
| R3 | Unicode work slows Classic | Separate crate and CI path filters; Classic gates on every shared-runtime change; staffing after Classic (§15.9 pattern) |
| R4 | Two cores drift in shared runtime assumptions | The engine trait (§5.2) is the only coupling; soundness sweeps run per mode |
| R5 | Users switch mode expecting nicer output | The switch says what changes ("pages will match xelatex, not pdflatex") and is undoable; stored pages per mode |

---

## 9. If adopted: what DESIGN.md would need

- A new section, "Modes", stating: the three modes and their references (§0 table); the
  non-goal of §1.2; one host binary for Classic and Unicode with the engine trait (§5.2);
  Typst unchanged per §15.
- §1.1: Unicode's parity tiers (Q2's ruling).
- §2: decisions for one binary with two cores (side by side, not merged) and for the
  no-xdvipdfmx writer.
- §3: the Unicode core and its linked libraries in the diagram and the licence list.
- §4.4: the `xelatex` format and the bundle's Unicode contents.
- §10/§12: Q1's ruling on P5 retirement; M0–M5 beside P0–P6 and T0–T3.
- §13: the rows for the owner's 2026-10-04 decisions (XeTeX in-engine, the bundle on GitHub
  Releases) and this proposal's rulings.

DESIGN.md is edited by the Commander, one open DESIGN.md PR at a time (§9.5).
