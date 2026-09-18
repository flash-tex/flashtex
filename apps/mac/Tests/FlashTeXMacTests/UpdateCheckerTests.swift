import XCTest
@testable import FlashTeXMac

/// `UpdateChecker` (#694 slice 1) against a stub transport: offline →
/// `.failed`, preference off → `.disabled` with no request, the 24 h
/// throttle, skipped-version suppression, HTTP errors and the preference
/// round trip. Deterministic: injected clock, temporary `UserDefaults`.
@MainActor
final class UpdateCheckerTests: XCTestCase {
    private var suiteName = ""
    private var defaults: UserDefaults!
    private var prefs: EditorPreferences!

    override func setUp() {
        super.setUp()
        suiteName = "flashtex.tests.UpdateChecker.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
        defaults.removePersistentDomain(forName: suiteName)
        prefs = EditorPreferences(defaults: defaults)
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil; prefs = nil
        super.tearDown()
    }

    // MARK: stub transport

    final class StubTransport: UpdateFeedTransport, @unchecked Sendable {
        enum Reply { case data(String, status: Int), error(Error) }
        var reply: Reply
        private(set) var requests: [URLRequest] = []
        init(_ reply: Reply) { self.reply = reply }

        func data(for request: URLRequest) async throws -> (Data, URLResponse) {
            requests.append(request)
            switch reply {
            case .error(let e): throw e
            case .data(let body, let status):
                let response = HTTPURLResponse(url: request.url!, statusCode: status, httpVersion: nil, headerFields: nil)!
                return (Data(body.utf8), response)
            }
        }
    }

    static let latestJSON = UpdateFeedTests.releaseJSON // v0.1.8
    let t0 = Date(timeIntervalSince1970: 1_800_000_000)

    private func checker(_ transport: StubTransport, current: String? = "0.1.5", now: Date? = nil) -> UpdateChecker {
        let clock = now ?? t0
        return UpdateChecker(transport: transport, preferences: prefs, currentVersion: current.flatMap(SemanticVersion.init), now: { clock })
    }

    // MARK: manual check

    func testManualCheckFindsAnUpdate() async throws {
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        let c = checker(transport, current: "0.1.5")
        let result = await c.check()
        guard case .available(let release, let current) = result else { return XCTFail("\(result)") }
        XCTAssertEqual(release.tagName, "v0.1.8")
        XCTAssertEqual(current, SemanticVersion("0.1.5"))
        XCTAssertEqual(transport.requests.count, 1)
        let request = try XCTUnwrap(transport.requests.first)
        XCTAssertEqual(request.url, UpdateFeed.latestReleaseURL)
        XCTAssertEqual(request.url?.absoluteString, "https://api.github.com/repos/flash-tex/flashtex/releases/latest")
        XCTAssertEqual(request.timeoutInterval, 10)
        XCTAssertEqual(request.value(forHTTPHeaderField: "Accept"), "application/vnd.github+json")
        XCTAssertEqual(prefs.lastUpdateCheck, t0, "a completed check is recorded")
        XCTAssertEqual(defaults.object(forKey: EditorPreferences.Key.lastUpdateCheck.storageKey) as? Date, t0, "…and persisted")
    }

    func testManualCheckReportsUpToDateAndNewerInstalled() async {
        let up = await checker(StubTransport(.data(Self.latestJSON, status: 200)), current: "0.1.8").check()
        XCTAssertEqual(up, .upToDate(SemanticVersion("0.1.8")!))
        let newer = await checker(StubTransport(.data(Self.latestJSON, status: 200)), current: "0.2.0").check()
        XCTAssertEqual(newer, .upToDate(SemanticVersion("0.2.0")!))
    }

    func testManualCheckIgnoresPreferenceThrottleAndSkip() async {
        prefs.checkForUpdatesAutomatically = false
        prefs.lastUpdateCheck = t0
        prefs.skippedUpdateVersion = "v0.1.8"
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        let result = await checker(transport, now: t0.addingTimeInterval(60)).check()
        guard case .available = result else { return XCTFail("a manual check always asks and always tells: \(result)") }
        XCTAssertEqual(transport.requests.count, 1)
    }

    func testOfflineIsFailedWithOneLineReason() async {
        let transport = StubTransport(.error(URLError(.notConnectedToInternet)))
        let result = await checker(transport).check()
        XCTAssertEqual(result, .failed("You appear to be offline."))
        XCTAssertNil(prefs.lastUpdateCheck, "a request that never completed is not a check")
        let timeout = await checker(StubTransport(.error(URLError(.timedOut)))).check()
        XCTAssertEqual(timeout, .failed("GitHub did not answer within 10 seconds."))
        let other = await checker(StubTransport(.error(NSError(domain: "x", code: 1, userInfo: [NSLocalizedDescriptionKey: "first line\nsecond line"])))).check()
        XCTAssertEqual(other, .failed("first line"))
    }

    func testHTTPErrorAndMalformedFeedAreFailedNeverAvailable() async {
        let http = await checker(StubTransport(.data(Self.latestJSON, status: 503))).check()
        XCTAssertEqual(http, .failed("GitHub answered HTTP 503."))
        let malformed = await checker(StubTransport(.data("<html>rate limited</html>", status: 200))).check()
        XCTAssertEqual(malformed, .failed("The release feed was not valid JSON."))
        let evil = Self.latestJSON.replacingOccurrences(of: "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/FlashTeX.dmg", with: "https://evil.example/FlashTeX.dmg")
        let untrusted = await checker(StubTransport(.data(evil, status: 200))).check()
        XCTAssertEqual(untrusted, .failed("Refused a download outside github.com/flash-tex/flashtex/releases: https://evil.example/FlashTeX.dmg"))
        let badTag = Self.latestJSON.replacingOccurrences(of: "\"tag_name\": \"v0.1.8\",", with: "\"tag_name\": \"latest\",")
        let unparsable = await checker(StubTransport(.data(badTag, status: 200))).check()
        XCTAssertEqual(unparsable, .failed("The release tag “latest” is not a version number."))
    }

    func testUnversionedBuildCannotCheck() async {
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        let result = await checker(transport, current: nil).check()
        guard case .failed(let reason) = result else { return XCTFail("\(result)") }
        XCTAssertTrue(reason.contains("no version number"), reason)
        XCTAssertEqual(transport.requests.count, 0, "nothing to compare against, so nothing is asked")
    }

    // MARK: automatic check

    func testPreferenceOffIsDisabledAndMakesNoRequest() async {
        XCTAssertFalse(prefs.checkForUpdatesAutomatically, "background checking is opt-in")
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        let result = await checker(transport).automaticCheckIfDue()
        XCTAssertEqual(result, .disabled)
        XCTAssertEqual(transport.requests.count, 0)
        XCTAssertNil(prefs.lastUpdateCheck)
    }

    func testAutomaticCheckSurfacesOnlyAnAvailableUpdate() async {
        prefs.checkForUpdatesAutomatically = true
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        let result = await checker(transport, current: "0.1.5").automaticCheckIfDue()
        guard case .available(let release, _)? = result else { return XCTFail("\(String(describing: result))") }
        XCTAssertEqual(release.tagName, "v0.1.8")
        XCTAssertEqual(transport.requests.count, 1)
        XCTAssertEqual(prefs.lastUpdateCheck, t0)

        prefs.lastUpdateCheck = nil
        let upToDate = await checker(StubTransport(.data(Self.latestJSON, status: 200)), current: "0.1.8").automaticCheckIfDue()
        XCTAssertNil(upToDate, "quiet when up to date")
        prefs.lastUpdateCheck = nil
        let offline = await checker(StubTransport(.error(URLError(.notConnectedToInternet)))).automaticCheckIfDue()
        XCTAssertNil(offline, "quiet when the check fails")
    }

    func testAutomaticCheckIsThrottledTo24Hours() async {
        prefs.checkForUpdatesAutomatically = true
        prefs.lastUpdateCheck = t0
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        // 23 h 59 m later: not due.
        let early = await checker(transport, now: t0.addingTimeInterval(24 * 3600 - 60)).automaticCheckIfDue()
        XCTAssertNil(early)
        XCTAssertEqual(transport.requests.count, 0)
        XCTAssertEqual(prefs.lastUpdateCheck, t0, "the stamp is untouched by a skipped check")
        // Exactly 24 h later: due, and the stamp moves.
        let due = await checker(transport, now: t0.addingTimeInterval(24 * 3600)).automaticCheckIfDue()
        XCTAssertNotNil(due)
        XCTAssertEqual(transport.requests.count, 1)
        XCTAssertEqual(prefs.lastUpdateCheck, t0.addingTimeInterval(24 * 3600))
        // Pure rule.
        XCTAssertTrue(UpdateChecker.isDue(lastCheck: nil, now: t0))
        XCTAssertFalse(UpdateChecker.isDue(lastCheck: t0, now: t0))
        XCTAssertTrue(UpdateChecker.isDue(lastCheck: t0.addingTimeInterval(-UpdateChecker.automaticInterval), now: t0))
        XCTAssertTrue(UpdateChecker.isDue(lastCheck: t0.addingTimeInterval(3600), now: t0.addingTimeInterval(3600 + 86_400)), "a clock that was ahead still counts a full day")
    }

    func testSkippedVersionIsSuppressedUntilANewerOneAppears() async {
        prefs.checkForUpdatesAutomatically = true
        prefs.skippedUpdateVersion = "v0.1.8"
        let transport = StubTransport(.data(Self.latestJSON, status: 200))
        let skipped = await checker(transport, current: "0.1.5").automaticCheckIfDue()
        XCTAssertNil(skipped, "the skipped tag stays quiet")
        XCTAssertEqual(transport.requests.count, 1, "…but the check still ran and was recorded")
        XCTAssertEqual(prefs.lastUpdateCheck, t0)
        // A newer release than the skipped one is surfaced again.
        prefs.lastUpdateCheck = nil
        let newer = Self.latestJSON.replacingOccurrences(of: "v0.1.8", with: "v0.1.9").replacingOccurrences(of: "0.1.8", with: "0.1.9")
        let result = await checker(StubTransport(.data(newer, status: 200)), current: "0.1.5").automaticCheckIfDue()
        guard case .available(let release, _)? = result else { return XCTFail("\(String(describing: result))") }
        XCTAssertEqual(release.tagName, "v0.1.9")
    }

    // MARK: preferences

    func testUpdatePreferencesDefaultsPersistenceAndReset() {
        XCTAssertFalse(prefs.checkForUpdatesAutomatically)
        XCTAssertNil(prefs.lastUpdateCheck)
        XCTAssertNil(prefs.skippedUpdateVersion)
        XCTAssertTrue(prefs.lastLoadRepairs.contains(.checkForUpdatesAutomatically), "the absent flag is written back as the default")
        XCTAssertFalse(prefs.lastLoadRepairs.contains(.lastUpdateCheck)); XCTAssertFalse(prefs.lastLoadRepairs.contains(.skippedUpdateVersion))
        XCTAssertEqual(defaults.object(forKey: EditorPreferences.Key.checkForUpdatesAutomatically.storageKey) as? Bool, false)

        let g = prefs.generation
        prefs.checkForUpdatesAutomatically = true
        prefs.skippedUpdateVersion = "  v0.1.8 "
        prefs.lastUpdateCheck = t0
        XCTAssertEqual(prefs.skippedUpdateVersion, "v0.1.8", "trimmed")
        XCTAssertEqual(prefs.generation, g + 3)
        prefs.skippedUpdateVersion = ""
        XCTAssertNil(prefs.skippedUpdateVersion, "empty clears")
        prefs.skippedUpdateVersion = "v0.1.8"

        let reloaded = EditorPreferences(defaults: defaults)
        XCTAssertTrue(reloaded.checkForUpdatesAutomatically)
        XCTAssertEqual(reloaded.skippedUpdateVersion, "v0.1.8")
        XCTAssertEqual(reloaded.lastUpdateCheck, t0)
        XCTAssertEqual(reloaded.snapshot, EditorPreferences.defaultSnapshot, "update settings are not part of the editor display snapshot")

        // Wrong types are repaired.
        defaults.set(12, forKey: EditorPreferences.Key.skippedUpdateVersion.storageKey)
        defaults.set("yesterday", forKey: EditorPreferences.Key.lastUpdateCheck.storageKey)
        let repaired = EditorPreferences(defaults: defaults)
        XCTAssertNil(repaired.skippedUpdateVersion); XCTAssertNil(repaired.lastUpdateCheck)
        XCTAssertTrue(repaired.lastLoadRepairs.contains(.skippedUpdateVersion))

        reloaded.resetToDefaults()
        XCTAssertFalse(reloaded.checkForUpdatesAutomatically); XCTAssertNil(reloaded.skippedUpdateVersion); XCTAssertNil(reloaded.lastUpdateCheck)
        XCTAssertNil(defaults.object(forKey: EditorPreferences.Key.skippedUpdateVersion.storageKey))
    }

    func testSettingsFootnote() {
        XCTAssertEqual(UpdateChecker.settingsFootnote(current: nil, lastCheck: nil, skipped: nil), "Installed: unversioned development build. Never checked.")
        let line = UpdateChecker.settingsFootnote(current: SemanticVersion("0.1.8"), lastCheck: t0, skipped: "v0.1.9")
        XCTAssertTrue(line.hasPrefix("Installed: FlashTeX 0.1.8. Last checked "), line)
        XCTAssertTrue(line.hasSuffix(". Skipping v0.1.9."), line)
    }

    /// The alert's release-notes accessory is built without a window and
    /// stays empty for empty notes; the `.disabled` result never presents.
    func testNotesAccessoryView() {
        XCTAssertNil(UpdatePresenter.notesView(""))
        let view = UpdatePresenter.notesView("## Fixes\n- one")
        XCTAssertNotNil(view)
        XCTAssertEqual(((view as? NSScrollView)?.documentView as? NSTextView)?.string, "Fixes\n- one")
        XCTAssertEqual(view?.frame.size, NSSize(width: DS.Layout.updateNotesWidth, height: DS.Layout.updateNotesHeight))
        XCTAssertFalse(UpdatePresenter.shared.inProgress)
        UpdatePresenter.shared.present(.disabled, manual: false) // no alert, returns immediately
    }
}
