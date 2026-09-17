//! Citation resolution, deterministic ordering, and label generation.
//!
//! Ordering and labels follow the standard styles:
//! - `unsrt`: citation order, numeric labels.
//! - `plain`: `plain.bst` `presort` (author/editor/organization names, year,
//!   title, all purified and lower-cased), numeric labels.
//! - `alpha`: `alpha.bst` `calc.label` (`[Knu84]`, `[GMS94]`, `[ABC+00]`)
//!   with `a`, `b`, ... suffixes on colliding labels, sorted by label then
//!   the `plain` key.

use crate::diagnostics::{Diagnostic, Span};
use crate::latex::{self, CaseMode};
use crate::model::{Database, Entry};
use crate::names::{Name, format_name, parse_names};

/// One `\cite` key as it appears in the document, with the span of the key
/// text so a missing key can be reported against the `.tex` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub key: String,
    pub span: Span,
}

impl Citation {
    pub fn new(key: impl Into<String>, span: Span) -> Self {
        Citation {
            key: key.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    /// Alphabetical order, numeric labels (`plain.bst`).
    Plain,
    /// Citation order, numeric labels (`unsrt.bst`).
    Unsrt,
    /// Author-year labels, sorted by label (`alpha.bst`).
    Alpha,
}

impl Style {
    pub fn as_str(self) -> &'static str {
        match self {
            Style::Plain => "plain",
            Style::Unsrt => "unsrt",
            Style::Alpha => "alpha",
        }
    }
}

/// One entry of the resulting bibliography list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibItem {
    /// The key as spelled in the database entry.
    pub key: String,
    /// Index into `Database::entries`.
    pub entry: usize,
    /// `1`, `2`, ... for numeric styles; `Knu84`, `Knu84a` for alpha
    /// (decoded to Unicode, without brackets).
    pub label: String,
    /// Indices into the `citations` slice that cited this item.
    pub cited_by: Vec<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    /// Bibliography items in final order.
    pub items: Vec<BibItem>,
    /// For each input citation, the index into `items` it resolved to.
    pub citations: Vec<Option<usize>>,
    /// One warning per citation whose key is not in the database, carrying
    /// the citation's own span.
    pub missing: Vec<Diagnostic>,
}

impl Resolution {
    pub fn label_for(&self, citation_index: usize) -> Option<&str> {
        let item = (*self.citations.get(citation_index)?)?;
        self.items.get(item).map(|i| i.label.as_str())
    }
}

/// Resolve citations against `db`. The key `*` (from `\nocite{*}`) includes
/// every entry, in database order, after the explicitly cited ones.
pub fn resolve(citations: &[Citation], db: &Database, style: Style) -> Resolution {
    let mut order: Vec<usize> = Vec::new(); // entry indices in first-citation order
    let mut cited_by: Vec<Vec<usize>> = Vec::new();
    let mut per_citation: Vec<Option<usize>> = Vec::with_capacity(citations.len());
    let mut missing = Vec::new();
    let mut cite_all = false;

    for (ci, c) in citations.iter().enumerate() {
        if c.key == "*" {
            cite_all = true;
            per_citation.push(None);
            continue;
        }
        match db.index_of(&c.key) {
            Some(idx) => {
                let pos = match order.iter().position(|&e| e == idx) {
                    Some(p) => p,
                    None => {
                        order.push(idx);
                        cited_by.push(Vec::new());
                        order.len() - 1
                    }
                };
                cited_by[pos].push(ci);
                per_citation.push(Some(pos));
            }
            None => {
                missing.push(Diagnostic::warning(
                    format!(
                        "citation key '{}' is not in the bibliography database",
                        c.key
                    ),
                    Some(c.span),
                    Some("rendered the citation as [?] and left it out of the bibliography".into()),
                ));
                per_citation.push(None);
            }
        }
    }
    if cite_all {
        for idx in 0..db.entries.len() {
            if !order.contains(&idx) {
                order.push(idx);
                cited_by.push(Vec::new());
            }
        }
    }

    // Positions into `order`, permuted into final order.
    let mut positions: Vec<usize> = (0..order.len()).collect();
    let base_labels: Vec<String> = match style {
        Style::Alpha => order
            .iter()
            .map(|&idx| alpha_label(db, &db.entries[idx]))
            .collect(),
        _ => Vec::new(),
    };
    match style {
        Style::Unsrt => {}
        Style::Plain => {
            let keys: Vec<String> = order
                .iter()
                .map(|&idx| sort_key(db, &db.entries[idx]))
                .collect();
            positions.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
        }
        Style::Alpha => {
            let keys: Vec<(String, String)> = order
                .iter()
                .enumerate()
                .map(|(i, &idx)| (sortify(&base_labels[i]), sort_key(db, &db.entries[idx])))
                .collect();
            positions.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
        }
    }

    let labels: Vec<String> = match style {
        Style::Alpha => {
            let sorted: Vec<&str> = positions.iter().map(|&p| base_labels[p].as_str()).collect();
            let suffixed = dedup_alpha_labels(&sorted);
            suffixed.into_iter().map(|l| latex::decode(&l)).collect()
        }
        _ => (1..=positions.len()).map(|n| n.to_string()).collect(),
    };

    let mut items = Vec::with_capacity(positions.len());
    let mut new_index_of_pos = vec![0usize; positions.len()];
    for (final_index, &p) in positions.iter().enumerate() {
        new_index_of_pos[p] = final_index;
        let idx = order[p];
        items.push(BibItem {
            key: db.entries[idx].key.clone(),
            entry: idx,
            label: labels[final_index].clone(),
            cited_by: cited_by[p].clone(),
        });
    }
    let citations_out = per_citation
        .into_iter()
        .map(|p| p.map(|p| new_index_of_pos[p]))
        .collect();

    Resolution {
        items,
        citations: citations_out,
        missing,
    }
}

/// `purify$` then `"l" change.case$`.
pub fn sortify(s: &str) -> String {
    latex::change_case(&latex::purify(s), CaseMode::Lower)
}

/// BibTeX `chop.word`: drop a leading word (case-sensitive, as `plain.bst`).
fn chop_word(s: &str, word: &str) -> String {
    match s.strip_prefix(word) {
        Some(rest) => rest.to_string(),
        None => s.to_string(),
    }
}

fn sort_format_names(field: &str) -> String {
    let mut out = String::new();
    for (i, name) in parse_names(field).iter().enumerate() {
        if i > 0 {
            out.push_str("   ");
        }
        if name.is_others() {
            out.push_str("et al");
        } else {
            let t = format_name(name, "{vv{ } }{ll{ }}{  ff{ }}{  jj{ }}");
            out.push_str(&sortify(&t));
        }
    }
    out
}

fn sort_format_title(title: &str) -> String {
    let t = chop_word(title, "A ");
    let t = chop_word(&t, "An ");
    let t = chop_word(&t, "The ");
    sortify(&t)
}

fn organization_sort(db: &Database, e: &Entry) -> Option<String> {
    db.effective_field(e, "organization")
        .map(|o| sortify(&chop_word(o, "The ")))
}

/// `plain.bst` `presort`: names, then year, then title.
pub fn sort_key(db: &Database, e: &Entry) -> String {
    let author = db.effective_field(e, "author");
    let editor = db.effective_field(e, "editor");
    let key_field = db.effective_field(e, "key").map(sortify);
    let names = match e.entry_type.as_str() {
        "book" | "inbook" => author
            .or(editor)
            .map(sort_format_names)
            .or(key_field)
            .unwrap_or_default(),
        "proceedings" => editor
            .map(sort_format_names)
            .or_else(|| organization_sort(db, e))
            .or(key_field)
            .unwrap_or_default(),
        "manual" => author
            .map(sort_format_names)
            .or_else(|| organization_sort(db, e))
            .or(key_field)
            .unwrap_or_default(),
        _ => author
            .map(sort_format_names)
            .or(key_field)
            .unwrap_or_default(),
    };
    let year = db
        .effective_field(e, "year")
        .map(sortify)
        .unwrap_or_default();
    let title = db
        .effective_field(e, "title")
        .map(sort_format_title)
        .unwrap_or_default();
    format!("{names}    {year}    {title}")
}

/// `alpha.bst` `format.lab.names`.
fn format_lab_names(field: &str) -> String {
    let names: Vec<Name> = parse_names(field);
    let numnames = names.len();
    if numnames == 0 {
        return String::new();
    }
    if numnames == 1 {
        let s = format_name(&names[0], "{v{}}{l{}}");
        if latex::text_length(&s) < 2 {
            return latex::text_prefix(&format_name(&names[0], "{ll}"), 3);
        }
        return s;
    }
    let namesleft = if numnames > 4 { 3 } else { numnames };
    let mut out = String::new();
    for (i, name) in names.iter().take(namesleft).enumerate() {
        if i + 1 == numnames && name.is_others() {
            out.push('+');
        } else {
            out.push_str(&format_name(name, "{v{}}{l{}}"));
        }
    }
    if numnames > 4 {
        out.push('+');
    }
    out
}

fn key_label(db: &Database, e: &Entry) -> String {
    match db.effective_field(e, "key") {
        Some(k) => latex::text_prefix(k, 3),
        None => e.key.chars().take(3).collect(),
    }
}

fn key_organization_label(db: &Database, e: &Entry) -> String {
    match db.effective_field(e, "key") {
        Some(k) => latex::text_prefix(k, 3),
        None => match db.effective_field(e, "organization") {
            Some(o) => latex::text_prefix(&chop_word(o, "The "), 3),
            None => e.key.chars().take(3).collect(),
        },
    }
}

/// `alpha.bst` `calc.label` without the disambiguating suffix.
pub fn alpha_label(db: &Database, e: &Entry) -> String {
    let author = db.effective_field(e, "author");
    let editor = db.effective_field(e, "editor");
    let names = match e.entry_type.as_str() {
        "book" | "inbook" => author
            .or(editor)
            .map(format_lab_names)
            .unwrap_or_else(|| key_label(db, e)),
        "proceedings" => editor
            .map(format_lab_names)
            .unwrap_or_else(|| key_organization_label(db, e)),
        "manual" => author
            .map(format_lab_names)
            .unwrap_or_else(|| key_organization_label(db, e)),
        _ => author
            .map(format_lab_names)
            .unwrap_or_else(|| key_label(db, e)),
    };
    let year = db
        .effective_field(e, "year")
        .map(latex::purify)
        .unwrap_or_default();
    let year_chars: Vec<char> = year.chars().collect();
    let suffix: String = year_chars[year_chars.len().saturating_sub(2)..]
        .iter()
        .collect();
    format!("{names}{suffix}")
}

/// Append `a`, `b`, ... to runs of identical labels (already in final order).
fn dedup_alpha_labels(labels: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = labels.iter().map(|l| (*l).to_string()).collect();
    let mut i = 0;
    while i < labels.len() {
        let mut j = i + 1;
        while j < labels.len() && labels[j] == labels[i] {
            j += 1;
        }
        if j - i > 1 {
            for (n, slot) in out[i..j].iter_mut().enumerate() {
                slot.push(extra_label_char(n));
            }
        }
        i = j;
    }
    out
}

fn extra_label_char(n: usize) -> char {
    // alpha.bst uses int.to.chr$ from 'a'; after 'z' it keeps counting.
    char::from_u32(('a' as u32) + n as u32).unwrap_or('?')
}
