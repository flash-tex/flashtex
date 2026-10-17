//! The fonts of the output: each font the XDV stream defines (a native
//! font's `define_native_font`, a TFM font's `fnt_def`), and the display
//! list's `FONT` resources that draw them (spec §5.1, §11.1).
//!
//! * A **native font** is a font instance: a file, a face in it (and the
//!   default variation instance). Its resource has `format` `opentype` with
//!   `glyph_ids` (spec §11.1): the glyphs of `set_glyphs` are glyph ids. The
//!   size, `extend` and `slant` go into each glyph's matrix, so every size
//!   of a face shares one resource.
//! * A **TFM font** is drawn by the Type 1 program its font map entry names
//!   (`pdftex.map`, as TeX Live's `dvipdfmx.cfg` reads it), re-encoded by
//!   the entry's `.enc` file or with the program's built-in encoding; the
//!   resource is Classic's `type1` resource (spec §5.1), with the same key.
//!
//! A font that cannot be drawn this way (a TFM font without a map entry or
//! program, a virtual font, a native font file that cannot be read) gets a
//! resource with a `problem` (spec §5.1), and the pages using it are
//! flagged INCOMPLETE.

use super::agl::Agl;
use super::fontmap::{parse_enc, FontMap, MapIndex};
use flashtex_display_list::json::Json;
use flashtex_display_list::sha256::{hex, sha256, Sha256};
use flashtex_engine::resolver::Format;
use flashtex_engine::system::StatSig;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// What the runs of a resident engine (`crate::resident`) share of their
/// fonts' output: the font map's index (46,000 lines of `pdftex.map`, a
/// third of a one-page article's keystroke when each run indexed it again)
/// and the font programs read, each kept while its file is unchanged (a
/// stat signature taken before it was read and still the same after; a
/// racy one, within the file system's time tick, keeps nothing); what the
/// output parses of each face of a program ([`FaceData`], by the program's
/// SHA-256 and the face index, so a function of the bytes alone); and the
/// Adobe Glyph List, kept while its files are the same and unchanged.
#[derive(Default)]
pub struct FontCache {
    map: Mutex<Option<(String, StatSig, Arc<MapIndex>)>>,
    programs: Mutex<HashMap<String, Program>>,
    /// By [`face_key`], with the program's SHA-256.
    faces: Mutex<HashMap<[u8; 32], KeptFace>>,
    agl: Mutex<Option<(AglFiles, Arc<Agl>)>>,
}

type KeptFace = ([u8; 32], Arc<FaceData>);

/// The glyph list files found (`agl::LIST_FILES`, in order), each with its
/// signature when read.
type AglFiles = Vec<Option<(String, Option<StatSig>)>>;

/// What the output reads of one face of a font program: the parsed face
/// (`flashtex_pdf::truetype`), what a font's definition takes from it, and
/// the PDF writer's last answers for it (`super::pdf::FaceMemo`). All of it
/// is a function of the program's bytes and the face index.
pub struct FaceData {
    pub font: Result<flashtex_pdf::truetype::TrueTypeFont, String>,
    /// Each glyph's advance (font units); empty when `font` is an error.
    pub advances: Arc<Vec<u16>>,
    /// The `name` table's family (empty if none).
    pub family: String,
    pub memo: Mutex<super::pdf::FaceMemo>,
}

impl FaceData {
    pub fn new(program: &Arc<Vec<u8>>, index: u32) -> FaceData {
        let font = flashtex_pdf::truetype::TrueTypeFont::parse_face_shared(program.clone(), index);
        let advances = match &font {
            Ok(f) => (0..f.num_glyphs()).map(|g| f.advance(g)).collect(),
            Err(_) => Vec::new(),
        };
        FaceData {
            font,
            advances: Arc::new(advances),
            family: family_name(program, index).unwrap_or_default(),
            memo: Mutex::default(),
        }
    }
}

/// A face's key in [`FontCache`]: its program's SHA-256 and its index.
pub fn face_key(sha: &[u8; 32], index: u32) -> [u8; 32] {
    flashtex_display_list::resource::opentype_font_key(sha, index, &[])
}

/// A font file's signature when read, its bytes and their SHA-256.
type Program = (StatSig, Arc<Vec<u8>>, [u8; 32]);

impl FontCache {
    /// The index of the map file at `path`.
    fn map_index(&self, path: &str) -> Option<Arc<MapIndex>> {
        let sig = StatSig::of(path);
        let mut kept = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some((p, s, i)), Some(sig)) = (kept.as_ref(), sig.as_ref()) {
            if p == path && s == sig {
                return Some(i.clone());
            }
        }
        let index = Arc::new(MapIndex::from_vec(std::fs::read(path).ok()?));
        *kept = match sig {
            Some(s) if Some(s) == StatSig::of(path) => Some((path.to_string(), s, index.clone())),
            _ => None,
        };
        Some(index)
    }

    /// The bytes of the font file at `path`, and their SHA-256.
    fn program(&self, path: &str) -> Option<(Arc<Vec<u8>>, [u8; 32])> {
        let sig = StatSig::of(path);
        let mut kept = self.programs.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some((s, d, h)), Some(sig)) = (kept.get(path), sig.as_ref()) {
            if s == sig {
                return Some((d.clone(), *h));
            }
        }
        let data = Arc::new(std::fs::read(path).ok()?);
        let sha = sha256(&data);
        match sig {
            Some(s) if Some(s) == StatSig::of(path) => {
                kept.insert(path.to_string(), (s, data.clone(), sha));
            }
            _ => {
                kept.remove(path);
            }
        }
        Some((data, sha))
    }

    /// Face `index` of `program` (whose SHA-256 is `sha`), parsed once.
    pub fn face(&self, sha: &[u8; 32], program: &Arc<Vec<u8>>, index: u32) -> Arc<FaceData> {
        let key = face_key(sha, index);
        let mut faces = self.faces.lock().unwrap_or_else(|e| e.into_inner());
        faces
            .entry(key)
            .or_insert_with(|| (*sha, Arc::new(FaceData::new(program, index))))
            .1
            .clone()
    }

    /// The Adobe Glyph List as `Agl::load` reads it now: the files it
    /// finds, read again when another file is found or one changed.
    pub fn agl(&self) -> Arc<Agl> {
        let found = Agl::find_files();
        let files: AglFiles = found
            .iter()
            .map(|f| f.as_ref().map(|p| (p.clone(), StatSig::of(p))))
            .collect();
        let mut kept = self.agl.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, agl)) = kept.as_ref() {
            // the same files, each with a signature that holds (a missing
            // or racy one equals none)
            let same = k.len() == files.len()
                && k.iter().zip(&files).all(|(a, b)| match (a, b) {
                    (None, None) => true,
                    (Some((p, Some(s))), Some((q, Some(t)))) => p == q && s == t,
                    _ => false,
                });
            if same {
                return agl.clone();
            }
        }
        let agl = Arc::new(Agl::load_files(&found));
        let after: AglFiles = found
            .iter()
            .map(|f| f.as_ref().map(|p| (p.clone(), StatSig::of(p))))
            .collect();
        *kept = (after == files).then(|| (files, agl.clone()));
        agl
    }

    /// Keep only the programs of `used` (the last run's fonts), and the
    /// faces of the programs kept.
    pub fn retain(&self, used: &HashSet<String>) {
        let mut programs = self.programs.lock().unwrap_or_else(|e| e.into_inner());
        programs.retain(|p, _| used.contains(p));
        let shas: HashSet<[u8; 32]> = programs.values().map(|(_, _, h)| *h).collect();
        drop(programs);
        self.faces
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, (sha, _)| shas.contains(sha));
    }
}

/// `define_native_font`'s flags (XeTeX_ext.h).
pub const XDV_FLAG_VERTICAL: u16 = 0x0100;
pub const XDV_FLAG_COLORED: u16 = 0x0200;
pub const XDV_FLAG_EXTEND: u16 = 0x1000;
pub const XDV_FLAG_SLANT: u16 = 0x2000;
pub const XDV_FLAG_EMBOLDEN: u16 = 0x4000;

/// A native font as `define_native_font` defines it.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeDef {
    /// The font file's path, as XeTeX recorded it.
    pub path: Vec<u8>,
    pub index: u32,
    /// The size in DVI units (scaled points).
    pub size: i32,
    pub flags: u16,
    /// `color=RRGGBBAA` (`XDV_FLAG_COLORED`).
    pub rgba: Option<u32>,
    /// `extend=`, `slant=`, `embolden=` as 16.16 fixed point.
    pub extend: Option<i32>,
    pub slant: Option<i32>,
    pub embolden: Option<i32>,
}

/// A TFM font as `fnt_def` defines it.
#[derive(Clone, Debug, PartialEq)]
pub struct TfmDef {
    pub checksum: u32,
    /// Scaled size and design size (DVI units).
    pub size: i32,
    pub dsize: i32,
    pub name: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontDef {
    Native(NativeDef),
    Tfm(TfmDef),
}

/// What the page builder needs of a defined font.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub def: FontDef,
    /// The display-list resource id.
    pub res: u16,
    /// Why its glyphs cannot be drawn, if they cannot.
    pub problem: Option<String>,
    /// Native fonts: units per em, `hhea` ascender and descender (font
    /// units, the descender negative), each glyph's advance (font units).
    pub upm: u16,
    pub ascent: i16,
    pub descent: i16,
    pub advances: Arc<Vec<u16>>,
}

impl Loaded {
    /// `extend`, `slant` and `embolden` of a native font (1, 0, 0 if none).
    pub fn transform(&self) -> (f64, f64, f64) {
        match &self.def {
            FontDef::Native(n) => (
                n.extend.map_or(1.0, fix2d),
                n.slant.map_or(0.0, fix2d),
                n.embolden.map_or(0.0, fix2d),
            ),
            FontDef::Tfm(_) => (1.0, 0.0, 0.0),
        }
    }

    /// The advance of glyph `gid` in font units (0 if unknown).
    pub fn advance(&self, gid: u16) -> u16 {
        let a = &self.advances;
        match a.get(gid as usize) {
            Some(&w) => w,
            None => a.last().copied().unwrap_or(0),
        }
    }
}

pub fn fix2d(f: i32) -> f64 {
    f as f64 / 65536.0
}

/// One display-list `FONT` resource.
#[derive(Clone, Debug)]
pub struct FontRes {
    pub id: u16,
    pub key: [u8; 32],
    pub info: Json,
    pub program: Arc<Vec<u8>>,
    pub kind: ResKind,
}

#[derive(Clone, Debug)]
pub enum ResKind {
    /// A face of an OpenType/TrueType file.
    Native {
        path: String,
        index: u32,
        cff: bool,
        ps_name: String,
        /// The program's SHA-256 (its [`FaceData`]'s key with `index`).
        sha: [u8; 32],
    },
    /// A Type 1 program for a TFM font, with its encoding.
    Type1 {
        path: String,
        ps_name: String,
        encoding: Arc<Vec<Vec<u8>>>,
        slant: i32,
        extend: i32,
        tfm: Vec<u8>,
    },
    /// Not drawable (see `problem` in its JSON).
    Missing,
}

/// The fonts of one run.
#[derive(Clone, Default)]
pub struct Fonts {
    /// By DVI font number.
    pub defined: HashMap<i32, Loaded>,
    /// Resources, by id - 1.
    pub res: Vec<FontRes>,
    by_key: HashMap<[u8; 32], u16>,
    map: Option<FontMap>,
    /// Map lines from `x:fontmapline`/`pdf:mapline`, applied once the map
    /// is read.
    pending_lines: Vec<Vec<u8>>,
    programs: HashMap<String, (Arc<Vec<u8>>, [u8; 32])>,
    /// TFM fonts' character widths by resource and code, in thousandths
    /// of the font's size (the PDF's `/Widths`), as the pages use them.
    pub widths: HashMap<u16, std::collections::BTreeMap<u16, f64>>,
    /// A resident engine's runs' shared cache.
    pub cache: Option<Arc<FontCache>>,
    /// Without one, the faces this run parsed ([`Fonts::face_data`]).
    faces: HashMap<[u8; 32], Arc<FaceData>>,
}

impl Fonts {
    /// Define DVI font `k`; a redefinition with the same parameters keeps
    /// the existing entry.
    pub fn define(&mut self, k: i32, def: FontDef) {
        if self.defined.get(&k).is_some_and(|l| l.def == def) {
            return;
        }
        let loaded = match &def {
            FontDef::Native(n) => self.load_native(n),
            FontDef::Tfm(t) => self.load_tfm(t),
        };
        let loaded = Loaded { def, ..loaded };
        self.defined.insert(k, loaded);
    }

    pub fn get(&self, k: i32) -> Option<&Loaded> {
        self.defined.get(&k)
    }

    pub fn resource(&self, id: u16) -> Option<&FontRes> {
        self.res.get(id as usize - 1)
    }

    /// `x:fontmapline` / `pdf:mapline`.
    pub fn map_line(&mut self, line: &[u8]) {
        match self.map.as_mut() {
            Some(m) => m.apply_line(line),
            None => self.pending_lines.push(line.to_vec()),
        }
    }

    fn add(&mut self, key: [u8; 32], info: Json, program: Arc<Vec<u8>>, kind: ResKind) -> u16 {
        if let Some(&id) = self.by_key.get(&key) {
            return id;
        }
        let id = u16::try_from(self.res.len() + 1).unwrap_or(u16::MAX);
        self.res.push(FontRes {
            id,
            key,
            info,
            program,
            kind,
        });
        self.by_key.insert(key, id);
        id
    }

    fn missing(&mut self, what: &str, problem: String) -> Loaded {
        let mut h = Sha256::new();
        h.update(b"display-list-v3 font\0none\0");
        h.update(what.as_bytes());
        h.update(b"\0problem\0");
        h.update(problem.as_bytes());
        let key = h.finish();
        let info = Json::Obj(vec![
            ("format".into(), Json::Str("none".into())),
            ("tex_name".into(), Json::Str(what.into())),
            ("problem".into(), Json::Str(problem.clone())),
        ]);
        let res = self.add(key, info, Arc::new(Vec::new()), ResKind::Missing);
        Loaded {
            def: FontDef::Tfm(TfmDef {
                checksum: 0,
                size: 0,
                dsize: 0,
                name: Vec::new(),
            }),
            res,
            problem: Some(problem),
            upm: 1000,
            ascent: 0,
            descent: 0,
            advances: Arc::new(Vec::new()),
        }
    }

    fn read_program(&mut self, path: &str) -> Option<(Arc<Vec<u8>>, [u8; 32])> {
        if let Some(p) = self.programs.get(path) {
            return Some(p.clone());
        }
        let (data, sha) = match &self.cache {
            Some(c) => c.program(path)?,
            None => {
                let data = Arc::new(std::fs::read(path).ok()?);
                let sha = sha256(&data);
                (data, sha)
            }
        };
        self.programs.insert(path.to_string(), (data.clone(), sha));
        Some((data, sha))
    }

    /// Face `index` of the program whose SHA-256 is `sha`, parsed once (by
    /// the resident engine's cache, else once per run).
    pub fn face_data(
        &mut self,
        sha: &[u8; 32],
        program: &Arc<Vec<u8>>,
        index: u32,
    ) -> Arc<FaceData> {
        match &self.cache {
            Some(c) => c.face(sha, program, index),
            None => self
                .faces
                .entry(face_key(sha, index))
                .or_insert_with(|| Arc::new(FaceData::new(program, index)))
                .clone(),
        }
    }

    fn load_native(&mut self, n: &NativeDef) -> Loaded {
        let path = String::from_utf8_lossy(&n.path).into_owned();
        let Some((program, sha)) = self.read_program(&path) else {
            return self.missing(&path, format!("cannot read the font file {path}"));
        };
        let data = self.face_data(&sha, &program, n.index);
        let font = match &data.font {
            Ok(f) => f,
            Err(e) => return self.missing(&path, format!("{path}: {e}")),
        };
        let cff = font.outlines == flashtex_pdf::truetype::Outlines::Cff;
        let upm = font.units_per_em;
        let advances = data.advances.clone();
        let key = flashtex_display_list::resource::opentype_font_key(&sha, n.index, &[]);
        let family = data.family.clone();
        let info = Json::Obj(vec![
            ("format".into(), Json::Str("opentype".into())),
            ("glyph_ids".into(), Json::Bool(true)),
            ("face_index".into(), Json::Int(n.index as i64)),
            ("units_per_em".into(), Json::Int(upm as i64)),
            (
                "font_matrix".into(),
                Json::Str(format!("{0} 0 0 {0} 0 0", 1.0 / upm as f64)),
            ),
            (
                "outlines".into(),
                Json::Str(if cff { "cff" } else { "truetype" }.into()),
            ),
            ("variations".into(), Json::Arr(vec![])),
            ("family".into(), Json::Str(family)),
            ("ps_name".into(), Json::Str(font.postscript_name.clone())),
            ("file".into(), Json::Str(path.clone())),
            ("program_sha256".into(), Json::Str(hex(&sha))),
            ("program_bytes".into(), Json::Int(program.len() as i64)),
        ]);
        let res = self.add(
            key,
            info,
            program,
            ResKind::Native {
                path,
                index: n.index,
                cff,
                ps_name: font.postscript_name.clone(),
                sha,
            },
        );
        Loaded {
            def: FontDef::Native(n.clone()),
            res,
            problem: None,
            upm,
            ascent: font.ascender,
            descent: font.descender,
            advances,
        }
    }

    /// Read the font map now rather than at the first TFM font (a hot
    /// spare, `crate::host::spare`, does it while it waits).
    pub fn prewarm(&mut self) {
        self.font_map();
    }

    fn font_map(&mut self) -> &FontMap {
        if self.map.is_none() {
            let path = flashtex_engine::system::find_file("pdftex.map", Format::Map);
            let mut m = match (&self.cache, path) {
                (Some(c), Some(p)) => c.map_index(&p).map(FontMap::over),
                (None, Some(p)) => std::fs::read(p).ok().map(FontMap::from_vec),
                (_, None) => None,
            }
            .unwrap_or_default();
            for l in std::mem::take(&mut self.pending_lines) {
                m.apply_line(&l);
            }
            self.map = Some(m);
        }
        self.map.as_ref().expect("just set")
    }

    /// Face `index` of the program whose SHA-256 is `sha`, if this run
    /// parsed it without a cache ([`Fonts::face_data`]).
    pub fn parsed_face(&self, sha: &[u8; 32], index: u32) -> Option<Arc<FaceData>> {
        self.faces.get(&face_key(sha, index)).cloned()
    }

    /// The font files this run read.
    pub fn programs_read(&self) -> HashSet<String> {
        self.programs.keys().cloned().collect()
    }

    fn load_tfm(&mut self, t: &TfmDef) -> Loaded {
        let tfm = String::from_utf8_lossy(&t.name).into_owned();
        let Some(e) = self.font_map().get(&t.name) else {
            let why = if flashtex_engine::system::find_file(&tfm, Format::Vf).is_some() {
                "a virtual font"
            } else {
                "no font map entry"
            };
            return self.missing(&tfm, format!("TFM font {tfm}: {why}"));
        };
        let Some(ff) = e
            .font_file
            .as_ref()
            .map(|f| String::from_utf8_lossy(f).into_owned())
        else {
            return self.missing(
                &tfm,
                format!("TFM font {tfm}: the map entry names no font file"),
            );
        };
        let is_t1 = ff.ends_with(".pfb") || ff.ends_with(".pfa") || !ff.contains('.');
        if !is_t1 {
            return self.missing(&tfm, format!("TFM font {tfm}: {ff} is not a Type 1 font"));
        }
        let Some(path) = flashtex_engine::system::find_file(&ff, Format::Type1) else {
            return self.missing(&tfm, format!("TFM font {tfm}: cannot find {ff}"));
        };
        let Some((program, sha)) = self.read_program(&path) else {
            return self.missing(&tfm, format!("TFM font {tfm}: cannot read {path}"));
        };
        let names: Vec<Vec<u8>> = match &e.enc_file {
            Some(enc) => {
                let enc = String::from_utf8_lossy(enc).into_owned();
                match flashtex_engine::system::find_file(&enc, Format::Enc)
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|d| parse_enc(&d))
                {
                    Some(n) => n,
                    None => {
                        return self.missing(&tfm, format!("TFM font {tfm}: cannot read {enc}"))
                    }
                }
            }
            None => builtin_encoding(&program),
        };
        let mut h = Sha256::new();
        h.update(b"display-list-v3 font\0");
        h.update(b"type1");
        h.update(&[0]);
        h.update(&sha);
        for g in &names {
            h.update(g);
            h.update(&[0]);
        }
        h.update(&e.slant.to_le_bytes());
        h.update(&e.extend.to_le_bytes());
        let key = h.finish();
        let ps_name = e
            .ps_name
            .as_ref()
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .unwrap_or_default();
        let lossy = |b: &[u8]| Json::Str(String::from_utf8_lossy(b).into_owned());
        let info = Json::Obj(vec![
            ("tex_name".into(), Json::Str(tfm.clone())),
            ("tex_size".into(), Json::Int(t.size as i64)),
            ("ps_name".into(), Json::Str(ps_name.clone())),
            ("format".into(), Json::Str("type1".into())),
            ("file".into(), Json::Str(path.clone())),
            ("program_sha256".into(), Json::Str(hex(&sha))),
            ("program_bytes".into(), Json::Int(program.len() as i64)),
            ("slant".into(), Json::Int(e.slant as i64)),
            ("extend".into(), Json::Int(e.extend as i64)),
            (
                "font_matrix".into(),
                if e.slant != 0 || e.extend != 0 {
                    let ex = if e.extend != 0 {
                        e.extend as f64 / 1000.0
                    } else {
                        1.0
                    };
                    let sl = e.slant as f64 / 1000.0;
                    Json::Str(format!("{} 0 {} 0.001 0 0", 0.001 * ex, 0.001 * sl))
                } else {
                    Json::Null
                },
            ),
            (
                "encoding".into(),
                Json::Arr(names.iter().map(|g| lossy(g)).collect()),
            ),
        ]);
        let res = self.add(
            key,
            info,
            program,
            ResKind::Type1 {
                path,
                ps_name,
                encoding: Arc::new(names),
                slant: e.slant,
                extend: e.extend,
                tfm: t.name.clone(),
            },
        );
        Loaded {
            def: FontDef::Tfm(t.clone()),
            res,
            problem: None,
            upm: 1000,
            ascent: 0,
            descent: 0,
            advances: Arc::new(Vec::new()),
        }
    }
}

/// A Type 1 program's built-in encoding (its clear-text `/Encoding`), as
/// 256 glyph names (`.notdef` where none).
pub fn builtin_encoding(program: &[u8]) -> Vec<Vec<u8>> {
    let clear: &[u8] = if program.first() == Some(&0x80) && program.len() >= 6 {
        let len = u32::from_le_bytes([program[2], program[3], program[4], program[5]]) as usize;
        program.get(6..6 + len).unwrap_or(&program[6..])
    } else {
        program
    };
    let map = flashtex_pdf::type1::builtin_encoding(clear);
    (0..256)
        .map(|c| {
            map.get(&(c as u8))
                .map(|n| n.as_bytes().to_vec())
                .unwrap_or_else(|| b".notdef".to_vec())
        })
        .collect()
}

/// The family name (name id 16, else 1) of face `index`, for display.
fn family_name(data: &[u8], index: u32) -> Option<String> {
    let rd16 = |o: usize| data.get(o..o + 2).map(|s| u16::from_be_bytes([s[0], s[1]]));
    let rd32 = |o: usize| {
        data.get(o..o + 4)
            .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    };
    let dir = if rd32(0)? == 0x7474_6366 {
        rd32(12 + 4 * index as usize)? as usize
    } else {
        0
    };
    let n = rd16(dir + 4)? as usize;
    let mut name_off = None;
    for i in 0..n {
        let rec = dir + 12 + 16 * i;
        if data.get(rec..rec + 4)? == b"name" {
            name_off = Some(rd32(rec + 8)? as usize);
        }
    }
    let t = name_off?;
    let count = rd16(t + 2)? as usize;
    let strings = t + rd16(t + 4)? as usize;
    let mut best: Option<(u8, String)> = None;
    for i in 0..count {
        let r = t + 6 + 12 * i;
        let (pid, eid, nid) = (rd16(r)?, rd16(r + 2)?, rd16(r + 6)?);
        let (len, off) = (rd16(r + 8)? as usize, rd16(r + 10)? as usize);
        let rank = match nid {
            16 => 2,
            1 => 1,
            _ => continue,
        };
        let raw = data.get(strings + off..strings + off + len)?;
        let s = if pid == 3 || (pid == 0) {
            let u: Vec<u16> = raw
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u)
        } else if pid == 1 && eid == 0 {
            raw.iter().map(|&b| b as char).collect()
        } else {
            continue;
        };
        if best.as_ref().is_none_or(|(r, _)| rank > *r) {
            best = Some((rank, s));
        }
    }
    best.map(|(_, s)| s)
}
