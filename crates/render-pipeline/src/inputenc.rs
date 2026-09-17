//! Literal UTF-8 input characters as pdfLaTeX reads them.
//!
//! LaTeX (since 2018, and identically under `\usepackage[utf8]{inputenc}`)
//! makes every non-ASCII character active; its meaning is the last
//! `\DeclareUnicodeCharacter` for it — the `omsenc/ot1enc/t1enc/ts1enc.dfu`
//! tables plus `utf8.def`'s own, all modelled by `flashtex-tex-text-encoding`.
//! Two things follow that a font's `cmap` knows nothing about:
//!
//! * A character nobody declared (Greek, Cyrillic, CJK, emoji, a combining
//!   accent such as U+0301) is `! LaTeX Error: Unicode character α (U+03B1)
//!   not set up for use with LaTeX.` and typesets nothing.
//! * A declared character whose text command the current encoding lacks
//!   (`«` is `\guillemetleft`, `þ` is `\th`, `ą` is `\k a`) is `! LaTeX Error:
//!   Command \guillemetleft unavailable in encoding OT1.`; nothing is typeset
//!   for a symbol, and the base letter alone for an accent (`\k a` sets `a`).
//!
//! And in OT1 an accented letter is not a character of the font: `\"u` is
//! `\accent` inside `\add@accent`'s group, `\L` a box, `°` a TS1 symbol in its
//! own group. None of those take part in the text font's ligature/kern
//! program with their neighbours (`W\"u` has no W–u kern), where T1's
//! precomposed `ü` (slot 252) does. [`cuts_ligkern`] says which.
//!
//! Every classification here is pinned against pdfLaTeX by
//! `tests/unicode_input_oracle.rs` (`fixtures/unicode-input/reference.json`).

use std::collections::BTreeSet;

use flashtex_tex_text_encoding::encoding::{self, Composite, Declared, Default, Encoding, Resolution};
use flashtex_tex_text_encoding::unicode;

/// What pdfLaTeX does with one rejected input character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// The LaTeX error, as pdfLaTeX logs it (without the leading `! `).
    pub message: String,
    /// What is still typeset in its place: the argument of an unavailable
    /// accent command (`\k a` → `a`), or nothing.
    pub keep: Option<char>,
}

/// The document-level input setup the checks run under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSetup {
    pub encoding: Encoding,
    /// Characters the project declares itself (`\DeclareUnicodeCharacter`,
    /// `\newunicodechar`) or that a loaded package, class or babel language
    /// makes available (`packages.json`): never rejected, never cut.
    pub extra: BTreeSet<char>,
}

/// What loading a package, class or babel language does to the input
/// errors, measured with pdfLaTeX by `fixtures/unicode-input/packages.py`
/// (`packages.json`, pinned by `tests/unicode_input_oracle.rs`). A name in
/// none of these tables is unknown, and the check fails open for it.
///
/// Neither declares nor makes available any character (their errors under
/// OT1 and T1 are the plain `article` ones).
pub const NEUTRAL_PACKAGES: &[&str] = &[
    "a4wide", "abstract", "academicons", "afterpage", "algorithm", "algorithmic", "algpseudocode", "amsbsy", "amsfonts",
    "amsmath", "amssymb", "amsthm", "appendix", "array", "arydshln", "authblk", "bbm", "beramono", "bigstrut", "bm",
    "bookmark", "booktabs", "braket", "calc", "cancel", "caption", "charter", "chemfig", "chngcntr", "cite", "cleveref",
    "cmap", "color", "colortbl", "comment", "courier", "csquotes", "dcolumn", "diagbox", "dsfont", "enumerate",
    "enumitem", "epstopdf", "etoolbox", "eucal", "eurosym", "exscale", "fancyhdr", "fancyvrb", "float", "fontawesome5",
    "footmisc", "framed", "fullpage", "gensymb", "geometry", "graphics", "graphicx", "helvet", "hhline", "hyperref",
    "ifthen", "inconsolata", "indentfirst", "kpfonts", "lastpage", "latexsym", "libertine", "lipsum", "listings",
    "lmodern", "longtable", "lscape", "ltablex", "makecell", "marvosym", "mathpazo", "mathptmx", "mathrsfs", "mathtools",
    "mdframed", "mhchem", "microtype", "mlmodern", "multicol", "multirow", "natbib", "needspace", "newtxmath",
    "newtxtext", "newunicodechar", "nicefrac", "orcidlink", "palatino", "parskip", "pdflscape", "pdfpages", "pgfplots",
    "physics", "pifont", "placeins", "qrcode", "ragged2e", "relsize", "rotating", "setspace", "siunitx", "soul",
    "stmaryrd", "subcaption", "subfig", "tabu", "tabularx", "tcolorbox", "textcomp", "tgheros", "tgpagella", "tgtermes",
    "threeparttable", "tikz", "times", "titlesec", "titling", "tocloft", "ulem", "units", "upgreek", "url", "verbatim",
    "wrapfig", "xcolor", "xfrac", "xparse", "xspace", "xurl",
];
/// Packages that load `[T1]{fontenc}` themselves.
pub const T1_PACKAGES: &[&str] = &["cfr-lm", "fourier"];
/// Packages that make these characters available under OT1 (none under T1).
pub const ACCEPTING_PACKAGES: &[(&str, &str)] = &[("babel", BABEL_ACCEPTS), ("wasysym", "Ðð")];
pub const NEUTRAL_CLASSES: &[&str] = &[
    "article", "beamer", "book", "exam", "extarticle", "IEEEtran", "letter", "llncs", "memoir", "report", "revtex4-2",
    "scrartcl", "scrbook", "scrreprt", "standalone",
];
pub const T1_CLASSES: &[&str] = &["elsarticle"];
pub const ACCEPTING_CLASSES: &[(&str, &str)] = &[("amsart", "ÐðĐđ"), ("amsbook", "ÐðĐđ"), ("amsproc", "ÐðĐđ")];
/// babel itself (`babel.def`'s `\ProvideTextCommandDefault`s) makes these
/// available under OT1, whatever the language.
pub const BABEL_ACCEPTS: &str = "«»Đđ‚„‹›";
/// babel languages that add nothing to [`BABEL_ACCEPTS`] (french's own
/// `\DeclareUnicodeCharacter{00AB}`/`{00BB}` included). Greek, Cyrillic,
/// Vietnamese and Hebrew load their own `.dfu` tables and are unknown here.
pub const BABEL_LANGUAGES: &[&str] = &[
    "acadian", "afrikaans", "american", "australian", "austrian", "bahasa", "basque", "brazil", "brazilian", "breton",
    "british", "canadian", "catalan", "croatian", "czech", "danish", "dutch", "english", "esperanto", "estonian",
    "finnish", "french", "friulan", "galician", "german", "hungarian", "icelandic", "indonesian", "interlingua", "irish",
    "italian", "latin", "latvian", "lithuanian", "magyar", "malay", "naustrian", "newzealand", "ngerman", "norsk",
    "nswissgerman", "nynorsk", "occitan", "polish", "portuges", "portuguese", "romanian", "romansh", "samin",
    "scottish", "serbian", "slovak", "slovene", "spanish", "swedish", "swissgerman", "UKenglish", "USenglish", "welsh",
];
/// Standard class options, which babel also receives and ignores.
const CLASS_OPTIONS: &[&str] = &[
    "10pt", "11pt", "12pt", "a4paper", "a5paper", "b5paper", "letterpaper", "legalpaper", "executivepaper", "landscape",
    "portrait", "oneside", "twoside", "onecolumn", "twocolumn", "titlepage", "notitlepage", "openright", "openany",
    "leqno", "fleqn", "draft", "final",
];

impl InputSetup {
    /// The setup for a project: `texts` is every source document as the
    /// pipeline resolved it (the entry at `entry`, its `\input`/`\include`d
    /// files, and any local `.sty`/`.cls` the request carries). `None` when
    /// the project leaves the pdfLaTeX `utf8` + OT1/T1 world this module
    /// models or does something it cannot see through, so that no error is
    /// ever invented: another input encoding, a package, class or babel
    /// language that is neither measured (`packages.json`) nor in the
    /// project, one that declares characters wholesale (`fontspec`,
    /// `CJKutf8`, `textgreek`, babel Greek, ...), text-command or encoding
    /// declarations of its own (`\DeclareTextCommand...`, `\fontencoding`),
    /// or an `\input` whose name is a macro.
    pub fn for_project(texts: &[&str], entry: usize) -> Option<InputSetup> {
        let texts: Vec<String> = texts.iter().map(|t| strip_comments(t)).collect();
        // Prefixes: `\DeclareTextCommandDefault`, `\PassOptionsToClass`, ...
        const OPAQUE: &[&str] = &[
            "\\DeclareText", "\\ProvideTextCommand", "\\UndeclareText", "\\DeclareFontEncoding", "\\fontencoding",
            "\\usefont", "\\inputencoding", "\\UseRawInputEncoding", "\\PassOptionsTo", "\\babelprovide",
        ];
        if texts.iter().any(|t| OPAQUE.iter().any(|c| t.contains(c)) || sets_input_catcodes(t)) {
            return None;
        }
        let mut extra = BTreeSet::new();
        for t in &texts {
            extra.extend(declared(t)?);
            for cmd in ["\\input", "\\include", "\\InputIfFileExists"] {
                for at in commands(t, cmd) {
                    let arg = t[at + cmd.len()..].trim_start();
                    if arg.strip_prefix('{').unwrap_or(arg).trim_start().starts_with('\\') {
                        return None;
                    }
                }
            }
        }
        let provides = |kind: &str, name: &str| texts.iter().any(|t| t.contains(&format!("\\Provides{kind}{{{name}}}")));
        let mut ot1_accepts = String::new();
        let mut loads_t1 = false;
        // The class: the entry's `\documentclass` and any `\LoadClass`.
        let class = texts.get(entry).and_then(|t| loads(t, "\\documentclass")).and_then(|l| l.into_iter().next());
        let class_options = class.as_ref().map(|(o, _)| o.clone()).unwrap_or_default();
        let mut classes: Vec<_> = class.clone().into_iter().collect();
        for t in &texts {
            classes.extend(loads(t, "\\LoadClass")?);
        }
        for (_, names) in classes {
            for name in names {
                if let Some((_, chars)) = ACCEPTING_CLASSES.iter().find(|(n, _)| *n == name) {
                    ot1_accepts.push_str(chars);
                } else if T1_CLASSES.contains(&name.as_str()) {
                    loads_t1 = true;
                } else if !NEUTRAL_CLASSES.contains(&name.as_str()) && !provides("Class", &name) {
                    return None;
                }
            }
        }
        let mut fontenc_last = None;
        let mut babel = false;
        for t in &texts {
            let usepackage = loads(t, "\\usepackage")?;
            let require = loads(t, "\\RequirePackage")?;
            for (options, names) in usepackage.into_iter().chain(require) {
                for name in names {
                    match name.as_str() {
                        "inputenc" => {
                            if options.iter().any(|o| o != "utf8") {
                                return None;
                            }
                        }
                        "fontenc" => {
                            // `LGR`, `T2A`, ... declare their own characters.
                            if options.iter().any(|o| !matches!(o.as_str(), "OT1" | "T1" | "TS1")) {
                                return None;
                            }
                            fontenc_last = Some(options.last().cloned().unwrap_or_default());
                        }
                        "babel" => {
                            babel = true;
                            ot1_accepts.push_str(BABEL_ACCEPTS);
                            for o in &options {
                                let language = o.strip_prefix("main=").unwrap_or(o);
                                if !BABEL_LANGUAGES.contains(&language) {
                                    return None;
                                }
                            }
                        }
                        _ => {
                            if let Some((_, chars)) = ACCEPTING_PACKAGES.iter().find(|(n, _)| *n == name) {
                                ot1_accepts.push_str(chars);
                            } else if T1_PACKAGES.contains(&name.as_str()) {
                                loads_t1 = true;
                            } else if !NEUTRAL_PACKAGES.contains(&name.as_str()) && !provides("Package", &name) {
                                return None;
                            }
                        }
                    }
                }
            }
        }
        // babel also reads the class options: a language there, or a
        // standard option it ignores.
        if babel && class_options.iter().any(|o| !BABEL_LANGUAGES.contains(&o.as_str()) && !CLASS_OPTIONS.contains(&o.as_str())) {
            return None;
        }
        // fontenc's last option is the default encoding (`[T1,OT1]` is OT1).
        let encoding = match (fontenc_last.as_deref(), loads_t1) {
            (None | Some("" | "OT1"), false) => Encoding::OT1,
            (None, true) | (Some("T1"), _) => Encoding::T1,
            _ => return None,
        };
        if encoding == Encoding::OT1 {
            extra.extend(ot1_accepts.chars());
        }
        Some(InputSetup { encoding, extra })
    }

    pub fn rejected(&self, c: char) -> Option<Rejected> {
        if self.extra.contains(&c) {
            return None;
        }
        rejected(c, self.encoding)
    }

    pub fn cuts_ligkern(&self, c: char) -> bool {
        !self.extra.contains(&c) && cuts_ligkern(c, self.encoding)
    }
}

/// `text` with every `%` comment removed (a `\%` is kept).
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let mut end = line.len();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 1,
                b'%' => {
                    end = i;
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        out.push_str(&line[..end]);
        if end < line.len() && line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Byte offsets of every `cmd` (a control word) in `text`.
fn commands<'t>(text: &'t str, cmd: &'t str) -> impl Iterator<Item = usize> + 't {
    text.match_indices(cmd)
        .map(|(at, _)| at)
        .filter(move |at| !text[at + cmd.len()..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '@'))
}

/// A `\catcode` of a non-ASCII byte (`\catcode`\é`, `\catcode"C3`,
/// `\catcode195`), or of a byte a macro names (`\catcode\count@`): the
/// characters may no longer be active.
fn sets_input_catcodes(text: &str) -> bool {
    commands(text, "\\catcode").any(|at| {
        let rest = text[at + "\\catcode".len()..].trim_start();
        let number = |digits: &str, radix| {
            let n: String = digits.chars().take_while(|c| c.is_digit(radix)).collect();
            u32::from_str_radix(&n, radix).is_ok_and(|n| n >= 0x80)
        };
        match rest.chars().next() {
            Some('`') => rest[1..].trim_start_matches('\\').starts_with(|c: char| !c.is_ascii()),
            Some('"') => number(&rest[1..], 16),
            Some('\'') => number(&rest[1..], 8),
            Some('\\') => true,
            Some(c) if c.is_ascii_digit() => number(rest, 10),
            _ => false,
        }
    })
}

/// Every `cmd[<options>]{<names>}` in `text`, options and names split at
/// commas; `None` when one is not in that form (`\usepackage\foo`).
fn loads(text: &str, cmd: &str) -> Option<Vec<(Vec<String>, Vec<String>)>> {
    let split = |s: &str| s.split(',').map(|x| x.split_whitespace().collect::<String>()).filter(|x| !x.is_empty()).collect::<Vec<_>>();
    let mut out = Vec::new();
    // `\LoadClassWithOptions`, `\RequirePackageWithOptions`.
    let with = format!("{cmd}WithOptions");
    for (at, len) in commands(text, cmd).map(|at| (at, cmd.len())).chain(commands(text, &with).map(|at| (at, with.len()))) {
        let mut rest = text[at + len..].trim_start();
        let mut options = Vec::new();
        if let Some(inner) = rest.strip_prefix('[') {
            let end = inner.find(']')?;
            options = split(&inner[..end]);
            rest = inner[end + 1..].trim_start();
        }
        let arg = rest.strip_prefix('{')?;
        let end = arg.find('}')?;
        out.push((options, split(&arg[..end])));
    }
    Some(out)
}

/// `\DeclareUnicodeCharacter{<hex>}{...}` and `\newunicodechar{<char>}{...}`;
/// `None` when an argument is not a literal (`\DeclareUnicodeCharacter{#1}`).
fn declared(source: &str) -> Option<BTreeSet<char>> {
    let mut out = BTreeSet::new();
    for (cmd, hex) in [("\\DeclareUnicodeCharacter", true), ("\\newunicodechar", false)] {
        for at in commands(source, cmd) {
            let rest = &source[at + cmd.len()..];
            let arg = rest.trim_start().strip_prefix('{')?;
            let end = arg.find('}')?;
            let arg = arg[..end].trim();
            let c = if hex {
                u32::from_str_radix(arg, 16).ok().and_then(char::from_u32)
            } else {
                let mut chars = arg.chars();
                chars.next().filter(|_| chars.next().is_none())
            };
            out.insert(c?);
        }
    }
    Some(out)
}

/// A dfu expansion's text command and its argument: `\@tabacckludge'e` →
/// (`\'`, `e`), `\k a` → (`\k`, `a`), `\k{}` → (`\k`, ``), `\th` → (`\th`, ``).
fn command_of(expansion: &str) -> Option<(String, String)> {
    let body = expansion.strip_prefix('\\')?;
    let (cmd, rest) = if let Some(after) = body.strip_prefix("@tabacckludge") {
        let accent = after.chars().next()?;
        (format!("\\{accent}"), &after[accent.len_utf8()..])
    } else {
        let letters = body.bytes().take_while(u8::is_ascii_alphabetic).count();
        if letters > 0 {
            (format!("\\{}", &body[..letters]), &body[letters..])
        } else {
            let c = body.chars().next()?;
            (format!("\\{c}"), &body[c.len_utf8()..])
        }
    };
    let arg: String = rest.chars().filter(|c| !matches!(c, ' ' | '{' | '}')).collect();
    Some((cmd, arg))
}

/// Whether `cmd` is an NFSS text command at all (declared for some encoding
/// or given a kernel default). `\nobreakspace`, `\-` and `\textellipsis`'s
/// relatives that are plain macros or primitives are not, and are never
/// "unavailable".
fn is_text_command(cmd: &str) -> bool {
    [Encoding::OT1, Encoding::T1, Encoding::TS1, Encoding::OMS, Encoding::OML]
        .iter()
        .any(|e| encoding::declared(*e, cmd).is_some())
        || encoding::kernel_default(cmd).is_some()
}

/// The error pdfLaTeX raises for input character `c` under text encoding
/// `enc`, or `None` when it typesets it.
pub fn rejected(c: char, enc: Encoding) -> Option<Rejected> {
    if c.is_ascii() {
        return None;
    }
    let Some((expansion, _)) = unicode::lookup_declared(c) else {
        return Some(Rejected { message: unicode::undeclared_message(c), keep: None });
    };
    let (cmd, arg) = command_of(expansion)?;
    if !is_text_command(&cmd) || encoding::resolve(enc, &cmd) != Resolution::Unavailable {
        return None;
    }
    let keep = {
        let mut chars = arg.chars();
        chars.next().filter(|b| b.is_ascii_alphabetic() && chars.next().is_none())
    };
    Some(Rejected { message: encoding::unavailable_message(enc, &cmd), keep })
}

/// Whether input character `c` stands outside the text font's ligature/kern
/// program in `enc`: it is typeset by `\accent` (no composite slot), as a box
/// (`\L` in OT1, `\r A`), or in another encoding's font (a TS1/OMS symbol).
/// A character that is a slot of `enc` (`ß`, `æ` in OT1; `ü` in T1) is not.
pub fn cuts_ligkern(c: char, enc: Encoding) -> bool {
    if c.is_ascii() {
        return false;
    }
    let Some((expansion, _)) = unicode::lookup_declared(c) else {
        return false;
    };
    let Some((cmd, arg)) = command_of(expansion) else {
        return false;
    };
    let composite_slot = || matches!(encoding::composite(enc, &cmd, &arg), Some(Composite::Slot(_)));
    match encoding::resolve(enc, &cmd) {
        Resolution::Declared(Declared::Symbol(_)) => false,
        Resolution::Declared(Declared::Accent(_) | Declared::Command { .. }) | Resolution::Default(Default::Accent(_)) => {
            !composite_slot()
        }
        Resolution::Default(Default::Symbol(other)) => other != enc,
        // Kernel macros (`\textellipsis` is `.\kern\fontdimen3\font` x3):
        // their first and last tokens are characters of the text font.
        Resolution::Default(Default::Command(_)) => false,
        Resolution::Unavailable => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansions_split_into_command_and_argument() {
        assert_eq!(command_of("\\@tabacckludge'e"), Some(("\\'".into(), "e".into())));
        assert_eq!(command_of("\\k a"), Some(("\\k".into(), "a".into())));
        assert_eq!(command_of("\\k{}"), Some(("\\k".into(), "".into())));
        assert_eq!(command_of("\\r A"), Some(("\\r".into(), "A".into())));
        assert_eq!(command_of("\\guillemetleft"), Some(("\\guillemetleft".into(), "".into())));
        assert_eq!(command_of("\\-"), Some(("\\-".into(), "".into())));
    }

    #[test]
    fn ot1_accents_cut_and_t1_composites_do_not() {
        assert!(cuts_ligkern('ü', Encoding::OT1));
        assert!(!cuts_ligkern('ü', Encoding::T1));
        assert!(cuts_ligkern('Ł', Encoding::OT1));
        assert!(!cuts_ligkern('Ł', Encoding::T1));
        assert!(!cuts_ligkern('ß', Encoding::OT1));
        assert!(!cuts_ligkern('æ', Encoding::OT1));
        assert!(cuts_ligkern('°', Encoding::T1));
        assert!(!cuts_ligkern('…', Encoding::OT1));
        assert!(!cuts_ligkern('“', Encoding::OT1));
    }

    #[test]
    fn preamble_declarations_are_read() {
        let s = "\\DeclareUnicodeCharacter{03B1}{\\alpha}\n\\newunicodechar{β}{\\beta}";
        let setup = InputSetup::for_project(&[s], 0).unwrap();
        assert!(setup.rejected('α').is_none() && setup.rejected('β').is_none());
        assert!(setup.rejected('γ').is_some());
        let one = |s: &str| InputSetup::for_project(&[s], 0);
        assert!(one("\\usepackage[LGR,T1]{fontenc}").is_none());
        assert!(one("\\usepackage[latin1]{inputenc}").is_none());
        assert!(one("\\usepackage[utf8]{inputenc}").is_some());
    }

    /// Declarations and package loads count in every document of the
    /// project, not only the entry.
    #[test]
    fn the_whole_project_is_read() {
        let main = "\\documentclass{article}\n\\usepackage[utf8]{inputenc}\n\\input{macros}\n\\usepackage{mystyle}";
        let macros = "\\DeclareUnicodeCharacter{2603}{X}";
        let sty = "\\ProvidesPackage{mystyle}\n\\newunicodechar{☄}{Y}\n\\RequirePackage{amsmath}";
        let setup = InputSetup::for_project(&[main, macros, sty], 0).unwrap();
        assert!(setup.rejected('☃').is_none() && setup.rejected('☄').is_none());
        assert!(setup.rejected('α').is_some());
        // `mystyle.sty` is not in the project: it may declare anything.
        assert!(InputSetup::for_project(&[main, macros], 0).is_none());
        // `[T1]{fontenc}` in an included preamble file sets the encoding.
        let t1 = InputSetup::for_project(&["\\documentclass{article}\\input{pre}", "\\usepackage[T1]{fontenc}"], 0).unwrap();
        assert_eq!(t1.encoding, Encoding::T1);
        // A package that loads the unknown one fails open too.
        assert!(InputSetup::for_project(&["\\input{pre}", "\\RequirePackage{fontspec}"], 0).is_none());
    }

    #[test]
    fn unknown_or_opaque_setups_fail_open() {
        let one = |s: &str| InputSetup::for_project(&[s], 0);
        assert!(one("\\documentclass{article}\\usepackage{amsmath,graphicx}").is_some());
        assert!(one("\\documentclass{mythesis}").is_none());
        assert!(one("\\documentclass{article}\\usepackage{somethingnew}").is_none());
        assert!(one("\\documentclass{article}% \\usepackage{somethingnew}").is_some());
        assert!(one("\\DeclareUnicodeCharacter{#1}{x}").is_none());
        assert!(one("\\DeclareTextCommandDefault{\\guillemetleft}{<}").is_none());
        assert!(one("\\input{\\jobname-extra}").is_none());
        assert!(one("\\catcode`\\é=12").is_none());
        assert!(one("\\catcode`\\@=11").is_some());
        assert!(one("\\usepackage[T1,TS1]{fontenc}").is_none());
        assert_eq!(one("\\usepackage[T1,OT1]{fontenc}").unwrap().encoding, Encoding::OT1);
    }

    /// babel makes `«`, `»` and the other `babel.def` defaults available
    /// under OT1 for every measured language; an unmeasured one fails open.
    #[test]
    fn babel_languages() {
        let one = |s: &str| InputSetup::for_project(&[s], 0);
        let french = one("\\documentclass{article}\\usepackage[french]{babel}").unwrap();
        assert!(french.rejected('«').is_none() && french.rejected('»').is_none());
        assert!(french.rejected('þ').is_some());
        assert!(one("\\documentclass[11pt,french]{article}\\usepackage{babel}").is_some());
        assert!(one("\\documentclass{article}\\usepackage[main=ngerman,english]{babel}").is_some());
        assert!(one("\\documentclass{article}\\usepackage[greek]{babel}").is_none());
        assert!(one("\\documentclass{article}\\usepackage[klingon]{babel}").is_none());
        assert!(one("\\documentclass[journal]{article}\\usepackage[english]{babel}").is_none());
        let plain = one("\\documentclass{article}").unwrap();
        assert!(plain.rejected('«').is_some());
    }
}
