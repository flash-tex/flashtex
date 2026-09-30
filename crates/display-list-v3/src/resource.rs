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
