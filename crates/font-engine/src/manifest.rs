//! Pinned, licensed font set: a manifest whose entries carry exactly the
//! fields of `crates/font-resources` (`FontDescriptor` with its eight wire
//! fields, `LicenseMetadata`, `ManifestEntry { font, path, license }`,
//! `schema_version: 1`), so the same JSON can be handed to that loader.
//!
//! Why a separate loader here: `crates/font-resources` currently accepts only
//! `format: "static-truetype"` (glyf, face 0). The pinned set is Latin Modern,
//! which is OpenType CFF (`format: "opentype-cff"`), so this crate performs
//! the same checks itself — SHA-256 and byte length of the font bytes,
//! declared units per em / glyph count / PostScript name against the parsed
//! program, and SHA-256 of the licence text — and refuses anything that does
//! not match. Font files are never committed; `path` is relative to a root
//! the caller passes explicitly (no discovery).
//!
//! JSON handling is a small purpose-built reader/writer (no external crates);
//! `Manifest::to_json` output is byte-stable so the committed
//! `fonts/manifest.json` can be regenerated and compared.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::truetype::{Outlines, TrueTypeFace};
use crate::{Error, Face, FontSource};

/// Mirrors `flashtex_font_resources::FontDescriptor` field for field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontDescriptor {
    pub font_id: String,
    pub sha256: String,
    pub byte_length: u64,
    /// `"static-truetype"` (glyf) or `"opentype-cff"`.
    pub format: String,
    pub face_index: u32,
    pub units_per_em: u32,
    pub glyph_count: u32,
    pub postscript_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingPermission {
    Allowed,
    Restricted,
    Unknown,
}

impl EmbeddingPermission {
    fn as_str(self) -> &'static str {
        match self {
            EmbeddingPermission::Allowed => "allowed",
            EmbeddingPermission::Restricted => "restricted",
            EmbeddingPermission::Unknown => "unknown",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "allowed" => EmbeddingPermission::Allowed,
            "restricted" => EmbeddingPermission::Restricted,
            "unknown" => EmbeddingPermission::Unknown,
            _ => return None,
        })
    }
}

/// Mirrors `flashtex_font_resources::LicenseMetadata`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseMetadata {
    pub identifier: String,
    pub copyright: String,
    pub source: String,
    pub text_path: String,
    pub text_sha256: String,
    pub embedding_permission: EmbeddingPermission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub font: FontDescriptor,
    pub path: String,
    pub license: LicenseMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema_version: u32,
    pub resources: Vec<ManifestEntry>,
}

const GUST_TEXT_SHA256: &str = "49ea6cb9257bbee0a3979c48a774cd221550ac1c20c95549efe45fc99cc18050";
const LM_COPYRIGHT: &str = "Copyright 2003, 2009 B. Jackowski and J. M. Nowacki (on behalf of TeX users groups). This work is released under the GUST Font License -- see http://tug.org/fonts/licenses/GUST-FONT-LICENSE.txt for details.";
const LM_MATH_COPYRIGHT: &str = "Copyright 2012--2014 for Latin Modern Math OTF by B. Jackowski, P. Strzelczyk and P. Pianowski (on behalf of TeX users groups). This work is released under the GUST Font License -- see http://tug.org/fonts/licenses/GUST-FONT-LICENSE.txt for details.";

fn gust(copyright: &str) -> LicenseMetadata {
    LicenseMetadata {
        identifier: "LicenseRef-GUST-Font-License-1.0".into(),
        copyright: copyright.into(),
        source: "TeX Live 2026 (BasicTeX) texmf-dist/fonts/opentype/public; licence text from http://tug.org/fonts/licenses/GUST-FONT-LICENSE.txt".into(),
        text_path: "GUST-FONT-LICENSE.txt".into(),
        text_sha256: GUST_TEXT_SHA256.into(),
        embedding_permission: EmbeddingPermission::Allowed,
    }
}

fn lm(
    id: &str,
    path: &str,
    sha: &str,
    len: u64,
    glyphs: u32,
    ps: &str,
    copyright: &str,
) -> ManifestEntry {
    ManifestEntry {
        font: FontDescriptor {
            font_id: id.into(),
            sha256: sha.into(),
            byte_length: len,
            format: "opentype-cff".into(),
            face_index: 0,
            units_per_em: 1000,
            glyph_count: glyphs,
            postscript_name: ps.into(),
        },
        path: path.into(),
        license: gust(copyright),
    }
}

/// The pinned Latin Modern set (GUST Font License). Paths are relative to
/// TeX Live's `texmf-dist/fonts/opentype/public`.
pub fn pinned_latin_modern() -> Manifest {
    Manifest {
        schema_version: 1,
        resources: vec![
            lm(
                "lm.roman10.regular",
                "lm/lmroman10-regular.otf",
                "1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852",
                111536,
                821,
                "LMRoman10-Regular",
                LM_COPYRIGHT,
            ),
            lm(
                "lm.roman10.bold",
                "lm/lmroman10-bold.otf",
                "102fe06c430a8b681b2bf6876b7cd967ae4d47b4b6b41d915eb7913b726d9fb1",
                111240,
                821,
                "LMRoman10-Bold",
                LM_COPYRIGHT,
            ),
            lm(
                "lm.roman10.italic",
                "lm/lmroman10-italic.otf",
                "c1fce25075567bb8dbf2151658c3b442690041db17a2d49fc9e55905ea5b7169",
                118828,
                821,
                "LMRoman10-Italic",
                LM_COPYRIGHT,
            ),
            lm(
                "lm.roman10.bolditalic",
                "lm/lmroman10-bolditalic.otf",
                "c37a28eed7a6e03f792b98b5e5f637b2fcda378bb4855f99284f1a88fe35f124",
                118204,
                821,
                "LMRoman10-BoldItalic",
                LM_COPYRIGHT,
            ),
            lm(
                "lm.sans10.regular",
                "lm/lmsans10-regular.otf",
                "d431b786b9b603662718e79cfe9b441f47a8b0b3e854dde89d5acb3ed7cfd682",
                95128,
                821,
                "LMSans10-Regular",
                LM_COPYRIGHT,
            ),
            lm(
                "lm.mono10.regular",
                "lm/lmmono10-regular.otf",
                "22deb6d3be3ffcb40b33f0e010afb256c677d97c2a2bdee1f5740cdc97751558",
                64684,
                786,
                "LMMono10-Regular",
                LM_COPYRIGHT,
            ),
            lm(
                "lm.math",
                "lm-math/latinmodern-math.otf",
                "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f",
                733736,
                4802,
                "LatinModernMath-Regular",
                LM_MATH_COPYRIGHT,
            ),
        ],
    }
}

/// Default TeX Live location of the pinned paths on this project's Macs.
pub const BASICTEX_OPENTYPE_ROOT: &str =
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public";

// ---------------------------------------------------------------- JSON out

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Manifest {
    /// Pretty JSON with 2-space indentation and fixed key order.
    pub fn to_json(&self) -> String {
        let mut o = String::new();
        o.push_str("{\n  \"schema_version\": ");
        o.push_str(&self.schema_version.to_string());
        o.push_str(",\n  \"resources\": [");
        for (i, e) in self.resources.iter().enumerate() {
            o.push_str(if i == 0 { "\n" } else { ",\n" });
            o.push_str("    {\n      \"font\": {\n");
            let f = &e.font;
            let fields: [(&str, String); 8] = [
                ("font_id", quoted(&f.font_id)),
                ("sha256", quoted(&f.sha256)),
                ("byte_length", f.byte_length.to_string()),
                ("format", quoted(&f.format)),
                ("face_index", f.face_index.to_string()),
                ("units_per_em", f.units_per_em.to_string()),
                ("glyph_count", f.glyph_count.to_string()),
                ("postscript_name", quoted(&f.postscript_name)),
            ];
            for (j, (k, v)) in fields.iter().enumerate() {
                o.push_str(&format!(
                    "        \"{k}\": {v}{}\n",
                    if j + 1 < fields.len() { "," } else { "" }
                ));
            }
            o.push_str("      },\n      \"path\": ");
            json_str(&mut o, &e.path);
            o.push_str(",\n      \"license\": {\n");
            let l = &e.license;
            let fields: [(&str, String); 6] = [
                ("identifier", quoted(&l.identifier)),
                ("copyright", quoted(&l.copyright)),
                ("source", quoted(&l.source)),
                ("text_path", quoted(&l.text_path)),
                ("text_sha256", quoted(&l.text_sha256)),
                (
                    "embedding_permission",
                    quoted(l.embedding_permission.as_str()),
                ),
            ];
            for (j, (k, v)) in fields.iter().enumerate() {
                o.push_str(&format!(
                    "        \"{k}\": {v}{}\n",
                    if j + 1 < fields.len() { "," } else { "" }
                ));
            }
            o.push_str("      }\n    }");
        }
        o.push_str("\n  ]\n}\n");
        o
    }
}

fn quoted(s: &str) -> String {
    let mut o = String::new();
    json_str(&mut o, s);
    o
}

// ---------------------------------------------------------------- JSON in

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn err<T>(&self, m: &str) -> Result<T, Error> {
        Err(Error::Malformed(format!(
            "manifest JSON: {m} at byte {}",
            self.i
        )))
    }
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }
    fn value(&mut self) -> Result<Json, Error> {
        self.ws();
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                loop {
                    self.ws();
                    if self.b.get(self.i) == Some(&b'}') {
                        self.i += 1;
                        return Ok(Json::Obj(m));
                    }
                    let Json::Str(k) = self.value()? else {
                        return self.err("object key must be a string");
                    };
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') {
                        return self.err("expected ':'");
                    }
                    self.i += 1;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {}
                        _ => return self.err("expected ',' or '}'"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                loop {
                    self.ws();
                    if self.b.get(self.i) == Some(&b']') {
                        self.i += 1;
                        return Ok(Json::Arr(v));
                    }
                    v.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {}
                        _ => return self.err("expected ',' or ']'"),
                    }
                }
            }
            Some(b'"') => {
                self.i += 1;
                let mut s = String::new();
                loop {
                    let Some(&c) = self.b.get(self.i) else {
                        return self.err("unterminated string");
                    };
                    self.i += 1;
                    match c {
                        b'"' => return Ok(Json::Str(s)),
                        b'\\' => {
                            let Some(&e) = self.b.get(self.i) else {
                                return self.err("bad escape");
                            };
                            self.i += 1;
                            match e {
                                b'"' => s.push('"'),
                                b'\\' => s.push('\\'),
                                b'/' => s.push('/'),
                                b'n' => s.push('\n'),
                                b'r' => s.push('\r'),
                                b't' => s.push('\t'),
                                b'b' => s.push('\u{8}'),
                                b'f' => s.push('\u{c}'),
                                b'u' => {
                                    let hex = self.b.get(self.i..self.i + 4).ok_or_else(|| {
                                        Error::Malformed("manifest JSON: short \\u".into())
                                    })?;
                                    let cp = u32::from_str_radix(
                                        std::str::from_utf8(hex).unwrap_or("zz"),
                                        16,
                                    )
                                    .map_err(|_| {
                                        Error::Malformed("manifest JSON: bad \\u".into())
                                    })?;
                                    self.i += 4;
                                    s.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                                }
                                _ => return self.err("bad escape"),
                            }
                        }
                        _ => {
                            // Copy one UTF-8 scalar.
                            let start = self.i - 1;
                            let len = utf8_len(c);
                            let bytes = self.b.get(start..start + len).ok_or_else(|| {
                                Error::Malformed("manifest JSON: truncated UTF-8".into())
                            })?;
                            let text = std::str::from_utf8(bytes).map_err(|_| {
                                Error::Malformed("manifest JSON: invalid UTF-8".into())
                            })?;
                            s.push_str(text);
                            self.i = start + len;
                        }
                    }
                }
            }
            Some(b't') if self.b[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(Json::Bool(true))
            }
            Some(b'f') if self.b[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(Json::Bool(false))
            }
            Some(b'n') if self.b[self.i..].starts_with(b"null") => {
                self.i += 4;
                Ok(Json::Null)
            }
            Some(_) => {
                let start = self.i;
                while self.i < self.b.len()
                    && matches!(
                        self.b[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                let text = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("");
                text.parse::<f64>()
                    .map(Json::Num)
                    .map_err(|_| Error::Malformed(format!("manifest JSON: bad number {text:?}")))
            }
        }
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

fn get<'a>(o: &'a BTreeMap<String, Json>, k: &str) -> Result<&'a Json, Error> {
    o.get(k)
        .ok_or_else(|| Error::Malformed(format!("manifest: missing field {k:?}")))
}

fn get_str(o: &BTreeMap<String, Json>, k: &str) -> Result<String, Error> {
    match get(o, k)? {
        Json::Str(s) => Ok(s.clone()),
        _ => Err(Error::Malformed(format!(
            "manifest: {k:?} must be a string"
        ))),
    }
}

fn get_u64(o: &BTreeMap<String, Json>, k: &str) -> Result<u64, Error> {
    match get(o, k)? {
        Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Ok(*n as u64),
        _ => Err(Error::Malformed(format!(
            "manifest: {k:?} must be a non-negative integer"
        ))),
    }
}

fn expect_keys(o: &BTreeMap<String, Json>, allowed: &[&str], what: &str) -> Result<(), Error> {
    for k in o.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Error::Malformed(format!(
                "manifest: unknown field {k:?} in {what}"
            )));
        }
    }
    Ok(())
}

impl Manifest {
    pub fn from_json(text: &str) -> Result<Manifest, Error> {
        let mut p = Parser {
            b: text.as_bytes(),
            i: 0,
        };
        let root = p.value()?;
        p.ws();
        if p.i != p.b.len() {
            return p.err("trailing data");
        }
        let Json::Obj(root) = root else {
            return Err(Error::Malformed("manifest: root must be an object".into()));
        };
        expect_keys(&root, &["schema_version", "resources"], "manifest")?;
        let schema_version = get_u64(&root, "schema_version")? as u32;
        if schema_version != 1 {
            return Err(Error::Unsupported(format!(
                "manifest schema_version {schema_version}"
            )));
        }
        let Json::Arr(items) = get(&root, "resources")? else {
            return Err(Error::Malformed(
                "manifest: resources must be an array".into(),
            ));
        };
        let mut resources = Vec::with_capacity(items.len());
        for item in items {
            let Json::Obj(e) = item else {
                return Err(Error::Malformed("manifest: entry must be an object".into()));
            };
            expect_keys(e, &["font", "path", "license"], "entry")?;
            let Json::Obj(f) = get(e, "font")? else {
                return Err(Error::Malformed("manifest: font must be an object".into()));
            };
            expect_keys(
                f,
                &[
                    "font_id",
                    "sha256",
                    "byte_length",
                    "format",
                    "face_index",
                    "units_per_em",
                    "glyph_count",
                    "postscript_name",
                ],
                "font",
            )?;
            let Json::Obj(l) = get(e, "license")? else {
                return Err(Error::Malformed(
                    "manifest: license must be an object".into(),
                ));
            };
            expect_keys(
                l,
                &[
                    "identifier",
                    "copyright",
                    "source",
                    "text_path",
                    "text_sha256",
                    "embedding_permission",
                ],
                "license",
            )?;
            let perm = get_str(l, "embedding_permission")?;
            resources.push(ManifestEntry {
                font: FontDescriptor {
                    font_id: get_str(f, "font_id")?,
                    sha256: get_str(f, "sha256")?,
                    byte_length: get_u64(f, "byte_length")?,
                    format: get_str(f, "format")?,
                    face_index: get_u64(f, "face_index")? as u32,
                    units_per_em: get_u64(f, "units_per_em")? as u32,
                    glyph_count: get_u64(f, "glyph_count")? as u32,
                    postscript_name: get_str(f, "postscript_name")?,
                },
                path: get_str(e, "path")?,
                license: LicenseMetadata {
                    identifier: get_str(l, "identifier")?,
                    copyright: get_str(l, "copyright")?,
                    source: get_str(l, "source")?,
                    text_path: get_str(l, "text_path")?,
                    text_sha256: get_str(l, "text_sha256")?,
                    embedding_permission: EmbeddingPermission::parse(&perm).ok_or_else(|| {
                        Error::Malformed(format!("manifest: embedding_permission {perm:?}"))
                    })?,
                },
            });
        }
        Ok(Manifest {
            schema_version,
            resources,
        })
    }
}

// ---------------------------------------------------------------- loading

/// A loaded, verified pinned font.
#[derive(Debug, Clone)]
pub struct PinnedFont {
    pub entry: ManifestEntry,
    pub face: TrueTypeFace,
}

/// Every manifest entry loaded and verified, keyed by `font_id`.
#[derive(Debug, Clone)]
pub struct PinnedFontSet {
    fonts: BTreeMap<String, PinnedFont>,
    license_texts: BTreeMap<String, Vec<u8>>,
}

fn relative_path(rel: &str) -> Result<PathBuf, Error> {
    let p = Path::new(rel);
    if p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::Io(format!(
            "manifest path {rel:?} must be relative without '..'"
        )));
    }
    Ok(p.to_path_buf())
}

impl PinnedFontSet {
    /// Loads every entry: font bytes from `font_root/<path>`, licence text from
    /// `license_root/<text_path>`. Any mismatch is an error; nothing partial.
    pub fn load(
        manifest: &Manifest,
        font_root: &Path,
        license_root: &Path,
    ) -> Result<PinnedFontSet, Error> {
        if manifest.schema_version != 1 {
            return Err(Error::Unsupported(format!(
                "manifest schema_version {}",
                manifest.schema_version
            )));
        }
        let mut fonts = BTreeMap::new();
        let mut license_texts = BTreeMap::new();
        for e in &manifest.resources {
            if fonts.contains_key(&e.font.font_id) {
                return Err(Error::Malformed(format!(
                    "duplicate font_id {:?}",
                    e.font.font_id
                )));
            }
            let path = font_root.join(relative_path(&e.path)?);
            let bytes = std::fs::read(&path)
                .map_err(|err| Error::Io(format!("{}: {err}", path.display())))?;
            verify_descriptor(&e.font, &bytes)?;
            let face = TrueTypeFace::parse_with_source(
                bytes,
                FontSource::File {
                    path: path.clone(),
                    face_index: e.font.face_index,
                },
            )?;
            check_parsed(&e.font, &face)?;
            let lic_path = license_root.join(relative_path(&e.license.text_path)?);
            let text = match license_texts.get(&e.license.text_path) {
                Some(t) => Vec::clone(t),
                None => std::fs::read(&lic_path)
                    .map_err(|err| Error::Io(format!("{}: {err}", lic_path.display())))?,
            };
            let got = crate::sha256::hex(&crate::sha256::digest(&text));
            if got != e.license.text_sha256 {
                return Err(Error::Malformed(format!(
                    "licence text {} sha256 {got} != declared {}",
                    e.license.text_path, e.license.text_sha256
                )));
            }
            license_texts.insert(e.license.text_path.clone(), text);
            fonts.insert(
                e.font.font_id.clone(),
                PinnedFont {
                    entry: e.clone(),
                    face,
                },
            );
        }
        Ok(PinnedFontSet {
            fonts,
            license_texts,
        })
    }

    pub fn get(&self, font_id: &str) -> Option<&PinnedFont> {
        self.fonts.get(font_id)
    }

    pub fn face(&self, font_id: &str) -> Option<&TrueTypeFace> {
        self.fonts.get(font_id).map(|f| &f.face)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.fonts.keys().map(String::as_str)
    }

    pub fn license_text(&self, text_path: &str) -> Option<&[u8]> {
        self.license_texts.get(text_path).map(Vec::as_slice)
    }
}

/// Byte-level checks shared with `font-resources`' loader semantics.
pub fn verify_descriptor(d: &FontDescriptor, bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() as u64 != d.byte_length {
        return Err(Error::Malformed(format!(
            "{}: byte_length {} != declared {}",
            d.font_id,
            bytes.len(),
            d.byte_length
        )));
    }
    let got = crate::sha256::hex(&crate::sha256::digest(bytes));
    if got != d.sha256 {
        return Err(Error::Malformed(format!(
            "{}: sha256 {got} != declared {}",
            d.font_id, d.sha256
        )));
    }
    Ok(())
}

fn check_parsed(d: &FontDescriptor, face: &TrueTypeFace) -> Result<(), Error> {
    let format = match face.outlines() {
        Outlines::Glyf => "static-truetype",
        Outlines::Cff => "opentype-cff",
    };
    if d.format != format {
        return Err(Error::Malformed(format!(
            "{}: format {:?} but program is {format}",
            d.font_id, d.format
        )));
    }
    if u32::from(face.units_per_em()) != d.units_per_em {
        return Err(Error::Malformed(format!(
            "{}: units_per_em {} != declared {}",
            d.font_id,
            face.units_per_em(),
            d.units_per_em
        )));
    }
    if u32::from(face.num_glyphs()) != d.glyph_count {
        return Err(Error::Malformed(format!(
            "{}: glyph_count {} != declared {}",
            d.font_id,
            face.num_glyphs(),
            d.glyph_count
        )));
    }
    if face.postscript_name() != d.postscript_name {
        return Err(Error::Malformed(format!(
            "{}: postscript_name {:?} != declared {:?}",
            d.font_id,
            face.postscript_name(),
            d.postscript_name
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trips_and_is_stable() {
        let m = pinned_latin_modern();
        let text = m.to_json();
        let back = Manifest::from_json(&text).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.to_json(), text);
        assert!(Manifest::from_json("{\"schema_version\": 2, \"resources\": []}").is_err());
        assert!(
            Manifest::from_json("{\"schema_version\": 1, \"resources\": [], \"x\": 1}").is_err()
        );
    }

    #[test]
    fn manifest_paths_are_bounded() {
        assert!(relative_path("lm/a.otf").is_ok());
        assert!(relative_path("../a.otf").is_err());
        assert!(relative_path("/etc/a.otf").is_err());
    }
}

// ------------------------------------------------ TFM encoding manifest

/// Mirrors `flashtex_font_resources::encoding::EncodingManifest`
/// (`fixtures/tfm/ec-lmr10.encoding.json`). Read-only here; the binding
/// logic lives in font-resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingManifest {
    pub tfm_sha256: String,
    pub font_sha256: String,
    pub face_index: u32,
    /// (code, glyph name)
    pub encoding: Vec<(u8, String)>,
    /// (glyph name, original glyph id)
    pub declared_glyphs: Vec<(String, u16)>,
}

impl EncodingManifest {
    pub fn from_json(text: &str) -> Result<EncodingManifest, Error> {
        let mut p = Parser {
            b: text.as_bytes(),
            i: 0,
        };
        let Json::Obj(root) = p.value()? else {
            return Err(Error::Malformed("encoding manifest: root".into()));
        };
        expect_keys(
            &root,
            &[
                "tfm_sha256",
                "font_sha256",
                "face_index",
                "encoding",
                "declared_glyphs",
            ],
            "encoding manifest",
        )?;
        let mut encoding = Vec::new();
        let Json::Arr(items) = get(&root, "encoding")? else {
            return Err(Error::Malformed("encoding must be an array".into()));
        };
        for it in items {
            let Json::Obj(o) = it else {
                return Err(Error::Malformed("encoding entry".into()));
            };
            expect_keys(o, &["code", "glyph_name"], "encoding entry")?;
            let code = get_u64(o, "code")?;
            if code > 255 {
                return Err(Error::Malformed(format!("encoding code {code} > 255")));
            }
            encoding.push((code as u8, get_str(o, "glyph_name")?));
        }
        let mut declared_glyphs = Vec::new();
        let Json::Arr(items) = get(&root, "declared_glyphs")? else {
            return Err(Error::Malformed("declared_glyphs must be an array".into()));
        };
        for it in items {
            let Json::Obj(o) = it else {
                return Err(Error::Malformed("declared glyph entry".into()));
            };
            expect_keys(o, &["glyph_name", "glyph_id"], "declared glyph")?;
            let gid = get_u64(o, "glyph_id")?;
            if gid > 0xFFFF {
                return Err(Error::Malformed(format!("glyph_id {gid} > 65535")));
            }
            declared_glyphs.push((get_str(o, "glyph_name")?, gid as u16));
        }
        Ok(EncodingManifest {
            tfm_sha256: get_str(&root, "tfm_sha256")?,
            font_sha256: get_str(&root, "font_sha256")?,
            face_index: get_u64(&root, "face_index")? as u32,
            encoding,
            declared_glyphs,
        })
    }

    pub fn glyph_id_of_name(&self, name: &str) -> Option<u16> {
        self.declared_glyphs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, g)| *g)
    }

    pub fn name_of_code(&self, code: u8) -> Option<&str> {
        self.encoding
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, n)| n.as_str())
    }
}
