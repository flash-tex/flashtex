import Foundation
import XCTest
@testable import FlashTeXMac

/// Session compiles with the real engine host (security review of #1540):
/// a project in a Live Share session copy compiles in a host launched
/// confined (`EngineV3HostProcess.confinedEnvironment`). Its text cannot
/// read files by absolute, `~`, `$` or `..` names, and its `\openout`
/// writes land only in the project copy's output folder. Skips without a
/// built `flashtex-host` and a TeX Live (EngineV3TestHost).
@MainActor
final class LiveShareConfinedCompileTests: XCTestCase {
    private var temp: URL!

    override func setUp() async throws {
        temp = FileManager.default.temporaryDirectory.appendingPathComponent("liveshare-confined-\(UUID().uuidString)")
        LiveShareController.sessionBaseOverride = temp.appendingPathComponent("collab")
    }

    override func tearDown() async throws {
        LiveShareController.sessionBaseOverride = nil
        try? FileManager.default.removeItem(at: temp)
    }

    /// Compiles `doc` as main.tex of a session copy; returns the model and
    /// the copy's folder once the compile has reported.
    private func compiled(_ doc: String, diagnostics: Bool = false, session: Bool = true) async throws -> (ShellModel, URL) {
        try EngineV3TestHost.require()
        let dir = session ? LiveShareController.sessionDirectory(String(UUID().uuidString.prefix(8)).lowercased().filter(\.isHexDigit) + "ab")
                          : temp.appendingPathComponent("plain-\(UUID().uuidString.prefix(8))")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try doc.write(to: dir.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let m = ShellModel()
        XCTAssertEqual(m.openTex(at: dir.appendingPathComponent("main.tex"), dirty: .discard), .opened)
        XCTAssertEqual(m.liveShare.forcesPinnedCompile(root: m.project.projectRoot), session)
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        try await EngineV3TestHost.awaitReady(s)
        let start = Date()
        while s.statusNote.isEmpty || s.compiling || (diagnostics && m.engineV3Diagnostics.isEmpty) {
            if Date().timeIntervalSince(start) > 180 { XCTFail("timeout: \(s.statusNote)"); break }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        return (m, dir)
    }

    private func notFound(_ m: ShellModel) -> [String] {
        m.engineV3Diagnostics.filter { $0.code == "latex/file-not-found" || $0.message.contains("not found") }.map(\.message)
    }

    func testSessionTextCannotReadFilesOutsideTheProject() async throws {
        // A real file in the home folder, so a `~` read that got through
        // would succeed (and define \probe) instead of failing.
        let probe = "flashtex-confine-probe-\(UUID().uuidString.prefix(8))"
        let home = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("\(probe).tex")
        try "\\def\\homeprobe{read}\n".write(to: home, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(at: home) }
        let outside = temp.appendingPathComponent("outside.tex")
        try FileManager.default.createDirectory(at: temp, withIntermediateDirectories: true)
        try "\\def\\upprobe{read}\n".write(to: outside, atomically: true, encoding: .utf8)
        // A missing \input stops a nonstop run, so one compile per \input.
        for name in ["/etc/hosts", "~/\(probe)"] {
            let (m, _) = try await compiled("\\documentclass{article}\n\\begin{document}\n\\input{\(name)}\nText.\n\\end{document}\n",
                                            diagnostics: true)
            let messages = m.engineV3Diagnostics.map(\.message)
            let missing = notFound(m)
            m.engineV3.stop() // (clears the diagnostics)
            XCTAssertTrue(missing.contains { $0.contains(name) }, "\(name): \(messages)")
            XCTAssertFalse(messages.contains { $0.contains("homeprobe") }, "\(messages)")
        }
        // \openin (\IfFileExists), `$`, `..` and the file primitives, in one
        // run: none of them is fatal, and each reports what it saw.
        let doc = """
        \\documentclass{article}
        \\begin{document}
        \\IfFileExists{/etc/hosts}{\\PackageError{probe}{ABS-READ}{}}{}
        \\IfFileExists{~/\(probe).tex}{\\PackageError{probe}{TILDE-READ}{}}{}
        \\IfFileExists{$HOME/\(probe).tex}{\\PackageError{probe}{DOLLAR-READ}{}}{}
        \\IfFileExists{../../outside.tex}{\\PackageError{probe}{DOTDOT-READ}{}}{}
        \\edef\\size{\\pdffilesize{/etc/hosts}}\\ifx\\size\\empty\\else\\PackageError{probe}{FILESIZE-READ}{}\\fi
        \\edef\\dump{\\pdffiledump length 4{/etc/hosts}}\\ifx\\dump\\empty\\else\\PackageError{probe}{FILEDUMP-READ}{}\\fi
        \\PackageWarning{probe}{PROBES-RAN}
        Text.
        \\end{document}

        """
        let (m, _) = try await compiled(doc, diagnostics: true)
        defer { m.engineV3.stop() }
        let messages = m.engineV3Diagnostics.map(\.message)
        XCTAssertTrue(messages.contains { $0.contains("PROBES-RAN") }, "the probes ran: \(messages)")
        for leak in ["ABS-READ", "TILDE-READ", "DOLLAR-READ", "DOTDOT-READ", "FILESIZE-READ", "FILEDUMP-READ"] {
            XCTAssertFalse(messages.contains { $0.contains(leak) }, "\(leak): \(messages)")
        }
    }

    func testSessionTextWritesOnlyIntoTheOutputFolder() async throws {
        let target = temp.appendingPathComponent("pwned.tex")
        try FileManager.default.createDirectory(at: temp, withIntermediateDirectories: true)
        for (i, name) in [target.path, "../../pwned-up.tex"].enumerated() {
            let doc = """
            \\documentclass{article}
            \\begin{document}
            \\newwrite\\w \\immediate\\openout\\w=\(name) \\immediate\\write\\w{x}\\immediate\\closeout\\w
            Text \(i).
            \\end{document}

            """
            let (m, dir) = try await compiled(doc)
            m.engineV3.stop()
            XCTAssertFalse(FileManager.default.fileExists(atPath: target.path), "absolute write outside")
            XCTAssertFalse(FileManager.default.fileExists(atPath: dir.deletingLastPathComponent().appendingPathComponent("pwned-up.tex").path))
            XCTAssertFalse(FileManager.default.fileExists(atPath: dir.deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("pwned-up.tex").path))
        }
        // A plain relative name is written, but into the copy's output
        // folder, never next to the sources (no Makefile, no latexmkrc).
        let doc = """
        \\documentclass{article}
        \\begin{document}
        \\newwrite\\w \\immediate\\openout\\w=Makefile \\immediate\\write\\w{all:}\\immediate\\closeout\\w
        Text.
        \\end{document}

        """
        let (m, dir) = try await compiled(doc)
        defer { m.engineV3.stop() }
        let names = try FileManager.default.contentsOfDirectory(atPath: dir.path)
        XCTAssertFalse(names.contains { $0.hasPrefix("Makefile") }, "\(names)")
    }

    /// The control: outside a session the same probes read (pdfTeX's own
    /// behaviour, unchanged), so the session test sees real refusals.
    func testOutsideASessionTheProbesRead() async throws {
        let doc = """
        \\documentclass{article}
        \\begin{document}
        \\IfFileExists{/etc/hosts}{\\PackageWarning{probe}{ABS-READ}}{}
        \\edef\\size{\\pdffilesize{/etc/hosts}}\\ifx\\size\\empty\\else\\PackageWarning{probe}{FILESIZE-READ}\\fi
        Text.
        \\end{document}

        """
        let (m, _) = try await compiled(doc, diagnostics: true, session: false)
        let messages = m.engineV3Diagnostics.map(\.message)
        m.engineV3.stop()
        XCTAssertTrue(messages.contains { $0.contains("ABS-READ") }, "\(messages)")
        XCTAssertTrue(messages.contains { $0.contains("FILESIZE-READ") }, "\(messages)")
    }
}
