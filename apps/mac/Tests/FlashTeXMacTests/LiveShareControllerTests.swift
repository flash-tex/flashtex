import AppKit
import FlashTeXCollabCore
import FlashTeXCollabSession
import XCTest
@testable import FlashTeXMac

/// Live Share end to end through the app's controller (LiveShare.swift):
/// a host `ShellModel` with a real project folder, a guest `ShellModel`
/// joining with the invitation link over loopback TLS (as a second app
/// instance on one Mac would), approval in the session panel, the guest's
/// copy opened as a project, edits both ways, and leaving. Nothing is
/// advertised and nothing is written outside a temporary folder.
@MainActor
final class LiveShareControllerTests: XCTestCase {
    private var temp: URL!
    private var models: [ShellModel] = []

    override func setUp() async throws {
        temp = FileManager.default.temporaryDirectory.appendingPathComponent("liveshare-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: temp.appendingPathComponent("project/chapters"), withIntermediateDirectories: true)
        LiveShareController.testLoopbackOnly = true
        LiveShareController.sessionBaseOverride = temp.appendingPathComponent("guests")
        LiveShareController.enabledOverride = true
    }

    override func tearDown() async throws {
        for m in models where m.liveShare.isActive { m.liveShare.leave() }
        models = []
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

    static let main = "\\documentclass{article}\n\\begin{document}\n\\input{chapters/one}\nHello.\n\\end{document}\n"
    static let chapter = "\\section{One}\nFirst chapter.\n"

    /// Host + guest, approved for editing; returns both models.
    private func hostAndGuest(decision: CollabHub.Decision = .allowEdit) async throws -> (host: ShellModel, guest: ShellModel) {
        let project = temp.appendingPathComponent("project")
        try Self.main.write(to: project.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try Self.chapter.write(to: project.appendingPathComponent("chapters/one.tex"), atomically: true, encoding: .utf8)
        try Data([0x89, 0x50, 0x4E, 0x47]).write(to: project.appendingPathComponent("figure.png"))
        try FileManager.default.createDirectory(at: project.appendingPathComponent(".flashtex"), withIntermediateDirectories: true)
        try "never shared".write(to: project.appendingPathComponent(".flashtex/state.tex"), atomically: true, encoding: .utf8)
        let host = ShellModel(), guest = ShellModel()
        models = [host, guest]
        XCTAssertEqual(host.openTex(at: project.appendingPathComponent("main.tex")), .opened)
        host.liveShare.startHosting()
        XCTAssertTrue(host.liveShare.isHost, host.liveShare.note ?? "")
        try await waitUntil("invitation") { host.liveShare.invite != nil }
        let link = try XCTUnwrap(host.liveShare.invite?.link)
        guest.liveShare.join(link: link, name: "Bob", disposition: .discard)
        try await waitUntil("approval request") { !host.liveShare.approvals.isEmpty }
        XCTAssertEqual(host.liveShare.approvals.first?.name, "Bob")
        XCTAssertEqual(guest.liveShare.phase, .awaitingApproval)
        host.liveShare.answer(host.liveShare.approvals[0], decision)
        try await waitUntil("guest project open") { guest.liveShare.phase == .joined && guest.documentURL != nil }
        return (host, guest)
    }

    func testGuestJoinsAndOpensTheSharedProject() async throws {
        let (host, guest) = try await hostAndGuest()
        let root = try XCTUnwrap(guest.liveShare.root)
        XCTAssertTrue(root.path.hasPrefix(temp.path), "the guest's copy stays in the session folder")
        XCTAssertEqual(guest.documentURL?.lastPathComponent, "main.tex")
        XCTAssertEqual(guest.activeText, Self.main)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("chapters/one.tex"), encoding: .utf8), Self.chapter)
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent(".flashtex/state.tex").path), "hidden folders are never shared")
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("figure.png").path), "figures are P2")
        XCTAssertNotNil(host.liveShare.link(for: "main.tex"))
        XCTAssertNotNil(guest.liveShare.link(for: "main.tex"))
        try await waitUntil("both listed") { host.liveShare.participants.count == 2 && guest.liveShare.participants.count == 2 }
        XCTAssertEqual(Set(host.liveShare.participants.map(\.name)), [LiveShareController.defaultDisplayName, "Bob"])
        XCTAssertNotNil(host.liveShare.invite, "a fresh invitation replaces the one just used")
    }

    /// An edit through one side's binding reaches the other side's model (and
    /// the files on disk for a source nobody has open).
    func testEditsFlowBothWaysIntoModelsAndFiles() async throws {
        let (host, guest) = try await hostAndGuest()
        let hostLink = try XCTUnwrap(host.liveShare.link(for: "main.tex"))
        // Typing as the editor would, with no editor attached: the open
        // buffer of the other side follows.
        let at = (Self.main as NSString).range(of: "Hello.").location
        hostLink.binding.localReplace(at..<at, with: "Hi! ")
        host.updateActiveText((host.activeText as NSString).replacingCharacters(in: NSRange(location: at, length: 0), with: "Hi! "))
        try await waitUntil("guest buffer") { guest.activeText.contains("Hi! Hello.") }
        let guestLink = try XCTUnwrap(guest.liveShare.link(for: "main.tex"))
        guestLink.binding.localReplace(0..<0, with: "% from Bob\n")
        guest.updateActiveText("% from Bob\n" + guest.activeText)
        try await waitUntil("host buffer") { host.activeText.hasPrefix("% from Bob\n") }
        XCTAssertEqual(host.activeText, guest.activeText)
        // A source the guest has not opened is kept current on its disk.
        let session = try XCTUnwrap(host.liveShare.session)
        let chapter = try XCTUnwrap(session.file(atPath: "chapters/one.tex"))
        let b = try XCTUnwrap(session.binding(for: chapter))
        b.localReplace(0..<0, with: "% edited\n")
        let guestFile = try XCTUnwrap(guest.liveShare.root).appendingPathComponent("chapters/one.tex")
        try await waitUntil("guest disk") { (try? String(contentsOf: guestFile, encoding: .utf8))?.hasPrefix("% edited\n") == true }
    }

    /// A viewer's editor is read-only; ending the session tells every guest,
    /// who keeps its copy.
    func testViewerCannotEditAndLeavingEndsTheSession() async throws {
        let (host, guest) = try await hostAndGuest(decision: .allowView)
        XCTAssertFalse(guest.liveShare.canEdit)
        host.liveShare.leave()
        try await waitUntil("guest notified") { if case .ended = guest.liveShare.phase { return true } else { return false } }
        XCTAssertFalse(guest.liveShare.isActive)
        XCTAssertNil(guest.liveShare.link(for: "main.tex"))
        XCTAssertEqual(guest.activeText, Self.main, "the guest keeps its copy")
    }

    func testHostingNeedsASavedProject() {
        let model = ShellModel()
        models = [model]
        model.replaceProject(entryText: "x")
        model.documentURL = nil
        model.liveShare.startHosting()
        XCTAssertFalse(model.liveShare.isActive)
        XCTAssertEqual(model.liveShare.note, "Save the document first: Live Share shares a project folder.")
    }

    func testBadInvitationSaysWhy() {
        let model = ShellModel()
        models = [model]
        model.liveShare.join(link: "https://example.com", name: "x", disposition: .none)
        XCTAssertEqual(model.liveShare.note, "This is not a FlashTeX Live Share invitation.")
        XCTAssertFalse(model.liveShare.isActive)
    }
}
