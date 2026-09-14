import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// End-to-end completion latency through the real `CompletingTextView` in a
/// hosted (never key) window, best-of-N: keystroke → list shown (pickup),
/// keystroke → list narrowed, arrow → selection moved in the popup, Return →
/// text inserted and list closed. Each stage is timed from the `keyDown` call
/// to the moment the observable state holds while the main run loop turns
/// (the scan runs off-main and is delivered by a run-loop block). Functional
/// checks always run; the wall-clock bounds are enforced only when the
/// 1-minute load average is below 20 (shared machine), and the measured
/// numbers are printed either way with the load they were taken under.
///
/// Stale-context refusal after a document switch is covered here too: the
/// editor swaps `string` when the active document changes, so an outcome
/// computed for the previous buffer/caret must neither open nor mutate the
/// list, and a helper reply bound to a revision that File > Open replaced
/// must never bind.
@MainActor
final class CompletionLatencyTests: XCTestCase {
    private static let demo: String = {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Samples/demo.tex")
        return try! String(contentsOf: url, encoding: .utf8)
    }()

    static var loadAverage1: Double {
        var load = [0.0, 0.0, 0.0]
        getloadavg(&load, 3)
        return load[0]
    }

    private var window: NSWindow!
    private var tv: CompletingTextView!

    override func setUp() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        tv.allowsUndo = true
        window.orderFrontRegardless() // never makeKey: the test must not steal focus
        window.makeFirstResponder(tv)
    }

    override func tearDown() async throws {
        tv.close(.escape)
        window.orderOut(nil)
        window = nil
        tv = nil
    }

    private func key(_ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    /// Turns the main run loop in short slices until `cond` holds; returns the
    /// wall time from `t0` (a `MonotonicClock` stamp) to the first observation.
    private func msUntil(_ what: String, from t0: UInt64, timeout: TimeInterval = 5, _ cond: () -> Bool) -> Double {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() >= deadline { XCTFail("timed out waiting for \(what)"); break }
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.0005))
        }
        return Double(MonotonicClock.nowNs() - t0) / 1e6
    }

    private func ms(since t0: UInt64) -> Double { Double(MonotonicClock.nowNs() - t0) / 1e6 }

    private struct Stage {
        var samples: [Double] = []
        var best: Double { samples.min() ?? .nan }
        var median: Double {
            let s = samples.sorted()
            return s.isEmpty ? .nan : s[s.count / 2]
        }
        var worst: Double { samples.max() ?? .nan }
        var summary: String { String(format: "best %.2f / median %.2f / max %.2f ms", best, median, worst) }
    }

    /// Places `\s` before `\end{document}` of demo.tex and the caret after it.
    private func resetProbe() -> Int {
        let demo = Self.demo as NSString
        let endDoc = demo.range(of: "\\end{document}").location
        tv.string = demo.replacingCharacters(in: NSRange(location: endDoc, length: 0), with: "\\s\n")
        let caret = endDoc + 2
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        // Settle the editor's own layout/display of the new text so the pickup
        // stage times the completion path, not the document swap.
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        window.displayIfNeeded()
        RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.02))
        return caret
    }

    func testPickupNarrowArrowAndReturnLatencyBestOfN() throws {
        let load = Self.loadAverage1
        let iterations = 15
        var pickup = Stage(), narrow = Stage(), arrow = Stage(), accept = Stage(), compute = Stage()
        var queued = Stage(), lag = Stage(), present = Stage(), keystroke = Stage(), afterPresent = Stage()
        for i in 0..<iterations {
            let caret = resetProbe()
            XCTAssertNil(tv.session)
            // Pickup: ⌃Space → list shown (scan off-main, delivery, popup on screen).
            var t0 = MonotonicClock.nowNs()
            key(" ", code: 49, flags: .control)
            keystroke.samples.append(ms(since: t0))
            XCTAssertNil(tv.session, "the keystroke path enqueues; it does not scan")
            pickup.samples.append(msUntil("popup \(i)", from: t0) { tv.session != nil && tv.completionPopup.isVisible })
            afterPresent.samples.append(pickup.samples.last! - keystroke.samples.last! - (tv.lastOutcome?.queuedMs ?? 0)
                                        - (tv.lastOutcome?.computeMs ?? 0) - tv.lastDeliveryLagMs - tv.lastPresentMs)
            XCTAssertEqual(tv.session?.items.first?.label, CompletionTestVocabulary.labels(forPrefix: "s").first)
            XCTAssertEqual(tv.session?.range, NSRange(location: caret - 2, length: 2))
            XCTAssertEqual(tv.completionPopup.selectedRow, 0)
            compute.samples.append(tv.lastOutcome?.computeMs ?? .nan)
            queued.samples.append(tv.lastOutcome?.queuedMs ?? .nan)
            lag.samples.append(tv.lastDeliveryLagMs)
            present.samples.append(tv.lastPresentMs)
            // Narrow: "u" → list updated for `\su` with the popup rows replaced.
            t0 = MonotonicClock.nowNs()
            key("u", code: 32)
            // The vocabulary generated from the compiler's inventory: text entries,
            // then operators, then symbols. Computed from the pure function (not a
            // hand-copied snapshot) so this test tracks the compiler's inventory.
            let narrowed = Completion.suggestions(in: "x \\su", caretUTF16: 5, result: nil).map(\.label)
            narrow.samples.append(msUntil("narrowed \(i)", from: t0) { tv.session?.range.length == 3 && tv.completionPopup.items.count == narrowed.count })
            XCTAssertEqual(tv.session?.items.map(\.label), narrowed)
            XCTAssertTrue(tv.completionPopup.isVisible)
            // Arrow: ↓ → selection moved in the session and in the table (synchronous),
            // walking down to `\sup`, the first plain (no-argument) candidate.
            let supIndex = try XCTUnwrap(narrowed.firstIndex(of: "\\sup"))
            for _ in 0..<max(0, supIndex - 1) { key("\u{F701}", code: 125) }
            t0 = MonotonicClock.nowNs()
            key("\u{F701}", code: 125)
            arrow.samples.append(ms(since: t0))
            XCTAssertEqual(tv.session?.selectedIndex, supIndex)
            XCTAssertEqual(tv.completionPopup.selectedRow, supIndex)
            XCTAssertEqual(tv.selectedRange(), NSRange(location: caret + 1, length: 0), "choosing never moves the caret")
            // Accept: Return → text replaced, caret placed, list closed and hidden (synchronous).
            t0 = MonotonicClock.nowNs()
            key("\r", code: 36)
            accept.samples.append(ms(since: t0))
            XCTAssertTrue(tv.string.contains("\\sup\n\\end{document}"), "iteration \(i)")
            XCTAssertNil(tv.session)
            XCTAssertFalse(tv.completionPopup.isVisible)
            XCTAssertEqual(tv.lastCloseReason, .accepted)
            XCTAssertEqual(tv.selectedRange(), NSRange(location: caret + 2, length: 0))
        }
        XCTAssertEqual(tv.scheduler.statistics.refusedStale, 0, "every delivered outcome was current")
        XCTAssertEqual(tv.scheduler.statistics.delivered, iterations * 2)
        print("completion latency on demo.tex (\(Self.demo.utf8.count) B, best of \(iterations), 1-min load \(String(format: "%.1f", load))): "
              + "pickup ⌃Space→list \(pickup.summary); narrow key→list \(narrow.summary); "
              + "arrow ↓→selection \(arrow.summary); Return→inserted \(accept.summary); "
              + "pickup breakdown: keystroke on main \(keystroke.summary); queue wait \(queued.summary); off-main scan \(compute.summary); "
              + "delivery lag \(lag.summary); present (session + popup) \(present.summary); rest of the run-loop turn (panel display) \(afterPresent.summary)")
        if load >= 20 { throw XCTSkip("latency bounds not enforced: 1-minute load average \(String(format: "%.1f", load)) >= 20 (numbers above are under load)") }
        XCTAssertLessThan(pickup.best, 25, "keystroke → list shown")
        XCTAssertLessThan(narrow.best, 25, "keystroke → list narrowed")
        XCTAssertLessThan(arrow.best, 5, "arrow → selection moved")
        XCTAssertLessThan(accept.best, 15, "Return → inserted and closed")
    }

    /// Moving the selection through a full 12-row list reuses the rows: the
    /// same-items path of `CompletionPopup.update` only moves the selection,
    /// and is compared in the same run against a forced reload of the same
    /// rows (what every arrow key paid before). Both are printed; the fast
    /// path must not be slower than the reload.
    func testArrowSelectionThroughTwelveRowsAvoidsTheTableReload() throws {
        let load = Self.loadAverage1
        let demo = Self.demo as NSString
        let endDoc = demo.range(of: "\\end{document}").location
        tv.string = demo.replacingCharacters(in: NSRange(location: endDoc, length: 0), with: "\\\n")
        tv.setSelectedRange(NSRange(location: endDoc + 1, length: 0))
        key(" ", code: 49, flags: .control)
        _ = msUntil("full list", from: MonotonicClock.nowNs()) { tv.session != nil }
        let items = try XCTUnwrap(tv.session?.items)
        XCTAssertEqual(items.count, Completion.maxSuggestions)
        let popup = tv.completionPopup
        var fast: [Double] = [], reload: [Double] = [], arrows: [Double] = []
        for round in 0..<10 {
            for k in 0..<items.count {
                // Arrow key through the real view (wraps at the end).
                var t0 = MonotonicClock.nowNs()
                key("\u{F701}", code: 125)
                arrows.append(ms(since: t0))
                XCTAssertEqual(tv.session?.selectedIndex, (k + 1) % items.count, "round \(round)")
                XCTAssertEqual(popup.selectedRow, (k + 1) % items.count)
                // Same rows, selection only.
                t0 = MonotonicClock.nowNs()
                popup.update(items: items, selected: k)
                fast.append(ms(since: t0))
                XCTAssertEqual(popup.selectedRow, k)
                // Forced reload of the same rows (the previous behaviour).
                popup.update(items: [], selected: 0)
                t0 = MonotonicClock.nowNs()
                popup.update(items: items, selected: k)
                reload.append(ms(since: t0))
                XCTAssertEqual(popup.selectedRow, k)
                popup.update(items: items, selected: (k + 1) % items.count) // back in step with the session
            }
        }
        func s(_ v: [Double]) -> String { String(format: "best %.3f / median %.3f / max %.3f ms", v.min()!, v.sorted()[v.count / 2], v.max()!) }
        print("completion arrow through 12 rows (\(arrows.count) presses, 1-min load \(String(format: "%.1f", load))): ↓ keyDown \(s(arrows)); popup.update same rows \(s(fast)); popup.update forced reload \(s(reload))")
        key("\u{1B}", code: 53)
        XCTAssertNil(tv.session)
        if load >= 20 { throw XCTSkip("bound not enforced: 1-minute load average \(String(format: "%.1f", load)) >= 20 (numbers above are under load)") }
        XCTAssertLessThanOrEqual(fast.sorted()[fast.count / 2], reload.sorted()[reload.count / 2], "selection-only update is not slower than a reload")
        XCTAssertLessThan(arrows.min()!, 2, "arrow → selection moved")
    }

    /// Holds jobs until the test runs them (deterministic interleaving).
    private final class ManualExecutor {
        var jobs: [@Sendable () -> Void] = []
        var run: CompletionScheduler.Executor { { [self] job in jobs.append(job) } }
        func runAll() { let j = jobs; jobs = []; j.forEach { $0() } }
    }

    private func spin(_ what: String, timeout: TimeInterval = 5, until cond: () -> Bool) {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond(), Date() < deadline {
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
        }
        XCTAssertTrue(cond(), "timed out waiting for \(what)")
    }

    func testOutcomeForThePreviousDocumentNeverOpensOrMutatesTheList() throws {
        let exec = ManualExecutor()
        tv.scheduler = CompletionScheduler(executor: exec.run)
        // Document A, list open for `\s`.
        let a = "\\begin{document}\nx \\s"
        tv.string = a
        let caretA = (a as NSString).length
        tv.setSelectedRange(NSRange(location: caretA, length: 0))
        tv.requestCompletion()
        exec.runAll()
        spin("session A") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.first?.label, CompletionTestVocabulary.labels(forPrefix: "s").first)
        XCTAssertTrue(tv.completionPopup.isVisible)
        // Typing through the list queues a narrowing scan for A...
        key("u", code: 32)
        XCTAssertEqual(exec.jobs.count, 1, "narrowing scan pending for document A")
        XCTAssertNotNil(tv.session)
        // ...then the editor switches documents: the owner replaces `string`
        // (as SourceEditorView does) and restores the incoming caret, which
        // here happens to be the same offset as the pending request's.
        let b = "\\begin{itemize}\n\\it \\t"
        XCTAssertEqual((b as NSString).length, caretA + 1, "same caret offset by construction")
        tv.string = b
        tv.setSelectedRange(NSRange(location: caretA + 1, length: 0))
        XCTAssertNil(tv.session, "the switch closed the list")
        XCTAssertEqual(tv.lastCloseReason, .textChanged)
        XCTAssertFalse(tv.completionPopup.isVisible)
        let generationBefore = tv.scheduler.generation
        // The pending job for A now runs and delivers: refused, nothing opens.
        exec.runAll()
        spin("refusal") { tv.scheduler.statistics.refusedStale == 1 }
        XCTAssertNil(tv.session)
        XCTAssertFalse(tv.completionPopup.isVisible)
        XCTAssertEqual(tv.completionPopup.items, [])
        XCTAssertEqual(tv.scheduler.generation, generationBefore)
        XCTAssertEqual(tv.string, b, "nothing was inserted")
        // A fresh request on B lists B's own candidates only.
        tv.requestCompletion()
        exec.runAll()
        spin("session B") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.first?.label, "\\tableofcontents")
        XCTAssertTrue(tv.session!.items.allSatisfy { $0.label.hasPrefix("\\t") }, "\(tv.session!.items.map(\.label))")
        XCTAssertEqual(tv.session?.range, NSRange(location: caretA - 1, length: 2))

        // A session on B with an outcome pending; the caret moves to the same
        // offset in a different line after a switch back to a longer A':
        // the outcome's caret offset matches but its generation does not.
        key("h", code: 4)
        XCTAssertEqual(exec.jobs.count, 1)
        let a2 = "\\begin{document}\nx \\se\n\\th"
        tv.string = a2
        tv.setSelectedRange(NSRange(location: caretA + 2, length: 0)) // the offset the pending B request carries
        exec.runAll()
        spin("refusal 2") { tv.scheduler.statistics.refusedStale == 2 }
        XCTAssertNil(tv.session)
        XCTAssertFalse(tv.completionPopup.isVisible)
        XCTAssertEqual(tv.scheduler.statistics, .init(scheduled: 4, delivered: 2, refusedStale: 2, cancelled: 2))
    }

    /// The helper's `complete` reply for a query made before File > Open
    /// replaced the project (which moves the editor revision) decodes fine
    /// but binds to the old revision: the view refuses it, and a list built
    /// afterwards carries no index vocabulary.
    func testHelperReplyAfterProjectReplacementIsRefusedAtBind() throws {
        let fetcher = ProjectIndexCompletionFetcher()
        var sent: [(id: String, type: String, category: String)] = []
        var n = 0
        fetcher.request(sourceVersions: ["main.tex": 3], editorRevision: 5) { type, payload in
            n += 1
            let id = "pc-\(n)"
            sent.append((id, type, payload["category"] as? String ?? "?"))
            return id
        }
        XCTAssertEqual(sent.map(\.type), ["complete", "complete", "complete", "snapshot"])
        XCTAssertEqual(Set(sent.prefix(3).map(\.category)), ["label", "citation", "command"])
        // The editor opened another file: revision 6 (ShellModel.replaceProject).
        tv.string = "\\cite{"
        tv.setSelectedRange(NSRange(location: 6, length: 0))
        tv.editorRevision = 6
        // The three replies land now, for the old snapshot.
        func reply(_ names: [String]) -> [String: Any] {
            ["source_versions": ["main.tex": 3],
             "completions": names.map { ["name": $0, "definitions": [["path": "main.tex", "revision": 3, "start_byte": 0, "end_byte": 3]], "occurrences": [], "locations_truncated": false] }]
        }
        var outcomes: [ProjectIndexCompletionFetcher.Outcome] = []
        for s in sent {
            let payload: [String: Any] = s.type == "snapshot"
                ? ["project_id": "p", "source_versions": ["main.tex": 3], "membership_generation": 1, "document_kinds": ["main.tex": "latex"]]
                : reply(s.category == "citation" ? ["knuth84"] : ["x"])
            outcomes.append(fetcher.handle(resultID: s.id, payload: payload))
        }
        guard case .complete(let metadata) = outcomes.last else { return XCTFail("\(outcomes)") }
        XCTAssertEqual(metadata.revision, 5)
        XCTAssertEqual(metadata.citations.map(\.name), ["knuth84"])
        XCTAssertTrue(tv.accept(projectIndex: metadata), "held (nothing newer), but not bound")
        XCTAssertNil(tv.boundMetadata, "revision 5 metadata never binds at revision 6")
        let exec = ManualExecutor()
        tv.scheduler = CompletionScheduler(executor: exec.run)
        tv.requestCompletion()
        exec.runAll()
        RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05))
        XCTAssertNil(tv.session, "no candidates: the stale index citation was not shown")
        XCTAssertEqual(tv.lastOutcome?.items, [])
        // Metadata for the new revision binds and the key appears.
        let rebound = try Completion.Metadata.decodeProjectIndexReply(
            JSONSerialization.data(withJSONObject: reply(["knuth84"])), category: .citation, editorRevision: 6,
            expectedSourceVersions: ["main.tex": 3])
        XCTAssertTrue(tv.accept(projectIndex: rebound))
        XCTAssertEqual(tv.boundMetadata?.revision, 6)
        tv.requestCompletion()
        exec.runAll()
        spin("session") { tv.session != nil }
        XCTAssertEqual(tv.session?.items.map(\.label), ["knuth84"])
        XCTAssertEqual(tv.session?.metadataRevision, 6)
    }
}
