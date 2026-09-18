import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
@testable import FlashTeXMac

/// Moving a project file (drag-and-drop in the tree / Move to…):
/// the six-command reference scan, the rewrite rules, the drop-target
/// decisions, and the model operation end to end (file moved, member
/// retargeted, open buffers edited through the reviewed edit path, closed
/// closure documents rewritten on disk). Hermetic: temp directories.
@MainActor
final class ProjectMoveTests: XCTestCase {
    private var tmp: URL!

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("move-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func model() -> ShellModel { let m = ShellModel(); m.detachWorker(); return m }

    private func expect<T: Equatable>(_ actual: T, _ expected: T, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(actual, expected, file: file, line: line)
    }

    private func openProject(_ m: ShellModel, main: String, extra: [String: String] = [:]) throws -> URL {
        let root = tmp.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let entry = root.appendingPathComponent("main.tex")
        try main.write(to: entry, atomically: true, encoding: .utf8)
        for (path, text) in extra {
            let url = root.appendingPathComponent(path)
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try text.write(to: url, atomically: true, encoding: .utf8)
        }
        XCTAssertEqual(m.openTex(at: entry), .opened)
        return root
    }

    // MARK: pure: scan and rewrite

    func testFileReferencesScanCoversTheSixCommands() {
        let text = """
        \\input{sections/a} \\include{sections/a.tex}
        \\includegraphics[width=0.5\\textwidth,
          trim=0 0 0 0]{fig/plot}
        \\bibliography{refs, extra/more}
        \\addbibresource[datatype=bibtex]{refs.bib}
        \\lstinputlisting[language=C]{code/main.c}
        % \\input{commented}
        \\verb|\\input{verb}| \\begin{verbatim}\\input{v}\\end{verbatim}
        \\inputx{notacommand} \\input{\\jobname}
        """
        let refs = FileReferences.scan(text)
        XCTAssertEqual(refs.map(\.command), ["input", "include", "includegraphics", "bibliography", "bibliography", "addbibresource", "lstinputlisting", "input"])
        XCTAssertEqual(refs.map(\.argument), ["sections/a", "sections/a.tex", "fig/plot", "refs", "extra/more", "refs.bib", "code/main.c", "\\jobname"])
        XCTAssertEqual(refs.map(\.literal), [true, true, true, true, true, true, true, false])
        let bytes = Array(text.utf8)
        for ref in refs {
            XCTAssertEqual(String(decoding: bytes[ref.argumentStartByte..<ref.argumentEndByte], as: UTF8.self), ref.argument, "argument span is exact")
        }
    }

    func testRewriteResolvesWithAndWithoutExtensionAndDotSlash() {
        // `.tex` without the extension stays bare; with it, keeps it; `./` is kept.
        XCTAssertEqual(FileReferences.rewrite(argument: "sections/a", oldPath: "sections/a.tex", newPath: "chapters/a.tex"), "chapters/a")
        XCTAssertEqual(FileReferences.rewrite(argument: "sections/a.tex", oldPath: "sections/a.tex", newPath: "chapters/a.tex"), "chapters/a.tex")
        XCTAssertEqual(FileReferences.rewrite(argument: "./sections/a", oldPath: "sections/a.tex", newPath: "chapters/a.tex"), "./chapters/a")
        XCTAssertEqual(FileReferences.rewrite(argument: "./sections/a.tex", oldPath: "sections/a.tex", newPath: "a.tex"), "./a.tex")
        // Graphics and bibliographies: any extension, same rules.
        XCTAssertEqual(FileReferences.rewrite(argument: "fig/plot", oldPath: "fig/plot.pdf", newPath: "img/plot.pdf"), "img/plot")
        XCTAssertEqual(FileReferences.rewrite(argument: "fig/plot.pdf", oldPath: "fig/plot.pdf", newPath: "img/plot.pdf"), "img/plot.pdf")
        XCTAssertEqual(FileReferences.rewrite(argument: "refs", oldPath: "refs.bib", newPath: "bib/refs.bib"), "bib/refs")
        // Not this file: another name, a different extension, a sibling with the same prefix.
        XCTAssertNil(FileReferences.rewrite(argument: "sections/b", oldPath: "sections/a.tex", newPath: "chapters/a.tex"))
        XCTAssertNil(FileReferences.rewrite(argument: "fig/plot.png", oldPath: "fig/plot.pdf", newPath: "img/plot.pdf"))
        XCTAssertNil(FileReferences.rewrite(argument: "sections/a2", oldPath: "sections/a.tex", newPath: "chapters/a.tex"))
        XCTAssertNil(FileReferences.rewrite(argument: "../sections/a", oldPath: "sections/a.tex", newPath: "chapters/a.tex"), "escaping paths never match")
        // A folder move rewrites everything under it.
        XCTAssertEqual(FileReferences.rewrite(argument: "sections/a", oldPath: "sections", newPath: "chapters"), "chapters/a")
    }

    func testPlanMoveGroupsOneEditPerDocumentAcrossAllSixCommands() {
        let docs: [RuntimeV1.Document] = [
            .init(path: "main.tex", text: "\\input{sections/a} % \\input{sections/a}\n\\input{./sections/a.tex}\n\\includegraphics{sections/a}\n"),
            .init(path: "notes.tex", text: "\\bibliography{sections/a,other} \\addbibresource{x.bib} \\lstinputlisting{sections/a.tex}\n"),
            .init(path: "other.tex", text: "nothing\n"),
        ]
        let edits = ReferenceRewrite.planMove(oldPath: "sections/a.tex", newPath: "chapters/a.tex", documents: docs)
        XCTAssertEqual(edits.map(\.path), ["main.tex", "notes.tex"])
        XCTAssertEqual(edits.map(\.count), [3, 2])
        guard edits.count == 2 else { return XCTFail("expected two edits, got \(edits.count)") }
        XCTAssertEqual((docs[0].text as NSString).replacingCharacters(in: edits[0].nsRange, with: edits[0].text),
                       "\\input{chapters/a} % \\input{sections/a}\n\\input{./chapters/a.tex}\n\\includegraphics{chapters/a}\n")
        XCTAssertEqual((docs[1].text as NSString).replacingCharacters(in: edits[1].nsRange, with: edits[1].text),
                       "\\bibliography{chapters/a,other} \\addbibresource{x.bib} \\lstinputlisting{chapters/a.tex}\n")
        XCTAssertEqual((docs[0].text as NSString).replacingCharacters(in: edits[0].nsRange, with: edits[0].before), docs[0].text, "`before` reverts the edit exactly")
        XCTAssertEqual(ReferenceRewrite.planMove(oldPath: "none.tex", newPath: "x.tex", documents: docs), [])
    }

    // MARK: pure: targets

    func testMoveTargetRefusesSelfDescendantSameFolderAndEntryName() {
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: "chapters"), .success("chapters/a.tex"))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: ""), .success("a.tex"))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: " ./chapters/ "), .success("chapters/a.tex"))
        XCTAssertEqual(MoveTarget.resolve(path: "a.tex", intoFolder: "."), .failure(.sameFolder))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: "sections"), .failure(.sameFolder))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: "sections/a.tex"), .failure(.ontoSelf))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: "sections/a.tex/deeper"), .failure(.intoDescendant))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/main.tex", intoFolder: ""), .failure(.isEntry))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: "../out"), .failure(.invalid("path escapes the project root via '..'")))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: "/abs"), .failure(.invalid("path must be project-relative, not absolute")))
    }

    func testProjectTreeRowsMapToDragSourcesAndDropFolders() {
        // The tree is flat: a file row is its folder, the background the root.
        XCTAssertEqual(ProjectTreeMove.path(forRowID: "sections/a.tex"), "sections/a.tex")
        XCTAssertEqual(ProjectTreeMove.path(forRowID: "closed:chapters/b.tex"), "chapters/b.tex")
        XCTAssertNil(ProjectTreeMove.path(forRowID: "missing:chapters/c:main.tex"))
        XCTAssertNil(ProjectTreeMove.path(forRowID: "outline:empty"))
        XCTAssertEqual(ProjectTreeMove.dropFolder(rowID: nil), "")
        XCTAssertEqual(ProjectTreeMove.dropFolder(rowID: "chapters/b.tex"), "chapters")
        XCTAssertEqual(ProjectTreeMove.dropFolder(rowID: "closed:deep/er/c.tex"), "deep/er")
        XCTAssertEqual(ProjectTreeMove.dropFolder(rowID: "main.tex"), "")
        XCTAssertNil(ProjectTreeMove.dropFolder(rowID: "missing:x:main.tex"))
        // A drop on a row of the same folder (or on itself) resolves to a refusal, so the tree shows no target.
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: ProjectTreeMove.dropFolder(rowID: "sections/a.tex")!), .failure(.sameFolder))
        XCTAssertEqual(MoveTarget.resolve(path: "sections/a.tex", intoFolder: ProjectTreeMove.dropFolder(rowID: "chapters/b.tex")!), .success("chapters/a.tex"))
    }

    // MARK: model

    func testMoveRelocatesTheFileRewritesOpenAndDiskReferencesAndTheEditReverts() async throws {
        let m = model()
        let root = try openProject(m, main: "\\begin{document}\n\\input{sections/a}\n\\input{sections/a.tex}\n\\input{notes}\n\\end{document}\n",
                                   extra: ["sections/a.tex": "A.\n",
                                           "notes.tex": "See \\input{./sections/a} and \\includegraphics{fig/x}.\n",
                                           "fig/x.pdf": "%PDF\n"])
        // Only the moved file is open; notes.tex stays a closed closure document.
        guard case .opened = await m.project.openDocument("sections/a.tex", role: .included(from: "main.tex")) else { return XCTFail("open") }
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "sections/a.tex"])
        m.project.switchDocument(to: "sections/a.tex")

        // Refused: the entry, unsaved edits, its own folder, a descendant, an existing name.
        await expect(m.project.moveDocument("main.tex", intoFolder: "chapters"), .refused("cannot move main.tex: main.tex is the entry document"))
        m.updateActiveText("A, edited.\n")
        await expect(m.project.moveDocument("sections/a.tex", intoFolder: "chapters"), .refused("cannot move sections/a.tex: sections/a.tex has unsaved edits; save them first (⌘S)"))
        m.updateActiveText("A.\n")
        await expect(m.project.moveDocument("sections/a.tex", intoFolder: "sections"), .refused("cannot move sections/a.tex: it is already in that folder"))
        await expect(m.project.moveDocument("sections/a.tex", intoFolder: "sections/a.tex/x"), .refused("cannot move sections/a.tex: cannot move a file into a folder under it"))
        await expect(m.project.moveDocument("sections/a.tex", intoFolder: "../out"), .refused("cannot move sections/a.tex: path escapes the project root via '..'"))
        try FileManager.default.createDirectory(at: root.appendingPathComponent("chapters"), withIntermediateDirectories: true)
        try "taken".write(to: root.appendingPathComponent("chapters/a.tex"), atomically: true, encoding: .utf8)
        await expect(m.project.moveDocument("sections/a.tex", intoFolder: "chapters"), .refused("cannot move sections/a.tex: chapters/a.tex already exists"))
        try FileManager.default.removeItem(at: root.appendingPathComponent("chapters/a.tex"))
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent("sections/a.tex").path), "nothing moved on a refusal")
        // A referencing open document with unsaved edits blocks the move.
        m.project.switchDocument(to: "main.tex")
        m.updateActiveText("\\begin{document}\n\\input{sections/a}\n\\input{sections/a.tex}\n\\input{notes}\nedited\n\\end{document}\n")
        await expect(m.project.moveDocument("sections/a.tex", intoFolder: "chapters"), .refused("cannot move sections/a.tex: main.tex references it and has unsaved edits; save it first"))
        m.updateActiveText("\\begin{document}\n\\input{sections/a}\n\\input{sections/a.tex}\n\\input{notes}\n\\end{document}\n")
        m.project.switchDocument(to: "sections/a.tex")

        let outcome = await m.project.moveDocument("sections/a.tex", intoFolder: "chapters")
        XCTAssertEqual(outcome, .moved(from: "sections/a.tex", to: "chapters/a.tex", references: 3, open: ["main.tex"], onDisk: ["notes.tex"]))
        XCTAssertEqual(m.project.status, "moved sections/a.tex → chapters/a.tex; 3 references updated in main.tex (⌘Z reverts the edit in each document; the file itself stays moved); on disk in notes.tex")
        // The file moved and the member followed it.
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("sections/a.tex").path))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("chapters/a.tex"), encoding: .utf8), "A.\n")
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "chapters/a.tex"])
        XCTAssertEqual(m.project.listing[1].role, .included(from: "main.tex"), "metadata followed the move")
        XCTAssertFalse(m.project.isDirty("chapters/a.tex"))
        // The closed closure document was rewritten on disk; the figure reference was left alone.
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("notes.tex"), encoding: .utf8), "See \\input{./chapters/a} and \\includegraphics{fig/x}.\n")
        // The open referencing document gets one grouped, undoable edit
        // (posted once the editor has swapped to it): both spellings rewritten.
        XCTAssertEqual(m.activePath, "main.tex")
        try await Task.sleep(for: .milliseconds(150))
        let edit = try XCTUnwrap(m.pendingEdit)
        XCTAssertEqual(edit.path, "main.tex")
        XCTAssertEqual(edit.revision, m.editorRevision)
        let before = m.activeText
        m.editApplied(edit, newText: (m.activeText as NSString).replacingCharacters(in: edit.nsRange, with: edit.text))
        XCTAssertEqual(m.activeText, "\\begin{document}\n\\input{chapters/a}\n\\input{chapters/a.tex}\n\\input{notes}\n\\end{document}\n")
        XCTAssertNil(m.pendingEdit, "no further open document references it")
        XCTAssertEqual(m.project.discoverClosure().nodes.map(\.state), [.open, .open, .available, .open],
                       "main.tex resolves the moved file twice; notes.tex is still closed and its rewritten reference resolves too")
        // Undo of that one edit (what ⌘Z does in the editor) restores the references exactly.
        let reverted = (m.activeText as NSString).replacingCharacters(in: NSRange(location: edit.nsRange.location, length: (edit.text as NSString).length), with: before.substring(edit.nsRange))
        XCTAssertEqual(reverted, before)
        // The moved member saves to its new file (disk baseline followed).
        m.project.switchDocument(to: "chapters/a.tex")
        m.updateActiveText("A, moved.\n")
        XCTAssertEqual(m.project.saveDocumentNow("chapters/a.tex"), .saved(path: "chapters/a.tex", sha256: SourceDigest.sha256Hex("A, moved.\n")))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("chapters/a.tex"), encoding: .utf8), "A, moved.\n")
    }

    func testMoveToTheRootAndOfAClosedIncludeAndTheSheetDefaults() async throws {
        let m = model()
        let root = try openProject(m, main: "\\begin{document}\n\\input{sections/a}\n\\end{document}\n",
                                   extra: ["sections/a.tex": "A.\n"])
        XCTAssertEqual(m.documents.map(\.path), ["main.tex"], "sections/a.tex is a closed include")
        // The sheet defaults to the file's current folder, so the field shows where it is now.
        m.scaffold.presentMove("sections/a.tex")
        XCTAssertEqual(m.scaffold.sheet, .move(path: "sections/a.tex"))
        XCTAssertEqual(m.scaffold.moveTo, "sections")
        m.scaffold.moveTo = ""
        await m.scaffold.move("sections/a.tex")
        XCTAssertNil(m.scaffold.sheet)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("a.tex"), encoding: .utf8), "A.\n")
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("sections/a.tex").path))
        try await Task.sleep(for: .milliseconds(100))
        let edit = try XCTUnwrap(m.pendingEdit)
        XCTAssertEqual(edit.path, "main.tex")
        m.editApplied(edit, newText: (m.activeText as NSString).replacingCharacters(in: edit.nsRange, with: edit.text))
        XCTAssertEqual(m.activeText, "\\begin{document}\n\\input{a}\n\\end{document}\n")
        // The entry is not movable from the sheet either.
        m.scaffold.presentMove("main.tex")
        XCTAssertNil(m.scaffold.sheet)
        XCTAssertEqual(m.navigationNote, "Cannot move main.tex: main.tex is the entry document")
    }

    func testMoveFileIsACommandWithoutAKeyEquivalent() {
        XCTAssertEqual(AccessibilityCommand.moveFile.entry.shortcuts, ["File > Move To…"])
        XCTAssertEqual(AccessibilityCommand.moveFile.entry.menuItem, "Move To…")
        XCTAssertTrue(CommandPaletteModel.isRunnable(.moveFile))
        XCTAssertTrue(CommandPaletteModel.rows(matching: "move").map(\.id).contains(.moveFile))
    }
}

private extension String {
    func substring(_ range: NSRange) -> String { (self as NSString).substring(with: range) }
}
