# Cross-platform evaluation (CROSS-PLATFORM-EVAL), 2026-09-30

**Lane:** CROSS-PLATFORM-EVAL (kabir-claude, mac-m5pro-kabir). **Pinned main:** `67a2ea078`.
**Scope:** read-only evaluation. No product code was changed. **Governing design:**
[`docs/design/engine-v2/DESIGN.md`](../../design/engine-v2/DESIGN.md) (§0–§3, §4.4–4.5, §5.1–5.2, §6, §9–§10).

**Owner question:** "With the current app architecture, how difficult would it be to make
FlashTeX OS-agnostic (macOS, Linux, Windows)? Is there any real reason to do it now rather
than later, and what design choices can we make now that make it easier later?"

This is an evaluation, not a decision to build. Each claim is tagged:
**[measured]** (run or counted in this session, with the command or run id),
**[code]** (read in the source at the cited line),
or **[belief]** (judgement or background knowledge that was not re-verified here).

---

## 1. Verdict

| | Linux | Windows |
|---|---|---|
| Engine + host + CLI | **Easy.** It already builds and passes trip, etrip and the pdfTeX regressions on `ubuntu-latest` [measured]. **1–2 engineer-weeks** to make it a product: gate the LaTeX-level tests with TeX Live on Linux, handle Debian and Fedora TeX Live layouts, package it. | **Moderate.** It does not compile today: about 15 Unix-only code sites (§2.1) plus Windows builds of 4 C libraries. **3–6 engineer-weeks** to reach trip/etrip/regressions and P-T1 against TeX Live's Windows pdfTeX. MiKTeX support is a separate **2–4 weeks**. |
| App | **Hard. This is where the cost is.** The Mac app is 67k lines of Swift/SwiftUI/AppKit/Core Text [measured]. No UI line carries over. A second shell reaching today's Mac feature level is **3–5 engineer-months**; a minimum viable one (editor, preview, build, errors, source sync) is **4–6 weeks**, on top of a shared Rust app core (**4–8 weeks**, which also helps the Mac). | Same shell as Linux if one cross-platform toolkit is used (§3). Add **1–2 weeks** for Authenticode signing, MSIX/MSI, winget and the updater. |
| Preview parity (§6.2) | A portable renderer plus a redefined parity gate: **2–4 weeks**. | Same. |
| **Total, first non-Mac OS** | **About 4–6 engineer-months** (1 lane), or about 2–3 calendar months with 3 parallel lanes. | **About 2–3 engineer-months more** after Linux. |

These are ranges for one competent engineer or lane [belief]. Agent throughput on
mechanical work has been higher, but UI and IME work is feedback-bound and does not
compress well.

- **Architecture if and when we do it:** keep the native Swift Mac app. Move all non-UI
  logic into a portable Rust core (MIT), which the app already partly has
  (bridge, edit-ledger, project-files and preview-controller: about 21k lines of Rust
  run as JSON Lines helpers [measured]). Build **one** cross-platform shell for Windows
  and Linux. Pick its toolkit after a 2-week spike between Tauri 2 + CodeMirror 6 and
  Qt 6 (§3). Draw the preview with a deterministic CPU rasteriser in Rust, and define
  parity against the same rasteriser rendering the exported PDF (§2.4).
- **Now or later: later.** There is no market, parity or speed reason to start now.
  Parity is priority 1, P3 and P4 are still in progress, and every extra OS multiplies
  the gates. pdflatex itself differs by OS: `\pdfmatch` uses the platform's own C regex,
  and `\write18` quotes differently on Windows (§2.1). **But about ten design choices are
  nearly free now and expensive later** (§4). Two of them touch work in flight: the
  display-list-v3 transport and path wording, and P3-APP-V3's Type 1 preview path.

---

## 2. Inventory by layer

### 2.1 Engine (`crates/flashtex-engine`, GPL-2+): Rust plus vendored C

**Linux status [measured].** The full-tier CI run
[`36653568565`](https://github.com/flash-tex/flashtex/actions/runs/36653568565)
(workflow_dispatch, head `5dc33d999`, 2026-09-30T01:07Z) passed all of these on `ubuntu-latest`:
- `rust workspace`, which builds every target and runs the tests;
- `trip test` and `etrip test`;
- `pdfTeX regression tests`.

On main `67a2ea078` (run `36662960668`), trip, etrip, the regressions and `build (workspace,
all targets)` passed on Ubuntu as well. **Caveat [code]:** the LaTeX-level engine tests skip
when TeX Live is absent. That covers `display_list_host.rs:47`, `host_incremental.rs:441`,
`latex_format.rs` and `incremental.rs`, and the hosted runners have no TeX Live. So Linux
has been proven for build and primitive-level parity, **not for LaTeX-level parity or the
resident host.** `ci.yml:381` also records that the engine's INITEX tests assume an FHS
layout and fail on the NixOS runner.

**Windows status [code; not compiled, since no Windows target or cross toolchain is installed here].**
The crate would not compile for `*-windows-*` today. Each site below is either
`std::os::unix` without a `cfg` or a libc symbol that is declared by hand:

| # | Site | What it relies on | Windows replacement | Effort |
|---|---|---|---|---|
| 1 | `src/host/server.rs:48–49, 259–268` | `UnixListener`/`UnixStream` (std has them only on Unix), mode 0600 | AF_UNIX via `uds_windows` or `socket2` (Windows 10 1803+ supports stream sockets only), or a named pipe | 1–2 d |
| 2 | `src/host/server.rs:57, 650–670` (`start_export`) | `UnixStream::pair()` + `pre_exec` + `dup2(fd, 3)`, plus `FLASHTEX_DISPLAY_LIST=fd:3` | Windows AF_UNIX has **no `socketpair`** and no numbered-descriptor inheritance ([MS devblog](https://devblogs.microsoft.com/commandline/af_unix-comes-to-windows/)). Pass a listener path or pipe name through the environment, or an inherited HANDLE | 1–2 d |
| 3 | `server.rs:49`, `formats.rs:770` | `CommandExt::arg0("pdftex")` (the engine dispatches on argv[0]) | an explicit `--progname pdftex` switch, or a hard link named `pdftex.exe` | 0.5 d |
| 4 | `src/arena.rs:56–100` | `mmap(MAP_ANON)` of hundreds of MB, relying on lazy commit (overcommit) | `VirtualAlloc(MEM_RESERVE)` plus a commit per chunk, or a full commit. Windows charges commit up front (a pagefile charge, not RAM) | 1 d |
| 5 | `src/host/mod.rs:744–796` | `mmap` of the persisted S₀ file | `CreateFileMapping`/`MapViewOfFile`, or the `memmap2` crate | 0.5 d |
| 6 | `src/formats.rs:276` | `MetadataExt` (ino, ctime) in `StatSig` | file index and change time from `GetFileInformationByHandle` | 0.5 d |
| 7 | `src/system.rs:1411–1415` | `\write18` via `/bin/sh -c` with `OsStrExt::from_bytes` | `cmd.exe` as `fsyscp_system` does. **Parity detail:** web2c quotes restricted-mode arguments with `"` on WIN32 and `'` elsewhere ([texmfmp.c](https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/lib/texmfmp.c)). Our `QUOTE` is `'` unconditionally (`system.rs:1360`) | 1–2 d |
| 8 | `src/pdftex/utils.rs:166` | `localtime_r`, `gmtime_r` | `localtime_s`/`gmtime_s` (reversed arguments) | 0.5 d |
| 9 | `src/incr.rs:1326–1335` | `clock_gettime(CLOCK_THREAD_CPUTIME_ID)` | `GetThreadTimes` | 0.5 d |
| 10 | `csrc/flashtex_regex.c` | `<regex.h>` (the MSVC C runtime has none) | vendor `pdftexdir/regex` (glibc's copy), which pdfTeX itself compiles only for MinGW, as the file's own header notes | 1 d |
| 11 | `build.rs` kpathsea list; `kpathsea-config/c-auto.h` | a Unix `c-auto.h`; the list omits `win32lib.c`, `knj.c`, `mingw32.c` | kpathsea supports WIN32 upstream (all three are vendored in `third_party/kpathsea/` [measured]); needs a Windows `c-auto.h` and file list | 2–4 d |
| 12 | `xpdf-config/aconf.h`, libpng, zlib | configure-style headers | xpdf, libpng and zlib all build on Windows upstream [belief] | 1–2 d |
| 13 | `src/resolver.rs:205–290` | `kpsewhich` without `.exe`; `/usr/local/texlive`, `/etc/paths.d`, `$HOME` | `C:\texlive\<year>\bin\windows`, `kpsewhich.exe`, `%LOCALAPPDATA%` | 0.5 d |
| 14 | `src/formats.rs:95–109`, `src/bundle/mod.rs:77` | cache root from `HOME`/XDG | `%LOCALAPPDATA%\FlashTeX` | 0.5 d |
| 15 | `src/bin/flashtex-dist.rs:384` | `std::os::unix::fs::symlink` | copy or hard link | 0.5 d |
| 16 | `crates/display-list-v3/src/lib.rs:46–80`, `client.rs:22` (MIT) | `UnixStream`, hand-declared `setsockopt` constants | same transport abstraction as #1 | 0.5 d |

**Already portable [code]:**
- The format-cache lock uses `std::fs::File::lock` (`formats.rs:684`), not `flock`.
- The S₀ codec is explicitly little-endian and keyed by engine build (`persist.rs:1–10`).
- `clone_file` falls back to a copy off macOS (`system.rs:2586–2605`).
- `cfile::os_path` already has a non-Unix branch (`cfile.rs:118–129`).
- Paths use `std::env::split_paths`.

**Most important, decided by design:** §5.2 chose **software copy-on-write** (b3) over
`mach_vm_remap`/`vm_copy`, and the engine does not use `fork` checkpoints (TeXpresso's
approach). The whole L2/L3 incremental system is therefore plain Rust over one flat word
space, with no OS virtual-memory tricks, and is portable by construction. This was the
choice most likely to have blocked Windows, and it is already the right one.

**Windows behaviours that are more than compile fixes [belief, from experience with Windows TeX ports]:**
- **File names as bytes versus UTF-16.** TeX sees bytes. kpathsea's WIN32 code converts
  through `win32_codepage` (texmfmp.c does the same for the working directory). We must
  match that exactly for parity with Windows pdflatex on non-ASCII names.
- **You cannot rename or delete an open file.** This affects format-slot replacement,
  S₀ persistence, atomic saves in `project-files` and the `.aux` splice (§5.3).
- **Antivirus scanning on file open.** Defender makes small-file opens markedly slower.
  kpathsea's `ls-R` helps, but it needs measuring against the §1.2 targets.
- **Process creation cost.** It matters for format builds and `\write18`, not for the
  resident per-keystroke path.

**MiKTeX [belief].** MiKTeX does not use kpathsea's `ls-R`. It has its own file-name
database and installs packages on demand. D12 (build our format from *their* `latex.ltx`)
still works, but lookup needs a MiKTeX resolver, either through its `kpsewhich`-compatible
tool or its database. Supporting TeX Live on Windows first is the cheaper path.

**Parity oracles become per-OS.** `csrc/flashtex_regex.c` documents that `\pdfmatch` is
whatever regex the platform's C library provides. pdflatex on macOS, glibc and Windows
(pdftexdir/regex) can therefore disagree on edge cases. The `\write18` quote character
differs as well. The rule in DESIGN §8, "never record host-dependent data on a different
host", extends to "a P-T1 oracle per OS". That is a real, recurring gate cost, and the
main argument for **later**.

### 2.2 Protocol (`display-list-v3`, MIT)

- **Payload: platform-neutral [code].** It uses little-endian integers, a
  `u32 length + u8 kind` frame (spec §2), scaled points and PDF units, and fonts shipped
  as programs with SHA-256 keys. None of it is Mac-specific.
- **Transport: Unix-specific in the words and in two mechanisms [code].**
  - Spec §6.1 says "Unix-domain stream socket"; DESIGN D11 and §3 say "Unix socket".
  - §6.6 defines `FLASHTEX_DISPLAY_LIST=fd:N`, which is an inherited numbered
    descriptor, and the host's export path depends on it.
  - AF_UNIX exists on Windows 10+ for `SOCK_STREAM`, but not `socketpair` or descriptor
    passing (MS devblog). Named pipes are the native alternative.
  - Framing needs nothing from the transport beyond an ordered, reliable byte stream,
    so the fix is wording plus one abstraction (§4, items 1–2).
- **Paths are under-specified [code].** Several fields carry OS paths: `IMAGE.file`
  (absolute), `FONT.file`, `SOURCES.files`, `COMPILE.root`, and `buffers[].path` and
  `edits[].path` ("relative to `root`"). The spec defines neither their encoding (UTF-8?
  what about non-UTF-8 bytes on Unix or unpaired UTF-16 surrogates on Windows?) nor the
  separator in relative paths. This is cheap to pin before v3 has external clients and
  a breaking change after.
- **Absolute paths are assumed shared [code].** `IMAGE` asks the client to read and decode
  the file itself (§5.2). That is fine for every desktop OS. It would not hold for a remote
  or iPad client, but that is not a cross-OS issue.

### 2.3 App (`apps/mac`, MIT Swift)

**Size [measured]:** `find Sources Tests tools -name '*.swift' | xargs wc -l`.

| Module | Lines | Notes |
|---|---:|---|
| `Sources/FlashTeXMac` | 58,438 | 132 files, one flat directory |
| `Sources/FlashTeXProtocol` | 4,562 | Foundation only (display list v2 decoding, runtime-v1, transfer-v1) |
| `Sources/FlashTeXAccessibility` | 2,093 | mostly Foundation models, plus AppKit views |
| `Sources/FlashTeXEditorCore` | 1,981 | Foundation only; already shared with the iPad app |
| **Sources total** | **67,074** | of which 18,041 are comments or blank lines |
| Tests | 57,480 | `FlashTeXMacTests` 54,394 |
| `apps/ios` (iPad companion, non-test) | 3,956 | |

**Classification by imports [measured].** A file counts as UI if it imports AppKit,
SwiftUI, Core Text, Core Graphics, QuartzCore, PDFKit, ImageIO, CoreImage, Accessibility
or UniformTypeIdentifiers:

| Class | Files | Lines | Share |
|---|---:|---:|---:|
| UI or Apple graphics | 82 | 42,025 | 63% |
| Apple non-UI: CryptoKit, Combine, Network, Security, Darwin, Compression, ObjC runtime | 14 | 7,112 | 11% |
| Foundation only (portable Swift, or portable to Rust) | 59 | 17,937 | 27% |

**The import count overstates UI coupling [measured].** Only 3,906 lines (about 8% of
about 49k code lines) touch a UI or Apple API directly. Examples:
- `Completion.swift`: 4,053 lines, 164 touching AppKit.
- `ShellModel.swift`: 1,686 lines, 13.
- `ProjectDocuments.swift`: 1,519 lines, 8.
- `DocumentFiles.swift`: 1,489 lines, 8.

So much of the "UI" 63% is logic that sits in UI files: completion ranking, Vim mode,
document and project models, diagnostics, navigation and caret context. My estimate
[belief] is that **35–45% of app code is portable logic by function**. A second shell would
reimplement it unless it moves to a shared core first.

**Portable logic already outside Swift [measured].** These helpers are MIT Rust and are
spawned as JSON Lines processes (`LineProcessClient.swift`, `BridgeClient.locateHelper`):

| Helper | Lines |
|---|---:|
| `project-files` | 7,115 |
| `bridge` | 5,833 |
| `edit-ledger` | 4,331 |
| `preview-controller` | 3,996 |

With `rendering-core` (11,684), `font-resources` (13,707) and `project-index` (2,291),
this pattern is already the "shared Rust app core". Porting mostly means continuing it.
Its portability [code]:
- `project-files/src/sys.rs` implements rooted `openat`-family operations for macOS and
  Linux only. Every other target gets `Unsupported` (`sys.rs:1–13, 360–372`).
  Windows needs a real implementation; the `cap-std` crate is the likely reuse [belief].
- `bridge/src/store.rs` has a `cfg(not(unix))` branch.

**Mac-only subsystems and their replacements:**

| Subsystem | Mac implementation | Cross-platform equivalent | Note |
|---|---|---|---|
| Editor text system | `NSTextView`/`NSLayoutManager` (TextKit 1), `SourceEditorView.swift` and about 20 `Editor*.swift` files | CodeMirror 6, Qt `QPlainTextEdit`/KTextEditor, or a custom editor on gpui | the hardest part: IME, bidi, accessibility, and large documents at < 16 ms |
| Syntax highlighting | `FlashTeXEditorCore/SyntaxHighlighter.swift` (Foundation) plus AppKit painting | tokenizer portable; painting per toolkit | move the tokenizer to Rust or keep it Foundation-only |
| Spell check | `LaTeXSpellCheck.swift` (NSSpellChecker) | Hunspell or the OS spell checker; the `spellcheck` crate exists (1,379 lines) | |
| Preview | Core Graphics/Core Text, Core Animation layers (§2.4) | Rust CPU rasteriser plus GPU compositing | parity redefinition needed |
| Printing / PDF | PDFKit (`PrintController`, `ProposalPreview`) | OS print dialogs; export is the engine's PDF anyway | |
| iPad link | `NearbyListener.swift`: Network.framework, Bonjour `_flashtex._tcp`, **TLS 1.2 PSK** `TLS_PSK_WITH_AES_128_GCM_SHA256` (`:69–130`) | mDNS (`mdns-sd`) plus TLS-PSK through OpenSSL or mbedTLS [belief: rustls has no TLS 1.2 external-PSK suites] | Mac-only unless the listener moves into the core |
| Credentials | Keychain (`ConversionCredential.swift`) | Windows Credential Manager, libsecret (the `keyring` crate) | |
| Settings | `UserDefaults`/`@AppStorage`, only 7 keys [measured] | JSON file | trivial |
| Updater | own GitHub-Releases feed (`UpdateFeed.swift`); no self-update yet | winget/MSIX update, AppImage zsync, Flatpak | the pure-value design ports easily |

### 2.4 Rendering and the zero-tolerance parity gate

**Today [code, DESIGN §6.2 and B.3].** `GlyphRunRenderer` draws with `CTFontDrawGlyphs`
from `CGFont`s built from the font programs, with anti-aliasing on and smoothing off
(`GlyphRunRenderer.swift:403–404`). `V2Parity` requires **0 differing pixels** against Core
Graphics rendering the exported PDF.

This gate is meaningful on macOS for a Mac-specific reason: Core Graphics is also the
system's PDF renderer (Preview.app, PDFKit). The preview is pixel-identical to what the
user sees when they open the PDF.

**Off macOS there is no such system renderer [belief].**
- Linux viewers disagree among themselves: Evince and Okular use poppler (Cairo/Splash);
  others use MuPDF or pdf.js.
- On Windows, Edge, Acrobat and SumatraPDF (MuPDF) disagree too.
- "Identical to the PDF" must therefore mean **identical to the PDF as rendered by a
  reference rasteriser we choose.** The only way to get that by construction is to use
  the same rasteriser for both: the display list and the exported PDF.

Candidates:

| Option | Licence (MIT side?) | Deterministic across OS and GPU | Fit |
|---|---|---|---|
| **tiny-skia** (CPU, a Skia subset in Rust) | BSD-3 (yes) | yes (pure CPU Rust) | fills paths from outlines we decode ourselves; pairs with a Rust PDF renderer for the reference side. Typst's PNG export uses it [belief] |
| **Skia** (`skia-safe`) + FreeType | BSD-3, FTL (yes) | CPU raster yes; GPU no | heavy build; the most mature |
| **PDFium** (the reference side) + FreeType | BSD-3 [belief] (yes) | yes on CPU | "PDF rendered by PDFium" is a credible reference because Chrome uses it; the preview would need the same AA and glyph path |
| **Vello / vello_cpu** | Apache/MIT (yes) | GPU no; the CPU variant is young | DESIGN D6 rejected Vello on Mac because it does not match Core Text; off Mac that objection disappears, but maturity remains |
| poppler / MuPDF | **GPL / AGPL (no)** | | **must not** be linked into the MIT app (§3); usable only as a separate-process oracle |
| DirectWrite/Direct2D (Windows), Cairo + FreeType (Linux) | system (yes) | per-OS | means two more per-OS renderers and two more parity gates |

**Type 1 (the decision being made in P3-APP-V3).** Core Text dropped Type 1, so §6.2 names
two routes: "a converted in-memory font or FreeType". Portability ranks them as follows
[code + belief]:
- **Decoding outlines in portable code, then filling paths with the platform rasteriser,
  is the portable route.** Only "fill this path" is platform-specific.
  `crates/font-resources/src/type1_outline.rs` (717 lines, MIT) is already an original
  bounded Type 1 charstring decoder, with `eexec.rs`, `pfb.rs` and `type1_matrix.rs`. It
  or FreeType (FTL licence) serves every OS.
- A **Type 1 → CFF/OpenType conversion** for `CTFontCreateWithGraphicsFont` is also
  consumable by FreeType and DirectWrite. It is portable in principle, but costs more to
  build.
- **Only a route that depends on Core Text accepting the font directly would be
  Mac-locked.** Neither candidate does.
- **Recommendation:** if both candidates give 0 differing pixels against Core Graphics'
  rendering of the exported PDF, take the one whose outline decoding lives in the shared
  Rust core. If only the CT-glyph route reaches 0 px, take it on the Mac. Parity outranks
  portability, and the Rust decoder still serves the other OSes.

**Performance [belief, from B.3].** Core Graphics/Core Text at 2x takes 0.75 ms for a
3,500-glyph page. A CPU rasteriser like tiny-skia with a glyph-path cache should stay
inside the 8.3 ms budget of a 120 Hz frame, and 512 px tiles rasterised in parallel
(§6.2) port unchanged. Measuring this is part of the 2–4-week renderer estimate.

---

## 3. Architecture options for a multi-OS product

Constraints: an editor at the quality of a native text system (IME, accessibility,
bidi, large files); **≤ 16 ms p95 keystroke to pixels** and 120 Hz scroll (§1.2); the MIT
app and GPL engine boundary on every OS (§3); signed packages (DMG, MSIX/MSI,
AppImage/Flatpak/deb); and the iPad companion.

The licence boundary **holds on every OS**. It is a process boundary plus a versioned
socket protocol (D11), and nothing in it is Mac-specific. The engine ships as a separate
executable next to the app, "mere aggregation" exactly as in the DMG.
- Flatpak: the engine goes in the same bundle as a separate binary.
- Windows: MSI/MSIX or winget; no Store needed [belief].

### (a) Native shell per OS over a shared Rust app core

- **Pros:** the best editor and platform feel on each OS. The Mac app stays exactly as is.
- **Cons:** three UI code bases (AppKit, WinUI 3, GTK4 with GtkSourceView) triple UI
  maintenance. WinUI 3 plus Rust interop is awkward: C#/C++ calling Rust through a C ABI.
- **Effort:** 3–5 months per extra OS for the shell, after the core [belief].

### (a′) Native Mac shell + ONE cross-platform shell for Windows and Linux (recommended)

The Mac stays at native quality. Windows and Linux share one shell. Finalists for that
shell:

**Tauri 2 + CodeMirror 6** (MIT/Apache; web front end in a thin native shell)
- CodeMirror 6 is a mature editor with IME, screen-reader support and large-document
  virtualisation [belief]. Tauri's Rust back end links our MIT core directly.
- Risks:
  - The preview inside a web view. Drawing glyphs with Canvas2D uses the web view's own
    rasteriser, which differs per OS: WebView2/Skia on Windows, WebKitGTK on Linux. So
    the preview must be rasterised in Rust and handed to the web view as bitmaps, or
    drawn in a native child surface. Moving bitmaps into WebKitGTK has no zero-copy path
    [belief], so meeting 16 ms on Linux is the thing to measure.
  - WebKitGTK performance on Linux [belief].

**Qt 6 (LGPLv3, dynamically linked; C++ or `cxx-qt`)**
- Proven for exactly this product class (TeXstudio). Native IME and accessibility.
  `QPainter`/QRhi can show our Rust tiles without copies through the web stack.
- Risks: editor quality needs KTextEditor (LGPL) or custom work; C++ or bindings
  friction; LGPL relinking obligations (manageable).

**Decision rule:** a 2-week spike that implements the edit → display list → preview loop
on Windows and Linux with both finalists and measures:
- keystroke-to-pixels p95;
- IME with a Japanese or Chinese input method;
- screen-reader navigation;
- a 1,000-page scroll at 120 Hz.

### (b) One cross-platform UI everywhere, including macOS

**Rust-native (gpui, Slint, egui, Iced, Xilem)**
- gpui (Apache-2.0) is the strongest: Zed ships on it on all three OSes. But Zed's editor
  crate is **GPL-3** and cannot be reused on the MIT side. Screen-reader accessibility in
  gpui was weak as of 2025 [belief]. FlashTeX has a whole accessibility module (2,093
  lines plus 1,392 lines of tests).
- egui, Iced and Slint lack a professional multi-line code editor with full IME and
  accessibility [belief].

**Flutter:** a reasonable desktop editor and IME, custom rendering through Impeller/Skia,
Rust through `flutter_rust_bridge`. On the Mac it is a regression from AppKit text
[belief].

**Swift everywhere:** Swift toolchains exist for Linux and Windows, and the 17.9k
Foundation-only lines would compile. But SwiftUI and AppKit do not exist there, and the
WinUI and GTK bindings (`swift-winrt`, adwaita-swift) are niche [belief].

**Verdict:** rewriting the Mac app into any of these throws away the 67k-line native app
that DESIGN §10 keeps "as-is", for a worse Mac editor. **Rejected.**

### (c) Web front end in a desktop shell everywhere (Electron/Tauri, VS Code-style)

- **Pros:** one UI code base for all three OSes, and a future web or Overleaf-style
  product for free.
- **Cons:**
  - It gives up the native Mac app and the Core Graphics/Core Animation preview that
    currently holds 0-px parity with Preview.app (D6).
  - Electron's memory cost [belief].
  - The 16 ms / 120 Hz preview inside a web view is unproven (see (a′)).
- **Verdict:** only if a browser product becomes a goal. In that case the engine would
  need WASM or a server, which is a separate decision.

---

## 4. Now or later

**Later, for building it.**
- The owner's order is parity > speed > maintainability.
- P3 and P4 are in flight.
- A second OS adds a per-OS parity oracle (§2.1), a second preview-parity definition
  (§2.4), Windows file semantics, per-OS CI runners and signing. None of that serves
  priority 1 or 2 on the Mac today.
- The portable parts (engine, incremental system, protocol) keep getting more portable as
  they mature, provided the items below are adopted.

**But these choices get more expensive over time, and are nearly free now:**

| # | Choice to adopt now | Cost now | Why it gets expensive later |
|---|---|---|---|
| 1 | **Protocol text:** display-list-v3 §6.1 and DESIGN D11 say "an ordered, reliable byte stream; the reference transport is a Unix-domain socket (a named pipe or AF_UNIX on Windows)" | words | clients hard-code the assumption once v3 is public |
| 2 | **Inherited-endpoint grammar:** `FLASHTEX_DISPLAY_LIST` accepts `fd:N` **or** `socket:PATH` / `pipe:NAME`; the host's export path uses the named form where `socketpair` is unavailable | hours | `fd:N` + `socketpair` + `dup2` has no Windows equivalent |
| 3 | **Pin path encoding in v3:** every path field is UTF-8 (WTF-8 for non-Unicode OS names), absolute paths are OS-native, and `buffers/edits.path` are relative with `/` separators | words | a breaking wire change after release |
| 4 | **One `os` module per Rust crate:** put the engine's ~15 Unix sites (§2.1) behind `cfg(unix)`/`cfg(windows)`, with `compile_error!` on unknown targets. Today several `#[cfg(not(target_os = "macos"))]` branches silently assume **Linux** constants (`arena.rs:70–73` `MAP_ANON = 0x20`, `display-list-v3/src/lib.rs:47–58` socket constants, `incr.rs:1329–1332` clock id) | 0.5–1 d | the site count keeps growing with host work |
| 5 | **Keep new app logic out of Swift UI files:** new non-UI logic goes into the Rust helpers (the existing JSON Lines pattern) or, at minimum, into Foundation-only Swift targets like `FlashTeXEditorCore` | none (a convention) | every logic line added to an AppKit file is a line to port |
| 6 | **P3-APP-V3 Type 1 path:** when parity is equal, choose the route whose outline decoding lives in portable Rust (`font-resources/type1_outline.rs`) or FreeType (§2.4) | none if equal | a Core-Text-specific conversion pipeline would be Mac-only work to redo |
| 7 | **Generalise the §6.2 principle in DESIGN:** "The preview is pixel-identical to the exported PDF as rendered by the platform's reference rasteriser (Core Graphics on macOS)" | words | keeps D6 from reading as a mandate that forbids other OSes |
| 8 | **Keep the non-Mac Linux legs green and extend one:** the Linux engine legs already gate (§2.1). Add a nightly Linux job **with TeX Live** so LaTeX-level engine and host tests stop skipping there. A NixOS runner exists; fix the FHS assumption in `tests/initex.rs` | about 1 d | Linux rot is cheap to catch now and costly to bisect later |
| 9 | **Cache and config roots through one function** (it exists: `formats::cache_dir_default_root`). Add `%LOCALAPPDATA%` there and do not add other `HOME`-based paths | hours | scattered roots |
| 10 | **Nearby protocol:** keep the companion protocol on standard primitives (TLS-PSK over TCP, mDNS) as it is now, and document them in the spec so a non-Mac host could implement it. Do not add Apple-only pieces such as Multipeer or iCloud | none | an Apple-only pairing path would strand Windows and Linux users' iPads |
| 11 | **MIT-side licence list:** add poppler (GPL) and MuPDF (AGPL) to the licence-boundary check's deny list for MIT crates. They are the obvious PDF renderers on Linux | minutes | stops an accidental reference-renderer link |

**In flight, and what to change:**
- **display-list-v3** (the host-unify/P4 work, landed as `b78e5ea4b`, 3.1): adopt items 1–3
  in the spec now. The code change for item 2 can wait until a port.
- **P3-APP-V3 (Type 1 preview):** apply item 6. Nothing else in the lane is Mac-locked
  beyond the Mac shell itself, which is expected.
- **The engine host (`src/host/server.rs`, `mod.rs`):** the newest Unix-specific code
  (`socketpair`/`dup2`/`pre_exec`, the `mmap` of S₀). Apply item 4 at the next touch of
  those files.
- **Nothing in flight needs to stop or be redone.** The largest past risk (checkpoints
  through `mach_vm`/`fork`) was already avoided by §5.2's measured choice.

---

## 5. Prior art

| Project | How it went cross-platform | What it cost |
|---|---|---|
| **Zed** (Rust, gpui) | Mac-only first. Linux shipped **2024-07-10** with a new Blade (Vulkan) renderer and **447 PRs** from the community ([Zed blog](https://zed.dev/blog/zed-on-linux)). Windows stable shipped **2025-10-15** on DirectX 11 + DirectWrite, "to match the Windows look and feel", with a dedicated Windows team and a full-time platform lead ([Zed blog](https://zed.dev/blog/zed-for-windows-is-here)) | Even with a UI framework designed to be portable, Windows took about 15 more months after Linux. The listed pain points were IME, keyboard layouts, multiple monitors, 120–144 Hz displays and WSL. The same pain points would apply to us |
| **VS Code** (Electron + Monaco) | cross-platform from day one [belief] | a large team; memory and latency are the known trade-off; the editor is a web editor |
| **TeXstudio** (Qt, C++) | Qt from the start, as a fork of Texmaker [belief] | a non-native feel on the Mac; the PDF preview uses poppler-qt, which is possible because TeXstudio is itself GPL; a MIT app cannot do that |
| **Overleaf** (web) | a browser app; TeX Live runs server-side in containers [belief] | no OS problem, but each edit costs a server round trip, which is exactly the latency FlashTeX is built to remove |
| **Typst** | a Rust CLI (Apache-2.0) on all OSes; the web app runs the compiler as WASM in the browser [belief] | the compiler was designed pure-Rust and deterministic from day one, so portability came almost free. That supports our item 4 and the tiny-skia route |
| **TeXShop** (Cocoa, PDFKit) | never left the Mac [belief] | kept Mac quality and never reached Windows or Linux users. This is FlashTeX's current D14 position |
| **Tectonic** (Rust + XeTeX C) | builds on Windows, Linux and macOS through vcpkg/MSYS2 [belief] | Windows C-dependency builds were the recurring pain, matching our §2.1 rows 10–12 |

---

## 6. What was verified, and what was not

**Verified in this session:**
- The CI results on Ubuntu (runs `36653568565` and `36662960668`).
- Every line count and import classification in §2.3.
- Every code citation in §2.1–2.4.
- That kpathsea's WIN32 sources (`win32lib.c`, `knj.c`, `mingw32.c`) are vendored.
- web2c's WIN32 `QUOTE '"'` and `fsyscp_system`, read in upstream texmfmp.c.
- Windows AF_UNIX limitations (no `socketpair`, no descriptor passing, stream only), from
  the MS devblog.
- The Zed Linux and Windows dates and backends, from the Zed blog.

**Not verified (beliefs):**
- **Windows compilation.** No Windows target or cross toolchain is installed on this host,
  and the lane forbids installing one. Every Windows effort is a reasoned estimate.
- Toolkit claims: CodeMirror, Qt, gpui accessibility, WebKitGTK bitmap transfer.
- rustls TLS 1.2 PSK support.
- The licences and users of PDFium and tiny-skia.
- The effort ranges generally.

The 2-week toolkit spike (§3) and a Windows `cargo build` job would turn the largest of
these beliefs into measurements.
