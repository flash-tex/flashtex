import Foundation
import XCTest
@testable import FlashTeXMac

/// Main-thread cost per caret move (review lane REVIEW-APP, round 5): after
/// a manual preview scroll, each caret move (an arrow key, a click) asked at
/// once for the caret's source line and its preview target, each a walk of
/// the whole buffer up to the caret (UTF-16 → UTF-8 offset, line count,
/// line and column). On a 3 MB book that is several O(document) walks per
/// key on main; they are now made once, when the caret settles.
@MainActor
final class CaretFollowCostTests: XCTestCase {
    private var env = EnvironmentOverride()

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("caret-follow-cost-\(UUID().uuidString)").path)
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
        CaretFollow.enabledOverride = true
        CaretFollow.debounceOverride = nil
    }

    override func tearDown() {
        CaretFollow.enabledOverride = nil
        CaretFollow.debounceOverride = nil
        env.restore()
    }

    /// A synthetic book of about 3 MB.
    static func book() -> String {
        var s = "\\documentclass{book}\n\\begin{document}\n"
        let para = String(repeating: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ", count: 4) + "\n\n"
        var k = 0
        while s.utf8.count < 3_000_000 {
            if k % 40 == 0 { s += "\\chapter{Chapter \(k / 40)}\n" }
            s += para
            k += 1
        }
        return s + "\\end{document}\n"
    }

    /// 200 caret moves along the last line after a manual scroll (arrow
    /// keys while reading): each costs no walk of the buffer; the one check
    /// when the caret settles still keeps a same-line move from re-arming.
    func testCaretMovesAfterAManualScrollDoNotWalkTheBuffer() async throws {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: Self.book(), named: "main.tex")
        model.engineV3Enabled = true
        defer { model.engineV3Enabled = false }
        let text = model.activeText as NSString
        let lastLine = text.range(of: "\n\nLorem", options: .backwards).location + 2 // the last line's start (228 characters)
        model.caretUTF16 = lastLine
        model.caretFollow.userDidScrollPreview()
        XCTAssertFalse(model.caretFollow.isArmed)
        let t0 = Date()
        for k in 1 ... 200 { model.caretUTF16 = lastLine + k }
        let ms = Date().timeIntervalSince(t0) * 1000
        print(String(format: "CaretFollowCostTests: 200 caret moves on a %.1f MB buffer held main %.1f ms", Double(model.activeText.utf8.count) / 1e6, ms))
        XCTAssertLessThan(ms, 100, String(format: "%.0f ms for 200 caret moves", ms))
        // Settled: the same line, so following stays off.
        try await Task.sleep(nanoseconds: UInt64((CaretFollow.debounceInterval + 0.3) * 1e9))
        XCTAssertFalse(model.caretFollow.isArmed, "a move within the line does not re-arm")
    }
}
