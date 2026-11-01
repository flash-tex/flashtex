use crate::catcode::CatCode;
use crate::span::Span;

/// A TeX token: either a character token (a character plus the catcode it
/// had when it was read, TeXbook ch. 7) or a control sequence (which
/// includes control symbols like `\%` and the control word with empty name
/// `\ ` as well as active characters, represented separately below).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Char(char, CatCode),
    ControlSequence(String),
    /// An active character behaves like a control sequence whose "name" is
    /// the single character (TeXbook ch. 7); kept distinct so `\string` and
    /// `\meaning` can render it correctly.
    ActiveChar(char),
    /// A parameter placeholder inside a macro body/replacement text
    /// (`#1`..`#9`), only valid inside stored macro bodies, never in an
    /// expanded output stream.
    Param(u8),
    /// Marks end-of-file / end of the current input level.
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    pub fn synthetic(kind: TokenKind) -> Self {
        Token { kind, span: Span::synthetic() }
    }

    pub fn cs_name(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::ControlSequence(name) => Some(name),
            _ => None,
        }
    }

    pub fn is_cs(&self, name: &str) -> bool {
        matches!(&self.kind, TokenKind::ControlSequence(n) if n == name)
    }

    pub fn char_cat(&self) -> Option<(char, CatCode)> {
        match self.kind {
            TokenKind::Char(c, cat) => Some((c, cat)),
            _ => None,
        }
    }

    /// The "csname" TeX would print for `\string`/`\meaning`/`\noexpand`
    /// purposes: control words get a leading backslash (`\foo`), control
    /// symbols/empty name get `\` + the symbol, active chars print bare.
    pub fn display_name(&self) -> String {
        match &self.kind {
            TokenKind::ControlSequence(name) => format!("\\{name}"),
            TokenKind::ActiveChar(c) => c.to_string(),
            TokenKind::Char(c, _) => c.to_string(),
            TokenKind::Param(n) => format!("#{n}"),
            TokenKind::Eof => String::new(),
        }
    }

    /// This is just the *token identity* check (same char+cat, or same cs
    /// name) -- full `\ifx` meaning-level equality (macro bodies etc.) is
    /// handled in `conditionals.rs` where the symbol table is available.
    pub fn same_token(&self, other: &Token) -> bool {
        self.kind == other.kind
    }
}
