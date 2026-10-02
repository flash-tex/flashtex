import FlashTeXPadKit
import ImageIO
import UIKit
import UniformTypeIdentifiers
import XCTest
@testable import FlashTeXPad

/// IPAD-FOLDER-ACCESS, the app half: the editor's paste across the "Allow
/// access" offer and a folder grant, the off-main conversion, a reload of the
/// same file versus another document, and the status note's lifetime. The
/// bookmark store, the inside-the-project check and the conversion itself are
/// FlashTeXPadKit's `PadFolderAccessTests` (`swift test`).
@MainActor
final class FolderAccessTests: XCTestCase {
    private var tmp: URL!
    private var suiteName: String!
    private var defaults: UserDefaults!
    private var readOnly: [URL] = []

    override func setUp() async throws {
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("folder-access-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        suiteName = "flashtexpad.tests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
    }

    override func tearDown() async throws {
        for url in readOnly { try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path) }
        try? FileManager.default.removeItem(at: tmp)
        defaults.removePersistentDomain(forName: suiteName)
    }

    private func folder(_ name: String) throws -> URL {
        let url = tmp.appendingPathComponent(name, isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func makeReadOnly(_ url: URL) throws {
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: url.path)
        readOnly.append(url)
    }

    private func path(_ url: URL) -> String { PadFolderBookmarks.path(of: url) }

    /// A codec whose bookmarks are the path plus a version, with scripted staleness.
    final class FakeCodec: @unchecked Sendable {
        var made = 0
        var stale: Set<String> = []
        var moved: [String: URL] = [:]
        var broken: Set<String> = []
        var codec: PadFolderBookmarks.Codec {
            .init(make: { [self] url in
                made += 1
                return Data("\(url.path)|\(made)".utf8)
            }, resolve: { [self] data in
                let s = String(decoding: data, as: UTF8.self)
                if broken.contains(s) { throw CocoaError(.fileNoSuchFile) }
                let p = String(s.split(separator: "|")[0])
                return (moved[p] ?? URL(fileURLWithPath: p, isDirectory: true), stale.contains(s))
            })
        }
    }

    // MARK: the editor's paste

    static let doc = "\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\nHello\n\\end{document}\n"

    static func image(width: Int, height: Int, type: UTType = .png, orientation: Int? = nil) throws -> Data {
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

    private func pasteboard(_ data: Data, type: UTType) -> UIPasteboard {
        let pb = UIPasteboard(name: UIPasteboard.Name("flashtexpad.test.\(UUID().uuidString)"), create: true)!
        pb.setData(data, forPasteboardType: type.identifier)
        addTeardownBlock { @MainActor in UIPasteboard.remove(withName: pb.name) }
        return pb
    }

    /// A model with `main.tex` opened from `folder`, and an editor wired as `EditorView` wires it.
    private func editor(in folder: URL, codec: PadFolderBookmarks.Codec? = nil) throws -> (PadModel, EditorController) {
        let file = folder.appendingPathComponent("main.tex")
        if !FileManager.default.fileExists(atPath: file.path) { try Self.doc.write(to: file, atomically: true, encoding: .utf8) }
        let model = PadModel(link: MacLink(store: nil),
                             folderBookmarks: PadFolderBookmarks(defaults: defaults, codec: codec ?? FakeCodec().codec))
        model.open(url: file)
        let editor = EditorController()
        editor.onChange = { [weak model, weak editor] text, caret, math in
            guard let model, let editor else { return }
            editor.loadedRevision = model.textChanged(text, caret: caret, mathMode: math)
        }
        editor.imagePasteHost = { [weak model] in model?.imagePasteHost() }
        let d = try XCTUnwrap(model.document)
        let caret = (Self.doc as NSString).range(of: "Hello").upperBound
        editor.load(text: d.text, caret: caret, revision: d.revision)
        return (model, editor)
    }

    /// Every outcome `completion` reported, in order (one per save attempt:
    /// a refused paste, then its retry after a grant).
    private var outcomes: [Bool] = []

    /// The first save attempt's outcome; later ones land in `outcomes`.
    private func paste(_ editor: EditorController, _ pb: UIPasteboard) async -> Bool {
        await withCheckedContinuation { c in
            let handled = editor.pasteImage(from: pb, date: Date(timeIntervalSince1970: 1_800_000_000)) { [weak self] ok in
                guard let self else { return }
                self.outcomes.append(ok)
                if self.outcomes.count == 1 { c.resume(returning: ok) }
            }
            XCTAssertTrue(handled, "types \(pb.types) images \(pb.hasImages) strings \(pb.hasStrings) urls \(pb.hasURLs)")
            if !handled { c.resume(returning: false) }
        }
    }

    func testUnwritableFolderOffersAccessAndTheGrantRetriesThePaste() async throws {
        let fake = FakeCodec()
        let root = try folder("shared")
        let (model, editor) = try editor(in: root, codec: fake.codec)
        try makeReadOnly(root)
        let inserted = await paste(editor, pasteboard(try Self.image(width: 2, height: 2), type: .png))
        XCTAssertFalse(inserted)
        let request = try XCTUnwrap(model.folderAccessRequest, "the paste offers folder access")
        XCTAssertEqual(path(request.folder), path(root))
        XCTAssertTrue(model.editorStatus?.contains("Allow access") == true, model.editorStatus ?? "")
        XCTAssertFalse(editor.text.contains("includegraphics"))

        // A pick that does not contain the folder keeps the offer and says what to choose.
        model.grantFolderAccess(try folder("unrelated"))
        XCTAssertNotNil(model.folderAccessRequest)
        XCTAssertTrue(model.editorStatus?.contains("does not contain “shared”") == true, model.editorStatus ?? "")

        // Granting the folder (here: the file mode the sandbox stands in for) runs the paste again.
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: root.path)
        let retried = expectation(description: "retried paste inserted")
        let watch = Task { @MainActor in
            while !editor.text.contains("includegraphics") { try? await Task.sleep(nanoseconds: 20_000_000) }
            retried.fulfill()
        }
        model.grantFolderAccess(root)
        await fulfillment(of: [retried], timeout: 10)
        watch.cancel()
        XCTAssertEqual(outcomes, [false, true], "one outcome per save attempt: refused, then the retry")
        XCTAssertNil(model.folderAccessRequest)
        XCTAssertTrue(editor.text.contains("{figures/pasted-202701"), editor.text)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("figures").path).count, 1)
        XCTAssertTrue(model.imagePasteHost().hasFolderGrant, "the grant's scope is entered for later pastes")
        XCTAssertEqual(model.imagePasteHost().scopeURLs.count, 2)
    }

    func testTIFFIsConvertedToPNGOffTheMainThread() async throws {
        let (model, editor) = try editor(in: try folder("tiff"))
        let pb = pasteboard(try Self.image(width: 3, height: 3, type: .tiff), type: .tiff)
        guard case .convert = try XCTUnwrap(EditorController.pasteSource(pb)) else { return XCTFail("TIFF bytes are converted, not decoded on main") }
        let inserted = await withCheckedContinuation { c in
            XCTAssertTrue(editor.pasteImage(from: pb) { c.resume(returning: $0) })
            XCTAssertEqual(model.editorStatus, PadImagePaste.convertingNote, "the conversion says so while it runs")
        }
        XCTAssertTrue(inserted)
        XCTAssertTrue(editor.text.contains(".png}"), editor.text)
        XCTAssertTrue(model.editorStatus?.hasPrefix("Pasted image saved as figures/") == true, model.editorStatus ?? "")
    }

    func testReloadOfTheSameFileKeepsTheInFlightPaste() async throws {
        let (model, editor) = try editor(in: try folder("same"))
        let pb = pasteboard(try Self.image(width: 2, height: 2), type: .png)
        let done = await withCheckedContinuation { c in
            XCTAssertTrue(editor.pasteImage(from: pb) { c.resume(returning: $0) })
            // The same file is reloaded behind the editor's back before the save finishes.
            let d = model.document!
            editor.load(text: d.text, caret: 0, revision: d.revision + 100)
        }
        XCTAssertTrue(done, model.editorStatus ?? "")
        XCTAssertTrue(editor.text.contains("\\includegraphics"))
    }

    func testAnotherDocumentDropsTheInFlightPaste() async throws {
        let root = try folder("first")
        let (model, editor) = try editor(in: root)
        let other = try folder("second").appendingPathComponent("other.tex")
        try Self.doc.write(to: other, atomically: true, encoding: .utf8)
        let pb = pasteboard(try Self.image(width: 2, height: 2), type: .png)
        let done = await withCheckedContinuation { c in
            XCTAssertTrue(editor.pasteImage(from: pb) { c.resume(returning: $0) })
            model.open(url: other)
            let d = model.document!
            editor.load(text: d.text, caret: 0, revision: d.revision + 100)
        }
        XCTAssertFalse(done)
        XCTAssertFalse(editor.text.contains("\\includegraphics"))
        XCTAssertTrue(model.editorStatus?.contains("another document was opened") == true, model.editorStatus ?? "")
    }

    // MARK: the status note

    func testStatusNoteClearsOnTheNextEditAndAfterItsLifetime() async throws {
        let model = PadModel(link: MacLink(store: nil), folderBookmarks: PadFolderBookmarks(defaults: defaults))
        model.openBundledSample()
        model.editorStatus = "Pasted image saved as figures/x.png."
        model.textChanged(model.document!.text + "x")
        XCTAssertNil(model.editorStatus, "the next edit clears the note")

        model.editorStatusLifetime = 0.1
        model.editorStatus = "saved"
        model.textChanged(model.document!.text)
        XCTAssertEqual(model.editorStatus, "saved", "a caret-only report is no edit")
        try await Task.sleep(nanoseconds: 400_000_000)
        XCTAssertNil(model.editorStatus, "the note clears itself after a few seconds")
    }
}
