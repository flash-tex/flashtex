//! `\settowidth`, `\settoheight` and `\settodepth` (PLAN3 S2): the natural
//! width, height and depth of `\hbox{<content>}`, set the way pdfTeX sets it
//! from the fonts' TFM metrics.
//!
//! The expansion engine hands over the fully expanded content and the font
//! selector in force ([`crate::font_units`]). That selector names an NFSS
//! request; `font-resources` resolves it to the `.tfm` file LaTeX's font
//! definition files load ([`flashtex_font_resources::nfss`],
//! [`flashtex_font_resources::tfm_files`]), found with the render pipeline's
//! discovery rules ([`flashtex_font_resources::discovery`]). Every dimension
//! is the font's fix_word scaled by TeX's `store_scaled`; characters go
//! through the TFM ligature/kern program with both boundaries (tex.web §1034–
//! 1040); interword glue is `\fontdimen2`, plus `\fontdimen7` at a space
//! factor of 2000 or more, with LaTeX's `\nonfrenchspacing` `\sfcode`s;
//! `\textit`-style commands add the italic correction LaTeX's `\check@icr`
//! adds; OT1 accents are TeX's `\accent` construction (§1123–1125).
//!
//! Content this module does not set (math, rules, commands it does not
//! know) counts as empty and is returned in [`Measured::unmeasured`], so the
//! engine warns instead of storing a silent guess.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use flashtex_font_resources::discovery;
use flashtex_font_resources::nfss::{self, FamilyKind, FontKey, Scheme, Shape};
use flashtex_font_resources::tfm::{BoundaryOptions, FixWord, Tfm};
use flashtex_font_resources::tfm_files;
use flashtex_tex_expansion::{self as tex, CatCode, Measured, Span, Token, TokenKind};
use flashtex_tex_text_encoding::encoding::{self as enc, Declared, Default as KernelDefault, Encoding, Resolution};
use flashtex_tex_text_encoding::sfcode::{adjust_space_factor, SfCodes};

use crate::font_units::{selector_request, FontSetup};
use crate::text_builtins::{self, DimenContext, LogoFont, LogoMetrics, MathSubParams, TextDimen, TextLogo};

type Scaled = i32;

/// The engine's measurer for the document's font setup.
pub(crate) struct TfmBoxMeasurer {
    setup: FontSetup,
    preamble_latin_modern: bool,
    switches: HashMap<&'static str, tex::FontSwitch>,
}

impl TfmBoxMeasurer {
    pub(crate) fn new(setup: FontSetup, preamble_latin_modern: bool) -> Self {
        TfmBoxMeasurer {
            setup,
            preamble_latin_modern,
            switches: crate::font_units::font_switches().into_iter().collect(),
        }
    }
}

impl tex::BoxMeasurer for TfmBoxMeasurer {
    fn width(&self, tokens: &[Token]) -> i64 {
        self.measure(0, tokens).width
    }

    fn height(&self, tokens: &[Token]) -> i64 {
        self.measure(0, tokens).height
    }

    fn depth(&self, tokens: &[Token]) -> i64 {
        self.measure(0, tokens).depth
    }

    fn measure(&self, font: u32, tokens: &[Token]) -> Measured {
        let mut s = Setter::new(self, font);
        s.run(tokens);
        s.finish()
    }
}

/// A TFM loaded at a size.
#[derive(Clone)]
struct Font {
    file: String,
    tfm: Arc<Tfm>,
    size: Scaled,
    encoding: Encoding,
}

impl Font {
    fn fix(&self, v: FixWord) -> Scaled {
        v.scaled(self.size)
    }

    fn param(&self, n: usize) -> Scaled {
        self.tfm.parameter(n).map_or(0, |v| self.fix(v))
    }

    /// `(width, height, depth, italic)` of `code`; `None` when the font has
    /// no such character.
    fn dims(&self, code: u8) -> Option<(Scaled, Scaled, Scaled, Scaled)> {
        let m = self.tfm.char_metrics(code)?;
        Some((self.fix(m.width), self.fix(m.height), self.fix(m.depth), self.fix(m.italic)))
    }
}

/// `.tfm` files by name, parsed once per process from the discovery
/// directories (`FLASHTEX_TFM_DIRS`, the bundled texmf trees, then a host
/// TeX installation's), as the render pipeline finds them.
fn load_tfm(file: &str) -> Option<Arc<Tfm>> {
    static DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<Tfm>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok()?.get(file) {
        return hit.clone();
    }
    let dirs = DIRS.get_or_init(discovery::default_tfm_dirs);
    let loaded = dirs
        .iter()
        .map(|d| d.join(file))
        .find(|p| p.is_file())
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| Tfm::parse_tex_file(&bytes).ok())
        .map(Arc::new);
    cache.lock().ok()?.insert(file.to_string(), loaded.clone());
    loaded
}

/// `ot1cmtt.fd` (TeX Live 2026): `m/n` `<5>..<8>cmtt8 <9>cmtt9
/// <10><10.95>cmtt10 <12>..cmtt12`; `m/it` `cmitt10`, `m/sl` `cmsltt10`,
/// `m/sc` `cmtcsc10` at every size (`bx` is `ssub`stituted by `m` before
/// this is asked).
fn ot1_typewriter_file(key: FontKey, size_pt: f64) -> Option<String> {
    Some(
        match key.shape {
            Shape::N if size_pt < 8.5 => "cmtt8",
            Shape::N if size_pt < 9.5 => "cmtt9",
            Shape::N if size_pt < 11.5 => "cmtt10",
            Shape::N => "cmtt12",
            Shape::It => "cmitt10",
            Shape::Sl => "cmsltt10",
            Shape::Sc => "cmtcsc10",
            _ => return None,
        }
        .to_string()
            + ".tfm",
    )
}

/// The Latin Modern metrics `t1lm*.fd` loads for `key` at `size_pt`.
fn latin_modern_file(key: FontKey, size_pt: f64) -> Option<String> {
    let (otf, _) = tfm_files::latin_modern_outline(key, size_pt);
    tfm_files::latin_modern_tfm(otf.trim_end_matches(".otf"))
}

/// The metric files to try, in order, for a terminal NFSS shape. The later
/// candidates are the ones the render pipeline falls back to when the first
/// is not installed (OT1 typewriter is set with `ec-lmtt` there), so a
/// measurement agrees with what is drawn.
fn metric_candidates(scheme: Scheme, key: FontKey, size_pt: f64) -> Vec<String> {
    let mut out = Vec::new();
    match scheme {
        Scheme::CmT1 => out.extend(tfm_files::ec_tfm_file(key, size_pt)),
        Scheme::CmOt1 if key.family == FamilyKind::Tt => {
            out.extend(ot1_typewriter_file(key, size_pt));
            out.extend(latin_modern_file(key, size_pt));
        }
        Scheme::CmOt1 => out.extend(tfm_files::ot1_tfm_file(key, size_pt)),
        Scheme::LmT1 => out.extend(latin_modern_file(key, size_pt)),
        Scheme::LmOt1 => out.extend(latin_modern_file(key, size_pt).map(|f| f.replacen("ec-", "rm-", 1))),
    }
    out
}

/// A TeX size in pt as `\fontsize` scans it (whole sp).
fn size_sp(size_pt: f64) -> Scaled {
    (size_pt * 65536.0).round() as Scaled
}

#[derive(Clone, Copy)]
struct Group {
    font: u32,
    /// Opened by a text-font command (`\textit{..}`): its end runs
    /// LaTeX's `\check@icr`.
    text_command: bool,
}

struct Setter<'a> {
    m: &'a TfmBoxMeasurer,
    font: u32,
    groups: Vec<Group>,
    /// The next group belongs to this text-font command.
    pending_text_command: Option<tex::FontSwitch>,
    run: Option<(Font, Vec<u8>)>,
    width: i64,
    height: i64,
    depth: i64,
    space_factor: i32,
    sfcodes: SfCodes,
    /// Italic correction of the last item when it is a character.
    last_italic: Option<Scaled>,
    unmeasured: Vec<(String, Span)>,
    fonts: HashMap<u32, Option<Font>>,
}

impl<'a> Setter<'a> {
    fn new(m: &'a TfmBoxMeasurer, font: u32) -> Self {
        Setter {
            m,
            font,
            groups: Vec::new(),
            pending_text_command: None,
            run: None,
            width: 0,
            height: 0,
            depth: 0,
            space_factor: 1000,
            sfcodes: SfCodes::default(),
            last_italic: None,
            unmeasured: Vec::new(),
            fonts: HashMap::new(),
        }
    }

    fn finish(mut self) -> Measured {
        self.flush();
        Measured {
            width: self.width,
            height: self.height,
            depth: self.depth,
            unmeasured: self.unmeasured,
        }
    }

    fn scheme(&self, selector: u32) -> Scheme {
        let (_, _, body) = selector_request(selector);
        let lm = if body { self.m.setup.latin_modern } else { self.m.preamble_latin_modern };
        match (lm, self.m.setup.t1) {
            (false, false) => Scheme::CmOt1,
            (false, true) => Scheme::CmT1,
            (true, false) => Scheme::LmOt1,
            (true, true) => Scheme::LmT1,
        }
    }

    /// The font `selector` loads, at `size_override` sp when given.
    fn load(&self, selector: u32, size_override: Option<f64>) -> Result<Font, String> {
        let (request, level, _) = selector_request(selector);
        let scheme = self.scheme(selector);
        let key = nfss::select(scheme, request).key;
        let size_pt = size_override.unwrap_or_else(|| self.m.setup.size_pt(level));
        let candidates = metric_candidates(scheme, key, size_pt);
        for file in &candidates {
            if let Some(tfm) = load_tfm(file) {
                let encoding = if matches!(scheme, Scheme::CmT1 | Scheme::LmT1) || file.starts_with("ec-") {
                    Encoding::T1
                } else {
                    Encoding::OT1
                };
                return Ok(Font { file: file.clone(), tfm, size: size_sp(size_pt), encoding });
            }
        }
        Err(match candidates.first() {
            Some(file) => format!("text in the font {file} (metrics not found)"),
            None => format!("text in the undeclared font shape {key:?}"),
        })
    }

    fn current(&mut self, span: Span) -> Option<Font> {
        let selector = self.font;
        if !self.fonts.contains_key(&selector) {
            let loaded = self.load(selector, None);
            if let Err(what) = &loaded {
                self.unmeasured.push((what.clone(), span));
            }
            self.fonts.insert(selector, loaded.ok());
        }
        self.fonts[&selector].clone()
    }

    fn add_box(&mut self, w: Scaled, h: Scaled, d: Scaled) {
        self.width += i64::from(w);
        self.height = self.height.max(i64::from(h));
        self.depth = self.depth.max(i64::from(d));
    }

    fn add_space(&mut self, w: Scaled) {
        self.flush();
        self.width += i64::from(w);
        self.last_italic = None;
    }

    /// Sets the pending run of characters: one ligature/kern program run.
    fn flush(&mut self) {
        let Some((font, codes)) = self.run.take() else {
            return;
        };
        let run = match font.tfm.glyph_run(&codes, BoundaryOptions::default()) {
            Ok(run) => run,
            Err(e) => {
                self.unmeasured.push((format!("text in {} ({e:?})", font.file), Span::synthetic()));
                return;
            }
        };
        self.width += i64::from(font.fix(run.leading_kern));
        let mut last = None;
        for g in &run.glyphs {
            match font.dims(g.code) {
                Some((w, h, d, i)) => {
                    self.add_box(w, h, d);
                    last = Some(i);
                }
                None => self.unmeasured.push((format!("character {} of {}", g.code, font.file), Span::synthetic())),
            }
            if g.kern_after.0 != 0 {
                self.width += i64::from(font.fix(g.kern_after));
                last = None;
            }
        }
        self.last_italic = last;
    }

    /// Appends character `code` of the current font.
    fn char_code(&mut self, code: u8, span: Span) {
        let Some(font) = self.current(span) else {
            return;
        };
        if font.dims(code).is_none() {
            self.flush();
            self.unmeasured.push((format!("character {code} (missing from {})", font.file), span));
            return;
        }
        if self.run.as_ref().is_some_and(|(f, _)| f.file != font.file || f.size != font.size) {
            self.flush();
        }
        self.run.get_or_insert_with(|| (font, Vec::new())).1.push(code);
        self.space_factor = adjust_space_factor(self.space_factor, self.sfcodes.get(code));
    }

    fn interword_space(&mut self, span: Span, control_space: bool) {
        let Some(font) = self.current(span) else {
            return;
        };
        let mut w = font.param(2);
        if !control_space && self.space_factor >= 2000 {
            w += font.param(7);
        }
        self.add_space(w);
    }

    fn quad(&mut self, span: Span) -> (Scaled, Scaled) {
        self.current(span).map_or((0, 0), |f| (f.param(6), f.param(5)))
    }

    fn kern(&mut self, w: Scaled) {
        self.flush();
        self.width += i64::from(w);
        self.last_italic = None;
    }

    fn italic_correction(&mut self) {
        self.flush();
        if let Some(i) = self.last_italic.take() {
            self.width += i64::from(i);
        }
    }

    fn open_group(&mut self, text_command: Option<tex::FontSwitch>, next: Option<&Token>, span: Span) {
        self.flush();
        self.groups.push(Group { font: self.font, text_command: text_command.is_some() });
        if let Some(switch) = text_command {
            self.font = switch.apply(self.font);
            // `\check@icl` (`\maybe@ic` in the new font): the italic
            // correction of the character before the command.
            self.maybe_ic(next, span);
        }
    }

    /// latex.ltx `\maybe@ic`: in an upright current font (`\fontdimen1`
    /// not positive), `\/` unless the next token is in `\nocorrlist`
    /// (`,.`). `\/` adds the italic correction of a final character.
    fn maybe_ic(&mut self, next: Option<&Token>, span: Span) {
        let upright = self.current(span).is_some_and(|f| f.param(1) <= 0);
        let nocorr = matches!(next.map(|t| &t.kind), Some(TokenKind::Char('.' | ',', _)));
        if upright && !nocorr {
            self.italic_correction();
        }
    }

    fn close_group(&mut self, next: Option<&Token>, span: Span) {
        self.flush();
        let Some(group) = self.groups.pop() else {
            return;
        };
        self.font = group.font;
        if group.text_command {
            // `\check@icr`: `\aftergroup\maybe@ic`, in the outer font.
            self.maybe_ic(next, span);
        }
    }

    fn run(&mut self, tokens: &[Token]) {
        let mut i = 0;
        while i < tokens.len() {
            i = self.step(tokens, i);
        }
    }

    /// Handles `tokens[i]` (and any arguments it takes); returns the index
    /// of the next unread token.
    fn step(&mut self, tokens: &[Token], i: usize) -> usize {
        let t = &tokens[i];
        let span = t.span;
        if let Some(switch) = self.pending_text_command.take() {
            if matches!(t.kind, TokenKind::Char(_, CatCode::BeginGroup)) {
                let empty = matches!(tokens.get(i + 1).map(|t| &t.kind), Some(TokenKind::Char(_, CatCode::EndGroup)));
                if empty {
                    // `\text@command`: no corrections around an empty argument.
                    self.flush();
                    self.groups.push(Group { font: self.font, text_command: false });
                    self.font = switch.apply(self.font);
                } else {
                    self.open_group(Some(switch), tokens.get(i + 1), span);
                }
                return i + 1;
            }
            // A one-token argument.
            self.open_group(Some(switch), Some(t), span);
            let next = self.step(tokens, i);
            self.close_group(tokens.get(next), span);
            return next;
        }
        match &t.kind {
            TokenKind::Char(c, cat) => match cat {
                CatCode::BeginGroup => self.open_group(None, None, span),
                CatCode::EndGroup => self.close_group(tokens.get(i + 1), span),
                CatCode::Space => self.interword_space(span, false),
                CatCode::Letter | CatCode::Other => self.text_char(*c, span),
                CatCode::MathShift => return self.skip_math(tokens, i),
                _ => self.unmeasured.push((format!("the character `{c}`"), span)),
            },
            TokenKind::ActiveChar('~') => {
                self.flush();
                self.interword_space(span, true);
            }
            TokenKind::ControlSequence(name) => return self.command(name, tokens, i),
            TokenKind::ActiveChar(c) => self.unmeasured.push((format!("the active character `{c}`"), span)),
            TokenKind::Param(_) | TokenKind::Eof => {}
        }
        i + 1
    }

    fn text_char(&mut self, c: char, span: Span) {
        if c.is_ascii() {
            self.char_code(c as u8, span);
        } else {
            self.flush();
            self.unmeasured.push((format!("the character `{c}`"), span));
        }
    }

    /// `$...$`: math is not set here (PLAN3 S3).
    fn skip_math(&mut self, tokens: &[Token], i: usize) -> usize {
        self.flush();
        self.unmeasured.push(("math".to_string(), tokens[i].span));
        let mut j = i + 1;
        while j < tokens.len() && !matches!(tokens[j].kind, TokenKind::Char(_, CatCode::MathShift)) {
            j += 1;
        }
        // `$$` closes with two.
        while j < tokens.len() && matches!(tokens[j].kind, TokenKind::Char(_, CatCode::MathShift)) {
            j += 1;
        }
        j
    }

    /// The tokens of the argument at `tokens[i]` (a braced group's
    /// contents or one token) and the index after it.
    fn argument(tokens: &[Token], mut i: usize) -> (&[Token], usize) {
        while i < tokens.len() && matches!(tokens[i].kind, TokenKind::Char(_, CatCode::Space)) {
            i += 1;
        }
        if i >= tokens.len() {
            return (&[], i);
        }
        if !matches!(tokens[i].kind, TokenKind::Char(_, CatCode::BeginGroup)) {
            return (&tokens[i..i + 1], i + 1);
        }
        let mut depth = 0usize;
        for (j, t) in tokens.iter().enumerate().skip(i) {
            match t.kind {
                TokenKind::Char(_, CatCode::BeginGroup) => depth += 1,
                TokenKind::Char(_, CatCode::EndGroup) => {
                    depth -= 1;
                    if depth == 0 {
                        return (&tokens[i + 1..j], j + 1);
                    }
                }
                _ => {}
            }
        }
        (&tokens[i + 1..], tokens.len())
    }

    /// Glue or a kern of the natural width of `text` (`<dimen> plus ..
    /// minus ..`).
    fn dimen_kern(&mut self, text: &str, what: &str, span: Span) {
        let natural = text.split(" plus").next().unwrap_or("");
        let natural = natural.split(" minus").next().unwrap_or("").trim();
        match TextDimen::parse(natural) {
            Some(d) => {
                let cx = self.dimen_context(span);
                self.kern(d.resolve(&cx));
            }
            None => self.unmeasured.push((format!("\\{what}{{{text}}}"), span)),
        }
    }

    fn dimen_context(&mut self, span: Span) -> DimenContext {
        let (quad, x_height) = self.quad(span);
        DimenContext { quad, x_height, text_width: 0, line_width: 0, column_width: 0 }
    }

    fn command(&mut self, name: &str, tokens: &[Token], i: usize) -> usize {
        let span = tokens[i].span;
        if let Some(&switch) = self.m.switches.get(name) {
            self.flush();
            if switch.argument {
                self.pending_text_command = Some(switch);
            } else {
                self.font = switch.apply(self.font);
            }
            return i + 1;
        }
        match name {
            " " | "nobreakspace" => self.interword_space(span, true),
            "@" => {
                self.flush();
                self.space_factor = 1000;
            }
            "/" => self.italic_correction(),
            "relax" | "leavevmode" | "protect" | "nobreak" | "null" | "strut" if name != "strut" => self.flush(),
            "mbox" | "hbox" | "text" | "textnormal@" => {}
            "quad" | "qquad" => {
                let (quad, _) = self.quad(span);
                self.kern(if name == "quad" { quad } else { 2 * quad });
            }
            "enskip" => {
                let (quad, _) = self.quad(span);
                self.kern(quad / 2);
            }
            "hspace" | "hspace*" => {
                let (arg, next) = Self::argument(tokens, i + 1);
                let text: String = arg.iter().map(token_text).collect();
                self.dimen_kern(&text, "hspace", span);
                return next;
            }
            "flashtexhspacedone" => {
                // The engine's `\hspace`: `\flashtexhspacedone[*]{<dimen>}`.
                let mut j = i + 1;
                if matches!(tokens.get(j).map(|t| &t.kind), Some(TokenKind::Char('*', _))) {
                    j += 1;
                }
                let (arg, next) = Self::argument(tokens, j);
                let text: String = arg.iter().map(token_text).collect();
                self.dimen_kern(&text, "hspace", span);
                return next;
            }
            "hskip" | "kern" => {
                // The engine emits `\hskip`/`\kern` with the operand it
                // scanned, printed (`10.0pt plus 1.0fil`), then
                // `\flashtexwordbreak`.
                let mut j = i + 1;
                let mut text = String::new();
                while let Some(TokenKind::Char(c, _)) = tokens.get(j).map(|t| &t.kind) {
                    text.push(*c);
                    j += 1;
                }
                self.dimen_kern(&text, name, span);
                return j;
            }
            "flashtexwordbreak" => {}
            "AA" | "aa" => {
                // latex.ltx: `\r A`, `\r a`.
                let base = if name == "AA" { 'A' } else { 'a' };
                let arg = [Token::new(TokenKind::Char(base, CatCode::Letter), span)];
                let slot = self.current(span).and_then(|f| match enc::resolve(f.encoding, "\\r") {
                    Resolution::Declared(Declared::Accent(slot)) => Some(slot),
                    _ => None,
                });
                match slot {
                    Some(slot) => self.accent("\\r", slot, &arg, span),
                    None => self.unmeasured.push((format!("\\{name}"), span)),
                }
            }
            "TeX" | "LaTeX" => self.logo(if name == "TeX" { TextLogo::TeX } else { TextLogo::LaTeX }, span),
            _ => {
                if let Some(d) = text_builtins::text_kern(name, false) {
                    let cx = self.dimen_context(span);
                    self.kern(d.resolve(&cx));
                } else {
                    return self.text_command(name, tokens, i);
                }
            }
        }
        i + 1
    }

    /// An encoding-dependent text command: a symbol or an accent of the
    /// current encoding, or a kernel default in another encoding's font.
    fn text_command(&mut self, name: &str, tokens: &[Token], i: usize) -> usize {
        let span = tokens[i].span;
        let alias = match name {
            "S" => "textsection",
            "P" => "textparagraph",
            "dag" => "textdagger",
            "ddag" => "textdaggerdbl",
            "copyright" => "textcopyright",
            "pounds" => "textsterling",
            "dots" | "ldots" => "textellipsis",
            "$" => "textdollar",
            "{" => "textbraceleft",
            "}" => "textbraceright",
            other => other,
        };
        let cs = format!("\\{alias}");
        let Some(font) = self.current(span) else {
            return i + 1;
        };
        match enc::resolve(font.encoding, &cs) {
            Resolution::Declared(Declared::Symbol(slot)) => self.char_code(slot, span),
            Resolution::Declared(Declared::Accent(slot)) => {
                let (arg, next) = Self::argument(tokens, i + 1);
                self.accent(&cs, slot, arg, span);
                return next;
            }
            Resolution::Default(KernelDefault::Symbol(other)) => {
                if let Resolution::Declared(Declared::Symbol(slot)) = enc::resolve(other, &cs) {
                    self.foreign_symbol(other, slot, span);
                } else {
                    self.unmeasured.push((format!("\\{name}"), span));
                }
            }
            Resolution::Declared(Declared::Command { .. }) | Resolution::Default(KernelDefault::Command(_))
                if alias == "textellipsis" =>
            {
                // latex.ltx `\textellipsis`: `.\kern\fontdimen3\font` three
                // times.
                let thin = font.param(3);
                for _ in 0..3 {
                    self.char_code(b'.', span);
                    self.kern(thin);
                }
            }
            _ => self.unmeasured.push((format!("\\{name}"), span)),
        }
        i + 1
    }

    /// A symbol from another encoding's font of the same family, series,
    /// shape and size (`\UseTextSymbol`): TS1 `tc*`/`ts1-lm*`, OMS `cmsy`/`lmsy`.
    fn foreign_symbol(&mut self, encoding: Encoding, slot: u8, span: Span) {
        self.flush();
        let Some(font) = self.current(span) else {
            return;
        };
        let size_pt = f64::from(font.size) / 65536.0;
        let file = match encoding {
            // `ts1lmr.fd` pairs `rm-lm*` (OT1) with the same `ts1-lm*` file
            // as `ec-lm*` (T1).
            Encoding::TS1 => tfm_files::ts1_companion_tfm(&font.file.replacen("rm-lm", "ec-lm", 1), size_pt),
            Encoding::OMS => Some(oms_file(&font.file, size_pt)),
            _ => None,
        };
        let Some(other) = file.and_then(|f| load_tfm(&f).map(|t| (f, t))) else {
            self.unmeasured.push((format!("a {} symbol (metrics not found)", encoding.name()), span));
            return;
        };
        let f = Font { file: other.0, tfm: other.1, size: font.size, encoding };
        match f.dims(slot) {
            Some((w, h, d, i)) => {
                self.add_box(w, h, d);
                self.last_italic = Some(i);
            }
            None => self.unmeasured.push((format!("character {slot} of {}", f.file), span)),
        }
    }

    /// `\<accent>{<base>}`: a precomposed character when the encoding
    /// declares one, else TeX's `\accent` (§1123–1125), whose box is as
    /// wide as the base.
    fn accent(&mut self, cs: &str, accent_slot: u8, arg: &[Token], span: Span) {
        let base: String = arg.iter().map(token_text).collect();
        let Some(font) = self.current(span) else {
            return;
        };
        if let Some(c) = enc::composite(font.encoding, cs, &base) {
            if let Some(slot) = composite_slot(&c) {
                self.char_code(slot, span);
                return;
            }
            if font.encoding == Encoding::OT1 && cs == "\\r" && base == "A" {
                self.ring_a(&font, span);
                return;
            }
        }
        let base_code = match base.as_str() {
            b if b.len() == 1 && b.is_ascii() => b.as_bytes()[0],
            "\\i" | "\\j" => match enc::resolve(font.encoding, &base) {
                Resolution::Declared(Declared::Symbol(s)) => s,
                _ => {
                    self.unmeasured.push((format!("{cs}{{{base}}}"), span));
                    return;
                }
            },
            _ => {
                self.unmeasured.push((format!("{cs}{{{base}}}"), span));
                return;
            }
        };
        self.flush();
        let (Some((a_w, a_h, a_d, _)), Some((w, h, d, i))) = (font.dims(accent_slot), font.dims(base_code)) else {
            self.unmeasured.push((format!("{cs}{{{base}}}"), span));
            return;
        };
        let _ = a_w;
        let x = font.param(5);
        let (acc_h, acc_d) = if h != x { (a_h + (h - x), a_d - (h - x)) } else { (a_h, a_d) };
        self.add_box(w, h.max(acc_h), d.max(acc_d));
        self.last_italic = Some(i);
        self.space_factor = 1000;
    }

    /// OT1 `\r A` (ot1enc.def): `\setbox\z@\hbox{!}\dimen@\ht\z@
    /// \advance\dimen@-1ex\rlap{\raise.67\dimen@\hbox{\char23}}A`.
    fn ring_a(&mut self, font: &Font, span: Span) {
        self.flush();
        let (Some((_, bang_h, _, _)), Some((_, ring_h, ring_d, _))) = (font.dims(b'!'), font.dims(23)) else {
            self.unmeasured.push(("\\r{A}".to_string(), span));
            return;
        };
        let dimen = bang_h - font.param(5);
        let raise = flashtex_tex_boxes::scaled::scale_internal(false, 0, &[6, 7], dimen).unwrap_or(0);
        self.add_box(0, ring_h + raise, ring_d - raise);
        self.last_italic = None;
        self.space_factor = 1000;
        self.char_code(b'A', span);
    }

    /// `\TeX` and `\LaTeX` (latex.ltx), from the current font.
    fn logo(&mut self, logo: TextLogo, span: Span) {
        self.flush();
        let Some(font) = self.current(span) else {
            return;
        };
        let script = self.load(self.font, Some(f64::from(text_builtins::sf_size(font.size)) / 65536.0)).ok();
        let metrics = Logo { current: font, script };
        let layout = text_builtins::layout_logo(logo, &metrics);
        for g in &layout.glyphs {
            let b = metrics.char_box(g.font, g.ch);
            self.height = self.height.max(i64::from(b.height + g.raise));
            self.depth = self.depth.max(i64::from(b.depth - g.raise));
        }
        self.width += i64::from(layout.width);
        self.last_italic = None;
        self.space_factor = 1000;
    }
}

struct Logo {
    current: Font,
    script: Option<Font>,
}

impl LogoMetrics for Logo {
    fn char_box(&self, font: LogoFont, ch: char) -> text_builtins::CharBox {
        let f = match font {
            LogoFont::Current => Some(&self.current),
            LogoFont::ScriptSize => self.script.as_ref(),
            LogoFont::MathItalic => None,
        };
        let dims = f.zip(u8::try_from(ch).ok()).and_then(|(f, c)| f.dims(c));
        let (width, height, depth, italic) = dims.unwrap_or((0, 0, 0, 0));
        text_builtins::CharBox { width, height, depth, italic }
    }

    fn quad(&self) -> Scaled {
        self.current.param(6)
    }

    fn x_height(&self) -> Scaled {
        self.current.param(5)
    }

    fn math_sub_params(&self) -> MathSubParams {
        MathSubParams::default()
    }
}

/// The OMS (math symbol) font `omscmr.fd`/`omslmr.fd` substitute for a text
/// font: `cmsy`/`lmsy` at the nearest design size, 5 to 10.
fn oms_file(text_file: &str, size_pt: f64) -> String {
    let design = [5u32, 6, 7, 8, 9, 10].into_iter().rfind(|d| f64::from(*d) <= size_pt + 0.01).unwrap_or(5);
    if text_file.contains("-lm") {
        format!("lmsy{design}.tfm")
    } else {
        format!("cmsy{design}.tfm")
    }
}

fn composite_slot(c: &enc::Composite) -> Option<u8> {
    match c {
        enc::Composite::Slot(s) => Some(*s),
        _ => None,
    }
}

/// The text a token stands for in a dimension or accent argument.
fn token_text(t: &Token) -> String {
    match &t.kind {
        TokenKind::Char(c, _) => c.to_string(),
        TokenKind::ControlSequence(n) => format!("\\{n}"),
        TokenKind::ActiveChar(c) => c.to_string(),
        _ => String::new(),
    }
}
