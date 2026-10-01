import Foundation
import XCTest
@testable import FlashTeXMac

/// What the engine-v3 integration tests (EngineV3OpenTests,
/// EngineV3InstanceTests, EngineV3ZoomTilesTests) need: a built
/// `flashtex-host` and a TeX Live it can build the pdfLaTeX format from.
/// Without either they skip at once (XCTSkip). With both, a host that fails
/// to start or is not ready in time FAILS the test (a host-start regression
/// is never masked as a skip). The Mac CI job builds no host today, so there
/// these tests skip; running them in CI needs a workflow step (Commander-owned).
@MainActor
enum EngineV3TestHost {
    /// Skips unless a host is built and a TeX Live answers `kpsewhich pdflatex.ini`.
    static func require() throws {
        guard EngineV3.locateHost() != nil else {
            throw XCTSkip("no flashtex-host built (cargo build --release -p flashtex-engine --bin flashtex-host)")
        }
        guard texLive != nil else { throw XCTSkip("no TeX Live found (kpsewhich pdflatex.ini)") }
    }

    /// The `pdflatex.ini` a TeX Live resolves, or nil when none is installed.
    static let texLive: String? = {
        var dirs = (ProcessInfo.processInfo.environment["PATH"] ?? "").split(separator: ":").map(String.init)
        dirs.append("/Library/TeX/texbin")
        if let years = try? FileManager.default.contentsOfDirectory(atPath: "/usr/local/texlive") {
            for y in years.sorted().reversed() {
                let bin = "/usr/local/texlive/\(y)/bin"
                for arch in (try? FileManager.default.contentsOfDirectory(atPath: bin)) ?? [] { dirs.append("\(bin)/\(arch)") }
            }
        }
        for d in dirs {
            let k = URL(fileURLWithPath: d).appendingPathComponent("kpsewhich")
            guard FileManager.default.isExecutableFile(atPath: k.path) else { continue }
            let p = Process()
            p.executableURL = k
            p.arguments = ["pdflatex.ini"]
            let out = Pipe()
            p.standardOutput = out
            p.standardError = FileHandle.nullDevice
            guard (try? p.run()) != nil else { continue }
            p.waitUntilExit()
            let path = String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
            if p.terminationStatus == 0, !path.isEmpty { return path }
        }
        return nil
    }()

    struct HostNotReady: Error, CustomStringConvertible { let description: String }

    /// Waits until `session`'s host is ready; fails the test (and throws)
    /// when it fails to start or is not ready within `timeout`.
    static func awaitReady(_ session: EngineV3Session, timeout: TimeInterval = 120, file: StaticString = #filePath, line: UInt = #line) async throws {
        let start = Date()
        while true {
            if session.phase == .ready { return }
            if case .failed(let why) = session.phase {
                XCTFail("the engine-v3 host did not start: \(why)", file: file, line: line)
                throw HostNotReady(description: why)
            }
            if Date().timeIntervalSince(start) > timeout {
                XCTFail("the engine-v3 host was not ready after \(Int(timeout)) s: \(session.phase)", file: file, line: line)
                throw HostNotReady(description: "timeout")
            }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}
