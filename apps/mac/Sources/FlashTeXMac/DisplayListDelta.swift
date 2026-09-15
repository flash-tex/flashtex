import CryptoKit
import Foundation
import FlashTeXProtocol

/// Consumer of `display-list-v2-delta` (docs/proposals/display-list-v2-delta.md
/// r5) on the direct worker route: requested next to `display-list-v2` whenever
/// the v2 pane holds an installed base (`ShellModel.compile`), applied in
/// `receiveDisplayListV2`. The producer decides per request; a full
/// `display_list` line is handled exactly as before. The helper route is
/// untouched (FT-049 sibling-type change pending). `FLASHTEX_DISPLAY_DELTA=0`
/// turns the request off (the producer then always answers full).
///
/// Responsibilities: `dl2-canon-1` digests over the semantic model; exact
/// `page_bytes` verification (changed pages: the wire length recorded by the
/// fast reader; unchanged pages: the installed page's cached length plus the
/// decimal-width change of its relocated offsets); the exact full-line size
/// from raw wire slices and a fixed writer template — charged BEFORE any page
/// is built; relocation and reconstruction; typed refusals; installed-base
/// acknowledgement; and the residency accounting (installed + in-flight +
/// queued, in-flight preraster included).
enum DisplayListDelta {
    static let capability = "display-list-v2-delta"
    static let messageType = "display_list_delta"
    static let digestScheme = "dl2-canon-1"
    static let maxSnapshotPages = 1024
    static let maxSnapshotBytes = 16 * 1024 * 1024

    /// On unless `FLASHTEX_DISPLAY_DELTA=0`.
    static var enabled: Bool { ProcessInfo.processInfo.environment["FLASHTEX_DISPLAY_DELTA"] != "0" }
    /// `protocol/proposals/display-list-v2-only.md`: with the v2 pane active the
    /// runtime-v1 `pages` are dead weight (2.5 MB per keystroke on a 60 KB
    /// body); the producer elides them when it emits a sibling. On unless
    /// `FLASHTEX_DISPLAY_V2_ONLY=0`.
    static let v2OnlyCapability = "display-list-v2-only"
    static var v2OnlyEnabled: Bool { ProcessInfo.processInfo.environment["FLASHTEX_DISPLAY_V2_ONLY"] != "0" }
    /// The per-request capabilities (never part of the mode set
    /// `requestedLayoutCapabilities`; echoed only on replies that honour them).
    static let perRequestCapabilities: Set<String> = [capability, v2OnlyCapability]
    static func stripPerRequest(_ caps: [String]) -> [String] { caps.filter { !perRequestCapabilities.contains($0) } }

    // MARK: dl2-canon-1

    private struct Canon {
        var bytes = Data()
        mutating func i(_ n: Int64) { withUnsafeBytes(of: n.littleEndian) { bytes.append(contentsOf: $0) } }
        mutating func u(_ n: Int) { i(Int64(n)) }
        mutating func s(_ str: String) { let d = Data(str.utf8); u(d.count); bytes.append(d) }
        /// Raw IEEE-754 bits after normalising -0.0 to +0.0; non-finite values never reach here
        /// (`RenderingV2.validate` refuses paint outside 0...1).
        mutating func f(_ x: Double) { let v = x == 0 ? 0.0 : x; withUnsafeBytes(of: v.bitPattern.littleEndian) { bytes.append(contentsOf: $0) } }
        mutating func ranges(_ rs: [RenderingV2.SourceRange]) { u(rs.count); for r in rs { s(r.path); u(r.startByte); u(r.endByte) } }
        mutating func provenance(sources: [RenderingV2.SourceRange]?, synthetic: String?) {
            if let sources { bytes.append(0x10); ranges(sources) } else { bytes.append(0x11); s(synthetic ?? "") }
        }
        mutating func paint(_ p: RenderingV2.Paint) { f(p.r); f(p.g); f(p.b); f(p.a) }
        func sha() -> Data { Data(SHA256.hash(data: bytes)) }
    }

    static func pageDigest(_ p: RenderingV2.Page) -> Data {
        var c = Canon(bytes: Data("flashtex:dl2:page:1\0".utf8))
        c.u(p.number); c.i(p.width); c.i(p.height); c.u(p.items.count)
        for it in p.items {
            switch it {
            case .glyphRun(let r):
                c.bytes.append(0x01); c.s(r.fontId); c.i(r.fontSize); c.s(r.text)
                c.u(r.glyphs.count)
                for g in r.glyphs { c.u(g.gid); c.i(g.originX); c.i(g.baselineY); c.i(g.advanceX); c.i(g.advanceY); c.u(g.cluster) }
                c.u(r.clusters.count)
                for cl in r.clusters {
                    c.u(cl.textStartByte); c.u(cl.textEndByte)
                    c.u(cl.hitRects.count); for h in cl.hitRects { c.i(h.x); c.i(h.top); c.i(h.width); c.i(h.height) }
                    c.u(cl.carets.count); for k in cl.carets { c.u(k.textByte); c.i(k.x); c.i(k.top); c.i(k.height) }
                    c.provenance(sources: cl.sources, synthetic: cl.syntheticReason)
                }
                c.paint(r.paint)
            case .rule(let r):
                c.bytes.append(0x02); c.i(r.x); c.i(r.top); c.i(r.width); c.i(r.height); c.paint(r.paint)
                c.provenance(sources: r.sources, synthetic: r.syntheticReason)
            case .image(let i):
                // Not part of the delta proposal's canon (r5 predates images):
                // a page with images digests differently from a producer that
                // omits them, so the acknowledgement simply fails to match and
                // a full list follows. Never a wrong reuse.
                c.bytes.append(0x03); c.i(i.x); c.i(i.top); c.i(i.width); c.i(i.height)
                c.u(i.transform.count); for v in i.transform { c.s(String(format: "%.3f", v)) }
                c.s(i.image.imageId); c.s(i.image.path); c.u(i.image.pdfPage ?? 0)
                c.provenance(sources: i.sources, synthetic: i.syntheticReason)
            case .path(let p):
                // path-v0 (TikZ), likewise outside the r5 canon: every command,
                // the operation (fill rule or full stroke), clips, paint and
                // provenance, so any change to the picture changes the digest.
                c.bytes.append(0x04)
                func commands(_ cmds: [RenderingV2.PathCommand]) {
                    c.u(cmds.count)
                    for cmd in cmds {
                        switch cmd {
                        case .move: c.bytes.append(0x6D)
                        case .line: c.bytes.append(0x6C)
                        case .cubic: c.bytes.append(0x63)
                        case .close: c.bytes.append(0x7A)
                        }
                        for v in cmd.coordinates { c.i(v) }
                    }
                }
                switch p.op {
                case .fill(let rule): c.bytes.append(0x01); c.s(rule.rawValue)
                case .stroke(let s):
                    c.bytes.append(0x02); c.i(s.width); c.s(s.cap.rawValue); c.s(s.join.rawValue); c.f(s.miterLimit)
                    if let d = s.dash { c.u(d.array.count); for v in d.array { c.i(v) }; c.i(d.phase) } else { c.u(0) }
                }
                commands(p.path)
                c.u(p.clips.count); for clip in p.clips { c.s(clip.fillRule.rawValue); commands(clip.path) }
                c.paint(p.paint)
                c.provenance(sources: p.sources, synthetic: p.syntheticReason)
            }
        }
        return c.sha()
    }

    static func headerDigest(_ l: RenderingV2.DisplayList) -> Data { Data(SHA256.hash(data: headerCanon(l))) }

    /// The canonical header bytes (exposed for cross-implementation debugging).
    static func headerCanon(_ l: RenderingV2.DisplayList) -> Data {
        var c = Canon(bytes: Data("flashtex:dl2:header:1\0".utf8))
        for s in [l.renderFormat, l.coordinateUnit, l.colorSpace, l.textExtraction, l.projectId] { c.s(s) }
        c.u(l.revision)
        c.u(l.requiredFeatures.count); for f in l.requiredFeatures { c.s(f) }
        c.u(l.documents.count); for d in l.documents { c.s(d.path); c.u(d.revision); c.s(d.sha256); c.i(d.byteLength) }
        c.u(l.fonts.count)
        for f in l.fonts { c.s(f.fontId); c.s(f.sha256); c.i(f.byteLength); c.s(f.format); c.u(f.faceIndex); c.u(f.unitsPerEm); c.u(f.glyphCount); c.s(f.postscriptName) }
        c.u(l.diagnostics.count); for d in l.diagnostics { c.s(d.code); c.s(d.message); c.s(d.severity.rawValue); c.ranges(d.sources) }
        return c.bytes
    }

    static func listDigest(_ l: RenderingV2.DisplayList, pageDigests: [Data]) -> Data {
        var c = Canon(bytes: Data("flashtex:dl2:list:1\0".utf8))
        c.bytes.append(headerDigest(l)); c.u(pageDigests.count); for d in pageDigests { c.bytes.append(d) }
        return c.sha()
    }

    static func hex(_ d: Data) -> String { d.map { String(format: "%02x", $0) }.joined() }

    // MARK: relocation

    typealias Relocation = RenderingV2Fast.DeltaEnvelope.Relocation

    /// Decimal width of a nonnegative integer as the producer prints it (plain decimal).
    static func digits(_ n: Int) -> Int { n == 0 ? 1 : String(n).utf8.count }

    static func relocate(_ r: RenderingV2.SourceRange, _ rl: Relocation) -> RenderingV2.SourceRange? {
        if r.endByte <= rl.editStart { return r }
        if r.startByte >= rl.editEnd {
            let (s, o1) = r.startByte.addingReportingOverflow(rl.delta), (e, o2) = r.endByte.addingReportingOverflow(rl.delta)
            guard !o1, !o2, s >= 0, e >= s else { return nil }
            return RenderingV2.SourceRange(path: r.path, startByte: s, endByte: e)
        }
        return nil
    }

    private static func relocate(_ sources: [RenderingV2.SourceRange]?, _ by: [String: Relocation]) -> [RenderingV2.SourceRange]?? {
        guard let sources else { return .some(nil) }
        var out: [RenderingV2.SourceRange] = []
        for r in sources {
            if let rl = by[r.path] { guard let m = relocate(r, rl) else { return nil }; out.append(m) } else { out.append(r) }
        }
        return .some(out)
    }

    /// A NEW page equal to `p` with every source range moved; nil when a range intersects an edited region.
    static func relocatePage(_ p: RenderingV2.Page, _ relocs: [Relocation]) -> RenderingV2.Page? {
        let by = Dictionary(relocs.map { ($0.path, $0) }, uniquingKeysWith: { a, _ in a })
        var items: [RenderingV2.Item] = []
        items.reserveCapacity(p.items.count)
        for it in p.items {
            switch it {
            case .glyphRun(var r):
                for ci in r.clusters.indices {
                    guard let moved = relocate(r.clusters[ci].sources, by) else { return nil }
                    r.clusters[ci].sources = moved
                }
                items.append(.glyphRun(r))
            case .rule(var r):
                guard let moved = relocate(r.sources, by) else { return nil }
                r.sources = moved
                items.append(.rule(r))
            case .image(var i):
                guard let moved = relocate(i.sources, by) else { return nil }
                i.sources = moved
                items.append(.image(i))
            case .path(var p):
                guard let moved = relocate(p.sources, by) else { return nil }
                p.sources = moved
                items.append(.path(p))
            }
        }
        return RenderingV2.Page(number: p.number, width: p.width, height: p.height, items: items)
    }

    /// Exact byte length of `relocatePage(p)` from the cached length of `p`: only the
    /// decimal width of moved offsets changes. Checked arithmetic; nil on invalid/overflow.
    static func relocatedPageBytes(_ p: RenderingV2.Page, cached: Int, _ relocs: [Relocation]) -> Int? {
        let by = Dictionary(relocs.map { ($0.path, $0) }, uniquingKeysWith: { a, _ in a })
        var delta = 0
        func visit(_ sources: [RenderingV2.SourceRange]?) -> Bool {
            for r in sources ?? [] {
                guard let rl = by[r.path] else { continue }
                guard let m = relocate(r, rl) else { return false }
                delta += (digits(m.startByte) - digits(r.startByte)) + (digits(m.endByte) - digits(r.endByte))
            }
            return true
        }
        for it in p.items {
            switch it {
            case .glyphRun(let r): for c in r.clusters where !visit(c.sources) { return nil }
            case .rule(let r): if !visit(r.sources) { return nil }
            case .image(let i): if !visit(i.sources) { return nil }
            case .path(let p): if !visit(p.sources) { return nil }
            }
        }
        let (total, o) = cached.addingReportingOverflow(delta)
        return o || total < 0 ? nil : total
    }

    // MARK: exact full-line size (no writer: fixed template + raw wire slices)

    /// The producer's writer emits object keys sorted (BTreeMap); the full line is
    /// therefore exactly this template with the variable parts spliced in. The
    /// template is verified against the real producer in `DisplayListDeltaTests`.
    static let frameConstantBytes: Int = {
        let template = "{\"id\":" + ",\"payload\":{\"color_space\":\"srgb\",\"coordinate_unit\":\"bp_2pow20\",\"diagnostics\":" + ",\"documents\":" + ",\"fonts\":" + ",\"pages\":[" + "],\"project_id\":" + ",\"render_format\":\"display-list-v2\",\"required_features\":" + ",\"revision\":" + ",\"text_extraction\":\"cluster-actualtext\"},\"protocol_version\":2,\"type\":\"display_list\"}"
        return template.utf8.count
    }()

    /// `F + H + Σ page_bytes + max(N − 1, 0)`, checked; nil on overflow.
    static func fullLineBytes(raw: RenderingV2Fast.DeltaEnvelope.RawParts, pageBytes: [Int]) -> Int? {
        var total = frameConstantBytes
        for part in [raw.id, raw.projectId, raw.revision, raw.requiredFeatures, raw.documents, raw.fonts, raw.diagnostics, max(pageBytes.count - 1, 0)] {
            let (t, o) = total.addingReportingOverflow(part); guard !o else { return nil }; total = t
        }
        for b in pageBytes { let (t, o) = total.addingReportingOverflow(b); guard !o else { return nil }; total = t }
        return total
    }

    // MARK: installed base

    struct Installed: Equatable {
        var requestId: String
        var list: RenderingV2.DisplayList
        var pageDigests: [Data]
        var listDigest: Data
        /// Exact wire length of every page object (measured at install or verified from a delta).
        var pageBytes: [Int]
        /// Exact serialised size of the full line this snapshot corresponds to.
        var serialisedBytes: Int

        var acknowledgement: RuntimeV1.CompileRequest.DisplayListBase {
            .init(requestId: requestId, projectId: list.projectId, revision: list.revision, pageCount: list.pages.count, listDigest: DisplayListDelta.hex(listDigest))
        }
    }

    /// The snapshot a validated FULL frame installs; nil when it exceeds the retention caps.
    static func installed(from envelope: RenderingV2.Envelope, pageBytes: [Int], lineBytes: Int) -> Installed? {
        let list = envelope.payload
        guard list.pages.count <= maxSnapshotPages, pageBytes.count == list.pages.count, lineBytes <= maxSnapshotBytes else { return nil }
        let digests = list.pages.map(pageDigest)
        return Installed(requestId: envelope.id, list: list, pageDigests: digests, listDigest: listDigest(list, pageDigests: digests),
                         pageBytes: pageBytes, serialisedBytes: lineBytes)
    }

    // MARK: apply

    enum Refusal: Error, Equatable, CustomStringConvertible {
        case unsolicited(String)
        case unsupportedScheme(String)
        case baseMismatch(String)
        case pageCount(String)
        case removedPages(String)
        case pageBytesMismatch(page: Int, wire: Int, computed: Int)
        case targetOversize(bytes: Int, cap: Int)
        case relocationInvalid(page: Int)
        case digestMismatch(page: Int)
        case listDigestMismatch
        var code: String {
            switch self {
            case .unsolicited: return "delta_unsolicited"
            case .unsupportedScheme: return "delta_unsupported_scheme"
            case .baseMismatch: return "delta_base_mismatch"
            case .pageCount: return "delta_page_count"
            case .removedPages: return "delta_removed_pages"
            case .pageBytesMismatch: return "delta_page_bytes_mismatch"
            case .targetOversize: return "delta_target_oversize"
            case .relocationInvalid: return "delta_relocation_invalid"
            case .digestMismatch: return "delta_digest_mismatch"
            case .listDigestMismatch: return "delta_list_digest_mismatch"
            }
        }
        var description: String {
            switch self {
            case .unsolicited(let m), .unsupportedScheme(let m), .baseMismatch(let m), .pageCount(let m), .removedPages(let m): return "\(code): \(m)"
            case .pageBytesMismatch(let p, let w, let c): return "\(code): page \(p) wire page_bytes \(w) != computed \(c)"
            case .targetOversize(let b, let c): return "\(code): reconstructed size \(b) bytes over the cap \(c)"
            case .relocationInvalid(let p): return "\(code): page \(p)"
            case .digestMismatch(let p): return "\(code): page \(p)"
            case .listDigestMismatch: return code
            }
        }
        var asValidationError: RenderingV2.ValidationError { .init(code: code, message: description) }
    }

    /// Charges the target exactly, then reconstructs. Returns the full
    /// envelope (type `display_list`) and the verified per-page lengths; the
    /// caller runs the unchanged full validation (`V2Frame.prepare`) on it.
    static func apply(_ d: RenderingV2Fast.DeltaEnvelope, to installed: Installed, cap: Int = maxSnapshotBytes) throws -> (envelope: RenderingV2.Envelope, pageBytes: [Int], targetBytes: Int) {
        guard d.digestScheme == digestScheme else { throw Refusal.unsupportedScheme(d.digestScheme) }
        let ack = installed.acknowledgement
        guard d.base.requestId == ack.requestId, d.base.projectId == ack.projectId, d.base.revision == ack.revision,
              d.base.pageCount == ack.pageCount, d.base.listDigest == ack.listDigest else {
            throw Refusal.baseMismatch("delta names base \(d.base.requestId)/\(d.base.revision)/\(d.base.pageCount)p/\(d.base.listDigest.prefix(12))…, installed is \(ack.requestId)/\(ack.revision)/\(ack.pageCount)p/\(ack.listDigest.prefix(12))…")
        }
        let n = d.pageCount
        // --- bounds before indexing (the reader already enforced counts) ---
        var changedIndex: [Int: RenderingV2Fast.DeltaEnvelope.ChangedPage] = [:]
        var last = 0
        for cp in d.changedPages {
            let num = cp.page.number
            guard num >= 1, num <= n, num > last else { throw Refusal.pageCount("changed page numbers must be ascending within 1...\(n) (found \(num) after \(last))") }
            last = num
            changedIndex[num - 1] = cp
        }
        let baseCount = installed.list.pages.count
        let expectedRemoved = n < baseCount ? Array((n + 1)...baseCount) : []
        guard d.removedPages == expectedRemoved else { throw Refusal.removedPages("removed_pages \(d.removedPages) != \(expectedRemoved)") }
        for i in 0..<n where changedIndex[i] == nil && i >= baseCount {
            throw Refusal.pageCount("page \(i + 1) is neither changed nor within the base's \(baseCount) pages")
        }
        // --- page_bytes verification + exact target, before any page is built ---
        for i in 0..<n {
            if let cp = changedIndex[i] {
                guard cp.wireBytes == d.pageBytes[i] else { throw Refusal.pageBytesMismatch(page: i + 1, wire: d.pageBytes[i], computed: cp.wireBytes) }
            } else {
                guard let got = relocatedPageBytes(installed.list.pages[i], cached: installed.pageBytes[i], d.relocations) else { throw Refusal.relocationInvalid(page: i + 1) }
                guard got == d.pageBytes[i] else { throw Refusal.pageBytesMismatch(page: i + 1, wire: d.pageBytes[i], computed: got) }
            }
        }
        guard let target = fullLineBytes(raw: d.raw, pageBytes: d.pageBytes) else { throw Refusal.targetOversize(bytes: Int.max, cap: cap) }
        guard target <= cap, n <= maxSnapshotPages else { throw Refusal.targetOversize(bytes: target, cap: cap) }
        // --- reconstruction ---
        var pages: [RenderingV2.Page] = []
        pages.reserveCapacity(n)
        var digests: [Data] = []
        digests.reserveCapacity(n)
        for i in 0..<n {
            let page: RenderingV2.Page
            if let cp = changedIndex[i] { page = cp.page } else {
                guard let moved = relocatePage(installed.list.pages[i], d.relocations) else { throw Refusal.relocationInvalid(page: i + 1) }
                page = moved
            }
            let digest = pageDigest(page)
            guard hex(digest) == d.pageDigests[i] else { throw Refusal.digestMismatch(page: i + 1) }
            pages.append(page)
            digests.append(digest)
        }
        let list = RenderingV2.DisplayList(renderFormat: d.renderFormat, coordinateUnit: d.coordinateUnit, colorSpace: d.colorSpace,
                                           textExtraction: d.textExtraction, projectId: d.projectId, revision: d.revision,
                                           requiredFeatures: d.requiredFeatures, documents: d.documents, fonts: d.fonts,
                                           pages: pages, diagnostics: d.diagnostics, navigation: installed.list.navigation)
        guard hex(listDigest(list, pageDigests: digests)) == d.listDigest else { throw Refusal.listDigestMismatch }
        return (RenderingV2.Envelope(protocolVersion: d.protocolVersion, id: d.id, type: RenderingV2.messageType, payload: list), d.pageBytes, target)
    }
}

/// Residency accounting for the delta route (proposal r5 §6.2): exactly
/// three slots, with the in-flight slot held from dispatch of the off-main
/// callback THROUGH its main-thread completion — replacing or dropping the
/// queued line never releases a running callback's allocations. Pure state so
/// it is unit-testable; the shell owns one instance per worker attachment.
struct DisplayDeltaResidency: Equatable {
    struct Charge: Equatable {
        /// Exact serialised size of the model this slot holds (0 for a wire-only slot).
        var modelBytes: Int = 0
        /// Wire line bytes held (the input line captured by the closure / the queued line).
        var lineBytes: Int = 0
        /// Bitmap bytes held (painted frame's rasters / the candidate's preraster set).
        var rasterBytes: Int = 0
        var total: Int { modelBytes + lineBytes + rasterBytes }
    }
    private(set) var installed: Charge?
    private(set) var inFlight: Charge?
    private(set) var queued: Charge?
    private(set) var peakBytes = 0
    private(set) var peakSlots = 0
    private(set) var queuedReplaced = 0
    private(set) var completionsDropped = 0

    var liveBytes: Int { (installed?.total ?? 0) + (inFlight?.total ?? 0) + (queued?.total ?? 0) }
    var liveSlots: Int { [installed, inFlight, queued].compactMap { $0 }.count }

    private mutating func note() { peakBytes = max(peakBytes, liveBytes); peakSlots = max(peakSlots, liveSlots) }

    /// A wire line arrived. Becomes in-flight immediately when nothing is running; otherwise it
    /// replaces the queued line (the older queued line is dropped undecoded, never the running one).
    mutating func arrive(lineBytes: Int) -> Bool {
        if inFlight == nil { inFlight = Charge(lineBytes: lineBytes); note(); return true }
        if queued != nil { queuedReplaced += 1 }
        queued = Charge(lineBytes: lineBytes); note(); return false
    }
    /// The running callback allocated its target (after the exact charge admitted it) and its preraster set.
    mutating func inFlightAllocated(modelBytes: Int, rasterBytes: Int) {
        guard inFlight != nil else { return }
        inFlight!.modelBytes = modelBytes; inFlight!.rasterBytes = rasterBytes; note()
    }
    /// Main-thread completion of the running callback: install (replacing the old installed slot in the
    /// same step) or drop. Only now is the in-flight slot released; the queued line, if any, starts next.
    mutating func complete(install: Bool) -> Bool {
        guard let done = inFlight else { return false }
        if install { installed = Charge(modelBytes: done.modelBytes, lineBytes: 0, rasterBytes: done.rasterBytes) } else { completionsDropped += 1 }
        inFlight = nil
        if let q = queued { inFlight = q; queued = nil }
        note()
        return inFlight != nil
    }
    /// Clearing event: queued dropped, in-flight marked (its completion will be a drop), installed dropped.
    mutating func clear() {
        queued = nil
        installed = nil
        note()
    }
}
