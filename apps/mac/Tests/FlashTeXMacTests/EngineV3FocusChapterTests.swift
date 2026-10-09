import Foundation
import PDFKit
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// Chapter focus (lane FOCUS-CHAPTER): the preview compiles LaTeX's own
/// `\includeonly{chapter}` (host COMPILE `includeonly`) in an output folder
/// of its own, offered only for an `\include`d file; Export and Print stay
/// the whole document unless the user asks for the focused one. The rules
/// are pure; the end-to-end case needs a built `flashtex-host` and TeX Live
/// (skipped otherwise) and uses a private v3 cache.
@MainActor
final class EngineV3FocusChapterTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-focus-\(getpid())")
    private var env = EnvironmentOverride()
    private var dirs: [URL] = []

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
    }

    override func tearDown() {
        env.restore()
        for d in dirs { try? FileManager.default.removeItem(at: d) }
    }

    override class func tearDown() { try? FileManager.default.removeItem(at: cache) }

    private func tempDir(_ name: String) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-focus-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        dirs.append(dir)
        return dir
    }

    // MARK: the request

    func testIncludeOnlyIsSentOnlyWhenSet() {
        var r = DL3CompileRequest(id: 1, root: "/r", main: "main.tex")
        XCTAssertNil(r.json["includeonly"])
        r.includeOnly = ["chapters/03", "chapters/04"]
        XCTAssertEqual(r.json["includeonly"], .array([.string("chapters/03"), .string("chapters/04")]))
        XCTAssertEqual(DL3CompileRequest.includeOnlyCapability, "includeonly")
    }

    // MARK: which file is a chapter

    private func node(_ kind: ProjectIncludes.Kind, _ argument: String, resolved: String?, literal: Bool = true,
                      state: ProjectDocuments.Discovered.State = .available) -> ProjectDocuments.ClosureNode {
        let ref = ProjectIncludes.Reference(kind: kind, argument: argument, startByte: 0, endByte: 0,
                                           argumentStartByte: 0, argumentEndByte: 0, literal: literal)
        return .init(reference: ref, from: "main.tex", depth: 1, candidates: resolved.map { [$0] } ?? [], resolvedPath: resolved, state: state)
    }

    /// Only an `\include`d file is a chapter; its name is the argument as
    /// written (what `\includeonly` compares), never an `\input` file's.
    func testOnlyAnIncludedFileIsAChapter() {
        let closure = ProjectDocuments.Closure(nodes: [
            node(.input, "preamble/macros", resolved: "preamble/macros.tex"),
            node(.include, "chapters/ch1", resolved: "chapters/ch1.tex"),
            node(.include, "chapters/ch2.tex", resolved: "chapters/ch2.tex"),
            node(.include, "\\chapterdir/ch3", resolved: nil, literal: false, state: .unresolvable("argument needs macro expansion")),
            node(.include, "chapters/a,b", resolved: "chapters/a,b.tex"),
        ], paths: ["preamble/macros.tex", "chapters/ch1.tex", "chapters/ch2.tex", "chapters/a,b.tex"])
        XCTAssertEqual(EngineV3Focus.includeName(for: "chapters/ch1.tex", in: closure), "chapters/ch1")
        XCTAssertEqual(EngineV3Focus.includeName(for: "chapters/ch2.tex", in: closure), "chapters/ch2.tex")
        XCTAssertNil(EngineV3Focus.includeName(for: "preamble/macros.tex", in: closure), "an \\input file is not offered")
        XCTAssertNil(EngineV3Focus.includeName(for: "main.tex", in: closure))
        XCTAssertNil(EngineV3Focus.includeName(for: "chapters/a,b.tex", in: closure), "a name \\includeonly cannot carry")
    }

    /// The names the host takes (host/server.rs `includeonly_list`).
    func testNamesMatchTheHostsRules() {
        for ok in ["ch1", "chapters/03", "part 2/ch.tex", "ü/ch"] { XCTAssertTrue(EngineV3Focus.validName(ok), ok) }
        for bad in ["", " lead", "trail ", "a,b", "a{b", "a}b", "\\x", "a%", "#1", "\"q\"", "/abs", "../up", "a/../b", "line\nbreak"] {
            XCTAssertFalse(EngineV3Focus.validName(bad), bad)
        }
    }

    // MARK: the focus output folder

    /// Started from the whole document's output: its auxiliary files and
    /// folders, not its PDF or log, and nothing left of an earlier focus.
    func testTheFocusFolderStartsFromTheWholeDocumentsFiles() throws {
        let base = try tempDir("seed")
        let out = base.appendingPathComponent("out"), focus = EngineV3Focus.output(base: base)
        let fm = FileManager.default
        try fm.createDirectory(at: out.appendingPathComponent("chapters"), withIntermediateDirectories: true)
        try fm.createDirectory(at: out.appendingPathComponent("empty"), withIntermediateDirectories: true)
        for (path, text) in [("main.aux", "\\@input{chapters/ch1.aux}"), ("main.toc", "toc"), ("main.bbl", "bbl"),
                             ("chapters/ch1.aux", "\\newlabel{x}{{1}{7}}"), ("main.pdf", "%PDF"), ("main.log", "log")] {
            try text.write(to: out.appendingPathComponent(path), atomically: true, encoding: .utf8)
        }
        try fm.createSymbolicLink(at: out.appendingPathComponent("link.aux"), withDestinationURL: out.appendingPathComponent("main.aux"))
        try fm.createDirectory(at: focus, withIntermediateDirectories: true)
        try "stale".write(to: focus.appendingPathComponent("old.aux"), atomically: true, encoding: .utf8)

        EngineV3Focus.seed(from: out, to: focus)
        XCTAssertEqual(focus.lastPathComponent, "out-focus")
        for path in ["main.aux", "main.toc", "main.bbl", "chapters/ch1.aux"] {
            XCTAssertEqual(try String(contentsOf: focus.appendingPathComponent(path), encoding: .utf8),
                           try String(contentsOf: out.appendingPathComponent(path), encoding: .utf8), path)
        }
        var isDir: ObjCBool = false
        XCTAssertTrue(fm.fileExists(atPath: focus.appendingPathComponent("empty").path, isDirectory: &isDir) && isDir.boolValue,
                      "every folder: \\include{empty/x} writes there")
        for gone in ["main.pdf", "main.log", "link.aux", "old.aux"] {
            XCTAssertFalse(fm.fileExists(atPath: focus.appendingPathComponent(gone).path), gone)
        }
    }

    // MARK: the job

    func testTheJobFollowsScopeCapabilityAndExport() throws {
        let model = ShellModel()
        defer { model.engineV3.stop() }
        let mirror = EngineV3Mirror(source: nil, session: 9_500 + Int.random(in: 0 ..< 100))
        defer { try? FileManager.default.removeItem(at: mirror.base) }
        let f = EngineV3Focus()
        XCTAssertNil(f.job(model: model, main: "main.tex", project: mirror, offered: true, exporting: false), "no focus")
        let scope = EngineV3Focus.Scope(root: model.project.projectRoot, main: "main.tex", generation: model.projectGeneration)
        f.set([.init(name: "chapters/ch2", path: "chapters/ch2.tex")], scope: scope)
        XCTAssertNil(f.job(model: model, main: "main.tex", project: mirror, offered: false, exporting: false), "an older host")
        XCTAssertTrue(f.isActive)
        let job = try XCTUnwrap(f.job(model: model, main: "main.tex", project: mirror, offered: true, exporting: false))
        XCTAssertEqual(job.names, ["chapters/ch2"])
        XCTAssertEqual(job.output, EngineV3Focus.output(base: mirror.base))
        _ = f.job(model: model, main: "main.tex", project: mirror, offered: true, exporting: false)
        XCTAssertEqual(f.seeds, 1, "the folder starts once per chapter set")
        XCTAssertNil(f.job(model: model, main: "main.tex", project: mirror, offered: true, exporting: true), "Export is the whole document")
        f.exportFocused = true
        XCTAssertNotNil(f.job(model: model, main: "main.tex", project: mirror, offered: true, exporting: true), "unless the user chose it")
        XCTAssertNil(f.job(model: model, main: "other.tex", project: mirror, offered: true, exporting: false), "another main file")
        XCTAssertFalse(f.isActive, "a focus never outlives its document")
    }

    // MARK: end to end

    static func chapter(_ n: Int) -> String {
        let words = ["alpha", "beta", "gamma", "delta", "kernel", "glue", "penalty", "boxes"]
        var s = "\\chapter{Chapter \(n)}\\label{ch:\(n)}\nSee Chapter~\\ref{ch:\(n % 3 + 1)} on page~\\pageref{ch:\(n % 3 + 1)}.\n\n"
        for p in 0 ..< 14 {
            s += (0 ..< 80).map { words[(n * 5 + p * 3 + $0) % words.count] }.joined(separator: " ") + ".\n\n"
        }
        return s
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    private func export(_ model: ShellModel, to dest: URL) async -> ExportSession.Report {
        await withCheckedContinuation { c in model.exportPDFEngineV3(to: .recordingCurrentDisk(dest)) { c.resume(returning: $0) } }
    }

    /// A three-chapter `\include` book: the whole document (its chapter
    /// folder made in the output, so `chapters/ch1.aux` can be written),
    /// focused on chapter 2 (fewer pages, the banner's state, the focus
    /// folder started from the whole document's `.aux`), Export the whole
    /// document while focused, then Show All.
    func testFocusCompilesIncludeOnlyAndExportStaysWhole() async throws {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built (cargo build --release -p flashtex-engine --bin flashtex-host)") }
        let dir = try tempDir("book")
        try FileManager.default.createDirectory(at: dir.appendingPathComponent("chapters"), withIntermediateDirectories: true)
        for n in 1 ... 3 { try Self.chapter(n).write(to: dir.appendingPathComponent("chapters/ch\(n).tex"), atomically: true, encoding: .utf8) }
        let main = dir.appendingPathComponent("main.tex")
        try "\\documentclass{report}\n\\begin{document}\n\\tableofcontents\n\\include{chapters/ch1}\n\\include{chapters/ch2}\n\\include{chapters/ch3}\n\\end{document}\n"
            .write(to: main, atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: main, dirty: .discard), .opened)
        model.engineV3Enabled = true
        let s = model.engineV3
        defer { s.stop() }
        try await waitUntil("the host") { s.phase == .ready || { if case .failed = s.phase { true } else { false } }() }
        guard s.phase == .ready else { throw XCTSkip("host did not start: \(s.phase)") }
        try await waitUntil("the whole document") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount > 4 && s.staleCount == 0 }
        let whole = s.pageCount
        XCTAssertTrue(s.hostOffersIncludeOnly)
        model.chrome.refresh(from: model)
        XCTAssertNil(model.focusChapter(for: "main.tex"), "the main file is not a chapter")
        let chapter = try XCTUnwrap(model.focusChapter(for: "chapters/ch2.tex"))
        XCTAssertEqual(chapter.name, "chapters/ch2")

        model.focusPreview(on: "chapters/ch2.tex")
        XCTAssertTrue(s.focus.isActive)
        try await waitUntil("the focused chapter") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount < whole && s.staleCount == 0 }
        let focused = s.pageCount
        XCTAssertGreaterThan(focused, 1)
        XCTAssertEqual(s.focus.seeds, 1)

        // Export while focused: the whole document.
        let dest = try tempDir("out").appendingPathComponent("book.pdf")
        let report = await export(model, to: dest)
        guard case .succeeded = report.state else { return XCTFail("export: \(report.state)") }
        XCTAssertEqual(PDFDocument(url: dest)?.pageCount, whole, "Export is the whole document")
        XCTAssertFalse(PDFDocument(url: dest)?.string?.contains("??") ?? true)
        XCTAssertTrue(s.focus.isActive, "the focus stays after an export")

        // Back to the focused preview after the export, then Show All.
        try await waitUntil("the focused preview again", timeout: 60) { !s.compiling && !s.exporting }
        model.showWholeDocument()
        XCTAssertFalse(s.focus.isActive)
        try await waitUntil("the whole document again") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == whole && s.staleCount == 0 }
    }
}
