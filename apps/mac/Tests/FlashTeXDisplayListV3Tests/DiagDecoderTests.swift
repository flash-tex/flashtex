import Foundation
import XCTest
@testable import FlashTeXDisplayListV3

/// diag-v1 `DIAG` decoding (spec §6.7), on the specification's own example,
/// with the reference decoder's rules (unknown keys ignored, only a missing
/// `message` is an error).
final class DiagDecoderTests: XCTestCase {
    static let example = #"""
    {"id": 12, "seq": 1, "severity": "error",
     "code": "tex/undefined-control-sequence", "origin": "tex",
     "message": "Undefined control sequence.",
     "file": "/Users/me/paper/main.tex", "line": 6, "col": 19, "range": [10, 19],
     "offset": 133, "span": 593,
     "trace": [
       {"kind": "argument", "text": ["x \\undefinedthing ", ""]},
       {"kind": "macro", "name": "\\textbf",
        "text": ["#1->\\ifmmode \\nfss@text {\\bfseries #1}...", "\\check@icr ..."]},
       {"kind": "macro", "name": "\\mycmd", "text": ["#1->\\textbf {#1 \\undefinedthing }", ""],
        "def": {"file": "/Users/me/paper/main.tex", "line": 2}},
       {"kind": "file", "file": "/Users/me/paper/main.tex", "line": 6, "col": 19,
        "text": ["Some text \\mycmd{x}", " more."]}],
     "help": ["The control sequence at the end of the top line",
              "of your error message was never \\def'ed. ..."],
     "exact": true, "someFutureKey": [1, 2]}
    """#

    func testTheSpecificationsExample() throws {
        let d = try DL3Diag.decode(Array(Self.example.utf8))
        XCTAssertEqual(d.id, 12); XCTAssertEqual(d.seq, 1); XCTAssertEqual(d.severity, "error")
        XCTAssertEqual(d.code, "tex/undefined-control-sequence"); XCTAssertEqual(d.origin, "tex")
        XCTAssertEqual(d.line, 6); XCTAssertEqual(d.col, 19)
        XCTAssertEqual(d.range?.0, 10); XCTAssertEqual(d.range?.1, 19)
        XCTAssertEqual(d.offset, 133); XCTAssertEqual(d.span, 593); XCTAssertTrue(d.exact)
        XCTAssertEqual(d.trace.map(\.kind), ["argument", "macro", "macro", "file"])
        XCTAssertEqual(d.trace[2].name, "\\mycmd")
        XCTAssertEqual(d.trace[2].def?.line, 2)
        XCTAssertEqual(d.trace[3].loc?.col, 19)
        XCTAssertEqual(d.trace[3].before, "Some text \\mycmd{x}"); XCTAssertEqual(d.trace[3].after, " more.")
        XCTAssertNil(d.trace[1].loc)
        XCTAssertEqual(d.help.count, 2)
        // As a frame of its own kind.
        guard case .diag(let e) = try DL3Event.decode(kind: DL3Diag.kind, body: Array(Self.example.utf8)) else { return XCTFail("not a diag event") }
        XCTAssertEqual(e, d)
        XCTAssertEqual(DL3.Kind.name(DL3Diag.kind), "diag")
    }

    func testLenientButNotWithoutAMessage() throws {
        let d = try DL3Diag.decode(Array(#"{"message": "LaTeX Warning: something", "severity": "whatever"}"#.utf8))
        XCTAssertNil(d.severity); XCTAssertFalse(d.exact); XCTAssertEqual(d.trace, []); XCTAssertEqual(d.code, "")
        XCTAssertThrowsError(try DL3Diag.decode(Array(#"{"severity": "error"}"#.utf8)))
    }
}
