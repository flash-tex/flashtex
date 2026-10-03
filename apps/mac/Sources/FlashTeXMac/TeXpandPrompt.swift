import AppKit
import FlashTeXEditorCore

/// Editor ▸ Expand Abbreviation or Edit Structure… (⌃⌘T): inside a matrix,
/// tabular, cases or align the structure editor (M10b) toggles; anywhere
/// else the abbreviation prompt (PLAN §9.6, M7) opens at the caret. With a
/// selection the prompt wraps it: `<<selection>>`, and a bare `*` repeats
/// once per selected line.
enum TeXpandCommandAction {
    static func run() { NSApp.sendAction(#selector(CompletingTextView.texpandCommand(_:)), to: nil, from: nil) }
}

extension CompletingTextView {
    @objc func texpandCommand(_ sender: Any?) {
        if texpand.toggleStructureEditor() { return }
        texpand.openPrompt()
    }
}

extension TeXpandEditor {
    /// Expands `abbreviation` at the selection (wrapping it when not empty)
    /// as one undo step, with the preamble edit. False when TeXpand is off or
    /// the abbreviation does not expand (the reason is announced).
    @discardableResult
    func expandFromPrompt(_ abbreviation: String) -> Bool {
        ensureFresh()
        guard let controller, let storage = textViewForPrompt.textStorage else {
            announce("TeXpand is off (Settings › Abbreviations)")
            return false
        }
        let text = storage.mutableString
        let selection = textViewForPrompt.selectedRange()
        let result = controller.promptExpansion(abbreviation, selection: selection, text: text)
        switch result {
        case .success(let commit):
            applyPromptCommit(commit)
            return true
        case .failure(let f):
            announce(f.message)
            NSSound.beep()
            return false
        }
    }

    /// Opens the inline prompt at the caret.
    func openPrompt() {
        ensureFresh()
        guard controller != nil else {
            announce("TeXpand is off (Settings › Abbreviations)")
            NSSound.beep()
            return
        }
        TeXpandPromptPanel.shared.open(for: self)
    }

    /// Live preview for the prompt: the expansion's text, or why it fails.
    func promptPreview(_ abbreviation: String) -> (text: String, ok: Bool) {
        guard let controller, let storage = textViewForPrompt.textStorage else { return ("TeXpand is off", false) }
        guard !abbreviation.isEmpty else { return ("", true) }
        switch controller.promptExpansion(abbreviation, selection: textViewForPrompt.selectedRange(), text: storage.mutableString) {
        case .success(let c): return (c.snippet.text, true)
        case .failure(let f): return (f.message, false)
        }
    }
}

/// The prompt: a borderless panel under the caret with a field and the
/// live preview. Return or Tab expands, Esc closes. One per app.
@MainActor
final class TeXpandPromptPanel: NSPanel, NSTextFieldDelegate {
    static let shared = TeXpandPromptPanel()

    private let field = NSTextField()
    private let preview = NSTextField(labelWithString: "")
    private weak var editor: TeXpandEditor?

    private init() {
        super.init(contentRect: NSRect(x: 0, y: 0, width: 420, height: 60), styleMask: [.borderless, .nonactivatingPanel],
                   backing: .buffered, defer: true)
        isFloatingPanel = true
        becomesKeyOnlyIfNeeded = false
        hasShadow = true
        backgroundColor = .windowBackgroundColor
        let stack = NSStackView(views: [field, preview])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.edgeInsets = NSEdgeInsets(top: 8, left: 8, bottom: 8, right: 8)
        stack.spacing = 6
        field.placeholderString = "Abbreviation (enum3, fig>img+cap, pmat3x3:I)"
        field.font = .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        field.delegate = self
        field.setAccessibilityLabel("TeXpand abbreviation")
        preview.font = .monospacedSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular)
        preview.textColor = .secondaryLabelColor
        preview.maximumNumberOfLines = 12
        preview.lineBreakMode = .byClipping
        preview.setAccessibilityLabel("Expansion preview")
        field.widthAnchor.constraint(equalToConstant: 400).isActive = true
        contentView = stack
    }

    override var canBecomeKey: Bool { true }

    func open(for editor: TeXpandEditor) {
        self.editor = editor
        field.stringValue = ""
        preview.stringValue = ""
        let tv = editor.textViewForPrompt
        if let window = tv.window, let lm = tv.layoutManager, let tc = tv.textContainer {
            let caret = min(tv.selectedRange().location, tv.textStorage?.length ?? 0)
            let glyph = lm.glyphIndexForCharacter(at: max(0, caret - (caret > 0 ? 1 : 0)))
            var rect = lm.boundingRect(forGlyphRange: NSRange(location: glyph, length: 0), in: tc)
            rect.origin.x += tv.textContainerOrigin.x
            rect.origin.y += tv.textContainerOrigin.y
            let onScreen = window.convertToScreen(tv.convert(rect, to: nil))
            setFrameTopLeftPoint(NSPoint(x: onScreen.minX, y: onScreen.minY - 4))
            window.addChildWindow(self, ordered: .above)
        }
        orderFront(nil)
        makeKey()
        makeFirstResponder(field)
    }

    func close(expanding: Bool) {
        let abbreviation = field.stringValue
        let editor = self.editor
        parent?.removeChildWindow(self)
        orderOut(nil)
        if let tv = editor?.textViewForPrompt { tv.window?.makeKey(); tv.window?.makeFirstResponder(tv) }
        if expanding, !abbreviation.isEmpty { editor?.expandFromPrompt(abbreviation) }
    }

    func controlTextDidChange(_ obj: Notification) {
        guard let editor else { return }
        let p = editor.promptPreview(field.stringValue)
        preview.stringValue = p.text
        preview.textColor = p.ok ? .secondaryLabelColor : .systemRed
    }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        switch selector {
        case #selector(NSResponder.insertNewline(_:)), #selector(NSResponder.insertTab(_:)):
            close(expanding: true)
            return true
        case #selector(NSResponder.cancelOperation(_:)):
            close(expanding: false)
            return true
        default:
            return false
        }
    }
}
