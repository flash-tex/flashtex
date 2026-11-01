import AppKit
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// LIVE acceptance of the capture-conversion seam: one capture conversion
/// through the real `flashtex-bridge` with the selected provider's key from
/// the Mac credential adapter. Runs only when `FLASHTEX_CONVERSION_LIVE=1`
/// AND a key resolves for the selected provider; otherwise it is skipped, so
/// `swift test` never contacts a provider. `FLASHTEX_CONVERSION_EVIDENCE_DIR`
/// receives `conversion.json` (ids, model, sizes, timings — never a key, a
/// prompt body or a reply body).
@MainActor
final class CaptureConversionLiveTests: XCTestCase {
    static var live: Bool {
        let env = ProcessInfo.processInfo.environment
        return env["FLASHTEX_CONVERSION_LIVE"] == "1" || env["FLASHTEX_GROK_LIVE"] == "1"
    }
    static var evidenceDirectory: URL? {
        let env = ProcessInfo.processInfo.environment
        return (env["FLASHTEX_CONVERSION_EVIDENCE_DIR"] ?? env["FLASHTEX_GROK_EVIDENCE_DIR"]).map { URL(fileURLWithPath: $0, isDirectory: true) }
    }

    private func requireLive() throws -> (ConversionProvider, ConversionCredential.Resolution) {
        guard Self.live else { throw XCTSkip("FLASHTEX_CONVERSION_LIVE is not 1: no live provider call") }
        let provider = ConversionCredential.provider()
        guard provider != .none else { throw XCTSkip("conversion provider is none: no live provider call") }
        guard let credential = ConversionCredential.resolve(for: provider) else {
            throw XCTSkip("no \(provider.displayName) key in the Keychain or the environment: no live provider call")
        }
        return (provider, credential)
    }

    private func record(_ name: String, _ object: [String: Any]) throws {
        guard let dir = Self.evidenceDirectory else { return }
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        var object = object
        object["recorded_utc"] = ISO8601DateFormatter().string(from: Date())
        let data = try JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: dir.appendingPathComponent(name))
    }

    private func waitUntil(_ what: String, timeout: TimeInterval, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    /// The deterministic "photo-simulated" capture: handwriting-style text of a
    /// small identity rendered to a PNG (no fixture image exists in the repo;
    /// the 1×1 protocol fixture is rejected by providers).
    static func renderedCapture() throws -> RuntimeV1.CaptureImage {
        let size = NSSize(width: 900, height: 260)
        let image = NSImage(size: size)
        image.lockFocus()
        NSColor.white.setFill()
        NSRect(origin: .zero, size: size).fill()
        let font = NSFont(name: "Bradley Hand", size: 64) ?? NSFont(name: "Noteworthy", size: 64) ?? NSFont.systemFont(ofSize: 64)
        let text = NSAttributedString(string: "∫₀¹ x² dx = 1/3   and   α + β ≤ γ", attributes: [.font: font, .foregroundColor: NSColor.black])
        text.draw(at: NSPoint(x: 40, y: 90))
        image.unlockFocus()
        guard let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
              let png = rep.representation(using: .png, properties: [:]) else { throw XCTSkip("could not render the capture image") }
        return RuntimeV1.CaptureImage(mimeType: "image/png", dataBase64: png.base64EncodedString())
    }

    /// Distinct matches of `pattern` (or its capture `group`) in `text`, sorted.
    static func matches(_ pattern: String, in text: String, group: Int = 0) -> [String] {
        guard let re = try? NSRegularExpression(pattern: pattern) else { return [] }
        let ns = text as NSString
        let found = re.matches(in: text, range: NSRange(location: 0, length: ns.length)).map { ns.substring(with: $0.range(at: group)) }
        return Array(Set(found)).sorted()
    }

    /// submit (real bridge) → capture_convert (live provider, with the
    /// engine's `supported_features` policy) → proposal queued for the review
    /// sheet, nothing inserted. (The old engine's shadow compile of the
    /// proposal retired with that engine, #1236 S6.)
    func testLiveCaptureConversionThroughTheBridge() async throws {
        let (provider, credential) = try requireLive()
        guard let bridge = BridgeClient.locateBridge() else { throw XCTSkip("no flashtex-bridge (build crates/bridge or set FLASHTEX_BRIDGE)") }
        let launch = ShellModel.bridgeConversionLaunch()
        XCTAssertEqual(launch.provider, provider)
        XCTAssertEqual(launch.credential, credential)
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let model = ShellModel()
        model.autoCompile = false
        let attached = await model.attachBridgeAndWait(executable: bridge, storeDirectory: store, provider: launch.provider,
                                                       environment: launch.environment, discoverLedger: false)
        XCTAssertTrue(attached, model.captureNote ?? model.bridgeStatus)
        XCTAssertEqual(model.bridge?.conversionEnabled, true)
        let document = "\\documentclass{article}\n\\begin{document}\nThe identity \n\\end{document}\n"
        model.updateActiveText(document)
        model.caretUTF16 = 53
        model.pinAnchorAtCaret()
        try await waitUntil("pin", timeout: 10) { model.bridgeDestination != nil }
        let image = try Self.renderedCapture()
        let captureId = "conversion-live-\(UUID().uuidString.lowercased().prefix(8))"
        let received = await model.submitCapture(image: image, captureId: captureId, instructions: CaptureFeatures.defaultInstructions)
        XCTAssertNotNil(received, model.captureNote ?? "")
        let features = CaptureFeatures.supportedFeatures()
        let started = Date()
        let proposal = await model.convertCapture(captureId: captureId, supportedFeatures: features)
        let seconds = Date().timeIntervalSince(started)
        let keyVariable = provider.bridgeKeyVariable ?? ""
        var evidence: [String: Any] = [
            "bridge": bridge.path, "provider": provider.rawValue,
            "model": launch.environment[ConversionCredential.bridgeModelVariable] ?? "?", "key_source": credential.source.rawValue,
            "capture_id": captureId, "seconds": seconds, "bridge_flag": provider.bridgeFlag ?? "",
            "image_bytes": Data(base64Encoded: image.dataBase64)?.count ?? 0,
            "supported_features_count": features.count,
            "bridge_env_has_key": launch.environment[keyVariable] != nil,
            "bridge_env_sensitive_names_other_than_key": launch.environment.keys.filter { $0 != keyVariable && ConversionCredential.isSensitiveVariable($0) },
        ]
        if let proposal {
            evidence["outcome"] = "proposal"
            evidence["latex_bytes"] = proposal.latex.utf8.count
            evidence["ambiguities"] = proposal.ambiguities.count
            evidence["required_dependencies"] = proposal.requiredDependencies
            evidence["context_revision"] = proposal.contextRevision ?? -1
            evidence["latex_commands"] = Self.matches(#"\\[A-Za-z]+"#, in: proposal.latex)
            evidence["latex_environments"] = Self.matches(#"\\begin\{([A-Za-z*]+)\}"#, in: proposal.latex, group: 1)
        } else {
            evidence["outcome"] = "failed"
            evidence["failure"] = model.captureNote ?? model.bridgeStatus
        }
        try record("conversion.json", evidence)
        model.detachBridge()
        XCTAssertNotNil(proposal, model.captureNote ?? "")
        XCTAssertEqual(model.proposals.count, proposal == nil ? 0 : 1, "queued for review; never inserted")
        XCTAssertEqual(model.activeText, document)
    }
}
