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
5. **Move** a file by dragging its row onto another row (it lands in that
   row's folder — the tree is flat, so a file row stands for its folder) or
   onto the tree's empty space (the project root); **Move to…** in the
   row's context menu, or *File › Move To…* for the active document, asks
   for the folder instead. Every `\input`, `\include`, `\includegraphics`,
   `\bibliography`, `\addbibresource` and `\lstinputlisting` that resolved
   to the file (with or without its extension, `./`-prefixed or not) is
   rewritten to the new path: one undoable edit per open document (⌘Z
   there; the file itself stays moved), and closed documents of the
   include tree are rewritten on disk — the footer note lists both. A drop
   onto the file's own folder, onto a folder under it, or over an existing
   name is refused; so is moving the entry document.

### Opening and working in a project

- **Open** a file with *File › Open LaTeX File…* (⌘O).
  It becomes the **entry document** (sent to the engine as `main.tex`,
  whatever its real name) and its folder becomes the project root. Open a
  **folder** instead and its [`flashtex.toml`](project-manifest.md) names
  the entry (`[project] entry`); without one, the folder's only `.tex` file
  is it — two or none is refused, naming them. If the current buffer is
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
- **Packages and classes.** The `.sty`/`.cls`/`.def`/`.clo` files next to
  the entry, and every file under the manifest's `texinputs` directories,
  show in the Project tree as greyed rows with the class/style icon and go
  out with every compile on the direct route, so the engine can read a
  project-local `\usepackage{mystyle}`. Click one to open it. *File ›
  Create flashtex.toml…* writes the commented manifest template next to
  the entry and opens it (coloured as plain text with comments). See
  [the project manifest](project-manifest.md).
- **Project Fonts.** *File › Project Fonts…* (also in the command palette)
  sets the project's fonts globally — the manifest's `[fonts]` table — as
  four rows, Text, Math, Sans and Mono. Each row is a searchable picker over
  the families installed on the machine as the engine's own index finds
  them (the same names `\setmainfont{` completes with; the Math row lists
  only families with an OpenType MATH table), with a sample line set in the
  chosen family and *Class default* first. Apply writes the table into
  `flashtex.toml` — created from the template when there is none; *Class
  default* everywhere without a manifest writes nothing — and the preview
  recompiles with the new fonts. A `\setmainfont`, `\setsansfont` or
  `\setmonofont` in the document still wins over the table, and a
  `\fontspec{…}` group wins locally. The `\setmainfont{` completion rows
  show the same sample in each family.
- **Packages.** When a compile reports a `\usepackage` the engine does not
  model and the project does not supply, FlashTeX resolves it through the
  manifest's local libraries and the per-user package cache first (no
  network) and then, under the manifest's `[packages] fetch` policy, offers
  to fetch its LaTeX source files from CTAN: **one sheet per project**
  listing each package, its version, its files and the URL, with *Fetch*,
  *Not Now* and *Never for This Project* (which writes `fetch = "never"`);
  tick *Remember* and Fetch writes `fetch = "always"` so the sheet does not
  come back. Nothing is fetched without the sheet or that remembered answer.
  Fetched and library files are compiled from the cache — never from the
  project — and appear under a dimmed *Packages* group in the Project tree
  with their source in the tooltip; they cannot be opened as project files.
  *File › Fetch Missing Packages…* asks again about anything declined. A
  package that ships only `.dtx`/`.ins` sources is unpacked with FlashTeX's
  docstrip after the fetch (the consent names the sources); one whose
  sources cannot be unpacked is reported as needing docstrip. See
  [the project manifest](project-manifest.md#packages).
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
- **Completion opens on its own while you type** — you do not need ⌃Space:
  typing a control word (including a bare `\`, which lists the vocabulary) or
  an argument key for a command that has completions (`\begin{`, `\end{`,
  `\ref{`, `\cite{`, `\label{`, `\usepackage{`, `\input{`) opens the list a
  short pause (50 ms) after the keystroke, so one burst of fast typing costs
  one scan rather than one per character. A plain prose word does not open it
  on its own — word suggestions from the document are still available, just
  via explicit ⌃Space, since offering them on every letter would be noise.
  Esc dismisses the list for that token; typing more of the same token stays
  quiet. Turn the feature off with *Show completion list* in Preferences.
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
  those instead; Return keeps the indentation and, after `\begin{env}`,
  follows the environment's rule (Settings › Environments): one indent
  level deeper unless the rule says flat (`document` by default), the
  body's line text (`\item ` in `itemize`/`enumerate`, `\item[] ` in
  `description`, `\bibitem{} ` in `thebibliography`), then `\end{env}`;
  Return on an entry line repeats that text (a bare `\item` line just
  breaks). The `\begin{` completion skeletons, Wrap in Environment and
  Re-indent follow the same rules;
  **⌘/** comments or uncomments the selected lines with `%`; the bracket or
  `$` pair around the caret is highlighted.
- **Hover**: rest the pointer on a token for about half a second to see what
  it is (command with documentation, label, citation key, file, package,
  environment) plus any diagnostic at that position with its recovery note
  and explanation. Resting the pointer on an inline formula (`$…$`, `\(…\)`)
  instead shows a small preview of the formula as already rendered, cropped
  out of the current preview page — nothing is re-rendered to show it, so it
  can go blank right after an edit until the preview catches up, and it
  doesn't cover display math (`$$…$$`, `\[…\]`) or math environments, or a
  formula split across more than two lines.
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
- **Wrap in a command**: **⌘⇧B** (*Edit › Bold*), **⌘I** (*Emphasize*) and
  **⌘U** (*Underline*) wrap the selection in `\textbf{…}`, `\emph{…}` and
  `\underline{…}` — `\mathbf` and `\mathit` when the caret is in math mode
  — as one undoable edit with the caret after the closing brace; with nothing
  selected the caret lands between the braces and typing `}` steps over the
  one already there. **⌘⌥W** (*Wrap Selection in Command…*) asks for any
  command name (common ones first, then the document's own macros). ⌘B stays
  Compile.
- **`\left` … `\right`**: in math mode, typing `\left(`, `\left[`,
  `\left\{`, `\left|` or `\left.` inserts the matching `\right…` after
  the caret; typing it by hand steps over the inserted one.
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

### Packages and classes

The project's own `.sty`/`.cls` files (next to the entry document, under a
`texinputs` directory of `flashtex.toml`, or resolved into the package
cache — see [project-manifest.md](project-manifest.md)) are part of the
editor's picture of the project, not just of the compile:

- **Package editing mode**: a `.sty`, `.cls`, `.def` or `.clo` buffer is
  lexed with `@` as a letter, so `\@ifnextchar` and `\@tempdima` are one
  command token for colouring, hover, ⌘-click and completion — as they are
  while LaTeX reads the file. In any other buffer the same holds between
  `\makeatletter` and `\makeatother`; outside it `\@` stays the control
  symbol it is in a document. The ltclass vocabulary (`\ProvidesPackage`,
  `\NeedsTeXFormat`, `\DeclareOption`, `\ProcessOptions`, `\RequirePackage`,
  `\LoadClass`, `\PassOptionsToPackage`, `\CurrentOption`,
  `\@ifpackageloaded`, `\AtEndOfPackage`, `\PackageWarning`/`\PackageError`,
  `\ClassWarning`, `\newif`, `\def`/`\let`/`\edef`, `\csname`…`\endcsname`,
  `\expandafter`, `\@namedef`/`\@nameuse`, the scratch registers, …) has
  one-line hover documentation everywhere and leads the completion list
  inside a package buffer (or inside `\makeatletter`), each entry with its
  argument snippet; a document never sees those rows above its own
  vocabulary.
- **Completing package and class names**: `\RequirePackage{` and
  `\PassOptionsToPackage{…}{` complete like `\usepackage{`, `\LoadClass{`
  like `\documentclass{`. Both offer the project's own files first
  ("package in this project · mystyle.sty"), then the common CTAN names or
  the standard classes.
- **Macros from packages are first-class**: commands and environments a
  project package declares (`\newcommand`, `\def`, `\DeclareRobustCommand`,
  `\NewDocumentCommand`, `\newif` switches, `\newenvironment`,
  `\newtheorem`) complete in every document with the detail "declared in
  mystyle.sty" and their argument shape as the snippet (`\emphx{|}` from
  `\newcommand{\emphx}[1]`; the buffer's own macros get the same). Hovering
  such a macro peeks its definition line and file. **Go to definition**
  (⌘-click, ⌃⌘J) on it opens the package at the definition: a `.sty` next
  to the entry opens as an ordinary member; a file that lives outside the
  project root (a `texinputs` directory, a resolved package) opens
  **read-only** with a strip above the editor saying where it really comes
  from — it is compiled from there and never saved, renamed or moved.
- **Problems inside a package** are listed with the package path and line;
  *Go to source* opens the package file (read-only when it is not a project
  file) at the span. The `\usepackage` line that loaded it gets a secondary
  mark — "mystyle.sty: 2 problems — loaded here" — in the gutter, the
  underline, the hover and the error lens, so the line to look under is
  visible from the document.
- **Missing-package quick fixes**: the row for `packages X are recognised
  but not implemented` (or `no project file found: looked for X.sty`)
  offers **Create X.sty** — writes the package template (`\NeedsTeXFormat`,
  `\ProvidesPackage{X}[date v1.0 …]`, `\DeclareOption*`,
  `\ProcessOptions\relax`) next to the entry document and opens it; the next
  compile loads it — and **Fetch X…**, which opens the package consent
  sheet for that name (nothing is fetched until you agree there; see
  *Fetch Missing Packages…*). Several names fold into one menu. The
  compiler's own fix (remove the `\usepackage`) stays alongside.
- **New File…** (⌘N) accepts a `.sty` or `.cls` name and writes the same
  template (`\ProvidesClass` + `\LoadClass{article}` for a class); a new
  package is referenced from the caret with `\usepackage{name}` when the
  insert toggle is on, a class with nothing (the document's
  `\documentclass` names it).

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
  highlighted (accent fill + underline);
  *Navigate › Reveal Caret in Preview* (⌘⇧J) selects the whole span.
- **The preview follows what you are editing** (Preferences › *Preview follows
  the caret*, on by default): shortly after you stop typing — and after the
  recompiled preview lands — the pane scrolls to the text the caret is in, but
  only when that text is off screen or within a line of an edge; anything
  already in view is left exactly where it is, so the preview never twitches
  while you type. Scrolling the preview yourself (wheel, trackpad, scroller)
  stops it following; it starts again on your next edit, or the moment the
  caret lands on a different line or page (moving it around within the same
  line does not wake it back up — that's still just reading). A caret that
  maps to nothing — a comment, the preamble, text the compiler has not laid
  out yet — moves nothing. Short hops are animated; long jumps, and "Reduce
  motion", are instant. ⌘⇧J always scrolls to the caret at once, whether or
  not the preference above is even on.
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
  *File › Export PDF…* embeds the same images.
- **TikZ: drawn in the preview and v2 export** (v2 pane, `flashtex-render`
  attached). A `tikzpicture` (`\usepackage{tikz}`; the tikz-min subset —
  `\draw`, `\fill`, `\clip`, `\node`, lines, circles, rectangles, arrows,
  `dashed`, colours) is painted as vector paths: fills by their rule,
  strokes with their width, caps, joins and dash pattern, clips applied.
  Clicking anywhere on the drawn ink selects the whole `tikzpicture` in the
  editor (the engine attributes each path to the picture, not to one
  command). *File › Export PDF…* writes the same paths.

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
| **File › Export PDF…** | ⌘⇧E | The current display list through `flashtex-pdf-exact`: embedded Latin Modern subsets, original glyph IDs, exact positions, typed rules, `\includegraphics` images and TikZ paths. Needs a display list (i.e. `flashtex-render` attached). Progress and Cancel in the status bar; the file is written atomically |
| File › Print… | ⌘P | Not an export, but the same bytes: the PDF above, through the system print panel |

There is one export route. Three earlier ones — a CoreGraphics rendering of the
v1 layout (⌘⇧E), *Export PDF via Rust Writer…* (⌘⌥E) and the v2 pane's own
*Export PDF (v2)…* — were removed in favour of it: each wrote a lower-fidelity
version of the same document. `flashtex build` on the command line writes the
same bytes.

The export is black on white regardless of the dark-preview switch. It is not a
pdfTeX PDF: only what the engine laid out is written (no hyperlinks, no
metadata). Anything the writer cannot express exactly is refused by name rather
than approximated.

A long document is previewed through a **page window** — the engine cannot send
its whole display list in one reply — but it still exports in full: Export and
Print re-render the complete document through the render pipeline first, which
takes a few seconds and is reported in the status bar. That needs the render
pipeline attached (⌘⇧R); without it, Export says so and points you at
`flashtex build`.

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
2. The insertion point is the caret — the caret as it is when you click
   **Insert at caret**, not where it was when the iPad connected. Keep typing,
   move around, relaunch the app: the capture still goes where the caret is.
   *Edit › Pin Insertion Point* (⌘⌥P) is an explicit override that stays put;
   the inspector's destination line says "(caret)" or "(pinned)". If an edit
   removes the pinned spot, the row says so and offers **Insert at caret**.
3. Every capture the iPad sends appears in the inspector immediately with its
   image, instruction and state — *received* → *converting* → *proposal
   ready* → *inserted* (or *rejected* / *failed*, with the reason). With a
   conversion provider configured (Preferences → Capture conversion) the
   conversion starts on receipt; without one the row offers **Convert**.
4. When the proposal is ready the row shows exactly what will be inserted,
   syntax-coloured, with a small "as display math / inline math / as is"
   indicator: a formula on its own line is wrapped in `\[ … \]`, one inside
   a sentence in `$ … $`, nothing is wrapped when the caret is already inside
   math or the proposal brings its own delimiters, and an environment (a TikZ
   picture, say) goes in as is on its own lines. Move the caret and the
   indicator follows. **Insert at caret** approves it: the bridge prepares the
   edit at the caret, verifies it, and inserts exactly one undoable edit (⌘Z).
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
| Show completion list (off disables both automatic-while-typing and explicit ⌃Space / Esc completion) | on |
| Vim keybindings (also View › Toggle Vim Keybindings, ⌃⌘V) | off |
| Preview follows the caret while you edit | on |
| Environments tab: indent inside environments the table does not name; per environment, whether the body is indented and what each new line starts with (add your own rows; *Conventional Rules* restores the shipped set) | everything indented except `document`; `\item ` in lists |
| Capture conversion: provider (None / xAI), key in Keychain, model | xAI (no-op until a key is added) |
| Check for updates automatically (once a day after launch; only speaks up when a newer release exists) | off |
| Restore Defaults | |

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| ⌘, | Settings / Preferences |
| FlashTeX › Check for Updates… | Installed vs newest GitHub release with its notes; *Download* opens the release page (nothing is installed by the app) |
| ⌘O | Open LaTeX file… (becomes the entry document) |
| ⌘⌥N | New Project… (folder, name, template; opens `main.tex` with its include tree) |
| ⌘N | New File… (rooted `.tex` name; optional `\input` at the caret; also the sidebar's + and the project row's context menu) |
| File › Move To… | Move file: the active document into another project folder (also *Move to…* in a row's context menu, or drag a row onto another row / the tree's empty space); file references are rewritten |
| ⌘S / ⌘⇧S | Save / Save As… |
| ⌘⌥R | Show in Finder (the active document; sidebar rows have *Reveal in Finder*, the tree's empty space *Reveal Project in Finder*) |
| ⌘⇧P | Command palette |
| ⌘B | Compile now |
| ⌘⇧R | Attach render pipeline (Latin Modern) — the current engine |
| ⌘⇧K | Attach built compiler |
| ⌘K | Attach worker executable… |
| ⌘⇧O / ⌘R | Open compile-result fixture… / Reload fixture (developer) |
| ⌘⇧E | Export PDF… (the display list through `flashtex-pdf-exact`) |
| ⌘P | Print… (the same bytes) |
| ⌘Z | Undo (including an applied fix or capture insertion) |
| Esc / ⌃Space | Open the completion list explicitly (it also opens on its own — see below) |
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
| ⌘⇧B / ⌘I / ⌘U | Bold / emphasize / underline: wrap the selection in `\textbf{}` / `\emph{}` / `\underline{}` (`\mathbf` / `\mathit` in math mode) |
| ⌘⌥W | Wrap selection in command… |
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

## Vim mode

Settings › Typing › *Vim keybindings* (or View › Toggle Vim Keybindings, ⌃⌘V)
turns the source editor modal. The status bar shows `-- NORMAL --`,
`-- INSERT --`, `-- VISUAL --` / `-- VISUAL LINE --`, and the `:` or `/` line
as you type it; the caret is a block outside insert mode.

**Supported**

- Modes: normal, insert (`i a I A o O s S c C`), visual (`v`), visual line (`V`),
  `r{char}`. Esc or ⌃[ returns to normal. Selecting with the mouse enters
  visual mode.
- Insert mode is the ordinary editor: input methods, dead keys, completion,
  snippets and signature help work as usual; only Esc is taken (and not while
  a composition is in progress). ⌘-shortcuts always work.
- Counts; motions `h j k l w b e W B E 0 ^ $ gg G { } ( ) f F t T ; , % H M L`,
  ⌃D ⌃U ⌃F ⌃B (`%` also jumps between `\begin` and `\end`).
- Visual-row motions `gj gk g0 g^ g$`: one *screen* row rather than one logical
  line. Line wrapping is on by default, so a wrapped paragraph is many rows but
  one line, and plain `j` jumps over all of it; `gj` moves the way the text
  looks. They keep their own remembered column, so mixing `j` and `gj` does not
  make either drift, and they take counts and operators (`3gj`, `dgj`). With
  wrapping off — or on a line that does not wrap — `gj` is exactly `j`.
- Operators `d c y > <` with motions, `dd cc yy >> <<`, and text objects
  `iw aw i( a( i[ a[ i{ a{ i" a" i$ a$` (inline math) and `ie ae` (LaTeX environment).
- `x X D C Y p P J u ⌃R . ~`, marks `m a` / `'a` / `` `a ``, registers `"a`–`"z`
  and `"+` / `"*` (system clipboard).
- `/` `?` search with incremental preview, `n N *` (smart-case; the term is
  shared with the find bar, so ⌘G continues it).
- `:w :q :q! :wq :x :e file :%s/a/b/g :s/a/b/ :noh :set nu :set nonu`.
- `u` / ⌃R are the editor's normal undo: one step per insert session, one per operator.

**Not yet**

- `.` does not repeat visual-mode changes; `o`/`cw` followed by typing are two undo steps.
- No `gu gU gq =`, no `ip ap it at iS aS` objects, no regular expressions in
  `/` and `:s` (literal text), no `:g`, macros (`q`), jump list (⌃O / ⌃I),
  block mode (⌃V), replace mode (`R`), or key mappings / `.vimrc`.
- Marks do not move with edits above them.

## Not yet supported

- Automatic attachment of the Latin Modern render pipeline at launch (press
  ⌘⇧R); a menu item for the preview-controller helper route, so Find in
  Project, Rename Citation, Durable History and `\cite` navigation need the
  environment-variable launch described under *Compiling*.
- `\includegraphics` outside a `figure`/`table` float (and its `trim`/`clip`/
  `viewport` keys), tables, bibliographies and other constructs listed under
  [Supported LaTeX](compiler.md#supported-latex) render as diagnostics, not
  content. Images in floats: see *The preview*.
- Notarization: the app is ad-hoc signed, hence the right-click › Open step on
  first launch of a browser download.
- Capture-conversion providers other than xAI (the provider seam is documented
  in `docs/extensibility.md`; an on-device model is a later goal).
