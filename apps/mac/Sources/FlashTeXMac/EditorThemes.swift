import AppKit
import FlashTeXEditorCore
import Observation
import UniformTypeIdentifiers

// Editor colour themes on the Mac (lane EDITOR-THEMES). The theme model and
// the built-ins are platform-free (FlashTeXEditorCore/EditorColorTheme.swift,
// shared with the iPad); this file turns the selected theme into AppKit
// colours and keeps the user's theme folder.

// MARK: - runtime

/// The theme the editor draws with right now, as one stable dynamic `NSColor`
/// per role. Each colour looks the current theme up when AppKit resolves it
/// (per draw, per appearance), so switching themes or overriding a colour
/// needs a redraw only: the syntax painter's temporary attributes already hold
/// these colours, and nothing is re-lexed or repainted (`install(_:)`).
enum EditorThemeRuntime {
    typealias Role = EditorColorTheme.Role

    /// Resolved sRGB colours of one theme, per role, both appearances.
    private struct Table {
        var light: [NSColor?]
        var dark: [NSColor?]
        init(_ theme: EditorColorTheme) {
            let t = theme.resolved()
            func colors(_ p: EditorColorTheme.Palette) -> [NSColor?] {
                Role.allCases.map { role in p[role].map(EditorThemeRuntime.nsColor) }
            }
            light = colors(t.light); dark = colors(t.dark)
        }
    }

    private static let lock = NSLock()
    nonisolated(unsafe) private static var table = Table(.flashtex)
    nonisolated(unsafe) private static var installedTheme = EditorColorTheme.flashtex
    /// Bumped by every `install` (evidence for tests; observers compare it).
    nonisolated(unsafe) private(set) static var generation = 0

    /// Posted on the main thread after `install` changed the colours.
    static let didChange = Notification.Name("FlashTeX.EditorThemeRuntime.didChange")

    /// The theme (with overrides applied) the colours currently resolve to.
    static var current: EditorColorTheme {
        lock.lock(); defer { lock.unlock() }
        return installedTheme
    }

    /// Makes `theme` the one every editor colour resolves to. Views redraw on
    /// `didChange`; no text is re-lexed.
    @MainActor
    static func install(_ theme: EditorColorTheme) {
        let new = Table(theme)
        lock.lock()
        let changed = installedTheme != theme
        table = new
        installedTheme = theme
        if changed { generation += 1 }
        lock.unlock()
        guard changed else { return }
        NotificationCenter.default.post(name: didChange, object: nil)
    }

    /// The resolved colour of `role` for an appearance (nil: a syntax role the
    /// theme leaves to the plain text colour).
    static func resolved(_ role: Role, dark: Bool) -> NSColor? {
        lock.lock(); defer { lock.unlock() }
        return (dark ? table.dark : table.light)[role.index]
    }

    static func isDark(_ appearance: NSAppearance) -> Bool {
        appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
    }

    static func nsColor(_ c: EditorColorTheme.Color) -> NSColor {
        let v = c.components
        return NSColor(srgbRed: v.red, green: v.green, blue: v.blue, alpha: v.alpha)
    }

    /// One stable dynamic colour per role. A syntax role the theme leaves
    /// out resolves to the theme's text colour; comments are drawn at
    /// `commentAlpha` while hybrid conceal dims them.
    private static let dynamicColors: [NSColor] = Role.allCases.map { role in
        NSColor(name: NSColor.Name("FlashTeX.theme.\(role.rawValue)")) { appearance in
            let dark = isDark(appearance)
            let color = resolved(role, dark: dark) ?? resolved(.foreground, dark: dark) ?? .textColor
            if role == .comment, commentsDimmed { return color.withAlphaComponent(color.alphaComponent * commentAlpha) }
            return color
        }
    }

    /// Alpha comments are drawn at while dimmed (HybridConcealDisplay.swift).
    static let commentAlpha: CGFloat = 0.5
    nonisolated(unsafe) private static var dimmed = false
    static var commentsDimmed: Bool {
        lock.lock(); defer { lock.unlock() }
        return dimmed
    }

    /// Hybrid conceal's "dim comments": a redraw, like a theme change.
    @MainActor
    static func setCommentsDimmed(_ on: Bool) {
        lock.lock()
        let changed = dimmed != on
        dimmed = on
        if changed { generation += 1 }
        lock.unlock()
        if changed { NotificationCenter.default.post(name: didChange, object: nil) }
    }

    /// The dynamic colour for `role`.
    static func color(_ role: Role) -> NSColor { dynamicColors[role.index] }
}

// MARK: - the user's theme folder

/// Built-in themes plus the JSON themes in the user's folder
/// (`~/Library/Application Support/FlashTeX/Themes/*.json`; see
/// apps/mac/docs/editor-themes.md). Read at launch and after every import,
/// delete or `reload()`; a malformed file is listed in `problems`, never
/// silently dropped.
@Observable @MainActor
final class EditorThemeLibrary {
    static let shared = EditorThemeLibrary(directory: EditorThemeLibrary.defaultDirectory)

    /// `$FLASHTEX_THEMES_DIR`, else Application Support/FlashTeX/Themes.
    nonisolated static var defaultDirectory: URL {
        if let override = ProcessInfo.processInfo.environment["FLASHTEX_THEMES_DIR"], !override.isEmpty {
            return URL(fileURLWithPath: override, isDirectory: true)
        }
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return support.appendingPathComponent("FlashTeX/Themes", isDirectory: true)
    }

    let directory: URL
    /// User themes, sorted by name (built-ins are `EditorColorTheme.builtIns`).
    private(set) var userThemes: [EditorColorTheme] = []
    /// A theme file that could not be read, and why.
    struct Problem: Equatable, Identifiable {
        var file: String
        var message: String
        var id: String { file }
    }
    /// Files that could not be read.
    private(set) var problems: [Problem] = []

    init(directory: URL) {
        self.directory = directory
        reload()
    }

    /// Built-ins first, then the user's themes.
    var allThemes: [EditorColorTheme] { EditorColorTheme.builtIns + userThemes }

    func theme(id: String) -> EditorColorTheme? {
        EditorColorTheme.builtIn(id: id) ?? userThemes.first { $0.id == id }
    }

    func reload() {
        var themes: [EditorColorTheme] = []
        var problems: [Problem] = []
        let files = (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)) ?? []
        for url in files where url.pathExtension.lowercased() == "json" {
            let stem = url.deletingPathExtension().lastPathComponent
            do {
                let data = try Data(contentsOf: url)
                var theme = try EditorColorTheme.decode(json: data, fallbackID: stem)
                // A user theme never shadows a built-in.
                if EditorColorTheme.builtIn(id: theme.id) != nil { theme.id = "user." + theme.id }
                if themes.contains(where: { $0.id == theme.id }) { theme.id += "." + stem }
                themes.append(theme)
            } catch {
                problems.append(Problem(file: url.lastPathComponent, message: String(describing: error)))
            }
        }
        userThemes = themes.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        self.problems = problems.sorted { $0.file < $1.file }
    }

    /// Copies a theme file into the folder (after validating it) and returns
    /// the imported theme's id.
    @discardableResult
    func importTheme(from url: URL) throws -> String {
        let data = try Data(contentsOf: url)
        let stem = url.deletingPathExtension().lastPathComponent
        let theme = try EditorColorTheme.decode(json: data, fallbackID: stem)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        var target = directory.appendingPathComponent(Self.fileName(for: theme.id))
        var n = 2
        while FileManager.default.fileExists(atPath: target.path) {
            target = directory.appendingPathComponent(Self.fileName(for: "\(theme.id)-\(n)")); n += 1
        }
        try data.write(to: target, options: .atomic)
        reload()
        let imported = target.deletingPathExtension().lastPathComponent
        return userThemes.first { $0.id == theme.id || $0.id.hasSuffix("." + imported) }?.id ?? theme.id
    }

    /// Writes `theme` (every role, both appearances) as a theme file.
    func export(_ theme: EditorColorTheme, to url: URL) throws {
        try theme.encodedJSON().write(to: url, options: .atomic)
    }

    /// Removes a user theme's file (built-ins cannot be removed).
    func delete(id: String) throws {
        guard EditorColorTheme.builtIn(id: id) == nil else { return }
        let files = (try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)) ?? []
        for url in files where url.pathExtension.lowercased() == "json" {
            guard let data = try? Data(contentsOf: url),
                  let theme = try? EditorColorTheme.decode(json: data, fallbackID: url.deletingPathExtension().lastPathComponent) else { continue }
            let stem = url.deletingPathExtension().lastPathComponent
            if theme.id == id || "user." + theme.id == id || theme.id + "." + stem == id {
                try FileManager.default.removeItem(at: url)
            }
        }
        reload()
    }

    static func fileName(for id: String) -> String {
        let safe = id.map { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "_" || $0 == "." ? $0 : "-" }
        return String(safe) + ".json"
    }

    // MARK: panels (Settings > Themes)

    /// Open panel → import; returns the new theme's id, nil when cancelled.
    func runImportPanel() -> Result<String, Error>? {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false
        panel.message = "Choose a FlashTeX theme file (.json)"
        guard panel.runModal() == .OK, let url = panel.url else { return nil }
        return Result { try importTheme(from: url) }
    }

    /// Save panel → export `theme`; nil when cancelled.
    func runExportPanel(for theme: EditorColorTheme) -> Result<URL, Error>? {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = Self.fileName(for: theme.isBuiltIn ? theme.id + "-custom" : theme.id)
        panel.message = "Export the theme, including your colour overrides, as a theme file"
        guard panel.runModal() == .OK, let url = panel.url else { return nil }
        return Result { try export(theme, to: url); return url }
    }

    func revealFolder() {
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        NSWorkspace.shared.activateFileViewerSelecting([directory])
    }
}
