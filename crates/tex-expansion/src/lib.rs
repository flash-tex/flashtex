//! `flashtex-tex-expansion`: a faithful TeX/e-TeX/LaTeX macro-expansion
//! processor for FlashTeX.
//!
//! This crate implements the "front end" of TeX (TeXbook ch. 20, and the
//! `get_next`/`expand`/`macro_call`/conditional-processing sections of
//! *The TeX Program*): category codes and tokenization, the control
//! sequence table with grouping/save-stack semantics, `\def`/`\let`-style
//! macro expansion, `\expandafter`/`\noexpand`/`\csname`, TeX and e-TeX
//! conditionals, `\count`/`\dimen`/`\skip`/`\toks` registers, and a LaTeX
//! layer (`\newcommand`, `\newenvironment`, counters). It does **not**
//! typeset anything -- its output is a flat, fully expanded/executed
//! token stream with per-token source-span provenance, meant for
//! `crates/compiler`'s layout stage to consume. See `CONTRACT.md` for the
//! proposed adoption path.
//!
//! A handful of items (span/lexer accessors, `Meaning::Let`,
//! `ConditionalFrame::shape`, ...) are forward-looking public API surface
//! not yet exercised by this crate's own tests -- silenced here rather
//! than deleted, since the adoption work in CONTRACT.md is expected to
//! use them (source position queries for diagnostics, `\let` chain
//! introspection for future `\show`/`\meaning` work, etc).
#![allow(dead_code)]

mod catcode;
mod conditionals;
mod error;
mod expand;
mod incremental;
mod lexer;
mod macro_def;
mod prelude;
mod registers;
mod scopes;
mod span;
mod token;

pub use catcode::{CatCode, CatCodeTable};
pub use error::{is_output_limit, output_limit_message, Diagnostic, Limits, Severity};
pub use expand::{is_group_token, to_fnsymbol, BoxMeasurer, Checkpoint, DefaultBoxMeasurer, Engine, LabelRecord, Mode};
pub use incremental::{Edit, EditStats, IncrementalExpander};
pub use registers::{scale_internal_dimen, DefaultFontMetrics, FontMetrics, FontSwitch, Glue};
pub use span::Span;
pub use token::{Token, TokenKind};

/// Convenience entry point: expand a whole source string with default
/// limits and font metrics, returning the content-token stream plus any
/// diagnostics collected along the way.
pub struct ExpandResult {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
    pub labels: Vec<LabelRecord>,
}

pub fn expand_str(source: &str) -> ExpandResult {
    let mut engine = Engine::new(source);
    let tokens = engine.run();
    let diagnostics = engine.take_diagnostics();
    let labels = engine.take_labels();
    ExpandResult { tokens, diagnostics, labels }
}

/// Render a token stream back to a plain string (concatenating character
/// tokens; control sequences render as `\name `, matching how `\message`
/// would print them) -- mainly useful for tests and the oracle harness.
/// The null control sequence renders as `\ ` (a known simplification).
pub fn tokens_to_display_string(tokens: &[Token]) -> String {
    let mut out = String::new();
    for t in tokens {
        match &t.kind {
            TokenKind::Char(c, _) => out.push(*c),
            TokenKind::ControlSequence(name) => {
                // tex.web §262 `print_cs` with the default `\escapechar`
                // and catcodes: multi-letter names are always followed
                // by a space, one-character names only if a letter.
                out.push('\\');
                out.push_str(name);
                let mut cs = name.chars();
                let space = match (cs.next(), cs.next()) {
                    (Some(c), None) => c.is_ascii_alphabetic(),
                    _ => true,
                };
                if space {
                    out.push(' ');
                }
            }
            TokenKind::ActiveChar(c) => out.push(*c),
            TokenKind::Param(n) => {
                out.push('#');
                out.push_str(&n.to_string());
            }
            TokenKind::Eof => {}
        }
    }
    out
}
