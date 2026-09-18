import CryptoKit
import Foundation

/// The update feed as pure, testable values (#694, slice 1).
///
/// FlashTeX is distributed as GitHub Releases on `flash-tex/flashtex`
/// (tags `vX.Y.Z`; assets `FlashTeX-<ver>-macos-arm64.dmg`, `FlashTeX.dmg`,
/// CLI tarballs and a `SHA256SUMS` file). This file knows how to read that
/// feed and how to compare versions; it never touches the network, the file
/// system (except `verifySHA256`, which hashes a file it is handed) or the
/// running application. `UpdateChecker.swift` performs the request and
/// owns the menu item.
///
/// Slices (the issue says: no self-update until artifacts are signed and
/// verifiable):
/// 1. (this) check the feed, show version + notes, open the release page.
/// 2. download the signed DMG, verify it with `verifySHA256` against the
///    release's `SHA256SUMS`, then `open` the DMG for the user.
/// 3. in-place replacement of the running app, only after notarisation of
///    the DMG is confirmed and a rollback path exists.
enum UpdateFeed {
    /// The GitHub repository the feed is read from and the only host a
    /// download URL may point at.
    static let owner = "flash-tex"
    static let repository = "flashtex"
    static let latestReleaseURL = URL(string: "https://api.github.com/repos/\(owner)/\(repository)/releases/latest")!
    static let releasesURL = URL(string: "https://api.github.com/repos/\(owner)/\(repository)/releases?per_page=10")!
    /// Every asset must live under this prefix or the release is rejected.
    static let trustedDownloadPrefix = "https://github.com/\(owner)/\(repository)/releases/download/"
    /// The release page prefix (`Download` in the alert opens this).
    static let trustedReleasePagePrefix = "https://github.com/\(owner)/\(repository)/releases/tag/"

    // MARK: release

    struct Release: Equatable, Sendable {
        var version: SemanticVersion
        var tagName: String
        var htmlURL: URL
        var publishedAt: Date?
        var notesMarkdown: String
        /// The Mac app disk image (`FlashTeX-<ver>-macos-arm64.dmg`, else `FlashTeX.dmg`).
        var dmgAssetURL: URL?
        /// The release's `SHA256SUMS` asset (slice 2 verifies the DMG against it).
        var sha256sumsURL: URL?
    }

    enum DecodeError: Error, Equatable, Sendable {
        case malformedJSON
        case missingTagName
        case unparsableVersion(String)
        case missingHTMLURL
        case untrustedReleasePage(String)
        case untrustedAssetURL(String)
        case noPublishedRelease
    }

    /// Decodes one release object (`/releases/latest`). Strict: anything
    /// unexpected is a typed error, never a "found update".
    static func decodeRelease(_ data: Data) throws -> Release {
        guard let object = try? JSONSerialization.jsonObject(with: data), let json = object as? [String: Any] else {
            throw DecodeError.malformedJSON
        }
        return try release(from: json)
    }

    /// Decodes a `/releases` list and returns the newest published,
    /// non-draft release (pre-releases only when `includePrereleases`).
    static func decodeNewestRelease(_ data: Data, includePrereleases: Bool = false) throws -> Release {
        guard let object = try? JSONSerialization.jsonObject(with: data), let list = object as? [[String: Any]] else {
            throw DecodeError.malformedJSON
        }
        let candidates = try list
            .filter { ($0["draft"] as? Bool) != true && (includePrereleases || ($0["prerelease"] as? Bool) != true) }
            .map(release(from:))
        guard let newest = candidates.max(by: { $0.version < $1.version }) else { throw DecodeError.noPublishedRelease }
        return newest
    }

    static func release(from json: [String: Any]) throws -> Release {
        guard let tag = json["tag_name"] as? String, !tag.isEmpty else { throw DecodeError.missingTagName }
        guard let version = SemanticVersion(tag) else { throw DecodeError.unparsableVersion(tag) }
        guard let html = json["html_url"] as? String, let htmlURL = URL(string: html) else { throw DecodeError.missingHTMLURL }
        guard html.hasPrefix(trustedReleasePagePrefix) else { throw DecodeError.untrustedReleasePage(html) }
        var dmg: URL?
        var sums: URL?
        for asset in (json["assets"] as? [[String: Any]]) ?? [] {
            guard let name = asset["name"] as? String, let raw = asset["browser_download_url"] as? String else { continue }
            guard raw.hasPrefix(trustedDownloadPrefix), let url = URL(string: raw) else { throw DecodeError.untrustedAssetURL(raw) }
            if name == "SHA256SUMS" { sums = url }
            if name.hasSuffix(".dmg") {
                // Prefer the versioned name; `FlashTeX.dmg` is the same bytes under a stable name.
                if dmg == nil || name != "FlashTeX.dmg" { dmg = url }
            }
        }
        let published = (json["published_at"] as? String).flatMap { ISO8601DateFormatter().date(from: $0) }
        return Release(version: version, tagName: tag, htmlURL: htmlURL, publishedAt: published,
                       notesMarkdown: (json["body"] as? String) ?? "", dmgAssetURL: dmg, sha256sumsURL: sums)
    }

    // MARK: release notes

    /// A plain-text rendering of GitHub-flavoured release notes for an alert:
    /// headings lose their `#`, emphasis and code markers are dropped, table
    /// rows become "cell — cell" lines, links keep their text.
    static func plainText(fromMarkdown markdown: String) -> String {
        var out: [String] = []
        for rawLine in markdown.replacingOccurrences(of: "\r\n", with: "\n").split(separator: "\n", omittingEmptySubsequences: false) {
            var line = String(rawLine)
            if line.range(of: #"^\s*\|?\s*:?-{3,}"#, options: .regularExpression) != nil { continue } // table rule
            if line.hasPrefix("|") {
                let cells = line.split(separator: "|").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
                line = cells.joined(separator: " — ")
            }
            line = line.replacingOccurrences(of: #"^\s*#{1,6}\s*"#, with: "", options: .regularExpression)
            line = line.replacingOccurrences(of: #"\[([^\]]+)\]\([^)]*\)"#, with: "$1", options: .regularExpression)
            line = line.replacingOccurrences(of: "`", with: "")
            line = line.replacingOccurrences(of: #"(\*\*|__)"#, with: "", options: .regularExpression)
            out.append(line.trimmingCharacters(in: .whitespaces))
        }
        // Collapse runs of blank lines.
        var collapsed: [String] = []
        for l in out where !(l.isEmpty && collapsed.last?.isEmpty == true) { collapsed.append(l) }
        return collapsed.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // MARK: SHA256SUMS (slice 2 hook; nothing calls this in slice 1)

    enum VerifyError: Error, Equatable, Sendable {
        case assetNotListed(String)
        case unreadableFile
        case mismatch(expected: String, actual: String)
    }

    /// Parses `sha256sum`-style lines (`<hex>  <name>` or `<hex> *<name>`)
    /// into name → lowercase hex.
    static func parseSHA256SUMS(_ text: String) -> [String: String] {
        var out: [String: String] = [:]
        for line in text.split(whereSeparator: \.isNewline) {
            let parts = line.split(maxSplits: 1, whereSeparator: { $0 == " " || $0 == "\t" })
            guard parts.count == 2 else { continue }
            let hex = parts[0].lowercased()
            guard hex.count == 64, hex.allSatisfy(\.isHexDigit) else { continue }
            var name = parts[1].trimmingCharacters(in: .whitespaces)
            if name.hasPrefix("*") { name.removeFirst() }
            // Strip any directory component the sums file may carry.
            name = String(name.split(separator: "/").last ?? Substring(name))
            out[name] = hex
        }
        return out
    }

    /// Verifies that the file at `fileURL` hashes to the entry for
    /// `assetName` in `sumsText`. Pure apart from reading the file. Slice 2
    /// wires this between download and `open`; slice 3 (in-place replacement)
    /// additionally requires a notarised, stapled DMG.
    static func verifySHA256(of fileURL: URL, against sumsText: String, assetName: String) -> Result<Void, VerifyError> {
        guard let expected = parseSHA256SUMS(sumsText)[assetName] else { return .failure(.assetNotListed(assetName)) }
        guard let data = try? Data(contentsOf: fileURL) else { return .failure(.unreadableFile) }
        let actual = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        return actual == expected ? .success(()) : .failure(.mismatch(expected: expected, actual: actual))
    }
}

// MARK: - Result of a check

enum UpdateCheck: Equatable, Sendable {
    case upToDate(SemanticVersion)
    case available(UpdateFeed.Release, current: SemanticVersion)
    case failed(String)
    case disabled

    /// Pure decision given what the feed returned and what is installed.
    static func compare(latest: UpdateFeed.Release, current: SemanticVersion) -> UpdateCheck {
        latest.version > current ? .available(latest, current: current) : .upToDate(current)
    }
}

// MARK: - Semantic version

/// `MAJOR.MINOR.PATCH[-prerelease][+build]`, with an optional leading `v`.
/// Missing minor/patch read as 0 (`v0.2` == `0.2.0`). Ordering follows
/// SemVer 2.0 §11: numeric parts, then a pre-release sorts *before* its
/// release (`0.2.0-beta.1 < 0.2.0`), pre-release identifiers compare
/// numerically when both are digits, lexically otherwise, digits before
/// letters, and a shorter identifier list is lower. Build metadata is ignored.
struct SemanticVersion: Hashable, Comparable, CustomStringConvertible, Sendable {
    var major: Int
    var minor: Int
    var patch: Int
    var prerelease: [String]

    init(major: Int, minor: Int, patch: Int, prerelease: [String] = []) {
        self.major = major; self.minor = minor; self.patch = patch; self.prerelease = prerelease
    }

    init?(_ text: String) {
        var s = Substring(text.trimmingCharacters(in: .whitespacesAndNewlines))
        if s.hasPrefix("v") || s.hasPrefix("V") { s = s.dropFirst() }
        if let plus = s.firstIndex(of: "+") { s = s[..<plus] } // build metadata
        var pre: [String] = []
        if let dash = s.firstIndex(of: "-") {
            let tail = s[s.index(after: dash)...]
            guard !tail.isEmpty else { return nil }
            pre = tail.split(separator: ".", omittingEmptySubsequences: false).map(String.init)
            guard pre.allSatisfy({ !$0.isEmpty && $0.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" } }) else { return nil }
            s = s[..<dash]
        }
        let parts = s.split(separator: ".", omittingEmptySubsequences: false)
        guard (1...3).contains(parts.count) else { return nil }
        var nums: [Int] = []
        for p in parts {
            guard !p.isEmpty, p.allSatisfy(\.isNumber), let n = Int(p) else { return nil }
            nums.append(n)
        }
        self.init(major: nums[0], minor: nums.count > 1 ? nums[1] : 0, patch: nums.count > 2 ? nums[2] : 0, prerelease: pre)
    }

    var description: String {
        "\(major).\(minor).\(patch)" + (prerelease.isEmpty ? "" : "-" + prerelease.joined(separator: "."))
    }

    static func < (a: SemanticVersion, b: SemanticVersion) -> Bool {
        if a.major != b.major { return a.major < b.major }
        if a.minor != b.minor { return a.minor < b.minor }
        if a.patch != b.patch { return a.patch < b.patch }
        switch (a.prerelease.isEmpty, b.prerelease.isEmpty) {
        case (true, true): return false
        case (false, true): return true   // 1.0.0-x < 1.0.0
        case (true, false): return false
        case (false, false): break
        }
        for (x, y) in zip(a.prerelease, b.prerelease) where x != y {
            switch (Int(x), Int(y)) {
            case let (nx?, ny?): return nx < ny
            case (.some, .none): return true   // numeric identifiers are lower than alphanumeric
            case (.none, .some): return false
            case (.none, .none): return x < y
            }
        }
        return a.prerelease.count < b.prerelease.count
    }
}
