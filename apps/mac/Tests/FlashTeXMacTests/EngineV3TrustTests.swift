import Darwin
import Foundation
import XCTest
@testable import FlashTeXMac

/// Project trust (owner decision 9A): a quarantined project compiles with
/// shell escape off until trusted; trusting it is recorded per project and
/// gives restricted `\write18`, as pdflatex's default.
final class EngineV3TrustTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-trust-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE"); try? FileManager.default.removeItem(at: Self.cache) }

    func project(_ tex: String) throws -> (dir: URL, file: URL) {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-trust-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent("paper.tex")
        try tex.write(to: file, atomically: true, encoding: .utf8)
        return (dir, file)
    }

    /// What Safari writes on a download (flags;time;agent;UUID).
    func quarantine(_ url: URL) {
        let value = "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Safari;\(UUID().uuidString)"
        XCTAssertEqual(setxattr(url.path, EngineV3Trust.quarantineAttribute, value, value.utf8.count, 0, 0), 0)
    }

    func testQuarantineAndRecord() throws {
        let defaults = try XCTUnwrap(UserDefaults(suiteName: "engine-v3-trust-\(UUID().uuidString)"))
        let (dir, file) = try project("x")
        defer { try? FileManager.default.removeItem(at: dir) }
        XCTAssertTrue(EngineV3Trust.isTrusted(root: dir, main: file, defaults: defaults), "made here: trusted")
        XCTAssertTrue(EngineV3Trust.isTrusted(root: nil, main: nil, defaults: defaults), "untitled: nothing to run")
        quarantine(file)
        XCTAssertTrue(EngineV3Trust.isQuarantined(file))
        XCTAssertFalse(EngineV3Trust.isTrusted(root: dir, main: file, defaults: defaults))
        XCTAssertEqual(EngineV3Trust.shellEscape(trusted: false), "off")
        EngineV3Trust.record(dir, defaults: defaults)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: dir, main: file, defaults: defaults))
        XCTAssertEqual(EngineV3Trust.shellEscape(trusted: true), "restricted")
        // The record is per project: another quarantined folder is not trusted.
        let (other, otherFile) = try project("y")
        defer { try? FileManager.default.removeItem(at: other) }
        quarantine(other)
        XCTAssertFalse(EngineV3Trust.isTrusted(root: other, main: otherFile, defaults: defaults))
        // Stored in the defaults, keyed by the canonical folder path.
        XCTAssertEqual(defaults.stringArray(forKey: EngineV3Trust.recordKey), [EngineV3Trust.key(dir)])
    }

    /// End to end through the host: `\pdfshellescape` is 0 with shell escape
    /// off and 2 when restricted; the document has a second page only when 2.
    @MainActor
    func testQuarantinedProjectCompilesWithShellEscapeOffUntilTrusted() async throws {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built") }
        let (dir, file) = try project("\\documentclass{article}\n\\begin{document}\nOne.\n\\ifnum\\pdfshellescape=2 \\newpage Two.\\fi\n\\end{document}\n")
        defer { try? FileManager.default.removeItem(at: dir) }
        quarantine(file)
        let key = EngineV3Trust.key(dir)
        defer {
            let list = (UserDefaults.standard.stringArray(forKey: EngineV3Trust.recordKey) ?? []).filter { $0 != key }
            UserDefaults.standard.set(list, forKey: EngineV3Trust.recordKey)
        }
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

        m.engineV3.trustProject()
        XCTAssertTrue(m.engineV3.projectTrusted)
        try await wait { m.engineV3.statusNote.hasPrefix("ok") && m.engineV3.pageCount == 2 }
        XCTAssertEqual(m.engineV3.pageCount, 2, "trusted: restricted, \\pdfshellescape = 2")

        // Persisted per project: the next open of it is trusted.
        let again = ShellModel()
        XCTAssertEqual(again.openTex(at: file, dirty: .discard), .opened)
        XCTAssertTrue(EngineV3Trust.isTrusted(root: again.project.projectRoot, main: file))
    }
}
