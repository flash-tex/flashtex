import AppKit
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
            EngineV3ScrollView(session: session)
                .background(DS.Colors.surfaceGround)
            VStack(alignment: .leading, spacing: 2) {
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
                    Text(session.statusNote.isEmpty ? "Compiling…" : session.statusNote)
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

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.hasHorizontalScroller = false
        scroll.drawsBackground = false
        scroll.autohidesScrollers = true
        let pages = EngineV3PagesView(session: session)
        scroll.documentView = pages
        scroll.contentView.postsBoundsChangedNotifications = true
        NotificationCenter.default.addObserver(pages, selector: #selector(EngineV3PagesView.scrolled),
                                               name: NSView.boundsDidChangeNotification, object: scroll.contentView)
        session.view = pages
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        _ = session.layoutRevision // observed: page count/sizes changed
        (scroll.documentView as? EngineV3PagesView)?.relayout()
    }
}

/// One page on screen: a layer-backed view whose contents is a bitmap.
final class EngineV3PageView: NSView {
    var hashKey: [UInt8]?
    var rasterScale: Double = 0
    var generation = 0
    override var isFlipped: Bool { true }
    override var wantsUpdateLayer: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        layer?.backgroundColor = CGColor(gray: 1, alpha: 1)
        layer?.contentsGravity = .resize
        layer?.shadowOpacity = 0.18
        layer?.shadowRadius = 3
        layer?.shadowOffset = CGSize(width: 0, height: -1)
    }
    required init?(coder: NSCoder) { fatalError() }

    func setStale(_ stale: Bool) {
        layer?.opacity = stale ? 0.45 : 1
        layer?.borderWidth = stale ? 2 : 0
        layer?.borderColor = stale ? NSColor.systemOrange.cgColor : nil
        setAccessibilityValue(stale ? "stale" : nil)
    }
}

final class EngineV3PagesView: NSView {
    weak var session: EngineV3Session?
    private var pageViews: [Int: EngineV3PageView] = [:]
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

    private var available: CGFloat { enclosingScrollView?.contentSize.width ?? bounds.width }

    /// Recomputes page frames (fit to width) and the visible set.
    func relayout() {
        guard let session else { return }
        let n = session.pageCount
        let widest = (0 ..< n).compactMap { session.pages[$0]?.widthPt }.max() ?? 612
        let newScale = max(0.1, Double((available - 2 * margin) / widest))
        var y = margin
        var f: [CGRect] = []
        for i in 0 ..< n {
            let p = session.pages[i]
            let w = CGFloat((p?.widthPt ?? widest) * newScale), h = CGFloat((p?.heightPt ?? widest * 1.294) * newScale)
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

    /// Page indexes intersecting the visible rect, plus one screen around it.
    private func visibleIndexes() -> [Int] {
        let r = visibleRect.insetBy(dx: 0, dy: -visibleRect.height)
        return frames.indices.filter { frames[$0].intersects(r) }
    }

    private func updateVisible() {
        guard let session else { return }
        let visible = visibleIndexes()
        let strictly = frames.indices.filter { frames[$0].intersects(visibleRect) }
        if let first = strictly.first { session.visiblePage = first }
        let keep = Set(visible)
        for (i, v) in pageViews where !keep.contains(i) { v.removeFromSuperview(); pageViews[i] = nil }
        for i in visible {
            let v = pageView(i)
            v.setStale(session.stale.contains(i))
            if v.rasterScale != pixelsPerPoint || v.hashKey != currentHash(i) { raster(i, compileID: nil) }
        }
    }

    private func pageView(_ i: Int) -> EngineV3PageView {
        if let v = pageViews[i] { return v }
        let v = EngineV3PageView(frame: frames[i])
        v.setAccessibilityElement(true)
        v.setAccessibilityRole(.image)
        v.setAccessibilityLabel("Page \(i + 1)")
        addSubview(v)
        pageViews[i] = v
        return v
    }

    private func currentHash(_ i: Int) -> [UInt8]? {
        guard let session else { return nil }
        if session.pdfFallback[i] != nil { return [0xFF] + (session.pages[i]?.page.hash ?? []) }
        return session.pages[i]?.page.hash
    }

    /// Rasterises page `i` off-main and installs it; `compileID` marks a
    /// keystroke-driven update (latency is stamped at the commit).
    private func raster(_ i: Int, compileID explicit: Int?) {
        guard let session, i < frames.count, let v = pageViews[i] else { return }
        guard let prepared = session.pages[i] else { return }
        let compileID = explicit ?? pendingCompile[i]
        pendingCompile[i] = nil
        let forms = session.forms
        let fallback = session.pdfFallback[i]
        let ppp = pixelsPerPoint
        let key = currentHash(i)
        v.generation &+= 1
        let gen = v.generation
        v.rasterScale = ppp
        v.hashKey = key
        Self.rasterQueue.async { [weak self] in
            let image = fallback.flatMap { DL3Renderer.rasterize(pdfPage: $0, scale: ppp) }
                ?? DL3Renderer.rasterize(prepared, forms: forms, scale: ppp)
            EngineV3Session.onMain {
                guard let self, let v = self.pageViews[i], v.generation == gen, let image else { return }
                CATransaction.begin()
                CATransaction.setDisableActions(true)
                v.layer?.contents = image
                CATransaction.commit()
                CATransaction.flush()
                if let compileID { self.session?.latency.committed(compile: compileID, page: i, at: MonotonicClock.nowNs()) }
            }
        }
    }

    // MARK: session notifications (main thread)

    /// Returns whether the page is on screen (and so will be committed).
    @discardableResult
    func pageArrived(_ i: Int, changed: Bool, compileID: Int) -> Bool {
        if changed { pendingCompile[i] = compileID }
        if i >= frames.count || (frames[i].width - CGFloat((session?.pages[i]?.widthPt ?? 0) * scale)).magnitude > 0.5 { relayout() }
        if pendingCompile[i] == nil { return i < frames.count && frames[i].intersects(visibleRect) } // rastered by the relayout
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
