import Darwin
import Foundation
import XCTest
@testable import FlashTeXMac

/// Project trust (owner decision 9A): a quarantined project compiles with
/// shell escape off until trusted; trusting it records exactly the
/// quarantined item (path, inode, quarantine event) and gives restricted
/// `\write18`, as pdflatex's default. Nothing here touches the app's
/// defaults: each test has its own store file.
final class EngineV3TrustTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-trust-\(getpid())")
    var store: EngineV3Trust.Store!
    var storeFile: URL!
    var made: [URL] = []

    override func setUp() {
        setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1)
        storeFile = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-trust-store-\(UUID().uuidString).json")
        store = .file(storeFile)
    }

    override func tearDown() {
        unsetenv("FLASHTEX_V3_CACHE")
        EngineV3Trust.testSharedFolders = []
        try? FileManager.default.removeItem(at: Self.cache)
        try? FileManager.default.removeItem(at: Self.cache.deletingLastPathComponent().appendingPathComponent(Self.cache.lastPathComponent + "-trust.json"))
        try? FileManager.default.removeItem(at: storeFile)
        for u in made { try? FileManager.default.removeItem(at: u) }
    }

    /// A folder (standing in for a project, or for ~/Downloads) with `paper.tex`.
    func project(_ tex: String = "x", at dir: URL? = nil) throws -> (dir: URL, file: URL) {
        let dir = dir ?? FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-trust-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        made.append(dir)
        let file = dir.appendingPathComponent("paper.tex")
        try tex.write(to: file, atomically: true, encoding: .utf8)
        return (dir, file)
    }

    /// What Safari writes on a download (flags;time;agent;UUID); `value`
    /// copies another item's attribute (the same download event).
    @discardableResult
    func quarantine(_ url: URL, value: String? = nil) -> String {
        let value = value ?? "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Safari;\(UUID().uuidString)"
        XCTAssertEqual(setxattr(url.path, EngineV3Trust.quarantineAttribute, value, value.utf8.count, 0, 0), 0)
        return value
    }

    func trusted(_ root: URL, _ main: URL) -> Bool { EngineV3Trust.isTrusted(root: root, main: main, store: store) }

    func testQuarantineAndRecord() throws {
        let (dir, file) = try project()
        XCTAssertTrue(trusted(dir, file), "made here: trusted")
        XCTAssertTrue(EngineV3Trust.isTrusted(root: nil, main: nil, store: store), "untitled: nothing to run")
        quarantine(file)
        XCTAssertTrue(EngineV3Trust.isQuarantined(file))
        XCTAssertFalse(trusted(dir, file))
        XCTAssertEqual(EngineV3Trust.shellEscape(trusted: false), "off")
        EngineV3Trust.record(root: dir, main: file, store: store)
        XCTAssertTrue(trusted(dir, file))
        XCTAssertEqual(EngineV3Trust.shellEscape(trusted: true), "restricted")
        // Another quarantined folder is not trusted.
        let (other, otherFile) = try project("y")
        quarantine(other)
        XCTAssertFalse(trusted(other, otherFile))
    }

    /// `~/Downloads/paper.tex`: the record is that file, never the folder;
    /// a second download in the same folder asks again.
    func testSingleFileInDownloadsRecordsOnlyThatFile() throws {
        let (downloads, paper) = try project()
        quarantine(paper)
        EngineV3Trust.record(root: downloads, main: paper, store: store)
        XCTAssertTrue(trusted(downloads, paper))
        let records = store.load()
        XCTAssertEqual(records.map(\.path), [EngineV3Trust.canonical(paper).path], "the file, not its folder")
        XCTAssertNotNil(records.first?.quarantine)

        let second = downloads.appendingPathComponent("other.tex")
        try "y".write(to: second, atomically: true, encoding: .utf8)
        quarantine(second)
        XCTAssertFalse(trusted(downloads, second), "a second quarantined file in the same folder")
        XCTAssertTrue(trusted(downloads, paper))
    }

    /// The real shared folders are never recorded as a whole, even when
    /// they themselves carry a quarantine attribute.
    func testSharedFoldersAreNeverSubjects() {
        let shared = EngineV3Trust.sharedFolders
        let downloads = FileManager.default.urls(for: .downloadsDirectory, in: .userDomainMask).map { EngineV3Trust.canonical($0).path }
        XCTAssertFalse(downloads.isEmpty)
        for d in downloads { XCTAssertTrue(shared.contains(d)) }
    }

    /// A quarantined folder (an unpacked archive) records the folder; its
    /// files from the same download are covered, a file from another is not.
    func testUnpackedProjectFolder() throws {
        let (dir, file) = try project()
        let event = quarantine(dir)
        quarantine(file, value: event)
        XCTAssertFalse(trusted(dir, file))
        EngineV3Trust.record(root: dir, main: file, store: store)
        XCTAssertEqual(store.load().map(\.path), [EngineV3Trust.canonical(dir).path])
        XCTAssertTrue(trusted(dir, file))

        let added = dir.appendingPathComponent("added.tex")
        try "z".write(to: added, atomically: true, encoding: .utf8)
        quarantine(added)
        XCTAssertFalse(trusted(dir, added), "a later download into the trusted folder")
    }

    /// A new download unpacked to a trusted path: a new inode and a new
    /// quarantine event, so it asks again.
    func testNewDownloadAtATrustedPathAsksAgain() throws {
        let (dir, file) = try project()
        let event = quarantine(dir)
        quarantine(file, value: event)
        EngineV3Trust.record(root: dir, main: file, store: store)
        XCTAssertTrue(trusted(dir, file))

        try FileManager.default.removeItem(at: dir)
        let (again, againFile) = try project("other content", at: dir)
        let newEvent = quarantine(again)
        quarantine(againFile, value: newEvent)
        XCTAssertFalse(trusted(again, againFile))

        // Even with the old quarantine value (a copy of the same archive
        // unpacked again): a new folder, a new inode.
        try FileManager.default.removeItem(at: dir)
        let (third, thirdFile) = try project("third", at: dir)
        quarantine(third, value: event)
        quarantine(thirdFile, value: event)
        XCTAssertFalse(trusted(third, thirdFile))

        // A single downloaded file replaced by a new download at the same path.
        let (downloads, paper) = try project()
        quarantine(paper)
        EngineV3Trust.record(root: downloads, main: paper, store: store)
        XCTAssertTrue(trusted(downloads, paper))
        try FileManager.default.removeItem(at: paper)
        try "replaced".write(to: paper, atomically: true, encoding: .utf8)
        quarantine(paper)
        XCTAssertFalse(trusted(downloads, paper))
    }

    /// Renaming or moving a trusted item asks again.
    func testRenameOrMoveAsksAgain() throws {
        let (downloads, paper) = try project()
        quarantine(paper)
        EngineV3Trust.record(root: downloads, main: paper, store: store)
        let renamed = downloads.appendingPathComponent("renamed.tex")
        try FileManager.default.moveItem(at: paper, to: renamed)
        XCTAssertTrue(EngineV3Trust.isQuarantined(renamed))
        XCTAssertFalse(trusted(downloads, renamed), "renamed file")

        let (dir, file) = try project()
        let event = quarantine(dir)
        quarantine(file, value: event)
        EngineV3Trust.record(root: dir, main: file, store: store)
        XCTAssertTrue(trusted(dir, file))
        let moved = dir.deletingLastPathComponent().appendingPathComponent("engine-v3-trust-moved-\(UUID().uuidString)")
        try FileManager.default.moveItem(at: dir, to: moved)
        made.append(moved)
        XCTAssertFalse(trusted(moved, moved.appendingPathComponent("paper.tex")), "moved folder")
    }

    /// Other quarantined files in the folder (any can be `\input`): a later
    /// download into a trusted folder, or other downloads beside a single
    /// downloaded file, each need their own record.
    func testOtherDownloadedFilesInTheFolder() throws {
        let (dir, file) = try project()
        let event = quarantine(dir)
        quarantine(file, value: event)
        let style = dir.appendingPathComponent("evil.sty")
        try "\\immediate\\write18{echo}".write(to: style, atomically: true, encoding: .utf8)
        // Unpacked with the folder (same event): covered by the folder's record.
        quarantine(style, value: event)
        EngineV3Trust.record(root: dir, main: file, others: [style], store: store)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: dir, main: file, others: [style], store: store))
        // Downloaded later into the trusted folder: asks again, until recorded.
        quarantine(style)
        XCTAssertFalse(EngineV3Trust.isTrusted(root: dir, main: file, others: [style], store: store))
        let need = try XCTUnwrap(EngineV3Trust.subjects(root: dir, main: file, others: [style]))
        XCTAssertEqual(need.map(\.path), [EngineV3Trust.canonical(dir).path, EngineV3Trust.canonical(style).path])
        EngineV3Trust.record(need, store: store)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: dir, main: file, others: [style], store: store))

        // A single downloaded file in Downloads with another download beside it.
        let (downloads, paper) = try project()
        quarantine(paper)
        let chapter = downloads.appendingPathComponent("chapter.tex")
        try "y".write(to: chapter, atomically: true, encoding: .utf8)
        quarantine(chapter)
        let local = downloads.appendingPathComponent("local.tex")
        try "made here".write(to: local, atomically: true, encoding: .utf8)
        let others = [paper, chapter, local]
        let need2 = try XCTUnwrap(EngineV3Trust.subjects(root: downloads, main: paper, others: others))
        XCTAssertEqual(Set(need2.map(\.path)), [EngineV3Trust.canonical(paper).path, EngineV3Trust.canonical(chapter).path],
                       "the file and the other download, never the folder or a file made here")
        EngineV3Trust.record(root: downloads, main: paper, store: store) // the file alone
        XCTAssertFalse(EngineV3Trust.isTrusted(root: downloads, main: paper, others: others, store: store))
        EngineV3Trust.record(need2, store: store)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: downloads, main: paper, others: others, store: store))
    }

    /// A lone file in a shared folder (a stand-in for ~/Downloads): the
    /// folder is not walked; trust covers the file and what its own download
    /// brought; another download counts only if the document reads it.
    func testLoneFileInASharedFolder() throws {
        let (downloads, paper) = try project("\\documentclass{article}\n\\usepackage{local}\n\\begin{document}\n\\input{chapter}\n\\end{document}\n")
        EngineV3Trust.testSharedFolders = [EngineV3Trust.canonical(downloads).path]
        XCTAssertTrue(EngineV3Trust.isShared(downloads))
        let event = quarantine(paper)
        let chapter = downloads.appendingPathComponent("chapter.tex")
        try "Chapter.".write(to: chapter, atomically: true, encoding: .utf8)
        quarantine(chapter, value: event) // came with paper.tex's download
        let style = downloads.appendingPathComponent("local.sty")
        try "% a package".write(to: style, atomically: true, encoding: .utf8)
        let unrelated = downloads.appendingPathComponent("unrelated.tex")
        try "\\immediate\\write18{x}".write(to: unrelated, atomically: true, encoding: .utf8)
        quarantine(unrelated)
        let texts = ["paper.tex": try String(contentsOf: paper, encoding: .utf8)]

        XCTAssertEqual(Set(EngineV3Trust.referencedFiles(root: downloads, main: "paper.tex", texts: texts).map(\.lastPathComponent)),
                       ["chapter.tex", "local.sty"])
        // Even if a walk listed the unrelated download, a shared folder ignores it.
        var d = EngineV3Trust.decide(root: downloads, main: "paper.tex", texts: texts, walkQuarantined: [unrelated, chapter], store: store)
        XCTAssertFalse(d.trusted)
        XCTAssertEqual(d.need.map(\.path), [EngineV3Trust.canonical(paper).path], "the file only: not the folder, not its own download's chapter, not other downloads")
        EngineV3Trust.record(d.need, store: store)
        XCTAssertTrue(EngineV3Trust.decide(root: downloads, main: "paper.tex", texts: texts, walkQuarantined: [], store: store).trusted)

        // Another download arrives: no new prompt.
        let later = downloads.appendingPathComponent("later.tex")
        try "y".write(to: later, atomically: true, encoding: .utf8)
        quarantine(later)
        XCTAssertTrue(EngineV3Trust.decide(root: downloads, main: "paper.tex", texts: texts, walkQuarantined: [later], store: store).trusted)

        // A package the document loads, from another download: it asks.
        quarantine(style)
        d = EngineV3Trust.decide(root: downloads, main: "paper.tex", texts: texts, walkQuarantined: [], store: store)
        XCTAssertFalse(d.trusted)
        XCTAssertEqual(d.need.map(\.path), [EngineV3Trust.canonical(style).path])
        EngineV3Trust.record(d.need, store: store)
        XCTAssertTrue(EngineV3Trust.decide(root: downloads, main: "paper.tex", texts: texts, walkQuarantined: [], store: store).trusted)

        // The editor's text now inputs the unrelated download: it asks.
        let edited = ["paper.tex": texts["paper.tex"]! + "\\input{unrelated}\n"]
        d = EngineV3Trust.decide(root: downloads, main: "paper.tex", texts: edited, walkQuarantined: [], store: store)
        XCTAssertFalse(d.trusted)
        XCTAssertEqual(d.need.map(\.path), [EngineV3Trust.canonical(unrelated).path])
    }

    /// The button records the identities the prompt was computed from: if
    /// the main file was replaced in between, the new one is not trusted.
    func testRecordingThePromptsIdentitiesNotAFreshLook() throws {
        let (downloads, paper) = try project()
        quarantine(paper)
        let prompt = try XCTUnwrap(EngineV3Trust.subjects(root: downloads, main: paper))
        try FileManager.default.removeItem(at: paper)
        try "swapped".write(to: paper, atomically: true, encoding: .utf8)
        quarantine(paper)
        EngineV3Trust.record(prompt, store: store)
        XCTAssertFalse(trusted(downloads, paper), "the replaced file is not what was trusted")
    }

    /// The quarantine event ignores the flags (Gatekeeper updates them).
    func testQuarantineEvent() {
        XCTAssertEqual(EngineV3Trust.quarantineEvent("0083;66f0a1b2;Safari;ABC-123"), "ABC-123")
        XCTAssertEqual(EngineV3Trust.quarantineEvent("00c3;66f0a1b2;Safari;ABC-123"), "ABC-123")
        XCTAssertEqual(EngineV3Trust.quarantineEvent("0083;66f0a1b2;Safari"), "66f0a1b2;Safari")
    }

    /// An instance with its own cache keeps its records beside it; a test
    /// process without one uses a temporary file. Never the app's defaults.
    func testStoreIsolation() throws {
        let sibling = Self.cache.deletingLastPathComponent().appendingPathComponent(Self.cache.lastPathComponent + "-trust.json")
        guard case .file(let url) = EngineV3Trust.Store.current else { return XCTFail("defaults store with FLASHTEX_V3_CACHE set") }
        XCTAssertEqual(url.standardizedFileURL.path, sibling.standardizedFileURL.path)
        let before = UserDefaults.standard.data(forKey: EngineV3Trust.recordKey)
        let (dir, file) = try project()
        quarantine(file)
        EngineV3Trust.record(root: dir, main: file)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: dir, main: file))
        XCTAssertTrue(FileManager.default.fileExists(atPath: sibling.path))
        XCTAssertEqual(UserDefaults.standard.data(forKey: EngineV3Trust.recordKey), before)
        unsetenv("FLASHTEX_V3_CACHE")
        guard case .file(let t) = EngineV3Trust.Store.current else { return XCTFail("defaults store in a test process") }
        XCTAssertTrue(t.lastPathComponent.hasPrefix("flashtex-engine-v3-trust-"))
    }

    /// End to end through the host: `\pdfshellescape` is 0 with shell escape
    /// off and 2 when restricted; the document has a second page only when 2.
    @MainActor
    func testQuarantinedProjectCompilesWithShellEscapeOffUntilTrusted() async throws {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built") }
        let (dir, file) = try project("\\documentclass{article}\n\\begin{document}\nOne.\n\\ifnum\\pdfshellescape=2 \\newpage Two.\\fi\n\\end{document}\n")
        quarantine(file)
        // The folder stands in for ~/Downloads, with another download in it
        // that the document does not read: it never counts.
        EngineV3Trust.testSharedFolders = [EngineV3Trust.canonical(dir).path]
        let unrelated = dir.appendingPathComponent("unrelated.tex")
        try "\\immediate\\write18{x}".write(to: unrelated, atomically: true, encoding: .utf8)
        quarantine(unrelated)
        // The records go to the store beside FLASHTEX_V3_CACHE (setUp), removed in tearDown.
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        defer { if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) } }
        func wait(_ cond: @escaping () -> Bool) async throws {
            let start = Date()
            while !cond() { if Date().timeIntervalSince(start) > 60 { XCTFail("timeout"); return }; try await Task.sleep(nanoseconds: 50_000_000) }
        }
        let m = ShellModel()
        XCTAssertEqual(m.openTex(at: file, dirty: .discard), .opened)
        m.engineV3Enabled = true
        m.engineV3.start(model: m)
        defer { m.engineV3.stop() }
        try await wait { m.engineV3.statusNote.hasPrefix("ok") }
        XCTAssertFalse(m.engineV3.projectTrusted)
        XCTAssertEqual(m.engineV3.pageCount, 1, "shell escape off: \\pdfshellescape = 0")
        XCTAssertEqual(m.engineV3.trustOtherCount, 0, "the other download is not counted")
        XCTAssertEqual(m.engineV3.trustPrompt?.need.map(\.path), [EngineV3Trust.canonical(file).path])

        m.engineV3.trustProject()
        // Decided again off the main thread; the compile follows the decision.
        try await wait { m.engineV3.projectTrusted && m.engineV3.statusNote.hasPrefix("ok") && m.engineV3.pageCount == 2 }
        XCTAssertTrue(m.engineV3.projectTrusted)
        XCTAssertEqual(m.engineV3.pageCount, 2, "trusted: restricted, \\pdfshellescape = 2")

        // A later download into the folder, then an explicit compile (a new walk): no prompt.
        let later = dir.appendingPathComponent("later.tex")
        try "y".write(to: later, atomically: true, encoding: .utf8)
        quarantine(later)
        var dones = 0
        m.engineV3.afterEvent = { if case .done = $0 { dones += 1 } }
        m.engineV3.compile(model: m, reason: "explicit")
        try await wait { dones > 0 }
        XCTAssertTrue(m.engineV3.projectTrusted)
        XCTAssertNil(m.engineV3.trustPrompt)
        XCTAssertEqual(m.engineV3.pageCount, 2)

        // Persisted: the next open of the same file is trusted; the folder is not.
        let again = ShellModel()
        XCTAssertEqual(again.openTex(at: file, dirty: .discard), .opened)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: again.project.projectRoot, main: file))
        XCTAssertEqual(EngineV3Trust.Store.current.load().map(\.path), [EngineV3Trust.canonical(file).path])
        _ = dir
    }
}
