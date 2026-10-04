import Foundation
import SwiftUI

// The new engine without TeX Live (DESIGN.md §4.4, D12; lane NOTEX-WIRING):
// where no TeX Live is installed, `flashtex-host` reads a content-addressed
// bundle of unmodified TeX Live files, fetched on first use and pinned by
// its SHA-256 digest (crates/flashtex-engine/src/bundle/). Which bundle is
// configuration, not code -- where bundles are hosted is still the owner's
// decision, so there is no built-in default:
//
//   1. `FLASHTEX_BUNDLE_URL` and `FLASHTEX_BUNDLE_DIGEST` (developers, CI);
//   2. a `flashtex-bundle.lock` (`url = "…"`, `digest = "…"`): the file
//      `FLASHTEX_BUNDLE_LOCK` names, else the user's own in
//      ~/Library/Application Support/FlashTeX/, else the one shipped beside
//      the host or in the app's Contents/Resources/engine/.
//
// The same order as the host's `bundle::lock_candidates` (the app passes the
// lock it found to the host as FLASHTEX_BUNDLE_LOCK, so both read one file).
// Nothing is downloaded before the user agrees once (`consent`): until
// then the host runs with FLASHTEX_BUNDLE_OFFLINE=1. A bundle configured in
// the environment is the developer's own choice and needs no consent.

enum EngineV3Bundle {
    struct Config: Equatable, Sendable {
        var url: String
        var digest: String
        /// "environment", or the lock file's path.
        var origin: String
        var fromEnvironment: Bool { origin == "environment" }
        /// The host part of `url` ("example.org"), or "this Mac" for a file.
        var sourceLabel: String {
            if url.hasPrefix("file://") || url.hasPrefix("/") { return "this Mac" }
            return URL(string: url)?.host ?? url
        }
    }

    static let lockFileName = "flashtex-bundle.lock"
    static let consentKey = "FlashTeX.EngineV3.bundleConsent"

    /// The lock files looked for, first first (see the top of this file).
    static func lockCandidates(environment env: [String: String], host: URL? = nil) -> [String] {
        if let p = env["FLASHTEX_BUNDLE_LOCK"], !p.isEmpty { return [p] }
        var c: [String] = []
        if let home = env["HOME"], !home.isEmpty {
            c.append("\(home)/Library/Application Support/FlashTeX/\(lockFileName)")
        }
        if let host {
            let dir = host.deletingLastPathComponent()
            c.append(dir.appendingPathComponent(lockFileName).path)
            c.append(dir.appendingPathComponent("../Resources/engine/\(lockFileName)").standardizedFileURL.path)
        }
        if let r = Bundle.main.resourceURL?.appendingPathComponent("engine/\(lockFileName)").path, !c.contains(r) { c.append(r) }
        return c
    }

    /// The lock file's `url` and `digest` (the host's `bundle::parse_lock`):
    /// `key = value` lines, values optionally quoted, `#` comments, unknown
    /// keys ignored; a relative `url` is relative to the lock's directory.
    static func parseLock(_ text: String, directory: String) -> (url: String, digest: String)? {
        var url: String?, digest: String?
        for raw in text.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = raw.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }
            guard let eq = line.firstIndex(of: "=") else { return nil }
            let key = line[..<eq].trimmingCharacters(in: .whitespaces)
            var value = line[line.index(after: eq)...].trimmingCharacters(in: .whitespaces)
            if value.hasPrefix("\"") {
                guard value.count >= 2, value.hasSuffix("\"") else { return nil }
                value = String(value.dropFirst().dropLast())
            } else {
                value = (value.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false).first.map(String.init) ?? "")
                    .trimmingCharacters(in: .whitespaces)
            }
            if key == "url" { url = value } else if key == "digest" { digest = value.lowercased() }
        }
        guard let digest, digest.count == 64, digest.allSatisfy(\.isHexDigit), let url, !url.isEmpty else { return nil }
        if url.contains("://") || url.hasPrefix("/") { return (url, digest) }
        return ((directory as NSString).appendingPathComponent(url), digest)
    }

    /// The configured bundle, or nil (an unreadable lock counts as none here;
    /// the host reports its error in HELLO).
    static func configured(environment env: [String: String] = ProcessInfo.processInfo.environment,
                           host: URL? = EngineV3.locateHost()) -> Config? {
        if let d = env["FLASHTEX_BUNDLE_DIGEST"], !d.isEmpty {
            return Config(url: env["FLASHTEX_BUNDLE_URL"] ?? "", digest: d.lowercased(), origin: "environment")
        }
        let fm = FileManager.default
        for path in lockCandidates(environment: env, host: host) {
            var isDir: ObjCBool = false
            guard fm.fileExists(atPath: path, isDirectory: &isDir), !isDir.boolValue else { continue }
            guard let text = try? String(contentsOfFile: path, encoding: .utf8),
                  let (url, digest) = parseLock(text, directory: (path as NSString).deletingLastPathComponent) else { return nil }
            return Config(url: url, digest: digest, origin: path)
        }
        return nil
    }

    /// The host's bundle cache (`bundle::default_cache_dir`).
    static func cacheDirectory(environment env: [String: String]) -> String? {
        if let d = env["FLASHTEX_BUNDLE_CACHE_DIR"], !d.isEmpty { return d }
        guard let home = env["HOME"], !home.isEmpty else { return nil }
        return "\(home)/Library/Caches/FlashTeX/bundles"
    }

    /// The bundle's index is in the cache: it was fetched before, and opening
    /// it again downloads nothing up front.
    static func isCached(_ c: Config, environment env: [String: String]) -> Bool {
        guard let dir = cacheDirectory(environment: env) else { return false }
        return FileManager.default.fileExists(atPath: "\(dir)/\(c.digest)/index.gz")
    }

    /// The user's answer: true (download), false (not now), nil (not asked).
    static var consent: Bool? {
        get { EngineV3.defaults.object(forKey: consentKey) as? Bool }
        set {
            if let newValue { EngineV3.defaults.set(newValue, forKey: consentKey) } else { EngineV3.defaults.removeObject(forKey: consentKey) }
        }
    }

    /// What the new engine needs from the user before it can start (pure).
    enum Gate: Equatable {
        /// TeX Live is installed, or no bundle is configured, or it may be used.
        case none
        /// Ask first: the bundle would be downloaded.
        case ask(Config)
        /// The user said not now: the previous engine typesets.
        case declined
    }

    static func gate(texLiveInstalled: Bool, config: Config?, cached: Bool, consent: Bool?) -> Gate {
        guard !texLiveInstalled, let config, !cached, !config.fromEnvironment else { return .none }
        switch consent {
        case true?: return .none
        case false?: return .declined
        case nil: return .ask(config)
        }
    }

    static func currentGate(environment env: [String: String] = ProcessInfo.processInfo.environment) -> Gate {
        let c = configured(environment: env)
        return gate(texLiveInstalled: EngineChoice.texLiveInstalled(environment: env), config: c,
                    cached: c.map { isCached($0, environment: env) } ?? false, consent: consent)
    }

    /// The host's environment for the bundle: the lock the app found (so
    /// both read one file), and offline until the user agreed.
    static func hostEnvironment(_ env: inout [String: String], host: URL) {
        guard let c = configured(environment: env, host: host) else { return }
        if !c.fromEnvironment, (env["FLASHTEX_BUNDLE_LOCK"] ?? "").isEmpty { env["FLASHTEX_BUNDLE_LOCK"] = c.origin }
        if !c.fromEnvironment, consent != true, !isCached(c, environment: env) { env["FLASHTEX_BUNDLE_OFFLINE"] = "1" }
    }

    /// The status bar's line for a `bundle_progress` report, nil when done.
    static func progressText(what: String, name: String, done: Int64, total: Int64) -> String? {
        guard done < total || total == 0 else { return nil }
        func mb(_ b: Int64) -> String { String(format: "%.1f", Double(b) / 1_048_576) }
        switch what {
        case "index": return "downloading the TeX files' index…"
        case "core": return "downloading TeX files: \(mb(done)) of \(mb(total)) MB (\(total > 0 ? done * 100 / total : 0)%)"
        default: return "downloading \(name)…"
        }
    }
}

// MARK: - the consent sheet

/// Before the first download: what would be fetched, from where, and the
/// two answers. In the style of the package consent sheet (#1444,
/// ProjectPackagesSheet): nothing is fetched until the user says so.
struct EngineV3BundleSheet: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let config = model.engineV3.bundleConsentConfig
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Download TeX Files").font(.title2.bold())
            Text("No TeX Live is installed on this Mac. The new engine can typeset with a pinned set of unmodified TeX Live files instead, downloaded once into the cache and checked against their published SHA-256 digests. Nothing is downloaded until you say so; the files then serve every document.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            if let config {
                HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                    Image(systemName: "shippingbox").foregroundStyle(DS.Colors.textSecondary).accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: DS.Space.xxs) {
                        Text("TeX Live files from \(config.sourceLabel)").font(DS.Fonts.base)
                        Text(config.url).font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1).truncationMode(.middle)
                        Text("SHA-256 \(String(config.digest.prefix(16)))…").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary)
                    }
                }
                .padding(DS.Space.m)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(DS.Colors.surfaceRaised, in: RoundedRectangle(cornerRadius: DS.Radius.panel))
                .overlay(RoundedRectangle(cornerRadius: DS.Radius.panel).stroke(DS.Colors.componentBorder))
                .accessibilityElement(children: .combine)
                .accessibilityLabel("TeX Live files from \(config.sourceLabel), \(config.url)")
            }
            Text("Installing MacTeX instead makes the download unnecessary.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            HStack {
                Spacer()
                Button("Not Now") { model.engineV3.answerBundleConsent(false) }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityHint("Downloads nothing; this document is typeset with the previous engine.")
                    .accessibilityIdentifier("engine-v3.bundle.not-now")
                Button("Download") { model.engineV3.answerBundleConsent(true) }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityHint("Downloads the TeX files, then starts the new engine.")
                    .accessibilityIdentifier("engine-v3.bundle.download")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
    }
}
