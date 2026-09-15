import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXMac

/// The guard for the owner's "stop the windows flashing" request.
///
/// Two independent things have to hold, and the earlier version of this file
/// only checked the first one: the process must not activate, *and* its windows
/// must not be drawn over a display. A non-activating process still draws. These
/// tests assert the geometry directly, so the regression that put editor windows
/// back over the owner's work fails here instead of on their screen.
@MainActor
final class HostedWindowSupportTests: XCTestCase {

    // MARK: activation policy

    func testPrepareInstallsANonActivatingPolicy() {
        XCTAssertTrue(HostedWindowSupport.prepare(), "the process refused both .prohibited and .accessory")
        print("HostedWindowSupport: activation policy in force = \(HostedWindowSupport.currentPolicy.rawValue) (0=regular, 1=accessory, 2=prohibited)")
        XCTAssertTrue(HostedWindowSupport.isNonActivating,
                      "activation policy is \(HostedWindowSupport.currentPolicy.rawValue); a regular app pulls itself forward when a window is ordered in")
    }

    func testPrepareIsIdempotent() {
        let first = HostedWindowSupport.prepare()
        let policy = HostedWindowSupport.currentPolicy
        for _ in 0..<5 { XCTAssertEqual(HostedWindowSupport.prepare(), first) }
        XCTAssertEqual(HostedWindowSupport.currentPolicy, policy)
    }

    func testOrderingAHostedWindowFrontDoesNotActivateTheApp() {
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 200, height: 120),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.orderOut(nil) }
        window.orderFrontRegardless()
        XCTAssertFalse(NSApplication.shared.isActive, "the test process activated itself by ordering a window front")
    }

    // MARK: geometry — the part that actually stops the flashing

    /// The window must be clear of every display both before and after it is
    /// ordered front. Ordering front is the moment AppKit would otherwise drag
    /// the title bar back onto a screen.
    func testAHostedWindowIsClearOfEveryDisplayEvenAfterBeingOrderedFront() {
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.orderOut(nil) }

        XCTAssertEqual(window.frame.origin, HostedWindowSupport.offscreenOrigin,
                       "the initializer moved the window back on-screen")
        window.orderFrontRegardless()
        XCTAssertEqual(window.frame.origin, HostedWindowSupport.offscreenOrigin,
                       "ordering the window front moved it back on-screen")
        XCTAssertTrue(HostedWindowSupport.isClearOfEveryDisplay(window),
                      "hosted window at \(window.frame) overlaps a display: \(NSScreen.screens.map(\.frame))")
    }

    /// A caller asking for an on-screen origin must still get an off-screen
    /// window — the helper ignores the requested origin on purpose, and that is
    /// what makes a newly written test safe by default.
    func testARequestedOnScreenOriginIsIgnoredButTheSizeIsKept() {
        let requested = NSRect(x: 120, y: 80, width: 640, height: 480)
        let window = HostedWindowSupport.window(contentRect: requested, styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.orderOut(nil) }
        window.orderFrontRegardless()
        XCTAssertTrue(HostedWindowSupport.isClearOfEveryDisplay(window))
        XCTAssertEqual(window.contentView?.bounds.size, requested.size,
                       "the content size the test asked for must survive the move off-screen")
    }

    /// Moving off-screen must not weaken what the hosted suites rely on: the
    /// window still lays out, draws, takes a first responder, and reports itself
    /// as a standard window to the accessibility suites.
    func testAnOffScreenHostedWindowStillLaysOutAndTakesFirstResponder() {
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.orderOut(nil) }
        window.orderFrontRegardless()

        let text = NSTextView(frame: window.contentView!.bounds)
        window.contentView!.addSubview(text)
        window.contentView!.layoutSubtreeIfNeeded()

        XCTAssertTrue(window.isVisible, "an off-screen hosted window must still be 'visible' to AppKit or layout is skipped")
        XCTAssertTrue(window.makeFirstResponder(text), "the off-screen window refused a first responder")
        XCTAssertTrue(window.firstResponder === text)
        XCTAssertTrue(window.canBecomeKey, "hosted windows must still be able to become key")
        XCTAssertEqual(window.accessibilityRole(), .window)
        XCTAssertEqual(text.convert(text.bounds, to: nil).size, NSSize(width: 600, height: 400),
                       "window-relative geometry must be unaffected by the off-screen origin")
    }

    /// Panels the app creates itself are child windows positioned from the
    /// parent's caret rect in screen coordinates. `CompletionPopup.show` clamps
    /// that position against `parent.screen ?? NSScreen.main`, and an off-screen
    /// parent has no `screen` — so this drives the product's real positioning
    /// code and asserts the popup follows the host off-screen instead of being
    /// clamped onto the main display.
    func testTheCompletionPopupFollowsItsHostOffScreen() throws {
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let text = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }

        let popup = text.completionPopup
        defer { popup.hide() }
        let items = [
            Completion.Suggestion(label: "\\section", insertText: "\\section", kind: .command, detail: "sectioning"),
            Completion.Suggestion(label: "\\subsection", insertText: "\\subsection", kind: .command, detail: "sectioning"),
        ]
        // A caret rect in screen coordinates, the way the editor supplies one.
        let caret = NSRect(x: window.frame.minX + 100, y: window.frame.minY + 200, width: 1, height: 16)
        popup.show(items: items, selected: 0, below: caret, parent: window)

        XCTAssertTrue(popup.isVisible, "the popup did not open, so its placement was never exercised")
        XCTAssertTrue(HostedWindowSupport.isClearOfEveryDisplay(popup),
                      "the completion popup was placed at \(popup.frame), which overlaps a display: \(NSScreen.screens.map(\.frame))")
    }

    // MARK: the invariant for tests written later


    /// Every hosted window must come from the helper. Constructing the AppKit
    /// class directly is the exact mistake that put windows over the owner's
    /// work, and it cannot be caught at runtime because the offending suite may
    /// never run on the machine that would notice.
    func testEveryHostedWindowComesFromTheHelper() throws {
        let testsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        let files = FileManager.default.enumerator(at: testsDir, includingPropertiesForKeys: nil)?
            .compactMap { $0 as? URL }
            .filter { $0.pathExtension == "swift" && $0.lastPathComponent != "HostedWindowSupport.swift" } ?? []
        XCTAssertFalse(files.isEmpty, "no test sources found under \(testsDir.path)")

        // Split so this scanner's own source is not an offender.
        let construction = "NSWindow" + "("
        let sanctioned = "HostedWindowSupport.window("
        let identifierish = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "_."))
        var offenders: [String] = []
        var helperSites = 0
        for file in files {
            guard let text = try? String(contentsOf: file, encoding: .utf8) else { continue }
            for (i, raw) in text.components(separatedBy: "\n").enumerated() {
                helperSites += raw.components(separatedBy: sanctioned).count - 1
                // Prose about the rule is not a breach of it: drop line comments
                // so doc comments and examples do not trip the scan.
                let line = raw.components(separatedBy: "//").first ?? raw
                var cursor = line.startIndex
                while let found = line.range(of: construction, range: cursor..<line.endIndex) {
                    cursor = found.upperBound
                    // Only a bare construction counts. `HostedWindowSupport.window(`
                    // is the sanctioned call, and an identifier that merely ends
                    // in these characters is not a construction at all.
                    let preceding = found.lowerBound == line.startIndex
                        ? " "
                        : String(line[line.index(before: found.lowerBound)])
                    if preceding.rangeOfCharacter(from: identifierish) != nil { continue }
                    offenders.append("\(file.lastPathComponent):\(i + 1): \(raw.trimmingCharacters(in: .whitespaces))")
                }
            }
        }
        XCTAssertGreaterThan(helperSites, 0, "the scan found no hosted windows, so it is not guarding anything")
        XCTAssertEqual(offenders, [],
                       "hosted windows must be built with HostedWindowSupport.window(contentRect:…), which parks them off every display")
    }
}
