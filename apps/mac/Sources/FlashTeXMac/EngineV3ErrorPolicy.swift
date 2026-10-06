import Foundation
import FlashTeXDisplayListV3

/// How the engine-v3 preview presents TeX's errors (lane ERROR-RECOVERY; the
/// owner, 2026-10-04: "for recoverable errors it should still at least warn if
/// it would error on pdfTeX, but still compile/render a best-effort preview").
///
/// The engine runs exactly as `pdflatex -interaction=nonstopmode` does and its
/// output stays pdfTeX's; only the app's reading of the run changes:
///
/// * **Best effort (the default).** An error TeX recovers from (it goes on,
///   as nonstopmode does: an undefined command, a missing `$`, a stray `}`)
///   is shown as a **warning**, marked "pdfLaTeX would report an error here",
///   and the preview shows the pages that run made. A **fatal** error (TeX
///   stops: an emergency stop, a file that ended inside an argument, capacity
///   exceeded) stays an error, and so does its cause, the error TeX reported
///   just before it (``File `x.sty' not found``, "File ended while scanning
///   use of \textbf"): the pane keeps the pages the run made before it
///   stopped and the last good pages after them, stale.
/// * **Strict** (Settings > Compile, opt-in). TeX runs with `-halt-on-error`
///   and stops at the first error, as `pdflatex -halt-on-error` does; every
///   error is an error. The pages made before it stay current, the rest stale.
///
/// Pure: the session asks it about each compile's diagnostics.
enum EngineV3ErrorPolicy {
    enum Mode: String, Equatable { case bestEffort, strict }

    /// Settings > Compile > "Stop at the first error" (`ShellModel.strictTeXErrors`).
    static let strictKey = "FlashTeX.EngineV3.strictErrors"
    static var storedMode: Mode { EngineV3.defaults.bool(forKey: strictKey) ? .strict : .bestEffort }

    /// The mark on a recovered error's row.
    static let recoveredNote = "pdfLaTeX would report an error here"

    /// The messages TeX stops with (the host's `fatal` rule,
    /// crates/flashtex-engine/src/host/diag.rs), for hosts that send DIAGNOSTICs.
    static let fatalPrefixes = [
        "==> Fatal error occurred", "Emergency stop", "TeX capacity exceeded", "This can't happen",
        "I can't go on meeting you like this", "Interwoven alignment preambles are not allowed", "pdfTeX error",
    ]

    static func isFatal(message: String) -> Bool {
        let m = message.hasPrefix("!") ? String(message.drop(while: { $0 == "!" || $0 == " " })) : message
        return fatalPrefixes.contains { m.hasPrefix($0) }
    }

    static func isFatal(_ d: DL3Diag) -> Bool { d.fatal || isFatal(message: d.message) }

    /// One diagnostic as the rule sees it.
    struct Item: Equatable {
        var error: Bool
        var fatal: Bool
        /// The reported line (nil: none), which ties a cause to its stop.
        var line: Int? = nil
    }

    /// The indices of `items` that stay errors: under `.strict` every error;
    /// under `.bestEffort` the fatal ones and the cause of the first: the
    /// nearest error before it, not fatal itself, reported at the same line
    /// (``File `x.tex' not found`` and its "Emergency stop" share the
    /// `\input`'s line; "File ended while scanning use of \textbf" and its
    /// stop have none). An earlier error elsewhere is not the cause. Every
    /// other error is shown as a warning marked `recoveredNote`.
    static func keptErrors(_ items: [Item], mode: Mode) -> Set<Int> {
        let errors = items.indices.filter { items[$0].error }
        guard mode == .bestEffort else { return Set(errors) }
        var kept = Set(errors.filter { items[$0].fatal })
        if let first = errors.first(where: { items[$0].fatal }),
           let cause = errors.last(where: { $0 < first && !items[$0].fatal }), items[cause].line == items[first].line {
            kept.insert(cause)
        }
        return kept
    }

    /// `message` with the recovered mark.
    static func marked(_ message: String) -> String { "\(message) (\(recoveredNote))" }
}
