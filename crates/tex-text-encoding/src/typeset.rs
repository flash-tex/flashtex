//! Typesetting one line of LaTeX text into an `\hbox`, the way pdflatex does it
//! for the encoding-related commands this crate models.
//!
//! This is not a TeX macro engine: LaTeX's text-command machinery
//! (`\DeclareText*`, `\@changed@cmd`, `\add@accent`, `\UseTextSymbol`,
//! `\UseTextAccent`, `\@text@composite`, `\CheckEncodingSubset`) is implemented
//! directly, and the handful of macro-built constructs (`\b`, `\c`, `\d`, `\k`,
//! OT1 `\L`/`\l`/`\ij`/`\r A`, `\textellipsis`, `\textcommabelow`) are
//! transcribed box by box. Anything else is reported in `unsupported`.

use std::collections::VecDeque;

use crate::accent::make_accent;
use crate::encoding::{self, Composite, Declared, Default as KernelDefault, Encoding, Resolution};
use crate::fonts::{text_tfm, Family};
use crate::layout::{hpack, oalign, vbox_to_top, BoxNode, Glue, KernKind, Node, MAX_DIMEN};
use crate::ligkern::{lig_kern_run, RunItem, RunOptions};
use crate::sfcode::{adjust_space_factor, interword_glue, xn_over_d, SfCodes};
use crate::tfm::{Scaled, ScaledFont};
use crate::unicode::{self, InputChar};

/// Supplies TFM metrics by name (loaded at their design size).
pub trait FontProvider {
    fn font(&self, tfm: &str) -> Option<&ScaledFont>;
}

/// Document-level choices that decide encodings and fonts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Setup {
    /// Main text encoding: `OT1` (default) or `T1` (`\usepackage[T1]{fontenc}`).
    pub encoding: Encoding,
    pub family: Family,
    /// `\f@size` in pt (10 for the default article class).
    pub size_pt: f64,
    /// `\ssf@size` for this size (5 for 10pt, size10.clo).
    pub ssf_size_pt: f64,
}

impl Setup {
    pub fn article10(encoding: Encoding, family: Family) -> Setup {
        Setup {
            encoding,
            family,
            size_pt: 10.0,
            ssf_size_pt: 5.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Char(char),
    /// `\char<n>` / `\chardef` token.
    CharCode(u8),
    Space,
    Begin,
    End,
    Cs(String),
    /// Internal: `\@use@text@encoding{E}` (an assignment).
    SetEnc(Encoding),
    /// Internal: end of an `\hmode@bgroup … \egroup` with optional space factor restore.
    Restore {
        enc: Encoding,
        size: f64,
        sf: Option<i32>,
    },
}

/// Tokenizes LaTeX source. `at_letter` gives `@` catcode 11 (package/kernel code).
pub fn tokenize(s: &str, at_letter: bool) -> Vec<Tok> {
    let chars: Vec<char> = s.chars().collect();
    let is_letter = |c: char| c.is_ascii_alphabetic() || (at_letter && c == '@');
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    out.push(Tok::Cs("\\".into()));
                    break;
                }
                if is_letter(chars[i]) {
                    let start = i;
                    while i < chars.len() && is_letter(chars[i]) {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    out.push(Tok::Cs(format!("\\{name}")));
                    while i < chars.len() && matches!(chars[i], ' ' | '\t' | '\n') {
                        i += 1;
                    }
                } else {
                    out.push(Tok::Cs(format!("\\{}", chars[i])));
                    i += 1;
                }
            }
            '{' => {
                out.push(Tok::Begin);
                i += 1;
            }
            '}' => {
                out.push(Tok::End);
                i += 1;
            }
            ' ' | '\t' | '\n' => {
                if !matches!(out.last(), Some(Tok::Space)) {
                    out.push(Tok::Space);
                }
                i += 1;
            }
            '%' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                i += 1;
            }
            '~' => {
                out.extend([Tok::Cs("\\nobreakspace".into()), Tok::Begin, Tok::End]);
                i += 1;
            }
            _ => {
                out.push(Tok::Char(c));
                i += 1;
            }
        }
    }
    out
}

/// TeX's `round_decimals` (§102).
pub fn round_decimals(digits: &str) -> i32 {
    let mut a: i32 = 0;
    for d in digits.bytes().take(17).rev() {
        a = (a + (d - b'0') as i32 * 0o400000) / 10;
    }
    (a + 1) / 2
}

/// `<factor><unit>` as TeX scans it (§448–§455): e.g. `scale_by(".25", ex)`.
pub fn scale_by(factor: &str, unit: Scaled) -> Scaled {
    let neg = factor.starts_with('-');
    let f = factor.trim_start_matches(['-', '+']);
    let (ip, fp) = f.split_once('.').unwrap_or((f, ""));
    let int: i32 = if ip.is_empty() {
        0
    } else {
        ip.parse().unwrap_or(0)
    };
    let v = int * unit + xn_over_d(unit, round_decimals(fp), 65536);
    if neg {
        -v
    } else {
        v
    }
}

/// `print_scaled` (§103).
pub fn print_scaled(mut s: Scaled) -> String {
    let mut out = String::new();
    if s < 0 {
        out.push('-');
        s = -s;
    }
    out.push_str(&(s / 65536).to_string());
    out.push('.');
    let mut s = 10 * (s % 65536) + 5;
    let mut delta = 10;
    loop {
        if delta > 65536 {
            s += 0o100000 - 50000;
        }
        out.push((b'0' + (s / 65536) as u8) as char);
        s = 10 * (s % 65536);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

/// `\strip@pt\dimen` (latex.ltx `\rem@pt`).
pub fn strip_pt(s: Scaled) -> String {
    let p = print_scaled(s);
    match p.strip_suffix(".0") {
        Some(int) => int.to_string(),
        None => p,
    }
}

/// The result of typesetting a line.
#[derive(Debug, Clone)]
pub struct Typeset {
    pub list: Vec<Node>,
    pub hbox: BoxNode,
    /// Errors pdflatex would print (without the leading `! `).
    pub errors: Vec<String>,
    /// Commands/characters this crate does not model.
    pub unsupported: Vec<String>,
    pub space_factor: i32,
}

pub struct Typesetter<'a> {
    fonts: &'a dyn FontProvider,
    setup: Setup,
    enc: Encoding,
    size: f64,
    pub sfcodes: SfCodes,
    pub space_factor: i32,
    queue: VecDeque<Tok>,
    list: Vec<Node>,
    run: Option<(String, Vec<u8>)>,
    groups: Vec<(Encoding, f64)>,
    pub errors: Vec<String>,
    pub unsupported: Vec<String>,
}

/// Typesets `input` as `\setbox0\hbox{input}` would.
pub fn typeset_hbox(fonts: &dyn FontProvider, setup: Setup, input: &str) -> Typeset {
    let mut t = Typesetter::new(fonts, setup);
    t.queue.extend(tokenize(input, false));
    t.run_queue();
    let list = std::mem::take(&mut t.list);
    let hbox = hpack(list.clone(), None);
    Typeset {
        list,
        hbox,
        errors: t.errors,
        unsupported: t.unsupported,
        space_factor: t.space_factor,
    }
}

impl<'a> Typesetter<'a> {
    pub fn new(fonts: &'a dyn FontProvider, setup: Setup) -> Self {
        Typesetter {
            fonts,
            setup,
            enc: setup.encoding,
            size: setup.size_pt,
            sfcodes: SfCodes::default(),
            space_factor: 1000,
            queue: VecDeque::new(),
            list: Vec::new(),
            run: None,
            groups: Vec::new(),
            errors: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    fn child(&self) -> Typesetter<'a> {
        let mut c = Typesetter::new(self.fonts, self.setup);
        c.enc = self.enc;
        c.size = self.size;
        c.sfcodes = self.sfcodes.clone();
        c
    }

    fn tfm(&self) -> String {
        text_tfm(self.setup.family, self.enc, self.size).unwrap_or_default()
    }

    fn font(&self) -> Option<&'a ScaledFont> {
        let name = self.tfm();
        let f = self.fonts.font(&name);
        f
    }

    fn need_font(&mut self) -> Option<&'a ScaledFont> {
        let f = self.font();
        if f.is_none() {
            let msg = format!("font {}", self.tfm());
            if !self.unsupported.contains(&msg) {
                self.unsupported.push(msg);
            }
        }
        f
    }

    fn push_front(&mut self, toks: Vec<Tok>) {
        for t in toks.into_iter().rev() {
            self.queue.push_front(t);
        }
    }

    fn unsupported(&mut self, what: &str) {
        self.flush();
        self.unsupported.push(what.to_string());
    }

    pub fn run_queue(&mut self) {
        while let Some(t) = self.queue.pop_front() {
            self.step(t);
        }
        self.flush();
    }

    fn sub_list(&mut self, toks: Vec<Tok>) -> Vec<Node> {
        let mut c = self.child();
        c.queue.extend(toks);
        c.run_queue();
        self.errors.extend(c.errors);
        self.unsupported.extend(c.unsupported);
        c.list
    }

    fn measure_space_factor(&self, toks: &[Tok]) -> i32 {
        let mut c = self.child();
        c.queue.extend(toks.iter().cloned());
        c.run_queue();
        c.space_factor
    }

    fn step(&mut self, t: Tok) {
        match t {
            Tok::Char(c) => match unicode::classify(c) {
                InputChar::Ascii(b) => self.ascii(b),
                InputChar::Declared { expansion, .. } => self.push_front(tokenize(expansion, true)),
                InputChar::Undeclared { message } => {
                    self.flush();
                    self.errors.push(message);
                }
            },
            Tok::CharCode(b) => self.push_char(b),
            Tok::Space => self.space(false),
            Tok::Begin => {
                self.flush();
                self.groups.push((self.enc, self.size));
            }
            Tok::End => {
                self.flush();
                if let Some((e, s)) = self.groups.pop() {
                    self.enc = e;
                    self.size = s;
                }
            }
            Tok::SetEnc(e) => {
                self.flush();
                self.enc = e;
            }
            Tok::Restore { enc, size, sf } => {
                self.flush();
                self.enc = enc;
                self.size = size;
                if let Some(sf) = sf {
                    self.space_factor = sf;
                }
            }
            Tok::Cs(name) => self.command(&name),
        }
    }

    fn ascii(&mut self, b: u8) {
        match b {
            b'$' | b'^' | b'_' | b'&' | b'#' => {
                self.unsupported(&format!("special character {}", b as char))
            }
            _ => self.push_char(b),
        }
    }

    fn push_char(&mut self, code: u8) {
        let tfm = self.tfm();
        if self.run.as_ref().is_some_and(|(t, _)| *t != tfm) {
            self.flush();
        }
        self.run
            .get_or_insert_with(|| (tfm, Vec::new()))
            .1
            .push(code);
        self.space_factor = adjust_space_factor(self.space_factor, self.sfcodes.get(code));
    }

    fn flush(&mut self) {
        let Some((tfm, codes)) = self.run.take() else {
            return;
        };
        if codes.is_empty() {
            return;
        }
        let Some(font) = self.fonts.font(&tfm) else {
            self.unsupported.push(format!("font {tfm}"));
            return;
        };
        for item in lig_kern_run(font, &codes, RunOptions::default()) {
            match item {
                RunItem::Char { code, ligature } => {
                    let mut n = Node::char(font, code);
                    if let Node::Char { ligature: l, .. } = &mut n {
                        *l = ligature.is_some();
                    }
                    self.list.push(n);
                }
                RunItem::Kern(w) => self.list.push(Node::Kern {
                    width: w,
                    kind: KernKind::Font,
                }),
                RunItem::Disc => {}
            }
        }
    }

    /// A space token (`control` = `\ `, which always uses the normal space; §1041).
    fn space(&mut self, control: bool) {
        self.flush();
        let Some(f) = self.need_font() else { return };
        let g = if control || self.space_factor == 1000 {
            interword_glue(f, 1000)
        } else {
            interword_glue(f, self.space_factor)
        };
        self.list
            .push(Node::glue(Glue::finite(g.width, g.stretch, g.shrink)));
    }

    fn read_arg(&mut self) -> Vec<Tok> {
        while matches!(self.queue.front(), Some(Tok::Space)) {
            self.queue.pop_front();
        }
        match self.queue.pop_front() {
            Some(Tok::Begin) => {
                let mut depth = 1;
                let mut out = Vec::new();
                while let Some(t) = self.queue.pop_front() {
                    match t {
                        Tok::Begin => depth += 1,
                        Tok::End => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    out.push(t);
                }
                out
            }
            Some(t) => vec![t],
            None => vec![],
        }
    }

    fn command(&mut self, name: &str) {
        let alias = |s: &str| vec![Tok::Cs(s.to_string())];
        match name {
            "\\ " => self.space(true),
            "\\@" => {
                self.flush();
                self.space_factor = 1000;
            }
            "\\frenchspacing" => {
                self.flush();
                self.sfcodes.frenchspacing();
            }
            "\\nonfrenchspacing" => {
                self.flush();
                self.sfcodes.nonfrenchspacing();
            }
            "\\nobreakspace" => {
                self.flush();
                self.list.push(Node::Penalty(10000));
                self.space(true);
            }
            "\\relax" | "\\leavevmode" | "\\protect" => self.flush(),
            "\\@tabacckludge" => {
                if let Some(Tok::Char(c)) = self.queue.front().cloned() {
                    self.queue.pop_front();
                    self.queue.push_front(Tok::Cs(format!("\\{c}")));
                }
            }
            // latex.ltx:564–565
            "\\aa" => self.push_front(tokenize("\\r a", true)),
            "\\AA" => self.push_front(tokenize("\\r A", true)),
            // latex.ltx:10084–10095 robust wrappers (text-mode branch)
            "\\S" => self.push_front(alias("\\textsection")),
            "\\P" => self.push_front(alias("\\textparagraph")),
            "\\dag" => self.push_front(alias("\\textdagger")),
            "\\ddag" => self.push_front(alias("\\textdaggerdbl")),
            "\\copyright" => self.push_front(alias("\\textcopyright")),
            "\\pounds" => self.push_front(alias("\\textsterling")),
            "\\dots" | "\\ldots" => self.push_front(alias("\\textellipsis")),
            "\\$" => self.push_front(alias("\\textdollar")),
            "\\{" => self.push_front(alias("\\textbraceleft")),
            "\\}" => self.push_front(alias("\\textbraceright")),
            _ => self.text_command(name),
        }
    }

    fn is_text_command(name: &str) -> bool {
        [
            Encoding::OT1,
            Encoding::T1,
            Encoding::TS1,
            Encoding::OMS,
            Encoding::OML,
        ]
        .iter()
        .any(|e| encoding::declared(*e, name).is_some())
            || encoding::kernel_default(name).is_some()
    }

    fn text_command(&mut self, name: &str) {
        if !Self::is_text_command(name) {
            self.unsupported(name);
            return;
        }
        let enc = self.enc;
        match encoding::resolve(enc, name) {
            Resolution::Declared(d) => {
                if encoding::has_composites(enc, name) {
                    let arg = self.read_arg();
                    let first = match arg.first() {
                        Some(Tok::Char(c)) => c.to_string(),
                        Some(Tok::Cs(n)) => n.clone(),
                        _ => String::new(),
                    };
                    if let Some(comp) = encoding::composite(enc, name, &first) {
                        // \@text@composite discards the rest of the argument.
                        match comp {
                            Composite::Slot(s) => self.push_char(s),
                            Composite::Command(body) => {
                                self.composite_command(enc, name, &first, body)
                            }
                        }
                        return;
                    }
                    self.apply_declared(enc, name, d, Some(arg));
                } else {
                    self.apply_declared(enc, name, d, None);
                }
            }
            Resolution::Default(KernelDefault::Symbol(e)) => self.use_text_symbol(e, name),
            Resolution::Default(KernelDefault::Accent(e)) => self.use_text_accent(e, name),
            Resolution::Default(KernelDefault::Command(body)) => self.default_command(name, body),
            Resolution::Unavailable => {
                self.flush();
                self.errors.push(encoding::unavailable_message(enc, name));
            }
        }
    }

    fn apply_declared(&mut self, enc: Encoding, name: &str, d: Declared, arg: Option<Vec<Tok>>) {
        match d {
            Declared::Symbol(s) => {
                if let Some(arg) = arg {
                    self.push_front(arg);
                }
                self.push_char(s)
            }
            Declared::Accent(s) => {
                let arg = arg.unwrap_or_else(|| self.read_arg());
                self.add_accent(s, arg, true);
            }
            Declared::Command { args, .. } => {
                let arg = if args > 0 {
                    Some(arg.unwrap_or_else(|| self.read_arg()))
                } else {
                    arg
                };
                self.construct(enc, name, arg);
            }
        }
    }

    fn use_text_symbol(&mut self, e: Encoding, name: &str) {
        self.flush();
        let (enc, size) = (self.enc, self.size);
        self.enc = e;
        match encoding::declared(e, name) {
            Some(Declared::Symbol(s)) => self.push_char(s),
            Some(Declared::Command { args, .. }) => {
                let arg = if args > 0 {
                    Some(self.read_arg())
                } else {
                    None
                };
                self.construct(e, name, arg);
            }
            _ => self.unsupported(name),
        }
        self.flush();
        self.enc = enc;
        self.size = size;
    }

    fn use_text_accent(&mut self, e: Encoding, name: &str) {
        self.flush();
        let (outer, size) = (self.enc, self.size);
        let arg = self.read_arg();
        let mut wrapped = vec![Tok::SetEnc(outer)];
        wrapped.extend(arg);
        self.queue.push_front(Tok::Restore {
            enc: outer,
            size,
            sf: None,
        });
        self.enc = e;
        match encoding::declared(e, name) {
            Some(Declared::Accent(s)) => self.add_accent(s, wrapped, true),
            Some(Declared::Command { .. }) => self.construct(e, name, Some(wrapped)),
            _ => self.unsupported(name),
        }
    }

    fn default_command(&mut self, name: &str, body: &'static str) {
        let subset = encoding::ts1_subset(self.setup.family.nfss_name()).unwrap_or(9);
        let digit = body
            .chars()
            .rev()
            .find(|c| c.is_ascii_digit())
            .and_then(|c| c.to_digit(10))
            .unwrap_or(0) as u8;
        // latex.ltx:10430 \CheckEncodingSubset#1#2#3#4#5: `\ifnum#4>subset` selects
        // `#1{#2}#5` (the TS1 glyph), otherwise the fallback `#3#5` (not modelled).
        if body.starts_with("\\CheckEncodingSubset\\UseTextAccent{TS1}") {
            if digit > subset {
                self.use_text_accent(Encoding::TS1, name);
            } else {
                self.unsupported(name);
            }
        } else if body.starts_with("\\CheckEncodingSubset\\UseTextSymbol{TS1}")
            || body.starts_with("\\tc@check@symbol")
        {
            if digit > subset {
                self.use_text_symbol(Encoding::TS1, name);
            } else {
                self.unsupported(name);
            }
        } else if name == "\\textellipsis" {
            self.ellipsis();
        } else if name == "\\textcommabelow" {
            let arg = self.read_arg();
            self.comma_below(arg);
        } else if body.contains('@')
            || body.contains("\\kern")
            || body.contains("box")
            || body.contains("rule")
        {
            self.unsupported(name);
        } else {
            self.push_front(tokenize(body, true));
        }
    }

    fn composite_command(&mut self, enc: Encoding, name: &str, base: &str, body: &'static str) {
        if enc == Encoding::OT1 && name == "\\r" && base == "A" {
            self.ring_a();
        } else if body.contains("\\setbox") || body.contains("\\hbox") {
            self.unsupported(&format!("{name}{{{base}}} composite command"));
        } else {
            self.push_front(tokenize(body, true));
        }
    }

    /// `\add@accent{slot}{arg}` (`restore_sf`) or plain `\accent slot arg`.
    fn add_accent(&mut self, slot: u8, arg: Vec<Tok>, restore_sf: bool) {
        self.flush();
        let (saved_enc, saved_size) = (self.enc, self.size);
        let sf_after = if restore_sf {
            Some(self.measure_space_factor(&arg))
        } else {
            None
        };
        let Some(accent_font) = self.need_font() else {
            return;
        };
        let mut toks: VecDeque<Tok> = arg.into();
        while let Some(Tok::SetEnc(e)) = toks.front() {
            self.enc = *e;
            toks.pop_front();
        }
        let base = match toks.front() {
            Some(Tok::Char(c)) if c.is_ascii() && !c.is_ascii_whitespace() => Some(*c as u8),
            Some(Tok::CharCode(b)) => Some(*b),
            Some(Tok::Cs(n)) => match encoding::declared(self.enc, n) {
                Some(Declared::Symbol(s)) => Some(s),
                _ => None,
            },
            _ => None,
        };
        if base.is_some() {
            toks.pop_front();
        }
        match (base, self.need_font()) {
            (Some(b), Some(bf)) => {
                let p = make_accent(accent_font, slot, (bf, b));
                self.list.push(Node::Kern {
                    width: p.kern_before,
                    kind: KernKind::Accent,
                });
                let acc = Node::char(accent_font, slot);
                if let Some(shift) = p.shift {
                    let mut bx = hpack(vec![acc], None);
                    bx.shift = shift;
                    self.list.push(Node::HBox(bx));
                } else {
                    self.list.push(acc);
                }
                self.list.push(Node::Kern {
                    width: p.kern_after,
                    kind: KernKind::Accent,
                });
                self.list.push(Node::char(bf, b));
            }
            _ => self.list.push(Node::char(accent_font, slot)),
        }
        self.space_factor = 1000;
        let mut rest: Vec<Tok> = toks.into();
        rest.push(Tok::Restore {
            enc: saved_enc,
            size: saved_size,
            sf: sf_after,
        });
        self.push_front(rest);
    }

    fn construct(&mut self, enc: Encoding, name: &str, arg: Option<Vec<Tok>>) {
        let arg_or_empty = |a: Option<Vec<Tok>>| a.unwrap_or_default();
        match (enc, name) {
            (Encoding::OT1 | Encoding::T1, "\\b") => {
                let slot = if enc == Encoding::OT1 { 22 } else { 9 };
                self.bar_below(slot, arg_or_empty(arg));
            }
            (Encoding::OT1 | Encoding::T1, "\\c") => {
                let slot = if enc == Encoding::OT1 { 24 } else { 11 };
                self.cedilla(slot, arg_or_empty(arg));
            }
            (Encoding::OT1 | Encoding::T1, "\\d") => self.dot_below(arg_or_empty(arg)),
            (Encoding::T1, "\\k") => self.ogonek(arg_or_empty(arg), false),
            (Encoding::T1, "\\textogonekcentered") => self.ogonek(arg_or_empty(arg), true),
            (Encoding::OT1, "\\L") => self.ot1_lslash(b'L'),
            (Encoding::OT1, "\\l") => self.ot1_lslash(b'l'),
            (Encoding::OT1, "\\ij") => self.ot1_ij(b'i', b'j'),
            (Encoding::OT1, "\\IJ") => self.ot1_ij(b'I', b'J'),
            (Encoding::OT1, "\\textexclamdown") => {
                self.push_front(vec![Tok::Char('!'), Tok::Char('`')])
            }
            (Encoding::OT1, "\\textquestiondown") => {
                self.push_front(vec![Tok::Char('?'), Tok::Char('`')])
            }
            (_, "\\textfiguredash") => self.push_front(vec![Tok::Cs("\\textendash".into())]),
            (_, "\\texthorizontalbar") => self.push_front(vec![Tok::Cs("\\textemdash".into())]),
            _ => {
                self.unsupported(&format!("{name} ({})", enc.name()));
                if let Some(a) = arg {
                    self.push_front(a);
                }
            }
        }
    }

    fn ex(&self) -> Scaled {
        self.font().map_or(0, |f| f.x_height())
    }

    /// `\ltx@sh@ft{<factor>ex}`: `\kern \strip@pt\fontdimen1\font\dimen@`.
    fn slant_shift(&self, factor: &str) -> Scaled {
        let dimen = scale_by(factor, self.ex());
        let slant = self.font().map_or(0, |f| f.slant());
        scale_by(&strip_pt(slant), dimen)
    }

    fn single_char(&mut self, code: u8) -> Option<Node> {
        self.need_font().map(|f| Node::char(f, code))
    }

    /// `\o@lign{\relax#1\crcr\hidewidth\ltx@sh@ft{-1ex}.\hidewidth}` (ot1enc.def/t1enc.def `\d`).
    fn dot_below(&mut self, arg: Vec<Tok>) {
        self.flush();
        let row0 = self.sub_list(arg);
        let Some(dot) = self.single_char(b'.') else {
            return;
        };
        let row1 = vec![
            Node::glue(Glue::HIDEWIDTH),
            Node::Kern {
                width: self.slant_shift("-1"),
                kind: KernKind::Explicit,
            },
            dot,
            Node::glue(Glue::HIDEWIDTH),
        ];
        let v = oalign(vec![row0, row1], 0, scale_by(".25", self.ex()));
        self.list.push(Node::VBox(v));
        self.space_factor = 1000;
    }

    /// `\o@lign{\relax#1\crcr\hidewidth\ltx@sh@ft{-3ex}\vbox to.2ex{\hbox{\char<slot>}\vss}\hidewidth}`.
    fn bar_below(&mut self, slot: u8, arg: Vec<Tok>) {
        self.flush();
        let row0 = self.sub_list(arg);
        let Some(bar) = self.single_char(slot) else {
            return;
        };
        let ex = self.ex();
        let vb = vbox_to_top(scale_by(".2", ex), hpack(vec![bar], None));
        let row1 = vec![
            Node::glue(Glue::HIDEWIDTH),
            Node::Kern {
                width: self.slant_shift("-3"),
                kind: KernKind::Explicit,
            },
            Node::VBox(vb),
            Node::glue(Glue::HIDEWIDTH),
        ];
        let v = oalign(vec![row0, row1], 0, scale_by(".25", ex));
        self.list.push(Node::VBox(v));
        self.space_factor = 1000;
    }

    /// `\c`: `\setbox\z@\hbox{#1}\ifdim\ht\z@=1ex\accent<slot> #1\else{\ooalign{\unhbox\z@\crcr\hidewidth\char<slot>\hidewidth}}\fi`.
    fn cedilla(&mut self, slot: u8, arg: Vec<Tok>) {
        self.flush();
        let measured = hpack(self.child_list(&arg), None);
        let ex = self.ex();
        if measured.height == ex {
            self.add_accent(slot, arg, false);
            return;
        }
        let row0 = self.sub_list(arg);
        let Some(ced) = self.single_char(slot) else {
            return;
        };
        let row1 = vec![
            Node::glue(Glue::HIDEWIDTH),
            ced,
            Node::glue(Glue::HIDEWIDTH),
        ];
        let v = oalign(vec![row0, row1], -MAX_DIMEN, scale_by(".25", ex));
        self.list.push(Node::VBox(v));
        self.space_factor = 1000;
    }

    fn child_list(&self, toks: &[Tok]) -> Vec<Node> {
        let mut c = self.child();
        c.queue.extend(toks.iter().cloned());
        c.run_queue();
        c.list
    }

    /// T1 `\k`: `\ooalign{\null#1\crcr\hidewidth\char12}`; `\textogonekcentered` adds a trailing `\hidewidth`.
    fn ogonek(&mut self, arg: Vec<Tok>, centered: bool) {
        self.flush();
        let mut row0 = vec![Node::HBox(BoxNode::default())];
        row0.extend(self.sub_list(arg));
        let Some(og) = self.single_char(12) else {
            return;
        };
        let mut row1 = vec![Node::glue(Glue::HIDEWIDTH), og];
        if centered {
            row1.push(Node::glue(Glue::HIDEWIDTH));
        }
        let v = oalign(vec![row0, row1], -MAX_DIMEN, scale_by(".25", self.ex()));
        self.list.push(Node::VBox(v));
        self.space_factor = 1000;
    }

    /// OT1 `\L`: `\setbox\z@\hbox{L}\hb@xt@\wd\z@{\hss\@xxxii L}`; `\l`: `{\@xxxii l}`.
    fn ot1_lslash(&mut self, letter: u8) {
        self.flush();
        if letter == b'l' {
            self.push_char(32);
            self.push_char(b'l');
            self.flush();
            return;
        }
        let wd = hpack(self.child_list(&[Tok::Char('L')]), None).width;
        let mut contents = vec![Node::glue(Glue::SS)];
        contents.extend(self.child_list(&[Tok::CharCode(32), Tok::Char('L')]));
        self.list.push(Node::HBox(hpack(contents, Some(wd))));
        self.space_factor = 1000;
    }

    /// OT1 `\ij`: `\nobreak\hskip\z@skip i\kern-0.02em\nobreak\hskip\z@skip j`.
    fn ot1_ij(&mut self, i: u8, j: u8) {
        self.flush();
        let quad = self.font().map_or(0, |f| f.quad());
        self.list.push(Node::Penalty(10000));
        self.list.push(Node::glue(Glue::ZERO));
        self.push_char(i);
        self.flush();
        self.list.push(Node::Kern {
            width: scale_by("-0.02", quad),
            kind: KernKind::Explicit,
        });
        self.list.push(Node::Penalty(10000));
        self.list.push(Node::glue(Glue::ZERO));
        self.push_char(j);
    }

    /// OT1 `\r A`: `\setbox\z@\hbox{!}\dimen@\ht\z@\advance\dimen@-1ex\rlap{\raise.67\dimen@\hbox{\char23}}A`.
    fn ring_a(&mut self) {
        self.flush();
        let Some(f) = self.need_font() else { return };
        let dimen = f.height(b'!') - f.x_height();
        let mut inner = hpack(vec![Node::char(f, 23)], None);
        inner.shift = -scale_by(".67", dimen);
        let rlap = hpack(vec![Node::HBox(inner), Node::glue(Glue::SS)], Some(0));
        self.list.push(Node::HBox(rlap));
        self.space_factor = 1000;
        self.push_front(vec![Tok::Char('A')]);
    }

    /// `\textellipsis` default: `.\kern\fontdimen3\font` three times.
    fn ellipsis(&mut self) {
        for _ in 0..3 {
            self.push_char(b'.');
            self.flush();
            let k = self.font().map_or(0, |f| f.space_stretch());
            self.list.push(Node::Kern {
                width: k,
                kind: KernKind::Explicit,
            });
        }
    }

    /// `\textcommabelow` default (latex.ltx:10097): `\ooalign{\null#1\crcr\hidewidth
    /// \raise-.31ex\hbox{\fontsize\ssf@size\z@\selectfont,}\hidewidth}`.
    fn comma_below(&mut self, arg: Vec<Tok>) {
        self.flush();
        let ex = self.ex();
        let mut row0 = vec![Node::HBox(BoxNode::default())];
        row0.extend(self.sub_list(arg));
        let small =
            text_tfm(self.setup.family, self.enc, self.setup.ssf_size_pt).unwrap_or_default();
        let Some(sf) = self.fonts.font(&small) else {
            self.unsupported(&format!("font {small}"));
            return;
        };
        let mut inner = hpack(vec![Node::char(sf, b',')], None);
        inner.shift = -scale_by("-.31", ex);
        let row1 = vec![
            Node::glue(Glue::HIDEWIDTH),
            Node::HBox(inner),
            Node::glue(Glue::HIDEWIDTH),
        ];
        let v = oalign(vec![row0, row1], -MAX_DIMEN, scale_by(".25", ex));
        self.list.push(Node::VBox(v));
        self.space_factor = 1000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tex_number_conversions_round_trip() {
        assert_eq!(round_decimals("25"), 16384);
        assert_eq!(print_scaled(65536), "1.0");
        assert_eq!(print_scaled(109226), "1.66666");
        assert_eq!(strip_pt(0), "0");
        assert_eq!(scale_by("-1", 282168), -282168);
        // cmr10 quad 655361sp; pdflatex shows \kern-0.20004 (-13110sp) in OT1 \ij.
        assert_eq!(scale_by("-0.02", 655361), -13110);
    }

    #[test]
    fn tokenizer_skips_spaces_after_control_words_only() {
        assert_eq!(
            tokenize("\\u g\\'e \\@ b", false),
            vec![
                Tok::Cs("\\u".into()),
                Tok::Char('g'),
                Tok::Cs("\\'".into()),
                Tok::Char('e'),
                Tok::Space,
                Tok::Cs("\\@".into()),
                Tok::Space,
                Tok::Char('b'),
            ]
        );
    }
}
