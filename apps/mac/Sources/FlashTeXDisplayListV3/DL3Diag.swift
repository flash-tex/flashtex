import Foundation

/// `diag-v1` (docs/protocol/display-list-v3.md §6.7, lane P5-DIAGNOSTICS):
/// one `DIAG` message (kind 0x60, JSON) per error or warning of a compile,
/// for a client that put `"diag-v1"` in its HELLO's `accept`; such a client
/// gets no `DIAGNOSTIC` messages. Decoded as the reference decoder does
/// (`crates/display-list-v3/src/diag.rs`): unknown keys ignored, a missing
/// `message` is the only error.
public struct DL3Diag: Equatable, Sendable {
    public static let capability = "diag-v1"
    public static let kind: UInt8 = 0x60

    public struct Loc: Equatable, Sendable {
        public var file: String?, line: Int?, col: Int?, span: Int?
        static func from(_ j: DL3JSON) -> Loc {
            Loc(file: j["file"]?.string, line: j["line"]?.int.map(Int.init), col: j["col"]?.int.map(Int.init), span: j["span"]?.int.map(Int.init))
        }
    }

    /// One input level when TeX reported it, innermost first.
    public struct Frame: Equatable, Sendable {
        public var kind: String
        public var name: String?
        /// A file level's place (`col`: TeX's split).
        public var loc: Loc?
        /// What TeX had read of the level, and what was left.
        public var before: String, after: String
        /// A macro's definition site.
        public var def: Loc?
    }

    public var id: Int, seq: Int
    /// `error`, `warning`, `info` (nil: unknown).
    public var severity: String?
    public var code: String, origin: String
    public var package: String?
    public var message: String
    public var detail: String?
    public var file: String?
    /// 1-based line; 0-based byte column of TeX's split.
    public var line: Int?, col: Int?
    /// Byte columns `[from, to)` in `line`.
    public var range: (Int, Int)?
    public var offset: Int?, span: Int?
    public var end: Loc?
    public var lines: (Int, Int)?
    public var trace: [Frame]
    public var help: [String]
    public var fatal: Bool, output: Bool, exact: Bool

    public static func == (a: DL3Diag, b: DL3Diag) -> Bool {
        a.id == b.id && a.seq == b.seq && a.severity == b.severity && a.code == b.code && a.origin == b.origin
            && a.package == b.package && a.message == b.message && a.detail == b.detail && a.file == b.file
            && a.line == b.line && a.col == b.col && a.range?.0 == b.range?.0 && a.range?.1 == b.range?.1
            && a.offset == b.offset && a.span == b.span && a.end == b.end && a.lines?.0 == b.lines?.0 && a.lines?.1 == b.lines?.1
            && a.trace == b.trace && a.help == b.help && a.fatal == b.fatal && a.output == b.output && a.exact == b.exact
    }

    public static func decode(_ body: [UInt8]) throws -> DL3Diag { try from(DL3JSON.parse(body)) }

    public static func from(_ j: DL3JSON) throws -> DL3Diag {
        guard let message = j["message"]?.string else { throw DL3Error("DIAG without message") }
        func pair(_ v: DL3JSON?) -> (Int, Int)? {
            guard let a = v?.array, a.count >= 2, let x = a[0].int, let y = a[1].int else { return nil }
            return (Int(x), Int(y))
        }
        let severity = j["severity"]?.string.flatMap { ["error", "warning", "info"].contains($0) ? $0 : nil }
        let trace: [Frame] = (j["trace"]?.array ?? []).map { f in
            let text = f["text"]?.array ?? []
            let hasLoc = f["file"] != nil || f["line"] != nil
            return Frame(kind: f["kind"]?.string ?? "?", name: f["name"]?.string,
                         loc: hasLoc ? Loc.from(f) : nil,
                         before: text.count > 0 ? text[0].string ?? "" : "",
                         after: text.count > 1 ? text[1].string ?? "" : "",
                         def: f["def"].map(Loc.from))
        }
        return DL3Diag(
            id: Int(j["id"]?.int ?? 0), seq: Int(j["seq"]?.int ?? 0), severity: severity,
            code: j["code"]?.string ?? "", origin: j["origin"]?.string ?? "", package: j["package"]?.string,
            message: message, detail: j["detail"]?.string, file: j["file"]?.string,
            line: j["line"]?.int.map(Int.init), col: j["col"]?.int.map(Int.init), range: pair(j["range"]),
            offset: j["offset"]?.int.map(Int.init), span: j["span"]?.int.map(Int.init), end: j["end"].map(Loc.from),
            lines: pair(j["lines"]), trace: trace,
            help: (j["help"]?.array ?? []).compactMap(\.string),
            fatal: j["fatal"]?.bool ?? false, output: j["output"]?.bool ?? false, exact: j["exact"]?.bool ?? false)
    }
}
