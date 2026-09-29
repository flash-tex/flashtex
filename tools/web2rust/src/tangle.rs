//! TANGLE-equivalent stage: reads a WEB file and produces the fully expanded
//! Pascal token stream plus the string pool.
//!
//! This follows Knuth's `tangle.web` closely enough to be checkable against it:
//! `web2rust --emit-pascal` writes a normalised token dump that can be compared
//! against a token dump of the `tex.p` the real TANGLE produces.
//!
//! Deliberate deviations (all output-equivalent):
//!   * identifiers keep their WEB spelling (underscores and case) instead of
//!     being upper-cased and squashed, because the Rust we emit must keep every
//!     identifier;
//!   * numbers and string literals are lexed as whole tokens rather than being
//!     reassembled character by character in the output phase. `Int . Int` pairs
//!     are merged back into a real constant afterwards, which is what
//!     `float_constant(#) == #.0` needs.

use crate::tok::{op, Tok};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ctl {
    At,
    Octal,
    Hex,
    CheckSum,
    NewModule,
    Definition,
    Format,
    MetaBegin,
    MetaEnd,
    BeginPascal,
    ControlText,
    Join,
    ModuleName,
    Verbatim,
    ForceLine,
    Ignore,
}

fn control_code(c: u8) -> Ctl {
    match c {
        b'@' => Ctl::At,
        b'\'' => Ctl::Octal,
        b'"' => Ctl::Hex,
        b'$' => Ctl::CheckSum,
        b' ' | b'\t' | b'*' => Ctl::NewModule,
        b'D' | b'd' => Ctl::Definition,
        b'F' | b'f' => Ctl::Format,
        b'{' => Ctl::MetaBegin,
        b'}' => Ctl::MetaEnd,
        b'P' | b'p' => Ctl::BeginPascal,
        b'T' | b't' | b'^' | b'.' | b':' => Ctl::ControlText,
        b'&' => Ctl::Join,
        b'<' => Ctl::ModuleName,
        b'=' => Ctl::Verbatim,
        b'\\' => Ctl::ForceLine,
        _ => Ctl::Ignore,
    }
}

/// A token as stored in a replacement text: a real Pascal token, or one of the
/// things that only exist between reading and expansion.
#[derive(Clone, Debug, PartialEq)]
enum RTok {
    T(Tok),
    ModRef(usize),
    CheckSum,
    Join,
    MetaBegin,
    MetaEnd,
    Param,
    /// Start of the text contributed by WEB section `n`.
    SecBegin(u32),
    SecEnd,
}

#[derive(Debug)]
enum Macro {
    Numeric(i64),
    Simple(Rc<Vec<RTok>>),
    Parametric(Rc<Vec<RTok>>),
}

#[derive(Debug)]
enum Raw {
    Id(Rc<str>),
    Int(i64),
    Real(String),
    PStr(String),
    Op(&'static str),
    Ctl(Ctl),
    ModuleName(usize),
    Eof,
}

fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

struct Lex {
    lines: Vec<Vec<u8>>,
    line: usize,
    loc: usize,
    ended: bool,
    scanning_hex: bool,
}

impl Lex {
    fn new(src: &str) -> Lex {
        let lines: Vec<Vec<u8>> = src
            .split('\n')
            .map(|l| {
                let b = l.as_bytes();
                let mut end = b.len();
                while end > 0 && matches!(b[end - 1], b' ' | b'\t' | b'\r') {
                    end -= 1;
                }
                b[..end].to_vec()
            })
            .collect();
        Lex { lines, line: 0, loc: 0, ended: false, scanning_hex: false }
    }

    fn limit(&self) -> usize {
        self.lines[self.line].len()
    }
    fn at(&self, i: usize) -> u8 {
        let l = &self.lines[self.line];
        if i < l.len() {
            l[i]
        } else {
            b' '
        }
    }
    fn get_line(&mut self) -> bool {
        if self.line + 1 >= self.lines.len() {
            self.ended = true;
            return false;
        }
        self.line += 1;
        self.loc = 0;
        true
    }

    /// `skip_comment`: braces nest, `\` escapes the next character.
    fn skip_comment(&mut self) {
        let mut bal = 0i32;
        loop {
            while self.loc >= self.limit() {
                if !self.get_line() {
                    return;
                }
            }
            let c = self.at(self.loc);
            self.loc += 1;
            match c {
                b'@' => {
                    let d = self.at(self.loc);
                    self.loc += 1;
                    if control_code(d) == Ctl::NewModule {
                        self.loc -= 2;
                        return;
                    }
                }
                b'\\' if self.loc < self.limit() => self.loc += 1,
                b'{' => bal += 1,
                b'}' => {
                    if bal == 0 {
                        return;
                    }
                    bal -= 1;
                }
                _ => {}
            }
        }
    }

    /// `skip_ahead`: run to the next control code that is not ignorable.
    fn skip_ahead(&mut self) -> Ctl {
        loop {
            while self.loc >= self.limit() {
                if !self.get_line() {
                    return Ctl::NewModule;
                }
            }
            let lim = self.limit();
            let mut hit_at = false;
            while self.loc < lim {
                let c = self.at(self.loc);
                self.loc += 1;
                if c == b'@' {
                    hit_at = true;
                    break;
                }
            }
            if hit_at && self.loc < lim {
                let d = self.at(self.loc);
                self.loc += 1;
                let c = control_code(d);
                if c != Ctl::Ignore || d == b'>' {
                    return c;
                }
            }
        }
    }

    fn scan_module_name(&mut self) -> String {
        let mut s = String::new();
        loop {
            if self.loc >= self.limit() {
                if !self.get_line() {
                    break;
                }
                s.push(' ');
                continue;
            }
            let d = self.at(self.loc);
            if d == b'@' && self.at(self.loc + 1) == b'>' {
                self.loc += 2;
                break;
            }
            self.loc += 1;
            s.push(d as char);
        }
        normalise_name(&s)
    }

    /// After `@t`, `@^`, `@.`, `@:` — run to the closing `@>`.
    fn skip_control_text(&mut self) {
        loop {
            while self.loc >= self.limit() {
                if !self.get_line() {
                    return;
                }
            }
            let c = self.at(self.loc);
            self.loc += 1;
            if c == b'@' {
                let d = self.at(self.loc);
                self.loc += 1;
                if d == b'>' {
                    return;
                }
                if d == b'@' {
                    continue;
                }
            }
        }
    }

    fn text_between(&self, a: (usize, usize), b: (usize, usize)) -> String {
        let mut out = String::new();
        let (l0, c0) = a;
        let (l1, c1) = b;
        for l in l0..=l1.min(self.lines.len() - 1) {
            let line = &self.lines[l];
            let from = if l == l0 { c0.min(line.len()) } else { 0 };
            let to = if l == l1 { c1.min(line.len()) } else { line.len() };
            if from <= to {
                out.push_str(&String::from_utf8_lossy(&line[from..to]));
            }
            out.push('\n');
        }
        out
    }
}

pub fn normalise_name(s: &str) -> String {
    let mut out = String::new();
    let mut sp = false;
    for c in s.chars() {
        if c.is_whitespace() {
            sp = true;
        } else {
            if sp && !out.is_empty() {
                out.push(' ');
            }
            sp = false;
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Named-module table, with `...` prefix abbreviation
// ---------------------------------------------------------------------------

struct ModTable {
    names: Vec<String>,
    prefix: Vec<bool>,
    parts: Vec<Vec<RTok>>,
}

impl ModTable {
    fn new() -> ModTable {
        ModTable { names: vec![], prefix: vec![], parts: vec![] }
    }
    fn lookup(&mut self, raw: &str) -> usize {
        let (key, is_prefix) = match raw.strip_suffix("...") {
            Some(p) => (p.trim_end().to_string(), true),
            None => (raw.to_string(), false),
        };
        for i in 0..self.names.len() {
            let stored = self.names[i].clone();
            let hit = if is_prefix || self.prefix[i] {
                stored.starts_with(&key) || key.starts_with(&stored)
            } else {
                stored == key
            };
            if hit {
                if !is_prefix && (self.prefix[i] || key.len() > stored.len()) {
                    self.names[i] = key;
                    self.prefix[i] = false;
                }
                return i;
            }
        }
        self.names.push(key);
        self.prefix.push(is_prefix);
        self.parts.push(vec![]);
        self.names.len() - 1
    }
}

// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct Options {
    /// Define `stat`/`tats` as empty instead of `@{`/`@}`, i.e. compile the
    /// statistics code in (which is what web2c does).
    pub stat: bool,
    /// Likewise for `debug`/`gubed`.
    pub debug: bool,
    /// `@d name=value` numeric-macro overrides, applied as each definition is
    /// read so that later numeric macros see the new value. This is the other
    /// half of what a WEB change file does (see `--const` for Pascal
    /// constants); `mem_bot` for the trip test is the motivating case.
    pub macros: Vec<(String, i64)>,
}

pub struct Tangled {
    pub tokens: Vec<Tok>,
    /// WEB section number of each token in `tokens`.
    pub secs: Vec<u32>,
    /// Pool strings in the order TANGLE numbers them; index 0 is string 256.
    pub pool: Vec<Vec<u8>>,
    pub checksum: i64,
    /// WEB commentary (the TeX part) of each section; index 0 unused.
    pub comment: Vec<String>,
    /// Name of the module each section defines, if any.
    pub module_of: Vec<Option<String>>,
    pub n_sections: u32,
    /// WEB's numeric macros for `goto` labels (§6), so the Rust emitter can
    /// call them `'l_done` rather than `'l_L30`.
    pub label_macros: Vec<(String, i64)>,
}

const CHECK_SUM_PRIME: i64 = 0o3777777667;

struct Reader {
    lex: Lex,
    mods: ModTable,
    macros: HashMap<Rc<str>, Macro>,
    program: Vec<RTok>,
    pool: Vec<Vec<u8>>,
    pool_index: HashMap<Vec<u8>, i64>,
    checksum: i64,
    section: u32,
    comment: Vec<String>,
    module_of: Vec<Option<String>>,
    names: HashMap<String, Rc<str>>,
    macro_overrides: HashMap<String, i64>,
}

impl Reader {
    fn intern(&mut self, s: &str) -> Rc<str> {
        if let Some(r) = self.names.get(s) {
            return r.clone();
        }
        let r: Rc<str> = Rc::from(s);
        self.names.insert(s.to_string(), r.clone());
        r
    }

    /// Enter a preprocessed `"..."` string; returns its numeric value.
    fn pool_string(&mut self, raw: &[u8]) -> i64 {
        let mut content: Vec<u8> = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            let c = raw[i];
            content.push(c);
            if (c == b'"' || c == b'@') && i + 1 < raw.len() && raw[i + 1] == c {
                i += 2;
            } else {
                i += 1;
            }
        }
        if content.len() == 1 {
            return content[0] as i64;
        }
        if let Some(v) = self.pool_index.get(&content) {
            return *v;
        }
        let n = 256 + self.pool.len() as i64;
        let l = content.len() as i64;
        self.checksum = self.checksum + self.checksum + l;
        while self.checksum > CHECK_SUM_PRIME {
            self.checksum -= CHECK_SUM_PRIME;
        }
        for &b in &content {
            self.checksum = self.checksum + self.checksum + b as i64;
            while self.checksum > CHECK_SUM_PRIME {
                self.checksum -= CHECK_SUM_PRIME;
            }
        }
        self.pool_index.insert(content.clone(), n);
        self.pool.push(content);
        n
    }

    fn get_next(&mut self) -> Raw {
        loop {
            while self.lex.loc >= self.lex.limit() {
                if !self.lex.get_line() {
                    return Raw::Eof;
                }
            }
            let c = self.lex.at(self.lex.loc);
            self.lex.loc += 1;
            if self.lex.scanning_hex {
                self.lex.scanning_hex = false;
                if c.is_ascii_digit() || (b'A'..=b'F').contains(&c) {
                    let start = self.lex.loc - 1;
                    while self.lex.loc < self.lex.limit() {
                        let d = self.lex.at(self.lex.loc);
                        if d.is_ascii_digit() || (b'A'..=b'F').contains(&d) {
                            self.lex.loc += 1;
                        } else {
                            break;
                        }
                    }
                    let s = &self.lex.lines[self.lex.line][start..self.lex.loc];
                    let v = i64::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap();
                    return Raw::Int(v);
                }
            }
            match c {
                b'A'..=b'Z' | b'a'..=b'z' => {
                    let start = self.lex.loc - 1;
                    while self.lex.loc < self.lex.limit()
                        && is_ident_char(self.lex.at(self.lex.loc))
                    {
                        self.lex.loc += 1;
                    }
                    let s =
                        String::from_utf8_lossy(&self.lex.lines[self.lex.line][start..self.lex.loc])
                            .into_owned();
                    let r = self.intern(&s);
                    return Raw::Id(r);
                }
                b'0'..=b'9' => {
                    let start = self.lex.loc - 1;
                    while self.lex.loc < self.lex.limit()
                        && self.lex.at(self.lex.loc).is_ascii_digit()
                    {
                        self.lex.loc += 1;
                    }
                    let mut real = false;
                    if self.lex.loc < self.lex.limit()
                        && self.lex.at(self.lex.loc) == b'.'
                        && self.lex.at(self.lex.loc + 1).is_ascii_digit()
                    {
                        real = true;
                        self.lex.loc += 1;
                        while self.lex.loc < self.lex.limit()
                            && self.lex.at(self.lex.loc).is_ascii_digit()
                        {
                            self.lex.loc += 1;
                        }
                    }
                    if self.lex.loc < self.lex.limit()
                        && matches!(self.lex.at(self.lex.loc), b'e' | b'E')
                    {
                        let mut j = self.lex.loc + 1;
                        if matches!(self.lex.at(j), b'+' | b'-') {
                            j += 1;
                        }
                        if self.lex.at(j).is_ascii_digit() && j < self.lex.limit() {
                            real = true;
                            self.lex.loc = j;
                            while self.lex.loc < self.lex.limit()
                                && self.lex.at(self.lex.loc).is_ascii_digit()
                            {
                                self.lex.loc += 1;
                            }
                        }
                    }
                    let s =
                        String::from_utf8_lossy(&self.lex.lines[self.lex.line][start..self.lex.loc])
                            .into_owned();
                    return if real { Raw::Real(s) } else { Raw::Int(s.parse().unwrap()) };
                }
                b'"' => {
                    let start = self.lex.loc;
                    loop {
                        if self.lex.loc >= self.lex.limit() {
                            break;
                        }
                        let d = self.lex.at(self.lex.loc);
                        self.lex.loc += 1;
                        if (d == b'"' || d == b'@') && self.lex.at(self.lex.loc) == d {
                            self.lex.loc += 1;
                            continue;
                        }
                        if d == b'"' {
                            break;
                        }
                    }
                    let raw = self.lex.lines[self.lex.line][start..self.lex.loc - 1].to_vec();
                    let v = self.pool_string(&raw);
                    return Raw::Int(v);
                }
                b'\'' => {
                    let mut s = String::new();
                    loop {
                        if self.lex.loc >= self.lex.limit() {
                            break;
                        }
                        let d = self.lex.at(self.lex.loc);
                        self.lex.loc += 1;
                        if d == b'@' && self.lex.at(self.lex.loc) == b'@' {
                            self.lex.loc += 1;
                            s.push('@');
                            continue;
                        }
                        if d == b'\'' {
                            if self.lex.at(self.lex.loc) == b'\'' && self.lex.loc < self.lex.limit()
                            {
                                self.lex.loc += 1;
                                s.push('\'');
                                continue;
                            }
                            break;
                        }
                        s.push(d as char);
                    }
                    return Raw::PStr(s);
                }
                b'@' => {
                    let d = self.lex.at(self.lex.loc);
                    self.lex.loc += 1;
                    match control_code(d) {
                        Ctl::Ignore => continue,
                        Ctl::Hex => {
                            self.lex.scanning_hex = true;
                            continue;
                        }
                        Ctl::Octal => {
                            let start = self.lex.loc;
                            while self.lex.loc < self.lex.limit()
                                && (b'0'..=b'7').contains(&self.lex.at(self.lex.loc))
                            {
                                self.lex.loc += 1;
                            }
                            let s = &self.lex.lines[self.lex.line][start..self.lex.loc];
                            let v =
                                i64::from_str_radix(std::str::from_utf8(s).unwrap(), 8).unwrap();
                            return Raw::Int(v);
                        }
                        Ctl::ControlText => {
                            self.lex.skip_control_text();
                            continue;
                        }
                        Ctl::ModuleName => {
                            let name = self.lex.scan_module_name();
                            let idx = self.mods.lookup(&name);
                            return Raw::ModuleName(idx);
                        }
                        Ctl::At => return Raw::Op(op("@")),
                        Ctl::Verbatim => panic!("web2rust: @= verbatim is not supported"),
                        other => return Raw::Ctl(other),
                    }
                }
                b'{' => {
                    self.lex.skip_comment();
                    continue;
                }
                b'}' => continue,
                b' ' | b'\t' => continue,
                b'.' => {
                    if self.lex.loc < self.lex.limit() {
                        if self.lex.at(self.lex.loc) == b'.' {
                            self.lex.loc += 1;
                            return Raw::Op(op(".."));
                        }
                        if self.lex.at(self.lex.loc) == b')' {
                            self.lex.loc += 1;
                            return Raw::Op(op("]"));
                        }
                    }
                    return Raw::Op(op("."));
                }
                b':' => {
                    if self.lex.loc < self.lex.limit() && self.lex.at(self.lex.loc) == b'=' {
                        self.lex.loc += 1;
                        return Raw::Op(op(":="));
                    }
                    return Raw::Op(op(":"));
                }
                b'=' => {
                    if self.lex.loc < self.lex.limit() && self.lex.at(self.lex.loc) == b'=' {
                        self.lex.loc += 1;
                        return Raw::Op(op("=="));
                    }
                    return Raw::Op(op("="));
                }
                b'>' => {
                    if self.lex.loc < self.lex.limit() && self.lex.at(self.lex.loc) == b'=' {
                        self.lex.loc += 1;
                        return Raw::Op(op(">="));
                    }
                    return Raw::Op(op(">"));
                }
                b'<' => {
                    if self.lex.loc < self.lex.limit() {
                        if self.lex.at(self.lex.loc) == b'=' {
                            self.lex.loc += 1;
                            return Raw::Op(op("<="));
                        }
                        if self.lex.at(self.lex.loc) == b'>' {
                            self.lex.loc += 1;
                            return Raw::Op(op("<>"));
                        }
                    }
                    return Raw::Op(op("<"));
                }
                b'(' => {
                    if self.lex.loc < self.lex.limit() && self.lex.at(self.lex.loc) == b'.' {
                        self.lex.loc += 1;
                        return Raw::Op(op("["));
                    }
                    return Raw::Op(op("("));
                }
                _ => {
                    if c >= 128 {
                        continue;
                    }
                    return Raw::Op(op(&(c as char).to_string()));
                }
            }
        }
    }

    /// Collect a replacement text, following TANGLE's `scan_repl`. Returns the
    /// control code that ended it, plus the module index when that code is
    /// `ModuleName`. In a module text (`in_module_text`) a module name is a
    /// reference and `@d`/`@f`/`@p` are ignored; in a macro body all of them
    /// end the text.
    fn scan_repl(
        &mut self,
        parametric: bool,
        in_module_text: bool,
        out: &mut Vec<RTok>,
    ) -> (Ctl, Option<usize>) {
        loop {
            match self.get_next() {
                Raw::Eof => return (Ctl::NewModule, None),
                Raw::Id(n) => out.push(RTok::T(Tok::Id(n))),
                Raw::Int(v) => out.push(RTok::T(Tok::Int(v))),
                Raw::Real(s) => out.push(RTok::T(Tok::Real(s))),
                Raw::PStr(s) => out.push(RTok::T(Tok::Str(s))),
                Raw::Op("#") if parametric => out.push(RTok::Param),
                Raw::Op(o) => out.push(RTok::T(Tok::Op(o))),
                Raw::ModuleName(i) => {
                    if in_module_text {
                        out.push(RTok::ModRef(i));
                    } else {
                        return (Ctl::ModuleName, Some(i));
                    }
                }
                Raw::Ctl(c) => match c {
                    Ctl::CheckSum => out.push(RTok::CheckSum),
                    Ctl::Join => out.push(RTok::Join),
                    Ctl::MetaBegin => out.push(RTok::MetaBegin),
                    Ctl::MetaEnd => out.push(RTok::MetaEnd),
                    Ctl::ForceLine | Ctl::Ignore | Ctl::At => {}
                    Ctl::Definition | Ctl::Format | Ctl::BeginPascal if in_module_text => {}
                    other => return (other, None),
                },
            }
        }
    }

    fn scan_numeric(&mut self, name: &str) -> (i64, Option<(Ctl, Option<usize>)>) {
        let mut acc: i64 = 0;
        let mut sign: i64 = 1;
        loop {
            match self.get_next() {
                Raw::Op("+") => sign = 1,
                Raw::Op("-") => sign = -1,
                Raw::Int(v) => {
                    acc += sign * v;
                    sign = 1;
                }
                Raw::Id(n) => {
                    let v = match self.macros.get(&n) {
                        Some(Macro::Numeric(v)) => *v,
                        _ => panic!("web2rust: `{n}` in numeric macro `{name}` is not numeric"),
                    };
                    acc += sign * v;
                    sign = 1;
                }
                Raw::Op(";") => return (acc, None),
                Raw::Eof => return (acc, Some((Ctl::NewModule, None))),
                Raw::Ctl(c) => return (acc, Some((c, None))),
                Raw::ModuleName(i) => return (acc, Some((Ctl::ModuleName, Some(i)))),
                other => panic!("web2rust: unexpected {other:?} in numeric macro `{name}`"),
            }
        }
    }

    /// One WEB section: TeX part, definition part, Pascal part.
    fn scan_module(&mut self) {
        self.section += 1;
        let sec = self.section;
        while self.comment.len() <= sec as usize {
            self.comment.push(String::new());
            self.module_of.push(None);
        }
        let tex_start = (self.lex.line, self.lex.loc);
        let mut captured = false;
        // A milestone control code that is already in hand.
        let mut pending: Option<(Ctl, Option<usize>)> = None;
        let (ctl, mod_idx) = loop {
            let (c, idx) = match pending.take() {
                Some(p) => p,
                None => loop {
                    let c = self.lex.skip_ahead();
                    if c == Ctl::ModuleName {
                        // Re-read the name properly (handles `...` and `@@`).
                        self.lex.loc -= 2;
                        match self.get_next() {
                            Raw::ModuleName(i) => break (Ctl::ModuleName, Some(i)),
                            other => {
                                panic!("web2rust: §{sec}: expected module name, got {other:?}")
                            }
                        }
                    }
                    if matches!(c, Ctl::Definition | Ctl::BeginPascal | Ctl::NewModule) {
                        break (c, None);
                    }
                },
            };
            if !captured {
                let mut end = (self.lex.line, self.lex.loc);
                if end.1 >= 2 {
                    end.1 -= 2;
                }
                self.comment[sec as usize] = self.lex.text_between(tex_start, end);
                captured = true;
            }
            if c != Ctl::Definition {
                break (c, idx);
            }
            // `@d name = number` | `@d name == text` | `@d name(#) == text`
            let name = match self.get_next() {
                Raw::Id(n) => n,
                other => {
                    panic!("web2rust: §{sec}: @d must be followed by an identifier, got {other:?}")
                }
            };
            let term = match self.get_next() {
                Raw::Op("=") => {
                    let (v, t) = self.scan_numeric(&name);
                    let v = match self.macro_overrides.get(&*name) {
                        Some(o) => *o,
                        None => v,
                    };
                    self.macros.insert(name, Macro::Numeric(v));
                    t
                }
                Raw::Op("==") => {
                    let mut body = vec![];
                    let (t, i) = self.scan_repl(false, false, &mut body);
                    // A `--macro` override also replaces a simple macro whose
                    // body is just a number (`@d mem_top==30000`).
                    if let Some(o) = self.macro_overrides.get(&*name) {
                        body = vec![RTok::T(Tok::Int(*o))];
                    }
                    self.macros.insert(name, Macro::Simple(Rc::new(body)));
                    Some((t, i))
                }
                Raw::Op("(") => {
                    let a = self.get_next();
                    let b = self.get_next();
                    let c2 = self.get_next();
                    if !(matches!(a, Raw::Op("#"))
                        && matches!(b, Raw::Op(")"))
                        && matches!(c2, Raw::Op("==")))
                    {
                        panic!("web2rust: §{sec}: bad parametric macro `{name}`");
                    }
                    let mut body = vec![];
                    let (t, i) = self.scan_repl(true, false, &mut body);
                    self.macros.insert(name, Macro::Parametric(Rc::new(body)));
                    Some((t, i))
                }
                other => panic!("web2rust: §{sec}: bad @d for `{name}`: {other:?}"),
            };
            // `@f`, like anything at or below `format` in TANGLE's ordering,
            // means "keep skipping"; real milestones go to the next round.
            pending = match term {
                Some((Ctl::Format, _)) | None => None,
                other => other,
            };
        };
        match ctl {
            Ctl::BeginPascal => {
                let mut body = vec![RTok::SecBegin(sec)];
                self.scan_repl(false, true, &mut body);
                body.push(RTok::SecEnd);
                self.program.extend(body);
            }
            Ctl::ModuleName => {
                let idx = mod_idx.expect("module index");
                // `=` or `==` must follow the name.
                loop {
                    while self.lex.loc >= self.lex.limit() {
                        if !self.lex.get_line() {
                            return;
                        }
                    }
                    let e = self.lex.at(self.lex.loc);
                    if e == b' ' || e == b'\t' {
                        self.lex.loc += 1;
                        continue;
                    }
                    if e != b'=' {
                        panic!(
                            "web2rust: §{sec}: `=` must follow module name `{}`",
                            self.mods.names[idx]
                        );
                    }
                    self.lex.loc += 1;
                    if self.lex.loc < self.lex.limit() && self.lex.at(self.lex.loc) == b'=' {
                        self.lex.loc += 1;
                    }
                    break;
                }
                self.module_of[sec as usize] = Some(self.mods.names[idx].clone());
                let mut body = vec![RTok::SecBegin(sec)];
                self.scan_repl(false, true, &mut body);
                body.push(RTok::SecEnd);
                self.mods.parts[idx].extend(body);
            }
            _ => {}
        }
    }
}


// ---------------------------------------------------------------------------
// Expansion (TANGLE's phase II)
// ---------------------------------------------------------------------------

struct Frame {
    toks: Rc<Vec<RTok>>,
    pos: usize,
    param: Option<Rc<Vec<RTok>>>,
}

struct Expander {
    stack: Vec<Frame>,
    mod_text: Vec<Rc<Vec<RTok>>>,
    macros: HashMap<Rc<str>, Macro>,
    checksum: i64,
}

impl Expander {
    fn next_raw(&mut self) -> Option<RTok> {
        loop {
            let top = self.stack.last_mut()?;
            if top.pos < top.toks.len() {
                let t = top.toks[top.pos].clone();
                top.pos += 1;
                return Some(t);
            }
            self.stack.pop();
        }
    }
    fn cur_param(&self) -> Option<Rc<Vec<RTok>>> {
        for f in self.stack.iter().rev() {
            if let Some(p) = &f.param {
                return Some(p.clone());
            }
        }
        None
    }
    fn collect_arg(&mut self, name: &str) -> Rc<Vec<RTok>> {
        // Pop finished levels, as TANGLE does, then expect `(`.
        loop {
            match self.stack.last() {
                Some(f) if f.pos >= f.toks.len() && self.stack.len() > 1 => {
                    self.stack.pop();
                }
                _ => break,
            }
        }
        match self.next_raw() {
            Some(RTok::T(Tok::Op("("))) => {}
            other => panic!("web2rust: no parameter given for macro `{name}` (saw {other:?})"),
        }
        let mut depth = 1usize;
        let mut arg: Vec<RTok> = vec![];
        loop {
            let t = match self.next_raw() {
                Some(t) => t,
                None => panic!("web2rust: unterminated parameter for `{name}`"),
            };
            match &t {
                RTok::T(Tok::Op("(")) => {
                    depth += 1;
                    arg.push(t);
                }
                RTok::T(Tok::Op(")")) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    arg.push(t);
                }
                RTok::Param => {
                    if let Some(p) = self.cur_param() {
                        arg.extend(p.iter().cloned());
                    } else {
                        arg.push(t);
                    }
                }
                _ => arg.push(t),
            }
        }
        Rc::new(arg)
    }

    fn run(&mut self) -> (Vec<Tok>, Vec<u32>) {
        let mut out: Vec<Tok> = vec![];
        let mut secs: Vec<u32> = vec![];
        let mut sec_stack: Vec<u32> = vec![0];
        let mut meta = 0usize;
        let mut join = false;
        while let Some(t) = self.next_raw() {
            match t {
                RTok::SecBegin(n) => sec_stack.push(n),
                RTok::SecEnd => {
                    if sec_stack.len() > 1 {
                        sec_stack.pop();
                    }
                }
                RTok::MetaBegin => meta += 1,
                RTok::MetaEnd => meta = meta.saturating_sub(1),
                RTok::Join => join = true,
                RTok::CheckSum => {
                    push(&mut out, &mut secs, Tok::Int(self.checksum), *sec_stack.last().unwrap(), &mut join, meta == 0)
                }
                RTok::Param => {
                    let p = self.cur_param().expect("web2rust: `#` outside a parametric macro");
                    self.stack.push(Frame { toks: p, pos: 0, param: None });
                }
                RTok::ModRef(i) => {
                    let toks = self.mod_text[i].clone();
                    self.stack.push(Frame { toks, pos: 0, param: None });
                }
                RTok::T(Tok::Id(name)) => {
                    let kind = match self.macros.get(&name) {
                        None => 0,
                        Some(Macro::Numeric(v)) => {
                            let v = *v;
                            push(
                                &mut out,
                                &mut secs,
                                Tok::Int(v),
                                *sec_stack.last().unwrap(),
                                &mut join,
                                meta == 0,
                            );
                            continue;
                        }
                        Some(Macro::Simple(_)) => 1,
                        Some(Macro::Parametric(_)) => 2,
                    };
                    match kind {
                        1 => {
                            let body = match self.macros.get(&name) {
                                Some(Macro::Simple(b)) => b.clone(),
                                _ => unreachable!(),
                            };
                            self.stack.push(Frame { toks: body, pos: 0, param: None });
                        }
                        2 => {
                            let arg = self.collect_arg(&name);
                            let body = match self.macros.get(&name) {
                                Some(Macro::Parametric(b)) => b.clone(),
                                _ => unreachable!(),
                            };
                            self.stack.push(Frame { toks: body, pos: 0, param: Some(arg) });
                        }
                        _ => push(
                            &mut out,
                            &mut secs,
                            Tok::Id(name),
                            *sec_stack.last().unwrap(),
                            &mut join,
                            meta == 0,
                        ),
                    }
                }
                RTok::T(tk) => {
                    push(&mut out, &mut secs, tk, *sec_stack.last().unwrap(), &mut join, meta == 0)
                }
            }
        }
        let (mut out, mut secs) = fold_constants(out, secs);
        merge_reals(&mut out, &mut secs);
        (out, secs)
    }
}

/// Append a token, honouring a pending `@&` join.
fn push(
    out: &mut Vec<Tok>,
    secs: &mut Vec<u32>,
    t: Tok,
    sec: u32,
    join: &mut bool,
    keep: bool,
) {
    if !keep {
        // Inside `@{ ... @}` TANGLE writes a Pascal comment, so the tokens are
        // still expanded (`debug`/`gubed` are macros) but produce no code.
        *join = false;
        return;
    }
    if *join {
        *join = false;
        if let Some(last) = out.last_mut() {
            let merged = match (&*last, &t) {
                (Tok::Id(a), Tok::Id(b)) => Some(format!("{a}{b}")),
                (Tok::Id(a), Tok::Int(b)) => Some(format!("{a}{b}")),
                (Tok::Int(a), Tok::Id(b)) => Some(format!("{a}{b}")),
                _ => None,
            };
            if let Some(m) = merged {
                *last = Tok::Id(Rc::from(m.as_str()));
                return;
            }
        }
    }
    out.push(t);
    secs.push(sec);
}

/// TANGLE assembles real constants out of digits in its output phase; we lex
/// whole numbers, so `float_constant(20000)` arrives as `20000 . 0` and has to
/// be put back together.
fn merge_reals(out: &mut Vec<Tok>, secs: &mut Vec<u32>) {
    let mut i = 0;
    while i + 2 < out.len() {
        let is_real = matches!(
            (&out[i], &out[i + 1], &out[i + 2]),
            (Tok::Int(_), Tok::Op("."), Tok::Int(_))
        );
        if is_real {
            let a = match out[i] {
                Tok::Int(v) => v,
                _ => unreachable!(),
            };
            let b = match out[i + 2] {
                Tok::Int(v) => v,
                _ => unreachable!(),
            };
            out[i] = Tok::Real(format!("{a}.{b}"));
            out.drain(i + 1..i + 3);
            secs.drain(i + 1..i + 3);
        }
        i += 1;
    }
}

pub fn tangle(src: &str, opts: Options) -> Tangled {
    let mut r = Reader {
        lex: Lex::new(src),
        mods: ModTable::new(),
        macros: HashMap::new(),
        program: vec![],
        pool: vec![],
        pool_index: HashMap::new(),
        checksum: 271828,
        section: 0,
        comment: vec![String::new()],
        module_of: vec![None],
        names: HashMap::new(),
        macro_overrides: opts.macros.iter().cloned().collect(),
    };
    // Skip limbo.
    loop {
        if r.lex.skip_ahead() == Ctl::NewModule {
            break;
        }
        if r.lex.ended {
            break;
        }
    }
    while !r.lex.ended {
        r.scan_module();
    }

    if opts.stat {
        let s: Rc<str> = Rc::from("stat");
        let t: Rc<str> = Rc::from("tats");
        r.macros.insert(s, Macro::Simple(Rc::new(vec![])));
        r.macros.insert(t, Macro::Simple(Rc::new(vec![])));
    }
    if opts.debug {
        let s: Rc<str> = Rc::from("debug");
        let t: Rc<str> = Rc::from("gubed");
        r.macros.insert(s, Macro::Simple(Rc::new(vec![])));
        r.macros.insert(t, Macro::Simple(Rc::new(vec![])));
    }

    let n_sections = r.section;
    let checksum = r.checksum;
    let mod_text: Vec<Rc<Vec<RTok>>> =
        r.mods.parts.into_iter().map(Rc::new).collect();
    let program = Rc::new(std::mem::take(&mut r.program));
    let mut ex = Expander {
        stack: vec![Frame { toks: program, pos: 0, param: None }],
        mod_text,
        macros: r.macros,
        checksum,
    };
    let (tokens, secs) = ex.run();
    let mut label_macros: Vec<(String, i64)> = vec![];
    for name in LABEL_MACRO_NAMES {
        if let Some(Macro::Numeric(v)) = ex.macros.get(&Rc::from(*name) as &Rc<str>) {
            label_macros.push((name.to_string(), *v));
        }
    }
    Tangled {
        tokens,
        secs,
        pool: r.pool,
        checksum,
        comment: r.comment,
        module_of: r.module_of,
        n_sections,
        label_macros,
    }
}

/// The WEB macros §6 defines for `goto` targets. Their values come from
/// `tex.web` itself; only the set of names is fixed here.
const LABEL_MACRO_NAMES: &[&str] = &[
    "exit",
    "restart",
    "reswitch",
    "continue",
    "done",
    "done1",
    "done2",
    "done3",
    "done4",
    "done5",
    "done6",
    "found",
    "found1",
    "found2",
    "not_found",
    "not_found1",
    "common_ending",
    "start_of_TEX",
    "end_of_TEX",
    "final_end",
];

/// Write the string pool in TANGLE's `tex.pool` format.
pub fn write_pool(t: &Tangled) -> Vec<u8> {
    let mut out = Vec::new();
    for s in &t.pool {
        let l = s.len();
        out.push(b'0' + (l / 10) as u8);
        out.push(b'0' + (l % 10) as u8);
        out.extend_from_slice(s);
        out.push(b'\n');
    }
    out.push(b'*');
    let mut d = [0u8; 10];
    let mut cs = t.checksum;
    for i in 1..=9 {
        d[i] = (cs % 10) as u8;
        cs /= 10;
    }
    for i in (1..=9).rev() {
        out.push(b'0' + d[i]);
    }
    out.push(b'\n');
    out
}

// ---------------------------------------------------------------------------
// TANGLE's output state machine (constant folding)
// ---------------------------------------------------------------------------

// TANGLE does not just copy numbers through: `send_val` / `send_sign` keep a
// pending sign and value in the output buffer state and collapse runs of
// `number ± number` into one constant, which is why `mem_top-1` reaches `tex.p`
// as `29999`. That matters for us too: array bounds, `case` labels and `goto`
// labels have to be single constants in Rust as well. These are the states of
// `tangle.web` §§1781-1786, reproduced exactly.
const MISC: u8 = 0;
const NUM_OR_ID: u8 = 1;
const SIGN: u8 = 2;
const SIGN_VAL: u8 = 3;
const SIGN_VAL_SIGN: u8 = 4;
const SIGN_VAL_VAL: u8 = 5;
const UNBREAKABLE: u8 = 6;

/// Kind of an outgoing item, in TANGLE's `ident`/`frac` > `str`/`misc` order.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Ident,
    Frac,
    Misc,
}

struct Folder {
    out: Vec<Tok>,
    secs: Vec<u32>,
    state: u8,
    out_sign: u8, // 0, b' ' or b'+'
    out_val: i64,
    out_app: i64,
    last_sign: i64,
    sec: u32,
}

impl Folder {
    fn emit(&mut self, t: Tok) {
        self.out.push(t);
        self.secs.push(self.sec);
    }
    fn last_is_mul_like(&self) -> bool {
        match self.out.last() {
            Some(Tok::Op("*")) | Some(Tok::Op("/")) => true,
            Some(Tok::Id(n)) => {
                let n = n.to_ascii_lowercase();
                n == "div" || n == "mod"
            }
            _ => false,
        }
    }
    fn append_out_val(&mut self) {
        if self.out_val < 0 || (self.out_val == 0 && self.last_sign < 0) {
            self.emit(Tok::Op(op("-")));
        } else if self.out_sign == b'+' {
            self.emit(Tok::Op(op("+")));
        }
        let v = self.out_val.abs();
        self.emit(Tok::Int(v));
    }
    /// `@<Get the buffer ready for appending a new item@>`
    fn prepare(&mut self, k: Kind, mul_like: bool) {
        loop {
            match self.state {
                SIGN => {
                    let s = if self.out_app > 0 { "+" } else { "-" };
                    self.emit(Tok::Op(op(s)));
                    return;
                }
                SIGN_VAL | SIGN_VAL_SIGN => {
                    self.append_out_val();
                    self.state -= 2;
                }
                SIGN_VAL_VAL => {
                    if k == Kind::Frac || mul_like {
                        self.append_out_val();
                        self.out_sign = b'+';
                        self.out_val = self.out_app;
                    } else {
                        self.out_val += self.out_app;
                    }
                    self.state = SIGN_VAL;
                }
                _ => return, // NUM_OR_ID, MISC, UNBREAKABLE
            }
        }
    }
    fn send_out(&mut self, k: Kind, t: Tok) {
        let mul_like = matches!(&t, Tok::Op("*") | Tok::Op("/"))
            || matches!(&t, Tok::Id(n) if {
                let n = n.to_ascii_lowercase();
                n == "div" || n == "mod"
            });
        self.prepare(k, mul_like);
        self.emit(t);
        self.state = if k == Kind::Ident || k == Kind::Frac { NUM_OR_ID } else { MISC };
    }
    fn send_sign(&mut self, v: i64) {
        match self.state {
            SIGN | SIGN_VAL_SIGN => self.out_app *= v,
            SIGN_VAL => {
                self.out_app = v;
                self.state = SIGN_VAL_SIGN;
            }
            SIGN_VAL_VAL => {
                self.out_val += self.out_app;
                self.out_app = v;
                self.state = SIGN_VAL_SIGN;
            }
            _ => {
                self.out_app = v;
                self.state = SIGN;
            }
        }
        self.last_sign = self.out_app;
    }
    fn send_val(&mut self, v: i64) {
        match self.state {
            NUM_OR_ID | MISC if self.last_is_mul_like() => self.bad_case(v),
            NUM_OR_ID => {
                self.out_sign = b' ';
                self.state = SIGN_VAL;
                self.out_val = v;
                self.last_sign = 1;
            }
            MISC => {
                self.out_sign = 0;
                self.state = SIGN_VAL;
                self.out_val = v;
                self.last_sign = 1;
            }
            SIGN => {
                self.out_sign = b'+';
                self.state = SIGN_VAL;
                self.out_val = self.out_app * v;
            }
            SIGN_VAL => {
                self.state = SIGN_VAL_VAL;
                self.out_app = v;
            }
            SIGN_VAL_SIGN => {
                self.state = SIGN_VAL_VAL;
                self.out_app *= v;
            }
            SIGN_VAL_VAL => {
                self.out_val += self.out_app;
                self.out_app = v;
            }
            _ => self.bad_case(v),
        }
    }
    fn bad_case(&mut self, v: i64) {
        if v >= 0 {
            self.emit(Tok::Int(v));
            self.state = NUM_OR_ID;
        } else {
            self.emit(Tok::Op(op("(")));
            self.emit(Tok::Op(op("-")));
            self.emit(Tok::Int(-v));
            self.emit(Tok::Op(op(")")));
            self.state = MISC;
        }
    }
    fn flush(&mut self) {
        self.prepare(Kind::Misc, false);
        if self.state == SIGN {
            self.state = MISC;
        }
    }
}

fn fold_constants(toks: Vec<Tok>, secs: Vec<u32>) -> (Vec<Tok>, Vec<u32>) {
    let mut f = Folder {
        out: Vec::with_capacity(toks.len()),
        secs: Vec::with_capacity(toks.len()),
        state: MISC,
        out_sign: 0,
        out_val: 0,
        out_app: 0,
        last_sign: 0,
        sec: 0,
    };
    for (t, s) in toks.into_iter().zip(secs) {
        f.sec = s;
        match t {
            Tok::Int(v) => f.send_val(v),
            Tok::Op("+") => f.send_sign(1),
            Tok::Op("-") => f.send_sign(-1),
            Tok::Id(_) => f.send_out(Kind::Ident, t),
            Tok::Real(_) => f.send_out(Kind::Frac, t),
            _ => f.send_out(Kind::Misc, t),
        }
    }
    f.flush();
    (f.out, f.secs)
}

/// Replace the value of an outer-block `const` in the token stream. `tex.web`
/// is never edited, so this is where the capacities that web2c takes from
/// `tex.ch` and `texmf.cnf` are set (the trip test needs its own set).
pub fn override_consts(t: &mut Tangled, subs: &[(String, i64)]) -> Result<(), String> {
    // Find `const` ... up to the following `type`.
    let start = t
        .tokens
        .iter()
        .position(|x| x.is_id("const"))
        .ok_or("no `const` section in the program")?;
    let end = t.tokens[start..]
        .iter()
        .position(|x| x.is_id("type"))
        .map(|i| i + start)
        .unwrap_or(t.tokens.len());
    for (name, val) in subs {
        let mut done = false;
        let mut i = start;
        while i + 2 < end {
            if t.tokens[i].is_id(name) && t.tokens[i + 1].is_op("=") {
                t.tokens[i + 2] = Tok::Int(*val);
                done = true;
                break;
            }
            i += 1;
        }
        if !done {
            return Err(format!("--const: `{name}` is not an outer-block constant"));
        }
    }
    Ok(())
}
