import Foundation

/// What our compiler ACTUALLY renders today, as the `supported_features` list
/// the Mac sends with every `capture_convert` (transfer-v1; the bridge forwards
/// it to the conversion provider, whose prompt says "prefer the listed
/// supported features only when faithful; report an unsupported feature
/// instead of changing meaning"). A model otherwise returns correct LaTeX our
/// pipeline cannot typeset (`gather*`, `\mathbb`, `\text` …) and every symbol
/// inside cascades into errors (issue #2, Daniel's finding).
///
/// Pinned to `crates/compiler/src/math.rs` `COMMAND_GLYPHS` and
/// `crates/compiler/src/parser.rs` `BUILT_INS` / environments at
/// origin/agent/claude/compiler-foundation `compilerSHA`; `CaptureFeaturesTests`
/// re-derives the glyph list from that commit when it is present locally.
/// The bridge accepts at most 64 entries of at most 128 bytes each
/// (`crates/bridge/src/context.rs`), so symbols are packed into lines.
enum CaptureFeatures {
    static let compilerSHA = "49e6eb43808ac8fccb08d620b23667403fcb8667"

    /// `COMMAND_GLYPHS` command names, in the compiler's order.
    static let commandGlyphs: [String] = [
        "alpha", "beta", "gamma", "delta", "theta", "lambda", "mu", "nu", "pi", "sigma", "phi", "omega",
        "times", "div", "pm", "leq", "geq", "neq", "approx", "cdot", "infty", "sum", "int",
        "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi", "Psi", "Omega",
        "partial", "nabla", "in", "prod", "to", "gets", "Rightarrow", "Leftrightarrow", "wedge", "vee", "neg",
        "forall", "exists", "emptyset", "equiv", "sim", "subset", "subseteq", "perp", "angle", "ni", "notin",
        "supset", "supseteq", "cup", "cap",
    ]

    /// Structures the parser/math layer implement (parser.rs `BUILT_INS`,
    /// `equation`/`itemize`/`enumerate` environments, lexer `\[ \]`, math.rs
    /// `frac`/`sqrt`/`left`/`right`, `^`/`_`).
    static let structures: [String] = [
        "OUTPUT: complete LaTeX for a text paragraph; every formula wrapped in $...$ or \\[ ... \\]; never bare math commands in text",
        "inline math $...$",
        "display math \\[ ... \\] and \\begin{equation} ... \\end{equation} (with an optional \\label)",
        "\\frac{numerator}{denominator}",
        "\\sqrt{radicand}",
        "superscripts x^{...} and subscripts x_{...}",
        "\\left( ... \\right) and \\left[ ... \\right] sized delimiters",
        "\\section{...} and \\subsection{...}",
        "\\textbf{...}, \\emph{...}, \\textit{...}",
        "\\begin{itemize} / \\begin{enumerate} with \\item",
        "\\label{...} and \\ref{...}",
        "plain paragraphs of text; ASCII punctuation; digits and Latin letters in math",
    ]

    /// Constructs the pipeline cannot typeset; the provider must report them in
    /// `ambiguities` rather than emit them or silently rewrite the mathematics.
    static let unsupported: [String] = [
        "NOT supported: amsmath/amssymb environments (align, align*, gather, gather*, cases, pmatrix, bmatrix, matrix, split, multline)",
        "NOT supported: \\mathbb, \\mathcal, \\mathrm, \\text, \\operatorname, \\bigl/\\bigr, \\quad, \\qquad, \\hspace, \\displaystyle",
        "NOT supported: TikZ, tables (tabular), \\usepackage additions, \\newcommand, custom macros, \\dots/\\ldots/\\cdots",
        "RULE: an unsupported construct must be reported in ambiguities, never substituted silently",
        "RULE: write the closest supported form only when the meaning is unchanged; otherwise report and leave it out",
    ]

    /// The list sent as `supported_features` (≤64 entries, each ≤128 bytes).
    /// `flashtex-bridge` ignores it (issues #51/#23): it sends the provider its
    /// own table for the document's engine, which `capture_convert.engine`
    /// names (`crates/bridge/src/features.rs`; retirement plan #1236, S3r).
    /// It is still sent for wire compatibility.
    static func supportedFeatures() -> [String] {
        structures + symbolLines() + unsupported
    }

    /// `COMMAND_GLYPHS` packed as "symbols: \alpha \beta …" lines under the bridge's per-entry limit.
    static func symbolLines(limit: Int = 128) -> [String] {
        var lines: [String] = []
        var current = "symbols (only these render):"
        for name in commandGlyphs {
            let piece = " \\" + name
            if current.utf8.count + piece.utf8.count > limit {
                lines.append(current)
                current = "symbols (cont.):"
            }
            current += piece
        }
        lines.append(current)
        return lines
    }

    /// Default capture instructions from this shell (file picker, sample panel).
    static let defaultInstructions = "Transcribe the selected handwriting to LaTeX; preserve notation. Wrap every formula in $...$ or \\[...\\]. Use only the supported features listed; report anything else in ambiguities instead of substituting."
}
