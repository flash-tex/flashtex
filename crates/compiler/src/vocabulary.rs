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
    "stackrel", "underset", "sideset", "binom", "dbinom", "tbinom", "mathbf", "textbf", "boxed", "overline",
    "underline", "underbar", "tag", "pmod", "text", "bigl", "bigr", "quad", "qquad", "mathbb", "hat", "bar",
    "vec", "tilde", "dot", "ddot", "check", "breve", "acute", "grave", "widehat", "widetilde",
    "dddot", "ddddot", "mathring",
    "overbrace", "underbrace", "overrightarrow", "overleftarrow", "overleftrightarrow",
    "num", "qty", "unit", "si", "SI", "numlist", "numrange", "qtylist", "qtyrange", "SIlist",
    "SIrange", "ang", "sisetup",
    "underrightarrow", "underleftarrow", "underleftrightarrow", "Bbb", "bold", "dashrightarrow",
    "dasharrow", "dashleftarrow",
    "mathllap", "mathrlap", "mathclap",
];

/// Real LaTeX2e, amsmath/amssymb and widely used package commands this
/// compiler does not implement.
#[rustfmt::skip]
const KNOWN_UNIMPLEMENTED_COMMANDS: &[&str] = &[
    // LaTeX2e document structure and front matter.
    "part", "chapter", "subsubsection", "appendix", "maketitle",
    "title", "author", "date", "thanks", "and", "today", "tableofcontents", "listoffigures",
    "listoftables", "abstractname", "footnote", "footnotemark", "footnotetext", "marginpar",
    "index", "glossary", "bibliography", "bibliographystyle", "bibitem", "cite", "nocite",
    // Boxes, spacing, breaking and page control.
    "centering", "raggedright", "raggedleft", "linespread", "vfill", "hss", "vss", "vbox",
    "makebox", "fbox", "framebox", "parbox", "raisebox", "rule", "newline",
    "clearpage", "cleardoublepage", "thispagestyle",
    "indent", "phantom", "hphantom", "vphantom", "smash", "strut", "addvspace",
    "vskip", "kern", "enspace", "thinspace", "negthinspace", "hline", "cline",
    "multicolumn", "tabularnewline", "arraystretch",
    // Fonts and text symbols.
    "textsuperscript", "textsubscript", "LaTeX",
    "LaTeXe", "TeX", "dag", "ddag", "S", "P", "copyright", "pounds", "textbackslash",
    "textasciitilde", "textasciicircum", "textbar", "textless", "textgreater", "textendash",
    "textemdash", "textbullet", "textperiodcentered", "textquoteleft", "textquoteright",
    "textquotedblleft", "textquotedblright", "ldots", "slash", "selectfont", "fontsize",
    "fontfamily", "usefont",
    // Definitions, counters and programming.
    "def", "edef", "gdef", "let", "providecommand", "newenvironment", "renewenvironment",
    "newtheorem", "newcounter", "setcounter", "addtocounter", "stepcounter", "refstepcounter",
    "value", "arabic", "roman", "Roman", "alph", "Alph", "fnsymbol", "the", "makeatletter",
    "makeatother", "newif", "relax", "expandafter", "csname", "endcsname",
    "newlength", "settowidth", "DeclareMathOperator", "ensuremath", "protect",
    "verb", "graphicspath", "allowdisplaybreaks", "geometry", "hypersetup", "lstset", "RequirePackage",
    "PassOptionsToPackage", "AtBeginDocument",
    // Cross-references and links.
    "eqref", "autoref", "nameref", "url", "href", "hyperref", "hyperlink",
    "hypertarget", "cite", "parencite", "textcite", "autocite", "citep", "citet", "citeauthor", "citeyear", "nocite", "addbibresource", "printbibliography",
    // Colour and graphics packages.
    "tikz",
    "usetikzlibrary", "draw", "node", "fill", "path", "scalebox", "resizebox", "rotatebox",
    "subcaption", "listoflistings", "lstlistoflistings", "lstinline", "mintinline",
    // amsmath and amssymb.
    "intertext", "shortintertext", "substack", "xrightarrow", "xleftarrow", "overbrace",
    "underbrace", "overleftarrow", "overrightarrow", "mathcal", "mathfrak", "mathscr", "pmb",
    "limits", "nolimits", "displaylimits", "colon", "eqqcolon", "Coloneqq", "Eqqcolon",
    "vcentcolon", "dblcolon", "vdots", "ddots", "iff", "implies", "impliedby",
    "genfrac", "operatornamewithlimits", "cancel", "bcancel", "xcancel",
    "cancelto", "numberwithin", "allowdisplaybreaks", "lvert", "rvert", "lVert",
    "rVert", "varepsilon", "vartheta", "varphi", "varrho", "varsigma", "varpi", "digamma",
    "varkappa", "hbar", "hslash", "ell", "wp", "Re", "Im", "aleph", "beth", "gimel", "emptyset",
    "varnothing", "nabla", "partial", "infty", "forall", "exists", "nexists", "neg", "lnot", "top",
    "bot", "angle", "measuredangle", "triangle", "square", "blacksquare", "Diamond",
    "clubsuit", "diamondsuit", "heartsuit", "spadesuit", "flat", "natural", "sharp", "prime",
    "backprime", "surd", "mathstrut", "not", "neq", "ne", "leq", "le", "geq", "ge", "ll", "gg",
    "leqslant", "geqslant", "approx", "cong", "equiv", "sim", "simeq", "propto", "subset", "supset",
    "subseteq", "supseteq", "subsetneq", "supsetneq", "in", "ni", "notin", "cup", "cap", "bigcup",
    "bigcap", "setminus", "wedge", "vee", "bigwedge", "bigvee", "oplus", "otimes", "bigoplus",
    "bigotimes", "ominus", "oslash", "odot", "bigcirc", "times", "div", "cdot", "circ", "bullet", "star", "ast", "pm", "mp", "sum",
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
    "document", "figure", "frame", "center", "flushright", "flushleft", "quote", "quotation", "itemize",
    "enumerate", "equation", "equation*", "displaymath", "gather", "gather*", "align", "align*",
    "alignat", "alignat*", "flalign", "flalign*", "eqnarray", "eqnarray*", "multline", "multline*",
    "tiny", "scriptsize", "footnotesize", "small", "normalsize",
    "large", "Large", "LARGE", "huge", "Huge",
    "tabbing",
];

/// Real LaTeX2e / amsmath / common-package environments not implemented.
#[rustfmt::skip]
const KNOWN_UNIMPLEMENTED_ENVIRONMENTS: &[&str] = &[
    "description", "table", "table*", "figure*", "tabular", "tabular*", "tabularx", "longtable",
    "verbatim", "verbatim*", "verse", "abstract", "minipage", "titlepage", "thebibliography",
    "list", "trivlist", "picture", "math", "gathered", "multlined",
    "subequations", "proof", "tikzpicture", "lstlisting", "minted",
    "wrapfigure", "subfigure", "comment", "landscape", "filecontents",
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
    // A set, not a scan of every table: an unknown command inside a runaway
    // macro loop is diagnosed hundreds of thousands of times.
    static KNOWN: std::sync::OnceLock<std::collections::HashSet<&'static str>> = std::sync::OnceLock::new();
    KNOWN
        .get_or_init(|| implemented_commands().chain(KNOWN_UNIMPLEMENTED_COMMANDS.iter().copied()).collect())
        .contains(name)
}

pub fn is_known_environment(name: &str) -> bool {
    IMPLEMENTED_ENVIRONMENTS
        .iter()
        .copied()
        .chain(GRID_ENVIRONMENTS.iter().map(|(name, ..)| *name))
        .chain(KNOWN_UNIMPLEMENTED_ENVIRONMENTS.iter().copied())
        .any(|known| known == name)
}

/// Every *distinct* known command that is an equally good reading of the typo
/// `name`, best first. Empty when `name` is itself known or nothing is close
/// enough.
///
/// "Equally good" is two rounds. First the minimal edit distance, within 2
/// (1 for names of three characters or fewer, where 2 edits rewrite most of
/// the word). Then, among those, the smallest difference in length: an author
/// who typed five characters far more often transposed or mistyped one of them
/// than typed a whole extra one, so for `\alpah` the same-length `\alpha` beats
/// the shorter `\alph`, though both are one edit away.
///
/// What survives both rounds is a genuine ambiguity. Case-only variants always
/// do, since they cannot differ in length — and `\Bigl` versus `\bigl`, or
/// `\Alph` versus `\alph`, is exactly the pair no heuristic should pick
/// between: the two render differently and the author meant one of them.
///
/// The vocabulary tables overlap (`\alpha` is listed more than once), so the
/// result is de-duplicated: a name repeated across tables is one candidate,
/// not a tie with itself.
pub fn closest_commands(name: &str) -> Vec<&'static str> {
    // Memoised per thread: the same unknown name repeats (a runaway macro
    // loop diagnoses it hundreds of thousands of times), and each lookup
    // measures the distance to every vocabulary entry.
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<String, Vec<&'static str>>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    if let Some(hit) = CACHE.with(|cache| cache.borrow().get(name).cloned()) {
        return hit;
    }
    let result = closest_commands_uncached(name);
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 4096 {
            cache.clear();
        }
        cache.insert(name.to_string(), result.clone());
    });
    result
}

fn closest_commands_uncached(name: &str) -> Vec<&'static str> {
    let width = name.chars().count();
    let limit = if width <= 3 { 1 } else { 2 };
    let mut best = usize::MAX;
    let mut matches: Vec<&'static str> = Vec::new();
    for candidate in implemented_commands().chain(KNOWN_UNIMPLEMENTED_COMMANDS.iter().copied()) {
        if candidate == name {
            return Vec::new();
        }
        // The distance is at least the difference in length: skip before
        // `edit_distance` copies `name` (a 100k-character control sequence
        // was copied once per vocabulary entry).
        if candidate.chars().count().abs_diff(width) > limit {
            continue;
        }
        let distance = edit_distance(name, candidate);
        if distance > limit {
            continue;
        }
        if distance < best {
            best = distance;
            matches.clear();
            matches.push(candidate);
        } else if distance == best && !matches.contains(&candidate) {
            matches.push(candidate);
        }
    }
    let closest_width = matches
        .iter()
        .map(|c| c.chars().count().abs_diff(width))
        .min();
    if let Some(closest_width) = closest_width {
        matches.retain(|c| c.chars().count().abs_diff(width) == closest_width);
    }
    matches
}

/// The closest known command to an unknown `name`. Ties are broken by list
/// order (implemented commands first), so this is a *hint* for prose — it may
/// be one of several equally close names. Anything mechanical must use
/// [`unambiguous_command_fix`] instead.
pub fn suggest_command(name: &str) -> Option<&'static str> {
    closest_commands(name).first().copied()
}

/// The single known command to rewrite `name` to, or `None` when the closest
/// match is not unique.
///
/// This is the only suggestion source allowed to drive a mechanical edit
/// (`suggestion` / `help.replacement`), because such an edit is applied
/// without the author re-reading it — the editor accepts it on Tab. Roughly a
/// tenth of the typos this vocabulary can suggest for have two or more equally
/// close candidates (`\Bggl` is one edit from both `\Biggl` and `\Bigl`;
/// `\igl` from both `\Bigl` and `\bigl`), and picking by list order there is a
/// coin flip that silently changes delimiter size or case. Those keep prose
/// help naming the candidates and offer no edit.
pub fn unambiguous_command_fix(name: &str) -> Option<&'static str> {
    match closest_commands(name).as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

/// Closest of `names` to `needle`, same distance limit as [`suggest_command`].
pub fn nearest_name<'a>(needle: &str, names: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let limit = if needle.chars().count() <= 3 { 1 } else { 2 };
    let mut best: Option<(usize, &'a str)> = None;
    for candidate in names {
        if candidate == needle {
            continue;
        }
        let distance = edit_distance(needle, candidate);
        if distance <= limit && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, name)| name)
}

fn is_math_command(name: &str) -> bool {
    MATH_COMMANDS.contains(&name)
        || COMMAND_GLYPHS.iter().any(|(n, _)| *n == name)
        || OPERATOR_NAMES.contains(&name)
        || DELIMITER_COMMANDS.contains(&name)
}

/// Package that defines `name`, when that is the useful help.
pub fn command_package(name: &str) -> Option<&'static str> {
    match name {
        "tikz" | "usetikzlibrary" | "draw" | "node" | "fill" | "path" => Some("tikz"),
        "includegraphics" | "graphicspath" | "scalebox" | "resizebox" | "rotatebox"
        | "reflectbox" => Some("graphicx"),
        "lstinline" | "lstlistoflistings" | "lstset" => Some("listings"),
        "mintinline" | "listoflistings" => Some("minted"),
        "citep" | "citet" | "citeauthor" | "citeyear" => Some("natbib"),
        "cite" | "parencite" | "textcite" | "autocite" | "nocite" => Some("biblatex"),
        "addbibresource" | "printbibliography" => Some("biblatex"),
        "eqref" | "intertext" | "shortintertext" | "substack" | "DeclareMathOperator"
        | "numberwithin" | "allowdisplaybreaks" => Some("amsmath"),
        "cref" | "Cref" | "crefrange" | "Crefrange" | "cpageref" | "Cpageref"
        | "labelcref" | "crefname" | "Crefname" => Some("cleveref"),
        "autoref" | "nameref" | "url" | "href" | "hyperref" | "hyperlink" | "hypertarget"
        | "hypersetup" => Some("hyperref"),
        "geometry" => Some("geometry"),
        _ => None,
    }
}

/// `= help:` for an unsupported text-mode command (issue #277).
///
/// Returns `None` when the diagnostic message already says everything useful
/// (a known command with no extra package/mode hint).
pub fn command_help(name: &str) -> Option<String> {
    if is_math_command(name) {
        return Some(format!("wrap this in math mode: \\(\\{name}\\)"));
    }
    if let Some(package) = command_package(name) {
        return Some(format!(
            "\\{name} is a {package} command, which this compiler does not implement"
        ));
    }
    if is_known_command(name) {
        return None;
    }
    if let Some(known) = suggest_command(name) {
        return Some(format!("did you mean \\{known}?"));
    }
    Some("no known LaTeX command has this name; check the spelling".into())
}

/// Help when a command has no math-mode definition. Known text commands get a
/// mode hint; otherwise the message already says it is unsupported.
pub fn math_mode_help(name: &str) -> Option<String> {
    if is_known_command(name) && !is_math_command(name) {
        Some(format!(
            "\\{name} is a text command; use it outside math or inside \\text{{...}}"
        ))
    } else {
        None
    }
}

/// `= help:` for an unimplemented environment. Only when a package name is
/// extra information; the diagnostic already says the body is plain text.
pub fn environment_help(name: &str) -> Option<String> {
    let package = match name {
        "tikzpicture" => Some("tikz"),
        "lstlisting" => Some("listings"),
        "minted" => Some("minted"),
        "longtable" => Some("longtable"),
        "tabularx" => Some("tabularx"),
        "wrapfigure" => Some("wrapfig"),
        "subfigure" => Some("subcaption"),
        "landscape" => Some("lscape"),
        _ => None,
    };
    package.map(|p| {
        format!("environment '{name}' needs the {p} package, which this compiler does not implement")
    })
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
        assert_eq!(nearest_name("s2", ["s1", "sec2"].into_iter()), Some("s1"));
        assert_eq!(nearest_name("nope", ["intro", "later"].into_iter()), None);
    }

    /// The everyday typos keep their mechanical fix: one candidate, no tie.
    #[test]
    fn an_unambiguous_typo_still_offers_an_automatic_fix() {
        for (typo, fixed) in [
            ("alpah", "alpha"),
            ("textbff", "textbf"),
            ("sectoin", "section"),
        ] {
            assert_eq!(closest_commands(typo), vec![fixed], "{typo}");
            assert_eq!(unambiguous_command_fix(typo), Some(fixed), "{typo}");
        }
        assert_eq!(unambiguous_command_fix("frobnicate"), None);
        assert_eq!(unambiguous_command_fix("alpha"), None);
    }

    /// A tie must not be resolved by list order for anything mechanical.
    /// These all survive the length round because the candidates are the same
    /// length as each other — case-only variants (`\Bigl`/`\bigl`,
    /// `\Alph`/`\alph`) always do, and they render differently, so applying
    /// either silently would be wrong half the time. `suggest_command` may
    /// still name one for prose.
    #[test]
    fn an_ambiguous_typo_offers_no_automatic_fix() {
        for typo in ["igl", "lph", "igm"] {
            let candidates = closest_commands(typo);
            assert!(candidates.len() > 1, "{typo} expected a tie, got {candidates:?}");
            assert_eq!(unambiguous_command_fix(typo), None, "{typo} {candidates:?}");
            assert!(suggest_command(typo).is_some(), "{typo} still hints in prose");
        }
    }

    /// Candidate lists are de-duplicated: a command that appears in more than
    /// one vocabulary table is one candidate, not a self-tie that would
    /// suppress a perfectly good fix.
    #[test]
    fn a_name_repeated_across_tables_is_not_a_tie() {
        let repeated: Vec<&str> = {
            let mut seen: Vec<&str> = Vec::new();
            let mut twice: Vec<&str> = Vec::new();
            for c in implemented_commands().chain(KNOWN_UNIMPLEMENTED_COMMANDS.iter().copied()) {
                if seen.contains(&c) {
                    if !twice.contains(&c) {
                        twice.push(c);
                    }
                } else {
                    seen.push(c);
                }
            }
            twice
        };
        assert!(!repeated.is_empty(), "expected the tables to overlap");
        for name in repeated {
            let candidates = closest_commands(name);
            assert!(
                !candidates.contains(&name),
                "{name} must not be its own candidate: {candidates:?}"
            );
            // Every candidate list is distinct, whatever the tables do.
            let mut sorted = candidates.clone();
            sorted.sort_unstable();
            let before = sorted.len();
            sorted.dedup();
            assert_eq!(before, sorted.len(), "{name} has duplicate candidates");
        }
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

    #[test]
    fn help_does_not_restate_the_diagnostic_message() {
        assert_eq!(
            command_help("tikz").as_deref(),
            Some("\\tikz is a tikz command, which this compiler does not implement")
        );
        assert_eq!(
            command_help("alpha").as_deref(),
            Some("wrap this in math mode: \\(\\alpha\\)")
        );
        assert!(command_help("maketitle").is_none(), "{:?}", command_help("maketitle"));
        assert_eq!(
            command_help("alpah").as_deref(),
            Some("did you mean \\alpha?")
        );
        assert_eq!(
            command_help("frobnicate").as_deref(),
            Some("no known LaTeX command has this name; check the spelling")
        );
        assert_eq!(
            math_mode_help("centering").as_deref(),
            Some("\\centering is a text command; use it outside math or inside \\text{...}")
        );
        assert!(math_mode_help("bogusxyz").is_none());
        assert!(math_mode_help("alpha").is_none());
        // Slice 2 (#549 follow-up): the lap family is implemented
        // (`Nucleus::Lap`), so it must read as math vocabulary, not as
        // unimplemented text commands — otherwise text-mode use gets no
        // mode hint and math-mode help calls them text commands.
        for name in ["mathllap", "mathrlap", "mathclap"] {
            assert!(is_known_command(name), "{name}");
            assert_eq!(
                command_help(name),
                Some(format!("wrap this in math mode: \\(\\{name}\\)")),
                "{name}"
            );
            assert!(math_mode_help(name).is_none(), "{name}");
            assert!(closest_commands(name).is_empty(), "{name}");
        }
        assert!(environment_help("tabbing").is_none());
        assert_eq!(
            environment_help("tikzpicture").as_deref(),
            Some("environment 'tikzpicture' needs the tikz package, which this compiler does not implement")
        );
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
