//! Literal UTF-8 input characters as pdfLaTeX reads them.
//!
//! LaTeX (since 2018, and identically under `\usepackage[utf8]{inputenc}`)
//! makes every non-ASCII character active; its meaning is the last
//! `\DeclareUnicodeCharacter` for it — the `omsenc/ot1enc/t1enc/ts1enc.dfu`
//! tables plus `utf8.def`'s own, all modelled by `flashtex-tex-text-encoding`.
//! Two things follow that a font's `cmap` knows nothing about:
//!
//! * A character nobody declared (Greek, Cyrillic, CJK, emoji, a combining
//!   accent such as U+0301) is `! LaTeX Error: Unicode character α (U+03B1)
//!   not set up for use with LaTeX.` and typesets nothing.
//! * A declared character whose text command the current encoding lacks
//!   (`«` is `\guillemetleft`, `þ` is `\th`, `ą` is `\k a`) is `! LaTeX Error:
//!   Command \guillemetleft unavailable in encoding OT1.`; nothing is typeset
//!   for a symbol, and the base letter alone for an accent (`\k a` sets `a`).
//!
//! And in OT1 an accented letter is not a character of the font: `\"u` is
//! `\accent` inside `\add@accent`'s group, `\L` a box, `°` a TS1 symbol in its
//! own group. None of those take part in the text font's ligature/kern
//! program with their neighbours (`W\"u` has no W–u kern), where T1's
//! precomposed `ü` (slot 252) does. [`cuts_ligkern`] says which.
//!
//! Every classification here is pinned against pdfLaTeX by
//! `tests/unicode_input_oracle.rs` (`fixtures/unicode-input/reference.json`).

use std::collections::BTreeSet;

use flashtex_tex_text_encoding::encoding::{self, Composite, Declared, Default, Encoding, Resolution};
use flashtex_tex_text_encoding::unicode;

/// What pdfLaTeX does with one rejected input character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    /// The LaTeX error, as pdfLaTeX logs it (without the leading `! `).
    pub message: String,
    /// What is still typeset in its place: the argument of an unavailable
    /// accent command (`\k a` → `a`), or nothing.
    pub keep: Option<char>,
}

/// The document-level input setup the checks run under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSetup {
    pub encoding: Encoding,
    /// Characters the preamble declares itself (`\DeclareUnicodeCharacter`,
    /// `\newunicodechar`): never rejected, never cut.
    pub extra: BTreeSet<char>,
}

impl InputSetup {
    /// The setup for `source`, or `None` when the document leaves the
    /// pdfLaTeX `utf8` + OT1/T1 world this module models: another input
    /// encoding, a Unicode engine (`fontspec`), or a package that declares
    /// characters wholesale (`fontenc` with `LGR`/`T2A`, `babel` Greek or
    /// Cyrillic, `CJKutf8`, `textgreek`, ...). No error is invented then.
    pub fn for_document(source: &str, packages: &[String], t1: bool) -> Option<InputSetup> {
        const DECLARING_PACKAGES: &[&str] = &[
            "fontspec", "xunicode", "unicode-math", "polyglossia", "xeCJK", "luatexja", "ctex", "CJK", "CJKutf8", "ucs",
            "textgreek", "alphabeta", "greek-inputenc", "substitutefont", "pmboxdraw", "tipa", "vntex", "inputenx",
        ];
        if packages.iter().any(|p| DECLARING_PACKAGES.contains(&p.as_str())) {
            return None;
        }
        let options = |name: &str| -> Vec<String> {
            crate::adapter::package_options(source, name)
                .map(|o| o.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default()
        };
        if packages.iter().any(|p| p == "inputenc") && options("inputenc").iter().any(|o| o != "utf8") {
            return None;
        }
        if options("fontenc").iter().any(|o| !matches!(o.as_str(), "OT1" | "T1" | "TS1")) {
            return None;
        }
        const NON_LATIN_BABEL: &[&str] = &[
            "greek", "polutonikogreek", "russian", "ukrainian", "bulgarian", "belarusian", "serbianc", "macedonian",
            "mongolian", "vietnamese", "hebrew", "arabic", "farsi",
        ];
        if options("babel").iter().any(|o| NON_LATIN_BABEL.contains(&o.as_str())) {
            return None;
        }
        Some(InputSetup {
            encoding: if t1 { Encoding::T1 } else { Encoding::OT1 },
            extra: preamble_declared(source),
        })
    }

    pub fn rejected(&self, c: char) -> Option<Rejected> {
        if self.extra.contains(&c) {
            return None;
        }
        rejected(c, self.encoding)
    }

    pub fn cuts_ligkern(&self, c: char) -> bool {
        !self.extra.contains(&c) && cuts_ligkern(c, self.encoding)
    }
}

/// `\DeclareUnicodeCharacter{<hex>}{...}` and `\newunicodechar{<char>}{...}`.
fn preamble_declared(source: &str) -> BTreeSet<char> {
    let mut out = BTreeSet::new();
    for (cmd, hex) in [("\\DeclareUnicodeCharacter", true), ("\\newunicodechar", false)] {
        let mut rest = source;
        while let Some(at) = rest.find(cmd) {
            rest = &rest[at + cmd.len()..];
            let Some(arg) = rest.trim_start().strip_prefix('{') else { continue };
            let Some(end) = arg.find('}') else { continue };
            let arg = arg[..end].trim();
            let c = if hex {
                u32::from_str_radix(arg, 16).ok().and_then(char::from_u32)
            } else {
                let mut chars = arg.chars();
                chars.next().filter(|_| chars.next().is_none())
            };
            out.extend(c);
        }
    }
    out
}

/// A dfu expansion's text command and its argument: `\@tabacckludge'e` →
/// (`\'`, `e`), `\k a` → (`\k`, `a`), `\k{}` → (`\k`, ``), `\th` → (`\th`, ``).
fn command_of(expansion: &str) -> Option<(String, String)> {
    let body = expansion.strip_prefix('\\')?;
    let (cmd, rest) = if let Some(after) = body.strip_prefix("@tabacckludge") {
        let accent = after.chars().next()?;
        (format!("\\{accent}"), &after[accent.len_utf8()..])
    } else {
        let letters = body.bytes().take_while(u8::is_ascii_alphabetic).count();
        if letters > 0 {
            (format!("\\{}", &body[..letters]), &body[letters..])
        } else {
            let c = body.chars().next()?;
            (format!("\\{c}"), &body[c.len_utf8()..])
        }
    };
    let arg: String = rest.chars().filter(|c| !matches!(c, ' ' | '{' | '}')).collect();
    Some((cmd, arg))
}

/// Whether `cmd` is an NFSS text command at all (declared for some encoding
/// or given a kernel default). `\nobreakspace`, `\-` and `\textellipsis`'s
/// relatives that are plain macros or primitives are not, and are never
/// "unavailable".
fn is_text_command(cmd: &str) -> bool {
    [Encoding::OT1, Encoding::T1, Encoding::TS1, Encoding::OMS, Encoding::OML]
        .iter()
        .any(|e| encoding::declared(*e, cmd).is_some())
        || encoding::kernel_default(cmd).is_some()
}

/// The error pdfLaTeX raises for input character `c` under text encoding
/// `enc`, or `None` when it typesets it.
pub fn rejected(c: char, enc: Encoding) -> Option<Rejected> {
    if c.is_ascii() {
        return None;
    }
    let Some((expansion, _)) = unicode::lookup_declared(c) else {
        return Some(Rejected { message: unicode::undeclared_message(c), keep: None });
    };
    let (cmd, arg) = command_of(expansion)?;
    if !is_text_command(&cmd) || encoding::resolve(enc, &cmd) != Resolution::Unavailable {
        return None;
    }
    let keep = {
        let mut chars = arg.chars();
        chars.next().filter(|b| b.is_ascii_alphabetic() && chars.next().is_none())
    };
    Some(Rejected { message: encoding::unavailable_message(enc, &cmd), keep })
}

/// Whether input character `c` stands outside the text font's ligature/kern
/// program in `enc`: it is typeset by `\accent` (no composite slot), as a box
/// (`\L` in OT1, `\r A`), or in another encoding's font (a TS1/OMS symbol).
/// A character that is a slot of `enc` (`ß`, `æ` in OT1; `ü` in T1) is not.
pub fn cuts_ligkern(c: char, enc: Encoding) -> bool {
    if c.is_ascii() {
        return false;
    }
    let Some((expansion, _)) = unicode::lookup_declared(c) else {
        return false;
    };
    let Some((cmd, arg)) = command_of(expansion) else {
        return false;
    };
    let composite_slot = || matches!(encoding::composite(enc, &cmd, &arg), Some(Composite::Slot(_)));
    match encoding::resolve(enc, &cmd) {
        Resolution::Declared(Declared::Symbol(_)) => false,
        Resolution::Declared(Declared::Accent(_) | Declared::Command { .. }) | Resolution::Default(Default::Accent(_)) => {
            !composite_slot()
        }
        Resolution::Default(Default::Symbol(other)) => other != enc,
        // Kernel macros (`\textellipsis` is `.\kern\fontdimen3\font` x3):
        // their first and last tokens are characters of the text font.
        Resolution::Default(Default::Command(_)) => false,
        Resolution::Unavailable => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansions_split_into_command_and_argument() {
        assert_eq!(command_of("\\@tabacckludge'e"), Some(("\\'".into(), "e".into())));
        assert_eq!(command_of("\\k a"), Some(("\\k".into(), "a".into())));
        assert_eq!(command_of("\\k{}"), Some(("\\k".into(), "".into())));
        assert_eq!(command_of("\\r A"), Some(("\\r".into(), "A".into())));
        assert_eq!(command_of("\\guillemetleft"), Some(("\\guillemetleft".into(), "".into())));
        assert_eq!(command_of("\\-"), Some(("\\-".into(), "".into())));
    }

    #[test]
    fn ot1_accents_cut_and_t1_composites_do_not() {
        assert!(cuts_ligkern('ü', Encoding::OT1));
        assert!(!cuts_ligkern('ü', Encoding::T1));
        assert!(cuts_ligkern('Ł', Encoding::OT1));
        assert!(!cuts_ligkern('Ł', Encoding::T1));
        assert!(!cuts_ligkern('ß', Encoding::OT1));
        assert!(!cuts_ligkern('æ', Encoding::OT1));
        assert!(cuts_ligkern('°', Encoding::T1));
        assert!(!cuts_ligkern('…', Encoding::OT1));
        assert!(!cuts_ligkern('“', Encoding::OT1));
    }

    #[test]
    fn preamble_declarations_are_read() {
        let s = "\\DeclareUnicodeCharacter{03B1}{\\alpha}\n\\newunicodechar{β}{\\beta}";
        let setup = InputSetup::for_document(s, &[], false).unwrap();
        assert!(setup.rejected('α').is_none() && setup.rejected('β').is_none());
        assert!(setup.rejected('γ').is_some());
        assert!(InputSetup::for_document("\\usepackage[LGR,T1]{fontenc}", &["fontenc".into()], true).is_none());
        assert!(InputSetup::for_document("\\usepackage[latin1]{inputenc}", &["inputenc".into()], false).is_none());
        assert!(InputSetup::for_document("\\usepackage[utf8]{inputenc}", &["inputenc".into()], false).is_some());
    }
}
