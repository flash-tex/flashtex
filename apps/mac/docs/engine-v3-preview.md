# The engine (engine v3)

Lane P3-APP-V3 ([DESIGN.md](../../../docs/design/engine-v2/DESIGN.md) §3, §6.1–6.2, §12 P3).
The preview pane shows pages from the pdfLaTeX-compatible engine
(`crates/flashtex-engine`, GPL). Since the old-engine retirement plan's stage
S6 (#1236) it is the app's only engine: there is no engine choice and no
fallback. The app never links the engine. It starts `flashtex-host` as a
separate process and speaks the MIT
[display-list-v3 protocol](../../../docs/protocol/display-list-v3.md) over a Unix
socket. The licence boundary is that process boundary
(`scripts/check-license-boundary.sh`).

## Quick start (development)

```
scripts/run-mac-dev.sh                 # build flashtex-host, flashtex-bridge and the app (release), and launch
scripts/run-mac-dev.sh ~/thesis/main.tex   # … and open that file (or a folder) at launch
scripts/run-mac-dev.sh --no-build      # launch what is built
```

It exports `FLASHTEX_ENGINE_V3=1`, `FLASHTEX_HOST`, `FLASHTEX_BRIDGE`,
`FLASHTEX_REPO` and, for a file argument, `FLASHTEX_OPEN`. It needs Rust,
Xcode's Swift and a TeX Live. On the first launch, the pane shows "Preparing
the pdfLaTeX format…" for about 5 s.

**Your own project.** Use File > Open (⌘O) on a `.tex` file or a folder, or
pass it to the script.

- The preview follows whatever the window opens. A single `.tex` file is its
  own main file. If the file you open declares no `\documentclass` (a
  chapter), the preview compiles whichever open document does.
- Opening a file while the start-up buffer has unsaved text asks first.
  "Don't Save" keeps that text under Edit > Restore Discarded Buffer.
- The first error of a compile (`file:line: message`) shows in the pane and in
  the preview's status chip.
- Nothing else is needed. Your files are never written; the engine compiles a
  copy in `~/Library/Caches/FlashTeX/engine-v3/projects/`.

## One engine (retirement stage S6)

`EngineV3.enabledAtLaunch` (`Sources/FlashTeXMac/EngineV3Host.swift`)
decides whether a window's engine session runs (`ShellModel.engineV3Enabled`):

- It runs, unless `FLASHTEX_ENGINE_V3=0`, which keeps the session off (an
  editor only; nothing compiles).
- Under XCTest it is off unless `FLASHTEX_ENGINE_V3=1` or a test turns it on,
  so a model made for a test starts no host process.

`Sources/FlashTeXMac/EngineChoice.swift` keeps what is left of the former
per-document engine choice:

- **No TeX to compile with** (owner ruling RQ20). When no TeX Live is found
  (the same search as the engine's `resolver.rs`, or the host's own report at
  start-up) and no TeX bundle is configured, or the user answered "Not Now"
  to downloading the bundle, the window shows a plain, non-blocking message
  over the preview (`TeXMissingBanner`): "Compiling needs the TeX bundle or a
  TeX Live installation." Its **Download** button, shown when a bundle is
  configured, allows the download and starts the engine again. No other
  engine typesets the document.
- **Documents typeset by the previous engine.** The per-document records
  (UserDefaults `FlashTeX.EngineV3.documents`, keyed by project root plus
  entry, following a moved or renamed project folder, at most 500 entries,
  never in the project) say `new` or, for a record written before S6,
  `previous`. A document recorded as `previous` opens on the new engine with
  a one-time, dismissable notice (`PreviousEngineNoticeBanner`, ruling 4,
  plan §5.3) and a VoiceOver announcement; its record then says `new`. Any
  other document gets a `new` record when it first opens.
- **Retired defaults.** The first launch of this build removes
  `FlashTeX.EngineV3.enabled`, `FlashTeX.EngineV3.legacyMigrated.v1`,
  `FlashTeX.EngineV3.defaultEngine` and `FlashTeX.Preview.v1.debugStatus`
  (`EngineChoiceStore.removeRetiredDefaults`). The app no longer reads
  `FLASHTEX_PREVIEW_V2`, `FLASHTEX_LAYOUT_CAPABILITIES`, `FLASHTEX_PDF_EXACT`,
  `FLASHTEX_COMPILER`, `FLASHTEX_RENDER` or `FLASHTEX_PREVIEW_CONTROLLER`.

**Manifest settings the engine reads.** `[fonts]` applies in Unicode mode
(below). `[packages] pin` versions and `[packages] path` libraries are
resolved before a compile and linked into the project copy ahead of TeX
Live's copies (`ProjectPackages.swift`; docs/user/project-manifest.md). An
outside edit of `flashtex.toml` (or the Fonts sheet) is read again at once.

**`[project] texinputs` work in the new engine.** Each file the manifest lists
is linked at the top of the engine's project copy, so `\usepackage{mystyle}`
finds `styles/mystyle.sty` as `TEXINPUTS=.:styles:` would; a project file of the
same name wins. Files from outside the root are inputs of the stored pages:
changing one outside the app drops them at the next open.

## Unicode mode (XeLaTeX-compatible)

A document runs in **Unicode mode** (`EngineV3Mode.swift`, modes
PROPOSAL.md §4.1) when `FLASHTEX_MODE=unicode`, when `flashtex.toml` says
`[project] mode = "unicode"` (read by the project-files helper), or when a
`% !TEX program = xelatex` line (or `TS-program`) is among the main file's
leading comments (after a byte-order mark; `!TeX`, `! TeX`, either case;
`xetex` gives plain XeTeX's format), read from the disk when the main file
is not open. A document whose mode nothing of these sets runs in Unicode mode
when it needs it (below); otherwise it runs in Classic mode. What cannot be followed is
said in the pane's details and the log: `lualatex` (LuaTeX isn't supported;
Classic), a manifest value this version does not know (Classic; a `% !TEX`
line does not overrule it), `flashtex` (not available yet; Classic). Unicode mode starts
`flashtex-host-unicode` (crates/flashtex-xetex) in place of `flashtex-host`
and compiles with the `xelatex` format. The window shows its pages exactly
as Classic's. That host compiles cold for now: each compile is a full
xelatex run with its passes, with no checkpoints. An edit that changes the
mode relaunches the host: BYE first, SIGTERM a second later should it still
run, and a Unicode host's engine ends with the host however the host ends. `[fonts]` applies in this mode (PROPOSAL.md
§4.5). The host is found as below, from `FLASHTEX_HOST_UNICODE`, the
`FlashTeX.EngineV3.hostPath.unicode` default, `Contents/Helpers/flashtex-host-unicode`
or a repository build (`crates/flashtex-xetex/target/release` too).
`make-app.sh` bundles it as `Contents/Helpers/flashtex-host-unicode` when it
is built or given (`--engine-host-unicode`).

**Detected Unicode mode** (`UnicodeFonts.swift`, `ShellModel.unicodeRouting`).
A document whose mode nothing sets (no `[project] mode`, no `% !TEX program`
line, no `FLASHTEX_MODE`) runs in Unicode mode, when its engine is installed,
if its preamble uses fontspec, unicode-math, `\setmainfont` and the like
(a lexical scan of the entry's preamble and the project files it loads
there), if a Classic run stopped on one of those packages' errors, or if
`flashtex.toml` sets `[fonts]`. The routing is decided in memory and never
written to `flashtex.toml`; the resolution's source is
`EngineV3Mode.detectedSource`, and the mode item says "Detected: …". An edit
that changes the outcome is scanned again once typing pauses and relaunches
the host in the other mode. A mode the project or the document sets is
kept, Classic included (Classic does not apply `[fonts]`).

**The mode item** (status bar, and View › Mode for This Document;
`ProjectMode.swift`) shows Classic or Unicode and where that comes from. Choosing
the other mode writes `[project] mode` to `flashtex.toml` through the
project-files helper's `set_mode` (creating the file next to the entry when there
is none). Nothing is written until the user chooses. A document set to Classic
that needs Unicode mode (fontspec, unicode-math, `\setmainfont`, … in its
preamble, or a run stopped on fontspec's error) keeps Classic; the mode item
then shows a warning and its menu says what the document uses.

## Where the host comes from

The app looks for `flashtex-host` in these places, in order:

1. `FLASHTEX_HOST`
2. the `FlashTeX.EngineV3.hostPath` default
3. `Contents/Helpers/flashtex-host` in the app bundle, or beside the app's executable
4. `target/release/flashtex-host` (then `target/debug`) in the repository

**Packaged app.** `apps/mac/scripts/make-app.sh` bundles the host when
`target/release/flashtex-host` is built, or when it is given
`--engine-host <path>` (`--require-engine-host`, used by release builds,
fails the packaging when the host, its pool or its licence is missing):

- the host at `Contents/Helpers/flashtex-host`, signed before the app like the
  other helpers (identifier `<bundle id>.flashtex-host`; hardened runtime with
  `--sign`);
- its string pool at `Contents/Resources/engine/pdftex.pool`;
- its GPL licence at `Contents/Resources/engine/LICENSE`;
- the pinned no-TeX-Live bundle's lock at
  `Contents/Resources/engine/flashtex-bundle.lock`, from
  `tools/bundle/tl2026/flashtex-bundle.lock` (a GitHub Release asset of this
  repository; docs/distribution/texlive-bundle.md).

`components.json` records the host as `engine_host`. It is still a separate
process that the app only talks to over the socket.

**Lifecycle.**

- **One host per app.** The ShellModel owns one session with one project.
  The app has a single ShellModel (an App-level `@State` in
  `FlashTeXMacApp.swift`), so every window shows that one project and there
  is one host per app, not one per window. If windows ever get their own
  ShellModel, each gets its own session and host: document-scoped sessions
  inside one host would serialise every project on the host's single engine
  thread, and one project's COMPILE would evict another's checkpoints.
- **One engine.** The editor's underlines, explanations, the Problems
  panel, navigation, Export and Print all come from this engine; the app
  has no other worker, preview controller or display-list route.
- **The host dies with the app.** The host runs with `--once`: it serves one
  connection and exits when that socket closes, so it exits when the app
  quits or crashes.
- **Stale hosts are cleaned up.** A host left from an app that died before
  connecting (while the format was still being prepared) is killed at the next
  start, using its pid file in `~/Library/Caches/FlashTeX/engine-v3/hosts/`.
- **A crashed host is restarted.** If the host dies or the connection breaks,
  the session restarts it, at most 3 times a minute. The new host's first
  compile opens from the S₀ cache. Meanwhile the pages stay on screen, marked
  stale.

A development build therefore needs only
`cargo build --release -p flashtex-engine --bin flashtex-host`. The engine's string
pool (`pdftex.pool`) is read from beside the host, or else from
`crates/flashtex-engine/pdftex.pool` (the app sets `FLASHTEX_POOL`).

On first use the host builds `pdflatex.fmt` from the user's TeX Live. This took
4.75 s on an M5 Pro, and later uses read it from the cache. While it builds, the
pane shows "Preparing the pdfLaTeX format from your TeX Live…" with a timer. It
then shows which TeX Live was chosen and whether the format is ready.

The host no longer needs the pool beside it: a standalone `flashtex-host` or
`flashtex-initex` uses the copy compiled into it (written once to
`~/Library/Caches/FlashTeX/formats/pool/`), so `FLASHTEX_POOL` is optional.

**Without TeX Live** (DESIGN.md §4.4; `EngineV3Bundle.swift`). The host reads
a content-addressed bundle of unmodified TeX Live files instead, pinned by its
SHA-256 digest. Which bundle is data, not code; the packaged app ships the
lock of the published one (GitHub Release assets, `texbundle-tl2026-<n>`;
docs/distribution/texlive-bundle.md), and the first of these wins:

1. `FLASHTEX_BUNDLE_URL` and `FLASHTEX_BUNDLE_DIGEST`;
2. a `flashtex-bundle.lock` — the file `FLASHTEX_BUNDLE_LOCK` names, else
   `~/Library/Application Support/FlashTeX/flashtex-bundle.lock`, else one
   beside the host or in the app's `Contents/Resources/engine/`:

   ```toml
   url = "https://example.org/texlive-2026-core.ttb"   # or a path, relative to this file
   digest = "f7ed930fdd4e7a0138e7634bcef61a0ff3e192ecb09765f75e61bb1e840ec58b"
   ```

The app passes the lock it found to the host (`FLASHTEX_BUNDLE_LOCK`). Before
the first download of a bundle it asks (the Download TeX Files sheet, in the
style of the package consent sheet); the answer is kept for that bundle's
digest and source, so a new pinned bundle is asked about again. Downloads fail
closed: the host never fetches a lock file's bundle unless
`FLASHTEX_BUNDLE_ALLOW_FETCH` is `1` (a command-line user) or `<digest>@<url>`
of that bundle and the URL the app read, which the app passes only after the
user agreed to that bundle (a lock rewritten since, even with the same digest
at another server, is not fetched). The question is decided by the stored
answer, never by what is already in the cache; in
every other case the app starts the host with `FLASHTEX_BUNDLE_OFFLINE=1`,
whatever it made of the lock (a bundle set in the environment included), and
drops an inherited `FLASHTEX_BUNDLE_ALLOW_FETCH`. "Not Now" leaves the
window with the "Compiling needs the TeX bundle or a TeX Live installation."
message; its Download button allows the download and starts the engine
again. While the host fetches (the index and core on a cold cache, a
package on demand later) it prints `bundle_progress` lines, which the status
bar shows ("downloading TeX files: 1.2 of 2.8 MB (43%)"). `HELLO.texmf.bundle`
says which bundle the host reads and from which configuration.

## How it works

| piece | file | notes |
|---|---|---|
| protocol + decoder (MIT, Foundation only) | `Sources/FlashTeXDisplayListV3/` | frames, PAGE/FORM/FONT/IMAGE/SOURCES/control messages, COMPILE requests, the socket; `DL3Canonical` is the decoder-parity form (`dl3-dump --canonical`) |
| renderer (Core Graphics/Core Text) | `Sources/FlashTeXPreviewV3/DL3Renderer.swift` | draws a decoded page exactly as Core Graphics draws the engine's PDF |
| host process | `Sources/FlashTeXMac/EngineV3Host.swift` | locate, spawn (`--socket`, `--s0-cache ~/Library/Caches/FlashTeX/engine-v3/s0`), start-up line, `listening` |
| session | `Sources/FlashTeXMac/EngineV3Session.swift` | one incremental connection, edits per keystroke, pages in place, PAGES stale ranges, PDF fallback, latency |
| pane | `Sources/FlashTeXMac/EngineV3Preview.swift` | fit-to-width pages times the zoom (25–400 %, `PreviewZoom.swift`; pinch); only pages near the viewport hold bitmaps; rastered off the main thread; above 3 px/pt a page on screen is 512 px tiles (`EngineV3Tiles.swift`) |
| bench | `Sources/FlashTeXMac/EngineV3Bench.swift` | `FLASHTEX_V3_BENCH=main.tex`: keystroke → pixels |

- **⌘B and auto-compile.** ⌘B (File > Compile, the title bar's ▶, the
  palette) compiles with the host; after the host
  stopped (the restart limit) it starts it again. With Settings > Compile >
  Auto-compile off, edits wait ("edited — ⌘B to compile") until ⌘B or until
  auto-compile is turned on again. An outside change to an unopened
  `\input`/`\include` file (git checkout, another editor) recompiles.
- **Bibliography and index (protocol 3.2).** The app says `[3, 2]` and sends
  `external_tools: "auto"` for a trusted project (`"off"` for one #1332's
  trust check holds back, owner 9A), so the host runs bibtex, biber and
  makeindex from the user's TeX Live as latexmk would and compiles again
  with what they made. `TOOL` progress ("Running bibtex paper…", a failure,
  or why a tool did not run) shows in the pane and the status bar; the
  tools' DIAGNOSTICs join the Problems panel. An export waits for the
  tools to settle (a follow-up compile would interleave its frames) and
  runs none itself. It waits only for the newest finished compile's own
  cycle (a superseded one may never say `settled`) and fails after 300 s
  rather than waiting for ever. When the project folder's file set changes
  (a file created, removed, renamed, or its quarantine changed; hidden paths
  and the editor's own files aside), the next compile, an edit included,
  walks the project and decides trust again first.
- **Export PDF… and Print…** use the host's `export: true` run: the
  compressed PDF pdflatex would write (P-T2), with the resident run's
  `.aux`, so references are resolved. A compile first brings the host's
  copy up to the editor; the export is sent at its DONE, when the resident
  engine is idle, and edits typed meanwhile are held and sent after (the
  export's frames share the socket and have their own resource ids, so the
  reader drops them). Typing while the copy is brought up to date keeps the
  compile going, so the export starts at the next pause in typing. An
  export the host fails after it started still holds edits until its DONE.
  The bytes go through the export session: a sibling
  temp file, the overwrite-conflict check, an atomic rename, Cancel in the
  capture bar. Print prints the same bytes.
- **Edits.** Every change to the editor's text is sent at once as a COMPILE with
  byte `edits`. There is no debounce in the app.
  - **Fast path.** For typing in the main editor, the splice is computed from
    the text storage's edited range (`NSTextStorage.didProcessEditingNotification`,
    with UTF-8 offsets from Core Foundation) and sent before the editor's own
    processing. At 1,000 pages the editor's processing took 11.5 ms before the
    text reached the model.
  - **Slow path.** Everything else is diffed from the model's text: the common
    prefix and suffix, one splice per changed document. It also checks the fast
    path's byte count, and if the counts disagree it sends the whole buffer.
  - `FLASHTEX_V3_FAST_EDITS=0` turns the fast path off. A newer COMPILE supersedes the one
  that is running (protocol §6.3). The request carries `viewport` (the first
  visible page) and `have_fonts`.
- **The user's files are never written.** The host writes the editor's text to
  its files, so it compiles a copy of the project in
  `~/Library/Caches/FlashTeX/engine-v3/projects/<hash>-<pid>-<session>/src`.
  - **Copies are per session.** Each session of each app instance has its own
    copy, marked by an `owner` file (pid and process start time).
  - **Cleanup only touches exited instances.** At start-up the app removes
    only copies whose owner has exited; copies of running instances, and
    copies with no owner file, stay.
  - **A vanished copy is recovered.** If a copy disappears under a running
    host, the next edit re-creates it and restarts the host.
  - **`FLASHTEX_V3_CACHE`** moves the whole cache (the S₀ snapshots, the
    copies, the host pid files, and the formats, unless
    `FLASHTEX_FORMAT_CACHE_DIR` says otherwise). Benches and tests use it so they never share
    a running app's cache. The editor's
  documents are real files there. Every other project file is a symbolic link:
  images, `.bib` files, and includes the editor has not opened.
- **Pages** arrive edited page first. Each page is prepared (its resource ids
  resolved) on the socket's reader thread. A page that is on screen is
  rasterised there too, in Core Animation's native BGRA layout, so it reaches
  the main thread ready to install. The main thread only sets `layer.contents`
  in an explicit `CATransaction`, committed and flushed at once.
- **Latency stages.** `EngineV3Latency` records every stage of keystroke →
  pixels: edit hook, send, frame read, decode, prepare, raster, main-thread
  hop, commit, and the display link's next frame. Each is also an os_signpost
  (subsystem `tech.jay3332.flashtex.mac`, category `EngineV3Latency`) for
  Instruments.
- **Key → presented.** The bench (or `FLASHTEX_V3_PRESENT=1`) also records
  when the page actually went on screen, using `EngineV3PresentProbe`: a
  transparent 1×1 `CAMetalLayer` presented with the page's own transaction.
  - The window must be on screen. `FLASHTEX_V3_BENCH_FRONT=1` orders it in
    front without activating the app.
  - Measured, commit → presented is 16–24 ms, and never under two 120 Hz
    frames. That is the window server's pipeline. See the evidence README
    for the numbers.
  - `FLASHTEX_V3_BOOST=1` asks for 120 Hz while typing. It measured no gain,
    so it is off by default.
  Pages that `PAGES` marks stale are dimmed and get an orange border until they
  are current again. On `DONE` the pane drops pages past `count`.
- **Type 1 fonts.** Core Graphics still loads a Type 1 program from memory
  (`CGFont(CGDataProvider)`, macOS 26). That is also the rasteriser that draws
  the PDF's embedded subsets, so the FONT programs are loaded as sent and glyphs
  are drawn by name (`encoding[code]`), with no conversion and no FreeType. The
  protocol carries only the Type 1 programs, so a Linux or Windows client would
  load the same programs with FreeType, which reads Type 1 natively.
- **Fallback.** A page flagged INCOMPLETE is drawn from the compile's PDF
  (`DONE.pdf`) once `DONE` arrives. So is a page that draws an INCOMPLETE form
  (such as beamer's shaded balls) or has a resource that did not resolve.

## Gates

- `swift test --filter FlashTeXDisplayListV3Tests`: the Swift decoder must give
  the same canonical text as `dl3-dump --canonical` for the checked-in fixture,
  and for every parity fixture when `target/dl3-positions/` exists (it does after
  `tools/displaylist/check_positions.py`).
- `swift test --filter FlashTeXPreviewV3Tests`: the preview must be
  pixel-identical to Core Graphics' rendering of the engine's PDF at 1×, 2× and
  4×, with zero tolerance at every scale, Type 3 pages included. Glyphs and
  rules are drawn from the display list's `ORIGINS` and `RULE_GEOMETRY`
  (protocol §4.2, §4.4) when the host sends them. `FLASHTEX_V3_PARITY_SCALES`
  and `FLASHTEX_V3_PARITY_SMOOTH=1` run the scale sweep and the smoothing-on
  test; see `docs/evidence/p3-zero-tolerance-2026-10-02/`.

## Source mapping

- **Click on a page** (reverse search). The click selects the source character
  that glyph came from, opening an `\input`/`\include` file if it is not
  open yet.
- **⌘⇧J or ⌘-click in the editor** (forward search). The preview scrolls to
  the caret's glyph and flashes it.
- **While you type**, the preview follows the edit ("Preview follows the
  caret", as before). Scrolling the preview by hand pauses following until the
  next edit.
- **Agreement with pdflatex's SyncTeX** is measured in
  `docs/evidence/app-v3-preview-2026-09-29/`.

## Dark preview

The title bar's moon (its default follows Settings ▸ Appearance and the
system) draws the pages dark.

- **What changes:** the ground is dark, ink has its lightness inverted with
  hue kept, text stays readable, and images keep their pixels.
- **Light mode is the PDF exactly:** the zero-tolerance gate applies to light
  mode only.
- **Pages drawn from the PDF (INCOMPLETE)** are inverted as a whole bitmap,
  images included.

## Project trust

A project that came from another computer can run shell commands through
`\write18`. The preview decides per project:

| project | shell escape sent to the host | `\pdfshellescape` |
|---|---|---|
| made on this Mac (no quarantine attribute) | `restricted`, as pdflatex's default | 2 |
| downloaded or received, not yet trusted | `off` | 0 |
| trusted with the pane's button | `restricted` | 2 |

- "Came from another computer" means the main file or the project folder
  carries `com.apple.quarantine` (Safari, Mail, AirDrop, and the Archive
  Utility on a downloaded archive set it).
- An untrusted project shows "Trust This Project" above the pages.
  Trusting it records exactly the quarantined item and compiles again:
  - a downloaded `.tex` whose folder is not quarantined (`~/Downloads/paper.tex`)
    records that file, never its folder; another download beside it asks again;
  - a quarantined folder (an unpacked archive) records the folder; its files
    from the same download are covered, a file from a later download is not;
  - home, Downloads, Desktop, Documents and the temporary folder are never
    recorded as a whole;
  - in a project folder, every other quarantined file that did not come
    with the folder's download (TeX can `\input` any of them), such as a
    `.sty` downloaded later into a trusted folder, is asked about and
    recorded too. The pane counts them. The walk is the project copy's
    (20,000 entries, hidden files skipped);
  - a lone file in a shared folder (Downloads, Desktop, Documents, home,
    the temporary folder) is not a project: the folder is never walked for
    trust. Trust covers the file and the files its own download brought.
    Another download there counts only if the document reads it, found
    lexically at each decision (`\input`/`\include`, local
    `\usepackage`/`\documentclass` files, the main file's `.aux`/`.bbl`/...).
    Every other download never asks. A file name built by a macro is not
    seen; the worst case is restricted shell escape.
- The button records exactly the identities the prompt was computed from.
  If an item changed since, the project stays untrusted and the prompt is
  shown for what is there now.
- A project with no quarantine attribute anywhere (a git clone, `curl`,
  `unzip` in a shell) is trusted, by design.
- A record is the item's canonical path, inode and volume, and its
  quarantine event (the attribute's UUID). A rename, a move, or a new
  download at the same path asks again.
- The records are in the defaults key `FlashTeX.EngineV3.trustRecords.v2`.
  An instance with `FLASHTEX_V3_CACHE` keeps them in `<cache>-trust.json`
  beside that cache instead, and a test process without it in a temporary
  file: tests and benches never read or write the app's records.
- Restricted means only texmf.cnf's `shell_escape_commands` run, exactly as
  in pdflatex. Full shell escape (`on`) is never sent.
- The check runs with the project copy's walk, on a background queue (on
  open, on trust and on explicit compiles), never per keystroke and never
  on the main thread. A new project's first COMPILE is sent when the walk
  and the decision are done; until then nothing is sent with restricted
  shell escape.
- Tests: `EngineV3TrustTests`. The end-to-end test compiles a quarantined
  document whose second page exists only when `\pdfshellescape` is 2. It
  has one page before trusting and two after.

## Known gaps

- **Diagnostics** go to the Problems panel. The app asks for diag-v1
  (protocol §6.7, lane P5-DIAGNOSTICS), and a host that offers it sends
  structured `DIAG`s. Each row's underline is then the exact command TeX
  stopped at (`range`, byte columns), with the macro chain and TeX's help
  text. From an older host, the rows come from `DIAGNOSTIC` messages, placed
  on the reported line.
