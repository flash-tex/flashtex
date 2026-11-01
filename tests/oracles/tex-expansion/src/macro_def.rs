use crate::token::Token;

/// One element of a macro's parameter text (TeXbook ch. 20): either a
/// literal token that must match verbatim (delimited parameters), or a
/// parameter slot `#1`..`#9`.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamPart {
    Literal(Token),
    Param(u8),
}

/// One element of a macro's replacement text: a literal token, or a
/// reference to argument `#1..#9` substituted at call time.
#[derive(Debug, Clone, PartialEq)]
pub enum BodyPart {
    Literal(Token),
    Param(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacroFlags {
    /// `\long`: `\par` is allowed inside arguments (TeXbook p. 205).
    pub long: bool,
    /// `\outer`: the macro may not appear inside arguments, definitions,
    /// skipped conditional text, or `\uppercase`-style absorbed text.
    pub outer: bool,
    /// e-TeX `\protected`: not expanded in `\edef`/`\write`-style
    /// expand-only contexts.
    pub protected: bool,
    /// `#{` form: the last parameter is delimited by an immediately
    /// following `{`, which is *not* consumed (TeXbook p. 205).
    pub brace_delimited_last: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub params: Vec<ParamPart>,
    pub body: Vec<BodyPart>,
    pub flags: MacroFlags,
    /// Number of `#1..#9` parameters (highest index referenced).
    pub arity: u8,
}

impl MacroDef {
    pub fn param_count(&self) -> u8 {
        self.arity
    }

    pub fn simple(body: Vec<Token>) -> Self {
        MacroDef { params: Vec::new(), body: body.into_iter().map(BodyPart::Literal).collect(), flags: MacroFlags::default(), arity: 0 }
    }
}
