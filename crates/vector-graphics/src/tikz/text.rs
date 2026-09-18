//! Small text utilities shared by the TikZ reader: balanced groups,
//! top-level splitting, comments and macro substitution.

use std::borrow::Cow;
use std::collections::HashMap;

/// Index just past the delimiter matching `s[open_at]` (`{`, `[` or `(`),
/// honouring nesting of all three bracket kinds and `\{`-style escapes.
pub fn matching(s: &str, open_at: usize) -> Option<usize> {
    let b = s.as_bytes();
    let (open, close) = match b.get(open_at)? {
        b'{' => (b'{', b'}'),
        b'[' => (b'[', b']'),
        b'(' => (b'(', b')'),
        _ => return None,
    };
    let mut depth = 0usize;
    let mut brace = 0usize;
    let mut i = open_at;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' {
            i += 2;
            continue;
        }
        if open != b'{' {
            // Inside [..] or (..), braces protect their content.
            if c == b'{' {
                brace += 1;
            } else if c == b'}' {
                brace = brace.saturating_sub(1);
            } else if brace == 0 {
                if c == open {
                    depth += 1;
                } else if c == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
            }
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i + 1);
            }
        }
        i += 1;
    }
    None
}

/// Splits on `sep` outside `{}`, `[]` and `()`.
pub fn split_top(s: &str, sep: u8) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'{' | b'[' | b'(' => depth += 1,
            b'}' | b']' | b')' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&s[start.min(s.len())..]);
    out
}

/// Position of the first `sep` outside groups, if any.
pub fn find_top(s: &str, sep: u8) -> Option<usize> {
    let parts = split_top(s, sep);
    if parts.len() > 1 { Some(parts[0].len()) } else { None }
}

/// Removes one pair of enclosing braces.
pub fn strip_braces(s: &str) -> &str {
    let t = s.trim();
    if t.starts_with('{') && matching(t, 0) == Some(t.len()) {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}

fn next_unescaped_percent(bytes: &[u8], mut from: usize) -> Option<usize> {
    while let Some(rel) = bytes[from..].iter().position(|&b| b == b'%') {
        let at = from + rel;
        let mut slash = at;
        while slash > 0 && bytes[slash - 1] == b'\\' {
            slash -= 1;
        }
        if (at - slash) % 2 == 0 {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// Replaces `%` comments (to end of line) with spaces, keeping byte offsets.
pub fn blank_comments(s: &str) -> Cow<'_, str> {
    let bytes = s.as_bytes();
    let Some(first_comment) = next_unescaped_percent(bytes, 0) else {
        return Cow::Borrowed(s);
    };

    let mut out = String::with_capacity(s.len());
    let mut copy_from = 0;
    let mut comment_start = first_comment;
    loop {
        out.push_str(&s[copy_from..comment_start]);
        let line_end = bytes[comment_start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |rel| comment_start + rel);
        out.extend(std::iter::repeat(' ').take(line_end - comment_start));
        if line_end == bytes.len() {
            return Cow::Owned(out);
        }

        out.push('\n');
        copy_from = line_end + 1;
        let Some(next_comment) = next_unescaped_percent(bytes, copy_from) else {
            out.push_str(&s[copy_from..]);
            return Cow::Owned(out);
        };
        comment_start = next_comment;
    }
}

/// Reads a control word at `i` (which must be `\`): returns the name and the
/// index after it (spaces after a control word are skipped, as TeX does).
pub fn control_word(s: &str, i: usize) -> Option<(&str, usize)> {
    let b = s.as_bytes();
    if b.get(i) != Some(&b'\\') {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() && b[j].is_ascii_alphabetic() {
        j += 1;
    }
    if j == i + 1 {
        return None;
    }
    let name = &s[i + 1..j];
    Some((name, j))
}

/// Substitutes macros (`\x` -> value) as whole control words. Repeats so a
/// value may itself use macros, up to a fixed depth.
pub fn substitute(s: &str, macros: &HashMap<String, String>) -> String {
    if macros.is_empty() || !s.contains('\\') {
        return s.to_string();
    }
    let mut cur = s.to_string();
    for _ in 0..8 {
        let b = cur.as_bytes();
        let mut out = String::with_capacity(cur.len());
        let mut changed = false;
        let mut i = 0;
        let mut last = 0;
        while i < b.len() {
            if b[i] == b'\\' {
                if let Some((name, j)) = control_word(&cur, i) {
                    if let Some(v) = macros.get(name) {
                        out.push_str(&cur[last..i]);
                        out.push_str(v);
                        // TeX skips the blanks after a control word.
                        let mut k = j;
                        while k < b.len() && b[k] == b' ' {
                            k += 1;
                        }
                        // Keep one space when the next char would glue
                        // onto an identifier-like value.
                        if k > j && k < b.len() && b[k].is_ascii_alphanumeric() && !v.ends_with(|c: char| c.is_ascii_digit()) {
                            out.push(' ');
                        }
                        i = k;
                        last = k;
                        changed = true;
                        continue;
                    }
                    i = j;
                    continue;
                }
                i += 2;
                continue;
            }
            i += 1;
        }
        out.push_str(&cur[last.min(cur.len())..]);
        cur = out;
        if !changed {
            break;
        }
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_and_splitting() {
        assert_eq!(matching("{a{b}c}d", 0), Some(7));
        assert_eq!(matching("(1,{(2)})x", 0), Some(9));
        assert_eq!(split_top("a=1, b={2,3}, c=(4,5)", b','), vec!["a=1", " b={2,3}", " c=(4,5)"]);
        assert_eq!(strip_braces(" {x,y} "), "x,y");
        let mut m = HashMap::new();
        m.insert("x".to_string(), "2".to_string());
        assert_eq!(substitute(r"(\x,\x*2) \xx", &m), r"(2,2*2) \xx");
    }

    #[test]
    fn blank_comments_leaves_comment_free_input_unchanged() {
        let input = "plain text\ncafé";
        let out = blank_comments(input);
        assert_eq!(out.as_ref(), input);
        assert!(matches!(&out, Cow::Borrowed(_)));
    }

    #[test]
    fn blank_comments_preserves_length() {
        let input = "draw %comment\nnext";
        let out = blank_comments(input);
        assert_eq!(out.as_ref(), format!("draw {}\nnext", " ".repeat(8)));
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn blank_comments_keeps_escaped_percent() {
        let input = r"draw \% not a comment";
        assert_eq!(blank_comments(input).as_ref(), input);
    }

    #[test]
    fn blank_comments_handles_percent_at_end() {
        let input = "node%";
        let out = blank_comments(input);
        assert_eq!(out.as_ref(), "node ");
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn blank_comments_preserves_utf8_byte_offsets() {
        let input = "é%界😀\n后";
        let out = blank_comments(input);
        assert_eq!(out.as_ref(), format!("é{}\n后", " ".repeat(8)));
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn blank_comments_handles_crlf_and_unterminated_last_line() {
        let input = "a%comment\r\nb%last";
        let out = blank_comments(input);
        assert_eq!(
            out.as_ref(),
            format!("a{}\nb{}", " ".repeat(9), " ".repeat(5))
        );
        assert_eq!(out.len(), input.len());
    }
}
