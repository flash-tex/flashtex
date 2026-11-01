# One core, three compatible modes and an opt-in Native mode: design (MODES-DESIGN)

Status: **adopted, with owner rulings** (2026-10-04; §8 records each ruling). Q6 is still open.
Author: mac-claude-a (mac-m1max-a), 2026-10-04.
Requested by the owner (2026-10-04, [#1319](https://github.com/flash-tex/flashtex/issues/1319),
"owner decisions, 2026-10-04" item 3): one modern FlashTeX core with three per-document modes.
The owner's answers to §8 (2026-10-04) changed Q5 and Q10 and added a fourth, explicitly opt-in
**Native ("FlashTeX") mode** for projects made only with FlashTeX (§1.2, §10).

`docs/design/engine-v2/DESIGN.md` remains the single source of truth and is owned by the
Commander. Adoption here does not change it; until the Commander's DESIGN.md amendment (§9)
merges, DESIGN.md wins wherever they differ, and no lane may build anything below that DESIGN.md
does not yet allow. §9 lists what DESIGN.md needs to say. Every number below names its source;
**VERIFIED** means re-read from that source today, and anything else is a belief and says so.

---

## 0. The design in brief

| Mode | Language | Matches | Engine | Product label |
|---|---|---|---|---|
| **Classic** | LaTeX (`.tex`) | `pdflatex`, P-T1/P-T2 (DESIGN.md §1.1) | `pdftex.web` → Rust (D1–D3), today's engine | "LaTeX (pdfLaTeX-compatible)" |
| **Unicode** | LaTeX (`.tex`) | `xelatex`, P-T1 + XDV (§4.3) | `xetex.web` → Rust in **`flashtex-host-unicode`** ([XeTeX PLAN](https://github.com/flash-tex/flashtex/pull/1489), S0–S3) | "LaTeX, Unicode (XeLaTeX-compatible)" |
| **Modern** | Typst (`.typ`) | `typst` at the pinned version (TY4) | `flashtex-typst-host`, a separate MIT process (DESIGN.md §15.2) | "Typst" |
| **Native** (opt-in, §10) | LaTeX (`.tex`) | **no external engine**: FlashTeX's own golden output, pinned per `flashtex_version` | the Unicode core in `flashtex-host-unicode`, plus FlashTeX-only features | "FlashTeX (not portable)" |

1. **Three compatible modes, each faithful to a real, external reference,** and one opt-in
   Native mode that is not. Native is never the default, never chosen by detection, always
   labelled as not reproducible by other engines, and has a one-way "Export portable LaTeX"
   (§1.2, §10).
2. **One core** means one app, one protocol (`display-list-v3.x`), one renderer, one
   incremental runtime, one font index, one package manager, one diagnostics model and one
   switch. It does **not** mean one process for Typst: DESIGN.md §15.2 (TY2) keeps Typst out of
   the GPL host, and this proposal keeps that.
3. **Two FlashTeX engine programs over one shared runtime** (owner, Q10, 2026-10-04):
   `flashtex-host` runs Classic, `flashtex-host-unicode` runs Unicode and Native. Both are
   FlashTeX's own GPL builds, both cores translated by `web2rust` from unmodified WEB sources,
   both linking one runtime library crate behind one engine interface (§5). That keeps the
   owner's "no separate TeX" (2026-10-04): no `xetex` or `xdvipdfmx` executable is shipped or
   run; native fonts, shaping, OpenType math and the PDF writer live in FlashTeX's code.
4. **Classic modernises invisibly**: memoisation, macro replay, persisted state, pipeline
   parallelism, structured diagnostics and the idiomatic rewrite, each gated on P-T1/P-T2
   unchanged (§3). Unicode inherits them through the shared runtime.
5. **Mode is per document**, stored in `flashtex.toml` (`[project] mode`), suggested by
   detection, never switched silently, and shown in the status bar (§4).
6. **`[fonts]` is respected** (owner, Q5): in Unicode and Native mode as generated
   fontspec/unicode-math setup, injected and visible in the log; in Classic, where it cannot
   apply without breaking pdflatex parity, as a clear notice offering Unicode or Native (§4.5).

---

## 1. Goals and non-goals

### 1.1 Goals

- **G1. Faithfulness per mode.** Classic: P-T1 and P-T2 against pdfTeX 1.40.29 (DESIGN.md
  §1.1). Unicode: P-T1 and XDV identity against TeX Live 2026's XeTeX (PLAN §3), and an export
  gate of Q2. Modern: pixel identity with typst-pdf (DESIGN.md §15.5). Native has no external
  reference; its faithfulness is to itself: golden output pinned per `flashtex_version` and
  determinism (§10.6).
- **G2. One user experience.** Every shared feature (preview, Live Share, diagnostics,
  packages, fonts, stored pages, the switch) behaves the same in all modes, or the UI hides it
  by capability (DESIGN.md §15.6, `HELLO.capabilities`).
- **G3. Shared speed.** The §1.2 latency targets apply to Classic and Unicode alike; Modern
  keeps its owner-accepted relaxed target above about 100 pages (DESIGN.md §15.3).
- **G4. No regression to Classic, and Native never moves Unicode.** A Unicode, Native or
  Modern change never changes a Classic box dump, PDF content stream or latency gate (the
  pattern of DESIGN.md §15.9); a Native change never changes a Unicode P-T1/XDV result.
- **G5. No separate TeX.** No TeX Live engine binary or DVI driver runs at user time. TeX Live
  *content* (`latex.ltx`, packages, fonts) is still used, from the user's install or the bundle
  (D12).

### 1.2 Non-goals

- **No "modern-but-different LaTeX" mode by default, and never silently** (revised by the owner,
  Q5, 2026-10-04). A mode that sets `.tex` files with, say, better line breaking, different float
  placement or different defaults is **allowed only as the fourth, explicitly opt-in Native
  ("FlashTeX") mode** (§10), for projects made exclusively with FlashTeX. It is labelled
  everywhere as not reproducible by other engines, and it has a one-way "Export portable LaTeX"
  escape hatch that flattens FlashTeX-only settings into standard `xelatex` source (§10.5).
  **Why it must never be the default, nor be suggested, nor be reached by a document that did
  not ask for it:**
  - **A paper is typeset by other people's engines too.** arXiv recompiles submitted sources
    with its own TeX Live; journal production recompiles them; coauthors open them in
    Overleaf or TeX Live. If FlashTeX's pages differ, the author proofreads a document nobody
    else will see: page limits, figure placement, the line a referee cites, and widow and orphan
    fixes all stop holding.
  - **It breaks Live Share's guarantee** unless pinned. "Same state, same pins, same read set
    gives the same pages" ([live-collab §5.3](../live-collab/PROPOSAL.md)) holds for Native
    only because every peer runs a FlashTeX that implements the pinned `flashtex_version`
    (§10.4); collaborators on other tools cannot take part.
  - **It is the road D1 left.** Hand-written LaTeX semantics reached arXiv L0 = 1.4 % with 70
    hand-ported packages (DESIGN.md §2, D1). A different-by-design dialect has no oracle, so no
    parity gate can catch its bugs. Native therefore keeps the faithful Unicode core underneath
    and limits its differences to a short, versioned list (§10.2), each one gated by golden
    output and determinism (§10.6).
  - **Compatible modes already meet the common need.** A user who wants modern fonts and Unicode
    gets Unicode mode, which matches `xelatex` (and now reads `[fonts]`, §4.5). A user who wants
    a modern language gets Modern mode, which is openly a different language (Typst). Native is
    for the user who has decided their project lives in FlashTeX.
- **No relaxation of DESIGN.md §4.3 in any mode,** Native included: an unknown primitive is a
  hard, reported error ("No 'lenient mode' ever").
- **No LuaTeX mode now** (Q6; still open, pending the owner's ruling).
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
└──────┬──────────────────── display-list-v3.x + lang-v1 (MIT spec) ──────────┬─────────┘
       │ Unix socket, one host process per open document; binary chosen by mode │
┌──────┴─────────────────┐ ┌──────────────────────────────────┐ ┌──────────┴──────────────┐
│ flashtex-host (GPL-2+) │ │ flashtex-host-unicode (GPL-2+)   │ │ flashtex-typst-host     │
│ ┌─ CLASSIC core ─────┐ │ │ ┌─ UNICODE core ───────────────┐ │ │ (MIT; typst =pinned,    │
│ │ pdftex.web→web2rust│ │ │ │ xetex.web → web2rust         │ │ │  Apache-2.0)            │
│ │ pdfTeX change files│ │ │ │ change files (84 % shared)   │ │ │ World: confinement,     │
│ │ pdfTeX C parts     │ │ │ │ HarfBuzz, FreeType, ICU,     │ │ │  package lock, fonts    │
│ │  (writet1 …)       │ │ │ │ TECkit, Graphite2; OT math;  │ │ │ seeded loop, evict,     │
│ │ pdfTeX node iso    │ │ │ │ XeTeX node iso; PDF: native  │ │ │  watchdog (§15.2–15.3)  │
│ │ PDF: pdfTeX backend│ │ │ │ fonts + dvipdfmx specials    │ │ │ v3.3 writer (E1–E8)     │
│ └────────────────────┘ │ │ └──────────────────────────────┘ │ │                         │
│                        │ │ NATIVE layer (--mode native, §10)│ │ MODERN core             │
│                        │ │  [fonts]/defaults injection,     │ └─────────────────────────┘
│                        │ │  fixed-point refs, algo levels   │
├────────────────────────┴─┴──────────────────────────────────┤
│ SHARED RUNTIME LIBRARY CRATE (GPL-2+, linked by both hosts)  │
│  arena + checkpoints (§5.2) · convergence driver (§5.3)      │
│  read-sets, multi-pass, external tools (§5.5)                │
│  intrinsics + macro replay (§5.6, MACRO-REPLAY)              │
│  resolver: TeX Live or bundle, format cache (§4.4)           │
│  font index client · display-list writer · PDF object writer │
│  diagnostics side channel · host protocol, watchdog, crash   │
│  isolation · [fonts] → preamble generator (§4.5)             │
└──────────────────────────────────────────────────────────────┘
SHARED MIT crates (linked by all hosts and the CLI): flashtex-display-list, project-manifest,
package-resolver, flashtex-font-discovery.  SHARED DATA: the bundle (GitHub Release assets).
```

### 2.2 Per-mode table

| Concern | Classic | Unicode | Native (§10) | Modern |
|---|---|---|---|---|
| Reference oracle | pdfTeX 1.40.29 (TeX Live 2026) | XeTeX 0.999998 (TeX Live 2026; PLAN §1) | none external; golden output per `flashtex_version` (§10.6); at algorithm level 0 equal to Unicode on the exported source | `typst` `=0.15.1`, per-project pin (§15.2) |
| Engine source | `pdftex.web` + change files (§4.1); REWRITE.md may freeze it | `xetex.web` + change files (PLAN §2) | the Unicode core; algorithm changes only behind a level that is 0 in Unicode (§10.2) | upstream crates, unmodified |
| Format | `pdflatex.fmt` from the user's `latex.ltx` (D12) | `xelatex.fmt`, same rule | `xelatex.fmt` | none |
| Fonts in the engine | TFM + Type 1 via `pdftex.map` | TFM, plus OpenType/TrueType by name or `[file]` (PLAN S1) | as Unicode | Typst's font book |
| `[fonts]` | not applied; notice offering Unicode/Native (§4.5) | generated fontspec/unicode-math setup, in the log (§4.5) | as Unicode, on by default | n/a (Typst sets fonts in source) |
| Font in the protocol | `FONT.format` Type 1 / Type 3 / TrueType (§15.1) | `"opentype"` (E1), f64 origins (E2) | as Unicode | `"opentype"` (E1–E8) |
| PDF | ported pdfTeX backend (§6.3) | FlashTeX's own writer: CID/OpenType subsets, dvipdfmx specials (§5.4) | as Unicode | typst-pdf |
| Incremental | L1–L5, L6 (§5) | the same runtime; mode-specific `iso` and handle table (§5.3) | as Unicode, plus fixed-point references (§10.2) | comemo + seeded loop (§15.3) |
| Process | `flashtex-host` | `flashtex-host-unicode --mode unicode` | `flashtex-host-unicode --mode native` | `flashtex-typst-host` |
| Licence of the core | GPL-2+; binary GPL v2 or v3 (xpdf) | GPL-2+, its own binary | same binary as Unicode | MIT + Apache-2.0 |

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
| 3 | Manifest `[project] mode` | `mode = "unicode"` | Shared through git and Live Share; the only **persistent** way to select Native (row 2 can force it for one build, §10.4) |
| 4 | Magic comment `% !TEX program = xelatex` | TeXShop/TeXstudio convention | Read only when no manifest key; `pdflatex` → Classic, `xelatex` → Unicode, `lualatex` → Q6; never Native |
| 5 | Default | Classic | |

Detection (§4.2) **suggests**; it never decides. A mode change changes the output, so it
always takes one click by the user. **Detection never suggests Native**: Native is chosen only
by the user, in New Project or the mode menu, after a confirmation that names what it gives up
(§10.4).

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
mode = "unicode"     # "classic" (default) | "unicode" | "flashtex"; .typ entries are always Modern
# flashtex_version = "1.0"   # required with mode = "flashtex" (§10.3); ignored otherwise
```

- An unknown value is a warning and Classic, like every unknown key today
  ([project-manifest.md](../../user/project-manifest.md)). One exception, so that a Native
  project is never silently typeset as something else: `mode = "flashtex"` on a FlashTeX that
  does not implement the pinned `flashtex_version` is an error naming the version needed, with
  the project opened read-only on its stored pages (§10.3).
- The Typst version pin stays where §15.2 puts it.
- Today's per-document engine choice (old vs new engine, `EngineChoice.swift`) is app-local on
  purpose: it is a fallback choice, not document semantics. **Mode is document semantics**, so
  it goes in the project, where git, the CLI and Live Share see it (Q4). The old/new choice
  disappears at P5.

### 4.4 Status bar and other surfaces

- The **status-bar engine item** (today `EngineChoiceStatusItem`) becomes the **mode item**:
  "Classic", "Unicode", "FlashTeX" or "Typst". Its menu shows the source of the mode ("set in
  flashtex.toml", "from `% !TEX program`", "default"), lets the user switch Classic ↔ Unicode
  (writes `[project] mode`, creating `flashtex.toml` if needed), and shows the detection
  suggestion if there is one. View › Mode for This Document mirrors it; VoiceOver announces a
  change.
- **Switching** restarts the document's host in the other mode: a cold compile from the
  other format (§1.2 preamble row, ≤ 400 ms to first page, is the target). Stored pages are
  kept per mode, so switching back shows the old pages at once, stale until confirmed.
- **New Project** gets three entries (extending §15.6's LaTeX | Typst picker): LaTeX, LaTeX
  (Unicode fonts) and Typst, each over its own template list, plus a fourth, visually set
  apart, "FlashTeX-only (not portable)" entry for Native (§10.4).
- **Live Share:** the hub pins the mode in `join_ack` with the other session pins
  (live-collab §5.3); a peer whose manifest says otherwise is told, never silently compiled in
  another mode. Native also pins `flashtex_version` (§10.4).

### 4.5 `[fonts]` in each mode (owner, Q5, 2026-10-04: "the `[fonts]` setting should still be respected")

`[fonts]` (`text`, `sans`, `mono`, `math`; [project-manifest.md](../../user/project-manifest.md))
keeps its documented meaning and precedence (highest first: a local `\fontspec` group, the
document's `\setmainfont`/`\setsansfont`/`\setmonofont`/`\setmathfont`, `[fonts]`, the class
default). After the old engine is retired, the engine applies it as follows.

| Mode | How `[fonts]` applies |
|---|---|
| **Unicode** | The shared runtime's preamble generator turns `[fonts]` into the equivalent `fontspec`/`unicode-math` setup (`\setmainfont{…}`, `\setsansfont{…}`, `\setmonofont{…}`, `\setmathfont{…}`) and injects it at a fixed point so the document's own font commands still win. It is **visible**: the log and the Problems pane show one line per role applied (e.g. `FlashTeX [fonts]: \setmainfont{TeX Gyre Pagella}`), and *Show effective preamble* displays the generated code. |
| **Native** | The same generator, applied by default (§10.2). |
| **Classic** | Not applied: pdfTeX cannot load system OpenType fonts by name, and emulating them would break pdflatex parity (G1). If `[fonts]` sets any role, a non-blocking notice says so once per project and offers **Switch to Unicode** (matches `xelatex`) or **Switch to FlashTeX mode** (§10), each writing `[project] mode`. Nothing is applied silently. |
| **Modern** | Not applicable: Typst sets fonts in source; the font index still supplies the files (§5.5). |

**Injection point (design; the exact hooks are fixed in M3 by test).** Text roles: a generated
file `flashtex-fonts.sty`, loaded through LaTeX's `class/after` hook, so the class's defaults are
replaced and any later `\setmainfont` in the document overrides it, which keeps the precedence
above. Math role: loaded at `begindocument/before` and only if the document has not loaded
`unicode-math` or set a math font itself, so that `amsmath`-before-`unicode-math` ordering holds.

**Unicode stays an `xelatex` mode.** The injection is part of the document's *effective source*.
Unicode's parity gate (P-T1 + XDV, Q2) compares FlashTeX against `xelatex` run on that same
effective source (the same hook code passed on the command line), so `[fonts]` does not cost
Unicode its oracle. *Export portable LaTeX* (§10.5) writes the injection into the source, which
makes the project build with stock `xelatex`.

---

## 5. Unicode and Classic behind one engine interface

### 5.1 Two engine programs, one runtime library (owner, Q10, 2026-10-04)

- **Two binaries.** `flashtex-host` runs Classic only. `flashtex-host-unicode` runs Unicode and
  Native (`--mode unicode | native`). Both are FlashTeX's own builds; neither is, wraps or
  spawns `xetex`, `pdftex` or `xdvipdfmx`. The app and the CLI pick the binary from the
  resolved mode (§4.1).
- **One shared runtime library crate** (working name `flashtex-host-runtime`, GPL-2+): today's
  host code in `crates/flashtex-engine/src/host/` (the `flashtex-host` `[[bin]]` in
  `crates/flashtex-engine/Cargo.toml`) is split into a library holding everything in §2.1's
  SHARED RUNTIME box, and two thin `main`s that each link one core through the engine trait
  (§5.2). The runtime never names a core; the trait is the only coupling (R4).
- **Why split** (the owner's ruling; reasons recorded here): Classic's binary does not link
  HarfBuzz, FreeType, ICU, TECkit or Graphite2, so its size, start-up, licence list and
  required CI are untouched by Unicode work (G4); a fault in the Unicode core cannot be reached
  from a Classic process; the two can be released and rolled back independently.
- **Mode per process.** The mode arrives once per process (`--mode`, and an additive `mode` key
  in `OPEN`/`COMPILE` that the protocol owner assigns, R3). A process never changes mode:
  switching starts a new process, of the other binary when needed. Processes stay one per
  document (§15.2's isolation rule applies to LaTeX hosts as well).
- **Measured, not estimated:** at U3a, per binary, stripped size, idle RSS, and start-up to first
  page (cold and from the format cache) on the §1.2 bench documents; Classic's numbers before and
  after the runtime extraction must be unchanged within noise (G4). None are estimated here.
- **Build:** `crates/flashtex-xetex` lives in its own workspace today so that its build never
  slows the root workspace's gates (PLAN §2). At U3a `flashtex-host-unicode` joins as its own
  binary crate over `flashtex-host-runtime`, with its CI jobs path-filtered as Typst's are
  (§15.9). Classic's required checks build only `flashtex-host` and the runtime, and run
  Classic's gates on every runtime change.

### 5.2 The engine interface

One Rust trait in the shared runtime crate, implemented by each core:

| Method family | Classic implementation | Unicode implementation |
|---|---|---|
| `init(format, pins)` / `undump` | `pdflatex.fmt` | `xelatex.fmt` |
| `run_until(shipout or restart point)` | `main_control` | `main_control` |
| `word_space()` (arena for checkpoints, §5.2) | `mem`, `eqtb`, … | the same, plus the native-font handle table (PLAN §4.7) |
| `iso(a, b)` (convergence, §5.3) | pdfTeX node types | XeTeX node types (native word, glyph, 8-word boxes) |
| `read_set()` | files, `.aux`, cs reads | the same, plus **font files** (path, SHA-256) and the font-index generation |
| `ship_page() -> display list` | Type 1 / TFM glyphs | E1 OpenType glyph ids + E2 f64 origins |
| `export_pdf()` | pdfTeX backend | FlashTeX PDF writer (§5.4) |
| `effective_preamble()` | none | the `[fonts]` injection (§4.5); in Native also the defaults (§10.2) |
| `diagnostics()` | `print_err` hooks (TY6) | the same hooks in xetex change files |

The shared runtime (arena, checkpoint logs, convergence driver, resolver, format cache,
multi-pass and external tools, host protocol, watchdog) calls only the trait. Native is not a
third implementation: it is the Unicode implementation with a mode value (§10.2).

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
  directly (REWRITE.md §3.2). The XeTeX core keeps regenerating at least until M3's gate passes
  (Q8, Commander ruling), so a TeX Live XeTeX fix still arrives as a regeneration while its
  parity is unproven.

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
| Shared runtime (`flashtex-host-runtime`) | GPL-2.0-or-later | linked into both GPL hosts | Today's host code, extracted (§5.1) |
| Classic core (`flashtex-engine`) in `flashtex-host` | GPL-2.0-or-later | GPL host | Binary GPL v2 or v3 only, because xpdf is linked (§3) |
| Unicode core (`flashtex-xetex`) and the Native layer in `flashtex-host-unicode` | GPL-2.0-or-later (PLAN §2) | second GPL host | XeTeX's own code is MIT; `xetex.web` changed only by change files (its e-TeX notice) |
| HarfBuzz / FreeType / ICU / TECkit / Graphite2 | MIT / FTL or GPL-2 / Unicode / LGPL-2.1 or CPL / LGPL-2.1 or MPL (PLAN §2) | linked into `flashtex-host-unicode` only | *Belief, for the legal review:* that binary stays distributable under GPL v2 or v3, with FreeType taken under its GPL-2 option for v2 (the FSF lists FTL as GPLv3-compatible only). `flashtex-host`'s licence list does not change |
| Ported dvipdfmx special handlers | GPL-2.0-or-later | `flashtex-host-unicode` | Header rule of REWRITE.md §2 (derived-from, copyright, modified) |
| Typst host | MIT, linked to Apache-2.0 | separate process | Unchanged (§15.2); linking it into the host would make that binary GPLv3-only (§15.1) |
| `flashtex-display-list` | MIT | shared | Must not take GPL or Apache-only code (§15.9); E1 records are written by MIT code in each host |
| App, iPad | MIT | MIT | Never link either host (§3, D14) |

- **Product names:** "pdfLaTeX-compatible", "XeLaTeX-compatible", "Typst support", and for
  Native "FlashTeX mode (not portable)". Never
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
- The owner's §3 legal review covers Unicode mode, the special-handler port, the second binary
  and the bundle. Portable export (§10.5) copies a font into the exported project only when its
  licence permits redistribution (OFL, GUST), with the licence file; otherwise it lists the font.

---

## 7. Roadmap

Modes phases are **M0–M6**. They reference existing lanes and add no gate to them; each runs
under DESIGN.md's guard-rails (Classic first, path-filtered CI for the others).

| Phase | Scope | Lanes | Exit gate |
|---|---|---|---|
| **M0 Adopt** | Owner decision on this proposal and §8 (done 2026-10-04, Q6 open); DESIGN.md amendment (§9); `[project] mode` documented and read (Classic only; `"unicode"` and `"flashtex"` parsed and reported as not available yet) | MODES-DESIGN (this) | DESIGN.md PR merged; manifest tests for the key, unknown value → warning, `"flashtex"` without a usable `flashtex_version` → error |
| **M1 Classic is the core** | P5 switch-over; invisible modernisations (§3) continue | P5 lanes (P5-ENGINE-CHOICE, P5-T4-MAC, retirement #1236); MACRO-REPLAY; IDIOMATIC-REWRITE; P6-HYPEROPT; MEMORY-SAFETY | **P5 gate as DESIGN.md §12 states it** (decision 3; thresholds pending Q3 there), with structured diagnostics (TY6 item 1) in; each modernisation's own gate |
| **M2 Unicode engine** | XeTeX S0 → S1 (native fonts, shaping) → S2 (OT math, pictures, Graphite) | XETEX-S0 (#1489), then S1, S2 | PLAN §3 gates for S0, S1, S2 (P-T1 + XDV vs `xetex -no-pdf`; version-lock test); Classic drift test byte-identical |
| **M3 Unicode in the product** | **U3a** runtime extracted into `flashtex-host-runtime`, second binary `flashtex-host-unicode`, mode in protocol, per-core `iso`, handles checkpointed, E1/E2 pages, per-binary size and start-up measured (§5.1); `[fonts]` preamble generator (§4.5); **U3b** FlashTeX PDF writer with dvipdfmx specials (§5.4); **U3c** bundle for Unicode | XeTeX S3 (split as U3a–c); BUNDLE-PUBLISH | Unicode corpus (fontspec, unicode-math, polyglossia, xeCJK; Q2's corpus) at P-T1 100 %; export gate per Q2; §1.2 latency on xelatex bench documents; incremental soundness sweeps on Unicode; a no-TeX-Live run of the corpus from the bundle; `[fonts]` fixtures: P-T1 + XDV against `xelatex` on the effective source, precedence tests (document font commands win); **Classic lockstep, P-T1/P-T2 fixtures, latency, and `flashtex-host` size and start-up unchanged**. Behind a flag until met |
| **M4 Modern** | Typst T0–T3 | TYPST-T0T1 (#1481, #1484, #1487, #1511, #1513), then T2, T3 | DESIGN.md §15.10 gates, unchanged |
| **M5 One switch** | Mode item, detection, manifest writes, New Project entries, Live Share pins the mode, explanations that offer a switch, Classic's `[fonts]` notice (§4.5) | MODES-UX (new, app); LIVE-SHARE (P1+ `join_ack`) | Detection precision (§4.2) on the Classic corpora (0 Unicode suggestions) and the Unicode corpus (all flagged); 0 Native suggestions anywhere; switch → first page within §1.2's preamble row; TypingBench and preview gates unchanged; VoiceOver announces the mode |
| **M6 Native mode** | §10, in steps: **N1** `mode = "flashtex"` + `flashtex_version`, defaults and `[fonts]` on (algorithm level 0); **N2** portable export; **N3** fixed-point references and structured reference diagnostics; **N4** each improved algorithm, one per new `flashtex_version` | NATIVE-MODE (new; after M3, staffed after Classic and Unicode) | Per step, §10.6: N1 golden corpus and determinism; level-0 output equal to Unicode mode and to `xelatex` on the exported source; N2 every Native corpus document exports and builds with stock `xelatex` (TeX Live 2026) without errors; N3 one-compile convergence on the reference corpus, non-convergence reported as a diagnostic; N4 every earlier pin's goldens unchanged, the new level's goldens reviewed and recorded. Always: Unicode P-T1/XDV and Classic gates unchanged. Behind a flag until N1 and N2 both pass |

**Order and dependencies.**
- M1 comes first in staffing; M2 runs alongside it (it touches no Classic path).
- M3 needs M2's S2 gate and the P3 client (`P3-APP-V3`).
- M4 continues on its own track (DESIGN.md's R6 lifted, its §13).
- M5 needs M3 (or M4's T2) to have something to switch to. The manifest key and Classic ↔
  Typst by extension can ship earlier.
- M6 needs M3's gate (Native is the Unicode core) and M5's mode item. It starts no earlier than
  M3 passes, and never takes staff from M1 or M3.
- **Q1 links M1 and M3:** the old engine emulates fontspec today
  (`crates/render-pipeline/README.md`), so deleting it at P5 before M3 drops working
  fontspec documents.

---

## 8. Owner questions and rulings (2026-10-04)

The owner answered on 2026-10-04. "Adopted" means the recommended default stands as written.

| # | Question | Recommended default (as proposed) | Ruling | Status |
|---|---|---|---|---|
| Q1 | The old engine renders fontspec documents and `[fonts]` today; DESIGN.md decision 3 deletes it at P5. Delete it before Unicode mode exists? | Flip the default at P5 as planned. Keep the old engine only as the explicit fallback for documents that detection routes to Unicode, until M3's gate, then delete it | **Adopted** (owner). A deviation from "deletion at once" that the DESIGN.md amendment records (§9) | Resolved |
| Q2 | Unicode export gate. `xelatex`'s PDF comes from xdvipdfmx; matching its content streams byte-for-byte (P-T2) means re-creating xdvipdfmx's writer exactly | P-T1 + XDV identity (normalised, PLAN §3) as the primary gate; export: same embedded glyph sets and pixel-identical pages against xelatex's PDF at 2×/3× (§15.5's wording); content-stream identity not required. Corpus: fontspec/unicode-math documents from TeX Live's documentation sources and public thesis templates, assembled in U3 | **Adopted** (owner) | Resolved |
| Q3 | Manifest key name and values | `[project] mode = "classic" \| "unicode"`; `.typ` implies Modern; `% !TEX program` honoured as a hint | **Adopted** (owner), extended by Q5's ruling with `"flashtex"` and `flashtex_version` (§4.3, §10.3) | Resolved |
| Q4 | Where the status-bar choice is stored | In `flashtex.toml` (shared with git, CLI and Live Share), unlike today's app-local old/new choice | **Adopted** (owner) | Resolved |
| Q5 | What `[fonts]` means after the old engine | Ignored with a warning in Classic and Unicode, with a one-click "insert `\setmainfont{…}`" fix; deprecated after M3 | **Changed** (owner): "the `[fonts]` setting should still be respected." Unicode and Native apply it as generated, logged fontspec/unicode-math setup; Classic shows a notice offering Unicode or Native (§4.5). Also: a fourth, opt-in, non-portable **Native ("FlashTeX") mode** for FlashTeX-only projects, in addition to Classic, Unicode and Typst (§1.2, §10) | Resolved |
| Q6 | LuaLaTeX (`luacode`, `\directlua`, `luaotfload`-only features) | No LuaTeX mode now. Such documents get a clear diagnostic; revisit with measured demand after M3 | The owner asked for an explanation (the Commander answers in chat). The default stands meanwhile | **Open, pending the owner** |
| Q7 | The same Unicode document can pick different fonts on different machines, as `xelatex` does | Match XeTeX's lookup per platform; record the per-project font list (family → file SHA-256) and flag missing or changed fonts; Live Share compares it in the environment digest | **Adopted** (owner); applies to Native too (§10.4) | Resolved |
| Q8 | Freeze the XeTeX translation as REWRITE.md does for pdfTeX? | Not before M3's gate; decide then, using what pdfTeX's freeze has shown | **Commander decision** (mac-claude-a, delegated): not before Unicode passes its M3 gate; decide then | Resolved (deferred to M3's gate) |
| Q9 | UI name of Modern mode | "Typst" (Typst's brand guidance, §15.8); "Modern" only in design text | **Adopted** (owner) | Resolved |
| Q10 | One host binary for both LaTeX modes, or two | One; split only if U3a measures a start-up or size cost that §1.2 notices | **Changed** (owner): **two** engine programs, `flashtex-host` (Classic) and `flashtex-host-unicode` (Unicode and Native), both FlashTeX's own builds over one shared runtime library crate; still no `xetex` or `xdvipdfmx`; per-binary size and start-up measured at U3a (§5.1) | Resolved |

### Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | Shaping depends on exact HarfBuzz/FreeType versions | Vendor TeX Live's versions with sha256; a test fails on upgrade (PLAN §5) |
| R2 | dvipdfmx's special language is large and underspecified | Port its handlers per file under a lockstep against `xdvipdfmx` as a test oracle; start with what LaTeX's drivers emit (corpus census first) |
| R3 | Unicode work slows Classic | Separate crate and CI path filters; Classic gates on every shared-runtime change; staffing after Classic (§15.9 pattern) |
| R4 | Two cores drift in shared runtime assumptions | The engine trait (§5.2) is the only coupling; soundness sweeps run per mode |
| R5 | Users switch mode expecting nicer output | The switch says what changes ("pages will match xelatex, not pdflatex") and is undoable; stored pages per mode |
| R6 | Native output is mistaken for portable LaTeX, or a Native project is submitted as-is | Native is opt-in only, labelled "not portable" in the mode item, the window title's subtitle and New Project; PDF export from Native offers *Export portable LaTeX* alongside (§10.5) |
| R7 | Native has no external oracle, so its bugs are invisible to parity gates | Native's base is the gated Unicode core; level 0 is checked against `xelatex` on the exported source; each improved algorithm is a reviewed golden change under a new pin; determinism on every CI platform (§10.6) |
| R8 | Old `flashtex_version` levels accumulate and must be kept forever | Every level stays implemented and gated by its goldens; retiring a level needs an owner ruling and an export path for projects pinned to it |
| R9 | The runtime extraction for the split (§5.1) changes Classic | Extraction is a pure move under Classic's lockstep, P-T1/P-T2 and latency gates, landed before any Unicode code links the runtime |

---

## 9. DESIGN.md amendments this adoption needs

Until these merge, DESIGN.md wins and governs every lane (header). The Commander drafts them as
one DESIGN.md PR (one open at a time, DESIGN.md §9.5):

- **A new section, "Modes":** the four modes and their references (§0 table): Classic
  (pdflatex), Unicode (xelatex), Modern (Typst, unchanged per §15), and **Native**, the opt-in,
  non-portable FlashTeX mode with no external oracle (§10); the revised non-goal of §1.2 (no
  modern-but-different LaTeX by default or silently; why Native is never the default); mode
  resolution and detection rules, including "detection never suggests Native" (§4).
- **§1.1:** Unicode's parity tiers (Q2: P-T1 + XDV primary; export by embedded glyph sets and
  pixel identity). A Native tier: golden output per `flashtex_version` plus determinism, and
  level-0 equality with Unicode on the exported source (§10.6). State explicitly that Native
  output is not a parity claim.
- **§2 decisions:** **two engine programs**, `flashtex-host` (Classic) and
  `flashtex-host-unicode` (Unicode and Native), over **one shared runtime library crate**, both
  FlashTeX's own builds, no `xetex` or `xdvipdfmx` run; cores side by side, not merged; the
  no-xdvipdfmx writer; per-binary size and start-up measured at U3a (Q10).
- **§3:** the second binary, the runtime crate, the Unicode core and its linked libraries in the
  diagram and the licence list (§6 here); `flashtex-host`'s licence list unchanged.
- **§4.1 (design question for the Commander):** Native's improved algorithms (§10.2, N4) are
  behaviour changes to the XeTeX translation that no reference engine has. DESIGN.md §4.1 today
  admits change-file hunks to match the reference. The amendment must say whether
  level-guarded hunks (inert at level 0, proven by Unicode's gates) are admissible in the XeTeX
  change files, or whether improved algorithms must live outside the generated core (for
  example as a separately gated module the core calls through a hook). Until it does, N4 does
  not start; N1–N3 need no such hunks.
- **§4.3:** unchanged and restated for every mode: no lenient mode, Native included.
- **§4.4:** the `xelatex` format and the bundle's Unicode contents; the `[fonts]` preamble
  generator and its injection points (§4.5).
- **Manifest:** `[project] mode = "classic" | "unicode" | "flashtex"`, `flashtex_version`, and
  `[fonts]` semantics per mode (Q3, Q5).
- **§10/§12:** Q1's ruling on P5 retirement (old engine kept as the explicit fallback for
  Unicode-routed documents until M3's gate); M0–M6 beside P0–P6 and T0–T3.
- **§13:** rows for the owner's 2026-10-04 decisions (XeTeX in-engine, the bundle on GitHub
  Releases), this proposal's adoption with rulings Q1–Q5 and Q7–Q10, Q8 as the Commander's
  decision, and **Q6 open**.

---

## 10. Native ("FlashTeX") mode

Owner ruling (Q5, 2026-10-04): a non-compatible, more modern mode for projects made exclusively
with FlashTeX, **in addition to** Classic, Unicode and Typst. Working name "FlashTeX mode"
(product text) or Native (design text). This section is a design sketch; M6 (§7) builds it.

### 10.1 Principles

- **LaTeX syntax, Unicode core.** A Native document is ordinary LaTeX run by the Unicode core
  (`xetex.web` translation, `xelatex.fmt`, the real `latex.ltx` and packages). Existing LaTeX
  knowledge and packages transfer; nothing is a new language.
- **Opt-in, never silent, never default** (§1.2): selected only by `mode = "flashtex"`, written
  by the user's explicit choice (New Project or the mode menu, with a confirmation).
- **Labelled.** The mode item reads "FlashTeX", its menu and New Project say "not reproducible
  by other LaTeX engines", and the PDF's XMP/Info `Creator` names FlashTeX mode and its
  `flashtex_version`.
- **Pinned.** Output depends only on the source, the pins (`flashtex_version`, packages, fonts,
  Q7's font list) and the read set; upgrading FlashTeX never changes a Native document's pages
  unless the user raises the pin (§10.3).
- **Strict.** DESIGN.md §4.3 holds: an unknown primitive is a hard error.

### 10.2 What Native adds over Unicode

| Feature | How | Exportable (§10.5)? | Step |
|---|---|---|---|
| **`[fonts]` applied automatically**, any installed system font including OpenType math | §4.5's generator, on by default; the font index (`flashtex-font-discovery`) supplies files; Q7's per-project font list (family → SHA-256) is recorded | Yes: written as `fontspec`/`unicode-math` lines | N1 |
| **Modern defaults** | Unicode text and input (UTF-8, `fontspec`), microtype-style **protrusion** (microtype supports protrusion under XeTeX, not expansion; *belief, to confirm against microtype's manual in N1*), sensible page geometry (`geometry` with margins derived from the class's paper size), hyperref with sane link styling. Each default is a generated preamble line, overridable by the document, and listed in *Show effective preamble* | Yes: written as preamble lines | N1 |
| **References resolved in one compile** | The convergence driver iterates `.aux`, `.toc`, `.bbl`/`.ind` (with the external tools, DESIGN.md §5.5) to a fixed point before pages are reported final; intermediate pages are shown marked stale. Classic and Unicode already iterate inside one `compile` (DESIGN.md §5.5 "as built", capped at 5 passes) but must keep LaTeX's log, including its "Rerun" warnings; Native defines its output **as** the fixed point and replaces those warnings with a structured diagnostic. Oscillation or the cap is an error naming the labels that did not settle | Partly: export adds a `latexmkrc` (`$pdf_mode = 5`, xelatex) so stock tools rerun as needed | N3 |
| **Richer structured diagnostics** | DESIGN.md §15.7 / TY6's side channel, plus Native-only checks that need not match any log: undefined and multiply defined labels with both locations, overfull boxes with the offending source span, unused `[fonts]` roles, fonts substituted | Not applicable (diagnostics only) | N1, N3 |
| **Improved algorithms (optional)**, e.g. better line breaking or float placement | Each is introduced at a new `flashtex_version` level and is off below it; at level 0 Native lays out exactly as Unicode. Where the code lives is a DESIGN.md §4.1 question (§9) | **No**: export reports which pages may change | N4 |

Nothing in this table changes Unicode or Classic output: injection, defaults and diagnostics are
per mode in the shared runtime; algorithm levels are 0 outside Native (G4).

### 10.3 Manifest and version pin

```toml
[project]
entry = "main.tex"
mode = "flashtex"
flashtex_version = "1.0"   # Native behaviour level; required in this mode

[fonts]
text = "Source Serif 4"
math = "STIX Two Math"
```

- `flashtex_version` names a **Native behaviour level**, not the app's marketing version: the
  set of defaults and algorithms in force. A FlashTeX release implements every level up to its
  own and keeps each earlier one byte-stable (§10.6).
- **Missing pin:** writing `mode = "flashtex"` from the app or `flashtex manifest init --mode
  flashtex` always writes the current level. A hand-written manifest without it is an error that
  offers to pin the current level; Native never floats.
- **Pin newer than this FlashTeX:** an error naming the level needed; the project opens
  read-only on its stored pages (§4.3).
- **Raising the pin** is an explicit action (*Upgrade FlashTeX mode…* / `flashtex manifest
  set flashtex_version`) that compiles both levels and shows which pages change before writing
  the key.

### 10.4 Live Share, the app and the CLI

- **Live Share:** the hub pins `mode` and `flashtex_version` in `join_ack` with the other
  session pins (live-collab §5.3) and Q7's font list in the environment digest. A peer whose
  FlashTeX lacks the level, or whose fonts differ, is told and joins **view-only** on the hub's
  pages; it is never compiled at another level.
- **App:** New Project's fourth entry "FlashTeX-only (not portable)", and *Switch to FlashTeX
  mode…* in the mode menu, each with a confirmation listing what changes and that other engines
  will not reproduce the pages. Switching back to Unicode is always offered and is lossless for
  the source (the defaults then come only from what the document states, or from an export).
- **CLI:** `flashtex build` honours `mode = "flashtex"` and runs `flashtex-host-unicode --mode
  native`; `--mode flashtex` overrides for one build, as §4.1 row 2 does for the others.
  `flashtex export --portable <dir>` is §10.5. `flashtex build --print-effective-preamble`
  prints the generated setup for any mode.

### 10.5 "Export portable LaTeX" (one-way)

Produces a new folder (never edits the project) that builds with stock `xelatex` (TeX Live 2026):

1. **Flatten settings into source:** the effective preamble (`[fonts]` roles, defaults) is
   written literally after `\documentclass`, each block commented `% from FlashTeX mode,
   flashtex_version 1.0`; `% !TEX program = xelatex` is added.
2. **Manifest:** `mode = "unicode"`, no `flashtex_version`; `[fonts]` removed (now in source);
   `[packages]` pins kept.
3. **Build files:** a `latexmkrc` for the reruns Native did implicitly; `.bib` and local
   `.sty`/`.cls` copied.
4. **Fonts:** copied into `fonts/` with licence files only when the licence permits
   redistribution (§6); otherwise listed in `PORTABLE.md` with the family and SHA-256 Q7
   recorded.
5. **Report (`PORTABLE.md`):** what was flattened; for each improved-algorithm level in force,
   the pages whose layout `xelatex` will produce differently, found by compiling the export in
   Unicode mode and comparing pages.

It is one-way: re-importing gives a Unicode project, not a Native one. Pages are identical
between the Native project and its export exactly when the pin's algorithm level is 0, which is
also the N1/N2 gate (§10.6).

### 10.6 Gate (no external oracle)

| Check | What it compares | When |
|---|---|---|
| **Golden output per level** | For a Native corpus (templates, theses, papers, `[fonts]` and math-font variety): box dumps (P-T1 form), display-list pages and PDF pages at 2×/3×, stored per `flashtex_version` level. Any change to an existing level's goldens fails | Every PR touching `flashtex-host-unicode`, the runtime or the generator |
| **Level 0 = Unicode** | At level 0, Native's pages equal Unicode mode's on the exported source, and so `xelatex`'s through Unicode's own gate (Q2) | N1 onward |
| **Export builds** | Every corpus document's export compiles with stock `xelatex` (TeX Live 2026), no errors | N2 onward |
| **Determinism** | Same input twice, cold vs incremental (soundness sweeps, R5 of DESIGN.md), macOS vs Linux CI, any thread count: byte-identical PDF and display list | Every PR as above |
| **Convergence** | Fixed-point references settle within the cap on the corpus; constructed oscillating documents give the diagnostic | N3 onward |
| **Isolation** | Unicode P-T1/XDV and Classic P-T1/P-T2, lockstep and latency unchanged by any Native change | Always |
| **New level review** | A new level's golden changes are reviewed by a human with before/after pages and recorded with the level | N4, per level |
