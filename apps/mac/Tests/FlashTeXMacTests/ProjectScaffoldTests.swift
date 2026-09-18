import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
@testable import FlashTeXMac

/// Creating a project from scratch (lane `mac-new-project`): templates,
/// rooted New File names, insert-at-caret, the missing-include quick fix,
/// rename with reference rewrite, delete to Trash. Hermetic: temp
/// directories, no panels (the sheet state's providers are injected).
@MainActor
final class ProjectScaffoldTests: XCTestCase {
    private var tmp: URL!

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("scaffold-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func model() -> ShellModel { let m = ShellModel(); m.detachWorker(); return m }

    /// `XCTAssertEqual` for an async outcome (autoclosures cannot await).
    private func expect<T: Equatable>(_ actual: T, _ expected: T, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(actual, expected, file: file, line: line)
    }

    // MARK: templates

    func testEveryTemplateWritesMainAndItsIncludeTree() throws {
        for t in ProjectTemplate.allCases {
            let created = try ProjectScaffold.create(in: tmp, name: "proj-\(t.rawValue)", template: t).get()
            XCTAssertEqual(created.entry.lastPathComponent, "main.tex")
            XCTAssertEqual(created.written.first, "main.tex", t.rawValue)
            let main = try String(contentsOf: created.entry, encoding: .utf8)
            XCTAssertTrue(main.contains("\\begin{document}") && main.contains("\\end{document}"), t.rawValue)
            // Every reference the entry makes is a file the template wrote.
            let refs = ProjectIncludes.scan(main)
            for r in refs {
                let path = try XCTUnwrap(ProjectIncludes.candidates(for: r.argument).first)
                XCTAssertTrue(created.written.contains(path), "\(t.rawValue): \\\(r.kind.rawValue){\(r.argument)} → \(path) written")
                XCTAssertTrue(FileManager.default.fileExists(atPath: created.root.appendingPathComponent(path).path))
            }
            XCTAssertEqual(refs.count, created.written.count - 1, "\(t.rawValue): one reference per member")
            XCTAssertTrue(main.contains("\\title{proj-\(t.rawValue)}") || t == .homeworkSheet, t.rawValue)
        }
        XCTAssertEqual(ProjectTemplate.articleWithSections.files(projectName: "x").map(\.path), ["main.tex", "sections/introduction.tex", "sections/methods.tex"])
        XCTAssertEqual(ProjectTemplate.reportWithChapters.files(projectName: "x").map(\.path), ["main.tex", "chapters/introduction.tex", "chapters/background.tex"])
        XCTAssertTrue(ProjectTemplate.articleWithSections.files(projectName: "x")[0].text.contains("\\input{sections/introduction}"))
        XCTAssertTrue(ProjectTemplate.reportWithChapters.files(projectName: "x")[0].text.contains("\\include{chapters/introduction}"))
        let hw = ProjectTemplate.homeworkSheet.files(projectName: "HW 1")[0].text
        for needle in ["geometry", "amsmath,amssymb", "enumitem", "\\newcommand{\\problem}[2]", "\\problem{1}{10}"] { XCTAssertTrue(hw.contains(needle), needle) }
        XCTAssertEqual(ProjectTemplate.escapeTitle("A & B_1 100%"), "A \\& B\\_1 100\\%")
        XCTAssertTrue(ProjectTemplate.blankArticle.files(projectName: "A & B")[0].text.contains("\\title{A \\& B}"))
    }

    func testCreateRefusesBadNamesAndExistingFilesUnlessConfirmed() throws {
        XCTAssertEqual(ProjectScaffold.create(in: tmp, name: "  ", template: .blankArticle), .failure(.badName("empty")))
        XCTAssertEqual(ProjectScaffold.create(in: tmp, name: "a/b", template: .blankArticle), .failure(.badName("no path separators")))
        XCTAssertEqual(ProjectScaffold.create(in: tmp, name: "..", template: .blankArticle), .failure(.badName("'..' is not a folder name")))
        // A folder with other files is fine; the template's own files are the conflict.
        let folder = tmp.appendingPathComponent("paper")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try "notes".write(to: folder.appendingPathComponent("notes.md"), atomically: true, encoding: .utf8)
        XCTAssertEqual(ProjectScaffold.conflicts(in: tmp, name: "paper", template: .articleWithSections), [])
        _ = try ProjectScaffold.create(in: tmp, name: "paper", template: .articleWithSections).get()
        try "mine".write(to: folder.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        XCTAssertEqual(ProjectScaffold.conflicts(in: tmp, name: "paper", template: .articleWithSections), ["main.tex", "sections/introduction.tex", "sections/methods.tex"])
        XCTAssertEqual(ProjectScaffold.create(in: tmp, name: "paper", template: .articleWithSections),
                       .failure(.wouldOverwrite(["main.tex", "sections/introduction.tex", "sections/methods.tex"])))
        XCTAssertEqual(try String(contentsOf: folder.appendingPathComponent("main.tex"), encoding: .utf8), "mine", "nothing written on refusal")
        XCTAssertEqual(try String(contentsOf: folder.appendingPathComponent("notes.md"), encoding: .utf8), "notes")
        _ = try ProjectScaffold.create(in: tmp, name: "paper", template: .articleWithSections, overwrite: true).get()
        XCTAssertNotEqual(try String(contentsOf: folder.appendingPathComponent("main.tex"), encoding: .utf8), "mine")
        XCTAssertEqual(try String(contentsOf: folder.appendingPathComponent("notes.md"), encoding: .utf8), "notes", "other files untouched")
    }

    func testNewProjectSheetCreatesOpensEntryAndShowsIncludeTree() async throws {
        let m = model()
        let state = m.scaffold
        var chosen = 0
        state.chooseFolder = { [tmp] in chosen += 1; return tmp }
        state.confirmOverwrite = { _ in XCTFail("no conflict expected"); return false }
        XCTAssertEqual(state.createProject(), .refused("no folder"))
        state.folder = state.chooseFolder()
        XCTAssertEqual(chosen, 1)
        state.projectName = "thesis"
        state.template = .reportWithChapters
        let outcome = state.createProject()
        XCTAssertEqual(outcome, .opened(tmp.appendingPathComponent("thesis/main.tex")))
        XCTAssertNil(state.sheet)
        XCTAssertEqual(m.documentURL?.standardizedFileURL, tmp.appendingPathComponent("thesis/main.tex").standardizedFileURL)
        XCTAssertEqual(m.project.entryPath, "main.tex")
        XCTAssertFalse(m.isDirty)
        // Discovery sees both chapters immediately; the sheet also opens them.
        let closure = m.project.discoverClosure()
        XCTAssertEqual(closure.paths, ["chapters/introduction.tex", "chapters/background.tex"])
        try await Task.sleep(for: .milliseconds(200))
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "chapters/introduction.tex", "chapters/background.tex"])
        XCTAssertEqual(m.project.listing.map(\.role), [.entry, .included(from: "main.tex"), .included(from: "main.tex")])
        // The overwrite confirmation is asked, and a "no" writes nothing.
        var asked: [String] = []
        state.confirmOverwrite = { asked = $0; return false }
        XCTAssertEqual(state.createProject(), .cancelled)
        XCTAssertEqual(asked, ["main.tex", "chapters/introduction.tex", "chapters/background.tex"])
    }

    // MARK: new file names

    func testNewFilePathIsRootedAndGetsTheTexExtension() {
        XCTAssertEqual(NewFilePath.resolve("results"), .success("results.tex"))
        XCTAssertEqual(NewFilePath.resolve(" sections/results "), .success("sections/results.tex"))
        XCTAssertEqual(NewFilePath.resolve("sections/results.tex"), .success("sections/results.tex"))
        XCTAssertEqual(NewFilePath.resolve("./a//b"), .success("a/b.tex"))
        XCTAssertEqual(NewFilePath.resolve("../escape"), .failure(.invalid("path escapes the project root via '..'")))
        XCTAssertEqual(NewFilePath.resolve("/abs"), .failure(.invalid("path must be project-relative, not absolute")))
        XCTAssertEqual(NewFilePath.resolve("main"), .failure(.isEntry))
        XCTAssertEqual(NewFilePath.resolve("main.tex"), .failure(.isEntry))
        XCTAssertEqual(NewFilePath.resolve(""), .failure(.invalid("path is empty")))
        XCTAssertEqual(NewFilePath.resolve("dir/.tex"), .failure(.invalid("empty file name")))
        XCTAssertEqual(NewFilePath.inputArgument(for: "sections/a.tex"), "sections/a")
        XCTAssertEqual(NewFilePath.referenceText(for: "sections/a.tex", into: "x\n", atByte: 2), "\\input{sections/a}")
        XCTAssertEqual(NewFilePath.referenceText(for: "a.tex", into: "xy", atByte: 1), "\n\\input{a}\n")
        XCTAssertEqual(NewFilePath.referenceText(for: "a.tex", kind: .include, into: "", atByte: 0), "\\include{a}")
    }

    private func openProject(_ m: ShellModel, main: String = "\\begin{document}\nMain.\n\\end{document}\n", extra: [String: String] = [:]) throws -> URL {
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

    func testNewFileWritesOpensAndInsertsInputAtTheCaret() async throws {
        let m = model()
        let root = try openProject(m)
        // Caret at the end of "Main." (line 2).
        let text = m.activeText
        m.caretUTF16 = (text as NSString).range(of: "Main.").upperBound
        let outcome = await m.project.newFile("sections/results", insertReference: true)
        XCTAssertEqual(outcome, .created(path: "sections/results.tex"))
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.appendingPathComponent("sections/results.tex").path))
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "sections/results.tex"])
        guard m.project.listing.count > 1 else { return XCTFail("expected more than one listing entry, got \(m.project.listing.count)") }
        XCTAssertEqual(m.project.listing[1].role, .included(from: "main.tex"))
        // One undoable edit for the entry document, posted for the editor; the
        // switch to the new tab waits for the editor to apply it.
        let edit = try XCTUnwrap(m.pendingEdit)
        XCTAssertEqual(edit.path, "main.tex")
        XCTAssertEqual(edit.text, "\n\\input{sections/results}")
        XCTAssertEqual(edit.nsRange, NSRange(location: m.caretUTF16, length: 0))
        XCTAssertEqual(edit.revision, m.editorRevision)
        XCTAssertEqual(m.activePath, "main.tex", "not switched while the reference edit is pending")
        // The editor applies it → editApplied → the switch happens.
        let applied = (m.activeText as NSString).replacingCharacters(in: edit.nsRange, with: edit.text)
        m.editApplied(edit, newText: applied)
        XCTAssertNil(m.pendingEdit)
        XCTAssertEqual(m.activeText, "\\begin{document}\nMain.\n\\input{sections/results}\n\\end{document}\n")
        try await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual(m.activePath, "sections/results.tex")
        XCTAssertEqual(m.project.discoverIncludes().map(\.state), [.open])
        // Refusals: existing, entry, escaping, unsaved root.
        await expect(m.project.newFile("sections/results", insertReference: false), .refused("cannot create sections/results.tex: it is already open"))
        await expect(m.project.newFile("main", insertReference: false), .refused("cannot create main: main.tex is the entry document"))
        await expect(m.project.newFile("../x", insertReference: false), .refused("cannot create ../x: path escapes the project root via '..'"))
        try "x".write(to: root.appendingPathComponent("exists.tex"), atomically: true, encoding: .utf8)
        await expect(m.project.newFile("exists", insertReference: false), .refused("cannot create exists.tex: it already exists (open it instead)"))
        let unsaved = model()
        await expect(unsaved.project.newFile("a", insertReference: false), .refused("cannot create a.tex: the entry document is not saved, so there is no project root; save it first (⌘S)"))
        unsaved.scaffold.presentNewFile()
        XCTAssertNil(unsaved.scaffold.sheet)
        XCTAssertEqual(unsaved.navigationNote, "Save the entry document first (⌘S): a new file needs a project folder to live in.")
    }

    func testNewFileWithoutReferenceSwitchesAtOnceAndTheSheetDefaultsFollowTheActiveDocument() async throws {
        let m = model()
        _ = try openProject(m)
        m.scaffold.presentNewFile()
        XCTAssertEqual(m.scaffold.sheet, .newFile)
        XCTAssertTrue(m.scaffold.insertReference, "entry active: insert on by default")
        await expect(m.project.newFile("appendix", insertReference: false), .created(path: "appendix.tex"))
        XCTAssertNil(m.pendingEdit)
        XCTAssertEqual(m.activePath, "appendix.tex")
        XCTAssertEqual(m.project.listing[1].role, .opened)
        m.scaffold.presentNewFile()
        XCTAssertFalse(m.scaffold.insertReference, "a non-entry document is active: insert off by default")
    }

    // MARK: missing includes

    func testMissingIncludeIsParsedFromTheCompilerMessageOnly() {
        XCTAssertEqual(MissingIncludeFix.requested(from: "included file not found: looked for 'chapters/two' and 'chapters/two.tex'"), "chapters/two")
        XCTAssertNil(MissingIncludeFix.requested(from: "undefined control sequence \\foo"))
        XCTAssertNil(MissingIncludeFix.requested(from: "included file not found: looked for '' and '.tex'"))
        XCTAssertEqual(MissingIncludeFix.path(for: "two"), "two.tex")
        XCTAssertEqual(MissingIncludeFix.path(for: "two.tex"), "two.tex")
        XCTAssertNil(MissingIncludeFix.path(for: "../two"))
    }

    func testMissingIncludeQuickFixAndSidebarRowCreateTheFile() async throws {
        let m = model()
        let root = try openProject(m, main: "\\begin{document}\n\\input{chapters/two}\n\\end{document}\n")
        // Sidebar: the reference is unresolvable because the file is missing.
        let nodes = m.project.discoverClosure().nodes
        XCTAssertEqual(nodes.count, 1)
        guard let firstNode = nodes.first else { return XCTFail("expected one node") }
        XCTAssertEqual(firstNode.state, .unresolvable("no such file under the project root"))
        // Quick fix on the compiler's diagnostic (deterministic message, source names the document).
        let d = RuntimeV1.Diagnostic(severity: .error, message: "included file not found: looked for 'chapters/two' and 'chapters/two.tex'",
                                     source: .init(path: "main.tex", startByte: 17, endByte: 36), recovery: "skipped the missing include and continued")
        XCTAssertEqual(MissingIncludeFix.quickFix(for: d, projectRoot: m.project.projectRoot),
                       .init(argument: "chapters/two", path: "chapters/two.tex", from: "main.tex"))
        XCTAssertNil(MissingIncludeFix.quickFix(for: d, projectRoot: nil), "no root, no fix")
        await expect(m.project.createMissingInclude("chapters/two", from: "main.tex"), .created(path: "chapters/two.tex"))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("chapters/two.tex"), encoding: .utf8), "")
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "chapters/two.tex"])
        guard m.project.listing.count > 1 else { return XCTFail("expected more than one listing entry, got \(m.project.listing.count)") }
        XCTAssertEqual(m.project.listing[1].role, .included(from: "main.tex"))
        XCTAssertEqual(m.project.discoverClosure().nodes.map(\.state), [.open])
        XCTAssertNil(MissingIncludeFix.quickFix(for: d, projectRoot: m.project.projectRoot), "the file exists now: no fix offered")
        await expect(m.project.createMissingInclude("../up", from: "main.tex"), .refused("cannot create a file for \\input{../up}: not a rooted path"))
    }

    // MARK: rename

    func testReferenceRewritePlansOneGroupedEditPerDocument() {
        let docs: [RuntimeV1.Document] = [
            .init(path: "main.tex", text: "\\input{ch/one} % \\input{ch/one}\n\\include{ch/one.tex}\n\\input{other}\n"),
            .init(path: "notes.tex", text: "nothing here\n"),
            .init(path: "ch/two.tex", text: "see \\input{ch/one}\n"),
        ]
        let edits = ReferenceRewrite.plan(oldPath: "ch/one.tex", newPath: "ch/uno.tex", documents: docs)
        XCTAssertEqual(edits.map(\.path), ["main.tex", "ch/two.tex"])
        XCTAssertEqual(edits.map(\.count), [2, 1])
        guard edits.count == 2 else { return XCTFail("expected two edits, got \(edits.count)") }
        XCTAssertEqual(edits[0].before, "ch/one} % \\input{ch/one}\n\\include{ch/one.tex")
        XCTAssertEqual(edits[0].text, "ch/uno} % \\input{ch/one}\n\\include{ch/uno.tex", "comment untouched; explicit .tex kept")
        XCTAssertEqual(edits[1].before, "ch/one")
        XCTAssertEqual(edits[1].text, "ch/uno")
        XCTAssertEqual((docs[0].text as NSString).replacingCharacters(in: edits[0].nsRange, with: edits[0].text),
                       "\\input{ch/uno} % \\input{ch/one}\n\\include{ch/uno.tex}\n\\input{other}\n")
        XCTAssertEqual(ReferenceRewrite.plan(oldPath: "other.tex", newPath: "x.tex", documents: docs).map(\.path), ["main.tex"])
        XCTAssertEqual(ReferenceRewrite.plan(oldPath: "none.tex", newPath: "x.tex", documents: docs), [])
    }

    func testRenameMovesTheFileRetargetsTheMemberAndPostsOneEditPerDocument() async throws {
        let m = model()
        let root = try openProject(m, main: "\\begin{document}\n\\input{ch/one}\n\\input{ch/two}\n\\end{document}\n",
                                   extra: ["ch/one.tex": "One.\n", "ch/two.tex": "Two, see \\input{ch/one}.\n"])
        _ = await m.project.openDiscoveredIncludes()
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "ch/one.tex", "ch/two.tex"])
        m.project.switchDocument(to: "ch/one.tex")
        // Refused: entry, unsaved edits, existing target, escaping, open target.
        await expect(m.project.renameDocument("main.tex", to: "x"), .refused("cannot rename main.tex: main.tex is the entry document"))
        m.updateActiveText("One, edited.\n")
        await expect(m.project.renameDocument("ch/one.tex", to: "ch/uno"), .refused("cannot rename ch/one.tex: ch/one.tex has unsaved edits; save them first (⌘S)"))
        m.updateActiveText("One.\n")
        await expect(m.project.renameDocument("ch/one.tex", to: "ch/two"), .refused("cannot rename ch/one.tex: ch/two.tex is open"))
        await expect(m.project.renameDocument("ch/one.tex", to: "../one"), .refused("cannot rename ch/one.tex: path escapes the project root via '..'"))
        try "taken".write(to: root.appendingPathComponent("ch/taken.tex"), atomically: true, encoding: .utf8)
        await expect(m.project.renameDocument("ch/one.tex", to: "ch/taken"), .refused("cannot rename ch/one.tex: ch/taken.tex already exists"))
        // A referencing document with unsaved edits blocks the rename (its buffer would be rewritten).
        m.project.switchDocument(to: "ch/two.tex")
        m.updateActiveText("Two, edited, see \\input{ch/one}.\n")
        await expect(m.project.renameDocument("ch/one.tex", to: "ch/uno"), .refused("cannot rename ch/one.tex: ch/two.tex references it and has unsaved edits; save it first"))
        m.updateActiveText("Two, see \\input{ch/one}.\n")
        m.project.switchDocument(to: "ch/one.tex")

        let outcome = await m.project.renameDocument("ch/one.tex", to: "ch/uno")
        XCTAssertEqual(outcome, .renamed(from: "ch/one.tex", to: "ch/uno.tex", references: 2))
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("ch/one.tex").path))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("ch/uno.tex"), encoding: .utf8), "One.\n")
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "ch/uno.tex", "ch/two.tex"])
        XCTAssertEqual(m.activePath, "main.tex", "the first referencing document is active for its edit")
        guard m.project.listing.count > 1 else { return XCTFail("expected more than one listing entry, got \(m.project.listing.count)") }
        XCTAssertEqual(m.project.listing[1].role, .included(from: "main.tex"), "metadata followed the rename")
        XCTAssertFalse(m.project.isDirty("ch/uno.tex"))
        // First edit: main.tex, one grouped replacement (posted once the editor
        // has swapped its text after the switch).
        try await Task.sleep(for: .milliseconds(150))
        let first = try XCTUnwrap(m.pendingEdit)
        XCTAssertEqual(first.path, "main.tex")
        XCTAssertEqual(first.text, "ch/uno")
        XCTAssertEqual(first.revision, m.editorRevision)
        m.editApplied(first, newText: (m.activeText as NSString).replacingCharacters(in: first.nsRange, with: first.text))
        XCTAssertEqual(m.activeText, "\\begin{document}\n\\input{ch/uno}\n\\input{ch/two}\n\\end{document}\n")
        // Second edit: ch/two.tex after the switch.
        try await Task.sleep(for: .milliseconds(250))
        XCTAssertEqual(m.activePath, "ch/two.tex")
        let second = try XCTUnwrap(m.pendingEdit)
        XCTAssertEqual(second.path, "ch/two.tex")
        m.editApplied(second, newText: (m.activeText as NSString).replacingCharacters(in: second.nsRange, with: second.text))
        XCTAssertEqual(m.activeText, "Two, see \\input{ch/uno}.\n")
        XCTAssertEqual(m.project.discoverClosure().nodes.map(\.state), [.open, .open, .open])
        // The renamed member saves to its new file (disk baseline followed).
        m.project.switchDocument(to: "ch/uno.tex")
        m.updateActiveText("One, renamed.\n")
        XCTAssertEqual(m.project.saveDocumentNow("ch/uno.tex"), .saved(path: "ch/uno.tex", sha256: SourceDigest.sha256Hex("One, renamed.\n")))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("ch/uno.tex"), encoding: .utf8), "One, renamed.\n")
    }

    // MARK: delete

    func testDeleteDetachesAndTrashesTheFile() async throws {
        let m = model()
        let root = try openProject(m, main: "\\begin{document}\n\\input{extra}\n\\end{document}\n", extra: ["extra.tex": "Extra.\n"])
        _ = await m.project.openDiscoveredIncludes()
        await expect(m.project.deleteDocument("main.tex"), .refused("cannot delete main.tex: main.tex is the entry document"))
        m.project.switchDocument(to: "extra.tex")
        m.updateActiveText("Extra, edited.\n")
        await expect(m.project.deleteDocument("extra.tex"), .refused("cannot delete extra.tex: extra.tex has unsaved edits; save them first (⌘S)"))
        m.updateActiveText("Extra.\n")
        m.scaffold.presentDelete("extra.tex")
        XCTAssertEqual(m.scaffold.sheet, .delete(path: "extra.tex"))
        await m.scaffold.delete("extra.tex")
        XCTAssertNil(m.scaffold.sheet)
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("extra.tex").path))
        XCTAssertEqual(m.documents.map(\.path), ["main.tex"])
        XCTAssertEqual(m.project.status, "moved extra.tex to the Trash")
        // The reference is now a "missing — create" row.
        XCTAssertEqual(m.project.discoverClosure().nodes.map(\.state), [.unresolvable("no such file under the project root")])
        await expect(m.project.deleteDocument("extra.tex"), .refused("cannot delete extra.tex: no such file under the project root"))
    }

    // MARK: commands

    func testNewProjectAndNewFileAreCommandsWithNonCollidingShortcuts() {
        XCTAssertEqual(AccessibilityCommand.newProject.entry.shortcuts, ["⌘⌥N"])
        XCTAssertEqual(AccessibilityCommand.newFile.entry.shortcuts, ["⌘N"])
        XCTAssertEqual(AccessibilityCommand.nearbyCompanion.entry.shortcuts, ["⌘⇧N"], "Nearby keeps ⌘⇧N")
        XCTAssertTrue(CommandPaletteModel.isRunnable(.newProject) && CommandPaletteModel.isRunnable(.newFile))
        XCTAssertEqual(CommandPaletteModel.rows(matching: "new project").first?.id, .newProject)
        XCTAssertTrue(CommandPaletteModel.rows(matching: "new file").map(\.id).contains(.newFile))
    }
}
