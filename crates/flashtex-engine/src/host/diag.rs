//! `diag-v1` (docs/protocol/display-list-v3.md §6.7): the engine's
//! diagnostics side-channel notes ([`crate::diag::Note`]) as `DIAG`
//! messages ([`flashtex_display_list::diag::Diag`]).
//!
//! Every note becomes one `DIAG` with the place TeX was reading (file,
//! line, the column of TeX's context split, the byte range of the token
//! before it, its byte offset in the file and its display-list span), the
//! input-stack trace, the help text, a severity and a stable code. What the
//! terminal shows that no note covers (a message pdfTeX's C parts print, a
//! warning a `\message` prints) is read from the terminal as the protocol's
//! `DIAGNOSTIC` always was, and sent as a `DIAG` with `exact: false`, so a
//! `diag-v1` client never sees less than a 3.1 client.

use crate::diag::{Kind, Note, Pos, FLAG_OUTPUT_ACTIVE};
use flashtex_display_list::diag::{Diag, Frame, Loc, Severity};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Column value the display list uses for "unknown".
const NO_COLUMN: u16 = 0xFFFF;

/// What turns notes into `DIAG`s for one compile.
pub struct Builder<'a> {
    root: &'a Path,
    /// Give places display-list spans (when a display list is written).
    spans: bool,
    files: HashMap<PathBuf, Option<Vec<usize>>>,
    /// Spans the messages name, in order (to declare before them).
    pub used_spans: Vec<u32>,
}

impl<'a> Builder<'a> {
    pub fn new(root: &'a Path) -> Builder<'a> {
        Builder {
            root,
            spans: crate::displaylist::enabled(),
            files: HashMap::new(),
            used_spans: Vec::new(),
        }
    }

    fn path(&self, name: &[u8]) -> String {
        let n = String::from_utf8_lossy(name).into_owned();
        let p = Path::new(&n);
        if p.is_absolute() {
            n
        } else {
            self.root
                .join(n.trim_start_matches("./"))
                .display()
                .to_string()
        }
    }

    /// Byte offset of `line`'s start in `path` (1-based lines).
    fn line_start(&mut self, path: &str, line: i64) -> Option<usize> {
        let lines = self
            .files
            .entry(PathBuf::from(path))
            .or_insert_with(|| {
                let b = std::fs::read(path).ok()?;
                let mut v = vec![0];
                v.extend(
                    b.iter()
                        .enumerate()
                        .filter(|(_, &c)| c == b'\n')
                        .map(|(i, _)| i + 1),
                );
                Some(v)
            })
            .as_ref()?;
        lines.get((line - 1).max(0) as usize).copied()
    }

    fn span(&mut self, name: &[u8], line: i32) -> Option<i64> {
        if !self.spans || line <= 0 {
            return None;
        }
        let s = crate::displaylist::span_for(name, line as u32)?;
        self.used_spans.push(s);
        Some(s as i64)
    }

    /// A display-list location (span, column) as a `Loc`.
    fn span_loc(&mut self, (span, col): (u32, u16)) -> Option<Loc> {
        if span == 0 {
            return None;
        }
        let (file, line) = crate::displaylist::span_location(span)?;
        let path = crate::displaylist::file_path(file)?;
        self.used_spans.push(span);
        Some(Loc {
            file: Some(path),
            line: Some(line as i64),
            col: (col != NO_COLUMN).then_some(col as i64),
            span: Some(span as i64),
        })
    }

    fn place(&mut self, d: &mut Diag, p: &Pos) {
        let path = self.path(&p.file);
        d.line = Some(p.line as i64);
        if p.col >= 0 {
            d.col = Some(p.col as i64);
            if p.from >= 0 && p.from <= p.col {
                d.range = Some((p.from as i64, p.col as i64));
            }
            if let Some(s) = self.line_start(&path, p.line as i64) {
                d.offset = Some((s + p.col as usize) as i64);
            }
        }
        d.span = self.span(&p.file, p.line);
        d.file = Some(path);
    }

    fn frames(&mut self, n: &Note) -> Vec<Frame> {
        n.frames
            .iter()
            .map(|f| Frame {
                kind: f.kind.clone(),
                name: (!f.name.is_empty()).then(|| lossy(&f.name)),
                loc: f.pos.as_ref().map(|p| Loc {
                    file: Some(self.path(&p.file)),
                    line: Some(p.line as i64),
                    col: (p.col >= 0).then_some(p.col as i64),
                    span: None,
                }),
                before: lossy(&f.before),
                after: lossy(&f.after),
                def: f.def.as_ref().map(|p| Loc {
                    file: Some(self.path(&p.file)),
                    line: Some(p.line as i64),
                    col: None,
                    span: None,
                }),
            })
            .collect()
    }

    /// The `DIAG` a note stands for (`None`: a `\write` that is not a
    /// warning after all).
    pub fn note(&mut self, n: &Note) -> Option<Diag> {
        let mut d = Diag {
            exact: true,
            output: n.flags & FLAG_OUTPUT_ACTIVE != 0,
            help: n.help.iter().map(|h| lossy(h)).collect(),
            ..Diag::default()
        };
        let text = lossy(&n.text);
        match n.kind {
            Kind::Error => {
                let full = format!("{text}.");
                let (first, rest) = split_first_line(&full);
                d.severity = Some(Severity::Error);
                d.message = first.to_string();
                d.detail = rest;
                classify_error(&mut d);
            }
            Kind::Show => {
                let t = text.trim_start_matches('\n');
                let (first, rest) = split_first_line(t);
                d.severity = Some(Severity::Info);
                d.message = first.to_string();
                d.detail = rest;
                d.origin = "tex".into();
                d.code = "tex/show".into();
            }
            Kind::Write => {
                let lines: Vec<&str> = text.split('\n').collect();
                let k = lines.iter().position(|l| l.contains("Warning:"))?;
                let head = lines[k];
                let mut msg = head.trim_end().to_string();
                for l in &lines[k + 1..] {
                    if l.trim().is_empty() {
                        break;
                    }
                    msg.push(' ');
                    msg.push_str(strip_continuation(l).trim());
                }
                d.severity = Some(Severity::Warning);
                d.message = msg;
                classify_warning(&mut d);
            }
            Kind::Box => {
                let (first, rest) = split_first_line(text.trim_start_matches('\n'));
                d.message = first.to_string();
                d.detail = rest;
                let kind = first.split_whitespace().next().unwrap_or("").to_lowercase();
                let what = if first.contains("\\vbox") { "vbox" } else { "hbox" };
                d.severity = Some(match kind.as_str() {
                    "overfull" | "underfull" => Severity::Warning,
                    _ => Severity::Info,
                });
                d.origin = "tex".into();
                d.code = format!("tex/{kind}-{what}");
                let (pbl, line) = n.lines;
                if !d.output {
                    d.lines = Some(if pbl != 0 {
                        (pbl.abs() as i64, line as i64)
                    } else {
                        (line as i64, line as i64)
                    });
                }
            }
            Kind::PdfWarning => {
                let (first, rest) = split_first_line(text.trim_start_matches('\n'));
                d.severity = Some(Severity::Warning);
                d.message = first.to_string();
                d.detail = rest;
                d.origin = "pdftex".into();
                let cat = first
                    .strip_prefix("pdfTeX warning (")
                    .and_then(|r| r.split(')').next())
                    .map(slug)
                    .unwrap_or_else(|| slug(first.trim_start_matches("pdfTeX warning: ")));
                d.code = format!("pdftex/{cat}");
            }
        }
        d.trace = self.frames(n);
        // The place: a box's first character (the display list's side
        // table), else where TeX was reading.
        if n.kind == Kind::Box && n.first.0 != 0 {
            if let Some(l) = self.span_loc(n.first) {
                d.file = l.file.clone();
                d.line = l.line;
                d.col = l.col;
                d.span = l.span;
                if let (Some(f), Some(line), Some(c)) = (&l.file, l.line, l.col) {
                    if let Some(s) = self.line_start(&f.clone(), line) {
                        d.offset = Some((s + c as usize) as i64);
                    }
                }
                d.end = self.span_loc(n.last);
            }
        }
        if d.file.is_none() {
            if let Some(p) = &n.pos {
                if n.kind == Kind::Box && !d.output {
                    // No side table: TeX's lines, in the file it reads.
                    let first = d.lines.map(|l| l.0 as i32).unwrap_or(p.line);
                    let path = self.path(&p.file);
                    d.line = Some(first as i64);
                    d.span = self.span(&p.file, first);
                    d.file = Some(path);
                } else {
                    self.place(&mut d, p);
                }
            }
        }
        // A warning that names its own line ("on input line N") elsewhere
        // than where TeX read (a deferred write): the text's line wins.
        if n.kind == Kind::Write {
            if let Some(l) = input_line(&d.message) {
                if d.line != Some(l) {
                    d.line = Some(l);
                    d.col = None;
                    d.range = None;
                    d.offset = None;
                    d.span = match &n.pos {
                        Some(p) => self.span(&p.file, l as i32),
                        None => None,
                    };
                }
            }
        }
        Some(d)
    }
}

/// Every `DIAG` of a compile: the notes, and what the terminal shows that
/// no note covers (`exact: false`), in the order the terminal has them;
/// `id` and `seq` set.
pub fn build(root: &Path, id: i64, notes: &[Note], terminal: &[u8]) -> (Vec<Diag>, Vec<u32>) {
    let mut b = Builder::new(root);
    let mut out: Vec<(usize, Diag)> = Vec::new();
    for n in notes {
        if let Some(d) = b.note(n) {
            out.push((n.at, d));
        }
    }
    let covered = |off: usize| notes.iter().any(|n| off >= n.at && off <= n.end + 1);
    for (off, d) in scan_terminal(terminal, root) {
        if !covered(off) {
            out.push((off, d));
        }
    }
    out.sort_by_key(|x| x.0);
    let diags = out
        .into_iter()
        .enumerate()
        .map(|(i, (_, mut d))| {
            d.id = id;
            d.seq = i as i64;
            d
        })
        .collect();
    (diags, b.used_spans)
}

/// What the terminal shows, read as the 3.1 `DIAGNOSTIC`s were read
/// (`super::server::diagnostics`), with each report's terminal offset:
/// errors (`file:line: message`, `! message`, `!pdfTeX error`), LaTeX,
/// package and class warnings, pdfTeX warnings.
pub fn scan_terminal(term: &[u8], root: &Path) -> Vec<(usize, Diag)> {
    const MAX_PRINT_LINE: usize = 79;
    let mut out = Vec::new();
    let mut lines: Vec<(usize, String)> = Vec::new();
    let mut off = 0;
    for l in term.split(|&c| c == b'\n') {
        lines.push((off, String::from_utf8_lossy(l).into_owned()));
        off += l.len() + 1;
    }
    let abs = |f: &str| {
        let p = Path::new(f);
        if p.is_absolute() {
            f.to_string()
        } else {
            root.join(f.trim_start_matches("./")).display().to_string()
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let (off, l) = (&lines[i].0, &lines[i].1);
        let mut d = Diag::default();
        if let Some((file, line, msg)) = file_line_error(l) {
            d.severity = Some(Severity::Error);
            d.file = Some(abs(&file));
            d.line = Some(line);
            d.message = msg;
            classify_error(&mut d);
        } else if let Some(msg) = l.strip_prefix("! ") {
            d.severity = Some(Severity::Error);
            d.message = msg.to_string();
            classify_error(&mut d);
        } else if l.starts_with("!pdfTeX error") {
            d.severity = Some(Severity::Error);
            d.message = l.trim_start_matches('!').to_string();
            d.origin = "pdftex".into();
            d.code = "pdftex/error".into();
            d.fatal = true;
        } else if l.starts_with("pdfTeX warning") {
            d.severity = Some(Severity::Warning);
            d.message = l.clone();
            d.origin = "pdftex".into();
            d.code = "pdftex/warning".into();
        } else if ["LaTeX Warning: ", "Package ", "Class ", "LaTeX Font Warning: "]
            .iter()
            .any(|p| l.starts_with(p))
            && l.contains("Warning:")
        {
            let mut msg = l.clone();
            let mut prev = l.len();
            let mut j = i + 1;
            if !msg.ends_with('.') {
                while j < lines.len() {
                    let more = &lines[j].1;
                    if more.trim().is_empty() {
                        break;
                    }
                    if prev == MAX_PRINT_LINE {
                        msg.push_str(more);
                    } else {
                        msg.push(' ');
                        msg.push_str(strip_continuation(more).trim());
                    }
                    prev = more.len();
                    j += 1;
                    if more.ends_with('.') {
                        break;
                    }
                }
            }
            d.severity = Some(Severity::Warning);
            d.line = input_line(&msg);
            d.message = msg;
            classify_warning(&mut d);
            out.push((*off, d));
            i = j.max(i + 1);
            continue;
        } else {
            i += 1;
            continue;
        }
        out.push((*off, d));
        i += 1;
    }
    out
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn split_first_line(t: &str) -> (&str, Option<String>) {
    match t.split_once('\n') {
        Some((a, b)) => {
            let b = b.trim();
            (a, (!b.is_empty()).then(|| b.to_string()))
        }
        None => (t, None),
    }
}

/// A warning's continuation line without its `(name)   ` prefix.
fn strip_continuation(l: &str) -> &str {
    let t = l.trim_start();
    if t.starts_with('(') {
        if let Some(e) = t.find(')') {
            return &t[e + 1..];
        }
    }
    l
}

/// `./main.tex:12: Undefined control sequence.` -> (file, 12, message).
fn file_line_error(l: &str) -> Option<(String, i64, String)> {
    let mut parts = l.splitn(3, ':');
    let file = parts.next()?;
    let line = parts.next()?;
    let msg = parts.next()?;
    if file.is_empty() || file.contains(' ') || !file.contains('.') {
        return None;
    }
    let n: i64 = line.parse().ok()?;
    Some((file.to_string(), n, msg.trim_start().to_string()))
}

/// "... on input line 12." -> 12.
fn input_line(msg: &str) -> Option<i64> {
    let p = msg.rfind("input line ")?;
    msg[p + 11..]
        .trim_end_matches('.')
        .trim()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// A stable slug of a message: quoted names (`x'), control sequence
/// names, numbers and "on input line N" left out; lower case; words
/// joined by `-`.
pub fn slug(msg: &str) -> String {
    let mut t = String::new();
    let mut chars = msg.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => {
                // a quoted name, up to the closing '
                for d in chars.by_ref() {
                    if d == '\'' {
                        break;
                    }
                }
                t.push(' ');
            }
            '\\' => {
                // a control sequence name
                let mut any = false;
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_alphabetic() || d == '@' {
                        chars.next();
                        any = true;
                    } else {
                        break;
                    }
                }
                if !any {
                    chars.next();
                }
                t.push(' ');
            }
            '{' => {
                // an argument (`\begin{x}`, `name{y}`)
                let mut depth = 1;
                for d in chars.by_ref() {
                    match d {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                t.push(' ');
            }
            c if c.is_ascii_alphabetic() => t.push(c.to_ascii_lowercase()),
            _ => t.push(' '),
        }
    }
    let t = t.replace(" on input line", " ");
    let words: Vec<&str> = t
        .split_whitespace()
        .filter(|w| !matches!(*w, "s" | "t"))
        .collect();
    let mut s = words.join("-");
    if s.len() > 60 {
        s.truncate(60);
        while s.ends_with('-') {
            s.pop();
        }
    }
    if s.is_empty() {
        s.push_str("unknown");
    }
    s
}

/// Codes of TeX's and LaTeX's common errors whose slug would carry a name
/// or read badly; the rest are slugs.
fn curated(origin: &str, msg: &str) -> Option<&'static str> {
    let m = msg.trim_end_matches('.');
    let table: &[(&str, &str, &str)] = &[
        ("tex", "Undefined control sequence", "undefined-control-sequence"),
        ("tex", "Missing $ inserted", "missing-dollar"),
        ("tex", "Missing { inserted", "missing-left-brace"),
        ("tex", "Missing } inserted", "missing-right-brace"),
        ("tex", "Extra }, or forgotten $", "extra-right-brace-or-forgotten-dollar"),
        ("tex", "Extra }, or forgotten \\endgroup", "extra-right-brace-or-forgotten-endgroup"),
        ("tex", "Too many }'s", "too-many-right-braces"),
        ("tex", "Display math should end with $$", "display-math-should-end-with-dollars"),
        ("tex", "Missing number, treated as zero", "missing-number"),
        ("tex", "Illegal unit of measure", "illegal-unit"),
        ("tex", "Paragraph ended before", "paragraph-ended-before-argument-complete"),
        ("tex", "File ended while scanning", "file-ended-while-scanning"),
        ("tex", "Emergency stop", "emergency-stop"),
        ("tex", "TeX capacity exceeded", "capacity-exceeded"),
        ("tex", "I can't find file", "file-not-found"),
        ("tex", "You can't use", "cannot-use-in-this-mode"),
        ("tex", "Misplaced alignment tab character &", "misplaced-alignment-tab"),
        ("tex", "Extra alignment tab has been changed to", "extra-alignment-tab"),
        ("tex", "Double superscript", "double-superscript"),
        ("tex", "Double subscript", "double-subscript"),
        ("latex", "LaTeX Error: File `", "file-not-found"),
        ("latex", "LaTeX Error: Environment ", "environment-undefined"),
        ("latex", "LaTeX Error: \\begin{", "environment-mismatch"),
        ("latex", "LaTeX Error: Missing \\begin{document}", "missing-begin-document"),
        ("latex", "LaTeX Error: Something's wrong--perhaps a missing \\item", "missing-item"),
        ("latex", "LaTeX Error: Lonely \\item--perhaps a missing list environment", "lonely-item"),
        ("latex", "LaTeX Error: \\verb ended by end of line", "verb-ended-by-end-of-line"),
        ("latex", "LaTeX Error: Option clash for package", "option-clash"),
        ("latex", "LaTeX Error: Unknown option", "unknown-option"),
        ("latex", "LaTeX Error: Command ", "command-already-defined"),
    ];
    table
        .iter()
        .find(|(o, p, _)| *o == origin && m.starts_with(p) && (!p.ends_with("Command ") || m.ends_with("already defined")))
        .map(|x| x.2)
}

/// Origin, package and code of an error message.
fn classify_error(d: &mut Diag) {
    let m = d.message.clone();
    let (origin, package, rest) = if let Some(r) = m.strip_prefix("LaTeX Error: ") {
        ("latex", None, r.to_string())
    } else if let Some(r) = m.strip_prefix("LaTeX3 Error: ") {
        ("latex3", None, r.to_string())
    } else if let Some(r) = m.strip_prefix("Package ") {
        match r.split_once(" Error: ") {
            Some((p, rest)) => ("package", Some(p.to_string()), rest.to_string()),
            None => ("tex", None, m.clone()),
        }
    } else if let Some(r) = m.strip_prefix("Class ") {
        match r.split_once(" Error: ") {
            Some((p, rest)) => ("class", Some(p.to_string()), rest.to_string()),
            None => ("tex", None, m.clone()),
        }
    } else if let Some(r) = m.strip_prefix("pdfTeX error") {
        ("pdftex", None, r.to_string())
    } else {
        ("tex", None, m.clone())
    };
    d.fatal = [
        "Emergency stop",
        "TeX capacity exceeded",
        "This can't happen",
        "I can't go on meeting you like this",
        "Interwoven alignment preambles are not allowed",
        "pdfTeX error",
    ]
    .iter()
    .any(|p| m.starts_with(p));
    let code = curated(origin, &m)
        .map(str::to_string)
        .unwrap_or_else(|| slug(&rest));
    d.code = match &package {
        Some(p) => format!("{origin}/{p}/{code}"),
        None => format!("{origin}/{code}"),
    };
    d.origin = origin.into();
    d.package = package;
}

/// Origin, package and code of a warning (`… Warning: …`).
fn classify_warning(d: &mut Diag) {
    let m = d.message.clone();
    let Some((head, rest)) = m.split_once(" Warning: ") else {
        d.origin = "latex".into();
        d.code = format!("latex/{}", slug(&m));
        return;
    };
    let mut words = head.split_whitespace();
    let (origin, package) = match words.next() {
        Some("Package") => ("package", words.next().map(str::to_string)),
        Some("Class") => ("class", words.next().map(str::to_string)),
        Some("pdfTeX") => ("pdftex", None),
        Some("LaTeX") => match words.next() {
            Some(w) => ("latex", Some(w.to_lowercase())),
            None => ("latex", None),
        },
        _ => ("latex", None),
    };
    let r = rest.trim_end_matches('.');
    let code = if r.starts_with("Reference `") && r.contains("undefined") {
        "undefined-reference".to_string()
    } else if r.starts_with("Citation `") && r.contains("undefined") {
        "undefined-citation".to_string()
    } else if r.starts_with("Label `") && r.contains("multiply defined") {
        "multiply-defined-label".to_string()
    } else if r.starts_with("Label(s) may have changed") {
        "rerun".to_string()
    } else if r.starts_with("Font shape `") {
        "font-shape-undefined".to_string()
    } else {
        slug(r)
    };
    d.code = match (origin, &package) {
        ("latex", Some(p)) => format!("latex-{p}/{code}"),
        (o, Some(p)) => format!("{o}/{p}/{code}"),
        (o, None) => format!("{o}/{code}"),
    };
    d.origin = origin.into();
    d.package = if origin == "latex" { None } else { package };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(slug("File `foo.sty' not found."), "file-not-found");
        assert_eq!(slug("Argument of \\@caption has an extra }."), "argument-of-has-an-extra");
        assert_eq!(slug("Undefined color `nocolor'."), "undefined-color");
        let mut d = Diag {
            message: "LaTeX Error: Environment foo undefined.".into(),
            ..Diag::default()
        };
        classify_error(&mut d);
        assert_eq!(d.code, "latex/environment-undefined");
        let mut d = Diag {
            message: "Package xcolor Error: Undefined color `nocolor'.".into(),
            ..Diag::default()
        };
        classify_error(&mut d);
        assert_eq!(d.code, "package/xcolor/undefined-color");
        let mut d = Diag {
            message: "LaTeX Warning: Reference `a' on page 1 undefined on input line 5.".into(),
            ..Diag::default()
        };
        classify_warning(&mut d);
        assert_eq!(d.code, "latex/undefined-reference");
        let mut d = Diag {
            message: "LaTeX Font Warning: Font shape `OT1/cmr/zz/n' undefined".into(),
            ..Diag::default()
        };
        classify_warning(&mut d);
        assert_eq!(d.code, "latex-font/font-shape-undefined");
    }
}
