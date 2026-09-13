import AppKit
import SwiftUI

/// The switch that gates vim mode, shaped like `CaptureInboxFeature`
/// (ShellModel+Nearby.swift): a persisted preference, an environment escape
/// hatch for launch-time QA, and a `nonisolated(unsafe)` override tests set.
///
/// Unlike the capture switches this one defaults **off**: the preference is
/// `false` out of the box, the environment variable must say `1` to turn it on,
/// and the single `keyDown` hook returns `false` before touching anything when
/// it is off — so a user who never turns it on sees no change at all.
enum VimModeFeature {
    /// Tests set this to drive the editor without touching `UserDefaults`;
    /// nil (the default) means "whatever the preference says".
    nonisolated(unsafe) static var enabledOverride: Bool?

    static func flag(_ name: String, environment: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        environment[name] == "1"
    }

    @MainActor
    static var isEnabled: Bool {
        if let enabledOverride { return enabledOverride }
        if flag("FLASHTEX_VIM_MODE") { return true }
        return EditorPreferences.shared.vimMode
    }

    /// Length of a buffer of `length` once `edits` are applied. A vim
    /// outcome's selection is in *post-edit* coordinates, and `o`, `p` and
    /// friends grow the buffer, so it must be clamped against this — clamping
    /// against the pre-edit length drags the caret back onto the old end.
    static func length(after edits: [EditorKeyHandling.LineEdit], from length: Int) -> Int {
        edits.reduce(length) { $0 + ($1.replacement as NSString).length - $1.range.length }
    }

    /// `range` clamped inside a buffer of `length` (never a negative or
    /// out-of-range `NSRange`, which `setSelectedRange` would reject).
    static func clamp(_ range: NSRange, to length: Int) -> NSRange {
        let limit = max(0, length)
        let location = min(max(range.location, 0), limit)
        return NSRange(location: location, length: min(max(range.length, 0), limit - location))
    }

    /// One `NSEvent` as the machine sees it, or nil when the event is not
    /// something vim should look at (a Command/Option shortcut, a modified
    /// arrow, a dead key). Option is excluded on purpose: on a non-US layout
    /// it composes ordinary characters, which belong to insert mode's
    /// pass-through path and to AppKit's input-method handling.
    static func key(for event: NSEvent) -> VimKey? {
        let flags = event.modifierFlags.intersection([.command, .option, .control, .shift])
        if flags.contains(.command) || flags.contains(.option) { return nil }
        switch event.keyCode {
        case 53: return .escape
        case 36, 76: return .enter
        case 51: return .backspace
        case 123, 124, 125, 126: // ←, →, ↓, ↑ — plain only; ⇧-arrow keeps AppKit's selection
            guard !flags.contains(.shift) else { return nil }
            switch event.keyCode {
            case 123: return .c("h")
            case 124: return .c("l")
            case 125: return .c("j")
            default: return .c("k")
            }
        default: break
        }
        let source = flags.contains(.control) ? event.charactersIgnoringModifiers : event.characters
        guard let source, source.count == 1, let ch = source.first else { return nil }
        if flags.contains(.control) { return VimKey(character: Character(ch.lowercased()), control: true) }
        return .c(ch)
    }
}

/// What the editor's indicator shows. Equatable so the SwiftUI bar only
/// redraws when something actually changed.
struct VimModeStatus: Equatable {
    var enabled = false
    var mode: VimMode = .normal
    /// The pending command (`3d`) or command line (`:wq`) typed so far.
    var pending = ""
    var message: String?

    static let off = VimModeStatus()

    var spoken: String {
        var s = "Vim \(mode.indicator.lowercased()) mode"
        if !pending.isEmpty { s += ", pending \(pending)" }
        if let message { s += ", \(message)" }
        return s
    }
}

// MARK: - the editor hooks

extension SourceEditorView.Coordinator {
    /// The single key gate, called first thing from `CompletingTextView.keyDown`
    /// (Completion.swift) — after that view's own marked-text guard, so an
    /// input-method composition never reaches vim mode. Returns `true` when
    /// vim consumed the key; `false` leaves the event on exactly the path it
    /// takes with the feature off.
    ///
    /// Shortcut interactions, all resolved in favour of the app:
    /// - Anything with ⌘ or ⌥ is refused here, so every menu shortcut
    ///   (⌘S, ⌘B, ⌘⇧D, ⌘F, ⌘Z, ⌘/, ⌥⌘F …) and every ⌥-composed character
    ///   behaves unchanged.
    /// - ⌃Space (completion) and ⌘⇧Space (signature help) are refused because
    ///   the machine ignores every control key but `⌃R`.
    /// - Escape while the completion list or a snippet is live closes those
    ///   first (AppKit's existing behaviour); a second Escape leaves insert
    ///   mode. In normal mode Escape no longer opens the completion list —
    ///   that is the one binding vim mode shadows, and only while it is on.
    /// - Tab, Shift-Tab and Return in insert mode are not vim keys, so the
    ///   existing indent and auto-indent handling runs untouched.
    func handleVimKey(_ event: NSEvent, in tv: NSTextView) -> Bool {
        guard VimModeFeature.isEnabled, programmaticChanges == 0, !tv.hasMarkedText() else { return false }
        let completing = tv as? CompletingTextView
        if event.keyCode == 53, completing?.isCompletionActive == true || completing?.isSnippetActive == true {
            return false // the list/snippet owns the first Escape
        }
        guard let key = VimModeFeature.key(for: event) else { return false }
        // Insert mode consumes nothing but Escape. Returning before the buffer
        // sync keeps ordinary typing exactly as cheap as it is with vim off.
        if vim.mode == .insert, !(key.special == .escape || (key.control && key.character == "[")) { return false }
        vim.indentUnit = EditorPreferences.shared.indentString
        vim.sync(text: currentText(of: tv), caret: tv.selectedRange().location)
        let outcome = vim.handle(key)
        defer { publishVimStatus(tv) }
        guard outcome.handled else { return false }
        apply(vim: outcome, to: tv)
        return true
    }

    /// Applies one outcome: edits through the editor's existing grouped-undo
    /// path, then the selection, then the requests only the app can serve.
    func apply(vim outcome: VimOutcome, to tv: NSTextView) {
        if outcome.beep { NSSound.beep() }
        let length = tv.textStorage?.length ?? 0
        if !outcome.edits.isEmpty {
            // Clamp against the buffer the edits *produce*: `o` and `p` grow it,
            // and the machine's landing offset is already in those coordinates.
            let wanted = outcome.selection ?? tv.selectedRange()
            let selection = VimModeFeature.clamp(wanted, to: VimModeFeature.length(after: outcome.edits, from: length))
            applyLineEdits(outcome.edits, to: tv, actionName: outcome.actionName ?? "Vim Command", selection: selection)
            tv.scrollRangeToVisible(tv.selectedRange())
        } else if let selection = outcome.selection {
            tv.setSelectedRange(VimModeFeature.clamp(selection, to: length))
            tv.scrollRangeToVisible(tv.selectedRange())
        }
        for request in outcome.requests { perform(vim: request, in: tv) }
        if !outcome.edits.isEmpty || !outcome.requests.isEmpty {
            vim.sync(text: currentText(of: tv), caret: tv.selectedRange().location)
        }
    }

    private func perform(vim request: VimRequest, in tv: NSTextView) {
        switch request {
        // `u` / `⌃R` drive the text view's own undo manager: the very stack
        // ⌘Z and ⇧⌘Z use, so vim edits and typing interleave correctly.
        case .undo: if tv.undoManager?.canUndo == true { tv.undoManager?.undo() } else { NSSound.beep() }
        case .redo: if tv.undoManager?.canRedo == true { tv.undoManager?.redo() } else { NSSound.beep() }
        // The standard Find bar (EditorFind.swift), not a second search engine.
        case .openFind: EditorFindAction.send(.showFindInterface)
        case .findNext: EditorFindAction.send(.nextMatch)
        case .findPrevious: EditorFindAction.send(.previousMatch)
        // Save, close and Go to Matching belong to the model; the view owner
        // wires them (ContentView.swift).
        case .save, .close, .saveAndClose, .discardAndClose, .goToMatching:
            parent.onVimRequest(request)
        }
    }

    /// Pushes the indicator's state out to SwiftUI. Called after every key and
    /// whenever the feature is (re)read, never during a view update.
    func publishVimStatus(_ tv: NSTextView) {
        guard VimModeFeature.isEnabled else {
            if lastVimStatus != .off { lastVimStatus = .off; parent.onVimStatus(.off) }
            return
        }
        let status = VimModeStatus(enabled: true, mode: vim.mode, pending: vim.pendingDisplay, message: vim.message)
        guard status != lastVimStatus else { return }
        lastVimStatus = status
        parent.onVimStatus(status)
        tv.needsDisplay = true // the block cursor changes shape with the mode
    }

    /// The block cursor normal and visual mode draw instead of the I-beam;
    /// nil (the thin caret) in insert mode and whenever the feature is off.
    /// One character wide in the editor's own font, so it lines up on the
    /// monospaced grid.
    func vimBlockCaretRect(_ rect: NSRect, in tv: NSTextView) -> NSRect? {
        guard VimModeFeature.isEnabled, vim.mode != .insert, rect.width < 4 else { return nil }
        let font = tv.font ?? EditorPreferences.shared.font
        let advance = ("m" as NSString).size(withAttributes: [.font: font]).width
        guard advance > 1 else { return nil }
        var block = rect
        block.size.width = advance
        return block
    }
}

// MARK: - the indicator

/// The status strip under the editor: the mode word, the pending command and
/// the caret's line and column. Shown only while the feature is on, so the
/// window's layout is byte-for-byte unchanged with the preference off.
struct VimModeIndicator: View {
    var status: VimModeStatus

    private var tint: Color {
        switch status.mode {
        case .normal: .secondary
        case .insert: .green
        case .visual, .visualLine: .orange
        }
    }

    var body: some View {
        HStack(spacing: 8) {
            Text(status.mode.indicator)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .padding(.horizontal, 6).padding(.vertical, 2)
                .background(tint.opacity(0.22), in: RoundedRectangle(cornerRadius: 3))
                .foregroundStyle(tint)
            if !status.pending.isEmpty {
                Text(status.pending).font(.system(size: 10, design: .monospaced)).foregroundStyle(.secondary)
            }
            if let message = status.message {
                Text(message).font(.system(size: 10, design: .monospaced)).foregroundStyle(.red).lineLimit(1)
            }
            Spacer()
        }
        .padding(.horizontal, 8).padding(.vertical, 3)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(status.spoken)
    }
}
