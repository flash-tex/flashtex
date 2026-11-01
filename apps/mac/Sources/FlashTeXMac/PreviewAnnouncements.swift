import AppKit

/// What the preview says to VoiceOver by itself: the page the reader moved
/// to from the keyboard ("Page 3 of 12"). A completed compile is spoken by
/// the diagnostics announcer (`ShellModel.noteCompileCompletedForVoiceOver`,
/// DiagnosticsPanel.swift), from the engine's DONE. `post` is replaceable
/// (tests, like `PairingFlowController.announcer`); `announcements` is the
/// evidence, capped like the editor's.
@MainActor
final class PreviewAnnouncer {
    var post: (String, NSAccessibilityPriorityLevel) -> Void = { message, priority in
        NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: priority.rawValue])
    }
    private(set) var announcements: [String] = []

    /// The reader moved between pages from the keyboard (Page Up/Down).
    func notePageJump(page: Int, of total: Int) {
        say("Page \(page)\(total > 0 ? " of \(total)" : "")", .low)
    }

    private func say(_ message: String, _ priority: NSAccessibilityPriorityLevel) {
        announcements.append(message)
        if announcements.count > 64 { announcements.removeFirst(announcements.count - 64) }
        post(message, priority)
    }
}
