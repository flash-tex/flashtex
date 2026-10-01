# Engine v3 preview (experimental, off by default)

Lane P3-APP-V3 ([DESIGN.md](../../../docs/design/engine-v2/DESIGN.md) §3, §6.1–6.2, §12 P3).
The preview pane can show pages from the pdfLaTeX-compatible engine
(`crates/flashtex-engine`, GPL) instead of the old engine. The app never links the
engine. It starts `flashtex-host` as a separate process and speaks the MIT
[display-list-v3 protocol](../../../docs/protocol/display-list-v3.md) over a Unix
socket. The licence boundary is that process boundary
(`scripts/check-license-boundary.sh`).

## Quick start (development)

```
scripts/run-mac-dev.sh                 # build flashtex-host, flashtex-bridge and the app (release), launch with engine v3 on
scripts/run-mac-dev.sh ~/thesis/main.tex   # … and open that file (or a folder) at launch
scripts/run-mac-dev.sh --no-build      # launch what is built
scripts/run-mac-dev.sh --old-engine    # the same app with the old preview
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

## Turning it on

Use any one of these:

- **View > Engine v3 Preview (Experimental)**, a toggle that is remembered.
- `defaults write FlashTeXMac FlashTeX.EngineV3.enabled -bool YES`. The domain is
  the app's bundle id when it runs as a bundle, or `FlashTeXMac` for the SwiftPM
  executable.
- `FLASHTEX_ENGINE_V3=1` in the environment. `0` forces it off.

When the flag is off, nothing of this runs: the old worker, the v2 pane and the
v1 pane behave exactly as before (D13).

## Where the host comes from

The app looks for `flashtex-host` in these places, in order:

1. `FLASHTEX_HOST`
2. the `FlashTeX.EngineV3.hostPath` default
3. `Contents/Helpers/flashtex-host` in the app bundle, or beside the app's executable
4. `target/release/flashtex-host` (then `target/debug`) in the repository

**Packaged app.** `apps/mac/scripts/make-app.sh` bundles the host when
`target/release/flashtex-host` is built, or when it is given
`--engine-host <path>`:

- the host at `Contents/Helpers/flashtex-host`, signed before the app like the
  other helpers (identifier `<bundle id>.flashtex-host`; hardened runtime with
  `--sign`);
- its string pool at `Contents/Resources/engine/pdftex.pool`;
- its GPL licence at `Contents/Resources/engine/LICENSE`.

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
- **One engine at a time.** With the v3 preview on, the old engine compiles
  nothing: not on open, not on ⌘B, not from a file watcher. Its last result
  is dropped when v3 is turned on, so the editor's underlines, explanations,
  the Problems panel's line labels, navigation, Export and Print never show
  old-engine output under v3. The old worker process stays attached and
  idle, so turning v3 off compiles with it at once. (The developer-only
  durable-helper route, `FLASHTEX_PREVIEW_CONTROLLER`, still talks to its
  ledger when a document is saved; its previews are not shown under v3.)
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

## How it works

| piece | file | notes |
|---|---|---|
| protocol + decoder (MIT, Foundation only) | `Sources/FlashTeXDisplayListV3/` | frames, PAGE/FORM/FONT/IMAGE/SOURCES/control messages, COMPILE requests, the socket; `DL3Canonical` is the decoder-parity form (`dl3-dump --canonical`) |
| renderer (Core Graphics/Core Text) | `Sources/FlashTeXPreviewV3/DL3Renderer.swift` | draws a decoded page exactly as Core Graphics draws the engine's PDF |
| host process | `Sources/FlashTeXMac/EngineV3Host.swift` | locate, spawn (`--socket`, `--s0-cache ~/Library/Caches/FlashTeX/engine-v3/s0`), start-up line, `listening` |
| session | `Sources/FlashTeXMac/EngineV3Session.swift` | one incremental connection, edits per keystroke, pages in place, PAGES stale ranges, PDF fallback, latency |
| pane | `Sources/FlashTeXMac/EngineV3Preview.swift` | fit-to-width pages; only pages near the viewport hold bitmaps; rastered off the main thread |
| bench | `Sources/FlashTeXMac/EngineV3Bench.swift` | `FLASHTEX_V3_BENCH=main.tex`: keystroke → pixels |

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
    copies, the host pid files). Benches and tests use it so they never share
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
  4×. A small measured floor is allowed at 1× only; see
  `docs/evidence/app-v3-preview-2026-09-29/`.

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

- Zoom is fit-to-width only, with no pinch, and there are no 512 px tiles yet (lanes #1228 and P3-SOURCE-MAP own zoom, tiles and click-to-source); that
  is lane #1228's work on the v2 pane.
- **Diagnostics** go to the Problems panel. The app asks for diag-v1
  (protocol §6.7, lane P5-DIAGNOSTICS), and a host that offers it sends
  structured `DIAG`s. Each row's underline is then the exact command TeX
  stopped at (`range`, byte columns), with the macro chain and TeX's help
  text. From an older host, the rows come from `DIAGNOSTIC` messages, placed
  on the reported line.
