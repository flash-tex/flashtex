import AppKit
import FlashTeXCollabCore
import FlashTeXCollabSession
import Network
import XCTest
@testable import FlashTeXMac

/// Security review of #1540: what Live Share may write on either Mac.
/// The host writes only the files it shared, at their original paths; the
/// guest's copy never holds a dotfile, a path through a link, or a case
/// twin; session compiles are pinned; turning the setting off ends the
/// session. Everything happens in a temporary folder.
@MainActor
final class LiveShareSecurityTests: XCTestCase {
    private var temp: URL!
    private var models: [ShellModel] = []
    private var hubs: [CollabHub] = []

    override func setUp() async throws {
        temp = FileManager.default.temporaryDirectory.appendingPathComponent("liveshare-sec-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: temp.appendingPathComponent("project/chapters"), withIntermediateDirectories: true)
        LiveShareController.testLoopbackOnly = true
        LiveShareController.sessionBaseOverride = temp.appendingPathComponent("guests")
        LiveShareController.enabledOverride = true
    }

    override func tearDown() async throws {
        for m in models where m.liveShare.isActive { m.liveShare.leave() }
        for h in hubs { h.stop() }
        models = []
        hubs = []
        LiveShareController.testLoopbackOnly = false
        LiveShareController.sessionBaseOverride = nil
        LiveShareController.enabledOverride = nil
        try? FileManager.default.removeItem(at: temp)
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > deadline { XCTFail("timed out: \(what)", file: file, line: line); return }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
    }

    // MARK: Path guard

    func testSafeTargetRefusesDotfilesLinksAndForeignFiles() throws {
        let root = temp.appendingPathComponent("project")
        let outside = temp.appendingPathComponent("outside")
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("linked"), withDestinationURL: outside)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("evil.tex"),
                                                   withDestinationURL: outside.appendingPathComponent("x.tex"))
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: ".git/config"))
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: ".git/hooks/pre-commit.tex"))
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: "chapters/.latexmkrc.tex"))
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: "run.sh"), "only text sources")
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: "linked/x.tex", creating: true), "a symlinked subfolder")
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: "evil.tex"), "a symlinked file")
        XCTAssertNil(LiveShareController.safeTarget(root: root, path: "../outside/x.tex"))
        XCTAssertFalse(FileManager.default.fileExists(atPath: outside.appendingPathComponent("x.tex").path))
        XCTAssertNotNil(LiveShareController.safeTarget(root: root, path: "chapters/one.tex"))
        XCTAssertNotNil(LiveShareController.safeTarget(root: root, path: "new/dir/two.tex", creating: true))
    }

    func testPathsCompareAsAPFSDoes() {
        XCTAssertEqual(LiveShareController.foldedKey("Main.tex"), LiveShareController.foldedKey("main.tex"))
        XCTAssertEqual(LiveShareController.foldedKey("caf\u{e9}.tex"), LiveShareController.foldedKey("cafe\u{301}.tex"))
        XCTAssertNotEqual(LiveShareController.foldedKey("a.tex"), LiveShareController.foldedKey("b.tex"))
    }

    // MARK: Host

    /// Even if a session's file map named another path (here forced on the
    /// host's own CRDT, as a slipped-through rename would), the host writes
    /// a shared file only where it shared it from, and never `.git/config`.
    func testHostWritesOnlyTheFilesItSharedAtTheirOwnPaths() async throws {
        let project = temp.appendingPathComponent("project")
        try "\\documentclass{article}\\begin{document}\\input{chapters/one}\\end{document}\n"
            .write(to: project.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "One.\n".write(to: project.appendingPathComponent("chapters/one.tex"), atomically: true, encoding: .utf8)
        try FileManager.default.createDirectory(at: project.appendingPathComponent(".git"), withIntermediateDirectories: true)
        try "[core]\n".write(to: project.appendingPathComponent(".git/config"), atomically: true, encoding: .utf8)
        let host = ShellModel(), guest = ShellModel()
        models = [host, guest]
        XCTAssertEqual(host.openTex(at: project.appendingPathComponent("main.tex")), .opened)
        host.liveShare.startHosting()
        try await waitUntil("invitation") { host.liveShare.invite != nil }
        XCTAssertFalse(host.liveShare.allowed.values.contains { $0.hasPrefix(".") }, "dotfiles are never shared")
        guest.liveShare.join(link: host.liveShare.invite!.link, name: "Mallory", disposition: .discard)
        try await waitUntil("approval") { !host.liveShare.approvals.isEmpty }
        host.liveShare.answer(host.liveShare.approvals[0], .allowEdit)
        try await waitUntil("guest open") { guest.documentURL != nil }
        let hs = try XCTUnwrap(host.liveShare.session)
        let chapter = try XCTUnwrap(hs.file(atPath: "chapters/one.tex"))
        _ = try hs.project.fileOp { try $0.rename(chapter, to: ".git/config") } // the hostile rename
        XCTAssertEqual(hs.path(of: chapter), ".git/config")
        let gs = try XCTUnwrap(guest.liveShare.session)
        try XCTUnwrap(gs.binding(for: chapter)).localReplace(0..<0, with: "[core]\n\tfsmonitor = evil\n")
        let chapterURL = project.appendingPathComponent("chapters/one.tex")
        try await waitUntil("written at its own path") {
            (try? String(contentsOf: chapterURL, encoding: .utf8))?.contains("fsmonitor") == true
        }
        XCTAssertEqual(try String(contentsOf: project.appendingPathComponent(".git/config"), encoding: .utf8), "[core]\n")
    }

    // MARK: Guest

    /// A hostile host offers `.git/config`, a case twin of the main file and
    /// a path through a symlink the guest's copy already has: the guest
    /// writes none of them, and its main file is the one it opened.
    func testGuestCopyRefusesDotfilesCaseTwinsAndLinks() async throws {
        let session = CollabSession(role: .hub, replica: UInt64.random(in: 1...UInt64.max), name: "Host", colourIndex: 0)
        try session.shareFile(path: "main.tex", text: "real main\n")
        try session.shareFile(path: "MAIN.TEX", text: "twin\n")
        try session.shareFile(path: ".git/config", text: "[core]\n\tfsmonitor = evil\n")
        try session.shareFile(path: "notes/a.tex", text: "ok\n")
        session.flush()
        let pins = CollabControl.SessionPins(main: "main.tex", sourceDateEpoch: 0, randomSeed: 0, shellEscape: "off",
                                             externalTools: false, readConfinement: true)
        let hub = CollabHub(session: session, identity: try CollabIdentity(), projectName: "x", pins: pins, environmentDigest: "t")
        hub.approve = { _, reply in reply(.allowEdit) }
        hubs.append(hub)
        try hub.start(loopbackOnly: true, advertise: false)
        try await waitUntil("hub ready") { hub.port != nil }
        let guest = ShellModel()
        models = [guest]
        guest.liveShare.join(link: hub.makeInvite(addresses: [], hostName: CollabGuest.localHostName).link, name: "Bob",
                             disposition: .discard)
        try await waitUntil("guest open") { guest.documentURL != nil }
        let root = try XCTUnwrap(guest.liveShare.root)
        let names = try FileManager.default.contentsOfDirectory(atPath: root.path)
        XCTAssertEqual(Set(names), ["main.tex", "notes"])
        XCTAssertEqual(guest.activeText, "real main\n")
        XCTAssertEqual(Set(guest.liveShare.allowed.values), ["main.tex", "notes/a.tex"])
    }

    // MARK: Compile pins and the switch

    func testSessionCompilesArePinned() async throws {
        let model = ShellModel()
        models = [model]
        let copy = LiveShareController.sessionDirectory("abcd")
        XCTAssertTrue(model.liveShare.forcesPinnedCompile(root: copy), "a guest's copy compiles pinned, session or not")
        XCTAssertFalse(model.liveShare.forcesPinnedCompile(root: temp.appendingPathComponent("project")))
    }

    func testTurningTheSettingOffEndsTheSession() async throws {
        let project = temp.appendingPathComponent("project")
        try "x\n".write(to: project.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let host = ShellModel()
        models = [host]
        XCTAssertEqual(host.openTex(at: project.appendingPathComponent("main.tex")), .opened)
        host.liveShare.startHosting()
        XCTAssertTrue(host.liveShare.isActive)
        XCTAssertTrue(host.liveShare.forcesPinnedCompile(root: project), "the host's compiles are pinned while it shares")
        LiveShareController.enabledOverride = false
        NotificationCenter.default.post(name: UserDefaults.didChangeNotification, object: nil)
        try await waitUntil("ended") { !host.liveShare.isActive }
        XCTAssertNil(host.liveShare.hub)
        XCTAssertEqual(host.liveShare.note, "Live Share was turned off in Settings.")
    }
}
