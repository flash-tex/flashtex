import XCTest
@testable import FlashTeXMac

/// `UpdateFeed.swift` (#694 slice 1): version comparison, strict feed
/// decoding with typed errors, up-to-date / available / newer-installed,
/// release-notes flattening and the `SHA256SUMS` hook. No network.
final class UpdateFeedTests: XCTestCase {

    // MARK: SemanticVersion

    func testParsesTagsWithAndWithoutPrefix() throws {
        XCTAssertEqual(SemanticVersion("v0.1.5"), SemanticVersion(major: 0, minor: 1, patch: 5))
        XCTAssertEqual(SemanticVersion("0.1.5"), SemanticVersion(major: 0, minor: 1, patch: 5))
        XCTAssertEqual(SemanticVersion("V1.2"), SemanticVersion(major: 1, minor: 2, patch: 0))
        XCTAssertEqual(SemanticVersion("2"), SemanticVersion(major: 2, minor: 0, patch: 0))
        XCTAssertEqual(SemanticVersion("0.2.0-beta.1"), SemanticVersion(major: 0, minor: 2, patch: 0, prerelease: ["beta", "1"]))
        XCTAssertEqual(SemanticVersion("1.0.0-rc.1+build.7"), SemanticVersion(major: 1, minor: 0, patch: 0, prerelease: ["rc", "1"]), "build metadata ignored")
        XCTAssertEqual(SemanticVersion(" v0.1.8\n"), SemanticVersion(major: 0, minor: 1, patch: 8))
        XCTAssertEqual(SemanticVersion("0.2.0-beta.1")?.description, "0.2.0-beta.1")
        XCTAssertEqual(SemanticVersion("v0.1.8")?.description, "0.1.8")
    }

    func testRejectsMalformedVersions() {
        for bad in ["", "v", "latest", "1.2.3.4", "1..2", "a.b.c", "1.2.3-", "1.2.3-beta..1", "1.2.3-be ta", "-1.0.0", "v1.0.x"] {
            XCTAssertNil(SemanticVersion(bad), bad)
        }
    }

    func testOrderingTable() {
        // Each entry is strictly less than the next (SemVer 2.0 §11 examples plus the v-prefix).
        let ascending = ["0.0.9", "v0.1.0", "0.1.5", "v0.1.8", "0.2.0-alpha", "0.2.0-alpha.1", "0.2.0-alpha.beta", "0.2.0-beta",
                         "0.2.0-beta.2", "0.2.0-beta.11", "0.2.0-rc.1", "0.2.0", "v1.0.0-rc.1", "1.0.0", "1.0.1", "1.1.0", "2.0.0"]
        let parsed = ascending.map { SemanticVersion($0)! }
        for i in 0..<(parsed.count - 1) {
            XCTAssertLessThan(parsed[i], parsed[i + 1], "\(ascending[i]) < \(ascending[i + 1])")
            XCTAssertFalse(parsed[i + 1] < parsed[i])
        }
        XCTAssertEqual(SemanticVersion("v0.1.5"), SemanticVersion("0.1.5"))
        XCTAssertFalse(SemanticVersion("0.1.5")! < SemanticVersion("v0.1.5")!)
        XCTAssertEqual(SemanticVersion("1.0.0+a"), SemanticVersion("1.0.0+b"), "build metadata does not affect equality")
        XCTAssertEqual(parsed.shuffled().sorted(), parsed)
    }

    // MARK: feed decoding

    static let releaseJSON = """
    {
      "tag_name": "v0.1.8",
      "name": "FlashTeX v0.1.8",
      "draft": false,
      "prerelease": false,
      "html_url": "https://github.com/flash-tex/flashtex/releases/tag/v0.1.8",
      "published_at": "2026-09-17T10:55:33Z",
      "body": "## Downloads\\n\\n| Asset | What |\\n|---|---|\\n| `FlashTeX.dmg` | The Mac app |\\n\\n**Fixes**: [#123](https://github.com/flash-tex/flashtex/issues/123) hyphenation",
      "assets": [
        {"name": "FlashTeX-0.1.8-macos-arm64.dmg", "browser_download_url": "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/FlashTeX-0.1.8-macos-arm64.dmg"},
        {"name": "flashtex-cli-0.1.8-macos-arm64.tar.gz", "browser_download_url": "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/flashtex-cli-0.1.8-macos-arm64.tar.gz"},
        {"name": "FlashTeX.dmg", "browser_download_url": "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/FlashTeX.dmg"},
        {"name": "SHA256SUMS", "browser_download_url": "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/SHA256SUMS"}
      ]
    }
    """

    func testDecodesAValidRelease() throws {
        let r = try UpdateFeed.decodeRelease(Data(Self.releaseJSON.utf8))
        XCTAssertEqual(r.version, SemanticVersion("0.1.8"))
        XCTAssertEqual(r.tagName, "v0.1.8")
        XCTAssertEqual(r.htmlURL.absoluteString, "https://github.com/flash-tex/flashtex/releases/tag/v0.1.8")
        XCTAssertEqual(r.publishedAt, ISO8601DateFormatter().date(from: "2026-09-17T10:55:33Z"))
        XCTAssertEqual(r.dmgAssetURL?.lastPathComponent, "FlashTeX-0.1.8-macos-arm64.dmg", "the versioned DMG wins over the stable name")
        XCTAssertEqual(r.sha256sumsURL?.absoluteString, "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/SHA256SUMS")
        XCTAssertTrue(r.notesMarkdown.hasPrefix("## Downloads"))
    }

    func testStableDMGNameIsUsedWhenItIsTheOnlyOne() throws {
        let json = Self.releaseJSON.replacingOccurrences(of: "FlashTeX-0.1.8-macos-arm64.dmg", with: "other.txt")
        let r = try UpdateFeed.decodeRelease(Data(json.utf8))
        XCTAssertEqual(r.dmgAssetURL?.lastPathComponent, "FlashTeX.dmg")
    }

    func testReleaseWithoutAssetsDecodesWithNilAssetURLs() throws {
        let json = """
        {"tag_name": "v0.1.9", "html_url": "https://github.com/flash-tex/flashtex/releases/tag/v0.1.9"}
        """
        let r = try UpdateFeed.decodeRelease(Data(json.utf8))
        XCTAssertNil(r.dmgAssetURL); XCTAssertNil(r.sha256sumsURL); XCTAssertNil(r.publishedAt); XCTAssertEqual(r.notesMarkdown, "")
    }

    func testMalformedJSONIsATypedError() {
        for bad in ["", "{", "[]", "\"v0.1.8\"", "null", "{\"tag_name\": [1]}"] {
            XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(bad.utf8)), bad) { error in
                let e = error as? UpdateFeed.DecodeError
                XCTAssertTrue(e == .malformedJSON || e == .missingTagName, "\(bad): \(error)")
            }
        }
    }

    func testMissingTagNameIsATypedError() {
        let json = Self.releaseJSON.replacingOccurrences(of: "\"tag_name\": \"v0.1.8\",", with: "")
        XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(json.utf8))) { XCTAssertEqual($0 as? UpdateFeed.DecodeError, .missingTagName) }
        let empty = Self.releaseJSON.replacingOccurrences(of: "\"tag_name\": \"v0.1.8\",", with: "\"tag_name\": \"\",")
        XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(empty.utf8))) { XCTAssertEqual($0 as? UpdateFeed.DecodeError, .missingTagName) }
    }

    func testUnparsableVersionIsATypedError() {
        let json = Self.releaseJSON.replacingOccurrences(of: "\"tag_name\": \"v0.1.8\",", with: "\"tag_name\": \"nightly-2026-09-17\",")
        XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(json.utf8))) {
            XCTAssertEqual($0 as? UpdateFeed.DecodeError, .unparsableVersion("nightly-2026-09-17"))
        }
    }

    func testAssetOutsideTheTrustedHostIsRejected() {
        let evil = "https://github.com/someone-else/flashtex/releases/download/v0.1.8/FlashTeX.dmg"
        let json = Self.releaseJSON.replacingOccurrences(of: "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/FlashTeX.dmg", with: evil)
        XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(json.utf8))) {
            XCTAssertEqual($0 as? UpdateFeed.DecodeError, .untrustedAssetURL(evil))
        }
        // Same path on a look-alike host, and plain http, are also refused.
        for host in ["https://github.com.evil.example/flash-tex/flashtex/releases/download/v0.1.8/FlashTeX.dmg",
                     "http://github.com/flash-tex/flashtex/releases/download/v0.1.8/FlashTeX.dmg"] {
            let j = Self.releaseJSON.replacingOccurrences(of: "https://github.com/flash-tex/flashtex/releases/download/v0.1.8/SHA256SUMS", with: host)
            XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(j.utf8)), host) {
                XCTAssertEqual($0 as? UpdateFeed.DecodeError, .untrustedAssetURL(host))
            }
        }
    }

    func testReleasePageOutsideTheRepositoryIsRejected() {
        let evil = "https://github.com/flash-tex/other/releases/tag/v0.1.8"
        let json = Self.releaseJSON.replacingOccurrences(of: "https://github.com/flash-tex/flashtex/releases/tag/v0.1.8", with: evil)
        XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(json.utf8))) {
            XCTAssertEqual($0 as? UpdateFeed.DecodeError, .untrustedReleasePage(evil))
        }
        let missing = Self.releaseJSON.replacingOccurrences(of: "\"html_url\": \"https://github.com/flash-tex/flashtex/releases/tag/v0.1.8\",", with: "")
        XCTAssertThrowsError(try UpdateFeed.decodeRelease(Data(missing.utf8))) { XCTAssertEqual($0 as? UpdateFeed.DecodeError, .missingHTMLURL) }
    }

    func testNewestReleaseFromAListSkipsDraftsAndPrereleases() throws {
        func entry(_ tag: String, draft: Bool = false, prerelease: Bool = false) -> String {
            """
            {"tag_name": "\(tag)", "draft": \(draft), "prerelease": \(prerelease), "html_url": "https://github.com/flash-tex/flashtex/releases/tag/\(tag)", "assets": []}
            """
        }
        let list = "[\(entry("v0.2.0", draft: true)), \(entry("v0.2.0-beta.1", prerelease: true)), \(entry("v0.1.7")), \(entry("v0.1.8"))]"
        XCTAssertEqual(try UpdateFeed.decodeNewestRelease(Data(list.utf8)).tagName, "v0.1.8")
        XCTAssertEqual(try UpdateFeed.decodeNewestRelease(Data(list.utf8), includePrereleases: true).tagName, "v0.2.0-beta.1", "drafts are never candidates")
        XCTAssertThrowsError(try UpdateFeed.decodeNewestRelease(Data("[\(entry("v0.2.0", draft: true))]".utf8))) {
            XCTAssertEqual($0 as? UpdateFeed.DecodeError, .noPublishedRelease)
        }
        XCTAssertThrowsError(try UpdateFeed.decodeNewestRelease(Data("{}".utf8))) { XCTAssertEqual($0 as? UpdateFeed.DecodeError, .malformedJSON) }
        // One bad entry poisons the list: strictness over "found something".
        let poisoned = "[\(entry("v0.1.8")), \(entry("v0.1.9").replacingOccurrences(of: "\"assets\": []", with: "\"assets\": [{\"name\": \"FlashTeX.dmg\", \"browser_download_url\": \"https://example.com/FlashTeX.dmg\"}]"))]"
        XCTAssertThrowsError(try UpdateFeed.decodeNewestRelease(Data(poisoned.utf8)))
    }

    // MARK: comparison outcomes

    func testCompareUpToDateAvailableAndNewerInstalled() throws {
        let latest = try UpdateFeed.decodeRelease(Data(Self.releaseJSON.utf8)) // 0.1.8
        XCTAssertEqual(UpdateCheck.compare(latest: latest, current: SemanticVersion("0.1.8")!), .upToDate(SemanticVersion("0.1.8")!))
        XCTAssertEqual(UpdateCheck.compare(latest: latest, current: SemanticVersion("0.1.5")!), .available(latest, current: SemanticVersion("0.1.5")!))
        XCTAssertEqual(UpdateCheck.compare(latest: latest, current: SemanticVersion("0.1.9")!), .upToDate(SemanticVersion("0.1.9")!), "a newer local build is not offered a downgrade")
        XCTAssertEqual(UpdateCheck.compare(latest: latest, current: SemanticVersion("0.1.8-beta.1")!), .available(latest, current: SemanticVersion("0.1.8-beta.1")!), "a pre-release build is offered its release")
    }

    // MARK: release notes

    func testReleaseNotesFlattenToPlainText() throws {
        let r = try UpdateFeed.decodeRelease(Data(Self.releaseJSON.utf8))
        let text = UpdateFeed.plainText(fromMarkdown: r.notesMarkdown)
        XCTAssertEqual(text, "Downloads\n\nAsset — What\nFlashTeX.dmg — The Mac app\n\nFixes: #123 hyphenation")
        XCTAssertEqual(UpdateFeed.plainText(fromMarkdown: ""), "")
        XCTAssertEqual(UpdateFeed.plainText(fromMarkdown: "a\r\n\r\n\r\nb"), "a\n\nb")
    }

    // MARK: SHA256SUMS (slice 2 hook)

    static let sums = """
    2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824  FlashTeX-0.1.8-macos-arm64.dmg
    2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824 *FlashTeX.dmg
    deadbeef  not-a-digest.txt
    e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\tdist/empty.bin
    garbage line without a digest
    """

    func testParsesSHA256SUMS() {
        let parsed = UpdateFeed.parseSHA256SUMS(Self.sums)
        XCTAssertEqual(parsed, [
            "FlashTeX-0.1.8-macos-arm64.dmg": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "FlashTeX.dmg": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "empty.bin": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ])
    }

    func testVerifySHA256MatchAndMismatch() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("UpdateFeedTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let hello = dir.appendingPathComponent("FlashTeX.dmg")
        try Data("hello".utf8).write(to: hello) // sha256("hello") = 2cf24dba…
        let empty = dir.appendingPathComponent("empty.bin")
        try Data().write(to: empty)

        XCTAssertNil(failure(UpdateFeed.verifySHA256(of: hello, against: Self.sums, assetName: "FlashTeX.dmg")))
        XCTAssertNil(failure(UpdateFeed.verifySHA256(of: hello, against: Self.sums, assetName: "FlashTeX-0.1.8-macos-arm64.dmg")))
        XCTAssertNil(failure(UpdateFeed.verifySHA256(of: empty, against: Self.sums, assetName: "empty.bin")))
        XCTAssertEqual(failure(UpdateFeed.verifySHA256(of: empty, against: Self.sums, assetName: "FlashTeX.dmg")),
                       .mismatch(expected: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
                                          actual: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))
        XCTAssertEqual(failure(UpdateFeed.verifySHA256(of: hello, against: Self.sums, assetName: "not-a-digest.txt")), .assetNotListed("not-a-digest.txt"))
        XCTAssertEqual(failure(UpdateFeed.verifySHA256(of: dir.appendingPathComponent("missing"), against: Self.sums, assetName: "FlashTeX.dmg")), .unreadableFile)
    }
}

/// nil for `.success`, the error otherwise (Result<Void, _> is not Equatable).
private func failure(_ r: Result<Void, UpdateFeed.VerifyError>) -> UpdateFeed.VerifyError? {
    if case .failure(let e) = r { return e }
    return nil
}
