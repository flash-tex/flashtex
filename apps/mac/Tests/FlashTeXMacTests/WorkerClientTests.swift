import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

final class WorkerClientTests: XCTestCase {
    static let fakeWorker = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().appendingPathComponent("Fixtures/fake_worker.py")
    static let python = URL(fileURLWithPath: "/usr/bin/python3")

    private func makeClient(_ handler: @escaping (WorkerClient.Event) -> Void) throws -> WorkerClient {
        try WorkerClient(executable: Self.python, arguments: [Self.fakeWorker.path],
                         queue: DispatchQueue(label: "test"), handler: handler)
    }

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

    func testRoundTripThroughFakeWorker() throws {
        let got = expectation(description: "compile_result")
        var received: RuntimeV1.Envelope<RuntimeV1.CompileResult>?
        var stderrSeen = false
        let client = try makeClient { event in
            switch event {
            case .result(let env): received = env; got.fulfill()
            case .stderr: stderrSeen = true
            default: break
            }
        }
        defer { client.terminate() }
        let req = RuntimeV1.CompileRequest(projectId: "demo", revision: 7, entryPath: "main.tex",
                                           documents: [.init(path: "main.tex", text: "Héllo\nsecond")])
        try client.send(req, id: "req-7")
        wait(for: [got], timeout: 10)
        let env = try XCTUnwrap(received)
        XCTAssertEqual(env.id, "req-7")
        XCTAssertEqual(env.payload.revision, 7)
        guard case .text(let item) = env.payload.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(item.text, "Héllo")
        XCTAssertEqual(item.source?.endByte, 6) // é is two bytes
        XCTAssertTrue(stderrSeen, "stderr diagnostics are surfaced, not swallowed")
    }

    func testErrorAndGarbageLinesAreReported() throws {
        let err = expectation(description: "error"), bad = expectation(description: "violation")
        let client = try makeClient { event in
            switch event {
            case .error(let id, let message): XCTAssertEqual(id, "e1"); XCTAssertEqual(message, "requested failure"); err.fulfill()
            case .protocolViolation: bad.fulfill()
            default: break
            }
        }
        defer { client.terminate() }
        try client.send(.init(projectId: "p", revision: 1, entryPath: "m", documents: [.init(path: "m", text: "%error")]), id: "e1")
        try client.send(.init(projectId: "p", revision: 2, entryPath: "m", documents: [.init(path: "m", text: "%garbage")]), id: "g1")
        wait(for: [err, bad], timeout: 10)
    }

    func testCompleteOversizedLineIsRejectedAndWorkerTerminated() throws {
        let bad = expectation(description: "violation"), exited = expectation(description: "exited")
        var message = ""
        let client = try makeClient { event in
            switch event {
            case .protocolViolation(let m): message = m; bad.fulfill()
            case .exited: exited.fulfill()
            case .result: XCTFail("oversized line must not be delivered as a result")
            default: break
            }
        }
        try client.send(.init(projectId: "p", revision: 1, entryPath: "m", documents: [.init(path: "m", text: "%huge")]), id: "h1")
        wait(for: [bad, exited], timeout: 30)
        XCTAssertTrue(message.contains("exceeds"), message)
        XCTAssertFalse(client.isRunning)
    }

    func testUnterminatedTrailingBytesAtEOFAreAViolation() throws {
        let bad = expectation(description: "violation"), exited = expectation(description: "exited")
        var message = ""
        let client = try makeClient { event in
            switch event {
            case .protocolViolation(let m): message = m; bad.fulfill()
            case .exited: exited.fulfill()
            case .result: XCTFail("partial line must not be delivered")
            default: break
            }
        }
        try client.send(.init(projectId: "p", revision: 1, entryPath: "m", documents: [.init(path: "m", text: "%trailing")]), id: "t1")
        wait(for: [bad, exited], timeout: 10)
        XCTAssertTrue(message.contains("unterminated"), message)
    }

    func testExitIsReported() throws {
        let exited = expectation(description: "exited")
        let client = try makeClient { if case .exited = $0 { exited.fulfill() } }
        client.terminate()
        wait(for: [exited], timeout: 10)
        XCTAssertFalse(client.isRunning)
    }

    func testDecodeRejectsWrongVersionAndUnknownType() {
        if case .protocolViolation = WorkerClient.decode(Data(#"{"protocol_version":9,"id":"x","type":"compile_result","payload":{}}"#.utf8)) {} else { XCTFail() }
        if case .protocolViolation = WorkerClient.decode(Data(#"{"protocol_version":1,"id":"x","type":"capture_proposal","payload":{}}"#.utf8)) {} else { XCTFail() }
    }
}
