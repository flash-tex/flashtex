import Darwin
import Foundation

/// Project trust (owner decision 9A). A project that came from elsewhere
/// (its main file or its folder carries the `com.apple.quarantine` extended
/// attribute: downloaded, unpacked from a downloaded archive, received) and
/// that the user has not trusted compiles with shell escape OFF (`\write18`
/// does nothing) and the preview shows "Trust this project?". Once trusted —
/// a per-project record, persisted — it compiles with restricted `\write18`,
/// as pdflatex does by default (texmf.cnf's `shell_escape = p`). A project
/// made on this Mac (no quarantine) is trusted.
enum EngineV3Trust {
    static let recordKey = "FlashTeX.EngineV3.trustedProjects"
    static let quarantineAttribute = "com.apple.quarantine"

    /// The shell-escape mode a COMPILE for this project sends.
    static func shellEscape(trusted: Bool) -> String { trusted ? "restricted" : "off" }

    static func isQuarantined(_ url: URL) -> Bool {
        getxattr(url.path, quarantineAttribute, nil, 0, 0, 0) >= 0
    }

    /// The record's identity for a project: its folder's canonical path.
    static func key(_ root: URL) -> String { root.standardizedFileURL.resolvingSymlinksInPath().path }

    static func isRecorded(_ root: URL, defaults: UserDefaults = .standard) -> Bool {
        (defaults.stringArray(forKey: recordKey) ?? []).contains(key(root))
    }

    static func record(_ root: URL, defaults: UserDefaults = .standard) {
        var list = defaults.stringArray(forKey: recordKey) ?? []
        if !list.contains(key(root)) { list.append(key(root)); defaults.set(list, forKey: recordKey) }
    }

    /// Whether a project (its folder and main file) may run restricted `\write18`.
    /// An untitled buffer (no folder) has nothing to run: trusted.
    static func isTrusted(root: URL?, main: URL?, defaults: UserDefaults = .standard) -> Bool {
        guard let root else { return true }
        if isRecorded(root, defaults: defaults) { return true }
        let quarantined = isQuarantined(root) || (main.map(isQuarantined) ?? false)
        return !quarantined
    }
}
