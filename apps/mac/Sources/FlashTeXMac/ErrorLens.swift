import AppKit
import SwiftUI
import FlashTeXProtocol

/// Error lens (lane mac-editor-dx-3): the diagnostic message of a line drawn
/// dimmed after its last glyph, from the same `EditorDiagnostics.Mark`s the
/// gutter and the underlines use — no second source of truth. Errors only by
/// default; warnings by preference. Nothing is inserted in the text storage:
/// the painter draws in the text view's foreground pass for the visible
/// lines only (one message per line, the worst severity first).
enum ErrorLens {
    /// Preference (UserDefaults; a separate small store so the versioned
    /// EditorPreferences snapshot stays untouched).
    static let enabledKey = "FlashTeX.ErrorLens.enabled"
    static let warningsKey = "FlashTeX.ErrorLens.warnings"

    static var enabled: Bool {
        get { UserDefaults.standard.object(forKey: enabledKey) as? Bool ?? true }
        set { UserDefaults.standard.set(newValue, forKey: enabledKey); NotificationCenter.default.post(name: changed, object: nil) }
    }
    static var showsWarnings: Bool {
        get { UserDefaults.standard.object(forKey: warningsKey) as? Bool ?? false }
        set { UserDefaults.standard.set(newValue, forKey: warningsKey); NotificationCenter.default.post(name: changed, object: nil) }
    }
    static let changed = Notification.Name("FlashTeX.ErrorLens.changed")

    /// One line's lens text.
    struct Line: Equatable {
        var line: Int
        var severity: RuntimeV1.Severity
        var text: String
    }

    /// The lens per line: the message of the worst mark on that line (an
    /// error wins; among equals the first), gaps and — unless
    /// `warnings` — warnings left out. `lineOf` maps a UTF-16 offset to its line.
    static func lines(for marks: [EditorDiagnostics.Mark], warnings: Bool, lineOf: (Int) -> Int) -> [Line] {
        var byLine: [Int: Line] = [:]
        for m in marks where !EditorDiagnostics.isGap(m.message) {
            guard m.severity == .error || warnings else { continue }
            let line = lineOf(m.nsRange.location)
            if let existing = byLine[line], existing.severity == .error || m.severity != .error { continue }
            byLine[line] = Line(line: line, severity: m.severity, text: summarize(m.message))
        }
        return byLine.values.sorted { $0.line < $1.line }
    }

    /// One line, ≤ 90 characters.
    static func summarize(_ message: String) -> String {
        let flat = message.split(whereSeparator: \.isNewline).first.map(String.init) ?? message
        return flat.count > 90 ? String(flat.prefix(89)) + "…" : flat
    }
}

/// Draws the lens in the text view's foreground pass. Owned by the
/// `SourceEditorView` coordinator, which feeds it the current marks.
@MainActor
final class ErrorLensPainter {
    private(set) var lines: [ErrorLens.Line] = []
    /// Evidence for tests: the last lines drawn (line index → text).
    private(set) var drawn: [Int: String] = [:]
    var lineTable: (() -> SyntaxHighlighter)?
    private var observer: NSObjectProtocol?
    private weak var textView: NSTextView?

    init() {
        observer = NotificationCenter.default.addObserver(forName: ErrorLens.changed, object: nil, queue: nil) { [weak self] _ in
            MainActor.assumeIsolated { self?.refresh() }
        }
    }

    deinit { if let observer { NotificationCenter.default.removeObserver(observer) } }

    private var marks: [EditorDiagnostics.Mark] = []

    func attach(_ tv: NSTextView) {
        textView = tv
        (tv as? CompletingTextView)?.foregroundDecorator = { [weak self] rect in self?.draw(in: rect) }
    }

    /// New marks (the coordinator's `MarkPainter` already holds the same list).
    func update(marks: [EditorDiagnostics.Mark]) {
        self.marks = marks
        refresh()
    }

    /// What the caret-fix hint says, or nil when no fix is offered.
    private(set) var caretFixText: String?
    /// Evidence for tests: the hint actually drawn last pass.
    private(set) var drawnCaretFix: String?

    /// The fix offered at the caret (`ShellModel.caretFix`). The hint names the
    /// key and the text it inserts, because that is exactly what Tab will do.
    func update(caretFix: EditorDiagnostics.CaretFix?) {
        let new = caretFix.map { "⇥ " + $0.replacement }
        guard new != caretFixText else { return }
        caretFixText = new
        textView?.needsDisplay = true
    }

    private func refresh() {
        guard let table = lineTable?() else { return }
        let new = ErrorLens.enabled
            ? ErrorLens.lines(for: marks.filter { $0.nsRange.location >= 0 && $0.nsRange.location <= table.length },
                              warnings: ErrorLens.showsWarnings, lineOf: { table.line(at: $0) })
            : []
        guard new != lines else { return }
        lines = new
        textView?.needsDisplay = true
    }

    /// The rect of a line's last fragment, in text-view coordinates, or nil.
    private func fragment(ofLine line: Int, lm: NSLayoutManager, container: NSTextContainer,
                          table: SyntaxHighlighter) -> NSRect? {
        guard line < table.lineCount else { return nil }
        let range = table.lineRange(line)
        let lineEnd = max(range.location, NSMaxRange(range) - (NSMaxRange(range) < table.length ? 1 : 0)) // before the newline
        let glyphs = lm.glyphRange(forCharacterRange: NSRange(location: range.location, length: max(0, lineEnd - range.location)), actualCharacterRange: nil)
        var fragment = lm.boundingRect(forGlyphRange: glyphs, in: container)
        if glyphs.length == 0, glyphs.location < lm.numberOfGlyphs { fragment = lm.lineFragmentRect(forGlyphAt: glyphs.location, effectiveRange: nil); fragment.size.width = 0 }
        if glyphs.length == 0, glyphs.location >= lm.numberOfGlyphs { fragment = lm.extraLineFragmentRect; fragment.size.width = 0 }
        // The last line fragment of a wrapped line: draw after its last glyph.
        if glyphs.length > 0 {
            fragment = lm.lineFragmentUsedRect(forGlyphAt: NSMaxRange(glyphs) - 1, effectiveRange: nil)
        }
        return fragment
    }

    private func draw(in rect: NSRect) {
        drawn = [:]
        drawnCaretFix = nil
        guard let tv = textView, let lm = tv.layoutManager, let container = tv.textContainer,
              let table = lineTable?(), table.length == (tv.textStorage?.length ?? 0) else { return }
        guard !lines.isEmpty || caretFixText != nil else { return }
        let font = NSFont.systemFont(ofSize: max(9, (tv.font?.pointSize ?? 13) - 2))
        let inset = tv.textContainerInset
        /// Where each line's lens text ended, so the fix hint can sit after it.
        var lensEndX: [Int: CGFloat] = [:]
        for entry in lines {
            guard let fragment = fragment(ofLine: entry.line, lm: lm, container: container, table: table) else { continue }
            var origin = NSPoint(x: fragment.maxX + inset.width + 24, y: fragment.minY + inset.height)
            let band = NSRect(x: origin.x, y: origin.y, width: tv.bounds.width - origin.x, height: fragment.height)
            guard band.intersects(rect) else { continue }
            let color = (entry.severity == .error ? NSColor.systemRed : NSColor.systemOrange).withAlphaComponent(0.85)
            let text = (entry.severity == .error ? "✕ " : "△ ") + entry.text
            let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
            let size = (text as NSString).size(withAttributes: attrs)
            origin.y += (fragment.height - size.height) / 2
            let available = max(0, tv.bounds.width - origin.x - 8)
            let clipped = clip(text, to: available, attrs: attrs)
            (clipped as NSString).draw(at: origin, withAttributes: attrs)
            drawn[entry.line] = clipped
            lensEndX[entry.line] = origin.x + (clipped as NSString).size(withAttributes: attrs).width
        }
        drawCaretFix(in: rect, tv: tv, lm: lm, container: container, table: table, font: font,
                     inset: inset, lensEndX: lensEndX)
    }

    /// The caret's fix, after that line's text (and after its lens message when
    /// one is showing). Drawn whatever `ErrorLens.enabled` says: Tab acts on
    /// this hint, so it must not be something the author can switch off and
    /// then be surprised by.
    private func drawCaretFix(in rect: NSRect, tv: NSTextView, lm: NSLayoutManager,
                              container: NSTextContainer, table: SyntaxHighlighter, font: NSFont,
                              inset: NSSize, lensEndX: [Int: CGFloat]) {
        guard let text = caretFixText else { return }
        let caret = tv.selectedRange().location
        guard caret >= 0, caret <= table.length else { return }
        let line = table.line(at: caret)
        guard let fragment = fragment(ofLine: line, lm: lm, container: container, table: table) else { return }
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font, .foregroundColor: NSColor.controlAccentColor.withAlphaComponent(0.95),
        ]
        let size = (text as NSString).size(withAttributes: attrs)
        let after = lensEndX[line].map { $0 + 12 } ?? (fragment.maxX + inset.width + 24)
        var origin = NSPoint(x: after, y: fragment.minY + inset.height)
        let band = NSRect(x: origin.x, y: origin.y, width: tv.bounds.width - origin.x, height: fragment.height)
        guard band.intersects(rect) else { return }
        origin.y += (fragment.height - size.height) / 2
        let available = max(0, tv.bounds.width - origin.x - 8)
        let clipped = clip(text, to: available, attrs: attrs)
        (clipped as NSString).draw(at: origin, withAttributes: attrs)
        drawnCaretFix = clipped
    }

    private func clip(_ text: String, to width: CGFloat, attrs: [NSAttributedString.Key: Any]) -> String {
        guard (text as NSString).size(withAttributes: attrs).width > width else { return text }
        var s = text
        while s.count > 4, (s as NSString).size(withAttributes: attrs).width > width { s = String(s.dropLast(2)) }
        return s.count > 4 ? String(s.dropLast()) + "…" : s
    }
}

/// Preferences rows (EditorPreferences.swift embeds this under the editor toggles).
struct ErrorLensPreferenceRows: View {
    @State private var enabled = ErrorLens.enabled
    @State private var warnings = ErrorLens.showsWarnings

    var body: some View {
        Toggle("Show diagnostics inline (error lens)", isOn: $enabled)
            .onChange(of: enabled) { _, v in ErrorLens.enabled = v }
            .help("Draws each line's error message dimmed after the line, from the same diagnostics as the gutter")
        Toggle("Include warnings in the error lens", isOn: $warnings)
            .disabled(!enabled)
            .onChange(of: warnings) { _, v in ErrorLens.showsWarnings = v }
    }
}
