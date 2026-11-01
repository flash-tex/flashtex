//! BibTeX name lists: splitting on `and`, the three comma forms, the `von`
//! rule, and `format.name$`-style formatting.
//!
//! Parts are kept raw (braces and control sequences preserved) exactly as
//! BibTeX keeps them; use [`crate::latex::decode`] to display them.
//!
//! Tokens are separated by whitespace and `~` at brace depth 0; commas at
//! depth 0 separate the parts of the comma forms. A token is a "von" token
//! when its first letter at depth 0 is lower-case; a leading non-special brace
//! group makes the token non-von; a special character (`{\...`) decides by the
//! case of a known letter command (`\o` vs `\O`) or of the first letter after
//! the command.

use crate::latex;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Name {
    pub first: String,
    pub von: String,
    pub last: String,
    pub jr: String,
}

impl Name {
    /// `and others` marker.
    pub fn is_others(&self) -> bool {
        self.last == "others" && self.first.is_empty() && self.von.is_empty() && self.jr.is_empty()
    }

    fn part(&self, letter: char) -> &str {
        match letter {
            'f' => &self.first,
            'v' => &self.von,
            'l' => &self.last,
            'j' => &self.jr,
            _ => "",
        }
    }
}

/// Split a name-list field (`author`, `editor`) on ` and ` at brace depth 0.
pub fn split_names(field: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let bytes = field.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'a' | b'A'
                if depth == 0
                    && i > 0
                    && bytes[i - 1].is_ascii_whitespace()
                    && i + 3 < bytes.len()
                    && bytes[i + 1].eq_ignore_ascii_case(&b'n')
                    && bytes[i + 2].eq_ignore_ascii_case(&b'd')
                    && bytes[i + 3].is_ascii_whitespace() =>
            {
                names.push(field[start..i].trim().to_string());
                i += 4;
                start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    names.push(field[start..].trim().to_string());
    names.into_iter().filter(|n| !n.is_empty()).collect()
}

/// Parse every name in a name-list field.
pub fn parse_names(field: &str) -> Vec<Name> {
    split_names(field).iter().map(|n| parse_name(n)).collect()
}

/// Parse one name in any of the three BibTeX forms.
pub fn parse_name(name: &str) -> Name {
    let parts = split_depth0(name, |b| b == b',');
    let tokens: Vec<Vec<String>> = parts.iter().map(|p| tokenize(p)).collect();
    match tokens.len() {
        0 => Name::default(),
        1 => first_von_last(&tokens[0]),
        2 => {
            let (von, last) = von_last(&tokens[0]);
            Name {
                first: join(&tokens[1]),
                von,
                last,
                jr: String::new(),
            }
        }
        _ => {
            // "von Last, Jr, First"; anything after a third comma joins First
            // (BibTeX warns about too many commas and does the same).
            let (von, last) = von_last(&tokens[0]);
            let first: Vec<String> = tokens[2..].iter().flatten().cloned().collect();
            Name {
                first: join(&first),
                von,
                last,
                jr: join(&tokens[1]),
            }
        }
    }
}

fn join(tokens: &[String]) -> String {
    tokens.join(" ")
}

/// Split at depth-0 bytes matching `sep`, keeping empty pieces.
fn split_depth0(s: &str, sep: impl Fn(u8) -> bool) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b if depth == 0 && sep(b) => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Tokens separated by whitespace or `~` at depth 0.
fn tokenize(s: &str) -> Vec<String> {
    split_depth0(s, |b| b.is_ascii_whitespace() || b == b'~')
        .into_iter()
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

fn first_von_last(tokens: &[String]) -> Name {
    let n = tokens.len();
    if n == 0 {
        return Name::default();
    }
    // The last token is never a von candidate.
    let von_start = (0..n - 1).find(|&i| is_von_token(&tokens[i]));
    match von_start {
        None => Name {
            first: join(&tokens[..n - 1]),
            von: String::new(),
            last: tokens[n - 1].clone(),
            jr: String::new(),
        },
        Some(vs) => {
            let mut ve = vs + 1;
            for i in (vs..n - 1).rev() {
                if is_von_token(&tokens[i]) {
                    ve = i + 1;
                    break;
                }
            }
            Name {
                first: join(&tokens[..vs]),
                von: join(&tokens[vs..ve]),
                last: join(&tokens[ve..]),
                jr: String::new(),
            }
        }
    }
}

/// "von Last" part of the comma forms: von runs from the first token through
/// the last lower-case token (the final token always belongs to Last).
fn von_last(tokens: &[String]) -> (String, String) {
    let n = tokens.len();
    if n == 0 {
        return (String::new(), String::new());
    }
    let mut ve = 0;
    for i in (0..n - 1).rev() {
        if is_von_token(&tokens[i]) {
            ve = i + 1;
            break;
        }
    }
    (join(&tokens[..ve]), join(&tokens[ve..]))
}

/// BibTeX `von_token_found`.
fn is_von_token(token: &str) -> bool {
    let chars: Vec<char> = token.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() {
            return c.is_lowercase();
        }
        if c == '{' {
            if chars.get(i + 1) == Some(&'\\') {
                return special_char_is_lower(&chars[i + 2..]);
            }
            // A non-special brace group makes the token non-von.
            return false;
        }
        i += 1;
    }
    false
}

/// Case of a special character `{\cmd...}`; `rest` starts after the backslash.
fn special_char_is_lower(rest: &[char]) -> bool {
    let mut i = 0;
    let mut cmd = String::new();
    while i < rest.len() && rest[i].is_ascii_alphabetic() {
        cmd.push(rest[i]);
        i += 1;
    }
    if cmd.is_empty() && i < rest.len() {
        i += 1; // control symbol such as \' or \"
    }
    match cmd.as_str() {
        "i" | "j" | "oe" | "ae" | "aa" | "o" | "l" | "ss" => return true,
        "OE" | "AE" | "AA" | "O" | "L" | "SS" => return false,
        _ => {}
    }
    let mut depth = 1usize;
    while i < rest.len() {
        let c = rest[i];
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            c if c.is_alphabetic() => return c.is_lowercase(),
            _ => {}
        }
        i += 1;
    }
    false
}

/// A parsed `format.name$` piece: `{pre LL{inter}post}`.
struct Piece {
    pre: String,
    letter: char,
    full: bool,
    inter: Option<String>,
    post: String,
}

enum Template {
    Literal(String),
    Piece(Piece),
}

fn parse_template(template: &str) -> Vec<Template> {
    let mut out = Vec::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    let mut literal = String::new();
    while i < chars.len() {
        if chars[i] != '{' {
            literal.push(chars[i]);
            i += 1;
            continue;
        }
        if !literal.is_empty() {
            out.push(Template::Literal(std::mem::take(&mut literal)));
        }
        // Collect the balanced piece body.
        let mut depth = 1usize;
        let mut body = String::new();
        i += 1;
        while i < chars.len() {
            let c = chars[i];
            i += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            body.push(c);
        }
        out.push(Template::Piece(parse_piece(&body)));
    }
    if !literal.is_empty() {
        out.push(Template::Literal(literal));
    }
    out
}

fn parse_piece(body: &str) -> Piece {
    let chars: Vec<char> = body.chars().collect();
    let mut depth = 0usize;
    let mut idx = None;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            'f' | 'v' | 'l' | 'j' if depth == 0 => {
                idx = Some(i);
                break;
            }
            _ => {}
        }
    }
    let Some(start) = idx else {
        return Piece {
            pre: body.to_string(),
            letter: ' ',
            full: true,
            inter: None,
            post: String::new(),
        };
    };
    let letter = chars[start];
    let mut i = start + 1;
    let full = chars.get(i) == Some(&letter);
    if full {
        i += 1;
    }
    let mut inter = None;
    if chars.get(i) == Some(&'{') {
        let mut depth = 1usize;
        let mut s = String::new();
        i += 1;
        while i < chars.len() {
            let c = chars[i];
            i += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            s.push(c);
        }
        inter = Some(s);
    }
    Piece {
        pre: chars[..start].iter().collect(),
        letter,
        full,
        inter,
        post: chars[i..].iter().collect(),
    }
}

/// The BibTeX `format.name$` template language, enough for the standard
/// styles: `{ff~}{vv~}{ll}{, jj}`, `{f.~}{vv~}{ll}{, jj}`,
/// `{vv{ } }{ll{ }}{  ff{ }}{  jj{ }}`, `{v{}}{l{}}`, `{ll}`.
///
/// Rules implemented (reconstructed from btxhak and observed `.bbl` output;
/// see README "Name formatting"):
/// - A piece outputs nothing when its part is empty.
/// - `ff` copies tokens; `f` abbreviates each token to its first character
///   (a leading special character counts as one), hyphenated tokens give
///   `J.-B.`. With the default separator each abbreviated token except the
///   last is followed by `.`.
/// - Default token separator: a tie if the token just written is shorter
///   than three characters or the next token is the last of the part; else a
///   space. An explicit `{inter}` string is used verbatim.
/// - A single trailing `~` in the post text becomes a tie if the part text is
///   shorter than three characters, otherwise a space; `~~` forces a tie.
pub fn format_name(name: &Name, template: &str) -> String {
    let mut out = String::new();
    for item in parse_template(template) {
        match item {
            Template::Literal(s) => out.push_str(&s),
            Template::Piece(p) => {
                let part = name.part(p.letter);
                if part.is_empty() {
                    continue;
                }
                let tokens = tokenize(part);
                let mut text = String::new();
                for (i, tok) in tokens.iter().enumerate() {
                    let is_last = i + 1 == tokens.len();
                    let written = if p.full {
                        tok.clone()
                    } else {
                        abbreviate(tok, p.inter.as_deref(), is_last)
                    };
                    text.push_str(&written);
                    if !is_last {
                        match &p.inter {
                            Some(s) => text.push_str(s),
                            None => {
                                let short = latex::text_length(&written) < 3;
                                let next_is_last = i + 2 == tokens.len();
                                text.push(if short || next_is_last { '~' } else { ' ' });
                            }
                        }
                    }
                }
                out.push_str(&p.pre);
                out.push_str(&text);
                if let Some(stripped) = p.post.strip_suffix("~~") {
                    out.push_str(stripped);
                    out.push('~');
                } else if let Some(stripped) = p.post.strip_suffix('~') {
                    out.push_str(stripped);
                    out.push(if latex::text_length(&text) < 3 {
                        '~'
                    } else {
                        ' '
                    });
                } else {
                    out.push_str(&p.post);
                }
            }
        }
    }
    out
}

/// First character(s) of a token for `{f}`-style pieces.
fn abbreviate(token: &str, inter: Option<&str>, is_last: bool) -> String {
    let subs = split_depth0(token, |b| b == b'-');
    let initials: Vec<String> = subs
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| first_char(s))
        .collect();
    match inter {
        Some(sep) => initials.join(sep),
        None => {
            let mut s = initials.join(".-");
            // Every abbreviated token except the last carries its own period;
            // the last one relies on the piece's post text (`{f.}`).
            if !is_last {
                s.push('.');
            }
            s
        }
    }
}

fn first_char(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.first() == Some(&'{') && chars.get(1) == Some(&'\\') {
        // Whole special character.
        let mut depth = 0usize;
        for (i, &c) in chars.iter().enumerate() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return chars[..=i].iter().collect();
                    }
                }
                _ => {}
            }
        }
        return s.to_string();
    }
    chars
        .iter()
        .find(|c| **c != '{' && **c != '}')
        .map(|c| c.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(first: &str, von: &str, last: &str, jr: &str) -> Name {
        Name {
            first: first.into(),
            von: von.into(),
            last: last.into(),
            jr: jr.into(),
        }
    }

    #[test]
    fn classic_von_cases() {
        let cases = [
            ("Donald E. Knuth", n("Donald E.", "", "Knuth", "")),
            ("Knuth, Donald E.", n("Donald E.", "", "Knuth", "")),
            ("Jean de la Fontaine", n("Jean", "de la", "Fontaine", "")),
            ("jean de la fontaine", n("", "jean de la", "fontaine", "")),
            ("Jean de La Fontaine", n("Jean", "de", "La Fontaine", "")),
            ("de la Fontaine, Jean", n("Jean", "de la", "Fontaine", "")),
            ("De La Fontaine, Jean", n("Jean", "", "De La Fontaine", "")),
            ("de La Fontaine, Jean", n("Jean", "de", "La Fontaine", "")),
            ("Ford, Jr., Henry", n("Henry", "", "Ford", "Jr.")),
            ("{Barnes and Noble}", n("", "", "{Barnes and Noble}", "")),
            (
                "Charles Louis Xavier Joseph de la Vall{\\'e}e Poussin",
                n(
                    "Charles Louis Xavier Joseph",
                    "de la",
                    "Vall{\\'e}e Poussin",
                    "",
                ),
            ),
            (
                "{\\'E}mile {\\'e}cole",
                n("{\\'E}mile", "", "{\\'e}cole", ""),
            ),
            ("{\\o}rsted, Hans", n("Hans", "", "{\\o}rsted", "")),
            ("Hans {\\o}rsted Doe", n("Hans", "{\\o}rsted", "Doe", "")),
            ("Emile {\\'e}cole Doe", n("Emile", "{\\'e}cole", "Doe", "")),
            ("Jean {de} Gaulle", n("Jean {de}", "", "Gaulle", "")),
            ("Ludwig van Beethoven", n("Ludwig", "van", "Beethoven", "")),
            ("Knuth", n("", "", "Knuth", "")),
            ("others", n("", "", "others", "")),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_name(input), expected, "input: {input}");
        }
        assert!(parse_name("others").is_others());
    }

    #[test]
    fn splits_on_and_at_depth_zero() {
        assert_eq!(
            split_names("Knuth and {Barnes and Noble} AND Lamport and others"),
            vec!["Knuth", "{Barnes and Noble}", "Lamport", "others"]
        );
        assert_eq!(split_names("Alexander Grand"), vec!["Alexander Grand"]);
    }

    #[test]
    fn formats_names_like_plain_and_abbrv() {
        let full = "{ff~}{vv~}{ll}{, jj}";
        let abbr = "{f.~}{vv~}{ll}{, jj}";
        assert_eq!(
            format_name(&parse_name("Donald E. Knuth"), full),
            "Donald~E. Knuth"
        );
        assert_eq!(
            format_name(&parse_name("Donald E. Knuth"), abbr),
            "D.~E. Knuth"
        );
        assert_eq!(
            format_name(&parse_name("Leslie Lamport"), full),
            "Leslie Lamport"
        );
        assert_eq!(
            format_name(&parse_name("Leslie Lamport"), abbr),
            "L.~Lamport"
        );
        assert_eq!(
            format_name(&parse_name("Ford, Jr., Henry"), full),
            "Henry Ford, Jr."
        );
        assert_eq!(
            format_name(&parse_name("Jean-Baptiste Say"), abbr),
            "J.-B. Say"
        );
        assert_eq!(
            format_name(&parse_name("Ludwig van Beethoven"), full),
            "Ludwig van Beethoven"
        );
        assert_eq!(
            format_name(&parse_name("Charles de Gaulle"), abbr),
            "C.~de~Gaulle"
        );
        assert_eq!(
            format_name(&parse_name("C. A. R. Hoare"), abbr),
            "C.~A.~R. Hoare"
        );
        assert_eq!(
            format_name(&parse_name("Jon Louis Bentley"), full),
            "Jon~Louis Bentley"
        );
        assert_eq!(
            format_name(
                &parse_name("de la Fontaine, Jean"),
                "{vv{ } }{ll{ }}{  ff{ }}{  jj{ }}"
            ),
            "de la Fontaine  Jean"
        );
        assert_eq!(
            format_name(&parse_name("de Gaulle, Charles"), "{v{}}{l{}}"),
            "dG"
        );
        assert_eq!(format_name(&parse_name("Knuth, Donald"), "{v{}}{l{}}"), "K");
    }
}
