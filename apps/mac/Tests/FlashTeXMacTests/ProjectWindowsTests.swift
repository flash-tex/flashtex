import AppKit
import XCTest
@testable import FlashTeXMac

/// Multi-window and multi-project (ProjectWindows.swift; DESIGN.md §10,
/// decision 7): where a file opens, that no document opens twice, which
/// model the menus act on, and the restore list the next launch reopens.
/// No windows are shown here (ProjectWindowsHostedTests shows them).
@MainActor
final class ProjectWindowsTests: XCTestCase {
    private var dir: URL!
    private var suite: String!

    override func setUp() {
        OwnerStateGuard.install()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("windows-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        suite = "tech.flashtex.tests.windows.\(getpid()).\(UUID().uuidString)"
    }

    override func tearDown() {
        if let dir { try? FileManager.default.removeItem(at: dir) }
        if let suite { UserDefaults().removePersistentDomain(forName: suite) }
    }

    static func document(_ body: String) -> String {
        "\\documentclass{article}\n\\begin{document}\n\(body)\n\\end{document}\n"
    }

    @discardableResult
    private func write(_ path: String, _ text: String) throws -> URL {
        let url = dir.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// A list whose models start no host and attach no bridge, as the app's do in tests.
    private func makeWindows(defaults: UserDefaults? = nil) -> ProjectWindows {
        let w = ProjectWindows(defaults: defaults)
        w.makeModel = { _ in ShellModel(startup: .untitledDocument, launchHooks: false) }
        return w
    }

    // MARK: where a file opens

    func testTheFirstOpenFillsTheUntitledWindowAndTheNextOpensANewWindow() throws {
        let a = try write("a/main.tex", Self.document("A")), b = try write("b/main.tex", Self.document("B"))
        let windows = makeWindows()
        let first = windows.model(for: ProjectWindowID())
        XCTAssertTrue(first.holdsNothing, "the launch window holds the untouched untitled document")

        let r1 = windows.open(a, from: first)
        XCTAssertEqual(r1.result, .openedHere)
        XCTAssertTrue(r1.model === first)
        XCTAssertEqual(first.documentURL?.standardizedFileURL, a.standardizedFileURL)

        let r2 = windows.open(b, from: first)
        guard case .openedNew = r2.result else { return XCTFail("b opened \(r2.result), not in a new window") }
        let second = try XCTUnwrap(r2.model)
        XCTAssertFalse(second === first)
        XCTAssertEqual(first.documentURL?.standardizedFileURL, a.standardizedFileURL, "opening b left a's window as it was")
        XCTAssertEqual(second.documentURL?.standardizedFileURL, b.standardizedFileURL)
        XCTAssertEqual(windows.entries.count, 2)
        XCTAssertTrue(second.windows === windows, "the new window's own File › Open goes through the list too")
        XCTAssertFalse(second.engineV3 === first.engineV3, "each window has its own engine session (and host)")
    }

    func testAnEditedUntitledWindowIsNeverReplaced() throws {
        let a = try write("main.tex", Self.document("A"))
        let windows = makeWindows()
        let first = windows.model(for: ProjectWindowID())
        first.updateActiveText(first.activeText + "notes I have not saved")
        XCTAssertFalse(first.holdsNothing)
        let r = windows.open(a, from: first)
        guard case .openedNew = r.result else { return XCTFail("opened \(r.result)") }
        XCTAssertNil(first.documentURL)
        XCTAssertTrue(first.activeText.hasSuffix("notes I have not saved"), "the unsaved buffer is untouched")
    }

    func testFileOpenWithoutAWindowListStillOpensInPlace() throws {
        // Tests and automation make bare models: their opens are what they were.
        let a = try write("main.tex", Self.document("A"))
        let model = ShellModel(startup: .untitledDocument, launchHooks: false)
        XCTAssertNil(model.windows)
        let (outcome, opened) = model.openForUser(a)
        XCTAssertEqual(outcome, .opened)
        XCTAssertTrue(opened === model)
        XCTAssertEqual(model.documentURL?.standardizedFileURL, a.standardizedFileURL)
    }

    // MARK: the same document is never open twice

    func testOpeningAnOpenDocumentBringsItsWindowForward() throws {
        let a = try write("a/main.tex", Self.document("A")), b = try write("b/main.tex", Self.document("B"))
        let windows = makeWindows()
        let first = windows.model(for: ProjectWindowID())
        windows.open(a, from: first)
        let second = try XCTUnwrap(windows.open(b, from: first).model)
        let firstID = try XCTUnwrap(windows.entry(for: first)?.id)

        // From the other window, by another spelling of the path, and as its folder.
        let spellings = [a, dir.appendingPathComponent("b/../a/main.tex"), dir.appendingPathComponent("a")]
        for url in spellings {
            let r = windows.open(url, from: second)
            XCTAssertEqual(r.result, .focused(firstID, path: "main.tex"), "\(url.path)")
            XCTAssertTrue(r.model === first)
        }
        // A symbolic link to it.
        let link = dir.appendingPathComponent("link.tex")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: a)
        XCTAssertEqual(windows.open(link, from: second).result, .focused(firstID, path: "main.tex"))
        XCTAssertEqual(windows.entries.count, 2, "no window was added")
        XCTAssertEqual(windows.frontmost.map(ObjectIdentifier.init), ObjectIdentifier(first), "the window that has it is now the one in front")
    }

    func testOpeningAnOpenProjectMemberShowsItInItsWindow() async throws {
        let main = try write("book/main.tex", Self.document("\\input{chapter1}"))
        let chapter = try write("book/chapter1.tex", "Chapter one.\n")
        let windows = makeWindows()
        let first = windows.model(for: ProjectWindowID())
        windows.open(main, from: first)
        let opened = await first.project.openDocument("chapter1.tex")
        guard case .opened = opened else { return XCTFail("chapter1.tex: \(opened)") }
        first.project.switchDocument(to: "main.tex")
        let other = try XCTUnwrap(windows.open(try write("other/main.tex", Self.document("O")), from: first).model)

        let r = windows.open(chapter, from: other)
        XCTAssertEqual(r.result, .focused(try XCTUnwrap(windows.entry(for: first)?.id), path: "chapter1.tex"))
        XCTAssertEqual(first.activePath, "chapter1.tex", "the member's tab is shown")
        XCTAssertEqual(windows.entries.count, 2)
    }

    func testSameFileComparesTheFileNotTheSpelling() throws {
        let a = try write("x/a.tex", "a"), b = try write("x/b.tex", "b")
        XCTAssertTrue(ProjectWindows.sameFile(a, dir.appendingPathComponent("x/./a.tex")))
        XCTAssertFalse(ProjectWindows.sameFile(a, b))
        // A hard link is one file under two names.
        let hard = dir.appendingPathComponent("x/hard.tex")
        try FileManager.default.linkItem(at: a, to: hard)
        XCTAssertTrue(ProjectWindows.sameFile(a, hard))
        // On a case-insensitive volume (the default), another case is the same file.
        let upper = dir.appendingPathComponent("x/A.TEX")
        if FileManager.default.fileExists(atPath: upper.path) { XCTAssertTrue(ProjectWindows.sameFile(a, upper)) }
    }

    // MARK: what the menus act on

    func testTheMenusFollowTheKeyWindowAndFallBackToTheLastOne() throws {
        let windows = makeWindows()
        let id1 = ProjectWindowID(), id2 = ProjectWindowID()
        let m1 = windows.model(for: id1), m2 = windows.model(for: id2)
        XCTAssertTrue(windows.commandTarget(focused: m2) === m2, "the key window's model, when SwiftUI reports one")
        windows.becameKey(id2)
        XCTAssertTrue(windows.commandTarget(focused: nil) === m2, "Settings or Nearby key: the last project window")
        windows.becameKey(id1)
        XCTAssertTrue(windows.commandTarget(focused: nil) === m1)
        XCTAssertEqual(windows.models.map(ObjectIdentifier.init), [m1, m2].map(ObjectIdentifier.init), "most recently key first")

        windows.closed(id1)
        XCTAssertTrue(windows.commandTarget(focused: nil) === m2, "its window closed: the one behind it")
        windows.closed(id2)
        let placeholder = windows.commandTarget(focused: nil)
        XCTAssertFalse(placeholder === m1 || placeholder === m2, "no project window: the placeholder, never a closed window's model")
        XCTAssertFalse(placeholder.engineV3Enabled, "the placeholder starts no engine")
        XCTAssertTrue(windows.entries.isEmpty)
    }

    func testOnlyTheFirstWindowRunsTheLaunchHooks() {
        let windows = ProjectWindows()
        var hooks: [Bool] = []
        windows.makeModel = { hooks.append($0); return ShellModel(startup: .untitledDocument, launchHooks: false) }
        _ = windows.model(for: ProjectWindowID())
        _ = windows.newWindow()
        _ = windows.model(for: ProjectWindowID())
        XCTAssertEqual(hooks, [true, false, false], "one seed and one capture bridge for the app")
    }

    func testAClosedWindowsModelIsLetGo() throws {
        let windows = makeWindows()
        let id = windows.newWindow()
        weak var model = windows.entries.first?.model
        XCTAssertNotNil(model)
        windows.closed(id)
        XCTAssertNil(model, "the list held the closed window's model (and so its engine session)")
    }

    func testCapturesGoToTheFrontmostWindow() {
        let windows = makeWindows()
        let id1 = ProjectWindowID(), id2 = ProjectWindowID()
        let m1 = windows.model(for: id1), m2 = windows.model(for: id2)
        windows.becameKey(id2)
        XCTAssertTrue(windows.captures.target(nil) === m2)
        windows.becameKey(id1)
        XCTAssertTrue(windows.captures.target(nil) === m1)
        XCTAssertTrue(windows.captures === windows.captures, "one sink for the app: switching windows never re-attaches the listener")
    }

    // MARK: restoration

    func testTheNextLaunchReopensTheWindowsWithTheirDocumentsAndFrames() async throws {
        let a = try write("a/main.tex", Self.document("\\input{intro}"))
        try write("a/intro.tex", "Intro.\n")
        let b = try write("b/paper.tex", Self.document("B"))
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        let run1 = makeWindows(defaults: defaults)
        let first = run1.model(for: ProjectWindowID())
        run1.open(a, from: first)
        _ = await first.project.openDocument("intro.tex")
        first.project.switchDocument(to: "intro.tex")
        let second = try XCTUnwrap(run1.open(b, from: first).model)
        let untitled = run1.newWindow() // nothing to reopen
        run1.becameKey(try XCTUnwrap(run1.entry(for: second)?.id))
        run1.becameKey(try XCTUnwrap(run1.entry(for: first)?.id)) // in front at quit
        run1.becameKey(untitled)
        run1.beginTerminating()
        for e in run1.entries { run1.closed(e.id) } // the windows close as the app quits
        XCTAssertTrue(run1.entries.isEmpty)

        let saved = ProjectWindows(defaults: defaults).savedRestorable()
        XCTAssertEqual(saved.map { URL(fileURLWithPath: $0.path).lastPathComponent }, ["paper.tex", "main.tex"],
                       "the files' windows, the one in front last; the closes while quitting changed nothing")
        XCTAssertEqual(saved.last?.members, ["intro.tex"])
        XCTAssertEqual(saved.last?.activePath, "intro.tex")

        let run2 = makeWindows(defaults: defaults)
        let launch = run2.model(for: ProjectWindowID())
        run2.restore(run2.savedRestorable(), first: launch)
        XCTAssertEqual(run2.entries.count, 2)
        XCTAssertEqual(launch.documentURL?.lastPathComponent, "paper.tex", "the first reopens into the launch window")
        let reopened = try XCTUnwrap(run2.entries.last?.model)
        XCTAssertEqual(reopened.documentURL.map { ProjectWindows.sameFile($0, a) }, true)
        let deadline = Date().addingTimeInterval(10)
        while reopened.activePath != "intro.tex", Date() < deadline { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertEqual(reopened.documents.map(\.path), ["main.tex", "intro.tex"], "its open documents came back")
        XCTAssertEqual(reopened.activePath, "intro.tex", "showing the one it showed")
    }

    func testClosingEveryWindowLeavesNothingToReopen() throws {
        let a = try write("main.tex", Self.document("A"))
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        let windows = makeWindows(defaults: defaults)
        let id = ProjectWindowID()
        windows.open(a, from: windows.model(for: id))
        XCTAssertEqual(windows.savedRestorable().count, 1, "kept at each open: a crash still reopens it")
        windows.closed(id) // the user closed it (then the app quits: its last window)
        windows.beginTerminating()
        XCTAssertEqual(windows.savedRestorable(), [])
    }

    func testAFileThatIsGoneIsSkipped() throws {
        let b = try write("b.tex", Self.document("B"))
        let windows = makeWindows()
        let launch = windows.model(for: ProjectWindowID())
        windows.restore([.init(path: dir.appendingPathComponent("gone.tex").path), .init(path: b.path)], first: launch)
        XCTAssertEqual(windows.entries.count, 1)
        XCTAssertEqual(launch.documentURL?.lastPathComponent, "b.tex", "the next one took the launch window")
    }

    func testRestorationIsOffWhenTheLaunchSaysWhatToOpen() {
        XCTAssertTrue(ProjectWindows.restoresWindows([:]))
        XCTAssertFalse(ProjectWindows.restoresWindows(["FLASHTEX_OPEN": "/tmp/x.tex"]))
        XCTAssertFalse(ProjectWindows.restoresWindows(["FLASHTEX_V3_BENCH": "/tmp/bench.json"]))
        XCTAssertFalse(ProjectWindows.restoresWindows(["FLASHTEX_RESTORE_WINDOWS": "0"]))
        XCTAssertTrue(ProjectWindows.restoresWindows(["FLASHTEX_OPEN": "/tmp/x.tex", "FLASHTEX_RESTORE_WINDOWS": "1"]))
        XCTAssertNil(ProjectWindows.shared.defaults, "under XCTest the app's list keeps nothing in the runner's defaults")
    }
}
