//! UTF-8 input (LaTeX's default since 2018; identical with `[utf8]{inputenc}`).
//!
//! `utf8.def` makes every non-ASCII character active; its meaning is whatever
//! the last `\DeclareUnicodeCharacter` said. A default pdflatex format loads
//! `omsenc.dfu`, `ot1enc.dfu`, `t1enc.dfu`, `ts1enc.dfu` (in that order,
//! verified in `texmf-var/web2c/pdftex/pdflatex.log`) and then utf8.def's own
//! declarations (utf8.def:360–370). Declarations are global, so the mapping does
//! not depend on the current font encoding — whether the resulting command is
//! available in that encoding is decided later (see [`crate::encoding`]).
//!
//! Loading further encodings with `fontenc` (e.g. `T2A`, `LGR`) would add their
//! `.dfu` files; that is out of scope and reported as undeclared here.

use crate::generated::UNICODE_DECLARATIONS;

/// Outcome of looking up one input character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputChar {
    /// ASCII: typeset by its character code (catcodes permitting).
    Ascii(u8),
    /// Declared: the LaTeX token list it expands to.
    Declared {
        expansion: &'static str,
        files: &'static [&'static str],
    },
    /// `! LaTeX Error: Unicode character X (U+XXXX) not set up for use with LaTeX.`
    Undeclared { message: String },
}

pub fn lookup_declared(c: char) -> Option<(&'static str, &'static [&'static str])> {
    let cp = c as u32;
    UNICODE_DECLARATIONS
        .binary_search_by_key(&cp, |e| e.0)
        .ok()
        .map(|i| (UNICODE_DECLARATIONS[i].1, UNICODE_DECLARATIONS[i].2))
}

/// The error text pdflatex prints (without the leading `! `).
pub fn undeclared_message(c: char) -> String {
    format!(
        "LaTeX Error: Unicode character {} (U+{:04X}) not set up for use with LaTeX.",
        c, c as u32
    )
}

pub fn classify(c: char) -> InputChar {
    if c.is_ascii() {
        return InputChar::Ascii(c as u8);
    }
    match lookup_declared(c) {
        Some((expansion, files)) => InputChar::Declared { expansion, files },
        None => InputChar::Undeclared {
            message: undeclared_message(c),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_and_covers_known_characters() {
        assert!(UNICODE_DECLARATIONS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(lookup_declared('é').unwrap().0, "\\@tabacckludge'e");
        assert_eq!(lookup_declared('•').unwrap().0, "\\textbullet");
        assert_eq!(lookup_declared('…').unwrap().0, "\\textellipsis");
        assert!(lookup_declared('Ж').is_none());
    }
}
