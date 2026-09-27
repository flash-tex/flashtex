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
//!   and the body's first word;
//! * without amsthm, `\newtheorem` is the LaTeX kernel's, whose head is also
//!   an `\item` label (`\item[\hskip\labelsep{\bfseries Theorem 1\ (note)}]`,
//!   no punctuation): the same boxed head and rigid `\labelsep` as `proof`.
//!   The compiler emits amsthm's `\thm@headpunct` as a `.` run of its own
//!   on the `\begin` span, and the kernel head has none, which is how the
//!   two are told apart.
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
//! `\item`.

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
    /// Whether the head is an `\item[<label>]` label box: `proof`'s head,
    /// and a `\newtheorem` head when amsthm is not loaded (the kernel's
    /// `\@begintheorem` is `\item[\hskip\labelsep{\bfseries #1\ #2}]`).
    /// latex.ltx `\@item` sets the label in `\@tempboxa` and places
    /// `\box\@labels` (then `\penalty\z@`) when the paragraph starts, so the
    /// head is one box at its natural width: its interword glue neither
    /// stretches nor shrinks, and it is never broken or hyphenated. A
    /// `\newtheorem` head is not: amsthm's `\deferred@thm@head` also builds
    /// `\@labels`, but `\dth@everypar` *`\unhbox`es* it into the paragraph,
    /// so its words stay ordinary breakable, stretchable text. Measured with
    /// `\tracingparagraphs=1` (TeX Live 2026, oracle only): a proof head
    /// traces as `[]` before the body's first word, a theorem head as its
    /// own characters.
    pub(crate) boxed: bool,
    /// Whether the head is the LaTeX kernel's `\newtheorem` head (amsthm not
    /// loaded). Its environment is then a plain `\trivlist`, so its skips
    /// are `\@trivlist`'s, `\partopsep` included, not amsthm's
    /// `\thm@preskip`/`\thm@postskip` (`adapter::kernel_theorem_skips`).
    pub(crate) kernel: bool,
}

/// What the environment a head opens is, by name: the adapter's
/// `TheoremEnvs` (`\newtheorem` declarations, `proof`, and the
/// `\newenvironment`s whose begin code opens one of them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvKind {
    /// `proof`, or an environment whose begin code opens one.
    Proof,
    /// A `\newtheorem` environment, or one whose begin code opens one.
    Theorem,
    /// Anything else whose macro-expanded text carries the `\begin` span
    /// (`\newenvironment{solution}{\textit{Solution.}\ }{}`): no head and no
    /// separator, its text and spaces are the environment's own. Reading
    /// `Solution.` as an amsthm head swapped the `\ ` and the source blank
    /// after `\begin{solution}` for `\thm@headsep`, so the gap was 5pt
    /// ±1pt instead of pdflatex's `\ ` plus a sentence space (7.78pt).
    Other,
}

/// The separator for `inlines`, if they open a theorem-like `\item`.
/// `size` is the class size (10/11/12), for `proof`'s `\labelsep`; `kind`
/// says what the named environment is, and is asked only once a head is
/// found.
pub(crate) fn head_separator(source: &str, inlines: &[Inline], size: u32, kind: impl FnOnce(&str) -> EnvKind) -> Option<HeadSeparator> {
    let Some(Inline::Text { text, span, .. }) = inlines.first() else {
        return None;
    };
    // The synthesised head carries the `\begin` control word's own span.
    if source.get(span.start..span.end) != Some("\\begin") || text == "\\begin" {
        return None;
    }
    let (name, mut at) = braced_group(source, span.end)?;
    if name.is_empty() {
        return None;
    }
    // `\@oparg`: amsthm reads the head's optional `[<note>]` (or `proof`'s
    // replacement heading) before any of the body.
    if let Some(after) = bracket_group(source, at) {
        at = after;
    }
    // amsthm's `\newtheorem` head ends with `\the\thm@headpunct`, which the
    // compiler sets as its own `.` run on the `\begin` span; the LaTeX
    // kernel's head (`parser::begin_kernel_theorem`, amsthm not loaded) has
    // no punctuation. Both kernel heads and `proof`'s are `\item` labels.
    let kind = kind(name);
    if kind == EnvKind::Other {
        return None;
    }
    let amsthm_head = inlines
        .iter()
        .any(|inline| matches!(inline, Inline::Text { text, span: s, .. } if text == "." && s == span));
    let kernel = kind == EnvKind::Theorem && !amsthm_head;
    let boxed = kind == EnvKind::Proof || kernel;
    let (pt, stretch_pt, shrink_pt) = if boxed {
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
        boxed,
        kernel,
    })
}

/// Whether `inlines` open with a LaTeX kernel `\newtheorem` head (amsthm not
/// loaded): see [`HeadSeparator::kernel`].
pub(crate) fn kernel_theorem_head(source: &str, inlines: &[Inline], kind: impl FnOnce(&str) -> EnvKind) -> bool {
    head_separator(source, inlines, 10, kind).is_some_and(|sep| sep.kernel)
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

    /// `name`'s kind as the adapter's `TheoremEnvs` gives it for these
    /// sources: `proof`, and `theorem` declared by `\newtheorem`.
    fn kind(name: &str) -> EnvKind {
        match name {
            "proof" => EnvKind::Proof,
            "theorem" => EnvKind::Theorem,
            _ => EnvKind::Other,
        }
    }

    const AMSTHM: &str = "\\documentclass[11pt]{article}\n\\usepackage{amsthm}\n\\newtheorem{theorem}{Theorem}\n\\begin{document}\n";

    #[test]
    fn newtheorem_head_ends_with_thm_headsep() {
        let source = format!("{AMSTHM}\\begin{{theorem}}\nBody text.\n\\end{{theorem}}\n\\end{{document}}");
        let sep = head_separator(&source, &inlines(&source), 11, kind).expect("a theorem head");
        assert_eq!((sep.pt, sep.stretch_pt, sep.shrink_pt), (5.0, 1.0, 1.0));
        assert_eq!(&source[sep.head_end..sep.head_end + 5], "\nBody");
        assert!(!sep.boxed, "amsthm `\\unhbox`es a theorem head into the paragraph");
        assert!(!sep.kernel);
    }

    #[test]
    fn the_note_is_part_of_the_head() {
        let source = format!("{AMSTHM}\\begin{{theorem}}[Division with remainder]\\label{{k}}\nBody.\n\\end{{theorem}}\n\\end{{document}}");
        let sep = head_separator(&source, &inlines(&source), 11, kind).expect("a theorem head");
        assert!(
            source[sep.head_end..].starts_with("\\label"),
            "the head ends after `[...]`, not inside it: {:?}",
            &source[sep.head_end..sep.head_end + 8]
        );
    }

    /// A kernel head is amsthm's without the `\thm@headpunct` run (the
    /// compiler's `begin_kernel_theorem`); built here from amsthm's so the
    /// test does not depend on which compiler is vendored.
    #[test]
    fn a_kernel_theorem_head_is_a_label_box_with_labelsep() {
        let source = format!("{AMSTHM}\\begin{{theorem}}[Lock cleanup]\nBody text.\n\\end{{theorem}}\n\\end{{document}}");
        let mut inlines = inlines(&source);
        let begin = match inlines.first() {
            Some(Inline::Text { span, .. }) => *span,
            other => panic!("a theorem head, not {other:?}"),
        };
        let before = inlines.len();
        inlines.retain(|inline| !matches!(inline, Inline::Text { text, span, .. } if text == "." && *span == begin));
        assert_eq!(inlines.len(), before - 1, "amsthm's head has exactly one `\\thm@headpunct` run");
        let sep = head_separator(&source, &inlines, 11, kind).expect("a theorem head");
        assert!(sep.kernel && sep.boxed, "{sep:?}");
        assert!((sep.pt - 5.475).abs() < 1e-4, "the kernel's `\\@item` `\\hskip\\labelsep`: {sep:?}");
        assert_eq!((sep.stretch_pt, sep.shrink_pt), (0.0, 0.0));
        assert!(kernel_theorem_head(&source, &inlines, kind));
        // Only `\newtheorem` environments and proofs have a head: a
        // `\newenvironment` whose expanded text carries the `\begin` span
        // (`{\textit{Solution.}\ }`) is ordinary text.
        assert_eq!(head_separator(&source, &inlines, 11, |_| EnvKind::Other), None);
    }

    #[test]
    fn proof_takes_labelsep_rigid_at_the_class_size() {
        let source = format!("{AMSTHM}\\begin{{proof}}[Proof sketch]\nRun the algorithm.\n\\end{{proof}}\n\\end{{document}}");
        let sep = head_separator(&source, &inlines(&source), 11, kind).expect("a proof head");
        // `\showthe\labelsep` in an 11pt article prints 5.475pt.
        assert!((sep.pt - 5.475).abs() < 1e-4, "{sep:?}");
        assert_eq!((sep.stretch_pt, sep.shrink_pt), (0.0, 0.0));
        assert!(sep.boxed, "a proof head is an `\\item` label box");
        assert!(!sep.kernel);
        assert!((labelsep_pt(10) - 5.0).abs() < 1e-4);
        assert!((labelsep_pt(12) - 5.87494).abs() < 1e-4);
    }

    #[test]
    fn an_ordinary_paragraph_has_no_head_separator() {
        let source = "Just a paragraph of text.";
        assert_eq!(head_separator(source, &inlines(source), 11, kind), None);
        let centred = "\\begin{center}\nCentred text.\n\\end{center}";
        assert_eq!(head_separator(centred, &inlines(centred), 11, kind), None);
    }
}
