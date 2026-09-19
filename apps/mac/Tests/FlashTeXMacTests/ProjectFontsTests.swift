import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// File › Project Fonts… (ProjectFonts.swift, docs/user/project-manifest.md
/// `[fonts]`): the sheet's model reads the manifest's table, writes the
/// selection through the helper's `set_fonts` rewrite and the rooted save,
/// reads it back, sends it on the compile request, and writes nothing for
/// "Class default" everywhere without a manifest. Hermetic through the
/// `reader`/`rewriter` hooks; the real-helper case needs the built helper and
/// skips otherwise, never silently passes.
@MainActor
final class ProjectFontsTests: XCTestCase {
    private var tmp: URL!

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("project-fonts-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func write(_ rel: String, _ text: String) throws -> URL {
        let url = tmp.appendingPathComponent(rel)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// A helper `manifest` payload with the given `[fonts]` (nil: no manifest on disk).
    private func payload(root: URL, fonts: [String: String]?) -> ProjectFilesV1.Manifest {
        let table = ["text", "math", "mono", "sans"].map { role in "\"\(role)\":" + (fonts?[role].map { "\"\($0)\"" } ?? "null") }.joined(separator: ",")
        let json = """
        {"path":\(fonts == nil ? "null" : "\"\(root.path)/flashtex.toml\""),"exists":\(fonts != nil),"manifest_dir":\(fonts == nil ? "null" : "\"\(root.path)\""),
         "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null},"fonts":{\(table)},
                     "packages":{"source":"ctan","fetch":"ask","pin":{},"path":{}},"library":null},
         "warnings":[],"texinputs":[],"files":[],"diagnostics":[],"template":"[project]\\nentry = \\"main.tex\\"\\n"}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    private func project() throws -> ShellModel {
        _ = try write("main.tex", "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n")
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: hermetic (direct writes)")
        return model
    }

    // MARK: the selection (pure)

    func testSelectionReadsTheTableAndWritesOnlyTheNamedRoles() {
        var s = ProjectFontsState.Selection(manifest: nil)
        XCTAssertTrue(s.isClassDefault)
        XCTAssertEqual(s.byRole, [:])
        s[.text] = "Georgia"
        s[.math] = "  "
        s[.mono] = " Menlo "
        XCTAssertEqual(s.byRole, ["text": "Georgia", "mono": "Menlo"], "blank is Class default, names are trimmed")
        XCTAssertFalse(s.isClassDefault)
        let table = ProjectFilesV1.Manifest.Fonts(text: "Libertinus Serif", math: "Libertinus Math", mono: nil, sans: "Inter")
        let read = ProjectFontsState.Selection(manifest: table)
        XCTAssertEqual(read, .init(text: "Libertinus Serif", math: "Libertinus Math", sans: "Inter"))
        XCTAssertEqual(read.byRole, ["text": "Libertinus Serif", "math": "Libertinus Math", "sans": "Inter"])
    }

    func testPickerMatchesEveryTermAndKeepsClassDefaultFirst() {
        let families = ["Latin Modern Roman", "STIX Two Math", "Libertinus Math", "Georgia"]
        var picker = FontFamilyPicker(role: .math, family: .constant(nil), families: families, listing: false)
        XCTAssertEqual(picker.matches, [FontFamilyPicker.classDefault] + families)
        picker = FontFamilyPicker(role: .math, family: .constant(nil), families: families, listing: false)
        XCTAssertEqual(FontFamilyPicker(role: .text, family: .constant(nil), families: families, listing: false).matches.first, FontFamilyPicker.classDefault)
        XCTAssertEqual(picker.matches.count, families.count + 1)
    }

    // MARK: present / apply (hermetic)

    func testPresentNeedsAProjectRootAndOpensOnTheManifestsTable() throws {
        let bare = ShellModel()
        bare.detachWorker()
        bare.projectFonts.present()
        XCTAssertFalse(bare.projectFonts.shown)
        XCTAssertTrue(bare.navigationNote?.contains("Save the entry document first") == true, bare.navigationNote ?? "")

        let model = try project()
        model.manifest.reader = { root, _ in .success(self.payload(root: root, fonts: ["text": "Georgia", "sans": "Inter"])) }
        model.projectFonts.lister = { _ in (["Georgia", "Inter", "Latin Modern Math"], ["Latin Modern Math"]) }
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        model.projectFonts.present()
        XCTAssertTrue(model.projectFonts.shown)
        XCTAssertEqual(model.projectFonts.selection, .init(text: "Georgia", sans: "Inter"))
        XCTAssertFalse(model.projectFonts.canApply, "nothing changed yet")
        // The listing lands off-main; the Math row offers the MATH-table families only.
        let deadline = Date().addingTimeInterval(5)
        while model.projectFonts.listing || model.projectFonts.families.isEmpty, Date() < deadline {
            try await_(0.02)
        }
        XCTAssertEqual(model.projectFonts.families, ["Georgia", "Inter", "Latin Modern Math"])
        XCTAssertEqual(model.projectFonts.families(for: .math), ["Latin Modern Math"])
        XCTAssertEqual(model.projectFonts.families(for: .text), ["Georgia", "Inter", "Latin Modern Math"])
        // The request carries the table (ShellModel.compile sends `manifest.requestFonts`).
        XCTAssertEqual(model.manifest.requestFonts, .init(text: "Georgia", sans: "Inter"))
        XCTAssertEqual(model.manifest.fontsByRole, ["text": "Georgia", "sans": "Inter"])
    }

    func testApplyWritesTheRewrittenManifestReadsItBackAndClassDefaultWithoutAManifestWritesNothing() async throws {
        let model = try project()
        // No manifest on disk until the sheet writes one.
        var onDisk: [String: String]?
        model.manifest.reader = { root, _ in .success(self.payload(root: root, fonts: onDisk)) }
        var rewrites: [[String: String]] = []
        model.projectFonts.rewriter = { root, entry, fonts in
            rewrites.append(fonts)
            XCTAssertEqual(entry, "main.tex")
            let text = "[project]\nentry = \"main.tex\"\n\n[fonts]\n" + fonts.sorted { $0.key < $1.key }.map { "\($0.key) = \"\($0.value)\"\n" }.joined()
            return .success(.init(path: root.appendingPathComponent("flashtex.toml").path, exists: onDisk != nil, changed: true, text: text))
        }
        model.projectFonts.lister = { _ in ([], []) }
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        XCTAssertNil(model.manifest.requestFonts)

        // Class default everywhere, no manifest: nothing is written or rewritten.
        model.projectFonts.present()
        XCTAssertTrue(model.projectFonts.selection.isClassDefault)
        let nothing = await model.projectFonts.apply()
        guard case .unchanged(let why) = nothing else { return XCTFail("\(nothing)") }
        XCTAssertTrue(why.contains("nothing to write"), why)
        XCTAssertTrue(rewrites.isEmpty)
        XCTAssertFalse(FileManager.default.fileExists(atPath: tmp.appendingPathComponent("flashtex.toml").path))
        XCTAssertFalse(model.projectFonts.shown)

        // A choice: rewritten by the helper, saved through the rooted save, re-read.
        model.projectFonts.present()
        model.projectFonts.selection[.text] = "Georgia"
        model.projectFonts.selection[.mono] = "Menlo"
        XCTAssertTrue(model.projectFonts.canApply)
        let refreshesBefore = model.manifest.refreshes
        onDisk = ["text": "Georgia", "mono": "Menlo"] // what the reader finds after the save
        let outcome = await model.projectFonts.apply()
        XCTAssertEqual(outcome, .applied(path: tmp.appendingPathComponent("flashtex.toml").path))
        XCTAssertEqual(rewrites, [["text": "Georgia", "mono": "Menlo"]])
        let written = try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8)
        XCTAssertEqual(written, "[project]\nentry = \"main.tex\"\n\n[fonts]\nmono = \"Menlo\"\ntext = \"Georgia\"\n")
        XCTAssertEqual(model.projectFonts.applies, 1)
        XCTAssertFalse(model.projectFonts.shown)
        XCTAssertEqual(model.manifest.refreshes, refreshesBefore + 1, "re-read after the save")
        XCTAssertEqual(model.manifest.requestFonts, .init(text: "Georgia", mono: "Menlo"), "the next compile request carries it")
        XCTAssertTrue(model.navigationNote?.contains("text Georgia") == true && model.navigationNote?.contains("mono Menlo") == true, model.navigationNote ?? "")
        // Reopened: the sheet shows what was written.
        model.projectFonts.present()
        XCTAssertEqual(model.projectFonts.selection, .init(text: "Georgia", mono: "Menlo"))
        XCTAssertFalse(model.projectFonts.canApply)

        // Back to Class default with a manifest: the table is emptied (a write, not a no-op).
        model.projectFonts.selection = .init()
        onDisk = [:]
        let cleared = await model.projectFonts.apply()
        XCTAssertEqual(cleared, .applied(path: tmp.appendingPathComponent("flashtex.toml").path))
        XCTAssertEqual(rewrites.last, [:])
        XCTAssertNil(model.manifest.requestFonts)

        // The helper says nothing changed: no save, the sheet closes.
        model.projectFonts.rewriter = { root, _, _ in .success(.init(path: root.appendingPathComponent("flashtex.toml").path, exists: true, changed: false, text: nil)) }
        model.projectFonts.present()
        model.projectFonts.selection[.sans] = "Inter"
        let same = await model.projectFonts.apply()
        guard case .unchanged = same else { return XCTFail("\(same)") }
        XCTAssertEqual(model.projectFonts.applies, 2)

        // A refusal keeps the sheet up with the reason.
        model.projectFonts.rewriter = { _, _, _ in .failure(.init("helper gone")) }
        model.projectFonts.present()
        model.projectFonts.selection[.sans] = "Inter"
        let refused = await model.projectFonts.apply()
        guard case .refused(let reason) = refused else { return XCTFail("\(refused)") }
        XCTAssertTrue(reason.contains("helper gone"), reason)
        XCTAssertTrue(model.projectFonts.shown)
        XCTAssertEqual(model.projectFonts.note, reason)
    }

    // MARK: the compile request

    func testTheDirectRouteSendsTheTableAndAChangeReRequestsUnchangedBuffers() throws {
        let model = try project()
        var fonts: [String: String]? = ["text": "Georgia"]
        model.manifest.reader = { root, _ in .success(self.payload(root: root, fonts: fonts)) }
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        XCTAssertEqual(model.manifest.requestFonts, .init(text: "Georgia"))
        // A manifest re-read with a different table notifies the model
        // (ShellModel.manifestFontsDidChange); without a worker that is a no-op
        // beyond the request field, which the next compile reads.
        fonts = ["text": "Georgia", "math": "STIX Two Math"]
        model.manifest.refresh()
        XCTAssertEqual(model.manifest.requestFonts, .init(text: "Georgia", math: "STIX Two Math"))
        XCTAssertNotEqual(model.fontsApplied, model.manifest.requestFonts, "compile() must not short-circuit on unchanged buffers")
    }

    // MARK: the completion rows

    func testFontCompletionRowsCarryTheSampleFamily() {
        let typed = "\\setmainfont{Geo"
        let s = Completion.suggestions(in: typed, caretUTF16: (typed as NSString).length, metadata: nil, fontFamilies: ["Georgia", "Helvetica"])
        XCTAssertEqual(s.map(\.label), ["Georgia"])
        XCTAssertEqual(s.first?.sampleFamily, "Georgia")
        XCTAssertEqual(s.first?.detail, "installed font family")
        // The attributed row appends the sample when AppKit knows the family.
        if FontSamples.nsFont(family: "Georgia", size: 12) != nil {
            XCTAssertTrue(CompletionPopup.attributed(s[0]).string.hasSuffix(" — " + ProjectFontsState.sampleText))
        }
        let row = CompletionRowView(frame: NSRect(x: 0, y: 0, width: 600, height: 24))
        row.configure(s[0])
        XCTAssertEqual(row.accessibilityLabel(), CompletionPopup.spokenLabel(s[0]))
        // A family the app has no face for gets no sample and keeps the plain row.
        let unknown = Completion.Suggestion(label: "No Such Family 4711", insertText: "No Such Family 4711", kind: .command, detail: "installed font family", sampleFamily: "No Such Family 4711")
        XCTAssertNil(FontSamples.nsFont(family: "No Such Family 4711", size: 12))
        XCTAssertFalse(CompletionPopup.attributed(unknown).string.contains(ProjectFontsState.sampleText))
        // Other rows are untouched.
        let section = Completion.Suggestion(label: "\\section", insertText: "\\section", kind: .command, detail: "supported")
        XCTAssertNil(section.sampleFamily)
        XCTAssertFalse(CompletionPopup.attributed(section).string.contains(ProjectFontsState.sampleText))
    }

    // MARK: the real helper

    func testRealHelperRewritesTheTableAndTheAppReadsItBack() async throws {
        guard let helper = DocumentFilesTests.realHelper else {
            throw XCTSkip("build crates/project-files (cargo build --release) or set FLASHTEX_PROJECT_FILES")
        }
        _ = try write("main.tex", "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n")
        _ = try write("flashtex.toml", "# mine\n[project]\nentry = \"main.tex\"\n\n[fonts]\ntext = \"Old\"\n\n[packages]\nfetch = \"never\"\n")
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .executable(helper, arguments: [])
        model.projectFonts.lister = { _ in ([], []) }
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        XCTAssertEqual(model.manifest.requestFonts, .init(text: "Old"))
        model.projectFonts.present()
        XCTAssertEqual(model.projectFonts.selection, .init(text: "Old"))
        model.projectFonts.selection[.text] = "Georgia"
        model.projectFonts.selection[.math] = "STIX Two Math"
        let outcome = await model.projectFonts.apply()
        guard case .applied(let path) = outcome else { return XCTFail("\(outcome) — \(model.projectFonts.note ?? "")") }
        XCTAssertEqual(URL(fileURLWithPath: path).resolvingSymlinksInPath().path, tmp.appendingPathComponent("flashtex.toml").resolvingSymlinksInPath().path)
        let written = try String(contentsOf: tmp.appendingPathComponent("flashtex.toml"), encoding: .utf8)
        XCTAssertEqual(written, "# mine\n[project]\nentry = \"main.tex\"\n\n[fonts]\ntext = \"Georgia\"\nmath = \"STIX Two Math\"\n\n[packages]\nfetch = \"never\"\n", "only the table changed")
        XCTAssertEqual(model.manifest.requestFonts, .init(text: "Georgia", math: "STIX Two Math"), "re-read through the helper")
        model.projectFonts.present()
        XCTAssertEqual(model.projectFonts.selection, .init(text: "Georgia", math: "STIX Two Math"))
    }

    private func await_(_ seconds: TimeInterval) throws {
        RunLoop.main.run(until: Date().addingTimeInterval(seconds))
    }
}
