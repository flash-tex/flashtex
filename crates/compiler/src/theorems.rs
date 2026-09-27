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
//! shares another's (`\newtheorem{name}[shared]{Title}`), and resets on the
//! parent counter when declared `\newtheorem{name}{Title}[within]` — any
//! counter the general counter table (`crate::xref::Counters`) already
//! tracks (`section`, `chapter`, ...). An unknown counter name is an honest
//! "No counter defined" diagnostic rather than a fabricated reset.

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
    /// Set by `\newtheorem{name}{Title}[within]`: `counter` resets to 0
    /// every time the `within` counter steps (or a counter it is numbered
    /// within steps, as `\@stpelt` cascades) and prints as
    /// `\the<within>.\arabic{counter}` rather than bare `<n>`. `None`
    /// numbers plainly.
    pub within_counter: Option<String>,
}

/// Zero every theorem counter scoped to `parent` or to a counter `parent`
/// zeroes when it steps; called when that counter steps (the parent itself
/// lives in the general counter table, so this only mirrors its reset onto
/// the separate per-theorem counters). The cascade matters: LaTeX's
/// `\@stpelt` recurses, so stepping `section` zeroes `subsection` and with
/// it every `\newtheorem{name}{Title}[subsection]` counter -- after a bare
/// `\section{B}` the next such lemma is "Lemma 2.0.1", not a continuation.
pub fn reset_within_counter(
    theorems: &HashMap<String, TheoremDef>,
    theorem_counters: &mut HashMap<String, u32>,
    counters: &crate::xref::Counters,
    parent: &str,
) {
    let reset = counters.step_reset_names(parent);
    for def in theorems.values().filter(|def| {
        def.within_counter
            .as_deref()
            .is_some_and(|within| reset.iter().any(|name| name == within))
    }) {
        theorem_counters.insert(def.counter.clone(), 0);
    }
}
