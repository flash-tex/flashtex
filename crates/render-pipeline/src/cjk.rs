//! CJK.sty's characters (`\usepackage{CJKutf8}`, `\begin{CJK}{UTF8}{min}`):
//! the metrics pdflatex sets them with and the rules it puts between them.
//!
//! Inside the environment every UTF-8 character that inputenc does not
//! declare is CJK's (CJKutf8.sty 30-60: `\CJK@XX`/`\CJK@XXX` test `u8:..`
//! first). A two-byte character (U+0080-U+07FF) goes through `\CJK@char`
//! (UTF8.chr 35-48): the subfont glyph alone, no glue, no kern. A three- or
//! four-byte one goes through `\CJK@altchar`/`\CJK@altxchar` (UTF8.chr
//! 52-181) or, for the lead bytes E3 and EF, `\CJK@punctchar` (UTF8.chr
//! 185-267 with the plane-30/FE/FF tables of CJK.enc 291-305): `\CJKglue`
//! before it when the previous node is a CJK character (`\lastkern` = 1sp),
//! `\nobreak\CJKglue\nobreak` instead before a closing punctuation
//! character and after an opening one, the glyph, then `\kern-1sp\kern1sp`
//! (or `\kern-2sp\kern2sp` after an opening punctuation character), which
//! is what the next character reads. `\CJKglue` is `\hskip 0pt plus
//! .08\baselineskip` (CJK.sty 824). Measured with `\showoutput` on
//! `fixtures/real-world/unicode-accents` (pdflatex, TeX Live 2026):
//!
//! ```text
//! \C70/min/m/n/10.95/67 q        (東: udmj67 slot 0x71)
//! \kern -0.00002
//! \kern 0.00002
//! \glue 0.0 plus 1.08801        (0.08 * 13.6pt)
//! \C70/min/m/n/10.95/4e ...     (京)
//! ...
//! \penalty 10000
//! \glue 0.0 plus 1.08801
//! \penalty 10000
//! \C70/min/m/n/10.95/30 ^^B     (。: udmj30 slot 2, a postPunct character)
//! ```
//!
//! The subfont a character comes from is `<family><plane>` with the plane
//! the character's high byte (UTF8.chr 76-83): `udmj67` for U+6771. Every
//! subfont of a family shares one design size and, for the wadalab fonts,
//! one set of character dimensions, read with `tftopl` (oracle only, never
//! at run time) from `texmf-dist/fonts/tfm/{wadalab,arphic,uhc}` and copied
//! into [`FamilyMetrics`]. A plane with no subfont (`udmj12.tfm` does not
//! exist) is pdflatex's "Font C70/min/m/n/10.95/12=udmj12 at 10.95pt not
//! loadable: Metric (TFM) file not found": the character is set in
//! `\nullfont`, nothing on the page and no width, while the glue and kerns
//! around it are still appended.
//!
//! The outlines are another matter: the wadalab/arphic/uhc Type 1 designs
//! are TeX Live's, not the machine's, and this pipeline never reads them.
//! A character is painted from an installed CJK font found through the
//! discovery index ([`paint_families`]), at the TFM advance regardless of
//! the painted font's own, and the substitution is named once per family.

use flashtex_compiler::parser::CjkFamily;

/// One `C70` family's subfont metrics, in ems of the design size (TFM
/// fixwords / 2^20; the `CJK` size function loads every subfont at the
/// current size, so 1 em is the text size).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FamilyMetrics {
    /// The family's subfont name stem (`udmj`), for diagnostics.
    pub stem: &'static str,
    /// The design's name, for the substitution note.
    pub design: &'static str,
    /// `CHARWD` of a full-width character.
    pub width: f64,
    /// `CHARHT`: the same for every wadalab glyph; the arphic and uhc
    /// subfonts carry per-glyph heights and depths, of which the most
    /// common value is tabulated (a line's height is the tallest box on
    /// it, which only moves a baseline when `\baselineskip` cannot hold the
    /// line, never at the class sizes).
    pub height: f64,
    /// `CHARDP`, likewise.
    pub depth: f64,
    /// The planes (high bytes of the code point) that have a subfont, as
    /// `ls texmf-dist/fonts/tfm/<vendor>/<stem>/` lists them.
    pub planes: &'static [u8],
}

/// `wadalab/udmj` (93 subfonts): every glyph `CHARWD 1.0 CHARHT 0.84
/// CHARDP 0.16`, except the 63 half-width katakana U+FF61-U+FF9F of
/// `udmjff` at `CHARWD 0.5`. `udgj` (goth) and `umrj` (maru) have the same
/// planes and dimensions.
const WADALAB_PLANES: &[u8] = &[
    0x00, 0x03, 0x04, 0x20, 0x21, 0x22, 0x23, 0x25, 0x26, 0x30, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60, 0x61, 0x62,
    0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82,
    0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xff,
];

/// `arphic/gbsnu` (98 subfonts): `CHARWD 1.0` for every glyph outside the
/// ASCII range of `gbsnu00` (0.5); the heights are per glyph in
/// 0.75-0.85 em (0.84309 most often, 137 of 7574) and the depths in
/// 0.08-0.12 em (0.085471 most often; 0 for the 529 symbol glyphs). `gkaiu`
/// (gkai) has the same planes.
const GBSNU_PLANES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x30, 0x31, 0x32, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d,
    0x5e, 0x5f, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 0x7c,
    0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b,
    0x9c, 0x9e, 0x9f, 0xfe, 0xff,
];

/// `arphic/bsmiu` (102 subfonts, one of them the vertical `bsmiuv`):
/// `CHARWD 1.0` outside `bsmiu00`'s ASCII range; heights per glyph
/// (0.843499 most often, 256 of 14058), depths per glyph (0.085414 most
/// often). `bkaiu` (bkai) has the same planes.
const BSMIU_PLANES: &[u8] = &[
    0x00, 0x02, 0x03, 0x20, 0x21, 0x22, 0x25, 0x26, 0x30, 0x31, 0x32, 0x33, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60,
    0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f,
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e,
    0x9f, 0xee, 0xf6, 0xf7, 0xf8, 0xfa, 0xfe, 0xff,
];

/// `uhc/uwmj` (145 subfonts of the medium upright shape): `CHARWD 1.0` for
/// the Hangul syllables (planes AC-D7) and the ideographs; heights per
/// glyph (0.776 most often, 714 of 8222), depths per glyph (0.15 most
/// often). The Latin, Greek, Cyrillic, symbol and kana planes (00-04,
/// 20-23, 30-32, FF) are proportional in this design (0.242-1.231 em) and
/// are set at 1 em here, an approximation the substitution note names.
const UWMJ_PLANES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x30, 0x31, 0x32, 0x33, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c,
    0x5d, 0x5e, 0x5f, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b,
    0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a,
    0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xac, 0xad, 0xae, 0xaf, 0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5,
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xf9, 0xfa, 0xff,
];

const WADALAB: FamilyMetrics = FamilyMetrics { stem: "udmj", design: "Wadalab Mincho", width: 1.0, height: 0.84, depth: 0.16, planes: WADALAB_PLANES };

/// `zhmetrics/cyberb` (TeX Live ships the 256 TFMs of Bitstream Cyberbit,
/// one per plane, but no glyphs): every glyph `CHARWD 1.0 CHARHT 0.8
/// CHARDP 0.1`. This is what pdflatex lays an unknown family out with
/// (`\wrong@fontshape` substitutes `C70/song`, CJK.enc 798) before it
/// fails at shipout with `!pdfTeX error: Font cyberb4e at 657 not found`
/// and writes no PDF.
const CYBERB_PLANES: [u8; 256] = {
    let mut planes = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        planes[i] = i as u8;
        i += 1;
    }
    planes
};

/// The metrics of a family. [`CjkFamily::Unknown`] takes `cyberb`'s (see
/// [`CYBERB_PLANES`]): the layout pdflatex computes for it, though pdflatex
/// then ships no PDF at all.
pub fn metrics(family: CjkFamily) -> FamilyMetrics {
    match family {
        CjkFamily::Min => WADALAB,
        CjkFamily::Goth => FamilyMetrics { stem: "udgj", design: "Wadalab Gothic", ..WADALAB },
        CjkFamily::Maru => FamilyMetrics { stem: "umrj", design: "Wadalab Maru Gothic", ..WADALAB },
        CjkFamily::Gbsn => FamilyMetrics { stem: "gbsnu", design: "AR PL SungtiL GB", width: 1.0, height: 0.84309, depth: 0.085471, planes: GBSNU_PLANES },
        CjkFamily::Gkai => FamilyMetrics { stem: "gkaiu", design: "AR PL KaitiM GB", width: 1.0, height: 0.84309, depth: 0.085471, planes: GBSNU_PLANES },
        CjkFamily::Bsmi => FamilyMetrics { stem: "bsmiu", design: "AR PL Mingti2L Big5", width: 1.0, height: 0.843499, depth: 0.085414, planes: BSMIU_PLANES },
        CjkFamily::Bkai => FamilyMetrics { stem: "bkaiu", design: "AR PL KaitiM Big5", width: 1.0, height: 0.843499, depth: 0.085414, planes: BSMIU_PLANES },
        CjkFamily::Mj => FamilyMetrics { stem: "uwmj", design: "UHC Myoungjo", width: 1.0, height: 0.776, depth: 0.15, planes: UWMJ_PLANES },
        CjkFamily::Unknown => FamilyMetrics { stem: "cyberb", design: "Bitstream Cyberbit", width: 1.0, height: 0.8, depth: 0.1, planes: &CYBERB_PLANES },
    }
}

/// The width of `ch` in `family`, in ems: 0 for a character whose plane has
/// no subfont (set in `\nullfont`).
pub fn width_em(family: CjkFamily, ch: char) -> f64 {
    let m = metrics(family);
    let code = ch as u32;
    let plane = code >> 8;
    if plane > 0xff || !m.planes.contains(&(plane as u8)) {
        return 0.0;
    }
    // `udmjff`'s half-width katakana (and `gbsnu00`/`bsmiu00`'s ASCII range,
    // which the UTF-8 encoding never selects: ASCII is not active).
    if matches!(family, CjkFamily::Min | CjkFamily::Goth | CjkFamily::Maru) && (0xff61..=0xff9f).contains(&code) {
        return 0.5;
    }
    m.width
}

/// `\CJKbold` (CJK.sty 101-107): the `bx` series of every family
/// (`c70min.fd` 25-26, `c70gbsn.fd` 19, ...) sets each glyph three times,
/// the copies in `\hbox to \CJKboldshift{\hss ...}` boxes of
/// `\CJKboldshift` = 0.015 em (CJK.sty 98), so a bold character advances
/// 0.03 em more than its `CHARWD`.
pub const BOLD_EXTRA_EM: f64 = 0.03;

/// How CJK.sty reads one character of a `CJK` environment's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// ASCII, or a character inputenc declares (`u8:` defined): the text
    /// font's, set by the ordinary path.
    Latin,
    /// U+0080-U+07FF undeclared: `\CJK@char`, the subfont glyph with no
    /// glue or kern around it.
    Symbol,
    /// U+0800 and above: a CJK character with `\CJKglue` between it and a
    /// CJK neighbour, and the kern pair after it.
    Cjk(Punct),
}

/// CJK.enc's punctuation classes for the UTF-8 planes 30, FE and FF
/// (`\CJK@prePunct`/`\CJK@postPunct`, CJK.enc 293-305): a `Pre` character
/// (an opening bracket) allows no break after it, a `Post` one (a closing
/// bracket, a full stop) none before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punct {
    None,
    Pre,
    Post,
}

/// Whether inputenc declares `ch` (CJKutf8.sty's `u8:` test): the `utf8.def`
/// and `*.dfu` tables of `flashtex-tex-text-encoding`, plus the project's
/// own declarations.
pub fn classify(ch: char, declared: impl Fn(char) -> bool) -> Class {
    if ch.is_ascii() || declared(ch) {
        return Class::Latin;
    }
    let code = ch as u32;
    if code < 0x800 {
        return Class::Symbol;
    }
    Class::Cjk(punct(ch))
}

/// CJK.enc 293-305, the UTF-8 punctuation tables.
pub fn punct(ch: char) -> Punct {
    let code = ch as u32;
    // `\CJK@uniPunct` (CJK.enc 291): only these three planes are tested.
    let (plane, low) = (code >> 8, (code & 0xff) as u8);
    let pre: &[u8] = match plane {
        0x30 => &[0x08, 0x0a, 0x0c, 0x0e, 0x10, 0x12, 0x14, 0x16, 0x18, 0x1a, 0x1d, 0x1f, 0x36],
        0xfe => &[0x59, 0x5b, 0x5d, 0x5f, 0x60, 0x69, 0x6b],
        0xff => &[0x03, 0x04, 0x08, 0x20, 0x3b, 0x5b, 0xe0, 0xe1, 0xe5, 0xe6],
        _ => return Punct::None,
    };
    let post: &[u8] = match plane {
        0x30 => &[
            0x01, 0x02, 0x05, 0x06, 0x09, 0x0b, 0x0d, 0x0f, 0x11, 0x15, 0x17, 0x19, 0x1b, 0x1e, 0x41, 0x43, 0x45, 0x47, 0x49, 0x63, 0x83, 0x85, 0x87, 0x8e, 0x9b, 0x9c, 0x9d, 0x9e, 0xa1,
            0xa3, 0xa5, 0xa7, 0xa9, 0xc3, 0xe3, 0xe5, 0xe7, 0xee, 0xf5, 0xf6, 0xfb, 0xfc, 0xfd, 0xfe,
        ],
        0xfe => &[0x50, 0x51, 0x52, 0x54, 0x55, 0x56, 0x57, 0x5a, 0x5c, 0x5e, 0x6a],
        0xff => &[0x01, 0x05, 0x09, 0x0c, 0x0e, 0x1a, 0x1b, 0x1f, 0x3d, 0x5d, 0x61, 0x63, 0x64, 0x65, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x9e, 0x9f],
        _ => &[],
    };
    if pre.contains(&low) {
        Punct::Pre
    } else if post.contains(&low) {
        Punct::Post
    } else {
        Punct::None
    }
}

/// Installed families that paint a `C70` family's characters, tried in
/// order through the discovery index (`FontIndex::find_match`, weight 400
/// upright): the platform's own CJK fonts first (macOS: Hiragino, Songti,
/// Apple Myungjo; Windows: Yu Mincho, SimSun, PMingLiU, Batang), then the
/// Noto CJK families a Linux distribution installs. None of them is the
/// wadalab/arphic/uhc design; the note says so.
pub fn paint_families(family: CjkFamily) -> &'static [&'static str] {
    match family {
        CjkFamily::Min => &["Hiragino Mincho ProN", "Hiragino Mincho Pro", "Yu Mincho", "YuMincho", "MS Mincho", "IPAMincho", "IPAexMincho", "Noto Serif CJK JP", "Noto Serif JP", "Source Han Serif JP", "Noto Sans CJK JP", "Noto Sans JP", "Hiragino Sans"],
        CjkFamily::Goth | CjkFamily::Maru => &["Hiragino Sans", "Hiragino Kaku Gothic ProN", "Hiragino Kaku Gothic Pro", "Yu Gothic", "YuGothic", "MS Gothic", "IPAGothic", "IPAexGothic", "Noto Sans CJK JP", "Noto Sans JP", "Source Han Sans JP"],
        CjkFamily::Gbsn => &["Songti SC", "STSong", "SimSun", "NSimSun", "Noto Serif CJK SC", "Noto Serif SC", "Source Han Serif SC", "PingFang SC", "Heiti SC", "Noto Sans CJK SC", "Noto Sans SC", "WenQuanYi Zen Hei"],
        CjkFamily::Gkai => &["Kaiti SC", "STKaiti", "KaiTi", "Songti SC", "Noto Serif CJK SC", "Noto Serif SC", "PingFang SC", "Noto Sans CJK SC"],
        CjkFamily::Bsmi => &["Songti TC", "PMingLiU", "MingLiU", "Noto Serif CJK TC", "Noto Serif TC", "Source Han Serif TC", "PingFang TC", "Heiti TC", "Noto Sans CJK TC", "Noto Sans TC"],
        CjkFamily::Bkai => &["Kaiti TC", "BiauKai", "DFKai-SB", "Songti TC", "Noto Serif CJK TC", "Noto Serif TC", "PingFang TC", "Noto Sans CJK TC"],
        CjkFamily::Mj => &["AppleMyungjo", "Apple SD Gothic Neo", "Batang", "BatangChe", "Noto Serif CJK KR", "Noto Serif KR", "Source Han Serif KR", "Malgun Gothic", "Noto Sans CJK KR", "Noto Sans KR", "NanumMyeongjo", "NanumGothic"],
        // Cyberbit is a pan-CJK font: any installed CJK design stands in.
        CjkFamily::Unknown => &["Hiragino Mincho ProN", "Songti SC", "Noto Serif CJK JP", "Noto Serif CJK SC", "Noto Sans CJK JP", "Noto Sans CJK SC", "Yu Mincho", "SimSun", "Hiragino Sans", "PingFang SC", "Apple SD Gothic Neo"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wadalab_is_one_em_by_0_84_and_0_16() {
        let m = metrics(CjkFamily::Min);
        assert_eq!((m.width, m.height, m.depth), (1.0, 0.84, 0.16));
        assert_eq!(width_em(CjkFamily::Min, '東'), 1.0);
        assert_eq!(width_em(CjkFamily::Min, '。'), 1.0);
        assert_eq!(width_em(CjkFamily::Min, 'ｱ'), 0.5, "half-width katakana of udmjff");
        assert_eq!(width_em(CjkFamily::Min, '\u{1200}'), 0.0, "no udmj12 subfont");
        assert_eq!(width_em(CjkFamily::Min, '\u{20000}'), 0.0, "no four-byte plane");
        assert_eq!(width_em(CjkFamily::Unknown, '東'), 1.0, "cyberb's 1 em");
        assert_eq!(width_em(CjkFamily::Unknown, '\u{1200}'), 1.0, "cyberb has every plane");
    }

    #[test]
    fn punctuation_classes_follow_cjk_enc() {
        assert_eq!(punct('。'), Punct::Post);
        assert_eq!(punct('、'), Punct::Post);
        assert_eq!(punct('」'), Punct::Post);
        assert_eq!(punct('「'), Punct::Pre);
        assert_eq!(punct('（'), Punct::Pre);
        assert_eq!(punct('）'), Punct::Post);
        assert_eq!(punct('東'), Punct::None);
        assert_eq!(punct('は'), Punct::None, "hiragana is in plane 30 but in neither list");
        assert_eq!(punct('，'), Punct::Post, "U+FF0C fullwidth comma");
    }

    #[test]
    fn classification() {
        let declared = |c: char| c == 'ü';
        assert_eq!(classify('a', declared), Class::Latin);
        assert_eq!(classify('ü', declared), Class::Latin);
        assert_eq!(classify('α', declared), Class::Symbol);
        assert_eq!(classify('東', declared), Class::Cjk(Punct::None));
        assert_eq!(classify('。', declared), Class::Cjk(Punct::Post));
    }
}
