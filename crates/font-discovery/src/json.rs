//! The smallest JSON reader and writer the index cache needs: objects,
//! arrays, strings, integers and booleans. Written here rather than pulled
//! in, because this crate has no dependencies (render-pipeline depends on
//! it by path, like class-geometry) and the cache file is our own shape.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.get(key),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }
}

pub fn write(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Num(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{n}"));
            }
        }
        Value::Str(s) => write_str(s, out),
        Value::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, out);
            }
            out.push(']');
        }
        Value::Obj(m) => {
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_str(k, out);
                out.push(':');
                write(x, out);
            }
            out.push('}');
        }
    }
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser { b: text.as_bytes(), at: 0 };
    let v = p.value()?;
    p.ws();
    if p.at != p.b.len() {
        return Err(format!("trailing data at byte {}", p.at));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.at < self.b.len() && matches!(self.b[self.at], b' ' | b'\n' | b'\r' | b'\t') {
            self.at += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.b.get(self.at) == Some(&c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.at))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.b.get(self.at) {
            None => Err("unexpected end".into()),
            Some(b'{') => {
                self.at += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.b.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Value::Obj(m));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.b.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(Value::Obj(m));
                        }
                        _ => return Err(format!("expected ',' or '}}' at byte {}", self.at)),
                    }
                }
            }
            Some(b'[') => {
                self.at += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Value::Arr(a));
                }
                loop {
                    a.push(self.value()?);
                    self.ws();
                    match self.b.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            return Ok(Value::Arr(a));
                        }
                        _ => return Err(format!("expected ',' or ']' at byte {}", self.at)),
                    }
                }
            }
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') if self.b[self.at..].starts_with(b"true") => {
                self.at += 4;
                Ok(Value::Bool(true))
            }
            Some(b'f') if self.b[self.at..].starts_with(b"false") => {
                self.at += 5;
                Ok(Value::Bool(false))
            }
            Some(b'n') if self.b[self.at..].starts_with(b"null") => {
                self.at += 4;
                Ok(Value::Null)
            }
            Some(_) => {
                let start = self.at;
                while self.at < self.b.len() && matches!(self.b[self.at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.at += 1;
                }
                std::str::from_utf8(&self.b[start..self.at])
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(Value::Num)
                    .ok_or_else(|| format!("bad number at byte {start}"))
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let Some(&c) = self.b.get(self.at) else { return Err("unterminated string".into()) };
            self.at += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.b.get(self.at) else { return Err("unterminated escape".into()) };
                    self.at += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let hex = self.b.get(self.at..self.at + 4).ok_or("short \\u escape")?;
                            self.at += 4;
                            let n = u32::from_str_radix(std::str::from_utf8(hex).map_err(|e| e.to_string())?, 16).map_err(|e| e.to_string())?;
                            out.push(char::from_u32(n).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(format!("bad escape at byte {}", self.at)),
                    }
                }
                _ => {
                    // Copy one UTF-8 sequence.
                    let start = self.at - 1;
                    let len = utf8_len(c);
                    let end = (start + len).min(self.b.len());
                    out.push_str(std::str::from_utf8(&self.b[start..end]).map_err(|e| e.to_string())?);
                    self.at = end;
                }
            }
        }
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Arr(vec![Value::Num(1.0), Value::Bool(true), Value::Str("x\"y\\z\n é".into())]));
        m.insert("b".into(), Value::Null);
        let v = Value::Obj(m);
        let mut s = String::new();
        write(&v, &mut s);
        assert_eq!(s, "{\"a\":[1,true,\"x\\\"y\\\\z\\n é\"],\"b\":null}");
        assert_eq!(parse(&s).unwrap(), v);
        assert!(parse("{\"a\":}").is_err());
        assert!(parse("[1,2] x").is_err());
    }
}
