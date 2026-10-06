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
            !s.compiling && !s.statusNote.isEmpty && model.engineV3Diagnostics.contains { $0.message.hasPrefix("Overfull \\hbox") }
        }
        XCTAssertNil(s.firstError, "a box warning stops nothing")
        XCTAssertEqual(s.pageCount, 1)

        let boxes = model.engineV3Diagnostics.filter { $0.message.hasPrefix("Overfull \\hbox") }
        XCTAssertEqual(boxes.count, 1, model.engineV3Diagnostics.map(\.message).description)
        let d = try XCTUnwrap(boxes.first)
        XCTAssertEqual(d.severity, .warning)
        if s.hostOffersDiagV1 { XCTAssertEqual(d.code, "tex/overfull-hbox") }
        let src = try XCTUnwrap(d.source, d.message)
        XCTAssertEqual(src.path, "box.tex")
        XCTAssertEqual(Self.line(ofByte: src.startByte, in: text), 5, "the row is on the box's line")
        // What the Problems panel lists under v3 is this row.
        XCTAssertTrue(model.displayedDiagnostics.contains(d))
    }
}
