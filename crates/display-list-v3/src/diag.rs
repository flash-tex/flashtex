//! `diag-v1`: structured diagnostics (docs/protocol/display-list-v3.md
//! §6.7). One `DIAG` message ([`crate::kind::DIAG`], JSON) per error or
//! warning of a compile, in the order the engine reported them, for a
//! client that accepted `diag-v1` in its `HELLO`; such a client gets no
//! `DIAGNOSTIC` messages.
//!
//! A `DIAG` says where TeX was reading when it reported the problem, as
//! precisely as TeX knows it: the file, the line, and the **column** TeX's
//! own error context splits the line at (`l.12 \foo` / `bar`), the byte
//! range of the token before that split, the chain of input levels that
//! led there (every macro being expanded, with its definition site when
//! the run saw it), TeX's help text, a severity and a stable code. Places
//! are also given as display-list span ids (§5.3), which move with their
//! lines across edits like the pages' spans do.
//!
//! This module is the reference decoder (and the host's encoder): MIT, like
//! the rest of the crate.
//!
//! ```
//! use flashtex_display_list::diag::Diag;
//! use flashtex_display_list::json::Json;
//! let j = Json::parse(r#"{"id":3,"seq":0,"severity":"error",
//!   "code":"tex/undefined-control-sequence","origin":"tex",
//!   "message":"Undefined control sequence.","file":"/p/main.tex",
//!   "line":12,"col":10,"range":[6,10],"exact":true}"#).unwrap();
//! let d = Diag::from_json(&j).unwrap();
//! assert_eq!((d.line, d.col, d.range), (Some(12), Some(10), Some((6, 10))));
//! assert_eq!(Diag::from_json(&d.to_json()).unwrap(), d);
//! ```

use crate::json::{s, Json};

/// The capability a host announces and a client accepts (`HELLO`).
pub const CAPABILITY: &str = "diag-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    pub fn parse(v: &str) -> Option<Severity> {
        Some(match v {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            _ => return None,
        })
    }
}

/// A place in a file.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Loc {
    /// Absolute path.
    pub file: Option<String>,
    /// 1-based.
    pub line: Option<i64>,
    /// 0-based byte column.
    pub col: Option<i64>,
    /// The display-list span of (file, line) (§5.3).
    pub span: Option<i64>,
}

impl Loc {
    fn from_json(j: &Json) -> Loc {
        Loc {
            file: j.str_field("file").map(str::to_string),
            line: j.int_field("line"),
            col: j.int_field("col"),
            span: j.int_field("span"),
        }
    }

    fn to_json(&self) -> Json {
        let mut kv = vec![];
        push_opt_str(&mut kv, "file", &self.file);
        push_opt_int(&mut kv, "line", self.line);
        push_opt_int(&mut kv, "col", self.col);
        push_opt_int(&mut kv, "span", self.span);
        Json::Obj(kv)
    }
}

/// One level of TeX's input stack when it reported the problem,
/// innermost first (the last is the file level `line`/`col` name).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Frame {
    /// `file`, `scantokens`, `terminal`, `insert`, `read`, or a token
    /// list's kind: `macro`, `argument`, `template`, `backed_up`,
    /// `recently_read`, `inserted`, `output`, `everypar`, `everymath`,
    /// `everydisplay`, `everyhbox`, `everyvbox`, `everyjob`, `everycr`,
    /// `mark`, `everyeof`, `write`. Unknown kinds may appear later.
    pub kind: String,
    /// A macro's name (`\section`).
    pub name: Option<String>,
    /// A file level's place (`col`: TeX's split).
    pub loc: Option<Loc>,
    /// What TeX had read of the level, and what was left (a file's line;
    /// a token list as TeX shows it, a macro as `\name #1->body`).
    pub before: String,
    pub after: String,
    /// A macro's definition site (file, line), when known.
    pub def: Option<Loc>,
}

impl Frame {
    fn from_json(j: &Json) -> Frame {
        let text = j.get("text").and_then(Json::as_array).unwrap_or(&[]);
        let t = |i: usize| {
            text.get(i)
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let loc = (j.get("file").is_some() || j.get("line").is_some()).then(|| Loc::from_json(j));
        Frame {
            kind: j.str_field("kind").unwrap_or("?").to_string(),
            name: j.str_field("name").map(str::to_string),
            loc,
            before: t(0),
            after: t(1),
            def: j.get("def").map(Loc::from_json),
        }
    }

    fn to_json(&self) -> Json {
        let mut kv = vec![("kind".to_string(), s(self.kind.as_str()))];
        push_opt_str(&mut kv, "name", &self.name);
        if let Some(Json::Obj(l)) = self.loc.as_ref().map(Loc::to_json) {
            kv.extend(l);
        }
        kv.push((
            "text".into(),
            Json::Arr(vec![s(self.before.as_str()), s(self.after.as_str())]),
        ));
        if let Some(d) = &self.def {
            kv.push(("def".into(), d.to_json()));
        }
        Json::Obj(kv)
    }
}

/// One `DIAG` message.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Diag {
    /// The compile's id.
    pub id: i64,
    /// 0-based order within the compile.
    pub seq: i64,
    pub severity: Option<Severity>,
    /// Stable: `tex/…`, `latex/…`, `package/<name>/…`, `class/<name>/…`,
    /// `pdftex/…`, `engine/…` (§6.7).
    pub code: String,
    /// `tex`, `latex`, `package`, `class`, `pdftex`, `engine`.
    pub origin: String,
    /// The package or class that reported it.
    pub package: Option<String>,
    /// The first line of the message, as TeX printed it.
    pub message: String,
    /// The rest of the message, if any.
    pub detail: Option<String>,
    pub file: Option<String>,
    pub line: Option<i64>,
    /// 0-based byte column in `line`: where TeX's context line splits.
    pub col: Option<i64>,
    /// Byte columns `[from, to)` in `line`: the token before the split.
    pub range: Option<(i64, i64)>,
    /// Byte offset of `col` in the file.
    pub offset: Option<i64>,
    /// The display-list span of (file, line).
    pub span: Option<i64>,
    /// Where the material ends (box reports: the last character).
    pub end: Option<Loc>,
    /// TeX's line range (box reports: "at lines a--b").
    pub lines: Option<(i64, i64)>,
    pub trace: Vec<Frame>,
    pub help: Vec<String>,
    /// TeX stopped (emergency stop, capacity exceeded).
    pub fatal: bool,
    /// Reported while `\output` was active.
    pub output: bool,
    /// From the engine's own record (`true`) or read from its terminal
    /// output only (`false`: file and line at best).
    pub exact: bool,
}

fn push_opt_str(kv: &mut Vec<(String, Json)>, k: &str, v: &Option<String>) {
    if let Some(v) = v {
        kv.push((k.to_string(), s(v.as_str())));
    }
}

fn push_opt_int(kv: &mut Vec<(String, Json)>, k: &str, v: Option<i64>) {
    if let Some(v) = v {
        kv.push((k.to_string(), Json::Int(v)));
    }
}

fn pair(j: Option<&Json>) -> Option<(i64, i64)> {
    let a = j?.as_array()?;
    Some((a.first()?.as_i64()?, a.get(1)?.as_i64()?))
}

impl Diag {
    /// Decode a `DIAG` body. Unknown keys are ignored; a missing
    /// `message` is the only error.
    pub fn from_json(j: &Json) -> Result<Diag, String> {
        let message = j
            .str_field("message")
            .ok_or("DIAG without message")?
            .to_string();
        Ok(Diag {
            id: j.int_field("id").unwrap_or(0),
            seq: j.int_field("seq").unwrap_or(0),
            severity: j.str_field("severity").and_then(Severity::parse),
            code: j.str_field("code").unwrap_or_default().to_string(),
            origin: j.str_field("origin").unwrap_or_default().to_string(),
            package: j.str_field("package").map(str::to_string),
            message,
            detail: j.str_field("detail").map(str::to_string),
            file: j.str_field("file").map(str::to_string),
            line: j.int_field("line"),
            col: j.int_field("col"),
            range: pair(j.get("range")),
            offset: j.int_field("offset"),
            span: j.int_field("span"),
            end: j.get("end").map(Loc::from_json),
            lines: pair(j.get("lines")),
            trace: j
                .get("trace")
                .and_then(Json::as_array)
                .unwrap_or(&[])
                .iter()
                .map(Frame::from_json)
                .collect(),
            help: j
                .get("help")
                .and_then(Json::as_array)
                .unwrap_or(&[])
                .iter()
                .filter_map(|h| h.as_str().map(str::to_string))
                .collect(),
            fatal: j.get("fatal").and_then(Json::as_bool).unwrap_or(false),
            output: j.get("output").and_then(Json::as_bool).unwrap_or(false),
            exact: j.get("exact").and_then(Json::as_bool).unwrap_or(false),
        })
    }

    /// Decode a `DIAG` frame body.
    pub fn decode(body: &[u8]) -> Result<Diag, String> {
        let t = std::str::from_utf8(body).map_err(|e| e.to_string())?;
        Diag::from_json(&Json::parse(t)?)
    }

    pub fn to_json(&self) -> Json {
        let mut kv = vec![
            ("id".to_string(), Json::Int(self.id)),
            ("seq".to_string(), Json::Int(self.seq)),
        ];
        if let Some(sv) = self.severity {
            kv.push(("severity".into(), s(sv.as_str())));
        }
        kv.push(("code".into(), s(self.code.as_str())));
        kv.push(("origin".into(), s(self.origin.as_str())));
        push_opt_str(&mut kv, "package", &self.package);
        kv.push(("message".into(), s(self.message.as_str())));
        push_opt_str(&mut kv, "detail", &self.detail);
        push_opt_str(&mut kv, "file", &self.file);
        push_opt_int(&mut kv, "line", self.line);
        push_opt_int(&mut kv, "col", self.col);
        if let Some((a, b)) = self.range {
            kv.push(("range".into(), Json::Arr(vec![Json::Int(a), Json::Int(b)])));
        }
        push_opt_int(&mut kv, "offset", self.offset);
        push_opt_int(&mut kv, "span", self.span);
        if let Some(e) = &self.end {
            kv.push(("end".into(), e.to_json()));
        }
        if let Some((a, b)) = self.lines {
            kv.push(("lines".into(), Json::Arr(vec![Json::Int(a), Json::Int(b)])));
        }
        if !self.trace.is_empty() {
            kv.push((
                "trace".into(),
                Json::Arr(self.trace.iter().map(Frame::to_json).collect()),
            ));
        }
        if !self.help.is_empty() {
            kv.push((
                "help".into(),
                Json::Arr(self.help.iter().map(|h| s(h.as_str())).collect()),
            ));
        }
        if self.fatal {
            kv.push(("fatal".into(), Json::Bool(true)));
        }
        if self.output {
            kv.push(("output".into(), Json::Bool(true)));
        }
        kv.push(("exact".into(), Json::Bool(self.exact)));
        Json::Obj(kv)
    }

    /// The body of a `DIAG` frame.
    pub fn encode(&self) -> Vec<u8> {
        self.to_json().to_string().into_bytes()
    }

    /// The macro frames of the trace, innermost first: "error occurred in
    /// this call of `\name`" rows for a Problems panel.
    pub fn macros(&self) -> impl Iterator<Item = &Frame> {
        self.trace.iter().filter(|f| f.kind == "macro")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Diag {
        Diag {
            id: 7,
            seq: 2,
            severity: Some(Severity::Error),
            code: "tex/undefined-control-sequence".into(),
            origin: "tex".into(),
            package: None,
            message: "Undefined control sequence.".into(),
            detail: None,
            file: Some("/p/main.tex".into()),
            line: Some(12),
            col: Some(20),
            range: Some((16, 20)),
            offset: Some(345),
            span: Some(9),
            end: None,
            lines: None,
            trace: vec![
                Frame {
                    kind: "macro".into(),
                    name: Some("\\mycmd".into()),
                    loc: None,
                    before: "\\mycmd #1->\\undefined ".into(),
                    after: "#1".into(),
                    def: Some(Loc {
                        file: Some("/p/main.tex".into()),
                        line: Some(3),
                        col: None,
                        span: None,
                    }),
                },
                Frame {
                    kind: "file".into(),
                    name: None,
                    loc: Some(Loc {
                        file: Some("/p/main.tex".into()),
                        line: Some(12),
                        col: Some(20),
                        span: Some(9),
                    }),
                    before: "Some text \\mycmd{x}".into(),
                    after: " more".into(),
                    def: None,
                },
            ],
            help: vec!["The control sequence at the end of the top line".into()],
            fatal: false,
            output: false,
            exact: true,
        }
    }

    #[test]
    fn round_trips() {
        let d = sample();
        assert_eq!(Diag::decode(&d.encode()).unwrap(), d);
        assert_eq!(d.macros().count(), 1);
    }

    #[test]
    fn tolerates_unknown_and_missing_keys() {
        let j = Json::parse(r#"{"message":"x","later":{"a":1},"severity":"fatal?"}"#).unwrap();
        let d = Diag::from_json(&j).unwrap();
        assert_eq!(d.message, "x");
        assert_eq!(d.severity, None);
        assert!(Diag::from_json(&Json::parse("{}").unwrap()).is_err());
    }
}
