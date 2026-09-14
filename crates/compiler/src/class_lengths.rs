//! Class lengths the expansion engine leaves undefined (`\textwidth` and
//! friends) so the typesetter still sees them. `scan_dimen` needs their
//! values as `<internal dimen>`; this module fills that table from
//! `flashtex-class-geometry` for article/report/book, including geometry
//! and earlier preamble assignments to those names.

use std::collections::HashMap;

use flashtex_class_geometry::{resolve, DocumentSetup, ResolvedDocument, Sp};
use flashtex_tex_expansion::Engine;

/// Names `scan_dimen` treats as pass-through class lengths.
const PASS_THROUGH: &[&str] = &[
    "textwidth",
    "textheight",
    "paperwidth",
    "paperheight",
    "parindent",
    "linewidth",
    "columnwidth",
    "hsize",
];

pub(crate) fn install(engine: &mut Engine) {
    match DocumentSetup::from_preamble(engine.source()) {
        Some(setup) => {
            let doc = resolve(&setup);
            let mut map = table_from_resolved(&doc);
            apply_preamble_length_assignments(engine.source(), &mut map);
            engine.set_pass_through_dimens(map, true);
        }
        None => engine.set_pass_through_exact(false),
    }
}

/// Bytes up to `\begin{document}` (or the whole source). Used to decide
/// when an incremental edit must rebuild the pass-through table.
pub(crate) fn class_preamble(text: &str) -> &str {
    match text.find("\\begin{document}") {
        Some(i) => &text[..i],
        None => text,
    }
}

fn table_from_resolved(doc: &ResolvedDocument) -> HashMap<String, i64> {
    let p = &doc.params;
    let col = p.columnwidth(doc.flags.twocolumn).0;
    let mut m = HashMap::new();
    m.insert("textwidth".into(), p.textwidth.0);
    m.insert("textheight".into(), p.textheight.0);
    m.insert("paperwidth".into(), p.paperwidth.0);
    m.insert("paperheight".into(), p.paperheight.0);
    m.insert("parindent".into(), p.parindent.0);
    m.insert("linewidth".into(), col);
    m.insert("columnwidth".into(), col);
    m.insert("hsize".into(), col);
    m
}

fn apply_preamble_length_assignments(source: &str, map: &mut HashMap<String, i64>) {
    let src = strip_comments(source);
    let pre = class_preamble(&src);
    let mut i = 0;
    while let Some(off) = pre[i..].find('\\') {
        let at = i + off + 1;
        let name: String = pre[at..]
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let mut j = at + name.len();
        match name.as_str() {
            "setlength" | "addtolength" => {
                let add = name == "addtolength";
                let _ = read_group(pre, &mut j, '[', ']');
                let target = read_group(pre, &mut j, '{', '}');
                let value = read_group(pre, &mut j, '{', '}');
                if let (Some(t), Some(v)) = (target, value) {
                    let t = t.trim().trim_start_matches('\\');
                    if map.contains_key(t) {
                        if let Some(sp) = parse_length_value(&v, map) {
                            if add {
                                *map.get_mut(t).unwrap() += sp;
                            } else {
                                map.insert(t.to_string(), sp);
                            }
                        }
                    }
                }
            }
            n if PASS_THROUGH.contains(&n) => {
                let mut k = j;
                while k < pre.len() && pre.as_bytes()[k].is_ascii_whitespace() {
                    k += 1;
                }
                if pre.as_bytes().get(k) == Some(&b'=') {
                    k += 1;
                    while k < pre.len() && pre.as_bytes()[k].is_ascii_whitespace() {
                        k += 1;
                    }
                }
                let start = k;
                while k < pre.len() {
                    let c = pre[k..].chars().next().unwrap();
                    if c == '\\' || c == '%' || c == '\n' || c == '{' {
                        break;
                    }
                    k += c.len_utf8();
                }
                if let Some(sp) = parse_length_value(pre[start..k].trim(), map) {
                    map.insert(n.to_string(), sp);
                    j = k;
                }
            }
            _ => {}
        }
        i = if j > at { j } else { at };
    }
}

fn parse_length_value(raw: &str, map: &HashMap<String, i64>) -> Option<i64> {
    let s = raw.trim().trim_start_matches('=').trim();
    let s = s.split("plus").next().unwrap_or(s);
    let s = s.split("minus").next().unwrap_or(s).trim();
    if s.is_empty() {
        return None;
    }
    if let Some(bs) = s.find('\\') {
        let factor = s[..bs].trim();
        let name = s[bs + 1..].trim();
        if !name.chars().all(|c| c.is_ascii_alphabetic() || c == '@') {
            return None;
        }
        let base = *map.get(name)?;
        let factor = if factor.is_empty() || factor == "+" {
            "1"
        } else if factor == "-" {
            "-1"
        } else {
            factor
        };
        return Some(Sp(base).scaled(factor)?.0);
    }
    Sp::parse(s).map(|sp| sp.0)
}

fn strip_comments(s: &str) -> String {
    s.lines()
        .map(|line| {
            let b = line.as_bytes();
            let mut k = 0;
            while k < b.len() {
                if b[k] == b'\\' {
                    k += 2;
                    continue;
                }
                if b[k] == b'%' {
                    return &line[..k];
                }
                k += 1;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_group(s: &str, j: &mut usize, open: char, close: char) -> Option<String> {
    let rest = &s[*j..];
    let trimmed = rest.trim_start();
    if !trimmed.starts_with(open) {
        return None;
    }
    let start = *j + (rest.len() - trimmed.len()) + 1;
    let mut depth = 1i32;
    for (k, c) in s[start..].char_indices() {
        if c == open {
            depth += 1;
        }
        if c == close {
            depth -= 1;
            if depth == 0 {
                *j = start + k + 1;
                return Some(s[start..start + k].to_string());
            }
        }
    }
    None
}
