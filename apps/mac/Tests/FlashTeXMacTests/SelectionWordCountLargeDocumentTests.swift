import Foundation
import XCTest
@testable import FlashTeXMac

/// REVIEW-APP round 5: the status bar's "N of M words" counted the
/// selection in the item's body, on the main thread, every time it was
/// drawn. Select All in a 2.8 MB book was 270 ms (debug build) per
/// evaluation, and each Shift-arrow that grew the selection paid it again.
@MainActor
final class SelectionWordCountLargeDocumentTests: XCTestCase {
    private func settled(_ model: WordCountModel, count: Int = 1) async {
        var remaining = count
        await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
            var done = false
            model.onRecomputeSettled = {
                remaining -= 1
                if remaining <= 0, !done { done = true; model.onRecomputeSettled = nil; c.resume() }
            }
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: 20_000_000_000)
                if !done { done = true; model.onRecomputeSettled = nil; c.resume() }
            }
        }
    }

    func testSelectAllInABookIsCountedOffTheMainThread() async {
        let book = EditorInstantTests.book(sections: 720)
        let all = NSRange(location: 0, length: (book as NSString).length)
        let model = WordCountModel()
        model.scheduleDebounce = { _, item in item.perform() }
        var ranOnMain: Bool?
        model.recomputeExecutor = { work in
            DispatchQueue.global(qos: .userInitiated).async { ranOnMain = Thread.isMainThread; work() }
        }

        // What the status item asks while the count is pending: no count, at no cost.
        let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        let pending = model.selectionWords(text: book, path: "main.tex", range: all, revision: 7)
        let ms = Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6
        XCTAssertNil(pending)
        XCTAssertEqual(model.selectionCountsInline, 0, "a 3 MB selection was counted where the status item is drawn")
        // Before: 268 ms (DocumentStatistics.wordCount(inSelection:) in the body).
        TimingBudget.assertWithin(ms, 1, "status item's selection count for Select All in a \(book.utf8.count)-byte book (thread CPU)")

        model.scheduleSelectionCount(text: book, path: "main.tex", range: all, revision: 7)
        await settled(model)
        XCTAssertEqual(ranOnMain, false)
        let want = DocumentStatistics.wordCount(inSelection: book, utf16Range: all)
        XCTAssertEqual(model.selectionWords(text: book, path: "main.tex", range: all, revision: 7), want)
        XCTAssertEqual(model.selectionCountsInline, 0)
        // The same selection again is not counted again.
        model.recomputeExecutor = { _ in XCTFail("recounted an unchanged selection") }
        model.scheduleSelectionCount(text: book, path: "main.tex", range: all, revision: 7)
    }

    func testOnlyACountForExactlyThisSelectionAndRevisionIsShown() async {
        let text = String(repeating: "word ", count: 10_000) // 50 000 UTF-16 units
        let model = WordCountModel()
        model.scheduleDebounce = { _, item in item.perform() }
        model.recomputeExecutor = { work in work() }
        let a = NSRange(location: 0, length: 40_000), b = NSRange(location: 0, length: 45_000)
        model.scheduleSelectionCount(text: text, path: "main.tex", range: a, revision: 1)
        model.scheduleSelectionCount(text: text, path: "main.tex", range: b, revision: 1)
        await settled(model, count: 2)
        XCTAssertNil(model.selectionWords(text: text, path: "main.tex", range: a, revision: 1), "a superseded count landed")
        XCTAssertEqual(model.selectionWords(text: text, path: "main.tex", range: b, revision: 1), 9_000)
        XCTAssertNil(model.selectionWords(text: text, path: "main.tex", range: b, revision: 2), "a count of an older revision")
        XCTAssertNil(model.selectionWords(text: text, path: "other.tex", range: b, revision: 1), "a count of another document")
    }

    func testAShortSelectionIsCountedWhereItIsShown() {
        let model = WordCountModel()
        model.recomputeExecutor = { _ in XCTFail("a short selection went to the background") }
        model.scheduleDebounce = { _, item in item.perform() }
        let text = "One two three four five."
        let range = (text as NSString).range(of: "two three four")
        model.scheduleSelectionCount(text: text, path: "main.tex", range: range, revision: 1)
        XCTAssertEqual(model.selectionWords(text: text, path: "main.tex", range: range, revision: 1), 3)
        XCTAssertEqual(model.selectionCountsInline, 1)
        XCTAssertNil(model.selectionWords(text: text, path: "main.tex", range: NSRange(location: 3, length: 0), revision: 1))
    }
}
