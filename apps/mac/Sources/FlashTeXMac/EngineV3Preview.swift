import AppKit
import IOSurface
import QuartzCore
import SwiftUI
import FlashTeXDisplayListV3
import FlashTeXPreviewV3

// The engine-v3 preview pane (flag-gated; EngineV3Host.swift). Pages are
// laid out at the fit-to-width scale times the preview zoom (pinch: a Core
// Animation transform until the gesture ends, EngineV3Tiles.swift) in a
// scroll view; only pages near the visible area hold a bitmap. Above about
// 3 px/pt a page shows 512 px tiles of its visible area over a 2 px/pt
// backdrop instead of one bitmap (EngineV3PageTiles). Every bitmap and tile
// is rasterised off the main thread by `DL3Renderer` (the same routine the zero-tolerance parity test checks
// against Core Graphics' rendering of the engine's PDF); the main thread
// only installs `layer.contents` in an explicit Core Animation transaction.

struct PreviewV3Pane: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let session = model.engineV3
        ZStack(alignment: .bottomLeading) {
            EngineV3ScrollView(session: session, zoom: model.previewZoom, follow: model.caretFollow.request, dark: model.darkPreview)
                .background(DS.Colors.surfaceGround)
            VStack(alignment: .leading, spacing: 2) {
                if !session.projectTrusted {
                    // Owner decision 9A: a downloaded project runs no shell commands until trusted.
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.shield")
                        Text("This project came from another computer\(session.trustOtherCount > 0 ? ", with \(session.trustOtherCount) other downloaded file\(session.trustOtherCount == 1 ? "" : "s") in its folder that it can read" : ""), so it compiles with shell escape off. Trust \(session.trustOtherCount > 0 ? "them" : "it") to allow restricted \\write18, as pdflatex does.")
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
    /// `ShellModel.previewZoom`: multiplies the fit-to-width scale.
    var zoom: CGFloat = 1
    /// The caret follower's latest request (CaretFollow.swift); acted on once per token.
    var follow: CaretFollowController.Request?
    /// The preview's dark toggle (title bar moon; default from the appearance setting).
    var dark = false

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = EngineV3ScrollContainer()
        scroll.hasVerticalScroller = true
        scroll.hasHorizontalScroller = true // pages wider than the pane when zoomed in
        scroll.drawsBackground = false
        scroll.autohidesScrollers = true
        scroll.wantsLayer = true
        let pages = EngineV3PagesView(session: session)
        pages.zoom = zoom
        scroll.pages = pages
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

    /// The pane takes what it is offered: no Auto Layout measuring of the
    /// scroll view on SwiftUI's layout passes (#1228's page-entry hitches).
    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSScrollView, context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 400, height: proposal.height ?? 400)
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let revision = session.layoutRevision // observed: page count/sizes changed
        let pages = scroll.documentView as? EngineV3PagesView
        pages?.setAppearance(dark ? .dark : .light)
        // Only when the layout inputs changed: a SwiftUI update for anything
        // else (the HUD, the status chip) costs no layout pass.
        pages?.update(revision: revision, zoom: zoom)
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
    /// A compile's raster was installed: a stored (instant reopen) bitmap
    /// never goes over it, whatever its ticket.
    private var hasReal = false
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
    func install(_ contents: AnyObject, ticket: UInt64) -> UInt64? { install(contents, ticket: ticket, stored: false) }

    /// A stored page's bitmap (instant reopen): never over a compile's.
    func installStored(_ contents: AnyObject, ticket: UInt64) -> UInt64? { install(contents, ticket: ticket, stored: true) }

    /// Takes a stored bitmap off the layer (its snapshot was dropped), unless a compile's is there.
    func clearStored(ticket: UInt64) {
        lock.lock(); defer { lock.unlock() }
        guard ticket > installed, !hasReal else { return }
        installed = ticket
        CATransaction.begin(); CATransaction.setDisableActions(true)
        layer.contents = nil
        CATransaction.commit()
    }

    private func install(_ contents: AnyObject, ticket: UInt64, stored: Bool) -> UInt64? {
        lock.lock()
        guard ticket > installed, !(stored && hasReal) else { lock.unlock(); return nil }
        installed = ticket
        if !stored { hasReal = true }
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
    /// High-zoom tiles over the page bitmap (the backdrop then).
    let tiles = EngineV3PageTiles()
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
        tiles.container.frame = l.bounds
        l.addSublayer(tiles.container)
    }
    required init?(coder: NSCoder) { fatalError() }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        CATransaction.begin(); CATransaction.setDisableActions(true)
        tiles.container.frame = CGRect(origin: .zero, size: newSize)
        CATransaction.commit()
    }

    /// The layer shows (or is about to show) a stored bitmap of an instant
    /// reopen: dimmed whatever the session's marks, until a compile's
    /// raster of the page is committed.
    var showsStored = false { didSet { if showsStored != oldValue { applyStale() } } }
    private var marked = false

    func setStale(_ stale: Bool) {
        marked = stale
        applyStale()
    }

    private func applyStale() {
        let stale = marked || showsStored
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
    /// `ShellModel.previewZoom` as last laid out (fit-to-width × zoom).
    var zoom: CGFloat = 1
    /// The fit-to-width scale of the last layout (the HUD's zoom readout).
    private(set) var fitScale: Double = 1
    /// The inputs of the last layout: page count/size revision, zoom, width.
    private var laidOut: (revision: Int, zoom: CGFloat, width: CGFloat)?
    /// `FLASHTEX_V3_PPP` (evidence only): pages at exactly this many pixels
    /// per point, whatever the pane width and zoom.
    static let fixedPixelsPerPoint = ProcessInfo.processInfo.environment["FLASHTEX_V3_PPP"].flatMap(Double.init)
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
    /// Another screen's backing scale: pixels per point (and tiles) change.
    override func viewDidChangeBackingProperties() { super.viewDidChangeBackingProperties(); relayout() }
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
        // Zoomed in, a page is wider than the pane: the target must be in
        // view horizontally too (centred when it is outside the margin).
        let insetX = min(CaretFollow.visibleMargin, max(0, visible.width / 2 - 1))
        let offX = rect.minX < visible.minX + insetX || rect.maxX > visible.maxX - insetX
        let offY = rect.minY < visible.minY + inset || rect.maxY > visible.maxY - inset
        if offX || offY {
            var y = visible.minY
            if offY {
                let room = CaretFollow.revealInset(viewportHeight: visible.height, targetHeight: rect.height)
                y = rect.minY < visible.minY + inset ? rect.minY - room : rect.maxY + room - visible.height
                y = min(max(y, 0), max(0, bounds.height - visible.height))
            }
            var x = visible.minX
            if offX {
                x = rect.width >= visible.width - 2 * insetX ? rect.minX - insetX : rect.midX - visible.width / 2
                x = min(max(x, 0), max(0, bounds.width - visible.width))
            }
            let animated = !ReduceMotion.isEnabled && abs(y - visible.minY) <= CaretFollow.animationDistanceLimit * visible.height
                && abs(x - visible.minX) <= CaretFollow.animationDistanceLimit * visible.width
            let origin = CGPoint(x: x, y: y)
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
    @objc func resized() { if abs(available - (laidOut?.width ?? -1)) > 0.5 { relayout() } }

    private var available: CGFloat { enclosingScrollView?.contentSize.width ?? bounds.width }
    private var backingScale: CGFloat { window?.backingScaleFactor ?? 2 }

    /// Lays out again only when its inputs changed (SwiftUI's updateNSView).
    func update(revision: Int, zoom newZoom: CGFloat) {
        let z = PreviewZoom.clamped(newZoom)
        if let l = laidOut, l.revision == revision, l.zoom == z, abs(l.width - available) <= 0.5 { return }
        zoom = z
        relayout(revision: revision)
    }

    /// Recomputes page frames (fit to width × zoom, origins on device pixels)
    /// and the visible set. On a scale change the page point at `anchor`
    /// (document coordinates; default the top centre of the visible rect)
    /// stays where it is in the viewport.
    func relayout(revision: Int? = nil, anchor: CGPoint? = nil) {
        guard let session else { return }
        let n = session.pageCount
        let avail = available
        // `pageSize`: a compiled page, or a stored one of an instant reopen.
        let widest = (0 ..< n).compactMap { session.pageSize($0).map { Double($0.width) } }.max() ?? 612
        let fit = max(0.1, Double((avail - 2 * margin) / widest))
        let bs = backingScale
        let newScale = Self.fixedPixelsPerPoint.map { $0 / Double(bs) } ?? fit * Double(PreviewZoom.clamped(zoom))
        let scaleChanged = abs(newScale - scale) > 1e-9
        // The page point under the anchor, and where it is in the viewport.
        var keep: (page: Int, point: CGPoint, offset: CGPoint)?
        if scaleChanged, !frames.isEmpty, let clip = enclosingScrollView?.contentView {
            let a = anchor ?? CGPoint(x: visibleRect.midX, y: visibleRect.minY)
            let i = frames.firstIndex { $0.maxY + gap >= a.y } ?? frames.count - 1
            let f = frames[i]
            keep = (i, CGPoint(x: (a.x - f.minX) / scale, y: (a.y - f.minY) / scale),
                    CGPoint(x: a.x - clip.bounds.minX, y: a.y - clip.bounds.minY))
        }
        func px(_ v: CGFloat) -> CGFloat { (v * bs).rounded() / bs }
        let width = max(avail, CGFloat(widest * newScale) + 2 * margin)
        var y = margin
        var f: [CGRect] = []
        for i in 0 ..< n {
            let p = session.pageSize(i)
            let w = CGFloat(Double(p?.width ?? CGFloat(widest)) * newScale), h = CGFloat(Double(p?.height ?? CGFloat(widest * 1.294)) * newScale)
            // Device-pixel origins: a tile (1 px = 1/backing pt) lands on the pixel grid.
            f.append(CGRect(x: px((width - w) / 2), y: px(y), width: w, height: h))
            y += h + gap
        }
        let height = max(y + margin - gap, enclosingScrollView?.contentSize.height ?? 0)
        frames = f
        scale = newScale
        fitScale = fit
        laidOut = (revision ?? laidOut?.revision ?? session.layoutRevision, PreviewZoom.clamped(zoom), avail)
        if let model = session.model, abs(model.previewFitScale - CGFloat(fit)) > 1e-6 { model.previewFitScale = CGFloat(fit) }
        if frame.size != CGSize(width: width, height: height) { setFrameSize(CGSize(width: width, height: height)) }
        for (i, v) in pageViews {
            if i >= n { v.tiles.removeAll(); v.removeFromSuperview(); pageViews[i] = nil; continue }
            v.frame = frames[i] // (updateVisible re-rasters a bitmap whose scale is not `wholeScale`)
        }
        if let keep, keep.page < frames.count, let scroll = enclosingScrollView {
            let clip = scroll.contentView
            let fr = frames[keep.page]
            let p = CGPoint(x: fr.minX + keep.point.x * newScale, y: fr.minY + keep.point.y * newScale)
            let origin = CGPoint(x: min(max(0, p.x - keep.offset.x), max(0, width - clip.bounds.width)),
                                 y: min(max(0, p.y - keep.offset.y), max(0, height - clip.bounds.height)))
            if origin != clip.bounds.origin {
                clip.scroll(to: origin)
                scroll.reflectScrolledClipView(clip)
            }
        }
        updateVisible()
    }

    /// The end of a pinch: lays out at `newZoom` (the transform is removed in
    /// the same transaction), then publishes it.
    func commitZoom(_ newZoom: CGFloat, anchor: CGPoint) {
        zoom = PreviewZoom.clamped(newZoom)
        relayout(anchor: anchor)
        if let model = session?.model, model.previewZoom != zoom { model.previewZoom = zoom }
    }

    /// While a pinch runs every tile samples linearly under the transform.
    func setPinching(_ on: Bool) {
        pinching = on
        for v in pageViews.values { v.tiles.pinching = on }
    }
    private(set) var pinching = false

    private var pixelsPerPoint: Double { scale * Double(backingScale) }
    var currentPixelsPerPoint: Double { pixelsPerPoint }
    /// Whether pages show tiles (the scale is above the tile threshold).
    var tiled: Bool { EngineV3TileGrid.tiles(pixelsPerPoint) }
    /// The scale of each page's whole bitmap: the scale on screen, or the
    /// backdrop's when tiled.
    private var wholeScale: Double { tiled ? min(pixelsPerPoint, EngineV3TileGrid.backdropPixelsPerPoint) : pixelsPerPoint }

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
        for (i, v) in pageViews where !keep.contains(i) { v.tiles.removeAll(); v.removeFromSuperview(); pageViews[i] = nil }
        for i in visible { _ = pageView(i) }
        // The reader thread may draw and install these pages as they arrive
        // (whole pages only: a tiled page's tiles come from the tile queue).
        rasterPlan?.set(targets: tiled ? [:] : pageViews.filter { keep.contains($0.key) }.mapValues(\.target), pixelsPerPoint: pixelsPerPoint,
                        appearance: pageAppearance)
        let whole = wholeScale
        for i in visible {
            let v = pageView(i)
            v.setStale(session.stale.contains(i))
            if v.rasterScale != whole || v.hashKey != currentHash(i) { raster(i, compileID: nil) } else { updateTiles(i, compileID: nil) }
        }
        EngineV3ScrollBench.startIfRequested(from: self)
    }

    /// Brings page `i`'s tiles in line with the scale, its content and the
    /// visible rect (no drawing here); drops them when not tiled.
    private func updateTiles(_ i: Int, compileID: Int?) {
        guard let v = pageViews[i] else { return }
        guard tiled, let session, let prepared = session.pages[i] else {
            if v.tiles.source != nil { v.tiles.removeAll() }
            return
        }
        // What the tiles show: the page's content and appearance, and the
        // content of the forms it draws (by hash, not by when they arrived: a
        // form sent again unchanged, or a fallback PDF reloaded at DONE, keeps
        // the tiles and the kept raster).
        let key = (currentHash(i) ?? []) + Self.formKey(prepared, session.forms)
        let pdf = session.pdfFallback[i]
        // A page drawn whole (paths, images, forms, PDF) keeps one full-scale
        // raster per source (EngineV3RasterHolder); only a raster over 1 GiB
        // takes a lower scale (DL3Renderer.tileScale, last resort).
        let drawnWhole = pdf != nil || (!DL3Renderer.tilesByTranslation(prepared) && !DL3Renderer.clipExact(prepared))
        let scale = DL3Renderer.tileScale(widthPt: prepared.widthPt, heightPt: prepared.heightPt, drawnWhole: drawnWhole, pixelsPerPoint: pixelsPerPoint)
        var source = EngineV3TileSource(prepared: prepared, forms: session.forms, pdf: pdf, key: key, pixelsPerPoint: scale,
                                        displayScale: Double(backingScale), screenPixelsPerPoint: pixelsPerPoint, appearance: pageAppearance)
        source.smoothFonts = session.smoothFonts // in `key` already (`currentHash`)
        if pdf != nil {
            // Its raster is light in both appearances: identified without it
            // (but with the font smoothing it was drawn with).
            source.pdfIdentity = [0xFF] + prepared.page.hash + (session.smoothFonts ? [1] : [])
        }
        v.tiles.pinching = pinching
        v.tiles.show(source, visible: v.visibleRect, compileID: compileID)
    }

    /// The hashes of the forms `page` draws (nested ones too, as drawn), in order.
    nonisolated static func formKey(_ page: DL3PreparedPage, _ forms: [UInt32: DL3PreparedPage], depth: Int = 0) -> [UInt8] {
        guard depth < 8 else { return [] }
        var out: [UInt8] = []
        for it in page.page.items {
            guard case .form(let id, _) = it else { continue }
            withUnsafeBytes(of: id) { out.append(contentsOf: $0) }
            if let f = forms[id] { out += f.page.hash + formKey(f, forms, depth: depth + 1) } else { out.append(0) }
        }
        return out
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
        v.tiles.onCommitted = { [weak self] compile, t0, t1 in self?.recordCommit(compileID: compile, page: i, installNs: t0, commitNs: t1) }
        addSubview(v)
        pageViews[i] = v
        return v
    }

    private func currentHash(_ i: Int) -> [UInt8]? {
        guard let session else { return nil }
        let smooth = session.smoothFonts
        if session.pdfFallback[i] != nil { return [0xFF] + Self.contentKey(session.pages[i]?.page.hash ?? [], pageAppearance, smoothFonts: smooth) }
        return session.pages[i].map { Self.contentKey($0.page.hash, pageAppearance, smoothFonts: smooth) }
    }

    /// What a page bitmap shows: the page's content hash, the appearance and
    /// whether its glyphs were drawn with font smoothing (off: no extra byte,
    /// the key as before the setting existed).
    nonisolated static func contentKey(_ hash: [UInt8], _ a: DL3Appearance, smoothFonts: Bool = false) -> [UInt8] {
        hash + [a == .dark ? 1 : 0] + (smoothFonts ? [1] : [])
    }

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
            v.showsStored = true
            let target = v.target, ticket = EngineV3LayerTarget.ticket()
            Self.rasterQueue.async {
                guard let img = EngineV3Snapshot.image(url), target.installStored(img, ticket: ticket) != nil else { return }
                EngineV3Session.onMain { [weak self] in self?.session?.noteOpenPixels(current: false) }
            }
            return
        }
        var compileID = explicit ?? pendingCompile[i]
        pendingCompile[i] = nil
        if tiled {
            // The tiles show this compile (their commit is the one stamped);
            // the bitmap below is the backdrop.
            updateTiles(i, compileID: compileID)
            compileID = nil
        } else if v.tiles.source != nil {
            v.tiles.removeAll()
        }
        let forms = session.forms
        let fallback = session.pdfFallback[i]
        let ppp = wholeScale
        let key = currentHash(i)
        v.generation &+= 1
        v.rasterScale = ppp
        v.hashKey = key
        let target = v.target
        let look = pageAppearance
        let ticket = EngineV3LayerTarget.ticket()
        let smooth = session.smoothFonts
        // Replacing a stored bitmap: it stays dimmed until this one is committed.
        let replacesStored = v.showsStored, generation = v.generation
        Self.rasterQueue.async { [weak self] in
            let t0 = MonotonicClock.nowNs()
            let image: AnyObject? = fallback.flatMap { DL3Renderer.rasterizeToSurface(pdfPage: $0, scale: ppp, appearance: look, smoothFonts: smooth) }
                ?? DL3Renderer.rasterizeToSurface(prepared, forms: forms, scale: ppp, appearance: look, smoothFonts: smooth)
            // Installed from this queue (the target is thread-safe); the main
            // thread only records it.
            guard let image, let committed = target.install(image, ticket: ticket) else { return }
            if replacesStored {
                EngineV3Session.onMain { [weak self] in
                    if let v = self?.pageViews[i], v.generation == generation { v.showsStored = false }
                }
            }
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
        if let image, !tiled, i < frames.count, let v = pageViews[i], image.pixelsPerPoint == pixelsPerPoint,
           session?.pdfFallback[i] == nil, (frames[i].width - CGFloat((session?.pages[i]?.widthPt ?? 0) * scale)).magnitude <= 0.5 {
            // Drawn (and, normally, already installed) on the reader thread at
            // the scale on screen.
            v.generation &+= 1
            v.rasterScale = image.pixelsPerPoint
            v.hashKey = image.hash
            let compile = pendingCompile.removeValue(forKey: i)
            let t0 = MonotonicClock.nowNs()
            let committed = image.committedNs ?? v.target.install(image.image, ticket: image.ticket)
            // The compile's bitmap is on the layer now (or a newer one is): undim after it, never before.
            v.showsStored = false
            v.setStale(false)
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
        if changed || pageViews[i]?.hashKey != currentHash(i) || pageViews[i]?.rasterScale != wholeScale {
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

    /// Drops every page's tiles: their queued jobs skip undrawn and their
    /// kept rasters are freed (the session stopped).
    func dropAllTiles() { for v in pageViews.values { v.tiles.removeAll() } }

    /// Page views held now (tests, the scroll bench).
    var heldPageViews: [Int: EngineV3PageView] { pageViews }

    /// The page and page point (bp from the top-left) at a document point
    /// (tests: the pinch's fixed point).
    func pagePoint(at p: CGPoint) -> (page: Int, point: CGPoint)? {
        guard let i = frames.firstIndex(where: { $0.contains(p) }) else { return nil }
        return (i, CGPoint(x: (p.x - frames[i].minX) / scale, y: (p.y - frames[i].minY) / scale))
    }

    /// Bitmap bytes held: page bitmaps (backdrops when tiled) and tiles.
    var retainedBytes: Int {
        pageViews.values.reduce(0) { sum, v in
            var own = 0
            if let c = v.layer?.contents {
                if CFGetTypeID(c as CFTypeRef) == IOSurfaceGetTypeID() { own = (c as! IOSurface).allocationSize }
                else { let i = c as! CGImage; own = i.bytesPerRow * i.height }
            }
            return sum + own + v.tiles.retainedBytes
        }
    }

    /// Visible tiles not yet up, over the pages on screen.
    var missingVisibleTiles: Int { pageViews.values.reduce(0) { $0 + $1.tiles.missingVisible($1.visibleRect) } }

    /// The bitmap on screen for page `i` (evidence and tests).
    func installedImage(_ i: Int) -> CGImage? {
        guard let c = pageViews[i]?.layer?.contents else { return nil }
        if CFGetTypeID(c as CFTypeRef) == IOSurfaceGetTypeID() { return DL3Renderer.image(of: c as! IOSurface) }
        return (c as! CGImage)
    }

    /// The snapshot was dropped: stored bitmaps come off their pages.
    func dropStored() {
        for (_, v) in pageViews where v.showsStored {
            v.target.clearStored(ticket: EngineV3LayerTarget.ticket())
            v.hashKey = nil
            v.showsStored = false
        }
    }

    func staleChanged() {
        guard let session else { return }
        for (i, v) in pageViews { v.setStale(session.stale.contains(i)) }
    }

    /// Settings > "Smooth fonts in preview" changed: every page bitmap held
    /// was drawn the other way (its `contentKey` no longer matches), so each
    /// is redrawn (`updateVisible`).
    func fontSmoothingChanged() {
        for v in pageViews.values { v.rasterScale = 0 }
        updateVisible()
    }

    func fallbacksChanged(_ indexes: [Int]) {
        for i in indexes where pageViews[i] != nil { raster(i, compileID: nil) }
    }
}
