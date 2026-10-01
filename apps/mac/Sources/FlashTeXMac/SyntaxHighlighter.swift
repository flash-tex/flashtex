import AppKit

// The token model (`SyntaxHighlighter`) lives in the shared FlashTeXEditorCore
// target (Sources/FlashTeXEditorCore/SyntaxHighlighter.swift); this file keeps
// the AppKit half: the theme and the `SyntaxPainter` that applies runs to an
// `NSTextView` as temporary attributes.

// MARK: - theme

/// Semantic colours for the runs, from the selected editor theme
/// (EditorThemes.swift, Settings > Themes). Every colour is a stable dynamic
/// `NSColor` that resolves against the current theme and the drawing
/// appearance, so neither an appearance change nor a theme change needs a
/// repaint — only a redraw. Restraint on purpose in the default theme: plain
/// text keeps the text view's colour, braces and brackets are only slightly
/// dimmed.
struct SyntaxTheme: Sendable {
    typealias Role = EditorColorTheme.Role

    static func dynamic(light: (CGFloat, CGFloat, CGFloat), dark: (CGFloat, CGFloat, CGFloat)) -> NSColor {
        NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            let c = isDark ? dark : light
            return NSColor(srgbRed: c.0 / 255, green: c.1 / 255, blue: c.2 / 255, alpha: 1)
        }
    }

    static var command: NSColor { EditorThemeRuntime.color(.command) }
    static var mathCommand: NSColor { EditorThemeRuntime.color(.mathCommand) }
    static var environment: NSColor { EditorThemeRuntime.color(.environment) }
    static var math: NSColor { EditorThemeRuntime.color(.math) }
    static var mathDelimiter: NSColor { EditorThemeRuntime.color(.mathDelimiter) }
    static var number: NSColor { EditorThemeRuntime.color(.number) }
    static var comment: NSColor { EditorThemeRuntime.color(.comment) }
    static var brace: NSColor { EditorThemeRuntime.color(.brace) }
    static var reference: NSColor { EditorThemeRuntime.color(.reference) }
    static var file: NSColor { EditorThemeRuntime.color(.file) }
    static var definition: NSColor { EditorThemeRuntime.color(.definition) }
    static var error: NSColor { EditorThemeRuntime.color(.error) }
    static var warning: NSColor { EditorThemeRuntime.color(.warning) }
    // Chrome.
    static var background: NSColor { EditorThemeRuntime.color(.background) }
    static var foreground: NSColor { EditorThemeRuntime.color(.foreground) }
    static var gutterBackground: NSColor { EditorThemeRuntime.color(.gutterBackground) }
    static var currentLine: NSColor { EditorThemeRuntime.color(.currentLine) }
    static var selection: NSColor { EditorThemeRuntime.color(.selection) }
    static var caret: NSColor { EditorThemeRuntime.color(.caret) }
    static var bracketMatch: NSColor { EditorThemeRuntime.color(.bracketMatch) }
    static var invisibles: NSColor { EditorThemeRuntime.color(.invisibles) }
    static var gutterText: NSColor { EditorThemeRuntime.color(.gutterText) }
    static var gutterCurrentText: NSColor { EditorThemeRuntime.color(.gutterActiveText) }

    /// The run colour for `kind`: every kind, `verbatim` included, gets its
    /// role's dynamic colour (a role the theme leaves out resolves to the
    /// text colour), so a theme switch never changes which runs are painted.
    static func color(for kind: SyntaxHighlighter.Kind) -> NSColor? {
        EditorThemeRuntime.color(Role(kind: kind))
    }
}

// MARK: - painter

/// Applies `SyntaxHighlighter` runs to a text view as `.foregroundColor`
/// temporary attributes over a window around the visible text (extended on
/// scroll); an edit repaints only the lines the highlighter re-lexed.
/// Nothing is painted while the view has marked text (IME composition): the
/// dirty range is kept and painted at the next flush after the commit,
/// cancel or unmark (GH#780).
@MainActor
final class SyntaxPainter {
    static let key = NSAttributedString.Key.foregroundColor
    static let padding = 4_000

    private(set) var highlighter = SyntaxHighlighter()
    /// Disjoint, sorted ranges whose temporary colours match the highlighter.
    private(set) var painted: [NSRange] = []
    /// Range whose paint is stale (kept while marked text exists).
    private(set) var pendingDirty: NSRange?
    /// Evidence: paint passes, runs painted, thread CPU of the last edit.
    private(set) var paints = 0
    private(set) var runsPainted = 0
    /// Evidence: flushes that found marked text and kept the dirty range
    /// instead of painting (GH#780: stays bounded — nothing re-schedules).
    private(set) var deferredFlushes = 0
    private(set) var lastEditCpuNs: UInt64 = 0
    private(set) var lastFlushCpuNs: UInt64 = 0
    private(set) var lastEditLinesLexed = 0
    var enabled = true { didSet { if !enabled { clear() } } }
    /// What the buffer is lexed as (`SyntaxHighlighter.Language`); a change
    /// swaps the model and re-lexes and repaints the window.
    var language: SyntaxHighlighter.Language {
        get { highlighter.language }
        set {
            guard newValue != highlighter.language else { return }
            highlighter = SyntaxHighlighter(language: newValue)
            reset()
        }
    }
    private weak var textView: NSTextView?
    private var observer: NSObjectProtocol?
    private var flushScheduled = false
    private var resetScheduled = false

    deinit { if let observer { NotificationCenter.default.removeObserver(observer) } }

    /// Starts following `tv`'s storage (every character edit, whatever its
    /// origin) and paints the current window.
    func attach(_ tv: NSTextView) {
        textView = tv
        guard let storage = tv.textStorage else { return }
        observer = NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification,
                                                          object: storage, queue: nil) { [weak self] note in
            MainActor.assumeIsolated {
                guard let self, let storage = note.object as? NSTextStorage, storage.editedMask.contains(.editedCharacters) else { return }
                let edited = storage.editedRange
                let delta = storage.changeInLength
                self.storageEdited(range: NSRange(location: edited.location, length: edited.length - delta), replacementLength: edited.length)
            }
        }
        reset()
    }

    /// The whole text changed (or the view was reset, dropping temporary attributes).
    func reset() {
        guard let tv = textView else { return }
        highlighter.reset(tv.textStorage?.string as NSString? ?? "")
        painted = []
        pendingDirty = nil
        guard enabled else { return }
        extend(to: Self.window(for: tv))
    }

    func clear() {
        guard let tv = textView, let lm = tv.layoutManager else { return }
        let whole = NSRange(location: 0, length: tv.textStorage?.length ?? 0)
        for range in painted {
            let r = NSIntersectionRange(range, whole)
            if r.length > 0 { lm.removeTemporaryAttribute(Self.key, forCharacterRange: r) }
        }
        painted = []
    }

    /// The lexer describes `text` (no reset pending, lengths agree): only then
    /// may editor intelligence (completion, hover, command-click) consult it.
    func inSync(with text: NSString) -> Bool {
        !resetScheduled && highlighter.length == text.length
    }

    /// The view scrolled: paint the part of the new window not yet painted.
    func scrolled() {
        // Not while a reset is pending (the lexer is stale; `reset()` paints
        // the window itself), and not mid-edit: a bounds change posted inside
        // `processEditing` must not query layout (GH#681) — retry after it.
        guard enabled, !resetScheduled, let tv = textView else { return }
        if let storage = tv.textStorage, !storage.editedMask.isEmpty || storage.length != highlighter.length {
            if !storage.editedMask.isEmpty {
                DispatchQueue.main.async { [weak self] in self?.scrolled() }
            }
            return
        }
        let window = Self.window(for: tv)
        guard window.length > 0, !gaps(in: window).isEmpty else { return }
        extend(to: window)
    }

    private func storageEdited(range: NSRange, replacementLength: Int) {
        guard let tv = textView else { return }
        // A reset is pending: the lexer is stale until it runs, and a later
        // edit can bring the lengths back into agreement by coincidence, so
        // the length check alone is not a barrier. The reset re-lexes the
        // whole text, which covers every edit skipped here.
        if resetScheduled { return }
        let t0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        let text = tv.textStorage?.string as NSString? ?? ""
        if text.length != highlighter.length - range.length + replacementLength {
            // Out of sync (should not happen): start over — after the edit,
            // because `reset()` paints the visible window, which needs layout
            // (see the note on `flush()` below). Until it runs, edits and
            // flushes are skipped (`resetScheduled`), so nothing stale is lexed
            // or painted.
            resetScheduled = true
            DispatchQueue.main.async { [weak self] in self?.resetScheduled = false; self?.reset() }
            return
        }
        let dirty = highlighter.edit(range: range, replacementLength: replacementLength, text: text)
        lastEditLinesLexed = highlighter.lastEditLinesLexed
        shiftPainted(edit: range, replacementLength: replacementLength)
        // Nothing here may touch layout (`Self.window(for:)`, glyph queries):
        // this runs inside `NSTextStorage.processEditing`, where the layout
        // manager raises "attempted layout while textStorage is editing"
        // (GH#681). That Objective-C exception unwound through Swift frames
        // — which Swift does not support — and, when the edit came from a
        // Swift task (an IME `setMarkedText` replacing marked text), left the
        // task's allocator state corrupt: SIGABRT "freed pointer was not the
        // last allocation". The lexer update above stays synchronous (it
        // needs `editedRange` and every edit in order); the visible-window
        // work moves to `flush()`, which runs after the edit.
        pendingDirty = pendingDirty.map { NSUnionRange(Self.shifted($0, edit: range, replacementLength: replacementLength), dirty) } ?? dirty
        lastEditCpuNs = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - t0
        // Painting waits until the layout manager has processed this edit
        // (it shifts the existing temporary attributes): the editor's
        // `textDidChange` flushes right away; this covers edits that post none.
        if !flushScheduled {
            flushScheduled = true
            DispatchQueue.main.async { [weak self] in self?.flushScheduled = false; self?.flush() }
        }
    }

    /// Repaints the pending dirty range (unless marked text exists; then the
    /// dirty range is kept and the flush scheduled by the composition's
    /// teardown edit paints it — every way a composition ends runs through
    /// `storageEdited`, so nothing here re-schedules: re-queueing ran once
    /// per main-queue turn for as long as the composition lasted (GH#780).
    func flush() {
        guard enabled, !resetScheduled, let tv = textView, let dirty = pendingDirty else { return }
        if tv.hasMarkedText() {
            deferredFlushes += 1
            return
        }
        pendingDirty = nil
        guard let lm = tv.layoutManager else { return }
        // Painting only repaints the overlap of `painted` and the dirty range,
        // on the assumption that the dirty range already sits inside the
        // painted window (true for ordinary typing). A whole-buffer replace
        // (select all, delete) shifts every painted range to length 0, which
        // `merged` drops — `painted` becomes `[]` and, since nothing but
        // `reset()` ever repopulates it, stays empty forever after (#673).
        // Re-registering the dirty range (clipped to the visible window, so a
        // large off-screen paste still cannot force painting outside it) as
        // painted is a no-op union in the ordinary case and self-heals this
        // one. It is done here, not in `storageEdited`, because computing the
        // window needs layout, which is invalid mid-edit (GH#681).
        let visibleDirty = NSIntersectionRange(dirty, Self.window(for: tv))
        if visibleDirty.length > 0 { painted = Self.merged(painted + [visibleDirty]) }
        let t0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        defer { lastFlushCpuNs = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - t0 }
        let text = tv.textStorage?.string as NSString? ?? ""
        let whole = NSRange(location: 0, length: text.length)
        var count = 0
        for range in painted {
            let r = NSIntersectionRange(NSIntersectionRange(range, dirty), whole)
            guard r.length > 0 else { continue }
            lm.removeTemporaryAttribute(Self.key, forCharacterRange: r)
            count += paint(highlighter.runs(in: r, text: text), layoutManager: lm)
        }
        paints += 1
        runsPainted += count
    }

    /// `r` after replacing `edit` with `replacementLength` characters. An edit
    /// that touches `r` (ends at its start or starts at its end) joins it, so
    /// text typed at either edge of a painted range is repainted with it
    /// (GH#280: typing at the end of the painted window stayed uncoloured).
    static func shifted(_ r: NSRange, edit: NSRange, replacementLength: Int) -> NSRange {
        let delta = replacementLength - edit.length
        if NSMaxRange(edit) < r.location { return NSRange(location: r.location + delta, length: r.length) }
        if edit.location > NSMaxRange(r) { return r }
        let start = min(r.location, edit.location)
        let end = max(NSMaxRange(r), NSMaxRange(edit)) + delta
        return NSRange(location: start, length: max(0, end - start))
    }

    private func shiftPainted(edit: NSRange, replacementLength: Int) {
        painted = Self.merged(painted.map { Self.shifted($0, edit: edit, replacementLength: replacementLength) })
    }

    func gaps(in window: NSRange) -> [NSRange] {
        var result: [NSRange] = []
        var cursor = window.location
        for range in painted where NSMaxRange(range) > cursor {
            if range.location >= NSMaxRange(window) { break }
            if range.location > cursor { result.append(NSRange(location: cursor, length: range.location - cursor)) }
            cursor = max(cursor, NSMaxRange(range))
        }
        if cursor < NSMaxRange(window) { result.append(NSRange(location: cursor, length: NSMaxRange(window) - cursor)) }
        return result
    }

    private func extend(to window: NSRange) {
        guard let tv = textView, let lm = tv.layoutManager else { return }
        let text = tv.textStorage?.string as NSString? ?? ""
        let whole = NSRange(location: 0, length: text.length)
        var count = 0
        for gap in gaps(in: window) {
            let r = NSIntersectionRange(gap, whole)
            guard r.length > 0 else { continue }
            count += paint(highlighter.runs(in: r, text: text), layoutManager: lm)
        }
        if count > 0 { paints += 1; runsPainted += count }
        painted = Self.merged(painted + [window])
    }

    @discardableResult
    private func paint(_ runs: [SyntaxHighlighter.Run], layoutManager lm: NSLayoutManager) -> Int {
        var n = 0
        for run in runs {
            guard let color = SyntaxTheme.color(for: run.kind) else { continue }
            lm.addTemporaryAttribute(Self.key, value: color, forCharacterRange: run.range)
            n += 1
        }
        return n
    }

    static func merged(_ ranges: [NSRange]) -> [NSRange] {
        SourceEditorView.MarkPainter.merged(ranges)
    }

    static func window(for tv: NSTextView) -> NSRange {
        let length = tv.textStorage?.length ?? 0
        guard let lm = tv.layoutManager, let container = tv.textContainer, tv.window != nil,
              !tv.visibleRect.isEmpty else { return NSRange(location: 0, length: length) }
        let glyphs = lm.glyphRange(forBoundingRect: tv.visibleRect, in: container)
        let visible = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let start = max(0, visible.location - padding)
        let end = min(length, NSMaxRange(visible) + padding)
        return NSRange(location: start, length: max(0, end - start))
    }
}
