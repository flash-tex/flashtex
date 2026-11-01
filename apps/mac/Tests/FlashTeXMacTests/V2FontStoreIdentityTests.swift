import XCTest
import CoreGraphics
import CryptoKit
import FlashTeXProtocol
@testable import FlashTeXMac

/// GH31: `V2FontStore` hashed the discovery bytes at init but built the
/// CGFont from a fresh read of the same path, caching it under the OLD hash.
/// The store must authenticate the bytes it actually loads (exact length and
/// raw SHA-256 against the immutable discovery record) before any CGFont is
/// constructed, and a verified CGFont must stay immutable for older frames.
final class V2FontStoreIdentityTests: XCTestCase {
    static let bundledFonts = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")

    /// A private copy of one pinned Latin Modern face so the test can change
    /// the file without touching the repository copy.
    private func stagedCopy(of name: String) throws -> (dir: URL, file: URL, original: Data) {
        let source = Self.bundledFonts.appendingPathComponent(name)
        guard let original = try? Data(contentsOf: source) else { throw XCTSkip("bundled \(name) not found") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("v2-font-identity-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent(name)
        try original.write(to: file)
        return (dir, file, original)
    }

    private func manifest(for file: V2FontStore.BundledFont, bytes: Data, sha256: String) throws -> RenderingV2.FontResource {
        let cg = try XCTUnwrap(CGFont(CGDataProvider(data: bytes as CFData)!))
        return RenderingV2.FontResource(fontId: "lm10", sha256: sha256, byteLength: file.byteLength, format: "opentype-cff",
                                        faceIndex: 0, unitsPerEm: Int(cg.unitsPerEm), glyphCount: Int(cg.numberOfGlyphs),
                                        postscriptName: (cg.postScriptName as String?) ?? "")
    }

    /// The path changes between discovery and the first resolve (one zero byte
    /// appended: CoreGraphics still parses the font and its metadata — glyph
    /// count, units per em, PostScript name — is unchanged, only the raw
    /// identity differs). The store must refuse before constructing a CGFont;
    /// nothing is cached under the discovered hash.
    func testChangedBytesAfterDiscoveryAreRefusedBeforeCGFontConstruction() throws {
        let staged = try stagedCopy(of: "lmroman10-regular.otf")
        defer { try? FileManager.default.removeItem(at: staged.dir) }
        let store = V2FontStore(directories: [staged.dir.path])
        let file = try XCTUnwrap(store.fonts.first)
        XCTAssertEqual(file.bytesSha256, V2FontStore.hex(SHA256.hash(data: staged.original)))
        XCTAssertEqual(file.byteLength, Int64(staged.original.count))
        // The raw-byte digest is the only lookup key; identity is the raw bytes.
        let resourceRaw = try manifest(for: file, bytes: staged.original, sha256: file.bytesSha256)

        var altered = staged.original
        altered.append(0)
        try altered.write(to: staged.file)
        // The altered file is still a loadable font with the same metadata:
        // the metadata checks alone would have accepted it.
        let alteredCG = try XCTUnwrap(CGFont(CGDataProvider(data: altered as CFData)!), "the altered file must still parse as a font for this test to be meaningful")
        XCTAssertEqual(Int(alteredCG.numberOfGlyphs), resourceRaw.glyphCount)
        XCTAssertEqual(Int(alteredCG.unitsPerEm), resourceRaw.unitsPerEm)
        XCTAssertEqual((alteredCG.postScriptName as String?) ?? "", resourceRaw.postscriptName)

        for resource in [resourceRaw] {
            XCTAssertThrowsError(try store.resolve(resource), "sha \(resource.sha256.prefix(12))") { error in
                let v = error as? RenderingV2.ValidationError
                XCTAssertEqual(v?.code, "font_resource_mismatch")
                XCTAssertTrue(v?.message.contains("no longer matches the bytes discovered at startup") == true, v?.message ?? "\(error)")
            }
        }
        // Nothing was cached from the refused bytes: restoring the original bytes resolves.
        try staged.original.write(to: staged.file)
        let resolved = try store.resolve(resourceRaw)
        XCTAssertEqual(resolved.file.bytesSha256, resourceRaw.sha256)
        XCTAssertEqual(Int(resolved.cgFont.numberOfGlyphs), resourceRaw.glyphCount)
    }

    /// A CGFont verified from the original bytes stays exactly that object for
    /// every later resolve, even after the file on disk changes: old frames
    /// keep painting the bytes they were validated against, and the changed
    /// file never replaces the cached font.
    func testVerifiedCGFontStaysImmutableAfterTheFileChanges() throws {
        let staged = try stagedCopy(of: "lmroman10-regular.otf")
        defer { try? FileManager.default.removeItem(at: staged.dir) }
        let store = V2FontStore(directories: [staged.dir.path])
        let file = try XCTUnwrap(store.fonts.first)
        let resource = try manifest(for: file, bytes: staged.original, sha256: file.bytesSha256)
        let first = try store.resolve(resource)

        var altered = staged.original
        altered.append(0)
        try altered.write(to: staged.file)
        let second = try store.resolve(resource) // cached: no reread of the changed path
        XCTAssertTrue(first.cgFont === second.cgFont, "the verified CGFont is reused as the same immutable object")
        XCTAssertEqual(second.file.bytesSha256, resource.sha256)
        // D3: the obsolete bytes+face0 spelling is no lookup key at all — unknown, never the changed file.
        var face0 = staged.original; face0.append(contentsOf: [0, 0, 0, 0])
        XCTAssertThrowsError(try store.resolve(try manifest(for: file, bytes: staged.original, sha256: V2FontStore.hex(SHA256.hash(data: face0))))) {
            XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_unavailable")
        }
    }

    /// The manifest's byte length must equal the discovered file's, and a
    /// hash the store never discovered is unavailable (unchanged behavior).
    func testUnknownHashAndLengthMismatchStayRefused() throws {
        let staged = try stagedCopy(of: "lmroman10-regular.otf")
        defer { try? FileManager.default.removeItem(at: staged.dir) }
        let store = V2FontStore(directories: [staged.dir.path])
        let file = try XCTUnwrap(store.fonts.first)
        var wrongLength = try manifest(for: file, bytes: staged.original, sha256: file.bytesSha256)
        wrongLength.byteLength += 1
        XCTAssertThrowsError(try store.resolve(wrongLength)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_mismatch") }
        var unknown = try manifest(for: file, bytes: staged.original, sha256: file.bytesSha256)
        unknown.sha256 = String(repeating: "0", count: 64)
        XCTAssertThrowsError(try store.resolve(unknown)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "font_resource_unavailable") }
    }
}
