import Foundation
import XCTest
@testable import FlashTeXCollabCore

/// The differential check against the Rust oracle (proposal §7.2): replay
/// every fixture in crates/collaboration-core/tests/fixtures/collab-v1, in
/// each of its delivery orders, into a fresh replica, and compare texts,
/// materialised paths and structure digests with what the generator
/// recorded. The oracle's `v1_fixtures` test replays the same files,
/// including the `swift-*.json` ones recorded here with
/// `FLASHTEX_COLLAB_RECORD=1 swift test --filter FixtureTests`.
final class FixtureTests: XCTestCase {
    static let dir = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("crates/collaboration-core/tests/fixtures/collab-v1")

    static func hex(_ b: [UInt8]) -> String { b.map { String(format: "%02x", $0) }.joined() }
    static func unhex(_ s: String) -> [UInt8] {
        var out: [UInt8] = []
        var it = s.utf8.makeIterator()
        func nib(_ c: UInt8) -> UInt8 { c <= 57 ? c - 48 : c - 87 }
        while let a = it.next(), let b = it.next() { out.append(nib(a) << 4 | nib(b)) }
        return out
    }

    static func replay(_ frames: [[UInt8]], _ order: [Int]) throws -> CollabProject {
        let p = CollabProject(replica: .max)
        for i in order {
            guard case let .update(_, sections)? = try CollabWire.decode(frames[i])?.0 else {
                throw CollabWireError.invalid("fixture frames are updates")
            }
            let errors = p.receive(sections)
            if !errors.isEmpty { throw CollabWireError.invalid("refused: \(errors)") }
        }
        if p.pendingCount != 0 { throw CollabWireError.invalid("\(p.pendingCount) ops left pending") }
        return p
    }

    static func expected(_ p: CollabProject) -> [String: Any] {
        let files: [[String: Any]] = p.files.files.map { e in
            var d: [String: Any] = ["file": e.file.hex, "path": e.path, "kind": e.kind == .text ? "text" : "blob"]
            if let t = p.text(e.file) {
                d["text"] = t.text
                d["digest"] = String(format: "%016llx", t.digest)
            }
            if let b = e.blob { d["sha256"] = hex(b.sha256) }
            return d
        }
        return ["project_digest": String(format: "%016llx", p.digest),
                "filemap_digest": String(format: "%016llx", p.files.digest),
                "files": files]
    }

    func testFixturesReplayIdentically() throws {
        if ProcessInfo.processInfo.environment["FLASHTEX_COLLAB_RECORD"] != nil { try record() }
        let names = try FileManager.default.contentsOfDirectory(atPath: Self.dir.path)
            .filter { $0.hasSuffix(".json") }.sorted()
        XCTAssertGreaterThanOrEqual(names.count, 13, "fixtures missing")
        XCTAssertTrue(names.contains { $0.hasPrefix("swift-") }, "the Swift-recorded fixtures are missing")
        for name in names {
            let data = try Data(contentsOf: Self.dir.appendingPathComponent(name))
            let fx = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
            XCTAssertEqual(fx["format"] as? String, "collab-v1-fixture/1")
            let frames = (fx["frames"] as! [String]).map(Self.unhex)
            for f in frames {
                let (msg, used) = try XCTUnwrap(try CollabWire.decode(f))
                XCTAssertEqual(used, f.count)
                XCTAssertEqual(CollabWire.encode(msg), f, "\(name): non-canonical frame")
            }
            let want = fx["expected"] as! [String: Any]
            for order in fx["orders"] as! [[Int]] {
                let p = try Self.replay(frames, order)
                for f in p.textFileIDs {
                    if let problem = p.text(f)!.validate() { XCTFail("\(name): \(problem)") }
                }
                let got = Self.expected(p)
                XCTAssertTrue(NSDictionary(dictionary: got).isEqual(to: want),
                              "\(name) order \(order.prefix(8))...: got \(got), want \(want)")
            }
        }
    }

    // MARK: Recording (Swift -> Rust direction)

    static func orders(_ rng: inout SplitMix64, _ n: Int) -> [[Int]] {
        let fwd = Array(0..<n)
        var shuffled = fwd
        if n > 1 { for i in stride(from: n - 1, through: 1, by: -1) { shuffled.swapAt(i, rng.below(i + 1)) } }
        for _ in 0..<n / 4 { shuffled.append(rng.below(n)) }
        return [fwd, fwd.reversed(), shuffled]
    }

    static func write(_ name: String, _ description: String, _ frames: [[UInt8]], seed: UInt64) throws {
        var rng = SplitMix64(seed: seed)
        let orders = orders(&rng, frames.count)
        let p = try replay(frames, orders[0])
        let obj: [String: Any] = [
            "format": "collab-v1-fixture/1", "name": name, "description": description,
            "generator": "swift (apps/mac/Sources/FlashTeXCollabCore)",
            "frames": frames.map(hex), "orders": orders, "expected": expected(p),
        ]
        let data = try JSONSerialization.data(withJSONObject: obj, options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes])
        try (data + Data("\n".utf8)).write(to: dir.appendingPathComponent("\(name).json"))
    }

    func record() throws {
        // Random project scenarios, plus local undo and redo: the oracle
        // has no undo manager, so this is where its operations get checked.
        for k in 0..<4 {
            let seed = UInt64(0x5f1f7 + k)
            var rng = SplitMix64(seed: seed)
            let n = 2 + rng.below(3)
            let sim = Sim(seed: seed, peers: n)
            let main = FileID.random(using: &sim.rng)
            sim.broadcast(0, [try sim.peers[0].fileOp { try $0.create(main, kind: .text, path: "main.tex") }])
            sim.flushInOrder()
            let undo = sim.peers.map { TextUndoManager(document: $0.text(main)!) }
            for _ in 0..<(k == 3 ? 1_500 : 80) {
                let p = rng.below(n)
                let doc = sim.peers[p].text(main)!
                switch rng.below(20) {
                case 0 where k != 3: sim.randomFileOp(p)
                case 1...5: for _ in 0..<rng.below(5) { _ = sim.deliverOne() }
                case 6, 7:
                    let ops = rng.chance(70) ? undo[p].undo() : undo[p].redo()
                    if !ops.isEmpty { sim.broadcast(p, [.text(main, ops)]) }
                default:
                    let len = doc.count
                    let ops: [TextOp]
                    if len > 0 && rng.chance(35) {
                        let pos = rng.below(len)
                        ops = doc.delete(at: pos, length: 1 + rng.below(min(len - pos, 8)))
                    } else {
                        ops = [try doc.insert(randomText(&rng, maxPieces: k == 2 ? 120 : 6), at: rng.below(len + 1))!]
                    }
                    undo[p].record(ops)
                    sim.broadcast(p, [.text(main, ops)])
                }
            }
            sim.heal()
            sim.assertConverged("record \(k)")
            try Self.write("swift-\(k)", "Swift simulation seed \(String(seed, radix: 16)), \(n) peers, with local undo/redo",
                           sim.frames, seed: seed)
        }
    }
}
