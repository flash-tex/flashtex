//! The vertical skips and head separator of declared theorem styles.
//!
//! amsthm's `\@thm` sets `\@topsep\thm@preskip`, `\@topsepadd\thm@postskip`
//! and ends the head with `\hskip\thm@headsep`; a style sets all three
//! (amsthm.sty 129-178, 215-270):
//!
//! * `plain` and `definition` take `\topsep` both ways; `remark` takes half
//!   of it (`\thm@preskip\topsep \divide\thm@preskip\tw@`);
//! * `\newtheoremstyle{name}{#2}{#3}...{#8}{#9}`: `#2`/`#3` are the skips
//!   (empty is `\topsep`, a bare number is points), `#8` the separator: a
//!   space token is `\fontdimen2` of the body font (rigid), `\newline` is a
//!   line break and zero glue, anything else is a glue;
//! * thmtools `\declaretheoremstyle[spaceabove=..,spacebelow=..,
//!   postheadspace=..]` builds a `\newtheoremstyle` with 3pt/3pt and a
//!   space by default; `\declaretheorem[style=..]` and mdframed's
//!   `\newmdtheoremenv` declare environments like `\newtheorem`.
//!
//! The compiler sets the head's text and fonts from the same declarations
//! (`flashtex_compiler::theorems::CustomTheoremStyle`); this module reads
//! only what the page builder and the head glue need. It scans the sources
//! in document order, as the declarations are executed.

use std::collections::HashMap;
use std::rc::Rc;

use crate::style::Skip;

/// What a declared style asks of the head separator (`#8`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HeadSep {
    /// amsthm's `\thm@headsep`, `5pt plus1pt minus1pt`.
    Default,
    /// A space token: the body font's interword width, rigid.
    Space,
    /// `\newline`: the body starts on the next line, no glue.
    Newline,
    /// An explicit glue, as written (`.5em`, `1em`, `5pt plus 1pt`).
    Glue(String),
}

/// One style's page-builder-facing parts.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThmStyle {
    /// `\thm@preskip`; `None` is `\topsep`.
    above: Option<SkipSpec>,
    /// `\thm@postskip`; `None` is `\topsep`.
    below: Option<SkipSpec>,
    pub(crate) headsep: HeadSep,
    /// The body font is italic (for a `.5em` or a space in it).
    body_italic: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum SkipSpec {
    /// Half of `\topsep` (`remark`).
    HalfTopsep,
    /// A glue in points.
    Points(Skip),
    /// A multiple of `\topsep` (`\topsep`, `.5\topsep`).
    Topsep(f64),
    /// A multiple of `\baselineskip`.
    Baselineskip(f64),
}

impl ThmStyle {
    fn builtin(name: &str) -> Option<ThmStyle> {
        let half = matches!(name, "remark").then_some(SkipSpec::HalfTopsep);
        matches!(name, "plain" | "definition" | "remark").then(|| ThmStyle {
            above: half.clone(),
            below: half,
            headsep: HeadSep::Default,
            body_italic: name == "plain",
        })
    }

    fn resolve(spec: &Option<SkipSpec>, topsep: Skip, baselineskip_pt: f64) -> Skip {
        match spec {
            None => topsep,
            Some(SkipSpec::HalfTopsep) => Skip::new(topsep.natural / 2.0, topsep.stretch / 2.0, topsep.shrink / 2.0),
            Some(SkipSpec::Points(skip)) => *skip,
            Some(SkipSpec::Topsep(f)) => Skip::new(topsep.natural * f, topsep.stretch * f, topsep.shrink * f),
            Some(SkipSpec::Baselineskip(f)) => Skip::fixed(baselineskip_pt * f),
        }
    }

    /// `(\thm@preskip, \thm@postskip)`.
    pub(crate) fn skips(&self, topsep: Skip, baselineskip_pt: f64) -> (Skip, Skip) {
        (Self::resolve(&self.above, topsep, baselineskip_pt), Self::resolve(&self.below, topsep, baselineskip_pt))
    }

    /// The separator glue at body size `size_pt`, `None` for amsthm's
    /// default. `em` is the body font's quad: 10.22pt for cmti10 at 10pt,
    /// the size itself for the upright faces; a space is the body font's
    /// `\fontdimen2` (cmr10 3.33333pt, cmti10 3.57774pt at 10pt).
    pub(crate) fn headsep_pt(&self, size_pt: f64) -> Option<(f64, f64, f64)> {
        let em = if self.body_italic { size_pt * 1.02222 } else { size_pt };
        match &self.headsep {
            HeadSep::Default => None,
            HeadSep::Newline => Some((0.0, 0.0, 0.0)),
            HeadSep::Space => Some((if self.body_italic { 0.357774 } else { 0.333333 } * size_pt, 0.0, 0.0)),
            HeadSep::Glue(text) => parse_glue(text, em).map(|s| (s.natural, s.stretch, s.shrink)),
        }
    }
}

/// Environment name → its style, for every theorem-like environment
/// declared in a declared style (builtin ones are left out: the callers'
/// defaults already cover them, except `remark`'s halved skips).
#[derive(Debug, Default)]
pub(crate) struct ThmStyles {
    envs: HashMap<String, ThmStyle>,
}

impl ThmStyles {
    pub(crate) fn get(&self, env: &str) -> Option<&ThmStyle> {
        self.envs.get(env)
    }
}

thread_local! {
    static CACHE: std::cell::RefCell<Option<(Vec<(usize, usize)>, Rc<ThmStyles>)>> = const { std::cell::RefCell::new(None) };
}

/// [`scan`] of `texts`, memoised on the texts' identity (the adapter asks
/// once per theorem paragraph).
pub(crate) fn styles(texts: &[&str]) -> Rc<ThmStyles> {
    let key: Vec<(usize, usize)> = texts.iter().map(|t| (t.as_ptr() as usize, t.len())).collect();
    CACHE.with(|cache| {
        if let Some((k, v)) = cache.borrow().as_ref() {
            if *k == key {
                return v.clone();
            }
        }
        let value = Rc::new(scan(texts));
        *cache.borrow_mut() = Some((key, value.clone()));
        value
    })
}

/// Walks every declaration in document order.
pub(crate) fn scan(texts: &[&str]) -> ThmStyles {
    let mut named: HashMap<String, ThmStyle> = HashMap::new();
    let mut current = ThmStyle::builtin("plain").expect("plain");
    let mut out = ThmStyles::default();
    for text in texts {
        let mut at = 0;
        while let Some((name, after)) = next_command(text, at) {
            at = after;
            match name {
                "theoremstyle" => {
                    if let Some((arg, end)) = group(text, at) {
                        at = end;
                        let arg = arg.trim();
                        // amsthm: an unknown style is `plain`.
                        current = named.get(arg).cloned().or_else(|| ThmStyle::builtin(arg)).unwrap_or_else(|| ThmStyle::builtin("plain").expect("plain"));
                    }
                }
                "newtheoremstyle" => {
                    let mut args = Vec::new();
                    let mut end = at;
                    for _ in 0..9 {
                        match group(text, end) {
                            Some((arg, e)) => {
                                args.push(arg);
                                end = e;
                            }
                            None => break,
                        }
                    }
                    if args.len() == 9 {
                        at = end;
                        named.insert(
                            args[0].trim().to_string(),
                            ThmStyle {
                                above: skip_spec(&args[1]),
                                below: skip_spec(&args[2]),
                                headsep: headsep_spec(&args[7]),
                                body_italic: italic(&args[3]),
                            },
                        );
                    }
                }
                "declaretheoremstyle" => {
                    let (keys, end) = bracket(text, at).unwrap_or(("", at));
                    if let Some((arg, end)) = group(text, end) {
                        at = end;
                        let mut style = ThmStyle {
                            above: Some(SkipSpec::Points(Skip::fixed(3.0))),
                            below: Some(SkipSpec::Points(Skip::fixed(3.0))),
                            headsep: HeadSep::Space,
                            body_italic: false,
                        };
                        apply_keys(&mut style, keys);
                        named.insert(arg.trim().to_string(), style);
                    }
                }
                "newtheorem" | "newmdtheoremenv" => {
                    let mut end = at;
                    if text[end..].starts_with('*') {
                        end += 1;
                    }
                    if name == "newmdtheoremenv" {
                        end = bracket(text, end).map_or(end, |(_, e)| e);
                    }
                    if let Some((env, e)) = group(text, end) {
                        at = e;
                        out.envs.insert(env.trim().to_string(), current.clone());
                    }
                }
                "declaretheorem" => {
                    let (keys, end) = bracket(text, at).unwrap_or(("", at));
                    if let Some((envs, end)) = group(text, end) {
                        let (after_keys, end) = bracket(text, end).unwrap_or(("", end));
                        at = end;
                        let mut style = current.clone();
                        for key in [keys, after_keys] {
                            for (k, v) in key_values(key) {
                                if k == "style" {
                                    if let Some(s) = named.get(v.trim()).cloned().or_else(|| ThmStyle::builtin(v.trim())) {
                                        style = s;
                                    }
                                }
                            }
                        }
                        apply_keys(&mut style, keys);
                        apply_keys(&mut style, after_keys);
                        for env in envs.split(',').map(str::trim).filter(|e| !e.is_empty()) {
                            out.envs.insert(env.to_string(), style.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Every theorem-like environment name `\declaretheorem` and
/// `\newmdtheoremenv` declare (the `\newtheorem` ones are the adapter's).
pub(crate) fn declared_environments(texts: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for text in texts {
        let mut at = 0;
        while let Some((name, after)) = next_command(text, at) {
            at = after;
            match name {
                "declaretheorem" => {
                    let end = bracket(text, at).map_or(at, |(_, e)| e);
                    if let Some((envs, e)) = group(text, end) {
                        at = e;
                        out.extend(envs.split(',').map(str::trim).filter(|e| !e.is_empty()).map(str::to_string));
                    }
                }
                "newmdtheoremenv" => {
                    let end = bracket(text, at).map_or(at, |(_, e)| e);
                    if let Some((env, e)) = group(text, end) {
                        at = e;
                        out.push(env.trim().to_string());
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn apply_keys(style: &mut ThmStyle, keys: &str) {
    for (k, v) in key_values(keys) {
        match k {
            "spaceabove" => style.above = skip_spec(v),
            "spacebelow" => style.below = skip_spec(v),
            "postheadspace" => style.headsep = headsep_spec(v),
            "bodyfont" => style.body_italic = italic(v),
            _ => {}
        }
    }
}

fn italic(fonts: &str) -> bool {
    let mut it = false;
    for word in fonts.split('\\').skip(1) {
        let word: String = word.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        match word.as_str() {
            // cmsl10's space and quad are cmr10's; only the italic face
            // differs.
            "itshape" | "it" | "em" => it = true,
            "slshape" | "sl" => it = false,
            "upshape" | "normalfont" | "scshape" => it = false,
            _ => {}
        }
    }
    it
}

fn skip_spec(text: &str) -> Option<SkipSpec> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    for (name, make) in [("\\topsep", SkipSpec::Topsep as fn(f64) -> SkipSpec), ("\\baselineskip", SkipSpec::Baselineskip)] {
        if let Some(factor) = text.strip_suffix(name) {
            let factor = factor.trim();
            let f = if factor.is_empty() { 1.0 } else { factor.parse().ok()? };
            return Some(make(f));
        }
    }
    // `\@defaultunits`: a bare number is points.
    if let Ok(pt) = text.parse::<f64>() {
        return Some(SkipSpec::Points(Skip::fixed(pt)));
    }
    parse_glue(text, 10.0).map(SkipSpec::Points)
}

fn headsep_spec(text: &str) -> HeadSep {
    match text.trim() {
        "" => HeadSep::Space,
        "\\newline" => HeadSep::Newline,
        t => HeadSep::Glue(t.to_string()),
    }
}

/// `<dimen> [plus <dimen>] [minus <dimen>]` in pt/em/bp/mm/cm/in/ex.
fn parse_glue(text: &str, em: f64) -> Option<Skip> {
    let text = text.trim();
    let (natural, rest) = match text.split_once(" plus") {
        Some((n, r)) => (n, Some(r)),
        None => (text, None),
    };
    let (natural, minus) = match natural.split_once(" minus") {
        Some((n, m)) => (n, Some(m)),
        None => (natural, rest.and_then(|r| r.split_once(" minus").map(|(_, m)| m))),
    };
    let plus = rest.map(|r| r.split_once(" minus").map_or(r, |(p, _)| p));
    Some(Skip::new(
        dimen(natural, em)?,
        plus.and_then(|p| dimen(p, em)).unwrap_or(0.0),
        minus.and_then(|m| dimen(m, em)).unwrap_or(0.0),
    ))
}

fn dimen(text: &str, em: f64) -> Option<f64> {
    let text = text.trim();
    let split = text.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let number = number.trim();
    let value: f64 = if number.is_empty() || number == "+" { 1.0 } else if number == "-" { -1.0 } else { number.parse().ok()? };
    let unit_pt = match unit.trim() {
        "pt" | "" => 1.0,
        "em" => em,
        "ex" => em * 0.430555,
        "bp" => 72.27 / 72.0,
        "mm" => 72.27 / 25.4,
        "cm" => 72.27 / 2.54,
        "in" => 72.27,
        "pc" => 12.0,
        _ => return None,
    };
    Some(value * unit_pt)
}

fn key_values(keys: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let bytes = keys.as_bytes();
    for i in 0..=bytes.len() {
        let c = bytes.get(i).copied().unwrap_or(b',');
        match c {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b',' if depth == 0 => {
                let part = keys[start..i].trim();
                if !part.is_empty() {
                    let (k, v) = part.split_once('=').unwrap_or((part, ""));
                    let v = v.trim();
                    let v = v.strip_prefix('{').and_then(|v| v.strip_suffix('}')).unwrap_or(v);
                    out.push((k.trim(), v));
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    out
}

/// The next control word at or after `at` (outside comments): its name and
/// the byte after it.
fn next_command(text: &str, mut at: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            b'\\' => {
                let start = at + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                    end += 1;
                }
                if end > start {
                    return Some((&text[start..end], end));
                }
                at = start + 1;
            }
            _ => at += 1,
        }
    }
    None
}

/// Skips spaces, newlines and `%` comments.
fn skip_filler(text: &str, mut at: usize) -> usize {
    let bytes = text.as_bytes();
    while at < bytes.len() {
        match bytes[at] {
            b' ' | b'\t' | b'\n' | b'\r' => at += 1,
            b'%' => {
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            _ => break,
        }
    }
    at
}

/// A `{...}` group after filler: its content with comments removed, and
/// the byte after it.
fn group(text: &str, at: usize) -> Option<(String, usize)> {
    let start = skip_filler(text, at);
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut out = String::new();
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'\\' if i + 1 < bytes.len() => {
                if depth >= 1 {
                    out.push_str(&text[i..i + 2]);
                }
                i += 2;
                continue;
            }
            b'{' => {
                depth += 1;
                if depth > 1 {
                    out.push('{');
                }
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((out, i + 1));
                }
                out.push('}');
            }
            _ => {
                if depth >= 1 {
                    let ch_end = (i + 1..=bytes.len()).find(|&e| text.is_char_boundary(e)).unwrap_or(bytes.len());
                    out.push_str(&text[i..ch_end]);
                    i = ch_end;
                    continue;
                }
            }
        }
        i += 1;
    }
    None
}

/// An optional `[...]` after filler (braces honoured): its content and the
/// byte after it.
fn bracket(text: &str, at: usize) -> Option<(&str, usize)> {
    let start = skip_filler(text, at);
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    for (i, &c) in bytes.iter().enumerate().skip(start + 1) {
        match c {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b']' if depth == 0 => return Some((&text[start + 1..i], i + 1)),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtheoremstyle_skips_and_separators() {
        let src = "\\newtheoremstyle{note}% name\n  {3pt}%\n  {3pt}%\n  {}{}{\\itshape}{:}{.5em}{}\n\
                   \\newtheoremstyle{break}{9pt}{9pt}{\\itshape}{}{\\bfseries}{.}{\\newline}{}\n\
                   \\newtheoremstyle{spc}{}{}{\\slshape}{}{\\scshape}{.}{ }{}\n\
                   \\theoremstyle{note}\\newtheorem{note}{Note}\n\\theoremstyle{break}\\newtheorem{brk}{Break}\n\
                   \\theoremstyle{spc}\\newtheorem{spc}{Spaced}\n\\theoremstyle{remark}\\newtheorem{rem}{Remark}\n";
        let s = scan(&[src]);
        let topsep = Skip::new(8.0, 2.0, 4.0);
        assert_eq!(s.get("note").unwrap().skips(topsep, 12.0), (Skip::fixed(3.0), Skip::fixed(3.0)));
        assert_eq!(s.get("note").unwrap().headsep_pt(10.0), Some((5.0, 0.0, 0.0)));
        assert_eq!(s.get("brk").unwrap().headsep_pt(10.0), Some((0.0, 0.0, 0.0)));
        assert_eq!(s.get("spc").unwrap().skips(topsep, 12.0), (topsep, topsep));
        assert_eq!(s.get("rem").unwrap().skips(topsep, 12.0), (Skip::new(4.0, 1.0, 2.0), Skip::new(4.0, 1.0, 2.0)));
    }

    #[test]
    fn thmtools_declarations() {
        let src = "\\declaretheoremstyle[spaceabove=6pt, spacebelow=6pt, headfont=\\bfseries, postheadspace=1em]{mine}\n\
                   \\declaretheorem[style=mine, name=Theorem]{theorem}\n\\declaretheorem{lemma,claim}\n";
        let s = scan(&[src]);
        assert_eq!(s.get("theorem").unwrap().skips(Skip::fixed(8.0), 12.0), (Skip::fixed(6.0), Skip::fixed(6.0)));
        assert_eq!(s.get("theorem").unwrap().headsep_pt(10.0), Some((10.0, 0.0, 0.0)));
        assert!(s.get("claim").is_some());
        assert_eq!(declared_environments(&[src]), vec!["theorem", "lemma", "claim"]);
    }
}
