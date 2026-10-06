import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// FlashTeX opened with no file shows an untitled LaTeX document — File ›
/// New Project…'s blank article titled "Untitled" — with the caret on the
/// empty line under `\section{Introduction}` (it showed the protocol
/// fixture's "Hello FlashTeX." before). A bare `ShellModel()` stays
/// fixture-backed for the rest of the suite.
@MainActor
final class UntitledDocumentTests: XCTestCase {
    /// The exact text, so a change to the blank-article template is a
    /// deliberate change to what every new window shows.
    static let expected = """
    \\documentclass{article}
    \\usepackage[margin=1in]{geometry}
    \\usepackage{amsmath,amssymb}

    \\title{Untitled}
    \\author{}
    \\date{\\today}

    \\begin{document}
    \\maketitle

    \\section{Introduction}

    \\end{document}

    """

    private var env = EnvironmentOverride()
    private var window: NSWindow?
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("untitled-v3-tests-\(getpid())")

    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() async throws {
        window?.orderOut(nil)
        window = nil
        env.restore()
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    func testTheUntitledDocumentIsTheBlankArticleTemplate() {
        XCTAssertEqual(UntitledDocument.text, Self.expected)
        XCTAssertEqual(UntitledDocument.text, ProjectTemplate.blankArticle.files(projectName: "Untitled")[0].text, "one source: the New Project picker's blank article")
        XCTAssertEqual(UntitledDocument.path, "main.tex")
        XCTAssertFalse(UntitledDocument.text.contains("inputenc"), "UTF-8 is the default")
        // The caret: the start of the empty line right under \section{Introduction}.
        let ns = UntitledDocument.text as NSString
        let caret = UntitledDocument.caretUTF16
        XCTAssertEqual(caret, NSMaxRange(ns.range(of: "\\section{Introduction}\n")))
        XCTAssertEqual(ns.substring(to: caret).components(separatedBy: "\n").count, 13, "line 13")
        XCTAssertEqual(ns.substring(with: NSRange(location: caret, length: 1)), "\n", "an empty line")
        XCTAssertTrue(ns.substring(from: caret).hasPrefix("\n\\end{document}\n"))
    }

    func testAppStartupHoldsTheUntitledDocumentAndFixtureStartupIsUnchanged() {
        let model = ShellModel(startup: .untitledDocument)
        XCTAssertNil(model.loadError)
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertEqual(model.activeText, Self.expected)
        XCTAssertEqual(model.caretUTF16, UntitledDocument.caretUTF16)
        XCTAssertEqual(model.untitledDocumentCaret, UntitledDocument.caretUTF16)
        XCTAssertNil(model.documentURL)
        XCTAssertFalse(model.isFixture, "no fixture preview over a real document")
        XCTAssertFalse(model.isDirty, "untouched: quitting or opening a file asks nothing")
        XCTAssertFalse(model.hasUnsavedDocuments)

        model.updateActiveText(Self.expected.replacingOccurrences(of: "\\section{Introduction}\n\n", with: "\\section{Introduction}\nHello.\n"))
        XCTAssertTrue(model.isDirty, "edited: Save As… / Discard is offered")
        XCTAssertNil(model.untitledDocumentCaret, "an edited buffer keeps the caret the user left")

        let fixture = ShellModel()
        XCTAssertEqual(fixture.activeText, "Hello FlashTeX.\n", "the default stays the contract fixture")
        XCTAssertTrue(fixture.isFixture)
        XCTAssertNil(fixture.untitledDocumentCaret)
    }

    /// The window as the app shows it: the editor holds the template with the
    /// caret on the empty line, ready for typing.
    func testTheWindowOpensWithTheCaretUnderTheSection() async throws {
        let model = ShellModel(startup: .untitledDocument)
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 1200, height: 640), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.orderFrontRegardless()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        let tv = try XCTUnwrap(found)
        try await waitUntil("the template reaches the view") { tv.string == Self.expected }
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: UntitledDocument.caretUTF16, length: 0))
        XCTAssertEqual(model.caretUTF16, UntitledDocument.caretUTF16)
    }

    /// The old engine (the producer the app attaches) compiles the template:
    /// one page, no diagnostics.
    func testTheOldEngineCompilesIt() async throws {
        guard let producer = ShellModel.locateDefaultProducer() else {
            throw XCTSkip("no built flashtex-render or flashtex-compiler (FLASHTEX_COMPILER)")
        }
        let model = ShellModel(startup: .untitledDocument)
        model.attachWorker(at: producer)
        defer { model.detachWorker() }
        let started = Date()
        model.compile()
        try await waitUntil("the compile", timeout: 60) { model.result?.revision == model.editorRevision }
        let result = try XCTUnwrap(model.result)
        XCTAssertEqual(result.status, .ok, "\(result.diagnostics)")
        XCTAssertEqual(result.pages.count, 1)
        XCTAssertEqual(result.diagnostics.filter { $0.severity == .error }, [])
        print("untitled document, old engine (\(producer.lastPathComponent)): \(Int(Date().timeIntervalSince(started) * 1000)) ms including worker start")
    }

    /// Engine v3 (the pdfTeX port) compiles the template: one page, no error.
    func testEngineV3CompilesIt() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel(startup: .untitledDocument)
        model.engineV3Enabled = true
        model.engineV3.start(model: model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        let started = Date()
        try await waitUntil("the v3 compile", timeout: 90) { !s.compiling && s.statusNote.hasPrefix("ok") && s.pageCount == 1 }
        XCTAssertEqual(s.mainFile, "main.tex")
        XCTAssertEqual(s.pageCount, 1, s.statusNote)
        XCTAssertNil(s.firstError)
        XCTAssertGreaterThan(s.pages[0]?.page.items.count ?? 0, 0, "the page has content")
        print("untitled document, engine v3: \(s.statusNote) (\(Int(Date().timeIntervalSince(started) * 1000)) ms after ready)")
    }
}
