import Foundation

extension TeXpand {
    /// `scope_at(offset)` (PLAN §8): the frame stack at a UTF-16 offset,
    /// from a scan of the document that tracks the preamble,
    /// `\begin`/`\end` environments, `$`/`$$`/`\(`/`\[` math, verbatim
    /// environments, `\verb` and `%` comments — the lexical rules of
    /// `LaTeXScan` and the syntax highlighter.
    ///
    /// Incremental: the state at line starts is checkpointed every
    /// `checkpointSpacing` units, so a query scans at most that far plus
    /// the distance from the last valid checkpoint. The host reports every
    /// edit (`noteEdit`), which drops the checkpoints after it. Not
    /// thread-safe; one per text view.
    public final class ScopeProvider {
        struct Checkpoint {
            var offset: Int
            var frames: [ScopeStack.Frame]
        }

        public static let checkpointSpacing = 2048
        var checkpoints: [Checkpoint] = [Checkpoint(offset: 0, frames: [ScopeStack.Frame(.document)])]

        public init() {}

        /// Forget everything (a new document).
        public func reset() {
            checkpoints = [Checkpoint(offset: 0, frames: [ScopeStack.Frame(.document)])]
        }

        /// An edit replaced `range` (pre-edit coordinates). The state at an
        /// offset depends only on the text before it, so checkpoints at or
        /// before the edit stay valid.
        public func noteEdit(range: NSRange, replacementLength: Int) {
            checkpoints.removeAll { $0.offset > range.location }
            if checkpoints.isEmpty { reset() }
        }

        /// The scope stack at `offset`: what a character typed there is in.
        public func scope(at offset: Int, in text: NSString) -> ScopeStack {
            let target = max(0, min(offset, text.length))
            let cp = checkpoints.last { $0.offset <= target } ?? checkpoints[0]
            var scanner = Scanner(text: text, from: cp.offset, to: target, frames: cp.frames)
            scanner.run { offset, frames in
                if offset - (checkpoints.last?.offset ?? 0) >= Self.checkpointSpacing, offset > (checkpoints.last?.offset ?? 0) {
                    checkpoints.append(Checkpoint(offset: offset, frames: frames))
                }
            }
            var frames = scanner.frames
            if let extra = scanner.transient { frames.append(extra) }
            return ScopeStack(frames)
        }

        /// The scan: `frames` at line starts are consistent (no token
        /// spans a line, except verbatim bodies, which are frames).
        struct Scanner {
            let buf: [unichar]
            let base: Int
            var frames: [ScopeStack.Frame]
            /// A state that ends before the next line (`%` comment, `\verb`),
            /// reported only for the query offset.
            var transient: ScopeStack.Frame?

            init(text: NSString, from: Int, to: Int, frames: [ScopeStack.Frame]) {
                var b = [unichar](repeating: 0, count: to - from)
                if to > from { text.getCharacters(&b, range: NSRange(location: from, length: to - from)) }
                buf = b; base = from; self.frames = frames
            }

            static func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }

            var verbatimEnvironment: String? {
                guard let name = frames.last?.environmentName else { return nil }
                return Scanner.isVerbatim(name) ? name : nil
            }

            static func isVerbatim(_ name: String) -> Bool {
                SyntaxHighlighter.verbatimEnvironments.contains(name) || name == "comment"
            }

            /// Scans the whole buffer; `lineStart(offset, frames)` at each line start.
            mutating func run(lineStart: (Int, [ScopeStack.Frame]) -> Void) {
                let n = buf.count
                var i = 0
                while i < n {
                    if let env = verbatimEnvironment {
                        // Inside a verbatim body: only its `\end{env}` matters.
                        let closer = Array("\\end{\(env)}".utf16)
                        guard let at = find(closer, from: i) else { return }
                        i = at + closer.count
                        frames.removeLast()
                        continue
                    }
                    let c = buf[i]
                    switch c {
                    case 0x0A: // newline
                        i += 1
                        lineStart(base + i, frames)
                    case 0x25: // % comment to the end of the line
                        var j = i
                        while j < n, buf[j] != 0x0A { j += 1 }
                        if j == n { transient = ScopeStack.Frame(.comment, start: base + i); return }
                        i = j
                    case 0x24: // $ or $$
                        if i + 1 < n, buf[i + 1] == 0x24 {
                            toggle(.displayMath, at: i)
                            i += 2
                        } else {
                            toggle(.inlineMath, at: i)
                            i += 1
                        }
                    case 0x5C: // backslash
                        i = controlSequence(at: i)
                    default:
                        i += 1
                    }
                }
            }

            mutating func toggle(_ kind: ScopeStack.Kind, at i: Int) {
                if frames.last?.kind == kind { frames.removeLast() } else { frames.append(ScopeStack.Frame(kind, start: base + i, bodyStart: base + i + (kind == .displayMath ? 2 : 1))) }
            }

            func find(_ needle: [unichar], from: Int) -> Int? {
                guard needle.count <= buf.count else { return nil }
                var k = from
                while k + needle.count <= buf.count {
                    if buf[k] == needle[0], Array(buf[k..<(k + needle.count)]) == needle { return k }
                    k += 1
                }
                return nil
            }

            /// Handles `\x` at `i`; returns where scanning resumes.
            mutating func controlSequence(at i: Int) -> Int {
                let n = buf.count
                guard i + 1 < n else { return n }
                let next = buf[i + 1]
                if !Scanner.isLetter(next) {
                    switch next {
                    case 0x28: frames.append(ScopeStack.Frame(.inlineMath, start: base + i, bodyStart: base + i + 2)) // \(
                    case 0x5B: frames.append(ScopeStack.Frame(.displayMath, start: base + i, bodyStart: base + i + 2)) // \[
                    case 0x29: if frames.last?.kind == .inlineMath { frames.removeLast() } // \)
                    case 0x5D: if frames.last?.kind == .displayMath { frames.removeLast() } // \]
                    default: break
                    }
                    return i + 2
                }
                var j = i + 1
                while j < n, Scanner.isLetter(buf[j]) { j += 1 }
                let name = String(decoding: buf[(i + 1)..<j], as: UTF16.self)
                switch name {
                case "verb", "lstinline":
                    var k = j
                    if k < n, buf[k] == 0x2A { k += 1 } // \verb*
                    if name == "lstinline", k < n, buf[k] == 0x5B { // \lstinline[opts]
                        while k < n, buf[k] != 0x5D, buf[k] != 0x0A { k += 1 }
                        k += 1
                    }
                    guard k < n else { transient = ScopeStack.Frame(.verbatim, start: base + i); return n }
                    var delim = buf[k]
                    if delim == 0x7B { delim = 0x7D } // \lstinline{…}
                    var m = k + 1
                    while m < n, buf[m] != delim, buf[m] != 0x0A { m += 1 }
                    if m >= n { transient = ScopeStack.Frame(.verbatim, start: base + i); return n }
                    return m + 1
                case "documentclass":
                    if frames.count == 1, frames[0].kind == .document, frames[0].start == nil {
                        frames = [ScopeStack.Frame(.preamble, start: base + i)]
                    }
                    return j
                case "begin", "end":
                    var k = j
                    while k < n, buf[k] == 0x20 { k += 1 }
                    guard k < n, buf[k] == 0x7B else { return j }
                    var m = k + 1
                    while m < n, buf[m] != 0x7D, buf[m] != 0x0A { m += 1 }
                    // The query offset inside `\begin{…}` is not in it yet.
                    guard m < n, buf[m] == 0x7D else { return m >= n ? n : m }
                    let env = String(decoding: buf[(k + 1)..<m], as: UTF16.self)
                    if name == "begin" {
                        if env == "document" {
                            frames = [ScopeStack.Frame(.document, start: base + i, bodyStart: base + m + 1)]
                        } else {
                            frames.append(ScopeStack.Frame(.environment(env), start: base + i, bodyStart: base + m + 1))
                        }
                    } else if let at = frames.lastIndex(where: { $0.environmentName == env }) {
                        frames.removeSubrange(at...)
                    }
                    return m + 1
                default:
                    return j
                }
            }
        }
    }
}
