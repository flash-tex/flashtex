import Foundation
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// REVIEW-APP round 5: Go to Symbol scanned the outline of every open
/// document on each evaluation of its sheet: each typed letter (twice: the
/// body and the query's onChange) and each arrow key. A 1.9 MB book (under
/// the outline's 2 M-unit cap) is 240 ms per scan in a debug build.
@MainActor
final class SymbolPickerLargeDocumentTests: XCTestCase {
    private func cpuMs(_ body: () -> Void) -> Double {
        let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        body()
        return Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6
    }

    func testTypingAQueryRanksTheOutlinesScannedOnceOffTheMainThread() async {
        let book = EditorInstantTests.book(sections: 480)
        XCTAssertLessThanOrEqual((book as NSString).length, DocumentOutline.maxScannedUTF16, "the book must be outlined")
        let m = ShellModel()
        m.documents = [.init(path: "main.tex", text: book), .init(path: "ch.tex", text: "\\section{Appendix}\\label{app:a}")]
        m.activePath = "main.tex"

        let outlines = await m.symbolPickerOutlinesOffMain()
        XCTAssertEqual(outlines.map(\.path), ["main.tex", "ch.tex"])
        XCTAssertGreaterThan(outlines[0].items.count, 480)

        var perQuery: [Double] = []
        for query in ["", "p", "pa", "par", "part", "part 3", "sec", "section 1", "app", "zzz"] {
            var ranked: [SymbolPickerEntry] = []
            perQuery.append(cpuMs { ranked = ShellModel.symbolPickerEntries(query: query, outlines: outlines) })
            XCTAssertEqual(ranked, m.symbolPickerEntries(query: query), "query \(query.debugDescription)")
        }
        let before = cpuMs { _ = m.symbolPickerEntries(query: "part") } // the scan the sheet used to make per evaluation
        print(String(format: "perf: Go to Symbol per evaluation, scanning: %.1f ms", before))
        TimingBudget.assertWithin(perQuery.max() ?? 0, 5, "Go to Symbol's ranking per query over a \(book.utf8.count)-byte book's outline (thread CPU)")
    }

    func testWrapInEnvironmentSuggestsTheCommonOnesThenTheDocumentsOwn() {
        let c = WrapEnvironmentSheet.candidates(used: ["theorem", "lemma", "itemize", "lemma"])
        XCTAssertEqual(Array(c.prefix(WrapEnvironmentSheet.common.count)), WrapEnvironmentSheet.common)
        XCTAssertEqual(Array(c.dropFirst(WrapEnvironmentSheet.common.count)), ["lemma"])
    }
}
