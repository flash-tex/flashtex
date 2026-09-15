import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Error lens (lane mac-editor-dx-3, ErrorLens.swift): one message per line
/// from the editor's marks, errors only by default, drawn after the line.
@MainActor
final class ErrorLensTests: XCTestCase {
    override func setUp() {
        super.setUp()
        UserDefaults.standard.removeObject(forKey: ErrorLens.enabledKey)
        UserDefaults.standard.removeObject(forKey: ErrorLens.warningsKey)
    }

    override func tearDown() {
        UserDefaults.standard.removeObject(forKey: ErrorLens.enabledKey)
        UserDefaults.standard.removeObject(forKey: ErrorLens.warningsKey)
        super.tearDown()
    }

    func testOneMessagePerLineErrorsWinWarningsOptionalGapsNever() {
        let text = "a\nb\nc\nd\n"
        var h = SyntaxHighlighter(); h.reset(text as NSString)
        let marks = [
            SourceEditorViewTests.mark(NSRange(location: 2, length: 1), .warning, "Overfull\nsecond line"),
            SourceEditorViewTests.mark(NSRange(location: 2, length: 1), .error, "Missing }"),
            SourceEditorViewTests.mark(NSRange(location: 0, length: 1), .error, String(repeating: "x", count: 120)),
            SourceEditorViewTests.mark(NSRange(location: 4, length: 1), .warning, "meh"),
            SourceEditorViewTests.mark(NSRange(location: 6, length: 1), .error, "tables are not implemented"),
        ]
        let errors = ErrorLens.lines(for: marks, warnings: false, lineOf: { h.line(at: $0) })
        XCTAssertEqual(errors.map(\.line), [0, 1])
        XCTAssertEqual(errors[1], .init(line: 1, severity: .error, text: "Missing }"))
        XCTAssertEqual(errors[0].text.count, 90); XCTAssertTrue(errors[0].text.hasSuffix("…"))
        let all = ErrorLens.lines(for: marks, warnings: true, lineOf: { h.line(at: $0) })
        XCTAssertEqual(all.map { "\($0.line):\($0.severity.rawValue)" }, ["0:error", "1:error", "2:warning"])
        XCTAssertEqual(ErrorLens.summarize("Overfull\nsecond"), "Overfull")
        // Defaults: on, errors only.
        XCTAssertTrue(ErrorLens.enabled); XCTAssertFalse(ErrorLens.showsWarnings)
    }

    struct Host: View {
        var model: ShellModel
        var marks: [EditorDiagnostics.Mark]
        var body: some View {
            SourceEditorView(text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                             selection: model.selection, pendingEdit: model.pendingEdit, marks: marks)
        }
    }

    func testPainterDrawsTheVisibleLinesAndFollowsThePreference() async throws {
        let model = ShellModel()
        model.updateActiveText("\\documentclass{article}\nHello $x\nfine\n")
        let marks = [SourceEditorViewTests.mark(NSRange(location: 30, length: 1), .error, "Unterminated math"),
                     SourceEditorViewTests.mark(NSRange(location: 33, length: 2), .warning, "Overfull box")]
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 300), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, marks: marks))
        window.orderFrontRegardless()
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, found == nil {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertEqual(co.errorLens.lines, [.init(line: 1, severity: .error, text: "Unterminated math")])
        tv.display()
        XCTAssertEqual(co.errorLens.drawn, [1: "✕ Unterminated math"])
        XCTAssertEqual(tv.string, "\\documentclass{article}\nHello $x\nfine\n", "nothing is inserted in the storage")
        // Warnings on: the second line appears; off: nothing is drawn.
        ErrorLens.showsWarnings = true
        XCTAssertEqual(co.errorLens.lines.map(\.line), [1, 2])
        tv.display()
        XCTAssertEqual(co.errorLens.drawn[2], "△ Overfull box")
        ErrorLens.enabled = false
        XCTAssertEqual(co.errorLens.lines, [])
        tv.display()
        XCTAssertEqual(co.errorLens.drawn, [:])
    }
}
