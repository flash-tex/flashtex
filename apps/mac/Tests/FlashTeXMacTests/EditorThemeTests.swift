import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// Editor themes (lane EDITOR-THEMES): the shared theme model and built-ins,
/// the JSON theme-file format, the user's theme folder, the preferences that
/// select and override a theme, the process-wide dynamic colours (a theme
/// switch redraws but never re-lexes or repaints), and the display switches.
@MainActor
final class EditorThemeTests: XCTestCase {
    typealias Theme = EditorColorTheme
    typealias Role = EditorColorTheme.Role

    private var suiteName = ""
    private var defaults: UserDefaults!
    private var folder: URL!

    override func setUp() {
        super.setUp()
        suiteName = "flashtex.tests.EditorTheme.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
        folder = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-themes-\(UUID().uuidString)", isDirectory: true)
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        try? FileManager.default.removeItem(at: folder)
        // Leave the process-wide colours as the shared preferences have them.
        EditorThemeRuntime.install(EditorPreferences.shared.resolvedTheme)
        super.tearDown()
    }

    // MARK: model and built-ins

    func testBuiltInsCoverEveryRoleInBothAppearances() {
        XCTAssertEqual(Theme.builtIns.map(\.id), ["flashtex", "solarized", "github", "one", "dracula", "nord"])
        XCTAssertEqual(Set(Theme.builtIns.map(\.id)).count, Theme.builtIns.count, "ids are unique")
        for theme in Theme.builtIns {
            XCTAssertTrue(theme.isBuiltIn)
            for variant in Theme.Variant.allCases {
                for role in Role.allCases where role != .verbatim {
                    XCTAssertNotNil(theme.color(role, variant), "\(theme.id) \(variant) lacks \(role)")
                }
            }
            XCTAssertNotEqual(theme.color(.background, .light), theme.color(.background, .dark), "\(theme.id): light and dark differ")
        }
    }

    /// Text and every token colour stay readable on the theme's ground
    /// (WCAG 4.5:1 for text; 3:1 for tokens, which are also shaped by the
    /// syntax; comments, braces and line numbers are deliberately quieter).
    /// Solarized is checked against its own published contrast (its base00
    /// text on base3 is 4.1:1 by design); its values are kept canonical.
    func testBuiltInsAreReadable() {
        for theme in Theme.builtIns {
            let canonicalLowContrast = theme.id == "solarized"
            for variant in Theme.Variant.allCases {
                let bg = theme.color(.background, variant)!
                XCTAssertGreaterThanOrEqual(theme.color(.foreground, variant)!.contrast(with: bg), canonicalLowContrast ? 4.0 : 4.5,
                                            "\(theme.id) \(variant) text")
                for role in [Role.command, .mathCommand, .environment, .math, .number, .reference, .definition, .error] {
                    let c = theme.color(role, variant)!
                    XCTAssertGreaterThanOrEqual(c.contrast(with: bg), canonicalLowContrast ? 2.9 : 3.0, "\(theme.id) \(variant) \(role) \(c.hex) on \(bg.hex)")
                }
                for role in [Role.comment, .brace, .gutterText] {
                    let c = theme.color(role, variant)!
                    XCTAssertGreaterThanOrEqual(c.contrast(with: bg), 1.6, "\(theme.id) \(variant) \(role) \(c.hex) on \(bg.hex)")
                }
            }
        }
    }

    func testFlashTeXThemeKeepsTheEditorsOriginalColours() {
        let t = Theme.flashtex
        XCTAssertEqual(t.color(.command, .light), Theme.Color(0x0033B3))
        XCTAssertEqual(t.color(.command, .dark), Theme.Color(0xCF8E6D))
        XCTAssertEqual(t.color(.background, .dark), Theme.Color(0x191A1C))
        XCTAssertEqual(t.color(.selection, .light), Theme.Color(0xD0DFFE))
        XCTAssertNil(t.color(.verbatim, .light), "verbatim stays plain")
    }

    func testHexColours() {
        XCTAssertEqual(Theme.Color(hex: "#0033B3"), Theme.Color(0x0033B3))
        XCTAssertEqual(Theme.Color(hex: "0033b3"), Theme.Color(0x0033B3))
        XCTAssertEqual(Theme.Color(hex: "#abc"), Theme.Color(0xAABBCC))
        XCTAssertEqual(Theme.Color(hex: "#11223380"), Theme.Color(red: 0x11, green: 0x22, blue: 0x33, alpha: 0x80))
        XCTAssertNil(Theme.Color(hex: "#12345"))
        XCTAssertNil(Theme.Color(hex: "blue"))
        XCTAssertEqual(Theme.Color(0x0033B3).hex, "#0033B3")
        XCTAssertEqual(Theme.Color(red: 1, green: 2, blue: 3, alpha: 4).hex, "#01020304")
        XCTAssertEqual(Theme.Color(0xFFFFFF).contrast(with: Theme.Color(0x000000)), 21, accuracy: 0.01)
    }

    func testKindsMapOneToOneOntoSyntaxRoles() {
        let roles = SyntaxHighlighter.Kind.allCases.map(Role.init(kind:))
        XCTAssertEqual(Set(roles).count, SyntaxHighlighter.Kind.allCases.count)
        XCTAssertTrue(Set(roles).isSubset(of: Set(Role.syntaxRoles)))
        XCTAssertTrue(Set(Role.chromeRoles).isDisjoint(with: Set(Role.syntaxRoles)))
    }

    // MARK: theme files

    func testThemeFileRoundTrips() throws {
        for theme in Theme.builtIns {
            var t = try Theme.decode(json: theme.encodedJSON(), fallbackID: "x")
            t.isBuiltIn = true
            XCTAssertEqual(t, theme, theme.id)
        }
    }

    func testPartialThemeFillsFromItsBaseAndASingleVariantServesBoth() throws {
        let json = ##"{"name": "Paper", "basedOn": "nord", "light": {"background": "#FFFFF8", "command": "#123456", "future": "#000000"}}"##
        let t = try Theme.decode(json: Data(json.utf8), fallbackID: "paper")
        XCTAssertEqual(t.id, "paper", "no id in the file: the file name")
        XCTAssertEqual(t.name, "Paper")
        XCTAssertFalse(t.isBuiltIn)
        XCTAssertEqual(t.color(.background, .light), Theme.Color(0xFFFFF8))
        XCTAssertEqual(t.color(.command, .light), Theme.Color(0x123456))
        XCTAssertEqual(t.color(.math, .light), Theme.nord.color(.math, .light), "left out: the base theme's")
        XCTAssertEqual(t.color(.background, .dark), Theme.Color(0xFFFFF8), "only a light palette: used for dark too")
        XCTAssertEqual(t.color(.math, .dark), Theme.nord.color(.math, .dark))
    }

    func testMalformedThemeFilesSayWhy() {
        XCTAssertThrowsError(try Theme.decode(json: Data("not json".utf8), fallbackID: "x")) {
            guard case .notJSON = $0 as? Theme.FileError else { return XCTFail("\($0)") }
        }
        XCTAssertThrowsError(try Theme.decode(json: Data(##"{"name": "x"}"##.utf8), fallbackID: "x")) {
            XCTAssertEqual($0 as? Theme.FileError, .noVariant)
        }
        XCTAssertThrowsError(try Theme.decode(json: Data(##"{"name": "x", "dark": {"command": "red"}}"##.utf8), fallbackID: "x")) {
            XCTAssertEqual($0 as? Theme.FileError, .badColor(role: "command", value: "red"))
            XCTAssertTrue(String(describing: $0).contains("command"))
        }
        XCTAssertThrowsError(try Theme.decode(json: Data(##"{"formatVersion": 9, "name": "x", "dark": {}}"##.utf8), fallbackID: "x")) {
            XCTAssertEqual($0 as? Theme.FileError, .unsupportedVersion(9))
        }
    }

    func testOverridesApplyPerAppearanceAndRoundTrip() throws {
        var o = Theme.Overrides()
        XCTAssertTrue(o.isEmpty)
        o[.dark][.background] = Theme.Color(0x000000)
        o[.light][.caret] = Theme.Color(0xFF0000)
        let t = Theme.solarized.applying(o)
        XCTAssertEqual(t.color(.background, .dark), Theme.Color(0x000000))
        XCTAssertEqual(t.color(.background, .light), Theme.solarized.color(.background, .light), "the other appearance is untouched")
        XCTAssertEqual(t.color(.caret, .light), Theme.Color(0xFF0000))
        let data = try XCTUnwrap(o.encoded())
        XCTAssertEqual(Theme.Overrides.decoded(data), o)
        XCTAssertNil(Theme.Overrides.decoded(Data("[]".utf8)))
    }

    // MARK: the user's theme folder

    private func writeTheme(_ name: String, _ json: String) throws -> URL {
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let url = folder.appendingPathComponent(name)
        try Data(json.utf8).write(to: url)
        return url
    }

    func testLibraryReadsImportsExportsAndDeletesThemes() throws {
        _ = try writeTheme("paper.json", ##"{"name": "Paper", "light": {"background": "#FFFFF8"}}"##)
        _ = try writeTheme("broken.json", ##"{"name": "Broken", "dark": {"command": "nope"}}"##)
        _ = try writeTheme("nord.json", ##"{"id": "nord", "name": "My Nord", "dark": {"background": "#000000"}}"##)
        _ = try writeTheme("notes.txt", "ignored")
        let library = EditorThemeLibrary(directory: folder)
        XCTAssertEqual(library.userThemes.map(\.name), ["My Nord", "Paper"])
        XCTAssertEqual(library.theme(id: "user.nord")?.name, "My Nord", "a user theme never shadows a built-in")
        XCTAssertEqual(library.theme(id: "nord"), Theme.nord)
        XCTAssertEqual(library.problems.map(\.file), ["broken.json"])
        XCTAssertEqual(library.allThemes.count, Theme.builtIns.count + 2)

        // Import from elsewhere: validated, copied in, listed.
        let outside = FileManager.default.temporaryDirectory.appendingPathComponent("ink-\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: outside) }
        try Data(##"{"id": "ink", "name": "Ink", "dark": {"background": "#101010"}}"##.utf8).write(to: outside)
        let id = try library.importTheme(from: outside)
        XCTAssertEqual(id, "ink")
        XCTAssertEqual(library.theme(id: "ink")?.color(.background, .dark), Theme.Color(0x101010))
        XCTAssertTrue(FileManager.default.fileExists(atPath: folder.appendingPathComponent("ink.json").path))
        // An invalid file is refused and nothing is copied.
        try Data("{".utf8).write(to: outside)
        XCTAssertThrowsError(try library.importTheme(from: outside))

        // Export writes a complete file that imports as the same colours.
        let exported = FileManager.default.temporaryDirectory.appendingPathComponent("export-\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: exported) }
        try library.export(Theme.dracula, to: exported)
        let back = try Theme.decode(json: Data(contentsOf: exported), fallbackID: "y")
        XCTAssertEqual(back.light, Theme.dracula.light)
        XCTAssertEqual(back.dark, Theme.dracula.dark)

        try library.delete(id: "ink")
        XCTAssertNil(library.theme(id: "ink"))
        try library.delete(id: "flashtex") // built-ins cannot be deleted: a no-op
        XCTAssertNotNil(library.theme(id: "flashtex"))
    }

    // MARK: preferences

    func testThemePreferencesPersistRepairAndReset() {
        let library = EditorThemeLibrary(directory: folder)
        let p = EditorPreferences(defaults: defaults, themeLibrary: library)
        XCTAssertEqual(p.codeThemeID, "flashtex")
        XCTAssertEqual(p.lineHeight, 1.2, accuracy: 1e-9)
        XCTAssertTrue(p.ligatures); XCTAssertTrue(p.showLineNumbers); XCTAssertTrue(p.highlightCurrentLine); XCTAssertFalse(p.showInvisibles)
        p.codeThemeID = "dracula"
        var o = Theme.Overrides(); o[.dark][.caret] = Theme.Color(0x00FF00)
        p.themeOverrides = o
        p.lineHeight = 1.53 // snaps to 0.05 steps
        p.ligatures = false; p.showLineNumbers = false; p.highlightCurrentLine = false; p.showInvisibles = true
        XCTAssertEqual(p.lineHeight, 1.55, accuracy: 1e-9)
        XCTAssertEqual(p.resolvedTheme.color(.caret, .dark), Theme.Color(0x00FF00))
        XCTAssertEqual(p.resolvedTheme.color(.command, .dark), Theme.dracula.color(.command, .dark))

        let q = EditorPreferences(defaults: defaults, themeLibrary: library)
        XCTAssertEqual(q.snapshot, p.snapshot, "every theme and display setting survives a relaunch")
        q.lineHeight = 9; XCTAssertEqual(q.lineHeight, 2.0)
        q.lineHeight = .nan; XCTAssertEqual(q.lineHeight, 1.2, accuracy: 1e-9)
        q.codeThemeID = "  "; XCTAssertEqual(q.codeThemeID, "flashtex", "blank: the default")
        q.codeThemeID = "gone"
        XCTAssertEqual(q.codeThemeID, "gone", "an unknown id is kept (the file may come back)")
        XCTAssertEqual(q.resolvedTheme.light, Theme.flashtex.light, "…and draws FlashTeX's colours")

        defaults.set(42, forKey: EditorPreferences.Key.codeThemeID.storageKey)
        defaults.set("x", forKey: EditorPreferences.Key.themeOverrides.storageKey)
        defaults.set("tall", forKey: EditorPreferences.Key.lineHeight.storageKey)
        defaults.set("no", forKey: EditorPreferences.Key.ligatures.storageKey)
        let r = EditorPreferences(defaults: defaults, themeLibrary: library)
        XCTAssertEqual(r.codeThemeID, "flashtex")
        XCTAssertTrue(r.themeOverrides.isEmpty)
        XCTAssertEqual(r.lineHeight, 1.2, accuracy: 1e-9)
        XCTAssertTrue(r.ligatures)
        XCTAssertTrue(Set(r.lastLoadRepairs).isSuperset(of: [.codeThemeID, .themeOverrides, .lineHeight, .ligatures]))
        r.resetToDefaults()
        XCTAssertEqual(r.snapshot, EditorPreferences.defaultSnapshot)
    }

    func testUserThemeIsSelectableByID() throws {
        _ = try writeTheme("ink.json", ##"{"id": "ink", "name": "Ink", "dark": {"command": "#ABCDEF"}}"##)
        let library = EditorThemeLibrary(directory: folder)
        let p = EditorPreferences(defaults: defaults, themeLibrary: library)
        p.codeThemeID = "ink"
        XCTAssertEqual(p.resolvedTheme.color(.command, .dark), Theme.Color(0xABCDEF))
        XCTAssertEqual(p.resolvedTheme.color(.command, .light), Theme.Color(0xABCDEF), "one variant serves both")
    }

    func testLigaturesOffSwitchesTheFontFeaturesOff() {
        let on = EditorPreferences.resolveFont(family: nil, size: 13, ligatures: true)
        let off = EditorPreferences.resolveFont(family: nil, size: 13, ligatures: false)
        XCTAssertEqual(on.pointSize, off.pointSize)
        XCTAssertEqual(on.fontName, off.fontName)
        let features = off.fontDescriptor.object(forKey: .featureSettings) as? [[NSFontDescriptor.FeatureKey: Int]]
        XCTAssertFalse(features?.isEmpty ?? true, "ligature features switched off")
        XCTAssertNil(on.fontDescriptor.object(forKey: .featureSettings))
        // Only the shared instance recolours the app: a private one never installs.
        let p = EditorPreferences(defaults: defaults)
        let before = EditorThemeRuntime.generation
        p.codeThemeID = "nord"
        XCTAssertEqual(EditorThemeRuntime.generation, before)
    }

    // MARK: runtime colours

    private func resolve(_ color: NSColor, dark: Bool) -> Theme.Color {
        var out = Theme.Color(0)
        NSAppearance(named: dark ? .darkAqua : .aqua)!.performAsCurrentDrawingAppearance {
            let c = color.usingColorSpace(.sRGB)!
            out = EditorThemeTests.color(c)
        }
        return out
    }

    nonisolated static func color(_ c: NSColor) -> Theme.Color {
        func b(_ v: CGFloat) -> UInt8 { UInt8(max(0, min(255, (v * 255).rounded()))) }
        return Theme.Color(red: b(c.redComponent), green: b(c.greenComponent), blue: b(c.blueComponent), alpha: b(c.alphaComponent))
    }

    func testDynamicColoursFollowTheInstalledThemeWithoutNewObjects() {
        let command = SyntaxTheme.command
        let background = SyntaxTheme.background
        EditorThemeRuntime.install(.flashtex)
        XCTAssertEqual(resolve(command, dark: false), Theme.Color(0x0033B3))
        XCTAssertEqual(resolve(command, dark: true), Theme.Color(0xCF8E6D))
        let generation = EditorThemeRuntime.generation
        EditorThemeRuntime.install(.solarized)
        XCTAssertEqual(EditorThemeRuntime.generation, generation + 1)
        XCTAssertTrue(command === SyntaxTheme.command, "the same colour object: painted runs need no repaint")
        XCTAssertTrue(background === SyntaxTheme.background)
        XCTAssertEqual(resolve(command, dark: false), Theme.Color(0x859900))
        XCTAssertEqual(resolve(background, dark: true), Theme.Color(0x002B36))
        // A syntax role a theme leaves out resolves to the text colour.
        XCTAssertEqual(resolve(SyntaxTheme.color(for: .verbatim)!, dark: true), Theme.Color(0x839496))
        EditorThemeRuntime.install(.solarized) // unchanged: no bump, no notification
        XCTAssertEqual(EditorThemeRuntime.generation, generation + 1)
    }

    /// A theme switch on a live editor: colours change on the next draw,
    /// with no re-lex and no repaint of the syntax runs.
    func testThemeSwitchOnAHostedEditorRedrawsWithoutRelexOrRepaint() async throws {
        let text = String(repeating: "\\section{A} $x^2 + \\alpha$ % note\nplain words here\n", count: 200)
        let model = ShellModel()
        model.replaceProject(entryText: text)
        let probe = LargeDocumentEditorTests.Probe()
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled])
        window.contentView = NSHostingView(rootView: LargeDocumentEditorTests.Host(model: model, probe: probe, marks: []))
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }
        var tv: NSTextView?
        let deadline = Date().addingTimeInterval(20)
        while tv == nil, Date() < deadline {
            tv = TypingBenchDriver.findTextView(in: [window.contentView!])
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        let textView = try XCTUnwrap(tv)
        let co = try XCTUnwrap(textView.delegate as? SourceEditorView.Coordinator)
        try await Task.sleep(nanoseconds: 100_000_000)
        let paints = co.syntax.paints, runs = co.syntax.runsPainted

        EditorThemeRuntime.install(.github)
        EditorPreferences.shared.apply(to: textView)
        let lm = try XCTUnwrap(textView.layoutManager)
        let at = (text as NSString).range(of: "\\section").location
        let painted = try XCTUnwrap(lm.temporaryAttribute(.foregroundColor, atCharacterIndex: at, effectiveRange: nil) as? NSColor)
        XCTAssertTrue(painted === SyntaxTheme.command)
        XCTAssertEqual(resolve(painted, dark: false), Theme.Color(0xCF222E), "the painted run now draws GitHub's keyword red")
        XCTAssertEqual(resolve(textView.backgroundColor, dark: true), Theme.Color(0x0D1117))
        XCTAssertEqual(co.syntax.paints, paints, "no repaint")
        XCTAssertEqual(co.syntax.runsPainted, runs)
        XCTAssertEqual(co.syntax.highlighter.lastEditLinesLexed, 0, "no re-lex")
    }

    // MARK: Settings > Themes

    func testThemesPaneHostsWithLabelledControls() {
        let library = EditorThemeLibrary(directory: folder)
        let p = EditorPreferences(defaults: defaults, themeLibrary: library)
        let host = NSHostingView(rootView: EditorThemeSettingsView(preferences: p, library: library))
        host.frame = NSRect(x: 0, y: 0, width: 480, height: 900)
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        func controls(_ v: NSView) -> [NSControl] { (v as? NSControl).map { [$0] } ?? [] + v.subviews.flatMap(controls) }
        let found = controls(host)
        XCTAssertTrue(found.contains { $0 is NSPopUpButton }, "the theme picker")
        XCTAssertTrue(found.contains { $0 is NSSlider }, "line height")
        let kinds = found.map { String(describing: type(of: $0)) }
        XCTAssertEqual(kinds.filter { $0.contains("ColorWell") }.count, EditorColorTheme.Role.chromeRoles.count, "one colour well per chrome role: \(kinds)")
        XCTAssertEqual(kinds.filter { $0.contains("Switch") }.count, 4, "ligatures, line numbers, current line, invisibles: \(kinds)")
        XCTAssertTrue(kinds.contains { $0.contains("SegmentedControl") }, "the light/dark preview switch")
        p.codeThemeID = "nord"
        p.showInvisibles = true
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertEqual(p.codeThemeID, "nord")
        if let path = ProcessInfo.processInfo.environment["FLASHTEX_THEMES_EVIDENCE"] {
            window.appearance = NSAppearance(named: .darkAqua)
            host.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.3))
            if let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) {
                host.cacheDisplay(in: host.bounds, to: rep)
                try? rep.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: path))
            }
        }
        window.orderOut(nil)
    }
}
