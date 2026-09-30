//! `FONT`, `IMAGE` and `SOURCES` bodies (spec §5).

use crate::frame::{Cursor, Put};
use crate::json::Json;

/// A font resource: one PDF font object (pdfTeX's `/F<n>`), i.e. a font
/// program and an encoding. Sizes are not part of it: every glyph's matrix
/// carries its size, as the PDF's `Tf` does.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// The id glyphs use (pdfTeX's internal font number n of `/F<n>`),
    /// valid for one compile.
    pub id: u16,
    /// SHA-256 of the program, the encoding and the transform: equal keys
    /// draw identically, in any compile or session.
    pub key: [u8; 32],
    /// Descriptive fields (spec §5.1): `pdf_name`, `tex_name`, `ps_name`,
    /// `format`, `file`, `encoding`, `slant`, `extend`, ...
    pub info: Json,
    /// The font program as the font file holds it (e.g. a `.pfb`), or empty
    /// when `format` is `none` (not embedded: a PDF viewer's base font).
    pub program: Vec<u8>,
}

impl Font {
    pub fn encode(&self) -> Vec<u8> {
        let j = self.info.to_string();
        let mut o = Vec::with_capacity(48 + j.len() + self.program.len());
        o.put_u32(self.id as u32);
        o.extend_from_slice(&self.key);
        o.put_u32(j.len() as u32);
        o.extend_from_slice(j.as_bytes());
        o.put_u32(self.program.len() as u32);
        o.extend_from_slice(&self.program);
        o
    }

    pub fn decode(body: &[u8]) -> Result<Font, String> {
        let mut c = Cursor::new(body);
        let id = c.u32()?;
        if id > u16::MAX as u32 {
            return Err(format!("font id {id} out of range"));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(c.take(32)?);
        let jl = c.u32()? as usize;
        let j = std::str::from_utf8(c.take(jl)?).map_err(|e| e.to_string())?;
        let info = Json::parse(j)?;
        let pl = c.u32()? as usize;
        let program = c.take(pl)?.to_vec();
        Ok(Font {
            id: id as u16,
            key,
            info,
            program,
        })
    }

    /// The glyph name of `code` (the encoding), if the font has one.
    pub fn glyph_name(&self, code: u16) -> Option<&str> {
        self.info
            .get("encoding")?
            .as_array()?
            .get(code as usize)?
            .as_str()
    }
}

/// One glyph of a `type3` font's program (spec §5.1.1): the image mask
/// pdfTeX's glyph procedure draws. In glyph space (pixels of the bitmap,
/// y up) the mask fills the rectangle from (`llx`, `lly`) to (`llx` +
/// `width`, `lly` + `height`); `rows` are its `height` rows, top row first,
/// each `(width + 7) / 8` bytes, most significant bit first; a 1 bit is
/// ink (the PDF's `/ImageMask true /Decode [1 0]`). `width == 0`: a glyph
/// with no ink.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type3Glyph {
    pub code: u8,
    pub llx: i32,
    pub lly: i32,
    pub width: u32,
    pub height: u32,
    pub rows: Vec<u8>,
}

impl Type3Glyph {
    /// Bytes per row.
    pub fn stride(&self) -> usize {
        (self.width as usize).div_ceil(8)
    }

    /// Whether pixel (`x`, `y`) is ink, `y` counted from the top row.
    pub fn ink(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let b = self.rows[y as usize * self.stride() + x as usize / 8];
        b & (0x80 >> (x % 8)) != 0
    }
}

/// A `type3` font's program (spec §5.1.1): `"T3B1"`, a `u32` count, then
/// per glyph, in ascending code: `u8 code`, `i32 llx`, `i32 lly`,
/// `u32 width`, `u32 height`, the rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Type3Bitmaps {
    pub glyphs: Vec<Type3Glyph>,
}

impl Type3Bitmaps {
    pub const MAGIC: &'static [u8; 4] = b"T3B1";

    pub fn encode(&self) -> Vec<u8> {
        let mut o = Self::MAGIC.to_vec();
        o.put_u32(self.glyphs.len() as u32);
        for g in &self.glyphs {
            o.put_u8(g.code);
            o.put_i32(g.llx);
            o.put_i32(g.lly);
            o.put_u32(g.width);
            o.put_u32(g.height);
            o.extend_from_slice(&g.rows);
        }
        o
    }

    pub fn decode(program: &[u8]) -> Result<Type3Bitmaps, String> {
        let mut c = Cursor::new(program);
        if c.take(4)? != Self::MAGIC {
            return Err("not a T3B1 program".into());
        }
        let n = c.count(17)?;
        let mut glyphs = Vec::with_capacity(n);
        for _ in 0..n {
            let code = c.u8()?;
            let llx = c.i32()?;
            let lly = c.i32()?;
            let width = c.u32()?;
            let height = c.u32()?;
            let len = (width as usize)
                .div_ceil(8)
                .checked_mul(height as usize)
                .ok_or("glyph size overflows")?;
            let rows = c.take(len)?.to_vec();
            glyphs.push(Type3Glyph {
                code,
                llx,
                lly,
                width,
                height,
                rows,
            });
        }
        if c.left() != 0 {
            return Err(format!("{} bytes after the last glyph", c.left()));
        }
        Ok(Type3Bitmaps { glyphs })
    }

    /// The glyph of `code`, if the font has one.
    pub fn glyph(&self, code: u8) -> Option<&Type3Glyph> {
        self.glyphs.iter().find(|g| g.code == code)
    }
}

/// Source files and spans (spec §5.3): `{"files": [[id, path], ...],
/// "spans": [[id, file, line], ...]}`, each entry sent once per compile,
/// before the first page that uses it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sources {
    pub files: Vec<(u32, String)>,
    pub spans: Vec<(u32, u32, u32)>,
}

impl Sources {
    pub fn to_json(&self) -> Json {
        Json::Obj(vec![
            (
                "files".into(),
                Json::Arr(
                    self.files
                        .iter()
                        .map(|(i, p)| Json::Arr(vec![Json::Int(*i as i64), Json::Str(p.clone())]))
                        .collect(),
                ),
            ),
            (
                "spans".into(),
                Json::Arr(
                    self.spans
                        .iter()
                        .map(|(i, f, l)| {
                            Json::Arr(vec![
                                Json::Int(*i as i64),
                                Json::Int(*f as i64),
                                Json::Int(*l as i64),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }

    pub fn from_json(j: &Json) -> Result<Sources, String> {
        let mut s = Sources::default();
        let int = |v: &Json| {
            v.as_i64()
                .filter(|i| (0..=u32::MAX as i64).contains(i))
                .map(|i| i as u32)
        };
        for f in j.get("files").and_then(Json::as_array).unwrap_or(&[]) {
            let a = f.as_array().ok_or("files entry is not an array")?;
            match a {
                [i, p] => s.files.push((
                    int(i).ok_or("bad file id")?,
                    p.as_str().ok_or("bad path")?.to_string(),
                )),
                _ => return Err("files entry is not [id, path]".into()),
            }
        }
        for e in j.get("spans").and_then(Json::as_array).unwrap_or(&[]) {
            let a = e.as_array().ok_or("spans entry is not an array")?;
            match a {
                [i, f, l] => s.spans.push((
                    int(i).ok_or("bad span id")?,
                    int(f).ok_or("bad span file")?,
                    int(l).ok_or("bad span line")?,
                )),
                _ => return Err("spans entry is not [id, file, line]".into()),
            }
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type3_bitmaps_round_trip() {
        let t = Type3Bitmaps {
            glyphs: vec![
                Type3Glyph {
                    code: 65,
                    llx: -1,
                    lly: -2,
                    width: 9,
                    height: 2,
                    rows: vec![0b1000_0000, 0b1000_0000, 0b0100_0000, 0],
                },
                Type3Glyph {
                    code: 66,
                    llx: 0,
                    lly: 0,
                    width: 0,
                    height: 0,
                    rows: vec![],
                },
            ],
        };
        let p = t.encode();
        assert_eq!(&p[..4], b"T3B1");
        let d = Type3Bitmaps::decode(&p).unwrap();
        assert_eq!(d, t);
        let a = d.glyph(65).unwrap();
        assert_eq!(a.stride(), 2);
        assert!(a.ink(0, 0) && a.ink(8, 0) && a.ink(1, 1) && !a.ink(0, 1));
        assert!(Type3Bitmaps::decode(&p[..p.len() - 1]).is_err());
        assert!(Type3Bitmaps::decode(b"T3B1\xff\xff\xff\xff").is_err());
    }
}
