import FlashTeXCollabCore
@testable import FlashTeXCollabSession
import Foundation
import Network
import XCTest

/// An editor stand-in: an NSMutableString with a selection and a
/// composition flag, replaying the session's changes as `NSTextStorage`
/// would and typing through the binding as the Mac editor does.
@MainActor
final class FakeHost: CollabTextHost {
    let text = NSMutableString()
    var selection = 0..<0
    var composing = false
    var applies = 0
    weak var binding: CollabTextBinding?

    var collabIsComposing: Bool { composing }
    var collabSelection: Range<Int> { selection }
    func collabFlushLocalEdits() {}

    func collabApply(_ changes: [TextChange], selection: Range<Int>?) {
        applies += 1
        for c in changes {
            precondition(c.location + c.length <= text.length, "change outside the text")
            text.replaceCharacters(in: NSRange(location: c.location, length: c.length), with: c.text)
        }
        if let selection { self.selection = selection }
    }

    /// Types `s` over the selection.
    func type(_ s: String) {
        let r = selection
        binding!.localReplace(r, with: s)
        text.replaceCharacters(in: NSRange(location: r.lowerBound, length: r.count), with: s)
        let end = r.lowerBound + (s as NSString).length
        selection = end..<end
    }

    /// Backspace at the caret (or deletes the selection).
    func backspace() {
        var r = selection
        if r.isEmpty {
            guard r.lowerBound > 0 else { return }
            let ns = text as NSString
            let comp = ns.rangeOfComposedCharacterSequence(at: r.lowerBound - 1)
            r = comp.location..<r.lowerBound
        }
        binding!.localReplace(r, with: "")
        text.replaceCharacters(in: NSRange(location: r.lowerBound, length: r.count), with: "")
        selection = r.lowerBound..<r.lowerBound
    }

    func moveCaret(to p: Int) {
        selection = p..<p
        binding?.breakUndoCoalescing()
    }
}

/// A hub on loopback with one shared file, plus helpers to join guests.
@MainActor
final class Harness {
    let hub: CollabHub
    let hubSession: CollabSession
    let file: FileID
    var decision: CollabHub.Decision = .allowEdit
    var requests: [CollabHub.JoinRequest] = []
    var events: [CollabHub.Event] = []

    init(text: String, path: String = "main.tex") throws {
        hubSession = CollabSession(role: .hub, replica: UInt64.random(in: 1...UInt64.max), name: "Hub", colourIndex: 0)
        file = try hubSession.shareFile(path: path, text: text)
        hubSession.flush()
        let pins = CollabControl.SessionPins(main: path, sourceDateEpoch: 1_700_000_000, randomSeed: 42, shellEscape: "off",
                                             externalTools: false, readConfinement: true)
        hub = CollabHub(session: hubSession, identity: try CollabIdentity(), projectName: "Test", pins: pins,
                        environmentDigest: "test")
        hub.approve = { [unowned self] req, reply in
            self.requests.append(req)
            reply(self.decision)
        }
        hub.onEvent = { [unowned self] in self.events.append($0) }
    }

    func start(_ test: XCTestCase) throws {
        let ready = test.expectation(description: "hub ready")
        let prev = hub.onEvent
        hub.onEvent = { e in
            prev(e)
            if case .ready = e { ready.fulfill() }
        }
        try hub.start(loopbackOnly: true, advertise: false)
        test.wait(for: [ready], timeout: 10)
        hub.onEvent = prev
    }

    func invite() -> CollabInvite { hub.makeInvite(addresses: ["127.0.0.1"], hostName: nil) }

    func guest(_ invite: CollabInvite? = nil, name: String = "Guest") -> CollabGuest {
        let g = CollabGuest(invite: invite ?? self.invite(), displayName: name)
        g.endpointsOverride = [.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: hub.port!)!)]
        return g
    }
}

extension XCTestCase {
    /// Spins the main run loop until `condition` holds or `timeout` passes.
    @MainActor
    func waitUntil(_ timeout: TimeInterval = 10, _ what: String = "condition", _ condition: () -> Bool) {
        let end = Date().addingTimeInterval(timeout)
        while !condition() {
            if Date() > end { XCTFail("timed out waiting for \(what)"); return }
            RunLoop.main.run(until: Date().addingTimeInterval(0.005))
        }
    }

    @MainActor
    func join(_ g: CollabGuest) -> CollabSession? {
        g.connect()
        waitUntil(10, "join") { g.state == .joined || { if case .ended = g.state { return true } else { return false } }() }
        return g.session
    }
}
