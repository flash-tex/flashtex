//! `mapfile.c`, ported: the font map (`pdftex.map`, `\pdfmapfile`,
//! `\pdfmapline`), and `subfont.c`'s test for subfont map entries.
//!
//! Map entries live in an arena; pdftex.web's `pdf_font_map[f]` holds
//! `0` (not looked up yet, C's `NULL`), [`DUMMY`] (no entry, C's
//! `dummy_fm_entry()`) or an arena index plus one.

use super::cfmt;
use super::fonts::Fonts;
use super::output::set_cur_file_name;
use super::shared::Shared;
use crate::generated::Globals;
use crate::resolver::Format;
use std::collections::{BTreeMap, BTreeSet};

/// `pdf_font_map[f]` of a font without a map entry.
pub const DUMMY: i32 = -1;

pub const FD_FLAGS_NOT_SET_IN_MAPLINE: i32 = -1;
pub const FD_FLAGS_DEFAULT_EMBED: i32 = 4;
pub const FD_FLAGS_DEFAULT_NON_EMBED: i32 = 0x22;

pub const F_INCLUDED: u16 = 0x01;
pub const F_SUBSETTED: u16 = 0x02;
pub const F_STDT1FONT: u16 = 0x04;
pub const F_SUBFONT: u16 = 0x08;
pub const F_TYPE1: u16 = 0x10;
pub const F_TRUETYPE: u16 = 0x20;
pub const F_OTF: u16 = 0x40;
pub const F_PK: u16 = 0x80;

const LINK_TFM: u16 = 0x01;
const LINK_PS: u16 = 0x02;

const FM_BUF_SIZE: usize = 1024;
const NONTFM: &[u8] = b"<nontfm>";

/// `fm_entry` (ptexlib.h).
#[derive(Clone, Debug)]
pub struct FmEntry {
    pub tfm_name: Vec<u8>,
    pub ps_name: Option<Vec<u8>>,
    pub fd_flags: i32,
    pub slant: i32,
    pub extend: i32,
    pub encname: Option<Vec<u8>>,
    pub ff_name: Option<Vec<u8>>,
    pub typ: u16,
    pub pid: i16,
    pub eid: i16,
    pub links: u16,
}
crate::codec_struct!(FmEntry {
    tfm_name,
    ps_name,
    fd_flags,
    slant,
    extend,
    encname,
    ff_name,
    typ,
    pid,
    eid,
    links
});

impl FmEntry {
    /// `new_fm_entry`.
    fn new() -> FmEntry {
        FmEntry {
            tfm_name: Vec::new(),
            ps_name: None,
            fd_flags: FD_FLAGS_NOT_SET_IN_MAPLINE,
            slant: 0,
            extend: 0,
            encname: None,
            ff_name: None,
            typ: 0,
            pid: -1,
            eid: -1,
            links: 0,
        }
    }
    pub fn is_included(&self) -> bool {
        self.typ & F_INCLUDED != 0
    }
    pub fn is_subsetted(&self) -> bool {
        self.typ & F_SUBSETTED != 0
    }
    pub fn is_std_t1font(&self) -> bool {
        self.typ & F_STDT1FONT != 0
    }
    pub fn is_type1(&self) -> bool {
        self.typ & F_TYPE1 != 0
    }
    pub fn is_truetype(&self) -> bool {
        self.typ & F_TRUETYPE != 0
    }
    pub fn is_opentype(&self) -> bool {
        self.typ & F_OTF != 0
    }
    pub fn is_pk(&self) -> bool {
        self.typ & F_PK != 0
    }
    pub fn is_reencoded(&self) -> bool {
        self.encname.is_some()
    }
    pub fn is_fontfile(&self) -> bool {
        self.ff_name.is_some()
    }
    pub fn is_t1fontfile(&self) -> bool {
        self.is_fontfile() && self.is_type1()
    }
    pub fn is_builtin(&self) -> bool {
        !self.is_fontfile()
    }
    fn ps_key(&self) -> (Vec<u8>, i32, i32) {
        (
            self.ps_name.clone().unwrap_or_default(),
            self.slant,
            self.extend,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    DupIgnore,
    Replace,
    Delete,
}
crate::codec_enum!(Mode {
    DupIgnore,
    Replace,
    Delete
});

/// `mapitem`: the map file or map line still to be read.
#[derive(Clone)]
struct MapItem {
    mode: Mode,
    is_file: bool,
    line: Option<Vec<u8>>,
}
crate::codec_struct!(MapItem {
    mode,
    is_file,
    line
});

/// The parsed map (entries and trees) is [`Shared`]: it is read once per
/// run (and once per process, see [`MapCache`]) and then only read, while a
/// checkpoint after every page copies the state (`super::shared`). What a
/// run changes afterwards, which entries are in use, is kept apart.
#[derive(Default, Clone)]
pub struct State {
    /// The map entries; `None` once deleted.
    pub fms: Shared<Vec<Option<FmEntry>>>,
    /// The entries a font has used (`fm_entry.in_use` in C).
    in_use: BTreeSet<usize>,
    /// `tfm_tree != NULL`: `create_avl_trees` has run.
    trees: bool,
    /// `tfm_tree`: entries by TFM name.
    tfm_tree: Shared<BTreeMap<Vec<u8>, usize>>,
    /// `ps_tree`: Type 1 entries with an included font file, by PostScript
    /// name, slant and extend.
    ps_tree: Shared<BTreeMap<(Vec<u8>, i32, i32), usize>>,
    /// `ff_tree`: font file name to the path found, or `None`.
    ff_tree: BTreeMap<Vec<u8>, Option<String>>,
    mitem: Option<MapItem>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State {
    fms,
    in_use,
    trees,
    tfm_tree,
    ps_tree,
    ff_tree,
    mitem
});

/// What reading a map file into an empty map depends on: the file (by its
/// path and stat signature, the fast path of the read-set checks in
/// `crate::host`), how it was asked for, and `\pdfsuppresswarningdupmap`.
#[derive(Clone, PartialEq)]
struct MapKey {
    mode: Mode,
    line: Vec<u8>,
    path: String,
    stat: crate::system::StatSig,
    suppress_dup: bool,
}

/// The entries and trees a map file parsed into.
#[derive(Clone)]
struct MapParse {
    fms: Shared<Vec<Option<FmEntry>>>,
    tfm_tree: Shared<BTreeMap<Vec<u8>, usize>>,
    ps_tree: Shared<BTreeMap<(Vec<u8>, i32, i32), usize>>,
}

/// Map files parsed in this process (DESIGN.md §4.2: the font map is read
/// once per process, not once per run). pdfTeX reads `pdftex.map` at the
/// first shipout of every run, which cost a resident engine 45 ms of the
/// 47 ms a one-page document takes to re-run from S₀. A parse is kept only
/// if it printed nothing but the braces around the file name, so a hit
/// prints exactly what the parse would have.
struct MapCache;

thread_local! {
    static MAP_CACHE: std::cell::RefCell<Vec<(MapKey, MapParse)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

impl MapCache {
    fn get(k: &MapKey) -> Option<MapParse> {
        MAP_CACHE.with(|c| {
            c.borrow()
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
        })
    }

    fn put(k: MapKey, v: MapParse) {
        MAP_CACHE.with(|c| {
            let mut c = c.borrow_mut();
            c.retain(|(key, _)| !(key.path == k.path && key.line == k.line));
            c.push((k, v));
        })
    }
}

impl State {
    /// Note that entry `id` is used by a font (`fm->in_use = true`).
    pub fn set_in_use(&mut self, id: usize) {
        self.in_use.insert(id);
    }

    /// The entry `pdf_font_map[f]` points to, for a value from
    /// [`Globals::hasfmentry`]'s lookup.
    pub fn fm(&self, ptr: i32) -> &FmEntry {
        self.fms[(ptr - 1) as usize]
            .as_ref()
            .expect("live map entry")
    }
}

/// `check_std_t1font`: the number of one of the 14 standard fonts, or -1.
pub fn check_std_t1font(s: &[u8]) -> i32 {
    const NAMES: [&[u8]; 14] = [
        b"Courier",
        b"Courier-Bold",
        b"Courier-Oblique",
        b"Courier-BoldOblique",
        b"Helvetica",
        b"Helvetica-Bold",
        b"Helvetica-Oblique",
        b"Helvetica-BoldOblique",
        b"Symbol",
        b"Times-Roman",
        b"Times-Bold",
        b"Times-Italic",
        b"Times-BoldItalic",
        b"ZapfDingbats",
    ];
    const INDEX: [i32; 22] = [
        -1, -1, -1, -1, -1, -1, 8, 0, -1, 4, 10, 9, -1, -1, 5, 2, 12, 6, -1, 3, -1, 7,
    ];
    let n = s.len();
    if n > 21 {
        return -1;
    }
    let k = if n == 12 {
        match s[0] {
            b'C' => 1,
            b'T' => 11,
            b'Z' => 13,
            _ => return -1,
        }
    } else {
        INDEX[n]
    };
    if k > -1 && NAMES[k as usize] == s {
        return k;
    }
    -1
}

/// `is_cfg_comment` (ptexmac.h).
fn is_cfg_comment(c: u8) -> bool {
    c == 10 || c == b'*' || c == b'#' || c == b';' || c == b'%'
}

/// `strcasecmp(strend(s) - 4, suffix) == 0`, for `strlen(s) >= 4`.
fn ends_with_ci(s: &[u8], suffix: &[u8]) -> bool {
    s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// A NUL-terminated C string as a cursor: `at(i)` is `r[i]`, 0 past the end.
struct CStr<'a>(&'a [u8]);
impl CStr<'_> {
    fn at(&self, i: usize) -> u8 {
        self.0.get(i).copied().unwrap_or(0)
    }
}

impl Globals {
    fn fm_buf_overflow(&mut self) -> ! {
        self.pdftex_fail("buffer overflow at file mapfile.c, line 53")
    }

    /// `read_field`: bytes up to a blank, `<`, `"` or the end, then skip one
    /// blank.
    fn read_field(&mut self, line: &CStr, r: &mut usize) -> Vec<u8> {
        let mut buf = Vec::new();
        loop {
            let c = line.at(*r);
            if c == b' ' || c == b'<' || c == b'"' || c == 0 {
                break;
            }
            if buf.len() + 1 > FM_BUF_SIZE {
                self.fm_buf_overflow();
            }
            buf.push(c);
            *r += 1;
        }
        if line.at(*r) == b' ' {
            *r += 1;
        }
        buf
    }

    /// `fm_scan_line`: parse one map line and register (or delete) it.
    fn fm_scan_line(&mut self, st: &mut Fonts, raw: &[u8], mode: Mode) {
        let line = CStr(raw);
        let mut r = 0usize;
        if line.at(0) == 0 || is_cfg_comment(line.at(0)) {
            return;
        }
        let mut fm = FmEntry::new();
        let (mut u, mut v) = (0u8, 0u8);
        'scan: {
            let buf = self.read_field(&line, &mut r);
            if !buf.is_empty() {
                fm.tfm_name = buf;
            }
            if line.at(r) == 0 {
                break 'scan;
            }
            if !line.at(r).is_ascii_digit() {
                // 2nd field ps_name may not start with a digit
                let buf = self.read_field(&line, &mut r);
                if !buf.is_empty() {
                    fm.ps_name = Some(buf);
                }
                if line.at(r) == 0 {
                    break 'scan;
                }
            }
            if line.at(r).is_ascii_digit() {
                // font descriptor /Flags given?
                let mut s = r;
                while line.at(s).is_ascii_digit() {
                    s += 1;
                }
                let c = line.at(s);
                if c == b' ' || c == b'"' || c == b'<' || c == 0 {
                    // not e. g. 8r.enc: atoi
                    let digits = &raw[r..s];
                    fm.fd_flags = std::str::from_utf8(digits)
                        .ok()
                        .and_then(|d| d.parse::<i64>().ok())
                        .map(|d| d as i32)
                        .unwrap_or(0);
                    r = s;
                }
            }
            loop {
                // "specials", encoding, font file
                if line.at(r) == b' ' {
                    r += 1;
                }
                match line.at(r) {
                    0 => break 'scan,
                    b'"' => {
                        r += 1;
                        u = 0;
                        v = 0;
                        loop {
                            if line.at(r) == b' ' {
                                r += 1;
                            }
                            if let Some((mut d, j)) = cfmt::scan_float_n(&raw[r.min(raw.len())..]) {
                                // jump behind the number, blanks eaten too
                                let mut s = r + j;
                                if matches!(line.at(s - 1), b'E' | b'e') {
                                    s -= 1; // e. g. 0.5ExtendFont: %f = 0.5E
                                }
                                let rest = &raw[s.min(raw.len())..];
                                if rest.starts_with(b"SlantFont") {
                                    d = (d as f64 * 1000.0) as f32;
                                    fm.slant = round_half_away(d);
                                    r = s + b"SlantFont".len();
                                } else if rest.starts_with(b"ExtendFont") {
                                    d = (d as f64 * 1000.0) as f32;
                                    fm.extend = round_half_away(d);
                                    if fm.extend == 1000 {
                                        fm.extend = 0;
                                    }
                                    r = s + b"ExtendFont".len();
                                } else {
                                    // unknown name
                                    r = s;
                                    while !matches!(line.at(r), b' ' | b'"' | 0) {
                                        r += 1;
                                    }
                                    let mut msg = b"invalid entry for `".to_vec();
                                    msg.extend_from_slice(&fm.tfm_name);
                                    msg.extend_from_slice(b"': unknown name `");
                                    msg.extend_from_slice(&raw[s..r]);
                                    msg.extend_from_slice(b"' ignored");
                                    self.pdftex_warn_bytes(&msg);
                                }
                            } else {
                                while !matches!(line.at(r), b' ' | b'"' | 0) {
                                    r += 1;
                                }
                            }
                            if line.at(r) != b' ' {
                                break;
                            }
                        }
                        if line.at(r) == b'"' {
                            r += 1; // closing quote
                        } else {
                            let mut msg = b"invalid entry for `".to_vec();
                            msg.extend_from_slice(&fm.tfm_name);
                            msg.extend_from_slice(b"': closing quote missing");
                            self.pdftex_warn_bytes(&msg);
                            return; // bad_line
                        }
                    }
                    c => {
                        if c == b'P' {
                            // subfonts: 'PidEid=3,1'
                            let (n, vals) = cfmt::scan_ints(&raw[r..], c"PidEid=%i, %i %n");
                            if n >= 2 {
                                fm.pid = vals[0] as i16;
                                fm.eid = vals[1] as i16;
                                r += vals[2] as usize;
                                continue;
                            }
                        }
                        // encoding or font file specification
                        let (mut a, mut b) = (0u8, 0u8);
                        if line.at(r) == b'<' {
                            a = b'<';
                            r += 1;
                            if line.at(r) == b'<' || line.at(r) == b'[' {
                                b = line.at(r);
                                r += 1;
                            }
                        }
                        let buf = self.read_field(&line, &mut r);
                        // encoding, formats: '8r.enc' or '<8r.enc' or '<[8r.enc'
                        if buf.len() > 4 && ends_with_ci(&buf, b".enc") {
                            fm.encname = Some(buf);
                            u = 0;
                            v = 0; // u, v used if intervening blank: "<< foo"
                        } else if !buf.is_empty() {
                            // font file, formats:
                            //  subsetting:    '<cmr10.pfa'
                            //  no subsetting: '<<cmr10.pfa'
                            //  no embedding:  'cmr10.pfa'
                            if a == b'<' || u == b'<' {
                                fm.typ |= F_INCLUDED;
                                if (a == b'<' && b == 0) || (a == 0 && v == 0) {
                                    fm.typ |= F_SUBSETTED;
                                }
                                // otherwise b == '<' (or '[') => no subsetting
                            }
                            fm.ff_name = Some(buf);
                            if line.at(r) == 0 {
                                break 'scan;
                            }
                            u = 0;
                            v = 0;
                        } else {
                            u = a;
                            v = b;
                        }
                    }
                }
            }
        }
        // done:
        if let Some(ps) = &fm.ps_name {
            if check_std_t1font(ps) >= 0 {
                fm.typ |= F_STDT1FONT;
            }
        }
        match &fm.ff_name {
            Some(ff) if ff.len() > 3 => {
                if ends_with_ci(ff, b".ttf") || ends_with_ci(ff, b".ttc") {
                    fm.typ |= F_TRUETYPE;
                } else if ends_with_ci(ff, b".otf") {
                    fm.typ |= F_OTF;
                } else {
                    fm.typ |= F_TYPE1;
                }
            }
            _ if fm.ps_name.is_none() => {
                // without ps_name and font file it can only be a bitmap PK font
                fm.typ |= F_PK;
            }
            _ => fm.typ |= F_TYPE1, // assume a builtin font is Type1
        }
        if self.check_fm_entry(&mut fm, true) != 0 {
            return; // bad_line
        }
        // If we get here, the map line has been completely scanned without
        // errors; now follows the actual work of registering/deleting.
        if self.handle_subfont_fm(&fm) {
            return;
        }
        self.avl_do_entry(st, fm, mode);
    }

    /// `check_fm_entry`: consistency check of a map entry; zero if it is
    /// valid, else the sum of the problems' bits.
    fn check_fm_entry(&mut self, fm: &mut FmEntry, warn: bool) -> i32 {
        let mut a = 0;
        let name = String::from_utf8_lossy(&fm.tfm_name).into_owned();
        if fm.is_fontfile() && !fm.is_included() {
            if warn {
                self.pdftex_warn(&format!(
                    "ambiguous entry for `{name}': font file present but not included, \
                     will be treated as font file not present"
                ));
            }
            fm.ff_name = None;
            // do not set variable |a| as this entry will still be accepted
        }
        // if no tfm name, nothing to do here; a bare tfm is ok
        if fm.tfm_name.is_empty() {
            if warn {
                self.pdftex_warn("invalid map entry: tfm missing");
            }
            a += 1;
        }
        // TrueType fonts cannot be reencoded without subsetting
        if fm.is_truetype() && fm.is_reencoded() && !fm.is_subsetted() {
            if warn {
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': only subsetted TrueType fonts can be reencoded"
                ));
            }
            a += 2;
        }
        // SlantFont and ExtendFont can be used only with Type1 fonts
        if (fm.slant != 0 || fm.extend != 0)
            && (fm.tfm_name.is_empty() || !(fm.is_t1fontfile() && fm.is_included()))
        {
            if warn {
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': SlantFont/ExtendFont can be used only with embedded Type1 fonts"
                ));
            }
            a += 4;
        }
        // the value of SlantFont and ExtendFont must be reasonable
        if fm.slant.abs() > 1000 {
            if warn {
                let g = cfmt::fmt_g(fm.slant as f64 / 1000.0);
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': SlantFont value too big: {}",
                    String::from_utf8_lossy(&g)
                ));
            }
            a += 8;
        }
        if fm.extend.abs() > 2000 {
            if warn {
                let g = cfmt::fmt_g(fm.extend as f64 / 1000.0);
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': ExtendFont value too big: {}",
                    String::from_utf8_lossy(&g)
                ));
            }
            a += 16;
        }
        // subfonts must be used with subsetted non-reencoded TrueType fonts
        if fm.pid != -1 && !(fm.is_truetype() && fm.is_subsetted() && !fm.is_reencoded()) {
            if warn {
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': PidEid can be used only with subsetted non-reencoded TrueType fonts"
                ));
            }
            a += 32;
        }
        // font file for bitmap PK fonts is determinated by kpse and depends
        // on current font size; writet3.c ignores font file for bitmap PK fonts
        if fm.is_fontfile() && fm.is_pk() {
            if warn {
                let ff = String::from_utf8_lossy(fm.ff_name.as_deref().unwrap_or_default());
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': FontFile cannot be specified for bitmap PK font: {ff}"
                ));
            }
            a += 64;
        }
        // ps name cannot be stored into PDF file for PDF Type3 fonts
        if fm.ps_name.is_some() && fm.is_pk() {
            if warn {
                let ps = String::from_utf8_lossy(fm.ps_name.as_deref().unwrap_or_default());
                self.pdftex_warn(&format!(
                    "invalid entry for `{name}': PsName cannot be specified for bitmap PK font: {ps}"
                ));
            }
            a += 128;
        }
        a
    }

    /// `handle_subfont_fm` (subfont.c): whether `fm` is a subfont entry
    /// (`name@sfd@`). Subfonts only exist for TrueType fonts, which this
    /// lane does not embed; such an entry stops the run rather than being
    /// registered as an ordinary one.
    fn handle_subfont_fm(&mut self, fm: &FmEntry) -> bool {
        let p = &fm.tfm_name;
        let Some(q) = p.iter().position(|&c| c == b'@') else {
            return false;
        };
        let Some(r) = p[q + 1..]
            .iter()
            .position(|&c| c == b'@')
            .map(|i| i + q + 1)
        else {
            return false;
        };
        if q == 0 || r <= q + 1 || r != p.len() - 1 {
            return false;
        }
        self.pdftex_fail(&format!(
            "subfont map entry `{}' (TrueType subfonts) is not supported yet",
            String::from_utf8_lossy(p)
        ))
    }

    /// `avl_do_entry`: register `fm` in `tfm_tree` and `ps_tree`, as `mode`
    /// says. The entry is dropped when neither tree keeps it.
    fn avl_do_entry(&mut self, st: &mut Fonts, mut fm: FmEntry, mode: Mode) {
        let suppress_warn = self.get_pdf_suppress_warning_dup_map() > 0;
        let id = st.map.fms.len();
        let mut linked_tfm = false;
        'exit: {
            // handle tfm_name link
            if fm.tfm_name.as_slice() != NONTFM {
                if let Some(&p) = st.map.tfm_tree.get(&fm.tfm_name) {
                    match mode {
                        Mode::DupIgnore => {
                            if !suppress_warn {
                                self.pdftex_warn(&format!(
                                    "fontmap entry for `{}' already exists, duplicates ignored",
                                    String::from_utf8_lossy(&fm.tfm_name)
                                ));
                            }
                            break 'exit;
                        }
                        Mode::Replace | Mode::Delete => {
                            if st.map.in_use.contains(&p) {
                                self.pdftex_warn(&format!(
                                    "fontmap entry for `{}' has been used, replace/delete not allowed",
                                    String::from_utf8_lossy(&fm.tfm_name)
                                ));
                                break 'exit;
                            }
                            st.map.tfm_tree.remove(&fm.tfm_name);
                            let pe = st.map.fms[p].as_mut().unwrap();
                            pe.links &= !LINK_TFM;
                            if pe.links & LINK_PS == 0 {
                                st.map.fms[p] = None;
                            }
                        }
                    }
                }
                if mode != Mode::Delete {
                    st.map.tfm_tree.insert(fm.tfm_name.clone(), id);
                    fm.links |= LINK_TFM;
                    linked_tfm = true;
                }
            }
            // handle ps_name link
            if fm.ps_name.is_some() {
                let key = fm.ps_key();
                if let Some(&p) = st.map.ps_tree.get(&key) {
                    match mode {
                        Mode::DupIgnore => break 'exit,
                        Mode::Replace | Mode::Delete => {
                            if st.map.in_use.contains(&p) {
                                break 'exit;
                            }
                            st.map.ps_tree.remove(&key);
                            let pe = st.map.fms[p].as_mut().unwrap();
                            pe.links &= !LINK_PS;
                            if pe.links & LINK_TFM == 0 {
                                st.map.fms[p] = None;
                            }
                        }
                    }
                }
                if mode != Mode::Delete && fm.is_t1fontfile() && fm.is_included() {
                    st.map.ps_tree.insert(key, id);
                    fm.links |= LINK_PS;
                }
            }
        }
        // exit: keep the entry only if some tree points to it.
        let _ = linked_tfm;
        if fm.links & (LINK_TFM | LINK_PS) != 0 {
            st.map.fms.push(Some(fm));
        }
    }

    /// `fm_read_info`: read the pending map file or map line.
    fn fm_read_info(&mut self, st: &mut Fonts) {
        st.map.trees = true;
        let Some(item) = st.map.mitem.as_mut() else {
            return;
        };
        let Some(line) = item.line.clone() else {
            return; // nothing to do
        };
        let (mode, is_file) = (item.mode, item.is_file);
        if is_file {
            set_cur_file_name(Some(&line));
            let name = String::from_utf8_lossy(&line).into_owned();
            let found = crate::system::find_file(&name, Format::Map);
            // A map file read before in this process into an empty map, and
            // unchanged since: its entries, without parsing it again.
            let key = found.as_ref().and_then(|path| {
                let pristine = st.map.fms.is_empty()
                    && st.map.tfm_tree.is_empty()
                    && st.map.ps_tree.is_empty()
                    && st.map.in_use.is_empty();
                let stat = crate::system::StatSig::of(path)?;
                pristine.then(|| MapKey {
                    mode,
                    line: line.clone(),
                    path: path.clone(),
                    stat,
                    suppress_dup: self.get_pdf_suppress_warning_dup_map() > 0,
                })
            });
            if let Some(hit) = key.as_ref().and_then(MapCache::get) {
                let path = found.as_deref().unwrap_or_default();
                set_cur_file_name(Some(path.as_bytes()));
                let mut s = b"{".to_vec();
                s.extend_from_slice(path.as_bytes());
                self.tex_printf(&s);
                st.map.fms = hit.fms;
                st.map.tfm_tree = hit.tfm_tree;
                st.map.ps_tree = hit.ps_tree;
                self.tex_printf(b"}");
                if let Some(item) = st.map.mitem.as_mut() {
                    item.line = None;
                }
                set_cur_file_name(None);
                return;
            }
            let warnings_before = super::warnings_so_far();
            match found.and_then(|p| std::fs::read(&p).ok().map(|d| (p, d))) {
                None => self.pdftex_warn("cannot open font map file"),
                Some((path, data)) => {
                    set_cur_file_name(Some(path.as_bytes()));
                    let mut s = b"{".to_vec();
                    s.extend_from_slice(path.as_bytes());
                    self.tex_printf(&s);
                    let mut pos = 0usize;
                    let mut eof = false;
                    while !eof {
                        // fm_scan_line's reading part
                        let mut buf: Vec<u8> = Vec::new();
                        loop {
                            let mut c: i32 = match data.get(pos) {
                                Some(&b) => {
                                    pos += 1;
                                    b as i32
                                }
                                None => {
                                    eof = true;
                                    -1
                                }
                            };
                            // append_char_to_buf
                            if c == 9 {
                                c = 32;
                            }
                            if c == 13 || c == -1 {
                                c = 10;
                            }
                            if c != b' ' as i32 || buf.last().is_some_and(|&l| l != 32) {
                                if buf.len() + 1 > FM_BUF_SIZE {
                                    self.pdftex_fail("buffer overflow at file mapfile.c, line 440");
                                }
                                buf.push(c as u8);
                            }
                            if c == 10 {
                                break;
                            }
                        }
                        buf.pop(); // *(--p) = '\0'
                        self.fm_scan_line(st, &buf, mode);
                    }
                    self.tex_printf(b"}");
                    // Only a parse that printed nothing is replayed by
                    // printing the braces.
                    if let Some(k) = key {
                        if super::warnings_so_far() == warnings_before {
                            MapCache::put(
                                k,
                                MapParse {
                                    fms: st.map.fms.clone(),
                                    tfm_tree: st.map.tfm_tree.clone(),
                                    ps_tree: st.map.ps_tree.clone(),
                                },
                            );
                        }
                    }
                }
            }
        } else {
            set_cur_file_name(None); // makes pdftex_warn() shorter
            self.fm_scan_line(st, &line, mode);
        }
        if let Some(item) = st.map.mitem.as_mut() {
            item.line = None; // done with this line
        }
        set_cur_file_name(None);
    }

    /// `fmlookup`: the map entry of font `f`, or [`DUMMY`].
    fn fm_lookup(&mut self, st: &mut Fonts, f: i32) -> i32 {
        if !st.map.trees {
            self.fm_read_info(st); // only to read default map file
        }
        let tfm = self.c_string(self.font_name[f as usize]);
        match st.map.tfm_tree.get(&tfm) {
            Some(&id) => {
                st.map.in_use.insert(id);
                id as i32 + 1
            }
            None => DUMMY,
        }
    }

    /// `hasfmentry`, with the font state at hand.
    pub fn fm_has_entry(&mut self, st: &mut Fonts, f: i32) -> bool {
        if self.pdf_font_map[f as usize] == 0 {
            let v = self.fm_lookup(st, f);
            self.pdf_font_map[f as usize] = v;
        }
        self.pdf_font_map[f as usize] != DUMMY
    }

    /// `isscalable`, with the font state at hand.
    pub fn fm_is_scalable(&mut self, st: &mut Fonts, f: i32) -> bool {
        self.fm_has_entry(st, f) && !st.map.fm(self.pdf_font_map[f as usize]).is_pk()
    }

    /// `hasfmentry` (mapfile.c): whether font `f` has a map entry.
    pub fn hasfmentry(&mut self, f: i32) -> bool {
        self.with_fonts(|g, st| g.fm_has_entry(st, f))
    }

    /// `isscalable` (mapfile.c): whether font `f` has a map entry that is
    /// not a bitmap (PK) font.
    pub fn isscalable(&mut self, f: i32) -> bool {
        self.with_fonts(|g, st| g.fm_is_scalable(st, f))
    }

    /// `hasspacechar` (mapfile.c): whether the encoding of font `f` has
    /// `space` at position 32.
    pub fn hasspacechar(&mut self, f: i32) -> bool {
        self.with_fonts(|g, st| {
            if !g.fm_is_scalable(st, f) {
                return false;
            }
            let fm = st.map.fm(g.pdf_font_map[f as usize]).clone();
            // if a font is not re-encoded via its map entry, we assume it
            // has no space char
            if let Some(enc) = &fm.encname {
                if let Some(fe) = g.get_fe_entry(st, enc) {
                    return st.enc.fes[fe].glyph_names[32].as_slice() == b"space";
                }
            }
            false
        })
    }

    /// `check_ff_exist`: the path of font file `ff_name`, looked up once.
    pub fn check_ff_exist(
        &mut self,
        st: &mut Fonts,
        ff_name: &[u8],
        is_tt: bool,
    ) -> Option<String> {
        if let Some(p) = st.map.ff_tree.get(ff_name) {
            return p.clone();
        }
        let name = String::from_utf8_lossy(ff_name).into_owned();
        // kpse_truetype_format has no resolver format yet (TrueType is not
        // embedded by this lane), so a TrueType file is never found.
        let path = if is_tt {
            None
        } else {
            crate::system::find_file(&name, Format::Type1)
        };
        st.map.ff_tree.insert(ff_name.to_vec(), path.clone());
        path
    }

    /// `lookup_fontmap` (mapfile.c): the first Type 1 map entry with an
    /// existing font file for PostScript name `ps_name` (subset tag and
    /// `-Slant_`/`-Extend_` suffixes understood); used when fonts of an
    /// included PDF are replaced.
    pub fn lookup_fontmap(&mut self, st: &mut Fonts, ps_name: &[u8]) -> Option<usize> {
        if !st.map.trees {
            self.fm_read_info(st);
        }
        let mut s = ps_name;
        if ps_name.len() > 7
            && ps_name[..6].iter().all(|c| c.is_ascii_uppercase())
            && ps_name[6] == b'+'
        {
            s = &ps_name[7..]; // skip behind the subset tag
        }
        let (mut slant, mut extend) = (0i32, 0i32);
        let mut name = s.to_vec();
        if let Some(a) = find(s, b"-Slant_") {
            let b = &s[a + 7..];
            let (sl, e) = strtol10(b);
            if e != 0 && e == b.len() {
                slant = sl;
                name.truncate(a);
            } else if e != 0 {
                if let Some(c) = find(&b[e..], b"-Extend_") {
                    let d = &b[e + c + 8..];
                    let (ex, e2) = strtol10(d);
                    if e2 != 0 && e2 == d.len() {
                        slant = sl;
                        extend = ex;
                        name.truncate(a);
                    }
                }
            }
        } else if let Some(a) = find(s, b"-Extend_") {
            let b = &s[a + 8..];
            let (ex, e) = strtol10(b);
            if e != 0 && e == b.len() {
                extend = ex;
                name.truncate(a);
            }
        }
        let key = (name, slant, extend);
        // the entries equal to `key` (there is at most one: ps_tree never
        // holds duplicates), tried forward then backward as the C code does
        let fm = *st.map.ps_tree.get(&key)?;
        let ff = st.map.fms[fm].as_ref().unwrap().ff_name.clone().unwrap();
        if self.check_ff_exist(st, &ff, false).is_some() {
            return Some(fm);
        }
        None
    }

    /// `process_map_item`: `\pdfmapfile` or `\pdfmapline` `s`.
    fn process_map_item(&mut self, st: &mut Fonts, s: &[u8], is_file: bool) {
        let mut s = s;
        if s.first() == Some(&b' ') {
            s = &s[1..]; // ignore leading blank
        }
        let mode = match s.first() {
            Some(b'+') => {
                s = &s[1..];
                Mode::DupIgnore // insert entry, if it is not duplicate
            }
            Some(b'=') => {
                s = &s[1..];
                Mode::Replace // try to replace earlier entry
            }
            Some(b'-') => {
                s = &s[1..];
                Mode::Delete // try to delete entry
            }
            _ => {
                // like +, but also: flush default map file name
                if let Some(item) = st.map.mitem.as_mut() {
                    item.line = None;
                }
                Mode::DupIgnore
            }
        };
        if s.first() == Some(&b' ') {
            s = &s[1..]; // ignore blank after [+-=]
        }
        let item: Vec<u8> = if is_file {
            // remove blank at end
            s.iter().copied().take_while(|&c| c != b' ').collect()
        } else {
            s.to_vec() // blank at end allowed
        };
        if st.map.mitem.as_ref().is_some_and(|m| m.line.is_some()) {
            self.fm_read_info(st); // read default map file first
        }
        if !item.is_empty() {
            st.map.mitem = Some(MapItem {
                mode,
                is_file,
                line: Some(item),
            });
            self.fm_read_info(st);
        }
    }

    /// `pdfmapfile` (mapfile.c): `\pdfmapfile`.
    pub fn pdfmapfile(&mut self, t: i32) {
        let s = self.tokens_to_string(t);
        let bytes = self.c_string(s);
        self.with_fonts(|g, st| g.process_map_item(st, &bytes, true));
        self.flush_str(self.last_tokens_string);
    }

    /// `pdfmapline` (mapfile.c): `\pdfmapline`.
    pub fn pdfmapline(&mut self, t: i32) {
        let s = self.tokens_to_string(t);
        let bytes = self.c_string(s);
        self.with_fonts(|g, st| g.process_map_item(st, &bytes, false));
        self.flush_str(self.last_tokens_string);
    }

    /// `pdfmaplinesp` (mapfile.c): the map line of the fake space font.
    pub fn pdfmaplinesp(&mut self) {
        self.with_fonts(|g, st| {
            g.process_map_item(st, b"=pdftexspace PdfTeX-Space <pdftexspace.pfb", false)
        });
    }

    /// `pdfinitmapfile("pdftex.map")` (mapfile.c).
    pub fn pdf_init_map_file(&mut self) {
        self.with_fonts(|_, st| {
            st.map.mitem = Some(MapItem {
                mode: Mode::DupIgnore,
                is_file: true,
                line: Some(b"pdftex.map".to_vec()),
            })
        });
    }
}

/// `(integer) (d > 0 ? d + 0.5 : d - 0.5)` for a C `float` `d`.
fn round_half_away(d: f32) -> i32 {
    let d = d as f64;
    (if d > 0.0 { d + 0.5 } else { d - 0.5 }) as i32
}

/// `strstr`.
fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

/// `strtol(s, &e, 10)` on a whole C string: the value and the number of
/// bytes it used (0 if none).
fn strtol10(s: &[u8]) -> (i32, usize) {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let neg = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let start = i;
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = (v * 10 + (s[i] - b'0') as i64).min(i64::MAX / 20);
        i += 1;
    }
    if i == start {
        return (0, 0);
    }
    let v = if neg { -v } else { v };
    (v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32, i)
}
