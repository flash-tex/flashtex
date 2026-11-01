import AppKit

/// FlashTeX > About FlashTeX: the standard About panel plus the GPL source
/// offer (#1806, RQ19 ruling in DESIGN.md §13). The engine host
/// (`flashtex-host`) is GPL-2.0-or-later; its source is public in this
/// repository, and the credits link it so the offer travels with the app.
enum AboutPanel {
    static let sourceURL = URL(string: "https://github.com/flash-tex/flashtex")!

    /// The line the release notes repeat (`.github/workflows/release.yml`).
    static let sourceLine = "Source code for the FlashTeX engine (GPL-2.0-or-later): https://github.com/flash-tex/flashtex"

    /// The credits shown under the version: the source line, with the URL a clickable link.
    static func credits() -> NSAttributedString {
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center
        let text = NSMutableAttributedString(string: sourceLine, attributes: [
            .font: NSFont.systemFont(ofSize: NSFont.smallSystemFontSize),
            .foregroundColor: NSColor.labelColor,
            .paragraphStyle: paragraph,
        ])
        let range = (sourceLine as NSString).range(of: sourceURL.absoluteString)
        text.addAttribute(.link, value: sourceURL, range: range)
        return text
    }

    @MainActor static func show() {
        NSApp.orderFrontStandardAboutPanel(options: [.credits: credits()])
        NSApp.activate(ignoringOtherApps: true)
    }
}
