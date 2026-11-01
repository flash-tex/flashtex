//! The amsthm head-to-body separator.
//!
//! A theorem-like environment is a `\trivlist` holding one `\item`, and the
//! gap between its head and the body is *not* an interword space. Two
//! different glues produce it, both measured against pdflatex (TeX Live
//! 2025, oracle only, never in the product path) with `\tracingoutput=1` on
//! an 11pt article:
//!
//! * a `\newtheorem`-declared environment ends its head with
//!   `\hskip\thm@headsep` (amsthm.sty `\@begintheorem`), and
//!   `\thm@headsep` is `5\p@ plus\p@ minus\p@` — the shipped page holds
//!   `\T1/cmr/bx/n/10.95 .` then `\glue 5.0 plus 1.0 minus 1.0`, an
//!   absolute value that does not scale with the class size;
//! * `proof` is not a `\newtheorem` style: it is
//!   `\item[\hskip\labelsep \itshape #1\@addpunct{.}]`, so the glue after
//!   the head is the `\hskip\labelsep` `\@item` appends *after* the label
//!   box. `\labelsep` is article's `.5em` at class-load size, rigid:
//!   `\showthe\labelsep` prints `5.0pt` at 10pt, `5.475pt` at 11pt and
//!   `5.87494pt` at 12pt, and the fixture's page traces
//!   `\hbox(...)x67.05003` (the `\@labels` box) followed by `\penalty 0`
//!   and the body's first word.
//!
//! amsthm's head ends with `\ignorespaces` (and `\label`'s `\@esphack`
//! restores it after removing and re-adding the same glue), so the source
//! newline after `\begin{theorem}` contributes nothing: this glue *replaces*
//! the interword space the adapter would otherwise read from the gap. Before
//! this module that space was a sentence space — `4.83948 plus 5.44014
//! minus 0.40297` at 11pt, because the head ends in `.` — which is both the
//! wrong width and, far more visibly on a stretched line, five times the
//! stretch amsthm asks for.
//!
//! The head is recognised from the compiler's own output rather than from a
//! `\newtheorem` scan: `parser::begin_theorem`/`begin_proof` are the only
//! places that give a synthesised `Inline::Text` the span of a `\begin`
//! control word, so a block whose first inline is a `Text` whose span covers
//! exactly `\begin` and whose text is not those bytes *is* a theorem-like
//! `\item`. A `\newenvironment` wrapper's begin code is stamped with the
//! whole `\begin{w}` invocation instead, so there the span alone cannot tell
//! a theorem head from ordinary begin-code text, and the caller's set of
//! theorem-like environments decides.

use flashtex_compiler::parser::Inline;
use flashtex_compiler::{DocumentId, Span};

/// `\thm@headsep`: amsthm.sty's `\newskip\thm@headsep \thm@headsep=5pt
/// plus1pt minus1pt`, also re-asserted by `\@thm` for every style.
const THM_HEADSEP: (f64, f64, f64) = (5.0, 1.0, 1.0);

/// Where a theorem-like head ends and what glue closes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HeadSeparator {
    /// The document the head was read from.
    pub(crate) document: DocumentId,
    /// Byte just past `\begin{<env>}` and its optional `[<note>]`: the first
    /// inline at or after this offset is the body, and the gap before it is
    /// the separator.
    pub(crate) head_end: usize,
    pub(crate) pt: f64,
    pub(crate) stretch_pt: f64,
    pub(crate) shrink_pt: f64,
}

/// A `\newenvironment{w}[args][default]{..\begin{T}..}{..}` wrapper of a
/// theorem-like `T`, as `\begin{w}` reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WrapperHead {
    /// `T` is `proof` (or wraps it): `\labelsep` after the head.
    pub(crate) proof: bool,
    /// The declared argument count, the optional one included.
    pub(crate) args: usize,
    /// The first argument is optional (a `[default]` was declared).
    pub(crate) optional: bool,
    /// Nothing follows `\begin{T}` in the begin code.
    pub(crate) inner_reads_note: bool,
}

/// The separator for `inlines`, if they open a theorem-like `\item`.
/// `size` is the class size (10/11/12), for `proof`'s `\labelsep`.
///
/// `wrapper(w)` answers for a `\newenvironment{w}` whose begin code opens a
/// theorem-like environment (GH-1126), with the call signature `\begin{w}`
/// reads. Its begin code carries the whole `\begin{w}` invocation as its
/// origin (tex-expansion `do_begin`), so the head synthesised inside it
/// spans `\begin{w}` rather than the bare `\begin`; the body starts after
/// that invocation and the wrapper's own arguments.
pub(crate) fn head_separator(
    source: &str,
    inlines: &[Inline],
    size: u32,
    wrapper: impl FnOnce(&str) -> Option<WrapperHead>,
) -> Option<HeadSeparator> {
    let Some(Inline::Text { text, span, .. }) = inlines.first() else {
        return None;
    };
    let head = source.get(span.start..span.end)?;
    if text == head {
        return None;
    }
    // The synthesised head carries the `\begin` control word's own span.
    let (mut at, proof) = if head == "\\begin" {
        let (name, at) = braced_group(source, span.end)?;
        if name.is_empty() {
            return None;
        }
        // `\@oparg`: amsthm reads the head's optional `[<note>]` (or
        // `proof`'s replacement heading) before any of the body.
        (bracket_group(source, at).unwrap_or(at), name == "proof")
    } else {
        let (name, end) = braced_group(head.strip_prefix("\\begin")?, 0)?;
        if name.is_empty() || "\\begin".len() + end != head.len() {
            return None;
        }
        let w = wrapper(name)?;
        // The wrapper's own arguments (`\newenvironment{w}[n][default]`):
        // the optional first one, if present, then the mandatory ones.
        // `\@ifnextchar[` looks past blanks and `%` comments.
        let next_bracket =
            |at: usize| tex_blanks(source, at).and_then(|j| bracket_group(source, j));
        let mut at = span.end;
        let mut mandatory = w.args;
        if w.optional {
            mandatory = mandatory.saturating_sub(1);
            if let Some(after) = next_bracket(at) {
                at = after;
            }
        }
        for _ in 0..mandatory {
            at = undelimited_argument(source, at)?;
        }
        // A begin code that ends at `\begin{T}` leaves `T`'s `\@oparg`
        // looking at the document: a `[<note>]` there is the head's.
        if w.inner_reads_note {
            at = next_bracket(at).unwrap_or(at);
        }
        (at, w.proof)
    };
    let (pt, stretch_pt, shrink_pt) = if proof {
        (labelsep_pt(size), 0.0, 0.0)
    } else {
        THM_HEADSEP
    };
    Some(HeadSeparator {
        document: span.document,
        head_end: at,
        pt,
        stretch_pt,
        shrink_pt,
    })
}

impl HeadSeparator {
    /// Whether `span` is the first thing after the head, i.e. the body — the
    /// gap in front of it is the separator.
    pub(crate) fn opens_the_body(&self, span: Span) -> bool {
        span.document == self.document && span.start >= self.head_end
    }
}

/// `\labelsep` at class-load size: article's `.5em` in the `\normalsize`
/// font, which is what `flashtex_document_style` resolves for every list.
fn labelsep_pt(size: u32) -> f64 {
    let base = match size {
        12 => flashtex_document_style::BaseSize::Pt12,
        11 => flashtex_document_style::BaseSize::Pt11,
        _ => flashtex_document_style::BaseSize::Pt10,
    };
    flashtex_document_style::list_level(base, 1).labelsep.0
}

/// The `{...}` group starting at or after `at` (spaces skipped): its content
/// and the byte after its closing brace.
fn braced_group(source: &str, at: usize) -> Option<(&str, usize)> {
    let start = skip_blanks(source, at)?;
    let rest = source.get(start..)?;
    let inner = rest.strip_prefix('{')?;
    let close = inner.find('}')?;
    Some((inner[..close].trim(), start + 1 + close + 1))
}

/// The byte after a `[...]` group at or after `at`, if one is there. Braces
/// inside it are honoured (`[\textbf{a}]`), as `\@oparg`'s argument scan is.
fn bracket_group(source: &str, at: usize) -> Option<usize> {
    let start = skip_blanks(source, at)?;
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    for (offset, byte) in source.get(start..)?.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b']' if depth == 0 => return Some(start + offset + 1),
            _ => {}
        }
    }
    None
}

/// The first byte at or after `at` that TeX reads as a token, as a macro's
/// argument scan or `\@ifnextchar` sees it: blanks, `%` comments (the
/// comment takes its line end with it) and a single line end are skipped,
/// and so are the spaces that open a line. `None` at a line end read at the
/// start of a line (a blank line: `\par`) or the end of the source.
fn tex_blanks(source: &str, at: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = at;
    let mut line_start = false;
    loop {
        match *bytes.get(i)? {
            b' ' | b'\t' | b'\r' => i += 1,
            b'\n' if line_start => return None,
            b'\n' => {
                line_start = true;
                i += 1;
            }
            b'%' => {
                i += bytes[i..].iter().position(|&c| c == b'\n')? + 1;
                line_start = true;
            }
            _ => return Some(i),
        }
    }
}

/// The byte after the undelimited macro argument at or after `at`: TeX skips
/// the blanks before it ([`tex_blanks`]), then takes a balanced `{...}`
/// group, a control sequence or one character. `None` at a blank line
/// (`\par`) or the end of the source.
fn undelimited_argument(source: &str, at: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let i = tex_blanks(source, at)?;
    match *bytes.get(i)? {
        b'{' => {
            let mut depth = 0i32;
            let mut k = i;
            while let Some(&b) = bytes.get(k) {
                match b {
                    b'\\' => k += 1,
                    b'%' => k += bytes[k..].iter().position(|&c| c == b'\n')?,
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(k + 1);
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            None
        }
        b'\\' => {
            let rest = source.get(i + 1..)?;
            let letters = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
            let len = if letters > 0 {
                letters
            } else {
                rest.chars().next()?.len_utf8()
            };
            Some(i + 1 + len)
        }
        _ => Some(i + source.get(i..)?.chars().next()?.len_utf8()),
    }
}

/// `at` with spaces and tabs skipped (never a newline: a blank line would
/// end the paragraph, and `\begin{theorem}\n\n` has no head at all).
fn skip_blanks(source: &str, at: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = at;
    while matches!(bytes.get(i), Some(b' ') | Some(b'\t')) {
        i += 1;
    }
    if i > source.len() {
        None
    } else {
        Some(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inlines(source: &str) -> Vec<Inline> {
        let parsed = flashtex_compiler::parser::parse(source);
        parsed
            .blocks
            .into_iter()
            .find_map(|b| match b {
                flashtex_compiler::parser::Block::Paragraph(inlines) => Some(inlines),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[test]
    fn newtheorem_head_ends_with_thm_headsep() {
        let source = "\\newtheorem{theorem}{Theorem}\n\\begin{theorem}\nBody text.\n\\end{theorem}";
        let sep = head_separator(source, &inlines(source), 11, |_| None).expect("a theorem head");
        assert_eq!((sep.pt, sep.stretch_pt, sep.shrink_pt), (5.0, 1.0, 1.0));
        assert_eq!(&source[sep.head_end..sep.head_end + 5], "\nBody");
    }

    #[test]
    fn the_note_is_part_of_the_head() {
        let source = "\\newtheorem{theorem}{Theorem}\n\\begin{theorem}[Division with remainder]\\label{k}\nBody.\n\\end{theorem}";
        let sep = head_separator(source, &inlines(source), 11, |_| None).expect("a theorem head");
        assert!(
            source[sep.head_end..].starts_with("\\label"),
            "the head ends after `[...]`, not inside it: {:?}",
            &source[sep.head_end..sep.head_end + 8]
        );
    }

    #[test]
    fn proof_takes_labelsep_rigid_at_the_class_size() {
        let source = "\\begin{proof}[Proof sketch]\nRun the algorithm.\n\\end{proof}";
        let sep = head_separator(source, &inlines(source), 11, |_| None).expect("a proof head");
        // `\showthe\labelsep` in an 11pt article prints 5.475pt.
        assert!((sep.pt - 5.475).abs() < 1e-4, "{sep:?}");
        assert_eq!((sep.stretch_pt, sep.shrink_pt), (0.0, 0.0));
        assert!((labelsep_pt(10) - 5.0).abs() < 1e-4);
        assert!((labelsep_pt(12) - 5.87494).abs() < 1e-4);
    }

    #[test]
    fn a_wrapper_head_spans_the_whole_invocation() {
        let source = "\\theoremstyle{remark}\\newtheorem{remark}{Remark}\n\\newenvironment{myremark}{\\begin{remark}}{\\end{remark}}\n\\begin{myremark}Beta body.\\end{myremark}";
        let inlines = inlines(source);
        let sep = head_separator(source, &inlines, 10, |w| {
            (w == "myremark").then(|| WrapperHead {
                inner_reads_note: true,
                ..WrapperHead::default()
            })
        })
        .expect("a wrapped theorem head");
        assert_eq!((sep.pt, sep.stretch_pt, sep.shrink_pt), (5.0, 1.0, 1.0));
        assert!(
            source[sep.head_end..].starts_with("Beta"),
            "{:?}",
            &source[sep.head_end..]
        );
        // Not a theorem-like wrapper: no head.
        assert_eq!(head_separator(source, &inlines, 10, |_| None), None);
    }

    #[test]
    fn undelimited_arguments_are_groups_control_sequences_or_characters() {
        let s = " {a{b}c}x";
        assert_eq!(&s[undelimited_argument(s, 0).unwrap()..], "x");
        let s = "\n {a}x";
        assert_eq!(&s[undelimited_argument(s, 0).unwrap()..], "x");
        let s = "\\foo bar";
        assert_eq!(&s[undelimited_argument(s, 0).unwrap()..], " bar");
        let s = "\\{x";
        assert_eq!(&s[undelimited_argument(s, 0).unwrap()..], "x");
        let s = "éx";
        assert_eq!(&s[undelimited_argument(s, 0).unwrap()..], "x");
        // A blank line is `\par`, not an argument.
        assert_eq!(undelimited_argument("\n\n{a}", 0), None);
        // A comment takes its line end, and the next line's spaces go too.
        let s = "% note\n   {a%}\n}x";
        assert_eq!(&s[undelimited_argument(s, 0).unwrap()..], "x");
        // ... so a line end right after it is a blank line.
        assert_eq!(undelimited_argument("%\n\n{a}", 0), None);
        assert_eq!(tex_blanks(" %c\n  [x]", 0), Some(6));
    }

    #[test]
    fn a_wrapper_skips_its_declared_arguments() {
        let source = "\\newtheorem{lemma}{Lemma}\n\\newenvironment{keylemma}[1]{\\begin{lemma}[#1]}{\\end{lemma}}\n\\begin{keylemma}{Important}Beta body.\\end{keylemma}";
        let inlines = inlines(source);
        let sep = head_separator(source, &inlines, 10, |w| {
            (w == "keylemma").then_some(WrapperHead {
                args: 1,
                ..WrapperHead::default()
            })
        })
        .expect("a wrapped theorem head");
        assert!(
            source[sep.head_end..].starts_with("Beta"),
            "{:?}",
            &source[sep.head_end..]
        );
    }

    #[test]
    fn an_ordinary_paragraph_has_no_head_separator() {
        let source = "Just a paragraph of text.";
        assert_eq!(head_separator(source, &inlines(source), 11, |_| None), None);
        let centred = "\\begin{center}\nCentred text.\n\\end{center}";
        assert_eq!(
            head_separator(centred, &inlines(centred), 11, |_| None),
            None
        );
    }
}
