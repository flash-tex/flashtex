import AppKit
import SwiftUI

// The document's mode in the window (docs/design/modes/PROPOSAL.md §4.2-§4.5,
// M5): the status bar's mode item and View > Mode for This Document, the
// switch, which writes `[project] mode` to flashtex.toml (through the
// project-files helper, `set_mode`; the app never writes TOML itself) only
// when the user chooses it, and the warning when the document needs Unicode
// mode while its mode is set to Classic. The mode itself is resolved by
// EngineV3Mode.swift; a document nothing sets the mode of is routed to
// Unicode mode when it needs it (UnicodeFonts.swift).

extension ShellModel {
    /// The open document's mode, and what chose it.
    var documentMode: EngineV3Mode.Resolution {
        EngineV3Session.mode(self, main: project.entryPath)
    }

    /// Whether Unicode mode's host is there to run (built, bundled or named).
    /// Looked up once per setting of where it may be (the status item asks
    /// on every redraw; the lookup stats a few paths).
    var unicodeModeAvailable: Bool {
        let key = (ProcessInfo.processInfo.environment["FLASHTEX_HOST_UNICODE"] ?? "") + "\u{0}"
            + (EngineV3.defaults.string(forKey: EngineV3.hostPathKey + ".unicode") ?? "")
        if let c = Self.unicodeHostLookup, c.key == key { return c.available }
        let available = EngineV3.locateHost(mode: .unicode) != nil
        Self.unicodeHostLookup = (key, available)
        return available
    }

    private static var unicodeHostLookup: (key: String, available: Bool)?

    /// Why the user cannot switch the mode here, if they cannot.
    var modeSwitchRefusal: String? {
        if let e = ProcessInfo.processInfo.environment["FLASHTEX_MODE"], !e.isEmpty {
            return "FLASHTEX_MODE=\(e) sets the mode for every document; flashtex.toml cannot change it."
        }
        if project.projectRoot == nil { return "Save the document first: the mode is kept in the project's flashtex.toml." }
        return nil
    }

    /// The warning that the document needs Unicode mode while it is in
    /// Classic mode (set so in flashtex.toml, by FLASHTEX_MODE or a
    /// `% !TEX program` line; a mode nothing sets is routed instead): from
    /// the last preamble scan and the host's report, no scan on a redraw.
    var unicodeModeSuggestion: UnicodeFontsNeed? {
        guard documentMode.mode == .classic else { return nil }
        if let s = unicodeFontsScan, s.entry == project.entryPath, let n = s.need { return n }
        return engineHostNeedsUnicode.flatMap { $0.document == documentURL ? $0.need : nil }
    }

    /// Switches the project to `mode` by writing `[project] mode` (creating
    /// flashtex.toml next to the entry when there is none); the manifest is
    /// read again (ShellModel.manifestDidRefresh)
    /// and the host restarts in the mode. Nil, or why it was not written.
    @discardableResult
    func setProjectMode(_ mode: EngineV3Mode) -> String? {
        if let why = modeSwitchRefusal { return refuseMode(why) }
        guard let root = project.projectRoot else { return refuseMode("the document is not saved") }
        if mode == .unicode, !unicodeModeAvailable {
            return refuseMode("Unicode mode's engine (flashtex-host-unicode) is not installed with this build")
        }
        // flashtex.toml open in the editor with edits not saved: those are
        // the user's; refuse rather than write over or beside them.
        let manifestPaths = [manifest.snapshot?.path].compactMap { $0 } + [root.appendingPathComponent(ProjectManifest.fileName).path]
        for open in documents.map(\.path) where ProjectManifest.isManifestPath(open) && open.hasSuffix(ProjectManifest.fileName) {
            let abs = root.appendingPathComponent(open).standardizedFileURL.path
            if manifestPaths.contains(where: { URL(fileURLWithPath: $0).standardizedFileURL.path == abs }), project.isDirty(open) {
                return refuseMode("\(ProjectManifest.fileName) has unsaved edits in the editor; save or revert them first")
            }
        }
        // What the helper rewrites is the file as it is now: the save checks
        // it is still that (a write in between is a conflict, not lost).
        let manifestURL = URL(fileURLWithPath: manifest.snapshot?.path ?? root.appendingPathComponent(ProjectManifest.fileName).path)
        let before = (try? Data(contentsOf: manifestURL)).map { SourceDigest.sha256Hex($0) }
        let reply: ProjectFilesV1.SetFonts
        switch files.setMode(for: root, entry: project.entryPath, mode: mode.rawValue) {
        case .success(let r): reply = r
        case .failure(let f): return refuseMode(Self.setModeFailure(f.why))
        }
        if reply.changed, let text = reply.text {
            let url = URL(fileURLWithPath: reply.path)
            let expected: ProjectFilesV1.Expected = !reply.exists ? .newFile
                : (url.standardizedFileURL == manifestURL.standardizedFileURL ? before.map { ProjectFilesV1.Expected.hash($0) } : nil) ?? .any
            switch files.save(url, text: text, expected: expected, force: false, recordsState: false) {
            case .saved: break
            case .conflict(let c): return refuseMode("cannot write \(ProjectManifest.fileName): \(c.summary)")
            case .failed(let why): return refuseMode("cannot write \(ProjectManifest.fileName): \(why)")
            }
        }
        manifest.refresh()
        let message = mode == .unicode
            ? "Unicode mode: this project is typeset like XeLaTeX (flashtex.toml)."
            : "Classic mode: this project is typeset like pdfLaTeX (flashtex.toml)."
        navigationNote = message
        FlashTeXLog.write("mode: \(mode.rawValue), written to \(reply.path)")
        NSAccessibility.post(element: NSApplication.shared, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.high.rawValue])
        return nil
    }

    /// What a failed `set_mode` says. A helper from before `set_mode`
    /// (a bundle that copied an old build) answers "unknown operation".
    static func setModeFailure(_ why: String) -> String {
        if why.contains("unknown operation") {
            return "The project-files helper in this build is out of date: it cannot write [project] mode to \(ProjectManifest.fileName). Rebuild crates/project-files and the app, or add mode = \"classic\" or \"unicode\" under [project] in \(ProjectManifest.fileName) yourself. (\(why))"
        }
        return "Cannot write \(ProjectManifest.fileName): \(why)"
    }

    private func refuseMode(_ why: String) -> String {
        navigationNote = "Mode not changed: \(why)"
        modeSwitchFailure = why
        FlashTeXLog.write("mode: not changed: \(why)")
        return why
    }
}

extension EngineV3Mode {
    /// The status item's and the menu's name.
    var title: String { self == .unicode ? "Unicode" : "Classic" }
    /// The menu's item, with what it is compatible with.
    var menuTitle: String { self == .unicode ? "Unicode (XeLaTeX-compatible)" : "Classic (pdfLaTeX-compatible)" }
    var symbol: String { self == .unicode ? "character.book.closed" : "doc.text" }
}

extension EngineV3Mode.Resolution {
    /// Where the mode comes from, as the menu says it.
    var sourceLine: String {
        switch source {
        case "FLASHTEX_MODE": "Set by FLASHTEX_MODE"
        case "flashtex.toml": "Set in flashtex.toml"
        case "% !TEX program": "From the document's % !TEX program line"
        case EngineV3Mode.detectedSource: "Detected: the document needs Unicode fonts, or flashtex.toml sets [fonts] (not saved in flashtex.toml)"
        default: "Default (no mode set)"
        }
    }
}

/// The status bar's mode item: Classic or Unicode, a mark when the document
/// needs Unicode mode while in Classic, and the switch.
struct ModeStatusItem: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let mode = model.documentMode
        let suggestion = model.unicodeModeSuggestion
        Menu {
            ModeMenuItems(model: model)
        } label: {
            Label(mode.mode.title, systemImage: suggestion == nil ? mode.mode.symbol : "exclamationmark.triangle.fill")
                .font(DS.Fonts.secondary)
                .foregroundStyle(suggestion == nil ? AnyShapeStyle(.secondary) : AnyShapeStyle(DS.Colors.severityWarning))
        }
        .font(DS.Fonts.secondary)
        .controlSize(.small)
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .help(Self.help(mode, suggestion: suggestion))
        .accessibilityLabel("Mode: \(mode.mode.title)")
        .accessibilityValue(suggestion.map { "suggestion: Unicode mode, the document uses \($0.what)" } ?? mode.sourceLine)
        .accessibilityHint("Choose how this project is typeset: like pdfLaTeX or like XeLaTeX.")
        .accessibilityIdentifier("mode.item")
    }

    static func help(_ mode: EngineV3Mode.Resolution, suggestion: UnicodeFontsNeed?) -> String {
        var s = "\(mode.mode.menuTitle). \(mode.sourceLine)."
        if let w = mode.warning { s += " \(w)" }
        if let n = suggestion { s += " This document uses \(n.what), which needs Unicode mode." }
        return s
    }
}

extension EngineV3Mode.Resolution {
    /// The menu's explanation: where the mode comes from and, when it is
    /// the `% !TEX program` line, that a choice here takes precedence.
    func menuExplanation(main: String) -> String {
        let file = main.isEmpty ? "the main file" : (main as NSString).lastPathComponent
        switch source {
        case "% !TEX program":
            return "Set by % !TEX program in \(file). Choosing a mode here saves it in \(ProjectManifest.fileName), which takes precedence."
        case "default":
            return "Default (no mode set). Choosing a mode here saves it in \(ProjectManifest.fileName)."
        case EngineV3Mode.detectedSource:
            return "\(sourceLine). Choosing a mode here saves it in \(ProjectManifest.fileName), which takes precedence."
        default:
            return sourceLine
        }
    }
}

/// The mode menu's items (the status bar item and View > Mode for This Document).
struct ModeMenuItems: View {
    let model: ShellModel

    /// An item is off when the mode cannot be switched here, when it is
    /// already the mode flashtex.toml sets, or Unicode without its engine.
    static func isDisabled(_ m: EngineV3Mode, mode: EngineV3Mode.Resolution, refusal: String?, unicodeAvailable: Bool) -> Bool {
        refusal != nil || mode.mode == m && mode.source == "flashtex.toml" || m == .unicode && !unicodeAvailable
    }

    var body: some View {
        let mode = model.documentMode
        let refusal = model.modeSwitchRefusal
        let unicode = model.unicodeModeAvailable
        ForEach([EngineV3Mode.classic, .unicode], id: \.self) { m in
            Button {
                model.setProjectMode(m)
            } label: {
                if mode.mode == m { Label(m.menuTitle, systemImage: "checkmark") } else { Text(m.menuTitle) }
            }
            .disabled(Self.isDisabled(m, mode: mode, refusal: refusal, unicodeAvailable: unicode))
        }
        Divider()
        Text(mode.menuExplanation(main: model.project.entryPath))
        if let w = mode.warning { Text(w) }
        if let refusal { Text(refusal) }
        if !unicode { Text("Unicode mode's engine is not installed with this build.") }
        if let n = model.unicodeModeSuggestion {
            Text("This document uses \(n.what), which needs Unicode mode.")
        }
    }
}
