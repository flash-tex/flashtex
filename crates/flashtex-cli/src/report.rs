//! The full (rustc-style) form of a diagnostic on stderr: a header, the
//! source lines it points at under a line-number gutter, carets under the
//! span, and `= recovery:` lines. `--diagnostics=short` keeps the one-line
//! `Diagnostic::line_text` form, which is also the default when stderr is not
//! a terminal, so piped output and scripts see exactly what they always did.
//!
//! ```text
//! error[compiler]: \tilde is not supported by this compiler version
//!   --> notes.tex:91:38
//!    |
//! 91 |     \item "\ul{candidate content}" \(\tilde{c}_t=g(W_cz_t+b_c)\) where
//!    |                                      ^^^^^^
//!    |
//!    = recovery: skipped the command
//! ```

use crate::compile::Diagnostic;

/// How diagnostics are printed on stderr (`--diagnostics`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Short,
    Full,
}

/// The column the `= recovery:` text wraps at.
const WIDTH: usize = 80;
/// Tabs are shown as this many spaces so carets line up.
const TAB: &str = "    ";
/// A span covering more lines than this shows its first and last lines only.
const MAX_SPAN_LINES: usize = 4;

struct Paint {
    on: bool,
}

impl Paint {
    fn wrap(&self, code: &str, s: &str) -> String {
        if self.on && !s.is_empty() {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
}

/// `source` is the text of `d.path` when the project has that document.
pub fn render_full(d: &Diagnostic, source: Option<&str>, color: bool) -> String {
    let p = Paint { on: color };
    let sev_code = if d.error { "1;31" } else { "1;33" };
    let blue = "1;34";
    let mut out = String::new();
    out.push_str(&p.wrap(sev_code, &format!("{}[{}]", d.severity(), d.code)));
    out.push_str(&p.wrap("1", &format!(": {}", d.message)));
    out.push('\n');

    let excerpt = match (source, d.start_byte) {
        (Some(text), Some(start)) => excerpt_lines(text, start, d.end_byte.unwrap_or(start)),
        _ => Vec::new(),
    };
    let gutter = excerpt.iter().filter_map(|l| l.number).max().map_or(1, digits);
    let pad = " ".repeat(gutter);
    let at = match (d.line, d.column) {
        (Some(l), Some(c)) => format!("{}:{l}:{c}", d.path),
        _ => d.path.clone(),
    };
    out.push_str(&format!("{pad}{} {at}\n", p.wrap(blue, "-->")));

    let bar = p.wrap(blue, "|");
    if !excerpt.is_empty() {
        out.push_str(&format!("{pad} {bar}\n"));
        for line in &excerpt {
            match line.number {
                Some(n) => {
                    let num = p.wrap(blue, &format!("{n:>gutter$}"));
                    out.push_str(format!("{num} {bar} {}", line.text).trim_end());
                    out.push('\n');
                    let (lead, width) = line.caret;
                    if width > 0 {
                        let carets = p.wrap(sev_code, &"^".repeat(width));
                        out.push_str(&format!("{pad} {bar} {}{carets}\n", " ".repeat(lead)));
                    }
                }
                None => out.push_str(&format!("{}\n", p.wrap(blue, &format!("{:>gutter$}", "...")))),
            }
        }
    }
    if let Some(r) = &d.recovery {
        if !excerpt.is_empty() {
            out.push_str(&format!("{pad} {bar}\n"));
        }
        let label = format!("{pad} {} recovery: ", p.wrap(blue, "="));
        let indent = gutter + " = recovery: ".len();
        out.push_str(&wrap_text(&label, indent, r));
    }
    out
}

struct ExcerptLine {
    /// `None` is the `...` elision row of a long span.
    number: Option<usize>,
    /// The line with tabs expanded, no newline.
    text: String,
    /// Display columns before the carets, and how many carets.
    caret: (usize, usize),
}

/// The lines `start..end` touches, each with the part of it the span covers.
fn excerpt_lines(text: &str, start: usize, end: usize) -> Vec<ExcerptLine> {
    let start = floor_boundary(text, start.min(text.len()));
    let end = floor_boundary(text, end.clamp(start, text.len()));
    let first_line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let first_number = text[..start].matches('\n').count() + 1;

    let mut rows = Vec::new();
    let mut line_start = first_line_start;
    let mut number = first_number;
    loop {
        let line_end = text[line_start..].find('\n').map_or(text.len(), |i| line_start + i);
        let raw = text[line_start..line_end].trim_end_matches('\r');
        let from = start.max(line_start) - line_start;
        let to = end.min(line_start + raw.len()).max(line_start) - line_start;
        let lead = display_width(&raw[..from.min(raw.len())]);
        let covered = display_width(&raw[from.min(raw.len())..to.max(from).min(raw.len())]);
        // An empty span, or one that starts at the end of the line, still
        // gets one caret on its first line so the position is visible.
        let width = if covered == 0 && number == first_number { 1 } else { covered };
        rows.push(ExcerptLine { number: Some(number), text: raw.replace('\t', TAB), caret: (lead, width) });
        if line_end >= end || line_end >= text.len() {
            break;
        }
        line_start = line_end + 1;
        number += 1;
    }
    if rows.len() > MAX_SPAN_LINES {
        let last = rows.pop().expect("non-empty");
        rows.truncate(2);
        rows.push(ExcerptLine { number: None, text: String::new(), caret: (0, 0) });
        rows.push(last);
    }
    rows
}

fn floor_boundary(text: &str, mut b: usize) -> usize {
    while b > 0 && !text.is_char_boundary(b) {
        b -= 1;
    }
    b
}

fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c == '\t' { TAB.len() } else { 1 }).sum()
}

fn digits(n: usize) -> usize {
    n.to_string().len()
}

/// `label` then `text` wrapped by words at `WIDTH`, continuation lines
/// indented to `indent` columns.
fn wrap_text(label: &str, indent: usize, text: &str) -> String {
    let mut out = label.to_string();
    let mut col = indent;
    let mut first = true;
    for word in text.split_whitespace() {
        let w = word.chars().count();
        if !first && col + 1 + w > WIDTH {
            out.push('\n');
            out.push_str(&" ".repeat(indent));
            col = indent;
        } else if !first {
            out.push(' ');
            col += 1;
        }
        out.push_str(word);
        col += w;
        first = false;
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(text: &str, needle: &str, len: usize, recovery: Option<&str>) -> Diagnostic {
        let start = text.find(needle).expect("needle");
        let (line, column) = crate::compile::line_col(text, start);
        Diagnostic {
            path: "notes.tex".into(),
            line: Some(line),
            column: Some(column),
            start_byte: Some(start),
            end_byte: Some(start + len),
            error: true,
            code: "compiler".into(),
            message: "\\tilde is not supported by this compiler version".into(),
            recovery: recovery.map(str::to_string),
        }
    }

    #[test]
    fn single_line_span_gets_gutter_and_carets() {
        let text = "\\documentclass{article}\n\\begin{document}\nx \\(\\tilde{c}\\) y\n\\end{document}\n";
        let d = diag(text, "\\tilde", 6, Some("skipped the command"));
        assert_eq!(
            render_full(&d, Some(text), false),
            "error[compiler]: \\tilde is not supported by this compiler version\n \
             --> notes.tex:3:5\n  \
             |\n\
             3 | x \\(\\tilde{c}\\) y\n  \
             |     ^^^^^^\n  \
             |\n  \
             = recovery: skipped the command\n"
        );
    }

    #[test]
    fn no_position_prints_header_and_path_only() {
        let mut d = diag("abc", "b", 1, None);
        d.line = None;
        d.column = None;
        d.start_byte = None;
        d.end_byte = None;
        d.error = false;
        assert_eq!(
            render_full(&d, None, false),
            "warning[compiler]: \\tilde is not supported by this compiler version\n --> notes.tex\n"
        );
    }

    #[test]
    fn tabs_multibyte_and_empty_spans_line_up() {
        let text = "\té \\x\n";
        let d = diag(text, "\\x", 0, None);
        let out = render_full(&d, Some(text), false);
        assert!(out.contains("1 |     é \\x\n  |       ^\n"), "{out}");
    }

    #[test]
    fn long_multi_line_span_is_elided() {
        let text = "a\n\\begin{x}\n1\n2\n3\n4\n\\end{x}\nz\n";
        let start = text.find("\\begin").unwrap();
        let end = text.find("\\end{x}").unwrap() + 7;
        let mut d = diag(text, "\\begin", 0, None);
        d.end_byte = Some(end);
        assert_eq!(d.start_byte, Some(start));
        let out = render_full(&d, Some(text), false);
        assert!(out.contains("2 | \\begin{x}\n  | ^^^^^^^^^\n3 | 1\n  | ^\n...\n7 | \\end{x}\n  | ^^^^^^^\n"), "{out}");
    }

    #[test]
    fn span_past_end_of_text_is_clamped() {
        let text = "abc";
        let mut d = diag(text, "c", 1, None);
        d.end_byte = Some(99);
        assert!(render_full(&d, Some(text), false).contains("1 | abc\n  |   ^\n"));
    }

    #[test]
    fn recovery_wraps_at_80_columns_with_hanging_indent() {
        let text = "x\n";
        let long = "skipped the command; any braced argument was typeset as plain text and the rest of the line kept";
        let d = diag(text, "x", 1, Some(long));
        let out = render_full(&d, Some(text), false);
        let lines: Vec<&str> = out.lines().filter(|l| l.contains("recovery") || l.starts_with("             ")).collect();
        assert_eq!(lines.len(), 2, "{out}");
        assert!(out.lines().all(|l| l.chars().count() <= WIDTH), "{out}");
        assert!(lines[1].starts_with(&" ".repeat(" = recovery: ".len() + 1)), "{out}");
    }

    #[test]
    fn color_wraps_severity_and_carets_only_when_on() {
        let text = "x\n";
        let d = diag(text, "x", 1, None);
        assert!(!render_full(&d, Some(text), false).contains('\x1b'));
        let colored = render_full(&d, Some(text), true);
        assert!(colored.starts_with("\x1b[1;31merror[compiler]\x1b[0m"), "{colored:?}");
        assert!(colored.contains("\x1b[1;31m^\x1b[0m"), "{colored:?}");
    }
}
