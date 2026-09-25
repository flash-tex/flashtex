import AppKit
import FlashTeXProtocol

/// What the preview says to VoiceOver when a compile lands: "Preview updated:
/// 3 pages", "Compile failed: 2 errors". A completed compile is the only
/// trigger — never a keystroke, a scroll or a hover — and under auto-compile
/// every keystroke completes one, so consecutive same-status results are
/// coalesced: the status flip (ok → failed, failed → ok) is spoken at once,
/// anything else waits `quietInterval` after the last result and then speaks
/// the latest state once, if a new revision arrived. `post` is replaceable
/// (tests, like `PairingFlowController.announcer`); `announcements` is the
/// evidence, capped like the editor's.
@MainActor
final class PreviewAnnouncer {
    struct State: Equatable {
        var revision: Int
        var status: RuntimeV1.Status
        var pages: Int
        var errors: Int
        var warnings: Int

        init(revision: Int, status: RuntimeV1.Status, pages: Int, errors: Int, warnings: Int = 0) {
            self.revision = revision; self.status = status; self.pages = pages; self.errors = errors; self.warnings = warnings
        }

        init(_ result: RuntimeV1.CompileResult) {
            let counts = EditorDiagnostics.counts(result.diagnostics)
            self.init(revision: result.revision, status: result.status, pages: result.pages.count,
                      errors: counts.errors, warnings: counts.warnings)
        }

        /// The spoken form. Failure names the errors; a recovered result says
        /// the pages are provisional; success is the page count alone.
        var message: String {
            let pageWord = "\(pages) page\(pages == 1 ? "" : "s")"
            switch status {
            case .ok: return "Preview updated: \(pageWord)"
            case .recovered: return "Preview updated with \(errors) error\(errors == 1 ? "" : "s") recovered: \(pageWord)"
            case .failed: return "Compile failed: \(errors) error\(errors == 1 ? "" : "s")"
            }
        }

        var priority: NSAccessibilityPriorityLevel { status == .failed ? .high : .low }
    }

    enum Decision: Equatable {
        /// Speak `message` now.
        case now
        /// Wait for the quiet interval, then speak whatever is latest.
        case wait
        /// Nothing new to say (same revision, or the same state again).
        case drop
    }

    /// Pure policy over the last state SPOKEN (not the last seen).
    static func decide(previous: State?, next: State) -> Decision {
        guard let previous else { return .now }
        if previous == next || (previous.revision == next.revision && previous.status == next.status) { return .drop }
        if previous.status != next.status { return .now }
        return .wait
    }

    /// Seconds of no new result before a coalesced update is spoken.
    var quietInterval: TimeInterval = 2
    var post: (String, NSAccessibilityPriorityLevel) -> Void = { message, priority in
        NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: priority.rawValue])
    }
    private(set) var announcements: [String] = []
    private(set) var spoken: State?
    private var latest: State?
    private var timer: Timer?

    /// A compile result was applied (the worker's reply, a fixture, an older
    /// snapshot). Nil (the preview was cleared) cancels a pending update.
    func noteResult(_ result: RuntimeV1.CompileResult?) {
        guard let result else { cancel(); return }
        note(State(result))
    }

    func note(_ state: State) {
        latest = state
        switch Self.decide(previous: spoken, next: state) {
        case .now: cancel(); speak(state)
        case .drop: break
        case .wait: arm()
        }
    }

    /// A display list the v2 pane refused with nothing verified on screen:
    /// the reader would otherwise hear an "updated" preview that shows a
    /// refusal. Spoken at once; the next accepted result speaks normally.
    func noteRefusal(_ error: RenderingV2.ValidationError) {
        cancel()
        say("Preview display list refused: \(error.message)", .high)
    }

    /// The reader moved between pages from the keyboard (Page Up/Down).
    func notePageJump(page: Int, of total: Int) {
        say("Page \(page)\(total > 0 ? " of \(total)" : "")", .low)
    }

    /// Speaks the coalesced state now if one is pending (tests, teardown).
    func flush() {
        guard timer != nil else { return }
        cancel()
        speakLatest()
    }

    private func speakLatest() {
        guard let latest, Self.decide(previous: spoken, next: latest) != .drop else { return }
        speak(latest)
    }

    private func arm() {
        timer?.invalidate()
        let t = Timer(timeInterval: quietInterval, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated { self?.timer = nil; self?.speakLatest() }
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    private func cancel() { timer?.invalidate(); timer = nil }

    private func speak(_ state: State) {
        spoken = state
        say(state.message, state.priority)
    }

    private func say(_ message: String, _ priority: NSAccessibilityPriorityLevel) {
        announcements.append(message)
        if announcements.count > 64 { announcements.removeFirst(announcements.count - 64) }
        post(message, priority)
    }
}
