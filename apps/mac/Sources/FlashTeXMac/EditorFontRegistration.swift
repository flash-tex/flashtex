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

    /// Candidate locations of the bundled `Fonts` directory, in the same
    /// order (and for the same reason — `Bundle.module` traps when absent) as
    /// `Completion.Vocabulary.inventoryCandidates()`: the packaged app's
    /// `Contents/Resources/Fonts`, then the SwiftPM resource bundle beside
    /// the executable or the test bundle.
    private static func fontsDirectory() -> URL? {
        let module = Bundle(for: EditorFontBundleMarker.self)
        var candidates: [URL] = []
        for bundle in [module, Bundle.main] {
            if let url = bundle.resourceURL?.appendingPathComponent("Fonts") { candidates.append(url) }
        }
        let resourceBundle = "FlashTeXMac_FlashTeXMac.bundle"
        var directories = [module.bundleURL, module.bundleURL.deletingLastPathComponent(), Bundle.main.bundleURL]
        if let exe = Bundle.main.executableURL { directories.append(exe.deletingLastPathComponent()) }
        for directory in directories {
            candidates.append(directory.appendingPathComponent(resourceBundle).appendingPathComponent("Fonts"))
        }
        return candidates.first {
            FileManager.default.fileExists(atPath: $0.appendingPathComponent(faces[0]).path)
        }
    }
}

private final class EditorFontBundleMarker {}
