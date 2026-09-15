import AppKit
import XCTest
import FlashTeXAccessibility
import HostedWindows
@testable import FlashTeXMac

/// VoiceOver rotor on the real editor text view (lane mac-editor-a11y-3):
/// `CompletingTextView.accessibilityCustomRotors()` exposes a Headings rotor
/// (built-in type) and an Environments rotor backed by
/// `AccessibleEditorModel.rotorItems`, searched the way
/// `NSAccessibilityCustomRotor.SearchParameters` describes (from the ends,
/// from the caret VoiceOver passes as the current item, filtered), with the
/// item cache dropped on every edit. Nothing here activates a window or
/// takes keyboard focus.
@MainActor
final class EditorRotorTests: XCTestCase {
    typealias Item = AccessibleEditorModel.RotorItem

    static let sample = """
    \\documentclass{article}
    \\begin{document}
    \\section{Introduction}
    Some prose.
    \\begin{itemize}
    \\item one
    \\end{itemize}
    \\subsection{Détail}
    More prose here.
    \\section{Results}
    \\begin{equation}
    x^2
    \\end{equation}
    \\end{document}

    """

    /// A hosted editor text view (never key) sized so that only the first
    /// few lines are visible; everything below is off-screen.
    private func hostedTextView(_ text: String, height: CGFloat = 60) -> (NSWindow, CompletingTextView) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 400, height: height), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView = scroll
        let tv = scroll.documentView as! CompletingTextView
        tv.string = text
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        return (window, tv)
    }

    private func params(current: NSAccessibilityCustomRotor.ItemResult?, forward: Bool, filter: String = "") -> NSAccessibilityCustomRotor.SearchParameters {
        let p = NSAccessibilityCustomRotor.SearchParameters()
        p.currentItem = current
        p.searchDirection = forward ? .next : .previous
        p.filterString = filter
        return p
    }

    private func caretItem(_ tv: NSTextView, at offset: Int) -> NSAccessibilityCustomRotor.ItemResult {
        let r = NSAccessibilityCustomRotor.ItemResult(targetElement: tv)
        r.targetRange = NSRange(location: offset, length: 0)
        return r
    }

    // MARK: pure search

    func testResolveFollowsAppKitSearchSemantics() {
        let model = AccessibleEditorModel(text: Self.sample)
        let items = model.rotorItems(.headings)
        XCTAssertEqual(items.map(\.label), ["Section “Introduction”, level 1", "Subsection “Détail”, level 2", "Section “Results”, level 1"])
        // No current item: first / last, inclusive.
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .fromEnds, forward: true, filter: ""), items[0])
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .fromEnds, forward: false, filter: ""), items[2])
        // From a caret: strictly after / before; a caret exactly on a heading start moves past it.
        let second = items[1].utf16.location
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .character(second), forward: true, filter: ""), items[2])
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .character(second), forward: false, filter: ""), items[0])
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .character(0), forward: true, filter: ""), items[0])
        // No wrap: nil past either end.
        XCTAssertNil(EditorRotorSearch.resolve(items: items, start: .character(items[2].utf16.location), forward: true, filter: ""))
        XCTAssertNil(EditorRotorSearch.resolve(items: items, start: .character(0), forward: false, filter: ""))
        // Type-ahead filter: case-insensitive substring of the label, diacritics exact.
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .fromEnds, forward: true, filter: "res"), items[2])
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .fromEnds, forward: false, filter: "SECTION"), items[2])
        XCTAssertEqual(EditorRotorSearch.resolve(items: items, start: .character(0), forward: true, filter: "détail"), items[1])
        XCTAssertNil(EditorRotorSearch.resolve(items: items, start: .fromEnds, forward: true, filter: "appendix"))
        // Empty document / no items.
        XCTAssertNil(EditorRotorSearch.resolve(items: [], start: .fromEnds, forward: true, filter: ""))
        XCTAssertNil(EditorRotorSearch.resolve(items: [], start: .character(5), forward: false, filter: ""))
    }

    func testStartOfSearchParameters() {
        let (_, tv) = hostedTextView(Self.sample)
        XCTAssertEqual(EditorRotorSearch.start(of: params(current: nil, forward: true)), .fromEnds)
        XCTAssertEqual(EditorRotorSearch.start(of: params(current: caretItem(tv, at: 42), forward: true)), .character(42))
        let notFound = NSAccessibilityCustomRotor.ItemResult(targetElement: tv)
        notFound.targetRange = NSRange(location: NSNotFound, length: 0)
        XCTAssertEqual(EditorRotorSearch.start(of: params(current: notFound, forward: false)), .fromEnds)
    }

    // MARK: the real text view

    func testTextViewExposesHeadingsAndEnvironmentsRotors() throws {
        let (_, tv) = hostedTextView(Self.sample)
        let rotors = tv.accessibilityCustomRotors()
        XCTAssertGreaterThanOrEqual(rotors.count, 2)
        XCTAssertEqual(rotors[0].type, .heading, "headings use the built-in rotor type so VoiceOver lists them under Headings")
        XCTAssertEqual(rotors[1].label, "Environments")
        XCTAssertTrue(rotors[0].itemSearchDelegate === tv.rotorSearch)
        XCTAssertEqual(tv.rotorSearch.category(of: rotors[0]), .headings)
        XCTAssertEqual(tv.rotorSearch.category(of: rotors[1]), .environments)
    }

    func testHeadingSearchFindsOffScreenHeadingsFromTheCaret() throws {
        let (_, tv) = hostedTextView(Self.sample, height: 40)
        let rotor = tv.accessibilityCustomRotors()[0]
        let search = tv.rotorSearch
        let ns = Self.sample as NSString
        let results = ns.range(of: "\\section{Results}")
        let intro = ns.range(of: "\\section{Introduction}")
        let detail = ns.range(of: "\\subsection{Détail}")

        // The third heading is below the visible rect of this 40 pt tall view.
        let lm = try XCTUnwrap(tv.layoutManager)
        let glyphs = lm.glyphRange(forCharacterRange: results, actualCharacterRange: nil)
        let rect = lm.boundingRect(forGlyphRange: glyphs, in: tv.textContainer!)
        XCTAssertGreaterThan(rect.minY, tv.visibleRect.maxY, "the Results heading starts off-screen")

        // No current item: first heading forward, last heading backward.
        let first = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: nil, forward: true)))
        XCTAssertEqual(first.targetRange, intro)
        XCTAssertEqual(first.customLabel, "Section “Introduction”, level 1")
        XCTAssertTrue(first.targetElement === tv)
        let last = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: nil, forward: false)))
        XCTAssertEqual(last.targetRange, results)

        // From a caret in the prose after the first heading: next is Détail, previous is Introduction.
        let caret = ns.range(of: "Some prose.").location
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        let next = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: caretItem(tv, at: caret), forward: true)))
        XCTAssertEqual(next.targetRange, detail)
        let prev = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: caretItem(tv, at: caret), forward: false)))
        XCTAssertEqual(prev.targetRange, intro)

        // Continue from the found item (VoiceOver passes the last result back): off-screen Results, then nothing.
        let afterNext = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: next, forward: true)))
        XCTAssertEqual(afterNext.targetRange, results)
        XCTAssertNil(search.rotor(rotor, resultFor: params(current: afterNext, forward: true)), "no wrap-around at the end")
        XCTAssertNil(search.rotor(rotor, resultFor: params(current: first, forward: false)), "no wrap-around at the start")

        // Selecting the result (what VoiceOver does with targetRange) is read-only and brings it into view.
        let before = tv.string
        tv.setSelectedRange(afterNext.targetRange)
        tv.scrollRangeToVisible(afterNext.targetRange)
        XCTAssertEqual(tv.selectedRange(), results)
        XCTAssertEqual(tv.string, before, "the rotor never edits")

        // Type-ahead.
        let filtered = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: nil, forward: true, filter: "dét")))
        XCTAssertEqual(filtered.targetRange, detail)
        XCTAssertNil(search.rotor(rotor, resultFor: params(current: nil, forward: true, filter: "appendix")))
    }

    func testEnvironmentsRotorAndEmptyDocument() throws {
        let (_, tv) = hostedTextView(Self.sample)
        let rotor = tv.accessibilityCustomRotors()[1]
        let ns = Self.sample as NSString
        let document = NSUnionRange(ns.range(of: "\\begin{document}"), ns.range(of: "\\end{document}"))
        let itemize = NSUnionRange(ns.range(of: "\\begin{itemize}"), ns.range(of: "\\end{itemize}"))
        // Document order of the `\begin`s: the enclosing document environment first.
        let first = try XCTUnwrap(tv.rotorSearch.rotor(rotor, resultFor: params(current: nil, forward: true)))
        XCTAssertEqual(first.targetRange, document)
        XCTAssertEqual(first.customLabel, "Environment document, lines 2 to 14")
        let second = try XCTUnwrap(tv.rotorSearch.rotor(rotor, resultFor: params(current: first, forward: true)))
        XCTAssertEqual(second.targetRange, itemize)
        XCTAssertEqual(second.customLabel, "Environment itemize, lines 5 to 7")
        let third = try XCTUnwrap(tv.rotorSearch.rotor(rotor, resultFor: params(current: second, forward: true)))
        XCTAssertEqual(third.customLabel, "Environment equation, lines 11 to 13")
        XCTAssertNil(tv.rotorSearch.rotor(rotor, resultFor: params(current: third, forward: true)))
        let last = try XCTUnwrap(tv.rotorSearch.rotor(rotor, resultFor: params(current: nil, forward: false)))
        XCTAssertEqual(last.customLabel, "Environment equation, lines 11 to 13")
        // Search order across categories shares one model build.
        XCTAssertEqual(tv.rotorSearch.rebuilds, 1)

        let (_, empty) = hostedTextView("")
        for r in empty.accessibilityCustomRotors().prefix(2) {
            XCTAssertNil(empty.rotorSearch.rotor(r, resultFor: params(current: nil, forward: true)))
            XCTAssertNil(empty.rotorSearch.rotor(r, resultFor: params(current: nil, forward: false)))
            XCTAssertNil(empty.rotorSearch.rotor(r, resultFor: params(current: caretItem(empty, at: 0), forward: true)))
        }
        XCTAssertEqual(empty.rotorSearch.rebuilds, 1, "an empty document is cached too")
    }

    func testItemsAreCachedUntilTheNextEdit() throws {
        let (_, tv) = hostedTextView(Self.sample)
        let rotor = tv.accessibilityCustomRotors()[0]
        let search = tv.rotorSearch
        _ = search.rotor(rotor, resultFor: params(current: nil, forward: true))
        _ = search.rotor(rotor, resultFor: params(current: nil, forward: false))
        XCTAssertEqual(search.rebuilds, 1)

        // A user-style edit through the storage adds a heading at the end: the cache drops.
        let insertAt = (tv.string as NSString).range(of: "\\end{document}").location
        tv.insertText("\\section{Appendix}\n", replacementRange: NSRange(location: insertAt, length: 0))
        let last = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: nil, forward: false)))
        XCTAssertEqual(last.customLabel, "Section “Appendix”, level 1")
        XCTAssertEqual(search.rebuilds, 2)

        // A programmatic replacement (`string =`, as the owner resets the text) drops it as well.
        tv.string = "\\section{Only}\n"
        let only = try XCTUnwrap(search.rotor(rotor, resultFor: params(current: nil, forward: true)))
        XCTAssertEqual(only.customLabel, "Section “Only”, level 1")
        XCTAssertEqual(only.targetRange, NSRange(location: 0, length: 14))
        XCTAssertEqual(search.rebuilds, 3)
    }

    func testRebuildCostOnALargeDocument() throws {
        let text = LargeDocumentEditorTests.proseDocument(bytes: 560_000)
        let (_, tv) = hostedTextView(text)
        let rotor = tv.accessibilityCustomRotors()[0]
        let t0 = MonotonicClock.nowNs()
        let first = tv.rotorSearch.rotor(rotor, resultFor: params(current: nil, forward: true))
        let build = Double(MonotonicClock.nowNs() - t0) / 1e6
        let t1 = MonotonicClock.nowNs()
        let count = tv.rotorSearch.items(.headings).count
        let cached = Double(MonotonicClock.nowNs() - t1) / 1e6
        print(String(format: "editor-rotor 560 KB (%@): first search (model build + regex) %.1f ms, cached lookup %.3f ms, %d headings", IMEHarness.uptime(), build, cached, count))
        XCTAssertNotNil(first)
        XCTAssertGreaterThan(count, 40, "a \\section every 30 paragraphs of ~11 KB")
        XCTAssertEqual(tv.rotorSearch.rebuilds, 1)
    }
}
