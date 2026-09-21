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
    /// `\usepackage[...]{cite}`'s resolved options (cite.sty, Arseneau),
    /// when the document loads it.
    cite: Option<CiteOptions>,
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

    /// cite.sty's resolved options, once `\usepackage{cite}` has been seen
    /// and natbib has not (natbib's `\cite` is the one that survives when
    /// both are loaded, whichever order). `None` when the document does not
    /// load it, and `\cite` keeps its kernel meaning.
    pub fn cite(&self) -> Option<CiteOptions> {
        self.cite.filter(|_| self.natbib.is_none())
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
    bibliography.cite = package_options(tokens, "cite").map(|options| CiteOptions::from_option_list(&options));
    let mut raw: Vec<RawItem> = Vec::new();
    let mut in_bibliography = false;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i].borrow().kind {
            TokenKind::Command(name) if name == "begin" || name == "end" => {
                let is_begin = name == "begin";
                match group_text(tokens, i + 1) {
                    Some((environment, after)) => {
                        if crate::parser::is_bibliography_environment(environment.trim()) {
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
                if let Some((text, after)) = optional_bracket_source(tokens, cursor) {
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
    package_options(tokens, "natbib").map(|options| natbib::Options::from_option_list(&options))
}

/// The `[options]` of the `\usepackage`/`\RequirePackage` that loads
/// `name` anywhere in the token stream (the package list is comma-separated,
/// so `\usepackage{amsmath,cite}` counts, with no options), or `None` when
/// nothing loads it.
fn package_options<T: Borrow<Token>>(tokens: &[T], name: &str) -> Option<String> {
    let mut i = 0;
    while i < tokens.len() {
        let TokenKind::Command(command) = &tokens[i].borrow().kind else {
            i += 1;
            continue;
        };
        if command != "usepackage" && command != "RequirePackage" {
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
            if packages.split(',').map(str::trim).any(|package| package == name) {
                return Some(options);
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
///
/// Shared with `expansion`'s biblatex-package scan, which reads the same
/// raw lexer tokens.
pub(crate) fn group_text<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<(String, usize)> {
    while matches!(
        tokens.get(i).map(|t| &t.borrow().kind),
        Some(TokenKind::Space | TokenKind::Comment)
    ) {
        i += 1;
    }
    if !matches!(tokens.get(i).map(|t| &t.borrow().kind), Some(TokenKind::LBrace)) {
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
///
/// Shared with `expansion`'s biblatex-package scan, which reads the same
/// raw lexer tokens.
pub(crate) fn optional_bracket_text<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<(String, usize)> {
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

/// A `\bibitem`'s `[label]` as TeX source text, and the index just past it
/// (#956).
///
/// The label is not plain text: natbib's `.bbl` labels read
/// `{\citenamefont{Jones} \emph{et~al.}(1990)\citenamefont{Jones, Baker,
/// and Williams}}`, and both natbib's `\@lbibitem` and the kernel's typeset
/// what they cut out of it. [`optional_bracket_text`] flattens its tokens
/// (braces dropped, commands kept as `\name`), which glued `\emph` to its
/// argument and printed `\emphet~al.` literally, 1253 times in one revtex
/// review. This keeps the source instead: braces, control words (with the
/// space that ends one before a letter), control symbols and ties, so the
/// caller can split it the way natbib does (at brace depth 0, see
/// `natbib::parse_label`) and the parser can set each piece
/// (`P::set_citation_source`).
///
/// The bracket is TeX's `[#1]` delimited argument: only a `]` at brace depth
/// 0 ends it, and one brace pair around the whole argument is stripped
/// (tex.web §392), which is why natbib labels are braced at all.
pub(crate) fn optional_bracket_source<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<(String, usize)> {
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
    let mut out = String::new();
    let mut depth = 0usize;
    let start = i;
    while i < tokens.len() {
        let token = tokens[i].borrow();
        match &token.kind {
            TokenKind::Word(word) => {
                let body = if i == start { &word[1..] } else { word.as_str() };
                if token.control_symbol {
                    out.push('\\');
                    out.push_str(body);
                } else {
                    // A letter right after a control word would extend its
                    // name when the source is read again.
                    if body.starts_with(|c: char| c.is_alphabetic()) && ends_in_control_word(&out) {
                        out.push(' ');
                    }
                    if depth == 0 {
                        if let Some(close) = body.find(']') {
                            out.push_str(&body[..close]);
                            return Some((strip_outer_group(&out).to_string(), i + 1));
                        }
                    }
                    out.push_str(body);
                }
            }
            TokenKind::Command(name) => {
                out.push('\\');
                out.push_str(name);
            }
            TokenKind::LBrace => {
                depth += 1;
                out.push('{');
            }
            TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
                out.push('}');
            }
            TokenKind::Space | TokenKind::ParBreak => out.push(' '),
            TokenKind::MathShift => out.push('$'),
            TokenKind::InlineMathOpen => out.push_str("\\("),
            TokenKind::InlineMathClose => out.push_str("\\)"),
            TokenKind::Superscript => out.push('^'),
            TokenKind::Subscript => out.push('_'),
            TokenKind::LineBreak => out.push_str("\\\\"),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Whether `source` ends in a control word (`\emph`), which a following
/// letter would run into.
fn ends_in_control_word(source: &str) -> bool {
    let letters = source.len() - source.trim_end_matches(|c: char| c.is_ascii_alphabetic()).len();
    letters > 0 && source[..source.len() - letters].ends_with('\\') && {
        // `\\emph` is a line break and then letters.
        let before = &source[..source.len() - letters - 1];
        (before.len() - before.trim_end_matches('\\').len()) % 2 == 0
    }
}

/// `{...}` spanning the whole of `source` loses that one pair, as a
/// delimited macro argument does (tex.web §392); anything else is kept.
fn strip_outer_group(source: &str) -> &str {
    let Some(inner) = source.strip_prefix('{').and_then(|s| s.strip_suffix('}')) else {
        return source;
    };
    let mut depth = 0usize;
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '{' => depth += 1,
            '}' => match depth.checked_sub(1) {
                Some(d) => depth = d,
                // The first `{` closed before the end: `{a}b{c}`.
                None => return source,
            },
            _ => {}
        }
    }
    if depth == 0 {
        inner
    } else {
        source
    }
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
    if let Some(options) = bibliography.cite() {
        cite_sty_labels(keys, bibliography, options, span, diags, &mut out);
    } else {
        for (index, key) in keys.iter().enumerate() {
            if index > 0 {
                kernel_citea(span, &mut out);
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
    }
    if let Some(note) = note {
        // `~` is TeX's tie: an ordinary interword space that just does not
        // break a line. This layout never breaks inside a `\cite` note, so a
        // plain space renders it faithfully.
        let note = natbib::note_source(&note);
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

/// The kernel's `\@citea` between two labels, `,\penalty\@m\ ` (latex.ltx
/// `\@citex`): a comma, a penalty of 1000 -- the only break point, since
/// glue after a penalty is none (tex.web §866) -- and a control space,
/// interword glue at space factor 1000 whatever the comma set.
fn kernel_citea(span: Span, out: &mut Vec<Inline>) {
    out.push(text_run(",", span, TextStyle::default(), false));
    out.push(Inline::Penalty { value: 1000, span, unskip: false });
    out.push(text_run(" ", span, TextStyle::default(), false));
}

/// `\usepackage[<options>]{cite}` (cite.sty v5.5, Arseneau) as its options
/// resolve. The package sorts a numeric key list, compresses three or more
/// consecutive numbers into a range, and sets its own glue after the comma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CiteOptions {
    /// `\citepunct`'s glue after the comma: the default `\hskip.13em
    /// plus.1em minus.1em` (cite.sty lines 46-47), `space`'s `\ ` (line
    /// 403) or `nospace`'s nothing (line 402).
    pub punct: CitePunct,
    /// Not `nosort`: numeric entries in ascending order.
    pub sort: bool,
    /// Not `nocompress`: `1,2,3` is `1--3`.
    pub compress: bool,
}

/// The cite.sty package options that leave the output as this module sets
/// it (the parser's `package_matches_layout`).
pub const CITE_IMPLEMENTED_OPTIONS: &[&str] = &["space", "nospace", "nosort", "nocompress", "sort", "compress", "adjust", "move", "verbose"];

/// See [`CiteOptions::punct`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitePunct {
    /// `\hskip.13em plus.1em minus.1em`.
    Thin,
    /// `\ `, an interword space at space factor 1000.
    Space,
    /// No glue: `[1,2]` is one word.
    None,
}

impl CiteOptions {
    /// cite.sty's `\DeclareOption`s as `\ProcessOptions` runs them, in
    /// declaration order (`nospace` line 402 before `space` line 403, so
    /// `[space,nospace]` is `space`). `superscript`/`super`, `noadjust`,
    /// `nomove`, `nobreak`, `biblabel`, `ref`, `verbose`, `adjust`, `sort`,
    /// `compress`, `move` and `break` change nothing here.
    pub fn from_option_list(options: &str) -> Self {
        let given: Vec<&str> = options.split(',').map(str::trim).collect();
        let punct = if given.contains(&"space") {
            CitePunct::Space
        } else if given.contains(&"nospace") {
            CitePunct::None
        } else {
            CitePunct::Thin
        };
        CiteOptions { punct, sort: !given.contains(&"nosort"), compress: !given.contains(&"nocompress") }
    }
}

/// cite.sty's `\citepunct` (lines 46-47): `,\penalty\citepunctpenalty
/// \hskip.13emplus.1emminus.1em` -- a comma, a penalty of 1000 (`\@m`),
/// then glue thinner than an interword space. pdflatex's `\showbox` of
/// `\hbox{block~\cite{a,b}. The}` under T1 cmr10: `[` `1` `,` `\penalty
/// 1000` `\glue 1.29973 plus 0.9998 minus 0.9998` `2` `]` `.` then the
/// sentence space `\glue 4.44336 plus 4.99878 minus 0.37027`. `[space]`
/// makes the glue `\ ` and `[nospace]` drops it; both keep the penalty.
fn cite_punct(options: CiteOptions, span: Span, out: &mut Vec<Inline>) {
    match options.punct {
        CitePunct::Thin => {
            out.push(text_run(",", span, TextStyle::default(), false));
            out.push(Inline::Penalty { value: 1000, span, unskip: false });
            out.push(Inline::TextGlue { em: 0.13, span, plus_em: 0.1, minus_em: 0.1 });
        }
        // `\def\citepunct{,\penalty\citepunctpenalty\ }` (line 403): the
        // kernel's own shape.
        CitePunct::Space => kernel_citea(span, out),
        CitePunct::None => {
            out.push(text_run(",", span, TextStyle::default(), false));
            out.push(Inline::Penalty { value: 1000, span, unskip: false });
        }
    }
}

/// cite.sty's citation body (`\@cite@n`, `\@compress@cite`, lines
/// 180-297): every key whose label is not a plain number is set at once,
/// in order (`\@cite@dump@now`; the bold `?` of an undefined key too, line
/// 109); the numeric ones are collected, sorted ascending (unless `nosort`)
/// and, unless `nocompress`, a run of three or more consecutive numbers
/// becomes `first\citedash last` -- `\citedash` is `\hbox{--}\penalty
/// \citepunctpenalty` (line 51), an en dash then a penalty of 1000
/// (pdflatex: `\cite{a,b,c}` is `[` `1` `\hbox(4.3045+0.0)x4.99878` (the
/// `--` ligature) `\penalty 1000` `3` `]`); two consecutive numbers stay
/// `1,2`. Entries are separated by [`cite_punct`].
///
/// A `[prefix]number[suffix]` label (`A12`, `12a`) is sortable in cite.sty
/// too; here it is set in place like any other non-numeric label, which is
/// the package's own behaviour for a label its number scan rejects.
fn cite_sty_labels(
    keys: &[String],
    bibliography: &Bibliography,
    options: CiteOptions,
    span: Span,
    diags: &mut Vec<Diagnostic>,
    out: &mut Vec<Inline>,
) {
    let mut first = true;
    let mut separate = |out: &mut Vec<Inline>| {
        if !first {
            cite_punct(options, span, out);
        }
        first = false;
    };
    let mut numbers: Vec<(u64, &str)> = Vec::new();
    for key in keys {
        match bibliography.resolve(key) {
            Some(label) => match label.parse::<u64>() {
                Ok(number) if label.bytes().all(|b| b.is_ascii_digit()) => numbers.push((number, label)),
                _ => {
                    separate(out);
                    out.push(text_run(label, span, TextStyle::default(), false));
                }
            },
            None => {
                separate(out);
                out.push(text_run("?", span, TextStyle::BOLD, false));
                diags.push(Diagnostic::warning(
                    format!("citation '{key}' is undefined"),
                    Some(span),
                    Some("rendered '?' in place of the undefined citation".into()),
                ));
            }
        }
    }
    if options.sort {
        numbers.sort_by_key(|(number, _)| *number);
    }
    let mut i = 0;
    while i < numbers.len() {
        let mut j = i;
        while options.compress && j + 1 < numbers.len() && numbers[j + 1].0 == numbers[j].0 + 1 {
            j += 1;
        }
        separate(out);
        out.push(text_run(numbers[i].1, span, TextStyle::default(), false));
        if j >= i + 2 {
            out.push(text_run("\u{2013}", span, TextStyle::default(), false));
            out.push(Inline::Penalty { value: 1000, span, unskip: false });
            out.push(text_run(numbers[j].1, span, TextStyle::default(), false));
            i = j + 1;
        } else {
            i += 1;
        }
    }
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

    /// #956: the label keeps its TeX source -- braces, control words and
    /// the blank that ends one before a letter, control symbols, ties --
    /// with the one brace pair around the whole argument stripped, and only
    /// a `]` at depth 0 closing it.
    #[test]
    fn a_label_is_kept_as_tex_source() {
        let label = |source: &str| {
            let tokens = tokenize_document(source, DocumentId::default());
            optional_bracket_source(&tokens, 0).map(|(text, _)| text)
        };
        assert_eq!(
            label(r"[{\citenamefont{Jones} \emph{et~al.}(1990)\citenamefont{Jones and Baker}}]{k}").as_deref(),
            Some(r"\citenamefont{Jones} \emph{et~al.}(1990)\citenamefont{Jones and Baker}")
        );
        assert_eq!(label(r#"[Gr{\"o}{\ss}er(1999)]"#).as_deref(), Some(r#"Gr{\"o}{\ss}er(1999)"#));
        assert_eq!(label(r"[\ss er]").as_deref(), Some(r"\ss er"));
        assert_eq!(label(r"[{a]b}(1990)]").as_deref(), Some(r"{a]b}(1990)"));
        assert_eq!(label(r"[{a}b{c}]").as_deref(), Some(r"{a}b{c}"));
        assert_eq!(label(r"[Knuth 1984]").as_deref(), Some("Knuth 1984"));
        assert_eq!(label(r"{k}"), None);
    }

    #[test]
    fn a_marked_up_kernel_label_resolves_to_its_source() {
        let (bib, _) = scan(r"\begin{thebibliography}{9}\bibitem[\emph{Knuth} 1984]{tex}A.\end{thebibliography}");
        assert_eq!(bib.resolve("tex"), Some(r"\emph{Knuth} 1984"));
    }

    /// The inlines of one `\cite`, spelled: text as itself, a penalty as
    /// `<n>`, `TextGlue` as `<em plus minus>`.
    fn spell(bib: &Bibliography, keys: &[&str]) -> String {
        let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        let mut diags = Vec::new();
        cite_inlines(&keys, None, bib, Span::new(0, 0), &mut diags)
            .iter()
            .map(|inline| match inline {
                Inline::Text { text, .. } => text.clone(),
                Inline::Penalty { value, .. } => format!("<{value}>"),
                Inline::TextGlue { em, plus_em, minus_em, .. } => format!("<{em} plus {plus_em} minus {minus_em}>"),
                other => panic!("{other:?}"),
            })
            .collect()
    }

    const BIB: &str = r"\begin{thebibliography}{9}\bibitem{a}A.\bibitem{b}B.\bibitem{c}C.\bibitem{d}D.\bibitem[Knu]{k}K.\end{thebibliography}";

    /// cite.sty: `\citepunct` is `,\penalty\@m\hskip.13em plus.1em
    /// minus.1em` (lines 46-47); three consecutive numbers are `1\citedash 3`
    /// (`\hbox{--}\penalty 1000`, line 51); keys are sorted; a label that
    /// is not a number is set at once, before the numbers; two consecutive
    /// numbers stay apart. pdflatex `\showbox`: `[1,\penalty 1000 \glue
    /// 1.29973 plus 0.9998 minus 0.9998 2]` and `[1 \hbox(--) \penalty 1000
    /// 3]` under T1 cmr10.
    #[test]
    fn cite_package_sorts_compresses_and_separates_with_thin_glue() {
        let (bib, _) = scan(&format!(r"\usepackage{{cite}}{BIB}"));
        assert_eq!(spell(&bib, &["a", "b"]), "[1,<1000><0.13 plus 0.1 minus 0.1>2]");
        assert_eq!(spell(&bib, &["c", "b", "a"]), "[1\u{2013}<1000>3]");
        assert_eq!(spell(&bib, &["d", "b", "a"]), "[1,<1000><0.13 plus 0.1 minus 0.1>2,<1000><0.13 plus 0.1 minus 0.1>4]");
        assert_eq!(spell(&bib, &["c", "k", "a"]), "[Knu,<1000><0.13 plus 0.1 minus 0.1>1,<1000><0.13 plus 0.1 minus 0.1>3]");
        assert_eq!(spell(&bib, &["a", "b", "c", "d"]), "[1\u{2013}<1000>4]");
    }

    /// `[nosort]` keeps the given order (and still compresses a consecutive
    /// run in that order, cite.sty lines 517-523); `[nocompress]` keeps every
    /// number; `[space]` is the kernel's `,\penalty\@m\ ` and `[nospace]`
    /// nothing after the comma's penalty. Without the package, `\cite` keeps
    /// its kernel `,\penalty\@m\ ` (pdflatex `\showbox` of
    /// `\hbox{\cite{a,b}}`: `[` `\hbox{1}` `,` `\penalty 1000` `\glue
    /// 3.33333 plus 1.66666 minus 1.11111` `\hbox{2}` `]`, cmr10).
    #[test]
    fn cite_package_options_and_the_kernel_default() {
        let (bib, _) = scan(&format!(r"\usepackage[nosort]{{cite}}{BIB}"));
        assert_eq!(spell(&bib, &["c", "a", "b"]), "[3,<1000><0.13 plus 0.1 minus 0.1>1,<1000><0.13 plus 0.1 minus 0.1>2]");
        assert_eq!(spell(&bib, &["b", "c", "d"]), "[2\u{2013}<1000>4]");
        let (bib, _) = scan(&format!(r"\usepackage[nocompress,space]{{cite}}{BIB}"));
        assert_eq!(spell(&bib, &["c", "a", "b"]), "[1,<1000> 2,<1000> 3]");
        let (bib, _) = scan(&format!(r"\usepackage[nospace]{{cite}}{BIB}"));
        assert_eq!(spell(&bib, &["a", "b"]), "[1,<1000>2]");
        let (bib, _) = scan(BIB);
        assert!(bib.cite().is_none());
        assert_eq!(spell(&bib, &["c", "a", "b"]), "[3,<1000> 1,<1000> 2]");
    }
}
