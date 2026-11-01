import Foundation
import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import FlashTeXPadKit

/// IPAD-FOLDER-ACCESS, the package half: the folder bookmark store (encode,
/// resolve, persist, stale, moved, dropped), the inside-the-project check
/// before anything is created, the permission refusal that becomes the
/// "Allow access" offer, the size cap and the conversion to PNG. The editor's
/// paste across a grant runs in the app's `FolderAccessTests` (simulator).
final class PadFolderAccessTests: XCTestCase {
    private var tmp: URL!
    private var suiteName: String!
    private var defaults: UserDefaults!
    private var readOnly: [URL] = []

    override func setUpWithError() throws {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("folder-access-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        suiteName = "flashtexpadkit.tests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
    }

    override func tearDownWithError() throws {
        for url in readOnly { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path) }
        try? FileManager.default.removeItem(at: tmp)
        defaults.removePersistentDomain(forName: suiteName)
    }

    private func folder(_ name: String) throws -> URL {
        let url = tmp.appendingPathComponent(name, isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func path(_ url: URL) -> String { PadFolderBookmarks.path(of: url) }

    /// Bookmarks are the path plus a version, with scripted staleness, moves and breakage.
    final class FakeCodec: @unchecked Sendable {
        var made = 0
        var stale: Set<String> = []
        var moved: [String: URL] = [:]
        var broken: Set<String> = []
        /// Fail for now (a provider offline, a volume unplugged).
        var offline: Set<String> = []
        var codec: PadFolderBookmarks.Codec {
            .init(make: { [self] url in
                made += 1
                return Data("\(url.path)|\(made)".utf8)
            }, resolve: { [self] data in
                let s = String(decoding: data, as: UTF8.self)
                if broken.contains(s) { throw CocoaError(.fileNoSuchFile) }
                if offline.contains(s) { throw CocoaError(.fileReadUnknown) }
                let p = String(s.split(separator: "|")[0])
                return (moved[p] ?? URL(fileURLWithPath: p, isDirectory: true), stale.contains(s))
            })
        }
    }

    // MARK: the bookmark store

    func testPathKeyIgnoresTrailingSlashAndSymlinkedTemp() {
        XCTAssertEqual(PadFolderBookmarks.path(of: URL(fileURLWithPath: "/a/b/", isDirectory: true)), "/a/b")
        XCTAssertEqual(PadFolderBookmarks.path(of: URL(fileURLWithPath: "/a/./c/../b")), "/a/b")
        XCTAssertTrue(PadFolderBookmarks.path("/a/b/c", isInside: "/a/b"))
        XCTAssertTrue(PadFolderBookmarks.path("/a/b", isInside: "/a/b"))
        XCTAssertFalse(PadFolderBookmarks.path("/a/bc", isInside: "/a/b"), "a sibling sharing the prefix")
        XCTAssertTrue(PadFolderBookmarks.path("/x", isInside: "/"))
    }

    func testSystemBookmarkRoundTripCoversTheFolderAndBelow() throws {
        let root = try folder("project")
        let store = PadFolderBookmarks(defaults: defaults)
        XCTAssertNil(store.grant(covering: root))
        try store.grant(root)
        XCTAssertEqual(store.grantedPaths, [path(root)])

        let reopened = PadFolderBookmarks(defaults: defaults) // persisted in UserDefaults
        let sub = root.appendingPathComponent("chapters", isDirectory: true)
        let grant = try XCTUnwrap(reopened.grant(covering: sub))
        XCTAssertEqual(path(grant.scope), path(root))
        XCTAssertEqual(path(grant.folder), path(sub))
        XCTAssertFalse(grant.refreshed)
        XCTAssertNil(reopened.grant(covering: tmp), "a grant covers its folder and below, not its parent")
        XCTAssertNil(reopened.grant(covering: tmp.appendingPathComponent("project-2")), "a sibling sharing the prefix is not covered")
        reopened.revoke(root)
        XCTAssertNil(reopened.grant(covering: root))
        XCTAssertEqual(reopened.grantedPaths, [])
    }

    func testRegrantingTheSameFolderReplacesItsBookmark() throws {
        let fake = FakeCodec()
        let store = PadFolderBookmarks(defaults: defaults, codec: fake.codec)
        let root = try folder("again")
        try store.grant(root)
        try store.grant(root.appendingPathComponent(".", isDirectory: true))
        XCTAssertEqual(store.grantedPaths, [path(root)], "one bookmark per folder path")
        XCTAssertEqual(fake.made, 2)
    }

    func testSystemBookmarkFollowsAMovedFolder() throws {
        let root = try folder("before")
        let store = PadFolderBookmarks(defaults: defaults)
        try store.grant(root)
        let after = tmp.appendingPathComponent("after", isDirectory: true)
        try FileManager.default.moveItem(at: root, to: after)
        let grant = try XCTUnwrap(store.grant(covering: root))
        XCTAssertEqual(path(grant.scope), path(after), "the bookmark resolves to where the folder is now")
    }

    func testStaleBookmarkIsMadeAgain() throws {
        let fake = FakeCodec()
        let store = PadFolderBookmarks(defaults: defaults, codec: fake.codec)
        let root = try folder("p")
        try store.grant(root)
        fake.stale = ["\(root.path)|1"]
        let first = try XCTUnwrap(store.grant(covering: root))
        XCTAssertTrue(first.refreshed)
        XCTAssertEqual(fake.made, 2, "a stale bookmark is made again from the URL it resolved to")
        let second = try XCTUnwrap(store.grant(covering: root))
        XCTAssertFalse(second.refreshed, "the fresh bookmark is the one stored")
        XCTAssertEqual(fake.made, 2)
    }

    func testStaleBookmarkOfAMovedFolderIsReKeyed() throws {
        let fake = FakeCodec()
        let store = PadFolderBookmarks(defaults: defaults, codec: fake.codec)
        let old = try folder("old"), new = try folder("new")
        try store.grant(old)
        fake.stale = ["\(old.path)|1"]
        fake.moved = [old.path: new]
        let grant = try XCTUnwrap(store.grant(covering: old.appendingPathComponent("figures")))
        XCTAssertEqual(path(grant.scope), path(new))
        XCTAssertEqual(path(grant.folder), path(new.appendingPathComponent("figures")))
        XCTAssertEqual(store.grantedPaths, [path(new)])
    }

    func testDeepestGrantWinsAndAnUnresolvableOneIsDropped() throws {
        let fake = FakeCodec()
        let store = PadFolderBookmarks(defaults: defaults, codec: fake.codec)
        let outer = try folder("outer"), inner = try folder("outer/inner")
        // Stored before grants were pruned: both, the inner one made second.
        fake.made = 2
        defaults.set([path(outer): Data("\(outer.path)|1".utf8), path(inner): Data("\(inner.path)|2".utf8)],
                     forKey: PadFolderBookmarks.defaultsKey)
        XCTAssertEqual(path(try XCTUnwrap(store.grant(covering: inner)).scope), path(inner), "the deepest grant covers")
        fake.broken = ["\(inner.path)|2"]
        let grant = try XCTUnwrap(store.grant(covering: inner))
        XCTAssertEqual(path(grant.scope), path(outer), "the deepest grant failed; its ancestor's covers")
        XCTAssertEqual(store.grantedPaths, [path(outer)])
    }

    func testATransientFailureKeepsTheBookmark() throws {
        let fake = FakeCodec()
        let store = PadFolderBookmarks(defaults: defaults, codec: fake.codec)
        let root = try folder("provider")
        try store.grant(root)
        fake.offline = ["\(root.path)|1"]
        XCTAssertNil(store.grant(covering: root), "an offline provider grants nothing for now")
        XCTAssertEqual(store.grantedPaths, [path(root)], "but its bookmark is kept")
        fake.offline = []
        XCTAssertEqual(path(try XCTUnwrap(store.grant(covering: root)).scope), path(root), "and works once it is back")
        XCTAssertFalse(PadFolderBookmarks.isPermanent(CocoaError(.fileReadUnknown)))
        XCTAssertTrue(PadFolderBookmarks.isPermanent(NSError(domain: NSCocoaErrorDomain, code: NSFileReadUnknownError, userInfo: [
            NSUnderlyingErrorKey: NSError(domain: NSPOSIXErrorDomain, code: Int(ENOENT))])))
    }

    func testAGrantPrunesTheGrantsItCovers() throws {
        let fake = FakeCodec()
        let store = PadFolderBookmarks(defaults: defaults, codec: fake.codec)
        let outer = try folder("top"), a = try folder("top/a"), b = try folder("top/b/c"), sibling = try folder("top-2")
        try store.grant(a)
        try store.grant(b)
        try store.grant(sibling)
        XCTAssertEqual(store.grantedPaths.count, 3)
        try store.grant(outer)
        XCTAssertEqual(store.grantedPaths, [path(outer), path(sibling)].sorted(), "grants below the new one are pruned")
        try store.grant(a)
        XCTAssertEqual(store.grantedPaths, [path(outer), path(sibling)].sorted(), "a folder a live grant covers is not stored again")
        XCTAssertEqual(path(try XCTUnwrap(store.grant(covering: a)).scope), path(outer))

        // A grant above that is gone for good does not cover: it is dropped and the new one stored.
        fake.broken = ["\(outer.path)|4"]
        try store.grant(a)
        XCTAssertEqual(store.grantedPaths, [path(a), path(sibling)].sorted())
    }

    // MARK: saving: the inside-the-project check before anything is created

    func testSymlinkOutsideTheProjectIsRefusedBeforeAnythingIsCreated() throws {
        let root = try folder("proj"), outside = try folder("elsewhere")
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("link"), withDestinationURL: outside)
        XCTAssertFalse(PadImagePaste.resolvesInside(root.appendingPathComponent("link/figures"), root: root))
        XCTAssertTrue(PadImagePaste.resolvesInside(root.appendingPathComponent("figures/new"), root: root))
        XCTAssertThrowsError(try PadImagePaste.save(Data([1]), fileExtension: "png", projectFolder: root, folder: "link/figures")) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .outsideProject("link/figures"))
        }
        XCTAssertFalse(FileManager.default.fileExists(atPath: outside.appendingPathComponent("figures").path),
                       "nothing is created outside the project")
    }

    func testDotDotOutsideTheProjectIsRefusedBeforeAnythingIsCreated() throws {
        let root = try folder("inner-proj")
        XCTAssertThrowsError(try PadImagePaste.save(Data([1]), fileExtension: "png", projectFolder: root, folder: "../escaped")) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .outsideProject("../escaped"))
        }
        XCTAssertFalse(FileManager.default.fileExists(atPath: tmp.appendingPathComponent("escaped").path))
    }

    func testSaveInsideTheProjectCreatesTheFolderAndNeverOverwrites() throws {
        let root = try folder("ok")
        let date = Date(timeIntervalSince1970: 1_800_000_000)
        let first = try PadImagePaste.save(Data([1]), fileExtension: "png", projectFolder: root, folder: "figures", date: date)
        let second = try PadImagePaste.save(Data([2]), fileExtension: "png", projectFolder: root, folder: "figures", date: date)
        XCTAssertNotEqual(first, second)
        XCTAssertTrue(first.hasPrefix("figures/pasted-"), first)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("figures").path).count, 2)
    }

    func testUnwritableFolderIsNoAccess() throws {
        let root = try folder("locked")
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: root.path)
        readOnly.append(root)
        XCTAssertThrowsError(try PadImagePaste.save(Data([1]), fileExtension: "png", projectFolder: root, folder: "figures")) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .noAccess("locked"))
        }
        XCTAssertTrue(PadImagePaste.isPermissionDenied(CocoaError(.fileWriteNoPermission)))
        XCTAssertTrue(PadImagePaste.isPermissionDenied(NSError(domain: NSCocoaErrorDomain, code: 4, userInfo: [
            NSUnderlyingErrorKey: NSError(domain: NSPOSIXErrorDomain, code: Int(EPERM))])))
        XCTAssertFalse(PadImagePaste.isPermissionDenied(CocoaError(.fileWriteOutOfSpace)))
    }

    func testInvisibleProjectFolderIsNoAccessNotOutside() throws {
        // A sandbox hides the folder of a file granted on its own; a folder
        // whose parent cannot be searched stands in for it here.
        let parent = try folder("hidden-parent")
        let root = try folder("hidden-parent/thesis")
        try FileManager.default.setAttributes([.posixPermissions: 0o000], ofItemAtPath: parent.path)
        readOnly.append(parent)
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.path))
        XCTAssertThrowsError(try PadImagePaste.save(Data([1]), fileExtension: "png", projectFolder: root, folder: "figures")) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .noAccess("thesis"), "the Allow access offer, not outsideProject")
        }
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: parent.path)
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("figures").path))
    }

    // MARK: the size cap and the conversion

    func testSizeCap() throws {
        XCTAssertNil(PadImagePaste.refusal(byteCount: PadImagePaste.maximumBytes))
        XCTAssertEqual(PadImagePaste.refusal(byteCount: PadImagePaste.maximumBytes + 1),
                       "The pasted image is larger than 50 MB; it was not saved.")
        let root = try folder("big")
        XCTAssertThrowsError(try PadImagePaste.save(Data(count: PadImagePaste.maximumBytes + 1), fileExtension: "png",
                                                    projectFolder: root, folder: "figures")) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .tooLarge)
        }
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("figures").path))
    }

    func testConversionToPNGAppliesOrientation() throws {
        XCTAssertNil(PadImagePaste.pngData(from: Data("not an image".utf8)))
        // A 4×2 TIFF tagged "rotate 90°" becomes an upright 2×4 PNG.
        let png = try XCTUnwrap(PadImagePaste.pngData(from: try Self.image(width: 4, height: 2, type: .tiff, orientation: 6)))
        let source = try XCTUnwrap(CGImageSourceCreateWithData(png as CFData, nil))
        XCTAssertEqual(CGImageSourceGetType(source) as String?, UTType.png.identifier)
        let props = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any])
        XCTAssertEqual(props[kCGImagePropertyPixelWidth] as? Int, 2)
        XCTAssertEqual(props[kCGImagePropertyPixelHeight] as? Int, 4)
        // Over the pixel cap: refused from the header, before decoding.
        XCTAssertFalse(PadImagePaste.tooManyPixels(width: 10_000, height: 5_000))
        XCTAssertTrue(PadImagePaste.tooManyPixels(width: 10_000, height: 5_001))
        XCTAssertTrue(PadImagePaste.tooManyPixels(width: Int.max, height: 2), "overflow is over the cap")
        let huge = try Self.resized(Self.image(width: 1, height: 1, type: .jpeg), width: 8_000, height: 8_000)
        XCTAssertEqual(CGImageSourceCopyPropertiesAtIndex(try XCTUnwrap(CGImageSourceCreateWithData(huge as CFData, nil)), 0, nil)
            .flatMap { ($0 as? [CFString: Any])?[kCGImagePropertyPixelWidth] as? Int }, 8_000)
        XCTAssertLessThan(huge.count, 1_000_000, "a small file can be a huge bitmap")
        XCTAssertThrowsError(try PadImagePaste.convertToPNG(huge)) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .tooManyPixels)
        }
        XCTAssertEqual(PadImagePaste.SaveError.tooManyPixels.errorDescription,
                       "The pasted image has more than 50 megapixels; it was not saved.")
        XCTAssertThrowsError(try PadImagePaste.convertToPNG(Data("x".utf8))) {
            XCTAssertEqual($0 as? PadImagePaste.SaveError, .unreadableImage)
        }
        // GIF: its first frame.
        XCTAssertNotNil(PadImagePaste.pngData(from: try Self.image(width: 3, height: 3, type: .gif)))
    }

    // MARK: "Open folder…"

    func testMainDocumentOfAFolder() throws {
        let a = try folder("a")
        XCTAssertNil(PadProjectFolder.mainDocument(in: a))
        try "notes".write(to: a.appendingPathComponent("b-notes.tex"), atomically: true, encoding: .utf8)
        XCTAssertEqual(PadProjectFolder.mainDocument(in: a)?.lastPathComponent, "b-notes.tex")
        try "\\documentclass{article}".write(to: a.appendingPathComponent("paper.tex"), atomically: true, encoding: .utf8)
        XCTAssertEqual(PadProjectFolder.mainDocument(in: a)?.lastPathComponent, "paper.tex")
        try "\\input{paper}".write(to: a.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        XCTAssertEqual(PadProjectFolder.mainDocument(in: a)?.lastPathComponent, "main.tex")
    }

    /// `jpeg` with its frame header (SOF0) claiming `width` × `height`: the
    /// header alone, as the pixel cap reads it, without a huge bitmap in the test.
    static func resized(_ jpeg: Data, width: Int, height: Int) throws -> Data {
        var b = [UInt8](jpeg)
        var i = 2
        while i + 8 < b.count, !(b[i] == 0xFF && b[i + 1] == 0xC0) { i += 2 + (Int(b[i + 2]) << 8) + Int(b[i + 3]) }
        guard i + 8 < b.count else { throw CocoaError(.fileReadCorruptFile) }
        b[i + 5] = UInt8(height >> 8); b[i + 6] = UInt8(height & 0xFF)
        b[i + 7] = UInt8(width >> 8); b[i + 8] = UInt8(width & 0xFF)
        return Data(b)
    }

    static func image(width: Int, height: Int, type: UTType, orientation: Int? = nil) throws -> Data {
        let ctx = try XCTUnwrap(CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                                          space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        ctx.setFillColor(CGColor(red: 1, green: 0, blue: 0, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: width, height: height))
        let out = NSMutableData()
        let dest = try XCTUnwrap(CGImageDestinationCreateWithData(out as CFMutableData, type.identifier as CFString, 1, nil))
        let props: [CFString: Any] = orientation.map { [kCGImagePropertyOrientation: $0] } ?? [:]
        CGImageDestinationAddImage(dest, try XCTUnwrap(ctx.makeImage()), props as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(dest))
        return out as Data
    }
}
