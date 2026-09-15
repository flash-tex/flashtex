import CryptoKit
import Foundation
import XCTest
@testable import FlashTeXMac

/// GH36: the rooted Latin Modern metrics vendored under `apps/mac/Fonts/texmf`
/// (staged by `make-app.sh` into `Contents/Resources/texmf`) and the producer
/// environment that connects them (`BundledMetrics`).
final class BundledMetricsTests: XCTestCase {
    /// `apps/mac` of this checkout.
    private static let macDir = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    private static let vendoredRoot = macDir.appendingPathComponent("Fonts/texmf")

    /// The Commander's pinned manifest entries for the rooted tree
    /// (`crates/rendering-core/docs/handoffs/native-assets/manifest.json`).
    private static let pinned: [(path: String, sha256: String, bytes: Int)] = [
        ("fonts/tfm/public/lm/ec-lmr10.tfm", "cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5", 12056),
        ("fonts/tfm/public/lm/ec-lmr12.tfm", "299021120f0a29ef61278a2363903bd8defbb8faaade458eb79067342aecb56f", 12092),
        ("fonts/tfm/public/lm/rm-lmr12.tfm", "9d4e3d8e39a41b93d91f79c1c47d2297efb7b1af220b94860693c08361f227aa", 11888),
        ("fonts/tfm/public/lm/rm-lmr6.tfm", "eb0bfdf8db3ae1409639fac9c88f84923872500d882d9ff8dc37aff445c723fe", 11836),
        ("fonts/tfm/public/lm/rm-lmr8.tfm", "80bcbfd844d2310ac1d3bead45aee25e91b1a4a0a60ff1771959b9a1e90ec1a2", 11864),
        ("doc/fonts/lm/GUST-FONT-LICENSE.TXT", "49ea6cb9257bbee0a3979c48a774cd221550ac1c20c95549efe45fc99cc18050", 1377),
    ]

    private static func sha256Hex(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    // MARK: vendored tree

    func testVendoredTreeMatchesPinnedManifest() throws {
        for entry in Self.pinned {
            let url = Self.vendoredRoot.appendingPathComponent(entry.path)
            let data = try Data(contentsOf: url)
            XCTAssertEqual(data.count, entry.bytes, entry.path)
            XCTAssertEqual(Self.sha256Hex(data), entry.sha256, entry.path)
        }
    }

    /// The in-repo pin for the supplementary (non-Commander) metrics
    /// (`SUPPLEMENTARY-METRICS.json`): every listed file exists with the
    /// pinned hash, never overlaps the pinned five, and every `.tfm` in the
    /// directory is accounted for by one of the two tiers.
    func testSupplementaryMetricsMatchTheirInRepoPin() throws {
        let pinURL = Self.vendoredRoot.appendingPathComponent("SUPPLEMENTARY-METRICS.json")
        let doc = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: pinURL)) as? [String: Any])
        XCTAssertEqual(doc["schema_version"] as? Int, 1)
        let entries = try XCTUnwrap(doc["entries"] as? [[String: Any]])
        // Latin Modern (latin_modern_tfm): 23 roman + 10 typewriter
        // (ec-lmtt8/9/10/12 plus one 10 pt design each for
        // tti/tto/tcsc/tcso/tk/tko) + 25 sans/slanted/caps (ec-lmro at
        // 8/9/10/12/17 plus bxo10, csc10, csco10, u10, b10, bo10; ec-lmss and
        // ec-lmsso at 8/9/10/12/17 plus ssbx10, ssbo10, ssdc10, ssdo10) = 58.
        // T1 Computer Modern (ec_tfm_file): 70 roman (ecrm/ecbx/ecti/ecbi/ecsl
        // x 14 t1cmr.fd sizes) + 98 further roman shapes
        // (eccc/ecsc/ecoc/ecui/ecbl/ecrb/ecxc x the same 14) + 44 typewriter
        // (ectt/ecst/ecit/ectc) + 44 sans (ecss/ecsi/ecsx/ecso); the last two
        // groups take the 11 distinct sizes their .fd files reach, which
        // declare <5><6><7><8>#50800 so 5/6/7 pt share 0800 = 256.
        // Plus 6 AMS symbols (msbm/msam at 5/7/10 pt) and 2 license files
        // (ec, amsfonts). 58 + 256 + 6 + 2 = 322.
        XCTAssertEqual(entries.count, 322)
        let pinnedPaths = Set(Self.pinned.map(\.path))
        var listed = Set<String>()
        for e in entries {
            let path = try XCTUnwrap(e["path"] as? String)
            XCTAssertFalse(pinnedPaths.contains(path), "\(path) is Commander-pinned, not supplementary")
            XCTAssertTrue(listed.insert(path).inserted, "duplicate \(path)")
            let data = try Data(contentsOf: Self.vendoredRoot.appendingPathComponent(path))
            XCTAssertEqual(data.count, e["byte_length"] as? Int, path)
            XCTAssertEqual(Self.sha256Hex(data), e["sha256"] as? String, path)
        }
        // Every vendored TFM under each subdirectory this pin covers is
        // accounted for by exactly one tier (Commander-pinned or listed here).
        for subdirectory in [BundledMetrics.tfmSubdirectory, BundledMetrics.ecTfmSubdirectory, BundledMetrics.amsSymbolsSubdirectory] {
            let dir = Self.vendoredRoot.appendingPathComponent(subdirectory)
            let onDisk = Set(try FileManager.default.contentsOfDirectory(atPath: dir.path).filter { $0.hasSuffix(".tfm") }
                .map { subdirectory + "/" + $0 })
            XCTAssertEqual(onDisk, pinnedPaths.filter { $0.hasPrefix(subdirectory + "/") }.union(listed.filter { $0.hasPrefix(subdirectory + "/") }),
                          "every vendored TFM under \(subdirectory) is pinned by one tier")
        }
        // The Latin Modern faces the pin claims are exactly the files present.
        for (family, sizes) in try XCTUnwrap(doc["faces_covered"] as? [String: [Int]]) {
            for size in sizes {
                let name = "\(BundledMetrics.tfmSubdirectory)/\(family)\(size).tfm"
                XCTAssertTrue(listed.contains(name) || pinnedPaths.contains(name), "claimed face missing: \(name)")
            }
        }
        // The EC faces the pin claims (family -> 4-digit t1cmr.fd size codes) are exactly the files present.
        for (family, codes) in try XCTUnwrap(doc["ec_faces_covered"] as? [String: [String]]) {
            for code in codes {
                let name = "\(BundledMetrics.ecTfmSubdirectory)/\(family)\(code).tfm"
                XCTAssertTrue(listed.contains(name), "claimed EC face missing: \(name)")
            }
        }
        // The AMS symbol faces the pin claims are exactly the files present.
        for (family, sizes) in try XCTUnwrap(doc["ams_symbols_covered"] as? [String: [Int]]) {
            for size in sizes {
                let name = "\(BundledMetrics.amsSymbolsSubdirectory)/\(family)\(size).tfm"
                XCTAssertTrue(listed.contains(name), "claimed AMS symbol face missing: \(name)")
            }
        }
        // The two license files this pin adds are present alongside the metrics.
        XCTAssertTrue(listed.contains("doc/fonts/ec/copyrite.txt"))
        XCTAssertTrue(listed.contains("doc/fonts/amsfonts/README"))
    }

    // MARK: environment

    func testTFMDirectoryPrefersFirstRootThatExists() throws {
        let missing = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-no-texmf-\(UUID().uuidString)")
        let found = BundledMetrics.tfmDirectory(roots: [missing, Self.vendoredRoot])
        XCTAssertEqual(found?.path, Self.vendoredRoot.appendingPathComponent(BundledMetrics.tfmSubdirectory).standardizedFileURL.path)
        XCTAssertNil(BundledMetrics.tfmDirectory(roots: [missing]))
        // A file (not a directory) at the rooted path does not count.
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-texmf-file-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: file.appendingPathComponent("fonts/tfm/public"), withIntermediateDirectories: true)
        try Data("x".utf8).write(to: file.appendingPathComponent(BundledMetrics.tfmSubdirectory))
        defer { try? FileManager.default.removeItem(at: file) }
        XCTAssertNil(BundledMetrics.tfmDirectory(roots: [file]))
    }

    func testDefaultDiscoveryFindsTheRepositoryCopyFromASwiftBuildProduct() {
        // Tests run from a bare build product: the bundle has no Resources/texmf,
        // so the vendored repository tree is what the producer would get.
        XCTAssertEqual(BundledMetrics.tfmDirectory()?.path,
                       Self.vendoredRoot.appendingPathComponent(BundledMetrics.tfmSubdirectory).standardizedFileURL.path)
    }

    func testExplicitEntriesComeFirstAndTheBundledDirectoryIsTheFallback() {
        XCTAssertEqual(BundledMetrics.merged(bundled: "/b", withExplicit: nil), "/b")
        XCTAssertEqual(BundledMetrics.merged(bundled: "/b", withExplicit: ""), "/b")
        XCTAssertEqual(BundledMetrics.merged(bundled: "/b", withExplicit: "/u1:/u2"), "/u1:/u2:/b")
        XCTAssertEqual(BundledMetrics.merged(bundled: "/b", withExplicit: "/u1::/b:/u2:"), "/u1:/u2:/b", "empty entries dropped, bundled repeat dropped, bundled last")
    }

    func testProducerEnvironmentAppendsAndPassesEverythingElseThrough() {
        let base = ["PATH": "/usr/bin", "FLASHTEX_TFM_DIRS": "/user/tfm", "FLASHTEX_RENDER": "/x"]
        let env = BundledMetrics.producerEnvironment(base: base, bundledDirectories: [URL(fileURLWithPath: "/App/Contents/Resources/texmf/fonts/tfm/public/lm")])
        XCTAssertEqual(env["FLASHTEX_TFM_DIRS"], "/user/tfm:/App/Contents/Resources/texmf/fonts/tfm/public/lm")
        XCTAssertEqual(env["PATH"], "/usr/bin")
        XCTAssertEqual(env["FLASHTEX_RENDER"], "/x")
        XCTAssertEqual(env.count, 3)
        let unset = BundledMetrics.producerEnvironment(base: ["PATH": "/usr/bin"], bundledDirectories: [URL(fileURLWithPath: "/b")])
        XCTAssertEqual(unset["FLASHTEX_TFM_DIRS"], "/b")
    }

    func testProducerEnvironmentIsUntouchedWithoutABundledDirectory() {
        let base = ["PATH": "/usr/bin", "FLASHTEX_TFM_DIRS": "/user/tfm"]
        XCTAssertEqual(BundledMetrics.producerEnvironment(base: base, bundledDirectories: []), base)
    }

    /// The three bundled directories are appended in order (Latin Modern,
    /// EC, AMS symbols), each preserving everything appended before it.
    func testProducerEnvironmentAppendsAllThreeBundledDirectoriesInOrder() {
        let dirs = [URL(fileURLWithPath: "/App/lm"), URL(fileURLWithPath: "/App/ec"), URL(fileURLWithPath: "/App/ams")]
        let env = BundledMetrics.producerEnvironment(base: ["FLASHTEX_TFM_DIRS": "/user/tfm"], bundledDirectories: dirs)
        XCTAssertEqual(env["FLASHTEX_TFM_DIRS"], "/user/tfm:/App/lm:/App/ec:/App/ams")
    }

    func testDefaultBundledDirectoriesFindsAllThreeFromTheRepositoryCopy() {
        let dirs = BundledMetrics.defaultBundledDirectories(roots: [Self.vendoredRoot])
        XCTAssertEqual(dirs.map(\.path), [
            Self.vendoredRoot.appendingPathComponent(BundledMetrics.tfmSubdirectory).standardizedFileURL.path,
            Self.vendoredRoot.appendingPathComponent(BundledMetrics.ecTfmSubdirectory).standardizedFileURL.path,
            Self.vendoredRoot.appendingPathComponent(BundledMetrics.amsSymbolsSubdirectory).standardizedFileURL.path,
        ])
    }

    // MARK: real producer (env route, host TeX excluded)

    /// Missing-metric diagnostic codes the producer emits (render-pipeline
    /// `typeset.rs`): non-required TFM absent, required 12 pt set absent,
    /// Latin Modern face substituted.
    private static let missingMetricCodes: Set<String> = ["tfm_missing", "required_metrics_unavailable", "font_unavailable"]

    private struct ProducerRun {
        var results: Int
        var missing: [(id: String, code: String, message: String)]
        /// Every diagnostic (id, code, message) — for override checks that key on a named file.
        var all: [(id: String, code: String, message: String)] = []
    }

    /// Runs `FLASHTEX_RENDER` on the 10 pt multi-document request (the
    /// verified capture's input), 12 pt text, 12 pt math, 10 pt bold/italic
    /// styles and an 11 pt document, with an
    /// `env -i`-style environment (no PATH to texbin, empty HOME, no
    /// FLASHTEX_*/TEXMF*) and host TeX trees denied by sandbox-exec.
    private static let defaultRequests = [
        #"{"protocol_version":1,"id":"preview-1","type":"compile","payload":{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{"path":"chapter.tex","text":"Chapter text with \\(a+b\\).\n"},{"path":"main.tex","text":"\\documentclass{article}\n\\begin{document}\nOffice AV fi.\\input{chapter}\n\\end{document}\n"},{"path":"refs.bib","text":"@article{sample, title={Example}, author={A. Author}, year={2026}}\n"}]}}"#,
        #"{"protocol_version":1,"id":"text-12pt","type":"compile","payload":{"project_id":"p","revision":2,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass[12pt]{article}\n\\begin{document}\nOffice AV fi. Twelve point text.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}"#,
        #"{"protocol_version":1,"id":"math-12pt","type":"compile","payload":{"project_id":"p","revision":3,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass[12pt]{article}\n\\begin{document}\nBody $x^2 + y_1$ text. \\[ \\sum_{i=1}^{n} a_i \\]\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}"#,
        // Supplementary metrics: 10 pt bold/italic/bold-italic and an 11 pt document.
        #"{"protocol_version":1,"id":"styles-10pt","type":"compile","payload":{"project_id":"p","revision":4,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass{article}\n\\begin{document}\nRegular \\textbf{bold} \\textit{italic} \\textbf{\\textit{bold italic}} with $x_i^2$ text.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}"#,
        #"{"protocol_version":1,"id":"text-11pt","type":"compile","payload":{"project_id":"p","revision":5,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass[11pt]{article}\n\\begin{document}\nEleven point \\textbf{bold} \\textit{italic} text with $a+b$.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}"#,
    ]

    private func runProducer(_ executable: String, tfmDirs: String?, home: URL, requests: [String] = defaultRequests) throws -> ProducerRun {
        let profile = home.appendingPathComponent("no-host-tex.sb")
        try """
        (version 1)
        (allow default)
        (deny file-read* (subpath "/usr/local/texlive"))
        (deny file-read* (subpath "/Library/TeX"))
        (deny file-read* (subpath "/usr/share/texmf"))
        (deny file-read* (subpath "/usr/share/texlive"))

        """.write(to: profile, atomically: true, encoding: .utf8)
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/sandbox-exec")
        process.arguments = ["-f", profile.path, executable]
        var env = ["PATH": "/usr/bin:/bin", "HOME": home.path,
                   // The OTFs: a bare producer outside a bundle has no ../Resources/Fonts.
                   "FLASHTEX_FONT_DIRS": Self.macDir.appendingPathComponent("Fonts").path]
        if let tfmDirs { env["FLASHTEX_TFM_DIRS"] = tfmDirs }
        process.environment = env
        process.currentDirectoryURL = home
        let stdin = Pipe(), stdout = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = FileHandle.nullDevice
        try process.run()
        try stdin.fileHandleForWriting.write(contentsOf: Data((requests.joined(separator: "\n") + "\n").utf8))
        try stdin.fileHandleForWriting.close()
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        var run = ProducerRun(results: 0, missing: [])
        for line in output.split(separator: UInt8(ascii: "\n")) {
            guard let obj = try JSONSerialization.jsonObject(with: Data(line)) as? [String: Any],
                  obj["type"] as? String == "compile_result" else { continue }
            run.results += 1
            let id = obj["id"] as? String ?? "?"
            let diags = (obj["payload"] as? [String: Any])?["diagnostics"] as? [[String: Any]] ?? []
            for d in diags {
                let code = d["code"] as? String ?? ""
                run.all.append((id, code, d["message"] as? String ?? ""))
                if Self.missingMetricCodes.contains(code) {
                    run.missing.append((id, code, d["message"] as? String ?? ""))
                }
            }
        }
        return run
    }

    func testRealProducerHasNoMissingMetricsThroughTheEnvRouteWithHostTeXExcluded() throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"],
              FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        guard FileManager.default.isExecutableFile(atPath: "/usr/bin/sandbox-exec") else { throw XCTSkip("sandbox-exec unavailable") }
        let home = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-texmf-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: home) }

        // Exactly what WorkerClient / PreviewControllerClient hand the child.
        let env = BundledMetrics.producerEnvironment(base: [:], bundledDirectories: [BundledMetrics.tfmDirectory(roots: [Self.vendoredRoot])].compactMap { $0 })
        let tfmDirs = try XCTUnwrap(env[BundledMetrics.environmentKey])
        let routed = try runProducer(render, tfmDirs: tfmDirs, home: home)
        XCTAssertEqual(routed.results, 5)
        XCTAssertTrue(routed.missing.isEmpty, "env route must leave no missing-metric diagnostics: \(routed.missing)")

        // A POPULATED explicit user directory overrides the bundle for a
        // name-resolved metric: the user's ec-lmr10.tfm is truncated to one byte,
        // so the 10 pt request can only degrade if the producer read the user's
        // copy first (the intact bundled copy sits behind it in the list).
        let user = home.appendingPathComponent("user")
        let userLM = user.appendingPathComponent(BundledMetrics.tfmSubdirectory)
        try FileManager.default.createDirectory(at: userLM, withIntermediateDirectories: true)
        try Data([0]).write(to: userLM.appendingPathComponent("ec-lmr10.tfm"))
        let overridden = try runProducer(render, tfmDirs: BundledMetrics.merged(bundled: tfmDirs, withExplicit: userLM.path), home: home)
        XCTAssertEqual(overridden.results, 5)
        XCTAssertTrue(overridden.all.contains { $0.id == "preview-1" && $0.message.contains("ec-lmr10") },
                      "the explicit user copy must be the one read (a diagnostic naming ec-lmr10 on the 10 pt request): \(overridden.all)")
        // An unpopulated explicit directory changes nothing (bundle answers).
        let empty = try runProducer(render, tfmDirs: BundledMetrics.merged(bundled: tfmDirs, withExplicit: home.appendingPathComponent("empty").path), home: home)
        XCTAssertEqual(empty.results, 5)
        XCTAssertTrue(empty.missing.isEmpty, "\(empty.missing)")

        // One metric removed from a copy of the tree: an explicit failure, never silence.
        let copy = home.appendingPathComponent("texmf")
        try FileManager.default.copyItem(at: Self.vendoredRoot, to: copy)
        try FileManager.default.removeItem(at: copy.appendingPathComponent("fonts/tfm/public/lm/ec-lmr10.tfm"))
        let removed = try runProducer(render, tfmDirs: copy.appendingPathComponent(BundledMetrics.tfmSubdirectory).path, home: home)
        XCTAssertEqual(removed.results, 5)
        XCTAssertTrue(removed.missing.contains { $0.id == "preview-1" && $0.message.contains("ec-lmr10.tfm") },
                      "10 pt request must name the missing ec-lmr10.tfm: \(removed.missing)")
    }

    /// GH111 (PR #111): rendering a real `[T1]{fontenc}` document (no
    /// `lmodern`) with ONLY the app's bundled metric directories
    /// (`BundledMetrics.defaultBundledDirectories`: Latin Modern, EC, AMS
    /// symbols) and host TeX excluded must produce zero
    /// `ec_metrics_unavailable` diagnostics — the fixture the render
    /// pipeline falls back to the OTF-metrics warning for when the EC TFMs
    /// are not on its search path.
    func testHW1WithOnlyBundledDirectoriesProducesNoECMetricsUnavailableDiagnostic() throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"],
              FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        guard FileManager.default.isExecutableFile(atPath: "/usr/bin/sandbox-exec") else { throw XCTSkip("sandbox-exec unavailable") }
        let repoRoot = Self.macDir.deletingLastPathComponent().deletingLastPathComponent()
        let hw1URL = repoRoot.appendingPathComponent("fixtures/real-world/hw1/HW1.tex")
        let hw1 = try String(contentsOf: hw1URL, encoding: .utf8)
        XCTAssertTrue(hw1.contains("[T1]{fontenc}") && !hw1.contains("lmodern"),
                      "fixture must actually exercise the EC-metrics path (T1 fontenc, no lmodern)")

        let home = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-hw1-ec-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: home) }

        let payload: [String: Any] = ["protocol_version": 1, "id": "hw1", "type": "compile",
                                       "payload": ["project_id": "p", "revision": 1, "entry_path": "HW1.tex",
                                                   "documents": [["path": "HW1.tex", "text": hw1]],
                                                   "layout_capabilities": ["display-list-v2"]]]
        let request = try String(data: JSONSerialization.data(withJSONObject: payload), encoding: .utf8)!

        // Exactly BundledMetrics.producerEnvironment()'s default: every
        // bundled metrics directory this app ships, nothing else.
        let dirs = BundledMetrics.defaultBundledDirectories(roots: [Self.vendoredRoot])
        XCTAssertEqual(dirs.count, 3, "Latin Modern, EC and AMS symbols must all be present in the vendored tree")
        let tfmDirs = dirs.map(\.path).joined(separator: ":")

        let run = try runProducer(render, tfmDirs: tfmDirs, home: home, requests: [request])
        XCTAssertEqual(run.results, 1)
        let ecWarnings = run.all.filter { $0.code == "ec_metrics_unavailable" }
        XCTAssertTrue(ecWarnings.isEmpty, "bundled EC metrics must satisfy every T1 cmr face HW1 requests: \(ecWarnings)")
        XCTAssertTrue(run.missing.isEmpty, "\(run.missing)")

        // Control: without the EC directory (Latin Modern only), the same
        // document DOES warn — proving the assertion above is meaningful
        // and not vacuous (e.g. a producer that never emits the diagnostic).
        let lmOnly = try XCTUnwrap(dirs.first { $0.path.hasSuffix(BundledMetrics.tfmSubdirectory) }).path
        let controlRun = try runProducer(render, tfmDirs: lmOnly, home: home, requests: [request])
        XCTAssertEqual(controlRun.results, 1)
        XCTAssertFalse(controlRun.all.filter { $0.code == "ec_metrics_unavailable" }.isEmpty,
                       "control (no EC dir) must reproduce the fallback diagnostic HW1 triggers without bundled EC metrics")
    }
}
