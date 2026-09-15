//! LaTeX counters for cross-references: `\newcounter{name}[within]`,
//! `\numberwithin`, `\counterwithin`/`\counterwithout`, `\refstepcounter`
//! and `\the<name>`.
//!
//! Shared by every numbered construct. Section headings, equations and
//! figure captions use it; any other numbered environment (theorems,
//! tables, ...) adopts it the same way: `define` the counter once, then call
//! `step` where LaTeX calls `\refstepcounter` and store the returned value
//! as the parser's current `\label` value (`P::current_counter`). Labels
//! themselves stay `Inline::Label` values resolved by
//! `layout::layout_converged`.
//!
//! A counter's reset list is LaTeX's `\cl@<parent>` (`\@addtoreset`): a step
//! resets every counter registered within it, transitively (`\@stpelt`). A
//! counter's `\the<name>` is a list of [`Piece`]s, so the kernel's and
//! amsmath's redefinitions (`\thesubsection` = `\thesection.\arabic{..}`,
//! `\numberwithin[\alph]`, `subequations`' `\theparentequation\alph{..}`) are
//! data rather than special cases.
//!
//! Counters live in a small `Vec` in definition order, so iteration and
//! therefore output are deterministic.

use std::collections::BTreeMap;

/// The document-level naming options and overrides from `cleveref`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleverefConfig {
    pub capitalise: bool,
    pub noabbrev: bool,
    pub names: BTreeMap<String, CleverefName>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CleverefName {
    pub singular: Option<String>,
    pub plural: Option<String>,
    pub capital_singular: Option<String>,
    pub capital_plural: Option<String>,
}

impl CleverefConfig {
    /// A configuration with no name table; allocates nothing. Only a
    /// placeholder while the real configuration is lent out.
    pub(crate) const fn empty() -> Self {
        Self {
            capitalise: false,
            noabbrev: false,
            names: BTreeMap::new(),
        }
    }
}

impl Default for CleverefConfig {
    fn default() -> Self {
        let mut names = BTreeMap::new();
        for (kind, singular, plural) in [
            ("equation", "Equation", "Equations"),
            ("figure", "Figure", "Figures"),
            ("table", "Table", "Tables"),
            ("page", "Page", "Pages"),
            ("part", "Part", "Parts"),
            ("chapter", "Chapter", "Chapters"),
            ("section", "Section", "Sections"),
            ("subsection", "Section", "Sections"),
            ("subsubsection", "Section", "Sections"),
            ("appendix", "Appendix", "Appendices"),
            ("item", "Item", "Items"),
            ("footnote", "Footnote", "Footnotes"),
        ] {
            names.insert(
                kind.to_string(),
                CleverefName {
                    capital_singular: Some(singular.to_string()),
                    capital_plural: Some(plural.to_string()),
                    ..CleverefName::default()
                },
            );
        }
        Self {
            capitalise: false,
            noabbrev: false,
            names,
        }
    }
}

impl CleverefConfig {
    pub fn set_options(&mut self, options: &str) {
        for option in options.split(',').map(str::trim) {
            match option {
                "capitalise" => self.capitalise = true,
                "noabbrev" => self.noabbrev = true,
                _ => {}
            }
        }
    }

    pub fn set_name(&mut self, kind: String, singular: String, plural: String, capital: bool) {
        let name = self.names.entry(kind).or_default();
        if capital {
            name.capital_singular = Some(singular.clone());
            name.capital_plural = Some(plural.clone());
            if name.singular.is_none() {
                name.singular = Some(if self.capitalise {
                    singular.clone()
                } else {
                    singular.to_lowercase()
                });
            }
            if name.plural.is_none() {
                name.plural = Some(if self.capitalise {
                    plural.clone()
                } else {
                    plural.to_lowercase()
                });
            }
        } else {
            name.singular = Some(singular.clone());
            name.plural = Some(plural.clone());
            if name.capital_singular.is_none() {
                name.capital_singular = Some(capitalize_first(&singular));
            }
            if name.capital_plural.is_none() {
                name.capital_plural = Some(capitalize_first(&plural));
            }
        }
    }
}

/// The default `cleveref` name for a label type. `capitalise` changes the
/// initial letter; `noabbrev` selects full equation/figure names.
pub fn cleveref_name(config: &CleverefConfig, kind: &str, plural: bool, capital: bool) -> String {
    if let Some(name) = configured_name(config, kind, plural, capital) {
        return name;
    }
    let canonical_kind = cleveref_kind(kind);
    if canonical_kind != kind {
        if let Some(name) = configured_name(config, canonical_kind, plural, capital) {
            return name;
        }
    }
    let kind = canonical_kind;
    let full = config.noabbrev;
    let (singular, plural_name) = match (kind, full) {
        ("equation", false) => ("eq.", "eqs."),
        ("figure", false) => ("fig.", "figs."),
        ("appendix", _) => ("appendix", "appendices"),
        ("section", _) => ("section", "sections"),
        ("table", _) => ("table", "tables"),
        ("item", _) => ("item", "items"),
        ("footnote", _) => ("footnote", "footnotes"),
        ("chapter", _) => ("chapter", "chapters"),
        ("part", _) => ("part", "parts"),
        (kind, _) => (kind, ""),
    };
    let name = if plural {
        if plural_name.is_empty() {
            let fallback = format!("{singular}s");
            return if capital || config.capitalise {
                capitalize_first(&fallback)
            } else {
                fallback
            };
        }
        plural_name.to_string()
    } else {
        singular.to_string()
    };
    if capital || config.capitalise {
        capitalize_first(&name)
    } else {
        name
    }
}

fn configured_name(
    config: &CleverefConfig,
    kind: &str,
    plural: bool,
    capital: bool,
) -> Option<String> {
    config.names.get(kind).and_then(|name| {
        match (capital, plural) {
            (true, true) => name.capital_plural.as_ref(),
            (true, false) => name.capital_singular.as_ref(),
            (false, true) => name.plural.as_ref(),
            (false, false) => name.singular.as_ref(),
        }
        .cloned()
    })
}

/// `cleveref` aliases subsections to the section name by default.
pub fn cleveref_kind(kind: &str) -> &str {
    match kind {
        "subsection" | "subsubsection" => "section",
        kind => kind,
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}

/// A counter's printed form (`\arabic`, `\alph`, `\Alph`, `\roman`,
/// `\Roman`; latex.ltx `\@arabic`, `\@alph`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberStyle {
    Arabic,
    AlphLower,
    AlphUpper,
    RomanLower,
    RomanUpper,
}

impl NumberStyle {
    /// The style named by a counter command (`arabic`, `\alph`, ...).
    pub fn from_command(name: &str) -> Option<NumberStyle> {
        match name.trim().trim_start_matches('\\') {
            "arabic" => Some(NumberStyle::Arabic),
            "alph" => Some(NumberStyle::AlphLower),
            "Alph" => Some(NumberStyle::AlphUpper),
            "roman" => Some(NumberStyle::RomanLower),
            "Roman" => Some(NumberStyle::RomanUpper),
            _ => None,
        }
    }

    /// `value` in this style. `\@alph` only covers 1..=26 (LaTeX stops with
    /// "Counter too large"); outside it, and for 0 in the letter and roman
    /// styles (which print nothing in LaTeX), the result is what LaTeX
    /// prints: empty for 0, arabic beyond the alphabet.
    pub fn format(self, value: u32) -> String {
        match self {
            NumberStyle::Arabic => value.to_string(),
            NumberStyle::AlphLower | NumberStyle::AlphUpper => match value {
                0 => String::new(),
                1..=26 => {
                    let base = if self == NumberStyle::AlphLower {
                        b'a'
                    } else {
                        b'A'
                    };
                    char::from(base + (value - 1) as u8).to_string()
                }
                _ => value.to_string(),
            },
            NumberStyle::RomanLower => roman(value),
            NumberStyle::RomanUpper => roman(value).to_uppercase(),
        }
    }
}

/// TeX's `\romannumeral` (lowercase; empty for 0).
fn roman(mut value: u32) -> String {
    const TABLE: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (n, text) in TABLE {
        while value >= n {
            out.push_str(text);
            value -= n;
        }
    }
    out
}

/// One piece of a `\the<name>` definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Literal text (`.`, or a frozen `\theparentequation`).
    Text(String),
    /// `\the<counter>`.
    The(String),
    /// `\arabic{counter}` and friends.
    Value(String, NumberStyle),
    /// `\ifnum \c@<counter>>\z@ <then>\fi`: the pieces are contributed
    /// only while that counter is non-zero. report.cls prints `\thefigure`
    /// this way, so a figure before the first `\chapter` is "1" rather than
    /// "0.1".
    IfPositive(String, Vec<Piece>),
}

/// One named counter.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Counter {
    name: String,
    value: u32,
    /// Indices of the counters whose step resets this one (`\@addtoreset`).
    reset_by: Vec<usize>,
    /// `\the<name>`.
    the: Vec<Piece>,
}

/// Why a counter command changed nothing (LaTeX's `\@nocounterr`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterError {
    /// "No counter '<name>' defined".
    NoCounter(String),
}

/// The document's counter table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counters {
    counters: Vec<Counter>,
}

/// Nesting limit for `\the<name>` pieces that name other counters, so a
/// cyclic definition cannot recurse forever (LaTeX would loop).
const THE_DEPTH: usize = 16;

impl Counters {
    /// article.cls: `section`, `subsection` numbered within `section`, and
    /// `subsubsection` numbered within `subsection` (`\thesubsection` is
    /// `\thesection.\arabic{subsection}`), plus the body counters every
    /// class defines ([`Counters::define_body_counters`]).
    pub fn article() -> Self {
        let mut counters = Counters::default();
        counters.define("section", None);
        counters.number_within("subsection", "section");
        counters.number_within("subsubsection", "subsection");
        counters.define_body_counters();
        counters
    }

    /// The class counters outside sectioning that article.cls leaves
    /// unreset: `equation`, `figure` and `table` (`\newcounter{equation}`
    /// etc., printed `\@arabic`), and amsmath's `parentequation` used by
    /// `subequations`.
    pub fn define_body_counters(&mut self) {
        for name in ["equation", "figure", "table", "parentequation"] {
            self.define(name, None);
        }
    }

    /// report.cls/book.cls: `chapter`, `section` numbered within it
    /// (`\thesection` is `\thechapter.\@arabic\c@section`), then
    /// `subsection` and `subsubsection` as in article.
    ///
    /// `equation`, `figure` and `table` are reset by `chapter`
    /// (`\@addtoreset`) and printed
    /// `\ifnum \c@chapter>\z@ \thechapter.\fi \@arabic\c@<name>` --
    /// note the guard, which [`Counters::counter_within`] (`\counterwithin`)
    /// does not have: a figure in the front matter of a report, before any
    /// `\chapter`, is "1", not "0.1".
    pub fn report() -> Self {
        let mut counters = Counters::default();
        counters.define("chapter", None);
        counters.number_within("section", "chapter");
        counters.number_within("subsection", "section");
        counters.number_within("subsubsection", "subsection");
        counters.define_body_counters();
        for name in ["equation", "figure", "table"] {
            let child = counters.index(name).expect("just defined");
            let chapter = counters.index("chapter").expect("just defined");
            counters.add_to_reset(child, chapter);
            counters.counters[child].the = within_chapter_pieces(name);
        }
        counters
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.counters
            .iter()
            .position(|counter| counter.name == name)
    }

    fn index_or_err(&self, name: &str) -> Result<usize, CounterError> {
        self.index(name)
            .ok_or_else(|| CounterError::NoCounter(name.to_string()))
    }

    /// Whether `\newcounter{name}` (or a class) defined `name`.
    pub fn exists(&self, name: &str) -> bool {
        self.index(name).is_some()
    }

    /// `\newcounter{name}[within]`: reset by `within`, printed as plain
    /// arabic. Returns false (changing nothing) when `name` already exists or
    /// `within` does not, mirroring LaTeX's errors for both cases.
    pub fn define(&mut self, name: &str, within: Option<&str>) -> bool {
        self.insert(name, within, false)
    }

    /// A new counter reset by `parent` and printed as
    /// `\the<parent>.\arabic{name}` (article's `subsection`). Returns false
    /// when `name` exists or `parent` does not; for an existing counter use
    /// [`Counters::numberwithin`].
    pub fn number_within(&mut self, name: &str, parent: &str) -> bool {
        self.insert(name, Some(parent), true)
    }

    fn insert(&mut self, name: &str, within: Option<&str>, prefixed: bool) -> bool {
        if self.index(name).is_some() {
            return false;
        }
        let reset_by = match within {
            Some(parent) => match self.index(parent) {
                Some(index) => vec![index],
                None => return false,
            },
            None => Vec::new(),
        };
        let the = if prefixed && within.is_some() {
            within_pieces(name, within.unwrap_or_default(), NumberStyle::Arabic)
        } else {
            vec![Piece::Value(name.to_string(), NumberStyle::Arabic)]
        };
        self.counters.push(Counter {
            name: name.to_string(),
            value: 0,
            reset_by,
            the,
        });
        true
    }

    /// amsmath `\numberwithin[style]{name}{parent}` (amsmath.sty v2.17:
    /// `\@addtoreset{name}{parent}` and `\xdef\the<name>{\the<parent>.
    /// \<style>{name}}`), on existing counters.
    pub fn numberwithin(
        &mut self,
        name: &str,
        parent: &str,
        style: NumberStyle,
    ) -> Result<(), CounterError> {
        let child = self.index_or_err(name)?;
        let parent_index = self.index_or_err(parent)?;
        self.add_to_reset(child, parent_index);
        self.counters[child].the = within_pieces(name, parent, style);
        Ok(())
    }

    /// LaTeX 2018-04 kernel `\counterwithin{name}{parent}`: reset by
    /// `parent`; unless starred, `\the<name>` becomes
    /// `\the<parent>.\arabic{name}`.
    pub fn counter_within(
        &mut self,
        name: &str,
        parent: &str,
        starred: bool,
    ) -> Result<(), CounterError> {
        let child = self.index_or_err(name)?;
        let parent_index = self.index_or_err(parent)?;
        self.add_to_reset(child, parent_index);
        if !starred {
            self.counters[child].the = within_pieces(name, parent, NumberStyle::Arabic);
        }
        Ok(())
    }

    /// `\counterwithout{name}{parent}`: no longer reset by `parent`; unless
    /// starred, `\the<name>` becomes `\arabic{name}`.
    pub fn counter_without(
        &mut self,
        name: &str,
        parent: &str,
        starred: bool,
    ) -> Result<(), CounterError> {
        let child = self.index_or_err(name)?;
        let parent_index = self.index_or_err(parent)?;
        self.counters[child].reset_by.retain(|&p| p != parent_index);
        if !starred {
            self.counters[child].the = vec![Piece::Value(name.to_string(), NumberStyle::Arabic)];
        }
        Ok(())
    }

    fn add_to_reset(&mut self, child: usize, parent: usize) {
        if child != parent && !self.counters[child].reset_by.contains(&parent) {
            self.counters[child].reset_by.push(parent);
        }
    }

    /// `\refstepcounter{name}`: increment, reset every counter numbered
    /// within it (transitively, as `\@stpelt` does), and return `\the<name>`.
    pub fn step(&mut self, name: &str) -> Option<String> {
        let index = self.index(name)?;
        self.counters[index].value += 1;
        self.reset_descendants(index, 0);
        self.the(name)
    }

    fn reset_descendants(&mut self, parent: usize, depth: usize) {
        if depth > self.counters.len() {
            return;
        }
        for child in 0..self.counters.len() {
            if self.counters[child].reset_by.contains(&parent) {
                self.counters[child].value = 0;
                self.reset_descendants(child, depth + 1);
            }
        }
    }

    /// `\value{name}`.
    pub fn value(&self, name: &str) -> Option<u32> {
        self.index(name).map(|index| self.counters[index].value)
    }

    /// `\setcounter{name}{value}` (no resets, as in LaTeX).
    pub fn set_value(&mut self, name: &str, value: u32) -> bool {
        match self.index(name) {
            Some(index) => {
                self.counters[index].value = value;
                true
            }
            None => false,
        }
    }

    /// The current `\the<name>` definition.
    pub fn representation(&self, name: &str) -> Option<Vec<Piece>> {
        self.index(name)
            .map(|index| self.counters[index].the.clone())
    }

    /// `\def\the<name>{...}`.
    pub fn set_representation(&mut self, name: &str, the: Vec<Piece>) -> bool {
        match self.index(name) {
            Some(index) => {
                self.counters[index].the = the;
                true
            }
            None => false,
        }
    }

    /// `\the<name>`.
    pub fn the(&self, name: &str) -> Option<String> {
        self.index(name).map(|index| self.format(index, 0))
    }

    fn format(&self, index: usize, depth: usize) -> String {
        self.format_pieces(&self.counters[index].the, depth)
    }

    fn format_pieces(&self, pieces: &[Piece], depth: usize) -> String {
        let mut out = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::IfPositive(name, then) => {
                    if self.value(name).unwrap_or(0) > 0 && depth < THE_DEPTH {
                        out.push_str(&self.format_pieces(then, depth + 1));
                    }
                }
                Piece::The(name) => {
                    if let Some(other) = self.index(name).filter(|_| depth < THE_DEPTH) {
                        out.push_str(&self.format(other, depth + 1));
                    }
                }
                Piece::Value(name, style) => {
                    if let Some(value) = self.value(name) {
                        out.push_str(&style.format(value));
                    }
                }
            }
        }
        out
    }
}

/// `\the<parent>.\<style>{name}`.
/// report.cls/book.cls `\the<name>` for a body counter:
/// `\ifnum \c@chapter>\z@ \thechapter.\fi \@arabic\c@<name>`.
fn within_chapter_pieces(name: &str) -> Vec<Piece> {
    vec![
        Piece::IfPositive(
            "chapter".to_string(),
            vec![
                Piece::The("chapter".to_string()),
                Piece::Text(".".to_string()),
            ],
        ),
        Piece::Value(name.to_string(), NumberStyle::Arabic),
    ]
}

fn within_pieces(name: &str, parent: &str, style: NumberStyle) -> Vec<Piece> {
    vec![
        Piece::The(parent.to_string()),
        Piece::Text(".".to_string()),
        Piece::Value(name.to_string(), style),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_prefixes_body_counters_with_the_chapter_only_once_one_exists() {
        // report.cls: `\thefigure` is
        // `\ifnum \c@chapter>\z@ \thechapter.\fi \@arabic\c@figure`,
        // so the guard matters in the front matter -- `\counterwithin`'s
        // unconditional `\thechapter.\arabic{figure}` would print "0.1".
        let mut counters = Counters::report();
        assert_eq!(counters.step("figure").as_deref(), Some("1"));
        assert_eq!(counters.step("figure").as_deref(), Some("2"));

        assert_eq!(counters.step("chapter").as_deref(), Some("1"));
        // Stepping `chapter` resets every body counter registered within it.
        assert_eq!(counters.step("figure").as_deref(), Some("1.1"));
        assert_eq!(counters.step("equation").as_deref(), Some("1.1"));
        assert_eq!(counters.step("table").as_deref(), Some("1.1"));
        // `\thesection` is unconditional in report.cls, unlike the three above.
        assert_eq!(counters.step("section").as_deref(), Some("1.1"));
        assert_eq!(counters.step("subsection").as_deref(), Some("1.1.1"));

        assert_eq!(counters.step("chapter").as_deref(), Some("2"));
        assert_eq!(counters.step("figure").as_deref(), Some("2.1"));
        assert_eq!(counters.step("section").as_deref(), Some("2.1"));
        // `\setcounter{chapter}{0}` puts the guard back (book.cls does this
        // for the front matter).
        assert!(counters.set_value("chapter", 0));
        assert_eq!(counters.step("figure").as_deref(), Some("2"));
    }

    #[test]
    fn article_sectioning_numbers_and_resets_transitively() {
        let mut counters = Counters::article();
        assert_eq!(counters.step("section").as_deref(), Some("1"));
        assert_eq!(counters.step("subsection").as_deref(), Some("1.1"));
        assert_eq!(counters.step("subsubsection").as_deref(), Some("1.1.1"));
        assert_eq!(counters.step("subsection").as_deref(), Some("1.2"));
        assert_eq!(counters.value("subsubsection"), Some(0));
        counters.step("subsubsection");
        assert_eq!(counters.step("section").as_deref(), Some("2"));
        assert_eq!(counters.value("subsection"), Some(0));
        assert_eq!(counters.value("subsubsection"), Some(0));
        assert_eq!(counters.step("subsubsection").as_deref(), Some("2.0.1"));
    }

    #[test]
    fn newcounter_within_resets_but_prints_plain_arabic() {
        let mut counters = Counters::article();
        assert!(counters.define("theorem", Some("section")));
        assert!(!counters.define("theorem", None), "redefinition is refused");
        assert!(!counters.define("lemma", Some("missing")));
        counters.step("section");
        counters.step("theorem");
        assert_eq!(counters.step("theorem").as_deref(), Some("2"));
        counters.step("section");
        assert_eq!(counters.the("theorem").as_deref(), Some("0"));
        assert_eq!(counters.step("unknown"), None);
    }

    #[test]
    fn numberwithin_section_prefixes_and_resets_equations() {
        // \numberwithin{equation}{section}: (1.1), (1.2), then (2.1).
        let mut counters = Counters::article();
        counters
            .numberwithin("equation", "section", NumberStyle::Arabic)
            .unwrap();
        counters.step("section");
        assert_eq!(counters.step("equation").as_deref(), Some("1.1"));
        assert_eq!(counters.step("equation").as_deref(), Some("1.2"));
        counters.step("section");
        assert_eq!(counters.step("equation").as_deref(), Some("2.1"));
        // A subsection step does not reset equations.
        counters.step("subsection");
        assert_eq!(counters.step("equation").as_deref(), Some("2.2"));
        assert_eq!(
            counters.numberwithin("equation", "chapter", NumberStyle::Arabic),
            Err(CounterError::NoCounter("chapter".into()))
        );
    }

    #[test]
    fn numberwithin_takes_a_number_style() {
        let mut counters = Counters::article();
        counters
            .numberwithin("figure", "section", NumberStyle::AlphLower)
            .unwrap();
        counters.step("section");
        counters.step("section");
        assert_eq!(counters.step("figure").as_deref(), Some("2.a"));
        assert_eq!(counters.step("figure").as_deref(), Some("2.b"));
    }

    #[test]
    fn counterwithin_star_keeps_the_representation_and_counterwithout_undoes() {
        let mut counters = Counters::article();
        counters.counter_within("table", "section", true).unwrap();
        counters.step("section");
        counters.step("table");
        assert_eq!(counters.step("table").as_deref(), Some("2"));
        counters.step("section");
        assert_eq!(counters.step("table").as_deref(), Some("1"));
        counters.counter_within("table", "section", false).unwrap();
        assert_eq!(counters.the("table").as_deref(), Some("2.1"));
        counters.counter_without("table", "section", false).unwrap();
        counters.step("section");
        assert_eq!(counters.step("table").as_deref(), Some("2"));
    }

    #[test]
    fn subequations_representation_is_frozen_parent_plus_alph() {
        // amsmath \subequations: \theequation = \theparentequation\alph{equation}.
        let mut counters = Counters::article();
        assert_eq!(counters.step("equation").as_deref(), Some("1"));
        let parent = counters.step("equation").unwrap();
        let saved = counters.representation("equation").unwrap();
        let value = counters.value("equation").unwrap();
        counters.set_value("equation", 0);
        counters.set_representation(
            "equation",
            vec![
                Piece::Text(parent.clone()),
                Piece::Value("equation".into(), NumberStyle::AlphLower),
            ],
        );
        assert_eq!(counters.step("equation").as_deref(), Some("2a"));
        assert_eq!(counters.step("equation").as_deref(), Some("2b"));
        counters.set_value("equation", value);
        counters.set_representation("equation", saved);
        assert_eq!(counters.step("equation").as_deref(), Some("3"));
    }

    #[test]
    fn number_styles_follow_latex() {
        assert_eq!(NumberStyle::AlphUpper.format(3), "C");
        assert_eq!(NumberStyle::RomanLower.format(14), "xiv");
        assert_eq!(NumberStyle::RomanUpper.format(1999), "MCMXCIX");
        assert_eq!(NumberStyle::AlphLower.format(0), "");
        assert_eq!(
            NumberStyle::from_command("\\Alph"),
            Some(NumberStyle::AlphUpper)
        );
        assert_eq!(NumberStyle::from_command("fnsymbol"), None);
    }
}
