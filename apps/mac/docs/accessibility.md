# FlashTeX Mac accessibility

Owner: `mac-accessibility` (parent `mac-claude-a`), issue #2. Target
`FlashTeXAccessibility` (`apps/mac/Sources/FlashTeXAccessibility`, depends only
on `FlashTeXProtocol`) plus `Tests/FlashTeXAccessibilityTests`. The shell
attaches it through seven hook lines (marked `// FlashTeXAccessibility`) in
`PreviewView.swift`, `ContentView.swift`, and `SourceEditorView.swift`. No
visible behavior changed.

## What the models provide

- `AccessibleDocumentModel` — a `compile_result` as pages → lines → elements in
  reading order. Lines are baseline clusters; a cluster whose items are ≤ 0.85×
  the size of a neighbour within 0.75 em attaches to it as a script (compiler
  geometry: superscript raised 0.45 em, subscript lowered 0.2 em, fraction
  numerator/denominator ≈ 0.6 em above/below, all at 0.7× or 0.5× size;
  attachment is transitive so second-order scripts join the root line). Labels:
  `Page 1, line 2: A naïve approach fails.`; elements read as their text,
  `superscript 2`, `subscript i`, `numerator a`, `fraction bar`, `denominator b`
  (a fraction reads numerator → bar → denominator regardless of x). Element
  values give size context (`8.4 point, 70% of the 12 point line`) and whether
  a source range exists / maps onto the current text. Each element carries the
  contract UTF-8 byte range and, when the document text is supplied, the
  UTF-16 `NSRange` (`String.nsRange(utf8Bytes:)`, rebased through
  `SourceMapping` after edits or nil — never mapped onto the wrong text).
  Diagnostics are actionable elements: label `Error: <message>`, value
  `recovery: … ; in page 2 line 3`, action `Go to source` when a source exists.
- `AccessibleEditorModel` — line / word / character navigation over the
  buffer with dual UTF-16/UTF-8 positions. Offsets inside a surrogate pair or a
  multi-byte scalar are rejected (`Range(NSRange, in:)` alone would snap
  them). Tokens are LaTeX-aware (`\section`, words incl. combining marks,
  single symbols such as emoji or `^`, punctuation, whitespace); word
  navigation stops on words, commands and symbols. Descriptions:
  `Line 3 of 4, column 2: x^2 — 1 diagnostic on this line`,
  `word “naïve”, 5 characters`, `command \section`,
  `i̇, LATIN SMALL LETTER I, COMBINING DIAERESIS; 2 scalars, 2 UTF-16 units, 3 UTF-8 bytes`.
  Diagnostics at the caret reuse the shell's `EditorDiagnostics.Mark` shape.
  Rotor categories: headings (`\chapter`…`\paragraph`, level = LaTeX depth),
  environments (`\begin`/`\end` pairs with nesting, unclosed, stray `\end`),
  diagnostics, captures (the pinned insertion anchor). `nextRotorItem` wraps.
- `AccessibilityCommand` / `FocusOrder` — every README shortcut as an entry
  with title, shortcut spelling(s), menu, description and requirement
  (`helpLines` for an "Accessibility help" list; `AccessibilityHelpView`
  renders it but is not yet attached — that needs a menu line in
  `FlashTeXMacApp.swift`, owned elsewhere). Pane focus order
  `Sidebar → Tabs → Editor → Capture bar → Bridge bar → Preview → Problems` with rationale
  (the main window is a `NavigationSplitView`: WorkspaceSidebar.swift,
  ContentView.swift, ProblemsPanel.swift; View > Command Palette… ⌘⇧P lists
  every command of the table, CommandPalette.swift).

## What is attached in the UI

| Hook | Effect for VoiceOver |
|---|---|
| `PreviewView` `PageView.overlay { AccessibilityOverlay(…) }` | Each page is a container labelled `Page N[ of T], k lines`; each item is a static-text element (label = spoken form, value = size/source context, hint = `Page N, line k`) in reading order (`accessibilitySortPriority`), with a `Go to source` custom action that calls the same closure as a mouse click. Frames are measured with the face the preview draws (`PreviewFonts.postScriptName(size:)`: Latin Modern or Times). Hit testing is disabled, so mouse behavior is unchanged. |
| `PreviewV2View` `PageV2View.overlay { PageV2AccessibilityOverlay(…) }` (`PreviewV2Accessibility.swift`) | The shipped v2 pane reads the same shape: each page is a landmark labelled `Page N[ of T], k lines` (an elided page of a windowed frame: `Page N of T, not loaded`) whose **value is the page's text**, one line per line; each line is a static-text element `Page N, line k: text` (value = the line's text, help = `Page N, line k`) with a `Go to source` action that navigates like a click on the line's first word (the clusters' source spans merged into the word). Lines follow the v1 model's script rule (`AccessibleDocumentModel.lines(of:)`): runs cluster by baseline, a smaller cluster within `scriptReachEm` of a larger one is a script of it (superscript, subscript, footnote mark, fraction part) and joins that line, so `$a^2+b_1$` reads "a 2 + b 1" and never "2" before "a"; within a line words read left to right, stacked scripts subscript then superscript ("x i 2"), a fraction numerator then denominator at its bar; beyond v1, full-size display fractions join the line their bar sits on (`\frac{a+b}{c} = d` reads "a + bc = d"). A space is inserted at an inter-word gap (> 0.15 em) or a change of baseline. Pinned over real producer output by `testMathReadsInSpokenOrderOverRealProducerOutput`. Built lazily on first request; afterwards updated in place — zoom moves the elements, a new page token relabels them (rebuilt only when the line count differs), a page-count change touches only the landmark's live label — so VoiceOver's cursor on a line survives typing and zooming; `layoutChanged` is posted only when elements actually changed. |
| `PreviewAnchorProbe.pagesRotor` (`PreviewPagesRotor.swift`), offered by every page view and line element | A custom **Pages** rotor listing every page of the document from the anchor probe's layout — `Page 3 of 12`, `Page 7 of 12, not loaded` for an elided page — because the v2 pane's lazy stack builds only the pages in view and the Landmarks rotor sees only those. Built pages are rotor targets themselves; the rest are `itemLoadingToken` results (VoiceOver lists them by label without touching the pane) and choosing one calls the rotor's `itemLoadingDelegate` (the same object) with `accessibilityElement(withToken:)`, which scrolls there without animation through the Page Up/Down path (announcing the landing), lays out, and returns the page's view; should the lazy stack build it only on a later turn, a stand-in landmark at the page's place is returned and a `layoutChanged` naming the real view is posted when it appears, so VoiceOver moves onto it; a mounted placeholder of an elided page is listed as a loading token too and stays pending until the page is resident. The load speaks no landing (VoiceOver reads the element it is handed), and a pending load is dropped when the reader scrolls, steps, follows the caret or activates a link, so a late appearance never pulls the cursor. The full transition tables are in the two source files. Search semantics as `EditorRotorSearch`: strictly after/before the current page, type-ahead on the label, no wrap. |
| `PreviewPane` (`ContentView.swift`) `.accessibilityElement(children: .contain)` | The pane is the `PDF preview` group; its value is the page under the top of the view (`Page 2 of 5`), so landing on it says where the reader is even while the HUD's readout is faded (and so hidden). Both panes are `.focusable()`; with focus, **Page Down / Page Up step whole pages** (`PreviewPageStep`, via the anchor probe) and announce `Page n of m`. |
| `ShellModel.previewAnnouncer` (`PreviewAnnouncements.swift`) | A completed compile is announced: `Preview updated: 3 pages` (low priority), `Preview updated with 2 errors recovered: 3 pages`, `Compile failed: 2 errors` (high). The first result speaks at once; every later result — status flips included, since typing through a brace alternates failed/ok — coalesces to one announcement of the newest state 2 s after the last result (a failure at high priority). A refused display list with nothing verified on screen says `Preview display list refused: …` (high) once per distinct error — auto-compile retries the same one on every keystroke — until a frame verifies; a project reset forgets everything spoken so the new project's first result is spoken at once; a refused LIVE sibling that keeps the previous frame says `Preview not updated: …` (low) once per distinct error, again only after a frame verified in between, and withdraws the pending `Preview updated` of the compile result it arrived with (the pages did not change). On the default live v2 route the reply honours `display-list-v2-only` and its v1 `pages` are empty by design, so such a result is not spoken until its v2 frame verifies and supplies the document's page count (`noteFrame`); a v1-route result counts its own pages. |
| `ContentView` diagnostics row `.accessibleDiagnostic(d, index:, total:)` | The row is one element: `Diagnostic 1 of 2: Error: Missing } inserted for \textbf.`, value `recovery: …`, custom action `Go to source` (or hint `No source mapping; listed only.`). |
| `ContentView` capture bar `.accessibleCaptureBar(anchor:, proposals:)` | Group `Capture bar` with value `Insertion point pinned: a1 at main.tex byte 66, revision 3; 1 proposal to review` (or `No insertion point pinned; 0 proposals to review`); the buttons inside stay reachable. |
| `SourceEditorView` `tv.setAccessibilityLabel("LaTeX source")` | The `NSTextView` is announced as `LaTeX source, text area`. Native VoiceOver text navigation (VO-arrows, line/word/character) is AppKit's. |

## What can be verified here, and what needs a human

Verified by unit tests (`swift test`, target `FlashTeXAccessibilityTests`,
23 tests; README shortcut parity fails the suite when a shortcut is added to
the README without a command entry — it caught `⌘⇧N` on merge): reading order across the multipage sample's pages and lines, the
exact label/value/action strings above, UTF-8↔UTF-16 offsets on non-ASCII
(`naïve`, `Résumé`, decomposed `ï`, emoji), rebase/refusal after edits,
script/fraction/second-order grouping on a synthetic math line, heading-vs-body
separation, editor line/word/character navigation with combining marks and
emoji, diagnostics at caret, rotor categories and wrapping, README shortcut
table parity (parsed from `README.md` in the test), determinism, and the
overlay's slot order, priorities and frames (Times-Roman metrics, rule
rectangles, scale).

Not verifiable from this agent's terminal: the app was built (`swift build`,
`xcodebuild -scheme FlashTeXMac -destination 'platform=macOS' build`) but the
accessibility tree of the running app was not inspected. `xcrun
accessibilityinspector` is a GUI (not scriptable), and reading another
process's AX tree (`AXUIElement`, Accessibility Inspector, VoiceOver) requires
the Accessibility permission for the terminal/agent, which is not granted.
The unit tests assert the strings and geometry the modifiers are given; the
following script confirms VoiceOver actually speaks them.

## VoiceOver test script (human)

Setup: `cd apps/mac && swift build && FLASHTEX_REPO=$(git rev-parse
--show-toplevel) .build/debug/FlashTeXMac`, then `File > Open Compile Result
Fixture…` (⌘⇧O) → `apps/mac/Samples/multipage-result.json`. Turn VoiceOver on
(⌘F5). VO = Control-Option (or Caps Lock if configured).

| # | Step | Expected announcement / result |
|---|---|---|
| 1 | VO-Right from the window title until the editor | `LaTeX source, text area`, then the first line `\documentclass{article}` |
| 2 | In the editor, VO-Down twice; VO-Right by word | Line 3 `\section{Introduction}`; words read as AppKit does (`section`, `Introduction`). Native text navigation; no FlashTeX strings here |
| 3 | Place the caret in `oops` (line 8) with arrows | The dotted red underline is under `\textbf{oops`; hovering with the mouse shows the diagnostic tooltip (VoiceOver has no caret-diagnostic announcement yet — see Not done) |
| 4 | VO-Right past the capture bar | `Capture bar, group` then value `No insertion point pinned; 0 proposals to review`; inside: `Pin insertion point, button` |
| 5 | ⌘⌥P, then VO-Left back to the group | Value now `Insertion point pinned: a1 at main.tex byte <n>, revision <r>; 0 proposals to review` |
| 6 | VO-Right into the preview; VO-Shift-Down to interact with the scroll area, then the page | `Page 1, 2 lines, group` (page count is not passed to the overlay yet, so `of 2` is absent) |
| 7 | VO-Right through the page | `Introduction`, value `17 point` (hint `Page 1, line 1`), then `A`, `naïve`, `approach`, `fails.` (each `12 point`) — in that order; no staleness note, since the overlay has no document text |
| 8 | On `naïve`, VO-Command-Space (actions menu) | Menu shows `Go to source`; choosing it selects `naïve` in the editor and the footer reads `Selected main.tex bytes 66..<72 …` |
| 9 | VO-Right to page 2 | `Page 2, 3 lines, group`; items `Method`, `Résumé`, `of the steps.`, `oops` |
| 10 | VO-Right into the diagnostics list | Header `Diagnostics (2) — the preview above is still shown; errors are not hidden`, then `Diagnostic 1 of 2: Error: Missing } inserted for \textbf.` with value `recovery: Closed the group at end of paragraph and rendered its contents in bold.` |
| 11 | On that row, VO-Command-Space → `Go to source` | Editor selects `\textbf{oops` (bytes 157..<169); footer names the range |
| 12 | VO-Right | `Diagnostic 2 of 2: Warning: Overfull \hbox on page 2.`, value `no provisional rendering`, hint `No source mapping; listed only.`; the actions menu has no `Go to source` |
| 13 | Edit line 4 (change `naïve` to `naive`), then step 8 again | With auto-compile off/no worker: the preview item still reads `naïve`; `Go to source` is refused with the footer's `recompile to navigate` note (the overlay never maps onto edited text) |
| 14 | Math (needs the FT-002 compiler, ⌘⇧K then a buffer with `$x^2 + \frac{a}{b}$`) | The line reads `x superscript 2 + numerator a fraction bar denominator b`; the superscript's value is `8.4 point, 70% of the 12 point line` |

Record pass/fail per row plus macOS and VoiceOver versions in
`docs/evidence/` when run.

## Not done

- No live "diagnostic at caret" / "current line" announcement in the editor:
  `AccessibleEditorModel` computes the strings but wiring them to
  `NSAccessibility` notifications on caret change needs a `SourceEditorView`
  coordinator change beyond the hook budget. Rotor categories are likewise
  model-only (no custom VoiceOver rotor yet).
- `AccessibilityHelpView` is not reachable from a menu (needs
  `FlashTeXMacApp.swift`).
- The overlay does not receive the page count or the current document text,
  so page labels omit `of N` and element values say `source not mapped to the
  current text` even when it would map; `AccessibleDocumentModel(result:
  documents:compiledDocuments:)` supports both when a caller passes them.
- Line grouping is heuristic (baseline clusters + size/reach); the compiler
  does not tag items with a line or script level in runtime v1.
