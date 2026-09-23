//! amsthm theorem-like environments (`\newtheorem`, `\theoremstyle`) and the
//! `plain`/`definition`/`remark` styles they select for a `\begin{name}...
//! \end{name}` body. Faithful to amsthm.sty: `plain` bolds the head and
//! italicises the body (theorem, lemma, corollary, proposition, ...),
//! `definition` bolds the head and leaves the body upright (definition,
//! example, ...), and `remark` italicises the head and leaves the body
//! upright (remark, note); `plain` is the default in force before the first
//! `\theoremstyle`. `proof` is a fixed environment (not registered through
//! `\newtheorem`) handled the same way in `parser.rs`: an italic "Proof."
//! head, an upright body, and a right-flushed "∎" placed with the existing
//! `\hfill` glue.
//!
//! Numbering advances a per-environment counter unless the environment
//! shares another's (`\newtheorem{name}[shared]{Title}`), and resets on
//! `\section` only when declared `\newtheorem{name}{Title}[section]` — the
//! only reset counter this compiler's counter model supports. Any other
//! counter name (`chapter`, `subsection`, ...) is an honest "recognised but
//! not implemented" diagnostic rather than a fabricated reset.

use std::collections::HashMap;

use crate::parser::TextStyle;

/// A theorem style as amsthm's `\newtheoremstyle` (or thmtools'
/// `\declaretheoremstyle`) defines it: the parts of `\th@<style>` that
/// change what the compiler emits for the head and the body. The vertical
/// skips (`#2`/`#3`) and the head separator (`#8`) are the render
/// pipeline's, which reads them from the same declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomTheoremStyle {
    /// `#6`, `\thm@headfont`, applied after `\normalfont`.
    pub head: TextStyle,
    /// `#4`, the body font declarations, applied after `\normalfont`.
    pub body: TextStyle,
    /// `\thm@notefont`: `None` is amsthm's `\fontseries\mddefault
    /// \upshape` over the head font (plain head spec only; a custom `#9`
    /// sets `#3` in the head font unless it changes the font itself).
    pub note: Option<TextStyle>,
    /// `#7`, `\thm@headpunct`, set in the head font (may be empty).
    pub punct: String,
    /// `#5`: the text of a nonzero indent (`\hbox to#5{}` in the head
    /// font), `None` for no indent.
    pub indent: Option<String>,
    /// `#8` is `\newline`: the body starts on a line of its own.
    pub newline: bool,
    /// `#9`, the head spec (`\thmname{#1}\thmnumber{ #2}\thmnote{ (#3)}`),
    /// as source text with `#1`/`#2`/`#3` still in it; `None` is
    /// `\thmhead@plain`.
    pub head_spec: Option<String>,
    /// thmtools `notebraces={(}{)}`: the note's delimiters in the plain head.
    pub note_braces: (String, String),
}

/// The style a `\newtheorem` records: one of amsthm's three, or a
/// declared one.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleRef {
    Builtin(TheoremStyle),
    Custom(Box<CustomTheoremStyle>),
}

impl Default for StyleRef {
    fn default() -> Self {
        StyleRef::Builtin(TheoremStyle::Plain)
    }
}

impl StyleRef {
    pub fn head_style(&self) -> TextStyle {
        match self {
            StyleRef::Builtin(style) => style.head_style(),
            StyleRef::Custom(custom) => custom.head,
        }
    }

    pub fn body_style(&self) -> TextStyle {
        match self {
            StyleRef::Builtin(style) => style.body_style(),
            StyleRef::Custom(custom) => custom.body,
        }
    }

    pub fn custom(&self) -> Option<&CustomTheoremStyle> {
        match self {
            StyleRef::Custom(custom) => Some(custom),
            StyleRef::Builtin(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TheoremStyle {
    #[default]
    Plain,
    Definition,
    Remark,
}

impl TheoremStyle {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(TheoremStyle::Plain),
            "definition" => Some(TheoremStyle::Definition),
            "remark" => Some(TheoremStyle::Remark),
            _ => None,
        }
    }

    /// The head run's style: bold for `plain`/`definition` ("**Theorem
    /// 1.**"), italic for `remark` ("*Remark 1.*").
    pub fn head_style(self) -> TextStyle {
        match self {
            TheoremStyle::Remark => TextStyle {
                italic: true,
                ..TextStyle::default()
            },
            TheoremStyle::Plain | TheoremStyle::Definition => TextStyle::BOLD,
        }
    }

    /// The body's default style: italic only for `plain`.
    pub fn body_style(self) -> TextStyle {
        match self {
            TheoremStyle::Plain => TextStyle {
                italic: true,
                ..TextStyle::default()
            },
            TheoremStyle::Definition | TheoremStyle::Remark => TextStyle::default(),
        }
    }
}

/// One `\newtheorem` registration.
#[derive(Debug, Clone)]
pub struct TheoremDef {
    pub title: String,
    /// The builtin style in force at the declaration (`plain` when a
    /// declared style was; see `spec`).
    pub style: TheoremStyle,
    /// The full style, builtin or declared (`\newtheoremstyle`).
    pub spec: StyleRef,
    /// `\swapnumbers` was in force at the declaration: the number leads.
    pub swap: bool,
    /// `\newtheorem*` defines an unnumbered environment.
    pub numbered: bool,
    /// The counter this environment advances: its own name, unless it
    /// shares another theorem's counter (`\newtheorem{name}[shared]{Title}`),
    /// in which case this is that theorem's counter name.
    pub counter: String,
    /// Set by `\newtheorem{name}{Title}[section]`: `counter` resets to 0 on
    /// every `\section` and prints as `<section>.<n>` rather than bare `<n>`.
    pub within_section: bool,
}

/// Zero every counter belonging to a `[section]`-scoped theorem; called when
/// `\section` advances its own counter.
pub fn reset_within_section(
    theorems: &HashMap<String, TheoremDef>,
    counters: &mut HashMap<String, u32>,
) {
    for def in theorems.values().filter(|def| def.within_section) {
        counters.insert(def.counter.clone(), 0);
    }
}
