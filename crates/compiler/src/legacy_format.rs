//! Recognition — not support — for documents that are not LaTeX2e at all
//! (issue #907).
//!
//! Ten of the hundred famous arXiv papers are plain TeX (`harvmac`, `epsf`,
//! `\magnification`) or LaTeX 2.09 (`\documentstyle`). The LaTeX2e parser
//! turns those into a flood of unknown-command errors that never name the
//! real cause. This module positively identifies the format from a
//! comment-aware token scan of the project sources, so the parser can emit
//! one early diagnostic naming it and suppress the symptom flood.
//!
//! The scan runs on the raw sources with [`crate::lexer::tokenize_document`]
//! — before expansion, because expansion consumes `\input` — and only sees
//! real command tokens: `%` comments never reach it, `\verb` bodies lex as
//! [`crate::lexer::TokenKind::Verb`] rather than commands, and prose words
//! like "magnification" are [`crate::lexer::TokenKind::Word`], never
//! [`crate::lexer::TokenKind::Command`]. A LaTeX2e manual that merely
//! *discusses* `\documentstyle` therefore stays silent.

use crate::lexer::{tokenize_document, Token, TokenKind};
use crate::parser::SourceDocument;
use crate::{DocumentId, Span};

/// A positively identified non-LaTeX2e format, with the source span of the
/// marker that identified it (used as the diagnostic's primary label).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyFormat {
    /// `\documentstyle` present and no `\documentclass` anywhere: LaTeX 2.09.
    Latex209 { marker: Span },
    /// Plain-TeX markers (`\magnification`, `\input harvmac`, `\input epsf`)
    /// or, failing those, the plain-TeX job terminator `\bye` in a document
    /// with no LaTeX structure at all.
    PlainTex { marker: Span },
}

/// Plain-TeX format files recognised after `\input`, with or without the
/// `.tex` suffix (`\input epsf.tex` is the form kitaev's and Maldacena's
/// papers use).
fn is_plain_tex_input(target: &str) -> bool {
    let stem = target.strip_suffix(".tex").unwrap_or(target);
    stem == "harvmac" || stem == "epsf" || stem.ends_with("/harvmac") || stem.ends_with("/epsf")
}

/// The braced or bare filename after an `\input` command token at `at`.
fn input_target(tokens: &[Token], at: usize) -> Option<Span> {
    let mut j = at + 1;
    loop {
        match tokens.get(j)?.kind {
            TokenKind::Space | TokenKind::ParBreak | TokenKind::Comment => j += 1,
            TokenKind::LBrace => {
                let mut target = String::new();
                let mut k = j + 1;
                loop {
                    match tokens.get(k)?.kind {
                        TokenKind::RBrace => break,
                        TokenKind::Word(ref word) => {
                            target.push_str(word);
                            k += 1;
                        }
                        // Anything else inside the braces (a nested command,
                        // math shift, …) is not a plain static filename.
                        _ => return None,
                    }
                }
                return is_plain_tex_input(&target).then(|| tokens[at].span);
            }
            TokenKind::Word(ref word) => {
                return is_plain_tex_input(word).then(|| tokens[at].span);
            }
            _ => return None,
        }
    }
}

struct Markers {
    documentclass: bool,
    documentstyle: Option<Span>,
    plain_marker: Option<Span>,
    bye: Option<Span>,
    begin_document: bool,
}

impl Markers {
    fn scan(text: &str, document: DocumentId) -> Self {
        let tokens = tokenize_document(text, document);
        let mut found = Markers {
            documentclass: false,
            documentstyle: None,
            plain_marker: None,
            bye: None,
            begin_document: false,
        };
        let mut i = 0;
        while i < tokens.len() {
            if let TokenKind::Command(ref name) = tokens[i].kind {
                match name.as_str() {
                    "documentclass" => found.documentclass = true,
                    "documentstyle" => {
                        if found.documentstyle.is_none() {
                            found.documentstyle = Some(tokens[i].span);
                        }
                    }
                    "magnification" => {
                        if found.plain_marker.is_none() {
                            found.plain_marker = Some(tokens[i].span);
                        }
                    }
                    "bye" => {
                        if found.bye.is_none() {
                            found.bye = Some(tokens[i].span);
                        }
                    }
                    "begin" => {
                        // `\begin{document}` marks LaTeX body content. The
                        // lexer emits `\begin` as a command and `{document}`
                        // as brace/word tokens; peeking two words ahead is
                        // enough — anything fancier is not a document open.
                        let mut j = i + 1;
                        while matches!(
                            tokens.get(j).map(|t| &t.kind),
                            Some(TokenKind::Space | TokenKind::ParBreak | TokenKind::Comment)
                        ) {
                            j += 1;
                        }
                        let is_document =
                            matches!(tokens.get(j).map(|t| &t.kind), Some(TokenKind::LBrace))
                                && matches!(
                                    tokens.get(j + 1).map(|t| &t.kind),
                                    Some(TokenKind::Word(word)) if word == "document"
                                );
                        if is_document {
                            found.begin_document = true;
                        }
                    }
                    "input" => {
                        if found.plain_marker.is_none() {
                            if let Some(span) = input_target(&tokens, i) {
                                found.plain_marker = Some(span);
                            }
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        found
    }
}

/// Identify the project format from its raw sources. `entry` is the index of
/// the entry document, whose markers win the diagnostic span when several
/// documents carry one.
///
/// Returns `None` — the common case — when any document uses
/// `\documentclass`: a LaTeX2e project is never legacy, even when one of its
/// files mentions `\documentstyle`, `magnification` or `\input epsf` in
/// prose, comments or a genuine `\input`.
pub(crate) fn detect_legacy_format(
    documents: &[SourceDocument<'_>],
    entry: usize,
) -> Option<LegacyFormat> {
    if documents.is_empty() {
        return None;
    }
    // The entry document first, so its markers win ties for the span.
    let mut order: Vec<usize> = Vec::with_capacity(documents.len());
    if entry < documents.len() {
        order.push(entry);
    }
    order.extend((0..documents.len()).filter(|index| *index != entry));
    let mut documentclass = false;
    let mut documentstyle = None;
    let mut plain_marker = None;
    let mut bye = None;
    let mut begin_document = false;
    for index in order {
        let scan = Markers::scan(documents[index].text, DocumentId(index));
        documentclass |= scan.documentclass;
        documentstyle = documentstyle.or(scan.documentstyle);
        plain_marker = plain_marker.or(scan.plain_marker);
        bye = bye.or(scan.bye);
        begin_document |= scan.begin_document;
    }
    // `\documentclass` present anywhere: LaTeX2e, never fire either
    // diagnostic — even when another file mentions legacy markers.
    if documentclass {
        return None;
    }
    if let Some(marker) = documentstyle {
        return Some(LegacyFormat::Latex209 { marker });
    }
    if let Some(marker) = plain_marker {
        return Some(LegacyFormat::PlainTex { marker });
    }
    // No LaTeX class command and none of the named markers: only the
    // plain-TeX job terminator `\bye` still positively identifies plain TeX.
    // Bare absence of `\documentclass` alone never fires — most parser unit
    // tests and every editor fragment lack one without being plain TeX —
    // and a `\begin{document}` alongside `\bye` is LaTeX body content, not
    // a plain-TeX job.
    match (bye, begin_document) {
        (Some(marker), false) => Some(LegacyFormat::PlainTex { marker }),
        _ => None,
    }
}

/// Whether `diagnostic` is a downstream symptom of a positively identified
/// legacy format rather than signal: the unknown-command flood, the LaTeX2e
/// `\input{...}` shape errors that bare plain-TeX `\input harvmac` trips,
/// and the unfindable plain-TeX format files themselves. Only call this once
/// [`detect_legacy_format`] has fired — never on a normal document.
pub(crate) fn is_legacy_symptom(
    message: &str,
    code: Option<crate::diagnostics::DiagnosticCode>,
) -> bool {
    if code == Some(crate::diagnostics::DiagnosticCode::UnknownCommand) {
        return true;
    }
    if message.starts_with("\\input requires a ") {
        return true;
    }
    if message.starts_with("included file not found")
        && (message.contains("harvmac") || message.contains("epsf"))
    {
        return true;
    }
    false
}
