# FlashTeX for Windows

**Work in progress. Not stable.** This native WinUI 3 IDE is an in-progress
port of the macOS app. It is not a supported product yet: expect missing
features, visual mismatches with the Mac preview, and breaking changes.

The shared Rust engine (`flashtex worker` / `flashtex-compiler`) is the same
one the Mac app drives. This tree is the Windows shell around it.

## Status

Working enough to open a `.tex` file, type, compile, and see a live preview:

- WinUI 3 unpackaged desktop app (Windows App SDK) with a Fluent title bar
- CodeMirror 6 editor hosted in WebView2
- Native preview of the runtime-v1 display list (positioned text and rules)
- Problems panel, project tree of open documents, PDF export via `flashtex-pdf`
- IPC clients for the compiler, preview controller, edit ledger, and project files

Still incomplete, including:

- Rendering-v2 exact preview (embedded-font glyph identity) and exact v2 PDF export
- Full folder-level project membership, watching, and some Mac-only workflows
- Nearby / iPad pairing
- Accessibility pass and MSIX packaging
- Visual parity with the macOS preview and with pdfLaTeX

Build and run it if you want to help; do not treat it as a daily driver.

## Requirements

- Windows 10 1809+ / Windows 11, x64
- .NET 8 SDK
- Windows App SDK runtime (framework package; the project is not
  self-contained)
- WebView2 Runtime
- A release build of the engine helpers next to the repo, at least
  `crates/compiler` (`flashtex-compiler`) and `crates/pdf` (`flashtex-pdf`,
  `flashtex-pdf-exact`)

This checkout is set up to build **without** the Visual Studio "Windows
application development" workload (`WindowsPackageType=None`). Do not flip
the app back to MSIX/PRI generation unless that workload is actually
installed.

## Build and run

From the repository root:

```powershell
cargo build --release --manifest-path crates/compiler/Cargo.toml
cargo build --release --manifest-path crates/pdf/Cargo.toml
cd apps/windows
dotnet build src/FlashTeX.App/FlashTeX.App.csproj -c Debug
dotnet run --project src/FlashTeX.App/FlashTeX.App.csproj -c Debug
```

The editor WebView loads `src/FlashTeX.Editor/web/dist`. After changing
`web/src`, regenerate it with `npm install` and `npm run build` in
`src/FlashTeX.Editor/web`.

If a previous run is still alive, kill it before launching again:

```powershell
taskkill /F /IM FlashTeX.App.exe /T
taskkill /F /IM flashtex-compiler.exe /T
```
