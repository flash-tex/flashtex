//! `MathFontMetrics` with TeX's own metrics: the Appendix G parameters and
//! glyph boxes of the `lmmi`/`lmsy`/`lmex` TFMs that pdfLaTeX+`lmodern`
//! lays math out with, and `rm-lmr*` for the roman family (digits,
//! parentheses, operators). Latin Modern's math TFMs are metric-identical
//! to Computer Modern's (verified byte for byte on `lmmi12`, `lmsy10`,
//! `lmex10`, `lmmi8`, `lmsy8` against `cmmi12`, `cmsy10`, `cmex10`,
//! `cmmi8`, `cmsy8`), so families 1–3 come from math-layout's embedded
//! `CmMathMetrics` (`latex_12pt`/`latex_10pt`, plus LaTeX's 11pt sizes) and
//! only family 0, where `rm-lmr` differs from `cmr` in heights by up to
//! 0.015 em, is read from the installed TFM.
//!
//! Painting still uses the Latin Modern Math OpenType program: every glyph
//! the layout places is a (TFM font, code) pair that [`TexMathMetrics::otf_gid`]
//! maps to an original glyph id of that face (base glyph by character,
//! `lmex` size chains by index into `MathVariants`). Extensible assemblies
//! are not mapped; a glyph without a mapping is reported, never drawn as
//! `.notdef`.

use std::cell::RefCell;
use std::rc::Rc;

use flashtex_math_layout::cm::{self, CmMathMetrics, Family};
use flashtex_math_layout::cm_tfm;
use flashtex_math_layout::metrics::Extensible;
use flashtex_math_layout::tfm as mtfm;
use flashtex_math_layout::{FontId as MathFontId, Glyph, MathFontMetrics, MathParams, SizeClass};

use crate::fonts::{FontSet, LoadedFace, Role, TfmStatus};
use crate::mathfont::{MathFonts, MathSizes};
use crate::tfm::Tfm;

/// Font id of glyphs [`TexMathMetrics`] takes straight from Latin Modern
/// Math because no CM TFM slot covers the character; `gid` is then the
/// face's own glyph id (see `MathProvider::otf_glyph`).
pub const OTF_FALLBACK_FONT: MathFontId = MathFontId(u32::MAX);
/// Font id of double-struck glyphs taken from the secondary math face (New
/// Computer Modern Math, [`crate::mathfont::BB_FONT`]) the same way.
pub const OTF_FALLBACK_BB_FONT: MathFontId = MathFontId(u32::MAX - 1);

/// Whether `font` is one of the two OpenType-fallback ids (above the
/// `\text` run range; answered by the provider, never looked up as a run).
pub fn is_otf_fallback(font: MathFontId) -> bool {
    font == OTF_FALLBACK_FONT || font == OTF_FALLBACK_BB_FONT
}

/// Font ids of glyphs boxed from the AMS symbol font TFMs (`msam`, `msbm`;
/// math-layout `ams`), `gid` the slot and `ch` the
/// [`crate::mathfont::ams_sentinel`]; painted by [`TexMathMetrics::otf_glyph`].
pub const AMS_MSAM_FONT: MathFontId = MathFontId(u32::MAX - 2);
pub const AMS_MSBM_FONT: MathFontId = MathFontId(u32::MAX - 3);

/// Whether a placed glyph's font id is answered by the math provider even
/// though it lies above the `\text` run range: the OpenType fallbacks and the
/// AMS symbol fonts.
pub fn is_provider_font(font: MathFontId) -> bool {
    is_otf_fallback(font) || font == AMS_MSAM_FONT || font == AMS_MSBM_FONT
}

pub struct TexMathMetrics {
    cm: CmMathMetrics,
    sizes: MathSizes,
    /// `rm-lmr*` at text/script/scriptscript, when installed.
    roman: [Option<Rc<Tfm>>; 3],
    /// Why a roman TFM is absent (the first failure), blocking when it is
    /// a required asset.
    roman_status: Option<TfmStatus>,
    /// The text faces that draw the roman family at text/script/
    /// scriptscript size: `lmroman12/8/6` are the OpenType siblings of the
    /// `lmr12/8/6` Type 1 designs the TFMs describe, so digits, parentheses
    /// and operators keep their optical design instead of Latin Modern
    /// Math's single 10 pt design.
    roman_faces: [Option<Rc<LoadedFace>>; 3],
    /// Resource selection actually made per TFM font: `(face name, exact
    /// optical design?)`, for the provenance report.
    resources: RefCell<std::collections::BTreeMap<String, (String, bool)>>,
    /// The OpenType face drawn (Latin Modern Math) and its variant table.
    otf: Rc<MathFonts>,
    unmapped: RefCell<Vec<(String, u8, char)>>,
    /// `\mathfrak` metrics (`ueuf.fd`: `eufm5/7/10`) at text/script/
    /// scriptscript, when installed.
    fraktur: [Option<Rc<Tfm>>; 3],
    /// Text faces and TFMs of the math alphabets the document uses
    /// ([`TexMathMetrics::with_alphabets`]): alphabet, size index, face, TFM.
    alphabets: Vec<(crate::mathalpha::MathAlphabet, usize, Rc<LoadedFace>, Rc<Tfm>)>,
}

/// Font ids of fraktur glyphs laid out from `eufm` at the three sizes: far
/// below the `\text` run ids and above math-layout's embedded CM ids.
pub const FRAKTUR_FONTS: [MathFontId; 3] = [MathFontId(0x100), MathFontId(0x101), MathFontId(0x102)];

impl TexMathMetrics {
    /// `base` is the document's body size (10/11/12). `otf` supplies the
    /// glyph program; `fonts` supplies `rm-lmr<d>.tfm` (digest-bound for
    /// the 12 pt set).
    pub fn new(base: u32, otf: Rc<MathFonts>, fonts: &FontSet) -> TexMathMetrics {
        let (cm, roman_names) = match base {
            10 => (CmMathMetrics::latex_10pt(), ["rm-lmr10", "rm-lmr7", "rm-lmr5"]),
            11 => (
                // size11.clo: \DeclareMathSizes{\@xipt}{\@xipt}{8}{6} with the
                // 10pt designs scaled to 10.95pt for text.
                CmMathMetrics {
                    sizes: [10.95, 8.0, 6.0],
                    extension: cm::ExtensionSizing::Fixed,
                    families: [
                        [&cm_tfm::CMR10, &cm_tfm::CMR8, &cm_tfm::CMR6],
                        [&cm_tfm::CMMI10, &cm_tfm::CMMI8, &cm_tfm::CMMI6],
                        [&cm_tfm::CMSY10, &cm_tfm::CMSY8, &cm_tfm::CMSY6],
                    ],
                },
                ["rm-lmr10", "rm-lmr8", "rm-lmr6"],
            ),
            _ => (CmMathMetrics::latex_12pt(), ["rm-lmr12", "rm-lmr8", "rm-lmr6"]),
        };
        let sizes = MathSizes {
            text: cm.sizes[0],
            script: cm.sizes[1],
            script_script: cm.sizes[2],
        };
        let mut roman_status = None;
        let mut load = |name: &str| -> Option<Rc<Tfm>> {
            match fonts.tfm(&format!("{name}.tfm")) {
                Ok(t) => Some(t),
                Err(status) => {
                    if roman_status.is_none() {
                        roman_status = Some(status);
                    }
                    None
                }
            }
        };
        let roman = [load(roman_names[0]), load(roman_names[1]), load(roman_names[2])];
        let text_face = |size: f64| -> Option<Rc<LoadedFace>> {
            let r = fonts.resolve(crate::fonts::Family::LatinModern, Role::Text { bold: false, italic: false }, size);
            if r.substituted.is_some() { None } else { Some(r.face) }
        };
        let roman_faces = [text_face(cm.sizes[0]), text_face(cm.sizes[1]), text_face(cm.sizes[2])];
        let fraktur = cm.sizes.map(|at| fonts.tfm(&format!("{}.tfm", crate::mathalpha::fraktur_tfm(at))).ok());
        TexMathMetrics {
            fraktur,
            alphabets: Vec::new(),
            cm,
            sizes,
            roman,
            roman_status,
            roman_faces,
            otf,
            unmapped: RefCell::new(Vec::new()),
            resources: RefCell::new(std::collections::BTreeMap::new()),
        }
    }

    /// `(TFM font, face drawn, exact optical design)` for every TFM font a
    /// glyph was mapped from, drained for the provenance report.
    pub fn take_resources(&self) -> Vec<(String, String, bool)> {
        std::mem::take(&mut *self.resources.borrow_mut())
            .into_iter()
            .map(|(k, (f, e))| (k, f, e))
            .collect()
    }

    /// The face and original glyph id that draw a placed TFM glyph: the
    /// optical-size text face for the roman family, Latin Modern Math (one
    /// 10 pt design) for the italic, symbol and extension families, and the
    /// secondary face (New Computer Modern Math, the compiler's binding for
    /// `\mathcal`, pin `dbf6ec78`) for the cmsy calligraphic capitals when it
    /// is loaded, reported once as its own resource profile because its
    /// script design is not cmsy10's calligraphic one.
    pub fn otf_glyph(&self, font: MathFontId, code: u8, ch: char) -> Option<(Rc<LoadedFace>, u16)> {
        // `\mathfrak`: eufm boxes, Latin Modern Math's fraktur outlines
        // (its fraktur alphabet is the Euler design).
        if let Some(i) = FRAKTUR_FONTS.iter().position(|f| *f == font) {
            let face = self.otf.face();
            let gid = face.face().glyph_id(ch)?;
            self.resources
                .borrow_mut()
                .entry(crate::mathalpha::fraktur_tfm(self.cm.sizes[i]).to_string())
                .or_insert((face.name.clone(), false));
            return Some((face.clone(), gid.0));
        }
        if let Some((_, _, face, _)) = self.alphabets.iter().find(|(a, i, ..)| Self::alphabet_font(*a, *i) == font) {
            let gid = face.face().glyph_id(char::from(code))?;
            self.resources.borrow_mut().entry(face.name.clone()).or_insert((face.name.clone(), true));
            return Some((face.clone(), gid.0));
        }
        let name = self.cm.font_name(font);
        if name.starts_with("cmr") {
            let idx = (0..3).find(|i| self.cm.families[0][*i].name == name).unwrap_or(0);
            if let Some(face) = &self.roman_faces[idx] {
                if let Some(gid) = face.face().glyph_id(ch) {
                    self.resources.borrow_mut().entry(lm_name(&name)).or_insert((face.name.clone(), true));
                    return Some((face.clone(), gid.0));
                }
            }
        }
        if name.starts_with("cmsy") && crate::mathfont::is_script_capital(ch) {
            if let Some(bb) = self.otf.bb_face() {
                if let Some(gid) = bb.face().glyph_id(ch) {
                    self.resources
                        .borrow_mut()
                        .entry(format!("{} \\mathcal capitals", lm_name(&name)))
                        .or_insert((bb.name.clone(), false));
                    return Some((bb.clone(), gid.0));
                }
            }
        }
        // `\varnothing` (`VARNOTHING_SENTINEL`): the box is cmsy10's
        // `\emptyset` slot, but the outline is msbm10's design (New Computer
        // Modern Math's secondary face reproduces it, like `\mathbb`), drawn
        // at the real U+2205 the sentinel stands for.
        if name.starts_with("cmsy") && ch == crate::mathfont::VARNOTHING_SENTINEL {
            if let Some(bb) = self.otf.bb_face() {
                if let Some(gid) = bb.face().glyph_id('\u{2205}') {
                    self.resources
                        .borrow_mut()
                        .entry(format!("{} \\varnothing", lm_name(&name)))
                        .or_insert((bb.name.clone(), false));
                    return Some((bb.clone(), gid.0));
                }
            }
        }
        // amssymb/amsfonts symbols: the table's text from New Computer Modern
        // Math (whose symbol designs track msam/msbm) when it carries it,
        // else Latin Modern Math; an empty text (`\dabar@`) paints nothing
        // (gid 0). The extra-wide accents take the Latin Modern Math
        // horizontal variant nearest the TFM width.
        if font == AMS_MSAM_FONT || font == AMS_MSBM_FONT {
            let ams = crate::mathfont::ams_of(ch)?;
            let tfm_name = if font == AMS_MSAM_FONT { "msam10" } else { "msbm10" };
            let Some(first) = ams.text.chars().next() else {
                return Some((self.otf.face().clone(), 0));
            };
            if matches!(first, '\u{0302}' | '\u{0303}') {
                let tfm = flashtex_math_layout::ams::tfm(crate::mathfont::ams_font(ams.font), 10.0);
                let wanted = tfm.char(code).map(|c| mtfm::scale(c.width, tfm.design_size)).unwrap_or(0.0);
                let gid = self.otf.hvariant_nearest(first, tfm.design_size, wanted)?;
                self.resources.borrow_mut().entry(tfm_name.to_string()).or_insert((self.otf.face().name.clone(), false));
                return Some((self.otf.face().clone(), gid));
            }
            if let Some(bb) = self.otf.bb_face() {
                if let Some(gid) = bb.face().glyph_id(first) {
                    self.resources.borrow_mut().entry(tfm_name.to_string()).or_insert((bb.name.clone(), false));
                    return Some((bb.clone(), gid.0));
                }
            }
            let face = self.otf.face();
            let gid = face.face().glyph_id(first).map(|g| g.0);
            if gid.is_none() {
                self.unmapped.borrow_mut().push((tfm_name.to_string(), code, first));
            }
            self.resources.borrow_mut().entry(tfm_name.to_string()).or_insert((face.name.clone(), false));
            return Some((face.clone(), gid?));
        }
        let gid = self.otf_gid(font, code, ch)?;
        self.resources
            .borrow_mut()
            .entry(lm_name(&name))
            .or_insert((self.otf.face().name.clone(), false));
        Some((self.otf.face().clone(), gid))
    }

    /// A symbol outside the CM tables (compiler pin `87df3e4a` lists
    /// `\cong`, `\propto`, `\aleph`, `\Re`, `\langle`, `\Longrightarrow`,
    /// ... that math-layout's `cm` slot table does not carry): Latin Modern
    /// Math's own glyph and OpenType box, tagged [`OTF_FALLBACK_FONT`] so the
    /// painter draws that glyph id directly. Its width is the OpenType
    /// advance, not the cmsy/msbm TFM width pdfLaTeX would use. Double-struck
    /// letters come from the secondary face (New Computer Modern Math, whose
    /// design and advances track msbm) when it is loaded, tagged
    /// [`OTF_FALLBACK_BB_FONT`].
    fn otf_fallback_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        let mut g = self.otf.glyph(ch, size)?;
        g.font_id = if g.font_id == crate::mathfont::BB_FONT { OTF_FALLBACK_BB_FONT } else { OTF_FALLBACK_FONT };
        Some(g)
    }

    /// Loads the text fonts of the math alphabets `used` (`\mathbf`,
    /// `\mathsf`, `\mathit`, `\mathtt`: fontmath.ltx OT1 cmr/bx/n, cmss/m/n,
    /// cmr/m/it, cmtt/m/n) at the three math sizes, with Latin Modern's
    /// metrics of those designs. A one-character argument is a math
    /// character (TeX §1186 unpacks a group holding one Ord): its box comes
    /// from here, while longer runs go through the math text sink.
    pub fn with_alphabets(mut self, fonts: &FontSet, used: &[crate::mathalpha::MathAlphabet]) -> TexMathMetrics {
        for &alphabet in used {
            let Some(key) = alphabet.text_key() else { continue };
            for (i, &at) in self.cm.sizes.iter().enumerate() {
                if self.alphabets.iter().any(|(a, j, ..)| *a == alphabet && *j == i) {
                    continue;
                }
                let r = fonts.resolve(crate::fonts::Family::LatinModern, Role::Font(key), at);
                if r.substituted.is_some() {
                    continue;
                }
                if let Some(tfm) = r.face.tfm.clone() {
                    self.alphabets.push((alphabet, i, r.face, tfm));
                }
            }
        }
        self
    }

    /// Font id of a math-alphabet character at size index `i`.
    fn alphabet_font(alphabet: crate::mathalpha::MathAlphabet, i: usize) -> MathFontId {
        MathFontId(0x110 + 3 * alphabet.index() as u32 + i as u32)
    }

    /// A one-character math alphabet box from its text font's TFM.
    fn alphabet_glyph(&self, alphabet: crate::mathalpha::MathAlphabet, letter: char, ch: char, size: SizeClass) -> Option<Glyph> {
        let i = Self::size_index(size);
        let (_, _, _, tfm) = self.alphabets.iter().find(|(a, j, ..)| *a == alphabet && *j == i)?;
        let m = tfm.metrics(letter as u8)?;
        let at = self.cm.sizes[i];
        Some(Glyph {
            font_id: Self::alphabet_font(alphabet, i),
            gid: letter as u16,
            ch,
            size: at,
            width: Tfm::pt(m.width, at),
            height: Tfm::pt(m.height, at),
            depth: Tfm::pt(m.depth, at),
            italic: Tfm::pt(m.italic, at),
            skew: 0.0,
        })
    }

    /// A `\mathfrak` character's box from the `eufm` TFM of the size class
    /// (letters at their ASCII slots, U encoding); `None` when that TFM is
    /// not installed (Latin Modern Math's own box is used then).
    fn fraktur_glyph(&self, letter: char, ch: char, size: SizeClass) -> Option<Glyph> {
        let i = Self::size_index(size);
        let m = self.fraktur[i].as_ref()?.metrics(letter as u8)?;
        let at = self.cm.sizes[i];
        Some(Glyph {
            font_id: FRAKTUR_FONTS[i],
            gid: letter as u16,
            ch,
            size: at,
            width: Tfm::pt(m.width, at),
            height: Tfm::pt(m.height, at),
            depth: Tfm::pt(m.depth, at),
            italic: Tfm::pt(m.italic, at),
            skew: 0.0,
        })
    }

    /// The first roman-TFM failure, if any.
    pub fn roman_status(&self) -> Option<&TfmStatus> {
        self.roman_status.as_ref()
    }

    pub fn sizes(&self) -> MathSizes {
        self.sizes
    }

    /// The roman-family (`\fam0`, OT1) TFM at text size and the size it is
    /// set at: the font of digits, `(`, `)`, `+`, `=` in a text-style formula.
    pub fn roman_text_tfm(&self) -> Option<(Rc<Tfm>, f64)> {
        self.roman[0].clone().map(|t| (t, self.cm.sizes[0]))
    }

    /// Whether the `rm-lmr` TFMs were found (the CM families are embedded).
    pub fn roman_available(&self) -> bool {
        self.roman.iter().all(Option::is_some)
    }

    pub fn face(&self) -> &Rc<LoadedFace> {
        self.otf.face()
    }

    pub fn otf_fonts(&self) -> &Rc<MathFonts> {
        &self.otf
    }

    /// The TFM box `(height, depth)` in pt of a placed extension-family
    /// (cmex) glyph, `None` for every other font. cmex outlines hang from
    /// the origin (`\big(`: 0.04 em above, 1.16 em below; `\sum`: nothing
    /// above) and their TFM box is that ink, while the Latin Modern Math
    /// variants drawn for them sit on the axis relative to their own
    /// origin, so the painter re-centres the drawn ink on this box.
    pub fn extension_box(&self, font: MathFontId, code: u8, size: f64) -> Option<(f64, f64)> {
        if !self.cm.font_name(font).starts_with("cmex") {
            return None;
        }
        let c = cm_tfm::CMEX10.char(code)?;
        Some((mtfm::scale(c.height, size), mtfm::scale(c.depth, size)))
    }

    /// Glyphs the layout placed that have no OpenType counterpart
    /// (`(tfm font, code, char)`), drained for diagnostics.
    pub fn take_unmapped(&self) -> Vec<(String, u8, char)> {
        std::mem::take(&mut *self.unmapped.borrow_mut())
    }

    fn size_index(size: SizeClass) -> usize {
        match size {
            SizeClass::Text => 0,
            SizeClass::Script => 1,
            SizeClass::ScriptScript => 2,
        }
    }

    /// The roman-family glyph from `rm-lmr` when available, else `cmr`.
    fn roman_glyph(&self, code: u8, ch: char, size: SizeClass) -> Option<Glyph> {
        let font_id = self.cm.text_glyph('0', size)?.font_id;
        let Some(tfm) = &self.roman[Self::size_index(size)] else {
            return self.cm.glyph(ch, size);
        };
        let m = tfm.metrics(code)?;
        let at = self.cm.sizes[Self::size_index(size)];
        Some(Glyph {
            font_id,
            gid: u16::from(code),
            ch,
            size: at,
            width: Tfm::pt(m.width, at),
            height: Tfm::pt(m.height, at),
            depth: Tfm::pt(m.depth, at),
            italic: Tfm::pt(m.italic, at),
            skew: 0.0,
        })
    }

    /// A symbol-family glyph the math-layout table does not list, from the
    /// `cmsy` TFM of the size class (metrics) and the symbol's own `char`.
    fn symbol_family_glyph(&self, code: u8, ch: char, size: SizeClass) -> Option<Glyph> {
        let font_id = self.cm.glyph('\u{221E}', size)?.font_id;
        let i = Self::size_index(size);
        let font = self.cm.families[2][i];
        let c = font.char(code)?;
        let at = self.cm.sizes[i];
        // `\varnothing` (`crate::mathfont::VARNOTHING_SENTINEL`): the box is
        // cmsy10's `\emptyset` (0x3B), but `MathAtom.width_em` (compiler pin
        // `c583d6d4`, now `pub`) forces the advance to msbm10's char "3F,
        // 0.777781em, read at `typeset::symbol_atoms` rather than re-scanning
        // the source for the control word.
        let width = if ch == crate::mathfont::VARNOTHING_SENTINEL { VARNOTHING_MSBM_EM * at } else { mtfm::scale(c.width, at) };
        Some(Glyph {
            font_id,
            gid: u16::from(code),
            ch,
            size: at,
            width,
            height: mtfm::scale(c.height, at),
            depth: mtfm::scale(c.depth, at),
            italic: mtfm::scale(c.italic, at),
            skew: 0.0,
        })
    }

    /// The Latin Modern Math glyph id for a placed TFM glyph.
    pub fn otf_gid(&self, font: MathFontId, code: u8, ch: char) -> Option<u16> {
        let name = self.cm.font_name(font);
        let face = self.otf.face();
        let base = |c: char| {
            // The painted glyph follows the TFM slot, not the Unicode letter
            // the compiler spelled: cmmi 0x0F/0x1E are TeX's `\epsilon`
            // (lunate) and `\phi` (straight), 0x22/0x27 the `\var` forms.
            let c = if name.starts_with("cmmi") {
                match code {
                    0x0F => '\u{1D716}',
                    0x22 => '\u{1D700}',
                    0x1E => '\u{1D719}',
                    0x27 => '\u{1D711}',
                    _ => c,
                }
            } else if name.starts_with("cmsy") && code == 0x00 {
                // cmsy slot 0 is the minus sign: the compiler spells it as
                // the ASCII hyphen, whose Latin Modern Math glyph is the
                // short text hyphen, not U+2212.
                '\u{2212}'
            } else {
                c
            };
            face.face().glyph_id(MathFonts::math_char(c)).or_else(|| face.face().glyph_id(c)).map(|g| g.0)
        };
        // `\widehat`/`\widetilde` (cmex "62-"64, "65-"67): the Latin Modern
        // Math horizontal variant nearest the TFM width. `\overbrace`/
        // `\underbrace` pieces (math-layout tags them U+23DE/U+23DF): Latin
        // Modern Math assembles those braces from a left end, a middle and a
        // right end, so cmex's two middle half-pieces map to one middle part
        // (drawn at the first, the second paints nothing, gid 0) and the
        // painter aligns the parts on the pieces (`typeset::math_items`).
        if name.starts_with("cmex") && matches!(ch, '\u{0302}' | '\u{0303}') {
            let at = cm_tfm::CMEX10.design_size;
            let wanted = cm_tfm::CMEX10.char(code).map(|c| mtfm::scale(c.width, at)).unwrap_or(0.0);
            let result = self.otf.hvariant_nearest(ch, at, wanted);
            if result.is_none() {
                self.unmapped.borrow_mut().push((name, code, ch));
            }
            return result;
        }
        if name.starts_with("cmex") && matches!(ch, '\u{23DE}' | '\u{23DF}') {
            let parts = self.otf.hassembly_parts(ch);
            let index = match (ch, code) {
                ('\u{23DE}', 0x7A) | ('\u{23DF}', 0x7C) => Some(0),
                ('\u{23DE}', 0x7D) | ('\u{23DF}', 0x7B) => Some(2),
                ('\u{23DE}', 0x7B) | ('\u{23DF}', 0x7D) => Some(4),
                _ => None,
            };
            return match index {
                None => Some(0),
                Some(i) => {
                    let gid = parts.get(i).copied();
                    if gid.is_none() {
                        self.unmapped.borrow_mut().push((name, code, ch));
                    }
                    gid
                }
            };
        }
        let result = if name.starts_with("cmex") {
            // Size chain in lmex: steps from the character's first cmex code
            // to `code` select the same-index vertical variant in MATH.
            let start = cm::delimiter_slot(ch)
                .map(|(_, large)| large)
                .or_else(|| cm::symbol_slot(ch).filter(|(f, _)| *f == Family::Extension).map(|(_, c)| c))
                .or(if ch == '\u{221A}' { Some(0x70) } else { None });
            match (start, base(ch)) {
                (Some(start), Some(base_gid)) => {
                    let font = &cm_tfm::CMEX10;
                    let mut cur = font.char(start);
                    let mut k = 0usize;
                    let mut found = None;
                    while let Some(c) = cur {
                        if c.code == code {
                            found = Some((k, c));
                            break;
                        }
                        cur = font.next_larger(c);
                        k += 1;
                        if k > 8 {
                            break;
                        }
                    }
                    match found {
                        // The text-size glyph of a cmex-based symbol (`\sum`)
                        // is the base glyph; delimiters/radicals start their
                        // chain one step above the cmr/cmsy base glyph.
                        Some((0, _)) if cm::symbol_slot(ch).is_some_and(|(f, _)| f == Family::Extension) => Some(base_gid),
                        // The variant whose ink box is nearest the TFM box of
                        // the placed cmex glyph (both at the cmex design size:
                        // only the ratio matters). Latin Modern Math lists
                        // more delimiter sizes than cmex's `\big`…`\Bigg`
                        // chain, so the chain index alone selects a glyph
                        // TeX would not (`\Big(` = cmex 0x10, 18 pt, is the
                        // 4th larger variant, not the 2nd).
                        Some((k, c)) => {
                            let at = font.design_size;
                            let wanted = mtfm::scale(c.height, at) + mtfm::scale(c.depth, at);
                            self.otf.variant_nearest(ch, at, wanted).or_else(|| {
                                let idx = if cm::symbol_slot(ch).is_some_and(|(f, _)| f == Family::Extension) { k } else { k + 1 };
                                self.otf.variant_gid(base_gid, idx)
                            })
                        }
                        None => None,
                    }
                }
                _ => None,
            }
        } else {
            base(ch)
        };
        if result.is_none() {
            self.unmapped.borrow_mut().push((name, code, ch));
        }
        result
    }
}

impl MathFontMetrics for TexMathMetrics {
    fn params(&self, size: SizeClass) -> MathParams {
        self.cm.params(size)
    }

    fn font_name(&self, font: MathFontId) -> String {
        if font == OTF_FALLBACK_FONT {
            return self.otf.face().name.clone();
        }
        if font == OTF_FALLBACK_BB_FONT {
            return self.otf.font_name(crate::mathfont::BB_FONT);
        }
        if font == AMS_MSAM_FONT {
            return "msam".to_string();
        }
        if font == AMS_MSBM_FONT {
            return "msbm".to_string();
        }
        if let Some(i) = FRAKTUR_FONTS.iter().position(|f| *f == font) {
            return crate::mathalpha::fraktur_tfm(self.cm.sizes[i]).to_string();
        }
        if let Some((_, _, face, _)) = self.alphabets.iter().find(|(a, i, ..)| Self::alphabet_font(*a, *i) == font) {
            return face.name.clone();
        }
        self.cm.font_name(font)
    }

    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        // amssymb/amsfonts symbols: the msam/msbm box `umsa.fd`/`umsb.fd`
        // select at this math size (`crate::mathfont::ams_sentinel`).
        if let Some(ams) = crate::mathfont::ams_of(ch) {
            let font = crate::mathfont::ams_font(ams.font);
            let id = match ams.font {
                flashtex_compiler::amssymb::SymbolFont::Msam => AMS_MSAM_FONT,
                flashtex_compiler::amssymb::SymbolFont::Msbm => AMS_MSBM_FONT,
            };
            return flashtex_math_layout::ams::glyph(font, ams.slot, ch, self.cm.sizes[Self::size_index(size)], id);
        }
        if let Some((crate::mathalpha::MathAlphabet::Fraktur, letter)) = crate::mathalpha::classify(ch) {
            if let Some(g) = self.fraktur_glyph(letter, ch, size) {
                return Some(g);
            }
        }
        if let Some((alphabet, letter)) = crate::mathalpha::classify(ch).filter(|(a, _)| a.text_key().is_some()) {
            if let Some(g) = self.alphabet_glyph(alphabet, letter, ch, size) {
                return Some(g);
            }
        }
        match cm::symbol_slot(ch) {
            Some((Family::Roman, code)) => self.roman_glyph(code, ch, size),
            Some(_) => self.cm.glyph(ch, size),
            None => match extra_symbol_slot(ch) {
                Some(code) => self.symbol_family_glyph(code, ch, size),
                None => self.cm.glyph(ch, size).or_else(|| self.otf_fallback_glyph(ch, size)),
            },
        }
    }

    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.cm.large_operator(ch, size)
    }

    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        let mut v = self.cm.delimiter_sizes(ch, size);
        // The smallest delimiter is the roman/symbol text glyph.
        if let (Some(first), Some(((Family::Roman, code), _))) = (v.first_mut(), cm::delimiter_slot(ch)) {
            if let Some(g) = self.roman_glyph(code, ch, size) {
                *first = g;
            }
        }
        v
    }

    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.cm.radical_sizes(size)
    }

    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        // amsfonts' extra-wide `\widehat`/`\widetilde` (msbm "5B/"5D): the
        // slot and its msbm TFM successors ("5C/"5E), each a table piece.
        if let Some(ams) = crate::mathfont::ams_of(ch) {
            let font = crate::mathfont::ams_font(ams.font);
            let tfm = flashtex_math_layout::ams::tfm(font, self.cm.sizes[Self::size_index(size)]);
            let mut out = Vec::new();
            let mut cur = tfm.char(ams.slot);
            while let Some(c) = cur {
                let Some(piece) = flashtex_compiler::amssymb::by_slot(ams.font, c.code) else { break };
                out.extend(self.glyph(crate::mathfont::ams_sentinel(piece), size));
                cur = tfm.next_larger(c);
                if out.len() > 4 {
                    break;
                }
            }
            return out;
        }
        self.cm.accent_sizes(ch, size)
    }

    fn extension_glyph(&self, code: u8, ch: char) -> Option<Glyph> {
        self.cm.extension_glyph(code, ch)
    }

    fn delimiter_extensible(&self, ch: char, size: SizeClass) -> Option<Extensible> {
        self.cm.delimiter_extensible(ch, size)
    }

    fn radical_extensible(&self, size: SizeClass) -> Option<Extensible> {
        self.cm.radical_extensible(size)
    }

    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        if ch.is_ascii() {
            self.roman_glyph(ch as u8, ch, size)
        } else {
            self.cm.text_glyph(ch, size)
        }
    }
}

/// `\\not` (`fontmath.ltx`: `\\mathrel{\\mathchar"3236}`, the zero-width
/// negation slash `\\neq`/`\\notin` overprint) and `\\perp` (`symbols "3F`):
/// cmsy slots math-layout's table does not list. The combining long solidus
/// overlay stands for `\\not` because it is what Latin Modern Math draws at
/// U+0338 and no compiler symbol uses it.
pub const NOT_SLASH: char = '\u{0338}';

/// `\varnothing`'s advance in ems (msbm10.tfm char "3F, `CHARWD R 0.777781`):
/// the same physical constant the compiler's `math::VARNOTHING_MSBM_EM`
/// carries (crate-private there), read from the now-`pub` `MathAtom.width_em`
/// at the atom in `typeset::symbol_atoms` and applied here to the sentinel's
/// forced advance.
const VARNOTHING_MSBM_EM: f64 = 0.777781;

fn extra_symbol_slot(ch: char) -> Option<u8> {
    match ch {
        NOT_SLASH => Some(0x36),
        '\u{22A5}' => Some(0x3F),
        // `\varnothing`'s box is still cmsy10's `\emptyset` slot 0x3B (the
        // compiler forces only the advance, `MathAtom.width_em`; see
        // `symbol_family_glyph`); the outline is painted from New Computer
        // Modern Math via `TexMathMetrics::otf_glyph`'s cmsy branch.
        crate::mathfont::VARNOTHING_SENTINEL => Some(0x3B),
        _ => script_capital_slot(ch),
    }
}

/// The cmsy slot of a `\mathcal` capital. Compiler pin `dbf6ec78` emits the
/// Unicode script code point (`newcm_math::script(letter)`); `fontmath.ltx`
/// declares `\mathcal` as the `symbols` (cmsy) alphabet whose slots 0x41–0x5A
/// are the calligraphic capitals, so the box, advance and italic correction
/// are cmsy10's exactly as pdfLaTeX sets them. The outline is drawn by
/// [`TexMathMetrics::otf_glyph`] from the secondary face when it is loaded.
fn script_capital_slot(ch: char) -> Option<u8> {
    ('A'..='Z').find(|l| flashtex_compiler::newcm_math::script(*l) == Some(ch)).map(|l| l as u8)
}

/// The Latin Modern TFM that carries the same metrics as a CM table name
/// (`cmmi12` → `lmmi12`), for provenance messages.
fn lm_name(cm: &str) -> String {
    cm.replacen("cm", "lm", 1)
}
