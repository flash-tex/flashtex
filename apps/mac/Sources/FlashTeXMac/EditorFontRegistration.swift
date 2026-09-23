import AppKit
import CoreText

/// The bundled UI-side editor face (context/PROMPT-appearance-overhaul.md §5):
/// JetBrains Mono 13 at 1.2 line spacing, falling back to the system
/// monospaced face (SF Mono) when the bundle is unavailable. This is the
/// *editor chrome* face only — preview/export faces (Latin Modern) are
/// `PreviewFonts` and are not touched here.
///
/// JetBrains Mono is SIL OFL 1.1 with no Reserved Font Name; the licence is
/// bundled beside the files (`Resources/Fonts/OFL.txt`).
enum EditorFontRegistration {
    /// PostScript name of the regular face, the one `resolveFont` asks for.
    static let regularPostScriptName = "JetBrainsMono-Regular"
    /// Family name, as shown in the Settings font picker.
    static let familyName = "JetBrains Mono"

    private static let faces = [
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Bold.ttf",
        "JetBrainsMono-Italic.ttf",
        "JetBrainsMono-BoldItalic.ttf",
    ]

    /// Registers the bundled faces for this process (never `.persistent`) the
    /// first time anything asks; safe to call from any font-resolution path.
    /// Returns whether the regular face is usable after registration.
    @discardableResult
    static func registerIfNeeded() -> Bool { registered }

    private static let registered: Bool = {
        // Already available (installed by the user, or registered by an
        // earlier bundle in this process): nothing to do.
        if NSFont(name: regularPostScriptName, size: 13) != nil { return true }
        guard let directory = fontsDirectory() else { return false }
        let urls = faces.map { directory.appendingPathComponent($0) }
            .filter { FileManager.default.fileExists(atPath: $0.path) }
        guard !urls.isEmpty else { return false }
        CTFontManagerRegisterFontsForURLs(urls as CFArray, .process, nil)
        return NSFont(name: regularPostScriptName, size: 13) != nil
    }()

    /// The bundled `Fonts` directory, resolved the same way (and for the same
    /// reason — `Bundle.module` traps when absent) as the completion
    /// inventory: the packaged app's `Contents/Resources/Fonts`, then the
    /// SwiftPM resource bundle in either of its layouts. See
    /// `BundledResources`.
    private static func fontsDirectory() -> URL? {
        BundledResources.directories(module: Bundle(for: EditorFontBundleMarker.self))
            .map { $0.appendingPathComponent("Fonts") }
            .first { FileManager.default.fileExists(atPath: $0.appendingPathComponent(faces[0]).path) }
    }
}

private final class EditorFontBundleMarker {}
