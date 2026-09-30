# Engine v3 preview (experimental, off by default)

Lane P3-APP-V3 ([DESIGN.md](../../../docs/design/engine-v2/DESIGN.md) §3, §6.1–6.2, §12 P3).
The preview pane can show pages from the pdfLaTeX-compatible engine
(`crates/flashtex-engine`, GPL) instead of the old engine. The app never links the
engine. It starts `flashtex-host` as a separate process and speaks the MIT
[display-list-v3 protocol](../../../docs/protocol/display-list-v3.md) over a Unix
socket. The licence boundary is that process boundary
(`scripts/check-license-boundary.sh`).

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
  byte `edits`: one splice per changed document, from the common prefix and
  suffix. There is no debounce in the app. A newer COMPILE supersedes the one
  that is running (protocol §6.3). The request carries `viewport` (the first
  visible page) and `have_fonts`.
- **The user's files are never written.** The host writes the editor's text to
  its files, so it compiles a copy of the project in
  `~/Library/Caches/FlashTeX/engine-v3/projects/<hash>/src`. The editor's
  documents are real files there. Every other project file is a symbolic link:
  images, `.bib` files, and includes the editor has not opened.
- **Pages** arrive edited page first. Each page is prepared (its resource ids
  resolved) on the socket's reader thread and rasterised on a concurrent queue.
  The main thread only sets `layer.contents` in an explicit `CATransaction`.
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

## Known gaps

- There is no SyncTeX click-through or caret-follow yet. The protocol provides
  the source spans; the pane does not use them.
- Zoom is fit-to-width only, with no pinch. There are no 512 px tiles yet; that
  is lane #1228's work on the v2 pane.
- Diagnostics are counted in the pane, not shown in the Problems panel.
- One host runs per app session, not per open document. If the app crashes, the
  host is left running.
