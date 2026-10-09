import Foundation

/// Which engine the new preview runs for a document (docs/design/modes/PROPOSAL.md
/// §4.1, owner Q10: one host program per mode). Classic is pdfLaTeX-compatible
/// (`flashtex-host`, the `pdflatex` format); Unicode is XeLaTeX-compatible
/// (`flashtex-host-unicode`, crates/flashtex-xetex, the `xelatex` format).
///
/// Resolution, highest first: `FLASHTEX_MODE`; the manifest's `[project] mode`
/// (as the project-files helper reports it: the app never reads TOML); a
/// `% !TEX program = xelatex` (or `TS-program`) line among the main file's
/// leading comments, the TeXShop/TeXstudio convention; Classic. The
/// opt-in FlashTeX mode (PROPOSAL.md §10) does not exist yet: it typesets as
/// Classic, and the log says so. The status-bar item, detection and the
/// manifest writes are MODES-UX's (M5).
enum EngineV3Mode: String, Equatable, Sendable {
    case classic, unicode

    /// The format a compile asks for.
    var format: String { self == .unicode ? "xelatex" : "pdflatex" }

    /// The host program's file name.
    var hostName: String { self == .unicode ? "flashtex-host-unicode" : "flashtex-host" }

    /// What the window says while the host prepares its format.
    var formatName: String { self == .unicode ? "XeLaTeX" : "pdfLaTeX" }

    /// `% !TEX program = NAME` (or `TS-program`, either case) among the
    /// leading comment lines of `text`: the mode it names, if any.
    static func magicComment(_ text: String) -> EngineV3Mode? {
        // only the head: this runs on every compile
        for line in text.prefix(4096).split(omittingEmptySubsequences: false, whereSeparator: \.isNewline).prefix(20) {
            let l = line.drop(while: { $0 == " " || $0 == "\t" })
            if l.isEmpty { continue }
            guard l.hasPrefix("%") else { break }
            let body = l.drop(while: { $0 == "%" }).drop(while: { $0 == " " || $0 == "\t" }).lowercased()
            guard body.hasPrefix("!tex"), let eq = body.firstIndex(of: "=") else { continue }
            let key = body[body.index(body.startIndex, offsetBy: 4) ..< eq].trimmingCharacters(in: .whitespaces)
            guard key == "program" || key == "ts-program" else { continue }
            switch body[body.index(after: eq)...].trimmingCharacters(in: .whitespacesAndNewlines) {
            case "xelatex", "xetex": return .unicode
            case "pdflatex", "pdftex", "latex": return .classic
            default: return nil
            }
        }
        return nil
    }

    /// The mode and what chose it.
    static func resolve(environment: String?, manifest: String?, mainText: String?) -> (mode: EngineV3Mode, source: String) {
        if let e = environment, !e.isEmpty {
            if let m = EngineV3Mode(rawValue: e) { return (m, "FLASHTEX_MODE") }
            return (.classic, "FLASHTEX_MODE=\(e) is not a mode this version has; Classic")
        }
        if let m = manifest {
            if let mode = EngineV3Mode(rawValue: m) { return (mode, "flashtex.toml") }
            return (.classic, "flashtex.toml mode = \"\(m)\" is not available in this version; Classic")
        }
        if let t = mainText, let m = magicComment(t) { return (m, "% !TEX program") }
        return (.classic, "default")
    }
}
