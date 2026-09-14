//! The full (rustc-style) form of a diagnostic on stderr: a header, the
//! source lines it points at under a line-number gutter, carets under the
//! span, `= recovery:` lines, and when a suggestion is present a `= help:`
//! block with a `+` gutter over the replacement. `--diagnostics=short` keeps
//! the one-line `Diagnostic::line_text` form, which is also the default when
//! stderr is not a terminal, so piped output and scripts see exactly what
//! they always did.
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
    if let (Some(suggestion), Some(start)) = (&d.suggestion, d.start_byte) {
        if d.recovery.is_none() && !excerpt.is_empty() {
            out.push_str(&format!("{pad} {bar}\n"));
        }
        let help = format!("did you mean `{suggestion}`?");
        let label = format!("{pad} {} help: ", p.wrap(blue, "="));
        let indent = gutter + " = help: ".len();
        out.push_str(&wrap_text(&label, indent, &help));
        if let Some(text) = source {
            let end = d.end_byte.unwrap_or(start);
            let plus = suggestion_excerpt(text, start, end, suggestion);
            let gutter = plus.iter().filter_map(|l| l.number).max().map_or(gutter, |n| gutter.max(digits(n)));
            let pad = " ".repeat(gutter);
            out.push_str(&format!("{pad} {bar}\n"));
            let green = "1;32";
            for line in &plus {
                match line.number {
                    Some(n) => {
                        let num = p.wrap(blue, &format!("{n:>gutter$}"));
                        out.push_str(format!("{num} {bar} {}", line.text).trim_end());
                        out.push('\n');
                        let (lead, width) = line.caret;
                        if width > 0 {
                            let marks = p.wrap(green, &"+".repeat(width));
                            out.push_str(&format!("{pad} {bar} {}{marks}\n", " ".repeat(lead)));
                        }
                    }
                    None => out.push_str(&format!("{}\n", p.wrap(blue, &format!("{:>gutter$}", "...")))),
                }
            }
        }
    }
    out
}

/// Folds positionless diagnostics that differ only in a leading `name:`
/// (one per math font, say), wherever else the message repeats that name,
/// into one entry at the first one's place listing every name. Everything
/// else passes through in order. Terminal presentation only: the counts,
/// `--json` and the short form never see it.
pub fn collapse_repeats(diags: &[Diagnostic]) -> Vec<Diagnostic> {
    /// The leading name and the message with every occurrence of it masked.
    fn key(d: &Diagnostic) -> Option<(&str, String)> {
        if d.start_byte.is_some() {
            return None;
        }
        let (name, rest) = d.message.split_once(": ")?;
        (!name.is_empty() && !name.contains(char::is_whitespace)).then(|| (name, rest.replace(name, "\u{0}")))
    }
    let mut out: Vec<Diagnostic> = Vec::new();
    // Per folded entry of `out`: its index, masked message and names so far.
    let mut folds: Vec<(usize, String, Vec<String>)> = Vec::new();
    for d in diags {
        let Some((name, masked)) = key(d) else {
            out.push(d.clone());
            continue;
        };
        let found = folds.iter_mut().find(|(i, m, _)| {
            let o = &out[*i];
            *m == masked && o.path == d.path && o.code == d.code && o.error == d.error && o.recovery == d.recovery && o.suggestion == d.suggestion
        });
        match found {
            Some((i, m, names)) => {
                names.push(name.to_string());
                let list = names.join(", ");
                out[*i].message = format!("{list}: {}", m.replace('\u{0}', &list));
            }
            None => {
                folds.push((out.len(), masked, vec![name.to_string()]));
                out.push(d.clone());
            }
        }
    }
    out
}

/// The source line(s) with `start..end` replaced by `suggestion`, carets
/// sized to the replacement so the renderer can draw a `+` gutter.
fn suggestion_excerpt(text: &str, start: usize, end: usize, suggestion: &str) -> Vec<ExcerptLine> {
    let start = floor_boundary(text, start.min(text.len()));
    let end = floor_boundary(text, end.clamp(start, text.len()));
    let mut replaced = String::with_capacity(text.len() - (end - start) + suggestion.len());
    replaced.push_str(&text[..start]);
    replaced.push_str(suggestion);
    replaced.push_str(&text[end..]);
    excerpt_lines(&replaced, start, start + suggestion.len())
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
            suggestion: None,
        }
    }

    fn positionless(name: &str, rest: &str) -> Diagnostic {
        Diagnostic {
            path: "HW1.tex".into(),
            line: None,
            column: None,
            start_byte: None,
            end_byte: None,
            error: false,
            code: "math_resource_profile".into(),
            message: format!("{name}: {rest}"),
            recovery: None,
            suggestion: None,
        }
    }

    #[test]
    fn repeats_differing_only_in_a_leading_name_are_folded_in_place() {
        let rest = "glyphs drawn from latinmodern-math (one 10pt design)";
        let text = "x\n";
        let located = diag(text, "x", 1, None);
        let other = positionless("lmr10", "a different message");
        let input = vec![positionless("lmex10", rest), located.clone(), positionless("lmmi10", rest), other.clone(), positionless("lmsy10", rest)];
        let out = collapse_repeats(&input);
        assert_eq!(out.len(), 3, "{out:#?}");
        assert_eq!(out[0].message, format!("lmex10, lmmi10, lmsy10: {rest}"));
        // The name repeated inside the message does not stop the fold.
        let own = |n: &str| positionless(n, &format!("not the reference's {n} design"));
        let folded = collapse_repeats(&[own("lmex10"), own("lmmi8")]);
        assert_eq!(folded.len(), 1, "{folded:#?}");
        assert_eq!(folded[0].message, "lmex10, lmmi8: not the reference's lmex10, lmmi8 design");
        assert_eq!(out[1], located);
        assert_eq!(out[2], other);
    }

    #[test]
    fn a_single_or_located_diagnostic_is_never_rewritten() {
        let text = "a: b\n";
        let mut located = diag(text, "a", 1, None);
        located.message = "a: b".into();
        let input = vec![located.clone(), located.clone(), positionless("lmex10", "once")];
        assert_eq!(collapse_repeats(&input), input);
        let mut spaced = positionless("two words", "same");
        spaced.message = "two words: same".into();
        assert_eq!(collapse_repeats(&[spaced.clone(), spaced.clone()]), vec![spaced.clone(), spaced]);
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

    #[test]
    fn suggestion_prints_help_and_plus_gutter() {
        let text = "\\documentclass{article}\n\\begin{document}\nHello $\\alpah$ world.\n\\end{document}\n";
        let start = text.find("\\alpah").unwrap();
        let mut d = diag(text, "\\alpah", 6, Some("typeset the command literally and continued"));
        d.code = "unknown_command".into();
        d.message = "\\alpah is not supported in math mode".into();
        d.suggestion = Some("\\alpha".into());
        assert_eq!(d.start_byte, Some(start));
        let out = render_full(&d, Some(text), false);
        assert!(out.contains("= help: did you mean `\\alpha`?\n"), "{out}");
        assert!(out.contains("3 | Hello $\\alpah$ world.\n  |        ^^^^^^\n"), "{out}");
        assert!(out.contains("3 | Hello $\\alpha$ world.\n  |        ++++++\n"), "{out}");
        let short = d.line_text();
        assert!(short.contains("(did you mean \\alpha?)"), "{short}");
        let colored = render_full(&d, Some(text), true);
        assert!(colored.contains("\x1b[1;32m++++++\x1b[0m"), "{colored:?}");
    }

    #[test]
    fn no_suggestion_keeps_the_existing_recovery_snapshot() {
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
        assert!(!d.line_text().contains("did you mean"), "{}", d.line_text());
    }
}
