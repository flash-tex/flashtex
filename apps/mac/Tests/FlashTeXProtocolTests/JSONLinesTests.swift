import XCTest
import FlashTeXProtocol

/// The JSON-lines framing the app's line-protocol clients share
/// (FlashTeXProtocol/JSONLines.swift): the splitter keeps a partial line
/// until its newline, and an encoded envelope is exactly one line.
final class JSONLinesTests: XCTestCase {
    func testLineSplitterKeepsPartialLine() {
        var s = LineSplitter()
        XCTAssertEqual(s.append(Data("ab".utf8)), [])
        XCTAssertEqual(s.append(Data("c\nde\nf".utf8)).map { String(decoding: $0, as: UTF8.self) }, ["abc", "de"])
        XCTAssertEqual(s.pendingBytes, 1)
        XCTAssertEqual(s.append(Data("\n".utf8)).map { String(decoding: $0, as: UTF8.self) }, ["f"])
    }

    func testEncodeLineIsSingleLineWithSnakeCaseKeys() throws {
        let req = RuntimeV1.CompileRequest(projectId: "p", revision: 3, entryPath: "main.tex",
                                           documents: [.init(path: "main.tex", text: "a\nb")])
        let line = try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "r1", req))
        let s = String(decoding: line, as: UTF8.self)
        XCTAssertTrue(s.hasSuffix("\n"))
        XCTAssertEqual(s.filter { $0 == "\n" }.count, 1, "newline inside text must be escaped")
        XCTAssertTrue(s.contains(#""protocol_version":1"#))
        XCTAssertTrue(s.contains(#""entry_path":"main.tex""#))
        XCTAssertTrue(s.contains(#""type":"compile""#))
    }
}
