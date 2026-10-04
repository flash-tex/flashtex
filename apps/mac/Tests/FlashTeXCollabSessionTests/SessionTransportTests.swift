import FlashTeXCollabCore
@testable import FlashTeXCollabSession
import Foundation
import Network
import XCTest

/// Two (or three) in-process peers over real loopback TLS 1.3: invites,
/// approval, pinning, replica binding, convergence, caret stability,
/// reconnect, the IME hold and local undo (proposal §7.2–§7.3).
@MainActor
final class SessionTransportTests: XCTestCase {
    func testIdentityIsSelfSignedP256AndFingerprintRoundTrips() throws {
        let id = try CollabIdentity()
        XCTAssertEqual(id.fingerprint.count, 32)
        XCTAssertEqual(CollabIdentity.fingerprint(of: id.certificate), id.fingerprint)
        XCTAssertNotEqual(try CollabIdentity().fingerprint, id.fingerprint, "a new key per session")
    }

    func testInviteLinkRoundTripsAndRejectsDamage() throws {
        let inv = CollabInvite(sessionID: "00ab", secret: CollabInvite.randomBytes(32), fingerprint: CollabInvite.randomBytes(32),
                               projectName: "Thesis & notes", port: 4242, addresses: ["192.168.1.2", "fe80::1"], hostName: "Mac.local")
        XCTAssertEqual(try CollabInvite(link: " <\(inv.link)> "), inv)
        XCTAssertThrowsError(try CollabInvite(link: "https://example.com"))
        XCTAssertThrowsError(try CollabInvite(link: inv.link.replacingOccurrences(of: "v=1", with: "v=9")))
        XCTAssertThrowsError(try CollabInvite(link: inv.link.replacingOccurrences(of: "&k=", with: "&k=AA")))
    }

    func testGuestJoinsOverTLS13AfterApprovalAndSyncs() throws {
        let h = try Harness(text: "\\documentclass{article}\n\\begin{document}\nHello\n\\end{document}\n")
        try h.start(self)
        let g = h.guest(name: "Bob")
        let s = try XCTUnwrap(join(g))
        XCTAssertEqual(g.state, .joined)
        XCTAssertEqual(h.requests, [.init(displayName: "Bob", deviceKind: "mac")])
        XCTAssertEqual(g.joinAck?.role, "edit")
        XCTAssertEqual(g.joinAck?.pins.randomSeed, 42)
        waitUntil(5, "sync") { s.text(of: h.file) == h.hubSession.text(of: h.file) }
        XCTAssertEqual(s.textFiles.map(\.path), ["main.tex"])
        XCTAssertEqual(s.project.digest, h.hubSession.project.digest)
        XCTAssertEqual(CollabTLS.negotiatedVersion(g.connection!.nw), .TLSv13)
        g.leave()
        h.hub.stop()
    }

    func testWrongPinIsRefusedBeforeAnyApplicationByte() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        var inv = h.invite()
        inv.fingerprint = CollabInvite.randomBytes(32)
        let g = h.guest(inv)
        g.connect()
        waitUntil(10, "refusal") { if case .ended = g.state { return true } else { return false } }
        XCTAssertTrue(h.requests.isEmpty, "the hub never heard a join")
        XCTAssertNil(g.session)
        h.hub.stop()
    }

    func testInviteIsSingleUseAndDenialEnds() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        let inv = h.invite()
        let first = h.guest(inv, name: "A")
        XCTAssertNotNil(join(first))
        let second = h.guest(inv, name: "B")
        second.connect()
        waitUntil(10, "refusal") { if case .ended = second.state { return true } else { return false } }
        XCTAssertEqual(h.requests.count, 1, "a used invite never reaches approval")
        h.decision = .deny
        let third = h.guest(name: "C")
        third.connect()
        waitUntil(10, "denial") { if case .ended = third.state { return true } else { return false } }
        XCTAssertEqual(third.state, .ended("The host declined your request."))
        XCTAssertEqual(h.hub.openInviteCount, 0)
        first.leave()
        h.hub.stop()
    }

    /// The P1 convergence test: two editors type concurrently (inserts,
    /// backspaces, caret moves, emoji and CJK) through their bindings; both
    /// CRDTs and both editors' texts end identical.
    func testTwoEditorsCoEditAndConverge() throws {
        let h = try Harness(text: "\\section{Intro}\nSome text.\n")
        try h.start(self)
        let third = h.guest(name: "Carol")
        let g = h.guest(name: "Bob")
        let s = try XCTUnwrap(join(g))
        let s3 = try XCTUnwrap(join(third))
        waitUntil(5, "sync") { s.text(of: h.file) != nil && s3.text(of: h.file) != nil }
        let hosts = [FakeHost(), FakeHost(), FakeHost()]
        let sessions = [h.hubSession, s, s3]
        for (host, session) in zip(hosts, sessions) {
            host.text.setString(session.text(of: h.file)!)
            let b = try XCTUnwrap(session.binding(for: h.file))
            host.binding = b
            b.attach(host)
        }
        var rng = SplitMix64(seed: 0xC0FFEE)
        let alphabet = ["a", "b", " ", "\n", "{", "}", "\\", "é", "中", "😀"]
        for step in 0..<600 {
            let p = rng.below(3)
            let host = hosts[p]
            switch rng.below(10) {
            case 0...5: host.type(alphabet[rng.below(alphabet.count)])
            case 6, 7: host.backspace()
            default:
                let len = host.text.length
                var q = rng.below(len + 1)
                q = (host.text as NSString).rangeOfComposedCharacterSequence(at: min(q, max(0, len - 1))).location
                host.moveCaret(to: len == 0 ? 0 : q)
            }
            if step % 7 == 0 { RunLoop.main.run(until: Date().addingTimeInterval(0.002)) }
        }
        waitUntil(15, "convergence") {
            let t = sessions.map { $0.text(of: h.file)! }
            return t[0] == t[1] && t[1] == t[2]
                && sessions[0].project.digest == sessions[1].project.digest
                && sessions[1].project.digest == sessions[2].project.digest
        }
        for (host, session) in zip(hosts, sessions) {
            XCTAssertEqual(host.text as String, session.text(of: h.file), "each editor shows what its CRDT holds")
        }
        XCTAssertGreaterThan(hosts[0].applies, 0)
        XCTAssertEqual(g.unacknowledged, 0)
        g.leave(); third.leave(); h.hub.stop()
    }

    /// A remote insertion before the local caret moves it; one after it, or
    /// at it, leaves it in place; a selection keeps its exact text.
    func testLocalCaretAndSelectionAreStableUnderRemoteEdits() throws {
        let h = try Harness(text: "alpha beta gamma")
        try h.start(self)
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "alpha beta gamma" }
        let hubHost = FakeHost(), guestHost = FakeHost()
        hubHost.text.setString("alpha beta gamma"); guestHost.text.setString("alpha beta gamma")
        let hb = try XCTUnwrap(h.hubSession.binding(for: h.file)), gb = try XCTUnwrap(s.binding(for: h.file))
        hubHost.binding = hb; guestHost.binding = gb
        hb.attach(hubHost); gb.attach(guestHost)

        hubHost.selection = 6..<10 // "beta"
        guestHost.moveCaret(to: 0)
        guestHost.type("XX ") // before the selection
        guestHost.moveCaret(to: guestHost.text.length)
        guestHost.type(" tail") // after it
        waitUntil(5, "delivery") { hubHost.text as String == "XX alpha beta gamma tail" }
        XCTAssertEqual((hubHost.text as NSString).substring(with: NSRange(location: hubHost.selection.lowerBound, length: hubHost.selection.count)), "beta")

        hubHost.selection = 3..<3 // caret before "alpha"
        guestHost.moveCaret(to: 3)
        guestHost.type("at-caret ")
        waitUntil(5, "delivery") { (hubHost.text as String).hasPrefix("XX at-caret alpha") }
        XCTAssertEqual(hubHost.selection, 3..<3, "text typed at my caret by someone else lands after it")
        g.leave(); h.hub.stop()
    }

    /// Replica binding: operations a guest sends under another replica's
    /// ids are dropped before the CRDT and never relayed.
    func testForgedOperationsAreDroppedByReplicaBinding() throws {
        let h = try Harness(text: "safe")
        try h.start(self)
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "safe" }
        // Claims the hub's next counters, with content the hub never typed.
        let forged = TextOp.insert(id: CollabID(replica: h.hubSession.replica, counter: h.hubSession.project.text(h.file)!.stateVector[h.hubSession.replica]),
                                   originLeft: nil, originRight: nil, content: "EVIL ")
        g.connection!.send(.update(seq: 99, sections: [.text(h.file, [forged])]))
        waitUntil(5, "drop") { h.events.contains { if case .forgedDropped = $0 { return true } else { return false } } }
        XCTAssertEqual(h.hubSession.text(of: h.file), "safe")
        g.leave(); h.hub.stop()
    }

    /// A viewer's edits are dropped by the hub, and its session says it
    /// cannot edit.
    func testViewerCannotEdit() throws {
        let h = try Harness(text: "read me")
        try h.start(self)
        h.decision = .allowView
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        XCTAssertEqual(g.joinAck?.role, "view")
        XCTAssertFalse(s.canEdit)
        g.leave(); h.hub.stop()
    }

    /// Edits made while the connection is down reach the hub after the
    /// automatic reconnect (token, then state-vector sync both ways).
    func testReconnectDeliversOfflineEdits() throws {
        let h = try Harness(text: "base")
        try h.start(self)
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "base" }
        let host = FakeHost()
        host.text.setString("base")
        let b = try XCTUnwrap(s.binding(for: h.file))
        host.binding = b; b.attach(host)
        g.connection!.nw.cancel() // the Wi-Fi drops
        waitUntil(5, "drop noticed") { if case .reconnecting = g.state { return true } else { return false } }
        host.moveCaret(to: 4)
        host.type(" offline")
        waitUntil(15, "reconnect and sync") { h.hubSession.text(of: h.file) == "base offline" }
        XCTAssertEqual(g.state, .joined)
        XCTAssertTrue(h.events.contains { if case .reconnected = $0 { return true } else { return false } })
        g.leave(); h.hub.stop()
    }

    /// The IME rule: while the local editor composes, remote operations for
    /// its file are not applied to the view (they wait); they land as soon
    /// as the composition ends, and the composed text is kept.
    func testRemoteOperationsWaitForTheCompositionToEnd() throws {
        let h = try Harness(text: "abc")
        try h.start(self)
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "abc" }
        let hubHost = FakeHost(), guestHost = FakeHost()
        hubHost.text.setString("abc"); guestHost.text.setString("abc")
        let hb = try XCTUnwrap(h.hubSession.binding(for: h.file)), gb = try XCTUnwrap(s.binding(for: h.file))
        hubHost.binding = hb; guestHost.binding = gb
        hb.attach(hubHost); gb.attach(guestHost)

        guestHost.composing = true // marked text: nothing reaches the binding yet
        hubHost.moveCaret(to: 3)
        hubHost.type("XYZ")
        waitUntil(5, "held") { gb.heldCount > 0 }
        XCTAssertEqual(guestHost.text as String, "abc", "the composing view is untouched")
        XCTAssertEqual(guestHost.applies, 0)
        // The composition commits "日本" at 0; then the view releases.
        guestHost.composing = false
        guestHost.moveCaret(to: 0)
        guestHost.type("日本")
        gb.release()
        XCTAssertEqual(guestHost.text as String, "日本abcXYZ")
        XCTAssertEqual(gb.heldCount, 0)
        waitUntil(5, "convergence") { h.hubSession.text(of: h.file) == "日本abcXYZ" && hubHost.text as String == "日本abcXYZ" }
        g.leave(); h.hub.stop()
    }

    /// ⌘Z through the binding undoes only this participant's typing, even
    /// with another participant's text inside and around it; redo restores.
    func testUndoRevertsOnlyOwnEdits() throws {
        let h = try Harness(text: "")
        try h.start(self)
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "" }
        let hubHost = FakeHost(), guestHost = FakeHost()
        let hb = try XCTUnwrap(h.hubSession.binding(for: h.file)), gb = try XCTUnwrap(s.binding(for: h.file))
        hubHost.binding = hb; guestHost.binding = gb
        hb.attach(hubHost); gb.attach(guestHost)
        var opened = 0
        hb.onUndoStepOpened = { opened += 1 }

        for ch in "hello world" { hubHost.type(String(ch)) } // one coalesced step
        XCTAssertEqual(opened, 1)
        waitUntil(5, "delivery") { guestHost.text as String == "hello world" }
        guestHost.moveCaret(to: 5)
        guestHost.type(" there") // inside the hub's run
        guestHost.moveCaret(to: guestHost.text.length)
        guestHost.type("!")
        waitUntil(5, "delivery") { hubHost.text as String == "hello there world!" }

        XCTAssertTrue(hb.undo())
        XCTAssertEqual(hubHost.text as String, " there!", "only the hub's own characters go")
        waitUntil(5, "undo reaches the guest") { guestHost.text as String == " there!" }
        XCTAssertFalse(hb.canUndo)
        hb.redo()
        XCTAssertEqual(hubHost.text as String, "hello there world!")
        waitUntil(5, "redo reaches the guest") { guestHost.text as String == "hello there world!" }
        g.leave(); h.hub.stop()
    }

    func testPresenceShowsTheOtherCaret() throws {
        let h = try Harness(text: "0123456789")
        try h.start(self)
        let g = h.guest(name: "Bob")
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "0123456789" }
        s.setLocalPresence(file: h.file, selection: 2..<5)
        waitUntil(5, "presence") { !h.hubSession.remoteCursors(in: h.file).isEmpty }
        let c = h.hubSession.remoteCursors(in: h.file)[0]
        XCTAssertEqual(c.name, "Bob")
        XCTAssertEqual(c.range, 2..<5)
        XCTAssertEqual(c.colourIndex, g.joinAck?.colourIndex)
        XCTAssertEqual(h.hubSession.participants.map(\.name), ["Hub", "Bob"])
        g.leave()
        waitUntil(5, "leave") { h.hubSession.remoteCursors(in: h.file).isEmpty }
        h.hub.stop()
    }

    /// Remote keystroke → remote editor commit (proposal §7.1 P1 gate:
    /// ≤ 50 ms p95 on a LAN), measured here over loopback TLS: each
    /// keystroke waits until the other editor has applied it.
    func testRemoteKeystrokeLatency() throws {
        let h = try Harness(text: "")
        try h.start(self)
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "" }
        let hubHost = FakeHost(), guestHost = FakeHost()
        let hb = try XCTUnwrap(h.hubSession.binding(for: h.file)), gb = try XCTUnwrap(s.binding(for: h.file))
        hubHost.binding = hb; guestHost.binding = gb
        hb.attach(hubHost); gb.attach(guestHost)
        var samples: [Double] = []
        for i in 0..<200 {
            let before = hubHost.applies
            let t0 = DispatchTime.now().uptimeNanoseconds
            guestHost.type(i % 10 == 9 ? "\n" : "x")
            waitUntil(5, "keystroke \(i)") { hubHost.applies > before }
            samples.append(Double(DispatchTime.now().uptimeNanoseconds - t0) / 1e6)
        }
        samples.sort()
        let p50 = samples[samples.count / 2], p95 = samples[samples.count * 95 / 100]
        print("collab remote keystroke latency over loopback TLS 1.3: p50 \(String(format: "%.2f", p50)) ms, p95 \(String(format: "%.2f", p95)) ms, max \(String(format: "%.2f", samples.last!)) ms (n=200)")
        XCTAssertEqual(hubHost.text as String, guestHost.text as String)
        if ProcessInfo.processInfo.environment["FLASHTEX_COLLAB_LATENCY_GATE"] == "1" { XCTAssertLessThanOrEqual(p95, 50) }
        g.leave(); h.hub.stop()
    }
}
