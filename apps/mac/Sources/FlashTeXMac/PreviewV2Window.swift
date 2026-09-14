import CoreGraphics
import Foundation
import FlashTeXProtocol

/// `display-list-v2-window` (protocol/proposals/display-list-v2-window.md), the
/// consumer half: which pages the shell asks the producer to actually build.
///
/// A 385-page document does not fit in one reply — today it comes back
/// `status: failed` with no preview at all — so the producer builds a bounded
/// window and elides the rest to their page frames. The window has to follow
/// the reader, and the whole difficulty is doing that *without thrashing*: the
/// pages the reader is looking at must be resident, but a two-line scroll must
/// not send a fresh whole-document layout to the worker.
///
/// The policy is deliberately pure and lives here rather than in the view, so
/// both halves of it are testable without a window server:
///
/// - **Where.** `desired` centres a window of `span` pages on the pages the
///   viewport actually intersects, clamped to the document. Centring rather
///   than starting at the first visible page, because reading goes both ways
///   and a reader who scrolls up one page should not immediately trigger a
///   re-request.
/// - **Whether.** `next` re-requests only when the viewport has come within
///   `margin` pages of an edge of what is resident (or has left it entirely).
///   While the visible pages sit comfortably inside the current window, the
///   answer is nil and nothing is sent. That is the hysteresis: the window is
///   wider than the viewport, and it only moves once the reader approaches the
///   part that is not loaded.
/// - **What the reply means.** The producer may narrow an over-limit window and
///   echoes what it actually served (§8); `next` is always given the *echoed*
///   window, never the requested one, so a narrowed reply does not loop.
enum PreviewV2Window {
    /// Pages requested at once. The producer narrows this to fit the reply
    /// limit if it must, and says so in the echo.
    static let defaultSpan = RenderingV2.maxWindowPages

    /// How close to the resident edge the viewport may come before the window
    /// moves. Two pages: one page of slack is consumed by a single scroll
    /// gesture, and three moves the window while the reader is still inside it.
    static let margin = 2

    /// The pages a viewport rect intersects, in the page column `layout`.
    /// Nil when the layout has no pages or the rect misses all of them.
    static func visiblePages(in visible: CGRect, layout: PreviewPageLayout) -> ClosedRange<Int>? {
        var lo: Int?, hi: Int?
        for (number, frame) in layout.frames where frame.intersects(visible) {
            lo = min(lo ?? number, number)
            hi = max(hi ?? number, number)
        }
        guard let lo, let hi else { return nil }
        return lo...hi
    }

    /// The window that should hold `visible`: `span` pages centred on it,
    /// clamped into `1...documentPageCount`. A viewport taller than `span`
    /// pages still gets a window starting at its first visible page.
    static func desired(around visible: ClosedRange<Int>, documentPageCount: Int,
                        span: Int = defaultSpan) -> RuntimeV1.CompileRequest.DisplayListWindow? {
        guard documentPageCount > 0, span > 0 else { return nil }
        let count = min(span, documentPageCount)
        let visibleCount = visible.upperBound - visible.lowerBound + 1
        let slack = max(0, count - visibleCount)
        var first = visible.lowerBound - slack / 2
        first = min(max(1, first), max(1, documentPageCount - count + 1))
        return .init(firstPage: first, pageCount: count)
    }

    /// The window to request now, or nil to leave the current one alone.
    ///
    /// `served` is the window the producer echoed (`display_list.window`), not
    /// what was asked for. `requested` is the last window this consumer sent,
    /// so a request already in flight for the same range is not sent twice.
    static func next(visible: ClosedRange<Int>,
                     documentPageCount: Int,
                     served: RenderingV2.PageWindow?,
                     requested: RuntimeV1.CompileRequest.DisplayListWindow?,
                     span: Int = defaultSpan,
                     margin: Int = margin) -> RuntimeV1.CompileRequest.DisplayListWindow? {
        // A document that fits in one window is never windowed at all: it gets
        // today's complete reply, so `-delta` keeps working on it (§7 makes the
        // two exclusive) and nothing about the common case changes. Only a
        // document with more pages than a window can hold is worth bounding.
        guard documentPageCount > span || served != nil else { return nil }
        guard let want = desired(around: visible, documentPageCount: documentPageCount, span: span) else { return nil }
        // …and once the producer has served the whole thing, stop asking.
        if let served, served.pageCount >= served.documentPageCount, served.documentPageCount <= span { return nil }
        // Nothing resident yet: ask.
        guard let served, served.pageCount > 0 else {
            return requested == want ? nil : want
        }
        // The whole document is resident: there is nothing to follow.
        if served.pageCount >= served.documentPageCount { return nil }
        let residentFirst = served.firstPage
        let residentLast = served.firstPage + served.pageCount - 1
        // Comfort band: the viewport is inside what is resident and not within
        // `margin` pages of an edge that has more document beyond it.
        let nearTop = visible.lowerBound - residentFirst < margin && residentFirst > 1
        let nearBottom = residentLast - visible.upperBound < margin && residentLast < served.documentPageCount
        let outside = visible.lowerBound < residentFirst || visible.upperBound > residentLast
        guard outside || nearTop || nearBottom else { return nil }
        // The producer narrowed the window to fewer pages than we asked for
        // (§8): asking for the same range again would loop. Only re-request
        // when the range we want actually differs from what is resident.
        if want.firstPage == residentFirst, want.pageCount == served.pageCount { return nil }
        if let requested, requested == want, want.firstPage == residentFirst { return nil }
        return want
    }

    /// The window to ask for after a reply the producer could not make at all.
    ///
    /// This is §1.0 itself, from the consumer's side: a 385-page document
    /// serialises to 152 MB against a 16 MiB reply limit, so the request comes
    /// back `status: failed` with no pages, no sibling and therefore **no page
    /// count** — the shell cannot learn how big the document is from a reply
    /// that does not exist. So the recovery does not try to: it asks for a
    /// window at wherever the reader is (page 1 until the pane has reported a
    /// viewport), and the echoed `window` then tells it everything it needs.
    ///
    /// Returns nil when a window was already asked for, so a document that
    /// fails for some other reason is retried once, not forever.
    static func afterFailure(visible: ClosedRange<Int>?,
                             alreadyRequested: RuntimeV1.CompileRequest.DisplayListWindow?,
                             span: Int = defaultSpan) -> RuntimeV1.CompileRequest.DisplayListWindow? {
        guard alreadyRequested == nil else { return nil }
        let first = visible.map { max(1, $0.lowerBound - span / 4) } ?? 1
        return .init(firstPage: first, pageCount: span)
    }
}
