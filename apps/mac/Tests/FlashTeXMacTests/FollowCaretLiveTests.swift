import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Live: "Preview follows the caret" (FollowCaret.swift) against a REAL
/// compile of HW1 (`fixtures/real-world/hw1/HW1.tex`) via `FLASHTEX_RENDER`.
/// Confirms the follow-caret target actually resolves to page 3 for a caret
/// placed on a page-3 item — the same target `PreviewAnchorProbe` debounces
/// and scrolls the preview to (`evaluateFollow()` / `performFollowScroll(to:)`
/// in PreviewAnchor.swift). Skipped (loudly, via `XCTSkip`) unless
/// FLASHTEX_RENDER names a built binary.
@MainActor
final class FollowCaretLiveTests: XCTestCase {
    /// Repo root is 5 path components up from this file
    /// (FlashTeXMacTests → Tests → mac → apps → root).
    static let fixture = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("fixtures/real-world/hw1/HW1.tex")

    private func render() throws -> URL {
        guard let path = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"], FileManager.default.isExecutableFile(atPath: path) else {
            throw XCTSkip("set FLASHTEX_RENDER=crates/render-pipeline/target/release/flashtex-render (cargo build --release in crates/render-pipeline)")
        }
        return URL(fileURLWithPath: path)
    }

    /// One compile request over the live producer (one line in, first line out) —
    /// same pattern as `EditorDiagnosticsPartialOutputTests.compile`.
    private func compile(_ text: String, with producer: URL) throws -> RuntimeV1.CompileResult {
        let req = RuntimeV1.CompileRequest(projectId: "hw1-live", revision: 1, entryPath: "main.tex",
                                           documents: [.init(path: "main.tex", text: text)])
        let line = try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "follow-caret-live-1", req))
        let p = Process(); p.executableURL = producer
        let stdin = Pipe(), stdout = Pipe()
        p.standardInput = stdin; p.standardOutput = stdout; p.standardError = FileHandle.nullDevice
        try p.run()
        try stdin.fileHandleForWriting.write(contentsOf: line)
        try stdin.fileHandleForWriting.close()
        let out = stdout.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        let first = try XCTUnwrap(out.split(separator: 0x0A).first, "the producer answered nothing")
        return try RuntimeV1.decodeCompileResult(Data(first)).payload
    }

    /// Moving the caret onto a page-3 item resolves the v1 follow-caret
    /// target to page 3 — the level `evaluateFollow()` acts on; the actual
    /// `NSScrollView` animation is chrome exercised interactively, not headlessly.
    func testCaretOnPageThreeResolvesTheFollowTargetToPageThree() throws {
        let producer = try render()
        guard FileManager.default.fileExists(atPath: Self.fixture.path) else {
            throw XCTSkip("HW1 fixture not found at \(Self.fixture.path)")
        }
        let text = try String(contentsOf: Self.fixture, encoding: .utf8)
        let result = try compile(text, with: producer)
        XCTAssertTrue(result.status == .ok || result.status == .recovered,
                      "HW1 is expected to produce pages (status: \(result.status))")
        guard result.pages.count >= 3 else {
            throw XCTSkip("HW1 rendered only \(result.pages.count) page(s) with this producer; need at least 3 for this check")
        }
        let page3 = try XCTUnwrap(result.pages.first { $0.number == 3 })
        var source: RuntimeV1.SourceRange?
        for item in page3.items {
            if case .text(let t) = item, let s = t.source, s.path == "main.tex" { source = s; break }
        }
        let span = try XCTUnwrap(source, "page 3 should have at least one text item with a source mapping")

        let model = ShellModel()
        model.documents = [.init(path: "main.tex", text: text)]
        model.activePath = "main.tex"
        model.result = result
        guard case .selected(let ns, _) = Navigation.editorRange(start: span.startByte, end: span.startByte, in: text, path: "main.tex") else {
            return XCTFail("page 3's source span \(span.startByte) does not map onto an editor caret in the compiled text")
        }
        model.caretUTF16 = ns.location

        let target = try XCTUnwrap(model.followCaretTargetV1(), "a caret on a page-3 item must resolve to a follow-caret target")
        XCTAssertEqual(target.page, 3, "moving the caret to page 3 must resolve the preview follow target to page 3")
    }
}
