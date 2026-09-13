//! Pseudocode packages: the `algorithm` float (`algorithm.sty` v0.1 on
//! `float.sty`), `algorithmic` (`algorithmic.sty` v0.1) and `algpseudocode`
//! on `algorithmicx` (v1.2), read from the exact source bytes.
//!
//! [`setup`] reads what the preamble selects (which package defines the
//! statement commands, their options, keyword renames); [`scan`] finds every
//! `algorithm` float and every bare `algorithmic` environment and turns each
//! statement into a [`Line`]: its list item kind, nesting depth, the line
//! counter and whether its number is printed, and the pieces the package's
//! macro sets (bold keywords, control spaces, source spans for conditions
//! and statement text, `\hfill` and `\(\triangleright\)` for algpseudocode
//! comments). Layout (list geometry, the `ruled` float rules) is left to the
//! consumer; the constants it needs are here with their package lines.
//!
//! Package lines cited below are TeX Live 2026:
//! `tex/latex/algorithms/algorithmic.sty`, `tex/latex/algorithms/algorithm.sty`,
//! `tex/latex/algorithmicx/algorithmicx.sty`, `tex/latex/algorithmicx/algpseudocode.sty`
//! and `tex/latex/float/float.sty`.

use std::collections::HashMap;

use crate::parser::FontSizeLevel;
use crate::{DocumentId, Span};

/// Which package defines the statement commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// `algorithmic.sty`: `\STATE`, `\IF{..}`, `\ENDIF`, nested `ALC@g` lists.
    Algorithmic,
    /// `algpseudocode.sty` on `algorithmicx.sty`: `\State`, `\If{..}`, one
    /// list whose items are indented by `\hskip\ALG@tlm`.
    Algpseudocode,
}

/// `\floatstyle` of the `algorithm` float (`algorithm.sty` options
/// `plain`/`ruled`/`boxed`; `ruled` is the default, line 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatStyle {
    Plain,
    Ruled,
    Boxed,
}

/// The face a fixed piece of text is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Roman,
    /// `\textbf`.
    Bold,
    /// `\textsc` (algpseudocode's `\textproc`, line 34).
    SmallCaps,
}

/// A length as written: `em` depends on the font in force where it is used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimen {
    Em(f64),
    Ex(f64),
    Pt(f64),
}

impl Dimen {
    /// Points, given the current font's quad and x-height.
    pub fn pt(self, em: f64, ex: f64) -> f64 {
        match self {
            Dimen::Em(v) => v * em,
            Dimen::Ex(v) => v * ex,
            Dimen::Pt(v) => v,
        }
    }

    pub fn parse(text: &str) -> Option<Dimen> {
        let t = text.trim();
        let split = t.find(|c: char| c.is_ascii_alphabetic())?;
        let value: f64 = t[..split].trim().parse().ok()?;
        let per_pt = match t[split..].trim() {
            "em" => return Some(Dimen::Em(value)),
            "ex" => return Some(Dimen::Ex(value)),
            "pt" => 1.0,
            "bp" => 72.27 / 72.0,
            "in" => 72.27,
            "cm" => 72.27 / 2.54,
            "mm" => 72.27 / 25.4,
            "pc" => 12.0,
            "sp" => 1.0 / 65536.0,
            _ => return None,
        };
        Some(Dimen::Pt(value * per_pt))
    }
}

/// One piece of a statement line, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// Fixed text of a package macro (`\textbf{while}`, the `(` of
    /// `\Call`), attributed to the command that produced it.
    Text {
        text: String,
        face: Face,
        span: Span,
    },
    /// Author material (statement text, a condition, a comment argument):
    /// ordinary paragraph material, math included. The span never starts
    /// or ends with whitespace.
    Source(Span),
    /// Author material set in `face` (`\textproc{..}`, `\Call{name}{..}`).
    StyledSource { face: Face, span: Span },
    /// An interword space: a control space `\ ` or a space token inside a
    /// macro definition, or real whitespace between two pieces.
    Space { face: Face },
    /// `\hfill` (algorithmicx's `\algorithmiccomment`, line 579).
    HFill { span: Span },
    /// A math-mode symbol set in text (`\(\triangleright\)`, and
    /// algorithmic's `\{`/`\}` around a comment, which in OT1 come from the
    /// math symbol font): `tex` is the control sequence, without `$`.
    MathSymbol { tex: &'static str, span: Span },
}

/// How a line enters the list.
#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    /// `\item` with the list's own label: the line counter steps and the
    /// number is printed when [`Line::show_number`] (every statement).
    Numbered,
    /// `\item[<label>]`: `\REQUIRE`/`\ENSURE`, `\Require`/`\Ensure` and
    /// `\algnewcommand\X{\item[..]}` commands; the counter does not step.
    Labelled(Vec<Piece>),
    /// `\item[]` with text: algorithmicx `\Statex` (line 632).
    Unlabelled,
    /// algorithmicx's `\item[]\nointerlineskip` for an entity without text
    /// (`noend`'s `\EndIf`, lines 193-194): an empty line of no height
    /// appended without interline glue.
    NoText,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub kind: LineKind,
    /// The line counter after this line's step (`ALC@line`/`ALG@line`).
    pub number: u32,
    /// The label shows the number (the step reset the frequency counter).
    pub show_number: bool,
    /// Open blocks around the line (0 = outermost).
    pub depth: u32,
    /// algpseudocode only: the line's text starts after
    /// `\noindent\hskip\ALG@tlm` (algorithmicx line 198), so a wrapped
    /// statement's later lines start at the list margin; `false` for
    /// `\Statex` and labelled lines, which never add `\ALG@tlm`.
    pub hskip_tlm: bool,
    pub pieces: Vec<Piece>,
    /// The command that starts the line.
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Algorithmic {
    pub dialect: Dialect,
    /// `\begin{algorithmic}[n]`: every n-th line is numbered (0 = none).
    pub frequency: u32,
    pub lines: Vec<Line>,
    /// `\begin{algorithmic}` through `\end{algorithmic}`.
    pub span: Span,
    /// Constructs read but not modelled, and package errors.
    pub problems: Vec<(Span, String)>,
}

impl Algorithmic {
    /// `\labelwidth`: 1.2em with numbering, 0.5em without
    /// (algorithmic.sty 222-224, algorithmicx.sty 94-96).
    pub fn label_width_em(&self) -> f64 {
        if self.frequency == 0 {
            0.5
        } else {
            1.2
        }
    }
}

/// `\labelsep 0.5em` (algorithmic.sty 221, algorithmicx.sty 93).
pub const LABELSEP_EM: f64 = 0.5;
/// `\topsep 0.2em` of the outer list (algorithmic.sty 221, algorithmicx.sty 93).
pub const TOPSEP_EM: f64 = 0.2;
/// `\@fs@pre` `\hrule height.8pt` and the `\kern2pt` after every ruled
/// rule (float.sty 153-155).
pub const RULED_TOP_RULE_PT: f64 = 0.8;
/// `\hrule` default height: `\@fs@mid` and `\@fs@post` rules.
pub const RULED_RULE_PT: f64 = 0.4;
pub const RULED_KERN_PT: f64 = 2.0;

/// Line number label format: algorithmic's `{\ALC@linenosize
/// \arabic{ALC@line}\ALC@linenodelimiter}` (lines 58-63, 129) or
/// algorithmicx's `\alglinenumber{n}` = `\footnotesize n:` (line 581).
#[derive(Debug, Clone, PartialEq)]
pub struct LineNumbers {
    /// `None` is `\normalsize`.
    pub size: Option<FontSizeLevel>,
    pub bold: bool,
    pub prefix: String,
    pub suffix: String,
}

impl Default for LineNumbers {
    fn default() -> Self {
        LineNumbers {
            size: Some(FontSizeLevel::FootnoteSize),
            bold: false,
            prefix: String::new(),
            suffix: ":".into(),
        }
    }
}

/// What the preamble selects.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    pub dialect: Option<Dialect>,
    pub float_loaded: bool,
    pub float_style: FloatStyle,
    /// `\ALG@name` (`algorithm.sty` line 11, any unknown option, or
    /// `\floatname{algorithm}{..}`).
    pub float_name: String,
    /// `algorithm.sty`'s `section`/`chapter`/... option: the counter the
    /// float number is reset by and prefixed with.
    pub within: Option<String>,
    /// `noend` (algorithmic.sty 43, algpseudocode.sty 23).
    pub noend: bool,
    /// algorithmic: `\algorithmicindent` = 1em unless `\algsetup{indent=..}`
    /// (lines 51-55); algorithmicx: 1.5em unless renewed (line 580).
    pub indent: Dimen,
    pub line_numbers: LineNumbers,
    /// Keyword macros (`algorithmicwhile`, ...) as redefined in the
    /// document, overriding the package defaults.
    keywords: HashMap<String, Vec<Tpl>>,
    /// `\algnewcommand\Input{\item[\textbf{Input:}]}`-style label commands.
    label_commands: HashMap<String, Vec<Tpl>>,
    /// Definitions that were read but not understood.
    pub problems: Vec<(usize, String)>,
}

impl Default for Setup {
    fn default() -> Self {
        Setup {
            dialect: None,
            float_loaded: false,
            float_style: FloatStyle::Ruled,
            float_name: "Algorithm".into(),
            within: None,
            noend: false,
            indent: Dimen::Em(1.0),
            line_numbers: LineNumbers::default(),
            keywords: HashMap::new(),
            label_commands: HashMap::new(),
            problems: Vec::new(),
        }
    }
}

/// A macro template piece, before it is attributed to a command span.
#[derive(Debug, Clone, PartialEq)]
enum Tpl {
    Text(String, Face),
    Space(Face),
    Keyword(String),
    HFill,
}

/// Package defaults of the keyword macros. algorithmic.sty 66-97.
fn algorithmic_default(name: &str) -> Option<Vec<Tpl>> {
    let bold = |t: &str| Some(vec![Tpl::Text(t.into(), Face::Bold)]);
    let pair = |a: &str, b: &str| {
        Some(vec![
            Tpl::Keyword(a.into()),
            Tpl::Space(Face::Roman),
            Tpl::Keyword(b.into()),
        ])
    };
    match name {
        "algorithmicrequire" => bold("Require:"),
        "algorithmicensure" => bold("Ensure:"),
        "algorithmicend" => bold("end"),
        "algorithmicif" => bold("if"),
        "algorithmicthen" => bold("then"),
        "algorithmicelse" => bold("else"),
        "algorithmicelsif" => pair("algorithmicelse", "algorithmicif"),
        "algorithmicendif" => pair("algorithmicend", "algorithmicif"),
        "algorithmicfor" => bold("for"),
        "algorithmicforall" => bold("for all"),
        "algorithmicdo" => bold("do"),
        "algorithmicendfor" => pair("algorithmicend", "algorithmicfor"),
        "algorithmicwhile" => bold("while"),
        "algorithmicendwhile" => pair("algorithmicend", "algorithmicwhile"),
        "algorithmicloop" => bold("loop"),
        "algorithmicendloop" => pair("algorithmicend", "algorithmicloop"),
        "algorithmicrepeat" => bold("repeat"),
        "algorithmicuntil" => bold("until"),
        "algorithmicprint" => bold("print"),
        "algorithmicreturn" => bold("return"),
        "algorithmicand" => bold("and"),
        "algorithmicor" => bold("or"),
        "algorithmicxor" => bold("xor"),
        "algorithmicnot" => bold("not"),
        "algorithmicto" => bold("to"),
        "algorithmicinputs" => bold("inputs"),
        "algorithmicoutputs" => bold("outputs"),
        "algorithmicglobals" => bold("globals"),
        "algorithmicbody" => bold("do"),
        "algorithmictrue" => bold("true"),
        "algorithmicfalse" => bold("false"),
        _ => None,
    }
}

/// algpseudocode.sty 17-32.
fn algpseudocode_default(name: &str) -> Option<Vec<Tpl>> {
    let bold = |t: &str| Some(vec![Tpl::Text(t.into(), Face::Bold)]);
    match name {
        "algorithmicend" => bold("end"),
        "algorithmicdo" => bold("do"),
        "algorithmicwhile" => bold("while"),
        "algorithmicfor" => bold("for"),
        "algorithmicforall" => bold("for all"),
        "algorithmicloop" => bold("loop"),
        "algorithmicrepeat" => bold("repeat"),
        "algorithmicuntil" => bold("until"),
        "algorithmicprocedure" => bold("procedure"),
        "algorithmicfunction" => bold("function"),
        "algorithmicif" => bold("if"),
        "algorithmicthen" => bold("then"),
        "algorithmicelse" => bold("else"),
        "algorithmicrequire" => bold("Require:"),
        "algorithmicensure" => bold("Ensure:"),
        "algorithmicreturn" => bold("return"),
        _ => None,
    }
}

/// Every keyword macro either package defines (the expansion pass declares
/// them as host commands so `\renewcommand{\algorithmicrequire}` is legal).
pub const KEYWORD_MACROS: &[&str] = &[
    "algorithmicrequire",
    "algorithmicensure",
    "algorithmicend",
    "algorithmicif",
    "algorithmicthen",
    "algorithmicelse",
    "algorithmicelsif",
    "algorithmicendif",
    "algorithmicfor",
    "algorithmicforall",
    "algorithmicdo",
    "algorithmicendfor",
    "algorithmicwhile",
    "algorithmicendwhile",
    "algorithmicloop",
    "algorithmicendloop",
    "algorithmicrepeat",
    "algorithmicuntil",
    "algorithmicprint",
    "algorithmicreturn",
    "algorithmicand",
    "algorithmicor",
    "algorithmicxor",
    "algorithmicnot",
    "algorithmicto",
    "algorithmicinputs",
    "algorithmicoutputs",
    "algorithmicglobals",
    "algorithmicbody",
    "algorithmictrue",
    "algorithmicfalse",
    "algorithmicprocedure",
    "algorithmicfunction",
    "algorithmiccomment",
    "algorithmicindent",
    "alglinenumber",
    "textproc",
];

// ---- source helpers ---------------------------------------------------------

fn is_commented(line_prefix: &str) -> bool {
    let b = line_prefix.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'%' => return true,
            _ => i += 1,
        }
    }
    false
}

/// `needle` at or after `from`, not inside a `%` comment.
fn find_uncommented(text: &str, needle: &str, from: usize) -> Option<usize> {
    let mut at = from;
    while let Some(rel) = text.get(at..)?.find(needle) {
        let pos = at + rel;
        let line_start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
        if !is_commented(&text[line_start..pos]) {
            return Some(pos);
        }
        at = pos + needle.len();
    }
    None
}

/// The end (exclusive, after `}`) of the balanced group opening at `open`.
fn group_end(b: &[u8], open: usize, limit: usize) -> Option<usize> {
    if b.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open;
    while i < limit {
        match b[i] {
            b'\\' => i += 1,
            b'%' => {
                while i < limit && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The end (after `]`) of an optional argument opening at `open`, honouring
/// braces inside it.
fn bracket_end(b: &[u8], open: usize, limit: usize) -> Option<usize> {
    if b.get(open) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open + 1;
    while i < limit {
        match b[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b']' if depth <= 0 => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }
    None
}

fn skip_ws(b: &[u8], mut i: usize, limit: usize) -> usize {
    while i < limit && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// Whitespace and `%` comments.
fn skip_ws_comments(b: &[u8], mut i: usize, limit: usize) -> usize {
    loop {
        i = skip_ws(b, i, limit);
        if i < limit && b[i] == b'%' {
            while i < limit && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        return i;
    }
}

/// The control word starting at `i` (`b[i] == b'\\'`): its name's end.
fn control_word_end(b: &[u8], i: usize, limit: usize) -> usize {
    let mut j = i + 1;
    while j < limit && b[j].is_ascii_alphabetic() {
        j += 1;
    }
    j
}

/// The byte range trimmed of whitespace.
fn trim(text: &str, start: usize, end: usize) -> (usize, usize) {
    let s = &text[start..end];
    let lead = s.len() - s.trim_start().len();
    let tail = s.len() - s.trim_end().len();
    (start + lead, (end - tail).max(start + lead))
}

/// A required argument after `i`: a braced group (inner range) or a single
/// token (a control word, or one character). Returns `(inner_start,
/// inner_end, after)`.
fn argument(b: &[u8], i: usize, limit: usize) -> Option<(usize, usize, usize)> {
    let j = skip_ws_comments(b, i, limit);
    if j >= limit {
        return None;
    }
    match b[j] {
        b'{' => group_end(b, j, limit).map(|e| (j + 1, e - 1, e)),
        b'\\' => {
            let e = control_word_end(b, j, limit).max(j + 2).min(limit);
            Some((j, e, e))
        }
        b'}' => None,
        _ => Some((j, j + 1, j + 1)),
    }
}

// ---- preamble -----------------------------------------------------------------

/// Reads the package selection, options and definitions of `text` (the
/// whole entry document; definitions after `\begin{document}` apply to
/// later algorithms in LaTeX, and are applied to all of them here).
pub fn setup(text: &str) -> Setup {
    let mut s = Setup::default();
    let b = text.as_bytes();
    let mut at = 0;
    while let Some(pos) = find_uncommented(text, "\\usepackage", at) {
        at = pos + "\\usepackage".len();
        let mut j = skip_ws_comments(b, at, b.len());
        let mut options = String::new();
        if let Some(e) = bracket_end(b, j, b.len()) {
            options = text[j + 1..e - 1].to_string();
            j = skip_ws_comments(b, e, b.len());
        }
        let Some(e) = group_end(b, j, b.len()) else {
            continue;
        };
        let options: Vec<String> = options
            .split(',')
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty())
            .collect();
        for package in text[j + 1..e - 1].split(',').map(str::trim) {
            match package {
                "algorithm" => {
                    s.float_loaded = true;
                    for o in &options {
                        match o.as_str() {
                            "plain" => s.float_style = FloatStyle::Plain,
                            "ruled" => s.float_style = FloatStyle::Ruled,
                            "boxed" => s.float_style = FloatStyle::Boxed,
                            "part" | "chapter" | "section" | "subsection" | "subsubsection" => {
                                s.within = Some(o.clone())
                            }
                            "nothing" => s.within = None,
                            // `\DeclareOption*{\edef\ALG@name{\CurrentOption}}` (line 48).
                            other => s.float_name = other.to_string(),
                        }
                    }
                }
                "algorithmic" => {
                    s.dialect.get_or_insert(Dialect::Algorithmic);
                    s.noend |= options.iter().any(|o| o == "noend");
                }
                "algpseudocode" => {
                    if s.dialect.is_none() {
                        s.dialect = Some(Dialect::Algpseudocode);
                        s.indent = Dimen::Em(1.5);
                    }
                    for o in &options {
                        match o.as_str() {
                            "noend" => s.noend = true,
                            "end" => s.noend = false,
                            _ => s.problems.push((
                                pos,
                                format!("algpseudocode option '{o}' is not implemented"),
                            )),
                        }
                    }
                }
                _ => {}
            }
        }
        at = e;
    }
    read_definitions(text, &mut s);
    s
}

fn read_definitions(text: &str, s: &mut Setup) {
    let b = text.as_bytes();
    let mut i = 0;
    while let Some(rel) = text[i..].find('\\') {
        let pos = i + rel;
        let line_start = text[..pos].rfind('\n').map_or(0, |n| n + 1);
        if is_commented(&text[line_start..pos]) {
            i = text[pos..].find('\n').map_or(text.len(), |n| pos + n + 1);
            continue;
        }
        let name_end = control_word_end(b, pos, b.len());
        let name = &text[pos + 1..name_end];
        i = name_end.max(pos + 2).min(text.len());
        match name {
            "newcommand" | "renewcommand" | "providecommand" | "algnewcommand"
            | "algrenewcommand" | "def" => {
                let mut j = name_end;
                if name != "def" && b.get(skip_ws(b, j, b.len())) == Some(&b'*') {
                    j = skip_ws(b, j, b.len()) + 1;
                }
                let Some((ns, ne, after)) = argument(b, j, b.len()) else {
                    continue;
                };
                let (ns, ne) = trim(text, ns, ne);
                if b.get(ns) != Some(&b'\\') {
                    continue;
                }
                let defined = text[ns + 1..ne].to_string();
                let mut j = skip_ws_comments(b, after, b.len());
                let mut params = 0;
                if name == "def" {
                    while j < b.len() && b[j] == b'#' {
                        params += 1;
                        j += 2;
                    }
                } else if let Some(e) = bracket_end(b, j, b.len()) {
                    params = text[j + 1..e - 1].trim().parse().unwrap_or(0);
                    j = skip_ws_comments(b, e, b.len());
                    if let Some(e2) = bracket_end(b, j, b.len()) {
                        j = skip_ws_comments(b, e2, b.len());
                    }
                }
                let Some(body_end) = group_end(b, j, b.len()) else {
                    continue;
                };
                let body = &text[j + 1..body_end - 1];
                i = body_end;
                define(s, &defined, params, body, pos);
            }
            "algsetup" => {
                let Some((bs, be, after)) = argument(b, name_end, b.len()) else {
                    continue;
                };
                i = after;
                for kv in text[bs..be].split(',') {
                    let Some((k, v)) = kv.split_once('=') else {
                        continue;
                    };
                    match k.trim() {
                        "indent" => match Dimen::parse(v) {
                            Some(d) => s.indent = d,
                            None => s.problems.push((
                                pos,
                                format!("\\algsetup indent '{}' is not a length", v.trim()),
                            )),
                        },
                        "linenodelimiter" => s.line_numbers.suffix = v.trim().to_string(),
                        "linenosize" => {
                            s.line_numbers.size =
                                size_declaration(v.trim().trim_start_matches('\\')).unwrap_or(None)
                        }
                        other => s
                            .problems
                            .push((pos, format!("\\algsetup key '{other}' is not implemented"))),
                    }
                }
            }
            "floatname" => {
                let Some((fs, fe, after)) = argument(b, name_end, b.len()) else {
                    continue;
                };
                if text[fs..fe].trim() != "algorithm" {
                    continue;
                }
                let Some((ts, te, after2)) = argument(b, after, b.len()) else {
                    continue;
                };
                s.float_name = text[ts..te].trim().to_string();
                i = after2;
            }
            "floatstyle" => {
                // `\floatstyle{..}\restylefloat{algorithm}`.
                let Some((fs, fe, after)) = argument(b, name_end, b.len()) else {
                    continue;
                };
                let style = match text[fs..fe].trim() {
                    "plain" => FloatStyle::Plain,
                    "ruled" => FloatStyle::Ruled,
                    "boxed" => FloatStyle::Boxed,
                    _ => continue,
                };
                if text[after..]
                    .trim_start()
                    .starts_with("\\restylefloat{algorithm}")
                {
                    s.float_style = style;
                }
                i = after;
            }
            _ => {}
        }
    }
}

/// An algpseudocode option this model implements (`noend`/`end`,
/// algpseudocode.sty 23-24; `compatible` loads algcompatible, not modelled).
pub fn algpseudocode_option(option: &str) -> bool {
    matches!(option, "noend" | "end")
}

/// `\small` .. `\Huge` as a size level (`Some(None)` for `\normalsize`).
pub fn size_declaration(name: &str) -> Option<Option<FontSizeLevel>> {
    Some(match name {
        "tiny" => Some(FontSizeLevel::Tiny),
        "scriptsize" => Some(FontSizeLevel::ScriptSize),
        "footnotesize" => Some(FontSizeLevel::FootnoteSize),
        "small" => Some(FontSizeLevel::Small),
        "normalsize" => None,
        "large" => Some(FontSizeLevel::Large1),
        "Large" => Some(FontSizeLevel::Large2),
        "LARGE" => Some(FontSizeLevel::Large3),
        "huge" => Some(FontSizeLevel::Huge1),
        "Huge" => Some(FontSizeLevel::Huge2),
        _ => return None,
    })
}

fn define(s: &mut Setup, name: &str, params: usize, body: &str, pos: usize) {
    match name {
        "algorithmicindent" => match Dimen::parse(body) {
            Some(d) => s.indent = d,
            None => s.problems.push((
                pos,
                format!("\\algorithmicindent '{}' is not a length", body.trim()),
            )),
        },
        "alglinenumber" => match line_number_format(body) {
            Some(format) => s.line_numbers = format,
            None => s.problems.push((
                pos,
                "this \\alglinenumber definition is not implemented; the default is used".into(),
            )),
        },
        "algorithmiccomment" => s.problems.push((
            pos,
            "a redefined \\algorithmiccomment is not implemented; the package comment is used"
                .into(),
        )),
        _ if name.starts_with("algorithmic") && params == 0 => match template(body, s) {
            Some(t) => {
                s.keywords.insert(name.to_string(), t);
            }
            None => s.problems.push((
                pos,
                format!("the definition of \\{name} is not implemented; the package text is used"),
            )),
        },
        _ if params == 0 && body.trim_start().starts_with("\\item[") => {
            let t = body.trim();
            let inner = &t["\\item[".len()..];
            let Some(close) = bracket_end(inner.as_bytes(), 0, inner.len()).or_else(|| {
                // `\item[` already consumed the `[`: find the matching `]`.
                let wrapped = format!("[{inner}");
                bracket_end(wrapped.as_bytes(), 0, wrapped.len()).map(|e| e - 1)
            }) else {
                return;
            };
            let label = &inner[..close.saturating_sub(1)];
            match template(label, s) {
                Some(t) if inner[close..].trim().is_empty() => {
                    s.label_commands.insert(name.to_string(), t);
                }
                _ => s.problems.push((
                    pos,
                    format!("the definition of \\{name} is not implemented"),
                )),
            }
        }
        _ => {}
    }
}

/// `{\footnotesize #1:}`-style `\alglinenumber` bodies.
fn line_number_format(body: &str) -> Option<LineNumbers> {
    let mut t = body.trim();
    if t.starts_with('{') && t.ends_with('}') {
        t = t[1..t.len() - 1].trim();
    }
    let mut format = LineNumbers {
        size: None,
        bold: false,
        prefix: String::new(),
        suffix: String::new(),
    };
    loop {
        let Some(rest) = t.strip_prefix('\\') else {
            break;
        };
        let end = rest
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        let word = &rest[..end];
        if let Some(size) = size_declaration(word) {
            format.size = size;
        } else if word == "bfseries" {
            format.bold = true;
        } else if word == "normalfont" {
        } else {
            return None;
        }
        t = rest[end..].trim_start();
    }
    let (prefix, suffix) = t.split_once("#1")?;
    if prefix.contains(['\\', '{', '}']) || suffix.contains(['\\', '{', '}']) {
        return None;
    }
    format.prefix = prefix.to_string();
    format.suffix = suffix.to_string();
    Some(format)
}

/// A macro body made of text, `\textbf{..}`/`\textsc{..}`, keyword macros,
/// control spaces and `\hfill`.
fn template(body: &str, s: &Setup) -> Option<Vec<Tpl>> {
    let mut out = Vec::new();
    template_into(body, Face::Roman, s, &mut out, 0)?;
    Some(out)
}

fn template_into(
    body: &str,
    face: Face,
    s: &Setup,
    out: &mut Vec<Tpl>,
    depth: usize,
) -> Option<()> {
    if depth > 8 {
        return None;
    }
    let b = body.as_bytes();
    let mut i = 0;
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut Vec<Tpl>| {
        if !word.is_empty() {
            out.push(Tpl::Text(std::mem::take(word), face));
        }
    };
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\r' => {
                flush(&mut word, out);
                if !matches!(out.last(), Some(Tpl::Space(_)) | None) {
                    out.push(Tpl::Space(face));
                }
                i = skip_ws(b, i, b.len());
            }
            b'%' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                i += 1;
            }
            b'{' => {
                let e = group_end(b, i, b.len())?;
                flush(&mut word, out);
                template_into(&body[i + 1..e - 1], face, s, out, depth + 1)?;
                i = e;
            }
            b'\\' => {
                flush(&mut word, out);
                let e = control_word_end(b, i, b.len());
                if e == i + 1 {
                    match b.get(i + 1) {
                        Some(b' ') => out.push(Tpl::Space(face)),
                        _ => return None,
                    }
                    i += 2;
                    continue;
                }
                let name = &body[i + 1..e];
                match name {
                    "textbf" | "textsc" | "textrm" | "textnormal" => {
                        let (gs, ge, after) = argument(b, e, b.len())?;
                        let inner_face = match name {
                            "textbf" => Face::Bold,
                            "textsc" => Face::SmallCaps,
                            _ => Face::Roman,
                        };
                        template_into(&body[gs..ge], inner_face, s, out, depth + 1)?;
                        i = after;
                    }
                    "bfseries" => {
                        template_into(
                            &body[skip_ws(b, e, b.len())..],
                            Face::Bold,
                            s,
                            out,
                            depth + 1,
                        )?;
                        return Some(());
                    }
                    "hfill" => {
                        out.push(Tpl::HFill);
                        i = skip_ws(b, e, b.len());
                    }
                    _ if name.starts_with("algorithmic")
                        && (s.keywords.contains_key(name)
                            || algorithmic_default(name).is_some()
                            || algpseudocode_default(name).is_some()) =>
                    {
                        out.push(Tpl::Keyword(name.to_string()));
                        // A control word swallows the spaces after it; `{}` ends it.
                        let mut j = skip_ws(b, e, b.len());
                        if body[j..].starts_with("{}") {
                            j += 2;
                        }
                        i = j;
                    }
                    _ => return None,
                }
            }
            b'$' | b'#' | b'&' | b'^' | b'_' | b'~' => return None,
            c => {
                let ch_len = body[i..].chars().next().map_or(1, char::len_utf8);
                word.push_str(&body[i..i + ch_len]);
                let _ = c;
                i += ch_len;
            }
        }
    }
    flush(&mut word, out);
    Some(())
}

// ---- body -------------------------------------------------------------------------

/// An `algorithm`/`algorithm*` float.
#[derive(Debug, Clone, PartialEq)]
pub struct AlgorithmFloat {
    pub starred: bool,
    /// The placement letters as written (`None` = `htbp`, float.sty's
    /// `\floatplacement{algorithm}{htbp}`, algorithm.sty 59-65).
    pub placement: Option<String>,
    /// `\begin{algorithm}` through `\end{algorithm}`.
    pub span: Span,
    /// Text of the same paragraph precedes the float.
    pub hmode: bool,
    pub body: Vec<FloatItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FloatItem {
    /// `\caption[short]{text}`: `span` covers the command, `arg` the text.
    Caption {
        span: Span,
        arg: Span,
    },
    Label {
        key: String,
        span: Span,
    },
    Algorithmic(Algorithmic),
    /// `\small`, `\footnotesize`, ...: the size of what follows.
    Size {
        size: Option<FontSizeLevel>,
        span: Span,
    },
    /// `\centering`, `\raggedright`: no effect on list items (they set
    /// `\leftskip`/`\rightskip` for paragraphs, which `\list` does not reset
    /// but which pseudocode lines rarely rely on); recorded to be reported.
    Declaration {
        span: Span,
    },
    /// Anything else.
    Other {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    Float(AlgorithmFloat),
    /// An `algorithmic` environment outside any `algorithm` float.
    Bare(Algorithmic),
}

impl Found {
    pub fn span(&self) -> Span {
        match self {
            Found::Float(f) => f.span,
            Found::Bare(a) => a.span,
        }
    }
}

/// Every `algorithm` float and bare `algorithmic` environment in the body
/// of `text`, in source order. Nothing is found when no pseudocode package
/// is loaded (`setup.dialect` is `None`) — except floats, which only need
/// the `algorithm` package.
pub fn scan(text: &str, document: DocumentId, setup: &Setup) -> Vec<Found> {
    let body_start =
        find_uncommented(text, "\\begin{document}", 0).map_or(0, |p| p + "\\begin{document}".len());
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut at = body_start;
    while let Some(pos) = find_uncommented(text, "\\begin{", at) {
        let name_start = pos + "\\begin{".len();
        let Some(close) = text[name_start..].find('}') else {
            break;
        };
        let name = &text[name_start..name_start + close];
        let after = name_start + close + 1;
        match name {
            "algorithm" | "algorithm*" if setup.float_loaded || setup.dialect.is_some() => {
                let end_tag = format!("\\end{{{name}}}");
                let Some(end) = find_uncommented(text, &end_tag, after) else {
                    break;
                };
                let mut cursor = after;
                let mut placement = None;
                let j = skip_ws(b, cursor, end);
                if let Some(e) = bracket_end(b, j, end) {
                    placement = Some(text[j + 1..e - 1].trim().to_string());
                    cursor = e;
                }
                let before = &text[body_start..pos];
                let hmode = !before.trim().is_empty() && !preceded_by_blank_line(before);
                let body = float_body(text, document, setup, cursor, end);
                out.push(Found::Float(AlgorithmFloat {
                    starred: name.ends_with('*'),
                    placement,
                    span: Span::in_document(document, pos, end + end_tag.len()),
                    hmode,
                    body,
                }));
                at = end + end_tag.len();
            }
            "algorithmic" if setup.dialect.is_some() => {
                match parse_algorithmic(text, document, setup, pos) {
                    Some((alg, end)) => {
                        out.push(Found::Bare(alg));
                        at = end;
                    }
                    None => break,
                }
            }
            _ => at = after,
        }
    }
    out
}

fn preceded_by_blank_line(before: &str) -> bool {
    let trimmed = before.trim_end();
    let gap = &before[trimmed.len()..];
    gap.matches('\n').count() >= 2
        || trimmed.ends_with("\\par")
        || trimmed.ends_with('}')
            && trimmed
                .rfind("\\end{")
                .is_some_and(|p| !trimmed[p..].contains('\n'))
}

fn float_body(
    text: &str,
    document: DocumentId,
    setup: &Setup,
    start: usize,
    end: usize,
) -> Vec<FloatItem> {
    let b = text.as_bytes();
    let span = |s: usize, e: usize| Span::in_document(document, s, e);
    let mut out = Vec::new();
    let mut i = start;
    while i < end {
        i = skip_ws_comments(b, i, end);
        if i >= end {
            break;
        }
        if b[i] != b'\\' {
            let s = i;
            while i < end && !matches!(b[i], b'\\' | b'%') {
                i += 1;
            }
            let (ts, te) = trim(text, s, i);
            if te > ts {
                out.push(FloatItem::Other { span: span(ts, te) });
            }
            continue;
        }
        let name_end = control_word_end(b, i, end);
        let name = &text[i + 1..name_end];
        match name {
            "caption" => {
                let mut j = skip_ws_comments(b, name_end, end);
                if let Some(e) = bracket_end(b, j, end) {
                    j = e;
                }
                match argument(b, j, end) {
                    Some((s, e, after)) => {
                        out.push(FloatItem::Caption {
                            span: span(i, after),
                            arg: span(s, e),
                        });
                        i = after;
                    }
                    None => {
                        out.push(FloatItem::Other {
                            span: span(i, name_end),
                        });
                        i = name_end;
                    }
                }
            }
            "label" => match argument(b, name_end, end) {
                Some((s, e, after)) => {
                    out.push(FloatItem::Label {
                        key: text[s..e].trim().to_string(),
                        span: span(i, after),
                    });
                    i = after;
                }
                None => {
                    out.push(FloatItem::Other {
                        span: span(i, name_end),
                    });
                    i = name_end;
                }
            },
            "begin" if text[name_end..].starts_with("{algorithmic}") && setup.dialect.is_some() => {
                match parse_algorithmic(text, document, setup, i) {
                    Some((alg, after)) if after <= end => {
                        out.push(FloatItem::Algorithmic(alg));
                        i = after;
                    }
                    _ => {
                        out.push(FloatItem::Other { span: span(i, end) });
                        i = end;
                    }
                }
            }
            "centering" | "raggedright" | "raggedleft" => {
                out.push(FloatItem::Declaration {
                    span: span(i, name_end),
                });
                i = name_end;
            }
            _ => match size_declaration(name) {
                Some(size) => {
                    out.push(FloatItem::Size {
                        size,
                        span: span(i, name_end),
                    });
                    i = name_end;
                }
                None => {
                    // Everything up to the next top-level command start.
                    let s = i;
                    i = name_end.max(i + 2).min(end);
                    if let Some((_, _, after)) =
                        argument(b, i, end).filter(|_| b.get(skip_ws(b, i, end)) == Some(&b'{'))
                    {
                        i = after;
                    }
                    out.push(FloatItem::Other { span: span(s, i) });
                }
            },
        }
    }
    out
}

/// What a statement command does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    If,
    Else,
    For,
    While,
    Loop,
    Repeat,
    Procedure,
    Function,
    Group,
}

/// A statement command: `(close, kind, open)` plus how its text is built.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Stmt {
    /// `\STATE`, `\State`.
    State,
    /// `\RETURN`/`\PRINT` (algorithmic 157-158): `\ALC@it\algorithmicX{} \ `.
    Keyword2Spaces(&'static str),
    /// `\REQUIRE`, `\Require`: `\item[\algorithmicX]`.
    Label(&'static str),
    /// `\Statex`.
    Statex,
    /// `\IF[c]{cond}` .. and algpseudocode `\If{cond}`: opens `block` after
    /// `\algorithmicA\ #\ \algorithmicB`.
    Cond {
        close: Option<Block>,
        open: Block,
        head: &'static str,
        tail: Option<&'static str>,
    },
    /// `\ELSE[c]`, `\LOOP[c]`, `\REPEAT[c]`, `\Else`, `\Loop`, `\Repeat`.
    Bare {
        close: Option<Block>,
        open: Block,
        head: &'static str,
    },
    /// `\UNTIL{c}`, `\Until{c}`: closes `close`, `\algorithmicuntil\ #1`.
    Until,
    /// `\ENDIF`, `\EndIf`, ...: closes `close`; the text is a keyword pair.
    End { close: Block, keyword: &'static str },
    /// `\INPUTS[c]` .. `\BODY[c]` (algorithmic 179-185): `\algorithmicX\ `
    /// plus an optional comment, opening a group.
    Group { head: &'static str },
    /// `\GLOBALS`: `\algorithmicglobals\ ` (no block).
    Globals,
    /// `\ENDINPUTS` ...: closes the group, no line.
    EndGroup,
    /// `\Procedure{name}{args}`, `\Function{name}{args}`.
    Proc { open: Block, head: &'static str },
}

fn algorithmic_stmt(name: &str, noend: bool) -> Option<Stmt> {
    use Block as B;
    Some(match name {
        "STATE" | "STMT" => Stmt::State,
        "RETURN" => Stmt::Keyword2Spaces("algorithmicreturn"),
        "PRINT" => Stmt::Keyword2Spaces("algorithmicprint"),
        "REQUIRE" => Stmt::Label("algorithmicrequire"),
        "ENSURE" => Stmt::Label("algorithmicensure"),
        "IF" => Stmt::Cond {
            close: None,
            open: B::If,
            head: "algorithmicif",
            tail: Some("algorithmicthen"),
        },
        "ELSIF" => Stmt::Cond {
            close: Some(B::If),
            open: B::If,
            head: "algorithmicelsif",
            tail: Some("algorithmicthen"),
        },
        "ELSE" => Stmt::Bare {
            close: Some(B::If),
            open: B::If,
            head: "algorithmicelse",
        },
        "FOR" => Stmt::Cond {
            close: None,
            open: B::For,
            head: "algorithmicfor",
            tail: Some("algorithmicdo"),
        },
        "FORALL" => Stmt::Cond {
            close: None,
            open: B::For,
            head: "algorithmicforall",
            tail: Some("algorithmicdo"),
        },
        "WHILE" => Stmt::Cond {
            close: None,
            open: B::While,
            head: "algorithmicwhile",
            tail: Some("algorithmicdo"),
        },
        "LOOP" => Stmt::Bare {
            close: None,
            open: B::Loop,
            head: "algorithmicloop",
        },
        "REPEAT" => Stmt::Bare {
            close: None,
            open: B::Repeat,
            head: "algorithmicrepeat",
        },
        "UNTIL" => Stmt::Until,
        "ENDIF" => Stmt::End {
            close: B::If,
            keyword: if noend { "" } else { "algorithmicendif" },
        },
        "ENDFOR" => Stmt::End {
            close: B::For,
            keyword: if noend { "" } else { "algorithmicendfor" },
        },
        "ENDWHILE" => Stmt::End {
            close: B::While,
            keyword: if noend { "" } else { "algorithmicendwhile" },
        },
        "ENDLOOP" => Stmt::End {
            close: B::Loop,
            keyword: if noend { "" } else { "algorithmicendloop" },
        },
        "INPUTS" => Stmt::Group {
            head: "algorithmicinputs",
        },
        "OUTPUTS" => Stmt::Group {
            head: "algorithmicoutputs",
        },
        "BODY" => Stmt::Group {
            head: "algorithmicbody",
        },
        "GLOBALS" => Stmt::Globals,
        "ENDINPUTS" | "ENDOUTPUTS" | "ENDBODY" => Stmt::EndGroup,
        _ => return None,
    })
}

fn algpseudocode_stmt(name: &str) -> Option<Stmt> {
    use Block as B;
    Some(match name {
        "State" => Stmt::State,
        "Statex" => Stmt::Statex,
        "Require" => Stmt::Label("algorithmicrequire"),
        "Ensure" => Stmt::Label("algorithmicensure"),
        "If" => Stmt::Cond {
            close: None,
            open: B::If,
            head: "algorithmicif",
            tail: Some("algorithmicthen"),
        },
        // `\algdef{C}[IF]{IF}{ElsIf}[1]{\algorithmicelse\ \algorithmicif\ #1\ \algorithmicthen}` (line 38).
        "ElsIf" => Stmt::Cond {
            close: Some(B::If),
            open: B::If,
            head: "ALGelsif",
            tail: Some("algorithmicthen"),
        },
        "Else" => Stmt::Bare {
            close: Some(B::If),
            open: B::Else,
            head: "algorithmicelse",
        },
        "EndIf" => Stmt::End {
            close: B::If,
            keyword: "ALGendif",
        },
        "For" => Stmt::Cond {
            close: None,
            open: B::For,
            head: "algorithmicfor",
            tail: Some("algorithmicdo"),
        },
        "ForAll" => Stmt::Cond {
            close: None,
            open: B::For,
            head: "algorithmicforall",
            tail: Some("algorithmicdo"),
        },
        "EndFor" => Stmt::End {
            close: B::For,
            keyword: "ALGendfor",
        },
        "While" => Stmt::Cond {
            close: None,
            open: B::While,
            head: "algorithmicwhile",
            tail: Some("algorithmicdo"),
        },
        "EndWhile" => Stmt::End {
            close: B::While,
            keyword: "ALGendwhile",
        },
        "Loop" => Stmt::Bare {
            close: None,
            open: B::Loop,
            head: "algorithmicloop",
        },
        "EndLoop" => Stmt::End {
            close: B::Loop,
            keyword: "ALGendloop",
        },
        "Repeat" => Stmt::Bare {
            close: None,
            open: B::Repeat,
            head: "algorithmicrepeat",
        },
        "Until" => Stmt::Until,
        "Procedure" => Stmt::Proc {
            open: B::Procedure,
            head: "algorithmicprocedure",
        },
        "EndProcedure" => Stmt::End {
            close: B::Procedure,
            keyword: "ALGendprocedure",
        },
        "Function" => Stmt::Proc {
            open: B::Function,
            head: "algorithmicfunction",
        },
        "EndFunction" => Stmt::End {
            close: B::Function,
            keyword: "ALGendfunction",
        },
        _ => return None,
    })
}

struct Reader<'a> {
    text: &'a str,
    b: &'a [u8],
    document: DocumentId,
    setup: &'a Setup,
    dialect: Dialect,
    problems: Vec<(Span, String)>,
}

impl<'a> Reader<'a> {
    fn span(&self, s: usize, e: usize) -> Span {
        Span::in_document(self.document, s, e)
    }

    /// A keyword macro's pieces, attributed to `span`.
    fn keyword(&self, name: &str, span: Span, out: &mut Vec<Piece>) {
        self.keyword_depth(name, span, out, 0);
    }

    fn keyword_depth(&self, name: &str, span: Span, out: &mut Vec<Piece>, depth: usize) {
        if depth > 8 {
            return;
        }
        // algpseudocode's composite texts (lines 32-45) built from the
        // current keyword definitions.
        let composite: Option<[&str; 2]> = match name {
            "ALGelsif" => Some(["algorithmicelse", "algorithmicif"]),
            "ALGendif" => Some(["algorithmicend", "algorithmicif"]),
            "ALGendfor" => Some(["algorithmicend", "algorithmicfor"]),
            "ALGendwhile" => Some(["algorithmicend", "algorithmicwhile"]),
            "ALGendloop" => Some(["algorithmicend", "algorithmicloop"]),
            "ALGendprocedure" => Some(["algorithmicend", "algorithmicprocedure"]),
            "ALGendfunction" => Some(["algorithmicend", "algorithmicfunction"]),
            _ => None,
        };
        if let Some([a, c]) = composite {
            self.keyword_depth(a, span, out, depth + 1);
            out.push(Piece::Space { face: Face::Roman });
            self.keyword_depth(c, span, out, depth + 1);
            return;
        }
        let tpl = self
            .setup
            .keywords
            .get(name)
            .cloned()
            .or_else(|| match self.dialect {
                Dialect::Algorithmic => algorithmic_default(name),
                Dialect::Algpseudocode => algpseudocode_default(name),
            });
        let Some(tpl) = tpl else { return };
        self.instantiate(&tpl, span, out, depth);
    }

    fn instantiate(&self, tpl: &[Tpl], span: Span, out: &mut Vec<Piece>, depth: usize) {
        for t in tpl {
            match t {
                Tpl::Text(text, face) => {
                    // A space inside `\textbf{for all}` is a space of the bold font.
                    for (k, word) in text.split(' ').enumerate() {
                        if k > 0 {
                            out.push(Piece::Space { face: *face });
                        }
                        if !word.is_empty() {
                            out.push(Piece::Text {
                                text: word.to_string(),
                                face: *face,
                                span,
                            });
                        }
                    }
                }
                Tpl::Space(face) => out.push(Piece::Space { face: *face }),
                Tpl::Keyword(name) => self.keyword_depth(name, span, out, depth + 1),
                Tpl::HFill => out.push(Piece::HFill { span }),
            }
        }
    }

    /// The comment of algorithmic's optional `[c]` (`\ALC@com`, line 153-154):
    /// `\ \algorithmiccomment{c}` unless `c` is `default`.
    fn algorithmic_comment(&mut self, command: Span, inner: (usize, usize), out: &mut Vec<Piece>) {
        let (s, e) = trim(self.text, inner.0, inner.1);
        if &self.text[s..e] == "default" {
            return;
        }
        out.push(Piece::Space { face: Face::Roman });
        self.comment_pieces(command, (inner.0, inner.1), out);
    }

    /// `\algorithmiccomment{c}`: `\{c\}` (algorithmic.sty 68) or `\hfill\(\triangleright\) c`
    /// (algorithmicx.sty 579).
    fn comment_pieces(&mut self, command: Span, inner: (usize, usize), out: &mut Vec<Piece>) {
        match self.dialect {
            Dialect::Algorithmic => {
                out.push(Piece::MathSymbol {
                    tex: "\\{",
                    span: command,
                });
                self.inline(inner.0, inner.1, out);
                out.push(Piece::MathSymbol {
                    tex: "\\}",
                    span: command,
                });
            }
            Dialect::Algpseudocode => {
                out.push(Piece::HFill { span: command });
                out.push(Piece::MathSymbol {
                    tex: "\\triangleright",
                    span: command,
                });
                out.push(Piece::Space { face: Face::Roman });
                self.inline(inner.0, inner.1, out);
            }
        }
    }

    /// Inline material between `start` and `end`: source runs split at the
    /// package's inline commands.
    fn inline(&mut self, start: usize, end: usize, out: &mut Vec<Piece>) {
        let b = self.b;
        let mut run: Option<usize> = None;
        let mut i = start;
        // Whether whitespace right here follows a control word (TeX skips it).
        let mut after_control_word = false;
        let flush = |me: &Self, run: &mut Option<usize>, upto: usize, out: &mut Vec<Piece>| {
            if let Some(s) = run.take() {
                let (ts, te) = trim(me.text, s, upto);
                if ts > s && !out.is_empty() && !matches!(out.last(), Some(Piece::Space { .. })) {
                    out.push(Piece::Space { face: Face::Roman });
                }
                if te > ts {
                    out.push(Piece::Source(me.span(ts, te)));
                    if te < upto {
                        out.push(Piece::Space { face: Face::Roman });
                    }
                }
            }
        };
        while i < end {
            match b[i] {
                b'%' => {
                    flush(self, &mut run, i, out);
                    while i < end && b[i] != b'\n' {
                        i += 1;
                    }
                    i = skip_ws(b, i, end);
                    after_control_word = false;
                }
                b' ' | b'\t' | b'\n' | b'\r' if run.is_none() => {
                    let j = skip_ws(b, i, end);
                    if !after_control_word
                        && j < end
                        && !out.is_empty()
                        && !matches!(out.last(), Some(Piece::Space { .. }))
                    {
                        run = Some(i);
                    }
                    i = j;
                }
                b'$' => {
                    run.get_or_insert(i);
                    let double = b.get(i + 1) == Some(&b'$');
                    let mut j = i + if double { 2 } else { 1 };
                    while j < end && b[j] != b'$' {
                        if b[j] == b'\\' {
                            j += 1;
                        }
                        j += 1;
                    }
                    i = (j + if double { 2 } else { 1 }).min(end);
                    after_control_word = false;
                }
                b'{' => {
                    run.get_or_insert(i);
                    i = group_end(b, i, end).unwrap_or(end);
                    after_control_word = false;
                }
                b'\\' => {
                    let e = control_word_end(b, i, end);
                    if e == i + 1 && matches!(b.get(i + 1), Some(b' ' | b'\n' | b'\t')) {
                        // A control space: one interword glue; TeX skips the
                        // blanks after it (tex.web §354).
                        flush(self, &mut run, i, out);
                        out.push(Piece::Space { face: Face::Roman });
                        i += 2;
                        after_control_word = true;
                        continue;
                    }
                    if e == i + 1 {
                        // A control symbol: `\\`, `\$`, `\(` ... `\)`.
                        run.get_or_insert(i);
                        if b.get(i + 1) == Some(&b'(') {
                            let close = self.text[i..end].find("\\)").map_or(end, |p| i + p + 2);
                            i = close;
                        } else {
                            i = (i + 2).min(end);
                        }
                        after_control_word = false;
                        continue;
                    }
                    let name = &self.text[i + 1..e];
                    let command = self.span(i, e);
                    let handled = match (self.dialect, name) {
                        (Dialect::Algorithmic, "COMMENT") => {
                            argument(b, e, end).map(|(s, ae, after)| {
                                flush(self, &mut run, i, out);
                                self.comment_pieces(command, (s, ae), out);
                                after
                            })
                        }
                        (Dialect::Algorithmic, "TRUE" | "FALSE") => {
                            flush(self, &mut run, i, out);
                            let kw = if name == "TRUE" {
                                "algorithmictrue"
                            } else {
                                "algorithmicfalse"
                            };
                            self.keyword(kw, command, out);
                            Some(e)
                        }
                        (Dialect::Algorithmic, "AND" | "OR" | "XOR" | "NOT" | "TO") => {
                            flush(self, &mut run, i, out);
                            let kw = format!("algorithmic{}", name.to_ascii_lowercase());
                            self.keyword(&kw, command, out);
                            // `\newcommand{\AND}{\algorithmicand{} }` (lines 161-165).
                            out.push(Piece::Space { face: Face::Roman });
                            Some(e)
                        }
                        (Dialect::Algpseudocode, "Comment") => {
                            argument(b, e, end).map(|(s, ae, after)| {
                                flush(self, &mut run, i, out);
                                self.comment_pieces(command, (s, ae), out);
                                after
                            })
                        }
                        (Dialect::Algpseudocode, "Return") => {
                            flush(self, &mut run, i, out);
                            // `\algnewcommand\Return{\algorithmicreturn{} }` (line 59).
                            self.keyword("algorithmicreturn", command, out);
                            out.push(Piece::Space { face: Face::Roman });
                            Some(e)
                        }
                        (Dialect::Algpseudocode, "Call") => {
                            argument(b, e, end).and_then(|(ns, ne, after)| {
                                let (as_, ae, after2) = argument(b, after, end)?;
                                flush(self, &mut run, i, out);
                                self.call(command, (ns, ne), (as_, ae), out);
                                Some(after2)
                            })
                        }
                        (Dialect::Algpseudocode, "textproc") => {
                            argument(b, e, end).map(|(s, ae, after)| {
                                flush(self, &mut run, i, out);
                                let (ts, te) = trim(self.text, s, ae);
                                if te > ts {
                                    out.push(Piece::StyledSource {
                                        face: Face::SmallCaps,
                                        span: self.span(ts, te),
                                    });
                                }
                                after
                            })
                        }
                        (_, "hfill") => {
                            flush(self, &mut run, i, out);
                            out.push(Piece::HFill { span: command });
                            Some(e)
                        }
                        (_, kw)
                            if kw.starts_with("algorithmic")
                                && (self.setup.keywords.contains_key(kw)
                                    || algorithmic_default(kw).is_some()
                                    || algpseudocode_default(kw).is_some()) =>
                        {
                            flush(self, &mut run, i, out);
                            self.keyword(kw, command, out);
                            let mut j = e;
                            if self.text[j..end].starts_with("{}") {
                                j += 2;
                            }
                            Some(j)
                        }
                        _ => None,
                    };
                    match handled {
                        Some(after) => {
                            i = after;
                            // After a braced argument whitespace is real; after a bare
                            // control word TeX skips it.
                            after_control_word = self.b.get(after.wrapping_sub(1)) != Some(&b'}');
                        }
                        None => {
                            run.get_or_insert(i);
                            i = e;
                            // Spaces after an unknown control word belong to the run;
                            // the adapter/parser skips them as TeX does.
                        }
                    }
                }
                _ => {
                    run.get_or_insert(i);
                    i += 1;
                    after_control_word = false;
                }
            }
        }
        flush(self, &mut run, end, out);
        while matches!(out.last(), Some(Piece::Space { .. })) {
            out.pop();
        }
    }

    /// `\Call{name}{args}` and the head of `\Procedure`: `\textproc{name}` and
    /// `(args)` unless `args` is empty (algpseudocode.sty 41-49, 60).
    fn call(
        &mut self,
        command: Span,
        name: (usize, usize),
        args: (usize, usize),
        out: &mut Vec<Piece>,
    ) {
        let (ns, ne) = trim(self.text, name.0, name.1);
        if ne > ns {
            out.push(Piece::StyledSource {
                face: Face::SmallCaps,
                span: self.span(ns, ne),
            });
        }
        if args.1 > args.0 {
            out.push(Piece::Text {
                text: "(".into(),
                face: Face::Roman,
                span: command,
            });
            self.inline(args.0, args.1, out);
            out.push(Piece::Text {
                text: ")".into(),
                face: Face::Roman,
                span: command,
            });
        }
    }
}

/// `ALC@line`/`ALC@rem` (algorithmic.sty 146-152) and `ALG@line`/`ALG@rem`
/// (algorithmicx.sty 66-73): every numbered item steps both; the number is
/// printed when the frequency counter reaches `\begin{algorithmic}[n]`'s n.
#[derive(Default)]
struct LineCounter {
    line: u32,
    rem: u32,
}

impl LineCounter {
    fn step(&mut self, frequency: u32) -> (u32, bool) {
        self.line += 1;
        self.rem += 1;
        let show = frequency != 0 && self.rem == frequency;
        if show {
            self.rem = 0;
        }
        (self.line, show)
    }
}

/// Parses `\begin{algorithmic}[n] ... \end{algorithmic}` starting at `begin`
/// (the `\begin` backslash). Returns the environment and the byte after
/// `\end{algorithmic}`; `None` when the environment is not closed.
pub fn parse_algorithmic(
    text: &str,
    document: DocumentId,
    setup: &Setup,
    begin: usize,
) -> Option<(Algorithmic, usize)> {
    let b = text.as_bytes();
    let open_end = begin + "\\begin{algorithmic}".len();
    let end_tag = "\\end{algorithmic}";
    let end = find_uncommented(text, end_tag, open_end)?;
    let dialect = setup.dialect.unwrap_or(Dialect::Algorithmic);
    let mut r = Reader {
        text,
        b,
        document,
        setup,
        dialect,
        problems: Vec::new(),
    };
    let mut i = open_end;
    let mut frequency = 0u32;
    let j = skip_ws(b, i, end);
    if let Some(e) = bracket_end(b, j, end) {
        let arg = text[j + 1..e - 1].trim();
        match arg.parse::<u32>() {
            Ok(n) => frequency = n,
            Err(_) => r.problems.push((
                r.span(j, e),
                format!("line numbering frequency '{arg}' is not a number; no lines are numbered"),
            )),
        }
        i = e;
    }
    let mut lines: Vec<Line> = Vec::new();
    let mut stack: Vec<Block> = Vec::new();
    let mut counter = LineCounter::default();
    // The line whose inline material is being read: (index, start byte).
    let mut open: Option<(usize, usize)> = None;
    let finish =
        |r: &mut Reader, lines: &mut Vec<Line>, open: &mut Option<(usize, usize)>, upto: usize| {
            if let Some((idx, s)) = open.take() {
                let mut pieces = std::mem::take(&mut lines[idx].pieces);
                let had = !pieces.is_empty();
                let mut tail = Vec::new();
                r.inline(s, upto, &mut tail);
                if had
                    && !tail.is_empty()
                    && !matches!(
                        pieces.last(),
                        Some(Piece::Space { .. } | Piece::HFill { .. })
                    )
                    && !matches!(tail.first(), Some(Piece::Space { .. }))
                {
                    // Whitespace after a braced argument (`\If{c} text`) is real.
                    let gap = &r.text[s..upto];
                    if gap.starts_with([' ', '\n', '\t']) {
                        pieces.push(Piece::Space { face: Face::Roman });
                    }
                }
                pieces.extend(tail);
                while matches!(pieces.last(), Some(Piece::Space { .. })) {
                    pieces.pop();
                }
                lines[idx].pieces = pieces;
            }
        };
    let close = |r: &mut Reader, stack: &mut Vec<Block>, want: Option<Block>, at: Span| {
        let Some(want) = want else { return };
        match stack.last() {
            Some(top) if *top == want || (want == Block::If && *top == Block::Else) => {
                stack.pop();
            }
            Some(_) => {
                r.problems.push((
                    at,
                    "this command closes a block that is not the innermost open one".into(),
                ));
                stack.pop();
            }
            None => r
                .problems
                .push((at, "this command closes a block that is not open".into())),
        }
    };
    while i < end {
        let Some(rel) = text[i..end].find(['\\', '%', '$', '{']) else {
            break;
        };
        let pos = i + rel;
        match b[pos] {
            b'%' => {
                i = text[pos..end].find('\n').map_or(end, |n| pos + n + 1);
                continue;
            }
            b'$' => {
                let double = b.get(pos + 1) == Some(&b'$');
                let mut j = pos + if double { 2 } else { 1 };
                while j < end && b[j] != b'$' {
                    if b[j] == b'\\' {
                        j += 1;
                    }
                    j += 1;
                }
                i = (j + if double { 2 } else { 1 }).min(end);
                continue;
            }
            b'{' => {
                i = group_end(b, pos, end).unwrap_or(end);
                continue;
            }
            _ => {}
        }
        let name_end = control_word_end(b, pos, end);
        if name_end == pos + 1 {
            i = (pos + 2).min(end);
            continue;
        }
        let name = &text[pos + 1..name_end];
        let command = r.span(pos, name_end);
        // A `\begin{..}` nested in statement text is skipped whole.
        if name == "begin" {
            if let Some((ns, ne, after)) = argument(b, name_end, end) {
                let env = &text[ns..ne];
                let tag = format!("\\end{{{env}}}");
                i = find_uncommented(text, &tag, after)
                    .filter(|p| *p < end)
                    .map_or(after, |p| p + tag.len());
                continue;
            }
        }
        let stmt = match dialect {
            Dialect::Algorithmic => algorithmic_stmt(name, setup.noend),
            Dialect::Algpseudocode => algpseudocode_stmt(name),
        };
        let label_command = setup.label_commands.get(name).cloned();
        if stmt.is_none() && label_command.is_none() {
            i = name_end;
            continue;
        }
        if open.is_none() && lines.is_empty() {
            let (ts, te) = trim(text, open_end, pos);
            let lead = &text[ts..te];
            if !lead.is_empty() && !lead.starts_with('[') {
                r.problems.push((
                    r.span(ts, te),
                    "text before the first statement (LaTeX: missing \\item)".into(),
                ));
            }
        }
        finish(&mut r, &mut lines, &mut open, pos);
        let depth = |stack: &Vec<Block>| stack.len() as u32;
        let push_line = |lines: &mut Vec<Line>,
                         kind: LineKind,
                         number: u32,
                         show: bool,
                         depth: u32,
                         pieces: Vec<Piece>,
                         hskip: bool| {
            lines.push(Line {
                kind,
                number,
                show_number: show,
                depth,
                hskip_tlm: hskip,
                pieces,
                span: command,
            });
            lines.len() - 1
        };
        let algx = dialect == Dialect::Algpseudocode;
        if let Some(tpl) = label_command {
            let mut label = Vec::new();
            r.instantiate(&tpl, command, &mut label, 0);
            let idx = push_line(
                &mut lines,
                LineKind::Labelled(label),
                counter.line,
                false,
                depth(&stack),
                Vec::new(),
                false,
            );
            open = Some((idx, name_end));
            i = name_end;
            continue;
        }
        let stmt = stmt.expect("checked");
        let mut next = name_end;
        match stmt {
            Stmt::State => {
                let (n, show) = counter.step(frequency);
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    Vec::new(),
                    algx,
                );
                open = Some((idx, name_end));
            }
            Stmt::Statex => {
                let idx = push_line(
                    &mut lines,
                    LineKind::Unlabelled,
                    counter.line,
                    false,
                    depth(&stack),
                    Vec::new(),
                    false,
                );
                open = Some((idx, name_end));
            }
            Stmt::Keyword2Spaces(kw) => {
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword(kw, command, &mut pieces);
                pieces.push(Piece::Space { face: Face::Roman });
                pieces.push(Piece::Space { face: Face::Roman });
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    false,
                );
                open = Some((idx, name_end));
                // The two spaces are fixed; the source's leading blanks are skipped.
                next = skip_ws(b, name_end, end);
                open = open.map(|(k, _)| (k, next));
            }
            Stmt::Label(kw) => {
                let mut label = Vec::new();
                r.keyword(kw, command, &mut label);
                let idx = push_line(
                    &mut lines,
                    LineKind::Labelled(label),
                    counter.line,
                    false,
                    depth(&stack),
                    Vec::new(),
                    false,
                );
                open = Some((idx, name_end));
            }
            Stmt::Cond {
                close: to_close,
                open: block,
                head,
                tail,
            } => {
                let mut j = name_end;
                let mut comment = None;
                if dialect == Dialect::Algorithmic {
                    let k = skip_ws(b, j, end);
                    if let Some(e) = bracket_end(b, k, end) {
                        comment = Some((k + 1, e - 1));
                        j = e;
                    }
                }
                let Some((cs, ce, after)) = argument(b, j, end) else {
                    r.problems
                        .push((command, format!("\\{name} is missing its condition")));
                    i = name_end;
                    continue;
                };
                close(&mut r, &mut stack, to_close, command);
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword(head, command, &mut pieces);
                pieces.push(Piece::Space { face: Face::Roman });
                r.inline(cs, ce, &mut pieces);
                if let Some(tail) = tail {
                    pieces.push(Piece::Space { face: Face::Roman });
                    r.keyword(tail, command, &mut pieces);
                }
                if let Some(c) = comment {
                    r.algorithmic_comment(command, c, &mut pieces);
                }
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    algx,
                );
                stack.push(block);
                open = Some((idx, after));
                next = after;
            }
            Stmt::Bare {
                close: to_close,
                open: block,
                head,
            } => {
                let mut j = name_end;
                let mut comment = None;
                if dialect == Dialect::Algorithmic {
                    let k = skip_ws(b, j, end);
                    if let Some(e) = bracket_end(b, k, end) {
                        comment = Some((k + 1, e - 1));
                        j = e;
                    }
                }
                close(&mut r, &mut stack, to_close, command);
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword(head, command, &mut pieces);
                if let Some(c) = comment {
                    r.algorithmic_comment(command, c, &mut pieces);
                }
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    algx,
                );
                stack.push(block);
                open = Some((idx, j));
                next = j;
            }
            Stmt::Until => {
                let Some((cs, ce, after)) = argument(b, name_end, end) else {
                    r.problems
                        .push((command, format!("\\{name} is missing its condition")));
                    i = name_end;
                    continue;
                };
                close(&mut r, &mut stack, Some(Block::Repeat), command);
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword("algorithmicuntil", command, &mut pieces);
                pieces.push(Piece::Space { face: Face::Roman });
                r.inline(cs, ce, &mut pieces);
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    algx,
                );
                open = Some((idx, after));
                next = after;
            }
            Stmt::End {
                close: block,
                keyword,
            } => {
                close(&mut r, &mut stack, Some(block), command);
                if dialect == Dialect::Algorithmic {
                    if !keyword.is_empty() {
                        let (n, show) = counter.step(frequency);
                        let mut pieces = Vec::new();
                        r.keyword(keyword, command, &mut pieces);
                        let idx = push_line(
                            &mut lines,
                            LineKind::Numbered,
                            n,
                            show,
                            depth(&stack),
                            pieces,
                            false,
                        );
                        open = Some((idx, name_end));
                    }
                } else if setup.noend {
                    // `\algtext*{EndIf}` (algpseudocode.sty 50-58): no text.
                    let idx = push_line(
                        &mut lines,
                        LineKind::NoText,
                        counter.line,
                        false,
                        depth(&stack),
                        Vec::new(),
                        false,
                    );
                    open = Some((idx, name_end));
                } else {
                    let (n, show) = counter.step(frequency);
                    let mut pieces = Vec::new();
                    r.keyword(keyword, command, &mut pieces);
                    let idx = push_line(
                        &mut lines,
                        LineKind::Numbered,
                        n,
                        show,
                        depth(&stack),
                        pieces,
                        true,
                    );
                    open = Some((idx, name_end));
                }
            }
            Stmt::Group { head } => {
                let k = skip_ws(b, name_end, end);
                let mut j = name_end;
                let mut comment = None;
                if let Some(e) = bracket_end(b, k, end) {
                    comment = Some((k + 1, e - 1));
                    j = e;
                }
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword(head, command, &mut pieces);
                pieces.push(Piece::Space { face: Face::Roman });
                if let Some(c) = comment {
                    r.algorithmic_comment(command, c, &mut pieces);
                }
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    false,
                );
                stack.push(Block::Group);
                open = Some((idx, j));
                next = j;
            }
            Stmt::Globals => {
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword("algorithmicglobals", command, &mut pieces);
                pieces.push(Piece::Space { face: Face::Roman });
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    false,
                );
                open = Some((idx, name_end));
            }
            Stmt::EndGroup => {
                close(&mut r, &mut stack, Some(Block::Group), command);
            }
            Stmt::Proc { open: block, head } => {
                let Some((ns, ne, after)) = argument(b, name_end, end) else {
                    r.problems
                        .push((command, format!("\\{name} is missing its name")));
                    i = name_end;
                    continue;
                };
                let Some((as_, ae, after2)) = argument(b, after, end) else {
                    r.problems
                        .push((command, format!("\\{name} is missing its arguments")));
                    i = after;
                    continue;
                };
                let (n, show) = counter.step(frequency);
                let mut pieces = Vec::new();
                r.keyword(head, command, &mut pieces);
                pieces.push(Piece::Space { face: Face::Roman });
                r.call(command, (ns, ne), (as_, ae), &mut pieces);
                let idx = push_line(
                    &mut lines,
                    LineKind::Numbered,
                    n,
                    show,
                    depth(&stack),
                    pieces,
                    true,
                );
                stack.push(block);
                open = Some((idx, after2));
                next = after2;
            }
        }
        i = next;
    }
    finish(&mut r, &mut lines, &mut open, end);
    if dialect == Dialect::Algorithmic && !stack.is_empty() {
        r.problems.push((
            r.span(begin, end + end_tag.len()),
            "some blocks are not closed".into(),
        ));
    }
    Some((
        Algorithmic {
            dialect,
            frequency,
            lines,
            span: r.span(begin, end + end_tag.len()),
            problems: r.problems,
        },
        end + end_tag.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(pieces: &[Piece], src: &str) -> String {
        pieces
            .iter()
            .map(|p| match p {
                Piece::Text {
                    text,
                    face: Face::Bold,
                    ..
                } => format!("*{text}*"),
                Piece::Text { text, .. } => text.clone(),
                Piece::Source(s) => format!("<{}>", &src[s.start..s.end]),
                Piece::StyledSource { span, .. } => format!("sc<{}>", &src[span.start..span.end]),
                Piece::Space { face: Face::Bold } => "_".into(),
                Piece::Space { .. } => " ".into(),
                Piece::HFill { .. } => "~fill~".into(),
                Piece::MathSymbol { tex, .. } => format!("[{tex}]"),
            })
            .collect()
    }

    fn one(src: &str) -> Algorithmic {
        let s = setup(src);
        match scan(src, DocumentId(0), &s).into_iter().next() {
            Some(Found::Bare(a)) => a,
            Some(Found::Float(f)) => f
                .body
                .into_iter()
                .find_map(|i| {
                    if let FloatItem::Algorithmic(a) = i {
                        Some(a)
                    } else {
                        None
                    }
                })
                .unwrap(),
            None => panic!("nothing found"),
        }
    }

    #[test]
    fn algorithmic_statements_nesting_and_numbering() {
        let src = "\\usepackage{algorithmic}\\begin{document}\n\\begin{algorithmic}[2]\n\\REQUIRE $a \\ge 0$\n\\STATE $x \\gets a$\n\\WHILE{$b \\neq 0$}\n\\IF[check]{$a > b$}\n\\STATE swap \\COMMENT{keep order}\n\\ELSE\n\\RETURN $b$\n\\ENDIF\n\\ENDWHILE\n\\end{algorithmic}\n\\end{document}";
        let a = one(src);
        let got: Vec<(String, u32, u32, bool)> = a
            .lines
            .iter()
            .map(|l| (texts(&l.pieces, src), l.depth, l.number, l.show_number))
            .collect();
        assert_eq!(
            got,
            vec![
                ("<$a \\ge 0$>".into(), 0, 0, false),
                ("<$x \\gets a$>".into(), 0, 1, false),
                ("*while* <$b \\neq 0$> *do*".into(), 0, 2, true),
                (
                    "*if* <$a > b$> *then* [\\{]<check>[\\}]".into(),
                    1,
                    3,
                    false
                ),
                ("<swap> [\\{]<keep order>[\\}]".into(), 2, 4, true),
                ("*else*".into(), 1, 5, false),
                ("*return*  <$b$>".into(), 2, 6, true),
                ("*end* *if*".into(), 1, 7, false),
                ("*end* *while*".into(), 0, 8, true),
            ]
        );
        assert!(matches!(&a.lines[0].kind, LineKind::Labelled(l) if texts(l, src) == "*Require:*"));
        assert!(a.problems.is_empty(), "{:?}", a.problems);
    }

    #[test]
    fn algpseudocode_procedures_calls_comments_and_noend() {
        let src = "\\usepackage[noend]{algpseudocode}\n\\algrenewcommand\\algorithmicrequire{\\textbf{Input:}}\n\\begin{document}\n\\begin{algorithmic}[1]\n\\Require $n$\n\\Procedure{Euclid}{$a,b$}\\Comment{The g.c.d.}\n\\ForAll{$v \\in V$}\n\\State \\Call{Visit}{$v$} and \\Return $b$\n\\EndFor\n\\Statex\n\\EndProcedure\n\\end{algorithmic}\n\\end{document}";
        let a = one(src);
        let got: Vec<String> = a
            .lines
            .iter()
            .map(|l| format!("{}|{}|{}", texts(&l.pieces, src), l.depth, l.number))
            .collect();
        assert_eq!(
            got,
            vec![
                "<$n$>|0|0",
                "*procedure* sc<Euclid>(<$a,b$>)~fill~[\\triangleright] <The g.c.d.>|0|1",
                "*for*_*all* <$v \\in V$> *do*|1|2",
                "sc<Visit>(<$v$>) <and> *return* <$b$>|2|3",
                "|1|3",
                "|1|3",
                "|0|3",
            ]
        );
        assert!(matches!(&a.lines[0].kind, LineKind::Labelled(l) if texts(l, src) == "*Input:*"));
        assert_eq!(a.lines[4].kind, LineKind::NoText);
        assert_eq!(a.lines[5].kind, LineKind::Unlabelled);
        assert!(a.lines[3].hskip_tlm && !a.lines[5].hskip_tlm);
    }

    #[test]
    fn float_setup_caption_and_labels() {
        let src = "\\usepackage[plain,section]{algorithm}\\usepackage{algpseudocode}\n\\floatname{algorithm}{Procedure}\n\\algrenewcommand\\algorithmicindent{2em}\n\\algnewcommand\\Input{\\item[\\textbf{Input:}]}\n\\begin{document}\nText\n\n\\begin{algorithm}[t]\n\\small\n\\caption{Search}\\label{alg:s}\n\\begin{algorithmic}\n\\Input graph\n\\State x\n\\end{algorithmic}\n\\end{algorithm}\n\\end{document}";
        let s = setup(src);
        assert_eq!(
            (
                s.float_style,
                s.float_name.as_str(),
                s.within.as_deref(),
                s.indent
            ),
            (
                FloatStyle::Plain,
                "Procedure",
                Some("section"),
                Dimen::Em(2.0)
            )
        );
        let found = scan(src, DocumentId(0), &s);
        let Found::Float(f) = &found[0] else { panic!() };
        assert_eq!((f.placement.as_deref(), f.hmode), (Some("t"), false));
        assert!(matches!(
            f.body[0],
            FloatItem::Size {
                size: Some(FontSizeLevel::Small),
                ..
            }
        ));
        assert!(
            matches!(&f.body[1], FloatItem::Caption { arg, .. } if &src[arg.start..arg.end] == "Search")
        );
        assert!(matches!(&f.body[2], FloatItem::Label { key, .. } if key == "alg:s"));
        let FloatItem::Algorithmic(a) = &f.body[3] else {
            panic!()
        };
        assert_eq!(a.frequency, 0);
        assert!(matches!(&a.lines[0].kind, LineKind::Labelled(l) if texts(l, src) == "*Input:*"));
        assert_eq!(texts(&a.lines[0].pieces, src), "<graph>");
    }

    #[test]
    fn renewcommand_keyword_and_line_number_format() {
        let src = "\\usepackage{algorithmic}\n\\renewcommand{\\algorithmicrequire}{\\textbf{Input:}}\n\\renewcommand\\algorithmicensure{\\textbf{Output:}}\n\\algsetup{indent=2em,linenodelimiter=.}\n\\begin{document}\\begin{algorithmic}[1]\\ENSURE y \\FORALL{$i$} \\STATE \\TRUE\\ \\AND\\ x\\ENDFOR\\end{algorithmic}\\end{document}";
        let s = setup(src);
        assert_eq!(
            (s.indent, s.line_numbers.suffix.as_str()),
            (Dimen::Em(2.0), ".")
        );
        let a = one(src);
        assert!(matches!(&a.lines[0].kind, LineKind::Labelled(l) if texts(l, src) == "*Output:*"));
        // `\textbf{for all}`: the space between the words is the bold font's.
        assert_eq!(texts(&a.lines[1].pieces, src), "*for*_*all* <$i$> *do*");
        // `\TRUE\ \AND\ x`: the control spaces, and `\AND`'s own `{} ` space.
        assert_eq!(texts(&a.lines[2].pieces, src), "*true* *and*  <x>");
    }

    #[test]
    fn dimen_and_line_number_formats() {
        assert_eq!(Dimen::parse("1.5em"), Some(Dimen::Em(1.5)));
        assert_eq!(Dimen::parse(" 10pt"), Some(Dimen::Pt(10.0)));
        let f = line_number_format("\\tiny #1.").unwrap();
        assert_eq!(
            (f.size, f.suffix.as_str()),
            (Some(FontSizeLevel::Tiny), ".")
        );
        assert!(line_number_format("\\textbf{#1}").is_none());
    }
}
