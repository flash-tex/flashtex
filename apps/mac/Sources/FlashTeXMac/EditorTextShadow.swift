import Foundation

/// The editor's text as a native UTF-8 `String`, kept in step with the text
/// storage one edit at a time (APP-EDITOR-INSTANT).
///
/// Every keystroke hands the model the whole text. It used to be transcoded
/// from the storage's UTF-16 on each key (`SourceEditorView.nativeText`): a
/// 4 MB book is milliseconds of main thread per key, half a frame. Here each
/// storage edit is spliced into the previous text instead: the edit's byte
/// offset comes from `EngineV3Edits.fastSplice` (counted from an anchor near
/// the last edit, not from the start), and the new text is two `memcpy`s
/// around the inserted bytes.
///
/// The shadow is only ever used when it provably matches: an edit it cannot
/// follow drops it (the next text is transcoded, and the shadow starts again
/// from that), and its UTF-16 length and UTF-8 length are checked against
/// the storage's on every edit. `FLASHTEX_EDITOR_SHADOW=0` turns it off;
/// `verify` (tests) compares it with a transcode on every use.
struct EditorTextShadow {
    private(set) var text: String
    /// `text`'s UTF-8 length, and the storage's UTF-16 length it matches.
    private(set) var bytes: Int
    private(set) var length16: Int
    private var anchors: EngineV3Edits.Anchors?

    static let enabled = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_SHADOW"] != "0"

    /// `text` is exactly the storage's characters, `length16` UTF-16 units.
    init(text: String, length16: Int) {
        var t = text
        t.makeContiguousUTF8()
        self.text = t
        bytes = t.utf8.count
        self.length16 = length16
    }

    /// Follows one storage edit (`storage` is the text after it; `r` the
    /// edited range in it; `delta` the change in length). False when it
    /// cannot: the shadow must then be dropped.
    mutating func apply(storage: NSString, edited r: NSRange, delta: Int) -> Bool {
        let length = storage.length
        guard r.location != NSNotFound, r.location >= 0, NSMaxRange(r) <= length, length16 == length - delta,
              r.length - delta >= 0 else { return false }
        let (prefix, delete, total, a) = EngineV3Edits.fastSplice(text: storage, edited: r, delta: delta, base: bytes, anchors: anchors)
        guard prefix >= 0, delete >= 0, prefix + delete <= bytes else { return false }
        var insert = storage.substring(with: r)
        let insertBytes = insert.withUTF8 { $0.count }
        let tail = bytes - prefix - delete
        guard prefix + insertBytes + tail == total else { return false }
        var old = text
        let new: String = old.withUTF8 { o in
            insert.withUTF8 { ins in
                String(unsafeUninitializedCapacity: max(total, 1)) { buf in
                    guard let dst = buf.baseAddress else { return 0 }
                    if prefix > 0, let src = o.baseAddress { memcpy(dst, src, prefix) }
                    if insertBytes > 0, let src = ins.baseAddress { memcpy(dst + prefix, src, insertBytes) }
                    if tail > 0, let src = o.baseAddress { memcpy(dst + prefix + insertBytes, src + prefix + delete, tail) }
                    return total
                }
            }
        }
        // `String(unsafeUninitializedCapacity:)` repairs invalid UTF-8 (a
        // splice inside a sequence): the lengths then disagree.
        guard new.utf8.count == total else { return false }
        text = new
        bytes = total
        length16 = length
        anchors = a
        return true
    }
}
