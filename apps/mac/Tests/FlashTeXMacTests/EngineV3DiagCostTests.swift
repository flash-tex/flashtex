import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// Main-thread cost per compile result (review lane REVIEW-APP, round 5):
/// the pane's error count was recomputed over every diagnostic so far at
/// each error DIAG, O(errors × diagnostics) on main for one run. A book
/// with thousands of recovered errors among its warnings (pdfTeX's 100-error
/// stop is per paragraph) held main for seconds as its DIAGs arrived.
@MainActor
final class EngineV3DiagCostTests: XCTestCase {
    private var env = EnvironmentOverride()

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-diag-cost-\(UUID().uuidString)").path)
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
    }

    override func tearDown() { env.restore() }

    private func diag(id: Int, seq: Int, error: Bool, line: Int) throws -> DL3Diag {
        let json = error
            ? #"{"id":\#(id),"seq":\#(seq),"severity":"error","code":"tex/undefined-control-sequence","origin":"tex","message":"Undefined control sequence","file":"./main.tex","line":\#(line),"col":2,"exact":true}"#
            : #"{"id":\#(id),"seq":\#(seq),"severity":"warning","code":"tex/overfull-hbox","origin":"tex","message":"Overfull \\hbox (1.0pt too wide) in paragraph at lines \#(line)--\#(line)","file":"./main.tex","line":\#(line),"col":0,"exact":true}"#
        return try DL3Diag.decode(Array(json.utf8))
    }

    /// 10,000 DIAGs of one run, one in five an error (recovered: best effort
    /// counts none), handled one by one as a drain applies them: the counts'
    /// work stays near linear in the diagnostics, and the counts at DONE are
    /// exact.
    func testThePanesCountsAreNotRecomputedOverEveryDiagnosticAtEachError() throws {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        model.engineV3Enabled = true
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        let n = 10_000
        var ds: [DL3Diag] = []
        for k in 0 ..< n { ds.append(try diag(id: 1, seq: k, error: k % 5 == 4, line: k + 1)) }
        s.handle(.started(.object(["id": .int(1)])))
        let visits0 = s.recountVisits
        let t0 = Date()
        for d in ds { s.handle(.diag(d)) }
        let ms = Date().timeIntervalSince(t0) * 1000
        let visits = s.recountVisits - visits0
        print(String(format: "EngineV3DiagCostTests: %d DIAGs handled in %.0f ms, %d diagnostics visited by the counts", n, ms, visits))
        // Before: every error recounted all so far, about n²/10 = 10,000,000 visits.
        XCTAssertLessThan(visits, 50 * n, String(format: "%d visits, %.0f ms", visits, ms))
        s.handle(.done(.object(["id": .int(1), "status": .string("ok"), "pages": .int(1)]), compileID: 1))
        XCTAssertEqual(s.errorCount, 0, "recovered errors are warnings in best effort")
        XCTAssertEqual(s.warningCount, n)
    }

    /// A few DIAGs (a document's usual handful): each error is counted at
    /// once, before the run ends.
    func testAFewErrorsAreCountedAsTheyArrive() throws {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n", named: "main.tex")
        model.engineV3Enabled = true
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        s.handle(.started(.object(["id": .int(1)])))
        let fatal = try DL3Diag.decode(Array(#"{"id":1,"seq":0,"severity":"error","code":"tex/emergency-stop","origin":"tex","message":"Emergency stop","file":"./main.tex","line":3,"col":0,"exact":true}"#.utf8))
        s.handle(.diag(try diag(id: 1, seq: 1, error: false, line: 2)))
        s.handle(.diag(fatal))
        XCTAssertEqual(s.errorCount, 1, "counted before DONE")
        XCTAssertNotNil(s.firstError)
    }
}
