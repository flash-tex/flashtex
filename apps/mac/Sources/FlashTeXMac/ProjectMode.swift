import AppKit
import SwiftUI

// The document's mode in the window (docs/design/modes/PROPOSAL.md §4.2-§4.5,
// M5): the status bar's mode item and View > Mode for This Document, the
// switch, which writes `[project] mode` to flashtex.toml (through the
// project-files helper, `set_mode`; the app never writes TOML itself) only
// when the user chooses it, and the suggestion when the document needs
// Unicode mode. The mode itself is resolved by EngineV3Mode.swift; the
// engine that typesets (new or compatibility) is still EngineChoice.swift's.

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

    /// The suggestion to switch to Unicode mode: the document (its preamble,
    /// or a Classic run's own error) needs it, while it is in Classic mode.
    /// (From the engine choice's rule, resolved when the document opens or
    /// changes, and the host's report: no scan on a redraw.)
    var unicodeModeSuggestion: UnicodeFontsNeed? {
        guard documentMode.mode == .classic else { return nil }
        if case .unicodeFonts(let need)? = engineChoice.blocker { return need }
        return engineHostNeedsUnicode.flatMap { $0.document == documentURL ? $0.need : nil }
    }

    /// Switches the project to `mode` by writing `[project] mode` (creating
    /// flashtex.toml next to the entry when there is none); the manifest is
    /// read again, the engine choice follows (ShellModel.manifestDidRefresh)
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
        case .failure(let f): return refuseMode("cannot write \(ProjectManifest.fileName): \(f.why)")
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
        engineFallbackDismissed = false
        let message = mode == .unicode
            ? "Unicode mode: this project is typeset like XeLaTeX (flashtex.toml)."
            : "Classic mode: this project is typeset like pdfLaTeX (flashtex.toml)."
        navigationNote = message
        FlashTeXLog.write("mode: \(mode.rawValue), written to \(reply.path)")
        NSAccessibility.post(element: NSApplication.shared, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.high.rawValue])
        return nil
    }

    private func refuseMode(_ why: String) -> String {
        navigationNote = "Mode not changed: \(why)"
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

/// The mode menu's items (the status bar item and View > Mode for This Document).
struct ModeMenuItems: View {
    let model: ShellModel

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
            .disabled(refusal != nil || mode.mode == m && mode.source == "flashtex.toml" || m == .unicode && !unicode)
        }
        Divider()
        Text(mode.sourceLine)
        if let w = mode.warning { Text(w) }
        if let refusal { Text(refusal) }
        if !unicode { Text("Unicode mode's engine is not installed with this build.") }
        if let n = model.unicodeModeSuggestion {
            Text("This document uses \(n.what), which needs Unicode mode.")
        }
    }
}

/// Over the preview: the document needs Unicode mode (its preamble, or a
/// Classic run stopped on fontspec's error), or the project sets `[fonts]`,
/// which Classic mode does not apply; one click switches. Dismissed until
/// the next open or change.
struct UnicodeModeSuggestionBanner: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        if let s = Self.message(model), !model.engineFallbackDismissed, model.unicodeModeAvailable, model.modeSwitchRefusal == nil {
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: "character.book.closed")
                    .foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: DS.Space.xs) {
                    Text(s.headline).font(DS.Fonts.secondary.weight(.semibold))
                    Text(s.detail).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
                Button("Switch to Unicode Mode") { model.setProjectMode(.unicode) }
                    .ideDefault()
                    .help("Writes mode = \"unicode\" to flashtex.toml: the project is then typeset like XeLaTeX.")
                    .accessibilityIdentifier("mode.suggestion.switch")
                Button("Not Now") { model.engineFallbackDismissed = true }
                    .ideSecondary()
            }
            .padding(DS.Space.m)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .contain)
            .accessibilityLabel(s.headline + " " + s.detail)
            .accessibilityIdentifier("mode.suggestion")
        }
    }

    struct Message: Equatable { var headline: String; var detail: String }

    /// What the banner says, if it shows: a document that needs Unicode
    /// mode, else `[fonts]` in a Classic project.
    @MainActor static func message(_ model: ShellModel) -> Message? {
        // Classic chosen in flashtex.toml (or by FLASHTEX_MODE, or a
        // `% !TEX program` line) is the user's choice: no suggestion.
        let mode = model.documentMode
        guard mode.mode == .classic, mode.source == "default" else { return nil }
        if let n = model.unicodeModeSuggestion {
            return Message(headline: "This document uses \(n.what), which needs Unicode mode.",
                           detail: "Classic mode is pdfLaTeX-compatible and cannot load OpenType fonts. Unicode mode typesets the project like XeLaTeX; until you switch, the compatibility engine typesets it.")
        }
        if case .projectFonts(let roles)? = model.manifest.currentSnapshot.flatMap({ EngineChoice.blocker(manifest: $0.manifest) }) {
            return Message(headline: "[fonts] is not applied in Classic mode.",
                           detail: "flashtex.toml sets \(roles.joined(separator: ", ")). Classic mode typesets with TeX's fonts, like pdfLaTeX; Unicode mode applies [fonts] (as fontspec would). Until you switch, the compatibility engine applies them.")
        }
        return nil
    }
}
