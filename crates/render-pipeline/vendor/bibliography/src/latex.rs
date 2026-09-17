//! Text-level LaTeX handling for BibTeX field values.
//!
//! Field values are stored raw (braces and control sequences preserved) so that
//! BibTeX's brace semantics survive; this module provides the conversions that
//! the standard styles apply:
//!
//! - [`decode`]: accents and letter commands to Unicode, braces removed.
//! - [`typeset`]: [`decode`] plus TeX ligature/dash conventions for typesetting.
//! - [`purify`]: BibTeX `purify$` (sort keys, labels).
//! - [`change_case`]: BibTeX `change.case$` with `"t"`, `"l"`, `"u"`.
//! - [`text_length`] / [`text_prefix`]: BibTeX `text.length$` / `text.prefix$`
//!   measured on decoded text (a special character counts as one character).
//!
//! The accent table is documented in `README.md` ("LaTeX accent decoding").

/// A `\` control sequence read from the input.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlSeq {
    /// `\word` (ASCII letters); trailing whitespace already consumed.
    Word(String),
    /// `\x` for one non-letter character `x`.
    Symbol(char),
}

struct Scanner<'a> {
    chars: Vec<(usize, char)>,
    src: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Self {
        Scanner {
            chars: src.char_indices().collect(),
            src,
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|&(_, c)| c)
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).map(|&(_, c)| c)
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    /// Byte offset of the current char (or the source length at the end).
    fn byte_pos(&self) -> usize {
        self.chars
            .get(self.pos)
            .map(|&(b, _)| b)
            .unwrap_or(self.src.len())
    }

    /// Reads a control sequence; the leading `\` has been consumed.
    fn read_control_seq(&mut self) -> ControlSeq {
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                let mut w = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphabetic() {
                        w.push(c);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                // TeX skips whitespace after a control word.
                while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                    self.pos += 1;
                }
                ControlSeq::Word(w)
            }
            Some(c) => {
                self.pos += 1;
                ControlSeq::Symbol(c)
            }
            None => ControlSeq::Word(String::new()),
        }
    }

    /// Reads a balanced `{...}` group whose `{` is at the current position and
    /// returns the raw inner text. Unbalanced input runs to the end.
    fn read_group(&mut self) -> &'a str {
        debug_assert_eq!(self.peek(), Some('{'));
        let start_byte = self.byte_pos() + 1;
        self.pos += 1;
        let mut depth = 1usize;
        let mut end_byte = self.src.len();
        while let Some(c) = self.next() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_byte = self.chars[self.pos - 1].0;
                        break;
                    }
                }
                _ => {}
            }
        }
        &self.src[start_byte..end_byte]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Decode,
    Typeset,
}

/// Decode accents and letter commands to Unicode and strip braces.
///
/// `~` becomes U+00A0 NO-BREAK SPACE. Unknown control words are dropped and
/// their brace arguments are kept as ordinary text (`\emph{x}` → `x`). Dashes
/// and quote ligatures are left alone; see [`typeset`].
pub fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    decode_into(s, &mut out, Mode::Decode);
    out
}

/// [`decode`] plus TeX's text conventions: `---` → em dash, `--` → en dash,
/// ``` `` ``` / `''` → curly double quotes, `` ` `` / `'` → curly single quotes.
pub fn typeset(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    decode_into(s, &mut out, Mode::Typeset);
    out
}

fn decode_into(s: &str, out: &mut String, mode: Mode) {
    let mut sc = Scanner::new(s);
    while let Some(c) = sc.next() {
        match c {
            '\\' => {
                let cs = sc.read_control_seq();
                handle_control_seq(&cs, &mut sc, out, mode);
            }
            '{' | '}' => {}
            '~' => out.push('\u{00A0}'),
            '-' if mode == Mode::Typeset => {
                if sc.peek() == Some('-') && sc.peek_at(1) == Some('-') {
                    sc.pos += 2;
                    out.push('\u{2014}');
                } else if sc.peek() == Some('-') {
                    sc.pos += 1;
                    out.push('\u{2013}');
                } else {
                    out.push('-');
                }
            }
            '`' if mode == Mode::Typeset => {
                if sc.peek() == Some('`') {
                    sc.pos += 1;
                    out.push('\u{201C}');
                } else {
                    out.push('\u{2018}');
                }
            }
            '\'' if mode == Mode::Typeset => {
                if sc.peek() == Some('\'') {
                    sc.pos += 1;
                    out.push('\u{201D}');
                } else {
                    out.push('\u{2019}');
                }
            }
            c => out.push(c),
        }
    }
}

fn handle_control_seq(cs: &ControlSeq, sc: &mut Scanner<'_>, out: &mut String, mode: Mode) {
    if let Some(accent) = accent_id(cs) {
        let arg = read_accent_argument(sc, mode);
        let mut chars = arg.chars();
        match chars.next() {
            Some(base) => {
                push_accented(out, accent, base);
                out.extend(chars);
            }
            None => {
                // `\'{}` or a stray accent: TeX would print a floating accent;
                // nothing meaningful survives in plain text.
            }
        }
        return;
    }
    if let Some(text) = letter_command(cs) {
        out.push_str(text);
        return;
    }
    if let Some(text) = text_command(cs) {
        out.push_str(text);
    }
    // Unknown control sequences produce nothing; their arguments (if any) are
    // decoded as ordinary text by the caller's loop.
}

/// Reads the argument of an accent command: a brace group (decoded), another
/// control sequence (for `\'{\i}` and `\'\i`), or a single character.
fn read_accent_argument(sc: &mut Scanner<'_>, mode: Mode) -> String {
    match sc.peek() {
        Some('{') => {
            let inner = sc.read_group();
            let mut s = String::new();
            decode_into(inner, &mut s, mode);
            s
        }
        Some('\\') => {
            sc.pos += 1;
            let cs = sc.read_control_seq();
            let mut s = String::new();
            handle_control_seq(&cs, sc, &mut s, mode);
            s
        }
        Some(_) => {
            let c = sc.next().unwrap_or(' ');
            c.to_string()
        }
        None => String::new(),
    }
}

/// Maps an accent control sequence to its identifying character.
fn accent_id(cs: &ControlSeq) -> Option<char> {
    match cs {
        ControlSeq::Symbol(c @ ('\'' | '`' | '^' | '"' | '~' | '=' | '.')) => Some(*c),
        ControlSeq::Word(w) => match w.as_str() {
            "u" | "v" | "H" | "c" | "k" | "r" | "d" | "b" | "t" => w.chars().next(),
            _ => None,
        },
        _ => None,
    }
}

/// Letter-producing commands (`\ss`, `\o`, ...). The name doubles as the
/// letters BibTeX `purify$` keeps for the special character.
fn letter_command(cs: &ControlSeq) -> Option<&'static str> {
    let ControlSeq::Word(w) = cs else {
        return None;
    };
    Some(match w.as_str() {
        "ss" => "ß",
        "SS" => "SS",
        "o" => "ø",
        "O" => "Ø",
        "aa" => "å",
        "AA" => "Å",
        "ae" => "æ",
        "AE" => "Æ",
        "oe" => "œ",
        "OE" => "Œ",
        "l" => "ł",
        "L" => "Ł",
        "i" => "ı",
        "j" => "ȷ",
        "dh" => "ð",
        "DH" => "Ð",
        "th" => "þ",
        "TH" => "Þ",
        "ng" => "ŋ",
        "NG" => "Ŋ",
        "dj" => "đ",
        "DJ" => "Đ",
        _ => return None,
    })
}

/// Text-producing commands that are neither accents nor letters.
fn text_command(cs: &ControlSeq) -> Option<&'static str> {
    Some(match cs {
        ControlSeq::Symbol('&') => "&",
        ControlSeq::Symbol('%') => "%",
        ControlSeq::Symbol('$') => "$",
        ControlSeq::Symbol('#') => "#",
        ControlSeq::Symbol('_') => "_",
        ControlSeq::Symbol('{') => "{",
        ControlSeq::Symbol('}') => "}",
        ControlSeq::Symbol(' ') | ControlSeq::Symbol('\n') | ControlSeq::Symbol('\t') => " ",
        ControlSeq::Symbol(',') => "\u{2009}",
        ControlSeq::Symbol(';') => "\u{2005}",
        ControlSeq::Symbol('\\') => " ",
        ControlSeq::Symbol(_) => return None,
        ControlSeq::Word(w) => match w.as_str() {
            "textendash" => "\u{2013}",
            "textemdash" => "\u{2014}",
            "ldots" | "dots" | "textellipsis" => "\u{2026}",
            "textquoteleft" => "\u{2018}",
            "textquoteright" => "\u{2019}",
            "textquotedblleft" => "\u{201C}",
            "textquotedblright" => "\u{201D}",
            "textquotesingle" => "'",
            "textbackslash" => "\\",
            "textasciitilde" => "~",
            "textasciicircum" => "^",
            "textbar" => "|",
            "textless" => "<",
            "textgreater" => ">",
            "textbullet" => "\u{2022}",
            "textperiodcentered" => "\u{00B7}",
            "textexclamdown" => "\u{00A1}",
            "textquestiondown" => "\u{00BF}",
            "textregistered" => "\u{00AE}",
            "texttrademark" => "\u{2122}",
            "copyright" | "textcopyright" => "\u{00A9}",
            "textdegree" => "\u{00B0}",
            "textmu" => "\u{00B5}",
            "pounds" | "textsterling" => "\u{00A3}",
            "S" => "\u{00A7}",
            "P" => "\u{00B6}",
            "dag" => "\u{2020}",
            "ddag" => "\u{2021}",
            "TeX" => "TeX",
            "LaTeX" => "LaTeX",
            "LaTeXe" => "LaTeX2e",
            "BibTeX" => "BibTeX",
            "etalchar" => "",
            _ => return None,
        },
    })
}

fn push_accented(out: &mut String, accent: char, base: char) {
    let base = match base {
        'ı' => 'i',
        'ȷ' => 'j',
        b => b,
    };
    match precomposed(accent, base) {
        Some(c) => out.push(c),
        None => {
            out.push(base);
            if let Some(mark) = combining_mark(accent) {
                out.push(mark);
            }
        }
    }
}

fn combining_mark(accent: char) -> Option<char> {
    Some(match accent {
        '\'' => '\u{0301}',
        '`' => '\u{0300}',
        '^' => '\u{0302}',
        '"' => '\u{0308}',
        '~' => '\u{0303}',
        '=' => '\u{0304}',
        '.' => '\u{0307}',
        'u' => '\u{0306}',
        'v' => '\u{030C}',
        'H' => '\u{030B}',
        'c' => '\u{0327}',
        'k' => '\u{0328}',
        'r' => '\u{030A}',
        'd' => '\u{0323}',
        'b' => '\u{0331}',
        't' => '\u{0361}',
        _ => return None,
    })
}

/// Precomposed forms for the accent/base pairs the README table lists.
fn precomposed(accent: char, base: char) -> Option<char> {
    let table: &[(char, &str, &str)] = &[
        (
            '\'',
            "aeiouyAEIOUYcCnNsSzZlLrRgG",
            "áéíóúýÁÉÍÓÚÝćĆńŃśŚźŹĺĹŕŔǵǴ",
        ),
        ('`', "aeiouAEIOUnN", "àèìòùÀÈÌÒÙǹǸ"),
        ('^', "aeiouAEIOUcCgGhHjJsSwWyY", "âêîôûÂÊÎÔÛĉĈĝĜĥĤĵĴŝŜŵŴŷŶ"),
        ('"', "aeiouyAEIOUY", "äëïöüÿÄËÏÖÜŸ"),
        ('~', "anoANOiIuU", "ãñõÃÑÕĩĨũŨ"),
        ('=', "aeiouAEIOU", "āēīōūĀĒĪŌŪ"),
        ('.', "cCeEgGzZI", "ċĊėĖġĠżŻİ"),
        ('u', "aAgGuUeEiIoO", "ăĂğĞŭŬĕĔĭĬŏŎ"),
        ('v', "cCdDeEnNrRsStTzZaAiIoOuU", "čČďĎěĚňŇřŘšŠťŤžŽǎǍǐǏǒǑǔǓ"),
        ('H', "oOuU", "őŐűŰ"),
        ('c', "cCsStTgGkKlLnNrR", "çÇşŞţŢģĢķĶļĻņŅŗŖ"),
        ('k', "aAeEiIuU", "ąĄęĘįĮųŲ"),
        ('r', "aAuU", "åÅůŮ"),
        ('d', "aAeEiIoOuU", "ạẠẹẸịỊọỌụỤ"),
    ];
    for (a, bases, results) in table {
        if *a != accent {
            continue;
        }
        if let Some(idx) = bases.chars().position(|b| b == base) {
            return results.chars().nth(idx);
        }
    }
    None
}

/// BibTeX `purify$`: keep alphanumerics and whitespace, turn `-` and `~` into
/// spaces, drop everything else. Inside a special character (`{\...}` at brace
/// depth 0) the control sequence is dropped unless it is a known letter
/// command, whose name is kept; then only alphanumerics survive. So
/// `{\'e}` → `e`, `{\ss}` → `ss`, `{\H{o}}` → `o`, `{\TeX}` → `` (empty).
pub fn purify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut sc = Scanner::new(s);
    let mut depth = 0usize;
    while let Some(c) = sc.next() {
        match c {
            '{' => {
                if depth == 0 && sc.peek() == Some('\\') {
                    // Special character: consume the balanced group.
                    sc.pos += 1;
                    let cs = sc.read_control_seq();
                    if let ControlSeq::Word(w) = &cs
                        && letter_command(&cs).is_some()
                    {
                        out.push_str(w);
                    }
                    let mut inner_depth = 1usize;
                    while let Some(c) = sc.next() {
                        match c {
                            '{' => inner_depth += 1,
                            '}' => {
                                inner_depth -= 1;
                                if inner_depth == 0 {
                                    break;
                                }
                            }
                            c if c.is_alphanumeric() => out.push(c),
                            _ => {}
                        }
                    }
                } else {
                    depth += 1;
                }
            }
            '}' => depth = depth.saturating_sub(1),
            '-' | '~' => out.push(' '),
            c if c.is_whitespace() => out.push(c),
            c if c.is_alphanumeric() => out.push(c),
            _ => {}
        }
    }
    out
}

/// The three modes of BibTeX `change.case$`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    /// `"t"`: lower-case everything except the first character and the first
    /// character after a colon-plus-whitespace; brace groups are untouched.
    Title,
    /// `"l"`: lower-case everything outside brace groups.
    Lower,
    /// `"u"`: upper-case everything outside brace groups.
    Upper,
}

/// BibTeX `change.case$`. Text inside non-special brace groups is preserved.
/// Special characters (`{\...}` at depth 0) are converted, including the
/// known letter commands (`\AE` ↔ `\ae`).
pub fn change_case(s: &str, mode: CaseMode) -> String {
    let mut out = String::with_capacity(s.len());
    let mut sc = Scanner::new(s);
    let mut depth = 0usize;
    let mut prev_colon = false;
    let mut at_start = true;
    while let Some(c) = sc.next() {
        let protect = mode == CaseMode::Title && (at_start || prev_colon);
        match c {
            '{' if depth == 0 && sc.peek() == Some('\\') => {
                out.push('{');
                convert_special(&mut sc, &mut out, mode, protect);
                prev_colon = false;
                at_start = false;
            }
            '{' => {
                depth += 1;
                out.push(c);
                prev_colon = false;
                at_start = false;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                out.push(c);
            }
            c if depth > 0 => out.push(c),
            ':' => {
                out.push(c);
                prev_colon = true;
                at_start = false;
            }
            c if c.is_whitespace() => {
                out.push(c);
                // `prev_colon` survives whitespace so the next letter is kept.
            }
            c => {
                if protect {
                    out.push(c);
                } else {
                    push_cased(&mut out, c, mode);
                }
                prev_colon = false;
                at_start = false;
            }
        }
    }
    out
}

fn push_cased(out: &mut String, c: char, mode: CaseMode) {
    match mode {
        CaseMode::Title | CaseMode::Lower => out.extend(c.to_lowercase()),
        CaseMode::Upper => out.extend(c.to_uppercase()),
    }
}

/// Converts the inside of a special character; `{` has been emitted and the
/// scanner is at the backslash.
fn convert_special(sc: &mut Scanner<'_>, out: &mut String, mode: CaseMode, protect: bool) {
    let mut depth = 1usize;
    // Control sequence first.
    if sc.peek() == Some('\\') {
        sc.pos += 1;
        out.push('\\');
        let cs = sc.read_control_seq();
        match &cs {
            ControlSeq::Word(w) => {
                let mapped = if protect {
                    w.clone()
                } else {
                    map_letter_command_case(w, mode)
                };
                out.push_str(&mapped);
                // read_control_seq consumed trailing whitespace; keep one
                // space if a letter follows so `\ss x` stays two tokens.
                if matches!(sc.peek(), Some(c) if c.is_ascii_alphabetic()) {
                    out.push(' ');
                }
            }
            ControlSeq::Symbol(c) => out.push(*c),
        }
    }
    while let Some(c) = sc.next() {
        match c {
            '{' => {
                depth += 1;
                out.push(c);
            }
            '}' => {
                depth -= 1;
                out.push(c);
                if depth == 0 {
                    return;
                }
            }
            c if c.is_alphabetic() && !protect => push_cased(out, c, mode),
            c => out.push(c),
        }
    }
}

fn map_letter_command_case(w: &str, mode: CaseMode) -> String {
    let pairs = [
        ("aa", "AA"),
        ("ae", "AE"),
        ("oe", "OE"),
        ("o", "O"),
        ("l", "L"),
        ("dh", "DH"),
        ("th", "TH"),
        ("ng", "NG"),
        ("dj", "DJ"),
        ("ss", "SS"),
    ];
    for (lower, upper) in pairs {
        if w == lower || w == upper {
            return match mode {
                CaseMode::Upper => upper.to_string(),
                CaseMode::Lower | CaseMode::Title => lower.to_string(),
            };
        }
    }
    w.to_string()
}

/// BibTeX `text.length$` measured on decoded text (special characters count
/// once, braces not at all).
pub fn text_length(s: &str) -> usize {
    decode(s).chars().count()
}

/// BibTeX `text.prefix$` measured on decoded text.
pub fn text_prefix(s: &str, n: usize) -> String {
    decode(s).chars().take(n).collect()
}

/// BibTeX `add.period$`: append `.` unless the text (ignoring trailing `}`)
/// already ends in `.`, `?`, or `!`, or is empty.
pub fn add_period(s: &str) -> String {
    let trimmed = s.trim_end_matches('}');
    if trimmed.is_empty() || trimmed.ends_with(['.', '?', '!']) {
        s.to_string()
    } else {
        format!("{s}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_accents_and_letters() {
        assert_eq!(decode(r"Andr\'e"), "André");
        assert_eq!(decode(r"Andr\'{e}"), "André");
        assert_eq!(decode(r"Andr{\'e}"), "André");
        assert_eq!(decode(r#"G\"odel"#), "Gödel");
        assert_eq!(decode(r"Stra{\ss}e"), "Straße");
        assert_eq!(decode(r"Fran\c{c}ois"), "François");
        assert_eq!(decode(r#"{\o}rsted \AA ngstr\"om"#), "ørsted Ångström");
        assert_eq!(decode(r#"na\"{\i}ve"#), "naïve");
        assert_eq!(decode(r"\v{C}apek"), "Čapek");
        assert_eq!(decode(r"Erd\H{o}s"), "Erdős");
        assert_eq!(decode(r"\L{}ukasiewicz"), "Łukasiewicz");
        assert_eq!(decode(r"\'{\ae}"), "æ\u{0301}");
        assert_eq!(decode(r"A \& B 100\%"), "A & B 100%");
        assert_eq!(decode(r"\emph{Foo} {Bar}"), "Foo Bar");
        assert_eq!(decode(r"x~y"), "x\u{00A0}y");
    }

    #[test]
    fn typesets_dashes_and_quotes() {
        assert_eq!(typeset("12--34 --- ``q'' `s'"), "12–34 — “q” ‘s’");
        assert_eq!(typeset(r"Knuth's \'e"), "Knuth’s é");
    }

    #[test]
    fn purifies_like_bibtex() {
        assert_eq!(purify(r"{\'E}cole-Normale, {\ss}!"), "Ecole Normale ss");
        assert_eq!(purify(r"The {\TeX}book"), "The book");
        assert_eq!(purify(r"Erd{\H{o}}s {\aa}"), "Erdos aa");
        assert_eq!(purify(r"a~b\TeX"), "a bTeX");
    }

    #[test]
    fn title_case_keeps_braces_and_first_char() {
        assert_eq!(
            change_case(
                "The Art of {Computer} Programming: A Study",
                CaseMode::Title
            ),
            "The art of {Computer} programming: A study"
        );
        assert_eq!(
            change_case(r"{\'E}COLE {\AE}", CaseMode::Lower),
            r"{\'e}cole {\ae}"
        );
        assert_eq!(change_case("ab {Cd}", CaseMode::Upper), "AB {Cd}");
        assert_eq!(change_case(r"{\'E}cole", CaseMode::Title), r"{\'E}cole");
    }

    #[test]
    fn add_period_rules() {
        assert_eq!(add_period("abc"), "abc.");
        assert_eq!(add_period("abc."), "abc.");
        assert_eq!(add_period("abc?}"), "abc?}");
        assert_eq!(add_period(""), "");
    }
}
