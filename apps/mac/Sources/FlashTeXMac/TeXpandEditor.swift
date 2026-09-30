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
    typealias T = TeXpand
    private unowned let textView: CompletingTextView
    /// The text view, for the prompt and the structure editor (TeXpandPrompt.swift).
    var textViewForPrompt: CompletingTextView { textView }
    let scopes = TeXpand.ScopeProvider()
    private(set) var controller: TeXpand.CaptureController?
    private(set) var region: NSRange?
    private(set) var preview: String?
    private(set) var diagnostic: String?
    private var observers: [NSObjectProtocol] = []
    /// While the adapter applies a commit (its edit is not typing).
    private var applying = false
    /// A commit a keystroke completed (an instant atom, a ligature, an auto
    /// fraction), applied once the keystroke's own edit is done
    /// (`CompletingTextView.didChangeText`): text cannot change while the
    /// storage is still processing the keystroke.
    private var pendingCommit: T.CaptureController.Commit?
    /// The open structure editor (TeXpandStructureEditor.swift), if any.
    var structureEditor: TeXpandStructureEditor?
    /// A notice drawn at the caret until the next edit or caret move (M6:
    /// packages that could not be added here).
    private(set) var notice: String?
    /// `auto_preamble = "prompt"`: asks whether to add `missing`; the reply
    /// adds them (as their own undo step). Tests replace it.
    var promptHandler: ([T.PackageRequirement], @escaping (Bool) -> Void) -> Void = { missing, reply in
        let alert = NSAlert()
        alert.messageText = "Add \(missing.map(\.name).joined(separator: ", ")) to the preamble?"
        alert.informativeText = missing.map(\.description).joined(separator: "\n")
        alert.addButton(withTitle: "Add")
        alert.addButton(withTitle: "Not Now")
        if let window = NSApp.keyWindow { alert.beginSheetModal(for: window) { reply($0 == .alertFirstButtonReturn) } } else { reply(false) }
    }

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
        guard settings.enabled, T.Settings.Tier.allCases.contains(where: settings.isActive) else {
            controller = nil
            configStamp = nil
            show(TeXpand.CaptureController.Output())
            return
        }
        let (config, stamp) = currentConfig()
        configStamp = stamp
        let engine = TeXpandPreferences.engine(for: settings, config: config)
        // A file or magic comment can switch TeXpand (or a kind) off.
        guard engine.settings.enabled, T.Settings.Tier.allCases.contains(where: engine.settings.isActive) else {
            controller = nil
            show(TeXpand.CaptureController.Output())
            return
        }
        reportConfigProblems(engine.diagnostics)
        let scopes = self.scopes
        let c = TeXpand.CaptureController(engine: engine) { scopes.scope(at: $0, in: $1) }
        c.documentClass = { [weak self] in self?.rootIndex().documentClass ?? self?.textView.projectDocumentClass() }
        c.packages = { [weak self] in self?.rootIndex().packages ?? [] }
        c.indentUnit = EditorPreferences.shared.indentString
        controller = c
    }

    // MARK: config layers and hot reload (M8)

    /// What the current config was built from: rebuilt when any of it changes.
    struct ConfigStamp: Equatable {
        var user: Date?
        var projectPath: String?
        var project: Date?
        var packs: [String]
        var magic: T.Layer?
    }
    private var configStamp: ConfigStamp?
    /// The last config problems reported, so one is announced once.
    private(set) var configDiagnostics: [T.Diagnostic] = []

    var projectConfigURL: URL? { textView.texpandProject()?.root?.appendingPathComponent("texpand.toml") }

    /// Layers 3–6 as they are now: pack files, the user file, the project's
    /// `texpand.toml`, and this document's `% !texpand` comments.
    func currentConfig() -> (T.Config, ConfigStamp) {
        let user = TeXpandPreferences.layer(at: TeXpandPreferences.userConfigURL, name: "texpand.toml (user)")
        let projectURL = projectConfigURL
        let project = projectURL.flatMap { TeXpandPreferences.layer(at: $0, name: "texpand.toml (project)") }
        let packs = TeXpandPreferences.packLayers()
        let magic = T.magicComments(in: magicRegion())
        let config = T.Config(packs: packs, user: user?.layer, project: project?.layer, magic: magic)
        return (config, ConfigStamp(user: user?.modified, projectPath: projectURL?.path, project: project?.modified,
                                    packs: packs.map { $0.name + String($0.source.hashValue) }, magic: magic))
    }

    /// The document's first lines, where magic comments live.
    func magicRegion() -> String {
        let ns = textView.string as NSString
        var end = 0, lines = 0
        while end < ns.length, lines < 30 {
            let r = ns.lineRange(for: NSRange(location: end, length: 0))
            end = NSMaxRange(r)
            lines += 1
        }
        return ns.substring(to: end)
    }

    /// Hot reload: rebuilds when a config file or magic comment changed.
    /// Cheap (two stats and the first lines), so it runs whenever TeXpand is
    /// about to act: the leader typed, Tab, the command.
    func ensureFresh() {
        let settings = TeXpandPreferences.settings
        guard settings.enabled else { return }
        let projectURL = projectConfigURL
        let quick = ConfigStamp(user: TeXpandPreferences.modified(TeXpandPreferences.userConfigURL), projectPath: projectURL?.path,
                                project: projectURL.flatMap(TeXpandPreferences.modified),
                                packs: configStamp?.packs ?? [], magic: T.magicComments(in: magicRegion()))
        if quick != configStamp { rebuild() }
    }

    /// Announces new problems in the user's layers (never the built-ins).
    private func reportConfigProblems(_ diagnostics: [T.Diagnostic]) {
        let mine = diagnostics.filter { !$0.layer.hasPrefix("built-in") && $0.severity != .note }
        defer { configDiagnostics = mine }
        guard mine != configDiagnostics, let first = mine.first(where: { $0.severity == .error }) ?? mine.first else { return }
        notice = first.description + (mine.count > 1 ? " (+\(mine.count - 1) more)" : "")
        announce(notice!)
        textView.setNeedsDisplay(textView.visibleRect)
    }

    // MARK: events from the text view

    /// The exact edit the text view announced (`shouldChangeText`): the
    /// storage's `editedRange` can be wider than the change (typing `;`
    /// before a `$` reports `;$` replacing `$`), which would hide a typed
    /// leader.
    private var announced: (range: NSRange, replacement: String)?

    func willChange(_ range: NSRange, replacement: String?) {
        announced = replacement.map { (range, $0) }
    }

    private func storageEdited(_ storage: NSTextStorage) {
        guard storage.editedMask.contains(.editedCharacters) else { return }
        let now = storage.editedRange
        let old = NSRange(location: now.location, length: max(0, now.length - storage.changeInLength))
        let text = storage.mutableString
        var range = old
        var replacement = text.substring(with: now)
        if let a = announced, (a.replacement as NSString).length - a.range.length == storage.changeInLength,
           a.range.location >= now.location, a.range.location + (a.replacement as NSString).length <= NSMaxRange(now),
           NSMaxRange(a.range) <= text.length - storage.changeInLength {
            range = a.range
            replacement = a.replacement
        }
        announced = nil
        scopes.noteEdit(range: range, replacementLength: (replacement as NSString).length)
        if let c = indexCache, c.isCurrent, range.location <= c.scanEnd { indexCache = nil } // the preamble changed
        if let open = structureEditor, !applying { open.close(apply: false) } // the source changed under it
        if !applying { notice = nil }
        if range.length == 0, textView.isTypingKeystroke, replacement == (controller?.engine.settings.leader ?? TeXpandPreferences.settings.leader) {
            ensureFresh() // the leader: the moment a changed config matters
        }
        guard let controller else { return }
        let undo = textView.undoManager.map { $0.isUndoing || $0.isRedoing } ?? false
        let kind: TeXpand.CaptureController.EditKind = undo ? .undo : (textView.isTypingKeystroke && !applying ? .typed : .programmatic)
        let out = controller.edited(range, replacement: replacement, kind: kind, text: text)
        if let commit = out.commit { pendingCommit = commit }
        show(out)
    }

    func applyPendingCommit() {
        guard let commit = pendingCommit, !applying else { return }
        pendingCommit = nil
        apply(commit)
    }

    func applyPromptCommit(_ commit: T.CaptureController.Commit) { apply(commit) }

    private func apply(_ commit: T.CaptureController.Commit) {
        let root = rootInfo()
        let mode = controller?.engine.settings.autoPreamble ?? .insert
        let action = T.preambleAction(for: commit.requires, mode: mode, rootIsCurrent: root.isCurrent, rootText: root.text)
        applying = true
        defer { applying = false }
        var range = commit.range
        // Missing packages go in first, in the same undo step, so the
        // snippet's stops are laid out after them.
        let undo = textView.undoManager
        var grouped = false
        if case .insert(let at, let text) = action, at <= range.location {
            textView.breakUndoCoalescing()
            undo?.beginUndoGrouping()
            grouped = true
            textView.insertTeXpandPreamble(text, at: at)
            range.location += (text as NSString).length
        }
        if commit.inline {
            textView.replaceTeXpandText(range, with: commit.snippet.text, actionName: "Expand")
        } else {
            textView.insertTeXpandSnippet(commit.snippet, replacing: range)
        }
        if grouped {
            undo?.setActionName("Expand Abbreviation")
            undo?.endUndoGrouping()
            textView.breakUndoCoalescing()
        }
        switch action {
        case .prompt(let missing, let at, let text):
            promptHandler(missing) { [weak self] yes in
                guard yes, let self else { return }
                // Recompute: the text may have changed while the sheet was up.
                let again = T.preambleAction(for: missing, mode: .insert, rootIsCurrent: true, rootText: self.textView.string)
                if case .insert(let at2, let text2) = again {
                    self.applying = true
                    self.textView.breakUndoCoalescing()
                    self.textView.insertTeXpandPreamble(text2, at: at2)
                    self.textView.undoManager?.setActionName("Add Packages")
                    self.textView.breakUndoCoalescing()
                    self.applying = false
                }
                _ = (at, text)
            }
        case .notice(let missing):
            notice = "Needs " + missing.map(\.description).joined(separator: ", ") + (root.isCurrent ? " (no preamble here)" : " in \((root.path as NSString).lastPathComponent)")
            announce(notice!)
            textView.setNeedsDisplay(textView.visibleRect)
        case .insert, .none:
            break
        }
    }

    // MARK: the root file and its packages (M6)

    struct IndexCache {
        var path: String
        var isCurrent: Bool
        var index: T.PackageIndex
        /// Where the scan stopped (`\begin{document}`): edits after it keep the cache.
        var scanEnd: Int
        var made: Date
    }
    private var indexCache: IndexCache?

    /// The root document (`% !TEX root`, the project's main file, this
    /// file): its path, whether it is the buffer being edited, and its text.
    func rootInfo() -> (path: String, isCurrent: Bool, text: String) {
        let current = textView.string
        guard let project = textView.texpandProject() else { return ("", true, current) }
        let path = T.rootPath(current: project.activePath, currentText: current, projectMain: project.entryPath)
        if path == project.activePath { return (path, true, current) }
        if let open = project.text(path) { return (path, false, open) }
        if let root = project.root, let disk = try? String(contentsOf: root.appendingPathComponent(path), encoding: .utf8) {
            return (path, false, disk)
        }
        return (path, false, "")
    }

    func rootIndex() -> T.PackageIndex {
        if let c = indexCache, c.isCurrent || Date().timeIntervalSince(c.made) < 3 { return c.index }
        let root = rootInfo()
        let index = T.PackageIndex.scan(root.text)
        let end = (root.text as NSString).range(of: "\\begin{document}").location
        indexCache = IndexCache(path: root.path, isCurrent: root.isCurrent, index: index,
                                scanEnd: end == NSNotFound ? Int.max : end, made: Date())
        return index
    }

    func selectionChanged() {
        guard let controller, !applying, let storage = textView.textStorage else { return }
        show(controller.cursorMoved(to: textView.selectedRange(), text: storage.mutableString))
    }

    /// Tab: true when TeXpand took it (a commit, or an incomplete capture's
    /// diagnostic); false passes it down the precedence.
    func tab() -> Bool {
        if controller?.isCapturing != true { ensureFresh() }
        guard let controller, !textView.hasMarkedText(), let storage = textView.textStorage else { return false }
        let out = controller.tab(selection: textView.selectedRange(), text: storage.mutableString)
        show(out)
        if let commit = out.commit {
            apply(commit)
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

    func announce(_ message: String) {
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
        if region == nil, let notice, let lm = textView.layoutManager, let tc = textView.textContainer {
            let caret = min(textView.selectedRange().location, textView.textStorage?.length ?? 0)
            let glyph = lm.glyphIndexForCharacter(at: max(0, caret - 1))
            var rect = lm.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
            rect.origin.x += textView.textContainerOrigin.x
            rect.origin.y += textView.textContainerOrigin.y
            drawBox(lines: [notice], color: .secondaryLabelColor, below: NSRect(x: rect.minX, y: rect.minY, width: 1, height: rect.height))
            _ = tc
        }
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
        drawBox(lines: lines, color: diagnostic != nil ? .systemRed : .secondaryLabelColor, below: rect)
    }

    /// A box of monospaced lines under `rect` (the region, or the caret line).
    private func drawBox(lines: [String], color: NSColor, below rect: NSRect) {
        let font = textView.font ?? NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
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

