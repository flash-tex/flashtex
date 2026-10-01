import FlashTeXEditorCore
import UIKit

/// Colours and font of the iPad source editor. The syntax colours come from
/// the theme definitions shared with the Mac (`EditorColorTheme` in
/// FlashTeXEditorCore: the same built-ins, light and dark) as dynamic
/// `UIColor`s, so the two editors colour a document identically and an
/// appearance change needs no repaint. The iPad keeps the system text and
/// ground colours; only the token colours are themed.
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

    /// `role` of `theme` as a colour that follows the trait collection; nil
    /// when the theme leaves the role to the plain text colour in both appearances.
    static func dynamic(_ role: EditorColorTheme.Role, of theme: EditorColorTheme) -> UIColor? {
        let light = theme.color(role, .light), dark = theme.color(role, .dark)
        guard light != nil || dark != nil else { return nil }
        func ui(_ c: EditorColorTheme.Color?) -> UIColor {
            guard let v = c?.components else { return .label }
            return UIColor(red: v.red, green: v.green, blue: v.blue, alpha: v.alpha)
        }
        let l = ui(light), d = ui(dark)
        return UIColor { traits in traits.userInterfaceStyle == .dark ? d : l }
    }

    /// The editor theme for a shared colour theme (FlashTeX's by default).
    init(colorTheme: EditorColorTheme = .flashtex,
         font: UIFont = .monospacedSystemFont(ofSize: 15, weight: .regular),
         text: UIColor = .label,
         matchBackground: UIColor = UIColor.tintColor.withAlphaComponent(0.22)) {
        self.font = font
        self.text = text
        self.matchBackground = matchBackground
        var colors: [SyntaxHighlighter.Kind: UIColor] = [:]
        for kind in SyntaxHighlighter.Kind.allCases {
            colors[kind] = Self.dynamic(EditorColorTheme.Role(kind: kind), of: colorTheme) // verbatim: plain unless a theme colours it
        }
        self.colors = colors
    }

    init(font: UIFont, text: UIColor, matchBackground: UIColor, colors: [SyntaxHighlighter.Kind: UIColor]) {
        self.font = font; self.text = text; self.matchBackground = matchBackground; self.colors = colors
    }

    static let standard = EditorTheme()

    /// Colour for a run kind, or nil for the plain text colour.
    func color(for kind: SyntaxHighlighter.Kind) -> UIColor? { colors[kind] }

    var baseAttributes: [NSAttributedString.Key: Any] { [.font: font, .foregroundColor: text] }
}
