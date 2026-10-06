import Foundation
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Lane ERROR-RECOVERY, Phase 1: how the engine-v3 preview behaves across the
/// mid-typing and common-error corpus (`docs/evidence/error-recovery-2026-10-04/`).
/// For every case, on a running session (the app's own `ShellModel` and
/// `EngineV3Session` over a real `flashtex-host`): the good document is on
/// screen; the error is typed (`updateActiveText`, as the editor does); what
/// the pane holds, the Problems rows and the DONE are recorded; then the error
/// is fixed and the recovery recorded. Writes `v3.json` into
/// `FLASHTEX_ERRREC_OUT`; skips unless that is set (it is evidence, not a gate).
@MainActor
final class ErrorRecoveryCorpusTests: XCTestCase {
    private var env = EnvironmentOverride()
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("errrec-tests-\(getpid())")
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    struct Corpus: Decodable {
        struct Edit: Decodable { var find: String; var replace: String }
        struct Case: Decodable { var id: String; var base: String; var edits: [Edit]; var what: String }
        var bases: [String: String]
        var cases: [Case]
    }

    static var corpusURL: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("docs/evidence/error-recovery-2026-10-04/corpus.json")
    }

    static func apply(_ text: String, _ edits: [Corpus.Edit]) -> String {
        var t = text
        for e in edits { if let r = t.range(of: e.find) { t.replaceSubrange(r, with: e.replace) } }
        return t
    }

    func waitUntil(_ what: String, timeout: TimeInterval = 180, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); throw EngineV3TestHost.HostNotReady(description: what) }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    /// Waits for the compile the edit caused to finish (a DONE that is not
    /// `cancelled`, nothing compiling) and for the session to stay settled.
    func settle(_ s: EngineV3Session, after done: Int) async throws {
        try await waitUntil("a DONE") { s.doneCount > done && !s.compiling && s.lastDone?["status"]?.string != "cancelled" }
        try await Task.sleep(nanoseconds: 400_000_000)
        try await waitUntil("settled") { !s.compiling }
    }

    func snapshot(_ s: EngineV3Session) -> [Int: String] {
        var h: [Int: String] = [:]
        for (i, p) in s.pages { h[i] = "\(p.page.hash)" }
        return h
    }

    func state(_ s: EngineV3Session, model: ShellModel, before: [Int: String], seconds: Double, text: String) -> [String: Any] {
        let now = snapshot(s)
        let d = s.lastDone
        let rows: [[String: Any]] = model.displayedDiagnostics.map { r in
            var o: [String: Any] = ["severity": r.severity == .error ? "error" : "warning", "message": r.message, "code": r.code ?? ""]
            if let src = r.source {
                let b = Array(text.utf8)
                if src.endByte <= b.count, src.startByte <= src.endByte {
                    o["marked"] = String(decoding: b[src.startByte ..< src.endByte], as: UTF8.self)
                    o["line"] = b[..<src.startByte].filter { $0 == 0x0A }.count + 1
                    let lineStart = (b[..<src.startByte].lastIndex(of: 0x0A).map { $0 + 1 }) ?? 0
                    o["col"] = src.startByte - lineStart + 1
                }
            }
            o["help"] = r.help?.message ?? ""
            o["fix"] = r.help?.replacement != nil
            o["notes"] = r.notes?.count ?? 0
            return o
        }
        let p0 = before.count
        return [
            "seconds": (seconds * 1000).rounded() / 1000,
            "status": d?["status"]?.string ?? "?",
            "mode": d?["mode"]?.string ?? "?",
            "cold_reason": d?["cold_reason"]?.string ?? "",
            "done_pages": d?["pages"]?.int ?? -1,
            "typeset_pages": d?["typeset_pages"]?.int ?? -1,
            "rerun_pages": d?["rerun_pages"]?.int ?? -1,
            "elapsed_ms": d?["elapsed_ms"]?.double ?? -1,
            "pane_pages": s.pageCount,
            "present": now.count,
            "stale": s.staleCount,
            "changed": now.keys.filter { before[$0] != nil && before[$0] != now[$0] }.sorted(),
            "lost": (0 ..< p0).filter { now[$0] == nil },
            "same_as_before": now == before,
            "errors": s.errorCount,
            "first_error": s.firstError ?? "",
            "status_note": s.statusNote,
            "result_failed": model.engineV3ResultStatus == .failed,
            "rows": rows,
        ]
    }

    func testCorpus() async throws {
        guard let out = ProcessInfo.processInfo.environment["FLASHTEX_ERRREC_OUT"], !out.isEmpty else {
            throw XCTSkip("set FLASHTEX_ERRREC_OUT=<dir> to characterise the error-recovery corpus")
        }
        try EngineV3TestHost.require()
        let corpus = try JSONDecoder().decode(Corpus.self, from: Data(contentsOf: Self.corpusURL))
        let only = ProcessInfo.processInfo.environment["FLASHTEX_ERRREC_ONLY"].map { Set($0.split(separator: ",").map(String.init)) }
        var results: [String: Any] = [:]
        for baseName in ["short", "long"] {
            let cases = corpus.cases.filter { $0.base == baseName && (only == nil || only!.contains($0.id)) }
            guard !cases.isEmpty, let base = corpus.bases[baseName] else { continue }
            let dir = FileManager.default.temporaryDirectory.appendingPathComponent("errrec-\(baseName)-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: dir) }
            let file = dir.appendingPathComponent("main.tex")
            try base.write(to: file, atomically: true, encoding: .utf8)
            let model = ShellModel()
            XCTAssertEqual(model.openTex(at: file, dirty: .discard), .opened)
            model.engineV3Enabled = true
            model.engineV3.start(model: model)
            defer { model.engineV3.stop() }
            let s = model.engineV3
            try await EngineV3TestHost.awaitReady(s)
            try await waitUntil("the first compile") { s.doneCount > 0 && !s.compiling && s.staleCount == 0 && s.pageCount > 0 }
            try await Task.sleep(nanoseconds: 500_000_000)
            for c in cases {
                // Start each case from the good document, fully on screen.
                if model.activeText != base {
                    let n = s.doneCount
                    model.updateActiveText(base)
                    try await settle(s, after: n)
                }
                let before = snapshot(s)
                let bad = Self.apply(base, c.edits)
                var n = s.doneCount
                var t = Date()
                model.updateActiveText(bad)
                try await settle(s, after: n)
                let err = state(s, model: model, before: before, seconds: Date().timeIntervalSince(t), text: bad)
                n = s.doneCount
                t = Date()
                model.updateActiveText(base)
                try await settle(s, after: n)
                let fix = state(s, model: model, before: before, seconds: Date().timeIntervalSince(t), text: base)
                results[c.id] = ["what": c.what, "good_pages": before.count, "error": err, "fix": fix]
                print("errrec \(c.id): error \(err["status"]!) pane \(err["pane_pages"]!)/\(before.count) stale \(err["stale"]!) lost \((err["lost"] as! [Int]).count) · fix \(fix["mode"]!) \(fix["elapsed_ms"]!) ms typeset \(fix["typeset_pages"]!) same \(fix["same_as_before"]!)")
            }
        }
        let url = URL(fileURLWithPath: out).appendingPathComponent("v3.json")
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try JSONSerialization.data(withJSONObject: results, options: [.prettyPrinted, .sortedKeys]).write(to: url)
    }
}
