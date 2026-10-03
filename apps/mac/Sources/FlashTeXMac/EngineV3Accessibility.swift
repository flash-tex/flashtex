import AppKit
import FlashTeXAccessibility
import FlashTeXDisplayListV3
import FlashTeXPreviewV3

// VoiceOver over the engine-v3 preview (DESIGN §10 app parity, gaps C18 and
// C19). Each page was an image labelled "Page N": a VoiceOver user could not
// read the document. A page is now what the v2 pane's page is
// (PreviewV2Accessibility.swift): a landmark whose value is its text, one
// line element per line with "Go to source", and the pane's Pages rotor
// lists every page, including those the pane has not built.
//
// The display list carries glyph codes, not text. A glyph's text comes from
// its name in the font program (the name the encoding gave the code), mapped
// by pdfTeX's own glyph-name table, `glyphtounicode.tex` (what
// `\pdfgentounicode` writes into a PDF), read from the TeX Live the engine
// already uses (D12); the Adobe Glyph List's rules (`uniXXXX`, `uXXXX`,
// suffixes, `_` ligatures) cover the rest. Lines are the v2 pane's grouping
// (`V2PageText.lines(words:rules:)`) over the glyphs, so both panes read a
// page alike.

/// Glyph name → text. Pure apart from locating the table once.
enum EngineV3GlyphText {
    /// `\pdfglyphtounicode{name}{0041 0301}` lines → name → string.
    static func parse(_ tex: String) -> [String: String] {
        var out: [String: String] = [:]
        out.reserveCapacity(6000)
        for line in tex.split(separator: "\n") {
            guard line.hasPrefix("\\pdfglyphtounicode{"), let close = line.firstIndex(of: "}") else { continue }
            let name = line[line.index(line.startIndex, offsetBy: 19) ..< close]
            let rest = line[line.index(after: close)...]
            guard rest.first == "{", let end = rest.firstIndex(of: "}") else { continue }
            let hex = rest[rest.index(after: rest.startIndex) ..< end]
            var s = ""
            for h in hex.split(separator: " ") {
                guard let v = UInt32(h, radix: 16), let u = Unicode.Scalar(v) else { s = ""; break }
                s.unicodeScalars.append(u)
            }
            if !s.isEmpty { out[String(name)] = s }
        }
        return out
    }

    /// pdfTeX's table from the user's TeX Live; empty when none is found
    /// (the AGL rules and plain letters still apply).
    static let table: [String: String] = {
        guard let url = locate("glyphtounicode.tex"), let tex = try? String(contentsOf: url, encoding: .utf8) else { return [:] }
        return parse(tex)
    }()

    /// Loads the table off the main thread (the session's start), so the
    /// first VoiceOver read of a page does not wait for `kpsewhich`.
    static func warmUp() { DispatchQueue.global(qos: .utility).async { _ = table } }

    /// `kpsewhich name` over PATH, MacTeX's texbin and TeX Live's own bins.
    static func locate(_ name: String) -> URL? {
        var dirs = (ProcessInfo.processInfo.environment["PATH"] ?? "").split(separator: ":").map(String.init)
        dirs.append("/Library/TeX/texbin")
        if let years = try? FileManager.default.contentsOfDirectory(atPath: "/usr/local/texlive") {
            for y in years.sorted().reversed() {
                let bin = "/usr/local/texlive/\(y)/bin"
                for arch in (try? FileManager.default.contentsOfDirectory(atPath: bin)) ?? [] { dirs.append("\(bin)/\(arch)") }
            }
        }
        for d in dirs {
            let k = URL(fileURLWithPath: d).appendingPathComponent("kpsewhich")
            guard FileManager.default.isExecutableFile(atPath: k.path) else { continue }
            let p = Process()
            p.executableURL = k
            p.arguments = [name]
            let out = Pipe()
            p.standardOutput = out
            p.standardError = FileHandle.nullDevice
            guard (try? p.run()) != nil else { continue }
            p.waitUntilExit()
            let path = String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
            if p.terminationStatus == 0, !path.isEmpty { return URL(fileURLWithPath: path) }
        }
        return nil
    }

    /// Names every table agrees on, for when TeX Live's is not found.
    static let basic: [String: String] = {
        var m: [String: String] = [
            "space": " ", "exclam": "!", "quotedbl": "\"", "numbersign": "#", "dollar": "$", "percent": "%",
            "ampersand": "&", "quoteright": "\u{2019}", "quoteleft": "\u{2018}", "quotesingle": "'", "parenleft": "(",
            "parenright": ")", "asterisk": "*", "plus": "+", "comma": ",", "hyphen": "-", "period": ".", "slash": "/",
            "colon": ":", "semicolon": ";", "less": "<", "equal": "=", "greater": ">", "question": "?", "at": "@",
            "bracketleft": "[", "backslash": "\\", "bracketright": "]", "braceleft": "{", "bar": "|", "braceright": "}",
            "endash": "\u{2013}", "emdash": "\u{2014}", "quotedblleft": "\u{201C}", "quotedblright": "\u{201D}",
            "ff": "ff", "fi": "fi", "fl": "fl", "ffi": "ffi", "ffl": "ffl", "dotlessi": "\u{0131}",
        ]
        for (i, n) in ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"].enumerated() { m[n] = String(i) }
        return m
    }()

    /// The text of glyph `name` (AGL rules after the table), nil when unknown.
    static func text(forName name: String, table: [String: String] = table) -> String? {
        if let s = table[name] ?? basic[name] { return s }
        // A suffix (`a.sc`, `one.oldstyle`) does not change the text.
        if let dot = name.firstIndex(of: "."), dot != name.startIndex {
            return text(forName: String(name[..<dot]), table: table)
        }
        // A ligature of components (`f_f_i`).
        if name.contains("_") {
            let parts = name.split(separator: "_").map { text(forName: String($0), table: table) }
            return parts.contains(where: { $0 == nil }) ? nil : parts.compactMap { $0 }.joined()
        }
        if name.hasPrefix("uni"), name.count >= 7, (name.count - 3) % 4 == 0 {
            var s = "", i = name.index(name.startIndex, offsetBy: 3)
            while i < name.endIndex {
                let j = name.index(i, offsetBy: 4)
                guard let v = UInt32(name[i ..< j], radix: 16), let u = Unicode.Scalar(v) else { return nil }
                s.unicodeScalars.append(u)
                i = j
            }
            return s
        }
        if name.hasPrefix("u"), (5 ... 7).contains(name.count), let v = UInt32(name.dropFirst(), radix: 16), let u = Unicode.Scalar(v) {
            return String(Character(u))
        }
        // A single letter or digit names itself (`a`, `Z`).
        if name.count == 1, let c = name.unicodeScalars.first, c.isASCII, CharacterSet.alphanumerics.contains(c) { return name }
        return nil
    }

    /// The outline program's glyph name for `code` in `font` (nil for Type 3).
    static func name(font: DL3RenderFont, code: UInt16) -> String? {
        guard Int(code) < 256, let cg = font.cgFont else { return nil }
        let g = font.glyphs[Int(code)]
        guard g != 0 else { return nil }
        return cg.name(for: g) as String?
    }

    /// The text of `code` in `font`: the outline program's glyph name for it;
    /// a Type 3 (bitmap) font has no names, so only its letters and digits,
    /// which every TeX text encoding puts at their ASCII codes.
    static func text(font: DL3RenderFont, code: UInt16) -> String? {
        guard Int(code) < 256 else { return nil }
        if font.cgFont != nil { return name(font: font, code: code).flatMap { text(forName: $0) } }
        guard let u = Unicode.Scalar(UInt32(code)), u.isASCII, CharacterSet.alphanumerics.contains(u) else { return nil }
        return String(Character(u))
    }

    /// A spacing accent glyph TeX's `\accent` puts over (or under) a letter
    /// in a 7-bit encoding (OT1: `\'e` is the `acute` glyph and the `e`
    /// glyph): its combining mark.
    static let combiningAccents: [String: Character] = [
        "acute": "\u{0301}", "grave": "\u{0300}", "circumflex": "\u{0302}", "dieresis": "\u{0308}", "tilde": "\u{0303}",
        "macron": "\u{0304}", "breve": "\u{0306}", "dotaccent": "\u{0307}", "ring": "\u{030A}", "caron": "\u{030C}",
        "cedilla": "\u{0327}", "ogonek": "\u{0328}", "hungarumlaut": "\u{030B}",
    ]
}

/// The text of one engine-v3 page as VoiceOver reads it.
enum EngineV3PageText {
    /// One glyph of a page, with its name and text (pure input of `words(_:)`).
    struct Glyph {
        var index: Int
        var name: String?
        var text: String
        /// The advance cell (a 1 em band on the baseline) and the ink, page points, y down.
        var cell: CGRect
        var ink: CGRect
        var baseline: Double
        var em: Double { max(cell.height, 0.1) }
    }

    static func glyphs(_ prepared: DL3PreparedPage, index: DL3SourceIndex) -> [Glyph] {
        var out: [Glyph] = []
        out.reserveCapacity(index.glyphs.count)
        for (i, g) in index.glyphs.enumerated() {
            guard let font = prepared.fonts[g.font], let text = EngineV3GlyphText.text(font: font, code: g.code), !text.isEmpty,
                  text.unicodeScalars.contains(where: { !CharacterSet.whitespaces.contains($0) }) else { continue }
            out.append(Glyph(index: i, name: EngineV3GlyphText.name(font: font, code: g.code), text: text,
                             cell: g.cell, ink: g.ink, baseline: g.origin.y))
        }
        return out
    }

    /// One word per glyph (its advance cell, its baseline); the v2 grouping
    /// joins them, with a space where TeX left a gap. A spacing accent over
    /// or under a letter (OT1's `\'e`, `\'E` raised for the capital,
    /// `\c{c}` below) becomes that letter's combining mark, whatever its
    /// vertical offset, and the letter is normalised to NFC ("é"), so it
    /// reads as the letter a T1 font's precomposed glyph gives.
    static func words(_ glyphs: [Glyph]) -> [V2PageText.Word] {
        var marks: [Int: [Character]] = [:] // base position → marks
        var dropped = Set<Int>()
        for (k, a) in glyphs.enumerated() {
            guard let name = a.name, let mark = EngineV3GlyphText.combiningAccents[name] else { continue }
            let box = a.ink.isEmpty ? a.cell : a.ink
            // The letter it sits on: a neighbour in painting order whose cell
            // spans the accent's centre, within reach above or below.
            var best: (k: Int, d: Double)?
            for j in max(0, k - 4) ... min(glyphs.count - 1, k + 4) where j != k {
                let b = glyphs[j]
                guard b.name.flatMap({ EngineV3GlyphText.combiningAccents[$0] }) == nil,
                      b.text.unicodeScalars.first.map({ CharacterSet.letters.contains($0) }) == true,
                      box.midX >= b.cell.minX - 0.1 * b.em, box.midX <= b.cell.maxX + 0.1 * b.em,
                      abs(b.baseline - a.baseline) <= 1.5 * b.em else { continue }
                let d = Double(abs(b.cell.midX - box.midX))
                if best == nil || d < best!.d { best = (j, d) }
            }
            guard let base = best?.k else { continue }
            marks[base, default: []].append(mark)
            dropped.insert(k)
        }
        var out: [V2PageText.Word] = []
        out.reserveCapacity(glyphs.count)
        for (k, g) in glyphs.enumerated() where !dropped.contains(k) {
            var text = g.text
            if let m = marks[k] {
                // `\"\i`: the dotless letter TeX puts under an accent is the
                // plain one in the text (ï, as a T1 font's idieresis reads).
                if text == "\u{0131}" { text = "i" } else if text == "\u{0237}" { text = "j" }
                text = (text + String(m)).precomposedStringWithCanonicalMapping
            }
            out.append(V2PageText.Word(itemIndex: g.index, text: text, rect: g.cell, fontSizePt: g.em, baseline: g.baseline,
                                       sources: [], syntheticReason: nil))
        }
        return out
    }

    /// The page's rules in page points (fraction and radical bars).
    static func rules(_ page: DL3Page) -> [CGRect] {
        let k = EngineV3Links.spPerBP
        return page.items.compactMap { item in
            guard case .rule(_, let x, let y, let w, let h) = item else { return nil }
            // RULE: left, top, width, height (page space, sp; spec §4.3).
            return CGRect(x: Double(x) / k, y: Double(y) / k, width: Double(w) / k, height: Double(h) / k)
        }
    }

    static func lines(_ prepared: DL3PreparedPage, index: DL3SourceIndex) -> [V2PageText.Line] {
        V2PageText.lines(words: words(glyphs(prepared, index: index)), rules: rules(prepared.page))
    }
}

// MARK: - The page as VoiceOver sees it

extension EngineV3PageView: PreviewPageAXTarget, NSAccessibilityElementLoading {
    var previewPageNumber: Int? { index + 1 }
    var previewPageIsLoaded: Bool { owner?.session?.pages[index] != nil }

    /// The page's lines (cached by the page's content hash).
    var axLines: [V2PageText.Line] {
        guard let session = owner?.session, let prepared = session.pages[index], let ix = session.sourceIndex(page: index) else { return [] }
        if let c = axCache, c.hash == prepared.page.hash { return c.lines }
        let lines = EngineV3PageText.lines(prepared, index: ix)
        axCache = (prepared.page.hash, lines, nil)
        return lines
    }

    override func isAccessibilityElement() -> Bool { true }
    override func accessibilityRole() -> NSAccessibility.Role? { .group }
    override func accessibilitySubrole() -> NSAccessibility.Subrole? { PreviewAccessibility.landmarkSubrole }
    override func accessibilityRoleDescription() -> String? { PreviewAccessibility.pageRoleDescription }
    override func accessibilityLabel() -> String? {
        let total = owner?.session?.pageCount ?? 0
        guard previewPageIsLoaded else { return V2PageText.elidedPageLabel(number: index + 1, totalPages: total) }
        return V2PageText.pageLabel(number: index + 1, totalPages: total, lineCount: axLines.count) + (axStale ? ", stale" : "")
    }
    /// The page's text, one line per line ("read page" from the landmark).
    override func accessibilityValue() -> Any? { axLines.map(\.text).joined(separator: "\n") }
    override func accessibilityChildren() -> [Any]? { axElements() }
    override func accessibilityChildrenInNavigationOrder() -> [NSAccessibilityElementProtocol]? {
        PreviewAXElement.navigationOrder(axElements())
    }
    override func accessibilityCustomRotors() -> [NSAccessibilityCustomRotor] { owner?.pagesRotor.rotors ?? [] }
    func accessibilityElement(withToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        owner?.pagesRotor.load(token)
    }

    /// One static-text element per line, built when a client first asks and
    /// kept while the page's content is the same (VoiceOver's cursor
    /// survives a zoom; a new page content rebuilds them).
    func axElements() -> [PreviewAXElement] {
        let lines = axLines
        if let els = axCache?.elements { return els }
        let els: [PreviewAXElement] = lines.map { line in
            let ax = PreviewAXElement.make(role: .staticText, label: "", viewFrame: .zero, pageView: self, parent: self)
            ax.setAccessibilityRoleDescription(PreviewAccessibility.lineRoleDescription)
            configure(ax, line: line)
            return ax
        }
        axCache?.elements = els
        return els
    }

    /// The page view was resized (zoom, pane width): lines keep their elements, which move.
    func axRescaled() {
        guard let els = axCache?.elements else { return }
        for (ax, line) in zip(els, axLines) { ax.setViewFrame(axFrame(line)) }
    }

    private func axFrame(_ line: V2PageText.Line) -> CGRect {
        let w = owner?.session?.pages[index]?.widthPt ?? 0
        let s = w > 0 ? bounds.width / CGFloat(w) : 1
        return CGRect(x: line.rect.minX * s, y: line.rect.minY * s, width: max(1, line.rect.width * s), height: max(1, line.rect.height * s))
    }

    private func configure(_ ax: PreviewAXElement, line: V2PageText.Line) {
        ax.setViewFrame(axFrame(line))
        let n = index + 1
        ax.setAccessibilityLabel(V2PageText.lineLabel(page: n, line: line))
        ax.setAccessibilityValue(line.text)
        ax.setAccessibilityHelp("Page \(n), line \(line.number)")
        // The click a mouse user makes on the line's first glyph: reverse search.
        guard let first = line.words.first else { ax.setAccessibilityCustomActions([]); return }
        let i = index, at = CGPoint(x: first.rect.midX, y: first.rect.midY)
        ax.setAccessibilityCustomActions([
            NSAccessibilityCustomAction(name: PreviewAccessibility.goToSourceAction) { [weak self] in
                guard let session = self?.owner?.session, let src = session.source(page: i, at: at) else { return false }
                session.model?.navigateEngineV3(path: src.path, line: src.line, col: src.col)
                return true
            },
        ])
    }

    /// The page's content changed (a compile sent it): a client that read
    /// the old lines is told to read again.
    func axContentChanged() {
        if let c = axCache, c.hash != owner?.session?.pages[index]?.page.hash {
            let hadElements = c.elements != nil
            axCache = nil
            if hadElements { NSAccessibility.post(element: self, notification: .layoutChanged) }
        }
        prepareAXLinesOffMain()
    }

    /// With VoiceOver on, a page's lines are worked out off the main thread
    /// when it arrives, so reading it does not do that work on main (a
    /// client that asks first still gets them at once, on main).
    func prepareAXLinesOffMain() {
        guard NSWorkspace.shared.isVoiceOverEnabled, axCache == nil, let prepared = owner?.session?.pages[index] else { return }
        let hash = prepared.page.hash
        let box = EngineV3AXBox(prepared)
        DispatchQueue.global(qos: .utility).async { [weak self] in
            let lines = EngineV3AXBox(EngineV3PageText.lines(box.value, index: DL3SourceIndex(box.value)))
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    guard let self, self.axCache == nil, self.owner?.session?.pages[self.index]?.page.hash == hash else { return }
                    self.axCache = (hash, lines.value, nil)
                }
            }
        }
    }
}

/// Carries a value to the page-text queue and back (the values are immutable).
struct EngineV3AXBox<T>: @unchecked Sendable {
    let value: T
    init(_ value: T) { self.value = value }
}

// MARK: - The Pages rotor

/// VoiceOver's "Pages" rotor over the engine-v3 pane: every page of the
/// document (the pane builds views only near the visible area), with the
/// v2 rotor's labels and search rules (`PreviewPagesRotor.label`,
/// `.resolve`, `.start`). Choosing a page the pane has not built scrolls to
/// it and hands VoiceOver its view.
@MainActor
final class EngineV3PagesRotor: NSObject, NSAccessibilityCustomRotorItemSearchDelegate, NSAccessibilityElementLoading {
    weak var pane: EngineV3PagesView?
    private(set) var rotors: [NSAccessibilityCustomRotor] = []
    /// Evidence for tests: pages loaded through the rotor, in order.
    private(set) var loads: [Int] = []

    init(pane: EngineV3PagesView) {
        self.pane = pane
        super.init()
        let rotor = NSAccessibilityCustomRotor(label: PreviewPagesRotor.rotorLabel, itemSearchDelegate: self)
        rotor.itemLoadingDelegate = self
        rotors = [rotor]
    }

    var items: [PreviewPagesRotor.Item] {
        let n = pane?.session?.pageCount ?? 0
        return (0 ..< n).map { PreviewPagesRotor.Item(number: $0 + 1, label: PreviewPagesRotor.label(number: $0 + 1, totalPages: n, elided: false)) }
    }

    func result(for item: PreviewPagesRotor.Item) -> NSAccessibilityCustomRotor.ItemResult {
        let r: NSAccessibilityCustomRotor.ItemResult
        if let v = pane?.heldPageView(item.number - 1), v.previewPageIsLoaded {
            r = NSAccessibilityCustomRotor.ItemResult(targetElement: v)
        } else {
            r = NSAccessibilityCustomRotor.ItemResult(itemLoadingToken: NSNumber(value: item.number), customLabel: item.label)
        }
        r.customLabel = item.label
        return r
    }

    /// The reader chose a page the pane has not built: scroll its top into
    /// view (no animation, no spoken landing: VoiceOver reads what it gets)
    /// and hand back its view.
    func load(_ token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        guard let number = (token as? NSNumber)?.intValue, let pane, number >= 1, number <= (pane.session?.pageCount ?? 0) else { return nil }
        loads.append(number)
        pane.scrollToPage(number - 1, announce: false)
        return pane.heldPageView(number - 1)
    }

    func search(_ p: NSAccessibilityCustomRotor.SearchParameters) -> NSAccessibilityCustomRotor.ItemResult? {
        let found = PreviewPagesRotor.resolve(items: items, start: PreviewPagesRotor.start(of: p),
                                              forward: p.searchDirection == .next, filter: p.filterString)
        return found.map(result(for:))
    }

    nonisolated func accessibilityElement(withToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        MainActor.assumeIsolated { load(token) }
    }

    nonisolated func rotor(_ rotor: NSAccessibilityCustomRotor,
                           resultFor searchParameters: NSAccessibilityCustomRotor.SearchParameters) -> NSAccessibilityCustomRotor.ItemResult? {
        MainActor.assumeIsolated { search(searchParameters) }
    }
}

extension EngineV3PagesView: PreviewPagesRotorSource {
    var previewPagesRotors: [NSAccessibilityCustomRotor] { pagesRotor.rotors }
    func previewPageElement(forToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? { pagesRotor.load(token) }
    func previewPageDidAppear(_ view: NSView) {}
    override func accessibilityCustomRotors() -> [NSAccessibilityCustomRotor] { pagesRotor.rotors }
}
