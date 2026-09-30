import AppKit
import FlashTeXEditorCore

/// TeXpand in the Mac editor (docs/texpand/HOST.md, PLAN M3): the adapter
/// between a `CompletingTextView` and the core's `CaptureController`.
///
/// - Edits: every text-storage edit (typed, programmatic, undo/redo) goes to
///   the scope provider and the controller, from the storage's
///   `didProcessEditing` notification, before the view's own
///   `didChangeText` work (so the completion list already knows a capture
///   is running and stays shut).
/// - Keys: `CompletingTextView.keyDown` offers Tab and Esc here first when
///   no completion list is open (PLAN §9.5: list, capture, snippet stops,
///   caret fix, indentation).
/// - Commit: `insertTeXpandSnippet`, one undo step ("Expand Abbreviation"),
///   placeholders selected. One ⌘Z restores the literal, and the controller
///   marks it so Tab does not expand it again.
/// - Preview: the capture region is underlined and the expansion drawn in a
///   box under it, in the text view's draw pass (the error-lens pattern).
///
/// Off (Settings › Abbreviations) it does nothing but check the switch.
@MainActor
final class TeXpandEditor {
    private unowned let textView: CompletingTextView
    let scopes = TeXpand.ScopeProvider()
    private(set) var controller: TeXpand.CaptureController?
    private(set) var region: NSRange?
    private(set) var preview: String?
    private(set) var diagnostic: String?
    private var observers: [NSObjectProtocol] = []
    /// While the adapter applies a commit (its edit is not typing).
    private var applying = false

    init(textView: CompletingTextView) {
        self.textView = textView
        rebuild()
        observers.append(NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification, object: nil, queue: nil) { [weak self] note in
            MainActor.assumeIsolated {
                guard let self, let storage = note.object as? NSTextStorage, storage === self.textView.textStorage else { return }
                self.storageEdited(storage)
            }
        })
        observers.append(NotificationCenter.default.addObserver(forName: TeXpandPreferences.changed, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.rebuild() }
        })
    }

    deinit {
        for o in observers { NotificationCenter.default.removeObserver(o) }
    }

    /// Whether a capture is running: the completion list stays shut.
    var isCapturing: Bool { controller?.isCapturing ?? false }

    /// (Re)builds the controller from the current settings; nil while off.
    func rebuild() {
        let settings = TeXpandPreferences.settings
        guard settings.isActive(.abbreviations) else {
            controller = nil
            show(TeXpand.CaptureController.Output())
            return
        }
        let scopes = self.scopes
        let c = TeXpand.CaptureController(engine: TeXpandPreferences.engine(for: settings)) { scopes.scope(at: $0, in: $1) }
        c.documentClass = { [weak textView] in textView?.projectDocumentClass() }
        c.indentUnit = EditorPreferences.shared.indentString
        controller = c
    }

    // MARK: events from the text view

    private func storageEdited(_ storage: NSTextStorage) {
        guard storage.editedMask.contains(.editedCharacters) else { return }
        let now = storage.editedRange
        let old = NSRange(location: now.location, length: max(0, now.length - storage.changeInLength))
        scopes.noteEdit(range: old, replacementLength: now.length)
        guard let controller else { return }
        let undo = textView.undoManager.map { $0.isUndoing || $0.isRedoing } ?? false
        let kind: TeXpand.CaptureController.EditKind = undo ? .undo : (textView.isTypingKeystroke && !applying ? .typed : .programmatic)
        let text = storage.mutableString
        show(controller.edited(old, replacement: text.substring(with: now), kind: kind, text: text))
    }

    func selectionChanged() {
        guard let controller, !applying, let storage = textView.textStorage else { return }
        show(controller.cursorMoved(to: textView.selectedRange(), text: storage.mutableString))
    }

    /// Tab: true when TeXpand took it (a commit, or an incomplete capture's
    /// diagnostic); false passes it down the precedence.
    func tab() -> Bool {
        guard let controller, !textView.hasMarkedText(), let storage = textView.textStorage else { return false }
        let out = controller.tab(selection: textView.selectedRange(), text: storage.mutableString)
        show(out)
        if let commit = out.commit {
            applying = true
            textView.insertTeXpandSnippet(commit.snippet, replacing: commit.range)
            applying = false
            announce("Expanded \(commit.literal)")
        } else if let d = out.diagnostic {
            announce(d)
        }
        return out.consumed
    }

    /// Esc: true when it ended a capture (the literal stays, and is marked).
    func escape() -> Bool {
        guard let controller else { return false }
        let out = controller.escape()
        show(out)
        return out.consumed
    }

    func focusLost() {
        guard let controller else { return }
        show(controller.focusLost())
    }

    private func announce(_ message: String) {
        NSAccessibility.post(element: textView, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.medium.rawValue])
    }

    // MARK: drawing

    private func show(_ out: TeXpand.CaptureController.Output) {
        let changed = out.region != region || out.preview != preview || out.diagnostic != diagnostic
        region = out.region
        preview = out.preview
        diagnostic = out.diagnostic
        if changed { textView.setNeedsDisplay(textView.visibleRect) }
    }

    /// The most lines of an expansion the preview shows.
    static let previewLines = 12

    func draw(_ dirtyRect: NSRect) {
        guard let region, let lm = textView.layoutManager, let tc = textView.textContainer,
              let storage = textView.textStorage, NSMaxRange(region) <= storage.length else { return }
        let origin = textView.textContainerOrigin
        let glyphs = lm.glyphRange(forCharacterRange: region, actualCharacterRange: nil)
        var rect = lm.boundingRect(forGlyphRange: glyphs, in: tc)
        rect.origin.x += origin.x
        rect.origin.y += origin.y
        // The region: a faint wash and an accent underline.
        NSColor.controlAccentColor.withAlphaComponent(0.12).setFill()
        NSBezierPath(roundedRect: rect.insetBy(dx: -1, dy: 0), xRadius: 3, yRadius: 3).fill()
        NSColor.controlAccentColor.setFill()
        NSRect(x: rect.minX, y: rect.maxY - 1.5, width: rect.width, height: 1.5).fill()

        guard let body = diagnostic ?? preview, !body.isEmpty else { return }
        var lines = body.components(separatedBy: "\n")
        if lines.count > Self.previewLines { lines = Array(lines.prefix(Self.previewLines - 1)) + ["…"] }
        let font = textView.font ?? NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        let color: NSColor = diagnostic != nil ? .systemRed : .secondaryLabelColor
        let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
        let strings = lines.map { NSAttributedString(string: $0.isEmpty ? " " : $0, attributes: attrs) }
        let lineHeight = ceil(font.ascender - font.descender + font.leading)
        let width = strings.map { ceil($0.size().width) }.max() ?? 0
        let pad: CGFloat = 6
        let box = NSRect(x: rect.minX - pad, y: rect.maxY + 3, width: width + 2 * pad, height: CGFloat(lines.count) * lineHeight + 2 * pad)
        let path = NSBezierPath(roundedRect: box, xRadius: 6, yRadius: 6)
        NSColor.textBackgroundColor.setFill()
        path.fill()
        NSColor.separatorColor.setStroke()
        path.lineWidth = 1
        path.stroke()
        for (k, s) in strings.enumerated() {
            s.draw(at: NSPoint(x: box.minX + pad, y: box.minY + pad + CGFloat(k) * lineHeight))
        }
    }
}
