//! Token type shared by the tangle stage and the Pascal parser.

use std::rc::Rc;

/// One token of the tangled Pascal program.
///
/// `Sec`/`SecEnd` are not Pascal at all: they carry the WEB section number
/// through to the Rust emitter so the generated code can be annotated with
/// `// §NNNN`, exactly as TANGLE writes `{NNN:}` / `{:NNN}` into `tex.p`.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Id(Rc<str>),
    /// Integer constant (decimal, `@'` octal, `@"` hex, numeric macro, or a
    /// preprocessed `"string"` that has become a string-pool number).
    Int(i64),
    Real(String),
    /// Pascal `'...'` string literal; contents with `''` already folded to `'`.
    Str(String),
    Op(&'static str),
    Sec(u32),
    SecEnd,
}

impl Tok {
    pub fn is_id(&self, s: &str) -> bool {
        matches!(self, Tok::Id(n) if &**n == s)
    }
    pub fn is_op(&self, s: &str) -> bool {
        matches!(self, Tok::Op(o) if *o == s)
    }
}

/// The operator spellings the WEB Pascal subset uses. Interning them as
/// `&'static str` keeps `Tok` cheap to clone.
pub const OPS: &[&str] = &[
    ":=", "<=", ">=", "<>", "==", "..", "+", "-", "*", "/", "(", ")", "[", "]", ",", ";", ":", ".",
    "=", "<", ">", "^", "#", "@", "$", "%", "&", "!", "?", "`", "\\", "|", "\"", "{", "}", "'", "_",
];

pub fn op(s: &str) -> &'static str {
    for o in OPS {
        if *o == s {
            return o;
        }
    }
    panic!("web2rust: unknown operator {s:?}");
}
