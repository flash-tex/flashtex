import Foundation

/// Where the app's bundled resources (`supported-latex.json`, `Fonts/`) can be
/// found, across every way this code is run.
///
/// There are two shapes to cope with, and the difference is what broke the
/// Mac test suite:
///
/// * the **packaged app**, where `make-app.sh` copies the resources straight
///   into `FlashTeXMac.app/Contents/Resources`; and
/// * the **SwiftPM resource bundle** `FlashTeXMac_FlashTeXMac.bundle`, built
///   for `swift build` / `swift run` / `swift test`. On macOS that bundle is
///   *structured* — the files live in `…​.bundle/Contents/Resources/` — and
///   with Swift 6.x it is also copied inside the `.xctest` bundle, at
///   `FlashTeXMacTests.xctest/Contents/Resources/FlashTeXMac_FlashTeXMac.bundle`.
///   Older layouts put the payload flat inside the `.bundle` directory, which
///   is still what SwiftPM does on Linux.
///
/// Resolving the inner bundle through `Bundle(url:)` covers both shapes;
/// the flat path is kept as a fallback. Looked up by hand rather than through
/// `Bundle.module`, whose generated accessor *traps* when the resource bundle
/// is absent — an unreadable resource must degrade, not abort the process.
enum BundledResources {
    private static let resourceBundleName = "FlashTeXMac_FlashTeXMac.bundle"

    /// Candidate resource directories, most specific first. Append the
    /// resource's own name to each and take the first that exists.
    static func directories(module: Bundle) -> [URL] {
        var out: [URL] = []
        func add(_ url: URL?) {
            guard let url, !out.contains(url) else { return }
            out.append(url)
        }

        // The packaged app, and the .xctest bundle's own Resources.
        add(module.resourceURL)
        add(Bundle.main.resourceURL)

        // The SwiftPM resource bundle, wherever it was copied to.
        var searched: [URL] = [module.bundleURL, module.bundleURL.deletingLastPathComponent()]
        if let resources = module.resourceURL { searched.append(resources) }
        searched.append(Bundle.main.bundleURL)
        if let resources = Bundle.main.resourceURL { searched.append(resources) }
        if let exe = Bundle.main.executableURL { searched.append(exe.deletingLastPathComponent()) }
        for directory in searched {
            let bundleURL = directory.appendingPathComponent(resourceBundleName)
            add(Bundle(url: bundleURL)?.resourceURL) // structured (macOS): Contents/Resources
            add(bundleURL)                           // flat (Linux, older SwiftPM)
        }
        return out
    }

    /// The first candidate location of `name` that exists on disk, plus the
    /// paths searched, so a miss can say where it looked.
    static func url(forResource name: String, module: Bundle) -> (url: URL?, searched: [URL]) {
        let searched = directories(module: module).map { $0.appendingPathComponent(name) }
        return (searched.first { FileManager.default.fileExists(atPath: $0.path) }, searched)
    }
}
