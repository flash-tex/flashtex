import CryptoKit
import Foundation
import FlashTeXProtocol

/// Bounded, immutable-identity reuse of decoded and prepared rendering-v2 pages
/// across frames. Under typing, the producer re-emits every page of the
/// document with each keystroke and all but the edited page arrive as exactly
/// the same bytes; decoding, validating, font-resolving, preparing and
/// rasterizing them again is the largest Mac-side cost of the v2 route
/// (measured: validate 28 ms of a 4-page sibling, 46 ms of 8 pages).
///
/// Identity is the raw bytes: a page is reused only for an incoming page
/// object whose SHA-256 and byte length equal the bytes it was decoded from,
/// prepared against the same font manifest bytes (the `fonts` array's
/// SHA-256 — a `font_id` that later names other bytes never lends its glyphs)
/// and the same font store. Identical bytes decode to identical values
/// (`RenderingV2Fast` is deterministic; `PreviewLatencyTests` checks the
/// painted frame's items equal a fresh decode), so nothing the consumer
/// checks changes: the whole list is still validated every time
/// (`RenderingV2.validate` over the reused pages included), fonts are still
/// resolved by content hash, and the D1/candidate bindings run on the result
/// exactly as before. Retention is bounded by entry count and source bytes;
/// the least recently used pages go first.
final class V2PageCache {
    struct Key: Hashable {
        /// SHA-256 of the page object's raw bytes and their length.
        var sha256: String
        var byteLength: Int
        /// The font store the page was prepared against (a different store may
        /// resolve the same hash differently or not at all).
        var store: ObjectIdentifier
        /// The image store and its root: image bytes are keyed by content
        /// hash, but a refusal (no root, symlink, stale bytes) is per root.
        var images: ObjectIdentifier
        var imageRoot: String
    }

    struct Entry {
        var page: RenderingV2.Page
        var prepared: V2PreparedPage
        /// SHA-256 of the `fonts` array bytes the page was prepared under.
        var fontsSha256: String
        /// Content identity of the page for bitmap keys and view equality.
        var token: String
    }

    static let shared = V2PageCache()

    let maxEntries: Int
    let maxBytes: Int
    private let lock = NSLock()
    private var entries: [Key: Entry] = [:]
    private var order: [Key] = []
    private(set) var retainedBytes = 0
    private(set) var hits = 0
    private(set) var misses = 0

    init(maxEntries: Int = 64, maxBytes: Int = 96 << 20) {
        self.maxEntries = maxEntries
        self.maxBytes = maxBytes
    }

    var count: Int { lock.lock(); defer { lock.unlock() }; return entries.count }

    static func sha256(_ bytes: UnsafeRawBufferPointer) -> String { V2FontStore.hex(SHA256.hash(data: bytes)) }
    static func sha256(_ data: Data) -> String { V2FontStore.hex(SHA256.hash(data: data)) }

    static func token(sha256: String, byteLength: Int, fontsSha256: String) -> String {
        "\(sha256.prefix(24))-\(byteLength)-\(fontsSha256.prefix(12))"
    }

    func lookup(_ key: Key) -> Entry? {
        lock.lock(); defer { lock.unlock() }
        guard let e = entries[key] else { misses += 1; return nil }
        hits += 1
        if let i = order.firstIndex(of: key) { order.remove(at: i); order.append(key) }
        return e
    }

    func store(_ entry: Entry, for key: Key) {
        lock.lock(); defer { lock.unlock() }
        if entries.updateValue(entry, forKey: key) == nil {
            retainedBytes += key.byteLength
        } else if let i = order.firstIndex(of: key) {
            order.remove(at: i)
        }
        order.append(key)
        while (entries.count > maxEntries || retainedBytes > maxBytes), order.count > 1, let oldest = order.first {
            order.removeFirst()
            entries.removeValue(forKey: oldest)
            retainedBytes -= oldest.byteLength
        }
    }

    func clear() {
        lock.lock(); defer { lock.unlock() }
        entries = [:]; order = []; retainedBytes = 0
    }
}

extension V2Frame {
    /// Decodes, validates, font-resolves and prepares one `display_list`
    /// envelope's bytes, reusing pages from `cache` whose raw bytes (and font
    /// manifest bytes, and store) match. Semantics equal `RenderingV2.decode`
    /// followed by `prepare(_:store:)`: every page of the result is either the
    /// value the fast reader produced for those bytes or a decoded page
    /// stored under its own bytes' hash, the whole list is validated, and a
    /// failure anywhere yields no frame. Falls back to the plain path (no
    /// reuse) whenever the fast reader does not accept the input.
    static func prepare(data: Data, store: V2FontStore = .shared, cache: V2PageCache? = .shared, images: V2ImageStore = .shared) throws -> V2Frame {
        guard let cache else { return try prepare(try RenderingV2.decode(data), store: store, images: images) }
        let storeID = ObjectIdentifier(store)
        let imagesID = ObjectIdentifier(images), imageRoot = images.rootKey
        var reused: [Int: V2PageCache.Entry] = [:]
        var keys: [Int: V2PageCache.Key] = [:]
        let decoded: RenderingV2Fast.Decoded
        do {
            decoded = try RenderingV2Fast.envelope(data) { index, bytes in
                let key = V2PageCache.Key(sha256: V2PageCache.sha256(bytes), byteLength: bytes.count, store: storeID, images: imagesID, imageRoot: imageRoot)
                keys[index] = key
                guard let entry = cache.lookup(key) else { return nil }
                reused[index] = entry
                return entry.page
            }
        } catch {
            // Not accepted by the fast reader: JSONDecoder decides, as on the plain path.
            return try prepare(try RenderingV2.decode(data), store: store, images: images)
        }
        var envelope = decoded.envelope
        try RenderingV2.checkHeader(version: envelope.protocolVersion, type: envelope.type)
        // Font manifest identity: a reused page prepared under other manifest
        // bytes is decoded afresh instead (its range is known).
        let fontsSha = decoded.fontsRange.map { V2PageCache.sha256(data.subdata(in: $0)) } ?? ""
        for (index, entry) in reused where entry.fontsSha256 != fontsSha {
            envelope.payload.pages[index] = try RenderingV2Fast.page(data, range: decoded.pageRanges[index])
            reused[index] = nil
        }
        try RenderingV2.validate(envelope.payload)
        // Every font referenced by any page resolves by content hash, exactly as
        // `prepare(_:store:)` does; a reused page's fonts are re-resolved too.
        var referenced: [String] = []
        for page in envelope.payload.pages {
            for case .glyphRun(let run) in page.items where !referenced.contains(run.fontId) { referenced.append(run.fontId) }
        }
        var fonts: [String: V2FontStore.ResolvedFont] = [:]
        for id in referenced {
            guard let resource = envelope.payload.font(id: id) else {
                throw RenderingV2.ValidationError(code: "invalid_resource", message: "font resource '\(id)' is not declared")
            }
            fonts[id] = try store.resolve(resource)
        }
        var prepared: [V2PreparedPage] = []
        var tokens: [String] = []
        prepared.reserveCapacity(envelope.payload.pages.count)
        for (index, page) in envelope.payload.pages.enumerated() {
            if let entry = reused[index] {
                prepared.append(entry.prepared)
                tokens.append(entry.token)
                continue
            }
            let p = try V2PreparedPage(page: page, fonts: fonts, images: images)
            let token: String
            if let key = keys[index] {
                token = V2PageCache.token(sha256: key.sha256, byteLength: key.byteLength, fontsSha256: fontsSha)
                cache.store(V2PageCache.Entry(page: page, prepared: p, fontsSha256: fontsSha, token: token), for: key)
            } else {
                token = "page\(page.number)#\(V2Frame.nextNonce())"
            }
            prepared.append(p)
            tokens.append(token)
        }
        var frame = V2Frame(id: envelope.id, list: envelope.payload, fonts: fonts, prepared: prepared, pageTokens: tokens, reusedPages: reused.count)
        frame.pageBytes = decoded.pageRanges.map(\.count)
        return frame
    }
}
