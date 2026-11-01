import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// Pieces random text is made of, as in the Rust tests: ASCII, LaTeX
/// punctuation, 2-, 3- and 4-byte scalars, a combining mark.
let pieces = ["a", "b", "c", "x", "y", "z", " ", "\\", "{", "}", "\n", "é", "中", "😀", "e\u{301}"]
let paths = ["main.tex", "a.tex", "ch/b.tex", "refs.bib"]

func randomText(_ rng: inout SplitMix64, maxPieces: Int) -> String {
    let n = 1 + rng.below(maxPieces)
    return (0..<n).map { _ in pieces[rng.below(pieces.count)] }.joined()
}

func envInt(_ name: String, _ fallback: Int) -> Int {
    ProcessInfo.processInfo.environment[name].flatMap(Int.init) ?? fallback
}

/// N peers exchanging collab-v1 frames over a hostile in-process network:
/// random order, duplicates, drops (repaired by state-vector sync) and
/// partitions. Every frame goes through the codec.
final class Sim {
    struct Msg {
        var from: Int
        var to: Int
        var frame: [UInt8]
    }

    var rng: SplitMix64
    var peers: [CollabProject]
    var queue: [Msg] = []
    var frames: [[UInt8]] = []
    var partition: [Bool]?
    var seq: UInt64 = 0
    /// Each peer's local undo, per file.
    var undo: [Int: [FileID: TextUndoManager]] = [:]

    func undoManager(_ p: Int, _ f: FileID) -> TextUndoManager {
        if let u = undo[p]?[f] { return u }
        let u = TextUndoManager(document: peers[p].text(f)!)
        undo[p, default: [:]][f] = u
        return u
    }

    init(seed: UInt64, peers n: Int) {
        rng = SplitMix64(seed: seed)
        var ids: [UInt64] = []
        while ids.count < n {
            let r = rng.next() | 1
            if !ids.contains(r) { ids.append(r) }
        }
        peers = ids.map { CollabProject(replica: $0) }
    }

    func broadcast(_ from: Int, _ sections: [Section]) {
        seq += 1
        let frame = CollabWire.encode(.update(seq: seq, sections: sections))
        for to in peers.indices where to != from { queue.append(Msg(from: from, to: to, frame: frame)) }
        frames.append(frame)
    }

    func receive(_ to: Int, _ frame: [UInt8], file: StaticString = #filePath, line: UInt = #line) {
        guard let (msg, used) = try! CollabWire.decode(frame) else { return XCTFail("incomplete frame") }
        XCTAssertEqual(used, frame.count, file: file, line: line)
        XCTAssertEqual(CollabWire.encode(msg), frame, "canonical re-encoding", file: file, line: line)
        let sections: [Section]
        switch msg {
        case let .update(_, s), let .syncReply(s): sections = s
        default: return XCTFail("unexpected \(msg)")
        }
        let errors = peers[to].receive(sections)
        XCTAssertEqual(errors, [], "peer \(to) refused", file: file, line: line)
    }

    func deliverOne() -> Bool {
        let candidates = queue.indices.filter { i in
            guard let side = partition else { return true }
            return side[queue[i].from] == side[queue[i].to]
        }
        guard !candidates.isEmpty else { return false }
        let i = candidates[rng.below(candidates.count)]
        let m = queue.remove(at: i)
        if rng.chance(3) { return true } // dropped; sync repairs it
        receive(m.to, m.frame)
        if rng.chance(8) { queue.append(m) } // duplicated
        return true
    }

    func flushInOrder() {
        while !queue.isEmpty {
            let m = queue.removeFirst()
            receive(m.to, m.frame)
        }
    }

    func randomTextEdit(_ p: Int) {
        let files = peers[p].textFileIDs
        guard !files.isEmpty else { return }
        let f = files[rng.below(files.count)]
        let len = peers[p].text(f)!.count
        let s: Section?
        if len > 0 && rng.chance(35) {
            let pos = rng.below(len)
            s = peers[p].delete(at: pos, length: 1 + rng.below(min(len - pos, 6)), in: f)
        } else {
            let pos = rng.below(len + 1)
            s = try! peers[p].insert(randomText(&rng, maxPieces: 6), at: pos, in: f)
        }
        if case let .text(_, ops)? = s { undoManager(p, f).record(ops) }
        if let s { broadcast(p, [s]) }
    }

    /// Undo (mostly) or redo one of peer `p`'s steps in a random file.
    func randomUndo(_ p: Int) {
        let files = peers[p].textFileIDs
        guard !files.isEmpty else { return }
        let f = files[rng.below(files.count)]
        let u = undoManager(p, f)
        let ops = rng.chance(30) ? u.redo() : u.undo()
        if !ops.isEmpty { broadcast(p, [.text(f, ops)]) }
    }

    func randomFileOp(_ p: Int) {
        let path = paths[rng.below(paths.count)]
        let views = peers[p].files.files
        let known = views.map(\.file)
        let blobs = views.filter { $0.kind == .blob }.map(\.file)
        let all = peers[p].textFileIDs
        let roll = rng.below(6)
        let s: Section
        if roll == 0 || all.isEmpty {
            let id = FileID.random(using: &rng)
            s = try! peers[p].fileOp { try $0.create(id, kind: .text, path: path) }
        } else if roll == 1 && !known.isEmpty {
            let f = known[rng.below(known.count)]
            s = try! peers[p].fileOp { try $0.rename(f, to: path) }
        } else if roll == 2 && blobs.isEmpty {
            let id = FileID.random(using: &rng)
            s = try! peers[p].fileOp { try $0.create(id, kind: .blob, path: "fig/plot.png") }
        } else if roll <= 3 && !blobs.isEmpty {
            let f = blobs[rng.below(blobs.count)]
            var sha: [UInt8] = []
            for _ in 0..<4 { var v = rng.next(); for _ in 0..<8 { sha.append(UInt8(truncatingIfNeeded: v)); v >>= 8 } }
            let blob = BlobRef(sha256: sha, bytes: rng.next() % 50_000_000, mediaType: "image/png")
            s = try! peers[p].fileOp { try $0.setBlob(f, blob) }
        } else {
            let f = all[rng.below(all.count)]
            let del = !(peers[p].files.isDeleted(f)!)
            s = try! peers[p].fileOp { try $0.setDeleted(f, del) }
        }
        broadcast(p, [s])
    }

    func heal() {
        partition = nil
        while deliverOne() {}
        for _ in 0..<2 {
            for i in peers.indices {
                for j in peers.indices where i != j {
                    let req = CollabWire.encode(.syncRequest(peers[j].stateVectors))
                    guard case let .syncRequest(vs)? = try! CollabWire.decode(req)?.0 else { return XCTFail("sync") }
                    receive(j, CollabWire.encode(.syncReply(peers[i].diff(vs))))
                }
            }
        }
    }

    func assertConverged(_ ctx: String, file: StaticString = #filePath, line: UInt = #line) {
        let first = peers[0]
        for (k, p) in peers.enumerated() {
            XCTAssertEqual(p.pendingCount, 0, "\(ctx): peer \(k) pending", file: file, line: line)
            XCTAssertEqual(p.stateVectors, first.stateVectors, "\(ctx): peer \(k) state vectors", file: file, line: line)
            for f in first.textFileIDs {
                XCTAssertEqual(Array(p.text(f)!.text.utf8), Array(first.text(f)!.text.utf8),
                               "\(ctx): peer \(k) text", file: file, line: line)
                if let problem = p.text(f)!.validate() { XCTFail("\(ctx): peer \(k): \(problem)", file: file, line: line) }
            }
            XCTAssertEqual(p.files.files, first.files.files, "\(ctx): peer \(k) files", file: file, line: line)
            XCTAssertEqual(p.digest, first.digest, "\(ctx): peer \(k) digest", file: file, line: line)
        }
    }
}
