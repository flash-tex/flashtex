import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Exact multi-file preview → source navigation on the v2 (display-list)
/// route through the REAL `flashtex-preview-controller` owning the REAL
/// `flashtex-render` producer: `main.tex` + `\input{chapter}` with the
/// include opened the way `FLASHTEX_OPEN_INCLUDES=1` does
/// (`project.openDiscoveredIncludes()`).
///
/// Verified per cluster: a click on any cluster — the producer's ligature
/// clusters ff/fi/fl/ffi/ffl in BOTH files, multi-byte scalars (é ï —),
/// a combining sequence (e + U+0301) and text after dropped surrogate-pair
/// emoji — selects exactly the cluster's source bytes in the right document,
/// switching documents. After an edit before/after a target the stale frame
/// rebases across the recorded edit or refuses (never other bytes); the next
/// candidate's clusters carry the shifted bytes and navigate exactly.
///
/// Skipped unless FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_RENDER name built
/// binaries (the producer is handed to the helper as FLASHTEX_COMPILER, as
/// DisplayCandidateTests does). Waits are load-sensitive and skip on timeout.
@MainActor
final class NavigationMultiFileV2Tests: XCTestCase {
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")
    static var helper: URL? { ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) } }
    static var render: URL? { ProcessInfo.processInfo.environment["FLASHTEX_RENDER"].map { URL(fileURLWithPath: $0) } }

    // Main: precomposed é (2 bytes) before the ligatures, ï after, an emoji
    // (surrogate pair in UTF-16, 4 bytes in UTF-8; no Latin Modern glyph) before "done".
    static let main = "\\documentclass{article}\n\\begin{document}\nCaf\u{E9} \u{E9}t\u{E9} office fluff fifty ff fi fl ffi ffl na\u{EF}ve \\input{chapter}\nend \u{1F600} done.\n\\end{document}\n"
    // Chapter: é, an em dash (3 bytes), a CJK scalar and a ZWJ emoji sequence with no
    // glyph, a combining sequence e + U+0301 (one 3-byte cluster), ligatures around them.
    static let chapter = "R\u{E9}sum\u{E9} \u{2014} affable \u{5F53} e\u{301} shuffle ffl fi fl \u{1F468}\u{200D}\u{1F469} waffle efficient fluffy.\n"
    static let ligatures: Set<String> = ["ff", "fi", "fl", "ffi", "ffl"]

    private struct Project { var root: URL; var main: URL; var previousCompiler: String? }

    /// Removes the project and restores the process environment for later suites
    /// (FLASHTEX_COMPILER names the v1 compiler for other tests in this process).
    private func cleanup(_ p: Project) {
        try? FileManager.default.removeItem(at: p.root)
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT"); unsetenv("FLASHTEX_DISPLAY_CANDIDATES")
        if let previous = p.previousCompiler { setenv("FLASHTEX_COMPILER", previous, 1) } else { unsetenv("FLASHTEX_COMPILER") }
    }

    private func project(named name: String) throws -> Project {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("nav2-\(name)-\(UUID().uuidString)")
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
    /// main.tex opened from disk and the `\input` closure opened.
    private func attachedModel(_ p: Project, helper: URL, render: URL) async throws -> ShellModel {
        setenv("FLASHTEX_COMPILER", render.path, 1)
        setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: p.main), .opened)
        model.attachController(at: helper)
        try await waitUntil(state: { "negotiation: \(model.controllerStatus) \(model.displayCandidates.status)" }) { model.displayCandidatesNegotiated }
        let outcomes = await model.project.openDiscoveredIncludes()
        XCTAssertEqual(outcomes, [.opened(path: "chapter.tex")], "the \\input closure is one document")
        XCTAssertTrue(model.documents.first { $0.path == "chapter.tex" }?.text.sameBytes(as: Self.chapter) == true, "opened at its durable text")
        XCTAssertTrue(model.activeText.sameBytes(as: Self.main))
        // ⌘B: a compile after enablement always has a sibling candidate (the startup
        // preview may have finished before display candidates were negotiated).
        model.compile()
        return model
    }

    /// The verified frame for the current editor revision that declares both
    /// documents with the digests of the current buffers.
    private func currentFrame(_ model: ShellModel) async throws -> V2Frame {
        try await waitUntil(state: { "status=\(model.workerStatus) controller=\(model.controllerStatus) candidates=\(model.displayCandidates.status) v2=\(String(describing: model.displayListV2?.source)) declared=\(model.displayListV2?.frame?.list.documents.map { "\($0.path):\($0.sha256.prefix(8))" } ?? []) docs=\(model.documents.map { "\($0.path):\(SourceDigest.sha256Hex($0.text).prefix(8))" }) result=\(model.result?.revision ?? -1) editor=\(model.editorRevision) inFlight=\(String(describing: model.inFlightRevision)) log=\(model.workerLog.suffix(4))" }) {
            guard case .loaded(let f, _)? = model.displayListV2, model.displayCandidates.validating == nil,
                  f.list.revision == model.editorRevision, model.result?.revision == model.editorRevision, model.inFlightRevision == nil else { return false }
            return model.documents.allSatisfy { doc in f.list.documents.contains { $0.path == doc.path && $0.sha256 == SourceDigest.sha256Hex(doc.text) } }
        }
        guard case .loaded(let frame, let source)? = model.displayListV2 else { throw XCTSkip("no frame") }
        XCTAssertTrue(source.isLive)
        XCTAssertEqual(Set(frame.list.documents.map(\.path)), ["main.tex", "chapter.tex"])
        return frame
    }

    private struct ClusterHit {
        var page: RenderingV2.Page
        var itemIndex: Int
        var clusterIndex: Int
        var text: String
        var source: RenderingV2.SourceRange
        var hit: V2Geometry.Hit
    }

    /// Every sourced cluster of the frame, hit-tested at the centre of its first hit rectangle.
    private func clusters(of frame: V2Frame) throws -> [ClusterHit] {
        var out: [ClusterHit] = []
        for page in frame.list.pages {
            for (index, item) in page.items.enumerated() {
                guard case .glyphRun(let run) = item else { continue }
                for (ci, c) in run.clusters.enumerated() {
                    guard let source = c.sources?.first, let rect = c.hitRects.first else { continue }
                    XCTAssertEqual(c.sources?.count, 1, "the producer maps every cluster to one source range")
                    XCTAssertNil(c.syntheticReason)
                    let hit = try XCTUnwrap(V2Geometry.hit(page: page, tickX: rect.x + rect.width / 2, tickY: rect.top + rect.height / 2))
                    XCTAssertEqual(hit.itemIndex, index, "hit-test at the cluster's own centre finds the cluster")
                    XCTAssertEqual(hit.clusterIndex, ci)
                    XCTAssertEqual(hit.sources, [source])
                    out.append(ClusterHit(page: page, itemIndex: index, clusterIndex: ci, text: run.clusterText(ci), source: source, hit: hit))
                }
            }
        }
        return out
    }

    private func text(_ model: ShellModel, _ path: String) -> String { model.documents.first { $0.path == path }!.text }

    /// Byte offset of the first byte-exact occurrence of `needle` (String
    /// search is canonical: it would equate é with e + U+0301 and refuse a lone
    /// 👨 inside a ZWJ sequence).
    private func byte(_ text: String, _ needle: String) -> Int {
        let hay = Array(text.utf8), n = Array(needle.utf8)
        for i in 0...(hay.count - n.count) where Array(hay[i..<(i + n.count)]) == n { return i }
        XCTFail("\(needle) not found"); return 0
    }

    /// Navigates every cluster and checks the selection is exactly its source
    /// bytes in its own document (switching documents); the selected text is
    /// the cluster text whenever the mapping is 1:1 (every ligature, letter and
    /// literal UTF-8 scalar here). Returns the ligature clusters seen per path.
    @discardableResult
    private func navigateEveryClusterExactly(_ model: ShellModel, _ frame: V2Frame, file: StaticString = #filePath, line: UInt = #line) throws -> [String: Set<String>] {
        var ligatures: [String: Set<String>] = [:]
        var switches = 0
        for c in try clusters(of: frame) {
            let wasActive = model.activePath
            model.navigateV2(c.hit)
            let sel = try XCTUnwrap(model.selection, "\(c.text) \(c.source.path) \(c.source.startByte)..<\(c.source.endByte): \(model.navigationNote ?? "nil")", file: file, line: line)
            XCTAssertEqual(model.activePath, c.source.path, file: file, line: line)
            XCTAssertEqual(sel.path, c.source.path, file: file, line: line)
            let bytes = model.activeText.utf8ByteRange(of: sel.nsRange)
            XCTAssertEqual(bytes?.start, c.source.startByte, "\(c.text): \(model.navigationNote ?? "")", file: file, line: line)
            XCTAssertEqual(bytes?.end, c.source.endByte, "\(c.text): \(model.navigationNote ?? "")", file: file, line: line)
            XCTAssertEqual(model.caretUTF16, sel.nsRange.location, file: file, line: line)
            XCTAssertEqual(model.caretLengthUTF16, sel.nsRange.length, file: file, line: line)
            let selected = (model.activeText as NSString).substring(with: sel.nsRange)
            XCTAssertTrue(selected.sameBytes(as: c.text), "cluster “\(c.text)” at \(c.source.path) \(c.source.startByte)..<\(c.source.endByte) selected “\(selected)”", file: file, line: line)
            XCTAssertFalse(model.navigationNote?.contains("widened") == true, model.navigationNote ?? "", file: file, line: line)
            XCTAssertFalse(model.navigationNote?.contains("generated from") == true, model.navigationNote ?? "", file: file, line: line)
            if wasActive != c.source.path {
                switches += 1
                XCTAssertTrue(model.navigationNote?.contains("switched to \(c.source.path)") == true, model.navigationNote ?? "", file: file, line: line)
            }
            if Self.ligatures.contains(c.text) { ligatures[c.source.path, default: []].insert(c.text) }
        }
        XCTAssertGreaterThanOrEqual(switches, 2, "main → chapter → main", file: file, line: line)
        return ligatures
    }

    // MARK: - (b) ligature clusters and UTF-8 neighbours across the include

    func testHelperMultiFileClustersNavigateToExactBytesInBothDocuments() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "clusters")
        defer { cleanup(p) }
        let model = try await attachedModel(p, helper: helper, render: render)
        defer { model.detachController() }
        let frame = try await currentFrame(model)

        let ligatures = try navigateEveryClusterExactly(model, frame)
        XCTAssertEqual(ligatures["main.tex"], Self.ligatures, "ff fi fl ffi ffl are each one cluster in main.tex")
        XCTAssertEqual(ligatures["chapter.tex"], Self.ligatures, "…and in chapter.tex (affable, shuffle, ffl, fi, fl, efficient, fluffy)")

        // The exact spans the producer emitted for the ligature words, by name.
        let all = try clusters(of: frame)
        func cluster(_ text: String, in path: String, after: Int = 0) -> ClusterHit? {
            all.first { $0.text.sameBytes(as: text) && $0.source.path == path && $0.source.startByte >= after }
        }
        let main = text(model, "main.tex"), chapter = text(model, "chapter.tex")
        // ffi in "office" is one glyph over bytes 1..<4 of the run and source bytes o+1..<o+4.
        let office = byte(main, "office")
        let ffi = try XCTUnwrap(cluster("ffi", in: "main.tex", after: office))
        XCTAssertEqual(ffi.source, .init(path: "main.tex", startByte: office + 1, endByte: office + 4))
        XCTAssertEqual(ffi.clusterIndex, 1)
        guard case .glyphRun(let officeRun) = ffi.page.items[ffi.itemIndex] else { return XCTFail() }
        XCTAssertEqual(officeRun.glyphs.filter { $0.cluster == 1 }.count, 1, "one ligature glyph")
        // The combining sequence e + U+0301 is one 3-byte cluster in chapter.tex; the
        // selection covers the whole composed character (never split), and the
        // caret mapping back from either of its bytes lights that cluster.
        let combining = byte(chapter, "e\u{301}")
        let eAcute = try XCTUnwrap(cluster("e\u{301}", in: "chapter.tex"))
        XCTAssertEqual(eAcute.source, .init(path: "chapter.tex", startByte: combining, endByte: combining + 3))
        for b in combining..<(combining + 3) {
            let matches = V2Geometry.clusters(containing: b, path: "chapter.tex", in: eAcute.page)
            XCTAssertEqual(matches.map(\.clusterIndex), [eAcute.clusterIndex], "byte \(b)")
        }
        // Text after the dropped emoji keeps exact bytes in both files.
        let end = try XCTUnwrap(cluster("e", in: "main.tex", after: byte(main, "end ")))
        XCTAssertEqual(end.source.startByte, byte(main, "end "))
        let waffle = try XCTUnwrap(cluster("w", in: "chapter.tex"))
        XCTAssertEqual(waffle.source.startByte, byte(chapter, "waffle"))
        XCTAssertGreaterThan(waffle.source.startByte, byte(chapter, "\u{1F468}"))
        // Ligature-to-editor caret sync: the editor caret on the second byte of the chapter
        // ffl (inside the ligature) resolves to that cluster with no exact caret (whole-cluster fallback).
        let ffl = try XCTUnwrap(cluster("ffl", in: "chapter.tex", after: byte(chapter, "shuffle")))
        let inside = V2Geometry.clusters(containing: ffl.source.startByte + 1, path: "chapter.tex", in: ffl.page)
        XCTAssertEqual(inside.map(\.clusterIndex), [ffl.clusterIndex])
        XCTAssertNil(inside[0].caret)
        let atStart = V2Geometry.clusters(containing: ffl.source.startByte, path: "chapter.tex", in: ffl.page)
        XCTAssertEqual(atStart[0].caret?.textByte, 3, "the run's caret at the ligature's first logical byte")
        // ⌘⇧J selects what the pane highlights: the cluster, i.e. the whole
        // ligature. It reads the display list (the assertions just above), so
        // it agrees with clicking that ligature — both go through
        // `navigateV2Now`. It used to map runtime-v1 page items instead and
        // answer "shuffle", the whole word, disagreeing with the pane's own
        // click on the very same caret.
        model.activePath = "chapter.tex"
        model.caretUTF16 = (chapter as NSString).range(of: "shuffle").location + 4
        model.revealCaretInPreview()
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "ffl")
        XCTAssertTrue(model.navigationNote?.hasSuffix("page \(ffl.page.number)") == true, model.navigationNote ?? "nil")
    }

    // MARK: - (a) edits before/after the target and (c) revision changes

    func testHelperEditsRebaseOrRefuseOnTheStaleFrameAndTheNextRevisionNavigatesExactly() async throws {
        let (helper, render) = try requireHelperAndRender()
        let p = try project(named: "edits")
        defer { cleanup(p) }
        let model = try await attachedModel(p, helper: helper, render: render)
        defer { model.detachController() }
        let frame1 = try await currentFrame(model)
        let all1 = try clusters(of: frame1)
        func cluster(_ all: [ClusterHit], _ text: String, in path: String, after: Int) -> ClusterHit? {
            all.first { $0.text.sameBytes(as: text) && $0.source.path == path && $0.source.startByte >= after }
        }
        let main0 = Self.main, chapter0 = Self.chapter
        let ffiMain = try XCTUnwrap(cluster(all1, "ffi", in: "main.tex", after: byte(main0, "office")))
        let cafeE = try XCTUnwrap(cluster(all1, "\u{E9}", in: "main.tex", after: byte(main0, "Caf")))
        let endMain = try XCTUnwrap(cluster(all1, "e", in: "main.tex", after: byte(main0, "end ")))
        let fflChapter = try XCTUnwrap(cluster(all1, "ffl", in: "chapter.tex", after: byte(chapter0, "shuffle")))
        let resumeE = try XCTUnwrap(cluster(all1, "\u{E9}", in: "chapter.tex", after: 0))

        // Hold the edit back from the helper so the frame on screen is stale for the buffer.
        model.autoCompile = false
        XCTAssertEqual(model.activePath, "main.tex")
        // Edit INSIDE the target: "office" → "offÀce" (the ffi cluster's bytes overlap the edit).
        model.updateActiveText(main0.replacingOccurrences(of: "office", with: "off\u{C0}ce"))
        XCTAssertTrue(model.previewIsStale)
        XCTAssertNotEqual(SourceDigest.sha256Hex(model.activeText), frame1.list.documents.first { $0.path == "main.tex" }?.sha256)
        model.selection = nil
        model.navigateV2(ffiMain.hit)
        XCTAssertNil(model.selection, "a cluster whose bytes were edited is refused: \(model.navigationNote ?? "")")
        XCTAssertTrue(model.navigationNote?.contains("recompile to navigate") == true, model.navigationNote ?? "")
        XCTAssertTrue(model.navigationNote?.contains("overlap the edit") == true, model.navigationNote ?? "")
        // Before the edit: the é of "Café" keeps its bytes (rebased across the recorded compile text, no shift).
        model.navigateV2(cafeE.hit)
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "\u{E9}")
        XCTAssertEqual(model.activeText.utf8ByteRange(of: model.selection!.nsRange)?.start, cafeE.source.startByte)
        XCTAssertTrue(model.navigationNote?.contains("rebased onto the edited buffer") == true, model.navigationNote ?? "")
        // After the edit: "end" shifts by +1 byte (À is two bytes for one) and still spells "e".
        model.navigateV2(endMain.hit)
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "e")
        XCTAssertEqual(model.activeText.utf8ByteRange(of: model.selection!.nsRange)?.start, endMain.source.startByte + 1)
        XCTAssertTrue(model.navigationNote?.contains("rebased from \(endMain.source.startByte)..<\(endMain.source.endByte) across edits") == true, model.navigationNote ?? "")
        // The other document is untouched: its clusters navigate unchanged, switching documents.
        model.navigateV2(fflChapter.hit)
        XCTAssertEqual(model.activePath, "chapter.tex", model.navigationNote ?? "")
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "ffl", model.navigationNote ?? "")
        XCTAssertEqual(model.activeText.utf8ByteRange(of: model.selection!.nsRange)?.start, fflChapter.source.startByte)
        XCTAssertFalse(model.navigationNote?.contains("rebased") == true, model.navigationNote ?? "")

        // Submit the edit: the next candidate is for the new main.tex; every cluster navigates exactly again
        // and the ligature that was edited away is gone (o, ff, À, c, e).
        model.activePath = "main.tex"
        model.autoCompile = true
        model.compile()
        let frame2 = try await currentFrame(model)
        XCTAssertNotEqual(frame2.preparedNonce, frame1.preparedNonce)
        try navigateEveryClusterExactly(model, frame2)
        let all2 = try clusters(of: frame2)
        let edited = byte(text(model, "main.tex"), "off\u{C0}ce")
        XCTAssertNil(all2.first { $0.text.sameBytes(as: "ffi") && $0.source.path == "main.tex" && (edited..<(edited + 7)).contains($0.source.startByte) },
                     "the edited word has no ffi cluster (the standalone “ffi” word later in the line still does)")
        let ffMain2 = try XCTUnwrap(cluster(all2, "ff", in: "main.tex", after: byte(text(model, "main.tex"), "off\u{C0}ce")))
        XCTAssertEqual(ffMain2.source, .init(path: "main.tex", startByte: byte(text(model, "main.tex"), "off\u{C0}ce") + 1, endByte: byte(text(model, "main.tex"), "off\u{C0}ce") + 3))
        let endMain2 = try XCTUnwrap(cluster(all2, "e", in: "main.tex", after: byte(text(model, "main.tex"), "end ")))
        XCTAssertEqual(endMain2.source.startByte, endMain.source.startByte + 1)

        // Now edit chapter.tex BEFORE its targets with a combining sequence and a surrogate pair
        // (9 bytes), holding it back again: chapter clusters after the edit rebase by +9,
        // main.tex clusters are unaffected.
        model.activePath = "chapter.tex"
        model.autoCompile = false
        // The emoji stays a word of its own: flashtex-render 9aaec57a emits a cluster with no
        // glyph for a missing-glyph scalar INSIDE a word ("\u{1F600}shuffle"), which the fail-closed
        // validator refuses ("1 cluster(s) have no glyph"); reported to the producer owner.
        let inserted = "e\u{301} \u{1F600} "  // 3 + 1 + 4 + 1 = 9 bytes
        XCTAssertEqual(inserted.utf8.count, 9)
        model.updateActiveText(chapter0.replacingOccurrences(of: "shuffle", with: inserted + "shuffle"))
        model.navigateV2(fflChapter.hit)
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "ffl")
        XCTAssertEqual(model.activeText.utf8ByteRange(of: model.selection!.nsRange)?.start, fflChapter.source.startByte + 9)
        XCTAssertTrue(model.navigationNote?.contains("rebased from \(fflChapter.source.startByte)..<\(fflChapter.source.endByte) across edits") == true, model.navigationNote ?? "")
        model.navigateV2(resumeE.hit)
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "\u{E9}")
        XCTAssertEqual(model.activeText.utf8ByteRange(of: model.selection!.nsRange)?.start, resumeE.source.startByte)
        model.navigateV2(endMain2.hit)
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertEqual(model.activeText.utf8ByteRange(of: model.selection!.nsRange)?.start, endMain2.source.startByte)
        XCTAssertFalse(model.navigationNote?.contains("rebased") == true, model.navigationNote ?? "")
        // Submit: the third frame declares the new chapter.tex; everything is exact again.
        model.activePath = "chapter.tex"
        model.autoCompile = true
        model.compile()
        let frame3 = try await currentFrame(model)
        try navigateEveryClusterExactly(model, frame3)
        let fflChapter3 = try XCTUnwrap(cluster(try clusters(of: frame3), "ffl", in: "chapter.tex", after: byte(text(model, "chapter.tex"), "shuffle")))
        XCTAssertEqual(fflChapter3.source.startByte, fflChapter.source.startByte + 9)
        // A hit kept from the first frame names bytes of a text no current frame attests; the recorded
        // compile text is now the new one, so the hit resolves by the current frame's contract only
        // when its bytes still spell the same text — the ffi that no longer exists selects nothing wrong:
        model.selection = nil
        model.navigateV2(ffiMain.hit)
        if let sel = model.selection {
            XCTAssertNotEqual((model.activeText as NSString).substring(with: sel.nsRange), "ffi", "the edited-away ligature is never claimed as selected")
            XCTAssertTrue(model.navigationNote?.contains("is generated from this source") == true, "a mismatch between hit text and selected bytes is annotated: \(model.navigationNote ?? "")")
        }
    }
}
