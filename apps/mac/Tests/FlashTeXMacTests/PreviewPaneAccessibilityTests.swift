import AppKit
import XCTest
import FlashTeXAccessibility
@testable import FlashTeXMac

/// VoiceOver over the preview pane (task ux-preview-pane-voiceover): Page
/// Up/Down stepping whole pages (PreviewAnchor.swift's rule), the Pages
/// rotor's search rules, the page-jump announcement (PreviewAnnouncements.swift)
/// and the pane's container label. The engine's pane itself is exercised by
/// EngineV3PreviewNavTests and EngineV3AccessibilityTests; the old engine's
/// v1 and v2 panes, their page tree and their compile announcer retired with
/// it (#1236 S6).
///
/// SwiftUI materialises its own accessibility modifiers only for an
/// assistive client (see PanelAccessibilityTests), so the pane's
/// `.accessibilityLabel` is pinned at the source level here, the way
/// `CommandTableTests` pins the focus order.
@MainActor
final class PreviewPaneAccessibilityTests: XCTestCase {
    static let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Sources")

    // MARK: Page Up / Page Down

    func testPageStepTargetsWholePages() {
        let layout = PreviewPageLayout(pages: (1...3).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }, scale: 0.5)
        XCTAssertEqual(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 1, fraction: 0, horizontal: 0), layout: layout), 2)
        XCTAssertEqual(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 2, fraction: 0.7, horizontal: 0), layout: layout), 3)
        XCTAssertNil(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 3, fraction: 0.2, horizontal: 0), layout: layout), "already on the last page")
        XCTAssertNil(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 1, fraction: 0, horizontal: 0), layout: layout), "first page at its top")
        XCTAssertEqual(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 2, fraction: 0, horizontal: 0), layout: layout), 1)
        XCTAssertEqual(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 2, fraction: 0.005, horizontal: 0), layout: layout), 1, "within the slack: previous page")
        XCTAssertEqual(PreviewPageStep.target(.up, anchor: PreviewAnchor(page: 2, fraction: 0.5, horizontal: 0), layout: layout), 2, "mid-page: this page's top first")
        XCTAssertNil(PreviewPageStep.target(.down, anchor: PreviewAnchor(page: 9, fraction: 0, horizontal: 0), layout: layout), "unknown page")
    }

    // MARK: the Pages rotor

    func testPagesRotorSearchFollowsAppKitSemantics() {
        let layout = PreviewPageLayout(pages: (1...4).map { PreviewPageLayout.Page(number: $0, widthPt: 612, heightPt: 792) }, scale: 0.5)
        let items = PreviewPagesRotor.items(layout: layout, elided: [3])
        XCTAssertEqual(items.map(\.label), ["Page 1 of 4", "Page 2 of 4", "Page 3 of 4, not loaded", "Page 4 of 4"])
        XCTAssertEqual(PreviewPagesRotor.label(number: 2, totalPages: 0, elided: false), "Page 2")
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: ""), items[0])
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: false, filter: ""), items[3])
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: 2, forward: true, filter: ""), items[2], "strictly after the current page")
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: 2, forward: false, filter: ""), items[0])
        XCTAssertNil(PreviewPagesRotor.resolve(items: items, start: 4, forward: true, filter: ""), "no wrap-around")
        XCTAssertNil(PreviewPagesRotor.resolve(items: items, start: 1, forward: false, filter: ""))
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: "not loaded"), items[2], "type-ahead on the label")
        XCTAssertEqual(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: "PAGE 4"), items[3])
        XCTAssertNil(PreviewPagesRotor.resolve(items: items, start: nil, forward: true, filter: "page 9"))
        // A current item names its page by its target view or by its loading token.
        let token = NSAccessibilityCustomRotor.SearchParameters()
        token.currentItem = NSAccessibilityCustomRotor.ItemResult(itemLoadingToken: NSNumber(value: 3), customLabel: "Page 3 of 4")
        XCTAssertEqual(PreviewPagesRotor.start(of: token), 3)
        XCTAssertNil(PreviewPagesRotor.start(of: NSAccessibilityCustomRotor.SearchParameters()))
    }

    // MARK: announcements

    /// What the preview says by itself is a keyboard page jump, low priority,
    /// every time (a rotor load lets VoiceOver read the page instead). A
    /// completed compile is spoken by the diagnostics announcer
    /// (DiagnosticsPanelVoiceOverTests).
    func testAnnouncerTransitionTable() {
        let announcer = PreviewAnnouncer()
        var posted: [(String, NSAccessibilityPriorityLevel)] = []
        announcer.post = { posted.append(($0, $1)) }
        announcer.notePageJump(page: 2, of: 4)
        XCTAssertEqual(posted.map(\.0), ["Page 2 of 4"])
        announcer.notePageJump(page: 2, of: 4)
        XCTAssertEqual(posted.map(\.0), ["Page 2 of 4", "Page 2 of 4"], "the same jump again is spoken again")
        announcer.notePageJump(page: 3, of: 0)
        XCTAssertEqual(posted.last?.0, "Page 3", "an unknown total is left out")
        XCTAssertTrue(posted.allSatisfy { $0.1 == .low })
        XCTAssertEqual(announcer.announcements, posted.map(\.0))
    }

    // MARK: the pane's container and keyboard focus, pinned at the source

    /// There is one pane, the engine's (`PreviewV3Pane`); it is the labelled
    /// container and takes keyboard focus for Page Up / Page Down.
    func testPreviewPaneIsALabelledContainerAndBothPanesTakeFocusForPageKeys() throws {
        XCTAssertEqual(PreviewPane.accessibilityLabel, "PDF preview")
        XCTAssertEqual(PreviewPane.accessibilityValue(page: 2, of: 5), "Page 2 of 5")
        XCTAssertEqual(PreviewPane.accessibilityValue(page: 9, of: 5), "Page 5 of 5", "clamped like the HUD readout")
        XCTAssertNil(PreviewPane.accessibilityValue(page: 1, of: 0), "no pages, no value")

        let content = try String(contentsOf: Self.sources.appendingPathComponent("FlashTeXMac/ContentView.swift"), encoding: .utf8)
        let pane = try XCTUnwrap(content.range(of: "struct PreviewPane: View {"))
        let paneBody = content[pane.lowerBound...]
        let next = paneBody.dropFirst().range(of: "\nstruct ")?.lowerBound ?? paneBody.endIndex
        let paneSource = paneBody[..<next]
        XCTAssertTrue(paneSource.contains("PreviewV3Pane()"), "the pane shows the engine's pages")
        XCTAssertTrue(paneSource.contains(".accessibilityElement(children: .contain)"), "the pane is one container")
        XCTAssertTrue(paneSource.contains(".accessibilityLabel(Self.accessibilityLabel)"))
        XCTAssertTrue(paneSource.contains(".accessibilityValue(Self.accessibilityValue(page: model.previewVisiblePage,"))
        // The value counts the engine's pages (#1340): `previewPageCount`
        // (EngineV3ChromeTests.testTheReadoutAndThePaneValueCountTheV3Pages pins its behaviour).
        XCTAssertTrue(paneSource.contains("of: model.previewPageCount)"))

        // The engine's pages view takes focus from the keyboard and VoiceOver
        // (never from a click), steps whole pages on Page Down / Page Up and
        // announces each landing (EngineV3PreviewNavTests pins the stepping).
        XCTAssertTrue(EngineV3PagesView.acceptsFocus(eventType: .keyDown, voiceOver: false), "Tab gives the pane focus")
        XCTAssertTrue(EngineV3PagesView.acceptsFocus(eventType: nil, voiceOver: true), "VoiceOver gives the pane focus")
        XCTAssertFalse(EngineV3PagesView.acceptsFocus(eventType: .leftMouseDown, voiceOver: false), "a click leaves the editor focused")
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 121, modifiers: []), .page(.down), "Page Down steps pages")
        XCTAssertEqual(EngineV3PagesView.keyAction(keyCode: 116, modifiers: []), .page(.up), "Page Up steps pages")
        let v3 = try String(contentsOf: Self.sources.appendingPathComponent("FlashTeXMac/EngineV3Preview.swift"), encoding: .utf8)
        XCTAssertTrue(v3.contains("model.previewAnnouncer.notePageJump("), "the pane's page jumps are announced")

        // The help window and the focus-order table describe what ships.
        let preview = try XCTUnwrap(FocusOrder.panes.first { $0.name == "Preview" })
        XCTAssertTrue(preview.contents.contains("“PDF preview”"))
        XCTAssertTrue(preview.contents.contains("Page Down / Page Up"))
        let note = try XCTUnwrap(AccessibilityHelpView.voiceOverNotes.first { $0.hasPrefix("Preview:") })
        XCTAssertTrue(note.contains("“PDF preview”"))
        XCTAssertTrue(note.contains("Page Down / Page Up"))
        XCTAssertTrue(note.contains("Pages rotor"))
    }
}
