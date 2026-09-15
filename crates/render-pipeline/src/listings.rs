//! The `listings` package: `\lstset` and the `lstlisting` keys
//! (listings.sty / lstmisc.sty v1.10c, TeX Live 2025).
//!
//! The compiler has no model for the package — it sets an `lstlisting` body
//! as literal typewriter lines and reports the `[...]` options as not
//! implemented — so the keys are read here from the source bytes, the way
//! `abstractenv`, `tikzpicture` and a list's `\begin` keys already are. That
//! is not a preference: `crates/render-pipeline` builds against
//! `vendor/compiler`, so a key model added to the compiler would not reach
//! this crate until the vendor pin moves.
//!
//! # What listings actually does, measured
//!
//! Every number below was read out of pdfTeX, not inferred from a key name:
//! `\showoutput` on a probe with `fixtures/real-world/listings-manual`'s own
//! preamble, and the content stream of that fixture's pinned reference PDF.
//!
//! ## `basicstyle` — the size and the leading
//!
//! `\lst@Init` runs the `basicstyle` declarations before the first line, so
//! the whole block is set at that size *with that size's own
//! `\baselineskip`*. For `basicstyle=\ttfamily\small` in an 11pt article
//! that is `\fontsize{10}{12}`: the reference's code glyphs are `SFTT1000`
//! at 9.9626 bp (10.0 pt) and its code baselines are 11.955 bp (12.0 pt)
//! apart. Every line is an `\hbox(8.39996+3.60004)` — LaTeX's strut, 0.7
//! and 0.3 of that `\baselineskip` — and consecutive lines are stacked with
//! `\lineskiplimit\maxdimen \lineskip\z@`, so the baseline pitch is exactly
//! height + depth = the `\baselineskip` again.
//!
//! ## `columns` — the fixed character grid
//!
//! listings' default is `columns=[c]fixed`: an output token of `n`
//! characters is set as `\hbox to n\lst@width` whose contents are the `n`
//! characters with `\hss` between them *and* at both ends (`\lst@FillFixed`
//! plus `\lst@lefthss`/`\lst@righthss`). With `basewidth={0.6em,0.45em}`
//! and a monospaced face the slack per token is `n(W - w)` shared equally
//! by the `n + 1` fils, so every fil is `n(W - w)/(n + 1)`. pdfTeX writes
//! exactly that: line 14 of the fixture's second listing is
//!
//! ```text
//! [-1079(d)-78(e)-79(f)]TJ  [-809(w)-100(r)-100(i)-100(t)-100(e)-99(_)…]TJ
//! ```
//!
//! `def` is 3 characters: `3 × 1.0498/4 = 0.78735 pt` → `-78`/`-79`.
//! `write_default_config` is 20: `20 × 1.0498/21 = 0.99981` → `-100`. The
//! underscore is a letter, so the whole identifier is one token. For
//! `ec-lmtt10` (and `cmtt10`) the quad is `1.05` and every character is
//! `0.525` of the design size, i.e. exactly half a quad, so `W - w =
//! 0.6em - 0.5em = 0.1em` — the one relation this module needs, and it is
//! asserted against the metrics rather than assumed (see [`column_fill_em`]).
//!
//! ## `numbers` / `numberstyle` / `numbersep`
//!
//! `numbers=left` is `\llap{\normalfont \lst@numberstyle{\thelstnumber}
//! \kern\lst@numbersep}` (lstmisc.sty 1183-1186). `\normalfont` is why the
//! reference sets the numbers in `SFRM0600` — roman, not the typewriter
//! basicstyle — and `numbersep` defaults to 10pt: the fixture's `1` sits at
//! x = 58.385 bp and is 3.652 bp wide, so its right edge plus 9.963 bp
//! (10 pt) is exactly the 72 bp text margin. `10` starts 3.652 bp further
//! left: the numbers are right-aligned, not left-aligned.
//!
//! ## `frame` and `caption` — the vertical list
//!
//! `\showoutput` on the probe gives, between the paragraph before and the
//! first code line (11pt article, `frame=single`, a `caption`):
//!
//! ```text
//! \penalty -50
//! \glue 6.0 plus 2.0 minus 2.0     \vspace\lst@aboveskip (\medskipamount)
//! \glue 3.0 plus 1.0 minus 1.0     \abovecaptionskip = \lst@abovecaption
//! \glue(\baselineskip) 3.92989     13.6: the caption is \normalsize
//! \hbox(7.54149+2.12863)           the caption line, centred
//! \glue 3.0 plus 1.0 minus 1.0     \belowcaptionskip
//! \glue -8.6                       \ht(frame rule box) - \baselineskip
//! \glue 1.0                        \lineskip
//! \glue(\baselineskip) 6.47137     12.0 now: basicstyle is in force
//! \hbox(3.4+0.0)                   the frame's top rule + framesep
//! \glue(\lineskip) 0.0             lineskiplimit is \maxdimen inside
//! \hbox(8.39996+3.60004)           the first code line
//! ```
//!
//! so the caption baseline to the first code baseline is
//! `belowcaptionskip + lineskip + framerule + framesep + 0.7·baselineskip`
//! = 3 + 1 + 0.4 + 3 + 8.4 = 15.8 pt, and the frame's top edge is
//! `belowcaptionskip + lineskip` = 4.0 pt under the caption baseline. At
//! the end the bottom rule is a final line of height 0 and depth
//! `framesep + framerule`, so the last code baseline sits
//! `0.3·baselineskip + framesep + framerule` = 7.0 pt above the frame's
//! bottom edge — the reference measures 6.978 bp = 7.004 pt — and the next
//! paragraph's baseline is `0.3·baselineskip + \lst@belowskip +
//! \baselineskip` below it.
//!
//! This block model has no vertical rule alongside a line and no way to
//! draw a box around a whole paragraph, so **the frame's rules are not
//! painted**: only the space they occupy is set. The limitation this module
//! reports says so, per listing, and names every key it did not apply.

use flashtex_compiler::text_builtins::{DimenUnit, PhysicalUnit, TextDimen};
use flashtex_compiler::{DocumentId, Span};

use crate::adapter::{Block, CharSrc, Item, Labels, ParaPart, ParaStyle, Segment, SizedPara, TextStyle, Word};
use crate::nfss::FamilyKind;
use crate::style::{Skip, Stylesheet};

/// `\lst@Key{numbersep}{10pt}` (lstmisc.sty 1193).
const NUMBERSEP_PT: f64 = 10.0;
/// `\lst@Key{framerule}{.4pt}` (lstmisc.sty 1370).
const FRAMERULE_PT: f64 = 0.4;
/// `\lst@Key{framesep}{3pt}` (lstmisc.sty 1371).
const FRAMESEP_PT: f64 = 3.0;
/// `\lst@Key{basewidth}{0.6em,0.45em}` (listings.sty 533); the first value
/// is the fixed-column one.
const BASEWIDTH_EM: f64 = 0.6;
/// `\lst@Key{abovecaptionskip}`/`{belowcaptionskip}`: `\smallskipamount`.
const CAPTIONSKIP: Skip = Skip { natural: 3.0, stretch: 1.0, shrink: 1.0 };
/// `\lst@Key{aboveskip}\medskipamount` / `{belowskip}\medskipamount`
/// (listings.sty 1735-1736).
const MEDSKIP: Skip = Skip { natural: 6.0, stretch: 2.0, shrink: 2.0 };
/// `\lstlistingname`.
const LISTING_NAME: &str = "Listing";
/// The character width of a monospaced face as a fraction of its quad:
/// `ec-lmtt10` and `cmtt10` both have `QUAD 1.05` and every character
/// `0.525`. [`column_fill_em`] needs the relation, not the numbers.
const MONO_WIDTH_EM: f64 = 0.5;

/// Which sides of the frame carry a rule (`\lst@frame`'s letters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Frame {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Frame {
    pub fn any(self) -> bool {
        self.top || self.bottom || self.left || self.right
    }

    /// `\lst@Key{frame}` (lstmisc.sty 1387-1400): the named values, else the
    /// literal letter string, where an upper-case letter is a double rule
    /// (set here as a single one, which is what `Frame` can carry).
    fn parse(value: &str) -> Frame {
        let letters = match value.trim() {
            "none" | "" => "",
            "leftline" => "l",
            "topline" => "t",
            "bottomline" => "b",
            "lines" => "tb",
            "single" => "trbl",
            "shadowbox" => "tRBl",
            other => other,
        };
        let mut f = Frame::default();
        for c in letters.chars() {
            match c.to_ascii_lowercase() {
                't' => f.top = true,
                'b' => f.bottom = true,
                'l' => f.left = true,
                'r' => f.right = true,
                _ => {}
            }
        }
        f
    }
}

/// `numbers=none|left|right` (lstmisc.sty 1181-1190).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Numbers {
    #[default]
    None,
    Left,
    Right,
}

/// A `listings` style value: a list of font declarations
/// (`basicstyle=\ttfamily\small`). Colour commands are read and dropped —
/// this module sets no colour of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Decl {
    pub family: Option<FamilyKind>,
    pub size: Option<flashtex_document_style::SizeName>,
    pub bold: bool,
    pub italic: bool,
    /// A declaration in the value this module does not model (`\color`
    /// excepted: it is deliberately dropped).
    pub unmodelled: bool,
}

impl Decl {
    fn parse(value: &str) -> Decl {
        use flashtex_document_style::SizeName as S;
        let mut d = Decl::default();
        let mut rest = value;
        while let Some(at) = rest.find('\\') {
            rest = &rest[at + 1..];
            let end = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
            let (name, tail) = rest.split_at(end);
            rest = tail;
            match name {
                "ttfamily" => d.family = Some(FamilyKind::Tt),
                "rmfamily" | "normalfont" => d.family = Some(FamilyKind::Rm),
                "sffamily" => d.family = Some(FamilyKind::Sf),
                "bfseries" => d.bold = true,
                "mdseries" => d.bold = false,
                "itshape" | "slshape" | "em" => d.italic = true,
                "upshape" => d.italic = false,
                "tiny" => d.size = Some(S::Tiny),
                "scriptsize" => d.size = Some(S::ScriptSize),
                "footnotesize" => d.size = Some(S::FootnoteSize),
                "small" => d.size = Some(S::Small),
                "normalsize" => d.size = Some(S::NormalSize),
                "large" => d.size = Some(S::Large),
                "Large" => d.size = Some(S::LARGE2),
                "LARGE" => d.size = Some(S::LARGE3),
                "huge" => d.size = Some(S::Huge),
                "Huge" => d.size = Some(S::HUGE2),
                // `\color{...}`/`\colorbox` take an argument; the colour is
                // deliberately not applied (see the module docs), and
                // skipping the braces keeps its name out of the scan.
                "color" | "textcolor" | "colorbox" => {
                    rest = rest.trim_start();
                    while rest.starts_with('{') {
                        rest = &rest[balanced(rest).unwrap_or(rest.len())..];
                    }
                }
                "" => {}
                _ => d.unmodelled = true,
            }
        }
        d
    }
}

/// The resolved key values in force for one listing.
#[derive(Debug, Clone, PartialEq)]
pub struct Keys {
    pub basicstyle: Decl,
    pub numbers: Numbers,
    pub numberstyle: Decl,
    pub numbersep_pt: f64,
    pub stepnumber: i64,
    pub firstnumber: i64,
    pub frame: Frame,
    pub framerule_pt: f64,
    pub framesep_pt: f64,
    pub basewidth_em: f64,
    pub xleftmargin_pt: f64,
    pub xrightmargin_pt: f64,
    pub aboveskip: Skip,
    pub belowskip: Skip,
    pub caption: Option<(usize, usize)>,
    pub label: Option<String>,
    pub language: Option<String>,
    pub showstringspaces: bool,
    pub tabsize: usize,
    pub breaklines: bool,
    /// Keys that were given but are not modelled here, in source order and
    /// without duplicates; the limitation names them.
    pub unmodelled: Vec<String>,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys {
            basicstyle: Decl::default(),
            numbers: Numbers::None,
            numberstyle: Decl::default(),
            numbersep_pt: NUMBERSEP_PT,
            stepnumber: 1,
            firstnumber: 1,
            frame: Frame::default(),
            framerule_pt: FRAMERULE_PT,
            framesep_pt: FRAMESEP_PT,
            basewidth_em: BASEWIDTH_EM,
            xleftmargin_pt: 0.0,
            xrightmargin_pt: 0.0,
            aboveskip: MEDSKIP,
            belowskip: MEDSKIP,
            caption: None,
            label: None,
            language: None,
            showstringspaces: true,
            tabsize: 8,
            breaklines: false,
            unmodelled: Vec::new(),
        }
    }
}

impl Keys {
    fn note_unmodelled(&mut self, key: &str) {
        if !self.unmodelled.iter().any(|k| k == key) {
            self.unmodelled.push(key.to_string());
        }
    }

    /// Applies one `key=value` list (`\lstset{...}` or an environment's
    /// `[...]`). `at` is the byte offset of `list` in its document, so a
    /// `caption` value keeps a source range.
    pub fn apply_list(&mut self, list: &str, at: usize) {
        for (key, value, value_at) in key_values(list) {
            self.apply(&key, value.as_deref(), value_at.map(|v| (at + v.0, at + v.1)));
        }
    }

    fn apply(&mut self, key: &str, value: Option<&str>, value_range: Option<(usize, usize)>) {
        // listings spells a boolean key `key`, `key=true` or `key=t`.
        let flag = |v: Option<&str>| !matches!(v, Some("false") | Some("f"));
        let v = value.unwrap_or("").trim();
        match key {
            "basicstyle" => {
                self.basicstyle = Decl::parse(v);
                if self.basicstyle.unmodelled {
                    self.note_unmodelled("basicstyle (part of it)");
                }
            }
            "numberstyle" => self.numberstyle = Decl::parse(v),
            "numbers" => {
                self.numbers = match v {
                    "left" => Numbers::Left,
                    "right" => Numbers::Right,
                    _ => Numbers::None,
                }
            }
            "numbersep" => self.numbersep_pt = dimen_pt(v).unwrap_or(NUMBERSEP_PT),
            "stepnumber" => self.stepnumber = v.trim().parse().unwrap_or(1),
            "firstnumber" => self.firstnumber = v.trim().parse().unwrap_or(1),
            "frame" => self.frame = Frame::parse(v),
            "framerule" => self.framerule_pt = dimen_pt(v).unwrap_or(FRAMERULE_PT),
            "framesep" => self.framesep_pt = dimen_pt(v).unwrap_or(FRAMESEP_PT),
            "xleftmargin" => self.xleftmargin_pt = dimen_pt(v).unwrap_or(0.0),
            "xrightmargin" => self.xrightmargin_pt = dimen_pt(v).unwrap_or(0.0),
            "aboveskip" => self.aboveskip = dimen_pt(v).map_or(MEDSKIP, Skip::fixed),
            "belowskip" => self.belowskip = dimen_pt(v).map_or(MEDSKIP, Skip::fixed),
            "caption" => self.caption = value_range,
            "label" => self.label = Some(v.to_string()),
            "language" => self.language = Some(v.to_string()),
            "showstringspaces" => self.showstringspaces = flag(value),
            "breaklines" => self.breaklines = flag(value),
            "tabsize" => self.tabsize = v.trim().parse().unwrap_or(8),
            // `basewidth={0.6em,0.45em}`: the first value is the fixed one.
            "basewidth" => {
                let fixed = v.split(',').next().unwrap_or(v);
                if let Some(em) = dimen_em(fixed) {
                    self.basewidth_em = em;
                } else {
                    self.note_unmodelled("basewidth");
                }
            }
            // Recognised, deliberately not applied: this module sets
            // geometry, not syntax colouring.
            "" => {}
            _ => self.note_unmodelled(key),
        }
    }
}

/// One `lstlisting` environment and the keys in force for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Listing {
    pub document: usize,
    /// The `\begin{lstlisting}` command itself.
    pub begin: (usize, usize),
    /// The `[...]` options, when given.
    pub options: Option<(usize, usize)>,
    /// The body between the two commands.
    pub body: (usize, usize),
    pub keys: Keys,
    /// `\thelstlisting`: every displayed listing steps the counter, whether
    /// or not it has a caption (`\lst@MakeCaption t` calls
    /// `\lst@HRefStepCounter` in the uncaptioned case).
    pub number: u32,
}

/// One `\lstinline` and the keys in force for it: the byte range of its
/// *delimited argument*, whose characters `\lst@Init` sets in the
/// `basicstyle` face just as a displayed listing's are (`\lstinline` adds
/// only `flexiblecolumns`, so the characters keep their natural advances).
/// The reference sets `\lstinline|ftxc build --watch|` in `SFTT1000` — 10 pt,
/// not the 10.95 pt body size.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineListing {
    /// The whole `\lstinline[keys]<d>...<d>` command, not just the text
    /// between the delimiters: the compiler gives every character of the
    /// argument the *command's* source span, so that is the range a word
    /// has to be matched against.
    pub command: (usize, usize),
    pub document: usize,
    pub keys: Keys,
}

/// Every `\lstset`, `lstlisting` and `\lstinline` of every document, in
/// source order, with the keys in force for each. `\lstset` is global from
/// its point of use; an environment's own `[...]` applies to that listing
/// alone.
pub fn scan(texts: &[&str]) -> (Vec<Listing>, Vec<InlineListing>) {
    let mut global = Keys::default();
    let mut counter = 0u32;
    let mut out = Vec::new();
    let mut inlines = Vec::new();
    for (document, text) in texts.iter().enumerate() {
        let mut at = 0usize;
        while at < text.len() {
            let set = find_command(text, at, "\\lstset");
            let begin = text[at..].find("\\begin{lstlisting}").map(|p| at + p);
            let inline = find_command(text, at, "\\lstinline");
            if let Some(i) = inline {
                if set.is_none_or(|s| i < s) && begin.is_none_or(|b| i < b) {
                    let mut keys = global.clone();
                    let after = i + "\\lstinline".len();
                    let tail = text[after..].trim_start();
                    let mut opt_end = text.len() - tail.len();
                    if tail.starts_with('[') {
                        if let Some(end) = bracketed(tail) {
                            keys.apply_list(&tail[1..end - 1], opt_end + 1);
                            opt_end += end;
                        }
                    }
                    at = opt_end;
                    if let Some(after_arg) = delimited(text, opt_end) {
                        inlines.push(InlineListing { command: (i, after_arg), document, keys });
                        at = after_arg;
                    }
                    continue;
                }
            }
            match (set, begin) {
                (Some(s), b) if b.is_none_or(|b| s < b) => {
                    let after = s + "\\lstset".len();
                    let arg = text[after..].trim_start();
                    let arg_at = text.len() - arg.len();
                    if let Some(end) = balanced(arg) {
                        global.apply_list(&arg[1..end - 1], arg_at + 1);
                        at = arg_at + end;
                    } else {
                        at = after;
                    }
                }
                (_, Some(b)) => {
                    let after = b + "\\begin{lstlisting}".len();
                    let mut keys = global.clone();
                    let mut options = None;
                    let mut content = after;
                    let tail = text[after..].trim_start();
                    if tail.starts_with('[') {
                        let opt_at = text.len() - tail.len();
                        if let Some(end) = bracketed(tail) {
                            keys.apply_list(&tail[1..end - 1], opt_at + 1);
                            options = Some((opt_at, opt_at + end));
                            content = opt_at + end;
                        }
                    }
                    if text.as_bytes().get(content) == Some(&b'\n') {
                        content += 1;
                    }
                    let end_tag = "\\end{lstlisting}";
                    let (body_end, after_end) = match text[content..].find(end_tag) {
                        Some(p) => (content + p, content + p + end_tag.len()),
                        None => (text.len(), text.len()),
                    };
                    counter += 1;
                    let body_end = body_end.saturating_sub(usize::from(text[content..body_end].ends_with('\n')));
                    out.push(Listing {
                        document,
                        begin: (b, after),
                        options,
                        body: (content, body_end.max(content)),
                        keys,
                        number: counter,
                    });
                    at = after_end;
                }
                (None, None) => break,
                (Some(_), None) => unreachable!("the first arm takes every `\\lstset` with no `lstlisting` after it"),
            }
        }
    }
    (out, inlines)
}

/// The `\lstset` commands and every control word inside their arguments.
///
/// The compiler has no `\lstset`, so it reports the command — and, because
/// it then reads the argument as preamble material, every declaration in
/// it: a document with the fixture's `\lstset` gets six errors for one
/// command it never had to typeset. This module reads those declarations,
/// so their diagnostics are superseded; nothing outside a `\lstset`
/// argument is touched.
pub fn lstset_spans(texts: &[&str]) -> Vec<Span> {
    let mut out = Vec::new();
    for (document, text) in texts.iter().enumerate() {
        let mut at = 0usize;
        while let Some(s) = find_command(text, at, "\\lstset") {
            at = s + "\\lstset".len();
            out.push(Span::in_document(DocumentId(document), s, at));
            let arg = text[at..].trim_start();
            let arg_at = text.len() - arg.len();
            let Some(end) = balanced(arg) else { continue };
            let group = &arg[..end];
            let mut i = 0usize;
            while let Some(p) = group[i..].find('\\') {
                let start = arg_at + i + p;
                let rest = &text[start + 1..];
                let len = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
                out.push(Span::in_document(DocumentId(document), start, start + 1 + len.max(1)));
                i += p + 1 + len.max(1);
            }
            at = arg_at + end;
        }
    }
    out
}

/// The whole `\lstset{...}`, command and argument, of every document.
///
/// The compiler has no `\lstset`, so a call in the *body* has its argument
/// set as ordinary text: a probe with `\lstset{basicstyle=\ttfamily
/// \footnotesize,frame=single}` between two paragraphs typeset the line
/// `basicstyle=,frame=single` and pushed everything after it a line down.
/// `\lstset` contributes no material at all, so that text is removed here.
fn lstset_ranges(texts: &[&str]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (document, text) in texts.iter().enumerate() {
        let mut at = 0usize;
        while let Some(s) = find_command(text, at, "\\lstset") {
            at = s + "\\lstset".len();
            let arg = text[at..].trim_start();
            let arg_at = text.len() - arg.len();
            match balanced(arg) {
                Some(end) => {
                    out.push((document, s, arg_at + end));
                    at = arg_at + end;
                }
                None => out.push((document, s, at)),
            }
        }
    }
    out
}

/// Every `lstlisting` of every document (see [`scan`]).
pub fn listings(texts: &[&str]) -> Vec<Listing> {
    scan(texts).0
}

/// The delimited argument `\verb`/`\lstinline` reads from `at`: the next
/// character after any optional argument is the delimiter, except `{`,
/// which closes on `}`; an unclosed one ends at the end of the line
/// (crates/compiler `lexer.rs` 207-224, listings.sty `\lst@InlineM`).
/// Returns the byte after the closing delimiter.
fn delimited(text: &str, at: usize) -> Option<usize> {
    let rest = &text[at..];
    let lead = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    let open_at = at + lead;
    let delimiter = text[open_at..].chars().next()?;
    let close = if delimiter == '{' { '}' } else { delimiter };
    let start = open_at + delimiter.len_utf8();
    let line_end = text[start..].find('\n').map_or(text.len(), |p| start + p);
    match text[start..line_end].find(close) {
        Some(p) => Some(start + p + close.len_utf8()),
        None => Some(line_end),
    }
}

/// `\label` inside an `lstlisting`'s keys: the label key and the listing
/// number it resolves to. `\lst@MakeCaption t` steps `lstlisting` and runs
/// `\label{\lst@label}` for every displayed listing, captioned or not
/// (listings.sty 1638-1641), so `Listing~\ref{lst:build-c}` is the
/// listing's own number.
pub fn label_values(texts: &[&str]) -> Vec<(String, String)> {
    listings(texts)
        .iter()
        .filter_map(|l| l.keys.label.clone().map(|k| (k, l.number.to_string())))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// The `caption` source ranges of every listing, for [`crate::toc::entry_items`]
/// to parse as body text: the value may hold any body command
/// (`caption={Generating a starter \texttt{ftxc.toml}}`).
pub fn caption_spans(texts: &[&str]) -> Vec<Span> {
    listings(texts)
        .iter()
        .filter_map(|l| l.keys.caption.map(|(s, e)| Span::in_document(DocumentId(l.document), s, e)))
        .collect()
}

/// Whether any document has an `lstlisting` at all, so the caller can skip
/// the extra parse pass for the overwhelming majority of documents.
pub fn present(texts: &[&str]) -> bool {
    texts.iter().any(|t| t.contains("\\begin{lstlisting}"))
}

/// The per-character fill of the fixed column grid, in `em` of the
/// basicstyle face: `basewidth - <character width>`. Only a monospaced face
/// has one character width; `None` for anything else, and the caller then
/// leaves the natural advances alone and says so.
pub fn column_fill_em(keys: &Keys) -> Option<f64> {
    if keys.basicstyle.family != Some(FamilyKind::Tt) {
        return None;
    }
    Some(keys.basewidth_em - MONO_WIDTH_EM)
}

/// Rewrites the plain typewriter paragraph the compiler produced for each
/// `lstlisting` into what the keys in force ask for, inserting the caption
/// line before it. Returns the source spans whose compiler diagnostic is
/// now superseded, and one limitation per listing naming what is still not
/// applied.
pub fn apply(
    texts: &[&str],
    blocks: &mut Vec<Block>,
    style: &Stylesheet,
    labels: &Labels,
) -> (Vec<Span>, Vec<(&'static str, Span, String)>) {
    let mut superseded = Vec::new();
    let mut limitations = Vec::new();
    let (found, inlines) = scan(texts);
    // `\lstinline` is `\lst@Init` in text style: its characters are set in
    // the `basicstyle` face too, which is why the reference's
    // `\lstinline|ftxc build --watch|` is `SFTT1000` (10 pt) inside a
    // 10.95 pt paragraph. The compiler already sets it literal and
    // typewriter; only the size is the package's.
    for inline in &inlines {
        let size_cpt = (size_of(style, inline.keys.basicstyle).0 * 100.0).round() as u16;
        if inline.keys.basicstyle.size.is_none() {
            continue;
        }
        for block in blocks.iter_mut() {
            let Block::Paragraph { parts, .. } = block else { continue };
            for part in parts.iter_mut() {
                let ParaPart::Lines(items) = part else { continue };
                for item in items.iter_mut() {
                    let Item::Word(word) = item else { continue };
                    for segment in &mut word.segments {
                        if segment.chars.iter().any(|c| {
                            c.document.0 == inline.document && c.start >= inline.command.0 && c.start < inline.command.1
                        }) {
                            segment.style.size_cpt = size_cpt;
                        }
                    }
                }
            }
        }
    }
    // A body `\lstset` had its argument set as text by the compiler (see
    // `lstset_ranges`); the command typesets nothing, so that material goes.
    let ranges = lstset_ranges(texts);
    if !ranges.is_empty() {
        let inside = |c: &CharSrc| {
            ranges
                .iter()
                .any(|(d, a, b)| c.document.0 == *d && c.start >= *a && c.start < *b)
        };
        for block in blocks.iter_mut() {
            let Block::Paragraph { parts, .. } = block else { continue };
            for part in parts.iter_mut() {
                let ParaPart::Lines(items) = part else { continue };
                items.retain(|item| match item {
                    Item::Word(word) => !word.segments.iter().flat_map(|s| s.chars.iter()).any(&inside),
                    _ => true,
                });
            }
        }
        // A paragraph that was nothing but the `\lstset` line is gone; one
        // left with only glue would set an empty line otherwise.
        blocks.retain(|block| match block {
            Block::Paragraph { parts, .. } => parts.iter().any(|part| match part {
                ParaPart::Lines(items) => items.iter().any(|i| matches!(i, Item::Word(_))),
                _ => true,
            }),
            _ => true,
        });
    }
    // Last first, so an inserted caption never moves a range not yet done.
    for listing in found.iter().rev() {
        let document = listing.document;
        let span = Span::in_document(DocumentId(document), listing.begin.0, listing.begin.1);
        let inside = |b: &Block| -> bool {
            crate::abstractenv::block_span(b)
                .is_some_and(|s| s.document.0 == document && s.start >= listing.body.0 && s.start < listing.body.1)
        };
        let Some(first) = blocks.iter().position(inside) else { continue };
        let last = blocks.iter().rposition(inside).unwrap_or(first);
        let basic = size_of(style, listing.keys.basicstyle);
        let fill_em = column_fill_em(&listing.keys);
        // `\llap{\normalfont \lst@numberstyle{\thelstnumber}\kern
        // \lst@numbersep}` (lstmisc.sty 1183-1186): `\normalfont` is why the
        // reference's numbers are roman, not the typewriter basicstyle.
        let number_src = CharSrc { document: DocumentId(document), start: listing.begin.0, end: listing.begin.1 };
        let number_style = TextStyle {
            family: listing.keys.numberstyle.family.unwrap_or(FamilyKind::Rm),
            bold: listing.keys.numberstyle.bold,
            italic: listing.keys.numberstyle.italic,
            size_cpt: (size_of(style, listing.keys.numberstyle).0 * 100.0).round() as u16,
            ..TextStyle::default()
        };
        for block in &mut blocks[first..=last] {
            let Block::Paragraph {
                parts,
                style: para_style,
                indent,
                sized,
                env_open,
                env_close,
                addvspace_before,
                addvspace_flex,
                endlist_adjust,
                ..
            } = block
            else {
                continue;
            };
            *indent = false;
            *para_style = ParaStyle::FlushLeft;
            // `\lst@Init` opens no list: the body is an ordinary paragraph
            // with `\parshape`, `\parskip\z@` and `\rightskip\z@`
            // (listings.sty 1793-1795, lstmisc.sty 1287). The compiler's
            // lowering made it a `flushleft` paragraph, which carries
            // `\trivlist`'s `\topsep + \partopsep` on both sides — 12pt at
            // an 11pt base, and exactly the error the probe showed before
            // this line: our first code baseline sat 11.95 bp below the
            // reference's and everything after it 23.9 bp below.
            *env_open = None;
            *env_close = false;
            *addvspace_before = 0.0;
            *addvspace_flex = (0.0, 0.0);
            *endlist_adjust = 0.0;
            *sized = Some(SizedPara {
                size_pt: basic.0,
                baselineskip_pt: basic.1,
                parindent_em: None,
                vspace_after_em: 0.0,
                close_skip: None,
            });
            for part in parts.iter_mut() {
                if let ParaPart::Lines(items) = part {
                    *items = code_items(std::mem::take(items), fill_em, &listing.keys, number_style, number_src);
                }
            }
        }
        // The vertical list above the first code line (see the module docs).
        let strut_depth = 0.3 * basic.1;
        let frame_gap = if listing.keys.frame.top {
            style.lineskip_pt + listing.keys.framerule_pt + listing.keys.framesep_pt - strut_depth
        } else {
            0.0
        };
        let caption_items = listing
            .keys
            .caption
            .map(|(s, e)| caption_block(texts, labels, listing, s, e, style));
        let pre = if caption_items.is_some() { CAPTIONSKIP } else { listing.keys.aboveskip };
        if let Some(Block::Paragraph { vspace_before, vspace_flex, .. }) = blocks.get_mut(first) {
            *vspace_before += pre.natural + frame_gap;
            *vspace_flex = (vspace_flex.0 + pre.stretch, vspace_flex.1 + pre.shrink);
        }
        // `\par\penalty-50\vspace\lst@belowskip` after the block, plus the
        // strut depth the bottom rule line hangs below the last baseline.
        let below = listing.keys.belowskip.natural + if listing.keys.frame.bottom { strut_depth } else { 0.0 };
        match blocks.get_mut(last + 1) {
            Some(Block::Paragraph { vspace_before, vspace_flex, .. }) => {
                *vspace_before += below;
                *vspace_flex = (vspace_flex.0 + listing.keys.belowskip.stretch, vspace_flex.1 + listing.keys.belowskip.shrink);
            }
            Some(Block::Heading { vspace_before, .. }) => *vspace_before += below,
            _ => {}
        }
        if let Some(block) = caption_items {
            blocks.insert(first, block);
        }
        superseded.push(span);
        if let Some((s, e)) = listing.options {
            superseded.push(Span::in_document(DocumentId(document), s, e));
        }
        let body = texts
            .get(document)
            .and_then(|t| t.get(listing.body.0..listing.body.1))
            .unwrap_or("");
        let lines = if body.is_empty() { 0 } else { body.split('\n').count() };
        limitations.push(("unsupported_block", span, limitation(&listing.keys, lines, fill_em.is_some(), body)));
    }
    (superseded, limitations)
}

/// What this module did and did *not* do for one listing. Never silent
/// while the feature is half-built: what was applied is listed, and so is
/// every key that was given and not applied.
fn limitation(keys: &Keys, lines: usize, columns: bool, body: &str) -> String {
    let mut applied: Vec<String> = Vec::new();
    if keys.basicstyle.size.is_some() || keys.basicstyle.family.is_some() {
        applied.push("`basicstyle` (its size and that size's `\\baselineskip`)".into());
    }
    if columns {
        applied.push("`columns=[c]fixed` cells of `basewidth`".into());
    }
    if keys.numbers == Numbers::Left {
        applied.push("`numbers=left` with `numberstyle`/`numbersep`/`stepnumber`".into());
    }
    if keys.caption.is_some() {
        applied.push("`caption` (and `label`)".into());
    }
    if keys.frame.any() {
        applied.push("the space `frame` takes".into());
    }

    let mut missing: Vec<String> = Vec::new();
    if keys.frame.any() {
        missing.push("the `frame` rules are not drawn — only the space they occupy is set, because this block model has no rule beside a line".into());
    }
    if keys.numbers == Numbers::Right {
        missing.push("`numbers=right` is not set (only `left` is)".into());
    }
    if !columns {
        // listings sets the grid whatever the face is; only a monospaced one
        // has a single character width, which is what makes the fill one
        // number. The default (empty) `basicstyle` is the body font, and the
        // reference spreads even `plain listing` across its cells — this
        // must say so rather than go quiet on the commonest case of all.
        missing.push(format!(
            "`columns=[c]fixed` is not set: the {} `basicstyle` has no single character width, so the characters keep their natural advances",
            match keys.basicstyle.family {
                Some(FamilyKind::Rm) => "roman",
                Some(FamilyKind::Sf) => "sans",
                Some(FamilyKind::Tt) => "typewriter",
                None => "default (body font)",
            }
        ));
    }
    if let Some(language) = &keys.language {
        missing.push(format!("`language={language}` is read but no keyword, string or comment style is applied"));
    }
    if keys.breaklines {
        missing.push("`breaklines` does not insert listings' per-character discretionaries, so an over-wide line breaks elsewhere than pdflatex breaks it".into());
    }
    if keys.showstringspaces {
        missing.push("`showstringspaces` (true here, and by default) does not mark the blanks inside strings".into());
    }
    if body.contains('\t') {
        missing.push(format!(
            "a tab is one blank, not a jump to the next multiple of `tabsize={}`",
            keys.tabsize
        ));
    }
    if !keys.unmodelled.is_empty() {
        missing.push(format!("the keys {} are not modelled", keys.unmodelled.join(", ")));
    }

    let did = if applied.is_empty() {
        "no key applied".to_string()
    } else {
        applied.join(", ")
    };
    if missing.is_empty() {
        return format!("lstlisting ({lines} line(s)): {did}.");
    }
    format!("lstlisting ({lines} line(s)): {did}. Still missing: {}", missing.join("; "))
}

/// `\lst@MakeCaption t`: `\@makecaption{\lstlistingname~\thelstlisting}{...}`
/// at `\normalsize\normalfont`, centred at the full measure.
fn caption_block(texts: &[&str], labels: &Labels, listing: &Listing, s: usize, e: usize, style: &Stylesheet) -> Block {
    let document = DocumentId(listing.document);
    let source = texts.get(listing.document).copied().unwrap_or("");
    let src = CharSrc { document, start: s, end: e };
    let word = |text: &str| {
        Item::Word(Word {
            segments: vec![Segment {
                chars: text.chars().map(|_| src).collect(),
                text: text.to_string(),
                style: TextStyle::default(),
            }],
        })
    };
    let mut items = vec![
        word(LISTING_NAME),
        // `~`: an ordinary interword space that may not break.
        Item::Space { style: TextStyle::default(), factor: 1000, no_break: true },
        word(&format!("{}:", listing.number)),
        // pdfTeX's space factor after `:` is 2000 (`\sfcode`), which the
        // probe's caption line shows as `\glue 4.83946 plus 3.62674
        // minus 0.60446` where an ordinary space is `3.63054 plus 1.81337`.
        Item::Space { style: TextStyle::default(), factor: 2000, no_break: false },
    ];
    items.extend(
        labels
            .entry_items
            .get(document, s, e)
            .unwrap_or_else(|| crate::adapter::words_from_source(source, document, s, e)),
    );
    Block::Paragraph {
        parts: vec![ParaPart::Lines(items)],
        indent: false,
        style: ParaStyle::Center,
        env_open: None,
        env_close: false,
        eject_before: false,
        penalty_before: None,
        breaking: Default::default(),
        // `\vspace\lst@aboveskip` then `\abovecaptionskip`.
        vspace_before: listing.keys.aboveskip.natural + CAPTIONSKIP.natural,
        addvspace_before: 0.0,
        addvspace_flex: (0.0, 0.0),
        vspace_flex: (
            listing.keys.aboveskip.stretch + CAPTIONSKIP.stretch,
            listing.keys.aboveskip.shrink + CAPTIONSKIP.shrink,
        ),
        endlist_adjust: 0.0,
        list: None,
        // The caption is `\normalsize`, which is the body size already; the
        // leading it needs comes with `sized`, not from a `leading_pt` of
        // its own.
        leading_pt: None,
        sized: Some(SizedPara {
            size_pt: style.body_size_pt,
            baselineskip_pt: style.baselineskip_pt,
            parindent_em: None,
            vspace_after_em: 0.0,
            close_skip: None,
        }),
    }
}

/// One code line's items: `columns=[c]fixed` cells, and the `\llap`ped
/// line number in front when `numbers` asks for one.
///
/// `columns=[c]fixed` sets every output token in `n` cells of `basewidth`,
/// centred, so the slack `n·fill` is shared by the `n + 1` `\hss`. Tokens
/// are listings' default rule — a maximal run of letters, digits and `_` is
/// one token, every other character is its own — which the reference's own
/// kerns confirm (`def` -> `-78`, the 20-character `write_default_config`
/// -> `-100`). `fill_em` is `None` when the basicstyle is not monospaced
/// and the grid therefore cannot be one number; the characters then keep
/// their natural advances and the limitation says so.
fn code_items(items: Vec<Item>, fill_em: Option<f64>, keys: &Keys, number: TextStyle, src: CharSrc) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::with_capacity(items.len() * 2);
    let mut line = keys.firstnumber;
    let mut opened = false;
    for item in items {
        if !opened {
            opened = true;
            // `\lst@SkipOrPrintLabel`: with a positive `stepnumber` the
            // number is set when it is a multiple of the step (lstmisc.sty
            // 1240-1270), and blank lines are numbered too
            // (`numberblanklines`, true by default) — the reference sets a
            // `3` on the empty third line of the first listing.
            if keys.numbers == Numbers::Left && keys.stepnumber > 0 && line % keys.stepnumber == 0 {
                out.push(Item::Lap {
                    items: vec![
                        Item::Word(Word {
                            segments: vec![Segment {
                                chars: line.to_string().chars().map(|_| src).collect(),
                                text: line.to_string(),
                                style: number,
                            }],
                        }),
                        Item::Kern { amount: pt_dimen(keys.numbersep_pt), style: number },
                    ],
                });
            } else if fill_em.is_some() {
                // A kern is discardable (TeX §879), so the `\hss` opening
                // the line's first cell would be dropped at the forced break
                // before it and every line but the first would start half a
                // cell too far left. In listings that fil is inside the
                // token's own `\hbox`, which never is; here the empty
                // `\hbox` LaTeX already puts in front of a verbatim blank
                // does the same job.
                out.push(Item::LeaveVmode);
            }
        }
        match (item, fill_em) {
            (Item::Word(word), Some(fill)) => {
                for (token, hss) in tokens(&word, fill) {
                    let style = token_style(&token);
                    out.push(kern(hss, style));
                    let mut first = true;
                    for piece in split_chars(&token) {
                        if !first {
                            out.push(kern(hss, style));
                        }
                        first = false;
                        out.push(Item::Word(piece));
                    }
                    out.push(kern(hss, style));
                }
            }
            // A literal blank is one cell too: `\lst@ProcessSpace` outputs a
            // box of one `\lst@width`, and the rigid `\fontdimen2` of a
            // monospaced face is its character width.
            (Item::Space { style, factor, no_break }, Some(fill)) => {
                out.push(Item::Space { style, factor, no_break });
                out.push(kern(fill, style));
            }
            (other, _) => {
                if matches!(other, Item::LineBreak { .. }) {
                    opened = false;
                    line += 1;
                }
                out.push(other);
            }
        }
    }
    out
}

fn token_style(word: &Word) -> TextStyle {
    word.segments.first().map(|s| s.style).unwrap_or_default()
}

/// A dimension of `pt` points.
fn pt_dimen(pt: f64) -> TextDimen {
    let negative = pt < 0.0;
    let v = pt.abs();
    let mut frac = Vec::new();
    let mut rest = v.fract();
    for _ in 0..5 {
        rest *= 10.0;
        let digit = rest.trunc() as u8;
        frac.push(digit.min(9));
        rest -= f64::from(digit);
    }
    TextDimen {
        negative,
        integer: v.trunc() as i32,
        frac,
        unit: DimenUnit::Physical(PhysicalUnit::Pt),
    }
}

/// A kern of `em` ems of `style`'s face.
fn kern(em: f64, style: TextStyle) -> Item {
    Item::Kern { amount: em_dimen(em), style }
}

/// `em` as a [`TextDimen`] (the sign and four fraction digits; `\lst@width`
/// is a multiple of `basewidth`, so four is exact to well under a scaled
/// point at every body size).
fn em_dimen(em: f64) -> TextDimen {
    let negative = em < 0.0;
    let v = em.abs();
    let integer = v.trunc() as i32;
    let mut frac = Vec::new();
    let mut rest = v.fract();
    for _ in 0..5 {
        rest *= 10.0;
        let digit = rest.trunc() as u8;
        frac.push(digit.min(9));
        rest -= f64::from(digit);
    }
    TextDimen { negative, integer, frac, unit: DimenUnit::Em }
}

/// One output token: the word and the `\hss` its `n + 1` fils each get,
/// `n·fill/(n + 1)` in `em`. That one amount is the fill before the token,
/// between each pair of its characters, and after it.
fn tokens(word: &Word, fill_em: f64) -> Vec<(Word, f64)> {
    split_tokens(word)
        .into_iter()
        .map(|token| {
            let n = token.text().chars().count().max(1) as f64;
            let hss = n * fill_em / (n + 1.0);
            (token, hss)
        })
        .collect()
}

/// listings' default token rule (see [`fixed_columns`]).
fn split_tokens(word: &Word) -> Vec<Word> {
    let letter = |c: char| c.is_alphanumeric() || c == '_';
    let mut out: Vec<Word> = Vec::new();
    let mut current: Vec<Segment> = Vec::new();
    let mut open_letters = false;
    let flush = |current: &mut Vec<Segment>, out: &mut Vec<Word>| {
        if !current.is_empty() {
            out.push(Word { segments: std::mem::take(current) });
        }
    };
    for segment in &word.segments {
        for (i, c) in segment.text.chars().enumerate() {
            let is_letter = letter(c);
            if !is_letter || !open_letters {
                flush(&mut current, &mut out);
            }
            open_letters = is_letter;
            let src = segment.chars.get(i).copied().unwrap_or(CharSrc {
                document: DocumentId(0),
                start: 0,
                end: 0,
            });
            match current.last_mut() {
                Some(last) if last.style == segment.style => {
                    last.text.push(c);
                    last.chars.push(src);
                }
                _ => current.push(Segment {
                    text: c.to_string(),
                    chars: vec![src],
                    style: segment.style,
                }),
            }
        }
    }
    flush(&mut current, &mut out);
    out
}

/// One `Word` per character of `word`, keeping each character's source.
fn split_chars(word: &Word) -> Vec<Word> {
    let mut out = Vec::new();
    for segment in &word.segments {
        for (i, c) in segment.text.chars().enumerate() {
            let src = segment.chars.get(i).copied().unwrap_or(CharSrc {
                document: DocumentId(0),
                start: 0,
                end: 0,
            });
            out.push(Word {
                segments: vec![Segment {
                    text: c.to_string(),
                    chars: vec![src],
                    style: segment.style,
                }],
            });
        }
    }
    out
}

/// The point size and `\baselineskip` a `basicstyle` selects, from the
/// class's own `size1x.clo` table; `\normalsize` when it names no size.
fn size_of(style: &Stylesheet, decl: Decl) -> (f64, f64) {
    match decl.size {
        Some(name) => {
            let fs = flashtex_document_style::font_size(style.base, name);
            (fs.size.0, fs.baselineskip.0)
        }
        None => (style.body_size_pt, style.baselineskip_pt),
    }
}

// ---------------------------------------------------------------------------
// key=value scanning

/// Splits a `listings` key list at top-level commas and each entry at its
/// first top-level `=`. Returns `(key, value, value byte range in `list`)`;
/// a braced value is unwrapped and its range is the text inside the braces.
fn key_values(list: &str) -> Vec<(String, Option<String>, Option<(usize, usize)>)> {
    let mut out = Vec::new();
    let bytes = list.as_bytes();
    let (mut start, mut depth, mut i) = (0usize, 0i32, 0usize);
    while i <= bytes.len() {
        let end = i == bytes.len();
        let c = if end { b',' } else { bytes[i] };
        match c {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'\\' if !end => i += 1,
            b',' if depth <= 0 => {
                if let Some(entry) = entry(list, start, i) {
                    out.push(entry);
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn entry(list: &str, start: usize, end: usize) -> Option<(String, Option<String>, Option<(usize, usize)>)> {
    let raw = list.get(start..end)?;
    if raw.trim().is_empty() {
        return None;
    }
    let bytes = raw.as_bytes();
    let (mut depth, mut i) = (0i32, 0usize);
    let mut eq = None;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'\\' => i += 1,
            b'=' if depth <= 0 => {
                eq = Some(i);
                break;
            }
            _ => {}
        }
        i += 1;
    }
    let Some(eq) = eq else {
        return Some((raw.trim().to_string(), None, None));
    };
    let key = raw.get(..eq)?.trim().to_string();
    let value_raw = raw.get(eq + 1..)?;
    let lead = value_raw.len() - value_raw.trim_start().len();
    let trimmed = value_raw.trim();
    let mut from = start + eq + 1 + lead;
    let mut to = from + trimmed.len();
    let value = if trimmed.starts_with('{') && balanced(trimmed) == Some(trimmed.len()) {
        from += 1;
        to -= 1;
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    };
    Some((key, Some(value), Some((from, to))))
}

/// The length of the brace group `s` opens with, braces included.
fn balanced(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'{') {
        return None;
    }
    let (mut depth, mut i) = (0i32, 0usize);
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The length of the bracket group `s` opens with, brackets included;
/// braces inside it are respected (`caption={a, b}`).
fn bracketed(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'[') {
        return None;
    }
    let (mut brace, mut i) = (0i32, 0usize);
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => brace += 1,
            b'}' => brace -= 1,
            b']' if brace <= 0 => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }
    None
}

/// `\command` at or after `from`, not part of a longer control word.
fn find_command(text: &str, from: usize, command: &str) -> Option<usize> {
    let mut at = from;
    while let Some(p) = text[at..].find(command) {
        let pos = at + p;
        let after = pos + command.len();
        if !text[after..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Some(pos);
        }
        at = after;
    }
    None
}

/// A dimension in points; `None` for a font-relative one, which the caller
/// then reports rather than guessing at.
fn dimen_pt(text: &str) -> Option<f64> {
    let d = TextDimen::parse(text.trim())?;
    if !matches!(d.unit, DimenUnit::Physical(_)) {
        return None;
    }
    let cx = flashtex_compiler::text_builtins::DimenContext::default();
    Some(flashtex_compiler::text_builtins::sp_to_pt(d.resolve(&cx)))
}

/// A dimension in `em`; `None` for anything else.
fn dimen_em(text: &str) -> Option<f64> {
    let d = TextDimen::parse(text.trim())?;
    if d.unit != DimenUnit::Em {
        return None;
    }
    let mut v = f64::from(d.integer);
    let mut scale = 0.1;
    for digit in &d.frac {
        v += f64::from(*digit) * scale;
        scale /= 10.0;
    }
    Some(if d.negative { -v } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &str) -> Keys {
        let mut k = Keys::default();
        k.apply_list(list, 0);
        k
    }

    /// `\lst@Key{frame}` (lstmisc.sty 1387-1400): the named values, then the
    /// literal letters, upper case for a double rule.
    #[test]
    fn frame_names_and_letters() {
        assert_eq!(Frame::parse("single"), Frame { top: true, bottom: true, left: true, right: true });
        assert_eq!(Frame::parse("lines"), Frame { top: true, bottom: true, ..Frame::default() });
        assert_eq!(Frame::parse("leftline"), Frame { left: true, ..Frame::default() });
        assert_eq!(Frame::parse("none"), Frame::default());
        assert_eq!(Frame::parse("tb"), Frame { top: true, bottom: true, ..Frame::default() });
        // `shadowbox` is `tRBl`: the two doubled sides still carry a rule.
        assert_eq!(Frame::parse("shadowbox"), Frame { top: true, bottom: true, left: true, right: true });
    }

    /// A style value is a list of declarations; `\color{...}` is read and
    /// its argument skipped, so the colour name never looks like one.
    #[test]
    fn basicstyle_is_a_declaration_list() {
        let k = keys(r"basicstyle=\ttfamily\small");
        assert_eq!(k.basicstyle.family, Some(FamilyKind::Tt));
        assert_eq!(k.basicstyle.size, Some(flashtex_document_style::SizeName::Small));
        assert!(!k.basicstyle.unmodelled);
        let k = keys(r"keywordstyle=\color{codekw}\bfseries");
        assert_eq!(k.unmodelled, vec!["keywordstyle".to_string()]);
        let k = keys(r"numberstyle=\color{grey}\tiny");
        assert_eq!(k.numberstyle.size, Some(flashtex_document_style::SizeName::Tiny));
        assert_eq!(k.numberstyle.family, None, "`\\color` is not a family");
    }

    /// Values may be braced and hold commas and `=`; a bare key is true, and
    /// `key=false` is false.
    #[test]
    fn key_list_splitting() {
        let k = keys("caption={Bootstrap build driver (C), part 2},numbers=left,breaklines");
        assert_eq!(k.numbers, Numbers::Left);
        assert!(k.breaklines);
        assert_eq!(&"caption={Bootstrap build driver (C), part 2},numbers=left,breaklines"[9..43], "Bootstrap build driver (C), part 2");
        assert_eq!(k.caption, Some((9, 43)));
        assert!(!keys("breaklines=false").breaklines);
        assert!(!keys("showstringspaces=f").showstringspaces);
        assert!(keys("").unmodelled.is_empty());
    }

    /// Dimensions come in points; `basewidth`'s first value is the fixed one.
    #[test]
    fn dimensions() {
        assert_eq!(keys("numbersep=5pt").numbersep_pt, 5.0);
        assert!((keys("framesep=1in").framesep_pt - 72.26999).abs() < 1e-4);
        assert!((keys("basewidth={0.6em,0.45em}").basewidth_em - 0.6).abs() < 1e-9);
        assert!((keys("basewidth=0.5em").basewidth_em - 0.5).abs() < 1e-9);
        // A font-relative margin is reported, not guessed at.
        assert_eq!(keys("xleftmargin=17pt").xleftmargin_pt, 17.0);
    }

    /// `\lstset` is global from its point of use; the environment's `[...]`
    /// is that listing's alone, and every displayed listing steps the
    /// counter whether or not it has a caption.
    #[test]
    fn lstset_is_global_and_environment_keys_are_local() {
        let text = concat!(
            "\\lstset{numbers=left,frame=single}\n",
            "\\begin{lstlisting}\nA\n\\end{lstlisting}\n",
            "\\begin{lstlisting}[numbers=none]\nB\n\\end{lstlisting}\n",
            "\\lstset{frame=none}\n",
            "\\begin{lstlisting}\nC\n\\end{lstlisting}\n",
        );
        let found = listings(&[text]);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].keys.numbers, Numbers::Left);
        assert!(found[0].keys.frame.any());
        assert_eq!(found[1].keys.numbers, Numbers::None, "the `[...]` applies here");
        assert_eq!(found[2].keys.numbers, Numbers::Left, "and only here");
        assert!(!found[2].keys.frame.any(), "the second `\\lstset` reached the third listing");
        assert_eq!([found[0].number, found[1].number, found[2].number], [1, 2, 3]);
        for (i, expected) in ["A", "B", "C"].iter().enumerate() {
            assert_eq!(&text[found[i].body.0..found[i].body.1], *expected);
        }
    }

    /// `\lstinline` reads a delimited argument the way `\verb` does, after
    /// an optional key list; `{` closes on `}`.
    #[test]
    fn lstinline_arguments() {
        let text = "\\lstset{basicstyle=\\ttfamily\\small}\nA \\lstinline|x y| B \\lstinline[language=C]!int z! C \\lstinline{p q} D\n";
        let (_, inlines) = scan(&[text]);
        assert_eq!(inlines.len(), 3);
        for (inline, expected) in inlines.iter().zip(["\\lstinline|x y|", "\\lstinline[language=C]!int z!", "\\lstinline{p q}"]) {
            assert_eq!(&text[inline.command.0..inline.command.1], expected);
            assert_eq!(inline.keys.basicstyle.size, Some(flashtex_document_style::SizeName::Small));
        }
        assert_eq!(inlines[1].keys.language.as_deref(), Some("C"));
        assert_eq!(inlines[0].keys.language, None, "the `[...]` did not escape");
    }

    /// For a monospaced face the fixed grid is one number: `ec-lmtt10` and
    /// `cmtt10` both have `QUAD 1.05` with every character `0.525`, so a
    /// cell of `0.6em` leaves `0.1em` of fill. A face that is not
    /// monospaced has no single character width and gets none.
    #[test]
    fn the_column_fill_needs_a_monospaced_basicstyle() {
        let k = keys(r"basicstyle=\ttfamily\small");
        assert!((column_fill_em(&k).expect("monospaced") - 0.1).abs() < 1e-9);
        let k = keys(r"basicstyle=\rmfamily\small");
        assert_eq!(column_fill_em(&k), None);
        let k = keys(r"basicstyle=\ttfamily\small,basewidth=0.55em");
        assert!((column_fill_em(&k).expect("monospaced") - 0.05).abs() < 1e-9);
    }

    /// A `\lstset` command and every control word in its argument, so the
    /// compiler's preamble errors for them are superseded — and nothing
    /// else is.
    #[test]
    fn lstset_spans_cover_the_argument_only() {
        let text = "\\lstset{basicstyle=\\ttfamily\\small}\n\\hypersetup{colorlinks=true}\n";
        let spans: Vec<(usize, usize)> = lstset_spans(&[text]).iter().map(|s| (s.start, s.end)).collect();
        let named: Vec<&str> = spans.iter().map(|(a, b)| &text[*a..*b]).collect();
        assert_eq!(named, vec!["\\lstset", "\\ttfamily", "\\small"]);
    }
}
