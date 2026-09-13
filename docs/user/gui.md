# The FlashTeX Mac app

FlashTeX is a source editor with a live page preview. This guide walks through
the window, projects, editing, compiling, the preview, problems and fixes, PDF
export, capture conversion, the iPad companion window, Preferences, and ends with
the full keyboard-shortcut table. Everything here was checked against the app
sources (`apps/mac`) at the time of writing; where a feature is partial, the
text says so.

## The window

```
┌ Sidebar ────┬ Editor ─────────────────────┬ Preview ───────────────┐
│ Project     │ tabs: main.tex  chapter1.tex│ header: producer, zoom │
│ Outline     │ gutter │ source              │ pages                  │
│ Problems    │        │                     │                        │
├─────────────┴─────────────────────────────┴────────────────────────┤
│ Problems panel (⌘⇧M): errors / warnings / not implemented           │
├──────────────────────────────────────────────────────────────────────┤
│ status bar: r12 · durable r12 · 31 ms · worker · 2 ⚠                 │
└──────────────────────────────────────────────────────────────────────┘
```

- **Sidebar** (left, drag to resize). *Project* lists the open files — the
  entry document first, an orange dot for unsaved changes, `rN` for the
  durable revision — followed by greyed rows for every `\input`/`\include`
  target that exists on disk but is not open yet (click to open). *Outline*
  lists sections (indented by level), environments and labels of the active
  file with line numbers; click to jump. *Problems* shows error / warning /
  not-implemented counts; clicking one opens the panel filtered to it.
- **Editor** (middle) with a tab per open file. Closing a tab (×) only
  detaches the file for this session; includes are rediscovered from the entry
  document. The tab bar's *Project* menu lists the include tree, *Open All
  Includes*, and the file's kind.
- **Preview** (right): the rendered pages. The preview header names the
  attached producer and holds the zoom controls; the window **toolbar** holds
  the *Auto-compile after edits*, *v2 pane* and *Dark preview* switches, the
  Problems toggle and *Commands*.
- **Problems panel** (bottom, ⌘⇧M) and the **status bar** (editor revision,
  last compile latency, route, a live word count — click it for a
  per-section breakdown popover, "M of N words" while there is a selection —
  and problem counts — click the counts to toggle the panel).
- **⌘⇧P** opens the command palette: every command with its shortcut; type to
  filter, Return runs.

## Projects and multi-file documents

A project is a folder with an **entry document** (`main.tex`) and the files it
pulls in with `\input{…}` / `\include{…}`. You can start one from scratch
without leaving the app, or open an existing `.tex` file.

### Creating a project from scratch

1. **File › New Project…** (⌘⌥N). Choose the folder the project folder goes
   in, type a name, and pick a template:
   - **Blank article** — `main.tex` only.
   - **Article with sections** — `main.tex` plus `sections/introduction.tex`
     and `sections/methods.tex` via `\input`.
   - **Report with chapters** — `report` class, `chapters/introduction.tex`
     and `chapters/background.tex` via `\include`.
   - **Homework sheet** — the problem-sheet preamble (geometry, amsmath,
     amssymb, amsthm, enumitem, `\N`/`\Z`/`\Q`/`\R`) with a
     `\problem{n}{points}` macro.

   The files are written to `<folder>/<name>/`, `main.tex` opens as the entry
   document and the include tree is in the sidebar, every member opened in a
   tab. If the folder already holds a template file, FlashTeX asks before
   replacing it; other files in the folder are never touched.
2. **Add a file** with **New File…** (⌘N, the sidebar's **+** button, or
   *New File…* in a project row's context menu). Type a name — subfolders are
   fine (`sections/results` becomes `sections/results.tex`); anything above
   the project root (`..`, absolute paths) is refused. With **Insert `\input`
   at the caret** checked (the default while the entry document is active)
   the reference is inserted at the caret as one undoable edit, then the new
   file opens in a tab. The New File items are enabled once the entry document
   is saved (a file needs a folder to live in).
3. **Type the reference first if you prefer.** Write `\input{chapters/two}`
   in any document: the sidebar shows `chapters/two.tex` as **missing —
   create**; click it and the empty file is created and opened. After a
   compile, the *Problems* row of the engine's "included file not found"
   diagnostic offers **Create chapters/two.tex** — the same action.
4. **Rename… / Delete…** are in a member's context menu (not the entry).
   Rename moves the file and rewrites every `\input`/`\include` that
   points at it in the open documents — one undoable edit per document
   (⌘Z undoes it there). Delete moves the file to the Trash after detaching
   it; references to it show as *missing — create* again. Both are refused
   while the file (or, for Rename, a document that references it) has
   unsaved edits, so nothing is rewritten under you.

### Opening and working in a project

- **Open** a file with *File › Open LaTeX File…* (⌘O). It becomes the
  **entry document** (sent to the engine as `main.tex`, whatever its real
  name) and its folder becomes the project root. If the current buffer is
  unsaved you are asked to Save / Discard / Cancel; a discarded buffer can be
  brought back with *Edit › Restore Discarded Buffer* until you quit.
- **`\input{…}` and `\include{…}`** are scanned lexically (no macro
  expansion; `\input{\jobname}` is reported as non-literal). Targets resolve
  against the project root, `name.tex` before `name`, never above the root
  and never through symlinks. Resolved files show in the sidebar; open them
  (click, ⌘-click the command, or *Open All Includes*) to edit them. On the
  direct route (no preview controller attached) the compile request carries
  the whole resolved include closure automatically — chapters you never
  opened are still compiled, read fresh from disk each time, and recompile
  when they change on disk. An unopened include still shows as a greyed
  sidebar row; opening it makes its buffer (not disk) authoritative, and
  closing it reverts to reading disk. On the helper route (below), an include
  still needs an explicit open: the durable helper compiles only its own
  ledger membership.
- **Saving** (⌘S) is compare-and-replace: if the file changed on disk since
  it was read, you get *File › Resolve On-Disk Conflict…* with **Overwrite /
  Reload / Keep Editing** instead of a silent overwrite. FlashTeX also watches
  open files and surfaces outside edits the same way; *File › Reload From
  Disk…* shows a line-count summary before importing.
- **Unsaved text is never lost silently.** Discarding, detaching, an external
  change or quitting without saving writes a snapshot under
  `~/Library/Application Support/FlashTeX/`; the next time you open that file,
  *File › Restore Unsaved Snapshot…* offers Restore / Discard Snapshot / Later.
- **Durable history.** On the helper route (below) every edit is also
  journaled; *Edit › Durable History…* shows undo/redo stacks that survive a
  crash or relaunch. Without the helper the window says "no preview
  controller attached".

## Editing

- **Syntax highlighting**: comments, commands, `\begin`/`\end` with
  environment names, math (`$…$`, `\[…\]`, math environments), numbers in
  math, dimmed braces, label/ref/cite keys, file arguments of
  `\input`/`\includegraphics`/`\usepackage`, `\newcommand`-defined names;
  verbatim is left plain. Colours follow the editor appearance (System /
  Light / Dark in Preferences).
- **Completion (IntelliSense)**: press **⌃Space** or **Esc** to open the list
  (it does not pop up on its own; typing narrows it once open). ↑/↓ or
  Tab/⇧Tab choose, Return inserts over the typed token as one undo step, Esc
  closes. Sources, in rank order: `\end{X}` for still-open environments;
  commands the engine supports (with snippets; the list is generated from the
  compiler's own inventory, `crates/compiler/supported/supported-latex.json`,
  so it always matches what renders); commands used elsewhere in
  the document, marked "not supported by this compiler version"; environment
  names after `\begin{`; labels after `\ref{`/`\eqref{`/`\autoref{`; citation
  keys after `\cite{` (from `\bibitem` and from the project index when a
  bibliography is declared); package names after `\usepackage{`; the
  project's files after `\input{`/`\include{`/`\includegraphics{`; a suggested
  key after `\label{` (`fig:` inside a figure, `tab:` in a table, `eq:` in a
  math display, `sec:` plus the heading's words otherwise); document words
  longer than three characters. At most 12 entries. Exact and prefix matches
  come first; when nothing starts with what you typed, entries containing
  those letters in order are offered (`\sbs` finds `\subsection`).
- **Snippets**: a command with arguments is inserted with its braces and the
  caret in the first one (`\frac{|}{}`); **Tab** / **⇧Tab** move between the
  placeholders and out of the snippet, **Esc** leaves it. `\begin{itemize}`
  inserts the list with its first `\item`, `\begin{figure}` a figure skeleton
  (`\centering`, `\includegraphics`, `\caption`, `\label{fig:}`), `\begin{table}`
  a table skeleton; other environments an indented empty body line and the
  matching `\end`.
- **Signature help**: typing `{` after a command (or pressing **⌘⇧Space**
  inside a command's argument) shows the argument pattern with the current
  argument highlighted and a one-line description; it closes on `}`, Esc, or
  when the caret leaves the argument.
- **Typing helpers**: `{`, `[`, `$`, `\(` and `\[` are closed automatically and
  the closer is typed over, including a completion snippet's own placeholder
  closer like `\section{}`'s `}` (switch off with *Auto-close brackets &
  math* in Preferences); **Tab** indents (a multi-line selection: every line
  it touches; a caret or single-line selection: just inserts the indent
  unit) and **⇧Tab** always outdents the touched line(s), except while the
  completion list or a snippet's placeholders are active, when Tab/⇧Tab mean
  those instead; Return keeps the indentation, indents inside a new
  `\begin{env}` and adds `\end{env}`, and continues a list with a new `\item`;
  **⌘/** comments or uncomments the selected lines with `%`; the bracket or
  `$` pair around the caret is highlighted.
- **Hover**: rest the pointer on a token for about half a second to see what
  it is (command with documentation, label, citation key, file, package,
  environment) plus any diagnostic at that position with its recovery note
  and explanation.
- **⌘-click** (or ⌘⇧D, *Navigate › Go to Matching*): `\ref{key}` → its
  `\label`; `\label` → cycles through its references; `\begin` ↔ `\end`;
  `\input{file}` → opens the file. `\cite{key}` and user macros resolve
  through the project index, which needs the preview-controller helper route
  (see *Compiling*); without it the footer says the caret is not on a
  matchable command.
- **Gutter**: line numbers, a red or orange dot on lines with an error or
  warning, a grey dot for "not implemented" gaps. Inline underlines mark the
  exact span (red dotted = error, orange = warning); hovering shows the
  message. After you edit, underlines that overlap the edit are dropped, never
  drawn under the wrong text; when a compile fails with no output the previous
  underlines are kept and flagged "kept from revision N".
- **Auto-close**: typing `{` inserts `}` when the brace is code and followed by
  whitespace or a closer (Preferences › Auto-close brackets & math). Return
  auto-indents and closes `\begin{env}` with the matching `\end{env}`.
- **Find** (⌘F / ⌘⌥F): AppKit's native find bar in the source editor —
  incremental search as you type, with a Replace row (⌘⌥F) whose replacements
  are one undoable edit. Find Next/Previous have no key equivalent (Return /
  Shift-Return in the find bar's own field do the same); **⌘E** sets the
  current selection as the search string, **⌘J** re-centers it in view. This
  is separate from *Find in Project* below (one file vs. the whole project).
- **Find in Project** (⌘⇧F): case-sensitive literal search across the
  project's durable source; Return or ⌘G goes to the next match; *Plan
  Replacement* → *Apply* performs a reviewed replace-all. **Helper route
  only** — otherwise the window says "Search requires the durable helper
  (flashtex-preview-controller) to be attached and ready."
- **Rename Citation** (*Edit › Rename Citation…*, also in the toolbar): plans
  a rename of a citation key across the project, shows every affected place,
  and applies it as one group after you confirm. Helper route only.
- **Environments**: a caret on `\begin{X}` or `\end{X}` highlights both ends
  like a bracket pair (nesting of the same name and stray `\end`s are
  tolerated). **⌘⇧A** (*Navigate › Select Environment*) selects the innermost
  environment around the caret and, pressed again, the enclosing one.
  **⌘⇧W** (*Wrap Selection in Environment…*) asks for a name — common
  environments first, then the ones the document already uses — and wraps
  the selection: whole lines become an indented block on their own lines,
  anything else is wrapped inline; one undoable edit, caret at the body.
- **Go to definition** (⌘-click a `\foo`, or ⌃⌘J): selects the
  `\newcommand`/`\renewcommand`/`\def`/`\let`/`\DeclareMathOperator`/
  `\NewDocumentCommand` (for `\begin{X}`: `\newenvironment`/`\newtheorem`)
  definition in whichever open document holds it. Hovering a user command
  peeks its definition body under the standard documentation.
- **Go to symbol** (⌘⇧T): a fuzzy picker over every heading, environment and
  label of the open documents; Return goes there.
- **Rename symbol** (⌥⇧R, *Navigate › Rename Symbol…*): with the caret on a
  `\label{key}` or any `\ref`/`\eqref`/`\pageref`/`\autoref`/`\cref` use of it,
  or on a command defined by `\newcommand`/`\def`, *Plan Rename* lists the
  occurrences per open document (word-boundary aware — `\foo` never touches
  `\foobar` — comments and verbatim skipped; a name that already exists is
  refused) and *Apply* rewrites them: one undoable edit per document, or one
  guarded `apply_group` per file (ledger undo) when the durable helper is
  attached. Buffer-only: files that are not open are not touched.
- **Outline** (sidebar): parts, chapters, sections and subsections nest by
  depth; theorem-like environments and figures/tables show their caption or
  first line; the row of the caret's section is highlighted and clicking any
  row jumps to it.
- **Error lens** (Preferences › *Show diagnostics inline*): each line with a
  diagnostic shows its message dimmed at the end of the line (errors only by
  default; a second toggle adds warnings), from the same marks as the gutter.
- **Editor font size**: ⌘⌥= / ⌘⌥- / ⌘⌥0 (8–36 pt, default 13), or pinch over
  the editor.

## Compiling

FlashTeX compiles through a **producer** process that ships inside the app.

- **At launch** the bundled `flashtex-render` — the current engine: Latin
  Modern fonts, TeX metrics, the larger LaTeX subset and the v2 preview —
  attaches automatically and the document compiles (status bar route
  `worker`; the header names `flashtex-render`). The older Times-metrics
  `flashtex-compiler` is still bundled and can be attached from the File
  menu (⌘⇧K) for comparison; **File › Attach Render Pipeline (Latin Modern)**
  (⌘⇧R) switches back. `FLASHTEX_COMPILER=<path>` names an explicit engine;
  `FLASHTEX_AUTOATTACH=0` disables auto-attach altogether.
- **Auto-compile** (toolbar switch, on by default) sends every edit to the
  producer immediately; one request is in flight at a time and the newest
  buffer is coalesced behind it, so the preview never shows an older
  revision over a newer one. **⌘B** compiles on demand (*File › Compile*).
  Latency (send → result) is shown in the status bar, typically tens of
  milliseconds; the engine retypesets only the paragraphs you touched.
- A result is `ok`, `recovered` (pages rendered around problems) or `failed`
  (no pages; the previous preview stays on screen). *View › Show Preview
  Debug Status* reveals the status word, a "provisional rendering" note and
  the display-list identity line in the preview header.
- **Other producers** (developer options): *Attach Built Compiler* (⌘⇧K) finds
  `$FLASHTEX_COMPILER` or a `crates/compiler` build; *Attach Worker
  Executable…* (⌘K) runs any program speaking the JSON Lines protocol
  (see [compiler.md](compiler.md)); *Detach Worker* stops it. The
  preview-controller **helper route** (durable ledger + project index owned
  by the helper, route `controller`; powers Find in Project, Rename Citation,
  Durable History and `\cite`/macro navigation) has no menu item yet. It
  starts only when the app is launched with
  `FLASHTEX_AUTOATTACH=1 FLASHTEX_PREVIEW_CONTROLLER=<path to flashtex-preview-controller>`
  in the environment, e.g. from a terminal:

  ```sh
  A=/Applications/FlashTeX.app/Contents/MacOS
  FLASHTEX_AUTOATTACH=1 FLASHTEX_PREVIEW_CONTROLLER=$A/flashtex-preview-controller $A/FlashTeX
  ```

  (This is the developer hook documented in `apps/mac/README.md`; it was not
  exercised while writing this guide.)

## The preview

- **Two panes.** The default is the **v2 pane**: it paints the rendering-v2
  display list (exact glyphs and positions, the same data the exact PDF
  export uses) that `flashtex-render` sends with every result. If the older
  `flashtex-compiler` is attached there is no display list and the pane says
  "No v2 display list yet" — press ⌘⇧R, or flip the toolbar's *v2 pane*
  switch off to see the v1 pane (text items drawn with CoreText).
  `FLASHTEX_PREVIEW_V2=0` starts on the v1 pane.
- **Zoom**: ⌘= / ⌘- step by ×1.25 between 25 % and 400 % of fit-width; ⌘9
  fits the widest page to the pane; ⌘0 shows 1 PDF point per screen point.
  The header's −/+ buttons and percentage (double-click = fit width) and
  pinch-to-zoom do the same. Wide pages scroll horizontally.
- **Click-to-source**: click any word or rule in the preview to select its
  source in the editor. If you edited that region since the last compile the
  click is refused with "recompile to navigate" rather than selecting the
  wrong text.
- **Caret sync**: preview text whose source contains the editor caret is
  highlighted (accent fill + underline); *Navigate › Reveal Caret in Preview*
  (⌘⇧J) selects the whole span.
- **Preview follows the caret** (Settings › Preview, on by default): about a
  quarter-second after you stop moving the caret or typing, the preview
  scrolls (a short animation) so the caret's highlighted item is visible.
  It stays out of your way: nothing scrolls if the item is already on
  screen, and if you scroll the preview yourself (wheel, trackpad, or the
  scrollbar) or are mid-drag, it leaves the view alone for about 3 seconds —
  moving the caret to a different page or line cancels that pause early. A
  caret with no preview mapping (the preamble, a comment, an uncompiled
  region) does nothing. Turn it off in Settings to scroll only via ⌘⇧J or a
  preview click; works on both the v1 and v2 panes.
- **Dark preview** (toolbar switch): inverts page and text colours on screen
  only; exports are unaffected. Its initial state follows the editor
  appearance preference.
- **Stale frames**: while a new frame is being prepared the previous one stays
  visible with a STALE mark; a frame that fails validation is never painted
  partially — the pane shows the refusal and offers the v1 preview.
- **Images: `\includegraphics` png/jpeg/pdf** (v2 pane, `flashtex-render`
  attached, a project opened from disk). `\includegraphics[width=…,
  height=…, scale=…, angle=…, page=…]{figures/plot}` inside a `figure` or
  `table` float paints the file — PNG, JPEG, or one page of a PDF — at the
  size pdfTeX would give it. The file is read from the project directory
  (no symlinks, never outside the project) and its bytes must match what the
  producer sized: if you replace the file, the preview shows "stale image:
  figures/plot.png" in the header and leaves that box empty until the next
  compile. Clicking the image selects its `\includegraphics` in the editor.
  `Export PDF (v2)…` embeds the same images; the exact export does not yet.
- **TikZ: drawn in the preview and v2 export** (v2 pane, `flashtex-render`
  attached). A `tikzpicture` (`\usepackage{tikz}`; the tikz-min subset —
  `\draw`, `\fill`, `\clip`, `\node`, lines, circles, rectangles, arrows,
  `dashed`, colours) is painted as vector paths: fills by their rule,
  strokes with their width, caps, joins and dash pattern, clips applied.
  Clicking anywhere on the drawn ink selects the whole `tikzpicture` in the
  editor (the engine attributes each path to the picture, not to one
  command). `Export PDF (v2)…` writes the same paths; `File › Export PDF
  (exact, v2)…` does not accept them yet and reports the item it refused.

## Problems and quick fixes

The Problems panel (⌘⇧M; also opened by the sidebar rows and the status-bar
counts) lists the diagnostics of the current result, grouped by identical
message ("12× `\in` is not supported…") with a *Show: All / Errors / Warnings*
filter. Each row shows the message, a `↳` recovery line (what was rendered
instead), an `↳ explain:` line from the offline explanation catalogue, and
an "N places" menu.

- **Go to source**: click a row or press Return; ⌘⇧] / ⌘⇧[ step through
  diagnostics from the editor, ⌘⌥] / ⌘⌥[ step through the places of the
  selected group.
- **Fix…** appears when the catalogue has a concrete edit for that
  diagnostic (for example *Remove `\foo` and keep its argument text*). It
  opens a *Suggested fix* sheet with Before / After; **Apply** performs one
  undoable edit, **Cancel** changes nothing. The fix is rebased byte-exactly
  onto the current text and refused if that region changed since the compile.
- **Copy Diagnostics as Text** (⌘⌥C, or ⌘C with the list focused) copies
  `path:line: error: message` lines for the selected row or all rows —
  handy for bug reports.

What each diagnostic code means is listed in
[compiler.md › Troubleshooting](compiler.md#troubleshooting).

## PDF export

| Menu item | Shortcut | What it writes |
|---|---|---|
| **File › Export PDF (exact, v2)…** | — | The current v2 display list through `flashtex-pdf-exact`: embedded Latin Modern subsets, original glyph IDs, exact positions and typed rules. Needs a v2 frame (i.e. `flashtex-render` attached). Progress and Cancel in the status bar; the file is written atomically. **Use this one.** |
| File › Export PDF via Rust Writer… | ⌘⌥E | The v1 result through `flashtex-pdf --verify`: base-14/Latin Modern text items; characters outside those encodings become `?` with a warning |
| File › Export PDF… | ⌘⇧E | A CoreGraphics rendering of the v1 layout (Times/Latin Modern, no images, no links) |
| Export PDF (v2)… (v2 pane header) | — | A CoreGraphics rendering of the v2 display list: the preview's own draw routine, including `\includegraphics` images and TikZ paths |

All exports are black on white regardless of the dark-preview switch. None of
them is a pdfTeX PDF: only what the engine laid out is written (no hyperlinks,
no metadata; `\includegraphics` images and TikZ paths only through *Export PDF (v2)…* for now).

## Capture conversion (the only model-backed feature)

FlashTeX has no built-in AI assistant: the editor stays unopinionated, and
third parties can add such features through the helper-process contract in
[`docs/extensibility.md`](../extensibility.md). The one model-backed feature
is **capture conversion**: a drawing or photo sent from the iPad companion is
converted to LaTeX/TikZ by the bridge helper and comes back as a *reviewed
proposal* — **nothing is inserted unless you approve it**.

- **Preferences (⌘,) › Capture conversion**: choose the provider (*None* —
  captures are journaled but not converted — or *xAI*), paste the API key
  and *Save to Keychain* (login Keychain item
  `tech.jay3332.flashtex.ai.<provider>`, never a file; *Remove* deletes it),
  and pick the model. Environment overrides for one launch:
  `FLASHTEX_CONVERSION_PROVIDER`, `FLASHTEX_CONVERSION_MODEL`,
  `FLASHTEX_AI_API_KEY`.
- The key is handed only to the bridge helper's environment for the capture
  it converts; the app never calls a provider itself. Without a key, captures
  arrive, are journaled and shown as "not converted"; nothing leaves the Mac.
- Details and the provider seam: [`apps/mac/docs/capture-conversion.md`](../../apps/mac/docs/capture-conversion.md).

## The Nearby companion (iPad)

The **Captures** inspector (*View › Toggle Captures*, ⌘⇧I, or the toolbar's
Captures button) is where captures from [FlashTeXPad on an iPad](ipad.md)
arrive and get inserted. Opening it starts advertising this Mac (macOS asks
for Local Network permission once) and attaches the capture bridge; the two
status pills at the top say so. The Mac also advertises at launch once a
companion is paired, so the iPad reconnects without any click.

1. **Pairing code…** opens the Nearby Companion window (*Edit › Nearby
   Companion…*, ⌘⇧N) with a fresh 6-digit code valid for 120 s, shown as
   digits and as a QR code; *Copy code* copies the digits. Scan, type, or pick
   the Mac from the iPad's *Find nearby Macs* list and enter the code. The
   window also lists **paired companions** (a "connected" badge, a
   per-companion permission pop-up, **Forget**).
2. The insertion point is the caret: when the iPad asks where to insert, the
   Mac pins the caret for it. *Edit › Pin Insertion Point* (⌘⌥P) is an
   explicit override; the inspector's destination line says "(caret)" or
   "(pinned)".
3. Every capture the iPad sends appears in the inspector immediately with its
   image, instruction and state — *received* → *converting* → *proposal
   ready* → *inserted* (or *rejected* / *failed*, with the reason). With a
   conversion provider configured (Preferences → Capture conversion) the
   conversion starts on receipt; without one the row offers **Convert**.
4. When the proposal is ready the LaTeX/TikZ is shown syntax-coloured.
   **Insert at caret** approves it: the bridge prepares the edit at the bound
   destination, verifies it, and inserts exactly one undoable edit (⌘Z).
   **Edit** changes the text first (the bridge refuses edited text —
   transfer-v1 inserts only the journaled proposal — so edit after inserting
   or reject and resend), **Review…** opens the full sheet with the shadow
   compile and ambiguities, **Reject** discards it. The iPad's row shows
   "Inserted on Mac ✓".
5. **Clear N** removes finished rows. Without a bridge, captures wait in the
   inspector as *received* until **Attach bridge**.

Switches (environment, `=0` turns each off): `FLASHTEX_CAPTURE_AUTO_CONVERT`,
`FLASHTEX_CAPTURE_CARET_DESTINATION`, `FLASHTEX_CAPTURES_AUTO_ATTACH`,
`FLASHTEX_NEARBY_AUTO_ADVERTISE`.

Pairings live in `~/Library/Application Support/FlashTeX/pairs.json`
(owner-only permissions, not the Keychain). The transport is TLS 1.2 with a
pre-shared key derived from the code; the code is short, so pair on a network
you trust.

## Preferences (⌘,)

| Setting | Default |
|---|---|
| Font family (installed monospaced fonts) and size 8–36 pt | System monospaced, 13 pt |
| Wrap long lines | on |
| Tab width 2–8, indent with spaces or tab | 4, spaces |
| Editor appearance: System / Light / Dark (also seeds the dark-preview switch) | System |
| Auto-close brackets & math | on |
| Show completion list (off disables ⌃Space / Esc completion) | on |
| Capture conversion: provider (None / xAI), key in Keychain, model | None |
| Restore Defaults | |

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| ⌘, | Settings / Preferences |
| ⌘O | Open LaTeX file… (becomes the entry document) |
| ⌘⌥N | New Project… (folder, name, template; opens `main.tex` with its include tree) |
| ⌘N | New File… (rooted `.tex` name; optional `\input` at the caret; also the sidebar's + and the project row's context menu) |
| ⌘S / ⌘⇧S | Save / Save As… |
| ⌘⇧P | Command palette |
| ⌘B | Compile now |
| ⌘⇧R | Attach render pipeline (Latin Modern) — the current engine |
| ⌘⇧K | Attach built compiler |
| ⌘K | Attach worker executable… |
| ⌘⇧O / ⌘R | Open compile-result fixture… / Reload fixture (developer) |
| File › Export PDF (exact, v2)… | Exact PDF from the v2 display list |
| ⌘⇧E | Export PDF… (CoreGraphics) |
| ⌘⌥E | Export PDF via Rust writer… |
| ⌘Z | Undo (including an applied fix or capture insertion) |
| Esc / ⌃Space | Open the completion list |
| ↑ ↓ / Tab ⇧Tab / Return / Esc | While the list is open: choose / insert / close |
| Tab / ⇧Tab / Esc | After inserting a snippet: next / previous placeholder / leave |
| Tab / ⇧Tab | Otherwise: indent / outdent the touched line(s) |
| ⌘⇧Space | Signature help for the command whose argument the caret is in |
| ⌘F / ⌘⌥F | Find… / Find and Replace… (native find bar in the editor) |
| ⌘E / ⌘J | Use selection for Find / jump to (center) the current selection |
| ⌘/ | Comment or uncomment the selected lines |
| ⌘-click / ⌘⇧D | Go to matching `\label`↔`\ref`, `\begin`↔`\end`, open `\input` file |
| ⌘-click / ⌃⌘J | Go to definition of a `\newcommand`/`\def`/`\DeclareMathOperator`/`\newenvironment` symbol |
| ⌘⇧T | Go to symbol… (fuzzy picker over headings, environments and labels of the open documents) |
| ⌘⇧A | Select environment (innermost `\begin`…`\end` around the caret; again widens) |
| ⌘⇧W | Wrap selection in environment… |
| ⌥⇧R | Rename symbol (`\label` key or user command, across the open documents) |
| ⌘⇧] / ⌘⇧[ | Next / previous diagnostic |
| ⌘⌥] / ⌘⌥[ | Next / previous occurrence within the selected Problems group |
| ⌘⇧M | Toggle Problems panel |
| ⌘⌥C | Copy diagnostics as text |
| ⌘⇧J | Reveal caret in preview |
| Click preview text | Select its source |
| ⌘= / ⌘- | Zoom preview in / out (×1.25, 25–400 %) |
| ⌘0 / ⌘9 | Preview actual size / fit width |
| ⌘⌥= / ⌘⌥- / ⌘⌥0 | Editor font size larger / smaller / reset (13 pt) |
| ⌘⇧F / ⌘G | Find in Project… / next match |
| Edit › Rename Citation… | Reviewed citation-key rename across the project |
| Edit › Durable History… | Undo/redo on the durable edit ledger |
| ⌘⇧N | Nearby Companion… (pairing, captures) |
| ⌘⌥P | Pin insertion point (capture destination) |
| Edit › Open Capture Proposal… | Open a capture proposal file (review sheet; Return approves) |
| ⌘⇧I | Toggle the Captures inspector (iPad captures, proposals, Insert at caret) |
| ⌘⇧U | Submit sample capture… (PNG/JPEG through the bridge) |
| ⌘⇧G | Convert the latest received capture |
| Edit › Restore Discarded Buffer | Bring back text discarded when opening another file |
| File › Resolve On-Disk Conflict… / Reload From Disk… / Restore Unsaved Snapshot… | File-state recovery (see *Projects*) |
| View › Show Preview Debug Status | Status word and display-list identity in the preview header |
| Help › FlashTeX Accessibility Help | Focus order, what VoiceOver reads, every command above |

Everything in this table is also reachable from the command palette (⌘⇧P) and
is read by VoiceOver; *Help › FlashTeX Accessibility Help* documents the focus
order of each pane.

## Not yet supported

- Automatic attachment of the Latin Modern render pipeline at launch (press
  ⌘⇧R); a menu item for the preview-controller helper route, so Find in
  Project, Rename Citation, Durable History and `\cite` navigation need the
  environment-variable launch described under *Compiling*.
- Completion does not pop up while typing (open it with ⌃Space / Esc); signature help does.
- `\includegraphics` outside a `figure`/`table` float (and its `trim`/`clip`/
  `viewport` keys), tables, bibliographies and other constructs listed under
  [Supported LaTeX](compiler.md#supported-latex) render as diagnostics, not
  content. Images in floats: see *The preview*.
- Notarization: the app is ad-hoc signed, hence the right-click › Open step on
  first launch of a browser download.
- Capture-conversion providers other than xAI (the provider seam is documented
  in `docs/extensibility.md`; an on-device model is a later goal).
