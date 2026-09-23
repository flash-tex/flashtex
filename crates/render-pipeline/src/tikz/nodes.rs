//! Node text the pinned TikZ reader cannot set: `$...$`/`\(...\)` in a
//! node's `{text}`.
//!
//! The reader measures node text through the pipeline's `TextMeasurer` and
//! hands back plain glyph runs; it reads `$` as "set this in italics" and
//! drops every control word, so `$x^2$` came out as the italic letters
//! `x^2` and `$\alpha$` as nothing. pgf sets the text in an `\hbox` with
//! TeX doing the typesetting, so the width, height and depth the node is
//! sized by, and the glyphs painted, are the formula's.
//!
//! The pipeline takes those texts over before the reader sees them: each
//! math-bearing `{text}` group of the picture body is typeset here as an
//! `\hbox` through the compiler (an isolated parse of the document keeping
//! just those bytes, as float captions are parsed) and replaced in the body
//! by a placeholder word of the same byte length, which the measurer answers
//! with the box's metrics and the painter replaces with the box. Byte
//! offsets are kept, so every span the reader reports still indexes the
//! document.

use flashtex_vector_graphics::tikz::text::matching;

/// Byte ranges (interiors, `{`..`}` excluded) of the `{text}` arguments of
/// `node`/`\node` operations in `s`, a picture body or statement with its
/// comments blanked, in source order. Mirrors the reader's `node_spec`:
/// after `node`, any number of `[options]`, `(name)` and `at (coordinate)`
/// precede the group.
pub fn node_text_groups(s: &str) -> Vec<(usize, usize)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = s[i..].find("node") {
        let at = i + rel;
        i = at + 4;
        // A whole word: `\node`, ` node`, `)node`, not `nodes`/`anode`.
        let before = b.get(at.wrapping_sub(1)).copied();
        if before.is_some_and(|c| c.is_ascii_alphabetic() || c == b'@') || b.get(at + 4).is_some_and(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let mut k = at + 4;
        loop {
            k = skip_ws(s, k);
            match b.get(k) {
                Some(b'[') | Some(b'(') => match matching(s, k) {
                    Some(e) => k = e,
                    None => break,
                },
                Some(b'a') if s[k..].starts_with("at") && !b.get(k + 2).is_some_and(|c| c.is_ascii_alphabetic()) => {
                    k = skip_ws(s, k + 2);
                    // `at (x,y)`, `at ($(a)+(b)$)`: one parenthesised group.
                    match b.get(k) {
                        Some(b'(') => match matching(s, k) {
                            Some(e) => k = e,
                            None => break,
                        },
                        _ => break,
                    }
                }
                Some(b'{') => {
                    if let Some(e) = matching(s, k) {
                        out.push((k + 1, e - 1));
                        i = e;
                    }
                    break;
                }
                _ => break,
            }
        }
    }
    out
}

/// Whether a node text needs the typesetter: inline math (`$`, `\(`),
/// which the reader would set as italic letters.
pub fn needs_typesetting(raw: &str) -> bool {
    raw.contains('$') || raw.contains("\\(") || raw.contains("\\ensuremath")
}

/// The placeholder word standing for the `n`th taken-over text: one
/// private-use character (three bytes), padded with spaces to `len` bytes
/// so the body keeps its byte offsets. The reader trims the padding. `None`
/// when the group is too short to hold the character.
pub fn placeholder(n: usize, len: usize) -> Option<String> {
    let c = char::from_u32(0xE000 + u32::try_from(n).ok()?)?;
    let mut s = String::with_capacity(len);
    s.push(c);
    if s.len() > len {
        return None;
    }
    while s.len() < len {
        s.push(' ');
    }
    Some(s)
}

/// The placeholder index a node text carries, if it is one.
pub fn placeholder_index(text: &str) -> Option<usize> {
    let mut chars = text.trim().chars();
    let c = chars.next()?;
    if chars.next().is_some() || !('\u{E000}'..='\u{F8FF}').contains(&c) {
        return None;
    }
    Some((c as u32 - 0xE000) as usize)
}

fn skip_ws(s: &str, i: usize) -> usize {
    i + s[i..].len() - s[i..].trim_start().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_node_text_groups() {
        let s = "\\draw (0,0) -- (1,1) node[midway,above] {hi} node (n) at (2,2) {$x^2$};\n\\node[draw] (a) {A};\n\\coordinate (c) at (0,0);\n\\node at ($(a)+(1,0)$) {b};";
        let g = node_text_groups(s);
        let texts: Vec<&str> = g.iter().map(|(a, b)| &s[*a..*b]).collect();
        assert_eq!(texts, ["hi", "$x^2$", "A", "b"]);
    }

    #[test]
    fn placeholders_round_trip_and_keep_length() {
        let p = placeholder(3, 5).unwrap();
        assert_eq!(p.len(), 5);
        assert_eq!(placeholder_index(&p), Some(3));
        assert_eq!(placeholder_index("x"), None);
        assert!(placeholder(0, 2).is_none());
    }
}
