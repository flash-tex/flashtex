//! natbib citations: `\citet`, `\citep`, `\citealt`, `\citealp`,
//! `\citeauthor`, `\citeyear`, `\citeyearpar`, `\citenum`, `\citetext` and the
//! `\cite` natbib redefines, plus the `\bibitem[Author(Year)]{key}` labels
//! that make author-year citations possible.
//!
//! Written against `natbib.sty` 2010/09/13 8.31b as shipped by TeX Live 2025
//! and checked form by form against pdfLaTeX (see `tests/natbib.rs`, whose
//! expectations are the strings pdfLaTeX actually sets). The line references
//! below are to that natbib.sty.
//!
//! The three switches natbib itself uses decide every shape:
//!
//! * `swa` ("switch a", `\ifNAT@swa`) — the whole list is wrapped by
//!   `\NAT@cite` (line 353): delimiters, pre-note before the list, post-note
//!   after it. `\citep`-like. When it is false each *entry* carries its own
//!   parentheses and the post-note is appended by `\NAT@citex`'s own tail
//!   (line 594). `\citet`-like.
//! * `par` (`\ifNAT@par`) — whether `\NAT@@open`/`\NAT@@close` expand to the
//!   delimiters at all (lines 604-605). `\citealt`/`\citealp` turn it off,
//!   which is the only difference between them and `\citet`/`\citep`.
//! * `ctype` — 0 name+year, 1 name only (`\citeauthor`), 2 year only
//!   (`\citeyear`), 3 alias (`\citetalias`, not implemented).
//!
//! The classic trap this module exists to get right: **one optional argument
//! is the post-note, not the pre-note.** `\NAT@citetp` (line 688) reads
//! `[#1]` and then, if a second bracket follows, calls `\@citex[#1][#2]`;
//! otherwise `\@citex[][#1]`. So `\citep[p.~7]{k}` is
//! "(Name, Year, p. 7)" and only `\citep[see][p.~7]{k}` puts "see" first.

use crate::diagnostics::Diagnostic;
use crate::parser::{Inline, TextStyle};
use crate::Span;

/// `\NAT@open`, `\NAT@close`, `\NAT@sep`, ... after the package options have
/// been processed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// `\ifNAT@numbers`: citations are `\bibitem` numbers, not author-year.
    pub numbers: bool,
    /// `\ifNAT@super`: numeric citations set as superscripts. Parsed and
    /// reported; this renderer sets them on the baseline.
    pub superscript: bool,
    /// `\NAT@open` / `\NAT@close` (line 348).
    pub open: String,
    pub close: String,
    /// `\NAT@sep` between two citations in one call (line 349).
    pub sep: String,
    /// `\NAT@aysep` between author and year in a `\citep` (line 351).
    pub aysep: String,
    /// `\NAT@yrsep` between two years of the same author (line 351).
    pub yrsep: String,
    /// `\NAT@cmt` before the post-note (line 352).
    pub cmt: String,
    /// `sort` / `compress`: parsed, not applied (reported by the caller).
    pub sort: bool,
    pub compress: bool,
    /// `longnamesfirst`: the first citation of an entry uses its long author
    /// list (`\ifNAT@longnames`, line 519).
    pub longnamesfirst: bool,
}

impl Default for Options {
    /// natbib.sty lines 235, 348-352: author-year, round, semicolon.
    fn default() -> Self {
        Options {
            numbers: false,
            superscript: false,
            open: "(".into(),
            close: ")".into(),
            sep: ";".into(),
            aysep: ",".into(),
            yrsep: ",".into(),
            cmt: ", ".into(),
            sort: false,
            compress: false,
            longnamesfirst: false,
        }
    }
}

/// Every option `\DeclareOption`d by natbib, **in declaration order**, which
/// is the order `\ProcessOptions` (line 350, not the starred form) executes
/// them in regardless of the order the document wrote them. That is why
/// `[numbers,round]` is round and `[round,numbers]` is square: `numbers`
/// is declared first and re-executes `square,comma`.
const DECLARED: &[&str] = &[
    "numbers",
    "super",
    "authoryear",
    "round",
    "square",
    "angle",
    "curly",
    "comma",
    "semicolon",
    "colon",
    "nobibstyle",
    "bibstyle",
    "openbib",
    "sectionbib",
    "sort",
    "compress",
    "sort&compress",
    "mcite",
    "merge",
    "elide",
    "longnamesfirst",
    "nonamebreak",
];

/// The options whose full effect this implementation reproduces. Anything
/// else keeps `\usepackage{natbib}`'s "recognised but not implemented"
/// warning (`parser::package_matches_layout`) so a document that depends on
/// it is not silently mis-set.
pub const IMPLEMENTED_OPTIONS: &[&str] = &[
    "numbers",
    "authoryear",
    "round",
    "square",
    "angle",
    "curly",
    "comma",
    "semicolon",
    "colon",
    "nobibstyle",
    "bibstyle",
    "sectionbib",
    "longnamesfirst",
    "nonamebreak",
];

impl Options {
    /// Applies a `\usepackage[...]{natbib}` option list. Unknown options are
    /// ignored here; `parser::package_matches_layout` is what decides whether
    /// the document still gets the package warning.
    pub fn from_option_list(raw: &str) -> Options {
        let given: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|option| !option.is_empty())
            .collect();
        let mut options = Options::default();
        for declared in DECLARED {
            if given.iter().any(|option| option == declared) {
                options.apply(declared);
            }
        }
        options
    }

    fn apply(&mut self, option: &str) {
        match option {
            // Line 238: `\ExecuteOptions{square,comma,nobibstyle}`.
            "numbers" => {
                self.numbers = true;
                self.apply("square");
                self.apply("comma");
            }
            // Line 240: numeric superscripts, no delimiters.
            "super" => {
                self.superscript = true;
                self.numbers = true;
                self.open.clear();
                self.close.clear();
            }
            // Line 243: `\ExecuteOptions{round,semicolon,bibstyle}`.
            "authoryear" => {
                self.numbers = false;
                self.apply("round");
                self.apply("semicolon");
            }
            "round" => {
                self.open = "(".into();
                self.close = ")".into();
            }
            "square" => {
                self.open = "[".into();
                self.close = "]".into();
            }
            // natbib sets these in math mode (`$<$`); the glyphs are the
            // same, the italic correction around them is not modelled.
            "angle" => {
                self.open = "<".into();
                self.close = ">".into();
            }
            "curly" => {
                self.open = "{".into();
                self.close = "}".into();
            }
            "comma" => self.sep = ",".into(),
            // Line 260: `colon` is a synonym for `semicolon`.
            "semicolon" | "colon" => self.sep = ";".into(),
            "sort" => self.sort = true,
            "compress" => self.compress = true,
            "sort&compress" => {
                self.sort = true;
                self.compress = true;
            }
            "longnamesfirst" => self.longnamesfirst = true,
            // `nobibstyle`/`bibstyle`/`sectionbib`/`nonamebreak`/`openbib`
            // change the .bst hook, the bibliography heading level, an
            // unbreakable name box or the bibliography's own paragraph
            // shape — none of which changes a citation's characters.
            _ => {}
        }
    }
}

/// One `\cite`-family command, as the three natbib switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind {
    /// `\ifNAT@swa`: the delimiters and notes wrap the whole list.
    pub swa: bool,
    /// `\ifNAT@par`: the delimiters are typeset at all.
    pub par: bool,
    /// 0 name+year, 1 name, 2 year.
    pub ctype: u8,
    /// The starred form: use the entry's long author list.
    pub full: bool,
    /// `\Citet`-style: `\NAT@Up` uppercases the author list's first letter.
    pub uppercase: bool,
    /// `\citenum`: always numeric, whatever the package options say.
    pub numeric: bool,
}

/// The natbib command names this module implements, mapped to their switches.
/// `\cite` itself is absent: natbib's `\NAT@cites` (line 696) decides `swa`
/// from whether an optional argument follows, so the parser sets it.
pub fn kind(name: &str) -> Option<Kind> {
    let (base, full) = match name.strip_suffix('*') {
        Some(base) => (base, true),
        None => (name, false),
    };
    let (base, uppercase) = match base.strip_prefix("Cite") {
        Some(rest) => (format!("cite{rest}"), true),
        None => (base.to_string(), false),
    };
    let base = base.as_str();
    // natbib's `\Cite...` forms exist only for these four plus \Citeauthor.
    if uppercase
        && !matches!(
            base,
            "citet" | "citep" | "citealt" | "citealp" | "citeauthor"
        )
    {
        return None;
    }
    let (swa, par, ctype, numeric) = match base {
        "citet" => (false, true, 0, false),
        "citep" => (true, true, 0, false),
        "citealt" => (false, false, 0, false),
        "citealp" => (true, false, 0, false),
        "citeauthor" => (false, false, 1, false),
        "citefullauthor" => (false, false, 1, false),
        "citeyear" => (false, false, 2, false),
        "citeyearpar" => (true, true, 2, false),
        "citenum" => (true, false, 0, true),
        _ => return None,
    };
    // `\citefullauthor` is `\citeauthor*` (line 744).
    let full = full || base == "citefullauthor";
    // `\citeyear`/`\citeyearpar`/`\citenum` take no star.
    if full && matches!(base, "citeyear" | "citeyearpar" | "citenum") {
        return None;
    }
    Some(Kind {
        swa,
        par,
        ctype,
        full,
        uppercase,
        numeric,
    })
}

/// `\citep`'s own switches, for the `\cite[note]{...}` form natbib routes
/// through `\NAT@@citetp` with `\NAT@swa` still true (line 696).
pub const CITE_WITH_NOTE: Kind = Kind {
    swa: true,
    par: true,
    ctype: 0,
    full: false,
    uppercase: false,
    numeric: false,
};

/// `\citet`'s switches: `\cite{keys}` with no optional argument in
/// author-year mode clears `\NAT@swa` (line 697).
pub const CITE_PLAIN: Kind = Kind {
    swa: false,
    par: true,
    ctype: 0,
    full: false,
    uppercase: false,
    numeric: false,
};

/// What a `\bibitem`'s optional argument resolved to, as natbib's
/// `\bibcite{key}{{num}{date}{{name}}{{all names}}}` records it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    /// `\NAT@num`: the `\c@NAT@ctr` value, or the label itself when the
    /// entry is an apalike-style `[Label]` with no year.
    pub num: String,
    /// `\NAT@date`: the year, possibly with a disambiguating letter.
    pub date: String,
    /// `\NAT@name`: the short author list.
    pub name: String,
    /// `\NAT@all@names`: the long author list, defaulting to the short one
    /// (`\NAT@split`, line 786).
    pub all_names: String,
}

impl Entry {
    /// `\NAT@year`/`\NAT@exlab` (`\NAT@parse@date`, line 791): the year is the
    /// digits before the first letter in the first four characters, and the
    /// extra label is that letter — or the fifth character (a `?` from
    /// natbib's own padding) when all four are digits.
    pub fn year_and_extra(&self) -> (String, String) {
        let chars: Vec<char> = self.date.chars().collect();
        for take in 0..4.min(chars.len()) {
            if chars[take].is_alphabetic() {
                return (chars[..take].iter().collect(), chars[take].to_string());
            }
        }
        let year: String = chars.iter().take(4).collect();
        let extra = chars.get(4).map_or("?".to_string(), |c| c.to_string());
        (year, extra)
    }
}

/// The result of reading one `\bibitem[...]` optional argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// `[Author(Year)]` or `[Short(Year)Long]` — `\NAT@bare` (line 812).
    AuthorYear(Entry),
    /// `[Author, Year]` — `\NAT@apalk` (line 1005).
    Apalike(Entry),
    /// A label with neither form, or no optional argument at all. natbib sets
    /// `\NAT@stdbst` for these and `\NAT@force@numbers` (line 974) makes the
    /// *next* run numeric for the whole document.
    Standard { num: String },
}

/// Parses one `\bibitem` optional argument the way `\@lbibitem` does.
/// `counter` is `\c@NAT@ctr` after this entry advanced it — natbib advances
/// for every entry, labelled or not (line 856), unlike the LaTeX kernel's
/// `\@lbibitem`.
pub fn parse_label(raw: Option<&str>, counter: usize) -> Label {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        // `\bibitem{key}`: `\NAT@wrout{\the\c@NAT@ctr}{}{}{}` plus
        // `\global\NAT@stdbsttrue` (line 862).
        return Label::Standard {
            num: counter.to_string(),
        };
    };
    if let Some((short, rest)) = raw.split_once('(') {
        if let Some((date, long)) = rest.split_once(')') {
            let short = short.trim().to_string();
            let long = long.trim();
            return Label::AuthorYear(Entry {
                num: counter.to_string(),
                date: date.trim().to_string(),
                all_names: if long.is_empty() {
                    short.clone()
                } else {
                    long.to_string()
                },
                name: short,
            });
        }
    }
    // `\NAT@apalk#1, #2, #3\@nil`: the delimiter is a comma *and a space*.
    if let Some((name, date)) = raw.split_once(", ") {
        let date = date.trim();
        if !date.is_empty() {
            return Label::Apalike(Entry {
                num: counter.to_string(),
                date: date.to_string(),
                name: name.trim().to_string(),
                all_names: name.trim().to_string(),
            });
        }
    }
    Label::Standard {
        num: raw.to_string(),
    }
}

/// A run of citation text and whether it is natbib's bold recovery marker.
type Run = (String, bool);

/// Builds the inlines for one natbib citation.
///
/// `resolve` returns the `\bibcite` record for a key, or `None` for an
/// undefined one (natbib's `{\reset@font\bfseries ?}` plus a warning).
pub fn cite_inlines(
    options: &Options,
    kind: Kind,
    pre: Option<&str>,
    post: Option<&str>,
    keys: &[String],
    resolve: &dyn Fn(&str) -> Option<Entry>,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Inline> {
    let pre = note_text(pre);
    let post = note_text(post);
    let numeric = options.numbers || kind.numeric;
    let body = if numeric {
        numeric_body(
            options,
            kind,
            pre.as_deref(),
            post.as_deref(),
            keys,
            resolve,
            span,
            diags,
        )
    } else {
        author_year_body(
            options,
            kind,
            pre.as_deref(),
            post.as_deref(),
            keys,
            resolve,
            span,
            diags,
        )
    };
    let runs = if kind.swa {
        wrap(options, kind, pre.as_deref(), post.as_deref(), body)
    } else {
        body
    };
    into_inlines(runs, span)
}

/// `\citetext{...}`: the delimiters around arbitrary text (line 741).
pub fn citetext_inlines(options: &Options, text: &str, span: Span) -> Vec<Inline> {
    into_inlines(
        vec![(format!("{}{text}{}", options.open, options.close), false)],
        span,
    )
}

/// `\NAT@cite`/`\NAT@citenum` (lines 353-358): open, pre-note and a space,
/// the list, `\NAT@cmt` and the post-note, close.
fn wrap(
    options: &Options,
    kind: Kind,
    pre: Option<&str>,
    post: Option<&str>,
    body: Vec<Run>,
) -> Vec<Run> {
    let (open, close) = delimiters(options, kind);
    let mut runs = vec![(open.to_string(), false)];
    if let Some(pre) = pre {
        runs.push((format!("{pre} "), false));
    }
    runs.extend(body);
    if let Some(post) = post {
        runs.push((format!("{}{post}", options.cmt), false));
    }
    runs.push((close.to_string(), false));
    runs
}

fn delimiters<'a>(options: &'a Options, kind: Kind) -> (&'a str, &'a str) {
    if kind.par {
        (options.open.as_str(), options.close.as_str())
    } else {
        ("", "")
    }
}

/// `\NAT@citex` (line 507): the author-year citation list.
#[allow(clippy::too_many_arguments)]
fn author_year_body(
    options: &Options,
    kind: Kind,
    pre: Option<&str>,
    post: Option<&str>,
    keys: &[String],
    resolve: &dyn Fn(&str) -> Option<Entry>,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Run> {
    let (open, close) = delimiters(options, kind);
    let mut runs: Vec<Run> = Vec::new();
    // `\@citea`, empty until the first entry has been set (line 602).
    let mut citea = String::new();
    // `\NAT@nm` / `\NAT@year` of the *previous* entry (line 521).
    let mut last_name: Option<String> = None;
    let mut last_year: Option<String> = None;
    // `\NAT@date` as the loop leaves it: an undefined final key clears it and
    // so suppresses `\citet`'s closing delimiter (line 594).
    let mut last_date = String::new();
    for key in keys {
        let Some(entry) = resolve(key) else {
            runs.push((citea.clone(), false));
            runs.push(("?".into(), true));
            last_date.clear();
            diags.push(undefined(key, span));
            continue;
        };
        let previous_name = last_name.clone();
        let previous_year = last_year.clone();
        let name = author(&entry, kind);
        let (year, extra) = entry.year_and_extra();
        last_name = Some(name.clone());
        last_year = Some(year.clone());
        last_date = entry.date.clone();
        match kind.ctype {
            0 if entry.date.is_empty() => {
                runs.push((format!("{citea}{name}"), false));
            }
            0 if previous_name.as_deref() == Some(name.as_str()) => {
                // Same author as the entry before: `\NAT@yrsep` and then only
                // the year — or only the disambiguating letter when the year
                // repeats too (line 530).
                if previous_year.as_deref() == Some(year.as_str()) {
                    runs.push((format!("{}{extra}", options.yrsep), false));
                } else {
                    runs.push((format!("{} {}", options.yrsep, entry.date), false));
                }
            }
            0 if kind.swa => {
                runs.push((
                    format!("{citea}{name}{} {}", options.aysep, entry.date),
                    false,
                ));
            }
            0 => {
                // `\citet`: each entry carries its own parentheses, and the
                // pre-note goes inside every one of them (line 578).
                let pre = pre.map(|pre| format!("{pre} ")).unwrap_or_default();
                runs.push((format!("{citea}{name} {open}{pre}{}", entry.date), false));
            }
            1 => runs.push((format!("{citea}{name}"), false)),
            _ => runs.push((format!("{citea}{}", entry.date), false)),
        }
        // `\NAT@def@citea` / `\NAT@def@citea@close` (lines 599-601): the
        // `\citet` branch has to close the parenthesis it opened before the
        // separator, unless this entry had no year to put in one.
        citea = if kind.swa || entry.date.is_empty() {
            format!("{} ", options.sep)
        } else {
            format!("{close}{} ", options.sep)
        };
    }
    if !kind.swa {
        if let Some(post) = post {
            runs.push((format!("{}{post}", options.cmt), false));
        }
        if !last_date.is_empty() {
            runs.push((close.to_string(), false));
        }
    }
    runs
}

/// `\NAT@citexnum` (line 377) with `\NAT@cmprs`/`\NAT@merge` at zero: the
/// numeric citation list. `sort` and `compress` are parsed but not applied,
/// so the keys stay in the order the document wrote them.
#[allow(clippy::too_many_arguments)]
fn numeric_body(
    options: &Options,
    kind: Kind,
    pre: Option<&str>,
    post: Option<&str>,
    keys: &[String],
    resolve: &dyn Fn(&str) -> Option<Entry>,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Run> {
    let (open, close) = delimiters(options, kind);
    let mut runs: Vec<Run> = Vec::new();
    let mut citea = String::new();
    let mut last_name: Option<String> = None;
    for key in keys {
        let Some(entry) = resolve(key) else {
            // Unlike `\NAT@citex`, the numeric loop emits no `\@citea` before
            // its recovery marker (line 384).
            runs.push(("?".into(), true));
            diags.push(undefined(key, span));
            continue;
        };
        let previous_name = last_name.clone();
        let name = author(&entry, kind);
        last_name = Some(name.clone());
        if kind.swa {
            // `\citep`/`\cite`/`\citenum`: the bare number, or the year for
            // `\citeyearpar`, which natbib routes here too under `numbers`.
            if kind.ctype > 1 {
                runs.push((citea.clone(), false));
                runs.push(year_or_marker(&entry, key, span, diags));
            } else {
                runs.push((format!("{citea}{}", entry.num), false));
            }
            citea = format!("{} ", options.sep);
            continue;
        }
        match kind.ctype {
            // `\citet`: "Name [n]", collapsing to ", n" when the author
            // repeats (line 551).
            0 => {
                if previous_name.as_deref() == Some(name.as_str()) {
                    runs.push((format!("{} ", options.yrsep), false));
                } else {
                    runs.push((citea.clone(), false));
                    runs.push(name_or_marker(&name, key, span, diags));
                    runs.push((format!(" {open}"), false));
                }
                if let Some(pre) = pre {
                    runs.push((format!("{pre} "), false));
                }
                runs.push((entry.num.clone(), false));
                citea = format!("{close}{} ", options.sep);
            }
            1 => {
                runs.push((citea.clone(), false));
                runs.push(name_or_marker(&name, key, span, diags));
                citea = format!("{} ", options.sep);
            }
            _ => {
                runs.push((citea.clone(), false));
                runs.push(year_or_marker(&entry, key, span, diags));
                citea = format!("{} ", options.sep);
            }
        }
    }
    if !kind.swa {
        if kind.ctype == 0 {
            if let Some(post) = post {
                runs.push((format!("{}{post}", options.cmt), false));
            }
        }
        runs.push((close.to_string(), false));
    }
    runs
}

/// `\NAT@test{\@ne}` (line 486): the author list, or the bold `(author?)`
/// recovery when the entry has none — which is what every plain
/// `\bibitem{key}` gives a `\citet` in numeric mode.
fn name_or_marker(name: &str, key: &str, span: Span, diags: &mut Vec<Diagnostic>) -> Run {
    if !name.is_empty() {
        return (name.to_string(), false);
    }
    diags.push(Diagnostic::warning(
        format!("Package natbib Warning: Author undefined for citation `{key}'"),
        Some(span),
        Some("rendered natbib's (author?) marker".into()),
    ));
    ("(author?)".into(), true)
}

/// `\NAT@test{\tw@}` (line 495): the year, or the bold `(year?)` recovery.
fn year_or_marker(entry: &Entry, key: &str, span: Span, diags: &mut Vec<Diagnostic>) -> Run {
    if !entry.date.is_empty() {
        return (entry.date.clone(), false);
    }
    diags.push(Diagnostic::warning(
        format!("Package natbib Warning: Year undefined for citation `{key}'"),
        Some(span),
        Some("rendered natbib's (year?) marker".into()),
    ));
    ("(year?)".into(), true)
}

/// `\ifNAT@full\let\NAT@nm\NAT@all@names\else\let\NAT@nm\NAT@name\fi`
/// (line 527), then `\NAT@nmfmt` / `\NAT@Up` (lines 287, 612).
fn author(entry: &Entry, kind: Kind) -> String {
    let name = if kind.full {
        entry.all_names.clone()
    } else {
        entry.name.clone()
    };
    if !kind.uppercase {
        return name;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => name,
    }
}

fn undefined(key: &str, span: Span) -> Diagnostic {
    Diagnostic::warning(
        format!("citation '{key}' is undefined"),
        Some(span),
        Some("rendered '?' in place of the undefined citation".into()),
    )
}

/// A `[...]` note as it is set: `~` is TeX's tie, an ordinary interword space
/// that does not break. This layout never breaks inside a note, so a plain
/// space renders it faithfully. An empty `[]` is `\if*#1*` — no note at all.
fn note_text(raw: Option<&str>) -> Option<String> {
    raw.map(|note| note.replace('~', " "))
        .filter(|note| !note.is_empty())
}

/// Merges adjacent runs of the same weight into `Inline::Text`. Only the
/// first run carries `space_before`, exactly as `bib::cite_inlines` does.
fn into_inlines(runs: Vec<Run>, span: Span) -> Vec<Inline> {
    let mut merged: Vec<Run> = Vec::new();
    for (text, bold) in runs {
        if text.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some((previous, previous_bold)) if *previous_bold == bold => previous.push_str(&text),
            _ => merged.push((text, bold)),
        }
    }
    merged
        .into_iter()
        .enumerate()
        .map(|(index, (text, bold))| Inline::Text {
            text,
            span,
            style: if bold {
                TextStyle::BOLD
            } else {
                TextStyle::default()
            },
            space_before: index == 0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, date: &str, all: &str, num: &str) -> Entry {
        Entry {
            num: num.into(),
            date: date.into(),
            name: name.into(),
            all_names: all.into(),
        }
    }

    fn library() -> impl Fn(&str) -> Option<Entry> {
        |key: &str| match key {
            "kp" => Some(entry("Knuth and Plass", "1981", "Knuth and Plass", "1")),
            "plass81" => Some(entry("Plass", "1981", "Plass", "2")),
            "plass90" => Some(entry("Plass", "1990", "Plass", "3")),
            "hobby" => Some(entry("Hobby", "1986", "Hobby", "4")),
            "frank" => Some(entry("Frank", "1990", "Frank", "5")),
            "jones" => Some(entry(
                "Jones et al.",
                "1990",
                "Jones, Baker, and Williams",
                "6",
            )),
            _ => None,
        }
    }

    fn set(command: &str, pre: Option<&str>, post: Option<&str>, keys: &[&str]) -> String {
        set_with(&Options::default(), command, pre, post, keys).0
    }

    fn set_with(
        options: &Options,
        command: &str,
        pre: Option<&str>,
        post: Option<&str>,
        keys: &[&str],
    ) -> (String, Vec<Diagnostic>) {
        let kind = kind(command).unwrap_or_else(|| panic!("unknown command {command}"));
        let keys: Vec<String> = keys.iter().map(|key| (*key).to_string()).collect();
        let mut diags = Vec::new();
        let inlines = cite_inlines(
            options,
            kind,
            pre,
            post,
            &keys,
            &library(),
            Span::new(0, 0),
            &mut diags,
        );
        let text = inlines
            .iter()
            .map(|inline| match inline {
                Inline::Text { text, .. } => text.clone(),
                _ => String::new(),
            })
            .collect();
        (text, diags)
    }

    // Every expectation below is what pdfLaTeX (TeX Live 2025, natbib 8.31b)
    // actually sets; see tests/natbib.rs for the same strings measured as box
    // widths against `\showthe\wd`.

    #[test]
    fn citet_and_citep_are_the_two_basic_shapes() {
        assert_eq!(set("citet", None, None, &["kp"]), "Knuth and Plass (1981)");
        assert_eq!(set("citep", None, None, &["kp"]), "(Knuth and Plass, 1981)");
    }

    #[test]
    fn one_optional_argument_is_the_post_note_not_the_pre_note() {
        assert_eq!(
            set("citep", None, Some("p.~7"), &["kp"]),
            "(Knuth and Plass, 1981, p. 7)"
        );
        assert_eq!(
            set("citep", Some("see"), Some("p.~7"), &["kp"]),
            "(see Knuth and Plass, 1981, p. 7)"
        );
        assert_eq!(
            set("citep", Some("see"), None, &["kp"]),
            "(see Knuth and Plass, 1981)"
        );
        assert_eq!(
            set("citet", None, Some("p.~7"), &["kp"]),
            "Knuth and Plass (1981, p. 7)"
        );
        assert_eq!(
            set("citet", Some("see"), Some("p.~7"), &["kp"]),
            "Knuth and Plass (see 1981, p. 7)"
        );
    }

    #[test]
    fn citealt_and_citealp_drop_the_delimiters() {
        assert_eq!(set("citealt", None, None, &["kp"]), "Knuth and Plass 1981");
        assert_eq!(
            set("citealt", None, Some("p.~7"), &["kp"]),
            "Knuth and Plass 1981, p. 7"
        );
        assert_eq!(set("citealp", None, None, &["kp"]), "Knuth and Plass, 1981");
        assert_eq!(
            set("citealp", None, Some("p.~7"), &["kp"]),
            "Knuth and Plass, 1981, p. 7"
        );
    }

    #[test]
    fn author_and_year_forms() {
        assert_eq!(set("citeauthor", None, None, &["kp"]), "Knuth and Plass");
        assert_eq!(set("citeyear", None, None, &["kp"]), "1981");
        assert_eq!(set("citeyearpar", None, None, &["kp"]), "(1981)");
        assert_eq!(
            set("citefullauthor", None, None, &["jones"]),
            "Jones, Baker, and Williams"
        );
        assert_eq!(
            set("citeauthor*", None, None, &["jones"]),
            "Jones, Baker, and Williams"
        );
        assert_eq!(set("citeauthor", None, None, &["jones"]), "Jones et al.");
    }

    #[test]
    fn starred_forms_use_the_long_author_list() {
        assert_eq!(set("citet", None, None, &["jones"]), "Jones et al. (1990)");
        assert_eq!(set("citep", None, None, &["jones"]), "(Jones et al., 1990)");
        assert_eq!(
            set("citet*", None, None, &["jones"]),
            "Jones, Baker, and Williams (1990)"
        );
    }

    #[test]
    fn several_keys_use_the_separator_in_given_order() {
        assert_eq!(
            set("citep", None, None, &["plass81", "hobby", "frank"]),
            "(Plass, 1981; Hobby, 1986; Frank, 1990)"
        );
        assert_eq!(
            set("citet", None, None, &["plass81", "hobby"]),
            "Plass (1981); Hobby (1986)"
        );
        assert_eq!(
            set("citep", Some("see"), Some("p.~7"), &["plass81", "hobby"]),
            "(see Plass, 1981; Hobby, 1986, p. 7)"
        );
        assert_eq!(
            set("citeauthor", None, None, &["plass81", "hobby"]),
            "Plass; Hobby"
        );
        assert_eq!(
            set("citeyear", None, None, &["plass81", "hobby"]),
            "1981; 1986"
        );
    }

    #[test]
    fn a_repeated_author_collapses_to_the_bare_year() {
        assert_eq!(
            set("citep", None, None, &["plass81", "plass90"]),
            "(Plass, 1981, 1990)"
        );
        assert_eq!(
            set("citet", None, None, &["plass81", "plass90"]),
            "Plass (1981, 1990)"
        );
    }

    #[test]
    fn a_repeated_author_and_year_collapses_to_the_extra_label() {
        let resolve = |key: &str| match key {
            "a" => Some(entry("Plass", "1981a", "Plass", "1")),
            "b" => Some(entry("Plass", "1981b", "Plass", "2")),
            _ => None,
        };
        let mut diags = Vec::new();
        let inlines = cite_inlines(
            &Options::default(),
            kind("citep").unwrap(),
            None,
            None,
            &["a".into(), "b".into()],
            &resolve,
            Span::new(0, 0),
            &mut diags,
        );
        let text: String = inlines
            .iter()
            .map(|i| match i {
                Inline::Text { text, .. } => text.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(text, "(Plass, 1981a,b)");
    }

    #[test]
    fn an_undefined_key_is_a_bold_question_mark_and_one_warning() {
        let (text, diags) = set_with(&Options::default(), "citep", None, None, &["nope"]);
        assert_eq!(text, "(?)");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("'nope' is undefined"));
    }

    #[test]
    fn cite_takes_citet_s_shape_without_a_note_and_citep_s_with_one() {
        let keys = vec!["kp".to_string()];
        let mut diags = Vec::new();
        let plain = cite_inlines(
            &Options::default(),
            CITE_PLAIN,
            None,
            None,
            &keys,
            &library(),
            Span::new(0, 0),
            &mut diags,
        );
        let noted = cite_inlines(
            &Options::default(),
            CITE_WITH_NOTE,
            None,
            Some("p. 7"),
            &keys,
            &library(),
            Span::new(0, 0),
            &mut diags,
        );
        let text = |inlines: &[Inline]| -> String {
            inlines
                .iter()
                .map(|i| match i {
                    Inline::Text { text, .. } => text.as_str(),
                    _ => "",
                })
                .collect()
        };
        assert_eq!(text(&plain), "Knuth and Plass (1981)");
        assert_eq!(text(&noted), "(Knuth and Plass, 1981, p. 7)");
    }

    #[test]
    fn numbers_option_implies_square_brackets_and_a_comma_separator() {
        let options = Options::from_option_list("numbers");
        assert!(options.numbers);
        assert_eq!((options.open.as_str(), options.close.as_str()), ("[", "]"));
        assert_eq!(options.sep, ",");
        let (text, _) = set_with(&options, "citep", None, None, &["plass81", "hobby"]);
        assert_eq!(text, "[2, 4]");
        let (text, _) = set_with(&options, "citet", None, None, &["plass81"]);
        assert_eq!(text, "Plass [2]");
        let (text, _) = set_with(&options, "citep", None, Some("p.~7"), &["plass81"]);
        assert_eq!(text, "[2, p. 7]");
    }

    #[test]
    fn declaration_order_decides_a_mixed_option_list() {
        // `numbers` is declared before `round`, so it runs first and `round`
        // has the last word on the delimiters; `square` is never reached
        // again by the later option.
        let options = Options::from_option_list("round,numbers");
        assert_eq!((options.open.as_str(), options.close.as_str()), ("(", ")"));
        assert_eq!(options.sep, ",");
        assert!(options.numbers);
        let options = Options::from_option_list("authoryear,round");
        assert_eq!((options.open.as_str(), options.close.as_str()), ("(", ")"));
        assert_eq!(options.sep, ";");
        assert!(!options.numbers);
        let options = Options::from_option_list("square");
        assert_eq!((options.open.as_str(), options.close.as_str()), ("[", "]"));
        assert!(!options.numbers);
    }

    #[test]
    fn bibitem_labels_parse_into_the_bibcite_record() {
        assert_eq!(
            parse_label(Some("Knuth and Plass(1981)"), 1),
            Label::AuthorYear(entry("Knuth and Plass", "1981", "Knuth and Plass", "1"))
        );
        assert_eq!(
            parse_label(Some("Jones et al.(1990)Jones, Baker, and Williams"), 6),
            Label::AuthorYear(entry(
                "Jones et al.",
                "1990",
                "Jones, Baker, and Williams",
                "6"
            ))
        );
        assert_eq!(
            parse_label(Some("Knuth, 1984"), 2),
            Label::Apalike(entry("Knuth", "1984", "Knuth", "2"))
        );
        assert_eq!(
            parse_label(Some("AWK"), 3),
            Label::Standard { num: "AWK".into() }
        );
        assert_eq!(parse_label(None, 4), Label::Standard { num: "4".into() });
    }

    #[test]
    fn the_year_splits_off_a_disambiguating_letter() {
        assert_eq!(
            entry("", "1981", "", "").year_and_extra(),
            ("1981".into(), "?".into())
        );
        assert_eq!(
            entry("", "1981a", "", "").year_and_extra(),
            ("1981".into(), "a".into())
        );
    }

    #[test]
    fn unknown_commands_are_not_natbib_citations() {
        assert!(kind("citep").is_some());
        assert!(kind("Citet").is_some());
        assert!(kind("citeyear*").is_none());
        assert!(kind("Citeyear").is_none());
        assert!(kind("emph").is_none());
    }
}
