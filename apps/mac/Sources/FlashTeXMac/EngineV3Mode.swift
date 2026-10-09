import Foundation

/// Which engine the new preview runs for a document (docs/design/modes/PROPOSAL.md
/// §4.1, owner Q10: one host program per mode). Classic is pdfLaTeX-compatible
/// (`flashtex-host`, the `pdflatex` format); Unicode is XeLaTeX-compatible
/// (`flashtex-host-unicode`, crates/flashtex-xetex, the `xelatex` format, or
/// plain XeTeX's `xetex` for `% !TEX program = xetex`).
///
/// Resolution, highest first: `FLASHTEX_MODE`; the manifest's `[project] mode`
/// (as the project-files helper reports it: the app never reads TOML); a
/// `% !TEX program = xelatex` (or `TS-program`) line among the main file's
/// leading comments, the TeXShop/TeXstudio/VS Code convention; Classic. The
/// opt-in FlashTeX mode (PROPOSAL.md §10) does not exist yet: it typesets as
/// Classic, and the warning says so. So does a program FlashTeX does not have
/// (LuaLaTeX). The status-bar item, detection and the manifest writes are
/// MODES-UX's (M5).
enum EngineV3Mode: String, Equatable, Sendable {
    case classic, unicode

    /// The format a compile asks for by default.
    var format: String { self == .unicode ? "xelatex" : "pdflatex" }

    /// The host program's file name.
    var hostName: String { self == .unicode ? "flashtex-host-unicode" : "flashtex-host" }

    /// What the window says while the host prepares its format.
    var formatName: String { self == .unicode ? "XeLaTeX" : "pdfLaTeX" }

    /// What a `% !TEX program` line says.
    enum Program: Equatable, Sendable {
        /// A program FlashTeX has: its mode and format.
        case known(EngineV3Mode, format: String)
        /// One it does not have (`lualatex`, anything else), as written.
        case unsupported(String)
    }

    /// `% !TEX program = NAME` (or `TS-program`; `!TeX`, `! TeX`, either
    /// case) among the leading comment lines of `text`, after a byte-order
    /// mark: the program it names, if a line names one.
    static func magicComment(_ text: String) -> Program? {
        // only the head: this runs on every compile
        var head = Substring(text.prefix(4096))
        if head.first == "\u{feff}" { head = head.dropFirst() }
        for line in head.split(omittingEmptySubsequences: false, whereSeparator: \.isNewline).prefix(20) {
            let l = line.drop(while: { $0 == " " || $0 == "\t" })
            if l.isEmpty { continue }
            guard l.hasPrefix("%") else { break }
            let body = l.drop(while: { $0 == "%" }).drop(while: { $0 == " " || $0 == "\t" }).lowercased()
            guard body.hasPrefix("!") else { continue }
            let afterBang = body.dropFirst().drop(while: { $0 == " " || $0 == "\t" })
            guard afterBang.hasPrefix("tex"), let eq = afterBang.firstIndex(of: "=") else { continue }
            let key = afterBang[afterBang.index(afterBang.startIndex, offsetBy: 3) ..< eq].trimmingCharacters(in: .whitespaces)
            guard key == "program" || key == "ts-program" else { continue }
            let value = afterBang[afterBang.index(after: eq)...].trimmingCharacters(in: .whitespacesAndNewlines)
            switch value {
            case "xelatex": return .known(.unicode, format: "xelatex")
            case "xetex": return .known(.unicode, format: "xetex")
            case "pdflatex", "latex": return .known(.classic, format: "pdflatex")
            default: return .unsupported(value)
            }
        }
        return nil
    }

    /// A document's mode: the mode, its format, what chose it, and a
    /// warning when what was asked for could not be followed.
    struct Resolution: Equatable, Sendable {
        var mode: EngineV3Mode
        var format: String
        var source: String
        var warning: String?
    }

    /// `manifestWarning`: the helper's warning about `[project] mode`, when
    /// the manifest has a value this version does not know (`manifest` is
    /// then nil).
    static func resolve(environment: String?, manifest: String?, manifestWarning: String? = nil, mainText: String?) -> Resolution {
        func of(_ m: EngineV3Mode, _ source: String, _ warning: String? = nil) -> Resolution {
            Resolution(mode: m, format: m.format, source: source, warning: warning)
        }
        if let e = environment, !e.isEmpty {
            if let m = EngineV3Mode(rawValue: e) { return of(m, "FLASHTEX_MODE") }
            return of(.classic, "FLASHTEX_MODE", "FLASHTEX_MODE=\(e) is not a mode this version has; using Classic (pdfTeX)")
        }
        if let m = manifest {
            if let mode = EngineV3Mode(rawValue: m) { return of(mode, "flashtex.toml") }
            // the manifest meant to choose: its choice is not overruled by
            // a `% !TEX` line
            return of(.classic, "flashtex.toml", "flashtex.toml: mode = \"\(m)\" is not available in this version; using Classic (pdfTeX)")
        }
        if let w = manifestWarning {
            return of(.classic, "flashtex.toml", "flashtex.toml: \(w); using Classic (pdfTeX)")
        }
        switch mainText.flatMap(magicComment) {
        case .known(let m, let format)?:
            var r = of(m, "% !TEX program")
            r.format = format
            return r
        case .unsupported(let p)?:
            return of(.classic, "default", p.hasPrefix("lua")
                ? "`% !TEX program = \(p)`: LuaTeX isn't supported; using Classic (pdfTeX). For OpenType fonts use Unicode mode (`% !TEX program = xelatex`)."
                : "`% !TEX program = \(p)` is not a program FlashTeX has; using Classic (pdfTeX).")
        case nil:
            return of(.classic, "default")
        }
    }
}
