import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `DocumentStatistics` (GH68): a pure, synchronous texcount-style scan, so
/// every case here is a plain string in, `Result`/`Counts` out — no editor,
/// no threading. `WordCountModel`'s debounce/background behavior is exercised
/// separately below with a short async wait, matching `DocumentWatcherTests`'
/// convention for timing-sensitive assertions.
final class DocumentStatisticsTests: XCTestCase {
    private func doc(_ body: String) -> String { "\\documentclass{article}\n\\begin{document}\n\(body)\n\\end{document}\n" }

    func testPlainProseCountsWords() {
        let r = DocumentStatistics.analyze(doc("The quick brown fox jumps over the lazy dog."))
        XCTAssertEqual(r.counts.bodyWords, 9)
        XCTAssertEqual(r.counts.totalWords, 9)
    }

    func testHyphenatedAndApostropheWordsCountAsOne() {
        let r = DocumentStatistics.analyze(doc("A well-known result: don't panic."))
        // "A", "well-known", "result", "don't", "panic" = 5
        XCTAssertEqual(r.counts.bodyWords, 5)
    }

    func testPreambleIsSkipped() {
        let text = "\\documentclass{article}\n\\title{Ignored Preamble Words Here}\n\\begin{document}\nOne two three.\n\\end{document}\n"
        let r = DocumentStatistics.analyze(text)
        XCTAssertEqual(r.counts.bodyWords, 3)
    }

    func testDocumentFragmentWithNoPreambleCountsEverything() {
        // No \begin{document} at all: the whole string is body text.
        let r = DocumentStatistics.analyze("One two three four.")
        XCTAssertEqual(r.counts.bodyWords, 4)
    }

    func testCommentsAreStripped() {
        let r = DocumentStatistics.analyze(doc("Visible words. % these words do not count\nMore visible words."))
        XCTAssertEqual(r.counts.bodyWords, 5) // Visible words More visible words
    }

    func testEscapedPercentIsNotACommentStart() {
        // `\%` must not start a comment; only a bare `%` does, and only to end of line.
        let r = DocumentStatistics.analyze(doc("One hundred\\% is fine. % comment word\nBut visible."))
        XCTAssertEqual(r.counts.bodyWords, 6) // One hundred is fine But visible; "comment"/"word" excluded
    }

    func testSectioningCommandsCountAsHeaderWordsAndProduceABreakdownEntry() {
        let r = DocumentStatistics.analyze(doc("\\section{Introduction to Widgets}\nBody text here.\n\\subsection{Details}\nMore body text."))
        XCTAssertEqual(r.counts.headerWords, 4) // "Introduction to Widgets" (3) + "Details" (1)
        XCTAssertEqual(r.sections.count, 2)
        XCTAssertEqual(r.sections[0].title, "Introduction to Widgets")
        XCTAssertEqual(r.sections[0].level, 1)
        XCTAssertEqual(r.sections[1].title, "Details")
        XCTAssertEqual(r.sections[1].level, 2)
    }

    func testBodyWordsAfterASectionAreAttributedToThatSection() {
        let r = DocumentStatistics.analyze(doc("\\section{First}\nOne two three.\n\\section{Second}\nFour five."))
        XCTAssertEqual(r.sections[0].counts.bodyWords, 3)
        XCTAssertEqual(r.sections[1].counts.bodyWords, 2)
    }

    func testCaptionWordsAreCountedSeparatelyFromBody() {
        let r = DocumentStatistics.analyze(doc("\\begin{figure}\nBody words near the figure.\n\\caption{A caption with five words}\n\\end{figure}"))
        XCTAssertEqual(r.counts.captionWords, 5)
        XCTAssertEqual(r.counts.bodyWords, 5) // "Body words near the figure"
    }

    func testInlineMathIsCountedAsABlockAndExcludedFromWords() {
        let r = DocumentStatistics.analyze(doc("Let $a + b = c$ be an equation and \\(x^2\\) too."))
        XCTAssertEqual(r.counts.inlineMath, 2)
        XCTAssertEqual(r.counts.displayMath, 0)
        // "Let", "be", "an", "equation", "and", "too" = 6; math content excluded
        XCTAssertEqual(r.counts.bodyWords, 6)
    }

    func testDisplayMathDelimitersAndEnvironmentsAreCounted() {
        let r = DocumentStatistics.analyze(doc("Before.\n\\[a^2+b^2=c^2\\]\nMiddle.\n\\begin{equation}\nE=mc^2\n\\end{equation}\nAfter. $$x=y$$ Done."))
        XCTAssertEqual(r.counts.displayMath, 3)
        XCTAssertEqual(r.counts.inlineMath, 0)
        XCTAssertEqual(r.counts.bodyWords, 4) // Before Middle After Done
    }

    func testVerbatimContentIsNotCountedAndNotScannedForCommands() {
        let r = DocumentStatistics.analyze(doc("Real word.\n\\begin{verbatim}\n\\section{Not a section} fake $not math$\n\\end{verbatim}\nAnother real word."))
        XCTAssertEqual(r.counts.bodyWords, 5) // "Real word" (2) + "Another real word" (3)
        XCTAssertTrue(r.sections.isEmpty)
        XCTAssertEqual(r.counts.inlineMath, 0)
    }

    func testInlineVerbIsSkipped() {
        let r = DocumentStatistics.analyze(doc("Type \\verb|\\section{x}| literally then continue."))
        XCTAssertTrue(r.sections.isEmpty)
        XCTAssertEqual(r.counts.bodyWords, 4) // Type literally then continue
    }

    func testLabelRefCiteAndIncludeArgumentsAreSkippedEntirely() {
        let r = DocumentStatistics.analyze(doc(
            "See \\ref{eq:one} and \\cite{knuth1986} for details.\n" +
            "\\label{sec:intro}\n" +
            "\\input{chapter-two}\n" +
            "\\includegraphics[width=3cm]{diagram.png}\n" +
            "Final word."
        ))
        // See and for details Final word = 6; none of eq:one/knuth1986/sec:intro/chapter-two/diagram.png/width=3cm counted
        XCTAssertEqual(r.counts.bodyWords, 6)
    }

    func testFormattingCommandsAreTransparentAndTheirArgumentTextCounts() {
        let r = DocumentStatistics.analyze(doc("This is \\textbf{very important} and \\emph{emphasized} text."))
        XCTAssertEqual(r.counts.bodyWords, 7) // This is very important and emphasized text
    }

    func testHrefSkipsTheURLButCountsTheDisplayText() {
        let r = DocumentStatistics.analyze(doc("Visit \\href{https://example.com/path}{our website} today."))
        XCTAssertEqual(r.counts.bodyWords, 4) // Visit our website today
    }

    func testSelectionWordCount() {
        let text = "One two three four five."
        let ns = text as NSString
        let range = ns.range(of: "two three four")
        XCTAssertEqual(DocumentStatistics.wordCount(inSelection: text, utf16Range: range), 3)
    }

    func testSelectionWordCountEmptyRangeIsZero() {
        XCTAssertEqual(DocumentStatistics.wordCount(inSelection: "One two", utf16Range: NSRange(location: 0, length: 0)), 0)
    }

    func testAggregateAcrossMultipleDocumentsSumsAndTagsSections() {
        let a = doc("\\section{A}\nOne two.")
        let b = doc("\\section{B}\nThree four five.")
        let agg = DocumentStatistics.analyze(documents: [(path: "main.tex", text: a), (path: "chapter2.tex", text: b)])
        XCTAssertEqual(agg.total.bodyWords, 5)
        XCTAssertEqual(agg.documentCount, 2)
        XCTAssertEqual(agg.sections.count, 2)
        XCTAssertEqual(agg.sections[0].documentPath, "main.tex")
        XCTAssertEqual(agg.sections[1].documentPath, "chapter2.tex")
    }

    func testAggregateSingleDocumentLeavesDocumentPathNil() {
        let agg = DocumentStatistics.analyze(documents: [(path: "main.tex", text: doc("\\section{A}\nOne two."))])
        XCTAssertNil(agg.sections[0].documentPath)
    }

    func testOversizedDocumentIsMarkedTruncatedNotScanned() {
        let huge = String(repeating: "word ", count: (DocumentStatistics.maxScannedBytes / 5) + 100)
        let r = DocumentStatistics.analyze(huge)
        XCTAssertTrue(r.truncated)
        XCTAssertEqual(r.counts.totalWords, 0)
    }

    // MARK: real-world fixture (evidence for the GH68 gate: does not regress on HW1-sized docs)

    /// Perf evidence for the GH68 gate ("must not regress typing latency"):
    /// best-of-5 scan time for HW1 and a synthesized ~500 KB document, printed
    /// unconditionally; the bound (well under the 300 ms debounce interval,
    /// which itself runs off the main thread) is only enforced under a quiet
    /// 1-minute load average, matching `CompletionLatencyTests`' convention.
    func testScanTimeOnHW1AndA500KBDocument() throws {
        guard let repo = ProcessInfo.processInfo.environment["FLASHTEX_REPO"] else {
            throw XCTSkip("FLASHTEX_REPO not set; run via `FLASHTEX_REPO=$(git rev-parse --show-toplevel) swift test`")
        }
        var load = [0.0, 0.0, 0.0]
        getloadavg(&load, 3)
        let quiet = load[0] < 20

        func bestOf(_ n: Int, _ body: () -> Void) -> Double {
            var best = Double.greatestFiniteMagnitude
            for _ in 0..<n {
                let start = DispatchTime.now()
                body()
                let ms = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
                best = min(best, ms)
            }
            return best
        }

        let hw1URL = URL(fileURLWithPath: repo).appendingPathComponent("fixtures/real-world/hw1/HW1.tex")
        let hw1 = try String(contentsOf: hw1URL, encoding: .utf8)
        let hw1Ms = bestOf(5) { _ = DocumentStatistics.analyze(hw1) }

        // Repeat HW1's own content (real LaTeX shape: prose, math, sections,
        // comments, macros) out to ~500 KB rather than synthetic filler.
        var big = hw1
        while big.utf8.count < 500_000 { big += hw1 }
        let bigMs = bestOf(5) { _ = DocumentStatistics.analyze(big) }

        print("[DocumentStatistics perf] HW1 (\(hw1.utf8.count) bytes): \(String(format: "%.2f", hw1Ms)) ms; "
              + "500KB doc (\(big.utf8.count) bytes): \(String(format: "%.2f", bigMs)) ms; load1=\(load[0]) quiet=\(quiet)")

        guard quiet else { throw XCTSkip("1-min load \(load[0]) >= 20; timing not enforced, see printed numbers") }
        XCTAssertLessThan(hw1Ms, 20, "HW1-sized scan should be well under the 300 ms debounce interval")
        XCTAssertLessThan(bigMs, 150, "a 500 KB scan should still leave headroom under the 300 ms debounce interval")
    }

    func testHW1FixtureCountsAreSane() throws {
        guard let repo = ProcessInfo.processInfo.environment["FLASHTEX_REPO"] else {
            throw XCTSkip("FLASHTEX_REPO not set; run via `FLASHTEX_REPO=$(git rev-parse --show-toplevel) swift test`")
        }
        let url = URL(fileURLWithPath: repo).appendingPathComponent("fixtures/real-world/hw1/HW1.tex")
        let text = try String(contentsOf: url, encoding: .utf8)
        let r = DocumentStatistics.analyze(text)
        XCTAssertFalse(r.truncated)
        XCTAssertGreaterThan(r.counts.totalWords, 100)
        XCTAssertGreaterThan(r.counts.displayMath + r.counts.inlineMath, 0)
        XCTAssertFalse(r.sections.isEmpty) // \subsection* from the \problem macro
    }
}

/// `WordCountModel`'s debounce/background contract: a scheduled update is
/// published on the main actor, and only the most recent schedule's result
/// survives a burst — the same shape `DocumentWatcherTests` uses for its own
/// coalesced-delivery assertions. These tests inject an immediate scheduler
/// AND a synchronous background executor, so the whole
/// schedule→scan→publish pipeline is deterministic with no real wall-clock
/// wait at all; production still debounces on the main queue and still scans
/// on a background queue.
@MainActor
final class WordCountModelTests: XCTestCase {
    /// Synchronous pipeline: the debounce fires immediately and the
    /// background scan runs inline, so the only remaining hop is the
    /// `Task { @MainActor }` publish inside `recompute`.
    private func makeModel() -> WordCountModel {
        let model = WordCountModel()
        model.scheduleDebounce = { _, item in item.perform() }
        model.recomputeExecutor = { work in work() }
        return model
    }

    /// Deterministically waits on the publication itself, via
    /// `onRecomputeSettled` — not on unstructured `Task { @MainActor }`
    /// jobs running in enqueue order, which Swift concurrency does not
    /// document as guaranteed. `settles` counts every `recompute` cycle
    /// that has reached its publish decision (published or superseded), not
    /// just successful publishes, so a caller that triggered N updates
    /// should await exactly N settles even when only the last one publishes.
    /// Must be called after the trigger(s) it is waiting on and before any
    /// other `await` in the same test, so the hook is armed before the
    /// already-enqueued Task(s) get a chance to run.
    private func published(_ model: WordCountModel, settles: Int = 1) async {
        var remaining = settles
        await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
            model.onRecomputeSettled = {
                remaining -= 1
                if remaining <= 0 {
                    model.onRecomputeSettled = nil
                    continuation.resume()
                }
            }
        }
    }

    func testScheduleUpdateEventuallyPublishesTotals() async {
        let model = makeModel()
        model.scheduleUpdate(documents: [.init(path: "main.tex", text: "One two three.")])
        await published(model)
        XCTAssertEqual(model.total?.totalWords, 3)
    }

    func testOnlyTheLastScheduledUpdateWins() async {
        let model = makeModel()
        // Both updates run their scans inline before either publish lands,
        // so only the `generation` guard — the real coalescing mechanism —
        // decides the winner.
        model.scheduleUpdate(documents: [.init(path: "main.tex", text: "One.")])
        model.scheduleUpdate(documents: [.init(path: "main.tex", text: "One two three four.")])
        await published(model, settles: 2)
        XCTAssertEqual(model.total?.totalWords, 4)
    }

    func testCancelledScheduleNeverPublishesItsResult() async {
        let model = WordCountModel()
        // Capturing scheduler: neither item runs until we say so, so the
        // second `scheduleUpdate` must cancel the still-pending first item —
        // the real production coalescing path (`debounce?.cancel()`).
        var captured: [DispatchWorkItem] = []
        model.scheduleDebounce = { _, item in captured.append(item) }
        model.scheduleUpdate(documents: [.init(path: "main.tex", text: "One.")])
        model.scheduleUpdate(documents: [.init(path: "main.tex", text: "One two three four.")])
        XCTAssertEqual(captured.count, 2)
        XCTAssertTrue(captured[0].isCancelled)
        // Even if a queue naively invoked the cancelled item anyway (real
        // `DispatchQueue.asyncAfter` would not), the `generation` guard is a
        // second line of defense: only the second update may publish.
        captured[0].perform()
        captured[1].perform()
        let ok = await settles { model.total?.totalWords == 4 }
        XCTAssertTrue(ok)
        XCTAssertEqual(model.total?.totalWords, 4)
    }
}
