import Foundation
import TOMLDecoder

// The bridge to TOMLDecoder (dduan/TOMLDecoder): parse, then convert its
// lazily-parsed tables into TeXpand's ordered `TOMLTable`, with each table's
// line recovered from its `[header]` so diagnostics can name it. This file
// imports the library alone: its module and a type share the name
// `TOMLDecoder`, and its `TOMLTable` would shadow ours elsewhere.

private typealias LibTable = TOMLTable
private typealias LibArray = TOMLArray

enum TOMLLibrary {
    static func parse(_ text: String) throws -> TeXpand.TOMLTable {
        let root: LibTable
        do {
            root = try LibTable(source: text)
            _ = try Dictionary(root) // values parse lazily: surface every error now
        } catch {
            var e = mapError(error)
            if e.line == nil { e.line = locate(e.message, in: text) }
            throw e
        }
        var lines = HeaderLines(text)
        return convert(root, path: [], line: 1, lines: &lines)
    }

    /// Some errors (an illegal escape, a stray control character) come
    /// without a line: the first prefix of the document, in whole lines,
    /// that fails with the same message ends on it. Only on error, and
    /// config files are short.
    static func locate(_ message: String, in text: String) -> Int? {
        let lines = text.components(separatedBy: "\n")
        guard lines.count <= 5000 else { return nil }
        var prefix = ""
        for (k, line) in lines.enumerated() {
            prefix += (k == 0 ? "" : "\n") + line
            do {
                _ = try Dictionary(try LibTable(source: prefix))
            } catch {
                if mapError(error).message == message { return k + 1 }
            }
        }
        return nil
    }

    /// `(Line 6) Syntax error: unterminated quote.` → line 6, the message.
    static func mapError(_ error: any Error) -> TeXpand.TOMLError {
        let text = "\(error)"
        if let r = text.range(of: #"^\(Line (\d+)\) "#, options: .regularExpression) {
            let number = text[r].dropFirst("(Line ".count).prefix { $0.isNumber }
            var message = String(text[r.upperBound...])
            if message.hasSuffix(".") { message.removeLast() }
            return TeXpand.TOMLError(line: Int(number), message: message)
        }
        return TeXpand.TOMLError(line: nil, message: text.hasSuffix(".") ? String(text.dropLast()) : text)
    }

    private static func convert(_ t: LibTable, path: [String], line: Int, lines: inout HeaderLines) -> TeXpand.TOMLTable {
        var out = TeXpand.TOMLTable(line: line)
        for key in t.keys {
            let sub = path + [key]
            if let table = try? t.table(forKey: key) {
                out.entries.append((key, .table(convert(table, path: sub, line: lines.next(sub, array: false) ?? line, lines: &lines))))
            } else if let array = try? t.array(forKey: key) {
                out.entries.append((key, .array(convert(array, path: sub, line: line, lines: &lines))))
            } else if let v = scalar(t, key) {
                out.entries.append((key, v))
            }
        }
        return out
    }

    private static func convert(_ a: LibArray, path: [String], line: Int, lines: inout HeaderLines) -> [TeXpand.TOMLValue] {
        (0..<a.count).compactMap { i in
            if let table = try? a.table(atIndex: i) {
                return .table(convert(table, path: path, line: lines.next(path, array: true) ?? line, lines: &lines))
            }
            if let inner = try? a.array(atIndex: i) { return .array(convert(inner, path: path, line: line, lines: &lines)) }
            if let s = try? a.string(atIndex: i) { return .string(s) }
            if let b = try? a.bool(atIndex: i) { return .bool(b) }
            if let n = try? a.integer(atIndex: i) { return .integer(Int(n)) }
            if let d = try? a.float(atIndex: i) { return .float(d) }
            if let d = try? a.offsetDateTime(atIndex: i) { return .string(d.description) }
            if let d = try? a.localDateTime(atIndex: i) { return .string(d.description) }
            if let d = try? a.localDate(atIndex: i) { return .string(d.description) }
            if let d = try? a.localTime(atIndex: i) { return .string(d.description) }
            return nil
        }
    }

    private static func scalar(_ t: LibTable, _ key: String) -> TeXpand.TOMLValue? {
        if let s = try? t.string(forKey: key) { return .string(s) }
        if let b = try? t.bool(forKey: key) { return .bool(b) }
        if let n = try? t.integer(forKey: key) { return .integer(Int(n)) }
        if let d = try? t.float(forKey: key) { return .float(d) }
        if let d = try? t.offsetDateTime(forKey: key) { return .string(d.description) }
        if let d = try? t.localDateTime(forKey: key) { return .string(d.description) }
        if let d = try? t.localDate(forKey: key) { return .string(d.description) }
        if let d = try? t.localTime(forKey: key) { return .string(d.description) }
        return nil
    }
}

/// The `[a.b]` and `[[a.b]]` headers of a document, in order, with their
/// lines (outside multi-line strings). Tables are converted in document
/// order, so the n-th table at a path is the n-th header for it.
struct HeaderLines {
    private var queue: [String: [Int]] = [:]

    init(_ text: String) {
        var inMultiline: String?
        for (k, raw) in text.components(separatedBy: "\n").enumerated() {
            var line = Substring(raw)
            // Track `'''` / `"""` bodies, where a `[` starts no header.
            if let delim = inMultiline {
                guard let end = line.range(of: delim) else { continue }
                line = line[end.upperBound...]
                inMultiline = nil
            }
            for delim in ["'''", "\"\"\""] {
                let count = line.components(separatedBy: delim).count - 1
                if count % 2 == 1 { inMultiline = delim }
            }
            let t = line.trimmingCharacters(in: .whitespaces)
            guard t.hasPrefix("["), inMultiline == nil || line.range(of: inMultiline!)!.lowerBound > line.firstIndex(of: "[")! else { continue }
            let isArray = t.hasPrefix("[[")
            let body = t.dropFirst(isArray ? 2 : 1)
            guard let close = body.range(of: isArray ? "]]" : "]") else { continue }
            let path = Self.split(String(body[..<close.lowerBound]))
            queue[Self.key(path, isArray), default: []].append(k + 1)
        }
    }

    /// The line of the next table at `path` (consumed in document order).
    mutating func next(_ path: [String], array: Bool) -> Int? {
        let key = Self.key(path, array)
        guard var list = queue[key], !list.isEmpty else { return nil }
        let line = list.removeFirst()
        queue[key] = list
        return line
    }

    static func key(_ path: [String], _ array: Bool) -> String { (array ? "[[" : "[") + path.joined(separator: "\u{1}") }

    /// `a."b.c" . d` → [a, b.c, d].
    static func split(_ s: String) -> [String] {
        var out: [String] = []
        var cur = ""
        var quote: Character?
        for c in s {
            if let q = quote {
                if c == q { quote = nil } else { cur.append(c) }
            } else if c == "\"" || c == "'" {
                quote = c
            } else if c == "." {
                out.append(cur.trimmingCharacters(in: .whitespaces)); cur = ""
            } else {
                cur.append(c)
            }
        }
        out.append(cur.trimmingCharacters(in: .whitespaces))
        return out
    }
}
