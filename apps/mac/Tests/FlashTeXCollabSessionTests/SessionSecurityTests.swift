import FlashTeXCollabCore
@testable import FlashTeXCollabSession
import Foundation
import Network
import XCTest

/// The hub against a misbehaving guest or an unauthenticated peer (security
/// review of #1530): file-map operations, viewers, pre-join frames,
/// per-address and participant caps, invitation expiry, awareness floods,
/// and the bound on operations held for a composition.
@MainActor
final class SessionSecurityTests: XCTestCase {
    private func raw(_ h: Harness) -> CollabConnection {
        CollabConnection(NWConnection(to: .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: h.hub.port!)!),
                                      using: CollabTLS.guestParameters(pinned: h.hub.identity.fingerprint)))
    }

    /// P1 shares no creates, renames or deletes: a guest's file-map
    /// operations (even under its own ids) never integrate at the hub and are
    /// never relayed, so a guest cannot point a shared file at `.git/config`.
    func testGuestFileMapOperationsNeverIntegrate() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        let g = h.guest(), other = h.guest(name: "Carol")
        let s = try XCTUnwrap(join(g)), s3 = try XCTUnwrap(join(other))
        waitUntil(5, "sync") { s.text(of: h.file) == "x" && s3.text(of: h.file) == "x" }
        let before = h.hubSession.project.files.digest
        let create = try s.project.files.create(FileID.random(), kind: .text, path: ".git/config")
        let rename = try s.project.files.rename(h.file, to: ".git/config")
        let delete = try s.project.files.setDeleted(h.file, true)
        g.connection!.send(.update(seq: 50, sections: [.fileMap([create, rename, delete])]))
        waitUntil(5, "drop") { h.events.contains { if case .fileOpsDropped(_, 3) = $0 { return true } else { return false } } }
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertEqual(h.hubSession.project.files.digest, before)
        XCTAssertEqual(h.hubSession.path(of: h.file), "main.tex")
        XCTAssertEqual(s3.path(of: h.file), "main.tex", "nothing was relayed")
        XCTAssertEqual(s3.textFiles.count, 1)
        g.leave(); other.leave(); h.hub.stop()
    }

    /// A viewer's text operations, under its own ids, are dropped by the hub.
    func testViewerOperationsAreDroppedByTheHub() throws {
        let h = try Harness(text: "read me")
        try h.start(self)
        h.decision = .allowView
        let g = h.guest()
        let s = try XCTUnwrap(join(g))
        waitUntil(5, "sync") { s.text(of: h.file) == "read me" }
        let ops = try XCTUnwrap(s.project.text(h.file)).replace(utf16Range: 0..<0, with: "EDIT ")
        g.connection!.send(.update(seq: 1, sections: [.text(h.file, ops)]))
        waitUntil(5, "drop") { h.events.contains { if case .forgedDropped = $0 { return true } else { return false } } }
        XCTAssertEqual(h.hubSession.text(of: h.file), "read me")
        g.leave(); h.hub.stop()
    }

    /// Before `join`, a frame over 4 KiB closes the connection without being
    /// buffered or parsed.
    func testOversizedPreJoinFrameIsRefused() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        let c = raw(h)
        var closed: String?
        c.onClosed = { closed = $0 }
        c.onReady = {
            let len = 64 * 1024
            var frame: [UInt8] = [UInt8(len & 0xFF), UInt8(len >> 8 & 0xFF), UInt8(len >> 16 & 0xFF), 0, CollabWire.Kind.join]
            frame += [UInt8](repeating: 0x20, count: len - 1)
            c.nw.send(content: Data(frame), completion: .contentProcessed { _ in })
        }
        c.start()
        var got: [CollabMessage] = []
        c.onMessage = { got.append($0) }
        waitUntil(10, "refusal and close") { closed != nil }
        XCTAssertEqual(got, [.error(.init(code: "bad_frame", message: "frame length 65536 is outside 1…4096"))])
        XCTAssertTrue(h.requests.isEmpty)
        h.hub.stop()
    }

    /// One address may hold only a few connections at once.
    func testConnectionsPerAddressAreCapped() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        h.hub.limits.maxConnectionsPerAddress = 2
        var conns: [CollabConnection] = []
        var closedCount = 0
        for _ in 0..<3 {
            let c = raw(h)
            c.onClosed = { _ in closedCount += 1 }
            c.start()
            conns.append(c)
            RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        }
        waitUntil(10, "third refused") { closedCount == 1 }
        XCTAssertTrue(h.events.contains { if case .refused = $0 { return true } else { return false } })
        for c in conns { c.close(reason: "done") }
        h.hub.stop()
    }

    /// IPv6 peers count against the per-address cap by their /64.
    func testIPv6PeersCountByTheirSlash64() {
        func address(_ h: String) -> String {
            CollabConnection(NWConnection(host: NWEndpoint.Host(h), port: 9, using: .tcp)).remoteAddress
        }
        XCTAssertEqual(address("2001:db8:1:2:aaaa::1"), address("2001:db8:1:2:bbbb::9"))
        XCTAssertEqual(address("2001:db8:1:2:aaaa::1"), "20010db800010002::/64")
        XCTAssertNotEqual(address("2001:db8:1:3::1"), address("2001:db8:1:2::1"))
        XCTAssertEqual(address("192.168.1.5"), "192.168.1.5")
    }

    /// An invitation stops working after its lifetime.
    func testInvitationExpires() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        h.hub.inviteLifetime = 0.2
        let inv = h.invite()
        RunLoop.main.run(until: Date().addingTimeInterval(0.4))
        let g = h.guest(inv)
        g.connect()
        waitUntil(10, "refusal") { if case .ended = g.state { return true } else { return false } }
        XCTAssertTrue(h.requests.isEmpty, "an expired invitation never reaches approval")
        XCTAssertNil(g.session)
        h.hub.stop()
    }

    /// The participant cap counts joiners still waiting for approval.
    func testParticipantCapCountsPendingJoiners() throws {
        let h = try Harness(text: "x")
        try h.start(self)
        h.hub.limits.maxParticipants = 2 // the hub and one more
        var pending: [(CollabHub.Decision) -> Void] = []
        h.hub.approve = { _, reply in pending.append(reply) }
        let a = h.guest(name: "A"), b = h.guest(name: "B")
        a.connect()
        waitUntil(5, "A waits") { pending.count == 1 }
        b.connect()
        waitUntil(10, "B refused") { if case .ended = b.state { return true } else { return false } }
        XCTAssertEqual(b.state, .ended("The session is full."))
        pending[0](.allowEdit)
        waitUntil(5, "A joins") { a.state == .joined }
        a.leave(); h.hub.stop()
    }

    /// Awareness beyond the per-second budget is dropped at the hub.
    func testAwarenessIsRateLimited() throws {
        let h = try Harness(text: "0123456789")
        try h.start(self)
        h.hub.limits.awarenessPerSecond = 5
        let g = h.guest(), other = h.guest(name: "Carol")
        let s = try XCTUnwrap(join(g)), s3 = try XCTUnwrap(join(other))
        waitUntil(5, "sync") { s3.text(of: h.file) != nil }
        var seen = 0
        s3.onPresenceChanged = { seen += 1 }
        RunLoop.main.run(until: Date().addingTimeInterval(1.2)) // the bucket refills
        seen = 0
        for i in 0..<100 {
            var a = s.localAwareness
            a.seq = UInt64(1000 + i)
            g.connection!.send(.awareness(a))
        }
        RunLoop.main.run(until: Date().addingTimeInterval(0.5))
        XCTAssertLessThanOrEqual(seen, 10, "the bucket (5) plus refill, plus heartbeats, passes through")
        XCTAssertGreaterThan(seen, 0)
        g.leave(); other.leave(); h.hub.stop()
    }

    /// Remote operations held for a composition are bounded: past the bound
    /// the editor commits its composition and everything held lands.
    func testHeldOperationsAreBounded() throws {
        let saved = CollabTextBinding.maxHeld
        CollabTextBinding.maxHeld = 5
        defer { CollabTextBinding.maxHeld = saved }
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
        guestHost.composing = true
        hubHost.moveCaret(to: 3)
        for ch in "0123456789" {
            hubHost.type(String(ch))
            RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        }
        waitUntil(5, "convergence") { guestHost.text as String == "abc0123456789" }
        XCTAssertEqual(guestHost.endedCompositions, 1)
        XCTAssertLessThanOrEqual(gb.heldCount, 5)
        g.leave(); h.hub.stop()
    }
}
