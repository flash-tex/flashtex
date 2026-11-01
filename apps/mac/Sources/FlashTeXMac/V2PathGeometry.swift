import CoreGraphics
import FlashTeXProtocol

// path-v0 consumer (TikZ pictures): the display list's `path_fill` /
// `path_stroke` items become CGPaths in PDF space and paint through the one
// `GlyphRunRenderer.draw` routine (preview bitmap and the CoreGraphics PDF
// export alike). Nothing is measured on the consumer side: coordinates are
// the producer's page ticks, the stroke width and dash are ticks → points.

/// A path item prepared for painting: PDF-space geometry (points, origin
/// bottom-left, y up — the space `GlyphRunRenderer.draw` paints in), the
/// paint operation and every clip.
struct V2PreparedPath {
    enum Op {
        case fill(CGPathFillRule)
        case stroke(width: CGFloat, cap: CGLineCap, join: CGLineJoin, miterLimit: CGFloat, dash: [CGFloat], phase: CGFloat)
    }
    var path: CGPath
    var clips: [(path: CGPath, rule: CGPathFillRule)]
    var op: Op
    var paint: RenderingV2.Paint

    init(item: RenderingV2.Path, pageHeight: Double) {
        let q = V2PreparedPage.serialized
        path = V2PathGeometry.cgPath(item.path, pageHeight: pageHeight, quantize: q)
        clips = item.clips.map { (V2PathGeometry.cgPath($0.path, pageHeight: pageHeight, quantize: q), V2PathGeometry.rule($0.fillRule)) }
        paint = item.paint
        switch item.op {
        case .fill(let rule): op = .fill(V2PathGeometry.rule(rule))
        case .stroke(let s):
            let dash: [CGFloat] = (s.dash?.array ?? []).map { (t: Int64) -> CGFloat in CGFloat(q(RenderingV2.points(t))) }
            op = .stroke(width: CGFloat(q(RenderingV2.points(s.width))), cap: V2PathGeometry.cap(s.cap), join: V2PathGeometry.join(s.join),
                         miterLimit: CGFloat(s.miterLimit), dash: dash, phase: CGFloat(q(RenderingV2.points(s.dash?.phase ?? 0))))
        }
    }

    /// Paints inside every clip; `color` is the (possibly dark-inverted) paint.
    func draw(in ctx: CGContext, color: CGColor) {
        ctx.saveGState()
        for clip in clips {
            ctx.beginPath()
            ctx.addPath(clip.path)
            ctx.clip(using: clip.rule)
        }
        ctx.beginPath()
        ctx.addPath(path)
        switch op {
        case .fill(let rule):
            ctx.setFillColor(color)
            ctx.fillPath(using: rule)
        case .stroke(let width, let cap, let join, let miterLimit, let dash, let phase):
            ctx.setStrokeColor(color)
            ctx.setLineWidth(width)
            ctx.setLineCap(cap)
            ctx.setLineJoin(join)
            ctx.setMiterLimit(miterLimit)
            ctx.setLineDash(phase: phase, lengths: dash)
            ctx.strokePath()
        }
        ctx.restoreGState()
    }
}

enum V2PathGeometry {
    static func rule(_ r: RenderingV2.FillRule) -> CGPathFillRule { r == .evenodd ? .evenOdd : .winding }
    static func cap(_ c: RenderingV2.LineCap) -> CGLineCap { switch c { case .butt: return .butt; case .round: return .round; case .square: return .square } }
    static func join(_ j: RenderingV2.LineJoin) -> CGLineJoin { switch j { case .miter: return .miter; case .round: return .round; case .bevel: return .bevel } }

    /// Commands (page ticks, y down) → CGPath in points. With `pageHeight`
    /// the y axis is flipped into PDF space (`y = pageHeight − top`); without
    /// it the path stays y down (hit-testing, where orientation is moot).
    static func cgPath(_ cmds: [RenderingV2.PathCommand], pageHeight: Double? = nil, quantize: (Double) -> Double = { $0 }) -> CGPath {
        let path = CGMutablePath()
        func pt(_ x: Int64, _ y: Int64) -> CGPoint {
            let py = RenderingV2.points(y)
            return CGPoint(x: quantize(RenderingV2.points(x)), y: quantize(pageHeight.map { $0 - py } ?? py))
        }
        var open = false
        for c in cmds {
            switch c {
            case .move(let x, let y): path.move(to: pt(x, y)); open = true
            case .line(let x, let y): guard open else { continue }; path.addLine(to: pt(x, y))
            case .cubic(let x1, let y1, let x2, let y2, let x, let y):
                guard open else { continue }
                path.addCurve(to: pt(x, y), control1: pt(x1, y1), control2: pt(x2, y2))
            case .close: guard open else { continue }; path.closeSubpath(); open = false
            }
        }
        return path
    }

    /// Ink containment for hit-testing: the query (ticks) lies inside the
    /// fill (by its rule) or on the stroked outline (at least 2 pt wide so a
    /// hairline is clickable), and inside every clip. Returns the item's
    /// bounding box in ticks, the `rect` a `V2Geometry.Hit` reports.
    static func hit(_ item: RenderingV2.Path, tickX x: Int64, tickY y: Int64) -> RenderingV2.Rect? {
        let point = CGPoint(x: RenderingV2.points(x), y: RenderingV2.points(y))
        for clip in item.clips where !cgPath(clip.path).contains(point, using: rule(clip.fillRule)) { return nil }
        let path = cgPath(item.path)
        let ink: CGPath
        switch item.op {
        case .fill(let r):
            guard path.contains(point, using: rule(r)) else { return nil }
            ink = path
        case .stroke(let s):
            ink = path.copy(strokingWithWidth: max(RenderingV2.points(s.width), 2), lineCap: cap(s.cap), lineJoin: join(s.join), miterLimit: CGFloat(s.miterLimit))
            guard ink.contains(point) else { return nil }
        }
        let box = ink.boundingBoxOfPath
        guard box.width.isFinite, box.height.isFinite else { return nil }
        let t = Double(RenderingV2.ticksPerPoint)
        return RenderingV2.Rect(x: Int64((box.minX * t).rounded(.down)), top: Int64((box.minY * t).rounded(.down)),
                                width: max(1, Int64((box.width * t).rounded(.up))), height: max(1, Int64((box.height * t).rounded(.up))))
    }
}
