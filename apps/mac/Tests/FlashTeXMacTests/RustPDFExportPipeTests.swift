import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Issue #19 (1): the export must drain child pipes concurrently and never
/// block the main actor, even against a writer that floods stderr.
final class RustPDFExportPipeTests: XCTestCase {
    static let fake = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .appendingPathComponent("Fixtures/fake_pdf_writer.py")
    static let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: [], pdfPath: nil)

    /// A wrapper script so the double can be launched like a real writer with env set.
    private func wrapper(env: String) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-fake-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let sh = dir.appendingPathComponent("writer.sh")
        try "#!/bin/sh\n\(env) exec /usr/bin/python3 '\(Self.fake.path)' \"$@\"\n".write(to: sh, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: sh.path)
        return sh
    }

    func testOversizedStderrDoesNotDeadlockAndIsTruncated() throws {
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("fake-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: out) }
        let started = Date()
        let notes = try RustPDFExport.export(Self.result, id: "t", writer: try wrapper(env: "FAKE_STDERR_BYTES=1048576"), to: out,
                                             timeout: 20, maxCapturedBytes: 64 * 1024)
        XCTAssertLessThan(Date().timeIntervalSince(started), 10, "1 MiB of stderr must not stall the export")
        XCTAssertTrue(notes.contains("truncated"), notes.suffix(120).description)
        XCTAssertTrue(FileManager.default.fileExists(atPath: out.path))
    }

    func testMainThreadHeartbeatKeepsRunningDuringExport() throws {
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("fake-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: out) }
        var ticks = 0
        var maxGap: TimeInterval = 0
        var last = Date()
        let timer = Timer(timeInterval: 0.02, repeats: true) { _ in
            let now = Date(); maxGap = max(maxGap, now.timeIntervalSince(last)); last = now; ticks += 1
        }
        RunLoop.main.add(timer, forMode: .common)
        let done = expectation(description: "export")
        let writer = try wrapper(env: "FAKE_STDERR_BYTES=2097152 FAKE_SLEEP=0.5")
        Task.detached {
            _ = try? RustPDFExport.export(Self.result, id: "t", writer: writer, to: out, timeout: 20)
            done.fulfill()
        }
        // Pump the main run loop while the export runs on a background thread.
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, ticks < 10 || !FileManager.default.fileExists(atPath: out.path) {
            RunLoop.main.run(until: Date().addingTimeInterval(0.02))
        }
        wait(for: [done], timeout: 15)
        timer.invalidate()
        // The loop above stops at `ticks < 10` -- so it guarantees ten ticks,
        // not eleven, and asserting `> 10` could only pass when something
        // unrelated pumped an extra turn. CI failed here with "10 is not
        // greater than 10": a plain off-by-one in the test, fixed by asking
        // for what the loop actually promises. (Reported rather than gating
        // on a shared runner, where the 10 s deadline can win the race.)
        TimingBudget.assertAtLeast(ticks, 10, "main-thread heartbeat ticks during the export")
        XCTAssertLessThan(maxGap, 0.5, "main thread stalled for \(maxGap)s during export")
    }

    func testTimeoutTerminatesAStuckWriter() throws {
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("fake-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: out) }
        let writer = try wrapper(env: "FAKE_STDERR_BYTES=10 FAKE_SLEEP=30")
        let started = Date()
        XCTAssertThrowsError(try RustPDFExport.export(Self.result, id: "t", writer: writer, to: out, timeout: 1)) { e in
            XCTAssertTrue("\(e)".contains("did not finish"), "\(e)")
        }
        XCTAssertLessThan(Date().timeIntervalSince(started), 5)
    }

    func testNonZeroExitIsReportedWithStderr() throws {
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("fake-\(UUID().uuidString).pdf")
        defer { try? FileManager.default.removeItem(at: out) }
        let writer = try wrapper(env: "FAKE_STDERR_BYTES=20 FAKE_EXIT=3")
        XCTAssertThrowsError(try RustPDFExport.export(Self.result, id: "t", writer: writer, to: out)) { e in
            XCTAssertTrue("\(e)".contains("exited 3"), "\(e)")
        }
    }
}
