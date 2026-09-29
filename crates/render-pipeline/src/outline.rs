//! hyperref bookmarks: the PDF document outline pdflatex writes for a
//! document that loads `hyperref` (whose `bookmarks` option is on by
//! default).
//!
//! hyperref writes one `\BOOKMARK` line to `\jobname.out` per
//! `\Hy@writebookmark` (hpdftex.def 1604-1673) and pdfTeX turns the file
//! into the `/Outlines` tree on the next run. A bookmark is written by
//!
//! - every `\addcontentsline{toc}{<level>}{<entry>}` -- so every unstarred
//!   `\part`, `\chapter` and `\section` .. `\subsubsection` the class writes
//!   to the `.toc`, and any the author writes, typically after
//!   `\phantomsection` -- with the destination `\@currentHref`, the last
//!   anchor set;
//! - `\pdfbookmark[<level>]{<text>}{<name>}` (destination `<name>.<level>`)
//!   and its relatives `\currentpdfbookmark`, `\subpdfbookmark` and
//!   `\belowpdfbookmark`, which take their level from the last bookmark's.
//!
//! A bookmark deeper than `bookmarksdepth` (default: `\c@tocdepth`) is not
//! written; one more than a level below the previous one is lifted to
//! just below it ("Difference ... between bookmark levels is greater than
//! one, level fixed"). The text is `\pdfstringdef` of the entry: formatting
//! and math shifts dropped, `\texorpdfstring`'s second argument, text
//! symbols and accents as Unicode, the number (`\numberline`) only under
//! `bookmarksnumbered`. The pipeline keeps the same records in memory
//! ([`Collector`]): each anchor is a synthetic `\label` ([`key`]) placed
//! where hyperref's `\hyper@anchorstart` stands, and its position on the
//! laid-out page (`typeset::anchor_positions`) becomes an `/XYZ`
//! destination.
//!
//! Beamer is left out: its outline (frames and `\section`s through
//! `beamerbasenavigation`) is a separate model.

use std::collections::{BTreeMap, BTreeSet};

use crate::links::Destination;

/// Prefix of the synthetic label keys that mark anchor positions; `\label`
/// keys cannot contain U+0001, and [`crate::toc::KEY_PREFIX`] differs.
const KEY_PREFIX: &str = "\u{1}ol:";

/// Bounds kept well inside the PDF writer's own limits
/// (`flashtex_pdf::navigation`: 1024-byte names, 64 KiB strings, 100 000
/// outline items), which refuse the whole document when exceeded.
const MAX_NAME_BYTES: usize = 256;
const MAX_TITLE_BYTES: usize = 4096;
const MAX_ENTRIES: usize = 50_000;

pub fn is_key(key: &str) -> bool {
    key.starts_with(KEY_PREFIX)
}

/// The label key of anchor `n`.
pub fn key(n: usize) -> String {
    format!("{KEY_PREFIX}{n}")
}

/// Suffix of an anchor that stands at the top of its page's text area
/// whatever line the label lands on: `\@chapter`'s `\refstepcounter` runs
/// before the heading's `\vspace*`.
const TOP_SUFFIX: &str = ":top";

/// Suffix of an anchor `\Hy@raisedlink`ed a `\baselineskip` above the
/// first line of its block: a starred head's (hyperref sets it inside the
/// head's paragraph, in horizontal mode), and report/book `\part`'s, which
/// follows the `\null` box its page starts with.
const RAISED_SUFFIX: &str = ":raised";

pub fn is_page_top(key: &str) -> bool {
    is_key(key) && key.ends_with(TOP_SUFFIX)
}

pub fn is_raised(key: &str) -> bool {
    is_key(key) && key.ends_with(RAISED_SUFFIX)
}

/// Suffix of an anchor set after the last block (nothing follows it
/// before `\end{document}`): it stands below that block's last line and
/// the skip after it.
const AFTER_SUFFIX: &str = ":after";

pub fn is_after(key: &str) -> bool {
    is_key(key) && key.ends_with(AFTER_SUFFIX)
}

/// hyperref's bookmark options, from `\usepackage[..]{hyperref}` and every
/// `\hypersetup{..}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `bookmarksnumbered`: the `\numberline` number and a space lead the
    /// text.
    pub numbered: bool,
    /// `bookmarksopen`.
    pub open: bool,
    /// `bookmarksopenlevel` (`None`: hyperref's `\maxdimen`).
    pub open_level: Option<i32>,
    /// `bookmarksdepth` (`None`: `\c@tocdepth`).
    pub depth: Option<i32>,
    /// `bookmarks` (a load-time option: hyperref disables it once the
    /// package is loaded).
    pub bookmarks: bool,
    /// The catalog's `/PageMode`: `pdfpagemode` when given, else
    /// `UseOutlines` with bookmarks and `UseNone` without (hyperref.sty
    /// 4467-4481), whether or not any bookmark is written.
    pub page_mode: String,
}

/// Whether `option` (a key of a key-value list) is a true boolean.
fn truth(value: Option<&str>) -> Option<bool> {
    match value.map(str::trim) {
        None | Some("true") | Some("") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

/// A hyperref level name or number (`bookmarksdepth=subsection`, `=2`).
fn level_value(value: &str) -> Option<i32> {
    let v = value.trim();
    v.parse::<i32>().ok().or_else(|| crate::toc::level_of(v).map(i32::from))
}

impl Settings {
    /// `None` when the document does not load hyperref. With
    /// `bookmarks=false` there is no `.out` file and no outline, but still
    /// a `/PageMode`.
    pub fn read(source: &str) -> Option<Settings> {
        let code = strip_comments(source);
        let preamble = &code[..code.find("\\begin{document}").unwrap_or(code.len())];
        let options = crate::adapter::package_options(preamble, "hyperref")?;
        let mut settings = Settings { numbered: false, open: false, open_level: None, depth: None, bookmarks: true, page_mode: String::new() };
        for option in split_options(&options) {
            let (name, value) = match option.split_once('=') {
                Some((n, v)) => (n.trim(), Some(v.trim())),
                None => (option.trim(), None),
            };
            if name == "bookmarks" {
                settings.bookmarks = truth(value).unwrap_or(settings.bookmarks);
            }
        }
        let mut page_mode = None;
        let mut lists = vec![options];
        let mut from = 0;
        while let Some(at) = code[from..].find("\\hypersetup") {
            let open = from + at + "\\hypersetup".len();
            from = open;
            let rest = &code[open..];
            let skip = rest.len() - rest.trim_start().len();
            if let Some(close) = group_end(&code, open + skip) {
                lists.push(code[open + skip + 1..close].to_string());
                from = close;
            }
        }
        for list in &lists {
            for option in split_options(list) {
                let (name, value) = match option.split_once('=') {
                    Some((n, v)) => (n.trim(), Some(v.trim())),
                    None => (option.trim(), None),
                };
                match name {
                    "pdfpagemode" => page_mode = value.map(|v| v.trim_matches(|c| c == '{' || c == '}' || c == ' ').to_string()).filter(|v| !v.is_empty()),
                    "bookmarksnumbered" => settings.numbered = truth(value).unwrap_or(settings.numbered),
                    "bookmarksopen" => settings.open = truth(value).unwrap_or(settings.open),
                    "bookmarksopenlevel" => {
                        settings.open_level = match value {
                            Some(v) if v == "\\maxdimen" => None,
                            Some(v) => level_value(v).or(settings.open_level),
                            None => None,
                        }
                    }
                    "bookmarksdepth" => {
                        settings.depth = match value {
                            None | Some("") => None,
                            Some(v) => level_value(v).or(settings.depth),
                        }
                    }
                    _ => {}
                }
            }
        }
        settings.page_mode = page_mode.unwrap_or_else(|| if settings.bookmarks { "UseOutlines" } else { "UseNone" }.to_string());
        Some(settings)
    }
}

/// A key-value list split at top-level commas.
fn split_options(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in list.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    out.push(current);
    out.into_iter().map(|o| o.trim().trim_matches(|c| c == '{' || c == '}').to_string()).filter(|o| !o.is_empty()).collect()
}

/// `source` with every `%` comment (not `\%`) blanked to the end of its
/// line, lengths kept.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_comment = false;
    let mut escaped = false;
    for c in source.chars() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
                out.push('\n');
            } else {
                out.extend(std::iter::repeat_n(' ', c.len_utf8()));
            }
            continue;
        }
        if c == '%' && !escaped {
            in_comment = true;
            out.push(' ');
            continue;
        }
        escaped = c == '\\' && !escaped;
        out.push(c);
    }
    out
}

/// The `}` closing the group opened at byte `open` (which must be `{`).
fn group_end(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The `]` closing the optional argument opened at byte `open` (`[`),
/// skipping braced groups.
fn bracket_end(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b']' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// The arguments of a sectioning command whose control word starts at
/// `at`: `(starred, [short], {title})` inner byte ranges, when the bytes
/// there really are `\<name>` (a heading a macro produced is not).
pub fn heading_arguments(source: &str, at: usize, name: &str) -> Option<(bool, Option<(usize, usize)>, (usize, usize))> {
    let rest = source.get(at..)?;
    let word = format!("\\{name}");
    if !rest.starts_with(&word) || rest[word.len()..].starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut k = at + word.len();
    let skip = |k: usize| k + source[k..].len() - source[k..].trim_start().len();
    k = skip(k);
    let starred = source.as_bytes().get(k) == Some(&b'*');
    if starred {
        k = skip(k + 1);
    }
    let mut short = None;
    if let Some(close) = bracket_end(source, k) {
        short = Some((k + 1, close));
        k = skip(close + 1);
    }
    let close = group_end(source, k)?;
    Some((starred, short, (k + 1, close)))
}

/// A user macro: `\newcommand{\name}[params][default]{body}` or
/// `\def\name{body}`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Macro {
    params: usize,
    default: Option<String>,
    body: String,
}

/// What `\pdfstringdef` needs besides the text: the document's own macros
/// and the `\ref` values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Strings {
    macros: BTreeMap<String, Macro>,
    refs: BTreeMap<String, String>,
}

impl Strings {
    /// The macros every document of the project defines (a later
    /// definition replaces an earlier one), and the label values.
    pub fn new(texts: &[&str], refs: &BTreeMap<String, String>) -> Strings {
        let mut macros = BTreeMap::new();
        for text in texts {
            collect_macros(&strip_comments(text), &mut macros);
        }
        Strings { macros, refs: refs.clone() }
    }

    /// `\pdfstringdef` of `text`, trimmed.
    pub fn pdf_string(&self, text: &str) -> String {
        let mut expanded = String::new();
        self.expand(&strip_comments(text), 0, &mut expanded);
        let mut out = String::new();
        reduce(&expanded, self, &mut out);
        out.trim().to_string()
    }

    /// Replaces user macros by their bodies (parameters substituted), up
    /// to a fixed depth so a recursive definition terminates.
    fn expand(&self, text: &str, depth: usize, out: &mut String) {
        const MAX_DEPTH: usize = 16;
        const MAX_BYTES: usize = 64 * 1024;
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if out.len() > MAX_BYTES {
                return;
            }
            if bytes[i] != b'\\' {
                let c = text[i..].chars().next().unwrap_or(' ');
                out.push(c);
                i += c.len_utf8();
                continue;
            }
            let name_end = control_word_end(bytes, i);
            let name = &text[i + 1..name_end];
            let Some(m) = self.macros.get(name).filter(|_| depth < MAX_DEPTH && name_end > i + 1) else {
                // A control symbol or unknown word: copied (the symbol's
                // character too, so `\\%` stays escaped).
                let end = if name_end == i + 1 { (i + 1 + text[i + 1..].chars().next().map_or(0, char::len_utf8)).min(text.len()) } else { name_end };
                out.push_str(&text[i..end]);
                i = end;
                continue;
            };
            let mut k = skip_spaces(text, name_end);
            let mut args: Vec<String> = Vec::new();
            if m.params > 0 {
                if let Some(default) = &m.default {
                    match bracket_end(text, k) {
                        Some(close) => {
                            args.push(text[k + 1..close].to_string());
                            k = close + 1;
                        }
                        None => args.push(default.clone()),
                    }
                }
                while args.len() < m.params {
                    k = skip_spaces(text, k);
                    match argument(text, k) {
                        Some((arg, next)) => {
                            args.push(arg.to_string());
                            k = next;
                        }
                        None => break,
                    }
                }
            } else {
                // A control word's trailing spaces were never tokens.
                k = skip_spaces(text, name_end);
            }
            let mut body = String::new();
            let b = m.body.as_bytes();
            let mut j = 0;
            while j < b.len() {
                if b[j] == b'#' && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
                    let n = usize::from(b[j + 1] - b'0');
                    if let Some(a) = n.checked_sub(1).and_then(|n| args.get(n)) {
                        body.push_str(a);
                    }
                    j += 2;
                    continue;
                }
                let c = m.body[j..].chars().next().unwrap_or(' ');
                body.push(c);
                j += c.len_utf8();
            }
            self.expand(&body, depth + 1, out);
            // An empty group (which reduces to nothing) keeps a following
            // letter from joining a body that ends in a control word.
            out.push_str("{}");
            i = k;
        }
    }
}

fn skip_spaces(text: &str, from: usize) -> usize {
    from + text[from..].len() - text[from..].trim_start().len()
}

/// The end of the control sequence starting with the `\` at `i`: after its
/// letters, or `i + 1` for a control symbol.
fn control_word_end(bytes: &[u8], i: usize) -> usize {
    let mut j = i + 1;
    while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
        j += 1;
    }
    j
}

/// One undelimited macro argument at `k`: a braced group's inside, or one
/// token. Returns the argument text and the byte after it.
fn argument(text: &str, k: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    match bytes.get(k)? {
        b'{' => {
            let close = group_end(text, k)?;
            Some((&text[k + 1..close], close + 1))
        }
        b'\\' => {
            let end = control_word_end(bytes, k);
            let end = if end == k + 1 { (k + 1 + text[k + 1..].chars().next().map_or(0, char::len_utf8)).min(text.len()) } else { end };
            Some((&text[k..end], end))
        }
        _ => {
            let c = text[k..].chars().next()?;
            Some((&text[k..k + c.len_utf8()], k + c.len_utf8()))
        }
    }
}

fn collect_macros(code: &str, macros: &mut BTreeMap<String, Macro>) {
    let bytes = code.as_bytes();
    let mut i = 0;
    while let Some(at) = code[i..].find('\\') {
        let start = i + at;
        let end = control_word_end(bytes, start);
        i = end.max(start + 1);
        let word = &code[start + 1..end];
        let def = match word {
            "newcommand" | "renewcommand" | "providecommand" => {
                let mut k = end;
                if bytes.get(k) == Some(&b'*') {
                    k += 1;
                }
                k = skip_spaces(code, k);
                // `{\name}` or `\name`.
                let (name, mut k) = match bytes.get(k) {
                    Some(b'{') => {
                        let Some(close) = group_end(code, k) else { continue };
                        let inner = code[k + 1..close].trim();
                        let Some(name) = inner.strip_prefix('\\') else { continue };
                        (name.to_string(), close + 1)
                    }
                    Some(b'\\') => {
                        let e = control_word_end(bytes, k);
                        (code[k + 1..e].to_string(), e)
                    }
                    _ => continue,
                };
                k = skip_spaces(code, k);
                let mut params = 0;
                if let Some(close) = bracket_end(code, k) {
                    params = code[k + 1..close].trim().parse::<usize>().unwrap_or(0).min(9);
                    k = skip_spaces(code, close + 1);
                }
                let mut default = None;
                if let Some(close) = bracket_end(code, k) {
                    default = Some(code[k + 1..close].to_string());
                    k = skip_spaces(code, close + 1);
                }
                let Some(close) = group_end(code, k) else { continue };
                i = close + 1;
                if word == "providecommand" && macros.contains_key(&name) {
                    continue;
                }
                (name, Macro { params, default, body: code[k + 1..close].to_string() })
            }
            "def" => {
                let k = skip_spaces(code, end);
                if bytes.get(k) != Some(&b'\\') {
                    continue;
                }
                let e = control_word_end(bytes, k);
                let name = code[k + 1..e].to_string();
                // `#1#2...` undelimited parameters only.
                let mut p = e;
                let mut params = 0;
                while bytes.get(p) == Some(&b'#') && bytes.get(p + 1).is_some_and(u8::is_ascii_digit) {
                    params += 1;
                    p += 2;
                }
                let Some(close) = group_end(code, p) else { continue };
                i = close + 1;
                (name, Macro { params, default: None, body: code[p + 1..close].to_string() })
            }
            _ => continue,
        };
        if !def.0.is_empty() {
            macros.insert(def.0, def.1);
        }
    }
}

/// Text symbols `\pdfstringdef` spells out (hyperref's PD1/PU encodings).
fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "ss" => "ß",
        "SS" => "SS",
        "o" => "ø",
        "O" => "Ø",
        "aa" => "å",
        "AA" => "Å",
        "ae" => "æ",
        "AE" => "Æ",
        "oe" => "œ",
        "OE" => "Œ",
        "l" => "ł",
        "L" => "Ł",
        "i" => "ı",
        "j" => "ȷ",
        "S" | "textsection" => "§",
        "P" | "textparagraph" => "¶",
        "dag" | "textdagger" => "†",
        "ddag" | "textdaggerdbl" => "‡",
        "copyright" | "textcopyright" => "©",
        "textregistered" => "®",
        "texttrademark" => "™",
        "ldots" | "dots" | "textellipsis" => "…",
        "textendash" => "–",
        "textemdash" => "—",
        "textquoteleft" => "‘",
        "textquoteright" => "’",
        "textquotedblleft" => "“",
        "textquotedblright" => "”",
        "textbackslash" => "\\",
        "textasciitilde" => "~",
        "textasciicircum" => "^",
        "textbar" => "|",
        "textless" => "<",
        "textgreater" => ">",
        "textbullet" => "•",
        "textdegree" => "°",
        "pounds" | "textsterling" => "£",
        "euro" | "texteuro" => "€",
        "textunderscore" => "_",
        "textbraceleft" => "{",
        "textbraceright" => "}",
        "guillemotleft" | "guillemetleft" => "«",
        "guillemotright" | "guillemetright" => "»",
        "textperiodcentered" => "·",
        "textexclamdown" => "¡",
        "textquestiondown" => "¿",
        "TeX" => "TeX",
        "LaTeX" => "LaTeX",
        "LaTeXe" => "LaTeX2e",
        "space" | "nobreakspace" | "quad" | "qquad" | "enspace" | "enskip" => " ",
        _ => return None,
    })
}

/// Commands whose (first) braced argument is not text: dropped with it.
fn drops_argument(name: &str) -> bool {
    matches!(
        name,
        "label" | "index" | "glossary" | "vspace" | "color" | "pagecolor" | "phantom" | "hphantom" | "vphantom" | "footnote" | "footnotetext" | "thanks" | "nocite" | "includegraphics" | "rule" | "raisebox" | "textcolor" | "colorbox" | "href" | "hyperlink" | "hypertarget" | "setlength" | "addtolength" | "pdfbookmark" | "kern" | "hskip" | "vskip"
    )
}

/// The Unicode letter `accent` over `base` makes, for the accents LaTeX's
/// text commands name.
fn accented(accent: char, base: char) -> Option<char> {
    let (bases, composed): (&str, &str) = match accent {
        '\'' => ("AEIOUYaeiouyCcNnSsZzRrLlGg", "ÁÉÍÓÚÝáéíóúýĆćŃńŚśŹźŔŕĹĺǴǵ"),
        '`' => ("AEIOUaeiou", "ÀÈÌÒÙàèìòù"),
        '^' => ("AEIOUaeiouCcGgHhJjSsWwYy", "ÂÊÎÔÛâêîôûĈĉĜĝĤĥĴĵŜŝŴŵŶŷ"),
        '"' => ("AEIOUaeiouyY", "ÄËÏÖÜäëïöüÿŸ"),
        '~' => ("ANOanoIiUu", "ÃÑÕãñõĨĩŨũ"),
        '=' => ("AEIOUaeiou", "ĀĒĪŌŪāēīōū"),
        '.' => ("CcEeGgIZz", "ĊċĖėĠġİŻż"),
        'u' => ("AaEeGgIiOoUu", "ĂăĔĕĞğĬĭŎŏŬŭ"),
        'v' => ("CcDdEeNnRrSsTtZz", "ČčĎďĚěŇňŘřŠšŤťŽž"),
        'H' => ("OoUu", "ŐőŰű"),
        'c' => ("CcSsTtGgKkLlNnRr", "ÇçŞşŢţĢģĶķĻļŅņŖŗ"),
        'k' => ("AaEeIiUu", "ĄąĘęĮįŲų"),
        'r' => ("AaUu", "ÅåŮů"),
        _ => return None,
    };
    let at = bases.chars().position(|b| b == base)?;
    composed.chars().nth(at)
}

/// `\pdfstringdef` of already expanded text into `out`.
fn reduce(text: &str, strings: &Strings, out: &mut String) {
    let bytes = text.as_bytes();
    let mut i = 0;
    // A space token is pending (source whitespace runs are one space).
    let mut space = false;
    let flush = |out: &mut String, space: &mut bool| {
        if std::mem::take(space) {
            out.push(' ');
        }
    };
    while i < bytes.len() {
        let c = text[i..].chars().next().unwrap_or(' ');
        match c {
            c if c.is_whitespace() => {
                space = true;
                i += c.len_utf8();
            }
            '{' | '}' | '$' | '^' | '_' | '&' | '#' => {
                // Group braces, math shifts and the math/alignment
                // characters are removed.
                i += 1;
            }
            '~' => {
                flush(out, &mut space);
                out.push(' ');
                i += 1;
            }
            '-' => {
                flush(out, &mut space);
                if text[i..].starts_with("---") {
                    out.push('—');
                    i += 3;
                } else if text[i..].starts_with("--") {
                    out.push('–');
                    i += 2;
                } else {
                    out.push('-');
                    i += 1;
                }
            }
            '\\' => {
                let end = control_word_end(bytes, i);
                if end == i + 1 {
                    // A control symbol.
                    let Some(s) = text[i + 1..].chars().next() else { break };
                    let after = i + 1 + s.len_utf8();
                    match s {
                        '&' | '%' | '#' | '$' | '_' | '{' | '}' => {
                            flush(out, &mut space);
                            out.push(s);
                            i = after;
                        }
                        ' ' | '\n' | '\t' => {
                            flush(out, &mut space);
                            out.push(' ');
                            i = after;
                        }
                        '\'' | '`' | '^' | '"' | '~' | '=' | '.' => {
                            i = accent(text, after, s, strings, out, &mut space);
                        }
                        '(' | ')' | '[' | ']' => i = after,
                        // `\\`, `\,`, `\;`, `\!`, `\/`, `\-`, ...: nothing.
                        _ => i = after,
                    }
                    continue;
                }
                let name = &text[i + 1..end];
                let next = skip_spaces(text, end);
                if matches!(name, "u" | "v" | "H" | "c" | "k" | "r" | "d" | "b" | "t") {
                    let a = name.chars().next().unwrap_or(' ');
                    i = accent(text, next, a, strings, out, &mut space);
                    continue;
                }
                if let Some(s) = symbol(name) {
                    flush(out, &mut space);
                    out.push_str(s);
                    // `\LaTeX{}`: the empty group is nothing.
                    i = if text[next..].starts_with("{}") { next + 2 } else { next };
                    continue;
                }
                match name {
                    "texorpdfstring" => {
                        let (_, a) = argument(text, next).unwrap_or(("", next));
                        let b = skip_spaces(text, a);
                        let (pdf, after) = argument(text, b).unwrap_or(("", b));
                        flush(out, &mut space);
                        reduce(pdf, strings, out);
                        i = after;
                    }
                    "ref" | "eqref" | "pageref" | "autoref" | "cref" | "Cref" | "nameref" => {
                        let (key, after) = argument(text, next).unwrap_or(("", next));
                        flush(out, &mut space);
                        let value = strings.refs.get(key.trim()).filter(|_| name != "pageref").cloned().unwrap_or_else(|| "??".to_string());
                        if name == "eqref" {
                            out.push('(');
                            out.push_str(&value);
                            out.push(')');
                        } else {
                            out.push_str(&value);
                        }
                        i = after;
                    }
                    "hyperref" | "linebreak" | "nolinebreak" | "pagebreak" | "nopagebreak" => {
                        // An optional argument, then (hyperref) the text.
                        i = match bracket_end(text, next) {
                            Some(close) => close + 1,
                            None => next,
                        };
                    }
                    "hspace" => {
                        // hyperref's `\hspace` in a PDF string is a space.
                        let mut k = next;
                        if text[k..].starts_with('*') {
                            k += 1;
                        }
                        let (_, after) = argument(text, k).filter(|_| text[k..].starts_with('{')).unwrap_or(("", k));
                        flush(out, &mut space);
                        out.push(' ');
                        i = after;
                    }
                    _ if drops_argument(name) => {
                        let mut k = next;
                        if let Some(close) = bracket_end(text, k) {
                            k = skip_spaces(text, close + 1);
                        }
                        let (_, after) = argument(text, k).filter(|_| text[k..].starts_with('{')).unwrap_or(("", k));
                        i = after;
                    }
                    // Formatting, declarations, math commands and anything
                    // unknown: the command goes, its argument stays text.
                    _ => i = next,
                }
            }
            _ => {
                flush(out, &mut space);
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    flush(out, &mut space);
}

/// An accent command's base at `at` (a letter, `{x}` or `\i`), composed.
fn accent(text: &str, at: usize, a: char, strings: &Strings, out: &mut String, space: &mut bool) -> usize {
    let (base, after) = argument(text, at).unwrap_or(("", at));
    let mut plain = String::new();
    reduce(base, strings, &mut plain);
    let plain = plain.trim();
    let base = match plain {
        "ı" => "i",
        "ȷ" => "j",
        other => other,
    };
    if std::mem::take(space) {
        out.push(' ');
    }
    let mut chars = base.chars();
    match (chars.next(), chars.next()) {
        (Some(b), None) => match accented(a, b) {
            Some(c) => out.push(c),
            None => out.push(b),
        },
        (None, _) => {
            // `\^{}` and `\~{}` print the accent itself.
            if matches!(a, '^' | '~' | '\'' | '`' | '"' | '=' | '.') {
                out.push(a);
            }
        }
        _ => out.push_str(base),
    }
    after
}

/// One bookmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// hyperref level (part -1, chapter 0, section 1, ...), after the depth
    /// filter and level fix.
    pub level: i32,
    pub title: String,
    /// The anchor's label key ([`key`]).
    pub anchor: String,
}

/// An anchor: its label key and destination name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub key: String,
    pub name: String,
}

/// The bookmarks of one document, collected in reading order while the
/// adapter lays out its blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct Collector {
    pub settings: Settings,
    /// `bookmarksdepth` resolved (`\c@tocdepth` by default).
    depth: i32,
    strings: Strings,
    /// `\Hy@currentbookmarklevel`.
    current: i32,
    /// hyperref checks level jumps only after the first bookmark
    /// (`\Hy@levelcheck`).
    checked: bool,
    entries: Vec<Entry>,
    anchors: Vec<Anchor>,
    names: BTreeSet<String>,
    /// `\@currentHref`: the last anchor set.
    last: Option<usize>,
    /// Anchors waiting for the next block (`hyperref`'s anchor in vertical
    /// mode lands before whatever comes next).
    pending: Vec<String>,
    /// hyperref's `Hy@linkcounter` for starred headings and
    /// `\phantomsection` (`section*.<n>`).
    link_counter: usize,
}

impl Collector {
    pub fn new(settings: Settings, tocdepth: i32, strings: Strings) -> Collector {
        Collector {
            depth: settings.depth.unwrap_or(tocdepth),
            settings,
            strings,
            current: 0,
            checked: false,
            entries: Vec::new(),
            anchors: Vec::new(),
            names: BTreeSet::new(),
            last: None,
            pending: Vec::new(),
            link_counter: 0,
        }
    }

    /// A new anchor named `name` (made unique), which becomes
    /// `\@currentHref`. Its label key is returned; the caller places it.
    pub fn anchor(&mut self, name: &str) -> String {
        self.new_anchor(name, "")
    }

    /// [`Collector::anchor`] for a chapter head: positioned at the top of
    /// the text area of the page it lands on.
    pub fn anchor_top(&mut self, name: &str) -> String {
        self.new_anchor(name, TOP_SUFFIX)
    }

    /// [`Collector::anchor`] a `\baselineskip` above its block's first
    /// baseline ([`is_raised`]).
    pub fn anchor_raised(&mut self, name: &str) -> String {
        self.new_anchor(name, RAISED_SUFFIX)
    }

    fn new_anchor(&mut self, name: &str, suffix: &str) -> String {
        // A name the PDF writer would refuse (it bounds names at 1024
        // bytes) is replaced: an outline must never fail the export.
        let fallback;
        let name = if name.is_empty() || name.len() > MAX_NAME_BYTES {
            fallback = format!("anchor.{}", self.anchors.len() + 1);
            fallback.as_str()
        } else {
            name
        };
        let mut unique = name.to_string();
        let mut n = 1;
        while self.names.contains(&unique) {
            n += 1;
            unique = format!("{name}-{n}");
        }
        self.names.insert(unique.clone());
        let mut key = key(self.anchors.len());
        key.push_str(suffix);
        self.anchors.push(Anchor { key: key.clone(), name: unique });
        self.last = Some(self.anchors.len() - 1);
        key
    }

    /// `\phantomsection` (and a starred heading's own anchor): the name
    /// `section*.<n>`.
    pub fn star_name(&mut self, counter: &str) -> String {
        self.link_counter += 1;
        format!("{counter}*.{}", self.link_counter)
    }

    /// The anchors waiting for the next block, as label keys.
    pub fn take_pending(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending)
    }

    /// The anchors still waiting when the document ends, re-keyed as
    /// [`is_after`] anchors for the caller to put at the end of the last
    /// block.
    pub fn take_pending_at_end(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        for key in std::mem::take(&mut self.pending) {
            let after = format!("{key}{AFTER_SUFFIX}");
            for a in self.anchors.iter_mut().filter(|a| a.key == key) {
                a.key = after.clone();
            }
            for e in self.entries.iter_mut().filter(|e| e.anchor == key) {
                e.anchor = after.clone();
            }
            out.push(after);
        }
        out
    }

    /// `\@currentHref`'s key, creating a pending anchor when none is set
    /// yet (hyperref's `Doc-Start`, where the body begins).
    fn current_anchor(&mut self) -> String {
        match self.last {
            Some(i) => self.anchors[i].key.clone(),
            None => {
                let key = self.anchor("Doc-Start");
                self.pending.push(key.clone());
                key
            }
        }
    }

    /// `\Hy@writebookmark{<number>}{<text>}{<anchor>}{<level>}{toc}` with
    /// `text` the source bytes of the entry (not yet `\pdfstringdef`ed).
    fn write(&mut self, number: Option<&str>, text: &str, anchor: String, level: i32) {
        if !self.settings.bookmarks || level > self.depth {
            return;
        }
        let mut level = level;
        if self.checked && level > self.current + 1 {
            level = self.current + 1;
        }
        self.checked = true;
        self.current = level;
        let mut title = self.strings.pdf_string(text);
        if let (true, Some(n)) = (self.settings.numbered, number.filter(|n| !n.is_empty())) {
            title = format!("{} {title}", self.strings.pdf_string(n));
        }
        if title.len() > MAX_TITLE_BYTES {
            let mut end = MAX_TITLE_BYTES;
            while !title.is_char_boundary(end) {
                end -= 1;
            }
            title.truncate(end);
        }
        if self.entries.len() >= MAX_ENTRIES {
            return;
        }
        self.entries.push(Entry { level, title, anchor });
    }

    /// A class's `\addcontentsline{toc}{..}` for a heading it just
    /// anchored (`anchor`).
    pub fn heading(&mut self, level: i32, number: Option<&str>, text: &str, anchor: String) {
        self.write(number, text, anchor, level);
    }

    /// `\addcontentsline{toc}{<level>}{<entry>}` in the document: the
    /// destination is the last anchor.
    pub fn contents_line(&mut self, level: i32, entry: &str) {
        let (number, title) = numberline(entry);
        let anchor = self.current_anchor();
        self.write(number, title, anchor, level);
    }

    /// `\pdfbookmark[<level>]{<text>}{<name>}` and its relatives. Returns
    /// the anchor's label key for the caller to place.
    pub fn pdf_bookmark(&mut self, kind: BookmarkKind, level: Option<i32>, text: &str, name: &str) -> String {
        let level = match kind {
            BookmarkKind::Plain => level.unwrap_or(0),
            BookmarkKind::Current => self.current,
            BookmarkKind::Sub | BookmarkKind::Below => self.current + 1,
        };
        let restore = self.current;
        let anchor = self.anchor(&format!("{name}.{level}"));
        self.write(None, text, anchor.clone(), level);
        if kind == BookmarkKind::Below {
            self.current = restore;
        }
        anchor
    }

    /// Places `key` in front of the next block.
    pub fn push_pending(&mut self, key: String) {
        self.pending.push(key);
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn anchors(&self) -> &[Anchor] {
        &self.anchors
    }
}

/// `[\protect]\numberline{<n>}<title>` split into the number and title.
fn numberline(entry: &str) -> (Option<&str>, &str) {
    let t = entry.trim_start();
    let t = t.strip_prefix("\\protect").map_or(t, str::trim_start);
    if let Some(rest) = t.strip_prefix("\\numberline") {
        let rest = rest.trim_start();
        if let Some(close) = group_end(rest, 0) {
            return (Some(&rest[1..close]), &rest[close + 1..]);
        }
    }
    (None, entry)
}

/// Which `\pdfbookmark` form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookmarkKind {
    /// `\pdfbookmark[<level>]` (default level 0).
    Plain,
    /// `\currentpdfbookmark`: the last bookmark's level.
    Current,
    /// `\subpdfbookmark`: one below it, which becomes the current level.
    Sub,
    /// `\belowpdfbookmark`: one below it, the current level unchanged.
    Below,
}

/// What the PDF writer needs: the outline entries in order and the
/// destination of every anchor they name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outline {
    pub entries: Vec<OutlineEntry>,
    /// Destination name -> where its anchor landed.
    pub destinations: BTreeMap<String, Destination>,
    pub open: bool,
    pub open_level: Option<i32>,
    /// `/PageMode` ([`Settings::page_mode`]).
    pub page_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineEntry {
    pub title: String,
    pub destination: String,
    pub level: i32,
}

impl Collector {
    /// The outline, given where each anchor landed (`positions`, by label
    /// key). An anchor that reached no page (the document ended right
    /// after it) points at the top of the last page. Empty, but still
    /// carrying the page mode, when no bookmark was written.
    pub fn outline(&self, positions: &BTreeMap<String, Destination>, last_page: Destination) -> Outline {
        let names: BTreeMap<&str, &str> = self.anchors.iter().map(|a| (a.key.as_str(), a.name.as_str())).collect();
        let mut destinations = BTreeMap::new();
        let mut entries = Vec::new();
        for e in &self.entries {
            let Some(name) = names.get(e.anchor.as_str()) else { continue };
            destinations.entry(name.to_string()).or_insert_with(|| positions.get(&e.anchor).copied().unwrap_or(last_page));
            entries.push(OutlineEntry { title: e.title.clone(), destination: name.to_string(), level: e.level });
        }
        Outline { entries, destinations, open: self.settings.open, open_level: self.settings.open_level, page_mode: self.settings.page_mode.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> String {
        Strings::default().pdf_string(text)
    }

    /// Every expectation below is what pdflatex (TeX Live 2026, hyperref
    /// 7.01p, `unicode` by default) wrote as the bookmark of
    /// `\section{<text>}`.
    #[test]
    fn pdfstringdef_matches_hyperref() {
        assert_eq!(s(r"Math $\alpha + \beta_i^2 \leq \frac{a}{b}$ end"), "Math + i2 ab end");
        assert_eq!(s(r"Font \textbf{bold} \textit{it} {\bfseries grp} \texttt{tt}"), "Font bold it grp tt");
        assert_eq!(s(r"Dashes -- and --- and - hyphen"), "Dashes – and — and - hyphen");
        assert_eq!(s(r"Spaces\ and\,thin and~tie"), "Spaces andthin and tie");
        assert_eq!(s(r"Logos \LaTeX{} and \TeX"), "Logos LaTeX and TeX");
        assert_eq!(s(r"Accents \'e \`a \^o \~n \c{c} \v{s} \o{} \aa{} \AE{}"), "Accents é à ô ñ ç š ø å Æ");
        assert_eq!(s(r"Symbols \S{} \dag{} \ldots{} \textendash{} \textemdash{} \copyright"), "Symbols § † … – — ©");
        assert_eq!(s(r"Refs \ref{x} and \cite{y}"), "Refs ?? and y");
        assert_eq!(s(r"Quote ``double'' and \textquotedblleft x\textquotedblright"), "Quote ``double'' and “x”");
        assert_eq!(s("Unicode direct: αβγ — ✓ “smart”"), "Unicode direct: αβγ — ✓ “smart”");
        assert_eq!(s(r"Braces \{x\} and \$ and \_ and \^{} and \&"), "Braces {x} and $ and _ and ^ and &");
        assert_eq!(s(r"Newline\\ break"), "Newline break");
        assert_eq!(s(r"\texorpdfstring{\textbf{A}}{B} and \texorpdfstring{C}{\textbf{D}}"), "B and D");
        assert_eq!(s(r"Second \emph{emph} \texorpdfstring{$\alpha$}{alpha}~tie"), "Second emph alpha tie");
        assert_eq!(s(r#"Stra\ss e \"Uber"#), "Straße Über");
        assert_eq!(s(r"Tilde~a\&b 50\% \#1 \textbackslash"), "Tilde a&b 50% #1 \\");
        assert_eq!(s(r"Background and $x^2$ math"), "Background and x2 math");
    }

    #[test]
    fn user_macros_expand() {
        let strings = Strings::new(&[r"\newcommand{\foo}{Foo macro}\newcommand\pair[2]{#1/#2}\def\bar{Bar}"], &BTreeMap::new());
        assert_eq!(strings.pdf_string(r"Macro \foo{} here"), "Macro Foo macro here");
        assert_eq!(strings.pdf_string(r"\pair{a}{b} \bar"), "a/b Bar");
        let recursive = Strings::new(&[r"\def\loop{\loop x}"], &BTreeMap::new());
        assert!(recursive.pdf_string(r"\loop").len() < 100);
    }

    #[test]
    fn settings_follow_options_and_hypersetup() {
        assert_eq!(Settings::read(r"\documentclass{article}\begin{document}x\end{document}"), None);
        let off = Settings::read(r"\documentclass{article}\usepackage[bookmarks=false]{hyperref}\hypersetup{bookmarks=true}\begin{document}").unwrap();
        assert!(!off.bookmarks, "`bookmarks` is a load-time option");
        assert_eq!(off.page_mode, "UseNone");
        assert_eq!(Settings::read("\\documentclass{article}\n%\\usepackage{hyperref}\n\\begin{document}"), None);
        let s = Settings::read(r"\usepackage[bookmarksnumbered,bookmarksopen]{hyperref}\hypersetup{bookmarksopenlevel=1, bookmarksdepth=subsection}\begin{document}").unwrap();
        assert_eq!(s, Settings { numbered: true, open: true, open_level: Some(1), depth: Some(2), bookmarks: true, page_mode: "UseOutlines".into() });
        let mode = Settings::read(r"\usepackage{hyperref}\hypersetup{pdfpagemode=FullScreen}\begin{document}").unwrap();
        assert_eq!(mode.page_mode, "FullScreen");
    }

    #[test]
    fn levels_follow_hyperref() {
        // `\pdfbookmark[0]`, `\section`, `\pdfbookmark[2]`,
        // `\currentpdfbookmark`, `\subpdfbookmark`, `\belowpdfbookmark`
        // (level 4 > tocdepth 3: dropped), `\section`: pdflatex's
        // `bookmarks.out`.
        let settings = Settings { numbered: false, open: false, open_level: None, depth: None, bookmarks: true, page_mode: String::new() };
        let mut c = Collector::new(settings, 3, Strings::default());
        c.pdf_bookmark(BookmarkKind::Plain, Some(0), "Top level", "top");
        let a = c.anchor("section.1");
        c.heading(1, Some("1"), "Sec", a);
        c.pdf_bookmark(BookmarkKind::Plain, Some(2), "Deeper", "deep");
        c.pdf_bookmark(BookmarkKind::Current, None, "Current", "cur");
        c.pdf_bookmark(BookmarkKind::Sub, None, "Sub", "sub");
        c.pdf_bookmark(BookmarkKind::Below, None, "Below", "below");
        let a = c.anchor("section.2");
        c.heading(1, Some("2"), "Another", a);
        let levels: Vec<(i32, &str)> = c.entries().iter().map(|e| (e.level, e.title.as_str())).collect();
        assert_eq!(levels, vec![(0, "Top level"), (1, "Sec"), (2, "Deeper"), (2, "Current"), (3, "Sub"), (1, "Another")]);
        // A jump of two levels is fixed to one.
        let mut c = Collector::new(Settings { numbered: true, open: false, open_level: None, depth: None, bookmarks: true, page_mode: String::new() }, 3, Strings::default());
        let a = c.anchor("section.1");
        c.heading(1, Some("1"), "A", a);
        c.contents_line(3, r"\numberline{1.1.1}Deep");
        let levels: Vec<(i32, &str)> = c.entries().iter().map(|e| (e.level, e.title.as_str())).collect();
        assert_eq!(levels, vec![(1, "1 A"), (2, "1.1.1 Deep")]);
    }

    #[test]
    fn heading_arguments_read_the_short_title() {
        let src = r"x \section*[Short]{Long {x}} y";
        let (starred, short, title) = heading_arguments(src, 2, "section").unwrap();
        assert!(starred);
        assert_eq!(&src[short.unwrap().0..short.unwrap().1], "Short");
        assert_eq!(&src[title.0..title.1], "Long {x}");
        assert!(heading_arguments(src, 2, "subsection").is_none());
    }
}
