import AppKit
import Foundation

/// Performs the update check against the GitHub Releases feed and owns
/// `FlashTeX > Check for Updates…` (#694, slice 1). Nothing here downloads or
/// installs anything: `Download` opens the release page in the browser.
/// The feed model and version comparison are in `UpdateFeed.swift`.
///
/// Network access is behind `UpdateFeedTransport` so tests inject a stub;
/// a failed or offline request becomes `.failed(reason)` — the checker never
/// throws into the UI.

// MARK: - Installed version

enum AppVersion {
    /// `CFBundleShortVersionString` from the app bundle, which
    /// `scripts/make-app.sh --version` (CI passes the release tag) writes.
    /// A bare `swift run` has no bundle version → nil, and the check reports
    /// that instead of comparing against 0.0.0.
    static var current: SemanticVersion? {
        (Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String).flatMap(SemanticVersion.init)
    }
}

// MARK: - Transport seam

protocol UpdateFeedTransport: Sendable {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: UpdateFeedTransport {}

// MARK: - Checker

/// One check = one GET of `/releases/latest` (published, non-draft,
/// non-prerelease is what that endpoint already means). Manual checks ignore
/// the automatic-check preference, the 24 h throttle and a skipped version;
/// the automatic check honours all three and only reports an update.
@MainActor
final class UpdateChecker {
    static let timeout: TimeInterval = 10
    static let automaticInterval: TimeInterval = 24 * 60 * 60
    static let userAgent = "FlashTeX-Mac-UpdateCheck"

    let transport: any UpdateFeedTransport
    let preferences: EditorPreferences
    let currentVersion: SemanticVersion?
    let now: @Sendable () -> Date
    /// Requests actually sent (evidence for tests: "preference off → no request").
    private(set) var requestsSent = 0

    init(transport: any UpdateFeedTransport = URLSession.shared,
         preferences: EditorPreferences,
         currentVersion: SemanticVersion? = AppVersion.current,
         now: @escaping @Sendable () -> Date = { Date() }) {
        self.transport = transport; self.preferences = preferences; self.currentVersion = currentVersion; self.now = now
    }

    /// The user asked (menu item): always fetches, records the time, reports everything.
    func check() async -> UpdateCheck {
        guard let current = currentVersion else {
            return .failed("This build has no version number (run from a packaged FlashTeX.app to check for updates).")
        }
        var request = URLRequest(url: UpdateFeed.latestReleaseURL, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: Self.timeout)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue(Self.userAgent, forHTTPHeaderField: "User-Agent")
        requestsSent += 1
        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await transport.data(for: request)
        } catch {
            return .failed(Self.oneLine(error))
        }
        preferences.lastUpdateCheck = now()
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            return .failed("GitHub answered HTTP \(http.statusCode).")
        }
        do {
            let latest = try UpdateFeed.decodeRelease(data)
            return UpdateCheck.compare(latest: latest, current: current)
        } catch let e as UpdateFeed.DecodeError {
            return .failed(Self.describe(e))
        } catch {
            return .failed(Self.oneLine(error))
        }
    }

    /// Whether the automatic check is due: never checked, or ≥ 24 h ago.
    static func isDue(lastCheck: Date?, now: Date) -> Bool {
        guard let lastCheck else { return true }
        return now.timeIntervalSince(lastCheck) >= automaticInterval
    }

    /// The launch-time check. `.disabled` when the preference is off (no
    /// request); nil when not due or when nothing should be surfaced
    /// (up to date, failed, or the skipped version); `.available` otherwise.
    func automaticCheckIfDue() async -> UpdateCheck? {
        guard preferences.checkForUpdatesAutomatically else { return .disabled }
        guard Self.isDue(lastCheck: preferences.lastUpdateCheck, now: now()) else { return nil }
        let result = await check()
        guard case .available(let release, _) = result, release.tagName != preferences.skippedUpdateVersion else { return nil }
        return result
    }

    /// One line for the alert / status; never a raw `Error.localizedDescription` dump.
    static func oneLine(_ error: Error) -> String {
        if let urlError = error as? URLError {
            switch urlError.code {
            case .notConnectedToInternet, .networkConnectionLost: return "You appear to be offline."
            case .timedOut: return "GitHub did not answer within \(Int(timeout)) seconds."
            case .cannotFindHost, .cannotConnectToHost, .dnsLookupFailed: return "Could not reach github.com."
            default: break
            }
        }
        let text = error.localizedDescription.split(whereSeparator: \.isNewline).first.map(String.init) ?? "The update check failed."
        return text.isEmpty ? "The update check failed." : text
    }

    static func describe(_ error: UpdateFeed.DecodeError) -> String {
        switch error {
        case .malformedJSON: return "The release feed was not valid JSON."
        case .missingTagName: return "The release feed has no tag name."
        case .unparsableVersion(let tag): return "The release tag “\(tag)” is not a version number."
        case .missingHTMLURL: return "The release feed has no release page."
        case .untrustedReleasePage(let url): return "Refused a release page outside github.com/flash-tex/flashtex: \(url)"
        case .untrustedAssetURL(let url): return "Refused a download outside github.com/flash-tex/flashtex/releases: \(url)"
        case .noPublishedRelease: return "No published release was found."
        }
    }

    /// The Settings › Updates footnote.
    static func settingsFootnote(current: SemanticVersion?, lastCheck: Date?, skipped: String?) -> String {
        var parts: [String] = []
        parts.append(current.map { "Installed: FlashTeX \($0)." } ?? "Installed: unversioned development build.")
        if let lastCheck {
            let f = DateFormatter(); f.dateStyle = .medium; f.timeStyle = .short
            parts.append("Last checked \(f.string(from: lastCheck)).")
        } else {
            parts.append("Never checked.")
        }
        if let skipped { parts.append("Skipping \(skipped).") }
        return parts.joined(separator: " ")
    }
}

// MARK: - Menu item and alert

/// The one process-wide presenter: runs a check and shows the result as an
/// alert on the key window. Re-entrant menu clicks while a check is running
/// are ignored.
@MainActor
final class UpdatePresenter {
    static let shared = UpdatePresenter()

    private(set) var inProgress = false
    private var checker: UpdateChecker { UpdateChecker(preferences: .shared) }

    /// `FlashTeX > Check for Updates…`.
    func checkForUpdatesInteractive() {
        guard !inProgress else { return }
        inProgress = true
        Task { @MainActor in
            defer { inProgress = false }
            let result = await checker.check()
            present(result, manual: true)
        }
    }

    /// Called once after launch; quiet unless an update is available.
    func automaticCheckAfterLaunch() {
        guard EditorPreferences.shared.checkForUpdatesAutomatically else { return }
        Task { @MainActor in
            guard let result = await checker.automaticCheckIfDue(), case .available = result else { return }
            present(result, manual: false)
        }
    }

    func present(_ result: UpdateCheck, manual: Bool) {
        let alert = NSAlert()
        switch result {
        case .upToDate(let current):
            alert.messageText = "FlashTeX is up to date"
            alert.informativeText = "FlashTeX \(current) is the newest version."
            alert.addButton(withTitle: "OK")
        case .failed(let reason):
            alert.messageText = "Could not check for updates"
            alert.informativeText = reason
            alert.alertStyle = .warning
            alert.addButton(withTitle: "OK")
        case .disabled:
            return
        case .available(let release, let current):
            alert.messageText = "FlashTeX \(release.version) is available"
            var info = "You have FlashTeX \(current)."
            if let date = release.publishedAt {
                let f = DateFormatter(); f.dateStyle = .medium; f.timeStyle = .none
                info += " Released \(f.string(from: date))."
            }
            info += " Download opens the release page in your browser; nothing is installed automatically."
            alert.informativeText = info
            alert.accessoryView = Self.notesView(release.notesMarkdown)
            alert.addButton(withTitle: "Download")
            alert.addButton(withTitle: "Later")
            alert.addButton(withTitle: "Skip This Version")
            if !manual, release.tagName == EditorPreferences.shared.skippedUpdateVersion { return }
        }
        let respond: (NSApplication.ModalResponse) -> Void = { response in
            guard case .available(let release, _) = result else { return }
            switch response {
            case .alertFirstButtonReturn: NSWorkspace.shared.open(release.htmlURL)
            case .alertThirdButtonReturn: EditorPreferences.shared.skippedUpdateVersion = release.tagName
            default: break
            }
        }
        if let window = NSApp.keyWindow ?? NSApp.mainWindow {
            alert.beginSheetModal(for: window, completionHandler: respond)
        } else {
            respond(alert.runModal())
        }
    }

    /// Release notes as plain text in a read-only scroller.
    static func notesView(_ markdown: String) -> NSView? {
        let text = UpdateFeed.plainText(fromMarkdown: markdown)
        guard !text.isEmpty else { return nil }
        let scroll = NSTextView.scrollableTextView()
        scroll.frame = NSRect(x: 0, y: 0, width: DS.Layout.updateNotesWidth, height: DS.Layout.updateNotesHeight)
        scroll.borderType = .bezelBorder
        if let tv = scroll.documentView as? NSTextView {
            tv.isEditable = false
            tv.isSelectable = true
            tv.font = DS.NSFonts.secondary
            tv.textColor = .labelColor
            tv.drawsBackground = false
            tv.string = text
            tv.setAccessibilityLabel("Release notes")
        }
        return scroll
    }
}
