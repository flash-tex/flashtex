import XCTest
import SwiftUI
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXAccessibility

/// What can be checked about the SwiftUI attachment without VoiceOver or
/// Accessibility permission: the overlay's element order, labels, and frames.
final class OverlayTests: XCTestCase {
    func loadMultipage() throws -> RuntimeV1.CompileResult {
        let url = DocumentModelTests.samples.appendingPathComponent("multipage-result.json")
        return try RuntimeV1.decodeCompileResult(Data(contentsOf: url)).payload
    }

    func testOverlaySlotsFollowReadingOrderWithDescendingPriority() throws {
        let res = try loadMultipage()
        let overlay = AccessibilityOverlay(page: res.pages[0], totalPages: 2, scale: 0.5) { _, _ in }
        XCTAssertEqual(overlay.slots.map(\.element.text), ["Introduction", "A", "naïve", "approach", "fails."])
        XCTAssertEqual(overlay.slots.map(\.priority), [5, 4, 3, 2, 1])
        XCTAssertEqual(overlay.slots.map(\.id), [0, 1, 2, 3, 4], "ids are page item indices")
        XCTAssertEqual(overlay.pageLabel, "Page 1 of 2, 2 lines")
        XCTAssertEqual(AccessibilityOverlay(page: res.pages[1], scale: 1) { _, _ in }.pageLabel, "Page 2, 3 lines")
    }

    func testOverlayFramesTrackItemGeometryAndScale() throws {
        let res = try loadMultipage()
        guard case .text(let intro) = res.pages[0].items[0] else { return XCTFail() }
        let full = AccessibilityOverlay(page: res.pages[0], scale: 1) { _, _ in }
        let half = AccessibilityOverlay(page: res.pages[0], scale: 0.5) { _, _ in }
        let f = full.slots[0].frame, h = half.slots[0].frame
        XCTAssertEqual(f.minX, intro.xPt)
        XCTAssertLessThan(f.minY, intro.baselineYPt, "box starts above the baseline")
        XCTAssertGreaterThan(f.maxY, intro.baselineYPt, "and ends below it (descender)")
        XCTAssertGreaterThan(f.width, intro.fontSizePt * 4, "twelve glyphs of Times at 17 pt")
        XCTAssertEqual(h.minX, f.minX / 2, accuracy: 0.001)
        XCTAssertEqual(h.width, f.width / 2, accuracy: 0.001)
        XCTAssertEqual(h.height, f.height / 2, accuracy: 0.001)
        // A rule item gets the RuleConvention rectangle.
        let rule = try JSONDecoder().decode(RuntimeV1.PageItem.self, from: Data(
            "{\"kind\":\"text\",\"text\":\"──\",\"x_pt\":100,\"baseline_y_pt\":97.36,\"font_size_pt\":8.4}".utf8))
        var page = res.pages[0]
        page.items = [rule]
        let r = AccessibilityOverlay(page: page, scale: 2) { _, _ in }.slots[0].frame
        XCTAssertEqual(r.minX, 200)
        XCTAssertEqual(r.width, 2 * RuleConvention.advanceEm * 8.4 * 2, accuracy: 0.001, "2 segments × 0.5 em × 8.4 pt × scale 2")
        XCTAssertEqual(r.height, RuleConvention.thicknessEm * 8.4 * 2, accuracy: 0.001)
        // The preview's face resolver changes the measured width; an unknown name falls back to Times.
        let helvetica = AccessibilityOverlay(page: res.pages[0], scale: 1, fontName: { _ in "Helvetica" }) { _, _ in }.slots[0].frame
        XCTAssertNotEqual(helvetica.width, f.width)
        let unknown = AccessibilityOverlay(page: res.pages[0], scale: 1, fontName: { "NoSuchFace-\($0)" }) { _, _ in }.slots[0].frame
        XCTAssertEqual(unknown.width, f.width)
    }

    func testOverlayActionForwardsSourceAndText() throws {
        let res = try loadMultipage()
        var received: (RuntimeV1.SourceRange?, String?)?
        let overlay = AccessibilityOverlay(page: res.pages[1], scale: 1) { received = ($0, $1) }
        // The action closure is what VoiceOver's "Go to source" invokes; drive it directly.
        let slot = overlay.slots[1]
        overlay.onSelect(slot.element.source, slot.element.text)
        XCTAssertEqual(received?.0, .init(path: "main.tex", startByte: 115, endByte: 123))
        XCTAssertEqual(received?.1, "Résumé")
        XCTAssertEqual(slot.element.actions, ["Go to source"])
    }

    func testModifiersAttach() {
        // Compile-time attachment check: the modifiers accept the shell's call shapes.
        let d = try! JSONDecoder().decode(RuntimeV1.Diagnostic.self, from: Data("{\"severity\":\"error\",\"message\":\"m\"}".utf8))
        _ = Text("row").accessibleDiagnostic(d, index: 0, total: 1) {}
        _ = Text("bar").accessibleCaptureBar(anchor: nil, proposals: 0)
        _ = AccessibilityHelpView()
    }

    // MARK: the lazy tree, read back through the NSAccessibility protocol

    /// Hosts one `PageAXView` in an ordered-out window (never key) at the
    /// page's size, as the preview does, and hands the page to it.
    func hostPage(_ page: RuntimeV1.Page, totalPages: Int, scale: CGFloat = 1,
                  onSelect: @escaping (RuntimeV1.SourceRange?, String?) -> Void = { _, _ in }) -> (NSWindow, PageAXView) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 40, y: 40, width: page.widthPt * scale, height: page.heightPt * scale),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        let view = PageAXView(frame: window.contentView!.bounds)
        window.contentView!.addSubview(view)
        view.update(page: page, totalPages: totalPages, scale: scale, fontName: { _ in "Times-Roman" }, onSelect: onSelect)
        return (window, view)
    }

    func testPageIsALandmarkWhoseLinesAreGroupsOfItems() throws {
        let res = try loadMultipage()
        let (window, view) = hostPage(res.pages[1], totalPages: 2)
        defer { window.orderOut(nil) }
        // Nothing is built until an assistive client asks.
        XCTAssertFalse(view.hasBuiltTree)
        XCTAssertTrue(view.isAccessibilityElement())
        XCTAssertEqual(view.accessibilityRole(), .group)
        XCTAssertEqual(view.accessibilitySubrole(), PreviewAccessibility.landmarkSubrole)
        XCTAssertEqual(view.accessibilitySubrole()?.rawValue, "AXLandmarkRegion")
        XCTAssertEqual(view.accessibilityRoleDescription(), "page")
        XCTAssertEqual(view.accessibilityLabel(), "Page 2 of 2, 3 lines")
        XCTAssertFalse(view.hasBuiltTree, "the label alone does not build the tree")

        let lines = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(view.hasBuiltTree)
        XCTAssertEqual(lines.map { $0.accessibilityRole() }, [.group, .group, .group])
        XCTAssertEqual(lines.map { $0.accessibilityRoleDescription() }, ["line", "line", "line"])
        XCTAssertEqual(lines.map { $0.accessibilityLabel() },
                       ["Page 2, line 1: Method", "Page 2, line 2: Résumé of the steps.", "Page 2, line 3: oops"])
        XCTAssertTrue(lines.allSatisfy { $0.accessibilityParent() as AnyObject === view })

        let items = try XCTUnwrap(lines[1].accessibilityChildren() as? [PreviewAXElement])
        XCTAssertEqual(items.map { $0.accessibilityRole() }, [.staticText, .staticText])
        XCTAssertEqual(items.map { $0.accessibilityLabel() }, ["Résumé", "of the steps."])
        XCTAssertEqual(items.map { $0.accessibilityHelp() }, Array(repeating: "Page 2, line 2", count: 2))
        XCTAssertTrue(items.allSatisfy { $0.accessibilityParent() as AnyObject === lines[1] })
        let model = AccessibleDocumentModel(result: res)
        XCTAssertEqual(items.map { $0.accessibilityValue() as? String }, model.pages[1].lines[1].elements.map(\.value))
        // Same order as the flat overlay slots (reading order).
        let overlay = AccessibilityOverlay(page: res.pages[1], totalPages: 2, scale: 1) { _, _ in }
        let flat = try lines.flatMap { try XCTUnwrap($0.accessibilityChildren() as? [PreviewAXElement]) }
        XCTAssertEqual(flat.map { $0.accessibilityLabel() }, overlay.slots.map(\.element.label))
        XCTAssertEqual(flat.count, 4, "one element per text item on the page")
    }

    func testNavigationOrderIsTheArrayOrder() throws {
        let res = try loadMultipage()
        let (window, view) = hostPage(res.pages[0], totalPages: 2)
        defer { window.orderOut(nil) }
        let children = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        let nav = try XCTUnwrap(view.accessibilityChildrenInNavigationOrder())
        XCTAssertEqual(nav.count, children.count)
        XCTAssertTrue(zip(nav, children).allSatisfy { $0 as AnyObject === $1 }, "navigation order is the children array")
        for line in children {
            let items = try XCTUnwrap(line.accessibilityChildren() as? [PreviewAXElement])
            let lineNav = try XCTUnwrap(line.accessibilityChildrenInNavigationOrder())
            XCTAssertEqual(lineNav.count, items.count)
            XCTAssertTrue(zip(lineNav, items).allSatisfy { $0 as AnyObject === $1 })
        }
        // Elements adopted the NSAccessibilityElement protocol at runtime (the
        // typed navigation-order array would be empty otherwise).
        XCTAssertTrue(PreviewAXElement.adoptsElementProtocol)
        XCTAssertTrue((children[0] as AnyObject) is NSAccessibilityElementProtocol)
    }

    func testGoToSourceActionOnlyWhereThereIsASourceAndItForwardsTheClickHandler() throws {
        var res = try loadMultipage()
        // Strip the source of the last item ("oops") so the page has an unmapped item.
        guard case .text(var oopsItem) = res.pages[1].items[3] else { return XCTFail() }
        oopsItem.source = nil
        res.pages[1].items[3] = .text(oopsItem)
        var received: [(RuntimeV1.SourceRange?, String?)] = []
        let (window, view) = hostPage(res.pages[1], totalPages: 2) { received.append(($0, $1)) }
        defer { window.orderOut(nil) }
        let lines = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        let model = AccessibleDocumentModel(result: res)
        for (li, line) in lines.enumerated() {
            let items = try XCTUnwrap(line.accessibilityChildren() as? [PreviewAXElement])
            for (ii, item) in items.enumerated() {
                let element = model.pages[1].lines[li].elements[ii]
                let actions = item.accessibilityCustomActions() ?? []
                XCTAssertEqual(actions.map(\.name), element.actions, "\(element.text)")
            }
        }
        let resume = try XCTUnwrap((lines[1].accessibilityChildren() as? [PreviewAXElement])?.first)
        let action = try XCTUnwrap(resume.accessibilityCustomActions()?.first)
        XCTAssertEqual(action.name, "Go to source")
        XCTAssertTrue(action.handler?() ?? false)
        XCTAssertEqual(received.count, 1)
        XCTAssertEqual(received.first?.0, .init(path: "main.tex", startByte: 115, endByte: 123))
        XCTAssertEqual(received.first?.1, "Résumé")
        // The unmapped item offers no action and says so in its value.
        let oops = try XCTUnwrap((lines[2].accessibilityChildren() as? [PreviewAXElement])?.first)
        XCTAssertNil(oops.accessibilityCustomActions()?.first)
        XCTAssertEqual(oops.accessibilityValue() as? String, "12 point, no source mapping")
    }

    func testFramesAreScreenRectsOfTheDrawnItemsAndLinesUnionThem() throws {
        let res = try loadMultipage()
        let (window, view) = hostPage(res.pages[0], totalPages: 2, scale: 0.5)
        defer { window.orderOut(nil) }
        let overlay = AccessibilityOverlay(page: res.pages[0], totalPages: 2, scale: 0.5) { _, _ in }
        let lines = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        let items = try lines.flatMap { try XCTUnwrap($0.accessibilityChildren() as? [PreviewAXElement]) }
        for (item, slot) in zip(items, overlay.slots) {
            XCTAssertEqual(item.viewFrame, slot.frame)
            // Screen rect through the (flipped) view: the AX outline lands on the glyphs.
            let expected = NSAccessibility.screenRect(fromView: view, rect: slot.frame)
            XCTAssertEqual(item.accessibilityFrame(), expected, slot.element.text)
            XCTAssertNotEqual(expected.minY, slot.frame.minY, "flipped view coordinates were converted, not copied")
        }
        // A line's frame is the union of its items' frames, in view space and on screen.
        let first = try XCTUnwrap(lines[1].accessibilityChildren() as? [PreviewAXElement])
        let union = first.dropFirst().reduce(first[0].viewFrame) { $0.union($1.viewFrame) }
        XCTAssertEqual(lines[1].viewFrame, union)
        XCTAssertEqual(lines[1].accessibilityFrame(), NSAccessibility.screenRect(fromView: view, rect: union))
        // Parent-space frames are relative to the line for window-less clients.
        XCTAssertEqual(first[1].accessibilityFrameInParentSpace().origin.x, first[1].viewFrame.minX - union.minX, accuracy: 0.001)
        XCTAssertEqual(first[0].accessibilityFrameInParentSpace().origin.y, 0, accuracy: 0.001)
    }

    func testTreeIsRebuiltOnlyWhenThePageChanges() throws {
        let res = try loadMultipage()
        let (window, view) = hostPage(res.pages[0], totalPages: 2)
        defer { window.orderOut(nil) }
        let before = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        view.update(page: res.pages[0], totalPages: 2, scale: 1, fontName: { _ in "Times-Roman" }, onSelect: { _, _ in })
        let same = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(zip(before, same).allSatisfy { $0 === $1 }, "an identical update keeps the tree")
        view.update(page: res.pages[1], totalPages: 2, scale: 1, fontName: { _ in "Times-Roman" }, onSelect: { _, _ in })
        XCTAssertFalse(view.hasBuiltTree, "a new page drops the cached tree")
        XCTAssertEqual(view.accessibilityLabel(), "Page 2 of 2, 3 lines")
        let after = try XCTUnwrap(view.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertEqual(after.count, 3)
        XCTAssertFalse(after.contains { a in before.contains { $0 === a } })
        // totalPages alone changes the label ("Page 2 of 3").
        view.update(page: res.pages[1], totalPages: 3, scale: 1, fontName: { _ in "Times-Roman" }, onSelect: { _, _ in })
        XCTAssertEqual(view.accessibilityLabel(), "Page 2 of 3, 3 lines")
    }

    // MARK: diagnostics rows, read back through NSAccessibility

    /// Observed on macOS 15/26 in `swift test`: `NSHostingView` has no subviews
    /// and answers no accessibility children (`accessibilityChildren`,
    /// `…InNavigationOrder`, the legacy children attribute all empty) unless
    /// the system accessibility server is the caller, so this test skips with
    /// that finding; `EditorDiagnosticsAccessibilityTests` pin the text the modifier applies.

    /// Walks a hosted SwiftUI view's AX tree (labels, values, actions), depth first.
    static func axNodes(_ root: AnyObject) -> [AnyObject] {
        var out: [AnyObject] = [root]
        let children = ((root as? NSAccessibilityProtocol)?.accessibilityChildren() as? [AnyObject]) ?? []
        for c in children { out += axNodes(c) }
        return out
    }

    @MainActor func testDiagnosticRowsExposeLabelValueAndActionThroughNSAccessibility() async throws {
        let res = try loadMultipage()
        var navigated = 0
        let list = VStack {
            ForEach(Array(res.diagnostics.enumerated()), id: \.offset) { i, d in
                Text(d.message).accessibleDiagnostic(d, index: i, total: res.diagnostics.count, status: res.status) { navigated += 1 }
            }
        }
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 500, height: 200), styleMask: [.titled], backing: .buffered, defer: false)
        let host = NSHostingView(rootView: list)
        window.contentView = host
        window.orderFrontRegardless() // never key
        defer { window.orderOut(nil) }
        host.layoutSubtreeIfNeeded()
        // SwiftUI builds its AppKit accessibility bridge on later run-loop turns.
        var nodes: [AnyObject] = []
        for _ in 0..<20 {
            try await Task.sleep(nanoseconds: 50_000_000)
            nodes = Self.axNodes(host)
            if nodes.count > 1 { break }
        }
        let rows = nodes.compactMap { $0 as? NSAccessibilityProtocol }
            .filter { ($0.accessibilityLabel() ?? "").hasPrefix("Diagnostic ") }
        guard rows.count == 2 else {
            throw XCTSkip("SwiftUI exposed \(rows.count) diagnostic rows through NSAccessibility in this in-process host (\(nodes.count) AX nodes); the row text is pinned by EditorDiagnosticsAccessibilityTests")
        }
        let expected = res.diagnostics.enumerated().map { DiagnosticRowAccessibility($0.element, index: $0.offset, total: 2, status: res.status) }
        XCTAssertEqual(rows.map { $0.accessibilityLabel() }, expected.map(\.label))
        XCTAssertEqual(rows.map { $0.accessibilityValue() as? String }, expected.map(\.value))
        let withSource = rows[0]
        let actions = withSource.accessibilityCustomActions() ?? []
        XCTAssertEqual(actions.map(\.name), ["Go to source"])
        _ = actions.first?.handler?()
        XCTAssertEqual(navigated, 1)
        XCTAssertEqual(rows[1].accessibilityCustomActions()?.count ?? 0, 0)
        XCTAssertEqual(rows[1].accessibilityHelp(), DiagnosticRowAccessibility.noSourceHint)
    }
}
