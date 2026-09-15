//! Manual bibliographies: `thebibliography`, `\bibitem`, `\cite`, `\nocite`.
//!
//! A `\bibitem`'s citation label depends only on how many earlier `\bibitem`s
//! precede it (or its own optional-label override) — never on page numbers or
//! line breaks. That makes it fundamentally simpler than `\label`/`\ref`,
//! which need the page-aware two-pass resolution in `layout.rs`: a
//! [`prescan`] of the token stream the parser walks (after macro expansion) is enough to
//! resolve every `\cite` in one pass, even one that appears (as citations
//! normally do) before the `thebibliography` it points into. This module is
//! deliberately self-contained and does not touch that separate label/ref
//! resolution pass.
//!
//! BibTeX/biblatex `.bib` files are out of scope; `\bibliography` and
//! `\bibliographystyle` in `parser.rs` report that honestly instead of
//! silently doing nothing.

use std::borrow::Borrow;
use std::collections::HashMap;

use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::natbib;
use crate::parser::{Inline, TextStyle};
use crate::Span;

/// One `\bibitem`'s resolved citation label — the bracket content `\cite`
/// substitutes: a sequential number for a plain `\bibitem{key}`, or the
/// verbatim optional argument of `\bibitem[label]{key}`.
#[derive(Debug, Clone)]
struct BibItem {
    label: String,
    /// The `\bibcite` record natbib's `\@lbibitem` writes for this entry
    /// (`{num}{date}{{name}}{{all names}}`), when natbib is loaded. `None`
    /// for a label natbib would call non-compliant — which is also what makes
    /// [`Bibliography::force_numbers`] true.
    entry: Option<natbib::Entry>,
    /// The printed marker of the entry in `thebibliography`, already
    /// bracketed: `[1]` normally, `[Knuth 1984]` for an overridden label, and
    /// **empty** under natbib's author-year mode, whose `\@biblabel` is
    /// `\hfill` (natbib.sty line 622).
    marker: String,
}

/// Every `\bibitem` found by [`prescan`], in document order, plus a
/// key → item lookup for `\cite`.
#[derive(Debug, Default)]
pub struct Bibliography {
    items: Vec<BibItem>,
    keys: HashMap<String, usize>,
    /// `\usepackage[...]{natbib}`'s resolved options, when the document
    /// loads it.
    natbib: Option<natbib::Options>,
    /// natbib's `\NAT@stdbst`: an entry whose label carries no author-year
    /// data. `\NAT@force@numbers` (natbib.sty line 974) then makes the whole
    /// document numeric on the next run, which is the steady state a
    /// two-pass pdfLaTeX run reaches.
    force_numbers: bool,
}

impl Bibliography {
    /// natbib's resolved options, once `\usepackage{natbib}` has been seen,
    /// with `\NAT@force@numbers` already applied. `None` when the document
    /// does not load natbib, and `\cite` keeps its kernel meaning.
    pub fn natbib(&self) -> Option<&natbib::Options> {
        self.natbib.as_ref()
    }

    /// natbib refused the bibliography's labels and fell back to numeric
    /// citations (natbib.sty line 974). Reported once by the parser.
    pub fn natbib_forced_numbers(&self) -> bool {
        self.force_numbers
    }

    fn push(
        &mut self,
        key: String,
        label: String,
        entry: Option<natbib::Entry>,
        marker: String,
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) {
        let index = self.items.len();
        self.items.push(BibItem {
            label,
            entry,
            marker,
        });
        if key.is_empty() {
            return;
        }
        if self.keys.insert(key.clone(), index).is_some() {
            diags.push(Diagnostic::warning(
                format!("duplicate \\bibitem{{{key}}}; the second definition wins"),
                Some(span),
                Some("used the later entry's label for \\cite".into()),
            ));
        }
    }

    /// The label of the `index`th (0-based) `\bibitem` in document order,
    /// consulted by the real parse — see `parser::P::bib_cursor` — which
    /// walks the same literal `\bibitem`s this pre-scan already numbered.
    pub fn label_at(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(|item| item.label.as_str())
    }

    /// The printed marker of the `index`th entry, already bracketed. Empty
    /// under natbib author-year, whose `\@biblabel` is `\hfill`.
    pub fn marker_at(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(|item| item.marker.as_str())
    }

    fn resolve(&self, key: &str) -> Option<&str> {
        self.keys
            .get(key)
            .and_then(|&index| self.items.get(index))
            .map(|item| item.label.as_str())
    }

    /// The `\bibcite` record for a key, for `natbib::cite_inlines`.
    pub fn entry(&self, key: &str) -> Option<natbib::Entry> {
        self.keys
            .get(key)
            .and_then(|&index| self.items.get(index))
            .and_then(|item| item.entry.clone())
    }
}

/// One `\bibitem` as the pre-scan read it, before the labels are assigned:
/// natbib's `\NAT@force@numbers` can only be decided once every entry has
/// been seen, and it changes what every marker prints.
struct RawItem {
    key: String,
    label: Option<String>,
    span: Span,
}

/// Scans a token stream for every `\bibitem` inside a `thebibliography`
/// environment, in order, and assigns each one's citation label (a plain
/// `\bibitem{key}` numbers sequentially; `\bibitem[label]{key}` uses `label`
/// verbatim and does not consume a number, mirroring real LaTeX's
/// `\@lbibitem`). The parser passes the same expanded stream it walks, so
/// `P::bib_cursor` meets exactly these `\bibitem`s (including one a macro
/// produced, and never one under `\iffalse`).
///
/// Under `\usepackage{natbib}` the rules are natbib's `\@lbibitem` instead
/// (natbib.sty line 827): **every** entry advances `\c@NAT@ctr`, labelled or
/// not, the optional argument is parsed into a `\bibcite` author-year record,
/// and the printed marker comes from natbib's own `\@biblabel` — nothing at
/// all in author-year mode.
pub fn prescan<T: Borrow<Token>>(tokens: &[T], diags: &mut Vec<Diagnostic>) -> Bibliography {
    let mut bibliography = Bibliography::default();
    bibliography.natbib = natbib_options(tokens);
    let mut raw: Vec<RawItem> = Vec::new();
    let mut in_bibliography = false;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i].borrow().kind {
            TokenKind::Command(name) if name == "begin" || name == "end" => {
                let is_begin = name == "begin";
                match group_text(tokens, i + 1) {
                    Some((environment, after)) => {
                        if environment.trim() == "thebibliography" {
                            in_bibliography = is_begin;
                        }
                        i = after;
                    }
                    None => i += 1,
                }
            }
            TokenKind::Command(name) if name == "bibitem" && in_bibliography => {
                let span = tokens[i].borrow().span;
                let mut cursor = i + 1;
                let mut label = None;
                if let Some((text, after)) = optional_bracket_text(tokens, cursor) {
                    label = Some(text);
                    cursor = after;
                }
                if let Some((key, after)) = group_text(tokens, cursor) {
                    cursor = after;
                    raw.push(RawItem {
                        key: key.trim().to_string(),
                        label,
                        span,
                    });
                }
                i = cursor;
            }
            _ => i += 1,
        }
    }
    match bibliography.natbib.clone() {
        Some(options) => fill_natbib(&mut bibliography, options, raw, diags),
        None => fill_kernel(&mut bibliography, raw, diags),
    }
    bibliography
}

/// The LaTeX kernel's `\@lbibitem`: an optional label is used verbatim and
/// does not consume a number.
fn fill_kernel(bibliography: &mut Bibliography, raw: Vec<RawItem>, diags: &mut Vec<Diagnostic>) {
    let mut next_number: u32 = 1;
    for item in raw {
        let label = item.label.unwrap_or_else(|| {
            let n = next_number;
            next_number += 1;
            n.to_string()
        });
        let marker = label_bracket(&label);
        bibliography.push(item.key, label, None, marker, item.span, diags);
    }
}

/// natbib's `\@lbibitem` plus the end-of-document `\NAT@force@numbers`.
fn fill_natbib(
    bibliography: &mut Bibliography,
    options: natbib::Options,
    raw: Vec<RawItem>,
    diags: &mut Vec<Diagnostic>,
) {
    let parsed: Vec<natbib::Label> = raw
        .iter()
        .enumerate()
        .map(|(index, item)| natbib::parse_label(item.label.as_deref(), index + 1))
        .collect();
    // `\NAT@stdbst`: any entry with no author-year data at all.
    bibliography.force_numbers = parsed
        .iter()
        .any(|label| matches!(label, natbib::Label::Standard { .. }));
    let numbers = options.numbers || bibliography.force_numbers;
    if let Some(natbib) = bibliography.natbib.as_mut() {
        natbib.numbers = numbers;
    }
    for (item, label) in raw.into_iter().zip(parsed) {
        let entry = match &label {
            natbib::Label::AuthorYear(entry) | natbib::Label::Apalike(entry) => Some(entry.clone()),
            natbib::Label::Standard { num } => Some(natbib::Entry {
                num: num.clone(),
                ..natbib::Entry::default()
            }),
        };
        let number = entry.as_ref().map(|e| e.num.clone()).unwrap_or_default();
        // `\@biblabel` is `\hfill` in author-year mode (line 622) and
        // `\bibnumfmt` = `[#1]` under `numbers` (line 623).
        let marker = if numbers {
            label_bracket(&number)
        } else {
            String::new()
        };
        bibliography.push(item.key, number, entry, marker, item.span, diags);
    }
}

/// `\usepackage[options]{natbib}` anywhere in the token stream, as the
/// options natbib resolves them to. The package list is comma-separated, so
/// `\usepackage{amsmath,natbib}` counts (with no options).
fn natbib_options<T: Borrow<Token>>(tokens: &[T]) -> Option<natbib::Options> {
    let mut i = 0;
    while i < tokens.len() {
        let TokenKind::Command(name) = &tokens[i].borrow().kind else {
            i += 1;
            continue;
        };
        if name != "usepackage" && name != "RequirePackage" {
            i += 1;
            continue;
        }
        let mut cursor = i + 1;
        let mut options = String::new();
        if let Some((text, after)) = optional_bracket_text(tokens, cursor) {
            options = text;
            cursor = after;
        }
        if let Some((packages, after)) = group_text(tokens, cursor) {
            if packages
                .split(',')
                .map(str::trim)
                .any(|package| package == "natbib")
            {
                return Some(natbib::Options::from_option_list(&options));
            }
            i = after;
            continue;
        }
        i = cursor;
    }
    None
}

/// The text inside the next `{...}` group starting at (after skipping
/// spaces/comments from) `i`, and the index just past its closing brace.
/// `None` if `i` is not followed by a brace group — a malformed `\bibitem`
/// or `\begin`/`\end` is left for the real parse's own diagnostics.
fn group_text<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<(String, usize)> {
    while matches!(
        tokens.get(i).map(|t| &t.borrow().kind),
        Some(TokenKind::Space | TokenKind::Comment)
    ) {
        i += 1;
    }
    if !matches!(
        tokens.get(i).map(|t| &t.borrow().kind),
        Some(TokenKind::LBrace)
    ) {
        return None;
    }
    i += 1;
    let mut depth = 1usize;
    let mut text = String::new();
    while i < tokens.len() {
        match &tokens[i].borrow().kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some((text, i + 1));
                }
            }
            TokenKind::Word(word) | TokenKind::Command(word) => text.push_str(word),
            TokenKind::Space | TokenKind::ParBreak => text.push(' '),
            _ => {}
        }
        i += 1;
    }
    None
}

/// The text inside a `[...]` immediately at (after skipping spaces/comments
/// from) `i`, mirroring `parser::P::optional_bracket_argument`'s word-based
/// bracket matching (brackets are ordinary lexer word characters, never
/// their own token kind) against a plain token slice instead of the live
/// parse cursor.
fn optional_bracket_text<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<(String, usize)> {
    while matches!(
        tokens.get(i).map(|t| &t.borrow().kind),
        Some(TokenKind::Space | TokenKind::Comment)
    ) {
        i += 1;
    }
    let TokenKind::Word(first) = &tokens.get(i)?.borrow().kind else {
        return None;
    };
    if !first.starts_with('[') {
        return None;
    }
    let mut raw = first.clone();
    let mut found = raw.contains(']');
    i += 1;
    while !found && i < tokens.len() {
        match &tokens[i].borrow().kind {
            TokenKind::Word(word) => {
                raw.push_str(word);
                found = word.contains(']');
            }
            TokenKind::Space | TokenKind::ParBreak => raw.push(' '),
            TokenKind::Command(word) => {
                raw.push('\\');
                raw.push_str(word);
            }
            _ => {}
        }
        i += 1;
    }
    if !found {
        return None;
    }
    let content = raw
        .strip_prefix('[')
        .unwrap_or(&raw)
        .split_once(']')
        .map_or(raw.as_str(), |(inside, _)| inside)
        .to_string();
    Some((content, i))
}

/// Builds the `Inline`s for one `\cite`/`\cite[note]{key1,key2,...}`:
/// `[<label>, <label>, ..., note]`. An undefined key renders as a bold `?`
/// (only the `?` is bold, matching real LaTeX's `\@citex`/`\bfseries ?`
/// recovery — the brackets and separators stay normal weight) and pushes one
/// warning, matching `\@citex`'s own "Citation ... undefined" warning.
pub fn cite_inlines(
    keys: &[String],
    note: Option<String>,
    bibliography: &Bibliography,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Inline> {
    let mut out = vec![text_run("[", span, TextStyle::default(), true)];
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            out.push(text_run(", ", span, TextStyle::default(), false));
        }
        match bibliography.resolve(key) {
            Some(label) => out.push(text_run(label, span, TextStyle::default(), false)),
            None => {
                out.push(text_run("?", span, TextStyle::BOLD, false));
                diags.push(Diagnostic::warning(
                    format!("citation '{key}' is undefined"),
                    Some(span),
                    Some("rendered '?' in place of the undefined citation".into()),
                ));
            }
        }
    }
    if let Some(note) = note {
        // `~` is TeX's tie: an ordinary interword space that just does not
        // break a line. This layout never breaks inside a `\cite` note, so a
        // plain space renders it faithfully.
        let note = note.replace('~', " ");
        out.push(text_run(
            &format!(", {note}"),
            span,
            TextStyle::default(),
            false,
        ));
    }
    out.push(text_run("]", span, TextStyle::default(), false));
    out
}

fn text_run(text: &str, span: Span, style: TextStyle, space_before: bool) -> Inline {
    Inline::Text {
        text: text.to_string(),
        span,
        style,
        space_before,
    }
}

/// The `[<widest-label>]` bracket text a `\bibitem`'s own marker or
/// `thebibliography`'s `\labelwidth` measures, given the environment's
/// widest-label argument (`\begin{thebibliography}{99}`'s `"99"`).
pub fn label_bracket(text: &str) -> String {
    format!("[{text}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize_document;
    use crate::DocumentId;

    fn scan(source: &str) -> (Bibliography, Vec<Diagnostic>) {
        let tokens = tokenize_document(source, DocumentId::default());
        let mut diags = Vec::new();
        let bibliography = prescan(&tokens, &mut diags);
        (bibliography, diags)
    }

    #[test]
    fn numbers_plain_bibitems_in_order() {
        let (bib, diags) = scan(
            r"\begin{thebibliography}{9}\bibitem{a}First.\bibitem{b}Second.\end{thebibliography}",
        );
        assert!(diags.is_empty());
        assert_eq!(bib.resolve("a"), Some("1"));
        assert_eq!(bib.resolve("b"), Some("2"));
        assert_eq!(bib.label_at(0), Some("1"));
        assert_eq!(bib.label_at(1), Some("2"));
    }

    #[test]
    fn optional_label_overrides_the_number_without_consuming_one() {
        let (bib, _) = scan(
            r"\begin{thebibliography}{9}\bibitem[Knuth 1984]{tex}A.\bibitem{b}B.\end{thebibliography}",
        );
        assert_eq!(bib.resolve("tex"), Some("Knuth 1984"));
        // The un-labelled entry after it is still numbered 1: the labelled
        // entry did not consume a number, matching `\@lbibitem`.
        assert_eq!(bib.resolve("b"), Some("1"));
    }

    #[test]
    fn duplicate_key_warns_and_the_second_wins() {
        let (bib, diags) = scan(
            r"\begin{thebibliography}{9}\bibitem{a}First.\bibitem{a}Second.\end{thebibliography}",
        );
        assert_eq!(bib.resolve("a"), Some("2"));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("duplicate \\bibitem{a}"));
    }

    #[test]
    fn bibitem_outside_thebibliography_is_not_registered() {
        let (bib, _) = scan(r"\bibitem{a}Stray.");
        assert_eq!(bib.resolve("a"), None);
    }
}
