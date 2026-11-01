//! Minimal JSON reader/writer.
//!
//! Hand-written on purpose: the crate takes zero external dependencies so the
//! build is offline and deterministic. JSON is transport, not TeX semantics, so
//! implementing it here makes no claim about language compatibility.

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(BTreeMap<String, Value>),
    /// Pre-rendered JSON, written verbatim.
    ///
    /// Large replies spend a large share of their time building a `Value` tree
    /// that is immediately thrown away: at 500 KB the page items alone are tens
    /// of thousands of maps with owned String keys. This lets a hot path render
    /// straight to text while the rest of the document stays structured. The
    /// caller is responsible for the content being valid JSON; the byte-exact
    /// pinned fixtures are what prove it.
    Raw(String),
}

impl Value {
    pub fn obj() -> Value {
        Value::Obj(BTreeMap::new())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, v: Value) {
        if let Value::Obj(m) = self {
            m.insert(key.to_string(), v);
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Num(n) if n.is_finite() => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }
}

pub fn num(n: f64) -> Value {
    Value::Num(n)
}
pub fn str_(s: impl Into<String>) -> Value {
    Value::Str(s.into())
}

#[derive(Debug)]
pub struct JsonError(pub String);

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn parse(input: &str) -> Result<Value, JsonError> {
    let b: Vec<char> = input.chars().collect();
    let mut p = Parser { b, i: 0 };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(JsonError("trailing content after JSON value".into()));
    }
    Ok(v)
}

struct Parser {
    b: Vec<char>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(
            self.peek(),
            Some(' ') | Some('\t') | Some('\n') | Some('\r')
        ) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: char) -> Result<(), JsonError> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(JsonError(format!(
                "expected '{}' at position {}",
                c, self.i
            )))
        }
    }

    fn value(&mut self) -> Result<Value, JsonError> {
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Value::Str(self.string()?)),
            Some('t') => self.lit("true", Value::Bool(true)),
            Some('f') => self.lit("false", Value::Bool(false)),
            Some('n') => self.lit("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(JsonError(format!("unexpected character '{}'", c))),
            None => Err(JsonError("unexpected end of input".into())),
        }
    }

    fn lit(&mut self, word: &str, v: Value) -> Result<Value, JsonError> {
        for c in word.chars() {
            self.eat(c)?;
        }
        Ok(v)
    }

    fn object(&mut self) -> Result<Value, JsonError> {
        self.eat('{')?;
        let mut m = BTreeMap::new();
        self.ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Value::Obj(m));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            self.eat(':')?;
            self.ws();
            let v = self.value()?;
            m.insert(k, v);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some('}') => {
                    self.i += 1;
                    return Ok(Value::Obj(m));
                }
                _ => return Err(JsonError("expected ',' or '}' in object".into())),
            }
        }
    }

    fn array(&mut self) -> Result<Value, JsonError> {
        self.eat('[')?;
        let mut a = Vec::new();
        self.ws();
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Value::Arr(a));
        }
        loop {
            self.ws();
            a.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some(']') => {
                    self.i += 1;
                    return Ok(Value::Arr(a));
                }
                _ => return Err(JsonError("expected ',' or ']' in array".into())),
            }
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.eat('"')?;
        let mut s = String::new();
        loop {
            let c = self
                .peek()
                .ok_or_else(|| JsonError("unterminated string".into()))?;
            self.i += 1;
            match c {
                '"' => return Ok(s),
                '\\' => {
                    let e = self
                        .peek()
                        .ok_or_else(|| JsonError("unterminated escape".into()))?;
                    self.i += 1;
                    match e {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '/' => s.push('/'),
                        'b' => s.push('\u{8}'),
                        'f' => s.push('\u{c}'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        'u' => s.push(self.unicode_escape()?),
                        other => return Err(JsonError(format!("bad escape '\\{}'", other))),
                    }
                }
                other => s.push(other),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self
                .peek()
                .ok_or_else(|| JsonError("short \\u escape".into()))?;
            self.i += 1;
            let d = c
                .to_digit(16)
                .ok_or_else(|| JsonError("bad hex digit".into()))?;
            v = v * 16 + d;
        }
        Ok(v)
    }

    /// Handles surrogate pairs so non-BMP characters survive the round trip.
    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let hi = self.hex4()?;
        if (0xD800..0xDC00).contains(&hi) {
            if self.peek() == Some('\\') {
                self.i += 1;
                self.eat('u')?;
                let lo = self.hex4()?;
                if (0xDC00..0xE000).contains(&lo) {
                    let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                    return char::from_u32(cp).ok_or_else(|| JsonError("bad code point".into()));
                }
                return Err(JsonError("unpaired high surrogate".into()));
            }
            return Err(JsonError("unpaired high surrogate".into()));
        }
        char::from_u32(hi).ok_or_else(|| JsonError("bad code point".into()))
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.i;
        if self.peek() == Some('-') {
            self.i += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        if self.peek() == Some('.') {
            self.i += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.i += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.i += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let text: String = self.b[start..self.i].iter().collect();
        text.parse::<f64>()
            .map(Value::Num)
            .map_err(|_| JsonError(format!("bad number '{}'", text)))
    }
}

pub fn write(v: &Value) -> String {
    let mut out = String::new();
    write_into(v, &mut out);
    out
}

fn write_into(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Num(n) => {
            if n.is_finite() && *n == n.trunc() && n.abs() < 1e15 {
                write_integer(*n as i64, out);
            } else if n.is_finite() {
                if !write_hundredths(*n, out) {
                    let _ = write!(out, "{}", n);
                }
            } else {
                out.push_str("null");
            }
        }
        Value::Str(s) => write_string(s, out),
        Value::Raw(raw) => out.push_str(raw),
        Value::Arr(a) => {
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        Value::Obj(m) => {
            out.push('{');
            for (i, (k, val)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_into(val, out);
            }
            out.push('}');
        }
    }
}

pub fn write_string_into(s: &str, out: &mut String) {
    write_string(s, out);
}

/// Number formatting identical to [`Value::Num`] serialisation.
pub fn write_number_into(n: f64, out: &mut String) {
    write_into(&Value::Num(n), out);
}

/// Same bytes as `write!(out, "{}", value)`, without the formatting machinery;
/// replies carry several integers per item (issue #65).
fn write_integer(value: i64, out: &mut String) {
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    let mut rest = value.unsigned_abs();
    loop {
        start -= 1;
        digits[start] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if value < 0 {
        out.push('-');
    }
    // ASCII digits: pushed as chars (FT-065: `str::from_utf8` validating the
    // slice showed up as a top frame of a display-list-v2 reply).
    out.reserve(digits.len() - start);
    for &d in &digits[start..] {
        out.push(char::from(d));
    }
}

/// Writes a non-integer `n` that is exactly `k / 100.0` with |n| < 10 000 (all
/// `layout::round2` page coordinates) as the same shortest round-trip digits
/// `write!(out, "{}", n)` produces, which is the dominant cost of a large reply
/// (issue #65). Returns false, writing nothing, for any other value. The
/// equivalence is checked exhaustively over that range by
/// `json_fast_paths_match_the_formatting_machinery`.
fn write_hundredths(n: f64, out: &mut String) -> bool {
    if n.abs() >= 10_000.0 {
        return false;
    }
    let hundredths = (n * 100.0).round();
    if hundredths / 100.0 != n {
        return false;
    }
    let hundredths = hundredths as i64;
    let (whole, fraction) = (hundredths.abs() / 100, hundredths.abs() % 100);
    if hundredths < 0 {
        out.push('-');
    }
    write_integer(whole, out);
    out.push('.');
    out.push(char::from(b'0' + (fraction / 10) as u8));
    if fraction % 10 != 0 {
        out.push(char::from(b'0' + (fraction % 10) as u8));
    }
    true
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    if !s
        .bytes()
        .any(|byte| byte == b'"' || byte == b'\\' || byte < 0x20)
    {
        // Nothing to escape: one copy instead of a push per char.
        out.push_str(s);
        out.push('"');
        return;
    }
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
