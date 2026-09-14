import Foundation

/// Models for FlashTeX runtime protocol v1 (docs/contracts/runtime-v1.md).
/// Envelope: `protocol_version`, `id`, `type`, `payload`. Replies preserve `id`.
public enum RuntimeV1 {
    public static let protocolVersion = 1

    public struct Envelope<Payload: Codable>: Codable {
        public var protocolVersion: Int
        public var id: String
        public var type: String
        public var payload: Payload

        enum CodingKeys: String, CodingKey {
            case protocolVersion = "protocol_version", id, type, payload
        }
        public init(protocolVersion: Int, id: String, type: String, payload: Payload) {
            self.protocolVersion = protocolVersion; self.id = id; self.type = type; self.payload = payload
        }
    }

    // MARK: compile

    public struct Document: Codable, Equatable {
        public var path: String
        public var text: String
        public init(path: String, text: String) { self.path = path; self.text = text }
    }

    public struct CompileRequest: Codable, Equatable {
        public var projectId: String
        public var revision: Int
        public var entryPath: String
        public var documents: [Document]
        /// Requested layout capabilities (`layout_capabilities`, see
        /// runtime-v1-layout-capabilities.md). Omitted from the wire when nil.
        public var layoutCapabilities: [String]?
        /// `display-list-v2-delta` installed-base acknowledgement
        /// (`display_list_base`; proposal r5 §3). Isolated feature: sent only
        /// when the delta capability is requested; omitted from the wire when nil.
        public var displayListBase: DisplayListBase?
        /// `display-list-v2-window`: where the viewer is
        /// (`display_list_window`, proposal §4). Isolated feature: sent only
        /// when the window capability is requested; omitted from the wire when
        /// nil, which asks the producer for an unwindowed reply even with the
        /// capability listed (so a consumer may advertise support before it
        /// knows where the viewer is).
        public var displayListWindow: DisplayListWindow?
        /// Absolute directory `\includegraphics` files are read from by the
        /// producer (`project_root`, display-list-v2-images proposal §2).
        /// Optional; omitted from the wire when nil. Old producers ignore it.
        public var projectRoot: String?
        /// The civil date `\today` renders, `YYYY-MM-DD`
        /// (`date`, runtime-v1-request-date proposal).
        ///
        /// The compiler must never read the wall clock -- runtime-v1 requires
        /// byte-identical output for byte-identical input -- so the app reads
        /// it and sends the answer. A civil date rather than a timestamp
        /// because `\today` is a local calendar date; see `RuntimeV1.localDate`.
        ///
        /// Optional; omitted from the wire when nil, and an omitted date
        /// compiles as the Unix epoch exactly as before. Old workers ignore it.
        public var date: String?

        public struct DisplayListBase: Codable, Equatable {
            public var requestId: String
            public var projectId: String
            public var revision: Int
            public var pageCount: Int
            public var listDigest: String
            enum CodingKeys: String, CodingKey {
                case requestId = "request_id", projectId = "project_id", revision, pageCount = "page_count", listDigest = "list_digest"
            }
            public init(requestId: String, projectId: String, revision: Int, pageCount: Int, listDigest: String) {
                self.requestId = requestId; self.projectId = projectId; self.revision = revision; self.pageCount = pageCount; self.listDigest = listDigest
            }
        }

        /// The resident page window the viewer is asking for. `firstPage` is
        /// 1-based; a window running past the last page is clamped by the
        /// producer, not refused (proposal §4).
        public struct DisplayListWindow: Codable, Equatable {
            public var firstPage: Int
            public var pageCount: Int
            enum CodingKeys: String, CodingKey { case firstPage = "first_page", pageCount = "page_count" }
            public init(firstPage: Int, pageCount: Int) { self.firstPage = firstPage; self.pageCount = pageCount }
        }

        enum CodingKeys: String, CodingKey {
            case projectId = "project_id", revision, entryPath = "entry_path", documents
            case layoutCapabilities = "layout_capabilities"
            case displayListBase = "display_list_base"
            case displayListWindow = "display_list_window"
            case projectRoot = "project_root"
            case date
        }
        public init(projectId: String, revision: Int, entryPath: String, documents: [Document],
                    layoutCapabilities: [String]? = nil, displayListBase: DisplayListBase? = nil,
                    displayListWindow: DisplayListWindow? = nil, projectRoot: String? = nil,
                    date: String? = nil) {
            self.projectId = projectId; self.revision = revision
            self.entryPath = entryPath; self.documents = documents
            self.layoutCapabilities = layoutCapabilities
            self.displayListBase = displayListBase
            self.displayListWindow = displayListWindow
            self.projectRoot = projectRoot
            self.date = date
        }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            projectId = try c.decode(String.self, forKey: .projectId)
            revision = try c.decode(Int.self, forKey: .revision)
            entryPath = try c.decode(String.self, forKey: .entryPath)
            documents = try c.decode([Document].self, forKey: .documents)
            layoutCapabilities = try c.decodeIfPresent([String].self, forKey: .layoutCapabilities)
            if let caps = layoutCapabilities { try LayoutCapabilities.validate(caps) }
            displayListBase = try c.decodeIfPresent(DisplayListBase.self, forKey: .displayListBase)
            displayListWindow = try c.decodeIfPresent(DisplayListWindow.self, forKey: .displayListWindow)
            if let w = displayListWindow, w.firstPage < 1 || w.pageCount < 1 {
                throw DecodeError.invalidLayoutCapabilities("display_list_window must ask for at least one page from page 1 onwards (got first_page \(w.firstPage), page_count \(w.pageCount))")
            }
            projectRoot = try c.decodeIfPresent(String.self, forKey: .projectRoot)
            date = try c.decodeIfPresent(String.self, forKey: .date)
            if let date { try RuntimeV1.validateDate(date) }
        }

        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(projectId, forKey: .projectId)
            try c.encode(revision, forKey: .revision)
            try c.encode(entryPath, forKey: .entryPath)
            try c.encode(documents, forKey: .documents)
            if let caps = layoutCapabilities {
                try LayoutCapabilities.validate(caps)
                try c.encode(caps, forKey: .layoutCapabilities)
            }
            if let base = displayListBase { try c.encode(base, forKey: .displayListBase) }
            if let w = displayListWindow {
                guard w.firstPage >= 1, w.pageCount >= 1 else {
                    throw DecodeError.invalidLayoutCapabilities("display_list_window must ask for at least one page from page 1 onwards (got first_page \(w.firstPage), page_count \(w.pageCount))")
                }
                try c.encode(w, forKey: .displayListWindow)
            }
            if let root = projectRoot { try c.encode(root, forKey: .projectRoot) }
            if let date {
                try RuntimeV1.validateDate(date)
                try c.encode(date, forKey: .date)
            }
        }
    }

    /// Rejects anything that is not exactly a `YYYY-MM-DD` civil date.
    ///
    /// Strict on purpose. The worker refuses a malformed date rather than
    /// guessing, so catching it here turns a failed compile into a programming
    /// error at the call site instead.
    public static func validateDate(_ value: String) throws {
        let bytes = Array(value.utf8)
        guard bytes.count == 10, bytes[4] == UInt8(ascii: "-"), bytes[7] == UInt8(ascii: "-") else {
            throw DecodeError.invalidDate(value)
        }
        func number(_ range: Range<Int>) throws -> Int {
            var n = 0
            for i in range {
                guard bytes[i] >= UInt8(ascii: "0"), bytes[i] <= UInt8(ascii: "9") else {
                    throw DecodeError.invalidDate(value)
                }
                n = n * 10 + Int(bytes[i] - UInt8(ascii: "0"))
            }
            return n
        }
        let year = try number(0..<4), month = try number(5..<7), day = try number(8..<10)
        guard (1...9999).contains(year), (1...12).contains(month) else {
            throw DecodeError.invalidDate(value)
        }
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
        let lengths = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        guard (1...lengths[month - 1]).contains(day) else {
            throw DecodeError.invalidDate(value)
        }
    }

    /// Today's date in the user's own calendar and timezone, in the wire form.
    ///
    /// This is the clock read the engine is forbidden to make. `Calendar.current`
    /// is deliberate: `\today` is a *local* calendar date, so a UTC instant
    /// would print the neighbouring day for much of the world near midnight.
    /// The Gregorian components are requested explicitly so a non-Gregorian
    /// user calendar still yields the Gregorian date LaTeX renders.
    public static func localDate(_ now: Date = Date(), timeZone: TimeZone = .current) -> String {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = timeZone
        let c = calendar.dateComponents([.year, .month, .day], from: now)
        return String(format: "%04d-%02d-%02d", c.year ?? 1970, c.month ?? 1, c.day ?? 1)
    }

    /// Negotiated layout capabilities (runtime-v1-layout-capabilities.md).
    /// A list is at most 16 unique, nonempty strings of at most 64 UTF-8 bytes.
    public enum LayoutCapabilities {
        public static let rulesV1 = "rules-v1"
        public static let fontHintsV1 = "font-hints-v1"
        /// Capabilities this client knows how to consume.
        public static let supported: [String] = [rulesV1, fontHintsV1]
        public static let maxCount = 16
        public static let maxBytes = 64

        public static func validate(_ caps: [String]) throws {
            guard caps.count <= maxCount else {
                throw DecodeError.invalidLayoutCapabilities("\(caps.count) capabilities exceed the limit of \(maxCount)")
            }
            var seen = Set<String>()
            for cap in caps {
                guard !cap.isEmpty else { throw DecodeError.invalidLayoutCapabilities("empty capability string") }
                guard cap.utf8.count <= maxBytes else {
                    throw DecodeError.invalidLayoutCapabilities("capability of \(cap.utf8.count) bytes exceeds \(maxBytes)")
                }
                guard seen.insert(cap).inserted else { throw DecodeError.invalidLayoutCapabilities("duplicate capability \(cap)") }
            }
        }
    }

    // MARK: compile_result

    public enum Status: String, Codable { case ok, recovered, failed }

    /// Zero-based, end-exclusive UTF-8 byte offsets into the input revision.
    public struct SourceRange: Codable, Equatable {
        public var path: String
        public var startByte: Int
        public var endByte: Int

        enum CodingKeys: String, CodingKey {
            case path, startByte = "start_byte", endByte = "end_byte"
        }
        public init(path: String, startByte: Int, endByte: Int) {
            self.path = path; self.startByte = startByte; self.endByte = endByte
        }
    }

    /// `kind: text` is base v1; `kind: rule` is decoded only as a typed rule
    /// (negotiated `rules-v1`) and fails decoding when malformed. Unknown kinds
    /// decode as `.unknown` (with their `source` when present) so a newer
    /// contract revision does not crash the shell; whether they are reported
    /// or skipped is the consumer's decision (see ShellModel).
    public enum PageItem: Codable, Equatable {
        case text(TextItem)
        case rule(RuleItem)
        case unknown(kind: String, source: SourceRange? = nil)

        public struct TextItem: Codable, Equatable {
            public var text: String
            public var xPt: Double
            public var baselineYPt: Double
            public var fontSizePt: Double
            public var source: SourceRange?
            /// Explicit face intent (`font-hints-v1`); nil means legacy selection.
            public var font: FontHint?

            enum CodingKeys: String, CodingKey {
                case text, xPt = "x_pt", baselineYPt = "baseline_y_pt"
                case fontSizePt = "font_size_pt", source, font
            }
            public init(text: String, xPt: Double, baselineYPt: Double, fontSizePt: Double,
                        source: SourceRange?, font: FontHint? = nil) {
                self.text = text; self.xPt = xPt; self.baselineYPt = baselineYPt
                self.fontSizePt = fontSizePt; self.source = source; self.font = font
            }
        }

        /// `font-hints-v1`: family/weight/style intent. Not font bytes, GIDs,
        /// or advances — a consumer that cannot resolve `family` must report
        /// substitution rather than claim the requested face was preserved.
        public struct FontHint: Codable, Equatable {
            public enum Weight: String, Codable { case normal, bold }
            public enum Style: String, Codable { case normal, italic }
            public var family: String
            public var weight: Weight
            public var style: Style
            public static let maxFamilyBytes = 128

            public init(family: String, weight: Weight = .normal, style: Style = .normal) {
                self.family = family; self.weight = weight; self.style = style
            }

            enum CodingKeys: String, CodingKey { case family, weight, style }

            public init(from decoder: Decoder) throws {
                let c = try decoder.container(keyedBy: CodingKeys.self)
                family = try c.decode(String.self, forKey: .family)
                weight = try c.decode(Weight.self, forKey: .weight)
                style = try c.decode(Style.self, forKey: .style)
                guard !family.isEmpty, family.utf8.count <= Self.maxFamilyBytes else {
                    throw DecodingError.dataCorruptedError(forKey: .family, in: c,
                        debugDescription: "font family must be 1...\(Self.maxFamilyBytes) UTF-8 bytes")
                }
                guard !family.unicodeScalars.contains(where: { $0.properties.generalCategory == .control }) else {
                    throw DecodingError.dataCorruptedError(forKey: .family, in: c,
                        debugDescription: "font family must not contain control characters")
                }
            }
        }

        /// `rules-v1`: a filled rectangle whose TOP-LEFT corner is `(x_pt, y_pt)`
        /// in page coordinates (y downward from the page top). `y_pt` is not a
        /// baseline. Dimensions are positive and finite; magnitudes ≤ 1e6.
        public struct RuleItem: Codable, Equatable {
            public var xPt: Double
            public var yPt: Double
            public var widthPt: Double
            public var heightPt: Double
            public var source: SourceRange?
            public static let maxMagnitude = 1_000_000.0

            enum CodingKeys: String, CodingKey {
                case xPt = "x_pt", yPt = "y_pt", widthPt = "width_pt", heightPt = "height_pt", source
            }
            public init(xPt: Double, yPt: Double, widthPt: Double, heightPt: Double, source: SourceRange?) {
                self.xPt = xPt; self.yPt = yPt; self.widthPt = widthPt; self.heightPt = heightPt; self.source = source
            }

            public init(from decoder: Decoder) throws {
                let c = try decoder.container(keyedBy: CodingKeys.self)
                xPt = try c.decode(Double.self, forKey: .xPt)
                yPt = try c.decode(Double.self, forKey: .yPt)
                widthPt = try c.decode(Double.self, forKey: .widthPt)
                heightPt = try c.decode(Double.self, forKey: .heightPt)
                source = try c.decodeIfPresent(SourceRange.self, forKey: .source)
                for (value, key) in [(xPt, CodingKeys.xPt), (yPt, .yPt), (widthPt, .widthPt), (heightPt, .heightPt)] {
                    guard value.isFinite, abs(value) <= Self.maxMagnitude else {
                        throw DecodingError.dataCorruptedError(forKey: key, in: c,
                            debugDescription: "rule \(key.rawValue) must be finite with magnitude ≤ 1e6")
                    }
                }
                guard widthPt > 0, heightPt > 0 else {
                    throw DecodingError.dataCorruptedError(forKey: .widthPt, in: c,
                        debugDescription: "rule width_pt and height_pt must be positive")
                }
            }
        }

        enum CodingKeys: String, CodingKey { case kind, source }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            let kind = try c.decode(String.self, forKey: .kind)
            switch kind {
            case "text": self = .text(try TextItem(from: decoder))
            case "rule": self = .rule(try RuleItem(from: decoder))
            default: self = .unknown(kind: kind, source: try? c.decodeIfPresent(SourceRange.self, forKey: .source))
            }
        }

        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            switch self {
            case .text(let item):
                try c.encode("text", forKey: .kind)
                try item.encode(to: encoder)
            case .rule(let item):
                try c.encode("rule", forKey: .kind)
                try item.encode(to: encoder)
            case .unknown(let kind, let source):
                try c.encode(kind, forKey: .kind)
                try c.encodeIfPresent(source, forKey: .source)
            }
        }
    }

    public struct Page: Codable, Equatable {
        public var number: Int
        public var widthPt: Double
        public var heightPt: Double
        public var items: [PageItem]

        enum CodingKeys: String, CodingKey {
            case number, widthPt = "width_pt", heightPt = "height_pt", items
        }
        public init(number: Int, widthPt: Double, heightPt: Double, items: [PageItem]) {
            self.number = number; self.widthPt = widthPt; self.heightPt = heightPt; self.items = items
        }
    }

    public enum Severity: String, Codable { case error, warning }

    public struct Diagnostic: Codable, Equatable {
        public var severity: Severity
        public var message: String
        public var source: SourceRange?
        public var recovery: String?
        public init(severity: Severity, message: String, source: SourceRange?, recovery: String?) {
            self.severity = severity; self.message = message; self.source = source; self.recovery = recovery
        }
    }

    public struct CompileResult: Codable, Equatable {
        public var projectId: String
        public var revision: Int
        public var status: Status
        public var pages: [Page]
        public var diagnostics: [Diagnostic]
        public var pdfPath: String?
        /// Capabilities the producer accepted for this result (a subset of the
        /// request's `layout_capabilities`). Nil/omitted means none.
        public var layoutCapabilities: [String]?

        enum CodingKeys: String, CodingKey {
            case projectId = "project_id", revision, status, pages, diagnostics
            case pdfPath = "pdf_path", layoutCapabilities = "layout_capabilities"
        }
        public init(projectId: String, revision: Int, status: Status, pages: [Page],
                    diagnostics: [Diagnostic], pdfPath: String?, layoutCapabilities: [String]? = nil) {
            self.projectId = projectId; self.revision = revision; self.status = status
            self.pages = pages; self.diagnostics = diagnostics; self.pdfPath = pdfPath
            self.layoutCapabilities = layoutCapabilities
        }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            projectId = try c.decode(String.self, forKey: .projectId)
            revision = try c.decode(Int.self, forKey: .revision)
            status = try c.decode(Status.self, forKey: .status)
            pages = try c.decode([Page].self, forKey: .pages)
            diagnostics = try c.decode([Diagnostic].self, forKey: .diagnostics)
            pdfPath = try c.decodeIfPresent(String.self, forKey: .pdfPath)
            layoutCapabilities = try c.decodeIfPresent([String].self, forKey: .layoutCapabilities)
            if let caps = layoutCapabilities { try LayoutCapabilities.validate(caps) }
        }

        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(projectId, forKey: .projectId)
            try c.encode(revision, forKey: .revision)
            try c.encode(status, forKey: .status)
            try c.encode(pages, forKey: .pages)
            try c.encode(diagnostics, forKey: .diagnostics)
            try c.encode(pdfPath, forKey: .pdfPath) // null until a real artifact exists
            if let caps = layoutCapabilities {
                try LayoutCapabilities.validate(caps)
                try c.encode(caps, forKey: .layoutCapabilities)
            }
        }
    }

    // MARK: decoding

    public enum DecodeError: Error, Equatable {
        case unsupportedVersion(Int)
        case unexpectedType(expected: String, actual: String)
        case invalidLayoutCapabilities(String)
        /// `payload.date` was not a `YYYY-MM-DD` civil date. The worker refuses
        /// a malformed date rather than guessing at another one, so this is
        /// caught here too rather than being sent and failing the compile.
        case invalidDate(String)
    }

    /// Fast path first (FastJSON, same values for every valid frame); any
    /// input it does not accept goes through `JSONDecoder`, whose error is the
    /// one reported.
    public static func decodeCompileResult(_ data: Data) throws -> Envelope<CompileResult> {
        if let env = try? FastJSON.compileResultEnvelope(data) {
            guard env.protocolVersion == protocolVersion else { throw DecodeError.unsupportedVersion(env.protocolVersion) }
            guard env.type == "compile_result" else { throw DecodeError.unexpectedType(expected: "compile_result", actual: env.type) }
            return env
        }
        return try decode(data, expectedType: "compile_result")
    }

    /// `JSONDecoder` only — for equivalence tests of the fast path.
    public static func decodeCompileResultReference(_ data: Data) throws -> Envelope<CompileResult> {
        try decode(data, expectedType: "compile_result")
    }

    public static func decodeCompileRequest(_ data: Data) throws -> Envelope<CompileRequest> {
        try decode(data, expectedType: "compile")
    }

    static func decode<P: Codable>(_ data: Data, expectedType: String) throws -> Envelope<P> {
        let env = try JSONDecoder().decode(Envelope<P>.self, from: data)
        guard env.protocolVersion == protocolVersion else {
            throw DecodeError.unsupportedVersion(env.protocolVersion)
        }
        guard env.type == expectedType else {
            throw DecodeError.unexpectedType(expected: expectedType, actual: env.type)
        }
        return env
    }
}
