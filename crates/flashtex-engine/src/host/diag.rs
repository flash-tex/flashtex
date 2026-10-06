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
            Kind::Write if text.split('\n').any(|l| l.starts_with("! ")) => {
                // An error the macros print themselves (LaTeX's missing
                // file, before it asks the terminal for another name).
                let lines: Vec<&str> = text.split('\n').collect();
                let k = lines.iter().position(|l| l.starts_with("! "))?;
                d.severity = Some(Severity::Error);
                d.message = lines[k][2..].trim_end().to_string();
                let rest: Vec<&str> = lines[k + 1..]
                    .iter()
                    .copied()
                    .skip_while(|l| l.trim().is_empty())
                    .collect();
                let rest = rest.join("\n");
                let rest = rest.trim();
                d.detail = (!rest.is_empty()).then(|| rest.to_string());
                classify_error(&mut d);
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
                let what = if first.contains("\\vbox") {
                    "vbox"
                } else {
                    "hbox"
                };
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
        if n.kind == Kind::Write && d.severity == Some(Severity::Warning) {
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
pub fn build(
    root: &Path,
    id: i64,
    notes: &[std::sync::Arc<Note>],
    terminal: &[u8],
) -> (Vec<Diag>, Vec<u32>) {
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
    locate_runaways(&mut out, terminal, root, &mut b);
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

/// "File ended while scanning use of \\X" (TeX reads an unclosed argument
/// to the end of the file) names no place: the file level has ended when
/// TeX reports it. TeX shows what it had read, though: the line after
/// "Runaway argument?" (§306, the start of the argument, cut at
/// `error_line`). That text is looked for in the project's files TeX
/// opened, and the report placed on the argument's opening brace, which
/// is never closed; so are the stop reports right after it that name no
/// place either ("Emergency stop", "Fatal error occurred"). Nothing is
/// placed when the text is not found. Read only from what TeX printed and
/// the files: the engine's output is not touched.
fn locate_runaways(out: &mut [(usize, Diag)], terminal: &[u8], root: &Path, b: &mut Builder) {
    let mut i = 0;
    while i < out.len() {
        let (at, d) = &out[i];
        if d.file.is_some() || !d.message.starts_with("File ended while scanning") {
            i += 1;
            continue;
        }
        let Some(text) = runaway_text(&terminal[..(*at).min(terminal.len())]) else {
            i += 1;
            continue;
        };
        // only a brace still open at its file's end (newest file first): a
        // closed copy of the same text elsewhere is not the runaway
        let Some(((path, name), (line, col, offset))) = opened_files(terminal, root)
            .into_iter()
            .rev()
            .find_map(|f| find_open_brace(&f.0, &text).map(|at| (f, at)))
        else {
            i += 1;
            continue;
        };
        let span = b.span(name.as_bytes(), line as i32);
        let mut j = i;
        while j < out.len() && (j == i || (out[j].1.file.is_none() && out[j].1.fatal)) {
            let d = &mut out[j].1;
            d.file = Some(path.display().to_string());
            d.line = Some(line as i64);
            d.col = Some(col as i64);
            d.range = Some((col as i64, col as i64 + 1));
            d.offset = Some(offset as i64);
            d.span = span;
            d.exact = false;
            j += 1;
        }
        i = j;
    }
}

/// The text TeX showed after the last "Runaway ...?" line of `term`
/// (before its "\\ETC." cut).
fn runaway_text(term: &[u8]) -> Option<String> {
    let t = String::from_utf8_lossy(term);
    let mut lines = t.lines().rev();
    let mut prev = None;
    for l in lines.by_ref().take(6) {
        if l.starts_with("Runaway ") && l.ends_with('?') {
            let text = prev?;
            let text: &str = text;
            let text = text.strip_suffix("\\ETC.").unwrap_or(text);
            return (!text.trim().is_empty()).then(|| text.to_string());
        }
        prev = Some(l);
    }
    None
}

/// The project's files TeX opened, in order, with the name TeX printed:
/// `(./main.tex` and the like on the terminal, inside `root` once both are
/// canonical (`(../x.tex` is not the project's), named under `root` as
/// given (the client strips that prefix).
fn opened_files(term: &[u8], root: &Path) -> Vec<(PathBuf, String)> {
    let t = String::from_utf8_lossy(term);
    let mut v: Vec<(PathBuf, String)> = Vec::new();
    let Ok(canon_root) = std::fs::canonicalize(root) else {
        return v;
    };
    for (i, _) in t.match_indices('(') {
        let name: String = t[i + 1..]
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ')' && *c != '(')
            .collect();
        if name.is_empty() {
            continue;
        }
        let p = Path::new(&name);
        let full = if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(name.trim_start_matches("./"))
        };
        let Ok(canon) = std::fs::canonicalize(&full) else {
            continue;
        };
        let Ok(rel) = canon.strip_prefix(&canon_root) else {
            continue;
        };
        let full = root.join(rel);
        if canon.is_file() && !v.iter().any(|(f, _)| *f == full) {
            v.push((full, name));
        }
    }
    v
}

/// Where in `path` an argument starting with `text` (as TeX shows tokens:
/// spaces and line ends are not compared, nor comments) opens and is never
/// closed: the line (1-based), byte column and byte offset of its `{`; of
/// several, the last. A copy whose brace closes is not a runaway: none.
fn find_open_brace(path: &Path, text: &str) -> Option<(usize, usize, usize)> {
    let src = std::fs::read(path).ok()?;
    // the source without blanks and comments, and each byte's offset
    let mut flat: Vec<u8> = Vec::with_capacity(src.len());
    let mut at: Vec<usize> = Vec::with_capacity(src.len());
    let mut k = 0;
    while k < src.len() {
        let c = src[k];
        if c == b'\\' && k + 1 < src.len() {
            flat.extend([c, src[k + 1]]);
            at.extend([k, k + 1]);
            k += 2;
            continue;
        }
        if c == b'%' {
            while k < src.len() && src[k] != b'\n' {
                k += 1;
            }
            continue;
        }
        if c.is_ascii_whitespace() {
            // a blank line is TeX's `\par` (§347), as the runaway text shows it
            let run = src[k..]
                .iter()
                .take_while(|c| c.is_ascii_whitespace())
                .count();
            if src[k..k + run].iter().filter(|&&c| c == b'\n').count() >= 2 {
                flat.extend(b"\\par");
                at.extend([k; 4]);
            }
            k += run;
            continue;
        }
        flat.push(c);
        at.push(k);
        k += 1;
    }
    // TeX shows `#` doubled, and a control word with a space after it
    let want: Vec<u8> = text
        .replace("##", "#")
        .bytes()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    if want.len() < 2 || want[0] != b'{' {
        return None;
    }
    // braces still open at the end: their positions in `flat`
    let mut open: Vec<usize> = Vec::new();
    let mut k = 0;
    while k < flat.len() {
        match flat[k] {
            b'\\' => k += 1,
            b'{' => open.push(k),
            b'}' => {
                open.pop();
            }
            _ => {}
        }
        k += 1;
    }
    let hits: Vec<usize> = flat
        .windows(want.len())
        .enumerate()
        .filter(|(i, w)| *w == want.as_slice() && (*i == 0 || flat[i - 1] != b'\\'))
        .map(|(i, _)| i)
        .collect();
    let hit = hits.iter().rev().find(|h| open.contains(h)).copied()?;
    let off = at[hit];
    let line_start = src[..off]
        .iter()
        .rposition(|&c| c == b'\n')
        .map_or(0, |p| p + 1);
    let line = src[..off].iter().filter(|&&c| c == b'\n').count() + 1;
    Some((line, off - line_start, off))
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
        // TeX broke an error's line after max_print_line characters.
        if (lines[i].1.starts_with("! ") || file_line_error(&lines[i].1).is_some())
            && lines[i].1.len() == MAX_PRINT_LINE
            && i + 1 < lines.len()
        {
            let more = lines.remove(i + 1).1;
            lines[i].1.push_str(&more);
            continue;
        }
        let (off, l) = (&lines[i].0, &lines[i].1);
        let mut d = Diag::default();
        if let Some((file, line, msg)) = file_line_error(l) {
            d.severity = Some(Severity::Error);
            d.file = Some(abs(&file));
            d.line = Some(line);
            d.message = msg.trim().to_string();
            classify_error(&mut d);
        } else if let Some(msg) = l.strip_prefix("! ") {
            d.severity = Some(Severity::Error);
            d.message = msg.trim().to_string();
            classify_error(&mut d);
        } else if l.starts_with("!pdfTeX error") {
            d.severity = Some(Severity::Error);
            d.message = l.trim_start_matches('!').to_string();
            d.origin = "pdftex".into();
            d.code = "pdftex/error".into();
            d.fatal = true;
        } else if let Some(name) = no_file(l) {
            // `\include` / `\InputIfFileExists` of a file that is not there:
            // LaTeX only `\typeout`s it (no note, no place; the app finds the call).
            d.severity = Some(Severity::Warning);
            d.message = format!("No file {name}.");
            d.origin = "latex".into();
            d.code = "latex/no-file".into();
        } else if l.starts_with("Missing character: There is no ") {
            // `\tracinglostchars` > 1 shows it on the terminal (no place).
            d.severity = Some(Severity::Warning);
            d.message = l.trim_end().to_string();
            d.origin = "tex".into();
            d.code = "tex/missing-character".into();
        } else if l.starts_with("pdfTeX warning") {
            d.severity = Some(Severity::Warning);
            d.message = l.clone();
            d.origin = "pdftex".into();
            d.code = "pdftex/warning".into();
        } else if [
            "LaTeX Warning: ",
            "Package ",
            "Class ",
            "LaTeX Font Warning: ",
        ]
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

/// "No file chap9.tex." (LaTeX's `\typeout` for a missing `\include` or
/// `\InputIfFileExists`) -> "chap9.tex". Only a `.tex` file: "No file
/// main.aux." (and `.toc`, `.bbl`, ...) on a first run is not a problem.
fn no_file(text: &str) -> Option<String> {
    text.split('\n').find_map(|l| {
        let name = l.trim().strip_prefix("No file ")?.strip_suffix('.')?;
        (name.ends_with(".tex") && !name.contains(' ')).then(|| name.to_string())
    })
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
        (
            "tex",
            "Undefined control sequence",
            "undefined-control-sequence",
        ),
        ("tex", "Missing $ inserted", "missing-dollar"),
        ("tex", "Missing { inserted", "missing-left-brace"),
        ("tex", "Missing } inserted", "missing-right-brace"),
        (
            "tex",
            "Missing \\right. inserted",
            "missing-right-delimiter",
        ),
        ("tex", "Extra \\right", "extra-right-delimiter"),
        (
            "tex",
            "Extra }, or forgotten $",
            "extra-right-brace-or-forgotten-dollar",
        ),
        (
            "tex",
            "Extra }, or forgotten \\endgroup",
            "extra-right-brace-or-forgotten-endgroup",
        ),
        ("tex", "Too many }'s", "too-many-right-braces"),
        (
            "tex",
            "Display math should end with $$",
            "display-math-should-end-with-dollars",
        ),
        ("tex", "Missing number, treated as zero", "missing-number"),
        ("tex", "Illegal unit of measure", "illegal-unit"),
        (
            "tex",
            "Paragraph ended before",
            "paragraph-ended-before-argument-complete",
        ),
        (
            "tex",
            "File ended while scanning",
            "file-ended-while-scanning",
        ),
        ("tex", "Emergency stop", "emergency-stop"),
        ("tex", "==> Fatal error occurred", "fatal-error-no-output"),
        ("tex", "TeX capacity exceeded", "capacity-exceeded"),
        ("tex", "I can't find file", "file-not-found"),
        ("tex", "You can't use", "cannot-use-in-this-mode"),
        (
            "tex",
            "Misplaced alignment tab character &",
            "misplaced-alignment-tab",
        ),
        (
            "tex",
            "Extra alignment tab has been changed to",
            "extra-alignment-tab",
        ),
        ("tex", "Double superscript", "double-superscript"),
        ("tex", "Double subscript", "double-subscript"),
        ("latex", "LaTeX Error: File `", "file-not-found"),
        (
            "latex",
            "LaTeX Error: Environment ",
            "environment-undefined",
        ),
        ("latex", "LaTeX Error: \\begin{", "environment-mismatch"),
        (
            "latex",
            "LaTeX Error: Missing \\begin{document}",
            "missing-begin-document",
        ),
        (
            "latex",
            "LaTeX Error: Something's wrong--perhaps a missing \\item",
            "missing-item",
        ),
        (
            "latex",
            "LaTeX Error: Lonely \\item--perhaps a missing list environment",
            "lonely-item",
        ),
        (
            "latex",
            "LaTeX Error: \\verb ended by end of line",
            "verb-ended-by-end-of-line",
        ),
        (
            "latex",
            "LaTeX Error: Option clash for package",
            "option-clash",
        ),
        ("latex", "LaTeX Error: Unknown option", "unknown-option"),
        (
            "latex",
            "LaTeX Error: Unicode character",
            "unicode-not-set-up",
        ),
        (
            "latex",
            "LaTeX Error: \\caption outside float",
            "caption-outside-float",
        ),
        ("latex", "LaTeX Error: Command ", "command-already-defined"),
    ];
    table
        .iter()
        .find(|(o, p, _)| {
            *o == origin
                && m.starts_with(p)
                && (!p.ends_with("Command ") || m.ends_with("already defined"))
        })
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
        "==> Fatal error occurred",
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
    } else if r.starts_with("Float too large for page") {
        "float-too-large".to_string()
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

    /// Lane ERROR-RECOVERY gap 4: "File ended while scanning use of
    /// \\textbf" is placed on the argument's unclosed `{`, found from the
    /// runaway text TeX showed; the stop reports after it go there too.
    #[test]
    fn a_runaway_argument_is_placed_on_its_open_brace() {
        let root = std::env::temp_dir().join(format!("flashtex-runaway-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let doc = "\\documentclass{article}\n\\begin{document}\n% a {comment\nSome {\\bfseries ok} and \\textbf{bold}.\nMore \\textbf{bold text\nAmet  amet enim.\n\n\\end{document}\n";
        std::fs::write(root.join("main.tex"), doc).unwrap();
        let term = b"(./main.tex\nRunaway argument?\n{bold text Amet amet enim. \\par \\end {document} \\ETC.\n! File ended while scanning use of \\textbf .\n<inserted text> \n                \\par \n<*> main.tex\n\n! Emergency stop.\n";
        let err = |m: &str, fatal: bool| Diag {
            message: m.into(),
            fatal,
            ..Diag::default()
        };
        let s = String::from_utf8_lossy(term);
        let mut out = vec![
            (
                s.find("! File ended").unwrap(),
                err("File ended while scanning use of \\textbf .", false),
            ),
            (s.find("! Emergency").unwrap(), err("Emergency stop.", true)),
        ];
        locate_runaways(&mut out, term, &root, &mut Builder::new(&root));
        for (_, d) in &out {
            assert_eq!(
                d.file.as_deref(),
                Some(root.join("main.tex").display().to_string().as_str())
            );
            assert_eq!(d.line, Some(5), "{}", d.message);
            assert_eq!(d.range, Some((12, 13)), "the `{{` after \\textbf on line 5");
        }
        // Text found nowhere: no place, as before.
        let mut none = vec![(
            s.find("! File ended").unwrap(),
            err("File ended while scanning use of \\textbf .", false),
        )];
        locate_runaways(
            &mut none,
            &term[..]
                .iter()
                .map(|&c| if c == b'A' { b'Z' } else { c })
                .collect::<Vec<u8>>(),
            &root,
            &mut Builder::new(&root),
        );
        assert_eq!(none[0].1.file, None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Review of #1566: the same text closed in an earlier `\input` file
    /// (opened after main.tex, so newer) is not the runaway; the unclosed
    /// brace in main.tex is. And a file outside the project (`(../x.tex`)
    /// is never read.
    #[test]
    fn a_closed_copy_in_another_file_does_not_take_the_runaway() {
        let base = std::env::temp_dir().join(format!("flashtex-runaway2-{}", std::process::id()));
        let root = base.join("proj");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("intro.tex"),
            "% the same text, closed\n\\textbf{bold text\nend. \\end{document}} intro.\n",
        )
        .unwrap();
        std::fs::write(
            root.join("main.tex"),
            "\\documentclass{article}\n\\begin{document}\n\\input{intro}\nMore \\textbf{bold text\nend.\n\\end{document}\n",
        )
        .unwrap();
        // outside the project: an unclosed copy that must not be read
        std::fs::write(base.join("x.tex"), "\\textbf{bold text\n").unwrap();
        let term = b"(./main.tex (./intro.tex) (../x.tex)\nRunaway argument?\n{bold text end. \\end {document} \\ETC.\n! File ended while scanning use of \\textbf .\n";
        let s = String::from_utf8_lossy(term);
        let mut out = vec![(
            s.find("! File ended").unwrap(),
            Diag {
                message: "File ended while scanning use of \\textbf .".into(),
                ..Diag::default()
            },
        )];
        locate_runaways(&mut out, term, &root, &mut Builder::new(&root));
        let d = &out[0].1;
        assert_eq!(
            d.file.as_deref(),
            Some(root.join("main.tex").display().to_string().as_str()),
            "the unclosed brace in main.tex, not intro.tex's closed copy nor ../x.tex"
        );
        assert_eq!((d.line, d.col), (Some(4), Some(12)));
        let main = std::fs::read(root.join("main.tex")).unwrap();
        assert_eq!(main[d.offset.unwrap() as usize], b'{');
        assert_eq!(
            opened_files(term, &root)
                .iter()
                .map(|(_, n)| n.as_str())
                .collect::<Vec<_>>(),
            ["./main.tex", "./intro.tex"]
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn codes_are_stable() {
        assert_eq!(slug("File `foo.sty' not found."), "file-not-found");
        assert_eq!(
            slug("Argument of \\@caption has an extra }."),
            "argument-of-has-an-extra"
        );
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
        for (msg, code) in [
            ("Missing \\right. inserted.", "tex/missing-right-delimiter"),
            ("Extra \\right.", "tex/extra-right-delimiter"),
            (
                "LaTeX Error: Unicode character \u{2713} (U+2713)",
                "latex/unicode-not-set-up",
            ),
            (
                "LaTeX Error: \\caption outside float.",
                "latex/caption-outside-float",
            ),
        ] {
            let mut d = Diag {
                message: msg.into(),
                ..Diag::default()
            };
            classify_error(&mut d);
            assert_eq!(d.code, code, "{msg}");
        }
        let mut d = Diag {
            message: "LaTeX Warning: Float too large for page by 327.5pt on input line 5.".into(),
            ..Diag::default()
        };
        classify_warning(&mut d);
        assert_eq!(d.code, "latex/float-too-large");
    }

    #[test]
    fn a_missing_include_is_a_warning_and_aux_files_are_not() {
        assert_eq!(
            no_file("\nNo file chap9.tex.\n").as_deref(),
            Some("chap9.tex")
        );
        assert_eq!(no_file("No file main.aux."), None);
        assert_eq!(no_file("No file main.toc."), None);
        let d = scan_terminal(
            b"(./main.aux)\nNo file main.aux.\nNo file chap9.tex.\n",
            Path::new("/tmp"),
        );
        assert_eq!(d.len(), 1);
        assert_eq!(
            (d[0].1.code.as_str(), d[0].1.message.as_str()),
            ("latex/no-file", "No file chap9.tex.")
        );
        let term = b"Missing character: There is no \xc8 in font cmr10!\n";
        let d = scan_terminal(term, Path::new("/tmp"));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].1.code, "tex/missing-character");
        assert_eq!(d[0].1.severity, Some(Severity::Warning));
    }
}
