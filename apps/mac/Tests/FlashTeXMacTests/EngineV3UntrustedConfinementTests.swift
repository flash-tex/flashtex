import Darwin
import Foundation
import XCTest
@testable import FlashTeXMac

/// An untrusted project (a quarantined download, a received project, an
/// arXiv tarball) compiles in a read-confined host until it is trusted
/// (Commander ruling, lane TRANSFER-AUDIT): the app compiles a project the
/// moment it is opened, which pdflatex never does unasked. Each read the
/// engine refuses is one Problems row with "Trust This Project". Git
/// clones and projects made here are trusted and compile as pdflatex does.
/// No host is started here (EngineV3TrustTests has the end-to-end test).
final class EngineV3UntrustedConfinementTests: XCTestCase {
    private var made: [URL] = []
    private var storeFile: URL!
    private var store: EngineV3Trust.Store!

    override func setUp() {
        storeFile = FileManager.default.temporaryDirectory.appendingPathComponent("untrusted-confine-store-\(UUID().uuidString).json")
        store = .file(storeFile)
    }

    override func tearDown() {
        for u in made { try? FileManager.default.removeItem(at: u) }
        try? FileManager.default.removeItem(at: storeFile)
    }

    private func project() throws -> (dir: URL, main: URL) {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("untrusted-confine-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        made.append(dir)
        let main = dir.appendingPathComponent("main.tex")
        try "x".write(to: main, atomically: true, encoding: .utf8)
        return (dir, main)
    }

    private func quarantine(_ url: URL) {
        let v = "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Safari;\(UUID().uuidString)"
        XCTAssertEqual(setxattr(url.path, EngineV3Trust.quarantineAttribute, v, v.utf8.count, 0, 0), 0)
    }

    func testAProjectMadeHereIsNeverConfined() throws {
        let (dir, main) = try project()
        XCTAssertFalse(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: nil, store: store))
        XCTAssertFalse(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: true, store: store))
    }

    func testADownloadIsConfinedFromItsFirstCompileUntilTrusted() throws {
        let (dir, main) = try project()
        quarantine(main)
        // Before the walk has decided: a quarantined main file not on record fails closed.
        XCTAssertTrue(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: nil, store: store))
        // The walk's decision.
        XCTAssertTrue(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: false, store: store))
        XCTAssertFalse(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: true, store: store))
        // Trusted (a persisted, per-project record): undecided no longer confines.
        EngineV3Trust.record(root: dir, main: main, store: store)
        XCTAssertFalse(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: nil, store: store))
    }

    func testAQuarantinedFolderIsConfinedToo() throws {
        let (dir, main) = try project()
        quarantine(dir)
        XCTAssertTrue(EngineV3Session.wantsUntrustedConfinement(root: dir, main: main, decided: nil, store: store))
    }

    func testTheAllowedRoots() throws {
        let (dir, _) = try project()
        let roots = EngineV3Session.untrustedRoots(projectRoot: dir)
        XCTAssertEqual(roots.first, dir.resolvingSymlinksInPath().path, "the project folder first")
        XCTAssertTrue(roots.contains("/System/Library/Fonts"))
        XCTAssertTrue(roots.contains("/Library/Fonts"))
        if let res = Bundle.main.resourceURL { XCTAssertTrue(roots.contains(res.resolvingSymlinksInPath().path)) }
        XCTAssertFalse(roots.contains(NSHomeDirectory()), "never the whole home folder")
        XCTAssertFalse(roots.contains(where: { $0.hasSuffix("/.ssh") || $0 == "/" }))
    }

    func testTheHostEnvironmentNamesTheRefusalsFile() {
        let env = EngineV3HostProcess.confinedEnvironment(["PATH": "/bin"], roots: ["/p"], refusals: "/tmp/r.refused")
        XCTAssertEqual(env["FLASHTEX_CONFINE_REFUSED"], "/tmp/r.refused")
        XCTAssertEqual(env["FLASHTEX_CONFINE_READS"], "1")
        XCTAssertNil(EngineV3HostProcess.confinedEnvironment(["PATH": "/bin"], roots: ["/p"])["FLASHTEX_CONFINE_REFUSED"])
    }

    func testRefusalsAreReadOnceEach() throws {
        let (dir, _) = try project()
        let file = dir.appendingPathComponent("r.refused")
        XCTAssertEqual(EngineV3HostProcess.readRefusals(at: file.path), [])
        // Each engine process appends its own (a Unicode host runs one per compile).
        try "/etc/hosts\n../shared/macros.tex\n/etc/hosts\nfig.pdf\t/Users/someone/.ssh/id_rsa\n".write(to: file, atomically: true, encoding: .utf8)
        XCTAssertEqual(EngineV3HostProcess.readRefusals(at: file.path),
                       ["/etc/hosts", "../shared/macros.tex", "fig.pdf\t/Users/someone/.ssh/id_rsa"])
    }

    func testEachRefusalIsOneProblemsRowWithTheTrustAction() {
        let rows = EngineV3Session.blockedReadProblems(["/etc/hosts", "fig.pdf\t/Users/someone/.ssh/id_rsa"])
        XCTAssertEqual(rows.count, 2)
        XCTAssertTrue(rows.allSatisfy { $0.code == EngineV3Session.untrustedReadCode && $0.severity == .warning })
        XCTAssertEqual(rows[0].message, "Blocked reading /etc/hosts: this project came from another computer and is not trusted yet.")
        XCTAssertEqual(rows[1].message, "Blocked reading fig.pdf (it leads to /Users/someone/.ssh/id_rsa): this project came from another computer and is not trusted yet.")
        XCTAssertEqual(EngineV3Session.blockedReadProblems([]), [])
    }
}
