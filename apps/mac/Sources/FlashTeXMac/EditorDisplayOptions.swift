import AppKit
import ObjectiveC

/// The editor's display switches that are drawn rather than laid out
/// (lane EDITOR-THEMES): the current-line band's on/off and the invisible
/// character marks. Both are drawn under the text by the coordinator's
/// background decorator (SourceEditorView.swift); nothing here edits the
/// storage, adds temporary attributes or touches layout.
@MainActor
enum EditorDisplayOptions {
    /// What was last pushed to a text view, so `EditorPreferences.apply(to:)`
    /// (which runs on every preference change) redraws only when a theme or
    /// a drawn switch actually changed.
    private struct Applied: Equatable {
        var themeGeneration: Int
        var highlightCurrentLine: Bool
        var showInvisibles: Bool
    }

    nonisolated(unsafe) private static var appliedKey: UInt8 = 0

    /// Redraws `textView` (and its gutter) when the installed theme or a drawn
    /// switch differs from what it last drew with. A theme change is a redraw
    /// only: every colour is a dynamic colour reading the installed theme.
    static func redrawIfNeeded(_ textView: NSTextView, preferences: EditorPreferences) {
        let now = Applied(themeGeneration: EditorThemeRuntime.generation,
                          highlightCurrentLine: preferences.highlightCurrentLine,
                          showInvisibles: preferences.showInvisibles)
        if let old = objc_getAssociatedObject(textView, &appliedKey) as? AppliedBox, old.value == now { return }
        objc_setAssociatedObject(textView, &appliedKey, AppliedBox(now), .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        textView.needsDisplay = true
        textView.enclosingScrollView?.contentView.needsDisplay = true
        textView.enclosingScrollView?.verticalRulerView?.needsDisplay = true
    }

    private final class AppliedBox: NSObject {
        let value: Applied
        init(_ value: Applied) { self.value = value }
    }

    // MARK: invisibles

    /// Marks drawn for invisible characters: space, tab, line end.
    static let spaceMark = "·" as NSString
    static let tabMark = "→" as NSString
    static let newlineMark = "¬" as NSString

    /// Draws a faint mark over every space, tab and line end whose glyph
    /// intersects `rect` (under the text; folded and hidden glyphs skipped).
    /// Cost is proportional to the characters in `rect`, and nothing is drawn
    /// unless the preference is on.
    static func drawInvisibles(in rect: NSRect, textView tv: NSTextView) {
        guard let lm = tv.layoutManager, let container = tv.textContainer, let storage = tv.textStorage, storage.length > 0 else { return }
        let origin = tv.textContainerOrigin
        let local = rect.offsetBy(dx: -origin.x, dy: -origin.y)
        let glyphs = lm.glyphRange(forBoundingRectWithoutAdditionalLayout: local, in: container)
        guard glyphs.length > 0 else { return }
        let chars = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let text = storage.string as NSString
        let font = tv.font ?? .monospacedSystemFont(ofSize: 13, weight: .regular)
        let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: SyntaxTheme.invisibles]
        let spaceSize = spaceMark.size(withAttributes: attrs)
        var i = chars.location
        let end = NSMaxRange(chars)
        while i < end {
            let c = text.character(at: i)
            defer { i += 1 }
            guard c == 0x20 || c == 0x09 || c == 0x0A else { continue }
            let g = lm.glyphIndexForCharacter(at: i)
            guard g < lm.numberOfGlyphs, !lm.propertyForGlyph(at: g).contains(.null) else { continue }
            let box = lm.boundingRect(forGlyphRange: NSRange(location: g, length: 1), in: container)
            if c == 0x0A {
                // The line end: after the last glyph of its line fragment.
                let used = lm.lineFragmentUsedRect(forGlyphAt: g, effectiveRange: nil, withoutAdditionalLayout: true)
                let p = NSPoint(x: used.maxX + origin.x + 1, y: used.minY + origin.y + (used.height - spaceSize.height) / 2)
                newlineMark.draw(at: p, withAttributes: attrs)
                continue
            }
            guard box.width > 0 else { continue }
            let mark = c == 0x09 ? tabMark : spaceMark
            let size = c == 0x09 ? mark.size(withAttributes: attrs) : spaceSize
            let x = c == 0x09 ? box.minX : box.midX - size.width / 2
            mark.draw(at: NSPoint(x: x + origin.x, y: box.minY + origin.y + (box.height - size.height) / 2), withAttributes: attrs)
        }
    }
}
