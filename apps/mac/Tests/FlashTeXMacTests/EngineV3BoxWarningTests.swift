import Foundation
import XCTest
@testable import FlashTeXMac

/// App-parity row B3 (box warnings) against a real host: an overfull \hbox
/// the engine reports reaches the Problems panel as a warning, on the line
/// of the box. (EngineV3DiagPresentTests covers the presentation from
/// decoded DIAG records; this is the whole route, host to panel.)
/// Needs a built `flashtex-host` and a TeX Live (EngineV3TestHost).
@MainActor
final class EngineV3BoxWarningTests: XCTestCase {
    /// Environment set for a test and put back after it (never just unset).
    private var env = EnvironmentOverride()
    /// A private cache root: never the one a running app uses.
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-box-\(getpid())")
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    /// 1-based line of a byte offset.
    static func line(ofByte byte: Int, in text: String) -> Int {
        1 + Array(text.utf8).prefix(byte).filter { $0 == UInt8(ascii: "\n") }.count
    }

    func testAnOverfullHboxFromTheHostIsAWarningOnItsLine() async throws {
        try EngineV3TestHost.require()
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-box-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let file = dir.appendingPathComponent("box.tex")
        // Line 5: a box far narrower than its text (TeX: "Overfull \hbox
        // (...pt too wide) detected at line 5").
        let text = "\\documentclass{article}\n\\begin{document}\nA fine first line.\n\n\\hbox to 1pt{wide text here}\n\nA fine last line.\n\\end{document}\n"
        try text.write(to: file, atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
        model.engineV3Enabled = true
        model.engineV3.start(model: model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile and its box warning") {
            !s.compiling && !s.statusNote.isEmpty && model.engineV3Diagnostics.contains { Self.isOverfullHbox($0.message) }
        }
        XCTAssertNil(s.firstError, "a box warning stops nothing")
        XCTAssertEqual(s.pageCount, 1)

        let boxes = model.engineV3Diagnostics.filter { Self.isOverfullHbox($0.message) }
        XCTAssertEqual(boxes.count, 1, model.engineV3Diagnostics.map(\.message).description)
        let d = try XCTUnwrap(boxes.first)
        XCTAssertEqual(d.severity, .warning)
        if s.hostOffersDiagV1 { XCTAssertEqual(d.code, "tex/overfull-hbox") }
        // What the Problems panel lists under v3 is this row.
        XCTAssertTrue(model.displayedDiagnostics.contains(d))

        // On the box's line, wherever the copy lives: the host names the
        // box's file by its real path (/private/var/... under a temporary
        // directory), the app knows the copy as /var/...; both are the copy.
        let src = try XCTUnwrap(d.source, "the box row has a source range: \(d.message)")
        XCTAssertEqual(src.path, "box.tex")
        XCTAssertEqual(Self.line(ofByte: src.startByte, in: text), 5, "the row is on the box's line")
        XCTAssertFalse(d.message.contains("box.tex:"), "placed in the editor, not named in the message: \(d.message)")
    }

    /// The same without a host: a box DIAG naming the copy's file by its
    /// real path, while the session knows the copy through a symlink, is
    /// placed on its line (and the standardized spelling still is).
    func testABoxNamedByTheCopysRealPathIsPlaced() throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-box-link-\(UUID().uuidString)")
        let real = base.appendingPathComponent("real/src"), link = base.appendingPathComponent("link")
        try FileManager.default.createDirectory(at: real, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: base) }
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: base.appendingPathComponent("real"))
        let text = "\\documentclass{article}\n\\begin{document}\nA fine first line.\n\n\\hbox to 1pt{wide text here}\n\n\\end{document}\n"
        try text.write(to: real.appendingPathComponent("box.tex"), atomically: true, encoding: .utf8)
        let copy = link.appendingPathComponent("src") // how the session knows it
        let realPath = try XCTUnwrap(realpath(copy.path, nil).map { p in defer { free(p) }; return String(cString: p) })
        XCTAssertNotEqual(realPath, copy.standardizedFileURL.path)
        let model = ShellModel()
        model.replaceProject(entryText: text, named: "box.tex")
        for named in [realPath, copy.standardizedFileURL.path] {
            let json = #"{"id":1,"seq":0,"exact":true,"severity":"warning","code":"tex/overfull-hbox","origin":"tex","message":"Overfull \\hbox (60.0pt too wide) detected at line 5","file":"\#(named)/box.tex","line":5,"col":13}"#
            let d = try DL3Diag.decode(Array(json.utf8))
            let rows = EngineV3Session.problems(diags: [d], model: model, projectRoot: copy, texts: ["box.tex": text])
            let row = try XCTUnwrap(rows.first, named)
            XCTAssertEqual(row.severity, .warning)
            let src = try XCTUnwrap(row.source, "\(named): \(row.message)")
            XCTAssertEqual(src.path, "box.tex")
            XCTAssertEqual(Self.line(ofByte: src.startByte, in: text), 5, named)
        }
    }

    /// "Overfull \hbox ...", or the same after a "file:line:col: " place.
    static func isOverfullHbox(_ message: String) -> Bool {
        message.hasPrefix("Overfull \\hbox") || message.contains(": Overfull \\hbox")
    }
}
