//! Hand-written JSON serialization of display lists (proposal; not runtime-v1).
//!
//! Two documents are produced:
//!
//! - **display list** (`format: "flashtex-display-list"`): the authored tree
//!   with groups, transforms, and clips. Round-trips exactly through
//!   [`write_display_list`] / [`read_display_list`].
//! - **device list** (`format: "flashtex-device-list"`): the flattened,
//!   device-space output of [`DisplayList::flatten`], for a preview consumer
//!   that wants to paint without resolving groups. Write-only.
//!
//! Field reference (display list):
//!
//! ```text
//! {"format":"flashtex-display-list","version":0,
//!  "page_size":{"width_pt":W,"height_pt":H},
//!  "items":[
//!   {"kind":"rule","id":N,"rect":{"x":..,"y":..,"width":..,"height":..},
//!    "paint":{"color":{"space":"gray","g":0},"alpha":1},"source":SRC|null},
//!   {"kind":"path_fill","id":N,"path":PATH,"fill_rule":"nonzero"|"evenodd","paint":..,
//!    "pattern":{"name":NAME,"color":COLOR},"source":..},
//!   {"kind":"path_stroke","id":N,"path":PATH,
//!    "stroke":{"width":..,"cap":"butt"|"round"|"square","join":"miter"|"round"|"bevel",
//!              "miter_limit":..,"dash":null|{"array":[..],"phase":..}},"paint":..,"source":..},
//!   {"kind":"image","id":N,"content_hash":"sha256:..","width_pt":..,"height_pt":..,
//!    "transform":[a,b,c,d,e,f],"alpha":..,"source":..},
//!   {"kind":"group","id":N,"transform":[a,b,c,d,e,f],
//!    "clip":null|{"kind":"rect","rect":{..}}|{"kind":"path","path":PATH,"fill_rule":..},
//!    "opacity":..,"items":[..],"source":..}
//!  ]}
//! PATH = [["m",x,y],["l",x,y],["q",cx,cy,x,y],["c",c1x,c1y,c2x,c2y,x,y],["z"]]
//! SRC  = {"path":"main.tex","start_byte":S,"end_byte":E}
//! color = {"space":"gray","g":..} | {"space":"rgb","r":..,"g":..,"b":..}
//!       | {"space":"cmyk","c":..,"m":..,"y":..,"k":..}
//! ```
//!
//! Numbers are written with Rust's shortest round-trip `f64` formatting (never
//! exponent notation), so parsing them back yields the identical value.
//! Object keys are emitted in a fixed order, which makes the output
//! deterministic and byte-comparable.
//!
//! Nesting limits (GH46): [`parse`] refuses arrays/objects nested deeper than
//! [`MAX_JSON_DEPTH`] and [`read_display_list`] refuses groups nested deeper
//! than [`MAX_GROUP_DEPTH`], both with [`JsonError::NestingTooDeep`] returned
//! before any further recursion, so adversarial input yields an error rather
//! than a stack-overflow abort. Both readers are the only entry points that
//! accept text from outside the process; the writers, `flatten`, and the PDF
//! generator walk in-process trees and are bounded by the same group limit
//! whenever the tree came from [`read_display_list`] or passes
//! [`DisplayList::validate`], which reports deeper trees without descending
//! into them.

use crate::clip::{Clip, ClipStack};
use crate::color::{Color, Paint};
use crate::display_list::{DeviceItem, DeviceList, DeviceShape, DisplayList, MAX_GROUP_DEPTH};
use crate::geom::{Point, Rect, Size, Transform};
use crate::item::{Group, Image, Item, ItemId, PathFill, PathStroke, Pattern, Rule, SourceRange};
use crate::path::{Dash, FillRule, LineCap, LineJoin, Path, PathCommand, StrokeStyle};
use std::fmt::Write as _;

pub const DISPLAY_LIST_FORMAT: &str = "flashtex-display-list";
pub const DEVICE_LIST_FORMAT: &str = "flashtex-device-list";
pub const FORMAT_VERSION: u64 = 0;

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Serializes a display list (the authored tree).
pub fn write_display_list(list: &DisplayList) -> String {
    let mut w = String::new();
    w.push('{');
    field(&mut w, "format", &string(DISPLAY_LIST_FORMAT));
    w.push(',');
    field(&mut w, "version", &FORMAT_VERSION.to_string());
    w.push(',');
    field(&mut w, "page_size", &size(list.page_size));
    w.push(',');
    field(&mut w, "items", &items(&list.items));
    w.push('}');
    w
}

/// Serializes a flattened device list for a preview consumer.
pub fn write_device_list(list: &DeviceList) -> String {
    let mut w = String::new();
    w.push('{');
    field(&mut w, "format", &string(DEVICE_LIST_FORMAT));
    w.push(',');
    field(&mut w, "version", &FORMAT_VERSION.to_string());
    w.push(',');
    field(&mut w, "page_size", &size(list.page_size));
    w.push(',');
    let entries: Vec<String> = list.items.iter().map(device_item).collect();
    field(&mut w, "items", &format!("[{}]", entries.join(",")));
    w.push('}');
    w
}

fn device_item(item: &DeviceItem) -> String {
    let mut w = String::from("{");
    match &item.shape {
        DeviceShape::Rule { rect: r, paint: p } => {
            field(&mut w, "kind", &string("rule"));
            w.push(',');
            id_field(&mut w, item.id);
            w.push(',');
            field(&mut w, "rect", &rect(*r));
            w.push(',');
            field(&mut w, "paint", &paint(*p));
        }
        DeviceShape::Fill {
            path: pa,
            rule,
            paint: p,
        } => {
            field(&mut w, "kind", &string("path_fill"));
            w.push(',');
            id_field(&mut w, item.id);
            w.push(',');
            field(&mut w, "path", &path(pa));
            w.push(',');
            field(&mut w, "fill_rule", &string(fill_rule(*rule)));
            w.push(',');
            field(&mut w, "paint", &paint(*p));
        }
        DeviceShape::Stroke {
            path: pa,
            style,
            paint: p,
        } => {
            field(&mut w, "kind", &string("path_stroke"));
            w.push(',');
            id_field(&mut w, item.id);
            w.push(',');
            field(&mut w, "path", &path(pa));
            w.push(',');
            field(&mut w, "stroke", &stroke_style(style));
            w.push(',');
            field(&mut w, "paint", &paint(*p));
        }
        DeviceShape::Image {
            content_hash,
            width_pt,
            height_pt,
            transform: t,
            alpha,
        } => {
            field(&mut w, "kind", &string("image"));
            w.push(',');
            id_field(&mut w, item.id);
            w.push(',');
            field(&mut w, "content_hash", &string(content_hash));
            w.push(',');
            field(&mut w, "width_pt", &number(*width_pt));
            w.push(',');
            field(&mut w, "height_pt", &number(*height_pt));
            w.push(',');
            field(&mut w, "transform", &transform(t));
            w.push(',');
            field(&mut w, "alpha", &number(*alpha));
        }
    }
    w.push(',');
    let anc: Vec<String> = item.ancestors.iter().map(|a| a.0.to_string()).collect();
    field(&mut w, "ancestors", &format!("[{}]", anc.join(",")));
    w.push(',');
    field(&mut w, "clips", &clip_stack(&item.clip));
    w.push(',');
    field(&mut w, "source", &source(item.source.as_ref()));
    w.push('}');
    w
}

fn items(list: &[Item]) -> String {
    let entries: Vec<String> = list.iter().map(item).collect();
    format!("[{}]", entries.join(","))
}

fn item(it: &Item) -> String {
    let mut w = String::from("{");
    match it {
        Item::Rule(r) => {
            field(&mut w, "kind", &string("rule"));
            w.push(',');
            id_field(&mut w, r.id);
            w.push(',');
            field(&mut w, "rect", &rect(r.rect));
            w.push(',');
            field(&mut w, "paint", &paint(r.paint));
            w.push(',');
            field(&mut w, "source", &source(r.source.as_ref()));
        }
        Item::PathFill(p) => {
            field(&mut w, "kind", &string("path_fill"));
            w.push(',');
            id_field(&mut w, p.id);
            w.push(',');
            field(&mut w, "path", &path(&p.path));
            w.push(',');
            field(&mut w, "fill_rule", &string(fill_rule(p.rule)));
            w.push(',');
            field(&mut w, "paint", &paint(p.paint));
            if let Some(pattern) = &p.pattern {
                w.push(',');
                field(&mut w, "pattern", &pattern_json(pattern));
            }
            w.push(',');
            field(&mut w, "source", &source(p.source.as_ref()));
        }
        Item::PathStroke(p) => {
            field(&mut w, "kind", &string("path_stroke"));
            w.push(',');
            id_field(&mut w, p.id);
            w.push(',');
            field(&mut w, "path", &path(&p.path));
            w.push(',');
            field(&mut w, "stroke", &stroke_style(&p.style));
            w.push(',');
            field(&mut w, "paint", &paint(p.paint));
            w.push(',');
            field(&mut w, "source", &source(p.source.as_ref()));
        }
        Item::Image(i) => {
            field(&mut w, "kind", &string("image"));
            w.push(',');
            id_field(&mut w, i.id);
            w.push(',');
            field(&mut w, "content_hash", &string(&i.content_hash));
            w.push(',');
            field(&mut w, "width_pt", &number(i.width_pt));
            w.push(',');
            field(&mut w, "height_pt", &number(i.height_pt));
            w.push(',');
            field(&mut w, "transform", &transform(&i.transform));
            w.push(',');
            field(&mut w, "alpha", &number(i.alpha));
            w.push(',');
            field(&mut w, "source", &source(i.source.as_ref()));
        }
        Item::Group(g) => {
            field(&mut w, "kind", &string("group"));
            w.push(',');
            id_field(&mut w, g.id);
            w.push(',');
            field(&mut w, "transform", &transform(&g.transform));
            w.push(',');
            field(
                &mut w,
                "clip",
                &g.clip.as_ref().map(clip).unwrap_or_else(|| "null".into()),
            );
            w.push(',');
            field(&mut w, "opacity", &number(g.opacity));
            w.push(',');
            field(&mut w, "items", &items(&g.items));
            w.push(',');
            field(&mut w, "source", &source(g.source.as_ref()));
        }
    }
    w.push('}');
    w
}

fn id_field(w: &mut String, id: ItemId) {
    field(w, "id", &id.0.to_string());
}

fn field(w: &mut String, key: &str, value: &str) {
    let _ = write!(w, "{}:{}", string(key), value);
}

fn size(s: Size) -> String {
    format!(
        "{{\"width_pt\":{},\"height_pt\":{}}}",
        number(s.width),
        number(s.height)
    )
}

fn rect(r: Rect) -> String {
    format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        number(r.x),
        number(r.y),
        number(r.width),
        number(r.height)
    )
}

fn transform(t: &Transform) -> String {
    let c: Vec<String> = t.coefficients().iter().map(|v| number(*v)).collect();
    format!("[{}]", c.join(","))
}

fn color(c: Color) -> String {
    match c {
        Color::Gray(g) => format!("{{\"space\":\"gray\",\"g\":{}}}", number(g)),
        Color::Rgb(r, g, b) => format!(
            "{{\"space\":\"rgb\",\"r\":{},\"g\":{},\"b\":{}}}",
            number(r),
            number(g),
            number(b)
        ),
        Color::Cmyk(c, m, y, k) => format!(
            "{{\"space\":\"cmyk\",\"c\":{},\"m\":{},\"y\":{},\"k\":{}}}",
            number(c),
            number(m),
            number(y),
            number(k)
        ),
    }
}

fn paint(p: Paint) -> String {
    format!(
        "{{\"color\":{},\"alpha\":{}}}",
        color(p.color),
        number(p.alpha)
    )
}

fn pattern_json(p: &Pattern) -> String {
    format!(
        "{{\"name\":{},\"color\":{}}}",
        string(&p.name),
        color(p.color)
    )
}

fn fill_rule(r: FillRule) -> &'static str {
    match r {
        FillRule::NonZero => "nonzero",
        FillRule::EvenOdd => "evenodd",
    }
}

fn stroke_style(s: &StrokeStyle) -> String {
    let dash = match &s.dash {
        Some(d) => {
            let a: Vec<String> = d.array.iter().map(|v| number(*v)).collect();
            format!(
                "{{\"array\":[{}],\"phase\":{}}}",
                a.join(","),
                number(d.phase)
            )
        }
        None => "null".into(),
    };
    format!(
        "{{\"width\":{},\"cap\":\"{}\",\"join\":\"{}\",\"miter_limit\":{},\"dash\":{}}}",
        number(s.width),
        match s.cap {
            LineCap::Butt => "butt",
            LineCap::Round => "round",
            LineCap::Square => "square",
        },
        match s.join {
            LineJoin::Miter => "miter",
            LineJoin::Round => "round",
            LineJoin::Bevel => "bevel",
        },
        number(s.miter_limit),
        dash
    )
}

fn path(p: &Path) -> String {
    let cmds: Vec<String> = p
        .commands()
        .iter()
        .map(|c| match *c {
            PathCommand::MoveTo(p) => format!("[\"m\",{},{}]", number(p.x), number(p.y)),
            PathCommand::LineTo(p) => format!("[\"l\",{},{}]", number(p.x), number(p.y)),
            PathCommand::QuadTo(c, p) => {
                format!(
                    "[\"q\",{},{},{},{}]",
                    number(c.x),
                    number(c.y),
                    number(p.x),
                    number(p.y)
                )
            }
            PathCommand::CubicTo(a, b, p) => format!(
                "[\"c\",{},{},{},{},{},{}]",
                number(a.x),
                number(a.y),
                number(b.x),
                number(b.y),
                number(p.x),
                number(p.y)
            ),
            PathCommand::Close => "[\"z\"]".into(),
        })
        .collect();
    format!("[{}]", cmds.join(","))
}

fn clip(c: &Clip) -> String {
    match c {
        Clip::Rect(r) => format!("{{\"kind\":\"rect\",\"rect\":{}}}", rect(*r)),
        Clip::Path { path: p, rule } => {
            format!(
                "{{\"kind\":\"path\",\"path\":{},\"fill_rule\":\"{}\"}}",
                path(p),
                fill_rule(*rule)
            )
        }
    }
}

fn clip_stack(s: &ClipStack) -> String {
    let entries: Vec<String> = s.clips().iter().map(clip).collect();
    format!("[{}]", entries.join(","))
}

fn source(s: Option<&SourceRange>) -> String {
    match s {
        Some(s) => format!(
            "{{\"path\":{},\"start_byte\":{},\"end_byte\":{}}}",
            string(&s.path),
            s.start_byte,
            s.end_byte
        ),
        None => "null".into(),
    }
}

/// Shortest round-trip decimal; non-finite values become `null` (they are
/// rejected by [`DisplayList::validate`] before serialization matters).
pub fn number(v: f64) -> String {
    if v.is_finite() {
        // Rust's Display never uses exponent notation for f64.
        let s = format!("{v}");
        if s == "-0" { "0".into() } else { s }
    } else {
        "null".into()
    }
}

/// JSON string literal with escapes.
pub fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
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
    out
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Maximum nesting of arrays/objects [`parse`] accepts; deeper input returns
/// [`JsonError::NestingTooDeep`] instead of recursing (GH46).
///
/// Rationale: the display-list contract states no nesting bound, so this is
/// the crate's own. The deepest legal display-list document is
/// `2 * MAX_GROUP_DEPTH + 2` levels (each group is an object plus its `items`
/// array, under the root object and its `items`), i.e. 130 with the group
/// limit of 64, and no other schema field nests past depth 3; 256 leaves that
/// with headroom while staying far below what the recursive parser can take
/// in a debug `cargo test` worker thread (2 MiB): 1200 nested arrays parsed
/// and 1500 overflowed on mac-m1max-a, so 256 keeps ~4.7x headroom.
pub const MAX_JSON_DEPTH: usize = 256;

/// A parse or schema error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonError {
    /// Malformed text or a schema violation, with a short message.
    Message(String),
    /// Arrays/objects nested more than [`MAX_JSON_DEPTH`] levels, or groups
    /// nested more than [`crate::display_list::MAX_GROUP_DEPTH`] levels
    /// (`limit` says which). `depth` is the level that exceeded it.
    NestingTooDeep { depth: usize, limit: usize },
}

impl JsonError {
    /// The human-readable message (every variant has one).
    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::Message(m) => f.write_str(m),
            JsonError::NestingTooDeep { depth, limit } => {
                write!(f, "nesting depth {depth} exceeds the limit of {limit}")
            }
        }
    }
}

impl std::error::Error for JsonError {}

/// Generic JSON value used by the reader.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn require(&self, key: &str) -> Result<&Value, JsonError> {
        self.get(key)
            .ok_or_else(|| JsonError::Message(format!("missing field `{key}`")))
    }

    fn as_f64(&self) -> Result<f64, JsonError> {
        match self {
            Value::Number(n) => Ok(*n),
            other => Err(JsonError::Message(format!(
                "expected number, found {}",
                other.kind()
            ))),
        }
    }

    fn as_str(&self) -> Result<&str, JsonError> {
        match self {
            Value::String(s) => Ok(s),
            other => Err(JsonError::Message(format!(
                "expected string, found {}",
                other.kind()
            ))),
        }
    }

    fn as_array(&self) -> Result<&[Value], JsonError> {
        match self {
            Value::Array(a) => Ok(a),
            other => Err(JsonError::Message(format!(
                "expected array, found {}",
                other.kind()
            ))),
        }
    }

    fn as_usize(&self) -> Result<usize, JsonError> {
        let n = self.as_f64()?;
        if n >= 0.0 && n.fract() == 0.0 && n <= usize::MAX as f64 {
            Ok(n as usize)
        } else {
            Err(JsonError::Message(format!(
                "expected non-negative integer, found {n}"
            )))
        }
    }

    fn as_u64(&self) -> Result<u64, JsonError> {
        let n = self.as_f64()?;
        if n >= 0.0 && n.fract() == 0.0 && n <= u64::MAX as f64 {
            Ok(n as u64)
        } else {
            Err(JsonError::Message(format!(
                "expected non-negative integer, found {n}"
            )))
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

/// Parses any JSON text into a [`Value`].
pub fn parse(text: &str) -> Result<Value, JsonError> {
    let mut p = Parser {
        bytes: text.as_bytes(),
        pos: 0,
        depth: 0,
    };
    p.skip_ws();
    let v = p.value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(JsonError::Message(format!(
            "trailing characters at byte {}",
            p.pos
        )));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// Number of arrays/objects currently open; bounded by [`MAX_JSON_DEPTH`].
    depth: usize,
}

impl Parser<'_> {
    /// Enters one container level, failing before any further recursion once
    /// the level would exceed [`MAX_JSON_DEPTH`].
    fn enter(&mut self) -> Result<(), JsonError> {
        let depth = self.depth + 1;
        if depth > MAX_JSON_DEPTH {
            return Err(JsonError::NestingTooDeep {
                depth,
                limit: MAX_JSON_DEPTH,
            });
        }
        self.depth = depth;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\n' | b'\r' | b'\t')
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn expect(&mut self, b: u8) -> Result<(), JsonError> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(JsonError::Message(format!(
                "expected '{}' at byte {}",
                b as char, self.pos
            )))
        }
    }

    fn value(&mut self) -> Result<Value, JsonError> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(c) => Err(JsonError::Message(format!(
                "unexpected '{}' at byte {}",
                c as char, self.pos
            ))),
            None => Err(JsonError::Message("unexpected end of input".into())),
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, JsonError> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(v)
        } else {
            Err(JsonError::Message(format!(
                "invalid literal at byte {}",
                self.pos
            )))
        }
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        while self.pos < self.bytes.len()
            && matches!(
                self.bytes[self.pos],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|e| JsonError::Message(e.to_string()))?;
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| JsonError::Message(format!("invalid number `{text}` at byte {start}")))
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while self.pos < self.bytes.len()
                && self.bytes[self.pos] != b'"'
                && self.bytes[self.pos] != b'\\'
            {
                self.pos += 1;
            }
            out.push_str(
                std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|e| JsonError::Message(e.to_string()))?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let esc = self
                        .peek()
                        .ok_or_else(|| JsonError::Message("unterminated escape".into()))?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            let ch = if (0xD800..0xDC00).contains(&cp) {
                                // Surrogate pair.
                                if self.bytes[self.pos..].starts_with(b"\\u") {
                                    self.pos += 2;
                                    let low = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&low) {
                                        return Err(JsonError::Message(
                                            "invalid low surrogate".into(),
                                        ));
                                    }
                                    0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00)
                                } else {
                                    return Err(JsonError::Message("lone high surrogate".into()));
                                }
                            } else {
                                cp
                            };
                            out.push(
                                char::from_u32(ch).ok_or_else(|| {
                                    JsonError::Message("invalid code point".into())
                                })?,
                            );
                        }
                        other => {
                            return Err(JsonError::Message(format!(
                                "invalid escape '\\{}'",
                                other as char
                            )));
                        }
                    }
                }
                _ => return Err(JsonError::Message("unterminated string".into())),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let s = self
            .bytes
            .get(self.pos..self.pos + 4)
            .and_then(|b| std::str::from_utf8(b).ok())
            .ok_or_else(|| JsonError::Message("truncated \\u escape".into()))?;
        let v = u32::from_str_radix(s, 16)
            .map_err(|_| JsonError::Message("invalid \\u escape".into()))?;
        self.pos += 4;
        Ok(v)
    }

    fn array(&mut self) -> Result<Value, JsonError> {
        self.expect(b'[')?;
        self.enter()?;
        let v = self.array_body();
        self.leave();
        v
    }

    fn array_body(&mut self) -> Result<Value, JsonError> {
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => {
                    return Err(JsonError::Message(format!(
                        "expected ',' or ']' at byte {}",
                        self.pos
                    )));
                }
            }
        }
    }

    fn object(&mut self) -> Result<Value, JsonError> {
        self.expect(b'{')?;
        self.enter()?;
        let v = self.object_body();
        self.leave();
        v
    }

    fn object_body(&mut self) -> Result<Value, JsonError> {
        let mut fields = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(fields));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let v = self.value()?;
            fields.push((key, v));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(fields));
                }
                _ => {
                    return Err(JsonError::Message(format!(
                        "expected ',' or '}}' at byte {}",
                        self.pos
                    )));
                }
            }
        }
    }
}

/// Parses a display-list document written by [`write_display_list`].
pub fn read_display_list(text: &str) -> Result<DisplayList, JsonError> {
    let v = parse(text)?;
    let format = v.require("format")?.as_str()?;
    if format != DISPLAY_LIST_FORMAT {
        return Err(JsonError::Message(format!("unsupported format `{format}`")));
    }
    let version = v.require("version")?.as_u64()?;
    if version != FORMAT_VERSION {
        return Err(JsonError::Message(format!("unsupported version {version}")));
    }
    let ps = v.require("page_size")?;
    let page_size = Size::new(
        ps.require("width_pt")?.as_f64()?,
        ps.require("height_pt")?.as_f64()?,
    );
    let items = read_items(v.require("items")?, 0)?;
    Ok(DisplayList { page_size, items })
}

/// `group_depth` is the number of groups enclosing these items; nesting past
/// [`MAX_GROUP_DEPTH`] is refused before descending (GH46). The bound is
/// separate from [`MAX_JSON_DEPTH`] because a `read_item` frame is far
/// heavier than a parser frame.
fn read_items(v: &Value, group_depth: usize) -> Result<Vec<Item>, JsonError> {
    v.as_array()?
        .iter()
        .map(|item| read_item(item, group_depth))
        .collect()
}

fn read_item(v: &Value, group_depth: usize) -> Result<Item, JsonError> {
    let kind = v.require("kind")?.as_str()?;
    let id = ItemId(v.require("id")?.as_u64()?);
    let source = read_source(v.get("source"))?;
    Ok(match kind {
        "rule" => Item::Rule(Rule {
            id,
            rect: read_rect(v.require("rect")?)?,
            paint: read_paint(v.require("paint")?)?,
            source,
        }),
        "path_fill" => Item::PathFill(PathFill {
            id,
            path: read_path(v.require("path")?)?,
            rule: read_fill_rule(v.require("fill_rule")?)?,
            paint: read_paint(v.require("paint")?)?,
            pattern: v.get("pattern").map(read_pattern).transpose()?,
            source,
        }),
        "path_stroke" => Item::PathStroke(PathStroke {
            id,
            path: read_path(v.require("path")?)?,
            style: read_stroke_style(v.require("stroke")?)?,
            paint: read_paint(v.require("paint")?)?,
            source,
        }),
        "image" => Item::Image(Image {
            id,
            content_hash: v.require("content_hash")?.as_str()?.to_string(),
            width_pt: v.require("width_pt")?.as_f64()?,
            height_pt: v.require("height_pt")?.as_f64()?,
            transform: read_transform(v.require("transform")?)?,
            alpha: v.require("alpha")?.as_f64()?,
            source,
        }),
        "group" => {
            let depth = group_depth + 1;
            if depth > MAX_GROUP_DEPTH {
                return Err(JsonError::NestingTooDeep {
                    depth,
                    limit: MAX_GROUP_DEPTH,
                });
            }
            Item::Group(Group {
                id,
                transform: read_transform(v.require("transform")?)?,
                clip: match v.get("clip") {
                    None | Some(Value::Null) => None,
                    Some(c) => Some(read_clip(c)?),
                },
                opacity: v.require("opacity")?.as_f64()?,
                items: read_items(v.require("items")?, depth)?,
                source,
            })
        }
        other => return Err(JsonError::Message(format!("unknown item kind `{other}`"))),
    })
}

fn read_source(v: Option<&Value>) -> Result<Option<SourceRange>, JsonError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(s) => Ok(Some(SourceRange {
            path: s.require("path")?.as_str()?.to_string(),
            start_byte: s.require("start_byte")?.as_usize()?,
            end_byte: s.require("end_byte")?.as_usize()?,
        })),
    }
}

fn read_rect(v: &Value) -> Result<Rect, JsonError> {
    Ok(Rect::new(
        v.require("x")?.as_f64()?,
        v.require("y")?.as_f64()?,
        v.require("width")?.as_f64()?,
        v.require("height")?.as_f64()?,
    ))
}

fn read_transform(v: &Value) -> Result<Transform, JsonError> {
    let a = v.as_array()?;
    if a.len() != 6 {
        return Err(JsonError::Message(format!(
            "transform needs 6 numbers, found {}",
            a.len()
        )));
    }
    let n: Result<Vec<f64>, _> = a.iter().map(Value::as_f64).collect();
    let n = n?;
    Ok(Transform::new(n[0], n[1], n[2], n[3], n[4], n[5]))
}

fn read_color(v: &Value) -> Result<Color, JsonError> {
    Ok(match v.require("space")?.as_str()? {
        "gray" => Color::Gray(v.require("g")?.as_f64()?),
        "rgb" => Color::Rgb(
            v.require("r")?.as_f64()?,
            v.require("g")?.as_f64()?,
            v.require("b")?.as_f64()?,
        ),
        "cmyk" => Color::Cmyk(
            v.require("c")?.as_f64()?,
            v.require("m")?.as_f64()?,
            v.require("y")?.as_f64()?,
            v.require("k")?.as_f64()?,
        ),
        other => {
            return Err(JsonError::Message(format!(
                "unknown colour space `{other}`"
            )));
        }
    })
}

fn read_paint(v: &Value) -> Result<Paint, JsonError> {
    Ok(Paint {
        color: read_color(v.require("color")?)?,
        alpha: v.require("alpha")?.as_f64()?,
    })
}

fn read_pattern(v: &Value) -> Result<Pattern, JsonError> {
    Ok(Pattern {
        name: v.require("name")?.as_str()?.to_string(),
        color: read_color(v.require("color")?)?,
    })
}

fn read_fill_rule(v: &Value) -> Result<FillRule, JsonError> {
    match v.as_str()? {
        "nonzero" => Ok(FillRule::NonZero),
        "evenodd" => Ok(FillRule::EvenOdd),
        other => Err(JsonError::Message(format!("unknown fill rule `{other}`"))),
    }
}

fn read_stroke_style(v: &Value) -> Result<StrokeStyle, JsonError> {
    let cap = match v.require("cap")?.as_str()? {
        "butt" => LineCap::Butt,
        "round" => LineCap::Round,
        "square" => LineCap::Square,
        other => return Err(JsonError::Message(format!("unknown cap `{other}`"))),
    };
    let join = match v.require("join")?.as_str()? {
        "miter" => LineJoin::Miter,
        "round" => LineJoin::Round,
        "bevel" => LineJoin::Bevel,
        other => return Err(JsonError::Message(format!("unknown join `{other}`"))),
    };
    let dash = match v.get("dash") {
        None | Some(Value::Null) => None,
        Some(d) => {
            let array: Result<Vec<f64>, _> = d
                .require("array")?
                .as_array()?
                .iter()
                .map(Value::as_f64)
                .collect();
            Some(Dash {
                array: array?,
                phase: d.require("phase")?.as_f64()?,
            })
        }
    };
    Ok(StrokeStyle {
        width: v.require("width")?.as_f64()?,
        cap,
        join,
        miter_limit: v.require("miter_limit")?.as_f64()?,
        dash,
    })
}

fn read_path(v: &Value) -> Result<Path, JsonError> {
    let mut commands = Vec::new();
    for cmd in v.as_array()? {
        let parts = cmd.as_array()?;
        let op = parts
            .first()
            .ok_or_else(|| JsonError::Message("empty path command".into()))?
            .as_str()?;
        let nums: Result<Vec<f64>, _> = parts[1..].iter().map(Value::as_f64).collect();
        let n = nums?;
        let need = |k: usize| -> Result<(), JsonError> {
            if n.len() == k {
                Ok(())
            } else {
                Err(JsonError::Message(format!(
                    "path op `{op}` needs {k} numbers, found {}",
                    n.len()
                )))
            }
        };
        commands.push(match op {
            "m" => {
                need(2)?;
                PathCommand::MoveTo(Point::new(n[0], n[1]))
            }
            "l" => {
                need(2)?;
                PathCommand::LineTo(Point::new(n[0], n[1]))
            }
            "q" => {
                need(4)?;
                PathCommand::QuadTo(Point::new(n[0], n[1]), Point::new(n[2], n[3]))
            }
            "c" => {
                need(6)?;
                PathCommand::CubicTo(
                    Point::new(n[0], n[1]),
                    Point::new(n[2], n[3]),
                    Point::new(n[4], n[5]),
                )
            }
            "z" => {
                need(0)?;
                PathCommand::Close
            }
            other => return Err(JsonError::Message(format!("unknown path op `{other}`"))),
        });
    }
    Ok(Path::from_commands(commands))
}

fn read_clip(v: &Value) -> Result<Clip, JsonError> {
    match v.require("kind")?.as_str()? {
        "rect" => Ok(Clip::Rect(read_rect(v.require("rect")?)?)),
        "path" => Ok(Clip::Path {
            path: read_path(v.require("path")?)?,
            rule: read_fill_rule(v.require("fill_rule")?)?,
        }),
        other => Err(JsonError::Message(format!("unknown clip kind `{other}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strings_and_numbers() {
        let v = parse(r#"{"a":[1,-2.5,1e3],"s":"x\"y\\é😀","n":null,"t":true}"#).unwrap();
        assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 3);
        assert_eq!(
            v.get("a").unwrap().as_array().unwrap()[2],
            Value::Number(1000.0)
        );
        assert_eq!(v.get("s").unwrap().as_str().unwrap(), "x\"y\\é😀");
        assert_eq!(v.get("n"), Some(&Value::Null));
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\":1} x").is_err());
    }

    #[test]
    fn number_formatting_round_trips() {
        for v in [
            0.0,
            -0.0,
            1.0,
            0.1,
            1e-7,
            123456789.125,
            f64::MAX,
            f64::MIN_POSITIVE,
        ] {
            let s = number(v);
            assert!(!s.contains('e'), "{s}");
            let back: f64 = s.parse().unwrap();
            assert_eq!(back, if v == 0.0 { 0.0 } else { v });
        }
        assert_eq!(number(f64::NAN), "null");
    }
}
