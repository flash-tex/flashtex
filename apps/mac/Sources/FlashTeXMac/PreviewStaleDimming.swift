import Foundation
import SwiftUI

/// Settings > Editor > Preview: "Dim pages that are being updated"
/// (owner request, 2026-10-09: the dim-and-orange-border stale marker
/// flashed during normal typing; it may be disabled, and when on it appears
/// only once a page has stayed stale for `delay`).
///
/// Only the look changes: `EngineV3PageView.axStale`, the session's stale set
/// and the status bar's "N stale" count report the true state at once.
enum PreviewStaleDimming {
    static let key = "FlashTeX.Preview.dimStalePages"
    /// On: a page that stays stale past `delay` is dimmed.
    static let defaultValue = true
    /// How long a page must stay stale before it is dimmed. New rasters
    /// normally arrive within tens of milliseconds while typing.
    static let delay: TimeInterval = 0.4
    /// Posted after the preference changes; `userInfo["on"]` is the new value.
    static let changed = Notification.Name("FlashTeX.Preview.dimStalePages.changed")

    static func isEnabled(in defaults: UserDefaults) -> Bool {
        defaults.object(forKey: key) as? Bool ?? defaultValue
    }

    /// The preference in `UserDefaults.standard`; setting it tells every open preview.
    static var enabled: Bool {
        get { isEnabled(in: .standard) }
        set {
            UserDefaults.standard.set(newValue, forKey: key)
            NotificationCenter.default.post(name: changed, object: nil, userInfo: ["on": newValue])
        }
    }

    /// Calls `body` with each new value, on the main queue whatever thread
    /// set the preference. Keep the token and remove it when done.
    static func observe(_ body: @escaping @MainActor (Bool) -> Void) -> NSObjectProtocol {
        NotificationCenter.default.addObserver(forName: changed, object: nil, queue: .main) { note in
            guard let on = note.userInfo?["on"] as? Bool else { return }
            MainActor.assumeIsolated { body(on) }
        }
    }

    /// One sentence for Settings.
    static let footnote = "Pages the next compile will change are dimmed with an orange border if they take more than a moment to redraw."
}

/// The Settings row (Settings > Editor > Preview).
struct PreviewStaleDimmingRows: View {
    @State private var on = PreviewStaleDimming.enabled

    var body: some View {
        Toggle("Dim pages that are being updated", isOn: $on)
            .onChange(of: on) { _, v in PreviewStaleDimming.enabled = v }
        Text(PreviewStaleDimming.footnote)
            .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
    }
}
