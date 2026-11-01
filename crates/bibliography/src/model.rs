//! The parsed `.bib` model and the standard entry-type tables.

use crate::diagnostics::{Diagnostic, Span};

/// One `name = value` pair. `name` is lower-cased; `value` is the resolved
/// concatenation with outer delimiters removed and whitespace runs collapsed
/// to single spaces, braces and control sequences preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub value: String,
    /// From the first byte of the name to the last byte of the value.
    pub span: Span,
    pub name_span: Span,
    /// From the first value part to the end of the last one (delimiters included).
    pub value_span: Span,
}

/// Insertion-ordered field map with case-insensitive lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fields {
    items: Vec<Field>,
}

impl Fields {
    pub fn new() -> Self {
        Fields::default()
    }

    /// Adds a field; returns `false` (and keeps the first) when the name is
    /// already present, mirroring BibTeX's "ignore repeated field" rule.
    pub fn insert(&mut self, field: Field) -> bool {
        if self.get_field(&field.name).is_some() {
            return false;
        }
        self.items.push(field);
        true
    }

    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.items
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.get_field(name).map(|f| f.value.as_str())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get_field(name).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Field> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub key_span: Span,
    /// Lower-cased entry type (`article`, `book`, ...).
    pub entry_type: String,
    pub type_span: Span,
    pub fields: Fields,
    /// From `@` to the closing delimiter, inclusive.
    pub span: Span,
}

impl Entry {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields.get(name)
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.get_field(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro {
    pub name: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preamble {
    pub value: String,
    pub span: Span,
}

/// Everything parsed from one `.bib` source plus its diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Database {
    pub entries: Vec<Entry>,
    /// `@string` definitions in file order (the built-in month macros are not
    /// listed; see [`crate::parser::MONTH_MACROS`]).
    pub macros: Vec<Macro>,
    pub preambles: Vec<Preamble>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Database {
    /// Case-insensitive lookup, as BibTeX compares cite keys.
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.key.eq_ignore_ascii_case(key))
    }

    pub fn index_of(&self, key: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.key.eq_ignore_ascii_case(key))
    }

    /// A field value with one level of `crossref` inheritance: if `entry`
    /// lacks `name` and names a `crossref` that exists, the parent's own
    /// field is used. Parents' own `crossref`s are not followed.
    pub fn effective_field<'a>(&'a self, entry: &'a Entry, name: &str) -> Option<&'a str> {
        if let Some(v) = entry.get(name) {
            return Some(v);
        }
        let parent_key = entry.get("crossref")?;
        let parent = self.get(parent_key)?;
        parent.get(name)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Required/optional fields of a standard entry type. A required slot lists
/// alternatives (`author` or `editor`); `chapter`/`pages` for `inbook` is
/// "chapter and/or pages", also modelled as one alternative slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryTypeSpec {
    pub name: &'static str,
    pub required: &'static [&'static [&'static str]],
    pub optional: &'static [&'static str],
}

/// The fourteen standard BibTeX entry types (btxdoc §3.1) as used by
/// `plain.bst`; `conference` is an alias for `inproceedings`.
pub const STANDARD_ENTRY_TYPES: &[EntryTypeSpec] = &[
    EntryTypeSpec {
        name: "article",
        required: &[&["author"], &["title"], &["journal"], &["year"]],
        optional: &["volume", "number", "pages", "month", "note"],
    },
    EntryTypeSpec {
        name: "book",
        required: &[&["author", "editor"], &["title"], &["publisher"], &["year"]],
        optional: &[
            "volume", "number", "series", "address", "edition", "month", "note",
        ],
    },
    EntryTypeSpec {
        name: "booklet",
        required: &[&["title"]],
        optional: &["author", "howpublished", "address", "month", "year", "note"],
    },
    EntryTypeSpec {
        name: "conference",
        required: &[&["author"], &["title"], &["booktitle"], &["year"]],
        optional: &[
            "editor",
            "volume",
            "number",
            "series",
            "pages",
            "address",
            "month",
            "organization",
            "publisher",
            "note",
        ],
    },
    EntryTypeSpec {
        name: "inbook",
        required: &[
            &["author", "editor"],
            &["title"],
            &["chapter", "pages"],
            &["publisher"],
            &["year"],
        ],
        optional: &[
            "volume", "number", "series", "type", "address", "edition", "month", "note",
        ],
    },
    EntryTypeSpec {
        name: "incollection",
        required: &[
            &["author"],
            &["title"],
            &["booktitle"],
            &["publisher"],
            &["year"],
        ],
        optional: &[
            "editor", "volume", "number", "series", "type", "chapter", "pages", "address",
            "edition", "month", "note",
        ],
    },
    EntryTypeSpec {
        name: "inproceedings",
        required: &[&["author"], &["title"], &["booktitle"], &["year"]],
        optional: &[
            "editor",
            "volume",
            "number",
            "series",
            "pages",
            "address",
            "month",
            "organization",
            "publisher",
            "note",
        ],
    },
    EntryTypeSpec {
        name: "manual",
        required: &[&["title"]],
        optional: &[
            "author",
            "organization",
            "address",
            "edition",
            "month",
            "year",
            "note",
        ],
    },
    EntryTypeSpec {
        name: "mastersthesis",
        required: &[&["author"], &["title"], &["school"], &["year"]],
        optional: &["type", "address", "month", "note"],
    },
    EntryTypeSpec {
        name: "misc",
        required: &[],
        optional: &["author", "title", "howpublished", "month", "year", "note"],
    },
    EntryTypeSpec {
        name: "phdthesis",
        required: &[&["author"], &["title"], &["school"], &["year"]],
        optional: &["type", "address", "month", "note"],
    },
    EntryTypeSpec {
        name: "proceedings",
        required: &[&["title"], &["year"]],
        optional: &[
            "editor",
            "volume",
            "number",
            "series",
            "address",
            "month",
            "organization",
            "publisher",
            "note",
        ],
    },
    EntryTypeSpec {
        name: "techreport",
        required: &[&["author"], &["title"], &["institution"], &["year"]],
        optional: &["type", "number", "address", "month", "note"],
    },
    EntryTypeSpec {
        name: "unpublished",
        required: &[&["author"], &["title"], &["note"]],
        optional: &["month", "year"],
    },
];

pub fn entry_type_spec(name: &str) -> Option<&'static EntryTypeSpec> {
    STANDARD_ENTRY_TYPES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
}

/// Semantic checks over a parsed database: duplicate keys (the later entry is
/// dropped), unknown entry types (warning), and missing required fields
/// (warning, as BibTeX reports them). Duplicate removal happens here so the
/// database `entries` afterwards hold one entry per key.
pub fn validate(db: &mut Database) {
    let mut diagnostics = Vec::new();
    let mut kept: Vec<Entry> = Vec::with_capacity(db.entries.len());
    for entry in std::mem::take(&mut db.entries) {
        if let Some(first) = kept.iter().find(|e| e.key.eq_ignore_ascii_case(&entry.key)) {
            diagnostics.push(Diagnostic::error(
                format!(
                    "duplicate entry key '{}' (first defined at bytes {})",
                    entry.key, first.key_span
                ),
                Some(entry.key_span),
                Some("ignored this entry and kept the first definition".into()),
            ));
            continue;
        }
        kept.push(entry);
    }
    db.entries = kept;

    for entry in &db.entries {
        let Some(spec) = entry_type_spec(&entry.entry_type) else {
            diagnostics.push(Diagnostic::warning(
                format!("unknown entry type '@{}'", entry.entry_type),
                Some(entry.type_span),
                Some("treated as @misc".into()),
            ));
            continue;
        };
        for slot in spec.required {
            let present = slot.iter().any(|f| db.effective_field(entry, f).is_some());
            if !present {
                let what = slot.join(" or ");
                diagnostics.push(Diagnostic::warning(
                    format!(
                        "missing required field '{}' in @{} entry '{}'",
                        what, spec.name, entry.key
                    ),
                    Some(entry.key_span),
                    Some("the entry is kept; the field is rendered as empty".into()),
                ));
            }
        }
        if let Some(parent) = entry.get("crossref")
            && db.get(parent).is_none()
        {
            diagnostics.push(Diagnostic::warning(
                format!(
                    "crossref '{}' of entry '{}' does not exist",
                    parent, entry.key
                ),
                Some(
                    entry
                        .field("crossref")
                        .map(|f| f.value_span)
                        .unwrap_or(entry.key_span),
                ),
                Some("no fields inherited".into()),
            ));
        }
    }
    db.diagnostics.extend(diagnostics);
}
