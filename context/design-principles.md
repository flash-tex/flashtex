# Design principles — native macOS LaTeX IDE

This document is the authority for all visual and interaction decisions in this
app. It replaces the example `design-principles.md` from the upstream
design-review workflow.

**Precedence:** where the `frontend-design` skill's advice about developing an
original, distinctive aesthetic conflicts with this document, **this document
wins**. This app is deliberately built to resemble a specific family of
references. Do not invent a new visual identity for it.

Reference screenshots live in `references/`. **Open them.** Comparing your own
render against an actual reference beats reasoning from prose.

---

## 1. Core direction

**Modern IntelliJ IDEA structure, executed with Xcode's restraint, built as a
native Mac app.**

Three loyalties, in priority order when they conflict:

1. **macOS conventions win on mechanics.** Window vs sheet, live-apply settings,
   standard shortcuts, menu bar placement, focus rings, full keyboard access, SF
   Symbols, system accent colour. If IntelliJ does something that would feel
   wrong on a Mac, do it the Mac way.
2. **IntelliJ wins on structure.** Where panels live, what a row contains, how
   the palette is organised, how diagnostics are grouped.
3. **Xcode wins on restraint.** If an element's right to exist is arguable, it
   loses.

Target: **a cleaner, more minimal modern IntelliJ.** Every JetBrains-derived
surface should land about one notch calmer than JetBrains ships it.

## 2. Explicitly do not copy

- **Pre-Islands JetBrains.** The reference is the Islands-era IDE (late 2025
  onward). No 2019-era grey toolbars, no gradient trapezoid tabs, no twelve-icon
  strips.
- **IntelliJ's settings modality.** No OK/Cancel/Apply. macOS applies live.
- **IntelliJ's toolbar density.** It carries five-plus chips in the title bar.
  Carry fewer.
- **Electron-editor idioms.** No web-style focus outlines, no CSS-ish shadows,
  no custom scrollbars, no non-native menus.
- **Overleaf's visual design.** Its *workflow* is the reference; its green
  buttons and its error-card stack in the preview column are not.
- **Xcode Canvas's emptiness applied literally.** Canvas can be bare because a
  SwiftUI preview has nothing to navigate. A 40-page PDF does.

## 3. Reference map

| File | Source | Borrow |
|---|---|---|
| `E1_intellij.jpg` | IntelliJ Islands | Tab shape, active-state treatment, shell proportions |
| `E3_xcode.jpg` | Xcode 26 | Restraint level, native window chrome |
| `G1_xcode_canvas.jpg` | Xcode Canvas | Preview treatment: flat neutral ground, floating page, near-zero chrome |
| `G2_overleaf.jpg` | Overleaf | Two-pane source↔PDF workflow and pane roles (not styling) |
| `G3_typst.jpg` | Typst | Live output with no compile button, calm preview ground |
| `D1_intellij_structure.jpg` | IntelliJ | Outline row anatomy, typed icons, in-header filter menu |
| `F1_intellij_problems.jpg` | IntelliJ | Problems grouping, severity vocabulary, dimmed trailing line numbers |
| `F3_overleaf.jpg` | Overleaf | Error content model: human explanation first, raw log second |
| `H1_intellij_searcheverywhere.jpg` | IntelliJ | Palette: scope tabs, dense rows, live preview, quiet footer |
| `I3_intellij_layout.jpg` | IntelliJ | Tool-window system: icon stripe, stacked panels, tabbed bottom panel |
| `K1_intellij_completion.jpg` | IntelliJ | Completion popup anatomy, footer hint row |
| `L1_intellij_replace.jpg` | IntelliJ | Find/replace bar layout and toggle treatment |
| `M1_intellij_settings.jpg` | IntelliJ | Settings IA — search + tree + detail, minus the OK/Cancel footer |

---

## 4. Window shell

```
┌──────────────────────────────────────────────────────────────┐
│ title bar: ≤3 chips + window controls + right-side toggles   │
├──┬────────────────────┬──────────────────┬───────────────────┤
│  │ Project (file tree)│                  │                   │
│ic│                    │   Source editor  │  PDF preview      │
│on│ ──── stacked ───   │   (tab bar top)  │  (Canvas-clean)   │
│ra│ Outline            │                  │                   │
│il│ (hidden by default)│                  │                   │
│  ├────────────────────┴──────────────────┴───────────────────┤
│  │ Bottom panel: Problems / Log / Terminal (tabbed)          │
├──┴───────────────────────────────────────────────────────────┤
│ status bar: LaTeX-semantic breadcrumb ......... state badges │
└──────────────────────────────────────────────────────────────┘
```

- Tool-window system is IntelliJ's: left icon rail toggles tool windows, tool
  windows stack sharing a column, bottom panel carries its own tabs.
- **Panel framing is flat.** Regions are separated by hairlines and at most a
  slight tonal shift — *not* by giving each region its own distinct background.
  The window must read as one surface. (Tabs still get the rounded Islands
  treatment; see §5.)
- Source↔preview is an Overleaf-style split rendered with Canvas cleanliness.
- Splits drag, respect minimums, and persist per project.
- Usable from ~900pt wide. Below the point where three columns fit, collapse the
  preview to a toggle rather than crushing it.

## 5. Tabs and navigation

- **Active tab:** filled, rounded, visually raised against a recessed strip.
  Unmistakable at a glance — that is the whole reason this style was chosen.
- **Inactive tabs:** file-type icon + name, no chrome whatsoever.
- Close affordance on hover and on the active tab only. Modified state is a
  filled dot, not an asterisk.
- **No breadcrumb row under the tabs.** The editor's top edge is the tab strip.
- **Breadcrumb lives in the status bar** and is **LaTeX-semantic**:
  `main.tex › Chapter 2 › 2.3 Experimental Setup › fig:results`.
  Segments are clickable, but discoverability is not assumed — the palette and
  outline are the real navigation.

## 6. File tree

JetBrains behaviour, one notch calmer: compact hierarchical rows, clear indent
guides, **colour-coded file-type icons** (`.tex`, `.bib`, `.cls`/`.sty`, `.pdf`,
images, generated artefacts all distinct), strong hover and selection with
**full-row highlight** (global rule for trees and lists), animated chevrons with
keyboard left/right and option-click to expand a subtree. Generated files
(`.aux`, `.log`, `.out`, `.synctex.gz`) are de-emphasised or grouped.

## 7. Outline

IntelliJ Structure-panel look — typed icons, dense rows, filter/sort controls in
the panel header. **Simpler than IntelliJ's**: a `.tex` file has far fewer item
kinds than a Java class, so do not build a settings-heavy panel. Item types:
sections, figures, tables, equations, labels, citations, TODOs — distinguished
by icon, with plain-text labels. **Ships collapsed.** Preserve document order;
never group by type, because for prose the order is the meaning.

## 8. Preview pane

- Xcode Canvas treatment: flat neutral ground, page floating with a soft shadow,
  no heavy frame, no border chrome.
- **Live compile.** No Recompile button. Compile activity is a quiet indicator
  that never reflows layout.
- Continuous vertical scroll.
- **Controls in two tiers:** always visible and dimmed — page `N / M` and zoom
  `%`. On pointer-over — zoom −/+, fit-width, fit-page, export.
- **No error cards in this column, ever.** Diagnostics live in the bottom panel.

## 9. Diagnostics

- Primary home is the **bottom Problems panel**, IntelliJ-style: grouped by file,
  severity icon, message, dimmed trailing line number.
- **Three distinct severity glyphs**, never colour alone.
- **The message is the human-readable translation.** TeX's literal string is
  secondary text, not the headline.
- **Raw log collapses under each entry** — a disclosure inside the selected
  problem, not a separate destination.
- **Fix-its** apply by pressing **Tab with the caret on the offending line**, or
  via a button in the panel row. Because the list is in the bottom panel and the
  action happens at the caret, the editor **must** carry a gutter marker on lines
  with an available fix plus a quiet inline hint on the active line — otherwise
  the Tab affordance is invisible.
- Error count is a status-bar badge in IntelliJ's icon vocabulary, not a
  sentence.

## 10. Command palette

IntelliJ Search Everywhere: visible scope tab strip (All / Files / Sections /
Labels / Citations / Commands / Text), dense rows with type icon + bold matched
substring + dimmed context, **live preview of the selected result**, quiet
footer, full-width selection band.

## 11. Completion popup

Rounded floating panel anchored under the caret. Row anatomy: **typed icon →
monospace name → right-aligned dimmed metadata**, and use that right slot well —
defining package for a command, the target's heading text for `\ref{}`, author
and year for `\cite{}`. Full-width selection band. Keyboard hints in the popup
footer. **Documentation in an adjacent side panel**, not a footer, so package
docs cannot inflate the popup.

## 12. Find / replace

IntelliJ's in-editor bar taken as-is: two stacked fields with inline clear and
history, `Aa` / `W` / `.*` toggles that highlight with the accent colour when
active, `7/9` match counter, prev/next, overflow, close, and Replace / Replace
All / Exclude grouped right. ⌘F ⌘⌥F ⌘G ⌘⇧G behave exactly as Mac users expect.

## 13. Settings

IntelliJ's IA — search field, shallow section tree, detail pane. **macOS
behaviour: applies live, no OK / Cancel / Apply.** ⌘, opens it. A window, not a
sheet.

## 14. Interaction states

Every interactive element needs rest, hover, pressed, focused, disabled, and
where applicable selected-but-unfocused. **Unfocused selection must be visibly
weaker than focused selection** — a macOS convention that is frequently missed.

Motion is short and functional: 150–200ms ease-out for disclosure, panel reveal,
popup appearance. Nothing bounces. Nothing resizes on hover. Hover-revealed
controls fade in without shifting layout. Respect Reduce Motion. Scrolling is
native — do not build custom scrollbars. Context menus are real `NSMenu`s.

## 15. Keyboard

Every action reachable from the palette. Standard macOS shortcuts are sacred:
⌘S ⌘W ⌘F ⌘⇧F ⌘, ⌘N ⌘O ⌘Z ⌘⇧Z ⌘+ ⌘- ⌘0. Full keyboard access works, focus ring
visible and follows the system setting. Escape always dismisses and returns
focus to the editor. Tab applies an available fix (§9); everywhere else Tab does
what a text editor's Tab does — do not break this.

## 16. Accessibility

- Contrast ≥ 4.5:1 body text, ≥ 3:1 large text and meaningful glyphs, in **both**
  appearances.
- Never encode meaning in colour alone — severity, modified state and file type
  each need a shape or glyph difference.
- Accessibility label on every control; help text on icon-only buttons.
- Support Increase Contrast, Reduce Transparency, Reduce Motion, and Dynamic
  Type where the platform offers it.
- VoiceOver: correct roles and traversal for tool windows, tabs and trees.

## 17. Implementation constraints

- **SwiftUI first.** Drop to AppKit (`NSViewRepresentable`) only where SwiftUI
  genuinely cannot do the job — text editing, PDF view, context menus, custom
  scroll. Comment why.
- SF Symbols wherever one fits; custom glyphs only for LaTeX-specific concepts
  with no symbol.
- **No new dependencies.**
- **No behaviour changes.** Same actions, same shortcuts, same data flow. If a
  visual change appears to require a behaviour change, stop and report.
- Do not modify models, persistence, the LaTeX engine bridge, or files matching
  `*Service.swift` / `*Engine*.swift` without asking.
- `swift build` and the snapshot tests must pass. One surface per commit.
