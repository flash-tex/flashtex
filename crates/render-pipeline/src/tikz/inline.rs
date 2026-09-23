//! `\tikz` shorthand pictures in running text.
//!
//! tikz.code.tex defines `\tikz` as a picture in a paragraph: `\tikz[opts]
//! {body}` and `\tikz[opts] <path command>;` both expand to
//! `\begin{tikzpicture}[opts] ... \end{tikzpicture}` (`\tikz@opt` collects a
//! brace group when one follows, otherwise everything up to the first `;`),
//! and `\pgfpicture` ends with `\leavevmode\box\pgfpic`: one `\hbox` in the
//! horizontal list, its width the bounding box's, its height and depth split
//! at the `baseline` key (the bounding box's bottom edge by default, so the
//! depth is 0).
//!
//! The scanner here is the pipeline's copy of
//! `flashtex_vector_graphics::tikz::find_inline_pictures` (crates/vector-
//! graphics), which the pinned `vendor/vector-graphics` predates; it goes
//! once that directory is re-pinned past it.

use flashtex_vector_graphics::tikz::text::{blank_comments, control_word, matching};
use flashtex_vector_graphics::tikz::{find_pictures, PictureSource};

/// Finds the `\tikz` shorthand pictures of a document, in source order:
/// `\tikz[options]{body}` and `\tikz[options] \path ... ;`. `%` comments
/// are skipped, and so are `tikzpicture` bodies (a `\tikz` inside one is
/// that picture's), and the replacement texts of `\newcommand`/`\def`-style
/// definitions (a `\tikz` there is typeset where the macro is used, which
/// the compiler expands and this byte scan cannot see). A `\tikz` followed
/// by anything but `[`, `{` or a control word is not a picture (amsldoc's
/// own `\def\tikz/{Ti\textit{k}Z}`).
pub fn find_inline_pictures(doc: &str) -> Vec<PictureSource> {
    let mut clean = blank_comments(doc);
    blank_definition_bodies(&mut clean);
    for p in find_pictures(&clean) {
        blank_range(&mut clean, p.start, p.end);
    }
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = clean[from..].find("\\tikz") {
        let start = from + rel;
        from = start + 1;
        let bytes = clean.as_bytes();
        // A longer control word: `\tikzset`, `\tikzstyle`, `\tikzcdset`...
        if bytes.get(start + 5).is_some_and(|b| b.is_ascii_alphabetic() || *b == b'@') {
            continue;
        }
        // Not the `tikz` after an escaped backslash (`\\tikz`).
        let backslashes = clean[..start].bytes().rev().take_while(|b| *b == b'\\').count();
        if backslashes % 2 == 1 {
            continue;
        }
        let mut j = skip_ws(&clean, start + 5);
        let mut options = None;
        if bytes.get(j) == Some(&b'[') {
            let Some(close) = matching(&clean, j) else { continue };
            options = Some((j + 1, close - 1));
            j = skip_ws(&clean, close);
        }
        let (body_start, body_end, end) = match bytes.get(j) {
            Some(b'{') => {
                let Some(close) = matching(&clean, j) else { continue };
                (j + 1, close - 1, close)
            }
            Some(b'\\') if control_word(&clean, j).is_some() => {
                // `\tikz@collect`: the tokens up to the first `;` outside
                // braces.
                let Some(semi) = top_level(&clean, j, b';') else { continue };
                (j, semi + 1, semi + 1)
            }
            _ => continue,
        };
        out.push(PictureSource { start, end, body_start, body_end, options });
        from = end;
    }
    out
}

/// Whether the project loads TikZ (or a package built on it), so that
/// `\tikz` is pgf's picture shorthand and not a document's own macro.
pub fn tikz_loaded(texts: &[&str]) -> bool {
    texts.iter().any(|t| {
        if !t.contains("tikz") && !t.contains("pgfplots") {
            return false;
        }
        let clean = blank_comments(t);
        ["\\usepackage", "\\RequirePackage"].iter().any(|word| {
            let mut from = 0;
            while let Some(rel) = clean[from..].find(word) {
                let at = from + rel;
                let word_end = at + word.len();
                from = word_end;
                if clean.as_bytes().get(word_end).is_some_and(|b| b.is_ascii_alphabetic()) {
                    continue;
                }
                let mut j = skip_ws(&clean, word_end);
                if clean.as_bytes().get(j) == Some(&b'[') {
                    let Some(close) = matching(&clean, j) else { break };
                    j = skip_ws(&clean, close);
                }
                if clean.as_bytes().get(j) != Some(&b'{') {
                    continue;
                }
                let Some(close) = matching(&clean, j) else { break };
                let list = &clean[j + 1..close - 1];
                if list.split(',').map(str::trim).any(|p| p == "tikz" || p.starts_with("tikz-") || p == "pgfplots" || p == "circuitikz") {
                    return true;
                }
                from = close;
            }
            false
        })
    })
}

fn skip_ws(s: &str, i: usize) -> usize {
    i + s[i..].len() - s[i..].trim_start().len()
}

/// The first `sep` at or after `from` outside `{}`/`[]`/`()`, in a string
/// whose comments are blanked.
fn top_level(s: &str, from: usize, sep: u8) -> Option<usize> {
    let b = s.as_bytes();
    let mut depth = 0i32;
    let mut i = from;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'{' | b'[' | b'(' => depth += 1,
            b'}' | b']' | b')' => depth -= 1,
            c if c == sep && depth <= 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Blanks `start..end` to spaces (newlines kept), keeping byte offsets.
fn blank_range(s: &mut String, start: usize, end: usize) {
    let end = end.min(s.len());
    if start >= end || !s.is_char_boundary(start) || !s.is_char_boundary(end) {
        return;
    }
    let blank: String = s[start..end].chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect();
    // Same byte length only when the range is ASCII; multi-byte characters
    // widen to one space per byte instead.
    if blank.len() == end - start {
        s.replace_range(start..end, &blank);
    } else {
        let per_byte: String = s[start..end].bytes().map(|b| if b == b'\n' { '\n' } else { ' ' }).collect();
        s.replace_range(start..end, &per_byte);
    }
}

/// Blanks the replacement texts of macro and environment definitions, so a
/// `\tikz` in one is not read as a picture at the definition.
fn blank_definition_bodies(s: &mut String) {
    const ONE: [&str; 8] = ["newcommand", "renewcommand", "providecommand", "DeclareRobustCommand", "def", "gdef", "edef", "xdef"];
    const TWO: [&str; 2] = ["newenvironment", "renewenvironment"];
    const DOC: [&str; 6] = ["NewDocumentCommand", "RenewDocumentCommand", "ProvideDocumentCommand", "DeclareDocumentCommand", "NewDocumentEnvironment", "RenewDocumentEnvironment"];
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = s[i..].find('\\') {
        let at = i + rel;
        let Some((name, mut j)) = control_word(s, at) else {
            i = at + 1;
            continue;
        };
        i = j;
        let (bodies, arg_groups) = if ONE.contains(&name) {
            (1usize, 0usize)
        } else if TWO.contains(&name) {
            (2, 0)
        } else if DOC.contains(&name) {
            (if name.ends_with("Environment") { 2 } else { 1 }, 1)
        } else {
            continue;
        };
        if s.as_bytes().get(j) == Some(&b'*') {
            j += 1;
        }
        j = skip_ws(s, j);
        // The defined name: `{\name}`, `\name` or, for environments, `{name}`.
        match s.as_bytes().get(j) {
            Some(b'{') => match matching(s, j) {
                Some(close) => j = close,
                None => continue,
            },
            Some(b'\\') => {
                j = match control_word(s, j) {
                    Some((_, e)) => e,
                    None => (j + 2).min(s.len()),
                };
            }
            _ => continue,
        }
        // `[n][default]`, or xparse's `{args}`; `\def`'s parameter text is
        // whatever stands before the first brace.
        let mut groups = 0usize;
        loop {
            j = skip_ws(s, j);
            match s.as_bytes().get(j) {
                Some(b'[') => match matching(s, j) {
                    Some(close) => j = close,
                    None => break,
                },
                Some(b'{') if groups < arg_groups => match matching(s, j) {
                    Some(close) => {
                        j = close;
                        groups += 1;
                    }
                    None => break,
                },
                _ => break,
            }
        }
        let mut done = 0usize;
        while done < bodies {
            let Some(rel) = s[j..].find('{') else { break };
            let open = j + rel;
            // A definition never spans a paragraph break or an
            // environment: past either, the brace belongs to something else.
            if s[j..open].contains("\\begin") || s[j..open].contains("\n\n") {
                break;
            }
            let Some(close) = matching(s, open) else { break };
            ranges.push((open + 1, close - 1));
            j = close;
            done += 1;
        }
        i = j.max(i);
    }
    for (a, b) in ranges {
        blank_range(s, a, b);
    }
}

/// The `baseline` key of a `\tikz`/`tikzpicture` option list: where the
/// surrounding line's baseline crosses the picture (tikz.code.tex
/// `/tikz/baseline`, `\pgfsetbaseline`).
#[derive(Clone, Debug, PartialEq)]
pub enum Baseline {
    /// A dimension in the picture's coordinate system (`baseline` alone is
    /// `0pt`): the y of the picture origin plus this.
    Dim(String),
    /// `baseline=(name.anchor)`: that anchor's y. `anchor` is `center`
    /// when the parenthesis names only the node.
    Node { name: String, anchor: String },
}

/// Reads `baseline`/`baseline=<value>` from a TikZ option list; `None` when
/// absent (the bounding box's bottom edge is then the baseline). The last
/// setting wins, as with pgfkeys.
pub fn baseline_option(options: &str) -> Option<Baseline> {
    let mut found = None;
    for opt in split_top_level(options, b',') {
        let opt = opt.trim();
        let (key, val) = match opt.find('=') {
            Some(eq) => (opt[..eq].trim(), Some(opt[eq + 1..].trim())),
            None => (opt, None),
        };
        if key != "baseline" {
            continue;
        }
        let val = val.map(|v| v.trim_start_matches('{').trim_end_matches('}').trim()).unwrap_or("0pt");
        found = Some(if let Some(inner) = val.strip_prefix('(').and_then(|v| v.strip_suffix(')')) {
            match inner.rsplit_once('.') {
                Some((name, anchor)) => Baseline::Node { name: name.trim().to_string(), anchor: anchor.trim().to_string() },
                None => Baseline::Node { name: inner.trim().to_string(), anchor: "center".to_string() },
            }
        } else {
            Baseline::Dim(val.to_string())
        });
    }
    found
}

/// Whether the option list sets `overlay` or `remember picture`: the
/// picture then contributes no bounding box (a zero-size box in the line).
pub fn overlay_option(options: &str) -> bool {
    split_top_level(options, b',').iter().map(|o| o.trim()).any(|o| {
        let key = o.split('=').next().unwrap_or("").trim();
        key == "overlay" || key == "remember picture"
    })
}

/// Splits on `sep` outside `{}`, `[]` and `()`.
fn split_top_level(s: &str, sep: u8) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in b.iter().enumerate() {
        match c {
            b'{' | b'[' | b'(' => depth += 1,
            b'}' | b']' | b')' => depth -= 1,
            c if *c == sep && depth <= 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_brace_and_path_forms() {
        let doc = "\\usepackage{tikz}\nA \\tikz{\\node[draw] {hi};} B \\tikz[baseline=-0.5ex] \\draw (0,0) -- (1,1); C % \\tikz{x}\n\\tikz/ D";
        let pics = find_inline_pictures(doc);
        assert_eq!(pics.len(), 2, "{pics:?}");
        assert_eq!(&doc[pics[0].start..pics[0].end], "\\tikz{\\node[draw] {hi};}");
        assert_eq!(&doc[pics[0].body_start..pics[0].body_end], "\\node[draw] {hi};");
        assert_eq!(pics[0].options, None);
        assert_eq!(&doc[pics[1].start..pics[1].end], "\\tikz[baseline=-0.5ex] \\draw (0,0) -- (1,1);");
        assert_eq!(&doc[pics[1].body_start..pics[1].body_end], "\\draw (0,0) -- (1,1);");
        let (a, b) = pics[1].options.unwrap();
        assert_eq!(&doc[a..b], "baseline=-0.5ex");
    }

    #[test]
    fn skips_definitions_pictures_and_longer_control_words() {
        let doc = "\\newcommand*\\circled[1]{\\tikz[baseline=(c.base)]{\\node (c) {#1};}}\n\\def\\x#1{\\tikz{#1}}\n\\tikzset{a/.style={b}}\n\\begin{tikzpicture}\\tikz{q}\\end{tikzpicture}\n\\circled{1} \\tikz \\node {x};";
        let pics = find_inline_pictures(doc);
        assert_eq!(pics.len(), 1, "{pics:?}");
        assert_eq!(&doc[pics[0].start..pics[0].end], "\\tikz \\node {x};");
    }

    #[test]
    fn nested_picture_in_node_text_is_one_inline_range() {
        let doc = "\\tikz \\node [scale=0.8] {\n\\begin{tikzpicture}\n\\draw (0,0) -- (1,1);\n\\end{tikzpicture}\n};\nafter";
        let pics = find_inline_pictures(doc);
        assert_eq!(pics.len(), 1, "{pics:?}");
        assert_eq!(pics[0].end, doc.find("\nafter").unwrap());
    }

    #[test]
    fn tikz_loaded_reads_package_lists() {
        assert!(tikz_loaded(&["\\usepackage[utf8]{inputenc}\n\\usepackage{amsmath, tikz}"]));
        assert!(tikz_loaded(&["\\RequirePackage{tikz-cd}"]));
        assert!(!tikz_loaded(&["\\def\\tikz/{TikZ} \\usepackage{amsmath}"]));
        assert!(!tikz_loaded(&["% \\usepackage{tikz}"]));
    }

    #[test]
    fn baseline_forms() {
        assert_eq!(baseline_option("scale=2"), None);
        assert_eq!(baseline_option("baseline"), Some(Baseline::Dim("0pt".into())));
        assert_eq!(baseline_option("x=1cm, baseline=-0.5ex"), Some(Baseline::Dim("-0.5ex".into())));
        assert_eq!(baseline_option("baseline=(char.base)"), Some(Baseline::Node { name: "char".into(), anchor: "base".into() }));
        assert_eq!(baseline_option("baseline={(current bounding box.center)}"), Some(Baseline::Node { name: "current bounding box".into(), anchor: "center".into() }));
        assert!(overlay_option("remember picture, overlay"));
        assert!(!overlay_option("baseline"));
    }
}
