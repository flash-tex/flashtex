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
/// The new text is usually not even a new buffer (APP-KEYPATH): the model,
/// the engine session and the compiled-text baseline each keep a recent
/// text, so a new version cannot be written over the last one, but a text a
/// few keys old is nobody's any more. `Spares` keeps the last few texts with
/// the edits each still lacks, and the oldest is brought up to date in place
/// (`replaceSubrange` on a buffer only it holds: the tail moves, nothing is
/// allocated or copied). One still shared is copied by the standard library,
/// as the splice would have been. At 3 MB: 0.93 ms per key for a fresh
/// splice, 0.11 ms in place (`swiftc -O`, mac-m5pro-kabir).
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
    /// Earlier texts, reused as the buffer of a later one (shared by copies
    /// of this value: `head` says which text their edits lead to).
    private let spares = Spares()

    static let enabled = ProcessInfo.processInfo.environment["FLASHTEX_EDITOR_SHADOW"] != "0"

    /// `text` is exactly the storage's characters, `length16` UTF-16 units.
    init(text: String, length16: Int) {
        var t = text
        t.makeContiguousUTF8()
        self.text = t
        bytes = t.utf8.count
        self.length16 = length16
    }

    /// Whether `other` is this shadow's text itself (the same UTF-8 storage),
    /// not merely equal: O(1).
    func holds(_ other: String) -> Bool {
        var a = text, b = other
        return a.withUTF8 { ab in b.withUTF8 { bb in ab.count == bb.count && ab.baseAddress == bb.baseAddress } }
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
        let edit = Spares.Edit(at: prefix, delete: delete, insert: insert)
        let old = text
        let new: String
        if let reused = spares.next(after: old, edit: edit, total: total) {
            new = reused
        } else {
            var o = old
            new = o.withUTF8 { o in
                insert.withUTF8 { ins in
                    // Room to grow: the buffer is reused for a later text in place.
                    String(unsafeUninitializedCapacity: max(total + Spares.growthRoom(total), 1)) { buf in
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
            guard new.utf8.count == total else { spares.clear(); return false }
            Spares.note(copied: total)
        }
        spares.retire(old, became: new, by: edit)
        text = new
        bytes = total
        length16 = length
        anchors = a
        return true
    }
}

extension EditorTextShadow {
    /// The last few texts of a shadow, oldest first, each with the byte
    /// edits that turn it into `head` (the newest text).
    final class Spares {
        struct Edit { var at: Int; var delete: Int; var insert: String }

        /// Texts kept: a text this many keys old is normally no one else's
        /// (the model, the engine session, the compiled baseline and the
        /// marks' anchors hold the last few). Each is at most twice the
        /// text's size (growth room).
        static let capacity = 4

        private var head = ""
        private var entries: [(text: String, pending: [Edit])] = []

        /// Bytes written per keystroke, by kind (tests and evidence): a new
        /// buffer (`copied`), or moved inside a reused one (`moved`).
        nonisolated(unsafe) private(set) static var copiedBytes = 0
        nonisolated(unsafe) private(set) static var movedBytes = 0
        /// Keystrokes whose text was a reused buffer, brought up to date in place.
        nonisolated(unsafe) private(set) static var reusedInPlace = 0
        static func note(copied: Int) { copiedBytes &+= copied }
        static func resetCounters() { copiedBytes = 0; movedBytes = 0; reusedInPlace = 0 }

        func clear() { head = ""; entries = [] }

        /// Spare capacity of a spliced buffer, so typing into it in place
        /// does not grow it (a growth is a copy).
        static func growthRoom(_ bytes: Int) -> Int { min(64 * 1024, bytes / 4) }

        /// `old` + `edit` as the oldest kept text brought up to date, or nil
        /// (the caller splices a new buffer). `old` must be `head`, and the
        /// edit's ends must be on scalar boundaries of `old`: then each
        /// replacement is byte-exact, as the splice is.
        func next(after old: String, edit: Edit, total: Int) -> String? {
            guard Self.sameStorage(head, old), Self.onScalarBoundaries(old, edit) else { clear(); return nil }
            guard entries.count >= Self.capacity else { return nil }
            var spare = entries.removeFirst() // moved out: the array no longer holds it
            let start = Self.address(spare.text)
            var moved = 0
            var edits = spare.pending
            Self.append(edit, to: &edits)
            for e in edits {
                let u = spare.text.utf8
                let a = u.index(u.startIndex, offsetBy: e.at), b = u.index(a, offsetBy: e.delete)
                moved += u.count - e.at - e.delete + e.insert.utf8.count
                spare.text.replaceSubrange(a ..< b, with: e.insert)
            }
            guard spare.text.utf8.count == total else { clear(); return nil }
            if start != nil, Self.address(spare.text) == start {
                Self.reusedInPlace &+= 1
                Self.movedBytes &+= moved
            } else {
                Self.copiedBytes &+= total // still shared: the standard library copied it
            }
            return spare.text
        }

        /// `old` became `new` by `edit`: every kept text now lacks `edit` too.
        func retire(_ old: String, became new: String, by edit: Edit) {
            defer { head = new }
            // Only a byte-exact edit is kept for replaying (a misaligned one
            // never reaches here through `next`, but may through the splice).
            guard Self.onScalarBoundaries(old, edit) else { entries = []; return }
            if !Self.sameStorage(head, old) { entries = [] }
            for i in entries.indices { Self.append(edit, to: &entries[i].pending) }
            if old.utf8.count > 15 { entries.append((old, [edit])) } // a small string has no buffer to reuse
            if entries.count > Self.capacity { entries.removeFirst(entries.count - Self.capacity) }
        }

        /// Typing runs as one edit (each one replacement, so one move of the
        /// tail): a key after the last edit's insertion, or a backspace
        /// inside it, joins that edit. Both ends are scalar boundaries of the
        /// text (`onScalarBoundaries`), so the cut insertion is still text.
        static func append(_ e: Edit, to pending: inout [Edit]) {
            guard var last = pending.last else { pending.append(e); return }
            let insEnd = last.at + last.insert.utf8.count
            if e.delete == 0, e.at == insEnd {
                last.insert += e.insert
            } else if e.delete > 0, e.at >= last.at, e.at + e.delete == insEnd {
                last.insert = String(decoding: Array(last.insert.utf8)[0 ..< (e.at - last.at)], as: UTF8.self) + e.insert
            } else {
                pending.append(e); return
            }
            pending[pending.count - 1] = last
        }

        static func onScalarBoundaries(_ s: String, _ e: Edit) -> Bool {
            s.utf8.withContiguousStorageIfAvailable { b in
                func boundary(_ i: Int) -> Bool { i == b.count || (i >= 0 && i < b.count && b[i] & 0xC0 != 0x80) }
                return boundary(e.at) && boundary(e.at + e.delete)
            } ?? false
        }

        static func address(_ s: String) -> UnsafeRawPointer? {
            s.utf8.withContiguousStorageIfAvailable { UnsafeRawPointer($0.baseAddress) } ?? nil
        }

        /// The same UTF-8 storage (not merely equal bytes): O(1). Small
        /// strings live inline and have no identity.
        static func sameStorage(_ a: String, _ b: String) -> Bool {
            guard let pa = a.utf8.withContiguousStorageIfAvailable({ ($0.baseAddress, $0.count) }),
                  let pb = b.utf8.withContiguousStorageIfAvailable({ ($0.baseAddress, $0.count) }) else { return false }
            if pa.1 <= 15 { return pa.1 == pb.1 && a == b } // inline: equal is the same
            return pa.0 == pb.0 && pa.1 == pb.1
        }
    }
}
