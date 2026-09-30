import AppKit
import IOSurface
import QuartzCore
import SwiftUI
import FlashTeXDisplayListV3
import FlashTeXPreviewV3

// The engine-v3 preview pane (flag-gated; EngineV3Host.swift). Pages are
// laid out fit-to-width in a scroll view; only pages near the visible area
// hold a bitmap. Every bitmap is rasterised off the main thread by
// `DL3Renderer` (the same routine the zero-tolerance parity test checks
// against Core Graphics' rendering of the engine's PDF); the main thread
// only installs `layer.contents` in an explicit Core Animation transaction.

struct PreviewV3Pane: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let session = model.engineV3
        ZStack(alignment: .bottomLeading) {
            EngineV3ScrollView(session: session, follow: model.caretFollow.request, dark: model.darkPreview)
                .background(DS.Colors.surfaceGround)
            VStack(alignment: .leading, spacing: 2) {
                if !session.projectTrusted {
                    // Owner decision 9A: a downloaded project runs no shell commands until trusted.
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.shield")
                        Text("This project came from another computer, so it compiles with shell escape off. Trust it to allow restricted \\write18, as pdflatex does.")
                            .fixedSize(horizontal: false, vertical: true)
                        Button("Trust This Project") { session.trustProject() }
                            .accessibilityIdentifier("engine-v3.trust")
                    }
                    .padding(.bottom, 4)
                }
                switch session.phase {
                case .idle:
                    Text("Engine v3 preview: idle")
                case .starting(let since):
                    HStack(spacing: 6) {
                        ProgressView().controlSize(.small)
                        TimelineView(.periodic(from: since, by: 0.5)) { ctx in
                            Text("\(session.environmentNote) \(Int(ctx.date.timeIntervalSince(since)))s")
                        }
                    }
                case .ready:
                    Text(session.statusNote.isEmpty ? "Compiling \(session.mainFile)…" : "\(session.mainFile) · \(session.statusNote)")
                    if let e = session.firstError {
                        Text(e).foregroundStyle(.red).lineLimit(3).textSelection(.enabled)
                    }
                    if session.errorCount + session.warningCount > 0 {
                        Text("\(session.errorCount) error\(session.errorCount == 1 ? "" : "s"), \(session.warningCount) warning\(session.warningCount == 1 ? "" : "s")")
                    }
                case .failed(let why):
                    Text(why).foregroundStyle(.red)
                }
                if !session.environmentNote.isEmpty, session.phase == .ready, model.previewDebugStatus {
                    Text(session.environmentNote)
                }
            }
            .font(.caption)
            .padding(6)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 6))
            .padding(8)
            .accessibilityElement(children: .combine)
        }
        .onAppear { session.start(model: model) }
    }
}

struct EngineV3ScrollView: NSViewRepresentable {
    let session: EngineV3Session
    /// The caret follower's latest request (CaretFollow.swift); acted on once per token.
    var follow: CaretFollowController.Request?
    /// The preview's dark toggle (title bar moon; default from the appearance setting).
    var dark = false

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.hasHorizontalScroller = false
        scroll.drawsBackground = false
        scroll.autohidesScrollers = true
        let pages = EngineV3PagesView(session: session)
        scroll.documentView = pages
        scroll.contentView.postsBoundsChangedNotifications = true
        scroll.contentView.postsFrameChangedNotifications = true
        NotificationCenter.default.addObserver(pages, selector: #selector(EngineV3PagesView.scrolled),
                                               name: NSView.boundsDidChangeNotification, object: scroll.contentView)
        // A window or split resize that is not a live resize (restored
        // frames, the window-frame automation, a sidebar toggle) re-lays out too.
        NotificationCenter.default.addObserver(pages, selector: #selector(EngineV3PagesView.resized),
                                               name: NSView.frameDidChangeNotification, object: scroll.contentView)
        // A scroll by hand stops following until the next edit (CaretFollow.swift).
        NotificationCenter.default.addObserver(pages, selector: #selector(EngineV3PagesView.userScrolled),
                                               name: NSScrollView.willStartLiveScrollNotification, object: scroll)
        session.view = pages
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        _ = session.layoutRevision // observed: page count/sizes changed
        let pages = scroll.documentView as? EngineV3PagesView
        pages?.setAppearance(dark ? .dark : .light)
        pages?.relayout()
        if let follow { pages?.follow(follow) }
    }
}

/// Where a page's bitmap goes, usable from any thread: the page view's
/// hosting layer (AppKit never draws into it) and the newest install's
/// ticket. The socket's reader thread and the raster queue install here
/// directly, in their own Core Animation transactions, so a page reaches the
/// screen without waiting for the main thread (measured 1–3 ms p50, more
/// while the editor works through a keystroke).
final class EngineV3LayerTarget: @unchecked Sendable {
    let layer: CALayer
    private let lock = NSLock()
    private var installed: UInt64 = 0
    private static let ticketLock = NSLock()
    private static var lastTicket: UInt64 = 0

    init(layer: CALayer) { self.layer = layer }

    /// Key → presented measurement (EngineV3PresentProbe): off unless enabled.
    private var probe: EngineV3PresentProbe?
    private var onPresented: (@Sendable (_ commitNs: UInt64, _ presentedNs: UInt64) -> Void)?

    /// Main thread, before the target is published to the raster threads.
    func enableProbe(_ report: @escaping @Sendable (_ commitNs: UInt64, _ presentedNs: UInt64) -> Void) {
        guard probe == nil, let p = EngineV3PresentProbe() else { return }
        CATransaction.begin(); CATransaction.setDisableActions(true)
        layer.addSublayer(p.layer)
        CATransaction.commit()
        lock.lock(); probe = p; onPresented = report; lock.unlock()
    }

    /// A ticket for a raster about to start: later tickets win.
    static func ticket() -> UInt64 { ticketLock.lock(); defer { ticketLock.unlock() }; lastTicket += 1; return lastTicket }

    /// Installs `contents` unless a newer raster already did; returns the
    /// commit time (after `CATransaction.commit()` + `flush()`), or nil.
    func install(_ contents: AnyObject, ticket: UInt64) -> UInt64? {
        lock.lock()
        guard ticket > installed else { lock.unlock(); return nil }
        installed = ticket
        // An explicit transaction, committed and flushed now: no implicit
        // transaction (which would wait for a run-loop turn) and no action.
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        layer.contents = contents
        var pair: EngineV3PresentPair?
        if let probe, let onPresented {
            let p = EngineV3PresentPair(onPresented)
            if probe.presentWithTransaction({ p.presented($0) }) { pair = p }
        }
        CATransaction.commit()
        CATransaction.flush()
        lock.unlock()
        let now = DispatchTime.now().uptimeNanoseconds
        pair?.committed(now)
        return now
    }
}

/// One page on screen: a layer-hosting view whose layer's contents is the
/// page bitmap (set through `target`, from any thread).
final class EngineV3PageView: NSView {
    var hashKey: [UInt8]?
    var rasterScale: Double = 0
    var generation = 0
    let target: EngineV3LayerTarget
    override var isFlipped: Bool { true }

    override init(frame: NSRect) {
        let l = CALayer()
        l.backgroundColor = CGColor(gray: 1, alpha: 1)
        l.contentsGravity = .resize
        l.shadowOpacity = 0.18
        l.shadowRadius = 3
        l.shadowOffset = CGSize(width: 0, height: -1)
        target = EngineV3LayerTarget(layer: l)
        super.init(frame: frame)
        layer = l // layer-hosting: AppKit positions the layer, never draws into it
        wantsLayer = true
    }
    required init?(coder: NSCoder) { fatalError() }

    func setStale(_ stale: Bool) {
        guard (layer?.opacity ?? 1) != (stale ? 0.45 : 1) || (layer?.borderWidth ?? 0) != (stale ? 2 : 0) else { return }
        // No implicit animation: a fresh page shows at full opacity in the
        // frame that carries it, not over the default 0.25 s fade.
        CATransaction.begin(); CATransaction.setDisableActions(true)
        defer { CATransaction.commit() }
        layer?.opacity = stale ? 0.45 : 1
        layer?.borderWidth = stale ? 2 : 0
        layer?.borderColor = stale ? NSColor.systemOrange.cgColor : nil
        setAccessibilityValue(stale ? "stale" : nil)
    }
}

final class EngineV3PagesView: NSView {
    weak var session: EngineV3Session?
    /// Published to the reader thread: which pages it may rasterise as they arrive.
    var rasterPlan: EngineV3RasterPlan?
    private var pageViews: [Int: EngineV3PageView] = [:]
    private var link: CADisplayLink?
    private lazy var boost = EngineV3FrameRateBoost(view: self)
    /// Pages changed by a keystroke's compile, not yet rastered: the compile id.
    private var pendingCompile: [Int: Int] = [:]
    private var frames: [CGRect] = []
    private var scale: Double = 1
    private static let rasterQueue = DispatchQueue(label: "flashtex.engine-v3.raster", qos: .userInteractive, attributes: .concurrent)
    private let margin: CGFloat = 16, gap: CGFloat = 12
    override var isFlipped: Bool { true }

    init(session: EngineV3Session) {
        self.session = session
        super.init(frame: .zero)
        setAccessibilityRole(.group)
        setAccessibilityLabel("PDF preview")
    }
    required init?(coder: NSCoder) { fatalError() }

    override func viewDidMoveToWindow() { relayout() }
    override func viewDidEndLiveResize() { relayout() }
    override func setFrameSize(_ newSize: NSSize) { super.setFrameSize(newSize) }

    @objc func scrolled() { updateVisible() }
    @objc func userScrolled() { session?.model?.caretFollow.userDidScrollPreview() }

    // MARK: forward search (source → preview)

    private var followedToken = 0
    private var flashLayer: CALayer?

    /// Acts on a caret-follow request once: scrolls the target into the
    /// comfort band only when it is outside it (CaretFollow's rules), and
    /// for an explicit request (⌘⇧J, ⌘-click in the editor) flashes it.
    func follow(_ r: CaretFollowController.Request) {
        guard r.token != followedToken else { return }
        followedToken = r.token
        guard r.target.page < frames.count, let scroll = enclosingScrollView else { return }
        let f = frames[r.target.page]
        let rect = CGRect(x: f.minX + r.target.rect.minX * scale, y: f.minY + r.target.rect.minY * scale,
                          width: max(r.target.rect.width * scale, 2), height: max(r.target.rect.height * scale, 2))
        let visible = scroll.contentView.bounds
        let inset = min(CaretFollow.visibleMargin, max(0, visible.height / 2 - 1))
        if rect.minY < visible.minY + inset || rect.maxY > visible.maxY - inset {
            let room = CaretFollow.revealInset(viewportHeight: visible.height, targetHeight: rect.height)
            var y = rect.minY < visible.minY + inset ? rect.minY - room : rect.maxY + room - visible.height
            y = min(max(y, 0), max(0, bounds.height - visible.height))
            let animated = !ReduceMotion.isEnabled && abs(y - visible.minY) <= CaretFollow.animationDistanceLimit * visible.height
            let origin = CGPoint(x: visible.minX, y: y)
            if animated {
                NSAnimationContext.runAnimationGroup { ctx in
                    ctx.duration = 0.18
                    scroll.contentView.animator().setBoundsOrigin(origin)
                } completionHandler: { [weak scroll] in scroll?.reflectScrolledClipView(scroll!.contentView) }
            } else {
                scroll.contentView.setBoundsOrigin(origin)
                scroll.reflectScrolledClipView(scroll.contentView)
            }
        }
        if r.reason == .explicit { flash(rect) }
    }

    /// A brief highlight over `rect` (document coordinates).
    func flash(_ rect: CGRect) {
        wantsLayer = true
        flashLayer?.removeFromSuperlayer()
        let l = CALayer()
        l.frame = rect.insetBy(dx: -2, dy: -2)
        l.backgroundColor = NSColor.systemYellow.withAlphaComponent(0.45).cgColor
        l.cornerRadius = 2
        l.zPosition = 10
        layer?.addSublayer(l)
        flashLayer = l
        let fade = CABasicAnimation(keyPath: "opacity")
        fade.fromValue = 1; fade.toValue = 0; fade.beginTime = CACurrentMediaTime() + 0.6; fade.duration = 0.5
        fade.fillMode = .forwards; fade.isRemovedOnCompletion = false
        l.add(fade, forKey: "fade")
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak l] in l?.removeFromSuperlayer() }
    }

    // MARK: reverse search (preview → source)

    override func mouseDown(with event: NSEvent) {
        let p = convert(event.locationInWindow, from: nil)
        guard let session, let i = frames.firstIndex(where: { $0.contains(p) }) else { return super.mouseDown(with: event) }
        let f = frames[i]
        let pagePoint = CGPoint(x: (p.x - f.minX) / scale, y: (p.y - f.minY) / scale)
        guard let src = session.source(page: i, at: pagePoint) else {
            session.model?.navigationNote = "Nothing here maps to the source."
            return
        }
        if let g = session.sourceIndex(page: i)?.hit(pagePoint) {
            flash(CGRect(x: f.minX + g.cell.minX * scale, y: f.minY + g.cell.minY * scale, width: g.cell.width * scale, height: g.cell.height * scale))
        }
        session.model?.navigateEngineV3(path: src.path, line: src.line, col: src.col)
    }
    @objc func resized() { if abs(available - bounds.width) > 0.5 { relayout() } }

    private var available: CGFloat { enclosingScrollView?.contentSize.width ?? bounds.width }

    /// Recomputes page frames (fit to width) and the visible set.
    func relayout() {
        guard let session else { return }
        let n = session.pageCount
        let widest = (0 ..< n).compactMap { session.pageSize($0).map { Double($0.width) } }.max() ?? 612
        let newScale = max(0.1, Double((available - 2 * margin) / widest))
        var y = margin
        var f: [CGRect] = []
        for i in 0 ..< n {
            let p = session.pageSize(i)
            let w = CGFloat(Double(p?.width ?? CGFloat(widest)) * newScale), h = CGFloat(Double(p?.height ?? CGFloat(widest * 1.294)) * newScale)
            f.append(CGRect(x: (available - w) / 2, y: y, width: w, height: h))
            y += h + gap
        }
        let height = max(y + margin - gap, enclosingScrollView?.contentSize.height ?? 0)
        let scaleChanged = abs(newScale - scale) > 1e-9
        frames = f
        scale = newScale
        if frame.size != CGSize(width: available, height: height) { setFrameSize(CGSize(width: available, height: height)) }
        for (i, v) in pageViews {
            if i >= n { v.removeFromSuperview(); pageViews[i] = nil; continue }
            v.frame = frames[i]
            if scaleChanged { v.rasterScale = 0 }
        }
        updateVisible()
    }

    private var pixelsPerPoint: Double { scale * Double(window?.backingScaleFactor ?? 2) }
    var currentPixelsPerPoint: Double { pixelsPerPoint }

    /// Page indexes intersecting the visible rect, plus one screen around it.
    private func visibleIndexes() -> [Int] {
        let r = visibleRect.insetBy(dx: 0, dy: -visibleRect.height)
        return frames.indices.filter { frames[$0].intersects(r) }
    }

    private func updateVisible() {
        guard let session else { return }
        let visible = visibleIndexes()
        let strictly = frames.indices.filter { frames[$0].intersects(visibleRect) }
        if let first = strictly.first {
            session.visiblePage = first
            if let model = session.model, model.previewVisiblePage != first + 1 { model.previewVisiblePage = first + 1 } // the HUD's page readout
        }
        let keep = Set(visible)
        for (i, v) in pageViews where !keep.contains(i) { v.removeFromSuperview(); pageViews[i] = nil }
        for i in visible { _ = pageView(i) }
        // The reader thread may draw and install these pages as they arrive.
        rasterPlan?.set(targets: pageViews.filter { keep.contains($0.key) }.mapValues(\.target), pixelsPerPoint: pixelsPerPoint, appearance: pageAppearance)
        for i in visible {
            let v = pageView(i)
            v.setStale(session.stale.contains(i))
            if v.rasterScale != pixelsPerPoint || v.hashKey != currentHash(i) { raster(i, compileID: nil) }
        }
    }

    private func pageView(_ i: Int) -> EngineV3PageView {
        if let v = pageViews[i] { return v }
        let v = EngineV3PageView(frame: frames[i])
        v.layer?.backgroundColor = pageAppearance.background
        if EngineV3PresentProbe.enabled, let session {
            let s = EngineV3WeakRef(session)
            v.target.enableProbe { c, p in EngineV3Session.onMain { s.value?.latency.presented(commitNs: c, presentedNs: p) } }
        }
        v.setAccessibilityElement(true)
        v.setAccessibilityRole(.image)
        v.setAccessibilityLabel("Page \(i + 1)")
        addSubview(v)
        pageViews[i] = v
        return v
    }

    private func currentHash(_ i: Int) -> [UInt8]? {
        guard let session else { return nil }
        if session.pdfFallback[i] != nil { return [0xFF] + Self.contentKey(session.pages[i]?.page.hash ?? [], pageAppearance) }
        return session.pages[i].map { Self.contentKey($0.page.hash, pageAppearance) }
    }

    /// What a page bitmap shows: the page's content hash and the appearance.
    nonisolated static func contentKey(_ hash: [UInt8], _ a: DL3Appearance) -> [UInt8] { hash + [a == .dark ? 1 : 0] }

    /// Light (the PDF, pixel-exact) or dark (EngineV3's reading mode:
    /// DL3Appearance). Changing it re-draws the pages near the viewport.
    private(set) var pageAppearance: DL3Appearance = .light
    func setAppearance(_ a: DL3Appearance) {
        guard a != pageAppearance else { return }
        pageAppearance = a
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for (_, v) in pageViews { v.layer?.backgroundColor = a.background }
        CATransaction.commit()
        updateVisible()
    }

    /// Rasterises page `i` off-main and installs it; `compileID` marks a
    /// keystroke-driven update (latency is stamped at the commit).
    private func raster(_ i: Int, compileID explicit: Int?) {
        guard let session, i < frames.count, let v = pageViews[i] else { return }
        guard let prepared = session.pages[i] else {
            // Instant reopen: the stored bitmap until the compile sends the page.
            guard let url = session.snapshotImageURL(i), v.hashKey != [0xEE] else { return }
            v.hashKey = [0xEE]; v.rasterScale = pixelsPerPoint
            let target = v.target, ticket = EngineV3LayerTarget.ticket()
            Self.rasterQueue.async {
                guard let img = EngineV3Snapshot.image(url), target.install(img, ticket: ticket) != nil else { return }
                EngineV3Session.onMain { [weak self] in self?.session?.noteOpenPixels(current: false) }
            }
            return
        }
        let compileID = explicit ?? pendingCompile[i]
        pendingCompile[i] = nil
        let forms = session.forms
        let fallback = session.pdfFallback[i]
        let ppp = pixelsPerPoint
        let key = currentHash(i)
        v.generation &+= 1
        v.rasterScale = ppp
        v.hashKey = key
        let target = v.target
        let look = pageAppearance
        let ticket = EngineV3LayerTarget.ticket()
        Self.rasterQueue.async { [weak self] in
            let t0 = MonotonicClock.nowNs()
            let image: AnyObject? = fallback.flatMap { DL3Renderer.rasterizeToSurface(pdfPage: $0, scale: ppp, appearance: look) }
                ?? DL3Renderer.rasterizeToSurface(prepared, forms: forms, scale: ppp, appearance: look)
            // Installed from this queue (the target is thread-safe); the main
            // thread only records it.
            guard let image, let committed = target.install(image, ticket: ticket) else { return }
            EngineV3Session.onMain {
                guard let self, let compileID else { return }
                self.recordCommit(compileID: compileID, page: i, installNs: t0, commitNs: committed)
            }
        }
    }

    /// Latency bookkeeping for a keystroke's page, on main after the fact.
    private func recordCommit(compileID: Int, page i: Int, installNs: UInt64, commitNs: UInt64) {
        guard let session else { return }
        session.noteOpenPixels(current: true)
        session.latency.committed(compile: compileID, page: i, at: commitNs, installNs: installNs)
        if session.latency.wantsVsync { armVsync() }
    }

    /// The next display-link frame after a commit: the frame that shows it.
    private func armVsync() {
        if link == nil {
            let l = displayLink(target: self, selector: #selector(frameTick(_:)))
            l.add(to: .main, forMode: .common)
            link = l
        }
        link?.isPaused = false
    }

    @objc private func frameTick(_ l: CADisplayLink) {
        // CADisplayLink times are CACurrentMediaTime(): mach absolute time, the same clock.
        session?.latency.vsync(targetNs: UInt64(l.targetTimestamp * 1e9))
        l.isPaused = true
    }

    // MARK: session notifications (main thread)

    /// A keystroke's compile was sent: keep the display at its fastest rate.
    func keystroke() { boost.keystroke() }
    var boostFrameNs: UInt64 { boost.frameNs }

    /// Returns whether the page is on screen (and so will be committed).
    /// `image`: the bitmap the reader thread already drew for it, if any.
    @discardableResult
    func pageArrived(_ i: Int, changed: Bool, compileID: Int, image: EngineV3Raster? = nil) -> Bool {
        if changed { pendingCompile[i] = compileID }
        if let image, i < frames.count, let v = pageViews[i], image.pixelsPerPoint == pixelsPerPoint,
           session?.pdfFallback[i] == nil, (frames[i].width - CGFloat((session?.pages[i]?.widthPt ?? 0) * scale)).magnitude <= 0.5 {
            // Drawn (and, normally, already installed) on the reader thread at
            // the scale on screen.
            v.generation &+= 1
            v.rasterScale = image.pixelsPerPoint
            v.hashKey = image.hash
            v.setStale(false)
            let compile = pendingCompile.removeValue(forKey: i)
            let t0 = MonotonicClock.nowNs()
            let committed = image.committedNs ?? v.target.install(image.image, ticket: image.ticket)
            if changed, let compile, let committed { recordCommit(compileID: compile, page: i, installNs: image.committedNs == nil ? t0 : image.installNs, commitNs: committed) }
            return frames[i].intersects(visibleRect)
        }
        if i >= frames.count || (frames[i].width - CGFloat((session?.pages[i]?.widthPt ?? 0) * scale)).magnitude > 0.5 { relayout() }
        if pendingCompile[i] == nil { return i < frames.count && frames[i].intersects(visibleRect) } // rastered by the relayout
        if i < frames.count, pageViews[i] == nil, frames[i].intersects(visibleRect.insetBy(dx: 0, dy: -visibleRect.height)) {
            updateVisible() // the page is near the viewport but has no view yet: make it (and raster it)
            if pendingCompile[i] == nil { return frames[i].intersects(visibleRect) }
        }
        guard i < frames.count, pageViews[i] != nil else { pendingCompile[i] = nil; return false }
        pageViews[i]?.setStale(false)
        if changed || pageViews[i]?.hashKey != currentHash(i) || pageViews[i]?.rasterScale != pixelsPerPoint {
            raster(i, compileID: nil)
            return frames[i].intersects(visibleRect)
        }
        pendingCompile[i] = nil
        return false
    }

    func formArrived(_ id: UInt32) {
        guard let session else { return }
        for (i, _) in pageViews {
            if session.pages[i]?.page.items.contains(where: { if case .form(let f, _) = $0 { f == id } else { false } }) == true {
                raster(i, compileID: nil)
            }
        }
    }

    /// The bitmap on screen for page `i` (evidence and tests).
    func installedImage(_ i: Int) -> CGImage? {
        guard let c = pageViews[i]?.layer?.contents else { return nil }
        if CFGetTypeID(c as CFTypeRef) == IOSurfaceGetTypeID() { return DL3Renderer.image(of: c as! IOSurface) }
        return (c as! CGImage)
    }

    func staleChanged() {
        guard let session else { return }
        for (i, v) in pageViews { v.setStale(session.stale.contains(i)) }
    }

    func fallbacksChanged(_ indexes: [Int]) {
        for i in indexes where pageViews[i] != nil { raster(i, compileID: nil) }
    }
}
