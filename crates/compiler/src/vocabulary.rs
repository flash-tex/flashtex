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
    "mbox", "hbox", "textrm", "textit", "textnormal", "textup", "displaystyle", "textstyle", "scriptstyle",
    "scriptscriptstyle", "nonumber", "notag", "middle", "left", "right", "big", "Big", "bigg",
    "Bigg", "bigm", "Bigm", "biggm", "Biggm", "Bigl", "Bigr", "biggl", "biggr", "Biggl", "Biggr",
    "dots", "ldots", "dotsc", "dotso", "cdots", "dotsb", "dotsm", "dotsi", "iint", "lbrace",
    "rbrace", "iiint", "bmod", "mod", "dfrac", "tfrac", "cfrac", "frac", "begin", "sqrt", "overset",
    "stackrel", "underset", "sideset", "binom", "dbinom", "tbinom", "mathbf", "textbf", "boxed", "overline",
    "underline", "underbar", "tag", "pmod", "pod", "text", "bigl", "bigr", "quad", "qquad", "mathbb", "hat", "bar",
    "vec", "tilde", "dot", "ddot", "check", "breve", "acute", "grave", "widehat", "widetilde",
    "dddot", "ddddot", "mathring",
    "overbrace", "underbrace", "overrightarrow", "overleftarrow", "overleftrightarrow",
    "num", "qty", "unit", "si", "SI", "numlist", "numrange", "qtylist", "qtyrange", "SIlist",
    "SIrange", "ang", "sisetup",
    "underrightarrow", "underleftarrow", "underleftrightarrow", "Bbb", "bold", "dashrightarrow",
    "dasharrow", "dashleftarrow",
    "mathllap", "mathrlap", "mathclap",
    // mathtools' sixteen further extensible arrows (`math.rs` gates each on
    // `\usepackage{mathtools}`, like the lap family above).
    "xmapsto", "xhookleftarrow", "xhookrightarrow", "xLeftarrow", "xRightarrow",
    "xLeftrightarrow", "xLongleftarrow", "xLongrightarrow", "xlongleftarrow",
    "xlongrightarrow", "xleftharpoonup", "xleftharpoondown", "xrightharpoonup",
    "xrightharpoondown", "xleftrightharpoons", "xrightleftharpoons",
    "cancel", "bcancel", "xcancel",
    // amsmath `\pmb` (poor-man's bold), kernel `\mathstrut` (`\vphantom{(})`)
    // and kernel `\smash` (amsmath's `[t]`/`[b]` option included).
    "pmb", "mathstrut", "smash",
    // Issue #846: the kernel/amsmath arms the real-document corpus dropped.
    "backslash", "lvert", "rvert", "lVert", "rVert", "vert", "Vert", "ensuremath", "mkern",
    "mskip", "medspace", "thickspace", "negmedspace", "negthickspace", "thinspace",
    "negthinspace", "hdots", "rm", "bf", "it", "sf", "tt", "cal", "mit",
];

/// Real LaTeX2e, amsmath/amssymb and widely used package commands this
/// compiler does not implement.
///
/// A name here must not be implemented anywhere: `parser::BUILT_INS`, the
/// tabular row scanner (`parser::tabular`), the math tables, the amssymb
/// inventory and the expansion pass all count — `implemented_commands` is
/// the check, and the unit test below enforces the disjointness.
/// Deliberately kept although handled elsewhere: `\chapter` (a class-gated
/// parser arm; article-class use must report unsupported, not unknown),
/// `\relax` (consumed silently by the expansion pass and glue parsing) and
/// `\makeatletter` (consumed silently by the engine prelude).
#[rustfmt::skip]
const KNOWN_UNIMPLEMENTED_COMMANDS: &[&str] = &[
    // LaTeX2e document structure and front matter.
    "part", "chapter", "appendix", "abstractname",
    // Boxes, spacing, breaking and page control.
    "makebox", "fbox", "framebox", "parbox", "raisebox", "llap", "rlap", "linespread",
    "vbox", "newline",
    // Fonts and text symbols.
    "fontfamily", "usefont",
    "textemdash", "textendash", "textquoteleft", "textquoteright",
    "textquotedblleft", "textquotedblright", "slash",
    // Definitions, counters and programming.
    "def", "edef", "gdef", "let", "the", "makeatletter", "relax",
    "expandafter", "csname", "endcsname", "protect",
    // Cross-references and links.
    "autoref", "nameref", "hyperref", "hyperlink", "hypertarget",
    // Colour and graphics packages.
    // `\usetikzlibrary` and the pgf setup commands have parser arms now
    // (`pgf_setup_command`); the picture commands stay unimplemented here.
    "tikz",
    "draw", "node", "fill", "path",
    "subcaption", "listoflistings", "lstinline", "mintinline",
    // amsmath and amssymb.
    "mathscr", "cancelto",
    // `\hookleftarrow` is `\leftarrow\joinrel\rhook` and cmmi "2D `\rhook`
    // has no glyph in a bundled face (tools/kernel-math-gap); the other
    // kernel symbols once listed here come from the generated declaration
    // table (`crate::math_symbols`) now.
    "hookleftarrow",
    // The geometry package is implemented; its `\geometry` command is not.
    "geometry",
    // fontspec (XeLaTeX/LuaLaTeX-only): recognised so unguarded use reports
    // `unsupported_feature` naming the package instead of an unknown-command
    // typo hunt. A block guarded by `\ifxetex`/`\ifluatex` (false here, as
    // under pdflatex) never reaches this diagnostic.
    "setmainfont", "setsansfont", "setmonofont", "newfontfamily", "fontspec",
    "defaultfontfeatures", "addfontfeature",
];

/// Real LaTeX2e / amsmath / common-package environments not implemented.
///
/// A name here must not be implemented anywhere: `supported::TEXT_ENVIRONMENTS`
/// and the math grids both count. `tabular`, `longtable` and friends used to
/// be listed here while the parser implemented them; the unit test below
/// enforces the disjointness now.
#[rustfmt::skip]
const KNOWN_UNIMPLEMENTED_ENVIRONMENTS: &[&str] = &[
    "table*", "figure*",
    "abstract", "minipage", "titlepage",
    "picture", "math", "multlined",
    "tikzpicture", "minted",
    "wrapfigure", "subfigure", "landscape", "filecontents",
];

/// Commands handled by name outside every table above: amsmath's
/// `\intertext`/`\shortintertext`, consumed with their braced argument by the
/// multi-row display environments (`parser.rs `multirow_environment`), never
/// by inline math. Kept out of [`MATH_COMMANDS`] on purpose: that list's unit
/// test compiles each entry in inline `$...$`, where these two do not belong.
const MATH_DISPLAY_COMMANDS: &[&str] = &["intertext", "shortintertext"];

fn implemented_commands() -> impl Iterator<Item = &'static str> {
    BUILT_INS
        .iter()
        .copied()
        .chain(crate::supported::TEXT_EXTRA_ARMS.iter().copied())
        .chain(MATH_COMMANDS.iter().copied())
        .chain(COMMAND_GLYPHS.iter().map(|(name, _)| *name))
        .chain(crate::amssymb::command_names())
        .chain(OPERATOR_NAMES.iter().copied())
        .chain(DELIMITER_COMMANDS.iter().copied())
        // Math structures (`supported::MATH_STRUCTURES`: `\genfrac`,
        // `\substack`, `\phantom`, ...) and expansion-pass commands
        // (`supported::EXPANSION_COMMANDS`): implemented, so a use outside
        // their context reports unsupported, never an unknown-command typo.
        .chain(
            crate::supported::MATH_STRUCTURES
                .iter()
                .flat_map(|(names, ..)| names.iter().copied()),
        )
        .chain(crate::supported::expansion_command_names())
        .chain(MATH_DISPLAY_COMMANDS.iter().copied())
}

pub fn is_known_command(name: &str) -> bool {
    // A set, not a scan of every table: an unknown command inside a runaway
    // macro loop is diagnosed hundreds of thousands of times.
    static KNOWN: std::sync::OnceLock<std::collections::HashSet<&'static str>> = std::sync::OnceLock::new();
    KNOWN
        .get_or_init(|| implemented_commands().chain(KNOWN_UNIMPLEMENTED_COMMANDS.iter().copied()).collect())
        .contains(name)
}

/// Whether `name` is still carried in `KNOWN_UNIMPLEMENTED_COMMANDS`
/// specifically — distinct from [`is_known_command`], which is also `true`
/// for anything actually implemented. A name that is real LaTeX and
/// implemented must not be in both: `unsupported` (`parser.rs`) asserts the
/// two lists are disjoint at debug time, and this lets a test enforce it for
/// a specific name without depending on that debug-only check.
pub fn is_listed_as_unimplemented(name: &str) -> bool {
    KNOWN_UNIMPLEMENTED_COMMANDS.contains(&name)
}

pub fn is_known_environment(name: &str) -> bool {
    crate::supported::TEXT_ENVIRONMENTS
        .iter()
        .map(|(name, _)| *name)
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
        "setmainfont" | "setsansfont" | "setmonofont" | "newfontfamily" | "fontspec"
        | "defaultfontfeatures" | "addfontfeature" => Some("fontspec"),
        _ => None,
    }
}

/// `= help:` for an unsupported text-mode command (issue #277).
///
/// Returns `None` when the diagnostic message already says everything useful
/// (a known command with no extra package/mode hint).
pub fn command_help(name: &str) -> Option<String> {
    if is_math_command(name) {
        return Some(format!("\\{name} is a math command; use it in math mode"));
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
///
/// `longtable` stays here although the environment is implemented: without
/// `\usepackage{longtable}` there is nothing to run, and the help names the
/// missing package rather than claiming the compiler cannot do it at all.
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
    package.map(|p| format!("environment '{name}' needs \\usepackage{{{p}}}"))
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
            Some("\\alpha is a math command; use it in math mode")
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
                Some(format!("\\{name} is a math command; use it in math mode")),
                "{name}"
            );
            assert!(math_mode_help(name).is_none(), "{name}");
            assert!(closest_commands(name).is_empty(), "{name}");
        }
        assert!(environment_help("tabbing").is_none());
        assert_eq!(
            environment_help("tikzpicture").as_deref(),
            Some("environment 'tikzpicture' needs \\usepackage{tikz}")
        );
    }

    /// Neither "not implemented" list may name an implemented command: the
    /// inventory (and the backlog rankings built from it) must agree with
    /// the parser. Implemented means anything `implemented_commands` knows
    /// plus every inventoried environment; the deliberately kept entries
    /// (`\chapter`, `\relax`, `\makeatletter`, `\geometry`) are covered by
    /// their own arms or silent handling, never by these lists.
    #[test]
    fn unimplemented_lists_name_nothing_implemented() {
        for name in KNOWN_UNIMPLEMENTED_COMMANDS {
            assert!(
                !implemented_commands().any(|known| known == *name),
                "\\{name} is implemented but listed as unimplemented"
            );
        }
        let implemented_envs: Vec<&str> = crate::supported::TEXT_ENVIRONMENTS
            .iter()
            .map(|(name, _)| *name)
            .chain(
                crate::math::GRID_ENVIRONMENTS
                    .iter()
                    .map(|(name, ..)| *name),
            )
            .collect();
        for name in KNOWN_UNIMPLEMENTED_ENVIRONMENTS {
            assert!(
                !implemented_envs.contains(name),
                "environment '{name}' is implemented but listed as unimplemented"
            );
        }
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
