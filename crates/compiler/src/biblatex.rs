//! Basic `biblatex` support backed by the shared `.bib` data layer.
//!
//! The compiler receives project files as input, so `\addbibresource` resolves
//! a project-relative `.bib` document instead of reading the host filesystem.
//! The complete expanded token stream is scanned before parsing; this gives
//! citations the same forward-reference behavior as a pdflatex/biber run.

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

use flashtex_bibliography::diagnostics::{Diagnostic as BibDiagnostic, Severity as BibSeverity};
use flashtex_bibliography::format::{format_bibliography, format_names, RunStyle};
use flashtex_bibliography::latex;
use flashtex_bibliography::model::Database;
use flashtex_bibliography::resolve::{resolve, BibItem, Citation, Resolution, Style};

use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::parser::{Block, Inline, ItemLabel, ListLeftMargin, SourceDocument, TextStyle};
use crate::{DocumentId, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CitationStyle {
    #[default]
    Numeric,
    Authoryear,
    Alphabetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Sorting {
    None,
    #[default]
    Nyt,
}

#[derive(Debug, Clone)]
struct CitationUse {
    key: String,
    span: Span,
}

#[derive(Debug, Clone)]
struct ResourceUse {
    requested: String,
    span: Span,
}

#[derive(Debug, Clone)]
struct Group {
    text: String,
    span: Span,
    after: usize,
}

/// The pre-resolved biblatex database for one compiler parse.
#[derive(Debug, Default)]
pub struct Bibliography {
    loaded: bool,
    style: CitationStyle,
    sorting: Sorting,
    database: Database,
    resolution: Resolution,
    entry_documents: Vec<usize>,
    labels: HashMap<String, String>,
}

/// Scan package options, project bibliography resources, and every citation
/// before the parser walks the document.
pub fn prescan<T: Borrow<Token>>(
    tokens: &[T],
    documents: &[SourceDocument<'_>],
    diags: &mut Vec<Diagnostic>,
) -> Bibliography {
    let Some((package_options, package_span)) = biblatex_package(tokens) else {
        return Bibliography::default();
    };

    let mut bibliography = Bibliography {
        loaded: true,
        ..Bibliography::default()
    };
    parse_package_options(
        &package_options,
        package_span,
        &mut bibliography.style,
        &mut bibliography.sorting,
        diags,
    );

    let (resources, citations) = scan_commands(tokens);
    load_resources(
        &mut bibliography.database,
        &mut bibliography.entry_documents,
        &resources,
        documents,
        diags,
    );

    let bib_citations: Vec<Citation> = citations
        .iter()
        .map(|citation| Citation::new(citation.key.clone(), bib_span(citation.span)))
        .collect();
    let resolution_style = match bibliography.sorting {
        Sorting::None => Style::Unsrt,
        Sorting::Nyt => Style::Plain,
    };
    bibliography.resolution = resolve(&bib_citations, &bibliography.database, resolution_style);
    for item in &bibliography.resolution.items {
        bibliography
            .labels
            .insert(item.key.to_ascii_lowercase(), item.label.clone());
    }

    let mut reported = HashSet::new();
    for citation in citations {
        if citation.key == "*"
            || bibliography.database.get(&citation.key).is_some()
            || !reported.insert(citation.key.to_ascii_lowercase())
        {
            continue;
        }
        diags.push(Diagnostic::warning(
            format!("Citation `{}` undefined", citation.key),
            Some(citation.span),
            Some("rendered the undefined key in bold and left it out of the bibliography".into()),
        ));
    }

    bibliography
}

impl Bibliography {
    pub fn enabled(&self) -> bool {
        self.loaded
    }

    /// Consumes `\addbibresource` after the parser has read its arguments.
    pub fn add_resource(&self, span: Span, diags: &mut Vec<Diagnostic>) {
        if !self.loaded {
            diags.push(Diagnostic::error(
                "\\addbibresource requires \\usepackage{biblatex}",
                Some(span),
                Some("loaded no bibliography resource".into()),
            ));
        }
    }

    /// Render one biblatex citation family command.
    pub fn cite_inlines(
        &self,
        name: &str,
        keys: &[String],
        pre: Option<&str>,
        post: Option<&str>,
        span: Span,
        space_before: bool,
        diags: &mut Vec<Diagnostic>,
    ) -> Vec<Inline> {
        if !self.loaded {
            diags.push(Diagnostic::error(
                format!("\\{name} requires \\usepackage{{biblatex}}"),
                Some(span),
                Some("used an empty citation and continued".into()),
            ));
            return Vec::new();
        }
        if keys.is_empty() {
            return Vec::new();
        }
        if self.style == CitationStyle::Authoryear {
            self.authoryear_citation(name, keys, pre, post, span, space_before)
        } else {
            self.numeric_citation(name, keys, pre, post, span, space_before)
        }
    }

    /// Render `\printbibliography[title=...]` as a heading followed by entry
    /// blocks. Numeric output uses the compiler's existing hanging-list item;
    /// the minimal author-year mode emits ordinary paragraphs without labels.
    pub fn print_bibliography(
        &self,
        options: Option<&str>,
        report_or_book: bool,
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Vec<Block> {
        if !self.loaded {
            diags.push(Diagnostic::error(
                "\\printbibliography requires \\usepackage{biblatex}",
                Some(span),
                Some("printed no bibliography and continued".into()),
            ));
            return Vec::new();
        }

        let default_title = if report_or_book {
            "Bibliography"
        } else {
            "References"
        };
        let title =
            print_title(options.unwrap_or_default()).unwrap_or_else(|| default_title.to_string());
        let heading = Block::Heading {
            level: 1,
            number: String::new(),
            number_span: span,
            content: vec![text_inline(&title, span, TextStyle::BOLD, false)],
        };
        let formatted = format_bibliography(&self.database, &self.resolution);
        let widest = self
            .resolution
            .items
            .iter()
            .map(|item| item.label.as_str())
            .max_by_key(|label| label.len())
            .unwrap_or("0")
            .to_string();
        let mut blocks = vec![heading];
        for (item, entry) in self.resolution.items.iter().zip(formatted) {
            for warning in &entry.warnings {
                diags.push(map_bib_diagnostic(
                    warning,
                    self.entry_documents.get(item.entry).copied().unwrap_or(0),
                ));
            }
            let source_span = self.entry_span(item).unwrap_or(span);
            let content = formatted_inlines(&entry, source_span);
            if self.style == CitationStyle::Authoryear {
                blocks.push(Block::Paragraph(content));
            } else {
                let label = format!("[{}]", item.label);
                blocks.push(Block::ListItem {
                    level: 1,
                    label: Some((label.clone(), source_span)),
                    content,
                    extra_gap_before_pt: 0.0,
                    extra_gap_after_pt: 0.0,
                    leftmargin: ListLeftMargin::Default,
                    widest_label: Some(widest.clone()),
                    lists: Vec::new(),
                    item: Some(ItemLabel::Template { text: label }),
                });
            }
        }
        blocks
    }

    fn numeric_citation(
        &self,
        name: &str,
        keys: &[String],
        pre: Option<&str>,
        post: Option<&str>,
        span: Span,
        space_before: bool,
    ) -> Vec<Inline> {
        if name == "citeauthor" || name == "citeyear" {
            let mut out = Vec::new();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(text_inline("; ", span, TextStyle::default(), false));
                }
                let Some(entry) = self.database.get(key) else {
                    out.push(text_inline(key, span, TextStyle::BOLD, false));
                    continue;
                };
                if name == "citeauthor" {
                    out.push(text_inline(
                        &author_text(&self.database, entry),
                        span,
                        TextStyle::default(),
                        index == 0 && space_before,
                    ));
                } else {
                    out.push(text_inline(
                        &entry.get("year").map(latex::typeset).unwrap_or_default(),
                        span,
                        TextStyle::default(),
                        index == 0 && space_before,
                    ));
                }
            }
            return out;
        }
        if name == "textcite" {
            let mut out = Vec::new();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(text_inline("; ", span, TextStyle::default(), false));
                }
                if let Some(entry) = self.database.get(key) {
                    out.push(text_inline(
                        &author_text(&self.database, entry),
                        span,
                        TextStyle::default(),
                        index == 0 && space_before,
                    ));
                } else {
                    out.push(text_inline(
                        key,
                        span,
                        TextStyle::BOLD,
                        index == 0 && space_before,
                    ));
                }
                out.extend(self.numeric_bracket(
                    std::slice::from_ref(key),
                    (index == 0).then_some(pre).flatten(),
                    (index + 1 == keys.len()).then_some(post).flatten(),
                    span,
                    true,
                ));
            }
            return out;
        }
        self.numeric_bracket(keys, pre, post, span, space_before)
    }

    fn numeric_bracket(
        &self,
        keys: &[String],
        pre: Option<&str>,
        post: Option<&str>,
        span: Span,
        space_before: bool,
    ) -> Vec<Inline> {
        let mut out = vec![text_inline("[", span, TextStyle::default(), space_before)];
        if let Some(pre) = nonempty_note(pre) {
            out.push(text_inline(
                &format!("{} ", clean_note(pre)),
                span,
                TextStyle::default(),
                false,
            ));
        }
        for (index, key) in keys.iter().enumerate() {
            if index > 0 {
                out.push(text_inline(", ", span, TextStyle::default(), false));
            }
            match self.labels.get(&key.to_ascii_lowercase()) {
                Some(label) => out.push(text_inline(label, span, TextStyle::default(), false)),
                None => out.push(text_inline(key, span, TextStyle::BOLD, false)),
            }
        }
        if let Some(post) = nonempty_note(post) {
            out.push(text_inline(
                &format!(", {}", clean_note(post)),
                span,
                TextStyle::default(),
                false,
            ));
        }
        out.push(text_inline("]", span, TextStyle::default(), false));
        out
    }

    fn authoryear_citation(
        &self,
        name: &str,
        keys: &[String],
        pre: Option<&str>,
        post: Option<&str>,
        span: Span,
        space_before: bool,
    ) -> Vec<Inline> {
        let mut out = Vec::new();
        let textcite = name == "textcite";
        let author_only = name == "citeauthor";
        let year_only = name == "citeyear";
        let parenthetical = !textcite && !author_only && !year_only;
        if parenthetical {
            out.push(text_inline("(", span, TextStyle::default(), space_before));
        }
        if let Some(pre) = nonempty_note(pre) {
            out.push(text_inline(
                &clean_note(pre),
                span,
                TextStyle::default(),
                !parenthetical,
            ));
            out.push(text_inline(" ", span, TextStyle::default(), false));
        }
        for (index, key) in keys.iter().enumerate() {
            if index > 0 {
                out.push(text_inline("; ", span, TextStyle::default(), false));
            }
            let Some(entry) = self.database.get(key) else {
                out.push(text_inline(key, span, TextStyle::BOLD, false));
                continue;
            };
            let author = author_text(&self.database, entry);
            let year = entry.get("year").map(latex::typeset).unwrap_or_default();
            if author_only {
                out.push(text_inline(&author, span, TextStyle::default(), false));
            } else if year_only {
                out.push(text_inline(&year, span, TextStyle::default(), false));
            } else if textcite {
                out.push(text_inline(&author, span, TextStyle::default(), false));
                out.push(text_inline(" (", span, TextStyle::default(), false));
                out.push(text_inline(&year, span, TextStyle::default(), false));
                out.push(text_inline(")", span, TextStyle::default(), false));
            } else {
                out.push(text_inline(&author, span, TextStyle::default(), false));
                out.push(text_inline(", ", span, TextStyle::default(), false));
                out.push(text_inline(&year, span, TextStyle::default(), false));
            }
        }
        if let Some(post) = nonempty_note(post) {
            out.push(text_inline(
                &format!(", {}", clean_note(post)),
                span,
                TextStyle::default(),
                false,
            ));
        }
        if parenthetical {
            out.push(text_inline(")", span, TextStyle::default(), false));
        }
        out
    }

    fn entry_span(&self, item: &BibItem) -> Option<Span> {
        let document = self.entry_documents.get(item.entry).copied()?;
        let entry = self.database.entries.get(item.entry)?;
        Some(Span::in_document(
            DocumentId(document),
            entry.span.start,
            entry.span.end,
        ))
    }
}

fn biblatex_package<T: Borrow<Token>>(tokens: &[T]) -> Option<(String, Span)> {
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
        let command_span = tokens[i].borrow().span;
        let mut cursor = i + 1;
        let options = optional_text(tokens, cursor);
        if let Some((_, after, _)) = options.as_ref() {
            cursor = *after;
        }
        if let Some(group) = group_text(tokens, cursor) {
            if group
                .text
                .split(',')
                .map(str::trim)
                .any(|package| package == "biblatex")
            {
                return Some((
                    options.map(|(text, _, _)| text).unwrap_or_default(),
                    command_span,
                ));
            }
            i = group.after;
        } else {
            i = cursor;
        }
    }
    None
}

fn parse_package_options(
    raw: &str,
    span: Span,
    style: &mut CitationStyle,
    sorting: &mut Sorting,
    diags: &mut Vec<Diagnostic>,
) {
    for option in split_options(raw) {
        let Some((key, value)) = option.split_once('=') else {
            warn_option(option, span, diags);
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches(['{', '}']).to_ascii_lowercase();
        match (key.as_str(), value.as_str()) {
            ("style", "numeric") => *style = CitationStyle::Numeric,
            ("style", "authoryear") => *style = CitationStyle::Authoryear,
            ("style", "alphabetic") => {
                *style = CitationStyle::Alphabetic;
                diags.push(Diagnostic::warning(
                    "Package biblatex Warning: style=alphabetic is not yet supported; using numeric labels",
                    Some(span),
                    Some("used numeric citation labels and entry order".into()),
                ));
            }
            ("sorting", "none") => *sorting = Sorting::None,
            ("sorting", "nyt") => *sorting = Sorting::Nyt,
            ("backend", "biber") => {}
            ("style", _) | ("sorting", _) | ("backend", _) => warn_option(option, span, diags),
            _ => warn_option(option, span, diags),
        }
    }
}

fn warn_option(option: &str, span: Span, diags: &mut Vec<Diagnostic>) {
    diags.push(Diagnostic::warning(
        format!("Package biblatex Warning: option `{option}` is not implemented"),
        Some(span),
        Some("continued with the supported biblatex subset".into()),
    ));
}

fn scan_commands<T: Borrow<Token>>(tokens: &[T]) -> (Vec<ResourceUse>, Vec<CitationUse>) {
    let mut resources = Vec::new();
    let mut citations = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let TokenKind::Command(name) = &tokens[i].borrow().kind else {
            i += 1;
            continue;
        };
        let command_span = tokens[i].borrow().span;
        if name == "addbibresource" {
            let mut cursor = i + 1;
            if let Some((_, after, _)) = optional_text(tokens, cursor) {
                cursor = after;
            }
            if let Some(group) = group_text(tokens, cursor) {
                resources.push(ResourceUse {
                    requested: group.text.trim().to_string(),
                    span: command_span.merge(group.span),
                });
                i = group.after;
            } else {
                i = cursor;
            }
        } else if name == "nocite" {
            if let Some(group) = group_text(tokens, i + 1) {
                add_citations(&mut citations, &group.text, group.span);
                i = group.after;
            } else {
                i += 1;
            }
        } else if is_cite_command(name) {
            let mut cursor = i + 1;
            if let Some((_, after, _)) = optional_text(tokens, cursor) {
                cursor = after;
            }
            if let Some((_, after, _)) = optional_text(tokens, cursor) {
                cursor = after;
            }
            if let Some(group) = group_text(tokens, cursor) {
                add_citations(&mut citations, &group.text, group.span);
                i = group.after;
            } else {
                i = cursor;
            }
        } else {
            i += 1;
        }
    }
    (resources, citations)
}

fn is_cite_command(name: &str) -> bool {
    matches!(
        name,
        "cite" | "parencite" | "textcite" | "autocite" | "citeauthor" | "citeyear"
    )
}

fn add_citations(out: &mut Vec<CitationUse>, text: &str, span: Span) {
    out.extend(
        text.split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(|key| CitationUse {
                key: key.to_string(),
                span,
            }),
    );
}

fn load_resources(
    database: &mut Database,
    entry_documents: &mut Vec<usize>,
    resources: &[ResourceUse],
    documents: &[SourceDocument<'_>],
    diags: &mut Vec<Diagnostic>,
) {
    let mut loaded = HashSet::new();
    for resource in resources {
        if !crate::parser::path_is_safe(&resource.requested) {
            diags.push(Diagnostic::error(
                format!(
                    "rejected bibliography resource path '{}'",
                    resource.requested
                ),
                Some(resource.span),
                Some("used only project-relative .bib paths".into()),
            ));
            continue;
        }
        let Some(document) = find_resource(&resource.requested, documents) else {
            diags.push(Diagnostic::warning(
                format!(
                    "bibliography resource '{}' was not found",
                    resource.requested
                ),
                Some(resource.span),
                Some("continued with the resources that were supplied".into()),
            ));
            continue;
        };
        if !loaded.insert(document) {
            continue;
        }
        let parsed = flashtex_bibliography::load(documents[document].text);
        for diagnostic in &parsed.diagnostics {
            diags.push(map_bib_diagnostic(diagnostic, document));
        }
        for entry in parsed.entries {
            if database.get(&entry.key).is_some() {
                let span =
                    Span::in_document(DocumentId(document), entry.span.start, entry.span.end);
                diags.push(Diagnostic::warning(
                    format!(
                        "duplicate bibliography key '{}'; kept the first entry",
                        entry.key
                    ),
                    Some(span),
                    Some("ignored the later resource entry".into()),
                ));
                continue;
            }
            database.entries.push(entry);
            entry_documents.push(document);
        }
    }
}

fn find_resource(requested: &str, documents: &[SourceDocument<'_>]) -> Option<usize> {
    if let Some(index) = documents
        .iter()
        .position(|document| document.path == requested)
    {
        return Some(index);
    }
    if requested.ends_with(".bib") {
        return None;
    }
    let appended = format!("{requested}.bib");
    documents
        .iter()
        .position(|document| document.path == appended)
}

fn group_text<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<Group> {
    skip_trivia(tokens, &mut i);
    let open = tokens.get(i)?.borrow();
    if !matches!(open.kind, TokenKind::LBrace) {
        return None;
    }
    let start = open.span;
    let mut depth = 1usize;
    let mut text = String::new();
    i += 1;
    while i < tokens.len() {
        let token = tokens[i].borrow();
        match &token.kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some(Group {
                        text,
                        span: start.merge(token.span),
                        after: i + 1,
                    });
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

fn optional_text<T: Borrow<Token>>(tokens: &[T], mut i: usize) -> Option<(String, usize, Span)> {
    skip_trivia(tokens, &mut i);
    let first = tokens.get(i)?.borrow();
    let TokenKind::Word(first_word) = &first.kind else {
        return None;
    };
    if !first_word.starts_with('[') {
        return None;
    }
    let start = first.span;
    let mut raw = first_word.clone();
    let mut found = raw.contains(']');
    let mut end = first.span;
    i += 1;
    while !found && i < tokens.len() {
        let token = tokens[i].borrow();
        end = token.span;
        match &token.kind {
            TokenKind::Word(word) => {
                raw.push_str(word);
                found = word.contains(']');
            }
            TokenKind::Space | TokenKind::ParBreak => raw.push(' '),
            TokenKind::Command(name) => {
                raw.push('\\');
                raw.push_str(name);
            }
            _ => {}
        }
        i += 1;
    }
    let text = raw
        .strip_prefix('[')
        .unwrap_or(&raw)
        .split_once(']')
        .map_or(raw.as_str(), |(inside, _)| inside)
        .to_string();
    Some((text, i, start.merge(end)))
}

fn skip_trivia<T: Borrow<Token>>(tokens: &[T], i: &mut usize) {
    while matches!(
        tokens.get(*i).map(|token| &token.borrow().kind),
        Some(TokenKind::Space | TokenKind::Comment)
    ) {
        *i += 1;
    }
}

fn split_options(raw: &str) -> Vec<&str> {
    let mut options = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (index, byte) in raw.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                if !raw[start..index].trim().is_empty() {
                    options.push(raw[start..index].trim());
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    if !raw[start..].trim().is_empty() {
        options.push(raw[start..].trim());
    }
    options
}

fn print_title(options: &str) -> Option<String> {
    split_options(options).into_iter().find_map(|option| {
        let (key, value) = option.split_once('=')?;
        (key.trim() == "title").then(|| value.trim().trim_matches(['{', '}']).to_string())
    })
}

fn author_text(database: &Database, entry: &flashtex_bibliography::Entry) -> String {
    database
        .effective_field(entry, "author")
        .or_else(|| database.effective_field(entry, "editor"))
        .map(|names| latex::typeset(&format_names(names)))
        .unwrap_or_default()
}

fn nonempty_note(note: Option<&str>) -> Option<&str> {
    note.map(str::trim).filter(|note| !note.is_empty())
}

fn clean_note(note: &str) -> String {
    latex::typeset(note).replace('\u{00a0}', " ")
}

fn text_inline(text: &str, span: Span, style: TextStyle, space_before: bool) -> Inline {
    Inline::Text {
        text: text.to_string(),
        span,
        style,
        space_before,
    }
}

fn formatted_inlines(entry: &flashtex_bibliography::FormattedEntry, span: Span) -> Vec<Inline> {
    entry
        .runs()
        .into_iter()
        .map(|run| {
            let style = match run.style {
                RunStyle::Plain => TextStyle::default(),
                RunStyle::Emphasis => TextStyle {
                    italic: true,
                    ..TextStyle::default()
                },
                RunStyle::Bold => TextStyle::BOLD,
            };
            text_inline(&run.text, span, style, false)
        })
        .collect()
}

fn map_bib_diagnostic(diagnostic: &BibDiagnostic, document: usize) -> Diagnostic {
    let span = diagnostic
        .span
        .map(|span| Span::in_document(DocumentId(document), span.start, span.end));
    match diagnostic.severity {
        BibSeverity::Error => Diagnostic::error(
            diagnostic.message.clone(),
            span,
            diagnostic.recovery.clone(),
        ),
        BibSeverity::Warning => Diagnostic::warning(
            diagnostic.message.clone(),
            span,
            diagnostic.recovery.clone(),
        ),
    }
}

fn bib_span(span: Span) -> flashtex_bibliography::Span {
    flashtex_bibliography::Span::new(span.start, span.end)
}
