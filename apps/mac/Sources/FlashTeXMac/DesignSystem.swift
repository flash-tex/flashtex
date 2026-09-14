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
        static let tab: CGFloat = 30
        static let statusBar: CGFloat = 24
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
        /// The pairing code: read across the room, typed on another device.
        static let pairingCode = Font.system(size: 34, weight: .semibold, design: .monospaced)
    }

    // MARK: colour — semantic AppKit colours so all three appearance modes are correct

    enum Colors {
        /// Content ground: editor, lists, trees.
        static let surfacePrimary = Color(nsColor: .controlBackgroundColor)
        /// Chrome: bars, panel headers, tab strip, status bar.
        static let surfaceSecondary = Color(nsColor: .windowBackgroundColor)
        /// Floating surfaces: completion popup, palette, popovers.
        static let surfaceRaised = Color(nsColor: .controlBackgroundColor)
        /// The preview column's neutral ground the page floats on.
        static let surfaceGround = Color(nsColor: .underPageBackgroundColor)
        /// QR ground: scanners need literal white behind the code in both
        /// appearances — the one deliberately non-semantic surface.
        static let qrGround = Color.white
        static let separator = Color(nsColor: .separatorColor)

        static let textPrimary = Color(nsColor: .labelColor)
        static let textSecondary = Color(nsColor: .secondaryLabelColor)
        static let textTertiary = Color(nsColor: .tertiaryLabelColor)

        /// The system accent; honours the user's accent-colour setting.
        static let accentSelection = Color(nsColor: .controlAccentColor)
        /// Focused selection band in lists and trees.
        static let selectionFocused = Color(nsColor: .selectedContentBackgroundColor)
        /// Selected-but-unfocused: visibly weaker than focused (macOS convention).
        static let selectionUnfocused = Color(nsColor: .unemphasizedSelectedContentBackgroundColor)

        static let severityError = Color(nsColor: .systemRed)
        static let severityWarning = Color(nsColor: .systemOrange)
        static let severityInfo = Color(nsColor: .systemBlue)
        static let severitySuccess = Color(nsColor: .systemGreen)

        /// The modified (unsaved) dot on tabs and tree rows.
        static let statusModified = Color(nsColor: .systemOrange)
        /// A completed older snapshot shown while a newer revision compiles.
        static let statusHistorical = Color(nsColor: .systemPurple)
        /// Gutter marker on lines with an available fix.
        static let gutterMarker = Color(nsColor: .controlAccentColor)

        /// Outline item-type identity (typed icons, IntelliJ-fashion):
        /// colour tells the kind apart together with the glyph.
        static let typeTable = Color(nsColor: .systemBlue)
        static let typeFloat = Color(nsColor: .systemGreen)
        static let typeMath = Color(nsColor: .systemPurple)
        static let typeLabel = Color(nsColor: .systemOrange)
    }

    /// AppKit type for panels the SwiftUI `Fonts` cannot reach (the
    /// completion popup is an NSPanel + NSTableView on purpose).
    enum NSFonts {
        static let base = NSFont.systemFont(ofSize: 13)
        static let secondary = NSFont.systemFont(ofSize: 11)
        static let header = NSFont.systemFont(ofSize: 11, weight: .semibold)
        /// Completion candidates: the editor's vocabulary, one step smaller.
        static let monoCandidate = NSFont.monospacedSystemFont(ofSize: 12, weight: .medium)
    }

    /// AppKit paint paths (gutter marks, rulers) use `NSColor` directly;
    /// same semantic mapping as `Colors`, one source of truth per meaning.
    enum NSColors {
        static let severityError = NSColor.systemRed
        static let severityWarning = NSColor.systemOrange
        static let gapDot = NSColor.tertiaryLabelColor
        static let gutterGlyph = NSColor.secondaryLabelColor
        static let gutterHairline = NSColor.separatorColor.withAlphaComponent(0.5)
        /// Ring around a severity dot whose line carries a Tab-applicable fix.
        static let fixRing = NSColor.controlAccentColor
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
        static let railButton: CGFloat = 28
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
    }

    // MARK: layout — split minimums and panel bounds

    enum Layout {
        /// The left icon rail (tool-window stripe).
        static let railWidth: CGFloat = 40
        /// The window itself: usable from ~900pt wide (design-principles §4).
        static let windowMinWidth: CGFloat = 900
        static let windowMinHeight: CGFloat = 600
        static let editorMinWidth: CGFloat = 340
        static let previewMinWidth: CGFloat = 380
        static let sidebarMinWidth: CGFloat = 200
        static let sidebarIdealWidth: CGFloat = 240
        static let sidebarMaxWidth: CGFloat = 360
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
        /// Disclosure, panel reveal, popup appearance. Nil under Reduce
        /// Motion — pass to `.animation(_:value:)` and the change is instant.
        @MainActor static var quick: Animation? { ReduceMotion.isEnabled ? nil : .easeOut(duration: durationQuick) }
        @MainActor static var standard: Animation? { ReduceMotion.isEnabled ? nil : .easeOut(duration: durationStandard) }
    }
}

// MARK: - Interaction

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

    var color: Color {
        switch self {
        case .tex, .texEntry: return Color(nsColor: .systemBlue)
        case .bibliography: return Color(nsColor: .systemPurple)
        case .classOrStyle: return Color(nsColor: .systemTeal)
        case .pdf: return Color(nsColor: .systemRed)
        case .image: return Color(nsColor: .systemGreen)
        case .generated: return Color(nsColor: .systemGray)
        case .other: return Color(nsColor: .secondaryLabelColor)
        }
    }
}
