import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// Form ids are the connection's until a `STARTED` says `"keep": false`
/// (display-list-v3.md §5, §6.4): another job (a chapter focus, another main
/// file) or a restarted host numbers its `\pdfxform`s from 1 again. The pages
/// kept on screen, stale, until the new run sends them were made with the old
/// numbering: they must keep drawing the forms they were made with, never the
/// new run's form that reuses the id.
@MainActor
final class EngineV3FormGenerationTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-form-gen-\(getpid())")
    private var env = EnvironmentOverride()

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
    }

    override func tearDownWithError() throws {
        env.restore()
        try? FileManager.default.removeItem(at: Self.cache)
    }

    private func v3Model() -> ShellModel {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n", named: "main.tex")
        model.engineV3Enabled = true
        return model
    }

    private static func hash(_ tag: UInt8) -> [UInt8] { [UInt8](repeating: tag, count: 32) }

    private static func form(_ id: UInt32, _ tag: UInt8) -> DL3PreparedPage {
        var p = DL3Page(kind: .form, index: id)
        p.hash = hash(tag)
        return DL3PreparedPage(page: p, fonts: [:], images: [:])
    }

    private static func page(_ index: UInt32, forms ids: [UInt32], _ tag: UInt8) -> DL3PreparedPage {
        var p = DL3Page(kind: .page, index: index)
        p.width = 612 * 65536; p.height = 792 * 65536; p.box = [0, 0, 612, 792]
        p.hash = hash(tag)
        p.items = ids.map { .form(id: $0, matrix: 0) }
        return DL3PreparedPage(page: p, fonts: [:], images: [:])
    }

    private func drawn(_ s: EngineV3Session, page i: Int, form id: UInt32) -> [UInt8]? {
        s.forms(forPage: i)[id]?.page.hash
    }

    func testAStalePageKeepsTheFormsOfItsOwnRunAfterKeepFalse() throws {
        let model = v3Model()
        defer { model.engineV3Enabled = false }
        let s = model.engineV3

        // Run 1: page 0 draws form 1, page 1 draws form 2 (each FORM after its page, as pdfTeX writes them).
        s.handle(.started(.object(["id": .int(1)])))
        s.handle(.page(Self.page(0, forms: [1], 0xA0), compileID: 1, timing: .init(), image: nil))
        s.handle(.form(Self.form(1, 0xA1)))
        s.handle(.page(Self.page(1, forms: [2], 0xA3), compileID: 1, timing: .init(), image: nil))
        s.handle(.form(Self.form(2, 0xA2)))
        s.handle(.done(.object(["id": .int(1), "status": .string("ok"), "pages": .int(2)]), compileID: 1))
        XCTAssertEqual(drawn(s, page: 1, form: 2), Self.hash(0xA2))

        // Run 2, another job: ids restart. Its page 0 draws forms 1 and 2, its own.
        s.handle(.started(.object(["id": .int(2), "keep": .bool(false)])))
        XCTAssertEqual(s.stale, [0, 1])
        s.handle(.page(Self.page(0, forms: [1, 2], 0xB0), compileID: 2, timing: .init(), image: nil))
        s.handle(.form(Self.form(1, 0xB1)))
        s.handle(.form(Self.form(2, 0xB2)))
        XCTAssertEqual(drawn(s, page: 0, form: 1), Self.hash(0xB1), "the new page draws the new run's forms")
        XCTAssertEqual(drawn(s, page: 0, form: 2), Self.hash(0xB2))

        // Page 1 is still the first run's (stale): its form 2 is that run's, not the new form 2.
        XCTAssertTrue(s.stale.contains(1))
        XCTAssertEqual(drawn(s, page: 1, form: 2), Self.hash(0xA2), "a stale page drew another run's form under its id")
        let key = EngineV3PagesView.formKey(try XCTUnwrap(s.pages[1]), s.forms(forPage: 1))
        XCTAssertTrue(key.starts(with: [2, 0, 0, 0] + Self.hash(0xA2)), "the tile key names the form the page draws")

        // When run 2 sends page 1, it draws run 2's forms.
        s.handle(.page(Self.page(1, forms: [2], 0xB3), compileID: 2, timing: .init(), image: nil))
        XCTAssertEqual(drawn(s, page: 1, form: 2), Self.hash(0xB2))
        s.handle(.done(.object(["id": .int(2), "status": .string("ok"), "pages": .int(2)]), compileID: 2))
        XCTAssertEqual(drawn(s, page: 1, form: 2), Self.hash(0xB2))
    }

    /// Within one run of ids (`keep: true`) a form is one PDF object: an
    /// edit that changes form K's content sends FORM K with the edited page,
    /// and the pages the host does not send again draw the new content, as
    /// pdflatex's PDF does (one object K). Only `keep: false` holds forms.
    func testWithKeepTrueEveryPageDrawsTheNewestFormOfItsID() {
        let model = v3Model()
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        s.handle(.started(.object(["id": .int(1)])))
        s.handle(.page(Self.page(0, forms: [3], 0xA0), compileID: 1, timing: .init(), image: nil))
        s.handle(.form(Self.form(3, 0xA3)))
        s.handle(.page(Self.page(1, forms: [3], 0xA1), compileID: 1, timing: .init(), image: nil))
        s.handle(.done(.object(["id": .int(1), "status": .string("ok"), "pages": .int(2)]), compileID: 1))
        s.handle(.started(.object(["id": .int(2), "keep": .bool(true)])))
        s.handle(.page(Self.page(0, forms: [3], 0xB0), compileID: 2, timing: .init(), image: nil))
        s.handle(.form(Self.form(3, 0xB3)))
        s.handle(.done(.object(["id": .int(2), "status": .string("ok"), "pages": .int(2)]), compileID: 2))
        XCTAssertEqual(drawn(s, page: 1, form: 3), Self.hash(0xB3), "a page the host kept draws the form's new content")
    }

    /// A form of the old run that the new run never defines is gone with the
    /// old run: a page of the new run never draws it.
    func testANewRunPageNeverDrawsAFormOfTheOldRun() {
        let model = v3Model()
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        s.handle(.started(.object(["id": .int(1)])))
        s.handle(.page(Self.page(0, forms: [7], 0xA0), compileID: 1, timing: .init(), image: nil))
        s.handle(.form(Self.form(7, 0xA7)))
        s.handle(.done(.object(["id": .int(1), "status": .string("ok"), "pages": .int(1)]), compileID: 1))
        s.handle(.started(.object(["id": .int(2), "keep": .bool(false)])))
        XCTAssertEqual(drawn(s, page: 0, form: 7), Self.hash(0xA7), "the stale page keeps its form")
        // The new page 0 draws form 7 before its FORM arrives (a FORM may follow its page, §5).
        s.handle(.page(Self.page(0, forms: [7], 0xB0), compileID: 2, timing: .init(), image: nil))
        XCTAssertNil(drawn(s, page: 0, form: 7), "the new page drew the old run's form 7")
        s.handle(.form(Self.form(7, 0xB7)))
        XCTAssertEqual(drawn(s, page: 0, form: 7), Self.hash(0xB7))
    }
}
