import Foundation
import PDFKit
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// Bibliography and index under the engine-v3 preview (lane P5-APP-PARITY,
/// gaps A10, A11): the app speaks protocol 3.2 and sends `external_tools`
/// `auto` for a trusted project and `off` for an untrusted one (#1332's
/// decision, owner 9A), shows `TOOL` progress, and an export waits for the
/// tools to settle. The end-to-end cases need a built `flashtex-host` and
/// TeX Live's bibtex (skipped otherwise) and use a private v3 cache.
@MainActor
final class EngineV3ToolsTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-tools-\(getpid())")
    /// Environment set for a test and put back after it (never just unset).
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

    private func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    // MARK: protocol

    func testRequestCarriesExternalToolsAndTheClientSays32() throws {
        XCTAssertEqual(DL3.versionMinor, 2)
        var req = DL3CompileRequest(id: 7, root: "/r", main: "main.tex")
        XCTAssertNil(req.json["external_tools"], "nil leaves the host's default")
        req.externalTools = "auto"
        XCTAssertEqual(req.json["external_tools"]?.string, "auto")
        let body = Array(#"{"id": 7, "event": "settled", "ran": true, "rounds": 1}"#.utf8)
        guard case .tool(let j) = try DL3Event.decode(kind: DL3.Kind.tool, body: body) else { return XCTFail("TOOL is not decoded") }
        XCTAssertEqual(j["event"]?.string, "settled")
        XCTAssertEqual(DL3.Kind.name(0x4C), "tool")
    }

    // MARK: with a host

    static let source = """
    \\documentclass{article}
    \\begin{document}
    As \\cite{knuth84} shows.
    \\bibliographystyle{plain}
    \\bibliography{refs}
    \\end{document}

    """
    static let bib = """
    @book{knuth84, author = {Donald E. Knuth}, title = {The {\\TeX}book}, publisher = {Addison-Wesley}, year = {1984}}

    """

    private func project(quarantined: Bool) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-tools-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        dirs.append(dir)
        let main = dir.appendingPathComponent("paper.tex")
        try Self.source.write(to: main, atomically: true, encoding: .utf8)
        try Self.bib.write(to: dir.appendingPathComponent("refs.bib"), atomically: true, encoding: .utf8)
        if quarantined { for url in [dir, main] { quarantine(url) } }
        return main
    }

    /// What a browser download leaves (#1332's trust treats it as from elsewhere).
    private func quarantine(_ url: URL) {
        let value = "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Safari;\(UUID().uuidString)"
        XCTAssertEqual(setxattr(url.path, EngineV3Trust.quarantineAttribute, value, value.utf8.count, 0, 0), 0)
    }

    private func started(_ main: URL) async throws -> ShellModel {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built (cargo build --release -p flashtex-engine --bin flashtex-host)") }
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: main, dirty: .discard), .opened)
        model.engineV3Enabled = true
        let s = model.engineV3
        try await waitUntil("the host") { s.phase == .ready || { if case .failed = s.phase { true } else { false } }() }
        guard s.phase == .ready else { throw XCTSkip("host did not start: \(s.phase)") }
        try await waitUntil("the first compile") { !s.statusNote.isEmpty && !s.compiling }
        return model
    }

    private func exportedText(_ model: ShellModel) async throws -> String {
        let dest = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-tools-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: dest) }
        let report: ExportSession.Report = await withCheckedContinuation { c in
            model.exportPDFEngineV3(to: .recordingCurrentDisk(dest)) { c.resume(returning: $0) }
        }
        guard case .succeeded = report.state else { XCTFail("export: \(report.state)"); return "" }
        // FLASHTEX_V3_EXPORT_KEEP=<dir>: keep the PDF and its sources for a P-T2
        // comparison with latexmk outside the test (tools/parity/tiers.py).
        if let keep = ProcessInfo.processInfo.environment["FLASHTEX_V3_EXPORT_KEEP"], !keep.isEmpty, model.engineV3.projectTrusted {
            let k = URL(fileURLWithPath: keep, isDirectory: true)
            try? FileManager.default.createDirectory(at: k, withIntermediateDirectories: true)
            try? FileManager.default.removeItem(at: k.appendingPathComponent("paper.pdf"))
            try FileManager.default.copyItem(at: dest, to: k.appendingPathComponent("paper.pdf"))
            try Self.source.write(to: k.appendingPathComponent("paper.tex"), atomically: true, encoding: .utf8)
            try Self.bib.write(to: k.appendingPathComponent("refs.bib"), atomically: true, encoding: .utf8)
        }
        return PDFDocument(url: dest)?.string ?? ""
    }

    func testTrustedProjectRunsBibtexAndTheCitationResolves() async throws {
        let model = try await started(try project(quarantined: false))
        defer { model.engineV3.stop() }
        let s = model.engineV3
        XCTAssertTrue(s.projectTrusted)
        try await waitUntil("the tools to settle") { s.toolsSettled && !s.compiling }
        XCTAssertNil(s.toolNote, "bibtex ran cleanly: \(s.toolNote ?? "")")
        let text = try await exportedText(model)
        XCTAssertTrue(text.contains("[1]"), "the citation is resolved: \(text)")
        XCTAssertTrue(text.contains("Knuth"), "the bibliography is typeset: \(text)")
    }

    func testUntrustedProjectRunsNoToolAndSaysWhy() async throws {
        let model = try await started(try project(quarantined: true))
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("trust decided") { !s.compiling && !s.projectTrusted }
        try await waitUntil("the skip note") { s.toolNote?.contains("not run") == true }
        XCTAssertTrue(s.toolNote?.contains("bibtex") == true, s.toolNote ?? "")
        let text = try await exportedText(model)
        XCTAssertTrue(text.contains("[?]"), "no bibtex for an untrusted project: \(text)")
    }

    /// Mid-session trust (review of #1344): an untrusted project's Trust
    /// button lets the tools run on its next compile.
    func testTrustingTheProjectMidSessionRunsTheTools() async throws {
        let model = try await started(try project(quarantined: true))
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("untrusted") { !s.compiling && !s.projectTrusted }
        try await waitUntil("the skip note") { s.toolNote?.contains("not run") == true }
        s.trustProject() // the banner's "Trust This Project"
        try await waitUntil("trusted") { s.projectTrusted }
        try await waitUntil("the tools to settle") { s.toolsSettled && !s.compiling && s.toolNote == nil }
        let text = try await exportedText(model)
        XCTAssertTrue(text.contains("[1]"), "bibtex ran after trusting: \(text)")
    }

    /// The project's file set changing on disk (review of #1344): a
    /// downloaded file appearing in a trusted project's folder makes the next
    /// compile, an edit, decide trust again, and the tools stop.
    func testAQuarantinedFileAppearingInTheProjectIsCheckedBeforeTheNextCompile() async throws {
        let main = try project(quarantined: false)
        let model = try await started(main)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("the tools to settle") { s.toolsSettled && !s.compiling }
        XCTAssertTrue(s.projectTrusted)
        let before = s.projectFileSetChanges
        let extra = main.deletingLastPathComponent().appendingPathComponent("downloaded.bib")
        try Self.bib.write(to: extra, atomically: true, encoding: .utf8)
        quarantine(extra)
        try await waitUntil("the file-set change", timeout: 10) { s.projectFileSetChanges > before }
        model.updateActiveText(Self.source.replacingOccurrences(of: "As ", with: "As shown, "))
        try await waitUntil("trust decided again") { !s.projectTrusted && !s.compiling }
    }
}
