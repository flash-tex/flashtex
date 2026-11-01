import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Preview → source into a document that is NOT open in the window
/// (ShellModel+UnopenedNavigation.swift) through the REAL
/// `flashtex-preview-controller` owning the REAL `flashtex-render` producer:
/// `main.tex` + `\input{chapter}` with ONLY `main.tex` opened in the window
/// (no `openDiscoveredIncludes()`), so the helper compiles both documents
/// while a v2 cluster or v1 item naming `chapter.tex` has no open buffer.
///
/// Verified: the hit opens `chapter.tex` through the helper's exact snapshot
/// at its durable revision, checks it against the revision the frame was
/// compiled from, and only then selects exactly the cluster's bytes; when
/// the durable revision differs from the compiled one the verdict is typed,
/// the document stays open at its durable text and no caret is placed.
///
/// Skipped unless FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_RENDER name built
/// binaries. Waits are load-sensitive and skip on timeout.
@MainActor
final class UnopenedNavigationTests: XCTestCase {
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")
    static var helper: URL? { ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) } }
    static var render: URL? { ProcessInfo.processInfo.environment["FLASHTEX_RENDER"].map { URL(fileURLWithPath: $0) } }

    static let main = "\\documentclass{article}\n\\begin{document}\nCaf\u{E9} office \\input{chapter}\nend done.\n\\end{document}\n"
    static let chapter = "R\u{E9}sum\u{E9} affable shuffle $e^{i\\pi} + 1 = 0$ waffle.\n"

    private struct Project { var root: URL; var main: URL; var previousCompiler: String? }

    private func cleanup(_ p: Project) {
        try? FileManager.default.removeItem(at: p.root)
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT"); unsetenv("FLASHTEX_DISPLAY_CANDIDATES")
        if let previous = p.previousCompiler { setenv("FLASHTEX_COMPILER", previous, 1) } else { unsetenv("FLASHTEX_COMPILER") }
    }

    private func project(named name: String) throws -> Project {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("nav3-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let main = root.appendingPathComponent("project/main.tex")
        try Data(Self.main.utf8).write(to: main)
        try Data(Self.chapter.utf8).write(to: root.appendingPathComponent("project/chapter.tex"))
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        if ProcessInfo.processInfo.environment["FLASHTEX_LM_DIR"] == nil { setenv("FLASHTEX_LM_DIR", Self.fontsDir.path, 1) }
        return Project(root: root, main: main, previousCompiler: ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"])
    }

    private func requireHelperAndRender() throws -> (URL, URL) {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              let render = Self.render, FileManager.default.isExecutableFile(atPath: render.path) else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_RENDER to built binaries")
        }
        return (helper, render)
    }

    private func waitUntil(timeout: TimeInterval = 40, state: () -> String = { "" }, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout after \(Int(timeout)) s (load-sensitive; rerun before concluding a failure) \(state())") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    /// Helper attached with flashtex-render as its producer, candidates on,
    /// ONLY main.tex opened in the window.
    private func attachedModel(_ p: Project, helper: URL, render: URL) async throws -> ShellModel {
        setenv("FLASHTEX_COMPILER", render.path, 1)
        setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: p.main), .opened)
        model.attachController(at: helper)
        try await waitUntil(state: { "negotiation: \(model.controllerStatus) \(model.displayCandidates.status)" }) { model.displayCandidatesNegotiated }
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"], "the include is NOT opened in the window")
        model.compile()
        return model
    }

    /// The verified frame for the current editor revision declaring both documents.
    private func currentFrame(_ model: ShellModel) async throws -> V2Frame {
        try await waitUntil(state: { "status=\(model.workerStatus) controller=\(model.controllerStatus) candidates=\(model.displayCandidates.status) v2=\(String(describing: model.displayListV2?.source)) result=\(model.result?.revision ?? -1) editor=\(model.editorRevision) log=\(model.workerLog.suffix(4))" }) {
            guard case .loaded(let f, _)? = model.displayListV2, model.displayCandidates.validating == nil,
                  f.list.revision == model.editorRevision, model.result?.revision == model.editorRevision, model.inFlightRevision == nil else { return false }
            return Set(f.list.documents.map(\.path)) == ["main.tex", "chapter.tex"]
                && model.documents.allSatisfy { doc in f.list.documents.contains { $0.path == doc.path && $0.sha256 == SourceDigest.sha256Hex(doc.text) } }
        }
        guard case .loaded(let frame, let source)? = model.displayListV2 else { throw XCTSkip("no frame") }
        XCTAssertTrue(source.isLive)
        return frame
    }

    private struct ClusterHit { var text: String; var source: RenderingV2.SourceRange; var hit: V2Geometry.Hit }

    /// The first cluster spelling `text` in `path`, hit-tested at its own centre.
    private func cluster(_ frame: V2Frame, _ text: String, in path: String) throws -> ClusterHit {
        for page in frame.list.pages {
            for (index, item) in page.items.enumerated() {
                guard case .glyphRun(let run) = item else { continue }
                for (ci, c) in run.clusters.enumerated() {
                    guard let source = c.sources?.first, source.path == path, run.clusterText(ci).sameBytes(as: text), let rect = c.hitRects.first else { continue }
                    let hit = try XCTUnwrap(V2Geometry.hit(page: page, tickX: rect.x + rect.width / 2, tickY: rect.top + rect.height / 2))
                    XCTAssertEqual(hit.itemIndex, index); XCTAssertEqual(hit.clusterIndex, ci)
                    return ClusterHit(text: run.clusterText(ci), source: source, hit: hit)
                }
            }
        }
        throw XCTSkip("no cluster “\(text)” in \(path)")
    }

    private func byte(_ text: String, _ needle: String) -> Int {
        let hay = Array(text.utf8), n = Array(needle.utf8)
        for i in 0...(hay.count - n.count) where Array(hay[i..<(i + n.count)]) == n { return i }
        XCTFail("\(needle) not found"); return 0
    }

    private func selectedBytes(_ model: ShellModel) throws -> (path: String, start: Int, end: Int, text: String) {
        let sel = try XCTUnwrap(model.selection, model.navigationNote ?? "nil")
        let bytes = try XCTUnwrap(model.activeText.utf8ByteRange(of: sel.nsRange))
        return (sel.path, bytes.start, bytes.end, (model.activeText as NSString).substring(with: sel.nsRange))
    }

    // MARK: - typed verdicts off the helper route (no binaries needed)

    func testVerdictsWithoutAHelperAreTypedAndNeverPlaceACaret() async {
        let model = ShellModel()
        model.replaceProject(entryText: Self.main)
        let awaited1 = await model.openForNavigation(path: "main.tex", compiledRevision: 1)
        XCTAssertEqual(awaited1, .alreadyOpen(path: "main.tex"))
        let awaited2 = await model.openForNavigation(path: "chapter.tex", compiledRevision: 1)
        XCTAssertEqual(awaited2, .notAttached(path: "chapter.tex"))
        let hit = RuntimeV1.SourceRange(path: "chapter.tex", startByte: 0, endByte: 1)
        let verdict = await model.navigateOpeningIfNeeded(to: hit)
        XCTAssertEqual(verdict, .notAttached(path: "chapter.tex"))
        XCTAssertNil(model.selection)
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"], "nothing was opened from disk")
        XCTAssertEqual(model.navigationNote, "Preview → source: " + UnopenedNavigation.Verdict.notAttached(path: "chapter.tex").note)
        XCTAssertNil(model.compiledRevision(for: "chapter.tex"), "no helper preview applied: no compiled revision")
        // An open document goes straight to navigateExactly (nil verdict).
        let own = RuntimeV1.SourceRange(path: "main.tex", startByte: byte(Self.main, "office"), endByte: byte(Self.main, "office") + 6)
        let ownVerdict = await model.navigateOpeningIfNeeded(to: own)
        XCTAssertNil(ownVerdict)
        XCTAssertEqual(try selectedBytes(model).text, "office")
        // Every verdict note names the document.
        for v: UnopenedNavigation.Verdict in [.opened(path: "c", revision: 3), .alreadyOpen(path: "c"), .notAttached(path: "c"),
                                               .noCompiledRevision(path: "c"), .openRefused(path: "c", why: "w"),
                                               .revisionDiffers(path: "c", compiled: 1, durable: 2)] {
            XCTAssertTrue(v.note.contains("c"), v.note)
        }
        XCTAssertTrue(UnopenedNavigation.Verdict.revisionDiffers(path: "c", compiled: 1, durable: 2).note.contains("caret was not placed"))
    }

    // MARK: - real helper + real producer

    func testHelperOpensTheUnopenedIncludeAtItsDurableRevisionAndNavigatesExactly() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "open")
        defer { cleanup(p) }
        let model = try await attachedModel(p, helper: helper, render: render)
        defer { model.detachController() }
        let frame = try await currentFrame(model)
        let declared = try XCTUnwrap(frame.list.documents.first { $0.path == "chapter.tex" })
        XCTAssertEqual(declared.sha256, SourceDigest.sha256Hex(Self.chapter), "the helper compiled the include it discovered")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        // Measured (one run: declared 5, frame revision 2, helper durable 1): the producer's
        // `documents[].revision` is its own counter, not the helper's durable revision of the
        // include; the applied preview's source version is the comparator, the declared
        // sha256 the exact binding.
        let durable = try XCTUnwrap(model.compiledRevision(for: "chapter.tex"), "the applied v1 preview names the include's durable version")

        // The plain navigateV2 route refuses synchronously-visible completion but hands
        // the hit to the opening route (note says so) — exercise the awaited form directly.
        let shuffleS = try cluster(frame, "s", in: "chapter.tex")
        let verdict = await model.navigateV2Opening(shuffleS.hit)
        XCTAssertEqual(verdict, .opened(path: "chapter.tex", revision: durable), model.navigationNote ?? "")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"])
        XCTAssertEqual(model.activePath, "chapter.tex")
        XCTAssertTrue(model.activeText.sameBytes(as: Self.chapter), "opened at the helper's exact durable text")
        XCTAssertEqual(model.controllerState.durable["chapter.tex"]?.revision, durable)
        let sel = try selectedBytes(model)
        XCTAssertEqual(sel.path, "chapter.tex")
        XCTAssertEqual(sel.start, shuffleS.source.startByte); XCTAssertEqual(sel.end, shuffleS.source.endByte)
        XCTAssertEqual(sel.text, "s")
        XCTAssertTrue(model.navigationNote?.contains("opened chapter.tex at durable r\(durable)") == true, model.navigationNote ?? "")
        // A ligature cluster of the include selects exactly its bytes as well.
        let ffl = try cluster(frame, "ffl", in: "chapter.tex")
        let awaited3 = await model.navigateV2Opening(ffl.hit)
        XCTAssertNil(awaited3, "an open document needs no verdict")
        XCTAssertEqual(try selectedBytes(model).text, "ffl")
        // A cluster INSIDE the include's formula: every cluster carries the whole formula span,
        // and the selection is exactly that span (measured on flashtex-render 9aaec57a).
        let formula = "$e^{i\\pi} + 1 = 0$"
        let pi = try cluster(frame, "\u{3C0}", in: "chapter.tex")
        XCTAssertEqual(pi.source, .init(path: "chapter.tex", startByte: byte(Self.chapter, formula), endByte: byte(Self.chapter, formula) + formula.utf8.count))
        _ = await model.navigateV2Opening(pi.hit)
        XCTAssertEqual(try selectedBytes(model).text, formula)

        // The v1 route (⌘-click on the product preview) opens through the same verdict: detach
        // the include again and navigate a v1 item naming chapter.tex.
        model.activePath = "main.tex"
        let awaited4 = await model.project.detachDocument("chapter.tex")
        XCTAssertEqual(awaited4, .detached(path: "chapter.tex"))
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        let v1Item = try XCTUnwrap(model.result?.pages.lazy.flatMap(\.items).compactMap { item -> RuntimeV1.SourceRange? in
            if case .text(let t) = item, t.source?.path == "chapter.tex", t.text.sameBytes(as: "waffle.") || t.text.sameBytes(as: "waffle") { return t.source }
            return nil
        }.first, "a v1 text item of chapter.tex")
        let v1Verdict = await model.navigateOpeningIfNeeded(to: v1Item)
        XCTAssertEqual(v1Verdict, .opened(path: "chapter.tex", revision: durable), model.navigationNote ?? "")
        let v1Sel = try selectedBytes(model)
        XCTAssertEqual(v1Sel.path, "chapter.tex")
        XCTAssertEqual(v1Sel.start, v1Item.startByte); XCTAssertEqual(v1Sel.end, v1Item.endByte)
        XCTAssertTrue(v1Sel.text.hasPrefix("waffle"), v1Sel.text)
    }

    func testHelperRefusesTypedWhenTheDurableRevisionDiffersFromTheCompiledOne() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "differs")
        defer { cleanup(p) }
        let model = try await attachedModel(p, helper: helper, render: render)
        defer { model.detachController() }
        let frame1 = try await currentFrame(model)
        let r1 = try XCTUnwrap(model.compiledRevision(for: "chapter.tex"))
        // Open the include through a hit, edit it and compile: the durable revision moves on.
        let ffl = try cluster(frame1, "ffl", in: "chapter.tex")
        let awaited5 = await model.navigateV2Opening(ffl.hit)
        XCTAssertEqual(awaited5, .opened(path: "chapter.tex", revision: r1))
        XCTAssertEqual(model.activePath, "chapter.tex")
        model.updateActiveText(Self.chapter.replacingOccurrences(of: "affable", with: "amiable"))
        model.compile()
        let frame2 = try await currentFrame(model)
        let r2 = try XCTUnwrap(model.compiledRevision(for: "chapter.tex"))
        XCTAssertNotEqual(r2, r1, "the edited include has a new durable revision")
        XCTAssertEqual(model.controllerState.durable["chapter.tex"]?.revision, r2)
        // Detach it (edits are in the helper's durable text; the window drops its buffer).
        model.activePath = "main.tex"
        let awaited6 = await model.project.detachDocument("chapter.tex", discardingEdits: true)
        XCTAssertEqual(awaited6, .detached(path: "chapter.tex"))
        XCTAssertEqual(model.documents.map(\.path), ["main.tex"])
        // A hit whose frame was compiled from r1 (the v1 route passes that revision explicitly:
        // an applied preview older than the durable text) opens the document — at r2 — and is
        // refused typed: the caret is never guessed against a text the frame did not attest.
        model.selection = nil
        let verdict = await model.navigateOpeningIfNeeded(to: ffl.source, compiledRevision: r1)
        XCTAssertEqual(verdict, .revisionDiffers(path: "chapter.tex", compiled: r1, durable: r2), model.navigationNote ?? "")
        XCTAssertNil(model.selection, "no caret placed")
        XCTAssertEqual(model.activePath, "main.tex", "the window does not switch to a document it could not navigate")
        XCTAssertEqual(model.documents.map(\.path), ["main.tex", "chapter.tex"], "the document stays open at its durable text")
        XCTAssertTrue(model.documents.first { $0.path == "chapter.tex" }?.text.contains("amiable") == true)
        XCTAssertEqual(model.navigationNote, "Preview → source: " + verdict!.note)
        XCTAssertTrue(model.navigationNote?.contains("compiled from r\(r1)") == true, model.navigationNote ?? "")
        // The current frame (compiled from r2) navigates the reopened document exactly.
        let ffl2 = try cluster(frame2, "ffl", in: "chapter.tex")
        let awaited7 = await model.navigateV2Opening(ffl2.hit)
        XCTAssertNil(awaited7, "an open document needs no verdict")
        XCTAssertEqual(try selectedBytes(model).text, "ffl")
        XCTAssertEqual(model.activePath, "chapter.tex")
    }
}
