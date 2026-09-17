import XCTest
@testable import FlashTeXMac

/// "View in Finder" for on-disk project documents (issue #691): availability
/// (no project root, a refused rooted path, a file gone from disk) and the
/// resolved target URL, plus that `reveal` only ever calls `NSWorkspace`
/// through the replaceable hook when there is a real file to show.
@MainActor
final class RevealInFinderTests: XCTestCase {
    private struct TempProject {
        let root: URL
        let main: URL
        init(main mainText: String = "\\begin{document}\nMain.\n\\input{chapter}\n\\end{document}\n",
             chapter: String = "Chapter one.\n") throws {
            root = FileManager.default.temporaryDirectory.appendingPathComponent("reveal-finder-test-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
            main = root.appendingPathComponent("project/main.tex")
            try mainText.write(to: main, atomically: true, encoding: .utf8)
            try chapter.write(to: root.appendingPathComponent("project/chapter.tex"), atomically: true, encoding: .utf8)
        }
        func remove() { try? FileManager.default.removeItem(at: root) }
    }

    // MARK: target(path:root:)

    func testTargetResolvesEachOpenDocumentToItsOwnFileOnDisk() async throws {
        let project = try TempProject()
        defer { project.remove() }
        let model = ShellModel()
        model.detachWorker()
        XCTAssertEqual(model.openTex(at: project.main), .opened)
        _ = await model.project.openDiscoveredIncludes() // opens chapter.tex
        let root = model.project.projectRoot

        let mainURL = RevealInFinder.target(path: "main.tex", root: root)
        let chapterURL = RevealInFinder.target(path: "chapter.tex", root: root)
        XCTAssertEqual(mainURL, project.root.appendingPathComponent("project/main.tex").standardizedFileURL)
        XCTAssertEqual(chapterURL, project.root.appendingPathComponent("project/chapter.tex").standardizedFileURL)
        XCTAssertNotEqual(mainURL, chapterURL, "a multi-file project reveals the row the user actually clicked, not a fixed document")
    }

    func testTargetIsNilWithNoProjectRoot() {
        // An entry document that was never saved: `documentURL` is nil, so
        // `projectRoot` is nil (ProjectDocuments.swift) — the same state
        // `testDirectModeNeedsAProjectRoot` in ProjectDocumentsTests exercises.
        let model = ShellModel()
        model.detachWorker()
        model.replaceProject(entryText: "\\input{chapter}\n")
        XCTAssertNil(model.documentURL)
        XCTAssertNil(model.project.projectRoot)
        XCTAssertNil(RevealInFinder.target(path: "main.tex", root: model.project.projectRoot))
    }

    func testTargetIsNilWhenTheFileIsGoneFromDisk() throws {
        let project = try TempProject()
        defer { project.remove() }
        try FileManager.default.removeItem(at: project.root.appendingPathComponent("project/chapter.tex"))
        let root = project.root.appendingPathComponent("project").standardizedFileURL
        XCTAssertNil(RevealInFinder.target(path: "chapter.tex", root: root), "deleted or moved out from under the app, not a crash")
    }

    func testTargetIsNilForAPathTheRootedFileCheckRefuses() throws {
        let project = try TempProject()
        defer { project.remove() }
        try FileManager.default.createSymbolicLink(at: project.root.appendingPathComponent("project/link.tex"),
                                                    withDestinationURL: project.root.appendingPathComponent("project/chapter.tex"))
        let root = project.root.appendingPathComponent("project").standardizedFileURL
        XCTAssertNil(RevealInFinder.target(path: "link.tex", root: root), "the same symlink refusal ProjectDocuments.rootedFile applies elsewhere")
        XCTAssertNil(RevealInFinder.target(path: "../outside.tex", root: root), "never escapes the project root")
    }

    // MARK: reveal(path:root:)

    func testRevealActivatesFileViewerWithTheResolvedURLAndReturnsTrue() throws {
        let project = try TempProject()
        defer { project.remove() }
        let root = project.root.appendingPathComponent("project").standardizedFileURL
        var seen: [[URL]] = []
        let previous = RevealInFinder.activateFileViewerSelecting
        RevealInFinder.activateFileViewerSelecting = { seen.append($0) }
        defer { RevealInFinder.activateFileViewerSelecting = previous }

        let didReveal = RevealInFinder.reveal(path: "chapter.tex", root: root)
        XCTAssertTrue(didReveal)
        XCTAssertEqual(seen, [[project.root.appendingPathComponent("project/chapter.tex").standardizedFileURL]])
    }

    func testRevealNeverCallsFinderAndReturnsFalseWithNoTarget() {
        var callCount = 0
        let previous = RevealInFinder.activateFileViewerSelecting
        RevealInFinder.activateFileViewerSelecting = { _ in callCount += 1 }
        defer { RevealInFinder.activateFileViewerSelecting = previous }

        // No project root — the state of an unsaved/new virtual document, and
        // of a missing-include row (never a real ProjectDocument to begin with).
        XCTAssertFalse(RevealInFinder.reveal(path: "main.tex", root: nil))
        XCTAssertEqual(callCount, 0, "must not crash or silently reach NSWorkspace with nothing to show")
    }
}
