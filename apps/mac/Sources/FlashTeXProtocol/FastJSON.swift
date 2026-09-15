import Foundation

/// A small recursive-descent JSON reader over UTF-8 bytes, used for the
/// `compile_result` hot path. `JSONDecoder` + `Codable` decoded a 1.6 MB
/// result (10.7k items, 60 KB source) in ~77 ms — the largest single cost per
/// keystroke on big documents; this reader builds the same `RuntimeV1` values
/// directly from the bytes. It accepts exactly RFC 8259 JSON; anything it does
/// not understand throws `FastJSON.Error` and the caller falls back to
/// `JSONDecoder`, whose error then stands (the fast path never changes which
/// inputs are rejected, only how fast valid ones are read).
///
/// Number handling mirrors `JSONDecoder`: integers must be integer literals
/// within `Int`; doubles are parsed with the correctly rounded `Double(_:)`.
/// Duplicate object keys keep the last value. Unknown keys are ignored.
public struct FastJSON {
    public struct Error: Swift.Error, CustomStringConvertible {
        public var offset: Int
        public var message: String
        public var description: String { "fast JSON: \(message) at byte \(offset)" }
    }

    /// Generic value for small frames (helper envelopes); `raw` keeps the byte
    /// range of a value that a typed reader parses separately.
    public indirect enum Value: Equatable {
        case null
        case bool(Bool)
        case number(Double, isInteger: Bool)
        case string(String)
        case array([Value])
        case object([String: Value])
        case raw(Range<Int>)

        public var string: String? { if case .string(let s) = self { return s } else { return nil } }
        public var int: Int? {
            if case .number(let d, let isInt) = self, isInt, d >= Double(Int.min), d <= Double(Int.max) { return Int(d) }
            return nil
        }
        public var double: Double? { if case .number(let d, _) = self { return d } else { return nil } }
        public var object: [String: Value]? { if case .object(let o) = self { return o } else { return nil } }
        public var array: [Value]? { if case .array(let a) = self { return a } else { return nil } }
        public var isNull: Bool { if case .null = self { return true } else { return false } }
    }

    private let bytes: UnsafeBufferPointer<UInt8>
    private var i: Int
    /// Object keys whose values are not parsed but returned as `.raw` ranges
    /// (top-level generic parsing only; typed readers ignore it).
    private let rawKeys: Set<String>

    private init(_ bytes: UnsafeBufferPointer<UInt8>, rawKeys: Set<String>) {
        self.bytes = bytes; self.i = 0; self.rawKeys = rawKeys
    }

    // MARK: entry points

    /// Parses one JSON document into a generic value.
    public static func parse(_ data: Data, rawKeys: Set<String> = []) throws -> Value {
        try data.withUnsafeBytes { raw -> Value in
            let buf = raw.bindMemory(to: UInt8.self)
            var p = FastJSON(buf, rawKeys: rawKeys)
            p.skipWhitespace()
            let v = try p.parseValue(depth: 0)
            p.skipWhitespace()
            guard p.i == buf.count else { throw p.error("trailing characters") }
            return v
        }
    }

    /// Parses a runtime-v1 `compile_result` envelope (the `pages` array is
    /// the bulk of the bytes). Throws for anything outside RFC 8259 or the
    /// contract's required shape; the caller falls back to `JSONDecoder`.
    public static func compileResultEnvelope(_ data: Data) throws -> RuntimeV1.Envelope<RuntimeV1.CompileResult> {
        try data.withUnsafeBytes { raw -> RuntimeV1.Envelope<RuntimeV1.CompileResult> in
            let buf = raw.bindMemory(to: UInt8.self)
            var p = FastJSON(buf, rawKeys: [])
            let env = try p.parseCompileResultEnvelope()
            p.skipWhitespace()
            guard p.i == buf.count else { throw p.error("trailing characters") }
            return env
        }
    }

    /// Same, for a sub-range of a larger frame (a helper `update.result`).
    public static func compileResultEnvelope(_ data: Data, range: Range<Int>) throws -> RuntimeV1.Envelope<RuntimeV1.CompileResult> {
        try data.withUnsafeBytes { raw -> RuntimeV1.Envelope<RuntimeV1.CompileResult> in
            let whole = raw.bindMemory(to: UInt8.self)
            guard range.lowerBound >= 0, range.upperBound <= whole.count else {
                throw Error(offset: range.lowerBound, message: "raw range outside the frame")
            }
            let sub = UnsafeBufferPointer(rebasing: whole[range])
            var p = FastJSON(sub, rawKeys: [])
            let env = try p.parseCompileResultEnvelope()
            p.skipWhitespace()
            guard p.i == sub.count else { throw p.error("trailing characters") }
            return env
        }
    }

    // MARK: typed readers

    private mutating func parseCompileResultEnvelope() throws -> RuntimeV1.Envelope<RuntimeV1.CompileResult> {
        var protocolVersion: Int?, id: String?, type: String?, payload: RuntimeV1.CompileResult?
        try parseObject { key, p in
            switch key {
            case "protocol_version": protocolVersion = try p.parseInt()
            case "id": id = try p.parseString()
            case "type": type = try p.parseString()
            case "payload": payload = try p.parseCompileResult()
            default: try p.skipValue(depth: 1)
            }
        }
        guard let protocolVersion else { throw error("missing protocol_version") }
        guard let id else { throw error("missing id") }
        guard let type else { throw error("missing type") }
        guard let payload else { throw error("missing payload") }
        return RuntimeV1.Envelope(protocolVersion: protocolVersion, id: id, type: type, payload: payload)
    }

    private mutating func parseCompileResult() throws -> RuntimeV1.CompileResult {
        var projectId: String?, revision: Int?, status: RuntimeV1.Status?
        var pages: [RuntimeV1.Page]?, diagnostics: [RuntimeV1.Diagnostic]?
        var pdfPath: String?, layoutCapabilities: [String]?
        try parseObject { key, p in
            switch key {
            case "project_id": projectId = try p.parseString()
            case "revision": revision = try p.parseInt()
            case "status":
                let s = try p.parseString()
                guard let st = RuntimeV1.Status(rawValue: s) else { throw p.error("unknown status \(s)") }
                status = st
            case "pages": pages = try p.parseArray { try $0.parsePage() }
            case "diagnostics": diagnostics = try p.parseArray { try $0.parseDiagnostic() }
            case "pdf_path": pdfPath = try p.parseOptionalString()
            case "layout_capabilities":
                if p.peekNull() { try p.parseNull(); layoutCapabilities = nil }
                else { layoutCapabilities = try p.parseArray { try $0.parseString() } }
            default: try p.skipValue(depth: 2)
            }
        }
        guard let projectId else { throw error("missing project_id") }
        guard let revision else { throw error("missing revision") }
        guard let status else { throw error("missing status") }
        guard let pages else { throw error("missing pages") }
        guard let diagnostics else { throw error("missing diagnostics") }
        if let caps = layoutCapabilities { try RuntimeV1.LayoutCapabilities.validate(caps) }
        return RuntimeV1.CompileResult(projectId: projectId, revision: revision, status: status, pages: pages,
                                       diagnostics: diagnostics, pdfPath: pdfPath, layoutCapabilities: layoutCapabilities)
    }

    private mutating func parsePage() throws -> RuntimeV1.Page {
        var number: Int?, width: Double?, height: Double?, items: [RuntimeV1.PageItem]?
        try parseObject { key, p in
            switch key {
            case "number": number = try p.parseInt()
            case "width_pt": width = try p.parseDouble()
            case "height_pt": height = try p.parseDouble()
            case "items": items = try p.parseArray { try $0.parsePageItem() }
            default: try p.skipValue(depth: 3)
            }
        }
        guard let number else { throw error("page missing number") }
        guard let width else { throw error("page missing width_pt") }
        guard let height else { throw error("page missing height_pt") }
        guard let items else { throw error("page missing items") }
        return RuntimeV1.Page(number: number, widthPt: width, heightPt: height, items: items)
    }

    /// One `items[]` element. Every key is read once (the item is a flat
    /// object); the `kind` decides which fields are required.
    private mutating func parsePageItem() throws -> RuntimeV1.PageItem {
        var kind: String?
        var text: String?, xPt: Double?, baselineYPt: Double?, fontSizePt: Double?
        var yPt: Double?, widthPt: Double?, heightPt: Double?
        var source: RuntimeV1.SourceRange?? = nil // nil = absent, .some(nil) = null, .some(.some) = present
        var sourceMalformed = false
        var font: RuntimeV1.PageItem.FontHint?
        var fontMalformed: Swift.Error?
        try parseObject { key, p in
            switch key {
            case "kind": kind = try p.parseString()
            case "text": text = try p.parseString()
            case "x_pt": xPt = try p.parseDouble()
            case "baseline_y_pt": baselineYPt = try p.parseDouble()
            case "font_size_pt": fontSizePt = try p.parseDouble()
            case "y_pt": yPt = try p.parseDouble()
            case "width_pt": widthPt = try p.parseDouble()
            case "height_pt": heightPt = try p.parseDouble()
            case "source":
                if p.peekNull() { try p.parseNull(); source = .some(nil) }
                else {
                    let start = p.i
                    do { source = .some(try p.parseSourceRange()) }
                    catch {
                        // `unknown` kinds tolerate a malformed source (decoded with try?);
                        // typed kinds do not. Skip it and remember.
                        p.i = start
                        try p.skipValue(depth: 4)
                        sourceMalformed = true
                    }
                }
            case "font":
                if p.peekNull() { try p.parseNull(); font = nil }
                else {
                    let start = p.i
                    do { font = try p.parseFontHint() }
                    catch {
                        p.i = start
                        try p.skipValue(depth: 4)
                        fontMalformed = error
                    }
                }
            default: try p.skipValue(depth: 4)
            }
        }
        guard let kind else { throw error("item missing kind") }
        switch kind {
        case "text":
            guard !sourceMalformed else { throw error("text item source malformed") }
            if let fontMalformed { throw fontMalformed }
            guard let text else { throw error("text item missing text") }
            guard let xPt else { throw error("text item missing x_pt") }
            guard let baselineYPt else { throw error("text item missing baseline_y_pt") }
            guard let fontSizePt else { throw error("text item missing font_size_pt") }
            return .text(.init(text: text, xPt: xPt, baselineYPt: baselineYPt, fontSizePt: fontSizePt,
                               source: source ?? nil, font: font))
        case "rule":
            guard !sourceMalformed else { throw error("rule item source malformed") }
            guard let xPt else { throw error("rule missing x_pt") }
            guard let yPt else { throw error("rule missing y_pt") }
            guard let widthPt else { throw error("rule missing width_pt") }
            guard let heightPt else { throw error("rule missing height_pt") }
            for v in [xPt, yPt, widthPt, heightPt] {
                guard v.isFinite, abs(v) <= RuntimeV1.PageItem.RuleItem.maxMagnitude else { throw error("rule magnitude") }
            }
            guard widthPt > 0, heightPt > 0 else { throw error("rule width_pt and height_pt must be positive") }
            return .rule(.init(xPt: xPt, yPt: yPt, widthPt: widthPt, heightPt: heightPt, source: source ?? nil))
        default:
            return .unknown(kind: kind, source: source ?? nil)
        }
    }

    private mutating func parseSourceRange() throws -> RuntimeV1.SourceRange {
        var path: String?, start: Int?, end: Int?
        try parseObject { key, p in
            switch key {
            case "path": path = try p.parseString()
            case "start_byte": start = try p.parseInt()
            case "end_byte": end = try p.parseInt()
            default: try p.skipValue(depth: 5)
            }
        }
        guard let path else { throw error("source missing path") }
        guard let start else { throw error("source missing start_byte") }
        guard let end else { throw error("source missing end_byte") }
        return RuntimeV1.SourceRange(path: path, startByte: start, endByte: end)
    }

    private mutating func parseFontHint() throws -> RuntimeV1.PageItem.FontHint {
        var family: String?, weight: RuntimeV1.PageItem.FontHint.Weight?, style: RuntimeV1.PageItem.FontHint.Style?
        try parseObject { key, p in
            switch key {
            case "family": family = try p.parseString()
            case "weight":
                let s = try p.parseString()
                guard let w = RuntimeV1.PageItem.FontHint.Weight(rawValue: s) else { throw p.error("unknown weight \(s)") }
                weight = w
            case "style":
                let s = try p.parseString()
                guard let st = RuntimeV1.PageItem.FontHint.Style(rawValue: s) else { throw p.error("unknown style \(s)") }
                style = st
            default: try p.skipValue(depth: 5)
            }
        }
        guard let family else { throw error("font missing family") }
        guard let weight else { throw error("font missing weight") }
        guard let style else { throw error("font missing style") }
        // Same limits as FontHint.init(from:).
        guard !family.isEmpty, family.utf8.count <= RuntimeV1.PageItem.FontHint.maxFamilyBytes else {
            throw error("font family must be 1...\(RuntimeV1.PageItem.FontHint.maxFamilyBytes) UTF-8 bytes")
        }
        guard !family.unicodeScalars.contains(where: { $0.properties.generalCategory == .control }) else {
            throw error("font family must not contain control characters")
        }
        return RuntimeV1.PageItem.FontHint(family: family, weight: weight, style: style)
    }

    private mutating func parseDiagnostic() throws -> RuntimeV1.Diagnostic {
        var severity: RuntimeV1.Severity?, message: String?, source: RuntimeV1.SourceRange?, recovery: String?
        var code: String?, suggestion: String?, labels: [RuntimeV1.Diagnostic.Label]?, notes: [String]?
        var help: RuntimeV1.Diagnostic.Help?
        try parseObject { key, p in
            switch key {
            case "severity":
                let s = try p.parseString()
                guard let sev = RuntimeV1.Severity(rawValue: s) else { throw p.error("unknown severity \(s)") }
                severity = sev
            case "message": message = try p.parseString()
            case "source":
                if p.peekNull() { try p.parseNull() } else { source = try p.parseSourceRange() }
            case "recovery": recovery = try p.parseOptionalString()
            case "code": code = try p.parseOptionalString()
            case "suggestion": suggestion = try p.parseOptionalString()
            case "labels":
                if p.peekNull() { try p.parseNull() } else { labels = try p.parseArray { try $0.parseDiagnosticLabel() } }
            case "notes":
                if p.peekNull() { try p.parseNull() } else { notes = try p.parseArray { try $0.parseString() } }
            case "help":
                if p.peekNull() { try p.parseNull() } else { help = try p.parseDiagnosticHelp() }
            default: try p.skipValue(depth: 3)
            }
        }
        guard let severity else { throw error("diagnostic missing severity") }
        guard let message else { throw error("diagnostic missing message") }
        return RuntimeV1.Diagnostic(severity: severity, message: message, source: source, recovery: recovery,
                                    code: code, suggestion: suggestion, labels: labels, notes: notes, help: help)
    }

    private mutating func parseDiagnosticLabel() throws -> RuntimeV1.Diagnostic.Label {
        var source: RuntimeV1.SourceRange?, text: String?, primary: Bool?
        try parseObject { key, p in
            switch key {
            case "source": source = try p.parseSourceRange()
            case "text": text = try p.parseString()
            case "primary": primary = try p.parseBool()
            default: try p.skipValue(depth: 3)
            }
        }
        guard let source else { throw error("label missing source") }
        guard let text else { throw error("label missing text") }
        guard let primary else { throw error("label missing primary") }
        return .init(source: source, text: text, primary: primary)
    }

    private mutating func parseDiagnosticHelp() throws -> RuntimeV1.Diagnostic.Help {
        var message: String?, replacement: RuntimeV1.Diagnostic.Replacement?
        try parseObject { key, p in
            switch key {
            case "message": message = try p.parseString()
            case "replacement":
                if p.peekNull() { try p.parseNull() } else { replacement = try p.parseDiagnosticReplacement() }
            default: try p.skipValue(depth: 3)
            }
        }
        guard let message else { throw error("help missing message") }
        return .init(message: message, replacement: replacement)
    }

    private mutating func parseDiagnosticReplacement() throws -> RuntimeV1.Diagnostic.Replacement {
        // The compiler nests the edit's range in `source` (the same object as
        // `labels[].source`); a flat `start_byte`/`end_byte` pair is also
        // accepted, and wins when both are present.
        var start: Int?, end: Int?, text: String?, path: String?
        var srcPath: String?, srcStart: Int?, srcEnd: Int?
        try parseObject { key, p in
            switch key {
            case "start_byte": start = try p.parseInt()
            case "end_byte": end = try p.parseInt()
            case "text": text = try p.parseString()
            case "path": path = try p.parseOptionalString()
            case "source":
                if p.peekNull() { try p.parseNull() } else {
                    let src = try p.parseSourceRange()
                    srcPath = src.path; srcStart = src.startByte; srcEnd = src.endByte
                }
            default: try p.skipValue(depth: 3)
            }
        }
        guard let start = start ?? srcStart else { throw error("replacement missing start_byte") }
        guard let end = end ?? srcEnd else { throw error("replacement missing end_byte") }
        guard let text else { throw error("replacement missing text") }
        let resolved = (path?.isEmpty == false) ? path : srcPath
        return .init(startByte: start, endByte: end, text: text, path: resolved)
    }

    private mutating func parseBool() throws -> Bool {
        if peek(UInt8(ascii: "t")) { try expectLiteral("true"); return true }
        if peek(UInt8(ascii: "f")) { try expectLiteral("false"); return false }
        throw error("expected a boolean")
    }

    // MARK: generic

    private static let maxDepth = 64

    private mutating func parseValue(depth: Int) throws -> Value {
        guard depth < Self.maxDepth else { throw error("nesting too deep") }
        guard i < bytes.count else { throw error("unexpected end") }
        switch bytes[i] {
        case UInt8(ascii: "{"):
            var out: [String: Value] = [:]
            try parseObject { key, p in
                if p.rawKeys.contains(key) {
                    let start = p.i
                    try p.skipValue(depth: depth + 1)
                    out[key] = .raw(start..<p.i)
                } else {
                    out[key] = try p.parseValue(depth: depth + 1)
                }
            }
            return .object(out)
        case UInt8(ascii: "["):
            return .array(try parseArray { try $0.parseValue(depth: depth + 1) })
        case UInt8(ascii: "\""):
            return .string(try parseString())
        case UInt8(ascii: "t"):
            try expectLiteral("true"); return .bool(true)
        case UInt8(ascii: "f"):
            try expectLiteral("false"); return .bool(false)
        case UInt8(ascii: "n"):
            try expectLiteral("null"); return .null
        default:
            let (d, isInt) = try parseNumber()
            return .number(d, isInteger: isInt)
        }
    }

    /// Skips one value without building it (unknown keys, raw ranges).
    private mutating func skipValue(depth: Int) throws {
        guard depth < Self.maxDepth else { throw error("nesting too deep") }
        guard i < bytes.count else { throw error("unexpected end") }
        switch bytes[i] {
        case UInt8(ascii: "{"):
            try parseObject { _, p in try p.skipValue(depth: depth + 1) }
        case UInt8(ascii: "["):
            _ = try parseArray { try $0.skipValue(depth: depth + 1) }
        case UInt8(ascii: "\""):
            _ = try parseString()
        case UInt8(ascii: "t"): try expectLiteral("true")
        case UInt8(ascii: "f"): try expectLiteral("false")
        case UInt8(ascii: "n"): try expectLiteral("null")
        default: _ = try parseNumber()
        }
    }

    /// `{ "key": value, ... }` — calls `body` positioned at each value; the
    /// body must consume exactly that value.
    private mutating func parseObject(_ body: (String, inout FastJSON) throws -> Void) throws {
        try expect(UInt8(ascii: "{"))
        skipWhitespace()
        if peek(UInt8(ascii: "}")) { i += 1; return }
        while true {
            skipWhitespace()
            let key = try parseString()
            skipWhitespace()
            try expect(UInt8(ascii: ":"))
            skipWhitespace()
            try body(key, &self)
            skipWhitespace()
            guard i < bytes.count else { throw error("unterminated object") }
            if bytes[i] == UInt8(ascii: ",") { i += 1; continue }
            if bytes[i] == UInt8(ascii: "}") { i += 1; return }
            throw error("expected ',' or '}'")
        }
    }

    private mutating func parseArray<T>(_ element: (inout FastJSON) throws -> T) throws -> [T] {
        try expect(UInt8(ascii: "["))
        var out: [T] = []
        skipWhitespace()
        if peek(UInt8(ascii: "]")) { i += 1; return out }
        while true {
            skipWhitespace()
            out.append(try element(&self))
            skipWhitespace()
            guard i < bytes.count else { throw error("unterminated array") }
            if bytes[i] == UInt8(ascii: ",") { i += 1; continue }
            if bytes[i] == UInt8(ascii: "]") { i += 1; return out }
            throw error("expected ',' or ']'")
        }
    }

    private mutating func parseOptionalString() throws -> String? {
        if peekNull() { try parseNull(); return nil }
        return try parseString()
    }

    private mutating func parseNull() throws { try expectLiteral("null") }
    private func peekNull() -> Bool { i < bytes.count && bytes[i] == UInt8(ascii: "n") }

    private mutating func parseInt() throws -> Int {
        let start = i
        let (d, isInt) = try parseNumber()
        guard isInt else { throw Error(offset: start, message: "expected an integer") }
        guard d >= -9007199254740992, d <= 9007199254740992 else {
            // Beyond 2^53 the double lost precision: re-read the literal exactly.
            let text = String(decoding: UnsafeBufferPointer(rebasing: bytes[start..<i]), as: UTF8.self)
            guard let v = Int(text) else { throw Error(offset: start, message: "integer out of range") }
            return v
        }
        return Int(d)
    }

    private mutating func parseDouble() throws -> Double { try parseNumber().0 }

    /// RFC 8259 number: -?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?
    private mutating func parseNumber() throws -> (Double, Bool) {
        let start = i
        var isInteger = true
        if peek(UInt8(ascii: "-")) { i += 1 }
        guard i < bytes.count else { throw error("expected number") }
        if bytes[i] == UInt8(ascii: "0") {
            i += 1
        } else if bytes[i] >= UInt8(ascii: "1"), bytes[i] <= UInt8(ascii: "9") {
            while i < bytes.count, isDigit(bytes[i]) { i += 1 }
        } else {
            throw error("expected number")
        }
        if peek(UInt8(ascii: ".")) {
            isInteger = false
            i += 1
            guard i < bytes.count, isDigit(bytes[i]) else { throw error("expected digits after '.'") }
            while i < bytes.count, isDigit(bytes[i]) { i += 1 }
        }
        if peek(UInt8(ascii: "e")) || peek(UInt8(ascii: "E")) {
            isInteger = false
            i += 1
            if peek(UInt8(ascii: "+")) || peek(UInt8(ascii: "-")) { i += 1 }
            guard i < bytes.count, isDigit(bytes[i]) else { throw error("expected exponent digits") }
            while i < bytes.count, isDigit(bytes[i]) { i += 1 }
        }
        let text = String(decoding: UnsafeBufferPointer(rebasing: bytes[start..<i]), as: UTF8.self)
        guard let d = Double(text) else { throw Error(offset: start, message: "invalid number \(text)") }
        return (d, isInteger)
    }

    /// A JSON string; the fast path copies unescaped runs of bytes and
    /// decodes escapes (including surrogate pairs) as it goes.
    private mutating func parseString() throws -> String {
        try expect(UInt8(ascii: "\""))
        // Fast scan: no escapes → one String from the slice.
        var j = i
        while j < bytes.count {
            let b = bytes[j]
            if b == UInt8(ascii: "\"") {
                let s = String(decoding: UnsafeBufferPointer(rebasing: bytes[i..<j]), as: UTF8.self)
                guard s.utf8.elementsEqual(bytes[i..<j]) else { throw error("invalid UTF-8 in string") }
                i = j + 1
                return s
            }
            if b == UInt8(ascii: "\\") { break }
            if b < 0x20 { throw Error(offset: j, message: "control character in string") }
            j += 1
        }
        var out: [UInt8] = []
        out.reserveCapacity(j - i + 16)
        out.append(contentsOf: bytes[i..<j])
        i = j
        while i < bytes.count {
            let b = bytes[i]
            if b == UInt8(ascii: "\"") {
                i += 1
                let s = String(decoding: out, as: UTF8.self)
                guard s.utf8.elementsEqual(out) else { throw error("invalid UTF-8 in string") }
                return s
            }
            if b == UInt8(ascii: "\\") {
                i += 1
                guard i < bytes.count else { throw error("unterminated escape") }
                let e = bytes[i]; i += 1
                switch e {
                case UInt8(ascii: "\""): out.append(0x22)
                case UInt8(ascii: "\\"): out.append(0x5C)
                case UInt8(ascii: "/"): out.append(0x2F)
                case UInt8(ascii: "b"): out.append(0x08)
                case UInt8(ascii: "f"): out.append(0x0C)
                case UInt8(ascii: "n"): out.append(0x0A)
                case UInt8(ascii: "r"): out.append(0x0D)
                case UInt8(ascii: "t"): out.append(0x09)
                case UInt8(ascii: "u"):
                    var scalar = try parseHex4()
                    if scalar >= 0xD800, scalar <= 0xDBFF {
                        // High surrogate: a low surrogate escape must follow.
                        guard i + 1 < bytes.count, bytes[i] == UInt8(ascii: "\\"), bytes[i + 1] == UInt8(ascii: "u") else {
                            throw error("unpaired surrogate")
                        }
                        i += 2
                        let low = try parseHex4()
                        guard low >= 0xDC00, low <= 0xDFFF else { throw error("unpaired surrogate") }
                        scalar = 0x10000 + ((scalar - 0xD800) << 10) + (low - 0xDC00)
                    } else if scalar >= 0xDC00, scalar <= 0xDFFF {
                        throw error("unpaired surrogate")
                    }
                    guard let u = Unicode.Scalar(scalar) else { throw error("invalid scalar") }
                    out.append(contentsOf: Array(String(Character(u)).utf8))
                default:
                    throw error("invalid escape")
                }
                continue
            }
            if b < 0x20 { throw error("control character in string") }
            out.append(b)
            i += 1
        }
        throw error("unterminated string")
    }

    private mutating func parseHex4() throws -> UInt32 {
        guard i + 4 <= bytes.count else { throw error("short \\u escape") }
        var v: UInt32 = 0
        for k in 0..<4 {
            let c = bytes[i + k]
            let d: UInt32
            switch c {
            case UInt8(ascii: "0")...UInt8(ascii: "9"): d = UInt32(c - UInt8(ascii: "0"))
            case UInt8(ascii: "a")...UInt8(ascii: "f"): d = UInt32(c - UInt8(ascii: "a") + 10)
            case UInt8(ascii: "A")...UInt8(ascii: "F"): d = UInt32(c - UInt8(ascii: "A") + 10)
            default: throw error("invalid hex digit")
            }
            v = v << 4 | d
        }
        i += 4
        return v
    }

    // MARK: low-level

    private func isDigit(_ b: UInt8) -> Bool { b >= UInt8(ascii: "0") && b <= UInt8(ascii: "9") }
    private func peek(_ b: UInt8) -> Bool { i < bytes.count && bytes[i] == b }

    private mutating func expect(_ b: UInt8) throws {
        guard peek(b) else { throw error("expected '\(Character(Unicode.Scalar(b)))'") }
        i += 1
    }

    private mutating func expectLiteral(_ s: StaticString) throws {
        let n = s.utf8CodeUnitCount
        guard i + n <= bytes.count else { throw error("unexpected end") }
        for k in 0..<n where bytes[i + k] != s.utf8Start[k] { throw error("invalid literal") }
        i += n
    }

    private mutating func skipWhitespace() {
        while i < bytes.count {
            switch bytes[i] {
            case 0x20, 0x09, 0x0A, 0x0D: i += 1
            default: return
            }
        }
    }

    private func error(_ message: String) -> Error { Error(offset: i, message: message) }
}
