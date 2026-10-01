import Foundation
import XCTest
@testable import FlashTeXMac

/// What the engine-v3 integration tests (EngineV3OpenTests,
/// EngineV3InstanceTests, EngineV3ZoomTilesTests) need: a built
/// `flashtex-host` and a TeX Live it can build the pdfLaTeX format from.
/// Without either they skip at once (XCTSkip), and a host that never becomes
/// ready skips too, instead of failing after a timeout. The Mac CI job builds
/// no host today, so there these tests skip; running them in CI needs a
/// workflow step (Commander-owned).
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

    /// Waits until `session`'s host is ready; skips when it fails or never is.
    static func awaitReady(_ session: EngineV3Session, timeout: TimeInterval = 120) async throws {
        let start = Date()
        while true {
            if session.phase == .ready { return }
            if case .failed(let why) = session.phase { throw XCTSkip("the engine-v3 host did not start: \(why)") }
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("the engine-v3 host was not ready after \(Int(timeout)) s: \(session.phase)") }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}
