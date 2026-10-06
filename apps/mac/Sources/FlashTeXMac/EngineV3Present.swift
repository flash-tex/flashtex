import AppKit
import Metal
import QuartzCore

/// Key → presented measurement (owner request, 2026-09-30).
///
/// A page's bitmap is installed in an explicit Core Animation transaction
/// that is committed and flushed at once (`EngineV3LayerTarget.install`).
/// When it reaches the glass is the window server's business, and AppKit
/// reports nothing. This probe is a 1×1, fully transparent `CAMetalLayer` inside
/// the page's layer. Its layer has `presentsWithTransaction`, so the probe's
/// drawable is presented by the same transaction as the page's new contents,
/// and the drawable's `presentedTime` is when that frame went on screen.
///
/// Measurement only: on with `FLASHTEX_V3_PRESENT=1` or in the keystroke
/// bench (`FLASHTEX_V3_BENCH`, unless `FLASHTEX_V3_PRESENT=0`). Off, nothing
/// here runs and the page layers have no sublayer.
///
/// The window must be on screen: an occluded window's frames are never shown,
/// and every `presentedTime` is 0. The bench's `FLASHTEX_V3_BENCH_FRONT=1`
/// orders it in front. Cross-check: with `FLASHTEX_V3_PRESENT_TX=0` the probe
/// presents on its own (`MTLCommandBuffer.present`) in the same instant
/// instead. Measured, the two agree: commit→presented p50 21.4 vs 21.6 ms,
/// minimum 16.0 ms in both.
final class EngineV3PresentProbe: @unchecked Sendable {
    static let enabled: Bool = {
        let e = ProcessInfo.processInfo.environment
        if let v = e["FLASHTEX_V3_PRESENT"] { return v == "1" }
        return (e["FLASHTEX_V3_BENCH"] ?? "").isEmpty == false
    }()
    private static let device = MTLCreateSystemDefaultDevice()
    private static let queue = device?.makeCommandQueue()
    static let withTransaction = ProcessInfo.processInfo.environment["FLASHTEX_V3_PRESENT_TX"] != "0"

    let layer: CAMetalLayer

    init?() {
        guard let device = Self.device, Self.queue != nil else { return nil }
        let l = CAMetalLayer()
        l.device = device
        l.pixelFormat = .bgra8Unorm
        l.framebufferOnly = true
        l.isOpaque = false
        l.drawableSize = CGSize(width: 1, height: 1)
        l.frame = CGRect(x: 0, y: 0, width: 1, height: 1)
        l.presentsWithTransaction = Self.withTransaction
        layer = l
    }

    /// Call inside an open CA transaction, before it commits: presents a
    /// transparent drawable with that transaction. `done` gets the drawable's
    /// `presentedTime` in `CLOCK_UPTIME_RAW` nanoseconds (the clock of
    /// `CACurrentMediaTime`), or 0 when the frame was not shown.
    func presentWithTransaction(_ done: @escaping @Sendable (UInt64) -> Void) -> Bool {
        guard let queue = Self.queue, let drawable = layer.nextDrawable(), let cb = queue.makeCommandBuffer() else { return false }
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = drawable.texture
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 0)
        pass.colorAttachments[0].storeAction = .store
        cb.makeRenderCommandEncoder(descriptor: pass)?.endEncoding()
        drawable.addPresentedHandler { d in done(d.presentedTime > 0 ? UInt64(d.presentedTime * 1e9) : 0) }
        if !Self.withTransaction {
            cb.present(drawable)
            cb.commit()
            return true
        }
        cb.commit()
        cb.waitUntilScheduled() // presentsWithTransaction: present after scheduling, inside the transaction
        drawable.present()
        return true
    }
}

/// Pairs an install's commit time with its probe's presented time, which may
/// arrive in either order on different threads.
final class EngineV3PresentPair: @unchecked Sendable {
    private let lock = NSLock()
    private var commitNs: UInt64?
    private var presentedNs: UInt64?
    private let report: @Sendable (_ commitNs: UInt64, _ presentedNs: UInt64) -> Void

    init(_ report: @escaping @Sendable (UInt64, UInt64) -> Void) { self.report = report }

    func committed(_ ns: UInt64) { lock.lock(); commitNs = ns; fire() }
    func presented(_ ns: UInt64) { lock.lock(); presentedNs = ns; fire() }
    private func fire() { // called locked
        guard let c = commitNs, let p = presentedNs else { lock.unlock(); return }
        commitNs = nil; presentedNs = nil
        lock.unlock()
        report(c, p)
    }
}

/// While the user types, asks for the display's highest refresh rate: a
/// display link with `preferredFrameRateRange` up to 120 Hz, running from a
/// keystroke until 1 s after the last one.
///
/// OFF by default; `FLASHTEX_V3_BOOST=1` turns it on. It was measured on the
/// built-in ProMotion display (2026-09-30, 80 keystrokes per arm and
/// document). It made no difference: key→presented p50 was 30.9 ms with it
/// and 30.9 ms without on plain-10. In both arms frames went on screen on the
/// 120 Hz grid, because macOS already raises the rate when a window's content
/// changes. It is not worth the power.
@MainActor
final class EngineV3FrameRateBoost: NSObject {
    static let enabled = ProcessInfo.processInfo.environment["FLASHTEX_V3_BOOST"] == "1"
    /// Invalidated in deinit (nonisolated(unsafe): deinit is nonisolated; it runs on main).
    nonisolated(unsafe) private var link: CADisplayLink?
    private var lastKeyNs: UInt64 = 0
    private weak var view: NSView?
    /// The newest frame duration the link reported (evidence).
    private(set) var frameNs: UInt64 = 0

    init(view: NSView) { self.view = view }

    deinit { link?.invalidate() }

    func keystroke() {
        guard Self.enabled, let view, view.window != nil else { return }
        lastKeyNs = MonotonicClock.nowNs()
        if link == nil {
            let l = view.displayLink(target: EngineV3WeakLinkTarget(self) { $0.tick($1) }, selector: #selector(EngineV3WeakLinkTarget.tick(_:)))
            l.preferredFrameRateRange = CAFrameRateRange(minimum: 80, maximum: 120, preferred: 120)
            l.add(to: .main, forMode: .common)
            link = l
        }
        link?.isPaused = false
    }

    private func tick(_ l: CADisplayLink) {
        frameNs = UInt64(max(0, l.targetTimestamp - l.timestamp) * 1e9)
        if MonotonicClock.nowNs() &- lastKeyNs > 1_000_000_000 { l.isPaused = true }
    }

    func stop() { link?.invalidate(); link = nil }
}

/// A display link's target that holds the real one weakly. `CADisplayLink`
/// retains its target until it is invalidated, and the run loop it was added
/// to retains the link, so a link aimed straight at a view (or at an object
/// the view owns) keeps that view, and whatever it references, alive forever,
/// paused or not: every preview pane that ever took a keystroke's compile
/// leaked that way (DocumentDeallocationTests). The link invalidates itself
/// on its first tick after the target is gone; owners also invalidate it
/// when they go.
@MainActor
final class EngineV3WeakLinkTarget: NSObject {
    private weak var target: AnyObject?
    private let action: (AnyObject, CADisplayLink) -> Void

    init<T: AnyObject>(_ target: T, _ action: @escaping @MainActor (T, CADisplayLink) -> Void) {
        self.target = target
        self.action = { t, l in action(t as! T, l) } // swiftlint:disable:this force_cast
    }

    @objc func tick(_ l: CADisplayLink) {
        guard let target else { l.invalidate(); return }
        action(target, l)
    }
}
