# Appearance overhaul — brief for the UI/UX subagent

Hand this to the agent whole. It supersedes the density and colour sections of
`context/style-guide.md`, which were written before real reference values were
available; everything else in `context/design-principles.md` still stands.

---

## 0. The problem, stated precisely

The app currently reads as a stock Apple utility, not as an IDE. That is **not**
a SwiftUI limitation. It is the predictable result of composing an IDE out of
`List`, `NavigationSplitView`, `Form`, `GroupBox` and default control styles —
components whose entire job is to impose Apple's visual identity.

Your mission: make this app read as a flagship IDE — JetBrains-class — while
staying lightweight, native, and minimal.

**Read `DECISION-swift-vs-alternatives.md` before starting.** It contains the
architectural finding this brief depends on.

---

## 1. Non-negotiable architecture: SwiftUI for pixels, AppKit for structure

This is the central lesson from [CodeEdit](https://github.com/CodeEditApp/CodeEdit),
the 23k-star SwiftUI macOS IDE. Its IDE-critical surfaces are all AppKit. Follow
the same split.

**Must be AppKit (bridged via `NSViewRepresentable` / `NSViewControllerRepresentable`):**

| Surface | Use | Why |
|---|---|---|
| File tree / project navigator | `NSOutlineView` | See §1.1 |
| Search results tree | `NSOutlineView` | same |
| Quick-open / palette results list | `NSTableView` | same |
| Window root + all splits | `NSWindowController` + `NSSplitViewController` | `HSplitView` has no configurability at all (§1.2) |
| Context menus | `NSMenu` + `NSMenuDelegate` | SwiftIU has no `validateMenuItem`, no submenus built at `menuNeedsUpdate`, no ⌥-alternate items |
| Materials | `NSVisualEffectView` | SwiftUI exposes 5 thicknesses; AppKit exposes 14 semantic materials |
| Code editor | adopt a component; do not build | See `DECISION-…` |

**SwiftUI is correct for:** tab bar, status bar, toolbar content, settings,
inspector panes, popovers, empty states, and every small custom control.

### 1.1 Why the tree must be AppKit

- **`List` on macOS is not lazy.** Reproduction at 101 rows shows it calls
  `init` and `body` for *every* element including off-screen ones;
  the same code is lazy on iOS.
  ([Apple forum 704778](https://developer.apple.com/forums/thread/704778))
- **Hierarchical `List`/`OutlineGroup` collapses.** A runnable repro with
  100,000 items reports *"takes minutes(!) for the app to become responsive"*
  on a Mac Studio M2.
  ([lemonmojo/swiftui-hierarchical-list-performance](https://github.com/lemonmojo/swiftui-hierarchical-list-performance))
- **~150,000 rows forced an `NSTableView` rewrite** in Pulse; the author names
  *"SwiftUI's infamous `List` automatic diffs that you still can't disable."*
  ([kean.blog](https://kean.blog/post/not-list))
- `ScrollView` + `LazyVStack` *is* lazy, but gives you nothing else: no
  selection, no arrow keys, no type-select, no cell recycling.

What `NSOutlineView` gives you for free, all of which an IDE navigator needs and
none of which SwiftUI offers: `autosaveExpandedItems` + `autosaveName` for
expansion restore, multiple selection, `doubleAction`, real drag-and-drop with
`setDraggingSourceOperationMask(.move, forLocal: false)`, per-row heights via
`heightOfRowByItem`, tooltips, `scrollRowToVisible`/`rect(ofRow:)` for
reveal-in-navigator, correct VoiceOver rotor semantics, and inline rename via an
embedded editable `NSTextField`.

### 1.2 Why splits must be AppKit

Apple's documentation for `HSplitView` is **one sentence with no discussion
section**. There is no API for minimum or maximum thickness, collapse, holding
priority, programmatic divider position, divider styling, or persistence. A
forum thread asking how to persist divider positions was archived with **zero
replies**.

Use `NSSplitViewController`. For divider styling, subclass `NSSplitView` and
override `dividerColor`, `dividerThickness` and `drawDivider(in:)`. Two quirks
you will hit, both documented in CodeEdit's source:

```swift
// AppKit hides dividers when there's only one item
override func splitView(_ sv: NSSplitView, shouldHideDividerAt i: Int) -> Bool {
    guard items.count > 1 else { return false }
    return super.splitView(sv, shouldHideDividerAt: i)
}
```

and divider position must be set **twice** on a vertical split for it to take.

Also use `NSTrackingSeparatorToolbarItem(identifier:splitView:dividerIndex:)` —
this is the API that makes the toolbar read as unified with the sidebar.

---

## 2. The stock-look kill list

Every API below, with the macOS version it requires and what it does *not* fix.

| API | Min macOS | Does | Does NOT |
|---|---|---|---|
| `.listStyle(.plain)` | 10.15 | Drops inset card, sidebar vibrancy, alternating rows | Row insets, selection highlight, row-height minimums, scroll background |
| `.scrollContentBackground(.hidden)` | **13.0** | Hides `List`/`Form`/`TextEditor` background so yours shows | Selection, separators, insets, the enclosing `NSScrollView` |
| `.buttonStyle(.plain)` | 10.15 | Removes bezel and accent fill | Keeps pressed-state dimming and focus ring — **it is not a null style** |
| custom `ButtonStyle` | 10.15 | Full control | — |
| `.focusEffectDisabled()` | **14.0** | Kills focus ring / hover effect for the subtree | Focusability; AppKit-hosted subviews |
| `.alternatingRowBackgrounds(.disabled)` | **14.0** | Kills zebra striping | Explicitly no effect on `.sidebar` list style |
| `.labelsHidden()` | 10.15 | Hides control labels, keeps VoiceOver | Does not reclaim `Form`'s label gutter |
| `.toolbarBackgroundVisibility(.hidden, for: .windowToolbar)` | **15.0** | Window background bleeds into toolbar | Pre-15 use `NSWindow.titlebarAppearsTransparent` |
| `.toolbar(removing: .title)` | **14.0** | Removes SwiftUI's title item | — |
| `.containerBackground(_, for: .window)` | **15.0** | Fills window including under titlebar | — |

**`Form` has no off switch.** `.formStyle(.columns)` gives a plain two-column
layout; `.formStyle(.grouped)` is the System-Settings look. **`GroupBox`:** kill
with a custom `GroupBoxStyle`, or don't use it — CodeEdit uses it zero times.

**The single most useful density lever, and it is easy to miss:**

```swift
.environment(\.defaultMinListRowHeight, 24)
```

Paired with negative padding to claw back `List`'s built-in insets:

```swift
MyRow()
    .listRowSeparator(.hidden)
    .padding(.vertical, -1)
```

**`List` selection styling has no public API.** If a list is small and bounded
(git changes, settings sidebar, open tabs) keep `List` and use the two levers
above. If it is a file tree, use `NSOutlineView`.

**Materials.** SwiftUI's `Material` cannot express what IDE chrome needs. Bridge:

```swift
struct EffectView: NSViewRepresentable {
    let material: NSVisualEffectView.Material   // .headerView for tab bars/toolbars
    let blendingMode: NSVisualEffectView.BlendingMode
    func makeNSView(context: Context) -> NSVisualEffectView {
        let v = NSVisualEffectView()
        v.material = material
        v.blendingMode = blendingMode
        v.state = .followsWindowActiveState   // ← without this, panels stay vibrant
        return v                               //   when the window loses key. #1 tell
    }                                          //   of a hand-rolled panel.
    func updateNSView(_ v: NSVisualEffectView, context: Context) {}
}
```

**Do not use `NSSplitViewItem(sidebarWithViewController:)`.** Apple's docs state
it applies a translucent material background and forces `canCollapse` and
`isSpringLoaded` to `true`. For a VS Code/JetBrains flat opaque sidebar, use
plain `NSSplitViewItem(viewController:)` and paint your own background.

---

## 3. Palette — real values, not invented ones

Primary target is **JetBrains Islands** (the current default JetBrains look),
pulled from `platform/platform-resources/src/themes/islands/` in
`JetBrains/intellij-community`. Both appearances are required; the app follows
the system setting.

### Islands Dark

| Role | Hex |
|---|---|
| Editor background | `#191A1C` |
| Editor foreground | `#BCBEC4` |
| Current line | `#1F2024` |
| Line numbers | `#4B5059` |
| Window background (outside panels) | `#26282C` |
| Tool window background | `#191A1C` |
| Tool window stripe (icon rail) | `#26282C` |
| Main toolbar | `#26282C` |
| Status bar | `#26282C` |
| Status bar border | transparent |
| Editor tab strip | `#191A1C` |
| Selected tab | `#233558` |
| Inactive-window selected tab | `#26282C` |
| Tab underline | `#2E4D89` |
| Selection | `#2A4371` |
| Hover | `#FFFFFF17` (white at 9%) |
| Borders | `#26282C` |
| Component border | `#40434A` |
| Focus | `#3871E1` |

### Islands Light

| Role | Hex |
|---|---|
| Editor background | `#FFFFFF` |
| Editor foreground | `#080808` |
| Current line | `#F5F8FE` |
| Line numbers | `#AEB3C2` |
| Window background | `#E9EAEE` |
| Tool window background | `#FFFFFF` |
| Tool window stripe | `#E9EAEE` |
| Main toolbar | `#E9EAEE` |
| Status bar | `#E9EAEE` |
| Editor tab strip | `#FFFFFF` |
| Selected tab | `#E3EBFE` |
| Inactive-window selected tab | `#E9EAEE` |
| Tab underline | `#A7C5FF` |
| Selection | `#D0DFFE` |
| Hover | `#00000012` (black at 7%) |
| Borders | `#E9EAEE` |
| Component border | `#D1D3D9` |
| Focus | `#3871E1` |

### Alternate target — VS Code Default Dark/Light Modern

From `microsoft/vscode` tag `1.108.0`,
`extensions/theme-defaults/themes/{dark,light}_modern.json`. Use only if the
JetBrains direction is later abandoned; **do not mix the two**.

| Role | Dark Modern | Light Modern |
|---|---|---|
| editor.background | `#1F1F1F` | `#FFFFFF` |
| editor.foreground | `#CCCCCC` | `#3B3B3B` |
| sideBar.background | `#181818` | `#F8F8F8` |
| sideBar.border | `#2B2B2B` | `#E5E5E5` |
| activityBar.background | `#181818` | `#F8F8F8` |
| activityBar.activeBorder | `#0078D4` | `#005FB8` |
| tab.activeBackground | `#1F1F1F` | `#FFFFFF` |
| tab.inactiveBackground | `#181818` | `#F8F8F8` |
| tab.activeForeground | `#FFFFFF` | `#3B3B3B` |
| tab.inactiveForeground | `#9D9D9D` | `#868686` |
| tab.activeBorderTop | `#0078D4` | `#005FB8` |
| titleBar.activeBackground | `#181818` | `#F8F8F8` |
| statusBar.background | `#181818` | `#F8F8F8` |
| panel.background | `#181818` | `#F8F8F8` |
| list.hoverBackground | `#2A2D2E` | `#F2F2F2` |
| list.activeSelectionBackground | `#04395E` | `#E8E8E8` |
| list.inactiveSelectionBackground | `#37373D` | `#E4E6F1` |
| focusBorder | `#0078D4` | `#005FB8` |
| input.background | `#313131` | `#FFFFFF` |
| editorLineNumber.foreground | `#6E7681` | `#6E7681` |
| editorLineNumber.activeForeground | `#CCCCCC` | `#171184` |

### Severity colours (VS Code registry, both themes)

| Role | Dark | Light |
|---|---|---|
| Error | `#F14C4C` | `#E51400` |
| Warning | `#CCA700` | `#BF8803` |
| Info | `#59A4F9` | `#0063D3` |

**Rule:** every one of these becomes a named token in `DesignSystem.swift` with
explicit light and dark values. No hex at a call site. Selection may use the
system accent colour instead of the fixed focus blue — that is a legitimate
native concession and the only one permitted in this palette.

---

## 4. Metrics — real values

Verified from source. Where the two IDEs disagree, the chosen value and its
provenance are stated.

| Element | Value | Provenance |
|---|---|---|
| Tree / list row height | **24pt** | JetBrains New UI + Islands `Tree.rowHeight`, `List.rowHeight` |
| Tree indent per level | **8pt** | VS Code `workbench.tree.indent` default |
| Editor tab height | **35pt** | VS Code `EDITOR_TAB_HEIGHT.normal = 35`. JetBrains' is derived, not a constant; 35 is the verified number |
| Tab label size | **13pt** | VS Code `editortabscontrol.css` |
| Tab underline thickness | **4pt**, corner arc 4 | JetBrains `EditorTabs.underlineHeight` / `underlineArc` |
| Icon rail width | **48pt** | VS Code activity bar |
| Status bar height | **22pt**, text **12pt** | VS Code `statusbarpart.css` |
| Custom title bar height | **35pt** | VS Code `DEFAULT_CUSTOM_TITLEBAR_HEIGHT` |
| Tool window header | **41pt** | JetBrains `JBUI.ToolWindow.defaultHeaderHeight()` |
| Sidebar default width | **300pt**, clamped to `min(300, windowWidth / 4)` | VS Code `layout.ts` |
| Sidebar minimum width | **170pt** | VS Code `sidebarPart.ts` |
| Bottom panel default height | **300pt** | VS Code `layout.ts` |
| Control corner radius | **8pt**; compact **6pt** | JetBrains `Component.arc` / `Button.arc` |
| Button minimum size | **72 × 28pt** | JetBrains theme JSON |
| Text field / combo minimum | **49 × 28pt** | JetBrains theme JSON |
| Default window size | **1440 × 900** | VS Code `DEFAULT_WORKSPACE_WINDOW_SIZE` |

**These supersede the row heights and radii in `context/style-guide.md`** — that
file's 22/24/28/30 values were estimates made before these numbers were in hand.
Keep its spacing scale (`2 4 6 8 12 16 24 32`) and its motion tokens.

---

## 5. Typography

| Role | Value | Provenance |
|---|---|---|
| UI font | System font (SF), **13pt** | VS Code uses `-apple-system` at 13px on macOS. JetBrains uses Inter 13 but only on their own JVM runtime |
| Secondary / dimmed | **11pt** | — |
| Editor font | **JetBrains Mono, 13pt**, line spacing **1.2** (→ 15.6pt) | JetBrains `FontPreferences` |
| Editor fallback | SF Mono → Menlo | VS Code macOS editor default is Menlo 12px |

**Do not switch the UI font away from SF.** Reasons, in descending order of
real pain: you lose SF's optical sizing and text/display crossover; you lose the
semantic scale (`.body`, `.headline`) and must re-plumb every AppKit bridge
(`NSToolbarItem` labels, `NSMenu`, `NSTableCellView` all stay SF regardless);
**SF Symbols stop aligning**, because they are metrically matched to SF baseline
and cap height; accessibility text sizes stop scaling. CodeEdit — the closest
thing to a successful SwiftUI IDE — **bundles no fonts at all** and uses the
system font for UI.

**Bundling JetBrains Mono for the editor** (both JetBrains Mono and Inter are
SIL OFL 1.1, free to bundle commercially, and neither declares a Reserved Font
Name):

1. Fonts in a folder with target membership **unchecked**.
2. Copy Files build phase → Destination `Resources`, Subpath `Fonts`.
3. Info.plist: `ATSApplicationFontsPath` = `Fonts` (macOS uses this, **not**
   `UIAppFonts`).
4. Reference by **PostScript name**: `Font.custom("JetBrainsMono-Regular", size: 13)`.

For SPM resource bundles use `CTFontManagerRegisterFontsForURLs(urls, .process, nil)`
— `.process` scope, never `.persistent`.

---

## 6. Window and title bar

The IDE look requires a custom title bar. Baseline:

```swift
window.styleMask.insert(.fullSizeContentView)
window.titlebarAppearsTransparent = true
window.titleVisibility = .hidden
window.titlebarSeparatorStyle = .none   // or .line only over the editor
toolbar.showsBaselineSeparator = false
```

Apple's own note: `titlebarAppearsTransparent` "only makes sense" together with
full-size content view. Set `titlebarSeparatorStyle` per `NSSplitViewItem` so
the hairline appears over the editor but not over the sidebar — that is how
CodeEdit gets its unified look.

**Budget for the maintenance, because it is real.** Ghostty — the most complete
public implementation of custom macOS window chrome — needs:

- `NSWindow.performDrag(with:)` in a dedicated drag strip, because full-size
  content view destroys the drag region
- a re-apply hook on `title` assignment, because macOS 15+ **re-reveals the
  native title view** when the title is set
- a re-apply hook on exiting fullscreen, because macOS resets the style
- `tabbingMode = .disallowed` if you hide the titlebar, since native tabs live there
- **separate code paths for macOS 13-15, 26, and 27** — AppKit keeps moving
  views into and out of the titlebar

Do not chase this to the Ghostty level. **Transparent titlebar + hidden title +
unified toolbar is enough for the IDE look**; hiding the traffic lights is not
worth what it costs.

---

## 7. Escape hatches — in order of desperation

1. `.background(EffectView(…))` / `.background(Color(token))` — covers most cases.
2. **SwiftUI-Introspect** — reach the backing AppKit view. CodeEdit uses it
   twice, both surgical. Note it is version-gated and goes stale.
3. **Swizzling** — CodeEdit swizzles `NSMenuItem` and `NSSplitViewItem` at
   launch. This is the level of hostility sometimes required. Use only with a
   comment explaining why.
4. **Do not** override an AppKit method in an extension. CodeEdit has a legacy
   `extension NSTableView { override open func viewDidMoveToWindow() }` that
   makes *every* table view in the process transparent, including ones AppKit
   creates internally. It predates `.scrollContentBackground(.hidden)` and is
   formally undefined behaviour. You will find it if you read that repo. Do not
   copy it.

---

## 8. Build order

Stop at each boundary, render, and look at the result.

1. `DesignSystem.swift` — every token from §3, §4, §5, both appearances. Refactor
   existing views onto it. **No visual change yet; this step is plumbing.**
2. Window shell — `NSWindowController` + `NSSplitViewController`, custom title
   bar, `NSTrackingSeparatorToolbarItem`, status bar.
3. File tree — `NSOutlineView` bridge, 24pt rows, colour-coded file-type icons,
   expansion autosave, `NSMenu` context menu, inline rename.
4. Editor tab strip — SwiftUI, 35pt, 4pt underline on the active tab.
5. Preview pane — `PDFView`, flat ground per `context/design-principles.md` §8.
6. Problems panel, gutter fix-it markers.
7. Outline, command palette (`NSTableView` results), completion popup.
8. Find/replace, settings.
9. Empty states, every interaction state, full light/dark pass.

---

## 9. Verification — non-negotiable

After **every** visual change: render, open the image, compare against
`references/` and against the values in §3 and §4. Name specifically what is
wrong. "Close enough" is not a finding.

Check both appearances before moving on. Three attempts without improvement →
stop and report rather than thrashing.

---

## 10. Acceptance criteria

Additional to those in `.claude/agents/ui-ux-implementer.md`.

**Architecture**
- [ ] File tree, search results and palette results are AppKit — no `List` for any of them
- [ ] Window root is `NSWindowController` + `NSSplitViewController`; zero uses of `HSplitView`/`VSplitView`
- [ ] Context menus are `NSMenu` with a delegate, not `.contextMenu`
- [ ] Materials go through `NSVisualEffectView` with `state = .followsWindowActiveState`
- [ ] `NSSplitViewItem(sidebarWithViewController:)` is **not** used

**Stock-look elimination**
- [ ] Zero `Form` and zero `GroupBox` outside Settings
- [ ] Every `Button`/`Toggle` uses a custom style — no default bezels anywhere
- [ ] No system material visible in the editor, sidebar, tab strip or status bar
- [ ] No `NavigationSplitView` in the main window
- [ ] Focus rings suppressed on custom controls (`.focusEffectDisabled()`, 14+)

**Fidelity**
- [ ] Every colour in §3 present as a token with explicit light and dark values
- [ ] Tree rows 24pt, tabs 35pt with a 4pt underline, status bar 22pt, icon rail 48pt
- [ ] Editor uses JetBrains Mono 13pt at 1.2 line spacing; UI stays system font
- [ ] Title bar transparent with hidden title; no stray hairline over the sidebar
- [ ] Window opens at 1440 × 900; sidebar defaults to `min(300, width/4)`, floors at 170

**Proof**
- [ ] Side-by-side screenshot against the matching `references/` image, both appearances
- [ ] File tree scrolled through 5,000+ files without stutter
- [ ] `swift build` passes; snapshot tests pass; no behaviour changes; no new dependencies
