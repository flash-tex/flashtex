import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The project manifest in the shell (ProjectManifest.swift,
/// docs/user/project-manifest.md): the helper's `manifest` payload decoded,
/// a folder resolving to its entry, package inputs joining the compile
/// request and the sidebar, `.toml` coloured as plain text with comments,
/// and Create flashtex.toml… writing the template. Hermetic through the
/// `reader` hook; the real-helper cases need the built helper and skip
/// otherwise, never silently pass.
@MainActor
final class ProjectManifestTests: XCTestCase {
    private var tmp: URL!

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("manifest-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func write(_ rel: String, _ text: String) throws -> URL {
        let url = tmp.appendingPathComponent(rel)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// A helper payload as the Rust helper writes it (`crates/project-files`
    /// `manifest` operation), for the hermetic cases.
    private func payload(exists: Bool, entry: String?, manifestDir: String?, files: [(String, String, Int?, String?)]) -> ProjectFilesV1.Manifest {
        let fileObjects = files.map { path, kind, texinput, origin -> String in
            "{\"path\":\"\(path)\",\"kind\":\"\(kind)\",\"texinput\":\(texinput.map(String.init) ?? "null"),\"origin\":\(origin.map { "\"\($0)\"" } ?? "null"),\"text\":\"\\\\def\\\\x{1}\\n\",\"sha256\":\"ab\",\"bytes\":10}"
        }.joined(separator: ",")
        let json = """
        {"path":\(exists ? "\"\(manifestDir ?? "")/flashtex.toml\"" : "null"),"exists":\(exists),"manifest_dir":\(manifestDir.map { "\"\($0)\"" } ?? "null"),
         "manifest":{"project":{"entry":\(entry.map { "\"\($0)\"" } ?? "null"),"texinputs":["styles"],"output":null},
                     "fonts":{"text":null,"math":null,"mono":null,"sans":null},
                     "packages":{"source":"ctan","fetch":"ask","pin":{"siunitx":"3.3.24"},"path":{}},"library":null},
         "warnings":[{"key":"fonts.serif","message":"unknown key; ignored"}],
         "texinputs":[{"index":0,"raw":"styles","location":"inside","dir":"styles"}],
         "files":[\(fileObjects)],"diagnostics":[],
         "template":"[project]\\nentry = \\"\(entry ?? "main.tex")\\"\\n"}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    // MARK: decoding

    func testHelperPayloadDecodes() {
        let m = payload(exists: true, entry: "paper/main.tex", manifestDir: "/p", files: [("styles/a.sty", "package", 0, nil), ("texinputs/1/b.cls", "class", 1, "/shared/b.cls")])
        XCTAssertEqual(m.path, "/p/flashtex.toml")
        XCTAssertEqual(m.manifestDir, "/p")
        XCTAssertEqual(m.manifest.project.entry, "paper/main.tex")
        XCTAssertEqual(m.manifest.project.texinputs, ["styles"])
        XCTAssertEqual(m.manifest.packages.pin, ["siunitx": "3.3.24"])
        XCTAssertNil(m.manifest.library)
        XCTAssertEqual(m.warnings.map(\.key), ["fonts.serif"])
        XCTAssertEqual(m.texinputs.first?.location, "inside")
        XCTAssertEqual(m.files.map(\.path), ["styles/a.sty", "texinputs/1/b.cls"])
        XCTAssertEqual(m.files[1].origin, "/shared/b.cls")
        XCTAssertEqual(m.files[0].texinput, 0)
    }

    // MARK: entry resolution (pure)

    func testEntryComesFromTheManifestElseTheOnlyTexFile() throws {
        let main = try write("paper/main.tex", "x")
        // The manifest names the entry relative to its own directory.
        let withEntry = payload(exists: true, entry: "paper/main.tex", manifestDir: tmp.path, files: [])
        XCTAssertEqual(try ProjectManifest.entry(in: tmp, manifest: withEntry).get().standardizedFileURL.path, main.standardizedFileURL.path)
        // A named entry that is not a file is a refusal, not a guess.
        let missing = payload(exists: true, entry: "gone.tex", manifestDir: tmp.path, files: [])
        guard case .failure(let why) = ProjectManifest.entry(in: tmp, manifest: missing) else { return XCTFail("must refuse") }
        XCTAssertTrue(why.why.contains("gone.tex"), why.why)
        // No manifest (or no entry in it): the folder's only .tex file.
        let none = payload(exists: false, entry: nil, manifestDir: nil, files: [])
        guard case .failure = ProjectManifest.entry(in: tmp, manifest: none) else { return XCTFail("no .tex at the top level") }
        _ = try write("thesis.tex", "y")
        XCTAssertEqual(try ProjectManifest.entry(in: tmp, manifest: none).get().lastPathComponent, "thesis.tex")
        XCTAssertEqual(try ProjectManifest.entry(in: tmp, manifest: nil).get().lastPathComponent, "thesis.tex")
        _ = try write("other.tex", "z")
        guard case .failure(let two) = ProjectManifest.entry(in: tmp, manifest: nil) else { return XCTFail("two candidates must refuse") }
        XCTAssertTrue(two.why.contains("other.tex, thesis.tex"), two.why)
    }

    // MARK: package inputs in the compile request and the sidebar (hermetic)

    func testPackageInputsJoinTheCompileDocumentsAndTheSidebarUntilOpened() async throws {
        let main = try write("main.tex", "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}x\\end{document}\n")
        _ = try write("mystyle.sty", "\\def\\x{1}\n")
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: hermetic")
        var reads: [(URL, String)] = []
        model.manifest.reader = { root, entry in
            reads.append((root, entry))
            return .success(self.payload(exists: true, entry: "main.tex", manifestDir: root.path,
                                         files: [("mystyle.sty", "package", nil, nil), ("texinputs/0/shared.cls", "class", 0, "/elsewhere/shared.cls")]))
        }
        XCTAssertEqual(model.openTex(at: main), .opened)
        XCTAssertEqual(model.manifest.refreshes, 1, "read once when the project opened")
        XCTAssertEqual(reads.last?.0.standardizedFileURL.path, tmp.standardizedFileURL.resolvingSymlinksInPath().path)
        XCTAssertEqual(reads.last?.1, "main.tex", "the template names the actual entry")
        XCTAssertTrue(model.manifest.exists)

        let implicit = model.project.implicitClosureDocuments()
        XCTAssertEqual(implicit.map(\.path), ["mystyle.sty", "texinputs/0/shared.cls"], "package inputs after the (empty) include closure")
        XCTAssertEqual(implicit[0].text, "\\def\\x{1}\n")

        let rows = WorkspaceSidebarRows.rows(model: model)
        XCTAssertEqual(rows.map(\.id), ["main.tex", "package:mystyle.sty", "package:texinputs/0/shared.cls"])
        XCTAssertEqual(rows[1].icon, FileTypeStyle.classOrStyle.systemImage)
        XCTAssertTrue(rows[1].dimmed && rows[2].dimmed)
        XCTAssertTrue(rows[2].tooltip?.contains("/elsewhere/shared.cls") == true, rows[2].tooltip ?? "")

        // Opening the package makes the buffer authoritative: it leaves the
        // implicit set and the dimmed row (the member row shows it now).
        let opened = await model.project.openDocument("mystyle.sty")
        XCTAssertEqual(opened, .opened(path: "mystyle.sty"))
        XCTAssertEqual(model.project.implicitClosureDocuments().map(\.path), ["texinputs/0/shared.cls"])
        XCTAssertEqual(WorkspaceSidebarRows.rows(model: model).map(\.id), ["main.tex", "mystyle.sty", "package:texinputs/0/shared.cls"])
        XCTAssertEqual(WorkspaceSidebarRows.rows(model: model)[1].icon, FileTypeStyle.classOrStyle.systemImage, "a .sty member keeps the class/style icon")

        // Switching to it colours it as LaTeX; a .toml member as TOML.
        _ = model.project.switchDocument(to: "mystyle.sty")
        XCTAssertEqual(model.editorLanguage, .latex)
        _ = try write("flashtex.toml", "# c\n[project]\n")
        let toml = await model.project.openDocument("flashtex.toml")
        XCTAssertEqual(toml, .opened(path: "flashtex.toml"))
        _ = model.project.switchDocument(to: "flashtex.toml")
        XCTAssertEqual(model.editorLanguage, .toml)
    }

    func testAFolderOpensAsTheManifestEntry() throws {
        let main = try write("paper/main.tex", "\\documentclass{article}\\begin{document}x\\end{document}")
        _ = try write("decoy.tex", "not the entry")
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: hermetic")
        model.manifest.reader = { root, _ in
            .success(self.payload(exists: true, entry: "paper/main.tex", manifestDir: root.standardizedFileURL.path, files: []))
        }
        XCTAssertEqual(model.openTex(at: tmp), .opened)
        XCTAssertEqual(model.documentURL?.standardizedFileURL.path, main.standardizedFileURL.path)
        XCTAssertEqual(model.project.entryPath, "main.tex")

        // Without any manifest read (no helper) a folder with one .tex still opens.
        let lone = tmp.appendingPathComponent("lone")
        _ = try write("lone/only.tex", "x")
        let bare = ShellModel()
        bare.detachWorker()
        bare.files.policy = .disabled(reason: "test: no helper")
        XCTAssertEqual(bare.openTex(at: lone), .opened)
        XCTAssertEqual(bare.documentURL?.lastPathComponent, "only.tex")
        XCTAssertFalse(bare.manifest.exists)
        XCTAssertTrue(bare.manifest.status.contains("helper"), bare.manifest.status)
        // And a folder that cannot be resolved is refused, the project untouched.
        _ = try write("lone/second.tex", "y")
        XCTAssertEqual(bare.openTex(at: lone), .readFailed)
        XCTAssertEqual(bare.documentURL?.lastPathComponent, "only.tex")
        XCTAssertTrue(bare.captureNote?.contains("only.tex, second.tex") == true, bare.captureNote ?? "")
    }

    // MARK: highlighting and file types

    func testTomlIsPlainTextWithComments() {
        let text = "# top\n[project]\nentry = \"main.tex\" # trailing\ntexinputs = [\"a\", \"$b\"]\n\\section{x} $y$\n" as NSString
        let runs = SyntaxHighlighter.runs(of: text, language: .toml).map { "\($0.kind):\(text.substring(with: $0.range))" }
        XCTAssertEqual(runs, ["comment:# top", "comment:# trailing"], "only comments; no LaTeX, no math")
        XCTAssertEqual(SyntaxHighlighter.Language.toml.initialMode, .toml)
        XCTAssertTrue(ProjectManifest.isManifestPath("flashtex.toml") && ProjectManifest.isManifestPath("sub/flashtex.toml") && ProjectManifest.isManifestPath("x.TOML"))
        XCTAssertFalse(ProjectManifest.isManifestPath("main.tex"))
        XCTAssertEqual(FileTypeStyle.of(path: "a.def"), .classOrStyle)
        XCTAssertEqual(FileTypeStyle.of(path: "size11.clo"), .classOrStyle)
        XCTAssertEqual(FileTypeStyle.of(path: "flashtex.toml"), .manifest)
    }

    // MARK: the real helper

    private func realHelper() throws -> URL {
        guard let url = DocumentFilesTests.realHelper else {
            throw XCTSkip("build crates/project-files (cargo build --release) or set FLASHTEX_PROJECT_FILES")
        }
        return url
    }

    func testRealHelperReadsTheManifestAndCreateWritesTheTemplate() async throws {
        let helper = try realHelper()
        let main = try write("proj/paper/main.tex", "\\documentclass{article}\\usepackage{mystyle}\\begin{document}x\\end{document}")
        _ = try write("proj/paper/local.sty", "\\def\\local{1}")
        _ = try write("proj/styles/mystyle.sty", "\\def\\mystyle{1}")
        _ = try write("proj/styles/README.md", "no")
        _ = try write("proj/flashtex.toml", "[project]\nentry = \"paper/main.tex\"\ntexinputs = [\"styles\", \"/abs\"]\n[fonts]\nserif = \"x\"\n")
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .executable(helper, arguments: [])
        // The folder opens as the manifest's entry.
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("proj")), .opened)
        XCTAssertEqual(model.documentURL?.resolvingSymlinksInPath().path, main.resolvingSymlinksInPath().path)
        guard let snapshot = model.manifest.snapshot else { return XCTFail(model.manifest.status) }
        XCTAssertTrue(snapshot.exists)
        XCTAssertEqual(snapshot.manifest.project.texinputs, ["styles", "/abs"])
        XCTAssertEqual(snapshot.warnings.map(\.key), ["fonts.serif"])
        XCTAssertEqual(snapshot.texinputs.map(\.location), ["inside", "invalid"])
        // The root is the entry's directory: `local.sty` is rooted, `styles`
        // beside the manifest is mounted virtually with its real origin.
        XCTAssertEqual(snapshot.files.map(\.path), ["local.sty", "texinputs/0/mystyle.sty"])
        XCTAssertEqual(snapshot.files[1].origin.map { URL(fileURLWithPath: $0).resolvingSymlinksInPath().path },
                       tmp.appendingPathComponent("proj/styles/mystyle.sty").resolvingSymlinksInPath().path)
        XCTAssertEqual(model.project.implicitClosureDocuments().map(\.path), ["local.sty", "texinputs/0/mystyle.sty"])
        XCTAssertTrue(model.manifest.exists, "a manifest above the root governs")

        // Create flashtex.toml… next to the entry: the template names the
        // entry, is opened as a member, and a second create is refused.
        XCTAssertNotNil(model.manifest.manifestURL)
        let created = await model.manifest.createManifest()
        XCTAssertEqual(created, .created(path: "flashtex.toml"))
        let written = try String(contentsOf: main.deletingLastPathComponent().appendingPathComponent("flashtex.toml"), encoding: .utf8)
        XCTAssertTrue(written.contains("entry = \"main.tex\""), written)
        XCTAssertTrue(written.hasPrefix("# flashtex.toml"), written)
        XCTAssertEqual(model.activePath, "flashtex.toml")
        XCTAssertEqual(model.editorLanguage, .toml)
        XCTAssertEqual(model.manifest.snapshot?.path.map { URL(fileURLWithPath: $0).lastPathComponent }, "flashtex.toml")
        XCTAssertEqual(model.manifest.snapshot?.manifestDir.map { URL(fileURLWithPath: $0).resolvingSymlinksInPath().path },
                       main.deletingLastPathComponent().resolvingSymlinksInPath().path, "the new file next to the entry now governs")
        let again = await model.manifest.createManifest()
        guard case .refused(let why) = again else { return XCTFail("must refuse an existing manifest") }
        XCTAssertTrue(why.contains("already exists"), why)
        model.files.detachHelper()
    }
}

/// The sidebar's row model, driven exactly as `ProjectSection` drives it.
enum WorkspaceSidebarRows {
    @MainActor static func rows(model: ShellModel) -> [SidebarTree.Row] {
        ProjectSection.rows(listing: model.project.listing, kinds: model.documentKinds, closure: model.project.discoverClosure(),
                            packages: model.manifest.rows, activePath: model.activePath)
    }
}
