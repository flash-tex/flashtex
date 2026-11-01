//! LaTeX font encodings and encoding-specific text commands (NFSS `\DeclareText*`).
//!
//! Resolution of a text command `\cmd` under current encoding `E` follows
//! latex.ltx's `\@changed@cmd` / `\DeclareTextCommandDefault` machinery:
//! 1. the `E`-specific declaration (from `<e>enc.def`, minus later
//!    `\UndeclareTextCommand{\cmd}{E}` in latex.ltx), else
//! 2. the kernel default (`?` encoding; last `\DeclareText*Default` wins), else
//! 3. `LaTeX Error: Command \cmd unavailable in encoding E.`
//!
//! Composites (`\DeclareTextComposite{\acc}{E}{base}{slot}`) are consulted
//! when the command is expanded with an argument whose string equals `base`.

use crate::generated::{self, DeclKind, KERNEL_TEXT_DEFAULTS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Encoding {
    OT1,
    T1,
    TS1,
    OMS,
    OML,
}

impl Encoding {
    pub fn name(self) -> &'static str {
        match self {
            Encoding::OT1 => "OT1",
            Encoding::T1 => "T1",
            Encoding::TS1 => "TS1",
            Encoding::OMS => "OMS",
            Encoding::OML => "OML",
        }
    }

    pub fn from_name(s: &str) -> Option<Encoding> {
        Some(match s {
            "OT1" => Encoding::OT1,
            "T1" => Encoding::T1,
            "TS1" => Encoding::TS1,
            "OMS" => Encoding::OMS,
            "OML" => Encoding::OML,
            _ => return None,
        })
    }

    fn decls(self) -> &'static [generated::Decl] {
        match self {
            Encoding::OT1 => generated::OT1ENC,
            Encoding::T1 => generated::T1ENC,
            Encoding::TS1 => generated::TS1ENC,
            Encoding::OMS => generated::OMSENC,
            Encoding::OML => generated::OMLENC,
        }
    }
}

/// An encoding-specific declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declared {
    Symbol(u8),
    Accent(u8),
    /// `\DeclareTextCommand` with `args` parameters and the macro body.
    Command {
        args: u8,
        body: &'static str,
    },
}

/// A kernel default for encodings without their own declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Default {
    Symbol(Encoding),
    Accent(Encoding),
    Command(&'static str),
}

/// What `\cmd` means in encoding `E`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Declared(Declared),
    Default(Default),
    Unavailable,
}

fn parse_slot(s: &str) -> u8 {
    s.parse::<u16>().expect("generated slot") as u8
}

/// The `E`-specific declaration, honouring latex.ltx's `\UndeclareTextCommand`s.
pub fn declared(enc: Encoding, cmd: &str) -> Option<Declared> {
    let undeclared = KERNEL_TEXT_DEFAULTS
        .iter()
        .any(|(_, kind, c, e)| *kind == "UndeclareTextCommand" && *c == cmd && *e == enc.name());
    if undeclared {
        return None;
    }
    enc.decls()
        .iter()
        .rev()
        .find_map(|(kind, c, _e, extra, payload)| {
            if *c != cmd {
                return None;
            }
            match kind {
                DeclKind::Symbol => Some(Declared::Symbol(parse_slot(payload))),
                DeclKind::Accent => Some(Declared::Accent(parse_slot(payload))),
                DeclKind::Command => Some(Declared::Command {
                    args: extra.parse().unwrap_or(0),
                    body: payload,
                }),
                _ => None,
            }
        })
}

/// The kernel default (last declaration in latex.ltx order).
pub fn kernel_default(cmd: &str) -> Option<Default> {
    KERNEL_TEXT_DEFAULTS
        .iter()
        .rev()
        .find_map(|(_, kind, c, extra)| {
            if *c != cmd {
                return None;
            }
            match *kind {
                "DeclareTextSymbolDefault" => Encoding::from_name(extra).map(Default::Symbol),
                "DeclareTextAccentDefault" => Encoding::from_name(extra).map(Default::Accent),
                "DeclareTextCommandDefault" => Some(Default::Command(extra)),
                _ => None,
            }
        })
}

pub fn resolve(enc: Encoding, cmd: &str) -> Resolution {
    if let Some(d) = declared(enc, cmd) {
        return Resolution::Declared(d);
    }
    if let Some(d) = kernel_default(cmd) {
        return Resolution::Default(d);
    }
    Resolution::Unavailable
}

/// A composite for accent command `acc` applied to argument string `base`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Composite {
    Slot(u8),
    Command(&'static str),
}

pub fn has_composites(enc: Encoding, acc: &str) -> bool {
    enc.decls().iter().any(|(k, c, ..)| {
        *c == acc && matches!(k, DeclKind::Composite | DeclKind::CompositeCommand)
    })
}

pub fn composite(enc: Encoding, acc: &str, base: &str) -> Option<Composite> {
    enc.decls().iter().rev().find_map(|(k, c, _e, b, payload)| {
        if *c != acc || *b != base {
            return None;
        }
        match k {
            DeclKind::Composite => Some(Composite::Slot(parse_slot(payload))),
            DeclKind::CompositeCommand => Some(Composite::Command(payload)),
            _ => None,
        }
    })
}

/// `LaTeX Error: Command \cmd unavailable in encoding E.`
pub fn unavailable_message(enc: Encoding, cmd: &str) -> String {
    format!(
        "LaTeX Error: Command {} unavailable in encoding {}.",
        cmd,
        enc.name()
    )
}

/// `\DeclareEncodingSubset{TS1}{family}{n}` values used by `\CheckEncodingSubset`
/// (ts1cmr.fd: cmr 0; latex.ltx:14701: lmr 1).
pub fn ts1_subset(family: &str) -> Option<u8> {
    match family {
        "cmr" => Some(0),
        "lmr" => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_declarations_defaults_and_errors() {
        assert_eq!(
            resolve(Encoding::OT1, "\\ss"),
            Resolution::Declared(Declared::Symbol(25))
        );
        assert_eq!(
            resolve(Encoding::T1, "\\ss"),
            Resolution::Declared(Declared::Symbol(255))
        );
        assert_eq!(
            resolve(Encoding::OT1, "\\'"),
            Resolution::Declared(Declared::Accent(19))
        );
        // latex.ltx:14411 \UndeclareTextCommand{\textsterling}{OT1}
        assert_eq!(
            resolve(Encoding::OT1, "\\textsterling"),
            Resolution::Default(Default::Symbol(Encoding::TS1))
        );
        assert_eq!(
            resolve(Encoding::T1, "\\textsterling"),
            Resolution::Declared(Declared::Symbol(191))
        );
        assert_eq!(
            resolve(Encoding::OT1, "\\textbackslash"),
            Resolution::Default(Default::Symbol(Encoding::OMS))
        );
        assert_eq!(resolve(Encoding::OT1, "\\k"), Resolution::Unavailable);
        assert_eq!(
            composite(Encoding::T1, "\\'", "e"),
            Some(Composite::Slot(233))
        );
        assert_eq!(
            composite(Encoding::T1, "\\'", "\\i"),
            Some(Composite::Slot(237))
        );
        assert_eq!(
            composite(Encoding::OT1, "\\.", "i"),
            Some(Composite::Slot(105))
        );
    }
}
