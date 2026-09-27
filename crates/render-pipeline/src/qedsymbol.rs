//! amsthm's `\qedsymbol`, redefined by the document.
//!
//! amsthm.sty 430 defines `\providecommand{\qedsymbol}{\openbox}`, and the end
//! of a `proof` (and `\qedhere`) typesets `\hbox{\qedsymbol}` at the end of
//! the `\qed` list. Student templates very often replace it:
//!
//! ```latex
//! \renewcommand{\qedsymbol}{$\blacksquare$}
//! ```
//!
//! The pinned `vendor/compiler` does not know amsthm defines the command, so
//! it reports `LaTeX Error: Command \qedsymbol undefined.` there (pdflatex
//! reports nothing), and its `\end{proof}` marker always means `\openbox`.
//! This pass reads the redefinitions from the source bytes, the way
//! `abstractenv` and `listings` read theirs:
//!
//! * a math body (`$..$`, `\(..\)` or `\ensuremath{..}`) replaces the open
//!   box of every text-mode end-of-proof mark after it with that formula,
//!   parsed by the compiler's own math parser under the document's packages.
//!   `\hbox{$\blacksquare$}` is the formula with `\mathsurround` 0, which is
//!   exactly an inline formula;
//! * the compiler's false error on the definition is dropped;
//! * what the pass cannot set yet stays an `\openbox` with a limitation that
//!   says so: a text body (`\textsc{qed}`), and the mark a display's
//!   `\qedhere` places in the equation-number position.
//!
//! A definition is in force from its position on, in its own source file; a
//! mark in another file sees the preamble's definitions: the entry file's
//! own, and those of a file read in that preamble (a project `.sty`, or an
//! `\input{macros}` file that sets no material). A definition no mark could
//! see while some mark kept the default is reported (`qedsymbol_scope`), not
//! dropped silently. Groups are not tracked: a redefinition inside an
//! environment is taken to last past its `\end`.

use crate::adapter::{Block, Item, ParaPart};
use flashtex_compiler::math::{MathList, MathPackages};
use flashtex_compiler::{DocumentId, Span};

/// What [`apply`] changed, for the caller's diagnostics.
#[derive(Default)]
pub struct Applied {
    /// The compiler's `Command \qedsymbol undefined` errors: amsthm defines
    /// the command, so pdflatex has none.
    pub superseded: Vec<Span>,
    /// Marks still drawn as the default `\openbox`.
    pub limitations: Vec<(&'static str, Span, String)>,
}

/// One `\renewcommand{\qedsymbol}{..}` / `\def\qedsymbol{..}`.
struct Definition {
    document: usize,
    /// Where the command starts: the definition is in force after it.
    at: usize,
    /// The whole command, for the diagnostics.
    span: Span,
    body: Body,
}

/// What a redefinition's body is.
enum Body {
    /// A formula the pipeline sets.
    Math(MathList),
    /// A text body (`\textsc{qed}`), not set yet.
    Text,
    /// A formula that does not parse under the loaded packages
    /// (`$\blacksquare$` without amssymb: pdflatex stops on it too).
    Unparsed,
}

/// Replaces the `\openbox` of every end-of-proof mark after a math
/// redefinition of `\qedsymbol` with that formula. `packages` are the
/// document's loaded packages (compiler `Parsed::packages`, the class's own
/// additions included) and `class` its `\documentclass`.
pub fn apply(texts: &[&str], entry: usize, class: &str, packages: &[String], package_files: &[usize], blocks: &mut [Block]) -> Applied {
    let mut applied = Applied::default();
    let amsthm = packages.iter().any(|p| p == "amsthm") || matches!(class, "amsart" | "amsbook" | "amsproc");
    if !amsthm {
        return applied;
    }
    let mut math_packages = MathPackages::KERNEL;
    math_packages.load_class(class);
    for p in packages {
        math_packages.load_package(p);
    }
    let definitions: Vec<Definition> = texts.iter().enumerate().flat_map(|(d, t)| definitions_in(t, d, math_packages)).collect();
    if definitions.is_empty() {
        return applied;
    }
    applied.superseded.extend(definitions.iter().map(|d| d.span));
    let preamble_end = texts.get(entry).and_then(|t| t.find("\\begin{document}"));
    // The documents read in the entry's preamble: the project `.sty` files
    // `\usepackage` loaded (`package_files`), and, when that preamble
    // `\input`s or `\include`s anything, every other document that sets no
    // material (`\input{macros}`, `\input{preamble}`: a student template
    // layout). A definition there is in force for the whole body, as one in
    // the entry's own preamble is.
    let preamble_inputs = preamble_end.is_some_and(|e| {
        let head = &texts[entry][..e];
        crate::adapter::find_command(head, "input").is_some() || crate::adapter::find_command(head, "include").is_some()
    });
    let mut body_documents = std::collections::HashSet::new();
    for block in blocks.iter_mut() {
        walk_block(
            block,
            &mut |item, _| {
                if let Some(s) = crate::adapter::item_source_span(item) {
                    body_documents.insert(s.document.0);
                }
            },
            &mut Vec::new(),
        );
    }
    let preamble_document =
        |d: usize| d != entry && (package_files.contains(&d) || (preamble_inputs && !body_documents.contains(&d)));
    // The definition in force at a mark: the last one before it in its own
    // file, else the last preamble one (the entry's, or a preamble file's).
    let in_force = |span: Span| -> Option<usize> {
        let own = definitions.iter().rposition(|d| d.document == span.document.0 && d.at < span.start);
        own.or_else(|| {
            definitions.iter().rposition(|d| {
                (d.document == entry && span.document.0 != entry && preamble_end.is_some_and(|e| d.at < e)) || preamble_document(d.document)
            })
        })
    };
    let used = std::cell::RefCell::new(vec![false; definitions.len()]);
    let unseen_mark = std::cell::Cell::new(false);
    let mut visit = |item: &mut Item, limitations: &mut Vec<(&'static str, Span, String)>| {
        let Item::QedBox { style, span } = item else { return };
        let Some(i) = in_force(*span) else {
            unseen_mark.set(true);
            return;
        };
        used.borrow_mut()[i] = true;
        let span = *span;
        match &definitions[i].body {
            Body::Math(list) => {
                *item = Item::Math { list: list.clone(), span, hidden: false, unpainted: false, size_cpt: style.size_cpt };
            }
            Body::Text => limitations.push((
                "qedsymbol_body",
                span,
                "this \\qedsymbol is a text body, which is not set yet: the proof ends with amsthm's default open box".to_string(),
            )),
            Body::Unparsed => limitations.push((
                "qedsymbol_body",
                span,
                "the \\qedsymbol formula does not parse under the loaded packages (pdflatex stops on it too): the proof ends with amsthm's default open box".to_string(),
            )),
        }
    };
    for block in blocks.iter_mut() {
        walk_block(block, &mut |item, lim| visit(item, lim), &mut applied.limitations);
        if let Block::Paragraph { parts, .. } = block {
            for part in parts.iter() {
                let ParaPart::Display { qed_here: Some(q), .. } = part else { continue };
                match in_force(*q) {
                    Some(i) => {
                        used.borrow_mut()[i] = true;
                        applied.limitations.push((
                            "qedsymbol_display",
                            *q,
                            "a \\qedhere in a display still draws amsthm's default open box, not the redefined \\qedsymbol".to_string(),
                        ));
                    }
                    None => unseen_mark.set(true),
                }
            }
        }
    }
    // A redefinition no mark saw while some mark kept the default: its file
    // is read at a point this pass does not place (an `\input` in the body
    // ahead of other files' proofs, say). Said, not drawn wrong silently.
    if unseen_mark.get() {
        for (d, _) in definitions.iter().zip(used.borrow().iter()).filter(|(_, used)| !**used) {
            applied.limitations.push((
                "qedsymbol_scope",
                d.span,
                "this \\qedsymbol redefinition is in a file whose place in the document is not tracked, so it is not applied: proofs in other files end with amsthm's default open box".to_string(),
            ));
        }
    }
    applied
}

type Limitations = Vec<(&'static str, Span, String)>;

fn walk_block(block: &mut Block, f: &mut dyn FnMut(&mut Item, &mut Limitations), lim: &mut Limitations) {
    match block {
        Block::Paragraph { parts, list, .. } => {
            for part in parts.iter_mut() {
                match part {
                    ParaPart::Lines(items) => walk_items(items, f, lim),
                    ParaPart::Rows { rows, .. } => {
                        for row in rows.iter_mut() {
                            for t in row.intertext.iter_mut() {
                                walk_items(&mut t.items, f, lim);
                            }
                        }
                    }
                    ParaPart::Display { .. } => {}
                }
            }
            if let Some(items) = list.as_mut().and_then(|l| l.label_items.as_mut()) {
                walk_items(items, f, lim);
            }
        }
        _ => {}
    }
}

fn walk_items(items: &mut [Item], f: &mut dyn FnMut(&mut Item, &mut Limitations), lim: &mut Limitations) {
    for item in items.iter_mut() {
        match item {
            Item::Footnote { text: Some(t), .. } | Item::Marginpar { text: t, .. } | Item::Lap { items: t } => walk_items(t, f, lim),
            Item::ColorBox(b) => walk_items(&mut b.items, f, lim),
            Item::HBox(b) => walk_items(&mut b.items, f, lim),
            Item::Table(t) => {
                for entry in t.entries.iter_mut() {
                    if let crate::table::TableEntry::Row { cells, .. } = entry {
                        for cell in cells.iter_mut() {
                            walk_items(&mut cell.items, f, lim);
                        }
                    }
                }
            }
            _ => f(item, lim),
        }
    }
}

/// Every `\renewcommand{\qedsymbol}` / `\renewcommand\qedsymbol` (starred or
/// not) and `\def\qedsymbol` in `text`, with its body read.
fn definitions_in(text: &str, document: usize, packages: MathPackages) -> Vec<Definition> {
    let mut out = Vec::new();
    for command in ["renewcommand", "def"] {
        let mut from = 0;
        while let Some(r) = crate::adapter::find_command(&text[from..], command) {
            let at = from + r;
            from = at + 1;
            let mut rest = at + 1 + command.len();
            if command == "renewcommand" {
                rest = skip_blanks(text, rest);
                if text[rest..].starts_with('*') {
                    rest += 1;
                }
            }
            rest = skip_blanks(text, rest);
            let braced = text[rest..].starts_with('{');
            let name_at = if braced { skip_blanks(text, rest + 1) } else { rest };
            let Some(after_name) = text[name_at..].strip_prefix("\\qedsymbol").map(|r| text.len() - r.len()) else { continue };
            if text[after_name..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                continue;
            }
            let mut body_open = skip_blanks(text, after_name);
            if braced {
                if !text[body_open..].starts_with('}') {
                    continue;
                }
                body_open = skip_blanks(text, body_open + 1);
            }
            if !text[body_open..].starts_with('{') {
                // `[<n>]` arguments or a `\def` parameter text: not a
                // plain symbol, left to the compiler.
                continue;
            }
            let Some(body_close) = matching_brace(text, body_open) else { continue };
            let span = Span::in_document(DocumentId(document), at, body_close + 1);
            let body = match math_body(text, body_open + 1, body_close) {
                Some((s, e)) => formula(text, s, e, document, packages).map_or(Body::Unparsed, Body::Math),
                None => Body::Text,
            };
            out.push(Definition { document, at, span, body });
        }
    }
    out.sort_by_key(|d| d.at);
    out
}

fn skip_blanks(text: &str, mut at: usize) -> usize {
    while text[at..].starts_with([' ', '\t', '\n', '\r']) {
        at += 1;
    }
    at
}

/// The byte of the `}` that closes the `{` at `open`.
fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The formula's byte range inside a body `text[start..end]` that is one
/// whole `$..$`, `\(..\)` or `\ensuremath{..}`.
fn math_body(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let body = &text[start..end];
    let lead = body.len() - body.trim_start().len();
    let trail = body.len() - body.trim_end().len();
    let (s, e) = (start + lead, end - trail);
    let t = &text[s..e];
    if t.len() >= 2 && t.starts_with('$') && t.ends_with('$') && !t[1..t.len() - 1].contains('$') {
        return Some((s + 1, e - 1));
    }
    if t.starts_with("\\(") && t.ends_with("\\)") && t.len() >= 4 {
        return Some((s + 2, e - 2));
    }
    if let Some(r) = t.strip_prefix("\\ensuremath") {
        let open = e - r.trim_start().len();
        if text[open..].starts_with('{') && matching_brace(text, open) == Some(e - 1) {
            return Some((open + 1, e - 1));
        }
    }
    None
}

/// `text[start..end]` parsed as an inline formula, with its spans in
/// `document`'s bytes. `None` when the parser reports anything (an
/// `\blacksquare` without `amssymb` is an undefined command in pdflatex too).
fn formula(text: &str, start: usize, end: usize, document: usize, packages: MathPackages) -> Option<MathList> {
    let tokens = flashtex_compiler::lexer::tokenize_document(&text[start..end], DocumentId(document));
    let mut diagnostics = Vec::new();
    let list = flashtex_compiler::math::parse_tokens(&tokens, packages, &mut diagnostics);
    if !diagnostics.is_empty() || list.atoms.is_empty() {
        return None;
    }
    Some(flashtex_compiler::math::shift_list(&list, start as isize))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amssymb() -> MathPackages {
        let mut packages = MathPackages::KERNEL;
        packages.load_package("amssymb");
        packages
    }

    fn read(text: &str) -> Vec<(usize, bool)> {
        definitions_in(text, 0, amssymb()).iter().map(|d| (d.at, matches!(d.body, Body::Math(_)))).collect()
    }

    #[test]
    fn every_spelling_of_the_redefinition_is_read() {
        assert_eq!(read(r"\renewcommand{\qedsymbol}{$\blacksquare$}"), [(0, true)]);
        assert_eq!(read(r"\renewcommand*{ \qedsymbol }{\ensuremath{\blacksquare}}"), [(0, true)]);
        assert_eq!(read(r"\renewcommand\qedsymbol{\(\square\)}"), [(0, true)]);
        assert_eq!(read(r"x \def\qedsymbol{$\Box$}"), [(2, true)]);
        assert_eq!(read(r"\renewcommand{\qedsymbol}{\textsc{qed}}"), [(0, false)]);
    }

    #[test]
    fn other_commands_comments_and_arguments_are_not() {
        assert!(read(r"\renewcommand{\qedsymbolx}{$\blacksquare$}").is_empty());
        assert!(read(r"% \renewcommand{\qedsymbol}{$\blacksquare$}").is_empty());
        assert!(read(r"\renewcommand{\qedsymbol}[1]{$#1$}").is_empty());
        assert!(read(r"\renewcommand{\qed}{$\blacksquare$}").is_empty());
    }

    /// `\blacksquare` is amssymb's: without it pdflatex has an undefined
    /// control sequence too, so the open box stays.
    #[test]
    fn a_formula_that_does_not_parse_cleanly_is_not_set() {
        let defs = definitions_in(r"\renewcommand{\qedsymbol}{$\blacksquare$}", 0, MathPackages::KERNEL);
        assert_eq!(defs.len(), 1);
        assert!(matches!(defs[0].body, Body::Unparsed));
    }

    #[test]
    fn the_formula_keeps_its_own_source_bytes() {
        let text = r"\renewcommand{\qedsymbol}{$\blacksquare$}";
        let defs = definitions_in(text, 0, amssymb());
        let Body::Math(list) = &defs[0].body else { panic!("a formula") };
        let span = list.atoms[0].span;
        assert_eq!(&text[span.start..span.end], r"\blacksquare");
    }
}
