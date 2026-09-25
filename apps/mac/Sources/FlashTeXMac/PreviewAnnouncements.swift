import AppKit
import FlashTeXProtocol

/// What the preview says to VoiceOver when a compile lands: "Preview updated:
/// 3 pages", "Compile failed: 2 errors". A completed compile is the only
/// trigger — never a keystroke, a scroll or a hover — and under auto-compile
/// every keystroke completes one, so results are coalesced: the first result
/// is spoken at once, every later one (a status flip included — typing
/// through a brace alternates failed/ok on consecutive keystrokes) waits
/// `quietInterval` after the last result and then the newest state is spoken
/// once, at its own priority (a failure is high). `post` is replaceable
/// (tests, like `PairingFlowController.announcer`); `announcements` is the
/// evidence, capped like the editor's.
///
/// Transition table (state: `spoken` = last state spoken, `latest` +
/// `timer` = a coalesced update pending, `awaitingFrame` = a v2-only result
/// waiting for its frame's page count, `spokenRefusal` /
/// `spokenLiveRefusal` = refusal dedupe). Pinned by
/// `PreviewPaneAccessibilityTests.testAnnouncerTransitionTable`.
///
/// | event                                   | effect                                                                 |
/// |-----------------------------------------|------------------------------------------------------------------------|
/// | result, nothing spoken yet              | spoken at once (any route; v2-only still waits for its frame first)    |
/// | result, same revision + status as spoken| dropped                                                                |
/// | result (v1 pages / failed), later       | pending; the quiet interval speaks the NEWEST pending state, once      |
/// | result v2-only (ok/recovered, no pages) | awaitingFrame = it; any pending update is withdrawn (newest wins)      |
/// | live frame for the awaited revision     | count filled in, then as a result; dedupes reset                       |
/// | live frame for another revision         | nothing (dedupes reset)                                                |
/// | any frame verified (`.loaded`)          | both refusal dedupes reset                                             |
/// | live refusal (frame kept)               | pending + awaiting withdrawn; "Preview not updated" low, once per error|
/// | refusal, nothing on screen (`.failed`)  | pending + awaiting withdrawn; "…refused" high, once per error          |
/// | page jump                               | "Page n of m" low, always (a rotor load lets VoiceOver read the page)  |
/// | result = nil (project reset)            | everything forgotten: the next result is a first result again         |
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

    /// Pure policy over the last state SPOKEN (not the last seen): the first
    /// result now, the same revision and status again never, anything else
    /// after the quiet interval (newest wins).
    static func decide(previous: State?, next: State) -> Decision {
        guard let previous else { return .now }
        if previous == next || (previous.revision == next.revision && previous.status == next.status) { return .drop }
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
    /// The live refusal last spoken; the same one again is silent until a frame verifies.
    private(set) var spokenLiveRefusal: RenderingV2.ValidationError?
    /// The `.failed` refusal last spoken; auto-compile retries go failed →
    /// loading → failed with the same error, which is silent until a frame
    /// verifies or the error changes.
    private(set) var spokenRefusal: RenderingV2.ValidationError?
    /// A v2-only result whose page count the frame has yet to supply.
    private(set) var awaitingFrame: State?

    /// A compile result was applied (the worker's reply, a fixture, an older
    /// snapshot). Nil (the preview was cleared) cancels a pending update.
    ///
    /// On the default live v2 route the reply's v1 `pages` are elided by
    /// design (`display-list-v2-only`), so its count is not the document's:
    /// such a result waits — nothing is armed — until its v2 frame delivers
    /// (`noteFrame`) and supplies the page count, or its sibling is refused
    /// (`noteLiveRefusal`, which withdraws it). A failure needs no count and
    /// is noted at once.
    func noteResult(_ result: RuntimeV1.CompileResult?) {
        guard let result else {
            // Project reset: nothing of the old project is "already spoken", so
            // the new project's first result is spoken at once (and is never
            // dropped for sharing the old one's revision and status).
            cancel()
            latest = nil; spoken = nil; awaitingFrame = nil
            spokenRefusal = nil; spokenLiveRefusal = nil
            return
        }
        let state = State(result)
        if result.status != .failed, result.pages.isEmpty,
           result.layoutCapabilities?.contains(DisplayListDelta.v2OnlyCapability) == true {
            // Newest wins: an older result still waiting for its quiet interval
            // is superseded by this one, which cannot be spoken before its frame.
            cancel()
            latest = spoken
            awaitingFrame = state
            return
        }
        awaitingFrame = nil
        note(state)
    }

    /// A live v2 frame verified for `revision` with `pageCount` pages (the
    /// document's length, elided pages included): completes the result that
    /// was waiting for it. A frame for another revision completes nothing.
    func noteFrame(revision: Int, pageCount: Int) {
        noteFrameVerified()
        guard var state = awaitingFrame, state.revision == revision else { return }
        awaitingFrame = nil
        state.pages = pageCount
        note(state)
    }

    func note(_ state: State) {
        latest = state
        switch Self.decide(previous: spoken, next: state) {
        case .now: cancel(); speak(state)
        case .drop: break
        case .wait: arm()
        }
    }

    /// A frame verified (`.loaded`, any source): the next refusal of either
    /// kind is news again.
    func noteFrameVerified() { spokenRefusal = nil; spokenLiveRefusal = nil }

    /// A display list the v2 pane refused with nothing verified on screen:
    /// the reader would otherwise hear an "updated" preview that shows a
    /// refusal, so a pending or awaiting update is withdrawn. Spoken at once,
    /// high priority — once: auto-compile retries the same refusal on every
    /// keystroke (failed → loading → failed), which is silent until a frame
    /// verifies or the error changes.
    func noteRefusal(_ error: RenderingV2.ValidationError) {
        cancel()
        latest = spoken
        awaitingFrame = nil
        guard error != spokenRefusal else { return }
        spokenRefusal = error
        say("Preview display list refused: \(error.message)", .high)
    }

    /// A LIVE display list the v2 pane refused while the previous verified
    /// frame stays on screen (`deliverDisplayListV2`): the pages did not
    /// change, so the compile result that arrived with it must not be spoken
    /// as "Preview updated" — a pending coalesced update is withdrawn — and
    /// the refusal itself is quiet (low priority): a repeat of the same one
    /// (every keystroke under auto-compile while, say, a font is missing)
    /// says nothing; a different refusal, or the same one after a frame
    /// verified in between, is spoken again.
    func noteLiveRefusal(_ error: RenderingV2.ValidationError) {
        cancel()
        latest = spoken // the refused revision is not news; the next result decides afresh
        awaitingFrame = nil
        guard error != spokenLiveRefusal else { return }
        spokenLiveRefusal = error
        say("Preview not updated: \(error.message)", .low)
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
