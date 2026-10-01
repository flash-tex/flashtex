import Foundation

/// A `type3` FONT's program (protocol §5.1.1, lane P3-FONTS-2): the glyph
/// bitmaps pdfTeX writes as a Type 3 font's image masks, bit for bit.
///
///     u8[4] "T3B1"; u32 n;
///     n × { u8 code; i32 llx, lly; u32 width, height; u8[height × ⌈width/8⌉] rows }
///
/// Rows top first, most significant bit first, 1 = ink. The mask fills
/// (llx, lly)–(llx+width, lly+height) of glyph space (the bitmap's pixel
/// grid, y up); `font_matrix` maps glyph space to text space.
public struct DL3Type3Glyph: Equatable, Sendable {
    public var code: UInt8
    public var llx: Int32, lly: Int32
    public var width: UInt32, height: UInt32
    public var rows: [UInt8]
    public var bytesPerRow: Int { Int((width + 7) / 8) }
}

public enum DL3Type3 {
    public static func decode(_ program: [UInt8]) throws -> [DL3Type3Glyph] {
        guard program.count >= 8, Array(program[0 ..< 4]) == Array("T3B1".utf8) else { throw DL3Error("type3 program without T3B1 magic") }
        var r = DL3Reader(program, from: 4)
        let n = try r.count(17)
        var out: [DL3Type3Glyph] = []
        out.reserveCapacity(n)
        for _ in 0 ..< n {
            let code = try r.u8()
            let llx = try r.i32(), lly = try r.i32()
            let w = try r.u32(), h = try r.u32()
            let (size, overflow) = Int((w + 7) / 8).multipliedReportingOverflow(by: Int(h))
            guard !overflow, size <= r.left else { throw DL3Error("type3 glyph \(code): \(w)×\(h) exceeds the program") }
            out.append(DL3Type3Glyph(code: code, llx: llx, lly: lly, width: w, height: h, rows: Array(try r.take(size))))
        }
        return out
    }
}

extension DL3Font {
    /// `type3`: the PK resolution.
    public var dpi: Int? { info["dpi"]?.int.map(Int.init) }
    /// Subfont entries: the character code of each of the 256 codes (-1: none).
    public var subfont: [Int]? { info["subfont"]?.array.map { $0.map { Int($0.int ?? -1) } } }
    /// Subfont entries: the `[platform, encoding]` of the cmap subtable.
    public var cmap: [Int]? { info["cmap"]?.array?.compactMap { $0.int.map(Int.init) } }
    /// Why the glyphs cannot be drawn from the display list (the pages using
    /// the font are INCOMPLETE).
    public var problem: String? { info["problem"]?.string }
}
