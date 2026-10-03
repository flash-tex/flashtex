# TeXpand host findings (M0) and integration (M3)

These are the answers to PLAN.md M0 for FlashTeX's two editors: the Mac app's
`NSTextView` and the iPad companion's `UITextView`. The engine itself (M1–M2)
is in `apps/mac/Sources/FlashTeXEditorCore/TeXpand/`. This file says where it
plugs in (M3 and later) and what the editors lack today.

## Key findings

1. **No bridge is needed.** Both editors are Swift. The core is pure Swift in
   `FlashTeXEditorCore`, which the iPad package links through a symlink
   (`apps/ios/Packages/FlashTeXPadKit/Sources/FlashTeXEditorCore →
   apps/mac/Sources/FlashTeXEditorCore`). It is Foundation-only and runs
   headlessly under `swift test` (target `TeXpandTests`).
2. **The native snippet mechanism is minimal.** `LaTeXSnippet` (core,
   `LaTeXVocabulary.swift`) is replacement text, a caret offset and a list of
   point **stops** (UTF-16). It has:
   - no placeholder selection;
   - no mirrors or transforms;
   - no choices;
   - no nesting.

   Tab and ⇧Tab move between stops on the Mac; the iPad goes forward only.
   M3 must extend it (see "Snippets" below). Until then, `Snippet.flattened()`
   and `.latexSnippet` degrade the IR as PLAN §5 allows: placeholder text is
   inserted, mirrors copy their placeholder, and a choice takes its first
   option.
3. **Tab is contested.** On the Mac, Tab goes to, in order:
   - the completion popup (it moves the list selection);
   - snippet stops;
   - the caret-fix hint;
   - indentation.

   TeXpand's capture and postfix slot in ahead of snippet stops (PLAN §9.5).
4. **Scope tracking exists.** The core `SyntaxHighlighter` keeps an
   incremental per-line lexer state that tracks math, verbatim and comment
   environments, and answers `mode(at:)` and `state(at:)` in constant time
   per line. It keeps **no environment stack**: list, tikz, frame,
   algorithmic and tabular are unknown to it. M3 adds that stack.
5. **Ghost text has a precedent.** The error lens draws text after a line's
   last glyph through `CompletingTextView.foregroundDecorator`, and the
   caret-fix hint is drawn the same way. The capture highlight can use layout
   manager temporary attributes, as syntax colours and spell check do.
6. **TOML is TOMLDecoder.** The app had no Swift TOML parser
   (`flashtex.toml` is parsed by the Rust helper, which the iPad does not
   have). TeXpand now uses dduan/TOMLDecoder, a maintained pure-Swift TOML
   1.1 parser. The evaluation is in "TOML parser evaluation" below. Project
   config is a separate `texpand.toml` (see PLAN "Adaptations").
7. **Undo grouping is established.** Multi-edit changes use
   `beginUndoGrouping`/`endUndoGrouping` with `programmaticChanges` raised
   (`applyLineEdits`), and completion inserts use `breakUndoCoalescing()`
   around the change. Commit plus preamble insertion as one undo step
   follows that pattern.

## Editor components

| | Mac | iPad |
|---|---|---|
| Text view | `CompletingTextView: NSTextView` (`FlashTeXMac/Completion.swift`) | `EditorTextView: UITextView` (`FlashTeXPad/EditorController.swift`) |
| Owner | `SourceEditorView` (SwiftUI `NSViewRepresentable`); its `Coordinator` is the `NSTextViewDelegate` | `EditorController` (delegate and all editing logic) |
| Text system | TextKit 1 (`NSLayoutManager`; temporary attributes are used throughout) | TextKit 1 (`EditorTextStorage` + `NSLayoutManager`) |
| Shared logic | `FlashTeXEditorCore`: `AutoClose`, `BraceMatcher`, `EnvironmentEditingRules`, `LaTeXEditing`, `LaTeXVocabulary`/`LaTeXSnippets`, `SyntaxHighlighter`, now `TeXpand` | same (symlink) |

## Keystrokes, Tab and Esc

### Mac path of a keystroke

1. `CompletingTextView.performKeyEquivalent` handles ⌘ chords and ⌥⇧↑/↓.
2. `CompletingTextView.keyDown`:
   - Marked text (IME) goes straight to AppKit.
   - Vim (`VimMode.handle`) runs first when enabled.
   - ⌃Space, ⌘⇧Space and ⌘/ are handled here.
   - **With no completion session:**
     - Tab or ⇧Tab moves between snippet stops (`moveSnippetStop`).
     - Esc ends the snippet and the signature help, then dismisses the
       caret-fix hint, then opens the completion list.
     - Any other key sets `typingKey` and calls `super.keyDown`.
   - **With a session:**
     - ↑, ↓, Tab, Page Up/Down, Home, End, Return and Esc drive the list.
     - ⌘ chords and ←/→ close it.
     - Typing narrows it.
3. AppKit calls `insertText`, then the delegate's
   `textView(_:shouldChangeTextIn:replacementString:)` in `SourceEditorView`.
   The delegate handles auto-close, type-over, linked `\begin`/`\end` renames
   and fold bookkeeping. It skips its work when `programmaticChanges > 0` or
   during undo/redo.
4. `textDidChange` flushes the syntax model, the gutter and folds, and sends
   the change to the model.
5. When `keyDown` passes Tab or Esc on, AppKit's `doCommandBy` calls
   `Coordinator.handleTab`. That hands off to an active completion list,
   accepts the caret fix, or indents or outdents.

### Mac hook points for M3

- **`CharTyped`.** Observe in `shouldChangeTextIn` for single typed
  characters, where the delegate already distinguishes typing from
  programmatic and undo changes. Skip when:
  - there is marked text;
  - Vim is not in insert mode;
  - `programmaticChanges > 0`;
  - the undo manager is undoing or redoing.

  The capture FSM needs the typed character and the caret. The delegate has
  both.
- **Tab and Esc.** Intercept in `keyDown` *before* the snippet branch, and
  only when there is no completion session with a selection. §9.5's step 1
  maps to "the popup owns Tab" in this app, because Tab moves its selection:
  1. an active completion session keeps Tab;
  2. capture commits, or shows its diagnostic;
  3. a postfix candidate applies;
  4. the next snippet stop;
  5. the caret fix;
  6. indentation.

  Esc cancels a capture before the snippet, caret-fix and open-list branches.
- **Suppress the completion popup while capturing** (§9.1). Gate
  `textChanged`'s automatic open and `requestCompletion()` with a closure
  wired by the owner. The closures `mathModeAtCaret` and `caretFixVisible`
  already follow this pattern.
- **Cursor moves and focus loss.** `setSelectedRanges` (it already ends
  snippets when the caret leaves them) and `resignFirstResponder`.

### iPad

- **Typing** goes through `textView(_:shouldChangeTextIn:replacementText:)`
  in `EditorController`. The accessory bar and the tests use the same path
  (`type(_:)`).
- **Hardware keys** are `UIKeyCommand`s (the `EditorCommand` enum): Tab maps
  to `acceptOrNextStop` (`pressTab()`), Esc to `dismiss` (`pressEscape()`).
  ⇧Tab (`previousStop`) is reserved, so stops are visited forward only.
- **Without a hardware keyboard** there is no Tab. `EditorAccessoryBar` is
  the place for an "Expand" key and the Tab replacement.

## Snippets (native mechanism)

- **Mac.**
  - `CompletingTextView.insertSnippet` replaces the range in one undo step,
    places the caret, then records `snippetStops` (absolute) and
    `snippetStart`.
  - `shouldChangeText` shifts the stops through each edit. An edit spanning a
    stop ends the snippet.
  - Tab and ⇧Tab move; the last stop ends the snippet.
  - Esc, a non-empty selection, or the caret leaving `[snippetStart,
    lastStop]` ends it.
- **iPad.** `EditorController.snippetStops` is shifted with
  `AutoClose.shifted`, and `pressTab()` pops the next stop.
- **What TeXpand needs on top, planned for M3:**
  - **Selected placeholders.** Add stop lengths to `LaTeXSnippet` (for
    example `stops: [NSRange]`, or a parallel `stopLengths`) so visiting a
    stop selects its placeholder, and typing replaces it. The shift logic
    already handles ranges.
  - **Mirrors.** Either implement linked fields, where an edit inside a
    field's range rewrites its mirrors in the same undo group (as the linked
    `\begin`/`\end` rename does), or degrade as §5 allows. `Snippet.Flat`
    already gives the degraded text.
  - **Choices.** Offer them through the existing completion popup when the
    stop is visited. Until then, the first option is inserted.
  - **`$0`.** Use the last stop (`Flat.final`); `latexSnippet` already
    appends it.
- **Conversion.** `TeXpand.Snippet.flattened(baseIndent:indentUnit:)` returns
  the plain text and each field's range. `.latexSnippet` maps that onto
  today's `LaTeXSnippet`, so the M3 adapter is a few lines on each platform.

## Completion popup

- **Mac.**
  - `CompletionPopup` (an `NSPanel` child window) is driven by the text view.
    Its API is `requestCompletion()`, `session`, `close(_:)` and
    `isCompletionActive`.
  - `CompletionScheduler` debounces the automatic open, which fires after a
    typed character.
  - Rows can carry a `LaTeXSnippet`.
- **iPad.** `LocalCompletion` (FlashTeXPadKit) plus `PadModel`, with
  `onCommand(.acceptOrNextStop)` for Tab.
- **TeXpand uses.**
  - Suppress the popup during capture.
  - Later (Open question 3), `ref:`/`cite:` params could reuse the
    completion contexts the Mac already has for `\ref{`/`\cite{` (labels,
    bibliography keys).

## Undo

- **Pattern (Mac):**
  1. `breakUndoCoalescing()`.
  2. `undoManager.beginUndoGrouping()` and `programmaticChanges += 1`.
  3. One or more `shouldChangeText` / `textStorage.replaceCharacters` /
     `didChangeText` calls.
  4. `setActionName`, `endUndoGrouping()`, `breakUndoCoalescing()`.

  See `Coordinator.applyLineEdits` (EditorKeyHandling.swift) and
  `insertSnippet`.
- **Commit (M3).** Replace `[leader … caret]` and insert the preamble lines
  (M6) in one such group, then start the snippet session.
- **Undo-to-literal.** The typed characters were earlier typing groups, so
  one ⌘Z removes the group and the literal abbreviation reappears. Set the
  suppression mark when the undo manager reports undoing that group. Either
  observe `NSUndoManagerDidUndoChange`, or register an undo action inside the
  group that sets the mark.
- **iPad.** `textView.undoManager` with the same grouping calls. Edits go
  through `EditorController.replace`, where `programmatic` raised plays the
  role of `programmaticChanges`.

## Decorations, ghost text and preview

- **Mac.**
  - `CompletingTextView.foregroundDecorator` and `backgroundDecorator` draw
    over and under the text. The error lens uses the foreground one for
    line-end messages and the caret-fix hint, which gives the ghost-text
    preview a precedent and a painter to follow.
  - `NSLayoutManager.addTemporaryAttribute` can mark the capture region, as
    syntax colours, spell check and the match highlight already do.
- **iPad.** Attributes in `EditorTextStorage`, or an overlay view positioned
  from `layoutManager.boundingRect(forGlyphRange:in:)`.
- **Rendered math preview (Open question 5).** `MathHoverPreview` crops the
  formula out of an already-compiled preview page. An expansion that has not
  been compiled has no rendering, so the preview is ghost text only for now.

## LaTeX scope tracking (for the M3 ScopeProvider)

- **What exists.**
  - `SyntaxHighlighter` (core) keeps a per-line start state (`Mode`: `text`,
    `inlineMath`, `dollarDisplayMath`, `displayMath`, `parenMath`,
    `mathEnvironment(depth)`, `verbatim(env)`, `commentEnvironment`, …).
    It updates incrementally on edit, answers `mode(at:)` and `state(at:)`,
    and lexes `\verb`. The Mac already asks it for completion
    (`mathModeAtCaret`).
  - `EditorNavigation.enclosingEnvironment(at:in:)` (Mac only) finds the
    enclosing `\begin`/`\end` pair.
  - `BraceMatcher` (core) matches delimiters.
- **What is missing.**
  - An environment stack (for `list`, `tikz`, `beamer`, `alg`, `table`,
    `float` and `env:NAME`).
  - The preamble boundary (`\begin{document}`).
  - Frame offsets (the structure editor needs the body range).
- **M3 plan.**
  - Add an environment stack to the highlighter's per-line state, or keep a
    parallel per-line cache invalidated with it. That keeps `scope_at` inside
    the 1 ms budget (§8).
  - Build `TeXpand.ScopeStack` from it. Its frames already carry `start`,
    `bodyStart` and `bodyEnd`.
  - Move `enclosingEnvironment` into the core so the iPad gets it too.

## TOML parser evaluation (owner decision, DESIGN §1: reuse before building)

The owner asked for a maintained, full TOML parser in place of the in-core
subset. These were evaluated on 2026-09-30:

- **TOMLKit** (LebJe/TOMLKit, MIT): a Swift wrapper around toml++ 3.4 (C++).
  Last release 0.6.0, January 2024.
- **swift-toml** (mattt/swift-toml, MIT): also wraps toml++ 3.4. Last pushed
  February 2026.
- **TOMLDecoder** (dduan/TOMLDecoder, MIT): pure Swift, TOML 1.1 (a superset
  of 1.0). Release 0.4.5, July 2026.
- **The old in-core subset,** for reference.
- **Not considered:** jdfergason/swift-toml (TOML 0.4, unmaintained since
  2020).

**Compliance.** Measured with the official toml-test suite
(toml-lang/toml-test; valid documents must parse, invalid ones must be
rejected), using a scratch harness and the release build of each library.

| | toml-test 1.0.0 (208 valid / 501 invalid) | toml-test 1.1.0 | Aborts the process |
|---|---|---|---|
| TOMLKit | 208 / 489 of the 496 that do not crash it | not run | **yes, on 5 files** |
| swift-toml | 208 / 494 of the 496 that do not crash it | not run | **yes, on 5 files** |
| TOMLDecoder | 208 / 492; the 9 accepted are exactly TOML 1.1's relaxations (optional seconds, newlines and trailing commas in inline tables, `\x` escapes) | 218/218 valid, 494/494 invalid | no |
| old subset | 183 / 409 | — | no |

**Why the toml++ wrappers are out.** Both abort on malformed table headers
such as `[`, `[.]`, `[..]`, `[a` and an invalid header character. It is an
assertion in toml++'s `parse_key`, and it fires in release builds too.
TeXpand re-reads `texpand.toml` while the user is still editing it, so a
half-typed `[` would take down the app. Fixing that means patching the
vendored C++.

**Other criteria:**

- **Error positions.**
  - TOMLDecoder reports a line for syntax and value errors ("(Line 6) Syntax
    error: unterminated quote."); it has no columns.
  - For its few line-less errors (an illegal escape, a stray control
    character), the bridge finds the line by re-parsing growing prefixes.
    This runs only on error.
  - toml++ has line and column, but that cannot outweigh the aborts.
  - Our diagnostics use file and line, as before.
- **Key order.** TOMLDecoder keeps document order. toml++ sorts keys (a
  `std::map`). Only diagnostics order depends on it, but document order reads
  better.
- **Platforms and integration.**
  - It is plain SPM: macOS 10.15+ and iOS 13+. It needs Swift tools 6.0
    (the toolchain here is 6.3).
  - It has no dependencies: benchmarks, docs and formatting are opt-in
    through environment variables.
  - It builds as-is for the iPad through FlashTeXPadKit.
  - No C++ in the iPad target, so the licence-boundary check has nothing new
    to inspect.
- **Size and build time.** Release, arm64, a one-line program parsing and
  materialising a document. The machine was under load (averages 44 to 82),
  so the times are indicative.

  | | Binary | Clean release build |
  |---|---|---|
  | Foundation alone | 52 KB | 3 s |
  | with TOMLDecoder (about 5,500 lines of Swift) | 593 KB (**+540 KB**) | 12 s (**+9 s**) |
  | with TOMLKit | 943 KB (+890 KB) | 13 s (+10 s) |
- **Speed.** The whole 1.0 suite (709 files) parses in 36–72 ms, against
  about 50 ms for the toml++ wrappers.

**Decision: TOMLDecoder 0.4.5,** pinned `exact:` in `apps/mac/Package.swift`
and `apps/ios/Packages/FlashTeXPadKit/Package.swift`; bump deliberately.

- **Bridge.** `FlashTeXEditorCore/TeXpand/TeXpandTOMLLibrary.swift` converts
  the library's lazily-parsed tables into the core's ordered
  `TeXpand.TOMLTable`. It forces every value first, so every error surfaces
  at load.
- **Table lines** come from a scan of the `[header]` lines, which skips
  multi-line strings.
- **Dates and times** arrive as their TOML text.
- **The hand-written subset parser is deleted.**

## Config, settings and keybindings

- **App settings.**
  - `EditorPreferences` is a versioned snapshot in UserDefaults, with rules
    JSON-encoded. Small features keep their own keys (`ErrorLens`).
  - TeXpand follows the small-feature route: `TeXpandPreferences` stores the
    core's Codable `TeXpand.Settings` under `FlashTeX.TeXpand.settings`.
  - The **Settings › Abbreviations** tab (`TeXpandSettingsView.swift`) holds
    the master switch (off), the kinds, leader, fractions, missing packages,
    notation and the per-pack switches.
- **Project config.**
  - `ProjectManifest` gets `flashtex.toml` from the Rust helper and watches
    it with `DocumentWatcher`.
  - `texpand.toml` at the project root will be read in Swift by the core
    parser and watched the same way (M8 hot reload).
  - The user's global file is `~/Library/Application Support/FlashTeX/texpand.toml`
    (proposed).
  - The iPad reads whichever `texpand.toml` the transferred project carries.
- **Keybindings.**
  - Mac chords are handled in `keyDown`/`performKeyEquivalent` and in the
    Editor menu (`EditorMenu.swift`). Every user-visible command must also be
    registered in `AccessibilityCommand`
    (`FlashTeXAccessibility/AccessibilityCommands.swift`), and a test holds
    the table complete.
  - New TeXpand commands (the prompt and structure editor shortcut, and
    `toggleLigatures` with no default key) go in both places.
  - **Check before binding ⌘;:** macOS's standard Spelling menu uses ⌘; for
    "Check Document Now" wherever that menu is present.
  - The iPad adds `EditorCommand` cases.

## Structure editor (M10b): what the editors need

- **Mac.**
  - **Overlay.** An `NSView` added as a subview of `CompletingTextView`,
    framed from `layoutManager.boundingRect(forGlyphRange:in:)` plus
    `textContainerOrigin` for the environment's body range. As a subview it
    scrolls with the text. It must re-frame on layout, resize and fold
    changes.
    - This beats `NSTextAttachment` here: view-backed attachments
      (`NSTextAttachmentViewProvider`) are TextKit 2 only, and this editor is
      TextKit 1.
    - It also beats a child `NSPanel` like the completion popup, which would
      have to follow scrolling by hand.
  - **Focus.** Grid cells take first responder. Esc or the shortcut returns
    it to the text view with the caret in the edited cell.
  - **Hiding the source.** Dim or clear the text underneath with a temporary
    foreground attribute.
  - **Accessibility.** Expose the grid as an accessibility table.
- **iPad.** A `UIView` subview of the `UITextView`. Being a `UIScrollView`,
  it scrolls its subviews with the content. Frame it from the TextKit 1
  layout manager plus `textContainerInset`. Tab and Return come through
  `UIKeyCommand`s, and add-row and add-column buttons suit touch.
- **Edits.** Parse the body with `TeXpand.Grid.parse`, edit, print with
  `Grid.lines()`, and apply as one undo group (the `applyLineEdits` pattern).
  A column-spec change is a second replacement in the same group.

## The M3 integration (Mac)

What was built on the findings above. The core is headless; the Mac adapter
is `FlashTeXMac/TeXpandEditor.swift`, one per `CompletingTextView`, created
on the first keystroke.

- **Edits.** The adapter observes the text storage's `didProcessEditing`.
  Every edit (typed, programmatic, undo or redo) reaches the scope provider
  and `TeXpand.CaptureController`. The kind comes from
  `CompletingTextView.isTypingKeystroke` (a key event in progress) and the
  undo manager. The notification arrives before the view's `didChangeText`,
  so the automatic completion list already knows a capture is running.
- **Caret.** `setSelectedRanges` forwards caret moves, except while a
  completion or snippet selection is being applied.
- **Tab and Esc.** Handled in `keyDown` in §9.5 order:
  1. an open completion list keeps its keys;
  2. `texpand.tab()`: commit, or an Incomplete diagnostic that swallows the
     Tab;
  3. (postfix, M5);
  4. snippet stops;
  5. the caret fix;
  6. indentation.

  Esc ends a capture before the snippet, caret-fix and open-list branches.
- **Commit.** `insertTeXpandSnippet` goes through the existing
  `insertSnippet`, so it is one undo step named "Expand Abbreviation". It
  starts the snippet session with the first placeholder selected. VoiceOver
  hears "Expanded …", or the diagnostic.
- **Undo-to-literal.** The typing and the commit are separate undo groups, so
  one ⌘Z brings the literal back. The controller sees the undo edit restore
  the commit's literal, and marks it.
- **Popup suppression.** `scheduleAutomaticCompletion` does nothing while
  `isCapturing`. Typing through an already open list closes it.
- **Preview.** Drawn in `CompletingTextView.draw` after the error lens's
  `foregroundDecorator`: an accent wash and underline under the region, and a
  box under the line with the expansion (up to 12 lines), or the diagnostic
  in red. System colours, so light and dark both work.
- **Settings.** `TeXpandPreferences` caches the decoded settings and one
  engine per settings value. Changing Settings › Abbreviations rebuilds every
  editor's controller live. Tests use `TeXpandPreferences.override`, never
  the user's defaults.
- **Keybindings.** The prompt/wrap command (§9.6, M7) is left unbound.
- **M4/M5 (inline commits).** Instant atoms, ligatures and auto fractions
  commit on the keystroke after them. The storage notification only records
  the commit; `CompletingTextView.didChangeText` applies it through
  `replaceTeXpandText`, as its own undo step with the caret kept. Postfix and
  Tab fractions go through `insertTeXpandSnippet`, like abbreviations.
- **Exact edits.** The storage's `editedRange` can be wider than the change:
  typing `;` before a `$` reports `;$` replacing `$`. So `shouldChangeText`
  hands the adapter the exact range and string, and the storage range is only
  the fallback for edits that bypass it.

### iPad (follow-up)

The core runs unchanged on the iPad. `LaTeXSnippet`'s new fields default to
0, so its snippet session behaves as before. The adapter is not built yet,
because the iPad has no settings surface to turn TeXpand on. It would be:

- the same controller fed from `EditorController.textView(_:shouldChangeTextIn:replacementText:)`
  (typed when it comes from `type(_:)` or the keyboard) and from
  `textViewDidChangeSelection`;
- `pressTab()` and `pressEscape()` asking the controller first;
- the commit through `replace` inside one undo group;
- `snippetStops` gaining lengths (select via `selectedRange`);
- an "Expand" key on `EditorAccessoryBar` for keyboards without Tab;
- a settings row in the iPad's settings screen.

### Next

M4 (math generators, instant atoms), M5 (ligatures, postfix with the `//`
operator), M6 (auto-preamble; `Commit.requires` is already reported), M7
(prompt and wrap), M8 (`texpand.toml` loading and hot reload, magic
comments), M9 (macros), M10 and M10b (structural actions, structure editor)
and M11 (packs, scripting).
