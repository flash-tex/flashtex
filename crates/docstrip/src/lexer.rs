//! A TeX-shaped tokenizer for `.ins` batch files.
//!
//! docstrip batch files are plain TeX read by `tex`/`latex` after
//! `\input docstrip`, so their lexical rules are TeX's (The TeXbook,
//! chapter 8) under plain TeX's category codes plus docstrip's
//! `\catcode`\@=11` (docstrip.dtx line 1100): `\` starts a control
//! sequence, `{`/`}` group, `#` is a parameter character, `^^` doubles
//! into a character code, `%` comments to the end of the line, spaces and
//! tabs are blanks that collapse and vanish after a control word, and the
//! end of a line is a space (or `\par` when the line was empty). Lines are
//! read the way TeX reads them: split on LF, CR or CR LF, trailing spaces
//! removed, `\endlinechar` (13) appended and read like any character
//! under its category code.
//!
//! The category codes are a table the interpreter owns and changes
//! between tokens, as `\catcode` does in TeX; the lexer is lazy, so a
//! change applies from the next character on. `\declarepreamble` makes
//! the end of a line *active* and a space *other* (docstrip.dtx line
//! 3534) so a preamble keeps its line structure and indentation.

/// TeX category codes, reduced to what the interpreter distinguishes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cat {
    /// `{`
    BeginGroup,
    /// `}`
    EndGroup,
    /// `#`
    Param,
    /// A blank (space token, character 32).
    Space,
    /// A letter (`a`–`z`, `A`–`Z`, and `@` under docstrip).
    Letter,
    /// Everything else that is a character.
    Other,
    /// An active character: the end of a line in a preamble, `~`.
    Active,
}

/// One token: a character with its category, or a control sequence
/// named by its bytes (a control word is letters; a control symbol is the
/// one byte after the backslash).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Token {
    Char(u8, Cat),
    Cs(Vec<u8>),
    /// `#n` inside a macro body (never produced by the lexer).
    Arg(u8),
}

impl Token {
    pub fn cs(name: &str) -> Token {
        Token::Cs(name.as_bytes().to_vec())
    }

    pub fn is_cs(&self, name: &str) -> bool {
        matches!(self, Token::Cs(n) if n == name.as_bytes())
    }

    pub fn is_char(&self, byte: u8) -> bool {
        matches!(self, Token::Char(b, _) if *b == byte)
    }

    pub fn cat(&self) -> Option<Cat> {
        match self {
            Token::Char(_, c) => Some(*c),
            _ => None,
        }
    }

    pub fn is_space(&self) -> bool {
        matches!(self, Token::Char(_, Cat::Space))
    }

    /// The active end-of-line character a preamble is read with.
    pub fn is_eol(&self) -> bool {
        matches!(self, Token::Char(13, Cat::Active))
    }
}

/// TeX's category codes 0–15 for every byte.
pub type Catcodes = [u8; 256];

pub const ESCAPE: u8 = 0;
pub const BEGIN_GROUP: u8 = 1;
pub const END_GROUP: u8 = 2;
pub const END_LINE: u8 = 5;
pub const PARAM: u8 = 6;
pub const SUPERSCRIPT: u8 = 7;
pub const IGNORE: u8 = 9;
pub const SPACE: u8 = 10;
pub const LETTER: u8 = 11;
pub const OTHER: u8 = 12;
pub const ACTIVE: u8 = 13;
pub const COMMENT: u8 = 14;
pub const INVALID: u8 = 15;

/// Plain TeX's category codes (plain.tex lines 46–60) with docstrip's `@`
/// a letter. `^^L` stays other rather than plain's active `\par`.
pub fn plain_catcodes() -> Catcodes {
    let mut c = [OTHER; 256];
    c[b'\\' as usize] = ESCAPE;
    c[b'{' as usize] = BEGIN_GROUP;
    c[b'}' as usize] = END_GROUP;
    c[b'$' as usize] = 3;
    c[b'&' as usize] = 4;
    c[13] = END_LINE;
    c[b'#' as usize] = PARAM;
    c[b'^' as usize] = SUPERSCRIPT;
    c[b'_' as usize] = 8;
    c[0] = IGNORE;
    c[b' ' as usize] = SPACE;
    c[b'\t' as usize] = SPACE;
    for b in b'a'..=b'z' {
        c[b as usize] = LETTER;
    }
    for b in b'A'..=b'Z' {
        c[b as usize] = LETTER;
    }
    c[b'@' as usize] = LETTER;
    c[b'~' as usize] = ACTIVE;
    c[b'%' as usize] = COMMENT;
    c[127] = INVALID;
    c
}

/// TeX's reading states (The TeXbook p. 46): the beginning of a line,
/// the middle of one, and skipping blanks after a control word or a space.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    NewLine,
    MidLine,
    SkipBlanks,
}

pub struct Lexer {
    lines: Vec<Vec<u8>>,
    /// Current line (0-based) and column in it.
    li: usize,
    col: usize,
    state: State,
    pub catcodes: Catcodes,
    /// The line (1-based) the last returned token started on.
    pub last_line: usize,
}

/// Splits `bytes` into lines the way TeX's `input_ln` does: LF, CR or
/// CR LF end a line, and trailing spaces (and a stray CR) are removed.
/// Trailing tabs stay: TeX Live 2026's `tex` turns `a}<tab>` into `a} `
/// (checked; an older web2c trimmed tabs too, which is why TeX Live's
/// installed ledmac.sty lacks the blanks its source has).
pub fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                out.push(trim_line_end(&bytes[start..i]));
                i += 1;
                start = i;
            }
            b'\r' => {
                out.push(trim_line_end(&bytes[start..i]));
                i += if bytes.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        out.push(trim_line_end(&bytes[start..]));
    }
    out
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && matches!(line[end - 1], b' ' | b'\r' | b'\n') {
        end -= 1;
    }
    &line[..end]
}

impl Lexer {
    pub fn new(bytes: &[u8]) -> Lexer {
        Lexer { lines: split_lines(bytes).into_iter().map(<[u8]>::to_vec).collect(), li: 0, col: 0, state: State::NewLine, catcodes: plain_catcodes(), last_line: 1 }
    }

    /// The 1-based line the lexer is on.
    pub fn line(&self) -> usize {
        self.li + 1
    }

    /// Ends the input: `\endbatchfile` in a nested batch file.
    pub fn finish(&mut self) {
        self.li = self.lines.len();
        self.col = 0;
    }

    fn code(&self, byte: u8) -> u8 {
        self.catcodes[byte as usize]
    }

    /// Reads one character at `col`, resolving `^^` doublings when `^`
    /// is a superscript character (The TeXbook p. 45: `^^` + two
    /// lowercase hex digits is that code; `^^c` is `c` xor 64). Answers
    /// the byte and how many input bytes it took.
    fn char_at(&self, line: &[u8], col: usize) -> Option<(u8, usize)> {
        let b = *line.get(col)?;
        if self.code(b) == SUPERSCRIPT && line.get(col + 1) == Some(&b) {
            if let (Some(&h), Some(&l)) = (line.get(col + 2), line.get(col + 3)) {
                if is_lower_hex(h) && is_lower_hex(l) {
                    return Some(((hex(h) << 4) | hex(l), 4));
                }
            }
            if let Some(&c) = line.get(col + 2) {
                if c < 128 {
                    return Some((c ^ 0x40, 3));
                }
            }
        }
        Some((b, 1))
    }

    /// The next token, or `None` at the end of the file. (Not an
    /// `Iterator`: the category codes may change between calls.)
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Token> {
        loop {
            self.last_line = self.line();
            let line = self.lines.get(self.li)?;
            // `\endlinechar` (13) follows every line and is read like any
            // other character under its category code.
            let (byte, len, at_end) = match self.char_at(line, self.col) {
                Some((b, l)) => (b, l, false),
                None => (13, 0, true),
            };
            if at_end {
                self.li += 1;
                self.col = 0;
            } else {
                self.col += len;
            }
            let state = self.state;
            if at_end {
                self.state = State::NewLine;
            }
            match self.code(byte) {
                ESCAPE => {
                    if at_end {
                        continue; // an escape as `\endlinechar` names nothing
                    }
                    // A control word (letters) or a control symbol (one character).
                    let line = &self.lines[self.li];
                    let mut name = Vec::new();
                    let mut col = self.col;
                    while let Some((b, l)) = self.char_at(line, col) {
                        if self.code(b) != LETTER {
                            break;
                        }
                        name.push(b);
                        col += l;
                    }
                    if name.is_empty() {
                        match self.char_at(line, col) {
                            Some((b, l)) => {
                                name.push(b);
                                col += l;
                                self.state = if self.code(b) == SPACE { State::SkipBlanks } else { State::MidLine };
                            }
                            // `\` at the very end of a line: the control symbol is the end-of-line character.
                            None => {
                                name.push(13);
                                self.state = State::MidLine;
                            }
                        }
                    } else {
                        self.state = State::SkipBlanks;
                    }
                    self.col = col;
                    return Some(Token::Cs(name));
                }
                COMMENT => {
                    // A comment discards the rest of the line, including its end.
                    self.li += 1;
                    self.col = 0;
                    self.state = State::NewLine;
                }
                IGNORE | INVALID => {}
                END_LINE => {
                    // The rest of the line is dropped; what the line end
                    // yields depends on the state (The TeXbook p. 47).
                    if !at_end {
                        self.li += 1;
                        self.col = 0;
                    }
                    self.state = State::NewLine;
                    match state {
                        State::NewLine => return Some(Token::cs("par")),
                        State::MidLine => return Some(Token::Char(b' ', Cat::Space)),
                        State::SkipBlanks => {}
                    }
                }
                SPACE => match state {
                    State::NewLine | State::SkipBlanks => {}
                    State::MidLine => {
                        if !at_end {
                            self.state = State::SkipBlanks;
                        }
                        return Some(Token::Char(b' ', Cat::Space));
                    }
                },
                code => {
                    let cat = match code {
                        BEGIN_GROUP => Cat::BeginGroup,
                        END_GROUP => Cat::EndGroup,
                        PARAM => Cat::Param,
                        LETTER => Cat::Letter,
                        ACTIVE => Cat::Active,
                        _ => Cat::Other,
                    };
                    if !at_end {
                        self.state = State::MidLine;
                    }
                    return Some(Token::Char(byte, cat));
                }
            }
        }
    }
}

fn is_lower_hex(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
}

fn hex(b: u8) -> u8 {
    if b.is_ascii_digit() {
        b - b'0'
    } else {
        b - b'a' + 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(src: &str) -> Vec<Token> {
        let mut l = Lexer::new(src.as_bytes());
        let mut out = Vec::new();
        while let Some(t) = l.next() {
            out.push(t);
        }
        out
    }

    fn text(tokens: &[Token]) -> String {
        tokens
            .iter()
            .map(|t| match t {
                Token::Char(b, Cat::Active) if *b == 13 => "<EOL>".to_string(),
                Token::Char(b, _) => (*b as char).to_string(),
                Token::Cs(n) => format!("[{}]", String::from_utf8_lossy(n)),
                Token::Arg(n) => format!("#{n}"),
            })
            .collect()
    }

    #[test]
    fn control_words_skip_blanks_and_lines_end_in_spaces() {
        assert_eq!(text(&all("\\input docstrip.tex\n\\keepsilent   \n\\Msg{a  b}")), "[input]docstrip.tex [keepsilent][Msg]{a b} ", "every line ends in a space, the last one too");
        assert_eq!(text(&all("a\n\nb")), "a [par]b ");
        assert_eq!(text(&all("  x % comment\ny")), "x y ");
        assert_eq!(text(&all("\\a\\b \\c")), "[a][b][c]");
        assert_eq!(text(&all("\\%\\ x\\{")), "[%][ ]x[{] ");
        assert_eq!(all("\t\tx\t \ty%"), vec![Token::Char(b'x', Cat::Letter), Token::Char(b' ', Cat::Space), Token::Char(b'y', Cat::Letter)]);
    }

    #[test]
    fn doubled_carets_are_character_codes() {
        assert_eq!(all("^^J%"), vec![Token::Char(10, Cat::Other)]);
        assert_eq!(all("^^e9%"), vec![Token::Char(0xe9, Cat::Other)]);
        assert_eq!(all("\\^^M%"), vec![Token::Cs(vec![13])]);
        // A `^^M` in the middle of a line is an end-of-line character: the rest of the line is dropped.
        assert_eq!(text(&all("a^^Mb\nc")), "a c ");
    }

    #[test]
    fn active_line_ends_and_other_spaces_as_in_a_preamble() {
        let mut l = Lexer::new(b"\\preamble\n  two  spaces\n\n\\endpreamble\nafter");
        assert_eq!(l.next(), Some(Token::cs("preamble")));
        l.catcodes[13] = ACTIVE;
        l.catcodes[b' ' as usize] = OTHER;
        let mut got = Vec::new();
        loop {
            let t = l.next().unwrap();
            if t.is_cs("endpreamble") {
                break;
            }
            got.push(t);
        }
        l.catcodes = plain_catcodes();
        assert_eq!(text(&got), "<EOL>  two  spaces<EOL><EOL>");
        assert_eq!(text(&[l.next().unwrap(), l.next().unwrap()]), "af", "the line end after \\endpreamble is read lazily, under the restored codes, and skipped after a control word");
    }

    #[test]
    fn catcode_changes_apply_from_the_next_character() {
        let mut l = Lexer::new(b"a b\n");
        assert_eq!(l.next(), Some(Token::Char(b'a', Cat::Letter)));
        l.catcodes[b' ' as usize] = ACTIVE;
        assert_eq!(l.next(), Some(Token::Char(b' ', Cat::Active)));
        l.catcodes[b'b' as usize] = OTHER;
        assert_eq!(l.next(), Some(Token::Char(b'b', Cat::Other)));
    }

    #[test]
    fn lines_split_like_tex() {
        let l = split_lines(b"a  \r\nb\rc\n\nd\t \t");
        assert_eq!(l, [&b"a"[..], b"b", b"c", b"", b"d\t \t"]);
        assert_eq!(split_lines(b"x\n"), [&b"x"[..]]);
        assert!(split_lines(b"").is_empty());
    }
}
