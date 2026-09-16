import SwiftUI

/// The design system (`context/style-guide.md`): every visual constant a view
/// file uses lives here, so "make the tree denser" is one edit and light/dark
/// correctness is structural. View files carry no raw numeric or colour
/// literals; if a value is missing, add a named token here rather than a
/// literal there.
///
/// Colour policy (`context/design-principles.md` §, style guide):
/// - Structural surfaces stay neutral and follow the system appearance.
/// - Colour means *identity* (file type) or *state* (error / warning /
///   success / modified) — never decoration.
/// - Selection uses the system accent and honours the user's accent setting.
enum DS {

    // MARK: spacing — `2 4 6 8 12 16 24 32`, nothing between

    enum Space {
        static let xxs: CGFloat = 2
        static let xs: CGFloat = 4
        static let s: CGFloat = 6
        static let m: CGFloat = 8
        static let l: CGFloat = 12
        static let xl: CGFloat = 16
        static let xxl: CGFloat = 24
        static let xxxl: CGFloat = 32
    }

    // MARK: corner radii

    enum Radius {
        /// Small controls, toggles, badges, key caps.
        static let control: CGFloat = 4
        /// Tabs, selection pills, chips.
        static let tab: CGFloat = 6
        /// Popovers, the completion popup, floating panels.
        static let panel: CGFloat = 8
        /// Sheets and modal surfaces.
        static let sheet: CGFloat = 10
    }

    // MARK: row heights — IntelliJ's density relaxed by one step

    enum Row {
        static let tree: CGFloat = 24
        static let outline: CGFloat = 24
        static let problem: CGFloat = 24
        /// 24, not the guide's 22: candidate rows carry 16pt kind icons, and
        /// 22 crowds them against the row edges (owner: match the surface,
        /// not the number).
        static let completion: CGFloat = 24
        static let paletteResult: CGFloat = 28
        /// Editor tabs: 35 (VS Code `EDITOR_TAB_HEIGHT.normal`; the brief's
        /// verified number — JetBrains' is derived, not a constant).
        static let tab: CGFloat = 35
        /// Status bar: 22 (VS Code `statusbarpart.css`).
        static let statusBar: CGFloat = 22
        /// Tool-window header row (Project / Outline / Problems). JetBrains'
        /// 41 includes a toolbar; ours carries only a label and one control,
        /// so 41 reads empty — 30 is the calm variant (owner: match the
        /// surface, not the number).
        static let toolWindowHeader: CGFloat = 30
    }

    // MARK: typography — base 13, secondary 11, headers 11 semibold; ≤3 sizes per surface

    enum Fonts {
        /// UI base: SF Pro Text 13.
        static let base = Font.system(size: 13)
        /// Secondary / dimmed detail: 11.
        static let secondary = Font.system(size: 11)
        /// Section headers: 11 semibold.
        static let header = Font.system(size: 11, weight: .semibold)
        /// Monospace UI detail (revisions, counters, shortcuts) at the
        /// secondary size. Editor text itself sizes from EditorPreferences.
        static let monoSecondary = Font.system(size: 11, design: .monospaced)
        /// Monospace content outside the editor (log excerpts, LaTeX snippets).
        static let mono = Font.system(size: 13, design: .monospaced)
        /// The palette / search field: one size up from base so the type-here
        /// surface reads as the primary element of its panel.
        static let field = Font.system(size: 15)
        /// Icon-rail glyphs: between VS Code's 24px-in-48 and SF's UI sizes —
        /// 16 with the 48pt rail reads as an activity bar, 13 as a toolbar.
        static let railIcon = Font.system(size: 16)
        /// Title-bar toolbar glyphs (IDEToolbar.swift): one step under the
        /// rail so the title bar reads lighter than the tool stripe, in the
        /// same outline SF style as the rail icons (owner: unify on the
        /// rail's icon look).
        static let toolbarIcon = Font.system(size: 15)
        /// The split button's chevron (its own click region, JetBrains-style).
        static let toolbarChevron = Font.system(size: 9, weight: .semibold)
        /// The pairing code: read across the room, typed on another device.
        static let pairingCode = Font.system(size: 34, weight: .semibold, design: .monospaced)
    }

    // MARK: colour — the JetBrains Islands palette (context/PROMPT-appearance-overhaul.md §3)
    //
    // Every value is a fixed light/dark pair from JetBrains' Islands theme
    // (platform-resources/themes/islands in JetBrains/intellij-community),
    // resolved per drawing appearance, so the app carries an IDE identity
    // instead of the system's utility-app surfaces. The severity colours are
    // VS Code's registry values. The one native concession the brief permits:
    // `accentSelection` stays the system accent colour.

    /// Dynamic appearance-resolved AppKit colours, the single source of truth;
    /// `Colors` wraps them for SwiftUI. Hex is 0xRRGGBB.
    enum Palette {
        static func dynamic(light: UInt32, dark: UInt32,
                            lightAlpha: CGFloat = 1, darkAlpha: CGFloat = 1) -> NSColor {
            NSColor(name: nil) { appearance in
                let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
                let hex = isDark ? dark : light
                return NSColor(srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
                               green: CGFloat((hex >> 8) & 0xFF) / 255,
                               blue: CGFloat(hex & 0xFF) / 255,
                               alpha: isDark ? darkAlpha : lightAlpha)
            }
        }

        // Surfaces.
        /// Editor ground; also the tool windows (tree, outline, problems) and
        /// the editor tab strip — Islands gives all three the same surface.
        static let editorBackground = dynamic(light: 0xFFFFFF, dark: 0x191A1C)
        /// Window chrome outside the panels: toolbar, icon rail, status bar.
        static let windowChrome = dynamic(light: 0xE9EAEE, dark: 0x26282C)
        /// Floating surfaces one step above the panels: completion popup,
        /// palette, popovers (JetBrains popup ground).
        static let raised = dynamic(light: 0xFFFFFF, dark: 0x2B2D30)
        /// The preview column's neutral ground the page floats on.
        static let previewGround = dynamic(light: 0xDFE1E5, dark: 0x1E1F22)

        // Text.
        static let editorForeground = dynamic(light: 0x080808, dark: 0xBCBEC4)
        static let textPrimary = editorForeground
        static let textSecondary = dynamic(light: 0x6C707E, dark: 0x9DA0A8)
        static let textTertiary = dynamic(light: 0xA8ADBD, dark: 0x6F737A)

        // Editor internals.
        static let editorCurrentLine = dynamic(light: 0xF5F8FE, dark: 0x1F2024)
        static let editorSelection = dynamic(light: 0xD0DFFE, dark: 0x2A4371)
        static let editorLineNumber = dynamic(light: 0xAEB3C2, dark: 0x4B5059)
        static let editorLineNumberActive = dynamic(light: 0x6C707E, dark: 0x9DA0A8)

        // Lists and trees. JetBrains keeps the row's own text colour on the
        // muted blue selection band — no white-on-accent macOS band.
        static let selectionFocused = dynamic(light: 0xD0DFFE, dark: 0x2A4371)
        static let selectionUnfocused = dynamic(light: 0xDFE1E5, dark: 0x393B40)
        static let hover = dynamic(light: 0x000000, dark: 0xFFFFFF,
                                   lightAlpha: 0.07, darkAlpha: 0.09)

        // Tabs.
        static let tabSelected = dynamic(light: 0xE3EBFE, dark: 0x233558)
        static let tabSelectedInactive = dynamic(light: 0xE9EAEE, dark: 0x26282C)
        static let tabUnderline = dynamic(light: 0xA7C5FF, dark: 0x2E4D89)

        // Lines.
        /// Panel-to-panel boundary: the chrome tone, so regions separate by
        /// tone rather than by drawn lines (the flat Islands look).
        static let border = dynamic(light: 0xE9EAEE, dark: 0x26282C)
        /// Border of an actual control (fields, popups) — visible on purpose.
        static let componentBorder = dynamic(light: 0xD1D3D9, dark: 0x40434A)
        /// Keyboard-focus ring on custom controls; also the fix-available ring.
        static let focus = dynamic(light: 0x3871E1, dark: 0x3871E1)

        // State (VS Code registry severities; both themes).
        static let severityError = dynamic(light: 0xE51400, dark: 0xF14C4C)
        static let severityWarning = dynamic(light: 0xBF8803, dark: 0xCCA700)
        static let severityInfo = dynamic(light: 0x0063D3, dark: 0x59A4F9)
        static let severitySuccess = dynamic(light: 0x1A7F37, dark: 0x57AB5A)
        /// The modified (unsaved) dot on tabs and tree rows — JetBrains blue,
        /// not a warning colour: an unsaved edit is a state, not a problem.
        static let statusModified = dynamic(light: 0x3574F0, dark: 0x548AF7)
        /// The Compile (run) glyph: JetBrains' run-button green, the one
        /// deliberate accent in the title bar (owner: more accents; the
        /// JetBrains run triangle is the signature one).
        static let runGreen = dynamic(light: 0x1A7F37, dark: 0x5FAD65)
        /// A completed older snapshot shown while a newer revision compiles.
        static let statusHistorical = dynamic(light: 0x834DF0, dark: 0xB189F5)

        // File-type / outline identity (muted JetBrains icon palette).
        static let typeBlue = dynamic(light: 0x3574F0, dark: 0x548AF7)
        static let typeGreen = dynamic(light: 0x208A3C, dark: 0x5FAD65)
        static let typePurple = dynamic(light: 0x834DF0, dark: 0xB189F5)
        static let typeOrange = dynamic(light: 0xE56D17, dark: 0xE08855)
        static let typeTeal = dynamic(light: 0x0E7C8E, dark: 0x24A394)
        static let typeRed = dynamic(light: 0xDB3B4B, dark: 0xE55765)
        static let typeGray = dynamic(light: 0x818594, dark: 0x6F737A)
    }

    enum Colors {
        /// Content ground: editor, lists, trees, tab strip (Islands gives the
        /// editor and the tool windows one surface).
        static let surfacePrimary = Color(nsColor: Palette.editorBackground)
        /// Chrome: toolbar, icon rail, panel headers, status bar.
        static let surfaceSecondary = Color(nsColor: Palette.windowChrome)
        /// Floating surfaces: completion popup, palette, popovers.
        static let surfaceRaised = Color(nsColor: Palette.raised)
        /// The preview column's neutral ground the page floats on.
        static let surfaceGround = Color(nsColor: Palette.previewGround)
        /// QR ground: scanners need literal white behind the code in both
        /// appearances — the one deliberately non-semantic surface.
        static let qrGround = Color.white
        static let separator = Color(nsColor: Palette.border)
        static let componentBorder = Color(nsColor: Palette.componentBorder)

        static let textPrimary = Color(nsColor: Palette.textPrimary)
        static let textSecondary = Color(nsColor: Palette.textSecondary)
        static let textTertiary = Color(nsColor: Palette.textTertiary)

        /// The system accent; honours the user's accent-colour setting (the
        /// one permitted native concession in the Islands palette).
        static let accentSelection = Color(nsColor: .controlAccentColor)
        /// Focused selection band in lists and trees: JetBrains' muted blue,
        /// which keeps the row's own text colour.
        static let selectionFocused = Color(nsColor: Palette.selectionFocused)
        /// Selected-but-unfocused: visibly weaker than focused (macOS convention).
        static let selectionUnfocused = Color(nsColor: Palette.selectionUnfocused)
        /// Hover wash over rows, tabs and icon buttons.
        static let hover = Color(nsColor: Palette.hover)

        /// Editor tabs (Islands): selected fill, its inactive-window fade,
        /// and the 4pt underline.
        static let tabSelected = Color(nsColor: Palette.tabSelected)
        static let tabSelectedInactive = Color(nsColor: Palette.tabSelectedInactive)
        static let tabUnderline = Color(nsColor: Palette.tabUnderline)

        static let severityError = Color(nsColor: Palette.severityError)
        static let severityWarning = Color(nsColor: Palette.severityWarning)
        static let severityInfo = Color(nsColor: Palette.severityInfo)
        static let severitySuccess = Color(nsColor: Palette.severitySuccess)

        /// The modified (unsaved) dot on tabs and tree rows.
        static let statusModified = Color(nsColor: Palette.statusModified)
        /// A completed older snapshot shown while a newer revision compiles.
        static let statusHistorical = Color(nsColor: Palette.statusHistorical)
        /// The Compile button's run-green glyph (IDEToolbar.swift).
        static let runAccent = Color(nsColor: Palette.runGreen)
        /// Gutter marker on lines with an available fix.
        static let gutterMarker = Color(nsColor: Palette.focus)
        /// Keyboard-focus ring on custom controls.
        static let focus = Color(nsColor: Palette.focus)

        /// Outline item-type identity (typed icons, IntelliJ-fashion):
        /// colour tells the kind apart together with the glyph.
        static let typeTable = Color(nsColor: Palette.typeBlue)
        static let typeFloat = Color(nsColor: Palette.typeGreen)
        static let typeMath = Color(nsColor: Palette.typePurple)
        static let typeLabel = Color(nsColor: Palette.typeOrange)
    }

    /// AppKit type for panels the SwiftUI `Fonts` cannot reach (the
    /// completion popup is an NSPanel + NSTableView on purpose).
    enum NSFonts {
        static let base = NSFont.systemFont(ofSize: 13)
        static let baseSemibold = NSFont.systemFont(ofSize: 13, weight: .semibold)
        static let secondary = NSFont.systemFont(ofSize: 11)
        static let header = NSFont.systemFont(ofSize: 11, weight: .semibold)
        /// Completion candidates: the editor's own face (JetBrains Mono when
        /// bundled), one step smaller.
        static var monoCandidate: NSFont {
            if EditorFontRegistration.registerIfNeeded(),
               let font = NSFont(name: EditorFontRegistration.regularPostScriptName, size: 12) { return font }
            return .monospacedSystemFont(ofSize: 12, weight: .medium)
        }
        /// Dimmed trailing detail in AppKit trees (revisions, line numbers).
        static let secondaryMono = NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular)
    }

    /// AppKit paint paths (gutter marks, rulers, the editor itself) use
    /// `NSColor` directly; same semantic mapping as `Colors`, one source of
    /// truth per meaning.
    enum NSColors {
        static let severityError = Palette.severityError
        static let severityWarning = Palette.severityWarning
        static let gapDot = Palette.textTertiary
        static let gutterGlyph = Palette.textSecondary
        static let gutterHairline = Palette.border
        /// Ring around a severity dot whose line carries a Tab-applicable fix.
        static let fixRing = Palette.focus
        /// The editor surface (also painted behind the gutter).
        static let editorBackground = Palette.editorBackground
        static let editorForeground = Palette.editorForeground
        static let editorSelection = Palette.editorSelection
        static let editorCurrentLine = Palette.editorCurrentLine
        static let editorLineNumber = Palette.editorLineNumber
        static let editorLineNumberActive = Palette.editorLineNumberActive
        static let raised = Palette.raised
        static let componentBorder = Palette.componentBorder
        static let windowChrome = Palette.windowChrome
        static let panelBackground = Palette.editorBackground
        static let textSecondary = Palette.textSecondary
        static let selectionFocused = Palette.selectionFocused
        static let selectionUnfocused = Palette.selectionUnfocused
        static let hover = Palette.hover
    }

    // MARK: interaction states

    enum State {
        /// Hover wash over rows, tabs and icon buttons.
        static let hoverOpacity: Double = 0.06
        /// Pressed wash (hover + pressed compose to ~0.14).
        static let pressedOpacity: Double = 0.08
        /// Subtle raised fill: the active tab against its strip, key caps.
        static let raisedFillOpacity: Double = 0.07
        /// Hairline stroke drawn from `textPrimary` (key caps, thumbnails).
        static let hairlineOpacity: Double = 0.15
        /// Selection tint when a band derives from the accent rather than the
        /// semantic selection colours (chips, pills).
        static let selectionTintOpacity: Double = 0.22
        /// De-emphasis for hidden-until-hover controls at rest.
        static let restingControlOpacity: Double = 0.35
        /// Badge fills behind tinted badge text (severity, source chips).
        static let badgeFillOpacity: Double = 0.18
        /// The fill of a default (primary) JetBrains button, and the wash a
        /// secondary one carries at rest.
        static let buttonSecondaryFillOpacity: Double = 0.04
        /// Disabled controls keep their shape and lose their contrast.
        static let disabledOpacity: Double = 0.4
    }

    // MARK: iconography and fixed part sizes

    enum Size {
        /// The modified-state dot (tabs, tree rows).
        static let modifiedDot: CGFloat = 6
        /// Hit target of small inline icon buttons (tab close).
        static let inlineIconButton: CGFloat = 16
        /// Square reserved for inline progress/status glyphs so toggling them
        /// never reflows the row.
        static let inlineStatusSlot: CGFloat = 12
        /// File-type icon column in trees and tabs.
        static let fileIcon: CGFloat = 14
        /// Rail buttons: icon hit target on the tool-window stripe.
        static let railButton: CGFloat = 32
        /// The active tab's underline (JetBrains `EditorTabs.underlineHeight`),
        /// drawn as a rounded bar (`underlineArc`).
        static let tabUnderline: CGFloat = 4
        /// Hairline separators drawn as frames.
        static let hairline: CGFloat = 1
        /// Short vertical divider between inline groups (status bar).
        static let inlineDividerHeight: CGFloat = 12
        /// Small status dot (connection state, capture state).
        static let statusDot: CGFloat = 7
        /// Image thumbnails in capture rows and previews.
        static let thumbnail: CGFloat = 72
        /// Larger imagery: pairing QR, proposal preview images.
        static let imageTile: CGFloat = 120
        /// Minimum width of the zoom percentage readout, so 100% → 1000%
        /// never shifts its neighbours.
        static let zoomReadoutMinWidth: CGFloat = 40
        /// Title-bar buttons (TitleBar.swift): square hit target, sized to
        /// the 28pt band with a hairline of breathing room.
        static let toolbarButton: CGFloat = 24
        /// Text buttons (IDEButtonStyle), measured off the OK / Cancel pair
        /// in `references/M1_intellij_settings.jpg`: a JetBrains button is
        /// taller, softer-cornered and much wider for the same label than
        /// the macOS one — a two-letter label still fills ~110px there. The
        /// style guide's row-height table does not cover buttons; these come
        /// from the reference.
        static let buttonHeight: CGFloat = 28
        static let buttonHorizontalPadding: CGFloat = 18
        static let buttonMinWidth: CGFloat = 76
        /// The split button's chevron click region (visibly its own zone).
        static let toolbarChevronWidth: CGFloat = 18
        /// The vertical separator inside a split button.
        static let toolbarSplitSeparatorHeight: CGFloat = 14
    }

    // MARK: layout — split minimums and panel bounds

    enum Layout {
        /// The left icon rail (tool-window stripe): 48 (VS Code activity bar).
        static let railWidth: CGFloat = 48
        /// The custom title-bar row (TitleBar.swift) is exactly the band
        /// AppKit lays the traffic lights out in for a title bar with no
        /// toolbar, so the row's controls and the lights share one centre
        /// line without moving AppKit's buttons (owner: the icons are not
        /// aligned with each other). Installing an NSToolbar to buy a taller
        /// band is what the previous pass did; it also changes
        /// `contentLayoutRect` asynchronously, which made every whole-window
        /// capture differ run to run.
        static let titleBarHeight: CGFloat = 28
        /// Leading clearance past the traffic lights for the first control.
        static let trafficLightClearance: CGFloat = 78
        /// The window itself: usable from ~900pt wide (design-principles §4).
        static let windowMinWidth: CGFloat = 900
        static let windowMinHeight: CGFloat = 600
        static let editorMinWidth: CGFloat = 340
        static let previewMinWidth: CGFloat = 380
        /// Sidebar floors at 170 (VS Code `sidebarPart.ts`); defaults to
        /// `min(300, windowWidth / 4)` (VS Code `layout.ts`), which the split
        /// controller computes at first layout.
        static let sidebarMinWidth: CGFloat = 170
        static let sidebarDefaultWidth: CGFloat = 300
        static let sidebarIdealWidth: CGFloat = 260
        static let sidebarMaxWidth: CGFloat = 420
        static let inspectorMinWidth: CGFloat = 300
        static let inspectorIdealWidth: CGFloat = 360
        static let inspectorMaxWidth: CGFloat = 560
        /// Problems panel: header plus two rows at least; ideal ~5 grouped rows.
        static let problemsMinHeight: CGFloat = 120
        static let problemsIdealHeight: CGFloat = 260
        /// The Problems panel never takes more than this share of the window …
        static let problemsMaxFraction: CGFloat = 0.4
        /// … and the editor above it keeps at least this much height.
        static let editorMinHeightAbovePanel: CGFloat = 240
        /// Height of the invisible grab strip around the panel resize divider.
        static let resizeHandleHeight: CGFloat = 7
        /// Command palette sheet.
        static let paletteSize = CGSize(width: 620, height: 440)
        /// Modal review/fix sheets.
        static let sheetMinWidth: CGFloat = 480
        static let sheetWidth: CGFloat = 520
        /// Minimum height of an embedded diagnostics list.
        static let diagnosticsListMinHeight: CGFloat = 80
        /// Default max height of the embedded (non-panel) diagnostics list.
        static let diagnosticsListMaxHeight: CGFloat = 180
        /// Minimum height of multiline text editors inside sheets.
        static let sheetTextEditorMinHeight: CGFloat = 140
        /// Popovers hanging off status-bar items.
        static let popoverMinWidth: CGFloat = 260
        static let popoverListMaxHeight: CGFloat = 240
        /// The hover quick-info popover.
        static let quickInfoMinWidth: CGFloat = 180
        static let quickInfoMaxWidth: CGFloat = 380
        /// The completion popup: fixed width, and a fixed-height
        /// documentation pane that package docs can never inflate.
        static let completionWidth: CGFloat = 480
        static let completionDocHeight: CGFloat = 58
        /// Secondary windows and sheets.
        static let sheetNarrowWidth: CGFloat = 440
        static let pickerWindowSize = CGSize(width: 560, height: 400)
        static let historyWindowMinWidth: CGFloat = 360
        static let historyWindowMinHeight: CGFloat = 320
        static let nearbyWindowMinWidth: CGFloat = 460
        static let nearbyWindowIdealWidth: CGFloat = 500
        static let nearbyWindowMinHeight: CGFloat = 560
        static let citationWindowMinWidth: CGFloat = 560
        static let citationWindowMinHeight: CGFloat = 300
        static let sheetListMaxHeight: CGFloat = 200
        static let nearbyEventsMaxHeight: CGFloat = 120
        /// The Settings window's fixed content width.
        static let settingsWidth: CGFloat = 460
        /// Find in Project window.
        static let searchWindowMinWidth: CGFloat = 520
        static let searchWindowMinHeight: CGFloat = 320
        static let searchPreviewMinHeight: CGFloat = 60
        static let searchPreviewMaxHeight: CGFloat = 160
        static let searchPathColumnWidth: CGFloat = 140
        /// The v2 pane's inline diagnostics strip.
        static let v2DiagnosticsMaxHeight: CGFloat = 160
    }

    // MARK: preview page rendering — the Canvas-clean ground and the page

    enum Preview {
        /// The dark-preview ground and page (user toggle, not the system
        /// appearance: the preference draws the *page* dark).
        static let darkGround = Color(white: 0.12)
        static let darkPage = Color(white: 0.16)
        static let darkPageCG = CGColor(gray: 0.16, alpha: 1)
        static let darkInkCG = CGColor(gray: 1, alpha: 1)
        static let lightInkCG = CGColor(gray: 0, alpha: 1)
        static let lightPageCG = CGColor(gray: 1, alpha: 1)
        /// Page-number captions on the two grounds.
        static let darkLabel = Color(white: 0.7)
        static let lightLabel = Color(white: 0.35)
        /// The soft shadow floating the page off the ground.
        static let pageShadowRadius: CGFloat = 4
        /// Shadow under the floating page/zoom HUD chip.
        static let hudShadowOpacity: Double = 0.18
        /// Space around and between pages on the ground.
        static let pageSpacing: CGFloat = 24
        /// Caret / navigation highlights painted over the page (accent-derived).
        static let caretHighlightOpacity: Double = 0.25
        static let occurrenceHighlightOpacity: Double = 0.15
        static let hoverHighlightOpacity: Double = 0.10
        static let linkBoxFillOpacity: Double = 0.12
        static let linkBoxStrokeOpacity: Double = 0.8
    }

    // MARK: motion — nothing exceeds 200 ms, nothing bounces

    enum Motion {
        static let durationQuick: Double = 0.15
        static let durationStandard: Double = 0.20
        /// How long the preview HUD stays after scroll/zoom activity.
        static let hudLinger: Double = 1.5
        /// Disclosure, panel reveal, popup appearance. Nil under Reduce
        /// Motion — pass to `.animation(_:value:)` and the change is instant.
        @MainActor static var quick: Animation? { ReduceMotion.isEnabled ? nil : .easeOut(duration: durationQuick) }
        @MainActor static var standard: Animation? { ReduceMotion.isEnabled ? nil : .easeOut(duration: durationStandard) }
    }
}

// MARK: - Interaction

/// Text buttons in the JetBrains / Android Studio manner (owner on #653:
/// reproduce that design, not Apple's Liquid Glass capsules): a 4pt rounded
/// rectangle with a 1px component border and a barely-there fill, the
/// default action filled in the accent with white text, hover lightening
/// the fill and press darkening it. No bezel, no gloss, no capsule.
///
/// Apply with `.buttonStyle(IDEButtonStyle())`, or `.ideDefault()` /
/// `.ideSecondary()` on a Button. Destructive roles keep the secondary
/// shape and take the error colour for their label, which is what IntelliJ
/// does — a red *fill* would shout louder than the action deserves.
struct IDEButtonStyle: ButtonStyle {
    /// The default action of its surface: accent fill, white label.
    var prominent = false
    var destructive = false
    @Environment(\.isEnabled) private var isEnabled
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        let pressed = configuration.isPressed
        configuration.label
            .font(DS.Fonts.base)
            .foregroundStyle(prominent ? Color.white
                             : destructive ? DS.Colors.severityError : DS.Colors.textPrimary)
            .padding(.horizontal, DS.Size.buttonHorizontalPadding)
            .frame(minWidth: DS.Size.buttonMinWidth, minHeight: DS.Size.buttonHeight)
            .background {
                let shape = RoundedRectangle(cornerRadius: DS.Radius.tab)
                if prominent {
                    shape.fill(DS.Colors.accentSelection)
                        .overlay { if pressed || hovering { shape.fill(.black.opacity(pressed ? DS.State.pressedOpacity : DS.State.hoverOpacity)) } }
                } else {
                    shape.fill(DS.Colors.textPrimary.opacity(
                        pressed ? DS.State.pressedOpacity
                        : hovering ? DS.State.hoverOpacity
                        : DS.State.buttonSecondaryFillOpacity))
                        .overlay { shape.strokeBorder(DS.Colors.componentBorder, lineWidth: DS.Size.hairline) }
                }
            }
            .opacity(isEnabled ? 1 : DS.State.disabledOpacity)
            .contentShape(Rectangle())
            .onHover { hovering = $0 && isEnabled }
    }
}

extension View {
    /// The default action of its surface (JetBrains' filled primary).
    func ideDefault() -> some View { buttonStyle(IDEButtonStyle(prominent: true)) }
    /// A secondary action: bordered, quiet fill.
    func ideSecondary(destructive: Bool = false) -> some View { buttonStyle(IDEButtonStyle(destructive: destructive)) }
}

/// Button style for custom rows, tabs and rail icons: a pressed wash over
/// whatever background the label draws, so every interactive element has a
/// visible pressed state (design-principles §14) without each call site
/// reinventing it.
struct PressableStyle: ButtonStyle {
    var cornerRadius: CGFloat = DS.Radius.tab

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .overlay {
                if configuration.isPressed {
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .fill(DS.Colors.textPrimary.opacity(DS.State.pressedOpacity))
                }
            }
    }
}

/// One flat icon control in the JetBrains/Android Studio manner: quiet
/// outline glyph on a square target, hover wash, muted accent fill + accent
/// glyph when `on` — no bezel, no glass capsule (owner on #653: reproduce
/// the JetBrains button design; unify on the left rail's icon style). Used
/// by the title bar row (TitleBar.swift) and inline icon toggles.
struct IconButtonLabel: View {
    let icon: String
    var on = false
    var hovering = false

    var body: some View {
        Image(systemName: icon)
            .font(DS.Fonts.toolbarIcon)
            .foregroundStyle(on ? DS.Colors.accentSelection : DS.Colors.textSecondary)
            .frame(width: DS.Size.toolbarButton, height: DS.Size.toolbarButton)
            .background(
                on ? DS.Colors.accentSelection.opacity(DS.State.badgeFillOpacity)
                   : hovering ? DS.Colors.hover : .clear,
                in: RoundedRectangle(cornerRadius: DS.Radius.tab))
            .contentShape(Rectangle())
    }
}

/// The quiet close control of a tool-window header: the same flat hover
/// wash as every other icon control, never a bordered system button.
struct PanelCloseButton: View {
    let help: String
    let label: String
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Image(systemName: "xmark")
                .font(DS.Fonts.header)
                .foregroundStyle(DS.Colors.textSecondary)
                .frame(width: DS.Size.inlineIconButton, height: DS.Size.inlineIconButton)
                .background(hovering ? DS.Colors.hover : .clear,
                            in: RoundedRectangle(cornerRadius: DS.Radius.control))
                .contentShape(Rectangle())
        }
        .buttonStyle(PressableStyle(cornerRadius: DS.Radius.control))
        .onHover { hovering = $0 }
        .help(help)
        .accessibilityLabel(label)
    }
}

/// A quiet inline action in JetBrains' manner: accent-coloured label text
/// with a hover wash, no bezel — what IntelliJ puts in a problem row where
/// macOS would use a bordered push button (owner on #653: no glass/bezelled
/// buttons). Keyboard focus and VoiceOver behaviour stay the Button's.
struct InlineActionLabel: View {
    let title: String
    var hovering = false

    var body: some View {
        Text(title)
            .font(DS.Fonts.secondary)
            .foregroundStyle(DS.Colors.accentSelection)
            .padding(.horizontal, DS.Space.s)
            .padding(.vertical, DS.Space.xxs)
            .background(hovering ? DS.Colors.hover : .clear,
                        in: RoundedRectangle(cornerRadius: DS.Radius.control))
            .contentShape(Rectangle())
    }
}

/// `InlineActionLabel` wired as a button, with its own hover state.
struct InlineActionButton: View {
    let title: String
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) { InlineActionLabel(title: title, hovering: hovering) }
            .buttonStyle(PressableStyle(cornerRadius: DS.Radius.control))
            .onHover { hovering = $0 }
    }
}

/// A quiet text filter chip (JetBrains tool-window filters): selected =
/// muted selection fill keeping the text's own colour, otherwise hover wash
/// only. Replaces stock segmented controls in panel headers.
struct FilterChip: View {
    let title: String
    let selected: Bool
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(DS.Fonts.secondary)
                .foregroundStyle(selected ? DS.Colors.textPrimary : DS.Colors.textSecondary)
                .padding(.horizontal, DS.Space.m)
                .padding(.vertical, DS.Space.xxs + 1)
                .background(selected ? DS.Colors.selectionFocused : hovering ? DS.Colors.hover : .clear,
                            in: RoundedRectangle(cornerRadius: DS.Radius.control))
                .contentShape(Rectangle())
        }
        .buttonStyle(PressableStyle(cornerRadius: DS.Radius.control))
        .onHover { hovering = $0 }
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

// MARK: - File-type identity

/// Colour-coded file identity in the IntelliJ manner (`design-principles.md`
/// §6): `.tex`, `.bib`, class/style files, PDF, images and generated
/// artefacts each get one icon and one colour, used identically in the file
/// tree, the tabs and the palette. Colour is identity here, never state, and
/// every case also differs by glyph so meaning is never colour-alone.
enum FileTypeStyle {
    case tex
    case texEntry
    case bibliography
    case classOrStyle
    case pdf
    case image
    case generated
    case other

    /// Style for a project-relative path. `entry` marks the root document,
    /// `bibliography` a helper-declared bibliography source.
    static func of(path: String, entry: Bool = false, bibliography: Bool = false) -> FileTypeStyle {
        if bibliography { return .bibliography }
        let ext = (path as NSString).pathExtension.lowercased()
        switch ext {
        case "tex", "ltx": return entry ? .texEntry : .tex
        case "bib": return .bibliography
        case "cls", "sty": return .classOrStyle
        case "pdf": return .pdf
        case "png", "jpg", "jpeg", "gif", "tiff", "svg", "eps": return .image
        case "aux", "log", "out", "toc", "lof", "lot", "bbl", "blg", "fls", "gz", "synctex":
            return .generated
        default: return .other
        }
    }

    var systemImage: String {
        switch self {
        case .tex: return "doc.text"
        case .texEntry: return "doc.text.fill"
        case .bibliography: return "books.vertical"
        case .classOrStyle: return "curlybraces.square"
        case .pdf: return "doc.richtext"
        case .image: return "photo"
        case .generated: return "gearshape"
        case .other: return "doc"
        }
    }

    var color: Color { Color(nsColor: nsColor) }

    /// The same identity for AppKit paint paths (the NSOutlineView tree).
    var nsColor: NSColor {
        switch self {
        case .tex, .texEntry: return DS.Palette.typeBlue
        case .bibliography: return DS.Palette.typePurple
        case .classOrStyle: return DS.Palette.typeTeal
        case .pdf: return DS.Palette.typeRed
        case .image: return DS.Palette.typeGreen
        case .generated: return DS.Palette.typeGray
        case .other: return DS.Palette.textSecondary
        }
    }
}
