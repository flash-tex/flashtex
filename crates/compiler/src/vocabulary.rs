//! Command and environment vocabulary for diagnostic classification.
//!
//! A name is *known* when this compiler implements it or it is listed below as
//! real LaTeX2e / amsmath / common-package vocabulary that is not implemented.
//! Anything else is reported as `unknown_command` (usually a typo), with a
//! did-you-mean suggestion drawn from the same vocabulary. The unimplemented
//! lists are deliberately modest: a real command missing from them is
//! misreported as unknown, never the reverse.

use crate::math::{COMMAND_GLYPHS, DELIMITER_COMMANDS, GRID_ENVIRONMENTS, OPERATOR_NAMES};
use crate::parser::BUILT_INS;

/// Commands `math.rs` handles by name in its command dispatch, beyond the
/// glyph, operator and delimiter tables.
#[rustfmt::skip]
pub(crate) const MATH_COMMANDS: &[&str] = &[
    "operatorname", "mathrm", "mathit", "mathsf", "mathtt", "mathnormal", "boldsymbol", "bm",
    "mbox", "hbox", "textrm", "textit", "textnormal", "displaystyle", "textstyle", "scriptstyle",
    "scriptscriptstyle", "nonumber", "notag", "middle", "left", "right", "big", "Big", "bigg",
    "Bigg", "bigm", "Bigm", "biggm", "Biggm", "Bigl", "Bigr", "biggl", "biggr", "Biggl", "Biggr",
    "dots", "ldots", "dotsc", "dotso", "cdots", "dotsb", "dotsm", "dotsi", "iint", "lbrace",
    "rbrace", "iiint", "bmod", "mod", "dfrac", "tfrac", "cfrac", "frac", "begin", "sqrt", "overset",
    "stackrel", "underset", "binom", "dbinom", "tbinom", "mathbf", "textbf", "boxed", "overline",
    "underline", "tag", "pmod", "text", "bigl", "bigr", "quad", "qquad", "mathbb", "hat", "bar",
    "vec", "tilde", "dot", "ddot", "check", "breve", "acute", "grave", "widehat", "widetilde",
    "overbrace", "underbrace", "overrightarrow", "overleftarrow", "overleftrightarrow",
    "num", "qty", "unit", "si", "SI", "numlist", "numrange", "qtylist", "qtyrange", "SIlist",
    "SIrange", "ang", "sisetup",
    "underrightarrow", "underleftarrow", "underleftrightarrow", "Bbb", "bold", "dashrightarrow",
    "dasharrow", "dashleftarrow",
];

/// Real LaTeX2e, amsmath/amssymb and widely used package commands this
/// compiler does not implement.
#[rustfmt::skip]
const KNOWN_UNIMPLEMENTED_COMMANDS: &[&str] = &[
    // LaTeX2e document structure and front matter.
    "part", "chapter", "subsubsection", "paragraph", "subparagraph", "appendix", "maketitle",
    "title", "author", "date", "thanks", "and", "today", "tableofcontents", "listoffigures",
    "listoftables", "abstractname", "footnote", "footnotemark", "footnotetext", "marginpar",
    "index", "glossary", "bibliography", "bibliographystyle", "bibitem", "cite", "nocite",
    // Boxes, spacing, breaking and page control.
    "centering", "raggedright", "raggedleft", "linespread", "vfill", "hss", "vss", "vbox",
    "makebox", "fbox", "framebox", "parbox", "raisebox", "rule", "newline", "linebreak",
    "nolinebreak", "pagebreak", "nopagebreak", "clearpage", "cleardoublepage", "thispagestyle",
    "enlargethispage", "indent", "phantom", "hphantom", "vphantom", "smash", "strut", "addvspace",
    "vskip", "hskip", "kern", "enspace", "thinspace", "negthinspace", "hline", "cline",
    "multicolumn", "tabularnewline", "arraystretch",
    // Fonts and text symbols.
    "textsuperscript", "textsubscript", "underbar", "sout", "uline", "LaTeX",
    "LaTeXe", "TeX", "dag", "ddag", "S", "P", "copyright", "pounds", "textbackslash",
    "textasciitilde", "textasciicircum", "textbar", "textless", "textgreater", "textendash",
    "nobreakspace",
    "textemdash", "textbullet", "textperiodcentered", "textquoteleft", "textquoteright",
    "textquotedblleft", "textquotedblright", "ldots", "slash", "selectfont", "fontsize",
    "fontfamily", "usefont",
    // Definitions, counters and programming.
    "def", "edef", "gdef", "let", "providecommand", "newenvironment", "renewenvironment",
    "newtheorem", "newcounter", "setcounter", "addtocounter", "stepcounter", "refstepcounter",
    "value", "arabic", "roman", "Roman", "alph", "Alph", "fnsymbol", "the", "makeatletter",
    "makeatother", "ifthenelse", "newif", "relax", "expandafter", "csname", "endcsname",
    "newlength", "addtolength", "settowidth", "DeclareMathOperator", "ensuremath", "protect",
    "verb", "hyphenation", "graphicspath", "geometry", "hypersetup", "RequirePackage",
    "PassOptionsToPackage", "AtBeginDocument",
    // Cross-references and links.
    "eqref", "autoref", "cref", "Cref", "nameref", "url", "href", "hyperref", "hyperlink",
    "hypertarget", "citep", "citet", "citeauthor", "addbibresource", "printbibliography",
    // Colour and graphics packages.
    "tikz",
    "usetikzlibrary", "draw", "node", "fill", "path", "scalebox", "resizebox", "rotatebox",
    "subcaption", "captionof", "listoflistings", "lstinline", "mintinline",
    // amsmath and amssymb.
    "intertext", "shortintertext", "substack", "sideset", "xrightarrow", "xleftarrow", "overbrace",
    "underbrace", "overleftarrow", "overrightarrow", "mathcal", "mathfrak", "mathscr", "pmb",
    "limits", "nolimits", "displaylimits", "colon", "vdots", "ddots", "iff", "implies", "impliedby",
    "genfrac", "operatornamewithlimits", "dddot", "ddddot", "cancel", "bcancel", "xcancel",
    "cancelto", "numberwithin", "allowdisplaybreaks", "mathring", "lvert", "rvert", "lVert",
    "rVert", "varepsilon", "vartheta", "varphi", "varrho", "varsigma", "varpi", "digamma",
    "varkappa", "hbar", "hslash", "ell", "wp", "Re", "Im", "aleph", "beth", "gimel", "emptyset",
    "varnothing", "nabla", "partial", "infty", "forall", "exists", "nexists", "neg", "lnot", "top",
    "bot", "angle", "measuredangle", "triangle", "square", "blacksquare", "Box", "Diamond",
    "clubsuit", "diamondsuit", "heartsuit", "spadesuit", "flat", "natural", "sharp", "prime",
    "backprime", "surd", "mathstrut", "not", "neq", "ne", "leq", "le", "geq", "ge", "ll", "gg",
    "leqslant", "geqslant", "approx", "cong", "equiv", "sim", "simeq", "propto", "subset", "supset",
    "subseteq", "supseteq", "subsetneq", "supsetneq", "in", "ni", "notin", "cup", "cap", "bigcup",
    "bigcap", "setminus", "wedge", "vee", "bigwedge", "bigvee", "oplus", "otimes", "bigoplus",
    "bigotimes", "odot", "times", "div", "cdot", "circ", "bullet", "star", "ast", "pm", "mp", "sum",
    "prod", "coprod", "int", "oint", "to", "gets", "mapsto", "rightarrow", "leftarrow",
    "leftrightarrow", "Rightarrow", "Leftarrow", "Leftrightarrow", "longrightarrow",
    "longleftarrow", "Longrightarrow", "Longleftarrow", "longmapsto", "hookrightarrow",
    "hookleftarrow", "uparrow", "downarrow", "nearrow", "searrow", "mid", "nmid", "parallel",
    "perp", "vdash", "dashv", "models", "langle", "rangle", "lceil", "rceil", "lfloor", "rfloor",
    "backslash", "vert", "Vert", "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta",
    "theta", "iota", "kappa", "lambda", "mu", "nu", "xi", "pi", "rho", "sigma", "tau", "upsilon",
    "phi", "chi", "psi", "omega", "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma",
    "Upsilon", "Phi", "Psi", "Omega",
];

/// Environments this compiler implements outside math mode.
#[rustfmt::skip]
const IMPLEMENTED_ENVIRONMENTS: &[&str] = &[
    "document", "figure", "center", "flushright", "flushleft", "quote", "quotation", "itemize",
    "enumerate", "equation", "equation*", "displaymath", "gather", "gather*", "align", "align*",
    "alignat", "alignat*", "flalign", "flalign*", "multline", "multline*",
];

/// Real LaTeX2e / amsmath / common-package environments not implemented.
#[rustfmt::skip]
const KNOWN_UNIMPLEMENTED_ENVIRONMENTS: &[&str] = &[
    "description", "table", "table*", "figure*", "tabular", "tabular*", "tabularx", "longtable",
    "verbatim", "verbatim*", "verse", "abstract", "minipage", "titlepage", "thebibliography",
    "list", "trivlist", "picture", "math", "eqnarray", "eqnarray*", "gathered", "multlined",
    "subequations", "dcases", "rcases", "proof", "tikzpicture", "lstlisting", "minted",
    "wrapfigure", "subfigure", "comment", "landscape", "samepage", "sloppypar", "filecontents",
    "frame", "tabbing", "small", "footnotesize",
];

fn implemented_commands() -> impl Iterator<Item = &'static str> {
    BUILT_INS
        .iter()
        .copied()
        .chain(MATH_COMMANDS.iter().copied())
        .chain(COMMAND_GLYPHS.iter().map(|(name, _)| *name))
        .chain(crate::amssymb::command_names())
        .chain(OPERATOR_NAMES.iter().copied())
        .chain(DELIMITER_COMMANDS.iter().copied())
}

pub fn is_known_command(name: &str) -> bool {
    implemented_commands()
        .chain(KNOWN_UNIMPLEMENTED_COMMANDS.iter().copied())
        .any(|known| known == name)
}

pub fn is_known_environment(name: &str) -> bool {
    IMPLEMENTED_ENVIRONMENTS
        .iter()
        .copied()
        .chain(GRID_ENVIRONMENTS.iter().map(|(name, ..)| *name))
        .chain(KNOWN_UNIMPLEMENTED_ENVIRONMENTS.iter().copied())
        .any(|known| known == name)
}

/// The closest known command to an unknown `name`, if one is within edit
/// distance 2 (1 for names of three characters or fewer, where 2 edits
/// rewrite most of the word). Implemented commands win ties, then list order.
pub fn suggest_command(name: &str) -> Option<&'static str> {
    let limit = if name.chars().count() <= 3 { 1 } else { 2 };
    let mut best: Option<(usize, &'static str)> = None;
    for candidate in implemented_commands().chain(KNOWN_UNIMPLEMENTED_COMMANDS.iter().copied()) {
        if candidate == name {
            return None;
        }
        let distance = edit_distance(name, candidate);
        if distance <= limit && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// Optimal string alignment distance: Levenshtein plus adjacent transposition,
/// so `alpah` is one edit from `alpha`.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > 2 {
        return usize::MAX;
    }
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut value = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                value = value.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = value;
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_prefer_the_closest_known_command() {
        assert_eq!(suggest_command("alpah"), Some("alpha"));
        assert_eq!(suggest_command("textbff"), Some("textbf"));
        assert_eq!(suggest_command("sectoin"), Some("section"));
        assert_eq!(suggest_command("frobnicate"), None);
        assert_eq!(suggest_command("alpha"), None);
    }

    #[test]
    fn known_versus_unknown_commands() {
        assert!(is_known_command("tikz"));
        assert!(is_known_command("alpha"));
        assert!(is_known_command("section"));
        assert!(!is_known_command("alpah"));
        assert!(!is_known_command("textbff"));
        assert!(is_known_environment("tabular"));
        assert!(is_known_environment("pmatrix"));
        assert!(!is_known_environment("itemze"));
    }

    /// `MATH_COMMANDS` is hand-kept beside `math.rs`'s dispatch; an entry the
    /// dispatch no longer handles would make a typo look like a known command.
    #[test]
    fn every_listed_math_command_is_handled_in_math_mode() {
        let control = crate::incremental::compile_full("${\\bogusxyz{a}{b}}$", Default::default());
        assert!(control
            .diagnostics
            .iter()
            .any(|d| d.message == "\\bogusxyz is not supported in math mode"));
        for name in MATH_COMMANDS {
            let text = format!("${{\\{name}{{a}}{{b}}}}$");
            let output = crate::incremental::compile_full(&text, Default::default());
            let unhandled = format!("\\{name} is not supported in math mode");
            assert!(
                !output.diagnostics.iter().any(|d| d.message == unhandled),
                "{name}: {:?}",
                output.diagnostics
            );
        }
    }
}
