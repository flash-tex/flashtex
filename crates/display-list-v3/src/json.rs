//! A small JSON value, reader and writer for the protocol's control messages
//! (RFC 8259). Numbers are kept as `f64` unless they are integers that fit an
//! `i64`; the protocol only sends integers, booleans, strings, arrays and
//! objects. Object keys keep their order.

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(i) => Some(*i),
            Json::Num(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Some(*f as i64),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
    /// A string field of an object.
    pub fn str_field(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Json::as_str)
    }
    pub fn int_field(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Json::as_i64)
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Json::Num(f) => {
                if f.is_finite() {
                    let _ = write!(out, "{f}");
                } else {
                    out.push_str("null");
                }
            }
            Json::Str(s) => write_str(out, s),
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(kv) => {
                out.push('{');
                for (i, (k, v)) in kv.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(out, k);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Parse one JSON text (surrounding white space allowed).
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut p = Parser {
            b: text.as_bytes(),
            i: 0,
            depth: 0,
        };
        p.ws();
        let v = p.value()?;
        p.ws();
        if p.i != p.b.len() {
            return Err(format!("trailing characters at byte {}", p.i));
        }
        Ok(v)
    }
}

/// Compact serialisation (`to_string()` comes with it).
impl std::fmt::Display for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        self.write(&mut s);
        f.write_str(&s)
    }
}

/// Build an object from pairs.
pub fn obj<const N: usize>(pairs: [(&str, Json); N]) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn s(v: impl Into<String>) -> Json {
    Json::Str(v.into())
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
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

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("{what} at byte {}", self.i))
    }
    fn value(&mut self) -> Result<Json, String> {
        if self.depth > 128 {
            return self.err("nesting too deep");
        }
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'n') => self.lit("null", Json::Null),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                self.depth += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    self.ws();
                    a.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return self.err("expected , or ]"),
                    }
                }
                self.depth -= 1;
                Ok(Json::Arr(a))
            }
            Some(b'{') => {
                self.i += 1;
                self.depth += 1;
                let mut kv = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Obj(kv));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return self.err("expected a key");
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return self.err("expected :");
                    }
                    self.i += 1;
                    self.ws();
                    let v = self.value()?;
                    kv.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return self.err("expected , or }"),
                    }
                }
                self.depth -= 1;
                Ok(Json::Obj(kv))
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            _ => self.err("unexpected character"),
        }
    }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            self.err("bad literal")
        }
    }
    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.b[self.i] == b'-' {
            self.i += 1;
        }
        let mut float = false;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'0'..=b'9' => {}
                b'.' | b'e' | b'E' | b'+' | b'-' => float = true,
                _ => break,
            }
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.b[start..self.i]).unwrap();
        if !float {
            if let Ok(i) = t.parse::<i64>() {
                return Ok(Json::Int(i));
            }
        }
        t.parse::<f64>()
            .map(Json::Num)
            .or_else(|_| self.err("bad number"))
    }
    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return self.err("short \\u escape");
        }
        let t = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|e| e.to_string())?;
        let v = u32::from_str_radix(t, 16).map_err(|e| e.to_string())?;
        self.i += 4;
        Ok(v)
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // "
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return self.err("unterminated string");
            };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return self.err("bad escape");
                    };
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let mut u = self.hex4()?;
                            if (0xd800..0xdc00).contains(&u)
                                && self.b.get(self.i) == Some(&b'\\')
                                && self.b.get(self.i + 1) == Some(&b'u')
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                u = 0x10000
                                    + ((u - 0xd800) << 10)
                                    + (lo.wrapping_sub(0xdc00) & 0x3ff);
                            }
                            char::from_u32(u).unwrap_or('\u{fffd}')
                        }
                        _ => return self.err("bad escape"),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c if c < 0x20 => return self.err("control character in string"),
                c => out.push(c),
            }
        }
        String::from_utf8(out).or_else(|_| self.err("string is not UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = r#"{"a":1,"b":[true,false,null,"x\"\né"],"c":-2.5,"d":{}}"#;
        let v = Json::parse(t).unwrap();
        assert_eq!(v.int_field("a"), Some(1));
        assert_eq!(
            v.get("b").unwrap().as_array().unwrap()[3].as_str(),
            Some("x\"\né")
        );
        assert_eq!(Json::parse(&v.to_string()).unwrap(), v);
        assert!(Json::parse("[1,").is_err());
        assert!(Json::parse("{} x").is_err());
    }
}
