import XCTest
@testable import FlashTeXProtocol

/// `payload.fonts` — the project manifest's `[fonts]` table on the compile
/// request (docs/user/project-manifest.md; the worker's `RenderOptions::fonts`).
/// Absent or empty means the class fonts, exactly the request before the
/// field existed, so every committed fixture and older worker is unchanged.
final class RequestFontsTests: XCTestCase {
    func testFontsAreOmittedFromTheWireWhenNilOrEmpty() throws {
        let plain = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [])
        let line = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "a", plain)), as: UTF8.self)
        XCTAssertFalse(line.contains("\"fonts\""), line)
        let empty = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [], fonts: .init())
        let emptyLine = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "a", empty)), as: UTF8.self)
        XCTAssertFalse(emptyLine.contains("\"fonts\""), "a table naming nothing is no field at all: \(emptyLine)")
        XCTAssertEqual(try RuntimeV1.decodeCompileRequest(Data(emptyLine.utf8)).payload.fonts, nil)
        XCTAssertTrue(RuntimeV1.CompileRequest.Fonts().isEmpty)
    }

    func testFontsEncodeOnlyTheNamedRolesAndRoundTrip() throws {
        let request = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [],
                                               fonts: .init(text: "Libertinus Serif", mono: "JetBrains Mono"))
        let line = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "b", request)), as: UTF8.self)
        XCTAssertTrue(line.contains(#""fonts":{"#), line)
        XCTAssertTrue(line.contains(#""text":"Libertinus Serif""#) && line.contains(#""mono":"JetBrains Mono""#), line)
        XCTAssertFalse(line.contains(#""math""#) || line.contains(#""sans""#), "unset roles are not sent: \(line)")
        let back = try RuntimeV1.decodeCompileRequest(Data(line.utf8))
        XCTAssertEqual(back.payload.fonts, request.fonts)
        XCTAssertEqual(back.payload, request)
        // An older request (no field) decodes with nil.
        let plain = try RuntimeV1.decodeCompileRequest(try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "c", RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: []))))
        XCTAssertNil(plain.payload.fonts)
    }
}
