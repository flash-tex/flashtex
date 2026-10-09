import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Owner report 2026-10-07: typing `\usepackage{a` on the way to `amsmath`
/// popped up "package `a` not found" at once. A compile's missing packages
/// now wait while the caret is in the load argument being typed: shown after
/// `graceDelay` (3 s) without an edit to it, or at once when the caret
/// leaves it (ProjectPackages.swift, "the grace period"). Clock and timer
/// are injected (`engineV3Now` / `engineV3Schedule`); no helper, no network.
@MainActor
final class PackagePopupGraceTests: XCTestCase {
    private var tmp: URL!
    private var now = Date(timeIntervalSince1970: 1_000_000)
    private var scheduled: [(at: Date, work: DispatchWorkItem)] = []
    private var resolved: [[String]] = []

    override func setUp() {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("package-grace-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func manifest(root: URL) -> ProjectFilesV1.Manifest {
        let json = """
        {"path":"\(root.path)/flashtex.toml","exists":true,"manifest_dir":"\(root.path)",
         "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null},"fonts":{"text":null,"math":null,"mono":null,"sans":null},
                     "packages":{"source":"ctan","fetch":"ask","pin":{},"path":{}},"library":null},
         "warnings":[],"texinputs":[],"files":[],"diagnostics":[],"template":""}
        """
        return try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8))
    }

    private func result(missing: [String]) -> RuntimeV1.CompileResult {
        let rows = missing.isEmpty ? "" : "{\"severity\":\"warning\",\"message\":\"packages \(missing.joined(separator: ", ")) are recognised but not implemented\",\"code\":\"unsupported_feature\"}"
        let json = "{\"project_id\":\"p\",\"revision\":1,\"status\":\"ok\",\"pages\":[],\"diagnostics\":[\(rows)]}"
        return try! JSONDecoder().decode(RuntimeV1.CompileResult.self, from: Data(json.utf8))
    }

    private func project() throws -> ShellModel {
        try "\\documentclass{article}\n\\usepackage{}\n\\begin{document}\nHi.\n\\end{document}\n"
            .write(to: tmp.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "[packages]\nfetch = \"ask\"\n".write(to: tmp.appendingPathComponent("flashtex.toml"), atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: hermetic (direct writes)")
        model.manifest.reader = { root, _ in .success(self.manifest(root: root)) }
        XCTAssertEqual(model.openTex(at: tmp.appendingPathComponent("main.tex")), .opened)
        let state = model.projectPackages
        state.engineV3Now = { [unowned self] in now }
        state.engineV3Schedule = { [unowned self] delay, work in scheduled.append((now.addingTimeInterval(delay), work)) }
        state.resolver = { [unowned self] _, names, _ in
            resolved.append(names)
            let rows = names.map { "{\"name\":\"\($0)\",\"status\":\"needs_consent\",\"version\":\"1\",\"source_url\":\"https://mirrors.ctan.org/x/\",\"would_fetch\":[\"\($0).sty\"]}" }
            let json = "{\"cache\":\"/tmp/c\",\"policy\":{\"source\":\"ctan\",\"fetch\":\"ask\"},\"diagnostics\":[],\"packages\":[\(rows.joined(separator: ","))]}"
            return .success(try! JSONDecoder().decode(ProjectFilesV1.ResolvePackages.self, from: Data(json.utf8)))
        }
        return model
    }

    /// Types `name` into `\usepackage{}` (caret right after it) and delivers that keystroke's compile.
    private func type(_ name: String, missing: [String], in model: ShellModel) {
        let text = "\\documentclass{article}\n\\usepackage{\(name)}\n\\begin{document}\nHi.\n\\end{document}\n"
        let i = model.documents.firstIndex { $0.path == model.activePath }!
        model.documents[i].text = text
        model.caretUTF16 = ("\\documentclass{article}\n\\usepackage{" + name as NSString).length
        model.result = result(missing: missing)
    }

    /// Runs the timers due by `now`.
    private func advance(_ seconds: TimeInterval) {
        now = now.addingTimeInterval(seconds)
        let due = scheduled.filter { $0.at <= now }
        scheduled.removeAll { $0.at <= now }
        for s in due where !s.work.isCancelled { s.work.perform() }
    }

    private func settle(_ what: String, _ until: @escaping () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(5)
        while !until(), Date() < deadline { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertTrue(until(), what)
    }

    private func quiet() async throws { try await Task.sleep(nanoseconds: 100_000_000) }

    // MARK: the argument at the caret (pure)

    func testLoadArgumentAtTheCaret() {
        let t = "\\usepackage[utf8]{am}\n\\input{ch1}\n\\textbf{x}\n"
        XCTAssertEqual(ProjectPackagesState.loadArgument(in: t, caret: 20)?.text, "am") // {am|}
        XCTAssertEqual(ProjectPackagesState.loadArgument(in: t, caret: 18)?.text, "am") // {|am}
        XCTAssertEqual(ProjectPackagesState.loadArgument(in: t, caret: 21)?.text, "am") // {am}|
        XCTAssertNil(ProjectPackagesState.loadArgument(in: t, caret: 15)) // in the options
        XCTAssertEqual(ProjectPackagesState.loadArgument(in: t, caret: 32)?.text, "ch1")
        XCTAssertNil(ProjectPackagesState.loadArgument(in: t, caret: 42)) // \textbf is not a load command
        XCTAssertNil(ProjectPackagesState.loadArgument(in: t, caret: 0))
        XCTAssertEqual(ProjectPackagesState.loadArgument(in: "\\documentclass{rep", caret: 18)?.text, "rep") // no `}` yet
    }

    // MARK: the owner's report

    func testTypingAPackageNameShowsNoPopupUntilThreeSecondsIdle() async throws {
        let model = try project()
        let state = model.projectPackages
        for (k, partial) in ["a", "am", "ams", "amsm"].enumerated() {
            type(partial, missing: [partial], in: model)
            advance(k == 0 ? 0.4 : 0.3)
        }
        try await quiet()
        XCTAssertFalse(state.shown, "no sheet while the name is being typed")
        XCTAssertEqual(resolved, [], "nothing resolved for a half-typed name")
        XCTAssertEqual(state.heldNames, ["amsm"], "its underline is held too")
        advance(2.0) // 2.3 s since the last edit
        try await quiet()
        XCTAssertFalse(state.shown)
        advance(1.0) // past 3 s: the unknown name is offered
        try await settle("the sheet after 3 s idle") { state.shown }
        XCTAssertEqual(resolved, [["amsm"]], "only the latest name, never the stale prefixes")
        XCTAssertEqual(state.heldNames, [])
    }

    func testMovingTheCaretOutShowsItAtOnce() async throws {
        let model = try project()
        let state = model.projectPackages
        type("foo", missing: ["foo"], in: model)
        try await quiet()
        XCTAssertFalse(state.shown)
        model.caretUTF16 = 0 // leaves the argument
        try await settle("the sheet once the caret left") { state.shown }
        XCTAssertEqual(resolved, [["foo"]])
    }

    func testExplicitCompileShowsItAtOnce() async throws {
        let model = try project()
        let state = model.projectPackages
        type("foo", missing: ["foo"], in: model)
        try await quiet()
        XCTAssertFalse(state.shown)
        state.explicitRequest(compiling: true)
        try await settle("the sheet on ⌘B") { state.shown }
    }

    func testAValidPackageNeverShowsIt() async throws {
        let model = try project()
        let state = model.projectPackages
        for partial in ["a", "am", "ams", "amsm", "amsma", "amsmat"] { type(partial, missing: [partial], in: model); advance(0.2) }
        type("amsmath", missing: [], in: model)
        XCTAssertEqual(state.heldNames, [])
        advance(10)
        model.caretUTF16 = 0
        try await quiet()
        XCTAssertFalse(state.shown)
        XCTAssertEqual(resolved, [])
    }
}
