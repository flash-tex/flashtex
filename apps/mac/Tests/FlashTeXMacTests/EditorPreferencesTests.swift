import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// `EditorPreferences`: defaults, clamping/validation, persistence round trip
/// through a temporary `UserDefaults` suite (with migration and repair of
/// absent/invalid values), `apply(to:)` on a real `NSTextView`, the observing
/// seam, appearance mapping and the Settings view hosting.
@MainActor
final class EditorPreferencesTests: XCTestCase {
    private var suiteName = ""
    private var defaults: UserDefaults!

    override func setUp() {
        super.setUp()
        suiteName = "flashtex.tests.EditorPreferences.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
        defaults.removePersistentDomain(forName: suiteName)
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        super.tearDown()
    }

    private func key(_ k: EditorPreferences.Key) -> String { k.storageKey }

    /// Some installed monospaced family other than the system face (Menlo
    /// ships with every macOS), and a proportional one.
    private var monoFamily: String { EditorPreferences.installedMonospacedFamilies().contains("Menlo") ? "Menlo" : EditorPreferences.installedMonospacedFamilies().first! }
    private let proportionalFamily = "Helvetica"

    // MARK: defaults

    func testDefaultsWhenNothingIsStored() {
        let p = EditorPreferences(defaults: defaults)
        XCTAssertEqual(p.snapshot, EditorPreferences.defaultSnapshot)
        XCTAssertNil(p.fontFamily)
        XCTAssertEqual(p.fontSize, 13)
        XCTAssertTrue(p.lineWrapping)
        XCTAssertEqual(p.tabWidth, 4)
        XCTAssertEqual(p.indentStyle, .spaces)
        XCTAssertEqual(p.appearance, .system)
        XCTAssertTrue(p.autoCloseBraces)
        XCTAssertTrue(p.completionPopup)
        XCTAssertEqual(p.indentString, "    ")
        // The system monospaced face at the default size.
        XCTAssertTrue(p.font.isFixedPitch)
        XCTAssertEqual(p.font.pointSize, 13)
        XCTAssertEqual(p.font, .monospacedSystemFont(ofSize: 13, weight: .regular))
        // Migration stamped the schema version and the absent keys were written as defaults.
        XCTAssertEqual(defaults.integer(forKey: EditorPreferences.schemaVersionKey), EditorPreferences.schemaVersion)
        XCTAssertEqual(Set(p.lastLoadRepairs), Set(EditorPreferences.Key.allCases).subtracting([.fontFamily]))
        XCTAssertEqual(defaults.double(forKey: key(.fontSize)), 13)
        XCTAssertNil(defaults.object(forKey: key(.fontFamily)), "the system face is stored as absence")
    }

    func testRangesAndDefaultsAreTheDocumentedOnes() {
        XCTAssertEqual(EditorPreferences.fontSizeRange, 8...36)
        XCTAssertEqual(EditorPreferences.tabWidthRange, 2...8)
        XCTAssertEqual(EditorPreferences.Key.fontSize.storageKey, "FlashTeX.EditorPreferences.v1.fontSize")
    }

    // MARK: clamping and validation

    func testFontSizeIsClamped() {
        let p = EditorPreferences(defaults: defaults)
        p.fontSize = 3; XCTAssertEqual(p.fontSize, 8)
        p.fontSize = 200; XCTAssertEqual(p.fontSize, 36)
        p.fontSize = 14.5; XCTAssertEqual(p.fontSize, 14.5)
        p.fontSize = .nan; XCTAssertEqual(p.fontSize, 13, "non-finite input keeps the default")
        p.fontSize = .infinity; XCTAssertEqual(p.fontSize, 13)
        XCTAssertEqual(EditorPreferences.clampedFontSize(-1), 8)
        XCTAssertEqual(EditorPreferences.clampedFontSize(36.0001), 36)
        XCTAssertEqual(defaults.double(forKey: key(.fontSize)), 13, "the clamped value is what gets persisted")
    }

    func testTabWidthIsClamped() {
        let p = EditorPreferences(defaults: defaults)
        p.tabWidth = 0; XCTAssertEqual(p.tabWidth, 2)
        p.tabWidth = 99; XCTAssertEqual(p.tabWidth, 8)
        p.tabWidth = 3; XCTAssertEqual(p.tabWidth, 3)
        XCTAssertEqual(p.indentString, "   ")
        p.indentStyle = .tabs
        XCTAssertEqual(p.indentString, "\t")
        XCTAssertEqual(defaults.integer(forKey: key(.tabWidth)), 3)
    }

    func testFontFamilyMustBeInstalledAndMonospaced() {
        let p = EditorPreferences(defaults: defaults)
        p.fontFamily = monoFamily
        XCTAssertEqual(p.fontFamily, monoFamily)
        XCTAssertEqual(p.font.familyName, monoFamily)
        XCTAssertTrue(p.font.isFixedPitch)
        XCTAssertEqual(defaults.string(forKey: key(.fontFamily)), monoFamily)

        p.fontFamily = proportionalFamily
        XCTAssertNil(p.fontFamily, "a proportional family falls back to the system face")
        XCTAssertNil(defaults.object(forKey: key(.fontFamily)))
        XCTAssertEqual(p.font, .monospacedSystemFont(ofSize: 13, weight: .regular))

        p.fontFamily = "No Such Font Family 9f3a"
        XCTAssertNil(p.fontFamily, "an uninstalled family falls back to the system face")
        p.fontFamily = ""
        XCTAssertNil(p.fontFamily)

        XCTAssertNil(EditorPreferences.validatedFontFamily(proportionalFamily))
        XCTAssertEqual(EditorPreferences.validatedFontFamily(monoFamily), monoFamily)
        XCTAssertFalse(EditorPreferences.installedMonospacedFamilies().contains(proportionalFamily))
        XCTAssertTrue(EditorPreferences.installedMonospacedFamilies().contains(monoFamily))
    }

    func testResolveFontFallsBackForUnavailableFamilyAtClampedSize() {
        let f = EditorPreferences.resolveFont(family: "No Such Font Family 9f3a", size: 100)
        XCTAssertEqual(f, .monospacedSystemFont(ofSize: 36, weight: .regular))
        let m = EditorPreferences.resolveFont(family: monoFamily, size: 20)
        XCTAssertEqual(m.familyName, monoFamily)
        XCTAssertEqual(m.pointSize, 20)
    }

    func testGenerationCountsOnlyActualChanges() {
        let p = EditorPreferences(defaults: defaults)
        let g0 = p.generation
        p.fontSize = 13
        XCTAssertEqual(p.generation, g0, "setting the current value is a no-op")
        p.fontSize = 5 // clamps to 8
        XCTAssertEqual(p.generation, g0 + 1)
        p.fontSize = 2 // clamps to 8 again
        XCTAssertEqual(p.generation, g0 + 1)
        p.lineWrapping.toggle()
        XCTAssertEqual(p.generation, g0 + 2)
    }

    // MARK: persistence

    func testRoundTripThroughTemporarySuite() {
        let a = EditorPreferences(defaults: defaults)
        a.fontFamily = monoFamily
        a.fontSize = 17
        a.lineWrapping = false
        a.tabWidth = 2
        a.indentStyle = .tabs
        a.appearance = .dark
        a.autoCloseBraces = false
        a.completionPopup = false
        let expected = a.snapshot

        // A second instance over the same suite (what a relaunch does).
        let b = EditorPreferences(defaults: UserDefaults(suiteName: suiteName)!)
        XCTAssertEqual(b.snapshot, expected)
        XCTAssertTrue(b.lastLoadRepairs.isEmpty, "a valid stored set needs no repair")
        XCTAssertEqual(b.fontFamily, monoFamily)
        XCTAssertEqual(b.fontSize, 17)
        XCTAssertFalse(b.lineWrapping)
        XCTAssertEqual(b.tabWidth, 2)
        XCTAssertEqual(b.indentStyle, .tabs)
        XCTAssertEqual(b.appearance, .dark)
        XCTAssertFalse(b.autoCloseBraces)
        XCTAssertFalse(b.completionPopup)
    }

    func testInvalidStoredValuesAreRepairedAndWrittenBack() {
        defaults.set(EditorPreferences.schemaVersion, forKey: EditorPreferences.schemaVersionKey)
        defaults.set(proportionalFamily, forKey: key(.fontFamily))
        defaults.set(999.0, forKey: key(.fontSize))
        defaults.set("yes", forKey: key(.lineWrapping)) // wrong type
        defaults.set(1, forKey: key(.tabWidth))
        defaults.set("elastic", forKey: key(.indentStyle))
        defaults.set("sepia", forKey: key(.appearance))
        defaults.set(false, forKey: key(.autoCloseBraces))
        defaults.set(2, forKey: key(.completionPopup)) // wrong type

        let p = EditorPreferences(defaults: defaults)
        XCTAssertNil(p.fontFamily)
        XCTAssertEqual(p.fontSize, 36)
        XCTAssertTrue(p.lineWrapping)
        XCTAssertEqual(p.tabWidth, 2)
        XCTAssertEqual(p.indentStyle, .spaces)
        XCTAssertEqual(p.appearance, .system)
        XCTAssertFalse(p.autoCloseBraces, "the one valid value survives")
        XCTAssertTrue(p.completionPopup)
        XCTAssertEqual(Set(p.lastLoadRepairs), Set(EditorPreferences.Key.allCases).subtracting([.autoCloseBraces]))
        // Written back as valid values.
        XCTAssertNil(defaults.object(forKey: key(.fontFamily)))
        XCTAssertEqual(defaults.double(forKey: key(.fontSize)), 36)
        XCTAssertEqual(defaults.object(forKey: key(.lineWrapping)) as? Bool, true)
        XCTAssertEqual(defaults.integer(forKey: key(.tabWidth)), 2)
        XCTAssertEqual(defaults.string(forKey: key(.indentStyle)), "spaces")
        XCTAssertEqual(defaults.string(forKey: key(.appearance)), "system")
        XCTAssertEqual(defaults.object(forKey: key(.completionPopup)) as? Bool, true)
    }

    func testMigrationFromAbsentVersionStampsAndIgnoresUnversionedKeys() {
        defaults.set(30.0, forKey: "FlashTeX.EditorPreferences.fontSize") // an unversioned stray key
        XCTAssertEqual(defaults.integer(forKey: EditorPreferences.schemaVersionKey), 0)
        EditorPreferences.migrate(defaults)
        XCTAssertEqual(defaults.integer(forKey: EditorPreferences.schemaVersionKey), 1)
        let p = EditorPreferences(defaults: defaults)
        XCTAssertEqual(p.fontSize, 13, "unversioned keys are not read")
        // A newer stamp is never downgraded.
        defaults.set(EditorPreferences.schemaVersion + 5, forKey: EditorPreferences.schemaVersionKey)
        EditorPreferences.migrate(defaults)
        XCTAssertEqual(defaults.integer(forKey: EditorPreferences.schemaVersionKey), EditorPreferences.schemaVersion + 5)
        let q = EditorPreferences(defaults: defaults)
        XCTAssertEqual(q.fontSize, 13, "values this version understands are still read")
    }

    func testResetToDefaultsPersists() {
        let p = EditorPreferences(defaults: defaults)
        p.fontSize = 20; p.tabWidth = 8; p.appearance = .light; p.fontFamily = monoFamily
        p.resetToDefaults()
        XCTAssertEqual(p.snapshot, EditorPreferences.defaultSnapshot)
        XCTAssertEqual(EditorPreferences(defaults: defaults).snapshot, EditorPreferences.defaultSnapshot)
    }

    // MARK: apply(to:) on a real NSTextView

    private func makeTextView(text: String = "a\tb\n\\section{x}\n") -> (NSScrollView, NSTextView) {
        let scroll = CompletingTextView.scrollable()
        scroll.frame = NSRect(x: 0, y: 0, width: 300, height: 200)
        let tv = scroll.documentView as! NSTextView
        _ = tv.layoutManager
        tv.isRichText = false
        tv.string = text
        return (scroll, tv)
    }

    func testApplySetsFontTabIntervalWrappingAndAppearance() {
        let p = EditorPreferences(defaults: defaults)
        let (scroll, tv) = makeTextView()
        p.fontFamily = monoFamily
        p.fontSize = 16
        p.tabWidth = 8
        p.lineWrapping = false
        p.appearance = .dark
        p.apply(to: tv)

        XCTAssertEqual(tv.font?.familyName, monoFamily)
        XCTAssertEqual(tv.font?.pointSize, 16)
        XCTAssertEqual((tv.typingAttributes[.font] as? NSFont)?.pointSize, 16)
        // The whole storage is re-fonted (one run).
        var runs = 0
        tv.textStorage!.enumerateAttribute(.font, in: NSRange(location: 0, length: tv.textStorage!.length)) { value, _, _ in
            runs += 1
            XCTAssertEqual((value as? NSFont)?.pointSize, 16)
        }
        XCTAssertEqual(runs, 1)

        let expectedInterval = EditorPreferences.tabInterval(columns: 8, font: p.font)
        XCTAssertGreaterThan(expectedInterval, 0)
        XCTAssertEqual(tv.defaultParagraphStyle?.defaultTabInterval, expectedInterval)
        XCTAssertEqual(tv.defaultParagraphStyle?.tabStops.count, 0)
        XCTAssertEqual((tv.typingAttributes[.paragraphStyle] as? NSParagraphStyle)?.defaultTabInterval, expectedInterval)
        let stored = tv.textStorage!.attribute(.paragraphStyle, at: 0, effectiveRange: nil) as? NSParagraphStyle
        XCTAssertEqual(stored?.defaultTabInterval, expectedInterval)
        // 8 columns is twice 4 columns in the same font.
        XCTAssertEqual(expectedInterval, 2 * EditorPreferences.tabInterval(columns: 4, font: p.font), accuracy: 0.001)

        XCTAssertFalse(tv.textContainer!.widthTracksTextView)
        XCTAssertTrue(tv.isHorizontallyResizable)
        XCTAssertTrue(scroll.hasHorizontalScroller)
        XCTAssertEqual(tv.textContainer!.containerSize.width, CGFloat.greatestFiniteMagnitude)

        XCTAssertEqual(scroll.appearance?.name, .darkAqua)
        XCTAssertEqual(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]), .darkAqua)

        // Back to wrapping and the system appearance.
        p.lineWrapping = true
        p.appearance = .system
        p.apply(to: tv)
        XCTAssertTrue(tv.textContainer!.widthTracksTextView)
        XCTAssertFalse(tv.isHorizontallyResizable)
        XCTAssertFalse(scroll.hasHorizontalScroller)
        XCTAssertEqual(tv.textContainer!.containerSize.width, scroll.contentView.bounds.width)
        XCTAssertNil(scroll.appearance)
        p.appearance = .light
        p.apply(to: tv)
        XCTAssertEqual(scroll.appearance?.name, .aqua)
    }

    func testApplyDoesNotChangeTextUndoOrTemporaryAttributes() {
        let p = EditorPreferences(defaults: defaults)
        let (_, tv) = makeTextView()
        tv.allowsUndo = true
        let before = tv.string
        final class Probe: NSObject, NSTextViewDelegate {
            var changes = 0
            let undo = UndoManager()
            func textDidChange(_ notification: Notification) { changes += 1 }
            func undoManager(for view: NSTextView) -> UndoManager? { undo }
        }
        let probe = Probe()
        tv.delegate = probe
        tv.layoutManager!.addTemporaryAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, forCharacterRange: NSRange(location: 0, length: 1))

        p.fontSize = 24
        p.tabWidth = 2
        p.apply(to: tv)
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))

        XCTAssertEqual(tv.string, before)
        XCTAssertEqual(probe.changes, 0, "attribute-only edits post no text change")
        XCTAssertTrue(tv.undoManager === probe.undo)
        XCTAssertFalse(probe.undo.canUndo, "no undo step")
        XCTAssertNotNil(tv.layoutManager!.temporaryAttribute(.underlineStyle, atCharacterIndex: 0, effectiveRange: nil))
        XCTAssertEqual(tv.font?.pointSize, 24)
    }

    func testApplyWithoutScrollViewStillWorks() {
        let p = EditorPreferences(defaults: defaults)
        let tv = NSTextView(frame: NSRect(x: 0, y: 0, width: 200, height: 100))
        tv.string = "x"
        p.lineWrapping = false
        p.appearance = .dark
        p.apply(to: tv)
        XCTAssertFalse(tv.textContainer!.widthTracksTextView)
        XCTAssertEqual(tv.appearance?.name, .darkAqua)
        XCTAssertEqual(tv.font, p.font)
    }

    func testObserveApplyingReappliesOnChange() async throws {
        let p = EditorPreferences(defaults: defaults)
        let (_, tv) = makeTextView()
        let token = p.observeApplying(to: tv)
        XCTAssertEqual(tv.font?.pointSize, 13)
        p.fontSize = 21
        try await waitUntil { tv.font?.pointSize == 21 }
        p.lineWrapping = false
        try await waitUntil { tv.textContainer?.widthTracksTextView == false }
        p.tabWidth = 6
        try await waitUntil { tv.defaultParagraphStyle?.defaultTabInterval == EditorPreferences.tabInterval(columns: 6, font: p.font) }
        token.cancel()
        p.fontSize = 9
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertEqual(tv.font?.pointSize, 21, "a cancelled token stops applying")
    }

    private func waitUntil(timeout: TimeInterval = 2, _ condition: @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !condition() {
            if Date() > deadline { XCTFail("timed out waiting"); return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    // MARK: appearance mapping

    func testAppearanceMapping() {
        XCTAssertNil(EditorPreferences.Appearance.system.nsAppearance)
        XCTAssertEqual(EditorPreferences.Appearance.light.nsAppearance?.name, .aqua)
        XCTAssertEqual(EditorPreferences.Appearance.dark.nsAppearance?.name, .darkAqua)
        XCTAssertNil(EditorPreferences.Appearance.system.colorScheme)
        XCTAssertEqual(EditorPreferences.Appearance.light.colorScheme, .light)
        XCTAssertEqual(EditorPreferences.Appearance.dark.colorScheme, .dark)
        // Preview dark toggle default: fixed for light/dark, follows the system otherwise.
        XCTAssertFalse(EditorPreferences.Appearance.light.prefersDarkPreview(systemIsDark: true))
        XCTAssertTrue(EditorPreferences.Appearance.dark.prefersDarkPreview(systemIsDark: false))
        XCTAssertTrue(EditorPreferences.Appearance.system.prefersDarkPreview(systemIsDark: true))
        XCTAssertFalse(EditorPreferences.Appearance.system.prefersDarkPreview(systemIsDark: false))
        let p = EditorPreferences(defaults: defaults)
        p.appearance = .dark
        XCTAssertTrue(p.darkPreviewDefault)
        p.appearance = .light
        XCTAssertFalse(p.darkPreviewDefault)
        p.appearance = .system
        XCTAssertEqual(p.darkPreviewDefault, EditorPreferences.systemAppearanceIsDark())
        XCTAssertEqual(EditorPreferences.Appearance.allCases.map(\.rawValue), ["system", "light", "dark"])
    }

    // MARK: settings view

    func testSettingsViewHostsAndReflectsChanges() {
        let p = EditorPreferences(defaults: defaults)
        let host = NSHostingView(rootView: EditorPreferencesView(preferences: p))
        host.frame = NSRect(x: 0, y: 0, width: 480, height: 600)
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertGreaterThan(host.fittingSize.height, 0)
        // Every control is a focusable AppKit control (keyboard access) and the labelled elements are there.
        func controls(_ v: NSView) -> [NSControl] { (v as? NSControl).map { [$0] } ?? [] + v.subviews.flatMap(controls) }
        let found = controls(host)
        XCTAssertFalse(found.isEmpty)
        XCTAssertTrue(found.contains { $0 is NSButton }, "toggles / buttons are NSButtons")
        // Model changes flow into the hosted view without crashing.
        p.fontSize = 30
        p.appearance = .dark
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertEqual(p.fontSize, 30)
        window.orderOut(nil)
    }

    /// Evidence: renders the settings view (with non-default values) to the
    /// PNG named by FLASHTEX_PREFS_EVIDENCE. Off-screen, never key.
    func testWritesSettingsViewEvidenceWhenRequested() throws {
        guard let path = ProcessInfo.processInfo.environment["FLASHTEX_PREFS_EVIDENCE"] else {
            throw XCTSkip("set FLASHTEX_PREFS_EVIDENCE=<png path> to render the settings view")
        }
        let p = EditorPreferences(defaults: defaults)
        p.fontFamily = monoFamily; p.fontSize = 15; p.tabWidth = 2; p.appearance = .dark
        let host = NSHostingView(rootView: EditorPreferencesView(preferences: p))
        host.frame = NSRect(x: 0, y: 0, width: 480, height: 660)
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.title = "Editor Preferences"
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.3))
        let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: rep)
        let png = try XCTUnwrap(rep.representation(using: .png, properties: [:]))
        try png.write(to: URL(fileURLWithPath: path))
        XCTAssertGreaterThan(png.count, 1000)
        window.orderOut(nil)
    }
}
