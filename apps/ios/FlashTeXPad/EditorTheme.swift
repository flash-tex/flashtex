import FlashTeXEditorCore
import UIKit

/// Colours and font of the iPad source editor. The values are the Mac's
/// `SyntaxTheme` (apps/mac/Sources/FlashTeXMac/SyntaxHighlighter.swift) — the
/// JetBrains code vocabulary, light and dark — as dynamic `UIColor`s, so the
/// two editors colour a document identically and an appearance change needs
/// no repaint. The iPad app has no design-token file of its own yet; this is
/// the editor's, kept small on purpose.
struct EditorTheme {
    let font: UIFont
    let text: UIColor
    let matchBackground: UIColor
    let colors: [SyntaxHighlighter.Kind: UIColor]

    static func dynamic(light: (CGFloat, CGFloat, CGFloat), dark: (CGFloat, CGFloat, CGFloat)) -> UIColor {
        UIColor { traits in
            let c = traits.userInterfaceStyle == .dark ? dark : light
            return UIColor(red: c.0 / 255, green: c.1 / 255, blue: c.2 / 255, alpha: 1)
        }
    }

    static let standard = EditorTheme(
        font: .monospacedSystemFont(ofSize: 15, weight: .regular),
        text: .label,
        matchBackground: UIColor.tintColor.withAlphaComponent(0.22),
        colors: [
            .command: dynamic(light: (0, 51, 179), dark: (207, 142, 109)),
            .mathCommand: dynamic(light: (0, 98, 122), dark: (42, 172, 184)),
            .environment: dynamic(light: (0, 98, 122), dark: (86, 168, 245)),
            .math: dynamic(light: (135, 16, 148), dark: (199, 125, 187)),
            .mathDelimiter: dynamic(light: (135, 16, 148), dark: (199, 125, 187)),
            .number: dynamic(light: (23, 80, 235), dark: (42, 172, 184)),
            .comment: dynamic(light: (140, 140, 140), dark: (122, 126, 133)),
            .brace: .secondaryLabel,
            .bracket: .secondaryLabel,
            .reference: dynamic(light: (6, 125, 23), dark: (106, 171, 115)),
            .file: dynamic(light: (6, 125, 23), dark: (106, 171, 115)),
            .definition: dynamic(light: (158, 136, 13), dark: (179, 174, 96)),
            // verbatim: plain text, deliberately uncoloured
        ]
    )

    /// Colour for a run kind, or nil for the plain text colour.
    func color(for kind: SyntaxHighlighter.Kind) -> UIColor? { colors[kind] }

    var baseAttributes: [NSAttributedString.Key: Any] { [.font: font, .foregroundColor: text] }
}
