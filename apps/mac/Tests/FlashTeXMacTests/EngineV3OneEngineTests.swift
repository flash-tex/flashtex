import Foundation
import XCTest
@testable import FlashTeXMac

/// One engine (the old-engine retirement plan's stage S6, #1236; app-parity
/// rows A2, B6, B7, E2): the app runs no old-engine helper, the engine's last
/// compile is the navigation baseline, and the capture project id is the
/// constant it always was. Runs with no host at all (`FLASHTEX_HOST=none`),
/// in a private v3 cache.
@MainActor
final class EngineV3OneEngineTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-one-engine-\(getpid())")
    /// Environment set for a test and put back after it (never just unset).
    private var env = EnvironmentOverride()

    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_HOST", "none")
    }

    override func tearDown() {
        env.restore()
        try? FileManager.default.removeItem(at: Self.cache)
    }

    /// A2: the app has one engine. No source file names an old-engine helper
    /// executable (the runtime-v1 worker, the render pipeline, the exact PDF
    /// writer, the preview controller), so none can be launched; ⌘B, Export
    /// and Print go to the engine.
    func testOneEngineNoOldEngineHelperIsLaunched() throws {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0 ..< 3 { root = root.deletingLastPathComponent() } // apps/mac
        let helpers = ["flashtex-compiler", "flashtex-render", "flashtex-pdf-exact", "flashtex-preview-controller"]
        let walker = try XCTUnwrap(FileManager.default.enumerator(at: root.appendingPathComponent("Sources"), includingPropertiesForKeys: nil))
        var checked = 0
        for case let url as URL in walker where url.pathExtension == "swift" {
            let text = try String(contentsOf: url, encoding: .utf8)
            checked += 1
            for h in helpers { XCTAssertFalse(text.contains("\"\(h)\""), "\(url.lastPathComponent) names \(h)") }
        }
        XCTAssertGreaterThan(checked, 100)

        let model = ShellModel()
        model.engineV3Enabled = true
        defer { model.engineV3.stop() }
        XCTAssertTrue(model.canCompile)
        XCTAssertEqual(model.exportPDFRefusal(), model.engineV3.exportRefusal(), "Export is the engine's")
        XCTAssertTrue(PrintController.documentHelp(model).contains("engine"), PrintController.documentHelp(model))
        XCTAssertTrue(model.displayedDiagnostics.isEmpty, "the fixture's runtime-v1 result is not shown")
    }

    /// B6 / B7: the engine's compile is made from these texts (at DONE): the
    /// baseline that the Problems panel's line labels and navigation rebase
    /// from. Turning the session off drops it.
    func testTheEnginesCompiledTextsAreTheNavigationBaseline() {
        let model = ShellModel()
        model.engineV3Enabled = true
        defer { model.engineV3.stop() }
        model.setEngineV3CompiledDocuments(["main.tex": "x"])
        XCTAssertEqual(model.compiledDocuments, ["main.tex": "x"])
        XCTAssertEqual(model.compiledText(for: "main.tex"), "x")
        model.engineV3Enabled = false
        XCTAssertTrue(model.compiledDocuments.isEmpty)
    }

    /// E2 (ruled the same on both routes, DESIGN.md §13 2026-10-21): the
    /// capture destination's project id is the constant "demo" it always
    /// was, so captures and pins already stored under it keep matching.
    func testTheCaptureProjectIdIsTheConstantItAlwaysWas() throws {
        let model = ShellModel()
        XCTAssertEqual(model.projectId, "demo")
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-one-engine-doc-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let main = dir.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\begin{document}\nHi.\n\\end{document}\n".write(to: main, atomically: true, encoding: .utf8)
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: main), .opened)
        XCTAssertEqual(model.projectId, "demo")
    }
}
