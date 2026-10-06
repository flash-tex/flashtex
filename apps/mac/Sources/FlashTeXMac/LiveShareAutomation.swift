import AppKit
import FlashTeXCollabSession

/// Launch-time automation for Live Share smoke runs (two app instances on
/// one Mac, no clicks), like FLASHTEX_OPEN and the bench hooks:
///
/// Debug builds only, all of them.
///
/// - `FLASHTEX_LIVE_SHARE_HOST=1`: once the opened project is up, start
///   hosting; `FLASHTEX_LIVE_SHARE_INVITE_OUT=<file>` receives each new
///   invitation link.
/// - `FLASHTEX_LIVE_SHARE_JOIN_FILE=<file>`: wait (≤ 2 min) for a link in
///   that file and join with it (`FLASHTEX_LIVE_SHARE_NAME` names you).
/// - `FLASHTEX_LIVE_SHARE_TYPE=<text>`: once joined, type it into the editor
///   at the end of the first line, one character every 80 ms.
/// - `FLASHTEX_LIVE_SHARE_AUTOAPPROVE=1` (debug builds only): admit every
///   joiner as an editor. A release build always asks the host user.
@MainActor
enum LiveShareAutomation {
    static var env: [String: String] { ProcessInfo.processInfo.environment }

    #if DEBUG
    static var autoApprove: Bool { env["FLASHTEX_LIVE_SHARE_AUTOAPPROVE"] == "1" }
    #else
    static let autoApprove = false
    #endif

    static func startIfConfigured(model: ShellModel) {
        #if DEBUG // a release build never hosts or joins by itself
        if env["FLASHTEX_LIVE_SHARE_HOST"] == "1" {
            poll(every: 0.5, upTo: 60) {
                guard model.documentURL != nil else { return false }
                model.liveShare.startHosting()
                return true
            }
        }
        if let file = env["FLASHTEX_LIVE_SHARE_JOIN_FILE"], !file.isEmpty {
            poll(every: 0.5, upTo: 120) {
                guard let link = try? String(contentsOfFile: file, encoding: .utf8),
                      link.hasPrefix(CollabInvite.scheme + "://") else { return false }
                model.liveShare.join(link: link, name: env["FLASHTEX_LIVE_SHARE_NAME"] ?? LiveShareController.defaultDisplayName,
                                     disposition: .discard)
                return true
            }
        }
        #endif
    }

    static func hostReady(_ invite: CollabInvite, model: ShellModel) {
        #if DEBUG
        guard let out = env["FLASHTEX_LIVE_SHARE_INVITE_OUT"], !out.isEmpty else { return }
        try? invite.link.write(toFile: out, atomically: true, encoding: .utf8)
        #endif
    }

    static func guestOpened(model: ShellModel) {
        #if DEBUG
        guard let text = env["FLASHTEX_LIVE_SHARE_TYPE"], !text.isEmpty else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            guard let tv = TypingBenchDriver.findTextView(in: NSApp.windows.compactMap(\.contentView)) else { return }
            let ns = tv.string as NSString
            let eol = ns.range(of: "\n").location
            tv.setSelectedRange(NSRange(location: eol == NSNotFound ? ns.length : eol, length: 0))
            var chars = Array(text)
            Timer.scheduledTimer(withTimeInterval: 0.08, repeats: true) { t in
                MainActor.assumeIsolated {
                    guard !chars.isEmpty else { t.invalidate(); return }
                    tv.insertText(String(chars.removeFirst()), replacementRange: tv.selectedRange())
                }
            }
        }
        #endif
    }

    private static func poll(every interval: TimeInterval, upTo limit: TimeInterval, _ body: @escaping @MainActor () -> Bool) {
        let end = Date().addingTimeInterval(limit)
        Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { t in
            MainActor.assumeIsolated {
                if body() || Date() > end { t.invalidate() }
            }
        }
    }
}
