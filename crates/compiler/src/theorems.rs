//! amsthm theorem-like environments (`\newtheorem`, `\theoremstyle`) and the
//! `plain`/`definition`/`remark` styles they select for a `\begin{name}...
//! \end{name}` body. Faithful to amsthm.sty: `plain` bolds the head and
//! italicises the body (theorem, lemma, corollary, proposition, ...),
//! `definition` bolds the head and leaves the body upright (definition,
//! example, ...), and `remark` italicises the head and leaves the body
//! upright (remark, note); `plain` is the default in force before the first
//! `\theoremstyle`. `proof` is a fixed environment (not registered through
//! `\newtheorem`) handled the same way in `parser.rs`: an italic "Proof."
//! head, an upright body, and a right-flushed proof-end marker placed with the
//! existing `\hfill` glue.
//!
//! Numbering advances a per-environment counter unless the environment
//! shares another's (`\newtheorem{name}[shared]{Title}`), and resets on
//! `\section` only when declared `\newtheorem{name}{Title}[section]` — the
//! only reset counter this compiler's counter model supports. Any other
//! counter name (`chapter`, `subsection`, ...) is an honest "recognised but
//! not implemented" diagnostic rather than a fabricated reset.

use std::collections::HashMap;

use crate::parser::TextStyle;

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
    pub style: TheoremStyle,
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
