# Can Swift do this? Yes — but not the way it was attempted

Short answer: **the look is achievable in Swift. The architecture you have is what's wrong, not the language.** But there's a second, sharper finding underneath that, and it's the one worth your attention.

---

## Finding 1 — the "SwiftUI can do it" existence proof does not say what people claim

[CodeEdit](https://github.com/CodeEditApp/CodeEdit) (23k stars) is cited everywhere as proof that SwiftUI can build an IDE that doesn't look stock. I had the codebase cloned and read. What it actually shows:

**Every IDE-critical surface in CodeEdit is AppKit.**

| Surface | What it actually is |
|---|---|
| Project navigator | `NSOutlineView` + plain `NSTableCellView` — no SwiftUI per row |
| Search results | `NSOutlineView` |
| Quick-open (⌘⇧O) | `NSTableView` |
| Window root | `NSWindowController` + `NSSplitViewController` |
| All splits | `NSSplitViewController`, driven via **private `_VariadicView` SPI** |
| Text editor | A from-scratch `NSView` text engine — not even `NSTextView` |
| Context menus | 240-line `NSMenu` subclass with `NSMenuDelegate` |

**SwiftUI carries only the decorative chrome:** tab bar, status bar, toolbar items, settings, inspectors.

They also migrated *away* from a SwiftUI root window. The comment they left behind is the single most useful sentence in the repo:

> "The previous decision led to a very jank split controller mechanism because SwiftUI's layout system is not very compatible with AppKit's when it comes to the inspector/navigator toolbar & split view system."

`HSplitView` usage across 676 files: **zero.**

So the honest framing is: **SwiftUI for pixels, AppKit for structure.** Your Fable agent almost certainly did the opposite — reached for `List`, `NavigationSplitView`, `Form`, default `ButtonStyle` — which is exactly the combination that produces a Mac utility app.

## Finding 2 — the text editor is a genuine wall, and it's the one that matters

This is where the evidence is strongest and where I'd push back on "just fix the SwiftUI."

- Marcin Krzyżanowski, after four years building on TextKit 2: *"It's not a silver bullet."* Apple's own TextEdit **falls back to TextKit 1** when you insert a table. ([Michael Tsai's roundup](https://mjtsai.com/blog/2025/08/15/textkit-2-the-promised-land/))
- CodeEdit started in 2022, has 32 contributors and weekly meetings, and its README still says **"not yet recommended for production use."** Four years, still pre-production. They gave up on `NSTextView` and wrote a Core Text engine from scratch.
- [Lithe-IDEA](https://github.com/1lck/Lithe-IDEA/issues/687), another native macOS IDE, has an open issue to **replace its hand-built `NSTextView` editor with Monaco** — citing scroll stutter where *"IntelliJ IDEA and VS Code remain smooth."*

Meanwhile **[MarkEdit](https://github.com/MarkEdit-app/MarkEdit)** — native macOS AppKit shell, CodeMirror 6 inside a `WKWebView` — ships in **4 MB total**, handles million-line files, and keeps force-touch lookup, Writing Tools, Shortcuts and AppleScript. Their stated reason:

> "TextKit is not better than contentEditable, the community doesn't even have one single editor that can compete CodeMirror or Monaco."

Worth noting: at WWDC 2026 Apple announced Notion is migrating to SwiftUI for performance — **and the editor stays web-based.** Even the company moving toward native keeps the text surface on the web.

---

## Recommendation

**Keep the Swift app. Fix it in three separable moves, in this order.**

### Move 1 — restyle the chrome (do this first, it may be enough)

Rebuild the shell against real IDE token values with AppKit where AppKit is load-bearing. This is `PROMPT-appearance-overhaul.md`. **If a day of this gets you to "looks like Zed," stop here** — the rest may be unnecessary, and it's cheap to find out.

### Move 2 — adopt a real editor component

Two options, both better than hand-rolling:

- **[CodeEditSourceEditor](https://github.com/CodeEditApp/CodeEditSourceEditor)** (701 stars, MIT, SwiftPM) — tree-sitter highlighting, line numbers, minimap, bracket matching, find/replace, inline error messages. Stays pure Swift. Caveat: its own README says not production-ready.
- **CodeMirror 6 in a `WKWebView`**, MarkEdit-style. More capable, pixel-themeable to match VS Code exactly, and the LaTeX ecosystem is there.

### Move 3 — keep PDFKit regardless

This is the reason not to go Tauri or Electron. In a Swift shell the editor pane can be a web view while the preview pane stays `PDFView`: GPU rendering, free text selection, search, annotations, printing, thumbnails.

**And SyncTeX is not a factor in this decision.** It's a coordinate map the TeX engine emits into `.synctex.gz`; the renderer only has to hit-test. LaTeX Workshop — the most-installed LaTeX IDE — does full bidirectional SyncTeX on pdf.js. Either renderer works.

---

## Why not the alternatives

**Tauri** — I'd avoid it here, and the reason is not size. `WKWebView` has an [unresolved dead-key bug](https://github.com/xtermjs/xterm.js/issues/5894): type `Option+N` then `Shift+7` and you get `~~` instead of `~/`, with the next keypress dropped. Chromium hosts are verifiably unaffected. **For a LaTeX editor — accented characters, tildes, backslashes constantly — that's close to disqualifying**, and you can't fix it because you don't own the engine. Also: every "Tauri is 20-50× lighter" article I found is SEO content with no methodology, and [Tauri's own issue tracker](https://github.com/tauri-apps/tauri/issues/5889) has a report measuring real shipping apps at **Tauri 421-581 MB vs Electron 240-337 MB**. And `awesome-tauri` no longer has an apps section — there is no Tauri editor to learn from.

**Electron** — strictly worse than the hybrid for you: same web UI, but you lose PDFKit, sandboxing ergonomics, native menus, and ~200 MB, in exchange for cross-platform reach you haven't asked for.

**Zed's GPUI** — `gpui` 0.2.2 is on crates.io under Apache-2.0, so it's technically consumable. But the **last release was October 2025, eleven months stale**, there's no docs.rs link, and the README tells you to read Zed's source or ask in Discord. You'd also write your own PDF renderer and SyncTeX hit-testing.

**Qt** — nobody builds this look in Qt. Qt Creator and KDevelop look like Qt apps, which is the aesthetic you're escaping. Qt's own materials are about *achieving* native styling.

---

## Two traps to avoid

**Licence:** `codemirror-lang-latex` is **AGPL-3.0**, because it derives from Overleaf's grammar. If FlashTeX is or might become proprietary, that dependency is unusable. The clean substitute: **VS Code's LaTeX TextMate grammar is MIT**, redistributed ready-packaged in Shiki's `tm-grammars` — and it declares embedded languages for `lstlisting`/`minted` blocks, so you get real highlighting inside code listings for free.

**App Sandbox:** if Mac App Store distribution is ever on the table, spawning user-installed `/usr/local/texlive/...` binaries is heavily restricted, and that will shape the app far more than the UI toolkit does. Staying in Swift keeps every entitlement and notarisation path well-trodden. Worth deciding early.

---

## What would change this

1. **Cross-platform within 18 months** → the Swift shell is dead weight; go Tauri and eat the text-input risk, or Electron.
2. **You spike CM6-in-WKWebView and hit the dead-key bug unfixably** → Electron.
3. **Move 1 alone fixes the complaint** → skip Moves 2 and 3 entirely.
4. **Your documents are TikZ/pgfplots-heavy and pdf.js hangs on zoom** → keeping PDFKit becomes mandatory, which the recommendation already does.
