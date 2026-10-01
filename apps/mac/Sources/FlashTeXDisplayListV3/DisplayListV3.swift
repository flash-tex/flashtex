import Foundation

// display-list-v3 (docs/protocol/display-list-v3.md): the engine host's wire
// format, decoded. MIT, Foundation only: no engine code (DESIGN.md §3: the
// licence boundary is the host's process boundary), no AppKit/Core Graphics,
// so the same decoder serves any client platform. The reference decoder is
// the Rust crate `crates/display-list-v3` (`flashtex-display-list`); this one
// must decode every stream to the same values, which `DL3Canonical` (the
// canonical text of `dl3-dump --canonical`) checks on every parity fixture.
//
// Decoding fails closed (spec §7): a short body, an unknown item opcode, a
// reference to a path/matrix/unsupported entry that does not exist, or a
// colour of other than 1, 3 or 4 components throws; the caller treats the
// connection as broken and never draws a page it could not decode.

public enum DL3 {
    public static let protocolName = "display-list-v3"
    public static let versionMajor = 3
    /// 3.2: `external_tools`, `TOOL`, `cause` (bibtex, biber, makeindex in the host).
    public static let versionMinor = 2
    /// Scaled points per PDF point: 65536 × 72.27 / 72 = 6578176/100.
    public static let spPerBp = 65781.76
    public static let maxFrame = 1 << 28

    public enum Kind {
        public static let cHello: UInt8 = 0x01, compile: UInt8 = 0x02, cancel: UInt8 = 0x03, bye: UInt8 = 0x04
        public static let hello: UInt8 = 0x41, started: UInt8 = 0x42, font: UInt8 = 0x43, image: UInt8 = 0x44
        public static let page: UInt8 = 0x45, form: UInt8 = 0x46, sources: UInt8 = 0x47, diagnostic: UInt8 = 0x48
        public static let done: UInt8 = 0x49, error: UInt8 = 0x4A, pages: UInt8 = 0x4B
        /// 3.2: bibtex/biber/makeindex runs of a compile (spec §6.4).
        public static let tool: UInt8 = 0x4C

        /// The names `dl3-dump` and the Rust crate's `kind::name` use.
        public static func name(_ k: UInt8) -> String {
            switch k {
            case cHello: "client-hello"
            case compile: "compile"
            case cancel: "cancel"
            case bye: "bye"
            case hello: "hello"
            case started: "started"
            case font: "font"
            case image: "image"
            case page: "page"
            case form: "form"
            case sources: "sources"
            case diagnostic: "diagnostic"
            case done: "done"
            case error: "error"
            case pages: "pages"
            case tool: "tool"
            case DL3Diag.kind: "diag"
            default: "unknown"
            }
        }
    }
}

public struct DL3Error: Error, CustomStringConvertible, Equatable, Sendable {
    public var message: String
    public init(_ message: String) { self.message = message }
    public var description: String { message }
}

// MARK: - Bytes

/// Little-endian reader over a byte array (bounds-checked, never traps).
public struct DL3Reader {
    public let bytes: [UInt8]
    public private(set) var i: Int
    public let end: Int

    public init(_ bytes: [UInt8], from: Int = 0, to: Int? = nil) {
        self.bytes = bytes; i = from; end = to ?? bytes.count
    }

    public var left: Int { end - i }

    @inline(__always) mutating func need(_ n: Int) throws {
        guard n >= 0, i + n <= end else { throw DL3Error("truncated at byte \(i) (need \(n), have \(end - i))") }
    }
    public mutating func u8() throws -> UInt8 { try need(1); defer { i += 1 }; return bytes[i] }
    public mutating func u16() throws -> UInt16 {
        try need(2); defer { i += 2 }
        return UInt16(bytes[i]) | UInt16(bytes[i + 1]) << 8
    }
    public mutating func u32() throws -> UInt32 {
        try need(4); defer { i += 4 }
        return UInt32(bytes[i]) | UInt32(bytes[i + 1]) << 8 | UInt32(bytes[i + 2]) << 16 | UInt32(bytes[i + 3]) << 24
    }
    public mutating func i32() throws -> Int32 { Int32(bitPattern: try u32()) }
    public mutating func u64() throws -> UInt64 {
        let lo = UInt64(try u32()), hi = UInt64(try u32())
        return lo | hi << 32
    }
    public mutating func f64() throws -> Double { Double(bitPattern: try u64()) }
    public mutating func take(_ n: Int) throws -> ArraySlice<UInt8> {
        try need(n); defer { i += n }
        return bytes[i ..< i + n]
    }
    /// A `u32` count checked against the bytes left (each element at least
    /// `min` bytes), so a corrupt count cannot make a decoder allocate.
    public mutating func count(_ min: Int) throws -> Int {
        let n = Int(try u32())
        let (product, overflow) = n.multipliedReportingOverflow(by: Swift.max(min, 1))
        guard !overflow, product <= left else { throw DL3Error("count \(n) exceeds the data at byte \(i)") }
        return n
    }
}

// MARK: - Pages and forms (spec §4)

public struct DL3Matrix: Equatable, Sendable {
    public var a, b, c, d, e, f: Double
    public static let identity = DL3Matrix(a: 1, b: 0, c: 0, d: 1, e: 0, f: 0)
    public init(a: Double, b: Double, c: Double, d: Double, e: Double, f: Double) {
        self.a = a; self.b = b; self.c = c; self.d = d; self.e = e; self.f = f
    }
}

public enum DL3RuleKind: UInt8, Sendable { case fill = 0, strokeH = 1, strokeV = 2 }

public enum DL3Item: Equatable, Sendable {
    case glyph(font: UInt16, code: UInt16, x: Int32, y: Int32, col: UInt16)
    case rule(kind: DL3RuleKind, x: Int32, y: Int32, w: Int32, h: Int32)
    case path(UInt32)
    case clip(UInt32)
    case image(id: UInt32, matrix: UInt32)
    case form(id: UInt32, matrix: UInt32)
    case save
    case restore
    case fillColor([Double])
    case strokeColor([Double])
    case matrix(UInt32)
    case span(UInt32)
    case textRender(UInt8)
    case unsupported(UInt32)
}

public struct DL3Stroke: Equatable, Sendable {
    public var width: Double, cap: UInt8, join: UInt8, miter: Double, dash: [Double], phase: Double
}

public enum DL3Seg: Equatable, Sendable {
    case move(Double, Double)
    case line(Double, Double)
    case curve(Double, Double, Double, Double, Double, Double)
    case close
}

public struct DL3Path: Equatable, Sendable {
    public enum Paint {
        public static let fill: UInt8 = 1, fillEvenOdd: UInt8 = 2, stroke: UInt8 = 4, clip: UInt8 = 8, clipEvenOdd: UInt8 = 16
    }
    public var paint: UInt8
    public var matrix: UInt32
    public var stroke: DL3Stroke?
    public var segs: [DL3Seg]
}

public struct DL3Link: Equatable, Sendable {
    /// left, top, right, bottom (page space, sp).
    public var rect: [Int32]
    public var span: UInt32
    /// 1 goto name, 2 goto num, 3 goto page, 4 URI, 5 raw action, 6 thread
    /// (an unknown kind reads as 5, as the reference decoder does).
    public var kind: UInt8
    public var file: [UInt8]
    public var data: [UInt8]
    public var dataString: String { String(decoding: data, as: UTF8.self) }
}

public struct DL3Dest: Equatable, Sendable {
    public var named: Bool
    public var name: [UInt8]
    public var kind: UInt8
    public var rect: [Int32]
    public var zoom: Int32
}

public struct DL3Page: Equatable, Sendable {
    public enum StreamKind: Sendable { case page, form }
    public static let flagIncomplete: UInt32 = 1, flagNoGeometry: UInt32 = 2

    public var kind: StreamKind
    /// PAGE: 0-based ship-out index; FORM: the form's id.
    public var index: UInt32
    public var flags: UInt32 = 0
    /// PAGE: page width/height (sp); FORM: box width, height + depth (sp).
    public var width: Int32 = 0, height: Int32 = 0
    public var counts = [Int32](repeating: 0, count: 10)
    /// MediaBox (PAGE) or BBox (FORM), bp.
    public var box = [Double](repeating: 0, count: 4)
    public var hash = [UInt8](repeating: 0, count: 32)
    /// Matrices 1...n (0 is the identity and is not stored).
    public var matrices: [DL3Matrix] = []
    public var paths: [DL3Path] = []
    public var items: [DL3Item] = []
    public var links: [DL3Link] = []
    public var dests: [DL3Dest] = []
    public var unsupported: [String] = []

    public init(kind: StreamKind, index: UInt32) { self.kind = kind; self.index = index }

    public var incomplete: Bool { flags & Self.flagIncomplete != 0 }
    public var noGeometry: Bool { flags & Self.flagNoGeometry != 0 }
    /// Box width/height in bp.
    public var boxWidth: Double { box[2] - box[0] }
    public var boxHeight: Double { box[3] - box[1] }
    public var hashHex: String { DL3Hex.string(hash) }

    public func matrix(_ n: UInt32) -> DL3Matrix {
        n == 0 || Int(n) > matrices.count ? .identity : matrices[Int(n) - 1]
    }

    public static func decode(kind: StreamKind, body: [UInt8]) throws -> DL3Page {
        var c = DL3Reader(body)
        var p = DL3Page(kind: kind, index: try c.u32())
        p.flags = try c.u32()
        p.width = try c.i32()
        p.height = try c.i32()
        for k in 0 ..< 10 { p.counts[k] = try c.i32() }
        for k in 0 ..< 4 { p.box[k] = try c.f64() }
        p.hash = Array(try c.take(32))
        let n = try c.count(8)
        for _ in 0 ..< n {
            let tag = try c.u32()
            let len = Int(try c.u32())
            try c.need(len)
            var d = DL3Reader(body, from: c.i, to: c.i + len)
            _ = try c.take(len)
            switch tag {
            case 1: // MATRICES
                let m = try d.count(48)
                p.matrices.reserveCapacity(m)
                for _ in 0 ..< m {
                    p.matrices.append(DL3Matrix(a: try d.f64(), b: try d.f64(), c: try d.f64(), d: try d.f64(), e: try d.f64(), f: try d.f64()))
                }
            case 2: p.paths = try decodePaths(&d)
            case 3: p.items = try decodeItems(&d)
            case 4: // LINKS
                let m = try d.count(29)
                for _ in 0 ..< m {
                    let rect = [try d.i32(), try d.i32(), try d.i32(), try d.i32()]
                    let span = try d.u32()
                    let raw = try d.u8()
                    let kind: UInt8 = [1, 2, 3, 4, 6].contains(raw) ? raw : 5
                    let fl = Int(try d.u32()); let file = Array(try d.take(fl))
                    let dl = Int(try d.u32()); let data = Array(try d.take(dl))
                    p.links.append(DL3Link(rect: rect, span: span, kind: kind, file: file, data: data))
                }
            case 5: // DESTS
                let m = try d.count(26)
                for _ in 0 ..< m {
                    let named = try d.u8() != 0
                    let nl = Int(try d.u32()); let name = Array(try d.take(nl))
                    let kind = try d.u8()
                    let rect = [try d.i32(), try d.i32(), try d.i32(), try d.i32()]
                    let zoom = try d.i32()
                    p.dests.append(DL3Dest(named: named, name: name, kind: kind, rect: rect, zoom: zoom))
                }
            case 6: // UNSUPPORTED
                let m = try d.count(2)
                for _ in 0 ..< m {
                    let l = Int(try d.u16())
                    p.unsupported.append(String(decoding: try d.take(l), as: UTF8.self))
                }
            default: break // a later minor version's section: skipped
            }
        }
        // Every reference must resolve (fail closed).
        for it in p.items {
            switch it {
            case .path(let n), .clip(let n):
                if Int(n) >= p.paths.count { throw DL3Error("item names path \(n) of \(p.paths.count)") }
            case .matrix(let n), .image(_, let n), .form(_, let n):
                if Int(n) > p.matrices.count { throw DL3Error("item names matrix \(n) of \(p.matrices.count)") }
            case .unsupported(let n):
                if Int(n) >= p.unsupported.count { throw DL3Error("item names unsupported entry \(n)") }
            default: break
            }
        }
        for path in p.paths where Int(path.matrix) > p.matrices.count {
            throw DL3Error("path names matrix \(path.matrix)")
        }
        return p
    }

    static func decodePaths(_ d: inout DL3Reader) throws -> [DL3Path] {
        let n = try d.count(9)
        var out: [DL3Path] = []
        out.reserveCapacity(n)
        for _ in 0 ..< n {
            let paint = try d.u8()
            let matrix = try d.u32()
            var stroke: DL3Stroke?
            if paint & DL3Path.Paint.stroke != 0 {
                let width = try d.f64(), cap = try d.u8(), join = try d.u8(), miter = try d.f64()
                let nd = Int(try d.u16())
                var dash: [Double] = []
                for _ in 0 ..< nd { dash.append(try d.f64()) }
                stroke = DL3Stroke(width: width, cap: cap, join: join, miter: miter, dash: dash, phase: try d.f64())
            }
            let ns = try d.count(1)
            var segs: [DL3Seg] = []
            segs.reserveCapacity(ns)
            for _ in 0 ..< ns {
                switch try d.u8() {
                case 0: segs.append(.move(try d.f64(), try d.f64()))
                case 1: segs.append(.line(try d.f64(), try d.f64()))
                case 2: segs.append(.curve(try d.f64(), try d.f64(), try d.f64(), try d.f64(), try d.f64(), try d.f64()))
                case 3: segs.append(.close)
                case let o: throw DL3Error("unknown path segment \(o)")
                }
            }
            out.append(DL3Path(paint: paint, matrix: matrix, stroke: stroke, segs: segs))
        }
        return out
    }

    static func decodeItems(_ d: inout DL3Reader) throws -> [DL3Item] {
        var out: [DL3Item] = []
        out.reserveCapacity(d.left / 12)
        while d.left > 0 {
            let o = try d.u8()
            switch o {
            case 0x01: out.append(.glyph(font: try d.u16(), code: try d.u16(), x: try d.i32(), y: try d.i32(), col: try d.u16()))
            case 0x02:
                let k = try d.u8()
                guard let kind = DL3RuleKind(rawValue: k) else { throw DL3Error("unknown rule kind \(k)") }
                out.append(.rule(kind: kind, x: try d.i32(), y: try d.i32(), w: try d.i32(), h: try d.i32()))
            case 0x03: out.append(.path(try d.u32()))
            case 0x04: out.append(.clip(try d.u32()))
            case 0x05: out.append(.image(id: try d.u32(), matrix: try d.u32()))
            case 0x06: out.append(.form(id: try d.u32(), matrix: try d.u32()))
            case 0x07: out.append(.save)
            case 0x08: out.append(.restore)
            case 0x09, 0x0A:
                let n = Int(try d.u8())
                guard n == 1 || n == 3 || n == 4 else { throw DL3Error("colour with \(n) components") }
                var v: [Double] = []
                for _ in 0 ..< n { v.append(try d.f64()) }
                out.append(o == 0x09 ? .fillColor(v) : .strokeColor(v))
            case 0x0B: out.append(.matrix(try d.u32()))
            case 0x0C: out.append(.span(try d.u32()))
            case 0x0D: out.append(.textRender(try d.u8()))
            case 0x0E: out.append(.unsupported(try d.u32()))
            default: throw DL3Error("unknown item opcode 0x\(String(o, radix: 16))")
            }
        }
        return out
    }
}

// MARK: - JSON (control messages)

/// A JSON value, Sendable (control messages cross threads).
public enum DL3JSON: Equatable, Sendable {
    case null
    case bool(Bool)
    case int(Int64)
    case double(Double)
    case string(String)
    case array([DL3JSON])
    case object([String: DL3JSON])

    public static func parse(_ bytes: some Collection<UInt8>) throws -> DL3JSON {
        let data = Data(bytes)
        guard String(data: data, encoding: .utf8) != nil else { throw DL3Error("JSON body is not UTF-8") }
        do {
            return from(try JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed]))
        } catch {
            throw DL3Error("bad JSON: \(error.localizedDescription)")
        }
    }

    static func from(_ any: Any) -> DL3JSON {
        switch any {
        case is NSNull: return .null
        case let s as String: return .string(s)
        case let n as NSNumber:
            if CFGetTypeID(n) == CFBooleanGetTypeID() { return .bool(n.boolValue) }
            if CFNumberIsFloatType(n) {
                let d = n.doubleValue
                return d == d.rounded() && abs(d) < 9e15 ? .int(Int64(d)) : .double(d)
            }
            return .int(n.int64Value)
        case let a as [Any]: return .array(a.map(from))
        case let o as [String: Any]: return .object(o.mapValues(from))
        default: return .null
        }
    }

    public subscript(key: String) -> DL3JSON? { if case .object(let o) = self { o[key] } else { nil } }
    public var string: String? { if case .string(let s) = self { s } else { nil } }
    public var int: Int64? {
        switch self {
        case .int(let i): i
        case .double(let d) where d == d.rounded(): Int64(d)
        default: nil
        }
    }
    public var double: Double? {
        switch self {
        case .int(let i): Double(i)
        case .double(let d): d
        default: nil
        }
    }
    public var bool: Bool? { if case .bool(let b) = self { b } else { nil } }
    public var array: [DL3JSON]? { if case .array(let a) = self { a } else { nil } }

    /// Serialises for the socket (JSONSerialization; keys sorted, so a
    /// request's bytes are deterministic).
    public func data() -> Data {
        (try? JSONSerialization.data(withJSONObject: foundation, options: [.fragmentsAllowed, .sortedKeys, .withoutEscapingSlashes])) ?? Data("null".utf8)
    }

    var foundation: Any {
        switch self {
        case .null: NSNull()
        case .bool(let b): b
        case .int(let i): i
        case .double(let d): d
        case .string(let s): s
        case .array(let a): a.map(\.foundation)
        case .object(let o): o.mapValues(\.foundation)
        }
    }
}

// MARK: - Resources (spec §5)

public struct DL3Font: Sendable {
    public var id: UInt16
    public var key: [UInt8]
    public var info: DL3JSON
    public var program: [UInt8]

    public var keyHex: String { DL3Hex.string(key) }
    public var format: String? { info["format"]?.string }
    public var pdfName: String? { info["pdf_name"]?.string }
    public var psName: String? { info["ps_name"]?.string }
    /// 256 glyph names (code → name), or nil when the font sent none.
    public var encoding: [String?]? {
        guard let a = info["encoding"]?.array else { return nil }
        return (0 ..< 256).map { $0 < a.count ? a[$0].string : nil }
    }
    /// The map's SlantFont/ExtendFont × 1000 (0 = none).
    public var slant: Int { Int(info["slant"]?.int ?? 0) }
    public var extend: Int { Int(info["extend"]?.int ?? 0) }
    /// When slanted or extended: the `/FontMatrix` pdfTeX writes, six numbers.
    public var fontMatrix: [Double]? {
        guard let s = info["font_matrix"]?.string else { return nil }
        let v = s.split(whereSeparator: { $0 == " " || $0 == "[" || $0 == "]" }).compactMap { Double($0) }
        return v.count == 6 ? v : nil
    }

    public static func decode(_ body: [UInt8]) throws -> DL3Font {
        var c = DL3Reader(body)
        let id = try c.u32()
        guard id <= UInt32(UInt16.max) else { throw DL3Error("font id \(id) out of range") }
        let key = Array(try c.take(32))
        let jl = Int(try c.u32())
        let info = try DL3JSON.parse(try c.take(jl))
        let pl = Int(try c.u32())
        let program = Array(try c.take(pl))
        return DL3Font(id: UInt16(id), key: key, info: info, program: program)
    }
}

public struct DL3Sources: Equatable, Sendable {
    public var files: [(UInt32, String)] = []
    public var spans: [(UInt32, UInt32, UInt32)] = []

    public static func == (a: DL3Sources, b: DL3Sources) -> Bool {
        a.files.elementsEqual(b.files, by: { $0 == $1 }) && a.spans.elementsEqual(b.spans, by: { $0 == $1 })
    }

    public static func from(_ j: DL3JSON) throws -> DL3Sources {
        var s = DL3Sources()
        func u32(_ v: DL3JSON) -> UInt32? {
            guard case .int(let i) = v, i >= 0, i <= Int64(UInt32.max) else { return nil }
            return UInt32(i)
        }
        for f in j["files"]?.array ?? [] {
            guard let a = f.array else { throw DL3Error("files entry is not an array") }
            guard a.count == 2 else { throw DL3Error("files entry is not [id, path]") }
            guard let id = u32(a[0]) else { throw DL3Error("bad file id") }
            guard let p = a[1].string else { throw DL3Error("bad path") }
            s.files.append((id, p))
        }
        for e in j["spans"]?.array ?? [] {
            guard let a = e.array else { throw DL3Error("spans entry is not an array") }
            guard a.count == 3 else { throw DL3Error("spans entry is not [id, file, line]") }
            guard let id = u32(a[0]) else { throw DL3Error("bad span id") }
            guard let f = u32(a[1]) else { throw DL3Error("bad span file") }
            guard let l = u32(a[2]) else { throw DL3Error("bad span line") }
            s.spans.append((id, f, l))
        }
        return s
    }
}

// MARK: - Events

/// One decoded host message.
public enum DL3Event: Sendable {
    case hello(DL3JSON)
    case started(DL3JSON)
    case font(DL3Font)
    case image(DL3JSON)
    case page(DL3Page)
    case form(DL3Page)
    case sources(DL3Sources)
    case diagnostic(DL3JSON)
    case done(DL3JSON)
    case error(DL3JSON)
    case pages(DL3JSON)
    /// 3.2: `{"id", "event": run|done|skip|settled, ...}` (§6.4).
    case tool(DL3JSON)
    /// `diag-v1` (§6.7), for a client that accepted it.
    case diag(DL3Diag)
    /// A kind this version does not know (a later minor version's): skipped.
    case other(UInt8, Int)

    public static func decode(kind k: UInt8, body: [UInt8]) throws -> DL3Event {
        switch k {
        case DL3.Kind.hello: .hello(try DL3JSON.parse(body))
        case DL3.Kind.started: .started(try DL3JSON.parse(body))
        case DL3.Kind.font: .font(try DL3Font.decode(body))
        case DL3.Kind.image: .image(try DL3JSON.parse(body))
        case DL3.Kind.page: .page(try DL3Page.decode(kind: .page, body: body))
        case DL3.Kind.form: .form(try DL3Page.decode(kind: .form, body: body))
        case DL3.Kind.sources: .sources(try DL3Sources.from(try DL3JSON.parse(body)))
        case DL3.Kind.diagnostic: .diagnostic(try DL3JSON.parse(body))
        case DL3.Kind.done: .done(try DL3JSON.parse(body))
        case DL3.Kind.error: .error(try DL3JSON.parse(body))
        case DL3.Kind.pages: .pages(try DL3JSON.parse(body))
        case DL3.Kind.tool: .tool(try DL3JSON.parse(body))
        case DL3Diag.kind: .diag(try DL3Diag.decode(body))
        default: .other(k, body.count)
        }
    }
}

// MARK: - Frames

public enum DL3Frames {
    /// Splits a file of frames (back to back, no HELLO: spec §6.6).
    public static func split(_ bytes: [UInt8]) throws -> [(kind: UInt8, body: [UInt8])] {
        var out: [(UInt8, [UInt8])] = []
        var r = DL3Reader(bytes)
        while r.left > 0 {
            guard r.left >= 5 else { throw DL3Error("truncated frame header") }
            let len = Int(try r.u32())
            guard len >= 1, len <= DL3.maxFrame else { throw DL3Error("bad frame length \(len)") }
            guard r.left >= len else { throw DL3Error("truncated frame") }
            let k = try r.u8()
            out.append((k, Array(try r.take(len - 1))))
        }
        return out
    }

    /// One frame's bytes.
    public static func encode(kind: UInt8, body: Data) -> Data {
        var d = Data(capacity: body.count + 5)
        let len = UInt32(body.count + 1)
        d.append(contentsOf: [UInt8(len & 0xFF), UInt8(len >> 8 & 0xFF), UInt8(len >> 16 & 0xFF), UInt8(len >> 24), kind])
        d.append(body)
        return d
    }
}

public enum DL3Hex {
    static let digits = Array("0123456789abcdef".utf8)
    public static func string(_ bytes: some Sequence<UInt8>) -> String {
        var out: [UInt8] = []
        for b in bytes { out.append(digits[Int(b >> 4)]); out.append(digits[Int(b & 15)]) }
        return String(decoding: out, as: UTF8.self)
    }
}
