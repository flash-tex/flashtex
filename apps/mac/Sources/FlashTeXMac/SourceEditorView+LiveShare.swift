import AppKit
import FlashTeXCollabCore
import FlashTeXCollabSession

/// One session file as the editor sees it (Live Share, proposal §2.4): the
/// binding to the CRDT plus the `UndoManager` that mirrors the binding's
/// local-only undo, so ⌘Z, Edit ▸ Undo and VoiceOver keep working and undo
/// only this participant's edits. Owned by the session controller; the
/// editor holds it while it shows that file.
@MainActor
final class LiveShareFileLink {
    let binding: CollabTextBinding
    let session: CollabSession
    let undoManager = UndoManager()

    init(binding: CollabTextBinding, session: CollabSession) {
        self.binding = binding
        self.session = session
        // Grouped by event, as the text view's own: one ⌘Z undoes what one
        // event registered, and each registration pops exactly one binding
        // step, so the two stacks always agree.
        undoManager.levelsOfUndo = binding.undoManager.limit
        binding.onUndoStepOpened = { [weak self] in self?.registerStep(redo: false) }
    }

    var file: FileID { binding.file }

    private func registerStep(redo: Bool) {
        undoManager.registerUndo(withTarget: self) { $0.perform(redo: redo) }
        undoManager.setActionName("Typing")
    }

    private func perform(redo: Bool) {
        // Each successful revert pushes its inverse; mirror exactly that.
        if redo ? binding.redo() : binding.undo() {
            undoManager.registerUndo(withTarget: self) { $0.perform(redo: !redo) }
            undoManager.setActionName("Typing")
        }
    }
}

/// The coordinator's Live Share state (stored on the coordinator; the
/// logic is in the extension below).
@MainActor
final class LiveShareEditorState {
    var link: LiveShareFileLink?
    var observer: NSObjectProtocol?
    /// > 0 while the coordinator itself resets the text (a document switch,
    /// a reload): that is never a local edit by itself.
    var suspended = 0
    /// True while remote changes are being applied to the storage.
    var applyingRemote = false
    /// The local edit not yet handed to the binding (an input-method
    /// composition): the old range's start and length, and the new length.
    var dirty: (location: Int, oldLength: Int, newLength: Int)?
    /// Remote change batches applied (tests and evidence).
    var remoteApplies = 0
    /// Other participants' carets (nil outside a session).
    var overlay: LiveShareCursorOverlay?
    var presenceObserver: NSObjectProtocol?

    deinit {
        if let observer { NotificationCenter.default.removeObserver(observer) }
        if let presenceObserver { NotificationCenter.default.removeObserver(presenceObserver) }
    }
}

extension SourceEditorView.Coordinator: CollabTextHost {
    // MARK: Binding lifecycle

    /// Called from `updateNSView` after any text reset: attach the file the
    /// editor now shows, or detach.
    func syncLiveShare(_ link: LiveShareFileLink?, in tv: NSTextView, textReset: Bool) {
        let st = liveShare
        if st.link !== link {
            if let old = st.link {
                collabFlushLocalEdits()
                old.binding.attach(nil)
                if let o = st.observer { NotificationCenter.default.removeObserver(o) }
                st.observer = nil
                if let o = st.presenceObserver { NotificationCenter.default.removeObserver(o) }
                st.presenceObserver = nil
                st.overlay?.removeFromSuperview()
                st.overlay = nil
            }
            st.link = link
            st.dirty = nil
            guard let link else {
                tv.allowsUndo = true
                return
            }
            // The text view's own undo would mix everyone's edits; the
            // link's UndoManager (via `undoManager(for:)`) takes over.
            tv.allowsUndo = false
            st.observer = NotificationCenter.default.addObserver(
                forName: NSTextStorage.didProcessEditingNotification, object: tv.textStorage, queue: nil
            ) { [weak self] note in
                MainActor.assumeIsolated {
                    guard let self, let storage = note.object as? NSTextStorage else { return }
                    self.storageEdited(storage)
                }
            }
            link.binding.attach(self)
            adoptViewText(tv, into: link)
            let overlay = LiveShareCursorOverlay(frame: tv.bounds)
            overlay.autoresizingMask = [.width, .height]
            overlay.textView = tv
            overlay.cursors = { [weak link] in link.map { $0.session.remoteCursors(in: $0.file) } ?? [] }
            tv.addSubview(overlay)
            st.overlay = overlay
            st.presenceObserver = NotificationCenter.default.addObserver(
                forName: .liveSharePresenceChanged, object: link.session, queue: nil
            ) { [weak overlay] _ in MainActor.assumeIsolated { overlay?.needsDisplay = true } }
            link.session.setLocalPresence(file: link.file, selection: collabSelection)
        } else if textReset, let link {
            // A reload or revert of the same file replaced the buffer: the
            // difference is a local edit.
            adoptViewText(tv, into: link)
        }
    }

    private func adoptViewText(_ tv: NSTextView, into link: LiveShareFileLink) {
        let viewText = SourceEditorView.nativeText(of: tv)
        if viewText != link.binding.document.text { link.binding.adopt(text: viewText) }
    }

    // MARK: Local edits

    /// Every storage edit the user (or the coordinator, for a capture or an
    /// auto-closed pair) makes reaches the CRDT here, one edit at a time;
    /// during an input-method composition the edits are only collected, and
    /// handed over as one when it commits (proposal §2.4: composition steps
    /// never reach peers).
    private func storageEdited(_ storage: NSTextStorage) {
        let st = liveShare
        guard st.suspended == 0, !st.applyingRemote, st.link != nil,
              storage.editedMask.contains(.editedCharacters) else { return }
        let r = storage.editedRange
        guard r.location != NSNotFound else { return }
        let delta = storage.changeInLength
        let e = (location: r.location, oldLength: r.length - delta, newLength: r.length)
        if let d = st.dirty {
            // Union of the pending edit and this one, in current coordinates.
            let start = min(d.location, e.location)
            let endBefore = max(d.location + d.newLength, e.location + e.oldLength)
            let span = endBefore - start
            st.dirty = (start, span - (d.newLength - d.oldLength), span + (e.newLength - e.oldLength))
        } else {
            st.dirty = e
        }
        if textView?.hasMarkedText() != true { collabFlushLocalEdits() }
        st.overlay?.needsDisplay = true // remote carets move with the text
    }

    func collabFlushLocalEdits() {
        let st = liveShare
        guard let d = st.dirty, let link = st.link, let storage = textView?.textStorage,
              textView?.hasMarkedText() != true else { return }
        st.dirty = nil
        let new = (storage.string as NSString).substring(with: NSRange(location: d.location, length: d.newLength))
        link.binding.localReplace(d.location..<(d.location + d.oldLength), with: new)
    }

    /// End of a composition or a turn: flush, then integrate what waited.
    func liveShareSettle() {
        guard let link = liveShare.link, textView?.hasMarkedText() != true else { return }
        collabFlushLocalEdits()
        if link.binding.heldCount > 0 { link.binding.release() }
    }

    /// The caret moved by itself: the next keystroke starts a new undo step,
    /// and everyone sees where it went.
    func liveShareSelectionChanged(_ tv: NSTextView, typing: Bool) {
        guard let link = liveShare.link, !tv.hasMarkedText() else { return }
        liveShareSettle()
        if !typing, programmaticChanges == 0 { link.binding.breakUndoCoalescing() }
        link.session.setLocalPresence(file: link.file, selection: collabSelection)
    }

    func liveShareBreakUndoCoalescing() { liveShare.link?.binding.breakUndoCoalescing() }

    // MARK: CollabTextHost

    var collabIsComposing: Bool { textView?.hasMarkedText() ?? false }

    var collabSelection: Range<Int> {
        guard let r = textView?.selectedRange(), r.location != NSNotFound else { return 0..<0 }
        return r.location..<(r.location + r.length)
    }

    /// Remote changes (or this participant's undo) as minimal storage edits
    /// in one editing group: never `tv.string = …`, never on the text view's
    /// undo stack. The side state `shouldChangeTextIn` keeps aligned with
    /// user edits (marks, pending closers, folds) is shifted by the same
    /// helpers, the viewport stays on the text it showed, and the selection
    /// goes where the session resolved it.
    func collabApply(_ changes: [TextChange], selection: Range<Int>?) {
        guard let tv = textView, let storage = tv.textStorage, !changes.isEmpty else { return }
        let st = liveShare
        st.applyingRemote = true
        programmaticChanges += 1
        defer {
            programmaticChanges -= 1
            st.applyingRemote = false
        }
        let anchor = viewportAnchor(tv)
        var top = anchor?.index
        storage.beginEditing()
        for c in changes {
            let r = NSRange(location: c.location, length: c.length)
            let len = (c.text as NSString).length
            guard NSMaxRange(r) <= storage.length else { continue }
            marks.noteEdit(range: r, replacementLength: len)
            shiftPendingClosers(edit: r, replacementLength: len)
            (tv as? CompletingTextView)?.noteFoldEdit(r, replacementLength: len)
            if let t = top {
                if t >= NSMaxRange(r) { top = t + len - r.length } else if t > r.location { top = r.location }
            }
            if storage.length == 0 {
                storage.replaceCharacters(in: r, with: NSAttributedString(string: c.text, attributes: tv.typingAttributes))
            } else {
                storage.replaceCharacters(in: r, with: c.text)
            }
        }
        storage.endEditing()
        linkedSession = nil // the captured \begin/\end spans no longer describe the buffer
        if let selection, selection.upperBound <= storage.length {
            tv.setSelectedRange(NSRange(location: selection.lowerBound, length: selection.count))
        }
        if let anchor, let top { restoreViewport(tv, index: top, offset: anchor.offset) }
        tv.didChangeText() // gutter, folds, syntax flush; `commitUserChange` is skipped (programmatic)
        st.remoteApplies += 1
        st.overlay?.needsDisplay = true
        let s = SourceEditorView.nativeText(of: tv)
        lastKnownText = s
        parent.text = s // the model, the engine and autosave follow as for typing
        refreshBraceHighlight(tv)
    }

    /// The first visible character and how far its line sits below the top
    /// of the visible rect.
    private func viewportAnchor(_ tv: NSTextView) -> (index: Int, offset: CGFloat)? {
        guard let lm = tv.layoutManager, let container = tv.textContainer, tv.window != nil,
              let storage = tv.textStorage, storage.length > 0 else { return nil }
        let visible = tv.visibleRect
        guard visible.minY > 0 else { return nil } // at the very top nothing needs keeping
        let point = NSPoint(x: 0, y: visible.minY - tv.textContainerInset.height)
        let glyph = lm.glyphIndex(for: point, in: container)
        let index = lm.characterIndexForGlyph(at: glyph)
        let line = lm.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
        return (index, line.minY + tv.textContainerInset.height - visible.minY)
    }

    private func restoreViewport(_ tv: NSTextView, index: Int, offset: CGFloat) {
        guard let lm = tv.layoutManager, let storage = tv.textStorage, index < storage.length else { return }
        let glyph = lm.glyphIndexForCharacter(at: index)
        let line = lm.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
        let y = line.minY + tv.textContainerInset.height - offset
        guard abs(y - tv.visibleRect.minY) > 0.5 else { return }
        tv.scroll(NSPoint(x: tv.visibleRect.minX, y: max(0, y)))
    }
}

/// Other participants' carets and selections over the editor (proposal §4):
/// a transparent, click-through subview of the text view that draws, over
/// the visible text only, each selection as a tint and each caret as a bar
/// in that person's colour, with a name flag that shows while they move and
/// fades 2.5 s after. Nothing touches the text storage or its attributes.
@MainActor
final class LiveShareCursorOverlay: NSView {
    weak var textView: NSTextView?
    var cursors: () -> [CollabSession.RemoteCursor] = { [] }
    /// Cursors drawn by the last pass (tests and evidence).
    private(set) var drawnCursors: [CollabSession.RemoteCursor] = []
    private var fade: Timer?
    static let flagSeconds: TimeInterval = 2.5

    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func isAccessibilityElement() -> Bool { false }

    override func draw(_ dirtyRect: NSRect) {
        guard let tv = textView, let lm = tv.layoutManager, let tc = tv.textContainer, let storage = tv.textStorage else { return }
        let all = cursors()
        drawnCursors = []
        guard !all.isEmpty else { return }
        let origin = tv.textContainerOrigin
        let length = storage.length
        let visibleGlyphs = lm.glyphRange(forBoundingRect: tv.visibleRect.offsetBy(dx: -origin.x, dy: -origin.y), in: tc)
        let visible = lm.characterRange(forGlyphRange: visibleGlyphs, actualGlyphRange: nil)
        let lo = max(0, visible.location - 200)
        let window = NSRange(location: lo, length: min(length, NSMaxRange(visible) + 200) - lo)
        var nextFade: TimeInterval?
        for c in all where c.range.upperBound <= length && c.head <= length {
            let colour = LiveShareColours.colour(c.colourIndex)
            let sel = NSIntersectionRange(NSRange(location: c.range.lowerBound, length: c.range.count), window)
            if sel.length > 0 {
                let glyphs = lm.glyphRange(forCharacterRange: sel, actualCharacterRange: nil)
                colour.withAlphaComponent(0.22).setFill()
                lm.enumerateEnclosingRects(forGlyphRange: glyphs, withinSelectedGlyphRange: NSRange(location: NSNotFound, length: 0),
                                           in: tc) { rect, _ in
                    let r = rect.offsetBy(dx: origin.x, dy: origin.y)
                    if r.intersects(dirtyRect) { r.fill(using: .sourceOver) }
                }
            }
            guard NSLocationInRange(c.head, window) || c.head == NSMaxRange(window),
                  let caret = caretRect(c.head, length: length, lm: lm, tc: tc, storage: storage, origin: origin) else { continue }
            drawnCursors.append(c)
            colour.setFill()
            NSRect(x: caret.minX - 1, y: caret.minY, width: 2, height: caret.height).fill()
            if c.idle < Self.flagSeconds {
                drawFlag(c.name, colour: colour, above: caret)
                let left = Self.flagSeconds - c.idle
                nextFade = min(nextFade ?? left, left)
            }
        }
        fade?.invalidate()
        if let nextFade {
            let t = Timer(timeInterval: nextFade + 0.05, repeats: false) { [weak self] _ in
                MainActor.assumeIsolated { self?.needsDisplay = true }
            }
            RunLoop.main.add(t, forMode: .common)
            fade = t
        }
    }

    private func caretRect(_ head: Int, length: Int, lm: NSLayoutManager, tc: NSTextContainer, storage: NSTextStorage,
                           origin: NSPoint) -> NSRect? {
        let fontHeight = (textView?.font).map { lm.defaultLineHeight(for: $0) } ?? 14
        if length == 0 { return NSRect(x: origin.x + tc.lineFragmentPadding, y: origin.y, width: 1, height: fontHeight) }
        if head < length {
            let g = lm.glyphIndexForCharacter(at: head)
            let line = lm.lineFragmentRect(forGlyphAt: g, effectiveRange: nil)
            let loc = lm.location(forGlyphAt: g)
            return NSRect(x: origin.x + line.minX + loc.x, y: origin.y + line.minY, width: 1, height: line.height)
        }
        if (storage.string as NSString).character(at: length - 1) == 10 {
            let extra = lm.extraLineFragmentRect
            guard extra != .zero else { return nil }
            return NSRect(x: origin.x + extra.minX + tc.lineFragmentPadding, y: origin.y + extra.minY, width: 1, height: extra.height)
        }
        let g = lm.glyphIndexForCharacter(at: length - 1)
        let r = lm.boundingRect(forGlyphRange: NSRange(location: g, length: 1), in: tc)
        return NSRect(x: origin.x + r.maxX, y: origin.y + r.minY, width: 1, height: r.height)
    }

    private func drawFlag(_ name: String, colour: NSColor, above caret: NSRect) {
        let attrs: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: 10, weight: .semibold), .foregroundColor: NSColor.white]
        let text = NSAttributedString(string: name, attributes: attrs)
        let size = text.size()
        let box = NSRect(x: caret.minX - 1, y: max(0, caret.minY - size.height - 2), width: size.width + 8, height: size.height + 2)
        colour.setFill()
        NSBezierPath(roundedRect: box, xRadius: 3, yRadius: 3).fill()
        text.draw(at: NSPoint(x: box.minX + 4, y: box.minY + 1))
    }
}
