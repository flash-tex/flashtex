//! Bibliography layout data: each entry as blocks of styled runs, following
//! the `plain.bst` conventions (`unsrt.bst` and `alpha.bst` format entries
//! identically; they differ only in ordering and labels).
//!
//! This is data for a layout consumer, not typesetting. Run text is decoded
//! Unicode (`--` → en dash, `~` → no-break space, accents composed); the raw
//! LaTeX of each run is kept alongside for consumers that typeset TeX.
//!
//! Provenance: the entry functions below are transliterations of the
//! corresponding `plain.bst` functions (`article`, `book`, ..., `format.names`,
//! `format.vol.num.pages`, `output.nonnull`, `new.block`, `fin.entry`, ...)
//! as documented in README "Style conventions". Deviations are listed there.

use crate::diagnostics::Diagnostic;
use crate::latex::{self, CaseMode};
use crate::model::{Database, Entry};
use crate::names::{format_name, parse_names};
use crate::resolve::{Resolution, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunStyle {
    Plain,
    /// `{\em ...}` in the `.bbl`: titles of books/theses, journal and series
    /// names, booktitles.
    Emphasis,
    /// Reserved for styles that bold volumes (`plain.bst` does not).
    Bold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Decoded, typeset-ready text.
    pub text: String,
    /// The same text as raw LaTeX (what `plain.bst` would write).
    pub latex: String,
    pub style: RunStyle,
}

/// A `\newblock` unit. Blocks end with their own punctuation; a consumer
/// joins them with a space (or a line break, as `openbib` would).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Block {
    pub runs: Vec<Run>,
}

impl Block {
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    pub fn latex(&self) -> String {
        self.runs
            .iter()
            .map(|r| match r.style {
                RunStyle::Plain => r.latex.clone(),
                RunStyle::Emphasis => format!("{{\\em {}}}", r.latex),
                RunStyle::Bold => format!("{{\\bf {}}}", r.latex),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedEntry {
    pub key: String,
    /// Label without brackets (`1` or `Knu84`).
    pub label: String,
    pub blocks: Vec<Block>,
    /// Style warnings `plain.bst` would print (`either.or.check` and friends).
    pub warnings: Vec<Diagnostic>,
}

impl FormattedEntry {
    /// All runs in order, blocks separated by a single plain space run.
    pub fn runs(&self) -> Vec<Run> {
        let mut out = Vec::new();
        for (i, b) in self.blocks.iter().enumerate() {
            if i > 0 {
                out.push(Run {
                    text: " ".into(),
                    latex: " ".into(),
                    style: RunStyle::Plain,
                });
            }
            out.extend(b.runs.iter().cloned());
        }
        out
    }

    /// Decoded text of the whole entry, blocks joined by a space.
    pub fn text(&self) -> String {
        self.blocks
            .iter()
            .map(Block::text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The `.bbl` item as `plain.bst`/`alpha.bst` would write it.
    pub fn to_bbl(&self, style: Style) -> String {
        let mut s = match style {
            Style::Alpha => format!("\\bibitem[{}]{{{}}}\n", self.label, self.key),
            _ => format!("\\bibitem{{{}}}\n", self.key),
        };
        for (i, b) in self.blocks.iter().enumerate() {
            if i > 0 {
                s.push_str("\\newblock ");
            }
            s.push_str(&b.latex());
            s.push('\n');
        }
        s
    }
}

/// Format every item of a resolution.
pub fn format_bibliography(db: &Database, resolution: &Resolution) -> Vec<FormattedEntry> {
    resolution
        .items
        .iter()
        .map(|item| format_entry(db, &db.entries[item.entry], &item.label))
        .collect()
}

/// Format one entry with the given label using the `plain.bst` conventions.
pub fn format_entry(db: &Database, entry: &Entry, label: &str) -> FormattedEntry {
    let mut f = Formatter {
        db,
        entry,
        out: Out::default(),
        warnings: Vec::new(),
    };
    match entry.entry_type.as_str() {
        "article" => f.article(),
        "book" => f.book(),
        "booklet" => f.booklet(),
        "inbook" => f.inbook(),
        "incollection" => f.incollection(),
        "inproceedings" | "conference" => f.inproceedings(),
        "manual" => f.manual(),
        "mastersthesis" => f.mastersthesis(),
        "phdthesis" => f.phdthesis(),
        "proceedings" => f.proceedings(),
        "techreport" => f.techreport(),
        "unpublished" => f.unpublished(),
        _ => f.misc(),
    }
    let blocks = f.out.finish();
    FormattedEntry {
        key: entry.key.clone(),
        label: label.to_string(),
        blocks,
        warnings: f.warnings,
    }
}

/// Raw (LaTeX) styled text under construction.
type Text = Vec<(String, RunStyle)>;

fn plain(s: impl Into<String>) -> Text {
    let s = s.into();
    if s.is_empty() {
        Vec::new()
    } else {
        vec![(s, RunStyle::Plain)]
    }
}

fn emph(s: &str) -> Text {
    if s.is_empty() {
        Vec::new()
    } else {
        vec![(s.to_string(), RunStyle::Emphasis)]
    }
}

fn cat(parts: Vec<Text>) -> Text {
    parts.into_iter().flatten().collect()
}

fn text_is_empty(t: &Text) -> bool {
    t.iter().all(|(s, _)| s.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeAll,
    MidSentence,
    AfterSentence,
    AfterBlock,
}

/// `plain.bst`'s `output.state` machine.
struct Out {
    blocks: Vec<Text>,
    current: Text,
    state: State,
}

impl Default for Out {
    fn default() -> Self {
        Out {
            blocks: Vec::new(),
            current: Vec::new(),
            state: State::BeforeAll,
        }
    }
}

impl Out {
    fn mid_sentence(&self) -> bool {
        self.state == State::MidSentence
    }

    /// `output`: nothing for empty text, else `output.nonnull`.
    fn output(&mut self, t: Text) {
        if text_is_empty(&t) {
            return;
        }
        match self.state {
            State::BeforeAll => {}
            State::MidSentence => self.push_plain(", "),
            State::AfterSentence => {
                self.add_period();
                self.push_plain(" ");
            }
            State::AfterBlock => {
                self.add_period();
                self.blocks.push(std::mem::take(&mut self.current));
            }
        }
        self.current.extend(t);
        self.state = State::MidSentence;
    }

    fn new_block(&mut self) {
        if self.state != State::BeforeAll {
            self.state = State::AfterBlock;
        }
    }

    fn new_sentence(&mut self) {
        if self.state != State::AfterBlock && self.state != State::BeforeAll {
            self.state = State::AfterSentence;
        }
    }

    fn push_plain(&mut self, s: &str) {
        self.current.push((s.to_string(), RunStyle::Plain));
    }

    /// `add.period$` on the current text.
    fn add_period(&mut self) {
        let last_text: String = self
            .current
            .iter()
            .map(|(s, _)| s.as_str())
            .collect::<String>();
        if latex::add_period(&last_text) == last_text {
            return;
        }
        match self.current.last_mut() {
            Some((s, RunStyle::Plain)) => s.push('.'),
            Some(_) => self.push_plain("."),
            None => {}
        }
    }

    /// `fin.entry`: `add.period$` and emit.
    fn finish(mut self) -> Vec<Block> {
        self.add_period();
        if !self.current.is_empty() {
            self.blocks.push(std::mem::take(&mut self.current));
        }
        self.blocks.into_iter().map(to_block).collect()
    }
}

fn to_block(text: Text) -> Block {
    let mut merged: Text = Vec::new();
    for (s, style) in text {
        if s.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some((prev, prev_style)) if *prev_style == style => prev.push_str(&s),
            _ => merged.push((s, style)),
        }
    }
    Block {
        runs: merged
            .into_iter()
            .map(|(latex_text, style)| Run {
                text: latex::typeset(&latex_text),
                latex: latex_text,
                style,
            })
            .collect(),
    }
}

struct Formatter<'a> {
    db: &'a Database,
    entry: &'a Entry,
    out: Out,
    warnings: Vec<Diagnostic>,
}

/// `tie.or.space.connect`: `a~b` if `b` is shorter than three characters.
fn tie_or_space_connect(a: &str, b: &str) -> String {
    if latex::text_length(b) < 3 {
        format!("{a}~{b}")
    } else {
        format!("{a} {b}")
    }
}

/// `n.dashify`: a lone `-` becomes `--`; runs of two or more are kept.
fn n_dashify(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '-' {
            let mut j = i;
            while j < chars.len() && chars[j] == '-' {
                j += 1;
            }
            if j - i == 1 {
                out.push_str("--");
            } else {
                out.extend(std::iter::repeat_n('-', j - i));
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// `multi.page.check`.
fn multi_page(pages: &str) -> bool {
    pages.contains(['-', ',', '+'])
}

/// `format.names` with `{ff~}{vv~}{ll}{, jj}`.
pub fn format_names(field: &str) -> String {
    let names = parse_names(field);
    let numnames = names.len();
    let mut out = String::new();
    for (i, name) in names.iter().enumerate() {
        let t = format_name(name, "{ff~}{vv~}{ll}{, jj}");
        if i == 0 {
            out.push_str(&t);
        } else if i + 1 < numnames {
            out.push_str(", ");
            out.push_str(&t);
        } else {
            if numnames > 2 {
                out.push(',');
            }
            if name.is_others() {
                out.push_str(" et~al.");
            } else {
                out.push_str(" and ");
                out.push_str(&t);
            }
        }
    }
    out
}

impl Formatter<'_> {
    fn field(&self, name: &str) -> Option<&str> {
        self.db.effective_field(self.entry, name)
    }

    fn get(&self, name: &str) -> String {
        self.field(name).unwrap_or_default().to_string()
    }

    fn empty(&self, name: &str) -> bool {
        self.field(name).is_none_or(|v| v.is_empty())
    }

    fn warn(&mut self, message: String) {
        self.warnings.push(Diagnostic::warning(
            format!("{message} in entry '{}'", self.entry.key),
            Some(self.entry.span),
            None,
        ));
    }

    fn format_authors(&self) -> Text {
        match self.field("author") {
            Some(a) => plain(format_names(a)),
            None => Vec::new(),
        }
    }

    fn format_editors(&self) -> Text {
        match self.field("editor") {
            Some(e) => {
                let n = parse_names(e).len();
                let suffix = if n > 1 { ", editors" } else { ", editor" };
                plain(format!("{}{}", format_names(e), suffix))
            }
            None => Vec::new(),
        }
    }

    fn format_title(&self) -> Text {
        match self.field("title") {
            Some(t) => plain(latex::change_case(t, CaseMode::Title)),
            None => Vec::new(),
        }
    }

    fn format_btitle(&self) -> Text {
        emph(&self.get("title"))
    }

    fn format_bvolume(&mut self) -> Text {
        if self.empty("volume") {
            return Vec::new();
        }
        let mut t = plain(tie_or_space_connect("volume", &self.get("volume")));
        if !self.empty("series") {
            t = cat(vec![t, plain(" of "), emph(&self.get("series"))]);
        }
        if !self.empty("number") {
            self.warn("can't use both volume and number fields".into());
        }
        t
    }

    fn format_number_series(&mut self) -> Text {
        if !self.empty("volume") {
            return Vec::new();
        }
        if self.empty("number") {
            return plain(self.get("series"));
        }
        let word = if self.out.mid_sentence() {
            "number"
        } else {
            "Number"
        };
        let mut s = tie_or_space_connect(word, &self.get("number"));
        if self.empty("series") {
            self.warn("there's a number but no series".into());
        } else {
            s.push_str(" in ");
            s.push_str(&self.get("series"));
        }
        plain(s)
    }

    fn format_edition(&self) -> Text {
        match self.field("edition") {
            None | Some("") => Vec::new(),
            Some(e) => {
                let mode = if self.out.mid_sentence() {
                    CaseMode::Lower
                } else {
                    CaseMode::Title
                };
                plain(format!("{} edition", latex::change_case(e, mode)))
            }
        }
    }

    fn format_date(&mut self) -> Text {
        match (self.field("year"), self.field("month")) {
            (None, None) => Vec::new(),
            (None, Some(m)) => {
                let m = m.to_string();
                self.warn("there's a month but no year".into());
                plain(m)
            }
            (Some(y), None) => plain(y),
            (Some(y), Some(m)) => plain(format!("{m} {y}")),
        }
    }

    fn format_pages(&self) -> Text {
        match self.field("pages") {
            None | Some("") => Vec::new(),
            Some(p) if multi_page(p) => plain(format!("pages {}", n_dashify(p))),
            Some(p) => plain(format!("page {p}")),
        }
    }

    fn format_vol_num_pages(&mut self) -> Text {
        let mut s = self.get("volume");
        if !self.empty("number") {
            s.push('(');
            s.push_str(&self.get("number"));
            s.push(')');
            if self.empty("volume") {
                self.warn("there's a number but no volume".into());
            }
        }
        if !self.empty("pages") {
            if s.is_empty() {
                return self.format_pages();
            }
            s.push(':');
            s.push_str(&n_dashify(&self.get("pages")));
        }
        plain(s)
    }

    fn format_chapter_pages(&self) -> Text {
        if self.empty("chapter") {
            return self.format_pages();
        }
        let word = match self.field("type") {
            None | Some("") => "chapter".to_string(),
            Some(t) => latex::change_case(t, CaseMode::Lower),
        };
        let mut s = tie_or_space_connect(&word, &self.get("chapter"));
        if !self.empty("pages") {
            s.push_str(", ");
            for (p, _) in self.format_pages() {
                s.push_str(&p);
            }
        }
        plain(s)
    }

    fn format_in_ed_booktitle(&self) -> Text {
        if self.empty("booktitle") {
            return Vec::new();
        }
        if self.empty("editor") {
            cat(vec![plain("In "), emph(&self.get("booktitle"))])
        } else {
            cat(vec![
                plain("In "),
                self.format_editors(),
                plain(", "),
                emph(&self.get("booktitle")),
            ])
        }
    }

    fn format_thesis_type(&self, default: &str) -> Text {
        match self.field("type") {
            None | Some("") => plain(default),
            Some(t) => plain(latex::change_case(t, CaseMode::Title)),
        }
    }

    fn format_tr_number(&self) -> Text {
        let t = match self.field("type") {
            None | Some("") => "Technical Report".to_string(),
            Some(t) => t.to_string(),
        };
        match self.field("number") {
            None | Some("") => plain(latex::change_case(&t, CaseMode::Title)),
            Some(n) => plain(tie_or_space_connect(&t, n)),
        }
    }

    fn note(&self) -> Text {
        plain(self.get("note"))
    }

    fn new_block_check_any(&mut self, fields: &[&str]) {
        if fields.iter().any(|f| !self.empty(f)) {
            self.out.new_block();
        }
    }

    fn new_sentence_check_any(&mut self, fields: &[&str]) {
        if fields.iter().any(|f| !self.empty(f)) {
            self.out.new_sentence();
        }
    }

    fn author_or_editor(&mut self) {
        let t = if self.empty("author") {
            self.format_editors()
        } else {
            self.format_authors()
        };
        self.out.output(t);
    }

    fn finish_with_note(&mut self) {
        self.out.new_block();
        let n = self.note();
        self.out.output(n);
    }

    fn article(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_title();
        self.out.output(t);
        self.out.new_block();
        let j = emph(&self.get("journal"));
        self.out.output(j);
        let t = self.format_vol_num_pages();
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn book(&mut self) {
        self.author_or_editor();
        self.out.new_block();
        let t = self.format_btitle();
        self.out.output(t);
        let t = self.format_bvolume();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_number_series();
        self.out.output(t);
        self.out.new_sentence();
        let t = plain(self.get("publisher"));
        self.out.output(t);
        let t = plain(self.get("address"));
        self.out.output(t);
        let t = self.format_edition();
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn booklet(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_title();
        self.out.output(t);
        self.new_block_check_any(&["howpublished", "address"]);
        let t = plain(self.get("howpublished"));
        self.out.output(t);
        let t = plain(self.get("address"));
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn inbook(&mut self) {
        self.author_or_editor();
        self.out.new_block();
        let t = self.format_btitle();
        self.out.output(t);
        let t = self.format_bvolume();
        self.out.output(t);
        let t = self.format_chapter_pages();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_number_series();
        self.out.output(t);
        self.out.new_sentence();
        let t = plain(self.get("publisher"));
        self.out.output(t);
        let t = plain(self.get("address"));
        self.out.output(t);
        let t = self.format_edition();
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn incollection(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_title();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_in_ed_booktitle();
        self.out.output(t);
        let t = self.format_bvolume();
        self.out.output(t);
        let t = self.format_number_series();
        self.out.output(t);
        let t = self.format_chapter_pages();
        self.out.output(t);
        self.out.new_sentence();
        let t = plain(self.get("publisher"));
        self.out.output(t);
        let t = plain(self.get("address"));
        self.out.output(t);
        let t = self.format_edition();
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn inproceedings(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_title();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_in_ed_booktitle();
        self.out.output(t);
        let t = self.format_bvolume();
        self.out.output(t);
        let t = self.format_number_series();
        self.out.output(t);
        let t = self.format_pages();
        self.out.output(t);
        if self.empty("address") {
            self.new_sentence_check_any(&["organization", "publisher"]);
            let t = plain(self.get("organization"));
            self.out.output(t);
            let t = plain(self.get("publisher"));
            self.out.output(t);
            let t = self.format_date();
            self.out.output(t);
        } else {
            let t = plain(self.get("address"));
            self.out.output(t);
            let t = self.format_date();
            self.out.output(t);
            self.out.new_sentence();
            let t = plain(self.get("organization"));
            self.out.output(t);
            let t = plain(self.get("publisher"));
            self.out.output(t);
        }
        self.finish_with_note();
    }

    fn manual(&mut self) {
        if self.empty("author") {
            if !self.empty("organization") {
                let t = plain(self.get("organization"));
                self.out.output(t);
                let t = plain(self.get("address"));
                self.out.output(t);
            }
        } else {
            let t = self.format_authors();
            self.out.output(t);
        }
        self.out.new_block();
        let t = self.format_btitle();
        self.out.output(t);
        if self.empty("author") {
            if self.empty("organization") {
                self.new_block_check_any(&["address"]);
                let t = plain(self.get("address"));
                self.out.output(t);
            }
        } else {
            self.new_block_check_any(&["organization", "address"]);
            let t = plain(self.get("organization"));
            self.out.output(t);
            let t = plain(self.get("address"));
            self.out.output(t);
        }
        let t = self.format_edition();
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn thesis(&mut self, default_type: &str, emphasized_title: bool) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = if emphasized_title {
            self.format_btitle()
        } else {
            self.format_title()
        };
        self.out.output(t);
        self.out.new_block();
        let t = self.format_thesis_type(default_type);
        self.out.output(t);
        let t = plain(self.get("school"));
        self.out.output(t);
        let t = plain(self.get("address"));
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn mastersthesis(&mut self) {
        self.thesis("Master's thesis", false);
    }

    fn phdthesis(&mut self) {
        self.thesis("PhD thesis", true);
    }

    fn misc(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.new_block_check_any(&["title", "howpublished"]);
        let t = self.format_title();
        self.out.output(t);
        self.new_block_check_any(&["howpublished"]);
        let t = plain(self.get("howpublished"));
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
        if ["author", "title", "howpublished", "month", "year", "note"]
            .iter()
            .all(|f| self.empty(f))
        {
            self.warn("all relevant fields are empty".into());
        }
    }

    fn proceedings(&mut self) {
        let t = if self.empty("editor") {
            plain(self.get("organization"))
        } else {
            self.format_editors()
        };
        self.out.output(t);
        self.out.new_block();
        let t = self.format_btitle();
        self.out.output(t);
        let t = self.format_bvolume();
        self.out.output(t);
        let t = self.format_number_series();
        self.out.output(t);
        if self.empty("address") {
            if self.empty("editor") {
                self.new_sentence_check_any(&["publisher"]);
            } else {
                self.new_sentence_check_any(&["organization", "publisher"]);
                let t = plain(self.get("organization"));
                self.out.output(t);
            }
            let t = plain(self.get("publisher"));
            self.out.output(t);
            let t = self.format_date();
            self.out.output(t);
        } else {
            let t = plain(self.get("address"));
            self.out.output(t);
            let t = self.format_date();
            self.out.output(t);
            self.out.new_sentence();
            if !self.empty("editor") {
                let t = plain(self.get("organization"));
                self.out.output(t);
            }
            let t = plain(self.get("publisher"));
            self.out.output(t);
        }
        self.finish_with_note();
    }

    fn techreport(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_title();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_tr_number();
        self.out.output(t);
        let t = plain(self.get("institution"));
        self.out.output(t);
        let t = plain(self.get("address"));
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
        self.finish_with_note();
    }

    fn unpublished(&mut self) {
        let t = self.format_authors();
        self.out.output(t);
        self.out.new_block();
        let t = self.format_title();
        self.out.output(t);
        self.out.new_block();
        let t = self.note();
        self.out.output(t);
        let t = self.format_date();
        self.out.output(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_joined_like_plain() {
        assert_eq!(format_names("Donald E. Knuth"), "Donald~E. Knuth");
        assert_eq!(
            format_names("Michel Goossens and Frank Mittelbach and Alexander Samarin"),
            "Michel Goossens, Frank Mittelbach, and Alexander Samarin"
        );
        assert_eq!(
            format_names("A. Smith and B. Jones"),
            "A.~Smith and B.~Jones"
        );
        assert_eq!(format_names("A. Smith and others"), "A.~Smith et~al.");
        assert_eq!(
            format_names("A. Smith and B. Jones and others"),
            "A.~Smith, B.~Jones, et~al."
        );
    }

    #[test]
    fn dashify_and_pages() {
        assert_eq!(n_dashify("1-2"), "1--2");
        assert_eq!(n_dashify("1--2"), "1--2");
        assert_eq!(n_dashify("1---2"), "1---2");
        assert_eq!(n_dashify("12"), "12");
        assert!(multi_page("1-2"));
        assert!(multi_page("1,3"));
        assert!(!multi_page("12"));
    }
}
