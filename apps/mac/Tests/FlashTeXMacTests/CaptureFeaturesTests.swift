import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `supported_features` sent with every `capture_convert` (CaptureFeatures.swift).
@MainActor
final class CaptureFeaturesTests: XCTestCase {
    func testFeatureListFitsTheBridgeLimits() {
        let features = CaptureFeatures.supportedFeatures()
        XCTAssertFalse(features.isEmpty)
        XCTAssertLessThanOrEqual(features.count, 64, "bridge context.rs: at most 64 entries")
        for f in features {
            XCTAssertLessThanOrEqual(f.utf8.count, 128, f)
            XCTAssertTrue(f.hasPrefix("POLICY: "), f)
        }
        XCTAssertTrue(CaptureFeatures.defaultInstructions.contains("report anything else in ambiguities"))
    }

    /// One list (the retirement plan's §1.5, S6): the Mac's is the bridge's
    /// new-engine list (`NEW_ENGINE_FEATURES` in the MIT crate
    /// `crates/bridge/src/features.rs`), byte for byte, read from the source.
    func testTheListIsTheBridgesNewEngineList() throws {
        let url = BridgeClientTests.repoRoot.appendingPathComponent("crates/bridge/src/features.rs")
        let source = try String(contentsOf: url, encoding: .utf8)
        let head = "pub const NEW_ENGINE_FEATURES: &[&str] = &["
        let start = try XCTUnwrap(source.range(of: head), "NEW_ENGINE_FEATURES is not in \(url.path)")
        let body = try XCTUnwrap(source[start.upperBound...].range(of: "];")).lowerBound
        var entries: [String] = []
        for line in source[start.upperBound..<body].split(separator: "\n") {
            let l = line.trimmingCharacters(in: .whitespaces)
            guard l.hasPrefix("\""), let close = l.lastIndex(of: "\""), close > l.startIndex else { continue }
            let literal = l[l.index(after: l.startIndex)..<close]
            entries.append(literal.replacingOccurrences(of: "\\\\", with: "\\").replacingOccurrences(of: "\\\"", with: "\""))
        }
        XCTAssertFalse(entries.isEmpty)
        XCTAssertEqual(CaptureFeatures.supportedFeatures(), entries)
    }

    /// `capture_convert` names the document's engine; the bridge picks the
    /// provider's list for it (crates/bridge tests `the_documents_engine_picks…`).
    /// The field is additive: omitted when nil, so an old payload is unchanged.
    func testCaptureConvertCarriesTheEngine() throws {
        func json(_ c: TransferV1.CaptureConvert) throws -> [String: Any] {
            try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(c)) as? [String: Any])
        }
        let new = try json(.init(captureId: "c", supportedFeatures: [], engine: "new"))
        XCTAssertEqual(new["engine"] as? String, "new")
        XCTAssertEqual(new["capture_id"] as? String, "c")
        XCTAssertNil(try json(.init(captureId: "c", supportedFeatures: []))["engine"])
    }
}
