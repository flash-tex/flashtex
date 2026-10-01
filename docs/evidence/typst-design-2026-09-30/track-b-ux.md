# Typst design, Track B: editing parity in both directions

- **Task:** TYPST-DESIGN-B (kabir-claude, mac-m5pro-kabir). This is research only; no
  product code was changed.
- **Base:** `origin/main` 67a2ea078. DESIGN.md §15 was read from `origin/agent/kabir-claude/design-typst`
  (1f6fda3cb). That section is not on main yet.
- **Owner requirement:** Typst "should integrate nicely and feel just as nice as LaTeX editing
  (and vice versa)".
- **Constraints taken from DESIGN.md:**
  - Typst runs in its own host process, `flashtex-typst-host` (MIT/Apache).
  - It emits `display-list-v3` over the same socket protocol, and the app picks the engine by file type (§15).
  - Nothing Apache-licensed links into `flashtex-engine`.
  - Typst lanes never take priority over LaTeX work (§15).
  - Reuse before building (§1).

Each claim is marked as either **verified** or **belief**. Verified means read in code or
docs, or queried from crates.io. Belief means an estimate or design judgement. §9 lists the
beliefs together.

---

## 1. Inventory: the app's editing features today

Every row was verified from the code on 67a2ea078: file paths, doc comments and `wc -l`.
Quality is my assessment from the code, doc comments and the named tests. I did not run
the app.

- `apps/mac/Sources` holds 155 Swift files, 67,074 lines in total.
- `FlashTeXEditorCore` is Foundation-only and is symlinked into the iPad app.

The **Lang** column says how the feature depends on the language:

- **L**: LaTeX-specific logic.
- **G**: generic UI or mechanism.
- **L/G**: a LaTeX model behind a generic UI.

| # | Feature | Where | Lang | Quality / notes |
|---|---|---|---|---|
| 1 | Syntax highlighting | `FlashTeXEditorCore/SyntaxHighlighter.swift` (790 lines: incremental per-line lexer, `Language` = latex/bibtex/toml/package, 13 semantic roles); `FlashTeXMac/SyntaxHighlighter.swift` (theme, `SyntaxPainter` using temporary attributes) | L/G | Strong. Incremental re-lex is tested to equal a fresh lex. On iPad it measures 0.75 ms per keystroke at 200 KB. The role-based theme is language-neutral. |
| 2 | Completion | `Completion.swift` (4,053 lines) | L/G | Strong UI, lexical data. **Popup:** `CompletionPopup` with a docs pane and font samples. **Sources:** `\end{…}`; commands from `supported-latex.json`; commands declared with `\newcommand`, in `.sty` files, or in `metadata.packages`; environments, labels, citations (`BibScanner.swift`), files, words, and `\setmainfont` families. **Snippets:** tab-stop snippets (`LaTeXSnippet`). **Staleness:** results are bound to the revision. **Caveat:** the vocabulary is the *old* compiler's inventory, and the "not supported by this compiler version" marks become meaningless under engine v2. |
| 3 | Signature help | `SignatureHelp.swift` | L/G | Good. It is aware of optional arguments, and ⌘⇧Space shows it. |
| 4 | Hover | `EditorIntelligence.quickInfo`, `EditorHoverResolution.swift`, `MathHoverPreview.swift`, `ShellModel+EditorHover.swift` | L/G | Good. **Command documentation:** yes. **Labels:** `\ref` shows the section or float a label sits in, but *not its number* ("Numbers are deliberately absent"). **Citations:** a `\cite` hover shows the `.bib` record. **Math:** a hovered inline formula is cropped from the page bitmap. **Diagnostics:** shown on hover. |
| 5 | Go to definition / peek | `EditorIntelligence.definitionTarget`, `EditorNavigation.swift`, `ShellModel+PackageNavigation.swift` | L | Lexical. Covers user macros, labels, files, and package definitions via `metadata.packages` spans. |
| 6 | Rename | `EditorNavigation.renamePlan` (labels and user commands across open documents), `CitationRename.swift` (helper `plan_citation_rename`) | L | Lexical and reviewed. It sees only open documents or the helper's index, not the real include graph. |
| 7 | Brace matching, auto-close | `FlashTeXEditorCore/BraceMatcher.swift`, `AutoClose.swift` | L/G | Strong. Handles `$…$` parity, `\verb`, `%`, and the pairs `\(`/`\[`/`\left(`. Shared with iPad. |
| 8 | Return key, environment rules | `FlashTeXEditorCore/LaTeXEditing.swift`, `EnvironmentEditingRules.swift`, `EnvironmentRulesSettings.swift` | L | Good, and user-configurable per environment. |
| 9 | Folding | `EditorFolding.swift` (glyph hiding in TextKit 1) | L/G | Good. Folds environments and sectioning blocks. The fold mechanism is generic. |
| 10 | Outline and structure | `DocumentOutline.swift`, `WorkspaceSidebar.swift`, `SidebarTree.swift` (`NSOutlineView`) | L/G | Lexical, per buffer. Covers sections, environments, labels and captions, and is rescanned about 150 ms after an edit. Has no numbers or pages. |
| 11 | Diagnostics | `EditorDiagnostics.swift` (1,194 lines), `ErrorLens.swift`, `ProblemsPanel.swift`, `DiagnosticsPanel.swift`, `ExplanationMemo.swift` | G (data L) | Strong UI. **Placement:** underlines rebased across edits, retained over failed compiles, and never drawn under the wrong text. **Problems panel:** grouping ("12 places"), keyboard navigation, copying as `path:line: msg`, and an error lens. **Explanations:** offline `flashtex-explain` explanations and reviewed quick fixes (`QuickFix`, `MissingIncludeFix`). **Data:** today's byte spans come from the *old* compiler (runtime-v1). |
| 12 | Spellcheck | `LaTeXSpellCheck.swift` (`LaTeXProse` ranges plus `NSSpellChecker`, background) | L/G | Good. `crates/spellcheck` (Rust, offline) exists but the Mac app does not use it. |
| 13 | Word count | `DocumentStatistics.swift`, `WordCountStatusView.swift` | L | texcount-style: counts body, headers and captions, excludes math and the preamble. |
| 14 | Editing commands | Wrap in Environment/Command, `EditorChangeEnvironment.swift` (⌃⌘E plus linked editing), `EditorLineCommands.swift`, `EditorIndentation.swift` (⌃I), Toggle Comment, `EditorFind.swift`, `VimMode.swift` (2,378 lines) | mixed | Good. Vim, find and the line commands are generic. Wrap, change-environment and re-indent are LaTeX-specific. |
| 15 | Project search | `ProjectSearchPanel.swift` (helper `search_literal`) | G | Good. Revision-exact, and refuses stale matches. |
| 16 | Accessibility | `EditorRotor.swift` (rotors for headings, environments and diagnostics), `FlashTeXAccessibility/*` | L/G | Good. The rotor items come from a LaTeX scan. |
| 17 | Preview renderer | `PreviewV2View.swift`, `GlyphRunRenderer.swift`, `V2PageCache.swift`, `PreviewZoom.swift`, `PreviewAnchor.swift` | G | Strong. Engine-agnostic: it consumes a display list. Tiling is planned (§6.2). |
| 18 | Source⇄preview sync | `CaretSync.swift`, `CaretFollow.swift` (debounced auto-follow), `MathCaretHighlight.swift`, `Navigation.navigateExactly`, `DisplayListLinks.swift` | G | Strong. Exact byte spans; stale spans are refused. **v3 change:** spans become `(file, line, col)` ids (spec §5.3). |
| 19 | Stale and last-good preview | `HistoricalPreview.swift`, `ShellModel+DiagnosticRetention.swift` | G | Good. A historical frame is display-only. |
| 20 | Main file detection | `ProjectManifest.entry` | L | Basic. Uses the `flashtex.toml` `[project] entry`, else the folder's *only* `.tex`; with several `.tex` files it refuses. |
| 21 | Multi-file projects | `ProjectDocuments.swift` (`\input`/`\include` scan), `DocumentTabBar.swift`, `ProjectScaffold.swift` (create a missing include; rename or delete with reference rewrite) | L/G | Good. Lexical: `\input{\jobname}` is reported as non-literal. |
| 22 | Bibliography | `BibScanner.swift`, `DocumentKinds.swift` | L (reusable) | Good. The `.bib` parser serves hover and completion, and **can serve Typst unchanged**. |
| 23 | Packages and fonts | `ProjectPackages.swift` (consent sheet: Fetch, Not now, Never), `ProjectFonts.swift` (`flashtex.toml [fonts]`) | L/G | The consent UX is reusable. The resolution logic belongs to the old engine. |
| 24 | New project and new file | `ProjectScaffold.swift`, `ProjectScaffoldViews.swift`: 7 templates (blank article, article with sections, report, homework, thesis, lecture notes, beamer) | L | Good. LaTeX only. |
| 25 | Files and durability | `DocumentFiles.swift`, `DocumentWatcher.swift`, `EditLedgerClient.swift`, `EditHistoryPanel.swift` | G | Strong. Rooted paths, conflicts, durable undo. |
| 26 | Export and print | `ExactPDFExport.swift` (`flashtex-pdf-exact from-v2`), `PrintController.swift` | G | Under engine v2, export becomes the host's `export: true` (spec §6.3). |
| 27 | Settings | `EditorPreferences.swift` (font, wrap, tab/indent, appearance, auto-close, completion, spellcheck, Vim, follow caret, relative numbers, autosave, environment rules, updates), with Editor, Compile and Conversion tabs | mostly G | Good. |
| 28 | Command palette and shortcuts | `CommandPalette.swift` over the `AccessibilityCommand` table (about 105 cases), `PaletteResults.swift` | G | Strong. One table serves the menus, the palette and VoiceOver. |
| 29 | AI and assistant | `ConversionCredential.swift` (capture → LaTeX through the bridge; "the ONLY model-backed feature"), `CaptureInbox.swift`, `CaretContext.swift` (mode-aware insertion, checked against pdflatex) | L | Good. The output language is LaTeX/TikZ only. |
| 30 | iPad companion | `apps/ios`: PencilKit/photo capture → Mac conversion; a local `.tex` editor using the shared `FlashTeXEditorCore` (highlighting, auto-close, Return rules, completion); no compile | L/G | A capture companion by design (D14). |

**Structural finding (verified):**

- There is no language abstraction today.
  - 36 of the files in `FlashTeXMac` call LaTeX scanners directly: `SyntaxHighlighter`,
    `Completion.`, `EditorNavigation.`, `DocumentOutline`, `LaTeXProse` and similar.
  - The only seed of one is `SyntaxHighlighter.Language` (latex/bibtex/toml/package) together
    with the role-based theme.
- Every helper is a subprocess speaking JSON Lines: `flashtex-preview-controller`, `-explain`,
  `-render`, `-pdf-exact`, `-project-files`, `-bridge` and `-edit-ledger`. The app links no
  Rust.
- The app does not consume `display-list-v3` yet. `grep` finds no v3 client in `apps/mac`.

## 2. Mapping to Typst: what the tooling gives us

### 2.1 Upstream components

Versions and licences were verified on crates.io and docs.rs on 2026-09-29.

| Component | Version | Licence | What it gives | Use |
|---|---|---|---|---|
| `typst-syntax` | 0.15.1 | Apache-2.0 | The **real parser**. `Source::edit(range, with) -> reparsed range` reparses incrementally. `LinkedNode`. `highlight(&LinkedNode) -> Option<Tag>` with 22 tags (Comment, Punctuation, Escape, Strong, Emph, Link, Raw, Label, Ref, Heading, ListMarker, ListTerm, MathDelimiter, MathOperator, MathGroupingParens, Keyword, Operator, Number, String, Function, Interpolated, Error). `Lines` converts UTF-8, UTF-16 and line indices. | In-process syntax tier (§4.2) |
| `typst-ide` | 0.15.1 | Apache-2.0 | `autocomplete(world, output, source, cursor, explicit)` returns items whose `apply` uses `${…}` snippets; `tooltip`, `definition`, `jump_from_click`, `jump_from_cursor`, `analyze_labels`, `analyze_import`, `analyze_expr`, `named_items`, and the `IdeWorld` trait. **Label completions and label tooltips need the compiled document** ("only generated when the document is available"). | Typst host, semantic tier |
| `tinymist-query` | 0.15.8 | Apache-2.0 | Completion, hover, goto definition and declaration, **references, prepare-rename, rename, will-rename-files**, document symbols, workspace symbols, folding, selection range, **semantic tokens** (full and delta), **inlay hints**, **code actions**, code lens, signature help, document highlight, links, colours, on-enter, `.bib` support. Requests are either `SyntaxRequest` (a `Source` only) or `SemanticRequest` (`LocalContext`). It depends on `typst ^0.15.1`, `lsp-types =0.95.0`, `tinymist-world`/`-analysis`/`-lint`, `hayagriva` and `biblatex`. **Its README says it "doesn't ensure stable APIs".** | Typst host, semantic tier (phase 2), pinned exactly |
| `tinymist` (LSP binary) | 0.15.2 on crates.io | Apache-2.0 | A complete LSP server plus a browser preview. | Prototype and fallback route only (§4.4) |
| `tinymist-preview` | 0.15.8 | MIT | A web or SVG preview over a websocket. | **Not used.** Our native CG preview and `display-list-v3` replace it. |
| `typstyle-core` | 0.15.1 | Apache-2.0 | The formatter tinymist uses. | Typst host: format document or selection |
| `hayagriva` / `biblatex` | 0.10.1 / 0.12.0 | MIT OR Apache-2.0 | Bibliography formatting from `.bib` or Hayagriva `.yml`. | Already used by the typst compiler; hover can use it too |
| `typst-kit` | 0.15.1 | Apache-2.0 | Font search, package storage and downloader, file watcher (feature-gated). | Typst host: packages and fonts |
| `tree-sitter-typst` (uben0) | git only, not on crates.io | MIT | An unofficial grammar. Its README says "Typst doesn't have yet an official Tree-Sitter grammar" and that a rewrite is underway. | **Rejected.** It lags the real parser, and `typst-syntax` is the parser itself. |

Licence position: Apache-2.0 components may sit on the MIT side of the app and host. They
must never go into `flashtex-engine` (GPL-2.0). We must ship their NOTICE and licence texts
(verified from the licences; the policy is DESIGN §3/§15).

### 2.2 Feature-by-feature mapping

**Same UI** means the Swift view and interaction are reused unchanged; only the provider
differs.

| Feature (§1 #) | LaTeX provider (today → v2) | Typst provider | Same UI? | Gap and effort (belief) |
|---|---|---|---|---|
| Highlighting (1) | Swift lexer | `typst-syntax` highlight tags mapped onto the existing roles, plus 5 new roles (strong, emph, heading, keyword/function, error) | yes; the theme gets new roles | FFI staticlib and tag mapping: 4–6 days |
| Brace match and auto-close (7) | Swift | Pairs from a per-language table (`[]`, `()`, `{}`, `$…$`, `"…"`, raw backticks). Matching comes from the syntax tree, which is exact. | yes | Generalise `AutoClose`'s pair table: 1–2 days |
| Return rules (8) | Environment rules | Continue list markers (`- `, `+ `, `/ term: `) and `//` comments; indent inside `{}`/`[]` (tinymist `on_enter`) | yes; a Settings page per language | 2–3 days |
| Folding (9) | Environments and sections | Headings by level, code blocks and content blocks (syntax tree, or tinymist `folding_range`) | yes | 1–2 days on the syntax tier |
| Outline (10) | Lexical | Instant headings from the syntax tree, then numbers and pages from the host (introspection via `analyze_labels`, tinymist `document_symbol`) | yes | 2–3 days |
| Completion (2) | Lexical plus inventory | `typst_ide::autocomplete`: functions, parameters, fields, packages, labels, citations and fonts from the *compiled* world | yes; snippets `${x}` → tab stops | Host endpoint 3–4 days; generalise the snippet type 1 day |
| Signature help (3) | Swift | tinymist `SignatureHelpRequest` (typst-ide has none) | yes | Phase 2 |
| Hover (4) | Swift, no numbers | `typst_ide::tooltip`: evaluated values, font lists, **label bodies**, and docs for built-ins. The bitmap-crop math hover works once v3 carries spans. | yes | 1–2 days |
| Definition (5) | Lexical | `typst_ide::definition` (cross-file, into packages) | yes | 1 day |
| References and rename (6) | Lexical labels and macros; citation plan | tinymist `references`, `prepare_rename`, `rename`, `will_rename_files` (fixes imports when a file moves). Applied through the app's existing reviewed plan UI. | yes | Phase 2: 3–4 days after tinymist is embedded |
| Diagnostics (11) | runtime-v1 byte spans → v2 file:line only | `SourceDiagnostic {severity, span, message, trace, hints}`: exact span ranges, a call trace and hints. Lints from `tinymist-lint`. | yes, with trace and hint sub-rows added for **both** languages | 3 days for the UI, 1 day for Typst plumbing |
| Quick fixes (11) | Explanation catalogue plus reviewed edits | tinymist `code_action` edits, shown through the same `QuickFix` preview and apply path | yes | Phase 2 |
| Formatting (new) | Only re-indent (⌃I) | `typstyle-core` (document or selection) | yes: Format Document (⌥⇧F) | 1–2 days |
| Spellcheck (12) | `LaTeXProse` | Prose = markup `Text` nodes, excluding code, math, raw, labels, refs and URLs (syntax tree) | yes | 1–2 days |
| Word count (13) | texcount-style | Same rule from the syntax tree: markup text, headings and figure captions | yes | 1–2 days |
| Wrap and change environment (14) | LaTeX | "Wrap in function" (`#emph[…]`, `#box[…]`) and "Change function". No environments. | Palette entries per language | Later; low value |
| Toggle comment | `%` | `//` | yes | Hours |
| Multi-file (21) | `\input`/`\include` scan | `#include "x.typ"`, `#import "x.typ"` from the syntax tree; `analyze_import` | yes | 2 days |
| Main file (20) | Only `.tex`, or the manifest | Candidates are `.typ` files not imported or included by another `.typ` (§4.5) | yes | 2 days for both languages |
| Bibliography (22) | `BibScanner` `.bib` | Same `.bib` reader for hover and completion when needed. Hayagriva `.yml` via the host. `@key` and `#cite(<key>)` completion comes from typst-ide. | yes | 1 day (`.yml` in the host) |
| Packages (23) | Consent sheet (old engine) | `@preview/name:ver` fetched from packages.typst.org into `~/Library/Caches/typst/packages/preview`, with `@local` from `~/Library/Application Support/typst/packages` taking precedence (verified, typst/packages README). Same consent sheet. | yes | 2–3 days |
| Templates (24) | 7 LaTeX templates | Bundled Typst equivalents plus "From Typst Universe…" (a template package's `template/` directory, `typst init` semantics) | yes; engine picker | 3 days |
| Preview, sync, zoom (17–19) | v3 spans | The host emits v3 `SOURCES` from Typst glyph spans (each glyph carries a span and an offset), so forward and inverse search use the same v3 path. `jump_from_click` refines non-text clicks such as images and links. | yes, identical | This is Track A's host work |
| Export (26) | Host `export: true` | `typst-pdf` (with PDF/A and PDF/UA options); PNG/SVG optional | yes | Track A |
| Capture conversion (29) | LaTeX/TikZ output | Typst math and CeTZ output. `CaretContext` needs a Typst mode (markup, math or code) from the syntax tree. | yes | 3–4 days (bridge prompt plus a Swift/Rust mode table) |
| iPad editor (30) | Shared Swift core | Needs the same `typst-syntax` FFI built for iOS. Apache-2.0 suits the App Store. | yes | 2 days beyond the Mac FFI |

**Missing on the Typst side, even with typst-ide and tinymist:**

- A texcount-equivalent word count and LaTeX-style environment editing rules. Both are
  cheap to build.
- The explanation catalogue. Typst's own hints make it largely unnecessary.

**Missing on the LaTeX side, relative to what Typst users get:** see §3.

## 3. Vice versa: Typst strengths LaTeX users should get

Our faithful engine knows several things a lexical LaTeX IDE cannot. The v3 spec already
records the reading position for every node it allocates, as `SOURCES` per-glyph `col`
(spec §5.3). It also holds the full `eqtb`/hash state at every checkpoint, and it reads
the log, the `.aux` file and the include graph as pdflatex would.

The main regression risk (verified from the spec): the v3 `DIAGNOSTIC` is
`{severity, message, file?, line?}`. It has no column and no range, so without work
**LaTeX diagnostics under engine v2 are less precise than today's runtime-v1 byte spans**.

Ranked by value for cost. Every item must leave P-T1 logs untouched: they are side
channels, never changes to terminal or log output.

| # | Win | How, given our engine | Cost (belief) |
|---|---|---|---|
| V1 | **Well-located diagnostics with a column, a trace and hints** (Typst: span, trace, hints) | See the note below this table. | Engine hook 3–5 days; protocol and app 2 days. **Do first**; it closes the v2 regression. |
| V2 | **Hover on `\ref`/`\eqref`/`\cite` shows the real number, page and formatted citation** (Typst tooltips show label content) | After a run, the engine's `\r@key` meaning is `{{2.1}{3}…}`, and `\b@key` holds the cite label. The engine exports them in an `INDEX` message. This removes today's "numbers deliberately absent" limit. It also allows **inlay hints** (`\ref{eq:x}` shows "(2.1)"), mirroring Typst's inlay hints. | 2–3 days once `INDEX` exists |
| V3 | **Engine-truth completion and hover docs**: every command and environment actually defined at the caret, with its parameter text and **definition site** | The engine walks the hash table on request, with no per-keystroke cost. It also keeps a side table *cs → (file, line)* updated at `\def`/`\let` assignment, excluded from the state hash and snapshotted with checkpoints. Together these replace the old `supported-latex.json` inventory (which is obsolete under v2) and give **Go to Definition into any `.sty`/`.cls`**, matching `typst_ide::definition` into packages. | 6–9 days (engine side table plus `INDEX` query plus app) |
| V4 | **Rename labels and macros across the *real* include graph** (tinymist rename) | The engine reports every file it opened (kpathsea-resolved `\input`, `\include`, `\subfile`, `\import`). The rename plan covers exactly those files, and warns when a label is defined inside macro expansion, where the definition site is not literal. | 2–3 days on top of V3 |
| V5 | **Structural outline with real numbers and pages** (Typst outline via introspection) | Take the `\addcontentsline`/`\@writefile{toc}` records the engine ships, merge them with the instant lexical outline (lexical first, numbers when they arrive), and include every included file. | 3–4 days |
| V6 | **Formatting that is proven safe** (typstyle) | Embed `tex-fmt` (Rust, MIT, 0.5.7). Alternatively offer `latexindent` (GPL-3) from the user's TeX Live as a subprocess only. Run the formatted text through the resident engine and accept it only if the box dumps are identical (the P-T1 machinery). **No LaTeX IDE can offer that.** | 5–7 days |
| V7 | **Last-good semantics without flicker** (Typst keeps the last good render) | See §5.2. It needs only v3 `PAGES` stale ranges plus a UI rule, both of which already exist. | 1–2 days of app work |
| V8 | Semantic highlighting from real catcodes (`\makeatletter`, verbatim-like and custom regimes) | Record the catcode class per source character inside `get_next`. This taxes the hot path. | High; **defer** until after the L6 speed work |

**How V1 works.** Most of it comes from state the engine already has:

- **Column.** At `print_err`, the input stack's `loc` gives the column of the split point that
  TeX's own error context prints on its `l.42 …` line. That puts the column on the
  `DIAGNOSTIC`.
- **Trace.** The token-list levels of the input stack name the macros being expanded, like
  Typst's "error occurred in this call of function x".
- **Hints.** TeX's `help_line`s are already written to the log in nonstopmode; they become
  hints.
- **Warning ranges.** LaTeX and package warnings such as `on input line N` and the
  over/underfull `lines a--b` carry ranges that can be parsed.

## 4. Unified architecture for language features

### 4.1 Tiers

```
┌──────────────── Mac / iPad app (Swift, MIT) ─────────────────────────────┐
│ Shared UI: editor, completion popup, hover, signature panel, Problems,   │
│ outline tree, folds, spell painter, palette, preview                     │
│        │ LanguageService (per document, chosen by file type)             │
│   ┌────┴──────────── syntax tier: in process, synchronous, <1 ms ──────┐ │
│   │ SyntaxProvider: LaTeX = FlashTeXEditorCore (Swift, as today)       │ │
│   │                 Typst = flashtex-typst-syntax (Rust staticlib, C   │ │
│   │                 ABI, typst-syntax only; Apache/MIT)                │ │
│   └────────────────────────────────────────────────────────────────────┘ │
│        │ semantic tier: async, revision-bound, off the main thread       │
└────────┼─────────────────────────────────────────────────────────────────┘
         │ same Unix socket as display-list-v3: new LANG_* and INDEX frames
 ┌───────┴──────────── flashtex-host (GPL) ───┐  ┌── flashtex-typst-host (MIT/Apache) ──┐
 │ engine INDEX export: labels and numbers,   │  │ typst World + comemo + last document │
 │ cs table and definition sites, outline,    │  │ typst-ide · tinymist-query (pinned)  │
 │ include graph; structured DIAGNOSTIC (V1)  │  │ typstyle-core · hayagriva · typst-kit│
 └────────────────────────────────────────────┘  └──────────────────────────────────────┘
```

- **The syntax tier stays local and synchronous.** Highlighting, bracket matching, auto-close,
  Return and comment rules, prose ranges, folds, the lexical outline and the include scan run
  on every keystroke. A process hop would put them behind a compile.
  - LaTeX keeps its tested Swift implementation.
  - Typst gets a tiny Rust staticlib exposing `typst-syntax` to Swift. It holds a `Source`
    with `edit()` and returns tag runs for the reparsed range, plus tree queries.
  - This is the app's first linked Rust, and it is Apache/MIT only. The CI licence check must
    extend to it.
  - It builds for macOS and iOS, and it is where portable (cross-platform) syntax logic should
    grow.
- **The semantic tier lives in the engine host that owns the compiled state.** `typst-ide`
  needs the `World` and the *last compiled document* for labels, citations and value tooltips
  (verified API). A separate process would have to compile the document a second time.
  - The Typst host answers `LANG_*` requests on a worker thread, against an immutable
    snapshot (the `World` plus an `Arc` of the document), so a request never waits behind a
    compile. This is a belief: typst's World is `Sync` and comemo is thread-safe, and it
    should be checked in the prototype.
  - The LaTeX host is single-threaded (one resident engine), so it answers from an `INDEX` it
    exports after `DONE`, or lazily on request between compiles. The Swift LaTeX provider
    merges that index with its lexical features: lexical answers are instant and the index
    adds correctness.

### 4.2 Protocol: `lang-v1` frames on the same socket

**Chosen: our own messages on the same socket.** They carry *LSP-shaped JSON payloads*
(CompletionItem, Hover markdown, Location, WorkspaceEdit, DocumentSymbol,
SemanticTokens), with **UTF-8 byte offsets and our revision binding**:

- Request: `{id, rev, source_versions, kind, path, offset | range, options}`.
- `kind`: `complete`, `hover`, `definition`, `references`, `prepare_rename`, `rename`,
  `symbols`, `folding`, `format`, `code_actions`, `signature`, `inlay`, `semantic_tokens`,
  `index`.
- Reply: `{id, rev, kind, result}`.

The design follows these rules:

- `HELLO.capabilities` lists the `lang` kinds a host supports, and the app hides or disables
  a UI surface that its provider lacks. This is how "same UI, different providers" degrades
  gracefully.
- The existing app rule applies: a reply for an older revision than the buffer is refused and
  never shown.
- tinymist-query's outputs are `lsp-types` values, so they pass through with little
  conversion. Their position encoding can be set to UTF-8 (verified in
  `tinymist-analysis` `PositionEncoding::Utf8`).
- A future LSP shim for external editors (FlashTeX's engine in VS Code) is then thin.

**Rejected: tinymist as an LSP subprocess.** It would be a second compiler of the same
document, meaning double CPU and memory and two comemo caches. Its positions could come from
a different revision than the preview's, and it would need a JSON-RPC client, a second
document-sync path (`didChange`) beside `COMPILE.edits`, and conversions from UTF-16
line/column. It stays a **prototype and fallback** route: Apache-2.0 permits bundling the
binary with NOTICE, and it would give day-one features while the in-process endpoints land.

**Also rejected: texlab** (GPL-3.0 per crates.io). It is lexical and not needed, because our
engine knows more.

### 4.3 Swift contract (sketch)

```swift
protocol SyntaxProvider {          // FlashTeXEditorCore; Foundation only
  var language: DocumentLanguage { get }   // .latex, .latexPackage, .bibtex, .typst, .toml, .yaml
  mutating func edit(utf16 range: NSRange, with: String) -> [TokenRun]   // re-lexed runs
  func match(at: Int) -> Int?; func autoClose(...) -> Closer?; func newline(...) -> Insertion
  var commentPrefix: String { get }; func proseRanges(in: NSRange) -> [NSRange]
  func folds() -> [Fold]; func outline() -> [OutlineItem]; func includes() -> [IncludeRef]
}
protocol SemanticProvider {        // async; results carry the revision they answer
  var capabilities: Set<LangKind> { get }
  func complete(_ at: Position, explicit: Bool) async -> Revisioned<[Suggestion]>
  func hover(_ at: Position) async -> Revisioned<Hover?>   // … definition, references, rename,
}                                                          //   format, codeActions, signature, inlay
```

- Existing LaTeX code moves *behind* these protocols without changing behaviour. `Completion`,
  `EditorIntelligence`, `EditorNavigation`, `DocumentOutline`, `LaTeXProse`,
  `DocumentStatistics` and `EditorFolding` become `LaTeXSyntaxProvider` and
  `LaTeXSemanticProvider`.
- The popup, panels and painters take provider output. `Completion.Suggestion.Kind`,
  `LaTeXSnippet` and the highlighter roles become language-neutral types.
- Effort (belief): 6–9 days of refactoring, gated by the existing hosted tests plus
  TypingBench, which must not regress.

### 4.4 Engine routing and project and file-type detection

- **Language by extension:**
  - `.tex`, `.ltx`: LaTeX.
  - `.sty`, `.cls`, `.def`, `.clo`, `.dtx`: LaTeX package mode.
  - `.bib`: BibTeX. The provider and `BibScanner` are shared, because Typst reads `.bib` too.
  - `.typ`: Typst.
  - `.yml`/`.yaml`: Hayagriva when a `#bibliography(...)` names it, else plain YAML.
  - `flashtex.toml` and `typst.toml`: TOML.
- **Entry (main file):**
  1. The `flashtex.toml [project] entry` wins. The engine follows from its extension, with an
     optional `engine = "pdflatex" | "typst"` for clarity.
  2. Otherwise, candidates are `.tex` files containing `\documentclass` or `\documentstyle`
     (replacing today's "only `.tex`" rule), and `.typ` files that no other `.typ` imports or
     includes (scanned by `typst-syntax`).
  3. With exactly one candidate, open it. With several, including a mixed repo, show one
     picker listing both kinds and offer to write the manifest entry.
- **Mixed repositories** (a `paper.tex` beside `slides.typ`) are two entries.
  - Each gets its own host, as DESIGN already starts one host per open document.
  - Shared assets (`refs.bib`, `figures/`) are ordinary project files for both.
  - Typst's `--root` is the project folder, so the two agree on what is readable.
- **A `.typ` opened alone** gets a single-file project, exactly like a lone `.tex` today.

## 5. UX details that make both feel native

### 5.1 Error presentation (both languages)

- **Problems panel.** Same panel, grouping and keyboard model as today. Each row gains:
  - a location `file:line:col`;
  - **hint lines** (Typst `hints`; LaTeX `help_line`s, V1);
  - an expandable **trace**: "in `#template(...)` at main.typ:3" for Typst, and "while expanding
    `\section` → `\@sect`" for LaTeX, with each entry jumpable.
- **Underline extent:**
  - Typst: the exact span range.
  - LaTeX: from the error column to the end of that token. The whole line is used only when
    no column is known (today's fallback).
- **LaTeX cascades.** Only the first error after an edit is reliably meaningful, so group
  later errors from the same run as "N follow-on errors". Typst errors are mostly
  independent and need no grouping.
- **While typing.** Don't flash red at the caret. Hold back underlines and the error lens on
  the caret's line until typing pauses (about 400 ms). Diagnostics elsewhere update
  immediately. This is a belief-based UX rule and should be measured with TypingBench plus a
  user check.

### 5.2 Preview during errors

- **Typst.** A compile with errors yields **no document** (verified: `typst::compile` returns
  `Warned<SourceResult<T>>` and `Err` on errors). The preview therefore keeps the **last good
  render**, and the header shows a chip: "Last good render · rev 41 · 2 errors".
  - Pages stay crisp, not dimmed; the chip and the Problems badge carry the state.
  - Click-to-source stays allowed where spans rebase cleanly. This is the existing
    `SourceMapping` rule, which is safer than today's historical frame, where it is
    display-only.
- **LaTeX.** nonstopmode keeps going, and the pages shipped are valid (spec §7). They are
  shown immediately, with the errors listed.
  - Pages the run did not re-typeset stay and are marked stale via `PAGES.stale`.
  - After `failed` (the engine died) or zero pages, the last good render stays (today's
    retention logic).
- **Result.** Both languages show "the newest pages that are correct, plus a clear stale or
  error indicator", and neither blanks the preview.

### 5.3 Packages and fonts

- **Typst `@preview`.** On the first network fetch per project, reuse the `ProjectPackages`
  consent sheet (Fetch, Not now, Always for Typst Universe).
  - Show download progress in the status bar.
  - Offline, show a located diagnostic on the `#import` with a Retry action.
  - Complete `@preview/` names and versions from the package index. typst-ide
    `IdeWorld::packages()` supports this.
- **LaTeX under v2.** A missing `.sty` or `.cls` from the user's TeX Live, or from the bundle
  (D12), gets the same sheet, offering the bundle fetch or `tlmgr` user-mode.
- **Fonts.** Typst uses system fonts plus project font paths (typst-kit). The Project Fonts
  picker UI can be reused. For Typst it writes `#set text(font: …)` suggestions, not the
  manifest, since fonts are document code in Typst.

### 5.4 New document and templates

- New Project gets an **engine segmented control (LaTeX | Typst)** over parallel template
  lists (blank, article with sections, report with chapters, homework, thesis, lecture notes,
  slides), so users can switch languages without relearning anything.
- Typst adds "From Typst Universe…", which is behind the consent sheet.
- New File offers the project language's extension and inserts `#include "x.typ"` or
  `\input{x}` at the caret, as today.

### 5.5 Settings and commands

- **Settings > Editor** stays shared.
- **Settings > Languages** has a LaTeX page and a Typst page with the same layout:
  - Return rules (environment rules or list continuation);
  - formatter (tex-fmt with verification, or typstyle), with width and format-on-save;
  - compile options (format and shell-escape for LaTeX; font paths, `sys.inputs` and PDF
    standard for Typst);
  - package network policy.
- **Palette.** Palette and menu commands declare language applicability:
  - ⌘/ toggles `%` or `//`.
  - Wrap in and Change Environment map to their Typst equivalents, or are hidden.
  - VoiceOver rotors read the provider's outline.

## 6. Recommended order (belief)

The LaTeX lanes keep priority (§15). V1 through V5 touch `flashtex-engine`, so they belong to
LaTeX engine lanes and are gated by P-T1. The Typst UI lanes never touch the engine.

1. Provider refactor, with no behaviour change: 6–9 days.
2. V1 structured diagnostics (it closes the v2 regression) and the `lang-v1`/`INDEX` protocol
   specification: 5–7 days.
3. Typst syntax-tier FFI (highlighting, brackets, folds, outline, prose): 4–6 days. It can run
   in parallel with the Track A host.
4. Typst host semantic tier with typst-ide and typstyle (completion, hover, definition,
   labels, format): 6–8 days.
5. LaTeX V2, V3 and V5 (numbers, engine vocabulary, outline): 10–14 days.
6. tinymist-query embedded at an exact pinned version (references, rename, signature,
   inlay, semantic tokens, code actions): 8–12 days. Its API is explicitly unstable, so the
   pin moves only together with `typst`.
7. UX items: consent sheet for `@preview`, templates, Settings > Languages, last-good chip:
   6–9 days.
8. V6 verified formatting: 5–7 days. V8 is deferred.

## 7. Risks

- **tinymist-query's API is unstable**, and it pins `lsp-types =0.95.0`.
  - Mitigation: an adapter crate in the Typst host, one pinned version set per `typst` bump,
    and the LSP-subprocess fallback kept buildable.
- **Typst latency on large documents.** DESIGN B.4 records comemo at 88 ms per edit at 300
  pages. The UI must show stale pages, not block. The latency itself is Track A's concern.
- **First linked Rust in the app** (the syntax staticlib). This affects the build (cargo
  inside the Swift build or a prebuilt xcframework) and the CI licence-boundary check.
- **The LaTeX side table in V3 must stay out of the convergence hash** (D8), and it must
  restore with checkpoints. It needs an engine-lane design review.

## 8. Sources

- Typst crates on crates.io, queried 2026-09-29: typst, typst-ide, typst-syntax, typst-pdf,
  typst-kit 0.15.1 (Apache-2.0); tinymist 0.15.2; tinymist-query and tinymist-analysis
  0.15.8 (Apache-2.0); tinymist-preview 0.15.8 (MIT); typstyle-core 0.15.1 (Apache-2.0);
  hayagriva 0.10.1 and biblatex 0.12.0 (MIT OR Apache-2.0); comemo 0.5.1; tex-fmt 0.5.7 (MIT);
  texlab 4.3.2 (GPL-3.0, last crates.io release).
  - https://crates.io/crates/typst-ide · https://crates.io/crates/tinymist-query ·
    https://crates.io/crates/typstyle-core · https://crates.io/crates/tex-fmt
- typst-ide API: https://docs.rs/typst-ide/0.15.1/typst_ide/ (autocomplete, tooltip,
  jump_from_cursor, `Completion.apply` `${…}` snippets).
- typst-syntax: https://docs.rs/typst-syntax/0.15.1/typst_syntax/ (`Source::edit`
  incremental reparse, `highlight`, `Tag`).
- Compile and diagnostics: https://docs.rs/typst/0.15.1/typst/fn.compile.html ·
  https://docs.rs/typst/0.15.1/typst/diag/struct.SourceDiagnostic.html
- tinymist: https://github.com/Myriad-Dreamin/tinymist (Apache-2.0; feature list).
  tinymist-query 0.15.8 source (`src/lib.rs`: `SyntaxRequest`/`SemanticRequest`, module list,
  dependencies) was downloaded from static.crates.io.
- Preview: https://myriad-dreamin.github.io/tinymist/feature/preview.html (web/SVG over a
  websocket; it does not document behaviour on error).
- Packages: https://github.com/typst/packages (cache and data directories on macOS, `@local`
  precedence).
- tree-sitter-typst: https://github.com/uben0/tree-sitter-typst (MIT, unofficial).
- In-repo: `docs/design/engine-v2/DESIGN.md` §1, §3, §6, B.4; §15 at 1f6fda3cb;
  `docs/protocol/display-list-v3.md` §5.3, §6, §7; the `apps/mac` and `apps/ios` sources
  cited in §1.

## 9. Verified vs belief

- **Verified:**
  - Every app file, path, line count and behaviour quoted in §1, from code and doc comments
    on 67a2ea078. I did not run the app.
  - The v3 `DIAGNOSTIC` fields and `SOURCES` semantics.
  - Crate versions and licences.
  - The typst-ide, typst-syntax and `typst::compile` signatures.
  - tinymist-query's request traits, modules, dependency pins and instability notice.
  - Typst package directories.
  - tinymist-analysis 0.15.8 `PositionEncoding` has `Utf16` (the default) and `Utf8`
    (`src/location.rs`), so LSP-shaped replies can carry our UTF-8 byte offsets directly.
- **Belief:**
  - All effort estimates.
  - That typst's `World` and comemo allow IDE queries concurrent with compiles.
  - That TeX's `loc` and input stack give a usable column and trace at `print_err` without
    changing the log. `help_line`s are in the nonstopmode log (tex.web §90).
  - The cost of the `\def`-site side table.
  - The `tex-fmt` plus box-dump verification design.
  - The UX timing rules (the 400 ms hold at the caret).
  - The claim that `latexindent` is GPL-3.
