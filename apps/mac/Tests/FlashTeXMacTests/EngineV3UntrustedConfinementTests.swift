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
        EngineV3Trust.testSharedFolders = []
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

    @discardableResult
    private func quarantine(_ url: URL, value: String? = nil) -> String {
        let v = value ?? "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Safari;\(UUID().uuidString)"
        XCTAssertEqual(setxattr(url.path, EngineV3Trust.quarantineAttribute, v, v.utf8.count, 0, 0), 0)
        return v
    }

    /// A stand-in for ~/Downloads: `main.tex` (downloaded), which reads
    /// `chapter.tex` (another download), beside `private.txt` (the user's)
    /// and `later.tex` (a later download nothing names).
    private func sharedFolder() throws -> (dir: URL, main: URL, chapter: URL, privateFile: URL, later: URL, texts: [String: String]) {
        let (dir, main) = try project()
        EngineV3Trust.testSharedFolders = [EngineV3Trust.canonical(dir).path]
        let text = "\\documentclass{article}\n\\begin{document}\n\\input{chapter}\n\\end{document}\n"
        try text.write(to: main, atomically: true, encoding: .utf8)
        quarantine(main)
        let chapter = dir.appendingPathComponent("chapter.tex"), privateFile = dir.appendingPathComponent("private.txt")
        let later = dir.appendingPathComponent("later.tex")
        try "Chapter.".write(to: chapter, atomically: true, encoding: .utf8)
        try "secret".write(to: privateFile, atomically: true, encoding: .utf8)
        try "\\input{/etc/hosts}".write(to: later, atomically: true, encoding: .utf8)
        quarantine(chapter)
        quarantine(later)
        return (dir, main, chapter, privateFile, later, ["main.tex": text])
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

    /// #2026 review, finding 1: an untrusted lone file in a shared folder
    /// may read itself and the files it names, never the rest of the folder
    /// (the copy links the user's other files there too).
    func testASharedFolderAllowsOnlyTheMainFileAndTheFilesItReads() throws {
        let f = try sharedFolder()
        let d = EngineV3Trust.decide(root: f.dir, main: "main.tex", texts: f.texts, walkQuarantined: [], store: store)
        XCTAssertFalse(d.trusted)
        let shared = try XCTUnwrap(d.shared, "a shared folder's decision names the files it may read")
        XCTAssertEqual(Set(shared.map { EngineV3Trust.canonical($0).path }),
                       Set([f.main, f.chapter].map { EngineV3Trust.canonical($0).path }))
        let roots = EngineV3Session.untrustedRoots(projectRoot: f.dir, sharedFiles: shared)
        XCTAssertFalse(roots.contains(f.dir.resolvingSymlinksInPath().path), "never the whole shared folder")
        XCTAssertTrue(roots.contains(f.main.resolvingSymlinksInPath().path))
        XCTAssertTrue(roots.contains(f.chapter.resolvingSymlinksInPath().path))
        XCTAssertFalse(roots.contains { f.privateFile.resolvingSymlinksInPath().path.hasPrefix($0 + "/") || $0 == f.privateFile.resolvingSymlinksInPath().path })
        XCTAssertTrue(EngineV3Session.areUntrustedRoots(roots))
        // A project folder still allows itself, whole.
        let (dir, _) = try project()
        XCTAssertNil(EngineV3Trust.decide(root: dir, main: "main.tex", texts: [:], walkQuarantined: [], store: store).shared)
    }

    /// #2026 review, finding 5: in a shared folder the copy never links a
    /// download the trust decision did not count (another download than the
    /// main file's that the document does not name), so `\input` of a name a
    /// macro builds cannot reach it, even once the file is trusted; the
    /// user's own files and the files the document names are linked.
    func testASharedFoldersCopyLinksNoDownloadTheDecisionDidNotCount() throws {
        let f = try sharedFolder()
        let mirror = EngineV3Mirror(source: f.dir, session: 7_000 + Int.random(in: 0 ..< 1000))
        defer { try? FileManager.default.removeItem(at: mirror.base) }
        let fm = FileManager.default
        // A link an earlier walk made (the file was not quarantined then) goes too.
        try fm.createSymbolicLink(at: mirror.root.appendingPathComponent("later.tex"), withDestinationURL: f.later)
        let refs = EngineV3Trust.referencedFiles(root: f.dir, main: "main.tex", texts: f.texts)
        _ = mirror.sync(except: ["main.tex"], fingerprints: false, quarantine: false,
                        sharedLinks: EngineV3Trust.SharedLinks(main: f.main, referenced: refs))
        func linked(_ name: String) -> Bool { (try? fm.destinationOfSymbolicLink(atPath: mirror.root.appendingPathComponent(name).path)) != nil }
        XCTAssertTrue(linked("chapter.tex"), "a file the document names")
        XCTAssertTrue(linked("private.txt"), "the user's own file (not quarantined)")
        XCTAssertFalse(linked("later.tex"), "a download nothing counted")
        // The main file's own download is linked whatever its name.
        let sibling = f.dir.appendingPathComponent("figure.tex")
        try "fig".write(to: sibling, atomically: true, encoding: .utf8)
        quarantine(sibling, value: try XCTUnwrap(EngineV3Trust.quarantineValue(f.main)))
        _ = mirror.sync(except: ["main.tex"], fingerprints: false, quarantine: false,
                        sharedLinks: EngineV3Trust.SharedLinks(main: f.main, referenced: refs))
        XCTAssertTrue(linked("figure.tex"))
        XCTAssertFalse(linked("later.tex"))
    }

    /// #2026 review, finding 2: an untrusted project's host finds files
    /// along the search paths only in its roots and the distribution's trees
    /// (`FLASHTEX_CONFINE_SEARCH=roots`), never in `TEXINPUTS` or `TEXMFHOME`
    /// directories; a Live Share host (the project alone) keeps the engine's
    /// TeX-tree rule.
    func testAnUntrustedHostSearchesOnlyItsRootsAndTheTrees() throws {
        let (dir, _) = try project()
        let roots = EngineV3Session.untrustedRoots(projectRoot: dir)
        XCTAssertTrue(EngineV3Session.areUntrustedRoots(roots))
        XCTAssertFalse(EngineV3Session.areUntrustedRoots([dir.path]), "Live Share's roots")
        XCTAssertFalse(roots.contains { $0.hasSuffix("/Library/texmf") }, "never TEXMFHOME")
        let env = EngineV3HostProcess.confinedEnvironment(["PATH": "/bin", "FLASHTEX_CONFINE_SEARCH": "roots"], roots: roots, searchInRoots: true)
        XCTAssertEqual(env["FLASHTEX_CONFINE_SEARCH"], "roots")
        let liveShare = EngineV3HostProcess.confinedEnvironment(["PATH": "/bin", "FLASHTEX_CONFINE_SEARCH": "roots"], roots: [dir.path])
        XCTAssertNil(liveShare["FLASHTEX_CONFINE_SEARCH"], "not inherited from the app's environment")
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
