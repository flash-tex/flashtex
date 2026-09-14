import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Owner report: "syntax highlighting often doesn't work the first time when
/// you create a project, but adding another file to the project fixes it."
/// File › New Project… writes the template, opens `main.tex` and dismisses
/// the sheet; the entry document must be coloured the moment it appears.
@MainActor
final class NewProjectSyntaxColourTests: XCTestCase {
    private var window: NSWindow?
    private var tmp: URL?

    override func tearDown() async throws {
        window?.orderOut(nil)
        window = nil
        if let tmp { try? FileManager.default.removeItem(at: tmp) }
        tmp = nil
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    private func turn() async throws { try await Task.sleep(nanoseconds: 50_000_000) }

    /// Number of distinct temporary foreground-colour runs over the text —
    /// what the person actually sees as colour.
    private func colourRuns(_ tv: NSTextView) -> Int {
        guard let lm = tv.layoutManager else { return -1 }
        let length = (tv.string as NSString).length
        var runs = 0, index = 0
        while index < length {
            var effective = NSRange()
            if lm.temporaryAttribute(SyntaxPainter.key, atCharacterIndex: index, effectiveRange: &effective) != nil { runs += 1 }
            index = max(NSMaxRange(effective), index + 1)
        }
        return runs
    }

    /// The whole window, as `FlashTeXMacApp` builds it.
    private func hostContentView(_ model: ShellModel) async throws -> CompletingTextView {
        HostedWindowSupport.prepare()
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1200, height: 640), styleMask: [.titled],
                              backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: ContentView().environment(model).environmentObject(NearbyState()))
        window.orderFrontRegardless()
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        return try XCTUnwrap(found as? CompletingTextView)
    }

    private func temporaryFolder() throws -> URL {
        let dir = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("flashtex-new-project-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        tmp = dir
        return dir
    }

    /// File › New Project… with the sheet actually presented, as a person does it.
    private func createProject(_ model: ShellModel, template: ProjectTemplate) async throws {
        let folder = try temporaryFolder()
        model.scaffold.presentNewProject()
        try await turn() // let the sheet mount over the window
        model.scaffold.folder = folder
        model.scaffold.projectName = "Paper"
        model.scaffold.template = template
        let outcome = model.scaffold.createProject()
        guard case .opened = outcome else { return XCTFail("New Project refused: \(outcome)") }
    }

    func testNewProjectBlankArticleIsColouredImmediately() async throws {
        let model = ShellModel()
        let tv = try await hostContentView(model)
        try await turn()
        try await createProject(model, template: .blankArticle)
        try await waitUntil("main.tex reaches the editor") { tv.string.contains("\\documentclass{article}") }
        try await turn()
        XCTAssertGreaterThan(colourRuns(tv), 0,
                             "the new project's main.tex is coloured without adding a second file")
    }

    func testNewProjectWithSectionsIsColouredImmediately() async throws {
        let model = ShellModel()
        let tv = try await hostContentView(model)
        try await turn()
        try await createProject(model, template: .articleWithSections)
        try await waitUntil("main.tex reaches the editor") { tv.string.contains("\\documentclass{article}") }
        try await turn()
        XCTAssertGreaterThan(colourRuns(tv), 0,
                             "the new project's main.tex is coloured without adding a second file")
    }
}
