//! `\renewcommand` of a command that pdflatex already has.
//!
//! The pinned compiler's macro expander only knows the commands it has seen
//! defined, so it answers `\renewcommand{\epsilon}{\varepsilon}` with
//! `LaTeX Error: Command \epsilon undefined.` (recovery: "defined the command
//! anyway"). pdflatex reports nothing: `\epsilon` comes from `fontmath.ltx`,
//! `\proofname` from amsthm, `\figurename` from the class. Student preambles
//! are full of these lines, and each one showed as an error.
//!
//! [`renewed`] tells those diagnostics apart, using what the document loads:
//!
//! * [`Renewed::Applied`]: a math command. The expander defined the new
//!   meaning and every later use expands to it, so the output already
//!   follows pdflatex (`\epsilon` sets cmmi's `\varepsilon`, measured). The
//!   error is dropped.
//! * [`Renewed::NotApplied`]: a name the compiler or the pipeline typesets
//!   from a fixed string (`Proof`, `Figure`, the `\labelenumi` label). The
//!   default is still printed, so the error becomes a warning that says that,
//!   instead of a false "undefined".
//!
//! Anything else keeps the compiler's error. The real fix is the expander
//! knowing the host's predefined commands and the compiler reading these
//! names; this is what reaches users without a `vendor/compiler` re-pin.

use flashtex_compiler::math::MathPackages;
use flashtex_compiler::math_symbols::{self, Provider};

/// What the document loads, for [`renewed`].
pub(crate) struct Loaded<'a> {
    class: &'a str,
    packages: &'a [String],
    math: MathPackages,
}

impl<'a> Loaded<'a> {
    pub(crate) fn new(class: &'a str, packages: &'a [String]) -> Self {
        let mut math = MathPackages::KERNEL;
        math.load_class(class);
        for p in packages {
            math.load_package(p);
        }
        Loaded { class, packages, math }
    }

    fn package(&self, name: &str) -> bool {
        self.packages.iter().any(|p| p == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Renewed {
    /// The new definition is in force; pdflatex has no diagnostic.
    Applied,
    /// The default is still printed.
    NotApplied,
}

/// The control sequence of an expander `LaTeX Error: Command \<name>
/// undefined.`, as the compiler words it.
pub(crate) fn undefined_command(message: &str) -> Option<&str> {
    message.strip_prefix("LaTeX Error: Command \\")?.strip_suffix(" undefined.")
}

/// Whether the compiler's "undefined" error for `\renewcommand{\<name>}` is
/// false, and if so whether the new definition shows in the output.
pub(crate) fn renewed(message: &str, loaded: &Loaded) -> Option<Renewed> {
    let name = undefined_command(message)?;
    if math_command(name, loaded) {
        return Some(Renewed::Applied);
    }
    text_name(name, loaded)
}

/// `fontmath.ltx`/`latex.ltx` math commands that are macros rather than
/// `\DeclareMath...` declarations, so the symbol table does not list them.
const KERNEL_MATH: &[&str] = &[
    // `\DeclareMathOperator`-style log-likes (latex.ltx).
    "arccos", "arcsin", "arctan", "arg", "cos", "cosh", "cot", "coth", "csc", "deg", "det", "dim", "exp", "gcd", "hom", "inf",
    "ker", "lg", "lim", "liminf", "limsup", "ln", "log", "max", "min", "Pr", "sec", "sin", "sinh", "sup", "tan", "tanh",
    // Math alphabets and constructions.
    "mathbf", "mathrm", "mathit", "mathsf", "mathtt", "mathcal", "mathnormal", "frac", "overline", "underline", "overbrace",
    "underbrace", "ldots", "cdots", "vdots", "ddots", "dots", "pmod", "bmod", "iff", "neq", "ne",
];

/// amsmath's own macros (`amsmath.sty`, `amsopn.sty`).
const AMSMATH: &[&str] = &["implies", "impliedby", "dfrac", "tfrac", "binom", "dbinom", "tbinom", "text", "operatorname", "iint", "iiint", "boldsymbol"];

/// amsfonts' alphabets (`amsfonts.sty`).
const AMSFONTS: &[&str] = &["mathbb", "mathfrak"];

fn math_command(name: &str, loaded: &Loaded) -> bool {
    let provided = |p: Provider| match p {
        Provider::Kernel => true,
        Provider::Latexsym => loaded.package("latexsym"),
        Provider::Amsfonts => loaded.math.amsfonts,
        Provider::Amssymb => loaded.math.amssymb,
        Provider::Stmaryrd => loaded.package("stmaryrd"),
        Provider::Mathrsfs => loaded.package("mathrsfs"),
        Provider::Amsmath => loaded.math.amsmath,
    };
    math_symbols::declarations(name).any(|s| provided(s.provider))
        || math_symbols::aliases_of(name).any(|(_, p)| provided(p))
        || KERNEL_MATH.contains(&name)
        || loaded.math.amsmath && AMSMATH.contains(&name)
        || loaded.math.amsfonts && AMSFONTS.contains(&name)
}

/// The standard classes (and the AMS and KOMA ones built on the same
/// names) that define the caption and list names below.
const TEXT_CLASSES: &[&str] = &["article", "report", "book", "amsart", "amsbook", "amsproc", "scrartcl", "scrreprt", "scrbook"];

/// Classes with `\chapter`: `\chaptername` and `\bibname` instead of
/// `\refname`.
const CHAPTER_CLASSES: &[&str] = &["report", "book", "amsbook", "scrreprt", "scrbook"];

/// Names pdflatex defines whose redefinition the pipeline does not print.
/// Each was measured against pdflatex (TeX Live 2026): the default text
/// (`Proof.`, `Figure 1:`, `1.`, `Chapter 1`, ...) is set where pdflatex
/// sets the new one.
fn text_name(name: &str, loaded: &Loaded) -> Option<Renewed> {
    let amsthm = loaded.package("amsthm") || matches!(loaded.class, "amsart" | "amsbook" | "amsproc" | "beamer");
    let class = TEXT_CLASSES.contains(&loaded.class);
    let chapters = CHAPTER_CLASSES.contains(&loaded.class);
    let defined = match name {
        "proofname" | "qed" => amsthm,
        "abstractname" => class && !chapters || matches!(loaded.class, "report" | "scrreprt"),
        "figurename" | "tablename" | "partname" | "appendixname" => class,
        "labelenumi" | "labelenumii" | "labelenumiii" | "labelenumiv" | "labelitemi" | "labelitemii" | "labelitemiii" | "labelitemiv" => class,
        "refname" => class && !chapters,
        "bibname" | "chaptername" => chapters,
        _ => false,
    };
    if !defined {
        return None;
    }
    // `abstract` is set by the pipeline from the source, `\abstractname`
    // included (`abstractenv`).
    Some(if name == "abstractname" { Renewed::Applied } else { Renewed::NotApplied })
}

/// The warning that replaces the error for a [`Renewed::NotApplied`] name.
pub(crate) fn not_applied_message(name: &str) -> String {
    format!("\\renewcommand{{\\{name}}} is not applied yet: FlashTeX still prints the default text there")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packages(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn check(class: &str, pkgs: &[&str], name: &str) -> Option<Renewed> {
        let p = packages(pkgs);
        renewed(&format!("LaTeX Error: Command \\{name} undefined."), &Loaded::new(class, &p))
    }

    #[test]
    fn kernel_math_symbols_are_applied() {
        for name in ["epsilon", "phi", "vec", "Re", "Im", "le", "emptyset", "bar", "hat", "sin", "lim", "mathbf"] {
            assert_eq!(check("article", &[], name), Some(Renewed::Applied), "{name}");
        }
    }

    #[test]
    fn package_math_needs_its_package() {
        assert_eq!(check("article", &[], "varnothing"), None);
        assert_eq!(check("article", &["amssymb"], "varnothing"), Some(Renewed::Applied));
        assert_eq!(check("article", &[], "implies"), None);
        assert_eq!(check("article", &["amsmath"], "implies"), Some(Renewed::Applied));
        assert_eq!(check("article", &["amssymb"], "mathbb"), Some(Renewed::Applied));
    }

    #[test]
    fn fixed_text_names_are_reported_as_not_applied() {
        assert_eq!(check("article", &["amsthm"], "proofname"), Some(Renewed::NotApplied));
        assert_eq!(check("article", &[], "proofname"), None);
        assert_eq!(check("article", &[], "labelenumi"), Some(Renewed::NotApplied));
        assert_eq!(check("article", &[], "refname"), Some(Renewed::NotApplied));
        assert_eq!(check("report", &[], "refname"), None);
        assert_eq!(check("report", &[], "bibname"), Some(Renewed::NotApplied));
        assert_eq!(check("article", &[], "chaptername"), None);
        assert_eq!(check("article", &[], "abstractname"), Some(Renewed::Applied));
        assert_eq!(check("book", &[], "abstractname"), None);
    }

    #[test]
    fn user_commands_and_other_messages_keep_the_error() {
        assert_eq!(check("article", &["amsmath", "amssymb"], "R"), None);
        assert_eq!(check("article", &[], "thetheorem"), None);
        let p = packages(&[]);
        assert_eq!(renewed("LaTeX Error: Command \\epsilon already defined.", &Loaded::new("article", &p)), None);
    }
}
