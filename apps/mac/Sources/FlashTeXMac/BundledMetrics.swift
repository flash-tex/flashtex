import Foundation

/// The rooted Latin Modern TeX metrics shipped in the app bundle (GH36), the
/// `jknappen/ec` T1 metrics `t1cmr.fd` loads (GH111 / PR #111) and the AMS
/// symbol metrics (`msbm`/`msam`, `\mathbb` advances), plus the producer
/// environment that connects them all to `flashtex-render`.
///
/// The render pipeline resolves TFMs from `FLASHTEX_TFM_DIRS` (colon
/// separated) before any inferred directory, and derives the `texmf` root of
/// its digest-bound required 12 pt set by stripping `/fonts/tfm/public/lm`
/// from those entries (`<root>/doc/fonts/lm/GUST-FONT-LICENSE.TXT` must sit
/// beside them). `make-app.sh` stages the pinned and supplementary TFMs and
/// their licenses under `Contents/Resources/texmf/…` with that exact shape;
/// this type finds those directories and appends them to the child
/// producer's `FLASHTEX_TFM_DIRS` so the packaged app never depends on a
/// host TeX installation for text, roman-math, T1 `cmr` or (once a producer
/// reads them) `\mathbb` metrics. Policy (GH36 review): explicit user
/// entries come FIRST, in their order — a populated user directory overrides
/// the bundled metrics — and the bundled directories are the fallback after
/// them, in the order Latin Modern / EC / AMS symbols; every other variable
/// is passed through untouched. Both producer routes share it: the directly
/// attached worker (`WorkerClient`) and the helper-spawned producer
/// (`PreviewControllerClient` → `flashtex-preview-controller` → compiler
/// child, which inherits the helper's environment).
///
/// Note: inside a real `.app` bundle the render pipeline's own
/// `Discovery::bundle_texmf_roots` (`fonts.rs`) already derives
/// `Contents/Resources/texmf` from the running executable's path and probes
/// `fonts/tfm/jknappen/ec` under it automatically — so a packaged
/// `flashtex-render` run directly (no `FLASHTEX_TFM_DIRS` at all) finds the
/// EC metrics without this type's help. `producerEnvironment` still
/// advertises the EC (and AMS symbols) directories explicitly so the same
/// environment works for a bare, non-bundled producer binary (as the real
/// producer tests below and CI use) and so `FLASHTEX_TFM_DIRS` always
/// reflects every metric directory the app ships, not only Latin Modern's.
enum BundledMetrics {
    /// The producer's TFM search-path variable (colon separated).
    static let environmentKey = "FLASHTEX_TFM_DIRS"

    /// Where the pinned Latin Modern metrics live below a `texmf` root; the
    /// suffix the producer strips to find
    /// `doc/fonts/lm/GUST-FONT-LICENSE.TXT`.
    static let tfmSubdirectory = "fonts/tfm/public/lm"

    /// Where the EC metrics (`ecrm`/`ecbx`/`ecti`/`ecbi`/`ecsl`) that
    /// `t1cmr.fd` loads for `[T1]{fontenc}` documents without `lmodern`
    /// live below a `texmf` root (`fonts.rs::EC_TFM_DIR`).
    static let ecTfmSubdirectory = "fonts/tfm/jknappen/ec"

    /// Where the AMS symbol metrics (`msbm`/`msam`, `\mathbb` advances)
    /// live below a `texmf` root.
    static let amsSymbolsSubdirectory = "fonts/tfm/public/amsfonts/symbols"

    /// Where Knuth's OT1 metrics (`cmr`/`cmbx`/`cmti`/`cmsl`/`cmss`...) that
    /// `ot1cmr.fd`/`ot1cmss.fd` load for documents without `fontenc` live
    /// below a `texmf` root (`fonts.rs::CM_TFM_DIR`).
    static let cmTfmSubdirectory = "fonts/tfm/public/cm"

    /// The `texmf` roots probed, in order: the app bundle's
    /// `Contents/Resources/texmf`, then the repository's vendored
    /// `apps/mac/Fonts/texmf` (development builds and tests).
    static var candidateRoots: [URL] {
        var roots: [URL] = []
        if let resources = Bundle.main.resourceURL {
            roots.append(resources.appendingPathComponent("texmf"))
        }
        let repositoryFonts = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Fonts").appendingPathComponent("texmf")
        roots.append(repositoryFonts)
        return roots
    }

    /// The first candidate root that actually carries `<root>/<subdirectory>`
    /// as a directory, or nil when this executable ships without it (a bare
    /// `swift build` product outside the repository, or a metrics set that
    /// was never vendored).
    static func directory(_ subdirectory: String, roots: [URL] = candidateRoots) -> URL? {
        let fm = FileManager.default
        for root in roots {
            let dir = root.appendingPathComponent(subdirectory)
            var isDir: ObjCBool = false
            if fm.fileExists(atPath: dir.path, isDirectory: &isDir), isDir.boolValue {
                return dir.standardizedFileURL
            }
        }
        return nil
    }

    /// The first candidate root that actually carries the rooted directory
    /// (`<root>/fonts/tfm/public/lm` exists and is a directory), or nil when
    /// this executable ships without the metrics (a bare `swift build`
    /// product outside the repository).
    static func tfmDirectory(roots: [URL] = candidateRoots) -> URL? {
        directory(tfmSubdirectory, roots: roots)
    }

    /// The first candidate root carrying the EC metrics directory, or nil.
    static func ecTfmDirectory(roots: [URL] = candidateRoots) -> URL? {
        directory(ecTfmSubdirectory, roots: roots)
    }

    /// The first candidate root carrying the AMS symbol metrics directory, or nil.
    static func amsSymbolsDirectory(roots: [URL] = candidateRoots) -> URL? {
        directory(amsSymbolsSubdirectory, roots: roots)
    }

    /// The first candidate root carrying the OT1 Computer Modern metrics directory, or nil.
    static func cmTfmDirectory(roots: [URL] = candidateRoots) -> URL? {
        directory(cmTfmSubdirectory, roots: roots)
    }

    /// `existing` (`FLASHTEX_TFM_DIRS` as the user set it, possibly nil or
    /// empty) with `bundled` appended: every non-empty explicit entry first,
    /// in its original order, then the bundled path (repeats of it dropped).
    /// Explicit entries therefore override the bundle for any metric they
    /// carry, and the bundle answers for everything they do not.
    static func merged(bundled: String, withExplicit existing: String?) -> String {
        var entries: [String] = []
        for entry in (existing ?? "").split(separator: ":", omittingEmptySubsequences: true) {
            let s = String(entry)
            if s != bundled { entries.append(s) }
        }
        entries.append(bundled)
        return entries.joined(separator: ":")
    }

    /// Every bundled metrics directory this app ships, in the order
    /// `producerEnvironment` appends them: Latin Modern, then EC, then AMS
    /// symbols, then OT1 Computer Modern. A directory that was never
    /// vendored (or is missing from a bare `swift build` product) is
    /// omitted rather than added empty.
    static func defaultBundledDirectories(roots: [URL] = candidateRoots) -> [URL] {
        [tfmDirectory(roots: roots), ecTfmDirectory(roots: roots), amsSymbolsDirectory(roots: roots), cmTfmDirectory(roots: roots)].compactMap { $0 }
    }

    /// The environment to launch a producer (or the helper that spawns one)
    /// with: `base` unchanged except `FLASHTEX_TFM_DIRS`, which gets every
    /// directory in `bundledDirectories` appended, in order, after the
    /// explicit entries (each via `merged`, so a later bundled directory
    /// never displaces an earlier one or a user entry). Returns `base`
    /// untouched when `bundledDirectories` is empty, so a bare build
    /// behaves exactly as before.
    static func producerEnvironment(base: [String: String] = ProcessInfo.processInfo.environment,
                                    bundledDirectories: [URL] = defaultBundledDirectories()) -> [String: String] {
        guard !bundledDirectories.isEmpty else { return base }
        var env = base
        var current = base[environmentKey]
        for directory in bundledDirectories {
            current = merged(bundled: directory.path, withExplicit: current)
        }
        env[environmentKey] = current
        return env
    }
}
