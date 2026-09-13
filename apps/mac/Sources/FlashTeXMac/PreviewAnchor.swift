import AppKit
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
/// Also hosts "Preview follows the caret" (gap: follow-caret, FollowCaret.swift):
/// `followTarget` is the caret's page item in RAW page points (nil: no
/// mapping), resolved by `ShellModel.followCaretTargetV1()`/`followCaretTargetV2()`
/// and re-supplied on every SwiftUI body pass, i.e. only once the pane's
/// layout/frame reflects the caret's revision — the same reactive dependency
/// `caretHighlights` update from. The probe debounces, tracks manual scrolls,
/// and performs the scroll; see `PreviewAnchorProbe.followCaretDidChange`.
struct PreviewAnchorKeeper: NSViewRepresentable {
    let layout: PreviewPageLayout
    var followTarget: FollowCaret.Target? = nil
    var followEnabled: Bool = false

    func makeNSView(context: Context) -> PreviewAnchorProbe { PreviewAnchorProbe() }
    func updateNSView(_ view: PreviewAnchorProbe, context: Context) {
        view.layoutDidChange(to: layout)
        view.followCaretDidChange(followTarget, enabled: followEnabled)
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

    // MARK: follow-caret (FollowCaret.swift)

    /// The caret's target, in RAW page points, as last supplied by
    /// `PreviewAnchorKeeper`; nil is the "no mapping" case. Compared by
    /// identity (`!=`) to decide whether the caret actually moved, so a
    /// SwiftUI pass that resupplies the same target does not reset the timer.
    private var followTarget: FollowCaret.Target?
    private var followEnabled = false
    private var followDebounceItem: DispatchWorkItem?
    /// When the user last scrolled manually (live scroll: wheel/trackpad/scrollbar)
    /// and the follow target current at that moment (for the re-arm rule).
    private(set) var lastUserScrollAt: Date?
    private(set) var lastUserScrollTarget: FollowCaret.Target?
    private(set) var isLiveScrolling = false
    /// Test injection; real time otherwise.
    var now: () -> Date = Date.init
    /// Debounce/behavior constants (`var` so tests can shrink the wait).
    var followDebounceInterval: TimeInterval = 0.25
    var followMargin: CGFloat = 24
    var followYieldWindow: TimeInterval = 3
    /// Every decision `evaluateFollow()` reached, in order (evidence for tests).
    private(set) var followActions: [FollowCaret.Action] = []
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
        if let doc = scroll.documentView {
            doc.postsFrameChangedNotifications = true
            observers.append(NotificationCenter.default.addObserver(forName: NSView.frameDidChangeNotification, object: doc, queue: nil) { [weak self] _ in
                MainActor.assumeIsolated { self?.note("docFrame"); self?.applyPending() }
            })
        }
        // Follow-caret "yield to the user": live scroll notifications fire for
        // every user-initiated scroll (scroll wheel, trackpad, AND scrollbar
        // knob dragging), never for our own programmatic `setBoundsOrigin` —
        // so this is the whole "did the user just scroll, or is dragging"
        // signal `FollowCaret.decide` needs, with no flag to distinguish our
        // own scroll from theirs.
        observers.append(NotificationCenter.default.addObserver(forName: NSScrollView.willStartLiveScrollNotification, object: scroll, queue: nil) { [weak self] _ in
            MainActor.assumeIsolated { self?.isLiveScrolling = true }
        })
        observers.append(NotificationCenter.default.addObserver(forName: NSScrollView.didEndLiveScrollNotification, object: scroll, queue: nil) { [weak self] _ in
            MainActor.assumeIsolated { self?.userDidScroll() }
        })
        capture()
    }

    private func userDidScroll() {
        isLiveScrolling = false
        lastUserScrollAt = now()
        lastUserScrollTarget = followTarget.flatMap { scaledTarget($0) }
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

    // MARK: follow-caret (FollowCaret.swift)

    /// `target`/`enabled` as supplied by `PreviewAnchorKeeper.updateNSView`,
    /// i.e. on every SwiftUI body pass — after both the caret and the current
    /// frame/result are the ones the body read, the same dependency
    /// `caretHighlights` update from. A resupplied, unchanged target does not
    /// reset the debounce timer (requirement: never jitter).
    func followCaretDidChange(_ target: FollowCaret.Target?, enabled: Bool) {
        followEnabled = enabled
        guard target != followTarget else { return }
        followTarget = target
        followDebounceItem?.cancel()
        guard enabled, target != nil else { return }
        let item = DispatchWorkItem { [weak self] in self?.evaluateFollow() }
        followDebounceItem = item
        DispatchQueue.main.asyncAfter(deadline: .now() + followDebounceInterval, execute: item)
    }

    /// RAW target (page points, scale 1) → document-coordinate target: the
    /// page's frame under the current layout (already at `layout.scale`) plus
    /// the local rect scaled the same way.
    private func scaledTarget(_ raw: FollowCaret.Target) -> FollowCaret.Target? {
        guard let layout, let frame = layout.frame(of: raw.page) else { return nil }
        let scale = layout.scale
        let rect = CGRect(x: frame.minX + raw.rect.minX * scale, y: frame.minY + raw.rect.minY * scale,
                          width: raw.rect.width * scale, height: raw.rect.height * scale)
        return FollowCaret.Target(page: raw.page, rect: rect)
    }

    /// Fires `followDebounceInterval` after the last caret move to a new
    /// target: resolves the document rect fresh (the pane may have resized
    /// or rescaled during the wait) and asks `FollowCaret.decide`.
    private func evaluateFollow() {
        guard let raw = followTarget, let target = scaledTarget(raw) else { return }
        let input = FollowCaret.Input(preferenceEnabled: followEnabled, target: target, visibleRect: documentVisibleRectTopDown,
                                      now: now(), lastUserScrollAt: lastUserScrollAt, lastUserScrollTarget: lastUserScrollTarget,
                                      isUserDragging: isLiveScrolling, margin: followMargin, yieldWindow: followYieldWindow)
        let action = FollowCaret.decide(input)
        followActions.append(action)
        if case .scroll(let rect) = action { performFollowScroll(to: rect) }
    }

    /// One short animated scroll (skipped under reduce motion) that brings
    /// `rect` (document coordinates) into view with `followMargin` of slack —
    /// a minimal correction, not a re-center, so the caret's neighborhood
    /// stays on screen. Never posts a live-scroll notification (`setBoundsOrigin`
    /// is programmatic), so this never re-arms `isLiveScrolling`/`lastUserScrollAt`.
    private func performFollowScroll(to rect: CGRect) {
        guard let scroll = enclosingScrollView, let doc = scroll.documentView, let visible = documentVisibleRectTopDown else { return }
        let clip = scroll.contentView
        let size = clip.bounds.size
        var top = visible.minY, left = visible.minX
        if rect.minY < visible.minY + followMargin { top = rect.minY - followMargin }
        else if rect.maxY > visible.maxY - followMargin { top = rect.maxY - size.height + followMargin }
        if rect.minX < visible.minX + followMargin { left = rect.minX - followMargin }
        else if rect.maxX > visible.maxX - followMargin { left = rect.maxX - size.width + followMargin }
        let content = doc.bounds.size
        top = min(max(top, 0), max(0, content.height - size.height))
        left = min(max(left, 0), max(0, content.width - size.width))
        var origin = CGPoint(x: left, y: top)
        if !doc.isFlipped { origin.y = content.height - top - size.height }
        guard abs(origin.y - clip.bounds.origin.y) > 0.5 || abs(origin.x - clip.bounds.origin.x) > 0.5 else { return }
        note(String(format: "follow %.1f→%.1f", clip.bounds.origin.y, origin.y))
        if reduceMotion() {
            clip.setBoundsOrigin(origin)
            scroll.reflectScrolledClipView(clip)
        } else {
            NSAnimationContext.runAnimationGroup { ctx in
                ctx.duration = 0.2
                ctx.allowsImplicitAnimation = true
                clip.animator().setBoundsOrigin(origin)
            } completionHandler: { [weak scroll, weak clip] in
                guard let scroll, let clip else { return }
                scroll.reflectScrolledClipView(clip)
            }
        }
    }
}
