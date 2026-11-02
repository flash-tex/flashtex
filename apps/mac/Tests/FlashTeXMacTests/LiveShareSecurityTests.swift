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

    /// A taint and a downloaded (quarantined) project folder are separate
    /// subjects: trusting records both, and neither replaces the other.
    func testTaintCoexistsWithAQuarantinedFolder() throws {
        let root = temp.appendingPathComponent("downloaded")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let main = root.appendingPathComponent("main.tex")
        try "x\n".write(to: main, atomically: true, encoding: .utf8)
        let q = "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Safari;\(UUID().uuidString)"
        XCTAssertEqual(setxattr(root.path, EngineV3Trust.quarantineAttribute, q, q.utf8.count, 0, 0), 0)
        EngineV3Trust.record(root: root, main: main)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: root, main: main), "the download, trusted")
        EngineV3Trust.taint(root: root, event: "session-1")
        XCTAssertFalse(EngineV3Trust.isTrusted(root: root, main: main))
        XCTAssertEqual(EngineV3Trust.subjects(root: root, main: main)?.count, 2)
        EngineV3Trust.record(root: root, main: main)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: root, main: main), "both subjects recorded")
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
        try "[packages]\nsource = \"https://example.invalid\"\nfetch = \"always\"\n"
            .write(to: project.appendingPathComponent("flashtex.toml"), atomically: true, encoding: .utf8)
        let host = ShellModel(), guest = ShellModel()
        models = [host, guest]
        XCTAssertEqual(host.openTex(at: project.appendingPathComponent("main.tex")), .opened)
        host.liveShare.startHosting()
        try await waitUntil("invitation") { host.liveShare.invite != nil }
        XCTAssertFalse(host.liveShare.allowed.values.contains { $0.hasPrefix(".") }, "dotfiles are never shared")
        XCTAssertEqual(Set(host.liveShare.allowed.values), ["main.tex", "chapters/one.tex"], "flashtex.toml stays the host's own")
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
        // The project is tainted in the trust store (the app's own record,
        // not the files'): untrusted until the user trusts it again, whatever
        // then happens to its files.
        let main = project.appendingPathComponent("main.tex")
        XCTAssertFalse(EngineV3Trust.isTrusted(root: project, main: main), "a guest's text reached the project")
        XCTAssertNil(EngineV3Trust.quarantineValue(chapterURL), "the files themselves carry no mark")
        // An open buffer the guest edits, saved by the app (through whichever
        // save path: the project-files helper keeps no extended attributes),
        // and a file replaced by another tool change nothing.
        try XCTUnwrap(gs.binding(for: try XCTUnwrap(gs.file(atPath: "main.tex")))).localReplace(0..<0, with: "% g\n")
        guest.updateActiveText("% g\n" + guest.activeText)
        try await waitUntil("host buffer") { host.activeText.hasPrefix("% g\n") }
        host.flushPendingAutosave()
        try await waitUntil("saved") { (try? String(contentsOf: main, encoding: .utf8))?.hasPrefix("% g\n") == true }
        try Data("rewritten\n".utf8).write(to: chapterURL, options: .atomic)
        XCTAssertFalse(EngineV3Trust.isTrusted(root: project, main: main), "the taint outlives any save")
        // Trusting the prompt records the taint's subject: trusted again.
        EngineV3Trust.record(root: project, main: main)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: project, main: main))
        // A later session's edits need trusting again.
        EngineV3Trust.taint(root: project, event: "a later session")
        XCTAssertFalse(EngineV3Trust.isTrusted(root: project, main: main))
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
        try session.shareFile(path: "flashtex.toml", text: "[packages]\nfetch = \"always\"\n")
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

    /// A guest takes at most `maxGuestFiles` files (the main file first).
    func testGuestFileCountIsCapped() async throws {
        let saved = LiveShareController.maxGuestFiles
        LiveShareController.maxGuestFiles = 3
        defer { LiveShareController.maxGuestFiles = saved }
        let session = CollabSession(role: .hub, replica: UInt64.random(in: 1...UInt64.max), name: "Host", colourIndex: 0)
        for i in 0..<6 { try session.shareFile(path: "part\(i).tex", text: "\(i)\n") }
        try session.shareFile(path: "main.tex", text: "main\n")
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
        XCTAssertEqual(guest.liveShare.allowed.count, 3)
        XCTAssertTrue(guest.liveShare.allowed.values.contains("main.tex"))
        let root = try XCTUnwrap(guest.liveShare.root)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: root.path).count, 3)
    }

    /// The write itself never follows a link at the target (second layer
    /// under `safeTarget`): the link is replaced, its target untouched.
    func testWriteReplacesALinkInsteadOfFollowingIt() throws {
        let root = temp.appendingPathComponent("project")
        let outside = temp.appendingPathComponent("outside.tex")
        try "outside\n".write(to: outside, atomically: true, encoding: .utf8)
        let link = root.appendingPathComponent("x.tex")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: outside)
        try LiveShareController.writeNoFollow(Data("new\n".utf8), to: link)
        XCTAssertEqual(try String(contentsOf: outside, encoding: .utf8), "outside\n")
        XCTAssertEqual(try FileManager.default.attributesOfItem(atPath: link.path)[.type] as? FileAttributeType, .typeRegular)
        XCTAssertEqual(try String(contentsOf: link, encoding: .utf8), "new\n")
    }

    /// The check and the write are one walk (`writeConfined`): each folder
    /// is opened without following links, relative to the one before, and
    /// the file is made and renamed inside that open folder. A folder
    /// swapped for a link to elsewhere after it was checked cannot redirect
    /// the write (the old check-by-path then write-by-path could).
    func testConfinedWriteCannotBeRedirectedAfterItsCheck() throws {
        let root = temp.appendingPathComponent("project")
        let outside = temp.appendingPathComponent("outside")
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        let chapters = root.appendingPathComponent("chapters")
        let written = try LiveShareController.writeConfined(Data("one\n".utf8), root: root, path: "chapters/one.tex", afterCheck: {
            try? FileManager.default.moveItem(at: chapters, to: root.appendingPathComponent("moved"))
            try? FileManager.default.createSymbolicLink(at: chapters, withDestinationURL: outside)
        })
        XCTAssertFalse(FileManager.default.fileExists(atPath: outside.appendingPathComponent("one.tex").path),
                       "the write followed a link made after the check")
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: outside.path), [])
        XCTAssertNotNil(written)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("moved/one.tex"), encoding: .utf8), "one\n")
    }

    func testConfinedWriteRefusesLinksAndCreatesFolders() throws {
        let root = temp.appendingPathComponent("project")
        let outside = temp.appendingPathComponent("outside")
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        try "outside\n".write(to: outside.appendingPathComponent("x.tex"), atomically: true, encoding: .utf8)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("linked"), withDestinationURL: outside)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("evil.tex"),
                                                   withDestinationURL: outside.appendingPathComponent("x.tex"))
        XCTAssertNil(try LiveShareController.writeConfined(Data("a".utf8), root: root, path: "linked/x.tex"))
        XCTAssertNil(try LiveShareController.writeConfined(Data("a".utf8), root: root, path: "evil.tex"))
        XCTAssertNil(try LiveShareController.writeConfined(Data("a".utf8), root: root, path: "../outside/x.tex"))
        XCTAssertNil(try LiveShareController.writeConfined(Data("a".utf8), root: root, path: ".git/config.tex"))
        XCTAssertEqual(try String(contentsOf: outside.appendingPathComponent("x.tex"), encoding: .utf8), "outside\n")
        XCTAssertNotNil(try LiveShareController.writeConfined(Data("two\n".utf8), root: root, path: "new/dir/two.tex"))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("new/dir/two.tex"), encoding: .utf8), "two\n")
        let control = root.appendingPathComponent("new/dir/control.tex")
        let controlFD = open(control.path, O_WRONLY | O_CREAT | O_EXCL, 0o644)
        XCTAssertGreaterThanOrEqual(controlFD, 0)
        close(controlFD)
        XCTAssertEqual(try FileManager.default.attributesOfItem(atPath: root.appendingPathComponent("new/dir/two.tex").path)[.posixPermissions] as? Int,
                       try FileManager.default.attributesOfItem(atPath: control.path)[.posixPermissions] as? Int)
        // Replacing keeps the file's extended attributes (a quarantine mark).
        let one = root.appendingPathComponent("chapters/one.tex")
        try "old\n".write(to: one, atomically: true, encoding: .utf8)
        XCTAssertEqual(setxattr(one.path, "com.example.mark", "m", 1, 0, 0), 0)
        XCTAssertNotNil(try LiveShareController.writeConfined(Data("new\n".utf8), root: root, path: "chapters/one.tex"))
        XCTAssertEqual(try String(contentsOf: one, encoding: .utf8), "new\n")
        XCTAssertEqual(getxattr(one.path, "com.example.mark", nil, 0, 0, 0), 1)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("chapters").path), ["one.tex"],
                       "no temporary file left behind")
    }

    func testConfinedWritePreservesPrivateFileMode() throws {
        let root = temp.appendingPathComponent("project")
        let file = root.appendingPathComponent("chapters/private.tex")
        try "old\n".write(to: file, atomically: true, encoding: .utf8)
        XCTAssertEqual(chmod(file.path, 0o600), 0)
        XCTAssertNotNil(try LiveShareController.writeConfined(Data("new\n".utf8), root: root, path: "chapters/private.tex"))
        XCTAssertEqual(try String(contentsOf: file, encoding: .utf8), "new\n")
        XCTAssertEqual(try FileManager.default.attributesOfItem(atPath: file.path)[.posixPermissions] as? Int, 0o600)
    }

    func testConfinedWriteGivesReplacementAFreshModificationTime() throws {
        let root = temp.appendingPathComponent("project")
        let file = root.appendingPathComponent("chapters/one.tex")
        try "old\n".write(to: file, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: 1_000_000_000)], ofItemAtPath: file.path)
        let writeStarted = Date()
        XCTAssertNotNil(try LiveShareController.writeConfined(Data("new\n".utf8), root: root, path: "chapters/one.tex"))
        XCTAssertEqual(try String(contentsOf: file, encoding: .utf8), "new\n")
        let mtime = try XCTUnwrap(FileManager.default.attributesOfItem(atPath: file.path)[.modificationDate] as? Date)
        XCTAssertGreaterThanOrEqual(mtime, writeStarted)
    }

    func testConfinedWriteKeepsOpenedRootAfterPathIsSwapped() throws {
        let root = temp.appendingPathComponent("project")
        let fd = open(root.path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
        XCTAssertGreaterThanOrEqual(fd, 0)
        defer { close(fd) }
        let moved = temp.appendingPathComponent("moved-project")
        let outside = temp.appendingPathComponent("outside")
        try FileManager.default.createDirectory(at: outside.appendingPathComponent("chapters"), withIntermediateDirectories: true)
        let outsideFile = outside.appendingPathComponent("chapters/one.tex")
        try "outside\n".write(to: outsideFile, atomically: true, encoding: .utf8)
        try FileManager.default.moveItem(at: root, to: moved)
        try FileManager.default.createSymbolicLink(at: root, withDestinationURL: outside)

        XCTAssertNotNil(try LiveShareController.writeConfined(Data("new\n".utf8), root: root, path: "chapters/one.tex", rootFD: fd))
        XCTAssertEqual(try String(contentsOf: moved.appendingPathComponent("chapters/one.tex"), encoding: .utf8), "new\n")
        XCTAssertEqual(try String(contentsOf: outsideFile, encoding: .utf8), "outside\n")
    }

    /// The host process for session compiles: reads and writes confined, no
    /// TEXMFOUTPUT.
    func testConfinedHostEnvironment() {
        let env = EngineV3HostProcess.confinedEnvironment(["TEXMFOUTPUT": "/", "openin_any": "a", "openout_any_pdftex": "a", "PATH": "/bin"],
                                                          roots: ["/p", "/q"])
        XCTAssertEqual(env["FLASHTEX_CONFINE_READS"], "1")
        XCTAssertEqual(env["FLASHTEX_CONFINE_ROOTS"], "/p:/q")
        for k in ["MKTEXTFM", "MKTEXPK", "MKTEXMF", "MKTEXTEX"] { XCTAssertEqual(env[k], "0", k) }
        XCTAssertEqual(env["openin_any"], "p")
        XCTAssertEqual(env["openout_any"], "p")
        XCTAssertNil(env["TEXMFOUTPUT"])
        XCTAssertNil(env["openout_any_pdftex"])
        XCTAssertEqual(env["PATH"], "/bin")
    }
}
