import Foundation
import XCTest
@testable import FlashTeXMac

/// Pure pairing-flow model: every transition of `PairingFlow.Machine`, stale
/// input rejection, journal/store persistence, and the accessibility text.
/// Owner: mac-pairing-ui.
final class PairingFlowMachineTests: XCTestCase {
    typealias M = PairingFlow.Machine
    typealias P = PairingFlow.Phase

    let t0 = Date(timeIntervalSince1970: 1_800_000_000)
    func attempt(_ g: Int, code: String = "123456", lifetime: TimeInterval = 120) -> PairingFlow.Attempt {
        .init(generation: g, code: code, pairId: "pair-\(code)", startedAt: t0, expiresAt: t0.addingTimeInterval(lifetime))
    }
    var a1: PairingFlow.Attempt { attempt(1) }
    var a2: PairingFlow.Attempt { attempt(2, code: "654321") }
    var later: Date { t0.addingTimeInterval(10) }

    // MARK: advertising and code issue

    func testAdvertisingTogglesBetweenOffAndAdvertising() {
        var m = M()
        XCTAssertEqual(m.phase, .off)
        XCTAssertEqual(m.apply(.advertising(true), now: t0), .none)
        XCTAssertEqual(m.phase, .advertising)
        XCTAssertEqual(m.apply(.advertising(true), now: t0), .none) // idempotent
        XCTAssertEqual(m.apply(.advertising(false), now: t0), .none)
        XCTAssertEqual(m.phase, .off)
    }

    func testCodeIssuedShowsCodePersistsAndAnnouncesDigits() {
        var m = M()
        let out = m.apply(.codeIssued(a1), now: t0)
        XCTAssertEqual(m.phase, .codeShown(a1))
        XCTAssertEqual(m.generation, 1)
        XCTAssertTrue(m.isAdvertising, "a code implies the listener is up")
        XCTAssertEqual(out.effects.first, .persist(a1))
        XCTAssertEqual(out.effects.last, .announce("Pairing code 1 2, 3 4, 5 6, valid for 120 seconds."))
    }

    func testCodeIssuedWithOlderGenerationIsStale() {
        var m = M(phase: .codeShown(a2), generation: 2, isAdvertising: true)
        XCTAssertEqual(m.apply(.codeIssued(a1), now: t0), .staleInput)
        XCTAssertEqual(m.phase, .codeShown(a2), "an older code never replaces the current one")
    }

    func testNewCodeReplacesAnyEarlierPhase() {
        for start in [P.off, .advertising, .paired(.init(pairId: "x", companionName: "X", generation: 1)),
                      .failed(reason: "r", generation: 1), .interrupted(a1, .relaunch, detail: "d")] {
            var m = M(phase: start, generation: 1, isAdvertising: false)
            m.apply(.codeIssued(a2), now: t0)
            XCTAssertEqual(m.phase, .codeShown(a2), "\(start)")
        }
    }

    // MARK: happy path

    func testVerifyingThenConfirmedClearsJournal() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        let v = m.apply(.bootstrapSessionOpened(generation: 1), now: later)
        XCTAssertEqual(m.phase, .verifying(a1))
        XCTAssertEqual(v.effects, [.announce("A companion connected; verifying the code.")])
        let c = m.apply(.confirmed(pairId: a1.pairId, companionName: "iPad", generation: 1), now: later)
        XCTAssertEqual(m.phase, .paired(.init(pairId: a1.pairId, companionName: "iPad", generation: 1)))
        XCTAssertEqual(c.effects, [.clearJournal, .announce("Paired with iPad.")])
        // Idempotent re-confirmation from a second observer is ignored, not stale.
        XCTAssertEqual(m.apply(.confirmed(pairId: a1.pairId, companionName: "iPad", generation: 1), now: later), .ignoredInput)
        XCTAssertEqual(m.apply(.dismiss, now: later), .none)
        XCTAssertEqual(m.phase, .advertising)
    }

    func testConfirmedStraightFromCodeShownWithoutSessionEvent() {
        // The transport today only publishes the stored pair, not the session open.
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.confirmed(pairId: a1.pairId, companionName: "iPad", generation: 1), now: later)
        XCTAssertEqual(m.phase, .paired(.init(pairId: a1.pairId, companionName: "iPad", generation: 1)))
    }

    func testOtherCompanionConnectingDuringVerificationReturnsToCodeShown() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.bootstrapSessionOpened(generation: 1), now: later)
        XCTAssertEqual(m.apply(.otherCompanionConnected(generation: 1), now: later), .none)
        XCTAssertEqual(m.phase, .codeShown(a1))
        XCTAssertEqual(m.apply(.otherCompanionConnected(generation: 1), now: later), .ignoredInput)
    }

    // MARK: stale reconnects never overwrite the current pairing

    func testStaleConfirmationForReplacedCodeIsIgnored() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.cancel, now: later)
        m.apply(.codeIssued(a2), now: later)
        // A session that authenticated with the first code reports late.
        let stale = m.apply(.confirmed(pairId: a1.pairId, companionName: "Old iPad", generation: 1), now: later)
        XCTAssertTrue(stale.stale)
        XCTAssertEqual(m.phase, .codeShown(a2))
        // Even with the current generation, a different pair id is not this attempt.
        let wrongPair = m.apply(.confirmed(pairId: a1.pairId, companionName: "Old iPad", generation: 2), now: later)
        XCTAssertTrue(wrongPair.stale)
        XCTAssertEqual(m.phase, .codeShown(a2))
        // The real one still lands.
        m.apply(.confirmed(pairId: a2.pairId, companionName: "New iPad", generation: 2), now: later)
        XCTAssertEqual(m.phase, .paired(.init(pairId: a2.pairId, companionName: "New iPad", generation: 2)))
    }

    func testStaleSessionOpenExpiryAndPeerGoneAreIgnored() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.codeIssued(a2), now: later)
        XCTAssertEqual(m.phase, .codeShown(a2))
        XCTAssertTrue(m.apply(.bootstrapSessionOpened(generation: 1), now: later).stale)
        XCTAssertEqual(m.phase, .codeShown(a2))
        XCTAssertTrue(m.apply(.codeExpired(generation: 1), now: later).stale)
        XCTAssertEqual(m.phase, .codeShown(a2))
        XCTAssertTrue(m.apply(.peerGone(pairId: a1.pairId, reason: "closed", generation: 1), now: later).stale)
        XCTAssertEqual(m.phase, .codeShown(a2))
        XCTAssertTrue(m.apply(.withdrawn(generation: 1, reason: "x"), now: later).stale)
        XCTAssertEqual(m.phase, .codeShown(a2))
    }

    func testStaleReconnectAfterPairedDoesNotOverwritePaired() {
        var m = M()
        m.apply(.codeIssued(a2), now: t0)
        m.apply(.confirmed(pairId: a2.pairId, companionName: "New", generation: 2), now: later)
        let paired = m.phase
        // Old session (other pair id) closes and reports late.
        XCTAssertTrue(m.apply(.peerGone(pairId: "pair-old", reason: "closed", generation: 2), now: later).stale)
        XCTAssertEqual(m.phase, paired)
        XCTAssertTrue(m.apply(.confirmed(pairId: "pair-old", companionName: "Old", generation: 1), now: later).stale)
        XCTAssertEqual(m.phase, paired)
        // The current companion's own close ends the banner.
        XCTAssertEqual(m.apply(.peerGone(pairId: a2.pairId, reason: "peer closed", generation: 2), now: later), .none)
        XCTAssertEqual(m.phase, .advertising)
    }

    func testRestoredOlderAttemptIsStaleAgainstNewerGeneration() {
        var m = M(phase: .advertising, generation: 3, isAdvertising: true)
        XCTAssertTrue(m.apply(.restored(a1), now: t0).stale)
        XCTAssertEqual(m.phase, .advertising)
    }

    // MARK: expiry, cancel, dismiss

    func testExpiryWhileShownOrVerifyingBecomesFailedAndClearsJournal() {
        for open in [false, true] {
            var m = M()
            m.apply(.codeIssued(a1), now: t0)
            if open { m.apply(.bootstrapSessionOpened(generation: 1), now: later) }
            let out = m.apply(.codeExpired(generation: 1), now: t0.addingTimeInterval(121))
            XCTAssertEqual(m.phase, .failed(reason: "The pairing code expired before a companion paired.", generation: 1))
            XCTAssertEqual(out.effects.first, .clearJournal)
            XCTAssertTrue(m.phase.canDismiss(now: t0))
            XCTAssertTrue(m.phase.canShowCode(now: t0))
            XCTAssertFalse(m.phase.canCancel(now: t0))
            m.apply(.dismiss, now: t0)
            XCTAssertEqual(m.phase, .advertising)
        }
    }

    // MARK: rolling codes (#355 follow-up)

    /// The replacement the controller offers: next generation, fresh code,
    /// minted at `now` for a full `codeLifetime`.
    func replacement(for a: PairingFlow.Attempt, now: Date, code: String = "987654") -> PairingFlow.Attempt {
        .init(generation: a.generation + 1, code: code, pairId: "pair-\(code)", startedAt: now,
              expiresAt: now.addingTimeInterval(Pairing.codeLifetime))
    }

    func testExpiryWhileShownWithReplacementRollsToANewCode() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        let expiry = t0.addingTimeInterval(120)
        let next = replacement(for: a1, now: expiry)
        let out = m.apply(.codeExpired(generation: 1, replacement: next), now: expiry)
        guard case .codeShown(let rolled) = m.phase else { return XCTFail("\(m.phase)") }
        XCTAssertEqual(rolled.generation, 2)
        XCTAssertEqual(m.generation, 2, "the machine follows the rolled generation")
        XCTAssertNotEqual(rolled.code, a1.code)
        XCTAssertEqual(rolled.code, "987654")
        XCTAssertEqual(rolled.expiresAt, a1.expiresAt.addingTimeInterval(Pairing.codeLifetime), "one more lifetime, never longer")
        XCTAssertEqual(rolled.rolls, 1, "the machine counts the roll")
        XCTAssertTrue(rolled.canRoll)
        XCTAssertEqual(out.effects, [
            .persist(rolled),
            .resumeTransport(rolled),
            .announce("The pairing code expired unused. New pairing code 9 8, 7 6, 5 4, valid for 120 seconds."),
        ])
        XCTAssertFalse(out.stale)
        XCTAssertFalse(out.ignored)
        XCTAssertTrue(m.isAdvertising)
        XCTAssertEqual(m.phase.detail(now: expiry.addingTimeInterval(30)),
                       "The previous code expired unused; enter the new code on the companion. Expires in 90 s.")
        XCTAssertTrue(m.phase.canCancel(now: expiry))
        XCTAssertFalse(m.phase.canShowCode(now: expiry))
    }

    func testOldCodeIsRefusedAfterARoll() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        let expiry = t0.addingTimeInterval(120)
        m.apply(.codeExpired(generation: 1, replacement: replacement(for: a1, now: expiry)), now: expiry)
        guard case .codeShown(let rolled) = m.phase else { return XCTFail("\(m.phase)") }
        // A companion that pasted the old code late: its session and hello are stale.
        XCTAssertTrue(m.apply(.bootstrapSessionOpened(generation: 1), now: expiry).stale)
        XCTAssertTrue(m.apply(.confirmed(pairId: a1.pairId, companionName: "Late iPad", generation: 1), now: expiry).stale)
        XCTAssertEqual(m.phase, .codeShown(rolled))
        // Same pair id but the new generation: still refused (the new code has its own pair id).
        XCTAssertTrue(m.apply(.confirmed(pairId: a1.pairId, companionName: "Late iPad", generation: 2), now: expiry).stale)
        // A duplicate expiry report for the old code (the second timer) is stale, not a failure.
        XCTAssertTrue(m.apply(.codeExpired(generation: 1), now: expiry.addingTimeInterval(0.25)).stale)
        XCTAssertEqual(m.phase, .codeShown(rolled))
        // The new code pairs.
        m.apply(.bootstrapSessionOpened(generation: 2), now: expiry.addingTimeInterval(5))
        let out = m.apply(.confirmed(pairId: rolled.pairId, companionName: "New iPad", generation: 2), now: expiry.addingTimeInterval(6))
        XCTAssertEqual(m.phase, .paired(.init(pairId: rolled.pairId, companionName: "New iPad", generation: 2)))
        XCTAssertEqual(out.effects.first, .clearJournal)
    }

    func testRollCapFallsIntoTheExpiredPath() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        var a = a1
        var now = t0
        for i in 1...Pairing.maxCodeRolls {
            now = a.expiresAt
            let out = m.apply(.codeExpired(generation: a.generation, replacement: replacement(for: a, now: now, code: "00000\(i)")), now: now)
            guard case .codeShown(let next) = m.phase else { return XCTFail("roll \(i): \(m.phase)") }
            XCTAssertEqual(next.rolls, i)
            XCTAssertEqual(next.generation, i + 1)
            XCTAssertEqual(out.effects.count, 3, "roll \(i)")
            a = next
        }
        XCTAssertFalse(a.canRoll, "the last code cannot roll")
        XCTAssertEqual(a.expiresAt, t0.addingTimeInterval(Double(Pairing.maxCodeRolls + 1) * Pairing.codeLifetime), "6 codes = 12 minutes of lifetime in total")
        // Expiry of the last code, even with a replacement on offer, fails as before.
        now = a.expiresAt
        let out = m.apply(.codeExpired(generation: a.generation, replacement: replacement(for: a, now: now, code: "555555")), now: now)
        XCTAssertEqual(m.phase, .failed(reason: "The pairing code expired before a companion paired.", generation: Pairing.maxCodeRolls + 1))
        XCTAssertEqual(out.effects, [.clearJournal, .announce("The pairing code expired. Show a new code to try again.")])
        XCTAssertTrue(m.phase.canDismiss(now: now))
        XCTAssertTrue(m.phase.canShowCode(now: now))
        m.apply(.dismiss, now: now)
        XCTAssertEqual(m.phase, .advertising)
    }

    func testReplacementIsIgnoredUnlessCodeIsShownWithoutACompanion() {
        let expiry = t0.addingTimeInterval(120)
        // Verifying: a companion connected; no roll.
        var v = M()
        v.apply(.codeIssued(a1), now: t0)
        v.apply(.bootstrapSessionOpened(generation: 1), now: later)
        v.apply(.codeExpired(generation: 1, replacement: replacement(for: a1, now: expiry)), now: expiry)
        XCTAssertEqual(v.phase, .failed(reason: "The pairing code expired before a companion paired.", generation: 1))
        // Interrupted: no roll.
        var i = M()
        i.apply(.codeIssued(a1), now: t0)
        i.apply(.withdrawn(generation: 1, reason: "r"), now: later)
        i.apply(.codeExpired(generation: 1, replacement: replacement(for: a1, now: expiry)), now: expiry)
        XCTAssertEqual(i.phase, .failed(reason: "The pairing code expired before it could be resumed.", generation: 1))
        // Shown, but a replacement that is not newer or not different is refused (attempt kept).
        var s = M()
        s.apply(.codeIssued(a1), now: t0)
        XCTAssertTrue(s.apply(.codeExpired(generation: 1, replacement: a1), now: expiry).ignored)
        XCTAssertTrue(s.apply(.codeExpired(generation: 1, replacement: replacement(for: a1, now: expiry, code: a1.code)), now: expiry).ignored)
        XCTAssertEqual(s.phase, .codeShown(a1))
        // No replacement at all: the pre-rolling behaviour.
        XCTAssertEqual(s.apply(.codeExpired(generation: 1), now: expiry).effects.first, .clearJournal)
        XCTAssertEqual(s.phase, .failed(reason: "The pairing code expired before a companion paired.", generation: 1))
    }

    func testExpiryWhileNotRunningIsUnchangedByRolling() {
        // Restored after a relaunch, already expired: no replacement is ever offered.
        var m = M()
        let now = t0.addingTimeInterval(500)
        m.apply(.restored(a1), now: now)
        XCTAssertEqual(m.phase, .interrupted(a1, .relaunch, detail: "the code expired while FlashTeX was not running"))
        XCTAssertFalse(m.phase.canResume(now: now))
        XCTAssertTrue(m.phase.canDismiss(now: now))
        // Even a replacement offered here is ignored: an interrupted code fails on expiry.
        let out = m.apply(.codeExpired(generation: 1, replacement: replacement(for: a1, now: now)), now: now)
        XCTAssertEqual(m.phase, .failed(reason: "The pairing code expired before it could be resumed.", generation: 1))
        XCTAssertEqual(out.effects.first, .clearJournal)
        // A rolled code that was pending at quit restores with its roll count.
        var r = M()
        var rolled = a2; rolled.rolls = 3
        r.apply(.restored(rolled), now: t0)
        XCTAssertEqual(r.attempt?.rolls, 3)
        XCTAssertTrue(r.phase.canResume(now: t0))
    }

    func testCancelAtEachCancellableStep() {
        // Code shown.
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        XCTAssertTrue(m.phase.canCancel(now: t0))
        var out = m.apply(.cancel, now: t0)
        XCTAssertEqual(out.effects, [.cancelTransport(a1), .clearJournal, .announce("Pairing cancelled.")])
        XCTAssertEqual(m.phase, .advertising)

        // Verifying.
        m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.bootstrapSessionOpened(generation: 1), now: t0)
        XCTAssertTrue(m.phase.canCancel(now: t0))
        out = m.apply(.cancel, now: t0)
        XCTAssertEqual(out.effects.first, .cancelTransport(a1))
        XCTAssertEqual(m.phase, .advertising)

        // Interrupted by the peer (transport still holds the key → cancel it).
        m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.bootstrapSessionOpened(generation: 1), now: t0)
        m.apply(.peerGone(pairId: a1.pairId, reason: "peer closed", generation: 1), now: t0)
        XCTAssertEqual(m.phase, .interrupted(a1, .peerGone, detail: "peer closed"))
        XCTAssertTrue(m.phase.canCancel(now: t0))
        out = m.apply(.cancel, now: t0)
        XCTAssertEqual(out.effects, [.cancelTransport(a1), .clearJournal, .announce("Pairing cancelled.")])

        // Interrupted by relaunch (transport never had the key → only the journal).
        m = M()
        m.apply(.restored(a1), now: t0)
        out = m.apply(.cancel, now: t0)
        XCTAssertEqual(out.effects, [.clearJournal, .announce("Pairing cancelled.")])
        XCTAssertEqual(m.phase, .off)

        // Failed: cancel doubles as dismiss.
        m = M(phase: .failed(reason: "x", generation: 1), generation: 1, isAdvertising: true)
        XCTAssertEqual(m.apply(.cancel, now: t0), .none)
        XCTAssertEqual(m.phase, .advertising)

        // Receiving: the Mac closes that pairing's session and says so; the
        // close event that follows must not overwrite the notice.
        m = M(phase: .advertising, generation: 1, isAdvertising: true)
        m.apply(.receiving(pairId: "p", companionName: "c", captureId: nil, bytes: 10, total: 100), now: t0)
        XCTAssertTrue(m.phase.canCancel(now: t0))
        out = m.apply(.cancel, now: t0)
        XCTAssertEqual(out.effects, [.closeSession(pairId: "p"), .announce("Receive cancelled.")])
        XCTAssertEqual(m.phase, .failed(reason: "Receive from c cancelled after 10 bytes; the companion can resend it with the same capture_id.", generation: 1))
        XCTAssertEqual(m.apply(.peerGone(pairId: "p", reason: "closed by the Mac", generation: 1), now: t0), .ignoredInput)
        XCTAssertTrue(m.phase.canDismiss(now: t0))
        // A resend while the notice is up replaces it with live progress.
        XCTAssertEqual(m.apply(.receiving(pairId: "p", companionName: "c", captureId: nil, bytes: 5, total: nil), now: t0), .none)
        if case .receiving = m.phase {} else { XCTFail("\(m.phase)") }

        // Nothing to cancel.
        m = M()
        XCTAssertEqual(m.apply(.cancel, now: t0), .ignoredInput)
    }

    // MARK: interruption and resume

    func testRelaunchWithValidCodeResumesThroughTransport() {
        var m = M()
        let out = m.apply(.restored(a1), now: later)
        XCTAssertEqual(m.phase, .interrupted(a1, .relaunch, detail: "FlashTeX was quit while the code was valid"))
        XCTAssertEqual(m.generation, 1)
        XCTAssertEqual(out.effects, [.announce("A pairing from a previous launch is waiting. Resume it or cancel.")])
        XCTAssertTrue(m.phase.canResume(now: later))
        XCTAssertTrue(m.phase.canCancel(now: later))
        XCTAssertFalse(m.phase.canShowCode(now: later), "must resume or cancel first")
        let r = m.apply(.resume, now: later)
        XCTAssertEqual(m.phase, .codeShown(a1))
        XCTAssertEqual(r.effects.first, .resumeTransport(a1))
        XCTAssertTrue(m.isAdvertising)
    }

    func testRelaunchWithExpiredCodeOnlyDismisses() {
        var m = M()
        let now = t0.addingTimeInterval(500)
        m.apply(.restored(a1), now: now)
        XCTAssertEqual(m.phase, .interrupted(a1, .relaunch, detail: "the code expired while FlashTeX was not running"))
        XCTAssertFalse(m.phase.canResume(now: now))
        XCTAssertFalse(m.phase.canCancel(now: now))
        XCTAssertTrue(m.phase.canDismiss(now: now))
        XCTAssertTrue(m.phase.canShowCode(now: now))
        // Resume anyway (e.g. the clock ran out between render and click) → failed, journal cleared.
        var copy = m
        let r = copy.apply(.resume, now: now)
        XCTAssertEqual(copy.phase, .failed(reason: "The pairing code expired before it could be resumed.", generation: 1))
        XCTAssertEqual(r.effects.first, .clearJournal)
        let d = m.apply(.dismiss, now: now)
        XCTAssertEqual(d.effects, [.clearJournal])
        XCTAssertEqual(m.phase, .off)
    }

    func testRestoredIsIgnoredWhileAnotherAttemptIsActive() {
        var m = M()
        m.apply(.codeIssued(a2), now: t0)
        XCTAssertEqual(m.apply(.restored(a2), now: t0), .ignoredInput)
        XCTAssertEqual(m.phase, .codeShown(a2))
    }

    func testPeerGoneDuringVerificationResumesWithoutTransportEffect() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.bootstrapSessionOpened(generation: 1), now: t0)
        m.apply(.peerGone(pairId: nil, reason: "handshake failed", generation: 1), now: t0)
        XCTAssertEqual(m.phase, .interrupted(a1, .peerGone, detail: "handshake failed"))
        let r = m.apply(.resume, now: later)
        XCTAssertEqual(m.phase, .codeShown(a1))
        XCTAssertEqual(r.effects, [.announce("Resumed pairing code 1 2, 3 4, 5 6, 110 seconds left.")], "key still served; no transport effect")
    }

    func testPeerGoneWhileCodeShownWithoutSessionIsIgnored() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        XCTAssertEqual(m.apply(.peerGone(pairId: nil, reason: "x", generation: 1), now: t0), .ignoredInput)
        XCTAssertEqual(m.phase, .codeShown(a1))
    }

    func testAdvertisingOffUnderCodeInterruptsAndResumeRestartsTransport() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.advertising(false), now: t0)
        XCTAssertEqual(m.phase, .interrupted(a1, .transportStopped, detail: "advertising was turned off"))
        XCTAssertFalse(m.isAdvertising)
        let r = m.apply(.resume, now: later)
        XCTAssertEqual(r.effects.first, .resumeTransport(a1))
        XCTAssertEqual(m.phase, .codeShown(a1))
        XCTAssertTrue(m.isAdvertising)
    }

    func testListenerFailureUnderCodeInterruptsOtherwiseFails() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.listenerFailed("listener error: port in use"), now: t0)
        XCTAssertEqual(m.phase, .interrupted(a1, .transportStopped, detail: "listener error: port in use"))
        XCTAssertEqual(m.apply(.listenerFailed("again"), now: t0), .ignoredInput)

        var n = M(phase: .advertising, generation: 1, isAdvertising: true)
        n.apply(.listenerFailed("listener error: boom"), now: t0)
        XCTAssertEqual(n.phase, .failed(reason: "listener error: boom", generation: 1))
        XCTAssertFalse(n.isAdvertising)
        n.apply(.dismiss, now: t0)
        XCTAssertEqual(n.phase, .off)
    }

    func testWithdrawnCodeInterruptsAndExpiryWhileInterruptedFails() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.withdrawn(generation: 1, reason: "the transport withdrew the code"), now: t0)
        XCTAssertEqual(m.phase, .interrupted(a1, .transportStopped, detail: "the transport withdrew the code"))
        let out = m.apply(.codeExpired(generation: 1), now: t0.addingTimeInterval(200))
        XCTAssertEqual(m.phase, .failed(reason: "The pairing code expired before it could be resumed.", generation: 1))
        XCTAssertEqual(out.effects.first, .clearJournal)
    }

    func testConfirmedWhileInterruptedByPeerStillPairs() {
        // The dropped session was not the one that completed; a second attempt
        // with the same code (still served) completes the pairing.
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        m.apply(.bootstrapSessionOpened(generation: 1), now: t0)
        m.apply(.peerGone(pairId: nil, reason: "x", generation: 1), now: t0)
        m.apply(.confirmed(pairId: a1.pairId, companionName: "iPad", generation: 1), now: t0)
        XCTAssertEqual(m.phase, .paired(.init(pairId: a1.pairId, companionName: "iPad", generation: 1)))
    }

    // MARK: receiving

    func testReceivingProgressAndCompletion() {
        var m = M(phase: .advertising, generation: 1, isAdvertising: true)
        m.apply(.receiving(pairId: "p", companionName: "iPad", captureId: nil, bytes: 1024, total: nil), now: t0)
        XCTAssertEqual(m.phase.title, "Receiving capture")
        XCTAssertEqual(m.phase.detail(now: t0), "Receiving 1024 bytes from iPad; total unknown until the line ends.")
        m.apply(.receiving(pairId: "p", companionName: "iPad", captureId: "c1", bytes: 4096, total: 8192), now: t0)
        XCTAssertEqual(m.phase.detail(now: t0), "Receiving 4096 of 8192 bytes from iPad.")
        let done = m.apply(.captureReceived(pairId: "p", captureId: "c1"), now: t0)
        XCTAssertEqual(m.phase, .advertising)
        XCTAssertEqual(done.effects, [.announce("Received capture c1 from iPad.")])
    }

    func testReceivingFromAnotherPairIsNotEndedByStaleCapture() {
        var m = M(phase: .advertising, generation: 1, isAdvertising: true)
        m.apply(.receiving(pairId: "p", companionName: nil, captureId: nil, bytes: 1, total: nil), now: t0)
        XCTAssertTrue(m.apply(.captureReceived(pairId: "q", captureId: "c"), now: t0).stale)
        if case .receiving = m.phase {} else { XCTFail("\(m.phase)") }
        XCTAssertTrue(m.apply(.peerGone(pairId: "q", reason: "x", generation: 1), now: t0).stale)
        m.apply(.peerGone(pairId: "p", reason: "reset", generation: 1), now: t0)
        XCTAssertEqual(m.phase, .failed(reason: "Connection to p closed while receiving: reset", generation: 1))
    }

    func testRefusedCaptureEndsReceivingOrIsJustAnnounced() {
        var m = M(phase: .advertising, generation: 1, isAdvertising: true)
        m.apply(.receiving(pairId: "p", companionName: "iPad", captureId: nil, bytes: 9, total: nil), now: t0)
        XCTAssertTrue(m.apply(.captureRefused(pairId: "q", captureId: "x", code: "bad_request"), now: t0).stale)
        let out = m.apply(.captureRefused(pairId: "p", captureId: "x", code: "unsupported_image"), now: t0)
        XCTAssertEqual(m.phase, .advertising)
        XCTAssertEqual(out.effects, [.announce("Capture x from iPad refused: unsupported_image.")])
        XCTAssertEqual(m.apply(.captureRefused(pairId: nil, captureId: nil, code: "line_too_long"), now: t0).effects,
                       [.announce("Capture ? refused: line_too_long.")])
        var n = M()
        n.apply(.codeIssued(a1), now: t0)
        XCTAssertEqual(n.apply(.captureRefused(pairId: "p", captureId: "x", code: "c"), now: t0), .ignoredInput)
    }

    func testReceivingIsIgnoredWhileACodeIsShown() {
        var m = M()
        m.apply(.codeIssued(a1), now: t0)
        XCTAssertEqual(m.apply(.receiving(pairId: "p", companionName: nil, captureId: nil, bytes: 1, total: nil), now: t0), .ignoredInput)
        XCTAssertEqual(m.phase, .codeShown(a1))
        // A capture from an already paired device while advertising is only announced.
        var n = M(phase: .advertising, generation: 1, isAdvertising: true)
        XCTAssertEqual(n.apply(.captureReceived(pairId: nil, captureId: "c9"), now: t0).effects, [.announce("Received capture c9.")])
        XCTAssertEqual(n.phase, .advertising)
    }

    func testForgetEndsPairedOrReceivingBanner() {
        var m = M(phase: .paired(.init(pairId: "p", companionName: "c", generation: 1)), generation: 1, isAdvertising: true)
        XCTAssertEqual(m.apply(.forgotten(pairId: "other"), now: t0), .ignoredInput)
        m.apply(.forgotten(pairId: "p"), now: t0)
        XCTAssertEqual(m.phase, .advertising)
    }

    func testAdvertisingOffEndsBannersAndKeepsInterrupted() {
        var m = M(phase: .paired(.init(pairId: "p", companionName: "c", generation: 1)), generation: 1, isAdvertising: true)
        m.apply(.advertising(false), now: t0)
        XCTAssertEqual(m.phase, .off)
        var n = M()
        n.apply(.restored(a1), now: t0)
        n.apply(.advertising(true), now: t0)
        XCTAssertEqual(n.phase, .interrupted(a1, .relaunch, detail: "FlashTeX was quit while the code was valid"))
    }

    // MARK: user-visible text

    func testPhaseTextIsPreciseAndCountdownFree() {
        XCTAssertEqual(P.off.title, "Off")
        XCTAssertEqual(P.advertising.detail(now: t0), "Paired companions can connect. No pairing in progress.")
        let shown = P.codeShown(a1)
        XCTAssertEqual(shown.title, "Pairing code shown")
        XCTAssertEqual(shown.detail(now: t0.addingTimeInterval(30)), "Enter the code on the companion. Expires in 90 s.")
        XCTAssertEqual(shown.accessibilityValue(now: t0), "Code 1 2, 3 4, 5 6. Enter it on the companion, or scan the QR code.")
        XCTAssertEqual(P.verifying(a1).detail(now: t0.addingTimeInterval(119.5)), "A companion connected with the code; waiting for its hello. Expires in 1 s.")
        let inter = P.interrupted(a1, .peerGone, detail: "peer closed")
        XCTAssertEqual(inter.detail(now: t0), "The companion disconnected (peer closed). Code 123 456 is still valid for 120 s: resume or cancel.")
        XCTAssertEqual(inter.detail(now: t0.addingTimeInterval(121)), "The companion disconnected (peer closed); the code has expired. Dismiss it or show a new code.")
        XCTAssertEqual(inter.accessibilityValue(now: t0), "peer closed. Code 1 2, 3 4, 5 6 is still valid.")
        XCTAssertEqual(P.failed(reason: "boom", generation: 1).detail(now: t0), "boom")
        XCTAssertEqual(P.paired(.init(pairId: "p", companionName: "iPad", generation: 1)).detail(now: t0), "Paired with iPad (p).")
        XCTAssertEqual(Pairing.displayCode("123456"), "123 456")
        XCTAssertEqual(Pairing.spokenCode("907"), "9 0, 7")
        XCTAssertEqual(Pairing.spokenCode("123456"), "1 2, 3 4, 5 6")
        XCTAssertEqual(Pairing.clipboardCode("123 456"), "123456")
    }

    // MARK: step indicator and reconnect announcements (mac-pairing-ui-2)

    func testStepIndicatorFollowsThePhase() {
        XCTAssertEqual(PairingFlow.Step.count, 4)
        XCTAssertEqual(PairingFlow.Step.shortNames.count, PairingFlow.Step.names.count)
        XCTAssertEqual(P.off.step, .init(index: 1, status: .pending))
        XCTAssertEqual(P.advertising.step, .init(index: 1, status: .pending))
        XCTAssertEqual(P.codeShown(a1).step, .init(index: 2, status: .active))
        XCTAssertEqual(P.verifying(a1).step, .init(index: 3, status: .active))
        XCTAssertEqual(P.paired(.init(pairId: "p", companionName: "iPad", generation: 1)).step, .init(index: 4, status: .done))
        XCTAssertEqual(P.interrupted(a1, .peerGone, detail: "x").step, .init(index: 3, status: .interrupted), "the peer got as far as the session")
        XCTAssertEqual(P.interrupted(a1, .relaunch, detail: "x").step, .init(index: 2, status: .interrupted))
        XCTAssertEqual(P.interrupted(a1, .transportStopped, detail: "x").step, .init(index: 2, status: .interrupted))
        XCTAssertNil(P.receiving(.init(pairId: "p", companionName: nil, captureId: nil, bytes: 1, total: nil)).step)
        XCTAssertNil(P.failed(reason: "boom", generation: 1).step)

        // The indicator is one spoken element: position, name, state, what is done.
        let s2 = P.codeShown(a1).step!
        XCTAssertEqual(s2.name, "Companion enters the code")
        XCTAssertEqual(s2.accessibilityLabel, "Pairing step 2 of 4: Companion enters the code, in progress. Done: Show a code.")
        XCTAssertEqual((1...4).map(s2.status(of:)), [.done, .active, .pending, .pending])
        XCTAssertEqual(P.off.step!.accessibilityLabel, "Pairing step 1 of 4: Show a code, not started.")
        let s4 = P.paired(.init(pairId: "p", companionName: "iPad", generation: 1)).step!
        XCTAssertEqual(s4.accessibilityLabel, "Pairing step 4 of 4: Paired, done. Done: Show a code, Companion enters the code, Verify the companion, Paired.")
        XCTAssertEqual((1...4).map(s4.status(of:)), [.done, .done, .done, .done])
        let s3 = P.interrupted(a1, .peerGone, detail: "x").step!
        XCTAssertEqual(s3.accessibilityLabel, "Pairing step 3 of 4: Verify the companion, interrupted. Done: Show a code, Companion enters the code.")

        // Machine-driven: the indicator moves with the real transitions.
        var m = M()
        m.apply(.advertising(true), now: t0)
        m.apply(.codeIssued(a1), now: t0)
        XCTAssertEqual(m.phase.step?.index, 2)
        m.apply(.bootstrapSessionOpened(generation: 1), now: t0)
        XCTAssertEqual(m.phase.step?.index, 3)
        m.apply(.confirmed(pairId: a1.pairId, companionName: "iPad", generation: 1), now: t0)
        XCTAssertEqual(m.phase.step, .init(index: 4, status: .done))
        m.apply(.dismiss, now: t0)
        XCTAssertEqual(m.phase.step, .init(index: 1, status: .pending))
    }

    func testKnownCompanionReconnectAndDisconnectAreAnnouncedWithoutChangingThePhase() {
        var m = M(phase: .advertising, generation: 3, isAdvertising: true)
        var o = m.apply(.companionConnected(pairId: "p", companionName: "iPad"), now: t0)
        XCTAssertEqual(o.effects, [.announce("iPad reconnected.")])
        XCTAssertEqual(m.phase, .advertising)
        o = m.apply(.companionDisconnected(pairId: "p", companionName: "iPad", reason: "peer closed"), now: t0)
        XCTAssertEqual(o.effects, [.announce("iPad disconnected: peer closed.")])
        XCTAssertEqual(m.phase, .advertising)

        // While a code is shown the reconnect is still announced and the code stays.
        m.apply(.codeIssued(attempt(4)), now: t0)
        o = m.apply(.companionConnected(pairId: "p", companionName: "iPad"), now: t0)
        XCTAssertEqual(o.effects, [.announce("iPad reconnected.")])
        XCTAssertEqual(m.phase, .codeShown(attempt(4)))

        // A receive that failed is reported once, by peerGone; the disconnect
        // notice does not repeat it, and nothing is announced on the error banner.
        var r = M(phase: .advertising, generation: 3, isAdvertising: true)
        r.apply(.receiving(pairId: "p", companionName: "iPad", captureId: nil, bytes: 10, total: nil), now: t0)
        XCTAssertEqual(r.apply(.companionDisconnected(pairId: "p", companionName: "iPad", reason: "x"), now: t0), .ignoredInput)
        r.apply(.peerGone(pairId: "p", reason: "reset", generation: 3), now: t0)
        guard case .failed = r.phase else { return XCTFail("\(r.phase)") }
        XCTAssertEqual(r.apply(.companionDisconnected(pairId: "p", companionName: "iPad", reason: "reset"), now: t0), .ignoredInput)
    }

    func testRowAccessibilityText() {
        let r = PairRecord(pairId: "abc", psk: "k", companionName: "iPad", createdAt: t0, lastSeenAt: nil)
        let a = PairingAccessibility.deviceRow(r, connected: true)
        XCTAssertEqual(a.label, "Companion iPad, pair id abc")
        XCTAssertTrue(a.value.hasPrefix("Connected. Permission: captures allowed. Paired "), a.value)
        XCTAssertFalse(a.value.contains("Last seen"))
        var seen = r
        seen.lastSeenAt = t0
        XCTAssertTrue(PairingAccessibility.deviceRow(seen, connected: false).value.hasPrefix("Not connected. Permission: captures allowed. Paired "))
        seen.permission = .viewOnly
        XCTAssertTrue(PairingAccessibility.deviceRow(seen, connected: false).value.hasPrefix("Not connected. Permission: view only, captures refused. Paired "))
        XCTAssertTrue(PairingAccessibility.deviceRow(seen, connected: false).value.contains("Last seen"))

        let inbox = PairingAccessibility.CaptureRow(captureId: "c1", source: .inbox, state: "received", durable: false, note: "image/png")
        XCTAssertEqual(inbox.accessibilityLabel, "Capture c1")
        XCTAssertEqual(inbox.accessibilityValue, "received, not durable (in memory only). image/png")
        let bridge = PairingAccessibility.CaptureRow(captureId: "c2", source: .bridge, state: "proposed", durable: nil, note: "")
        XCTAssertEqual(bridge.accessibilityValue, "proposed, via bridge")
        XCTAssertEqual(bridge.id, "bridge:c2")
    }
}

// MARK: - persistence

final class PairingPersistenceTests: XCTestCase {
    private func tempDir() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("pairing-tests-\(UUID().uuidString)")
    }

    func testPairRecordDescriptionNeverContainsTheKey() {
        let psk = Pairing.mintLongTermPSK().base64EncodedString()
        let r = PairRecord(pairId: "abcdef0123456789", psk: psk, companionName: "iPad", createdAt: Date(), lastSeenAt: Date())
        for text in [String(describing: r), String(reflecting: r), "\(r)", "record: \(r)"] {
            XCTAssertFalse(text.contains(psk), text)
            XCTAssertTrue(text.contains("<redacted>"), text)
            XCTAssertTrue(text.contains("abcdef0123456789"))
        }
    }

    func testPairStoreSurvivesRelaunchAndReportsSchemaVersion() throws {
        let dir = tempDir()
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("pairs.json")
        let store = PairStore(url: url)
        XCTAssertEqual(store.loadOutcome, .created)
        let psk = Pairing.mintLongTermPSK().base64EncodedString()
        XCTAssertTrue(store.upsert(PairRecord(pairId: "p1", psk: psk, companionName: "iPad", createdAt: Date(), lastSeenAt: nil)))
        let json = try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any]
        XCTAssertEqual(json?["version"] as? Int, PairStore.schemaVersion)
        // "Relaunch": a fresh store at the same path sees the pairing and the same salt.
        let again = PairStore(url: url)
        XCTAssertEqual(again.loadOutcome, .loaded(version: PairStore.schemaVersion))
        XCTAssertEqual(again.pairs.map(\.pairId), ["p1"])
        XCTAssertEqual(again.salt, store.salt)
        XCTAssertNil(again.loadError)
    }

    func testPairStoreRefusesNewerVersionWithoutOverwriting() throws {
        let dir = tempDir()
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("pairs.json")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let future = """
        {"version": \(PairStore.schemaVersion + 1), "salt": "000102030405060708090a0b0c0d0e0f", "pairs": [], "future_field": true}
        """
        try Data(future.utf8).write(to: url)
        let store = PairStore(url: url)
        guard case .refused(let reason) = store.loadOutcome else { return XCTFail("\(store.loadOutcome)") }
        XCTAssertTrue(reason.contains("newer than this build"), reason)
        XCTAssertNotNil(store.loadError)
        XCTAssertFalse(store.upsert(PairRecord(pairId: "x", psk: "k", companionName: "x", createdAt: Date(), lastSeenAt: nil)),
                       "a refused store must not persist over the newer file")
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), future, "file untouched")
    }

    func testPairStoreDecodeRejectsBadSaltOrKeyAndUpgradesOlderVersions() throws {
        let ok = Data("""
        {"version": 1, "salt": "000102030405060708090a0b0c0d0e0f", "pairs": []}
        """.utf8)
        // Schema v2 (per-record `generation`): a v1 file is upgraded in place, its records keep generation nil.
        XCTAssertEqual(try PairStore.decode(ok).get().1, .upgraded(from: 1))
        // Schema v3 (per-record `permission`): a v2 file is upgraded too; v3 loads as is.
        let v2 = Data("""
        {"version": 2, "salt": "000102030405060708090a0b0c0d0e0f", "pairs": []}
        """.utf8)
        XCTAssertEqual(try PairStore.decode(v2).get().1, .upgraded(from: 2))
        let current = Data("""
        {"version": 3, "salt": "000102030405060708090a0b0c0d0e0f", "pairs": []}
        """.utf8)
        XCTAssertEqual(try PairStore.decode(current).get().1, .loaded(version: 3))
        let badSalt = Data("""
        {"version": 1, "salt": "0001", "pairs": []}
        """.utf8)
        XCTAssertThrowsError(try PairStore.decode(badSalt).get())
        let badKey = Data("""
        {"version": 1, "salt": "000102030405060708090a0b0c0d0e0f", "pairs": [{"pair_id": "p", "psk": "c2hvcnQ=", "companion_name": "x", "created_at": "2026-09-12T00:00:00Z"}]}
        """.utf8)
        XCTAssertThrowsError(try PairStore.decode(badKey).get())
        XCTAssertThrowsError(try PairStore.decode(Data("{\"pairs\": []}".utf8)).get())
        XCTAssertThrowsError(try PairStore.decode(Data("{\"version\": 0, \"salt\": \"00\", \"pairs\": []}".utf8)).get())
        // Upgrade path is exercised once a version 2 exists; version 1 is current.
        var f = PairStore.File(version: 1, salt: "00", pairs: [])
        f = PairStore.upgrade(f)
        XCTAssertEqual(f.version, PairStore.schemaVersion)
    }

    func testJournalPersistsPendingAttemptAndGenerationAcrossRelaunch() throws {
        let dir = tempDir()
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("pairing-session.json")
        let j = PairingJournal(url: url)
        XCTAssertNil(j.pending)
        XCTAssertEqual(j.generation, 0)
        XCTAssertEqual(j.nextGeneration(), 1)
        XCTAssertEqual(j.nextGeneration(), 2)
        let a = PairingFlow.Attempt(generation: 2, code: "123456", pairId: "pid", startedAt: Date(), expiresAt: Date().addingTimeInterval(120))
        XCTAssertTrue(j.setPending(a))
        let attrs = try FileManager.default.attributesOfItem(atPath: url.path)
        XCTAssertEqual((attrs[.posixPermissions] as? Int).map { $0 & 0o777 }, 0o600)
        let text = try String(contentsOf: url, encoding: .utf8)
        XCTAssertTrue(text.contains("\"pair_id\""), text)
        XCTAssertFalse(text.contains("psk"), "the journal never holds a long-term key")

        // Relaunch.
        let k = PairingJournal(url: url)
        XCTAssertEqual(k.generation, 2)
        XCTAssertEqual(k.pending?.code, "123456")
        XCTAssertEqual(k.pending?.generation, 2)
        XCTAssertEqual(k.pending!.expiresAt.timeIntervalSince1970, a.expiresAt.timeIntervalSince1970, accuracy: 1)
        XCTAssertTrue(k.setPending(nil))
        XCTAssertNil(PairingJournal(url: url).pending)
        XCTAssertEqual(PairingJournal(url: url).generation, 2, "the counter never rewinds")

        // A pending attempt from a newer generation advances the counter.
        XCTAssertTrue(k.setPending(PairingFlow.Attempt(generation: 9, code: "1", pairId: "p", startedAt: Date(), expiresAt: Date())))
        XCTAssertEqual(PairingJournal(url: url).generation, 9)
    }

    func testJournalRoundTripsRollsAndDecodesOlderJournalsAsUnrolled() throws {
        let dir = tempDir()
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appendingPathComponent("pairing-session.json")
        var rolled = PairingFlow.Attempt(generation: 4, code: "246810", pairId: "pid", startedAt: Date(), expiresAt: Date().addingTimeInterval(120))
        rolled.rolls = 3
        XCTAssertTrue(PairingJournal(url: url).setPending(rolled))
        XCTAssertTrue(try String(contentsOf: url, encoding: .utf8).contains("\"rolls\" : 3"))
        XCTAssertEqual(PairingJournal(url: url).pending?.rolls, 3, "the cap survives a relaunch")

        // A journal written before rolling codes existed (no `rolls` key).
        let legacy = """
        {"version": 1, "generation": 2, "pending": {"generation": 2, "code": "123456", "pair_id": "p",
         "started_at": "2026-09-14T08:00:00Z", "expires_at": "2026-09-14T08:02:00Z"}}
        """
        try Data(legacy.utf8).write(to: url)
        let j = PairingJournal(url: url)
        XCTAssertNil(j.loadError)
        XCTAssertEqual(j.pending?.code, "123456")
        XCTAssertEqual(j.pending?.rolls, 0)
        XCTAssertTrue(j.pending!.canRoll)
    }

    func testJournalRefusesUnknownVersionWithoutOverwriting() throws {
        let dir = tempDir()
        defer { try? FileManager.default.removeItem(at: dir) }
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = dir.appendingPathComponent("pairing-session.json")
        let text = "{\"version\": 99, \"generation\": 5}"
        try Data(text.utf8).write(to: url)
        let j = PairingJournal(url: url)
        XCTAssertNotNil(j.loadError)
        XCTAssertEqual(j.generation, 0)
        XCTAssertFalse(j.setPending(nil))
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), text)
    }

    func testDefaultLocationsHonourAutomationOverrides() {
        XCTAssertEqual(PairingJournal.defaultURL().lastPathComponent, ProcessInfo.processInfo.environment["FLASHTEX_PAIRING_JOURNAL"].map { URL(fileURLWithPath: $0).lastPathComponent } ?? "pairing-session.json")
        XCTAssertEqual(PairStore.defaultURL().lastPathComponent, ProcessInfo.processInfo.environment["FLASHTEX_PAIR_STORE"].map { URL(fileURLWithPath: $0).lastPathComponent } ?? "pairs.json")
        if ProcessInfo.processInfo.environment["FLASHTEX_PAIRING_JOURNAL"] == nil {
            XCTAssertEqual(PairingJournal.defaultURL().deletingLastPathComponent(), PairStore.defaultURL().deletingLastPathComponent())
        }
    }
}
