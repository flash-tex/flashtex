import XCTest
@testable import FlashTeXEditorCore

/// PLAN M2 acceptance: `abbr | scope | packages | profile -> expected
/// snippet` golden tables for the whole text catalog, one file per area in
/// `Golden/`. A case:
///
///     === enum3
///     scope: text itemize        (text|math|display|preamble, then environments)
///     packages: amsmath          (already loaded; optional)
///     class: beamer              (optional)
///     profile: bold              (optional; default "default")
///     requires: graphicx         (missing packages reported; `-` for none)
///     ---
///     \begin{enumerate}
///     …
///
/// The expected block runs to the next `===`; trailing blank lines are
/// ignored. `error: …` expects a failure with that message. Run with
/// `TEXPAND_RECORD=1` to rewrite the expected blocks from the engine
/// (then review the diff).
final class GoldenTests: XCTestCase {
    typealias T = TeXpand

    struct Case {
        var abbreviation: String
        var headers: [(String, String)]
        var expected: String
        var line: Int
    }

    static var goldenDirectory: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Golden")
    }

    static func parse(_ text: String) -> (preamble: String, cases: [Case]) {
        var preamble: [String] = []
        var cases: [Case] = []
        var current: Case?
        var inBody = false
        var body: [String] = []
        func finish() {
            guard var c = current else { return }
            while body.last?.trimmingCharacters(in: .whitespaces).isEmpty == true { body.removeLast() }
            c.expected = body.joined(separator: "\n")
            cases.append(c)
            current = nil; body = []; inBody = false
        }
        for (n, line) in text.components(separatedBy: "\n").enumerated() {
            if line.hasPrefix("=== ") {
                finish()
                current = Case(abbreviation: String(line.dropFirst(4)), headers: [], expected: "", line: n + 1)
            } else if current == nil {
                preamble.append(line)
            } else if inBody {
                body.append(line)
            } else if line == "---" {
                inBody = true
            } else if let colon = line.firstIndex(of: ":") {
                current!.headers.append((String(line[..<colon]), line[line.index(after: colon)...].trimmingCharacters(in: .whitespaces)))
            }
        }
        finish()
        while preamble.last?.isEmpty == true { preamble.removeLast() }
        return (preamble.joined(separator: "\n"), cases)
    }

    var engines: [String: T.Engine] = [:]

    func engine(profile: String) -> T.Engine {
        if let e = engines[profile] { return e }
        var s = T.Settings()
        s.profile = profile
        let e = T.Engine(settings: s)
        engines[profile] = e
        return e
    }

    static func context(_ headers: [(String, String)]) -> T.Context {
        var ctx = T.Context()
        for (k, v) in headers {
            let words = v.split(separator: " ").map(String.init)
            switch k {
            case "scope":
                let envs = words.dropFirst().map { T.ScopeStack.Frame(.environment($0)) }
                switch words.first {
                case "math": ctx.scope = T.ScopeStack([.init(.document), .init(.inlineMath)] + envs)
                case "display": ctx.scope = T.ScopeStack([.init(.document), .init(.displayMath)] + envs)
                case "preamble": ctx.scope = T.ScopeStack([.init(.preamble)] + envs)
                default: ctx.scope = T.ScopeStack([.init(.document)] + envs)
                }
            case "packages": ctx.packages = Set(v.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) })
            case "class": ctx.documentClass = v
            case "indent": ctx.indentUnit = v == "tab" ? "\t" : String(repeating: " ", count: Int(v) ?? 2)
            default: break
            }
        }
        return ctx
    }

    func actual(_ c: Case) -> (text: String, requires: String) {
        let profile = c.headers.first { $0.0 == "profile" }?.1 ?? "default"
        let ctx = Self.context(c.headers)
        switch engine(profile: profile).expand(c.abbreviation, in: ctx) {
        case .success(let x):
            return (x.rendered(ctx), x.requires.isEmpty ? "-" : x.requires.map(\.name).joined(separator: ", "))
        case .failure(let f):
            return ("error: " + f.message, "-")
        }
    }

    func testGoldenTables() throws {
        let files = try FileManager.default.contentsOfDirectory(at: Self.goldenDirectory, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "golden" }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
        XCTAssertFalse(files.isEmpty, "no golden files in \(Self.goldenDirectory.path)")
        let record = ProcessInfo.processInfo.environment["TEXPAND_RECORD"] == "1"
        var total = 0
        for file in files {
            let text = try String(contentsOf: file, encoding: .utf8)
            let (preamble, cases) = Self.parse(text)
            var rewritten = preamble.isEmpty ? "" : preamble + "\n\n"
            for c in cases {
                total += 1
                let (got, requires) = actual(c)
                if record {
                    rewritten += "=== \(c.abbreviation)\n"
                    for (k, v) in c.headers where k != "requires" { rewritten += "\(k): \(v)\n" }
                    rewritten += "requires: \(requires)\n---\n\(got)\n\n"
                    continue
                }
                let where_ = "\(file.lastPathComponent):\(c.line) `\(c.abbreviation)`"
                XCTAssertEqual(got, c.expected, "\(where_)\n--- got ---\n\(got)\n--- expected ---\n\(c.expected)")
                let expectedRequires = c.headers.first { $0.0 == "requires" }?.1 ?? "-"
                XCTAssertEqual(requires, expectedRequires, "\(where_): requires")
            }
            if record { try rewritten.trimmingCharacters(in: .newlines).appending("\n").write(to: file, atomically: true, encoding: .utf8) }
        }
        XCTAssertGreaterThan(total, 50)
    }
}
