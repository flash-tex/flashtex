import Foundation
import XCTest
@testable import FlashTeXMac

/// Round 3 of the #1540 security review, with the real engine host: in a
/// confined (session) compile, NO file outside the project, the output
/// folder or the TeX trees is opened, whatever primitive names it:
/// `\pdfobj file`, `\pdfmapfile`, `\pdfmapline`, `\font` with an absolute
/// path, or a project file that is a relative link to a file outside. The
/// engine's read-set log (`FLASHTEX_READ_SET`: every file it opens) is the
/// witness; unconfined controls show the same probes do read. Also: leaving
/// a session clears the copy's output folder, and the fast path never sends
/// to a host whose confinement does not match (the keystroke race). Skips
/// without a built `flashtex-host` and a TeX Live.
@MainActor
final class LiveShareConfinedProbeTests: XCTestCase {
    private var temp: URL!
    private var outside: URL!

    override func setUp() async throws {
        temp = FileManager.default.temporaryDirectory.appendingPathComponent("liveshare-probe-\(UUID().uuidString)").resolvingSymlinksInPath()
        outside = temp.appendingPathComponent("outside")
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        try "SECRET-BYTES\n".write(to: outside.appendingPathComponent("secret.tex"), atomically: true, encoding: .utf8)
        try "evilfont EvilFont <\(outside.path)/evil.pfb\n".write(to: outside.appendingPathComponent("evil.map"), atomically: true, encoding: .utf8)
        try Data(repeating: 0x80, count: 64).write(to: outside.appendingPathComponent("evil.pfb"))
        if let tfm = Self.kpsewhich("cmr10.tfm") {
            try FileManager.default.copyItem(at: URL(fileURLWithPath: tfm), to: outside.appendingPathComponent("fake.tfm"))
        }
        LiveShareController.sessionBaseOverride = temp.appendingPathComponent("collab")
        LiveShareController.testLoopbackOnly = true
        LiveShareController.enabledOverride = true
    }

    override func tearDown() async throws {
        unsetenv("FLASHTEX_READ_SET")
        LiveShareController.sessionBaseOverride = nil
        LiveShareController.testLoopbackOnly = false
        LiveShareController.enabledOverride = nil
        try? FileManager.default.removeItem(at: temp)
    }

    static func kpsewhich(_ name: String) -> String? {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/Library/TeX/texbin/kpsewhich")
        p.arguments = [name]
        let out = Pipe()
        p.standardOutput = out
        p.standardError = FileHandle.nullDevice
        guard (try? p.run()) != nil else { return nil }
        p.waitUntilExit()
        let s = String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
        return s.isEmpty ? nil : s
    }

    /// Compiles `doc` (a session copy unless `session` is false) with a
    /// fresh read-set log; returns the diagnostics and every path opened.
    private func probe(_ doc: String, session: Bool = true) async throws -> (messages: [String], opened: [String]) {
        try EngineV3TestHost.require()
        let dir = session ? LiveShareController.sessionDirectory("ab" + String(UUID().uuidString.prefix(8)).lowercased().filter(\.isHexDigit))
                          : temp.appendingPathComponent("plain-\(UUID().uuidString.prefix(8))")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try doc.replacingOccurrences(of: "OUT", with: outside.path).write(to: dir.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        // A project file that is a relative link to a file outside it.
        let rel = String(repeating: "../", count: dir.pathComponents.count - temp.pathComponents.count) + "outside/secret.tex"
        try FileManager.default.createSymbolicLink(atPath: dir.appendingPathComponent("evil.tex").path, withDestinationPath: rel)
        let readSet = temp.appendingPathComponent("readset-\(UUID().uuidString.prefix(6)).txt")
        setenv("FLASHTEX_READ_SET", readSet.path, 1)
        defer { unsetenv("FLASHTEX_READ_SET") }
        let m = ShellModel()
        XCTAssertEqual(m.openTex(at: dir.appendingPathComponent("main.tex"), dirty: .discard), .opened)
        XCTAssertEqual(m.liveShare.forcesPinnedCompile(root: m.project.projectRoot), session)
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        try await EngineV3TestHost.awaitReady(s)
        XCTAssertEqual(s.hostConfineRoots.map { $0 != nil }, session, "the host is confined exactly for a session copy")
        let start = Date()
        while s.statusNote.isEmpty || s.compiling || m.engineV3Diagnostics.isEmpty {
            if Date().timeIntervalSince(start) > 180 { XCTFail("timeout: \(s.statusNote)"); break }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        let messages = m.engineV3Diagnostics.map(\.message)
        s.stop()
        let log = (try? String(contentsOf: readSet, encoding: .utf8)) ?? ""
        let opened = log.split(separator: "\n").compactMap { line -> String? in
            let f = line.split(separator: "\t", omittingEmptySubsequences: false)
            return f.first == "open" && f.count > 1 ? String(f[1]) : nil
        }
        return (messages, opened)
    }

    private func outsideOpens(_ opened: [String]) -> [String] {
        opened.filter { URL(fileURLWithPath: $0).resolvingSymlinksInPath().path.hasPrefix(outside.path) || $0.hasPrefix("/etc/") }
    }

    static let fontsAndMaps = """
    \\documentclass{article}
    \\pdfmapfile{=OUT/evil.map}
    \\pdfmapline{+evilfont2 EvilFont <OUT/evil.pfb}
    \\begin{document}
    \\font\\x=OUT/fake \\ifx\\x\\nullfont\\PackageWarning{probe}{FONT-REFUSED}\\else\\PackageWarning{probe}{FONT-LOADED}\\fi
    \\IfFileExists{evil.tex}{\\PackageWarning{probe}{LINK-READ}}{\\PackageWarning{probe}{LINK-REFUSED}}
    \\PackageWarning{probe}{PROBES-RAN}
    Text.
    \\end{document}

    """

    func testFontsMapsAndLinksStayInsideInASession() async throws {
        let r = try await probe(Self.fontsAndMaps)
        XCTAssertTrue(r.messages.contains { $0.contains("PROBES-RAN") }, "\(r.messages)")
        XCTAssertTrue(r.messages.contains { $0.contains("FONT-REFUSED") }, "\(r.messages)")
        XCTAssertTrue(r.messages.contains { $0.contains("LINK-REFUSED") }, "\(r.messages)")
        XCTAssertEqual(outsideOpens(r.opened), [], "files opened outside the project")
        XCTAssertFalse(r.opened.isEmpty, "the read set is recorded")
    }

    /// The control: the same probes read outside a session. (`\font` with
    /// an absolute name does not load in this host either way, so for it
    /// the read set is the evidence: nothing outside is opened in a session.)
    func testControlTheSameProbesReadOutsideASession() async throws {
        let r = try await probe(Self.fontsAndMaps, session: false)
        XCTAssertTrue(r.messages.contains { $0.contains("LINK-READ") }, "\(r.messages)")
        XCTAssertFalse(outsideOpens(r.opened).isEmpty, "the read set sees outside opens when unconfined: \(r.opened.suffix(5))")
    }

    func testPdfobjFileAndALinkedInputStayInside() async throws {
        for doc in [
            "\\documentclass{article}\n\\begin{document}\n\\immediate\\pdfobj stream file{OUT/secret.tex}\\pdfrefobj\\pdflastobj\nText.\n\\end{document}\n",
            "\\documentclass{article}\n\\begin{document}\n\\immediate\\pdfobj stream file{evil.tex}\\pdfrefobj\\pdflastobj\nText.\n\\end{document}\n",
            "\\documentclass{article}\n\\begin{document}\n\\input{evil}\nText.\n\\end{document}\n",
        ] {
            let r = try await probe(doc)
            XCTAssertEqual(outsideOpens(r.opened), [], "\(doc): opened outside")
            XCTAssertFalse(r.messages.contains { $0.contains("SECRET-BYTES") }, "\(r.messages)")
        }
    }

    /// Round 4: a tex.web device name (`TeXformats:`) in a document's name
    /// is literal (as in pdfTeX), so it can no longer select the format
    /// search and its exemption: neither `\openin` nor `\input` of
    /// `TeXformats:/etc/hosts` opens /etc/hosts in a session.
    func testDeviceNamesDoNotEscapeConfinement() async throws {
        let openin = """
        \\documentclass{article}
        \\begin{document}
        \\openin1=TeXformats:/etc/hosts \\ifeof1 \\PackageWarning{probe}{DEVICE-EOF}\\else \\PackageWarning{probe}{DEVICE-READ}\\fi
        \\IfFileExists{TeXformats:OUT/secret.tex}{\\PackageWarning{probe}{DEVICE-SECRET-READ}}{}
        Text.
        \\end{document}

        """
        let r = try await probe(openin)
        XCTAssertTrue(r.messages.contains { $0.contains("DEVICE-EOF") }, "\(r.messages)")
        XCTAssertFalse(r.messages.contains { $0.contains("DEVICE-READ") || $0.contains("DEVICE-SECRET-READ") }, "\(r.messages)")
        XCTAssertEqual(outsideOpens(r.opened), [])
        let input = try await probe("\\documentclass{article}\n\\begin{document}\n\\input TeXformats:/etc/hosts\nText.\n\\end{document}\n")
        XCTAssertEqual(outsideOpens(input.opened), [])
        // pdfTeX's own words for it.
        XCTAssertTrue(input.messages.contains { $0.contains("I can't find file `TeXformats:/etc/hosts'") }, "\(input.messages)")
    }

    // MARK: Sessions on a host's own project

    /// Hosting: the keystroke fast path refuses a host whose confinement no
    /// longer matches (the race), the slow path relaunches it confined, and
    /// ending the session relaunches it unconfined with the copy's output
    /// folder emptied.
    func testSessionStartAndEndRelaunchTheHostAndClearTheOutput() async throws {
        try EngineV3TestHost.require()
        let project = temp.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
        try "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n"
            .write(to: project.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let m = ShellModel()
        XCTAssertEqual(m.openTex(at: project.appendingPathComponent("main.tex"), dirty: .discard), .opened)
        m.engineV3Enabled = true
        let s = m.engineV3
        s.start(model: m)
        try await EngineV3TestHost.awaitReady(s)
        defer { s.stop(); if m.liveShare.isActive { m.liveShare.leave() } }
        XCTAssertEqual(s.hostConfineRoots.map { $0 == nil }, true, "an ordinary project: unconfined")
        XCTAssertTrue(s.fastPathAllowed(model: m))

        m.liveShare.startHosting()
        XCTAssertTrue(m.liveShare.isActive)
        XCTAssertFalse(s.fastPathAllowed(model: m), "a keystroke now must not reach the unconfined host")
        s.compile(model: m, reason: "edit")
        try await EngineV3TestHost.awaitReady(s)
        XCTAssertEqual(s.hostConfineRoots??.isEmpty, false, "relaunched confined")
        XCTAssertTrue(s.fastPathAllowed(model: m))
        let out = try XCTUnwrap(s.outputDirectory)
        try "\\gdef\\x{\\immediate\\write18{touch pwned}}\n".write(to: out.appendingPathComponent("main.aux"), atomically: true, encoding: .utf8)

        m.liveShare.leave()
        XCTAssertFalse(s.fastPathAllowed(model: m))
        s.compile(model: m, reason: "edit")
        XCTAssertEqual(s.outputClearedForSession, 1)
        XCTAssertFalse(FileManager.default.fileExists(atPath: out.appendingPathComponent("main.aux").path), "the session's .aux is gone")
        try await EngineV3TestHost.awaitReady(s)
        XCTAssertEqual(s.hostConfineRoots.map { $0 == nil }, true, "relaunched unconfined")
    }
}
