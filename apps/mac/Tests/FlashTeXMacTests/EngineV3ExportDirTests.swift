import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// An export runs in a folder of its own (spec §6.3 `export_dir`, the
/// Commander's ruling of 2026-11-02): the project copy's `export` folder in
/// the app's cache, beside its `src` and `out`, so the session's `.aux` and
/// log are never written by it. The host's side is tested by
/// `host_incremental.rs` and the Unicode host's `host.rs`.
final class EngineV3ExportDirTests: XCTestCase {
    func testAnExportNamesItsOwnFolderBesideTheCopy() {
        let base = URL(fileURLWithPath: "/cache/projects/k-1-2", isDirectory: true)
        let dir = EngineV3Session.exportDirectory(of: base)
        XCTAssertEqual(dir.path, "/cache/projects/k-1-2/export")
        XCTAssertNotEqual(dir.path, base.appendingPathComponent("out").path)
        XCTAssertNotEqual(dir.path, base.appendingPathComponent("src").path)

        var r = DL3CompileRequest(id: 1, root: "/cache/projects/k-1-2/src", main: "main.tex")
        XCTAssertNil(r.json["export_dir"], "a preview compile names none")
        r.export = true
        r.exportDir = dir.path
        XCTAssertEqual(r.json["export_dir"]?.string, "/cache/projects/k-1-2/export")
        XCTAssertEqual(r.json["export"]?.bool, true)
    }
}
