import Foundation
#if canImport(CryptoKit)
import CryptoKit
#endif

/// The canonical text of decoded frames, line for line what the reference
/// decoder prints (`crates/display-list-v3/src/canonical.rs`,
/// `dl3-dump --canonical`). Two decoders agree on a stream exactly when
/// their canonical texts are equal: every decoded field, floats as their
/// IEEE 754 bit patterns.
public enum DL3Canonical {
    public static func sha256(_ bytes: some Sequence<UInt8>) -> [UInt8] {
        #if canImport(CryptoKit)
        return Array(SHA256.hash(data: Data(bytes)))
        #else
        fatalError("DL3Canonical.sha256 needs CryptoKit or swift-crypto")
        #endif
    }

    static func b(_ x: Double) -> String {
        let s = String(x.bitPattern, radix: 16)
        return String(repeating: "0", count: 16 - s.count) + s
    }

    static func h(_ bytes: some Collection<UInt8>) -> String { bytes.isEmpty ? "-" : DL3Hex.string(bytes) }

    /// The canonical text of a whole file of frames.
    public static func text(ofFile bytes: [UInt8]) throws -> String {
        var o = ""
        for (k, body) in try DL3Frames.split(bytes) { o += try text(kind: k, body: body) }
        return o
    }

    public static func text(kind k: UInt8, body: [UInt8]) throws -> String {
        var o = ""
        switch try DL3Event.decode(kind: k, body: body) {
        case .page(let p): page(&o, "page", p)
        case .form(let p): page(&o, "form", p)
        case .font(let f):
            o += "font \(f.id) \(h(f.key)) \(f.program.count) \(DL3Hex.string(sha256(f.program))) \(f.format ?? "-")\n"
            if let enc = f.encoding {
                o += "enc"
                for name in enc { o += " " + ((name?.isEmpty ?? true) ? "-" : name!) }
                o += "\n"
            }
        case .sources(let s):
            o += "sources\n"
            for (id, path) in s.files { o += "file \(id) \(h(Array(path.utf8)))\n" }
            for (id, file, line) in s.spans { o += "span \(id) \(file) \(line)\n" }
        case .other(let kind, let len):
            o += "other \(kind) \(len)\n"
        default:
            o += "json \(DL3.Kind.name(k)) \(DL3Hex.string(sha256(body)))\n"
        }
        return o
    }

    static func page(_ o: inout String, _ tag: String, _ p: DL3Page) {
        o += "\(tag) \(p.index) \(p.flags) \(p.width) \(p.height)"
        for c in p.counts { o += " \(c)" }
        for v in p.box { o += " " + b(v) }
        o += " \(h(p.hash))\n"
        for m in p.matrices { o += "mat \(b(m.a)) \(b(m.b)) \(b(m.c)) \(b(m.d)) \(b(m.e)) \(b(m.f))\n" }
        for path in p.paths {
            o += "path \(path.paint) \(path.matrix)"
            if let s = path.stroke {
                o += " stroke \(b(s.width)) \(s.cap) \(s.join) \(b(s.miter)) \(s.dash.count)"
                for d in s.dash { o += " " + b(d) }
                o += " " + b(s.phase)
            }
            o += " \(path.segs.count)\n"
            for seg in path.segs {
                switch seg {
                case .move(let x, let y): o += "seg m \(b(x)) \(b(y))\n"
                case .line(let x, let y): o += "seg l \(b(x)) \(b(y))\n"
                case .curve(let a, let c, let d, let e, let f, let g): o += "seg c \(b(a)) \(b(c)) \(b(d)) \(b(e)) \(b(f)) \(b(g))\n"
                case .close: o += "seg h\n"
                }
            }
        }
        for it in p.items {
            switch it {
            case .glyph(let font, let code, let x, let y, let col): o += "g \(font) \(code) \(x) \(y) \(col)\n"
            case .rule(let kind, let x, let y, let w, let hh): o += "r \(kind.rawValue) \(x) \(y) \(w) \(hh)\n"
            case .path(let n): o += "p \(n)\n"
            case .clip(let n): o += "clip \(n)\n"
            case .image(let id, let m): o += "img \(id) \(m)\n"
            case .form(let id, let m): o += "form \(id) \(m)\n"
            case .save: o += "q\n"
            case .restore: o += "Q\n"
            case .fillColor(let c), .strokeColor(let c):
                if case .fillColor = it { o += "fill \(c.count)" } else { o += "stroke \(c.count)" }
                for v in c { o += " " + b(v) }
                o += "\n"
            case .matrix(let n): o += "m \(n)\n"
            case .span(let n): o += "span \(n)\n"
            case .textRender(let m): o += "tr \(m)\n"
            case .unsupported(let n): o += "u \(n)\n"
            }
        }
        for l in p.links {
            o += "link \(l.rect[0]) \(l.rect[1]) \(l.rect[2]) \(l.rect[3]) \(l.span) \(l.kind) \(h(l.file)) \(h(l.data))\n"
        }
        for d in p.dests {
            o += "dest \(d.named ? 1 : 0) \(h(d.name)) \(d.kind) \(d.rect[0]) \(d.rect[1]) \(d.rect[2]) \(d.rect[3]) \(d.zoom)\n"
        }
        for u in p.unsupported { o += "unsupported \(h(Array(u.utf8)))\n" }
    }
}
