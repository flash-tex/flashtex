import AppKit
import Foundation
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import FlashTeXProtocol

// Forward and reverse search in the engine-v3 preview (SyncTeX-equivalent,
// DESIGN.md §6.1). The display list's SOURCES and SPAN/col items are the
// map (protocol §5.3); DL3SourceIndex (FlashTeXPreviewV3) does the geometry.
// Agreement with pdflatex's own SyncTeX on the parity fixtures is measured by
// FlashTeXPreviewV3Tests.SourceMapOracleTests.

/// A place in the preview: a page (0-based index) and a rect in that page's
/// points, y down from its top-left corner.
struct EngineV3Place: Equatable {
    var page: Int
    var rect: CGRect
}

extension EngineV3Session {
    /// The project-relative path of a file the engine read (the project copy's
    /// path), or nil for a file outside the project (a package).
    func projectPath(ofEngineFile file: String) -> String? {
        guard let copy = projectCopy else { return nil }
        // The engine may name the copy by its real path (/private/var/… for /var/…).
        let real = realpath(copy.path, nil).map { p in defer { free(p) }; return String(cString: p) }
        let roots = [copy.standardizedFileURL.path] + (real.map { [$0] } ?? [])
        var f = file
        if let root = roots.first(where: { f.hasPrefix($0 + "/") }) { f.removeFirst(root.count + 1) } else if f.hasPrefix("/") { return nil }
        while f.hasPrefix("./") { f.removeFirst(2) }
        return f
    }

    func sourceIndex(page i: Int) -> DL3SourceIndex? {
        if let hit = sourceIndexes[i] { return hit }
        guard let p = pages[i] else { return nil }
        let ix = DL3SourceIndex(p)
        sourceIndexes[i] = ix
        return ix
    }

    /// Forward search: where `path` (project-relative) line `line` (1-based),
    /// byte column `col`, is in the preview: the glyph at (or the last before)
    /// that column on the first page that shows the line; without a column
    /// (or when the engine knew none), the line's box there.
    func place(path: String, line: Int, col: Int?) -> EngineV3Place? {
        let spans = sourceMap.spans(line: line) { self.projectPath(ofEngineFile: $0) == path }
        guard !spans.isEmpty else { return nil }
        for i in 0 ..< pageCount {
            guard let ix = sourceIndex(page: i) else { continue }
            let gs = ix.glyphs(of: spans, col: col)
            if let box = DL3SourceIndex.box(gs) { return EngineV3Place(page: i, rect: box) }
        }
        return nil
    }

    /// Forward search from a byte offset of a document.
    func place(path: String, byte: Int, in text: String) -> EngineV3Place? {
        let (line, col) = Self.lineAndColumn(byte: byte, in: text)
        return place(path: path, line: line, col: col)
    }

    /// Reverse search: the source of the glyph under `point` (page points) of page `i`.
    func source(page i: Int, at point: CGPoint) -> (path: String, line: Int, col: Int?)? {
        guard let g = sourceIndex(page: i)?.hit(point), let loc = sourceMap.location(of: g.span),
              let path = projectPath(ofEngineFile: loc.path) else { return nil }
        return (path, loc.line, g.col == 0xFFFF ? nil : Int(g.col))
    }

    /// 1-based line and 0-based byte column of `byte` in `text`.
    static func lineAndColumn(byte: Int, in text: String) -> (Int, Int) {
        var line = 1, start = 0, i = 0
        for b in text.utf8 {
            if i >= byte { break }
            if b == 0x0A { line += 1; start = i + 1 }
            i += 1
        }
        return (line, max(0, byte - start))
    }
}

extension ShellModel {
    /// The caret in the engine-v3 preview (CaretFollow's target): the glyph
    /// at the caret's column on the first page that shows its line.
    func engineV3CaretTarget() -> CaretFollow.Target? {
        guard let byte = caretByte, let place = engineV3.place(path: activePath, byte: byte, in: activeText) else { return nil }
        return CaretFollow.Target(page: place.page, rect: place.rect)
    }

    /// Reverse search result → the editor: open the file if the project has
    /// it but the editor does not yet (`\input`/`\include`), select the
    /// source character (or the line, when the column is unknown).
    func navigateEngineV3(path: String, line: Int, col: Int?) {
        if documents.contains(where: { $0.path == path }) {
            selectEngineV3(path: path, line: line, col: col)
            return
        }
        Task { @MainActor in
            let outcome = await project.openDocument(path)
            switch outcome {
            case .refused(let why): navigationNote = "Cannot open \(path): \(why)"
            default: selectEngineV3(path: path, line: line, col: col)
            }
        }
    }

    private func selectEngineV3(path: String, line: Int, col: Int?) {
        guard let doc = documents.first(where: { $0.path == path }) else { navigationNote = "No open document named \(path)."; return }
        guard let lineRange = EngineV3Session.lineByteRange(doc.text, line: line) else {
            navigationNote = "\(path) has no line \(line) now."
            return
        }
        let start = lineRange.lowerBound + min(col ?? 0, lineRange.count)
        // One character at the column (a caret with the character selected), else the whole line.
        var end = col == nil ? lineRange.upperBound : start
        if col != nil, start < lineRange.upperBound {
            let bytes = Array(doc.text.utf8)
            end = start + 1
            while end < lineRange.upperBound, bytes[end] & 0xC0 == 0x80 { end += 1 }
        }
        guard case .selected(let ns, _) = Navigation.editorRange(start: start, end: end, in: doc.text, path: path) else { return }
        if activePath != path { activePath = path }
        selection = .init(path: path, nsRange: ns, token: (selection?.token ?? 0) + 1)
        caretUTF16 = ns.location
        caretLengthUTF16 = ns.length
        navigationNote = "\(path):\(line)" + (col.map { ":\($0 + 1)" } ?? "")
    }
}
