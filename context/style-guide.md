# Style guide — tokens

> **How to read this document — the owner's ruling, which overrides the
> wording below.**
>
> This guide was written by an agent with limited context on the project. Its
> *numbers* are a considered starting point, not law. Take every "hard rule"
> here with a grain of salt: the spacing scale is the owner's own example —
> `2, 4, 6, 8, 12, 16, 24, 32` is a reasonable scale, not a sacred one, and the
> same goes for the row heights, radii, type sizes and durations.
>
> **What is actually binding is the target**, surface by surface: the file tree
> should read like IntelliJ's, the completion popup like IntelliJ's/Xcode's, and
> so on. `references/` is the authority for that, and `design-principles.md`
> carries the intent. Match the target using judgement; where a number here
> fights what the reference or the real app actually needs, trust your eyes and
> the reference, change the number, and say in your report what you changed and
> why.
>
> **One thing here is not a matter of taste and does stand:** values belong in
> `DesignSystem.swift` as named tokens rather than as literals scattered across
> view files. That is maintainability, not aesthetics — it is what makes "make
> the tree denser" one edit instead of forty that drift apart. Keep the
> discipline; revise the values freely.

Everything visual comes from `DesignSystem.swift`. **No raw numbers or colours
at a call site, ever.** If you need a value that isn't here, add a named token
rather than a literal.

Build this file and refactor existing views onto it **before** any other visual
work. Once tokens exist, "make the tree denser" is one line instead of forty
scattered edits that drift apart, and light/dark correctness becomes structural
rather than something you have to remember at every call site.

## Spacing scale

`2, 4, 6, 8, 12, 16, 24, 32` — nothing between.
Name them `.xxs .xs .s .m .l .xl .xxl .xxxl`.

## Corner radii

| Token | Value | Applies to |
|---|---|---|
| `radiusControl` | 4 | small controls, toggles, badges |
| `radiusTab` | 6 | tabs, selection pills, chips |
| `radiusPanel` | 8 | popovers, completion popup, floating panels |
| `radiusSheet` | 10 | sheets and modal surfaces |

## Row heights

IntelliJ's density relaxed by one step — not Xcode-roomy, not IntelliJ-tight.

| Token | Value |
|---|---|
| `rowTree` / `rowOutline` / `rowProblem` | 24 |
| `rowCompletion` | 22 |
| `rowPaletteResult` | 28 |
| `heightTab` | 30 |
| `heightStatusBar` | 24 |
| toolbar | system default |

## Typography

- UI: system font (SF Pro Text). Base **13**, secondary/dimmed **11**, section
  headers **11 semibold**.
- Editor, logs, any monospace content: SF Mono, **13** default, user-adjustable.
- **Never more than three type sizes in one surface.**
- Hierarchy comes from colour first, then weight, then size — in that order.

## Colour

Follow the system appearance. `.light`, `.dark` and follow-system must all be
correct. **No hardcoded default theme.**

Policy (a deliberate departure from Xcode, chosen explicitly):

- **Chrome may be coloured.** File-type icons, folder icons and tab icons are
  colour-coded in the IntelliJ manner.
- Structural surfaces stay neutral.
- Colour means **identity** (file type) or **state** (error / warning / success /
  modified). Never decoration.
- Selection uses the **system accent colour** and must honour the user's accent
  setting. No hardcoded blue.
- Every semantic colour is defined once with explicit light and dark values.

Minimum set to define:

```
surfacePrimary      surfaceSecondary    surfaceRaised
separator
textPrimary         textSecondary       textTertiary
accentSelection
severityError       severityWarning     severityInfo    severitySuccess
statusModified      gutterMarker
```

## Motion

| Token | Value |
|---|---|
| `durationQuick` | 150ms |
| `durationStandard` | 200ms |
| curve | ease-out |

Nothing exceeds 200ms. Nothing bounces. All of it disabled under Reduce Motion.

## Chrome budget

- Title bar: **at most three interactive chips at rest**, plus window controls
  and right-side panel toggles.
- Preview pane at rest: **page indicator and zoom % only**.
- Any other control must earn its place or live behind hover or a menu.
