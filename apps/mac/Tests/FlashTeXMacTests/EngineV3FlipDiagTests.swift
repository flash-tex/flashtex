import AppKit
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// DIAGNOSTIC (throwaway): a page view's hosted layer's flip state at each step of being attached.
@MainActor
final class EngineV3FlipDiagTests: XCTestCase {
    final class FlippedHolder: NSView { override var isFlipped: Bool { true } }

    func spin(_ s: Double) { RunLoop.main.run(until: Date().addingTimeInterval(s)) }

    func state(_ tag: String, _ v: EngineV3PageView) {
        let l = v.layer!
        var chain: [String] = []
        var c: CALayer? = l
        while let x = c { chain.append("\(type(of: x))(g=\(x.isGeometryFlipped),f=\(x.contentsAreFlipped()))"); c = x.superlayer }
        print("DIAG-FLIP \(tag): sup=\(l.superlayer != nil) win=\(v.window != nil) superFlipped=\(v.superview?.isFlipped ?? false) superHasLayer=\(v.superview?.layer != nil) delegate=\(l.delegate.map { "\(type(of: $0))" } ?? "nil") shown=\(v.layerShownFlipped) label=\(v.label.frame.minY) chain=\(chain.joined(separator: " < "))")
    }

    func window(_ size: CGSize) -> (NSWindow, NSView) {
        let w = HostedWindowSupport.window(contentRect: CGRect(origin: .zero, size: size), styleMask: [.titled])
        w.isReleasedWhenClosed = false
        let content = NSView(frame: CGRect(origin: .zero, size: size))
        content.wantsLayer = true
        w.contentView = content
        return (w, content)
    }

    func testDiagFlipStates() {
        let size = CGSize(width: 600, height: 800)
        // A: a holder with no layer, outside a window, then into a layer-backed window.
        do {
            let v = EngineV3PageView(frame: CGRect(origin: .zero, size: size))
            state("A0 fresh", v)
            let holder = FlippedHolder(frame: v.frame)
            holder.addSubview(v); v.setLabel(number: 1, dark: false, contentsScale: 2)
            state("A1 in layerless holder", v)
            let (w, content) = window(size)
            content.addSubview(holder)
            state("A2 holder in window, sync", v)
            spin(0.2)
            state("A3 after 0.2 s", v)
            w.contentView = nil
        }
        // B: a layer-backed holder already in the window.
        do {
            let (w, content) = window(size)
            let holder = FlippedHolder(frame: CGRect(origin: .zero, size: size))
            holder.wantsLayer = true
            content.addSubview(holder)
            spin(0.2)
            let v = EngineV3PageView(frame: CGRect(origin: .zero, size: size))
            holder.addSubview(v); v.setLabel(number: 1, dark: false, contentsScale: 2)
            state("B1 added to layer-backed holder, sync", v)
            spin(0.2)
            state("B2 after 0.2 s", v)
            v.removeFromSuperview()
            state("B3 removed", v)
            holder.addSubview(v)
            state("B4 re-added, sync", v)
            spin(0.2)
            state("B5 after 0.2 s", v)
            w.contentView = nil
        }
        // C: the pane's shape: an NSScrollView (wantsLayer) whose document view is flipped.
        do {
            let (w, content) = window(size)
            let scroll = NSScrollView(frame: CGRect(origin: .zero, size: size))
            scroll.wantsLayer = true
            let doc = FlippedHolder(frame: CGRect(origin: .zero, size: CGSize(width: 600, height: 2000)))
            scroll.documentView = doc
            content.addSubview(scroll)
            spin(0.2)
            let v = EngineV3PageView(frame: CGRect(origin: .zero, size: size))
            doc.addSubview(v); v.setLabel(number: 1, dark: false, contentsScale: 2)
            state("C1 added in scroll document, sync", v)
            spin(0.2)
            state("C2 after 0.2 s", v)
            w.contentView = nil
        }
    }
}
