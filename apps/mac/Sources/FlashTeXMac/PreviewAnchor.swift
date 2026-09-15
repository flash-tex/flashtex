import AppKit
import QuartzCore
import SwiftUI

// Preview scroll anchoring (gap 5). The v1 and v2 preview panes are plain
// SwiftUI `ScrollView`s whose content is a column of fit-to-width pages. A
// SwiftUI scroll view keeps its content OFFSET in points across a content
// change, so when the page count changes ahead of the visible page, or the
// pane is resized (which changes the fit-to-width scale), the reader lands on
// different content. `PreviewAnchor` is the pure model: the visible rect is
// reduced to (page, vertical fraction of that page, horizontal fraction) and
// re-expanded under the new layout. `PreviewAnchorKeeper` is the AppKit probe
// that observes the enclosing `NSScrollView`, keeps the latest anchor while the
// layout is stable, and re-scrolls to it once the layout changed. A result that
// does not change the layout (a stale candidate the model drops, or an
// identical page set) never triggers a scroll.

/// Geometry of the page column in content (document) coordinates, y down:
/// `padding` around a vertical stack of pages `spacing` apart, each page
/// drawn at `widthPt*scale × heightPt*scale` and centered on the widest page
/// (the `VStack`/`LazyVStack` default `.center` alignment).
struct PreviewPageLayout: Equatable {
    struct Page: Equatable {
        let number: Int
        let widthPt: CGFloat
        let heightPt: CGFloat
    }

    var pages: [Page]
    var scale: CGFloat
    var spacing: CGFloat = 24
    var padding: CGFloat = 24

    /// The panes' fit-to-width rule: the widest page fills the pane minus the
    /// 24pt padding on each side, never upscaled past 100%, never below 20%.
    static func fitScale(paneWidth: CGFloat, widestPt: CGFloat) -> CGFloat {
        min(1, max(0.2, (paneWidth - 48) / max(widestPt, 1)))
    }

    var widestPt: CGFloat { pages.map(\.widthPt).max() ?? 0 }

    var contentSize: CGSize {
        guard !pages.isEmpty else { return CGSize(width: 2 * padding, height: 2 * padding) }
        let height = pages.reduce(CGFloat(0)) { $0 + $1.heightPt * scale } + spacing * CGFloat(pages.count - 1)
        return CGSize(width: widestPt * scale + 2 * padding, height: height + 2 * padding)
    }

    /// Page frames in order, keyed by page number.
    var frames: [(number: Int, frame: CGRect)] {
        var y = padding
        let widest = widestPt * scale
        return pages.map { page in
            let size = CGSize(width: page.widthPt * scale, height: page.heightPt * scale)
            let frame = CGRect(x: padding + (widest - size.width) / 2, y: y, width: size.width, height: size.height)
            y += size.height + spacing
            return (page.number, frame)
        }
    }

    func frame(of number: Int) -> CGRect? { frames.first { $0.number == number }?.frame }
}

/// A position in the page column that survives a layout change: the page at
/// the visible rect's top edge, how far down that page the edge sits (as a
/// fraction of the page height, negative inside the gap above it) and the
/// visible rect's left edge as a fraction of the page width.
struct PreviewAnchor: Equatable, CustomStringConvertible {
    var page: Int
    var fraction: CGFloat
    var horizontal: CGFloat

    var description: String { String(format: "page %d @ %.4f (x %.4f)", page, fraction, horizontal) }

    /// The anchor of `visible` (document coordinates, y down) in `layout`; nil
    /// for an empty layout. The top edge selects the first page whose bottom
    /// is below it, so a top edge in the gap between pages anchors to the page
    /// below with a small negative fraction; below the last page it anchors to
    /// the last page with a fraction above 1.
    static func capture(visible: CGRect, layout: PreviewPageLayout) -> PreviewAnchor? {
        let frames = layout.frames
        guard let last = frames.last else { return nil }
        let hit = frames.first { $0.frame.maxY > visible.minY } ?? last
        let f = hit.frame
        return PreviewAnchor(page: hit.number,
                             fraction: f.height > 0 ? (visible.minY - f.minY) / f.height : 0,
                             horizontal: f.width > 0 ? (visible.minX - f.minX) / f.width : 0)
    }

    /// Restored content origin under `layout` for a viewport of `visibleSize`,
    /// clamped to the scrollable range. A page number past the new page count
    /// clamps to the last page (the anchored page vanished); an anchored page
    /// that no longer exists in a shorter document therefore keeps the reader
    /// at the end rather than jumping to the top. Nil for an empty layout.
    /// `contentSize` overrides the layout's computed size with the measured
    /// document size when the caller has one.
    func restore(in layout: PreviewPageLayout, visibleSize: CGSize, contentSize: CGSize? = nil) -> CGPoint? {
        let frames = layout.frames
        guard let last = frames.last else { return nil }
        let target = frames.first { $0.number == page } ?? (page > last.number ? last : frames[0])
        let f = target.frame
        let content = contentSize ?? layout.contentSize
        let y = f.minY + fraction * f.height
        let x = f.minX + horizontal * f.width
        return CGPoint(x: Self.clamp(x, 0, max(0, content.width - visibleSize.width)),
                       y: Self.clamp(y, 0, max(0, content.height - visibleSize.height)))
    }

    private static func clamp(_ v: CGFloat, _ lo: CGFloat, _ hi: CGFloat) -> CGFloat { min(max(v, lo), hi) }
}

/// One anchoring correction, for evidence: the offset the scroll view had
/// after the layout change and the offset restored from the anchor.
struct PreviewAnchorCorrection: Equatable {
    let anchor: PreviewAnchor
    let before: CGPoint
    let after: CGPoint
    var deltaY: CGFloat { after.y - before.y }
}

/// Placed as the `.background` of the page column inside the pane's
/// `ScrollView`: finds the backing `NSScrollView`, captures the anchor on every
/// user scroll while `layout` is unchanged, and re-scrolls to the anchor when
/// `layout` (page set or scale) changes. A no-op when no scroll view encloses it.
///
/// It is also the pane's only handle on the real scroll geometry, so caret
/// following (`CaretFollow.swift`) rides along: `follow` is the latest request
/// from the `CaretFollowController`, acted on once per token, and
/// `onUserScroll` reports a live scroll (wheel, trackpad, scroller drag) so the
/// controller can stop following until the reader edits again.
struct PreviewAnchorKeeper: NSViewRepresentable {
    let layout: PreviewPageLayout
    var follow: CaretFollowController.Request? = nil
    var reveal: CaretFollowController.Request? = nil
    var onUserScroll: (() -> Void)? = nil
    /// Reports the page under the viewport's top edge (the header's "N / M").
    var onVisiblePage: ((Int) -> Void)? = nil

    func makeNSView(context: Context) -> PreviewAnchorProbe { PreviewAnchorProbe() }
    func updateNSView(_ view: PreviewAnchorProbe, context: Context) {
        view.onUserScroll = onUserScroll
        view.onVisiblePage = onVisiblePage
        view.layoutDidChange(to: layout)
        view.follow(follow)
        view.reveal(reveal)
    }
}

/// The AppKit side of `PreviewAnchorKeeper`. Test-visible: `anchor`, `layout`,
/// `corrections` and `documentVisibleRectTopDown` describe what it observed and did.
final class PreviewAnchorProbe: NSView {
    private(set) var layout: PreviewPageLayout?
    private(set) var anchor: PreviewAnchor?
    /// Anchor captured under the previous layout, held while SwiftUI settles
    /// the new one. Captures are suspended while it is set; every clip bounds
    /// or document frame change inside the settle window re-applies it, because
    /// SwiftUI re-imposes its own stored offset during the layout that follows
    /// a content-size change (measured: one revert after the first correction).
    private var pending: PreviewAnchor?
    private var settleGeneration = 0
    /// How long after a layout change the anchor is enforced.
    static let settleWindow: TimeInterval = 0.15
    /// Reduce-motion source, read at each layout change (`ReduceMotion.swift`);
    /// replaceable so tests drive both paths without touching the system setting.
    var reduceMotion: () -> Bool = { ReduceMotion.isEnabled }
    /// Under reduce motion, the end-of-window step does not scroll; when the
    /// content had drifted from the anchor by then, the drift is counted here
    /// (evidence) and the anchor is re-captured where the content is.
    private(set) var driftsLeftUncorrected = 0
    private(set) var corrections: [PreviewAnchorCorrection] = []
    /// Caret following (`CaretFollow.swift`): reported live scrolls, the token
    /// of the last request acted on, and every decision made (evidence/tests).
    var onUserScroll: (() -> Void)?
    /// Header page indicator: called only when the anchored page changes.
    var onVisiblePage: ((Int) -> Void)?
    private var reportedPage: Int?
    private(set) var followedToken: Int?
    private(set) var revealedToken: Int?
    private(set) var followDecisions: [(token: Int, decision: CaretFollow.Decision)] = []
    /// Event trace for the acceptance harness: (ms since first event, event, visible top, document height).
    private(set) var trace: [(ms: Double, event: String, top: CGFloat, docHeight: CGFloat)] = []
    private var traceStart = Date()
    func note(_ event: String) {
        let scroll = enclosingScrollView
        trace.append((Date().timeIntervalSince(traceStart) * 1000, event + String(format: " x=%.1f flipped=%d clipH=%.0f", documentVisibleRectTopDown?.minX ?? -1, scroll?.documentView?.isFlipped == true ? 1 : 0, scroll?.contentView.bounds.height ?? -1), documentVisibleRectTopDown?.minY ?? -1, scroll?.documentView?.bounds.height ?? -1))
        if trace.count > 400 { trace.removeFirst(200) }
    }
    private var observers: [NSObjectProtocol] = []
    private weak var observedScrollView: NSScrollView?

    deinit { observers.forEach(NotificationCenter.default.removeObserver) }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        attach()
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        attach()
    }

    private func attach() {
        guard let scroll = enclosingScrollView, scroll !== observedScrollView else { return }
        observers.forEach(NotificationCenter.default.removeObserver)
        observers.removeAll()
        observedScrollView = scroll
        scroll.contentView.postsBoundsChangedNotifications = true
        observers.append(NotificationCenter.default.addObserver(forName: NSView.boundsDidChangeNotification, object: scroll.contentView, queue: nil) { [weak self] _ in
            MainActor.assumeIsolated { self?.clipBoundsDidChange() }
        })
        observers.append(NotificationCenter.default.addObserver(forName: NSScrollView.willStartLiveScrollNotification, object: scroll, queue: nil) { [weak self] _ in
            // Wheel, trackpad or scroller drag: the reader took over.
            MainActor.assumeIsolated { self?.onUserScroll?() }
        })
        if let doc = scroll.documentView {
            doc.postsFrameChangedNotifications = true
            observers.append(NotificationCenter.default.addObserver(forName: NSView.frameDidChangeNotification, object: doc, queue: nil) { [weak self] _ in
                MainActor.assumeIsolated { self?.note("docFrame"); self?.applyPending() }
            })
        }
        capture()
    }

    /// The visible rect in document coordinates with y down, or nil when not
    /// inside a scroll view.
    var documentVisibleRectTopDown: CGRect? {
        guard let scroll = enclosingScrollView, let doc = scroll.documentView else { return nil }
        var rect = scroll.documentVisibleRect
        if !doc.isFlipped { rect.origin.y = doc.bounds.height - rect.maxY }
        return rect
    }

    private func capture() {
        guard pending == nil, let layout, let visible = documentVisibleRectTopDown else { return }
        anchor = PreviewAnchor.capture(visible: visible, layout: layout)
        if let page = anchor?.page, page != reportedPage {
            reportedPage = page
            onVisiblePage?(page)
        }
    }

    private func clipBoundsDidChange() {
        note(pending == nil ? "bounds" : "bounds(pending)")
        if pending != nil { applyPending() } else { capture() }
    }

    func layoutDidChange(to new: PreviewPageLayout) {
        guard let old = layout else {
            layout = new
            capture()
            return
        }
        guard old != new else { return }
        layout = new
        note("layout \(new.pages.count)p @\(String(format: "%.3f", new.scale))")
        if let anchor, pending == nil { pending = anchor }
        settleGeneration += 1
        let generation = settleGeneration
        let reduced = reduceMotion()
        // SwiftUI resizes the document view after this update pass and then
        // re-imposes its stored offset; the observers re-apply the anchor at
        // each step until the settle window closes.
        applyPending()
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.settleWindow) { [weak self] in
            guard let self, self.settleGeneration == generation else { return }
            if reduced {
                // Reduce motion: no scroll from a timer on a drawn frame (see
                // ReduceMotion.swift); the synchronous corrections above and in
                // the observers already ran inside the layout passes.
                if self.pendingDrift() { self.driftsLeftUncorrected += 1; self.note("drift left (reduce motion)") }
                self.note("settled (reduce motion)")
            } else {
                self.applyPending()
                self.note("settled")
            }
            self.pending = nil
            self.capture()
        }
    }

    // MARK: caret following

    /// Acts on `request` once (by token): asks `CaretFollow.decide` what the
    /// real scroll geometry says and scrolls only if it says to.
    func follow(_ request: CaretFollowController.Request?) {
        guard let request, request.token != followedToken else { return }
        followedToken = request.token
        guard let layout, let scroll = enclosingScrollView, let doc = scroll.documentView,
              let visible = documentVisibleRectTopDown else { return }
        let decision = CaretFollow.decide(target: request.target, layout: layout, visible: visible,
                                          contentSize: doc.bounds.size, reduceMotion: reduceMotion())
        followDecisions.append((request.token, decision))
        if followDecisions.count > 200 { followDecisions.removeFirst(100) }
        guard case .scroll(let point, let animated) = decision else { return }
        // The follow is the newer intent: an anchor correction still pending
        // from a layout change must not drag the content back.
        pending = nil
        settleGeneration += 1
        scrollTopDown(to: point, animated: animated)
        note(String(format: "followed r%d %.1f→%.1f%@", request.token, visible.minY, point.y, animated ? " (animated)" : ""))
        capture()
    }

    /// Internal-destination reveal: same scroll policy as caret following,
    /// with its own token so a click is never dropped as a stale follow.
    func reveal(_ request: CaretFollowController.Request?) {
        guard let request, request.token != revealedToken else { return }
        revealedToken = request.token
        guard let layout, let scroll = enclosingScrollView, let doc = scroll.documentView,
              let visible = documentVisibleRectTopDown else { return }
        let decision = CaretFollow.decide(target: request.target, layout: layout, visible: visible,
                                          contentSize: doc.bounds.size, reduceMotion: reduceMotion())
        guard case .scroll(let point, let animated) = decision else { return }
        pending = nil
        settleGeneration += 1
        scrollTopDown(to: point, animated: animated)
        note(String(format: "revealed r%d %.1f→%.1f%@", request.token, visible.minY, point.y, animated ? " (animated)" : ""))
        capture()
    }

    /// Scrolls the clip view to `point` (document coordinates, y down),
    /// flipping y for an unflipped document view, exactly like `applyPending`.
    private func scrollTopDown(to point: CGPoint, animated: Bool) {
        guard let scroll = enclosingScrollView, let doc = scroll.documentView else { return }
        let clip = scroll.contentView
        var origin = point
        if !doc.isFlipped { origin.y = doc.bounds.height - point.y - clip.bounds.height }
        if doc.bounds.width <= clip.bounds.width + 0.5 { origin.x = clip.bounds.origin.x }
        if animated {
            NSAnimationContext.runAnimationGroup { context in
                context.duration = 0.25
                context.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
                context.allowsImplicitAnimation = true
                clip.animator().setBoundsOrigin(origin)
            } completionHandler: { [weak scroll, weak clip] in
                guard let scroll, let clip else { return }
                scroll.reflectScrolledClipView(clip)
            }
        } else {
            clip.scroll(to: origin)
            scroll.reflectScrolledClipView(clip)
        }
    }

    /// True when the visible top differs from the pending anchor's restore
    /// target by more than the correction threshold (0.5 pt).
    private func pendingDrift() -> Bool {
        guard let pending, let layout, let scroll = enclosingScrollView, let doc = scroll.documentView,
              let target = pending.restore(in: layout, visibleSize: scroll.contentView.bounds.size, contentSize: doc.bounds.size),
              let before = documentVisibleRectTopDown?.origin else { return false }
        return abs(before.y - target.y) > 0.5
    }

    private func applyPending() {
        guard let pending, let layout, let scroll = enclosingScrollView, let doc = scroll.documentView else { return }
        let clip = scroll.contentView
        guard let target = pending.restore(in: layout, visibleSize: clip.bounds.size, contentSize: doc.bounds.size) else { return }
        let before = documentVisibleRectTopDown?.origin ?? .zero
        var origin = target
        if !doc.isFlipped { origin.y = doc.bounds.height - target.y - clip.bounds.height }
        // Horizontal only matters when the document is wider than the clip;
        // otherwise AppKit centers/pins x itself and it is left alone.
        let horizontal = doc.bounds.width > clip.bounds.width + 0.5
        if !horizontal { origin.x = clip.bounds.origin.x }
        if abs(before.y - target.y) > 0.5 || (horizontal && abs(before.x - target.x) > 0.5) {
            clip.scroll(to: origin)
            scroll.reflectScrolledClipView(clip)
            corrections.append(PreviewAnchorCorrection(anchor: pending, before: before, after: target))
            note(String(format: "corrected %.1f→%.1f", before.y, target.y))
        }
    }
}
