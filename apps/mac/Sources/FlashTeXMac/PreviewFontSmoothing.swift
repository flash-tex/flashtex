import SwiftUI

/// Settings > Editor > Preview: "Smooth fonts in preview" (owner decision,
/// 2026-09-30: user-togglable, default off).
///
/// Off draws preview glyphs without Core Graphics font smoothing: the
/// configuration the zero-tolerance pixel parity with the exported PDF is
/// measured in. On draws them the way Preview.app draws that PDF; ligatures
/// such as fi/ffi then differ from the parity reference by about 11–13 px a
/// page.
///
/// UserDefaults is read at launch and written by the toggle; the value then
/// reaches the rasterizers as a plain Bool (`V2PageRasterizer.smoothFonts`,
/// `EngineV3Session.smoothFonts`), so nothing on a drawing path reads defaults.
enum PreviewFontSmoothing {
    static let key = "FlashTeX.Preview.fontSmoothing"
    /// Off: exact parity with the exported PDF.
    static let defaultValue = false
    /// Posted on the main thread after the preference changes; `userInfo["on"]` is the new value.
    static let changed = Notification.Name("FlashTeX.Preview.fontSmoothing.changed")

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

    /// Calls `body` with each new value (on the posting thread: main). Keep the token.
    static func observe(_ body: @escaping @MainActor (Bool) -> Void) -> NSObjectProtocol {
        NotificationCenter.default.addObserver(forName: changed, object: nil, queue: nil) { note in
            guard let on = note.userInfo?["on"] as? Bool else { return }
            MainActor.assumeIsolated { body(on) }
        }
    }

    /// One sentence for Settings.
    static let footnote = "Off matches the exported PDF pixel for pixel; on draws text the way Preview.app does, so ligatures such as “fi” differ slightly from the PDF."
}

/// The Settings rows (Settings > Editor > Preview).
struct PreviewFontSmoothingRows: View {
    @State private var on = PreviewFontSmoothing.enabled

    var body: some View {
        Toggle("Smooth fonts in preview", isOn: $on)
            .onChange(of: on) { _, v in PreviewFontSmoothing.enabled = v }
            .accessibilityHint(PreviewFontSmoothing.footnote)
        Text(PreviewFontSmoothing.footnote)
            .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
    }
}
