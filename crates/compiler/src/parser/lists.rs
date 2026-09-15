//! List structure: which `\list`-based environments enclose a paragraph,
//! what each `\item`'s label is in source terms, and the enumitem keys in
//! force, parsed as keys rather than label text.
//!
//! Provenance (TeX Live 2026):
//! - `latex.ltx` 15848-15869 `\list`: every list environment advances
//!   `\@listdepth`, adds its `\leftmargin` to `\@totalleftmargin` and sets
//!   `\parskip\parsep`, `\parindent\listparindent`.
//! - `latex.ltx` 15964-15966 `\item`: `\@ifnextchar[` (spaces skipped) takes
//!   an explicit label; without one `\@noitemargtrue` makes `\@item`
//!   `\refstepcounter\@listctr` (15979-16040), so an explicit label does not
//!   step the enumerate counter.
//! - `latex.ltx` 16052-16073 `\enumerate`/`\itemize`: `\@enumdepth` and
//!   `\@itemdepth` count only lists of the same kind and select
//!   `\labelenum<i>`/`\labelitem<i>`; both use `\makelabel` = `\hss\llap{#1}`.
//! - `article.cls` 344-358: `\theenumi`..`iv` are `\@arabic`, `\@alph`,
//!   `\@roman`, `\@Alph`; `\labelenumi` `\theenumi.`, `\labelenumii`
//!   `(\theenumii)`, `\labelenumiii` `\theenumiii.`, `\labelenumiv`
//!   `\theenumiv.`; `\labelitemi`..`iv` `\textbullet`,
//!   `\bfseries\textendash`, `\textasteriskcentered`, `\textperiodcentered`.
//! - `article.cls` 360-365 `description`: `\labelwidth\z@`,
//!   `\itemindent-\leftmargin`, `\makelabel` = `\descriptionlabel` =
//!   `\hspace\labelsep\normalfont\bfseries #1`.
//! - `article.cls` 389-410 `verse` (`\let\\\@centercr`, `\itemsep\z@`,
//!   `\itemindent -1.5em`, `\listparindent\itemindent`, `\rightmargin
//!   \leftmargin`, `\advance\leftmargin 1.5em`), `quotation`
//!   (`\listparindent 1.5em`, `\itemindent\listparindent`, `\rightmargin
//!   \leftmargin`, `\parsep \z@ \@plus\p@`) and `quote` (`\rightmargin
//!   \leftmargin`), each opened with `\item\relax`.
//! - `enumitem.sty` 244-254 (`topsep`/`itemsep`/`parsep`/`partopsep`),
//!   313-349 (`leftmargin`, `itemindent`, `listparindent`, `rightmargin`,
//!   `labelsep`), 366-432 (`series`, `resume`, `resume*`, `start` =
//!   `\setcounter{\@listctr}{#1}` then minus one), 490-527 (`align`,
//!   `label`, `label*` = the previous level's `\label<enum>` followed by
//!   this one), 735-743 (`nosep`: `\partopsep`, `\topsep`, `\itemsep`,
//!   `\parsep` all `\z@skip`; `noitemsep`: `\itemsep`, `\parsep`),
//!   1127-1157 (`\enit@endlist` saves the counter for `resume` under the
//!   environment name, and under the series name for `series=`), 1787-1795
//!   (`shortlabels`: a first option that is not a key is a label template).

use super::Inline;
use crate::Span;

/// One `\list`-based environment enclosing a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListEnvironment {
    Itemize,
    Enumerate,
    Description,
    /// `thebibliography` (article.cls 566-577: a `\list` of `\bibitem`s).
    Bibliography,
    Quote,
    Quotation,
    Verse,
}

impl ListEnvironment {
    pub fn from_name(name: &str) -> Option<ListEnvironment> {
        Some(match name {
            "itemize" => ListEnvironment::Itemize,
            "enumerate" => ListEnvironment::Enumerate,
            "description" => ListEnvironment::Description,
            "thebibliography" => ListEnvironment::Bibliography,
            "quote" => ListEnvironment::Quote,
            "quotation" => ListEnvironment::Quotation,
            "verse" => ListEnvironment::Verse,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            ListEnvironment::Itemize => "itemize",
            ListEnvironment::Enumerate => "enumerate",
            ListEnvironment::Description => "description",
            ListEnvironment::Bibliography => "thebibliography",
            ListEnvironment::Quote => "quote",
            ListEnvironment::Quotation => "quotation",
            ListEnvironment::Verse => "verse",
        }
    }

    /// `quote`, `quotation` and `verse`: one implicit `\item\relax`, no
    /// labels, paragraphs reported as `Block::Styled`.
    pub fn is_quote_like(self) -> bool {
        matches!(
            self,
            ListEnvironment::Quote | ListEnvironment::Quotation | ListEnvironment::Verse
        )
    }
}

/// One enclosing list environment of a paragraph. A block's `lists` holds
/// them outermost first, so its `\@listdepth` is `lists.len()` and the
/// frame at index `d - 1` supplies `\leftmargin<d>`.
#[derive(Debug, Clone, PartialEq)]
pub struct ListFrame {
    pub environment: ListEnvironment,
    /// Depth among lists of the same environment (1 = outermost):
    /// `\@itemdepth` for `itemize`, `\@enumdepth` for `enumerate`, which
    /// select `\labelitem<i>`/`\labelenum<i>`.
    pub kind_depth: u8,
    /// enumitem keys in force, in the order enumitem applies them: every
    /// matching `\setlist` (document order), then the `\begin` optional
    /// argument. Empty without enumitem keys.
    pub options: Vec<ListOption>,
    /// `\begin{<environment>}` through its optional argument.
    pub begin_span: Span,
}

impl ListFrame {
    /// `\@listdepth`-independent convenience: the last `leftmargin` key.
    pub fn leftmargin(&self) -> Option<ListLength> {
        self.options.iter().rev().find_map(|option| match option {
            ListOption::LeftMargin(length) => Some(*length),
            _ => None,
        })
    }

    /// The last `style` key (`nextline`, `sameline`, `multiline`,
    /// `unboxed`, `standard` or `normal`), verbatim.
    pub fn style(&self) -> Option<&str> {
        self.options.iter().rev().find_map(|option| match option {
            ListOption::Style(style) => Some(style.as_str()),
            _ => None,
        })
    }
}

/// An enumitem horizontal length value (`enumitem.sty` 260-349: `*`, `!`
/// or a dimension).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListLength {
    /// `*`: computed from the widest label.
    Star,
    /// `!`: computed from the other lengths.
    Bang,
    /// A dimension, in TeX points (`em`/`ex` at the class size).
    Pt(f64),
}

/// An enumitem vertical glue value, in TeX points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListSkip {
    pub pt: f64,
    pub plus: f64,
    pub minus: f64,
}

/// One enumitem key.
#[derive(Debug, Clone, PartialEq)]
pub enum ListOption {
    /// `label=<template>`: `\arabic*`, `\alph*`, `\Alph*`, `\roman*`,
    /// `\Roman*` stand for the counter.
    Label(String),
    /// `label*=<template>`: appended to the enclosing level's label.
    LabelStar(String),
    /// A `shortlabels` template (`[(a)]`): its first `a A i I 1` is the
    /// counter.
    ShortLabel(String),
    /// `start=<n>` (default 1).
    Start(i64),
    /// `resume` / `resume=<series>`.
    Resume(Option<String>),
    /// `resume*` / `resume*=<series>`: also re-applies the saved keys.
    ResumeStar(Option<String>),
    /// `series=<name>`.
    Series(String),
    LeftMargin(ListLength),
    RightMargin(ListLength),
    LabelSep(ListLength),
    LabelWidth(ListLength),
    LabelIndent(ListLength),
    ItemIndent(ListLength),
    ListParIndent(ListLength),
    TopSep(ListSkip),
    PartopSep(ListSkip),
    ItemSep(ListSkip),
    ParSep(ListSkip),
    /// `nosep`: `\partopsep`, `\topsep`, `\itemsep`, `\parsep` zero.
    NoSep,
    /// `noitemsep`: `\itemsep`, `\parsep` zero.
    NoItemSep,
    /// `align=left|right|parleft|<SetLabelAlign name>`.
    Align(String),
    /// `widest` / `widest=<text>`.
    Widest(Option<String>),
    /// `style=standard|normal|sameline|multiline|nextline|unboxed`
    /// (`enumitem.sty` `\enit@style@...`): kept verbatim; only `nextline`
    /// changes layout (the label takes a line of its own).
    Style(String),
    /// Any other enumitem key (`font`, `format`, `ref`, `before`, ...) or a
    /// recognised key whose value could not be read, kept verbatim.
    Other {
        key: String,
        value: Option<String>,
    },
}

/// How an `\item`'s label is produced.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemLabel {
    /// `\item[<label>]`: the argument as inline content (bold in
    /// `description`, `\descriptionlabel`), its plain text, and the span of
    /// the bracketed argument.
    Explicit {
        content: Vec<Inline>,
        text: String,
        span: Span,
    },
    /// article's `\labelitem<i>`: `command` is the text-symbol command
    /// (`textbullet`, `textendash`, `textasteriskcentered`,
    /// `textperiodcentered`), `bold` its `\bfseries`.
    Symbol {
        text: String,
        command: String,
        bold: bool,
    },
    /// A single-counter enumerate label: `\labelenum<i>` or an enumitem
    /// `label=`/shortlabels template with one counter. `value` already
    /// includes `start=`/`resume`.
    Counter {
        value: i64,
        style: CounterStyle,
        prefix: String,
        suffix: String,
        text: String,
    },
    /// Any other resolved label (several counters, `label*`, a template
    /// without a counter, a `\bibitem` label).
    Template { text: String },
    /// `description` (and every list without a label) with no `[<label>]`.
    Empty,
}

impl ItemLabel {
    pub fn text(&self) -> &str {
        match self {
            ItemLabel::Explicit { text, .. }
            | ItemLabel::Symbol { text, .. }
            | ItemLabel::Counter { text, .. }
            | ItemLabel::Template { text } => text,
            ItemLabel::Empty => "",
        }
    }
}

/// `\@arabic`, `\@alph`, `\@Alph`, `\@roman`, `\@Roman`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterStyle {
    Arabic,
    Alph,
    AlphUpper,
    Roman,
    RomanUpper,
}

impl CounterStyle {
    pub fn format(self, value: i64) -> String {
        match self {
            CounterStyle::Arabic => value.to_string(),
            CounterStyle::Alph => alph(value, b'a'),
            CounterStyle::AlphUpper => alph(value, b'A'),
            CounterStyle::Roman => roman(value),
            CounterStyle::RomanUpper => roman(value).to_uppercase(),
        }
    }

    /// enumitem's default `widest` value for this style (`enumitem.sty`
    /// `\enit@widest@<style>`: `m`, `M`, `viii`, `VIII`, `0`).
    pub fn widest(self) -> &'static str {
        match self {
            CounterStyle::Arabic => "0",
            CounterStyle::Alph => "m",
            CounterStyle::AlphUpper => "M",
            CounterStyle::Roman => "viii",
            CounterStyle::RomanUpper => "VIII",
        }
    }

    fn command(self) -> &'static str {
        match self {
            CounterStyle::Arabic => "\\arabic*",
            CounterStyle::Alph => "\\alph*",
            CounterStyle::AlphUpper => "\\Alph*",
            CounterStyle::Roman => "\\roman*",
            CounterStyle::RomanUpper => "\\Roman*",
        }
    }
}

/// `\@alph`: 1-26 only (LaTeX errors beyond; the number is kept).
fn alph(value: i64, base: u8) -> String {
    match value {
        1..=26 => char::from(base + (value - 1) as u8).to_string(),
        _ => value.to_string(),
    }
}

/// `\@roman` (`\romannumeral`: empty for values below 1).
fn roman(mut value: i64) -> String {
    const NUMERALS: [(i64, &str); 13] = [
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
    let mut text = String::new();
    for (step, numeral) in NUMERALS {
        while value >= step {
            text.push_str(numeral);
            value -= step;
        }
    }
    text
}

/// article.cls's default label for `environment` at `kind_depth`.
pub(crate) fn default_label(environment: ListEnvironment, kind_depth: u8, value: i64) -> ItemLabel {
    match environment {
        ListEnvironment::Itemize => {
            let (text, command, bold) = match kind_depth {
                0 | 1 => ("•", "textbullet", false),
                2 => ("–", "textendash", true),
                3 => ("∗", "textasteriskcentered", false),
                _ => ("⋅", "textperiodcentered", false),
            };
            ItemLabel::Symbol {
                text: text.to_string(),
                command: command.to_string(),
                bold,
            }
        }
        ListEnvironment::Enumerate => {
            let (style, prefix, suffix) = match kind_depth {
                0 | 1 => (CounterStyle::Arabic, "", "."),
                2 => (CounterStyle::Alph, "(", ")"),
                3 => (CounterStyle::Roman, "", "."),
                _ => (CounterStyle::AlphUpper, "", "."),
            };
            counter_label(value, style, prefix, suffix)
        }
        _ => ItemLabel::Empty,
    }
}

fn counter_label(value: i64, style: CounterStyle, prefix: &str, suffix: &str) -> ItemLabel {
    ItemLabel::Counter {
        value,
        style,
        prefix: prefix.to_string(),
        suffix: suffix.to_string(),
        text: format!("{prefix}{}{suffix}", style.format(value)),
    }
}

const COUNTER_STYLES: [CounterStyle; 5] = [
    CounterStyle::Alph,
    CounterStyle::AlphUpper,
    CounterStyle::Roman,
    CounterStyle::RomanUpper,
    CounterStyle::Arabic,
];

/// An enumitem `label=` template for counter `value`.
pub(crate) fn template_label(template: &str, value: i64) -> ItemLabel {
    let template = strip_outer_braces(template.trim());
    let found: Vec<CounterStyle> = COUNTER_STYLES
        .into_iter()
        .filter(|style| template.contains(style.command()))
        .collect();
    if let [style] = found.as_slice() {
        if template.matches(style.command()).count() == 1 {
            let (prefix, suffix) = template.split_once(style.command()).unwrap_or(("", ""));
            if !prefix.contains('\\') && !suffix.contains('\\') {
                return counter_label(value, *style, prefix, suffix);
            }
        }
    }
    ItemLabel::Template {
        text: COUNTER_STYLES
            .into_iter()
            .fold(template.to_string(), |text, style| {
                text.replace(style.command(), &style.format(value))
            }),
    }
}

/// A `shortlabels` template: the first `a A i I 1` is the counter
/// (`enumitem.sty` `\enit@shl`, `\enit@first`).
pub(crate) fn short_label(template: &str, value: i64) -> ItemLabel {
    match template.char_indices().find(|(_, c)| "aAiI1".contains(*c)) {
        Some((index, c)) => {
            let style = match c {
                'a' => CounterStyle::Alph,
                'A' => CounterStyle::AlphUpper,
                'i' => CounterStyle::Roman,
                'I' => CounterStyle::RomanUpper,
                _ => CounterStyle::Arabic,
            };
            counter_label(value, style, &template[..index], &template[index + 1..])
        }
        None => ItemLabel::Template {
            text: template.to_string(),
        },
    }
}

/// enumitem's own key names (`enumitem.sty` `\enitkv@key{}{...}`).
const KEYS: &[&str] = &[
    "topsep",
    "partopsep",
    "parsep",
    "itemsep",
    "leftmargin",
    "rightmargin",
    "listparindent",
    "labelwidth",
    "labelsep",
    "labelsep*",
    "labelindent",
    "labelindent*",
    "itemindent",
    "left",
    "widest",
    "widest*",
    "start",
    "resume",
    "resume*",
    "series",
    "align",
    "label",
    "label*",
    "ref",
    "font",
    "format",
    "before",
    "before*",
    "after",
    "after*",
    "nosep",
    "noitemsep",
    "wide",
    "style",
    "beginpenalty",
    "midpenalty",
    "endpenalty",
    "first",
    "first*",
    "mode",
    "itemjoin",
    "itemjoin*",
    "afterlabel",
    "fullwidth",
    "noitemsep",
    "halign",
];

/// Splits an option list on top-level commas (braces protect commas).
fn split_top_level(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn strip_outer_braces(value: &str) -> &str {
    let value = value.trim();
    if value.starts_with('{') && value.ends_with('}') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        let mut depth = 0i32;
        for c in inner.chars() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return value;
                    }
                }
                _ => {}
            }
        }
        if depth == 0 {
            return inner.trim();
        }
    }
    value
}

/// Whether the option list's first element is a key (otherwise it is a
/// `shortlabels` template).
fn is_key(part: &str) -> bool {
    let key = part.split_once('=').map_or(part, |(key, _)| key).trim();
    KEYS.contains(&key)
}

fn dimen_pt(value: &str, body_pt: f64) -> Option<f64> {
    let value = strip_outer_braces(value);
    match value {
        "0" | "\\z@" | "\\z@skip" => Some(0.0),
        _ => super::parse_dimen_pt_at(value, body_pt),
    }
}

fn length(value: &str, body_pt: f64) -> Option<ListLength> {
    match strip_outer_braces(value) {
        "*" => Some(ListLength::Star),
        "!" => Some(ListLength::Bang),
        other => dimen_pt(other, body_pt).map(ListLength::Pt),
    }
}

/// `<dimen> [plus <dimen>] [minus <dimen>]`; `fil` stretch is dropped.
fn skip(value: &str, body_pt: f64) -> Option<ListSkip> {
    let value = strip_outer_braces(value);
    let (natural, rest) = match value.find(" plus") {
        Some(at) => (&value[..at], Some(&value[at + " plus".len()..])),
        None => (value, None),
    };
    let (natural, minus_in_natural) = match natural.find(" minus") {
        Some(at) => (&natural[..at], Some(&natural[at + " minus".len()..])),
        None => (natural, None),
    };
    let pt = dimen_pt(natural, body_pt)?;
    let (plus, minus) = match rest {
        Some(rest) => match rest.find(" minus") {
            Some(at) => (Some(&rest[..at]), Some(&rest[at + " minus".len()..])),
            None => (Some(rest), None),
        },
        None => (None, minus_in_natural),
    };
    let finite = |part: Option<&str>| part.and_then(|p| dimen_pt(p, body_pt)).unwrap_or(0.0);
    Some(ListSkip {
        pt,
        plus: finite(plus),
        minus: finite(minus),
    })
}

/// Parses an enumitem option list (`\begin{..}[<here>]`, `\setlist{<here>}`).
/// `shortlabels`: a first element that is not a key is a label template.
pub(crate) fn parse_options(text: &str, body_pt: f64, allow_short_label: bool) -> Vec<ListOption> {
    let mut options = Vec::new();
    for (index, part) in split_top_level(text).into_iter().enumerate() {
        if index == 0 && allow_short_label && !is_key(part) && !part.contains('=') {
            options.push(ListOption::ShortLabel(strip_outer_braces(part).to_string()));
            continue;
        }
        let (key, value) = match part.split_once('=') {
            Some((key, value)) => (key.trim(), Some(value.trim())),
            None => (part, None),
        };
        let other = || ListOption::Other {
            key: key.to_string(),
            value: value.map(str::to_string),
        };
        let with_length = |make: fn(ListLength) -> ListOption| {
            value
                .and_then(|v| length(v, body_pt))
                .map_or_else(other, make)
        };
        let with_skip = |make: fn(ListSkip) -> ListOption| {
            value
                .and_then(|v| skip(v, body_pt))
                .map_or_else(other, make)
        };
        let name = || value.map(|v| strip_outer_braces(v).to_string());
        options.push(match key {
            "label" => value.map_or_else(other, |v| {
                ListOption::Label(strip_outer_braces(v).to_string())
            }),
            "label*" => value.map_or_else(other, |v| {
                ListOption::LabelStar(strip_outer_braces(v).to_string())
            }),
            "start" => match value {
                None => ListOption::Start(1),
                Some(v) => strip_outer_braces(v)
                    .parse()
                    .map_or_else(|_| other(), ListOption::Start),
            },
            "resume" => ListOption::Resume(name()),
            "resume*" => ListOption::ResumeStar(name()),
            "series" => value.map_or_else(other, |v| {
                ListOption::Series(strip_outer_braces(v).to_string())
            }),
            "leftmargin" => with_length(ListOption::LeftMargin),
            "rightmargin" => with_length(ListOption::RightMargin),
            "labelsep" => with_length(ListOption::LabelSep),
            "labelwidth" => with_length(ListOption::LabelWidth),
            "labelindent" => with_length(ListOption::LabelIndent),
            "itemindent" => with_length(ListOption::ItemIndent),
            "listparindent" => with_length(ListOption::ListParIndent),
            "topsep" => with_skip(ListOption::TopSep),
            "partopsep" => with_skip(ListOption::PartopSep),
            "itemsep" => with_skip(ListOption::ItemSep),
            "parsep" => with_skip(ListOption::ParSep),
            "nosep" if value.is_none_or(|v| v == "true") => ListOption::NoSep,
            "noitemsep" if value.is_none_or(|v| v == "true") => ListOption::NoItemSep,
            "align" => value.map_or_else(other, |v| {
                ListOption::Align(strip_outer_braces(v).to_string())
            }),
            "widest" => ListOption::Widest(name()),
            "style" => value.map_or_else(other, |v| {
                ListOption::Style(strip_outer_braces(v).to_string())
            }),
            _ => other(),
        });
    }
    options
}

/// A `\setlist[<names>]` target: environment names and level numbers.
#[derive(Debug, Clone, Default)]
pub(crate) struct SetlistTarget {
    pub names: Vec<String>,
    pub levels: Vec<u8>,
}

impl SetlistTarget {
    pub(crate) fn parse(text: &str) -> SetlistTarget {
        let mut target = SetlistTarget::default();
        for part in split_top_level(text) {
            match part.parse::<u8>() {
                Ok(level) => target.levels.push(level),
                Err(_) => target.names.push(part.to_string()),
            }
        }
        target
    }

    /// enumitem: names select environments (`itemize`, `enumerate`,
    /// `description`; none = all three), levels the environment's own
    /// depth when named, else `\@listdepth`.
    pub(crate) fn applies(
        &self,
        environment: ListEnvironment,
        kind_depth: u8,
        list_depth: u8,
    ) -> bool {
        if !matches!(
            environment,
            ListEnvironment::Itemize | ListEnvironment::Enumerate | ListEnvironment::Description
        ) {
            return false;
        }
        let named = self.names.is_empty() || self.names.iter().any(|n| n == environment.name());
        let level = if self.names.is_empty() {
            list_depth
        } else {
            kind_depth
        };
        named && (self.levels.is_empty() || self.levels.contains(&level))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_keys_not_label_text() {
        assert_eq!(
            parse_options("noitemsep", 10.0, true),
            vec![ListOption::NoItemSep]
        );
        assert_eq!(
            parse_options("(a), start=3, leftmargin=*", 10.0, true),
            vec![
                ListOption::ShortLabel("(a)".into()),
                ListOption::Start(3),
                ListOption::LeftMargin(ListLength::Star)
            ]
        );
        assert_eq!(
            parse_options(
                "label={(\\alph*)}, labelsep=1em, topsep=2pt plus 1pt",
                10.0,
                true
            ),
            vec![
                ListOption::Label("(\\alph*)".into()),
                ListOption::LabelSep(ListLength::Pt(10.0)),
                ListOption::TopSep(ListSkip {
                    pt: 2.0,
                    plus: 1.0,
                    minus: 0.0
                })
            ]
        );
        assert_eq!(
            parse_options("resume", 10.0, true),
            vec![ListOption::Resume(None)]
        );
    }

    #[test]
    fn labels_follow_article_and_enumitem() {
        assert_eq!(
            default_label(ListEnvironment::Enumerate, 2, 3).text(),
            "(c)"
        );
        assert_eq!(
            default_label(ListEnvironment::Enumerate, 3, 4).text(),
            "iv."
        );
        assert_eq!(default_label(ListEnvironment::Enumerate, 4, 2).text(), "B.");
        assert_eq!(default_label(ListEnvironment::Itemize, 2, 0).text(), "–");
        assert_eq!(template_label("(\\alph*)", 2).text(), "(b)");
        assert!(matches!(
            template_label("\\arabic*.\\alph*", 1),
            ItemLabel::Template { .. }
        ));
        assert_eq!(short_label("i)", 3).text(), "iii)");
    }
}
