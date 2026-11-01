//! `diag-v1` for Unicode mode (docs/protocol/display-list-v3.md §6.7):
//! structured diagnostics read from the run's log.
//!
//! The pdfTeX engine's host builds its `DIAG`s from the engine's side
//! channel, with the log's text for what no note covers
//! (`flashtex_engine::host::diag`). The XeTeX engine runs in a child
//! process and has no side channel yet, so everything comes from the log,
//! through the same reader and the same classification
//! (`flashtex_engine::host::diag::scan_terminal`: severity, code, origin,
//! package, file, line, `fatal`), and then, for an error, what TeX's error
//! display says beyond its first line ([`complete`]):
//!
//! * the place: TeX's context ends with the file level `l.<line> <text
//!   read>` over `<text still to read>`; the split is the column (UTF-8
//!   bytes of the line, found in the file when TeX shortened the text
//!   with `...`), and the token before it the range;
//! * the trace: every context level TeX printed (LaTeX prints the top and
//!   the file level), with a macro's name and its kind (`<argument>`,
//!   `<recently read>`, ...);
//! * the help text that follows (TeX writes it to the log only).
//!
//! Such a `DIAG` says `exact: false`: it is what TeX printed, not the
//! engine's own record.

use flashtex_display_list::diag::{Diag, Frame, Loc};
use std::collections::HashMap;
use std::path::Path;

/// The `DIAG`s of a compile from its log (`id`: the compile's).
pub fn from_log(log: &str, root: &Path, id: i64) -> Vec<Diag> {
    let lines: Vec<&str> = log.split('\n').collect();
    // byte offset of each line's start
    let mut starts = Vec::with_capacity(lines.len());
    let mut off = 0;
    for l in &lines {
        starts.push(off);
        off += l.len() + 1;
    }
    let mut files = Files::default();
    let mut out = Vec::new();
    for (k, (at, mut d)) in flashtex_engine::host::diag::scan_terminal(log.as_bytes(), root)
        .into_iter()
        .enumerate()
    {
        let i = starts.partition_point(|&s| s <= at).saturating_sub(1);
        if d.severity == Some(flashtex_display_list::diag::Severity::Error) {
            complete(&mut d, &lines, i, &mut files);
        }
        d.id = id;
        d.seq = k as i64;
        out.push(d);
    }
    out
}

/// The source files read for the columns, by path.
#[derive(Default)]
struct Files(HashMap<String, Option<String>>);

impl Files {
    /// Line `n` (1-based) of `path`, and the byte offset of its start.
    fn line(&mut self, path: &str, n: i64) -> Option<(usize, &str)> {
        let text = self
            .0
            .entry(path.to_string())
            .or_insert_with(|| std::fs::read_to_string(path).ok())
            .as_deref()?;
        let mut start = 0;
        for (k, l) in text.split('\n').enumerate() {
            if k as i64 + 1 == n {
                return Some((start, l.trim_end_matches('\r')));
            }
            start += l.len() + 1;
        }
        None
    }
}

/// What TeX printed after error line `i`: the context levels, the file
/// level's split, the help text.
fn complete(d: &mut Diag, lines: &[&str], i: usize, files: &mut Files) {
    // TeX's context: pairs of a line and, under it, the rest indented by
    // the first line's width; the last pair is the file level `l.<n> ...`.
    let mut frames: Vec<Frame> = Vec::new();
    let mut k = i + 1;
    let limit = (i + 60).min(lines.len());
    let mut file_level = None;
    while k + 1 < limit {
        let (b, a) = (lines[k], lines[k + 1]);
        let w = b.chars().count();
        let indented = w > 0 && a.chars().take(w).all(|c| c == ' ') && a.chars().count() >= w;
        if let Some(rest) = level_line(b) {
            if indented || a.trim().is_empty() {
                let after: String = a.chars().skip(w).collect();
                file_level = Some((rest.0, rest.1.to_string(), after));
                k += 2;
                break;
            }
        }
        if indented && !b.trim().is_empty() {
            let after: String = a.chars().skip(w).collect();
            frames.push(frame(b, &after));
            k += 2;
            continue;
        }
        if b.trim().is_empty() && frames.is_empty() && k > i + 12 {
            break;
        }
        k += 1;
    }
    let Some((line, before, after)) = file_level else {
        d.trace = frames;
        return;
    };
    // the help text: the lines after the context, up to a blank line
    let mut help = Vec::new();
    while k < lines.len() && !lines[k].trim().is_empty() && help.len() < 12 {
        help.push(lines[k].trim_end().to_string());
        k += 1;
    }
    d.help = help;
    if d.line.is_none() {
        d.line = Some(line);
    }
    let mut file_frame = Frame {
        kind: "file".into(),
        before: before.clone(),
        after: after.clone(),
        ..Frame::default()
    };
    if let (Some(path), Some(n)) = (d.file.clone(), d.line) {
        if let Some((start, src)) = files.line(&path, n) {
            if let Some(col) = split_column(src, &before) {
                d.col = Some(col as i64);
                d.offset = Some((start + col) as i64);
                let from = token_before(&src[..col]);
                if from < col {
                    d.range = Some((from as i64, col as i64));
                }
                file_frame.loc = Some(Loc {
                    file: Some(path.clone()),
                    line: Some(n),
                    col: Some(col as i64),
                    ..Loc::default()
                });
            }
        }
    }
    frames.push(file_frame);
    d.trace = frames;
}

/// `l.<n> <text>` -> (n, text).
fn level_line(l: &str) -> Option<(i64, &str)> {
    let rest = l.strip_prefix("l.")?;
    let digits = rest.find(|c: char| !c.is_ascii_digit())?;
    let n = rest[..digits].parse().ok()?;
    let text = rest[digits..].strip_prefix(' ').unwrap_or(&rest[digits..]);
    Some((n, text))
}

/// A context level other than the file's: `<argument> text`, or a macro
/// `\name #1->body` (TeX §314).
fn frame(before: &str, after: &str) -> Frame {
    let mut f = Frame {
        before: before.to_string(),
        after: after.to_string(),
        ..Frame::default()
    };
    if let Some(rest) = before.strip_prefix('<') {
        if let Some((label, _)) = rest.split_once('>') {
            f.kind = match label {
                "argument" => "argument",
                "template" => "template",
                "recently read" => "recently_read",
                "to be read again" => "backed_up",
                "inserted text" => "inserted",
                "output" => "output",
                "everypar" => "everypar",
                "everymath" => "everymath",
                "everydisplay" => "everydisplay",
                "everyhbox" => "everyhbox",
                "everyvbox" => "everyvbox",
                "everyjob" => "everyjob",
                "everycr" => "everycr",
                "mark" => "mark",
                "everyeof" => "everyeof",
                "write" => "write",
                "insert" => "insert",
                "read" => "read",
                _ => "token_list",
            }
            .into();
            return f;
        }
    }
    if before.starts_with('\\') {
        f.kind = "macro".into();
        let end = before.find([' ', '-', '#']).unwrap_or(before.len());
        f.name = Some(before[..end].to_string());
    } else {
        f.kind = "token_list".into();
    }
    f
}

/// The byte column of TeX's split in source line `src`, given the text
/// TeX printed before it (shortened to `...<end>` when long).
fn split_column(src: &str, before: &str) -> Option<usize> {
    if let Some(tail) = before.strip_prefix("...") {
        if tail.is_empty() {
            return None;
        }
        let p = src.find(tail)?;
        return Some(p + tail.len());
    }
    src.starts_with(before).then_some(before.len())
}

/// The start of the token that ends `s`: a control word `\name`, a control
/// symbol `\x`, or the last character.
fn token_before(s: &str) -> usize {
    let t = s.trim_end_matches(' ');
    if t.is_empty() {
        return s.len();
    }
    let letters = t.trim_end_matches(|c: char| c.is_alphabetic() || c == '@');
    if letters.len() < t.len() && letters.ends_with('\\') {
        return letters.len() - 1;
    }
    let mut it = t.char_indices().rev();
    let (last, _) = it.next().unwrap();
    if let Some((p, '\\')) = it.next() {
        return p;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_undefined_control_sequence_has_its_place_trace_and_help() {
        let dir = std::env::temp_dir().join(format!("ftx-uni-diag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("main.tex"),
            "\\documentclass{article}\n\\begin{document}\nnaïve \\undefinedmacro{} world.\n\\section{\\foo}\n\\end{document}\n",
        )
        .unwrap();
        // (TeX indents the rest of a context line by the width of its first part)
        let log = [
            "This is XeTeX",
            "./main.tex:3: Undefined control sequence.",
            "l.3 naïve \\undefinedmacro",
            &format!("{}{{}} world.", " ".repeat(25)),
            "The control sequence at the end of the top line",
            "of your error message was never \\def'ed.",
            "",
            "./main.tex:4: Undefined control sequence.",
            "<argument> \\foo ",
            &" ".repeat(16),
            "l.4 \\section{\\foo}",
            &" ".repeat(18),
            "The control sequence at the end of the top line",
            "",
        ]
        .join("\n");
        let d = from_log(&log, &dir, 7);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].code, "tex/undefined-control-sequence");
        assert_eq!(d[0].line, Some(3));
        assert_eq!(
            d[0].col,
            Some(22),
            "after `naïve \\undefinedmacro` (ï is two bytes)"
        );
        assert_eq!(d[0].range, Some((7, 22)));
        assert_eq!(d[0].help.len(), 2);
        assert_eq!(d[0].trace.last().map(|f| f.kind.as_str()), Some("file"));
        assert_eq!(d[1].col, Some(14));
        assert_eq!(
            d[1].range,
            Some((13, 14)),
            "the closing brace TeX read last"
        );
        assert_eq!(d[1].trace[0].kind, "argument");
        assert!(d.iter().all(|x| x.id == 7));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shortened_context_is_found_in_the_line() {
        assert_eq!(split_column("aaaa bbbb \\x cc", "...bbbb \\x"), Some(12));
        assert_eq!(split_column("abc", "abd"), None);
        assert_eq!(token_before("ab \\foo"), 3);
        assert_eq!(token_before("ab \\{"), 3);
        assert_eq!(token_before("abc"), 2);
    }
}
