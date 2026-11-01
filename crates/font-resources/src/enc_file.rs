//! Declarative .enc subset only: /Name [ exactly256 /literalNames ] [readonly] def.
//! Whitespace and %-to-end-of-line comments are allowed. No executable programs,
//! strings, numeric slot assignment, dictionaries, name escaping or evaluation.
use crate::{
    cff::{BoundCffTfmFont, CffEncodingManifest, CffOutlineCache},
    encoding::{BoundTfmFont, EncodingEntry, EncodingManifest, NamedGlyph},
    sha256,
    tfm::Tfm,
    FontResource,
};
use flashtex_project_files::{ProjectPath, ProjectRoot};
pub const MAX_ENC_BYTES: usize = 64 * 1024;
#[derive(Debug)]
pub enum EncError {
    Syntax {
        offset: usize,
        expected: &'static str,
    },
    SizeLimit,
    DigestMismatch,
    Path,
    Missing,
    Io(String),
    Binding(crate::Error),
}
impl From<crate::Error> for EncError {
    fn from(e: crate::Error) -> Self {
        Self::Binding(e)
    }
}
impl std::fmt::Display for EncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EncError {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingSource {
    pub file_sha256: String,
    pub encoding_name: String,
    pub project_path: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphNameAlias {
    pub literal_name: String,
    pub font_name: String,
    pub original_gid: u16,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnavailableSlot {
    pub code: u8,
    pub literal_name: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CffMappingDeclarations {
    pub encoding_file_sha256: String,
    pub font_sha256: String,
    pub cff_sha256: String,
    pub face_index: u32,
    pub aliases: Vec<GlyphNameAlias>,
    pub unavailable_slots: Vec<UnavailableSlot>,
}
pub struct EncFile {
    source: EncodingSource,
    slots: Vec<EncodingEntry>,
}
#[derive(PartialEq)]
enum Token<'a> {
    Name(&'a str),
    Open,
    Close,
    Word(&'a str),
    End,
}
struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}
fn delimiter(b: u8) -> bool {
    b.is_ascii_whitespace() || b"()<>[]{}/%".contains(&b)
}
impl<'a> Lexer<'a> {
    fn error(&self, expected: &'static str) -> EncError {
        EncError::Syntax {
            offset: self.pos,
            expected,
        }
    }
    fn next(&mut self) -> Result<Token<'a>, EncError> {
        loop {
            while self
                .bytes
                .get(self.pos)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.pos += 1
            }
            if self.bytes.get(self.pos) == Some(&b'%') {
                while self
                    .bytes
                    .get(self.pos)
                    .is_some_and(|b| *b != b'\n' && *b != b'\r')
                {
                    self.pos += 1
                }
            } else {
                break;
            }
        }
        let Some(&first) = self.bytes.get(self.pos) else {
            return Ok(Token::End);
        };
        self.pos += 1;
        match first {
            b'[' => Ok(Token::Open),
            b']' => Ok(Token::Close),
            b'/' => {
                let start = self.pos;
                while self.bytes.get(self.pos).is_some_and(|b| !delimiter(*b)) {
                    self.pos += 1
                }
                let name = &self.bytes[start..self.pos];
                if name.is_empty()
                    || name.len() > 256
                    || !name.iter().all(|b| (33..=126).contains(b))
                {
                    return Err(self.error("bounded literal ASCII glyph/name"));
                }
                Ok(Token::Name(std::str::from_utf8(name).unwrap()))
            }
            _ => {
                if delimiter(first) || !(33..=126).contains(&first) {
                    return Err(self.error("literal encoding syntax"));
                }
                let start = self.pos - 1;
                while self.bytes.get(self.pos).is_some_and(|b| !delimiter(*b)) {
                    self.pos += 1
                }
                let word = std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|_| self.error("ASCII operator"))?;
                Ok(Token::Word(word))
            }
        }
    }
}
impl EncFile {
    pub fn parse(bytes: &[u8], expected_sha256: &str) -> Result<Self, EncError> {
        if bytes.len() > MAX_ENC_BYTES {
            return Err(EncError::SizeLimit);
        }
        let digest = sha256(bytes);
        if digest != expected_sha256 {
            return Err(EncError::DigestMismatch);
        }
        let mut lexer = Lexer { bytes, pos: 0 };
        let Token::Name(name) = lexer.next()? else {
            return Err(lexer.error("one literal encoding name"));
        };
        let name = name.to_owned();
        if lexer.next()? != Token::Open {
            return Err(lexer.error("literal [ array"));
        }
        let mut slots = Vec::with_capacity(256);
        for code in 0..256 {
            let Token::Name(name) = lexer.next()? else {
                return Err(lexer.error("exactly256 literal glyph names"));
            };
            slots.push(EncodingEntry {
                code: code as u8,
                glyph_name: name.into(),
            });
        }
        if lexer.next()? != Token::Close {
            return Err(lexer.error("closing ] after256 slots"));
        }
        let mut last = lexer.next()?;
        if last == Token::Word("readonly") {
            last = lexer.next()?;
        }
        if last != Token::Word("def") {
            return Err(lexer.error("terminal def"));
        }
        if lexer.next()? != Token::End {
            return Err(lexer.error("end of single encoding declaration"));
        }
        Ok(Self {
            source: EncodingSource {
                file_sha256: digest,
                encoding_name: name,
                project_path: None,
            },
            slots,
        })
    }
    pub fn load(root: &ProjectRoot, path: &str, expected_sha256: &str) -> Result<Self, EncError> {
        if path.len() > 4096 || path.split('/').any(|p| p == "..") {
            return Err(EncError::Path);
        }
        let normalized = ProjectPath::normalize(path).map_err(|_| EncError::Path)?;
        if normalized.as_str() != path {
            return Err(EncError::Path);
        }
        let file = root
            .read(&normalized, MAX_ENC_BYTES as u64)
            .map_err(|e| EncError::Io(e.to_string()))?
            .ok_or(EncError::Missing)?;
        let mut result = Self::parse(&file.bytes, expected_sha256)?;
        result.source.project_path = Some(path.into());
        Ok(result)
    }
    pub fn source(&self) -> &EncodingSource {
        &self.source
    }
    pub fn slots(&self) -> &[EncodingEntry] {
        &self.slots
    }
    pub fn bind_truetype<'a>(
        &self,
        tfm: &'a Tfm,
        font: &'a FontResource,
        declared_glyphs: &[NamedGlyph],
    ) -> Result<EncodedTrueType<'a>, EncError> {
        if declared_glyphs.len() > 65536 {
            return Err(EncError::SizeLimit);
        }
        let manifest = EncodingManifest {
            tfm_sha256: tfm.source_sha256.clone(),
            font_sha256: font.descriptor().sha256.clone(),
            face_index: font.descriptor().face_index,
            encoding: self.slots.clone(),
            declared_glyphs: declared_glyphs.to_vec(),
        };
        Ok(EncodedTrueType {
            source: self.source.clone(),
            font: BoundTfmFont::new(tfm, font, &manifest)?,
        })
    }
    pub fn bind_cff_declared<'a>(
        &self,
        tfm: &'a Tfm,
        font: &CffOutlineCache,
        declarations: &CffMappingDeclarations,
    ) -> Result<EncodedCff<'a>, EncError> {
        use crate::encoding::GlyphIdentity;
        use std::collections::{BTreeMap, BTreeSet};
        let identity = font.identity();
        if declarations.encoding_file_sha256 != self.source.file_sha256
            || declarations.font_sha256 != identity.font_sha256
            || declarations.cff_sha256 != identity.cff_sha256
            || declarations.face_index != identity.face_index
        {
            return Err(EncError::Binding(crate::invalid(
                "encoding declaration identity mismatch",
            )));
        }
        if declarations.aliases.len() > 256 || declarations.unavailable_slots.len() > 256 {
            return Err(EncError::SizeLimit);
        }
        let names = font.glyph_names()?;
        let mut aliases = BTreeMap::new();
        let mut omitted = BTreeSet::new();
        for alias in &declarations.aliases {
            if alias.literal_name.len() > 256
                || alias.font_name.len() > 256
                || alias.literal_name == ".notdef"
                || alias.font_name == ".notdef"
                || !self
                    .slots
                    .iter()
                    .any(|s| s.glyph_name == alias.literal_name)
                || aliases
                    .insert(alias.literal_name.clone(), alias.font_name.clone())
                    .is_some()
            {
                return Err(EncError::Binding(crate::invalid(
                    "duplicate/unknown alias source",
                )));
            }
            if names.resolve(&alias.font_name)? != GlyphIdentity::Original(alias.original_gid)
                || alias.original_gid == 0
            {
                return Err(EncError::Binding(crate::invalid(
                    "alias target original GID mismatch",
                )));
            }
            if names
                .resolve(&alias.literal_name)
                .is_ok_and(|gid| gid != GlyphIdentity::Original(alias.original_gid))
            {
                return Err(EncError::Binding(crate::invalid(
                    "alias conflicts with existing font name",
                )));
            }
        }
        for unavailable in &declarations.unavailable_slots {
            if self.slots[unavailable.code as usize].glyph_name != unavailable.literal_name
                || !omitted.insert(unavailable.code)
                || aliases.contains_key(&unavailable.literal_name)
                || names.resolve(&unavailable.literal_name).is_ok()
            {
                return Err(EncError::Binding(crate::invalid(
                    "invalid/conflicting unavailable slot",
                )));
            }
        }
        let manifest = CffEncodingManifest {
            font_sha256: identity.font_sha256.clone(),
            cff_sha256: identity.cff_sha256.clone(),
            tfm_sha256: tfm.source_sha256.clone(),
            face_index: identity.face_index,
            encoding: self
                .slots
                .iter()
                .filter(|s| !omitted.contains(&s.code))
                .map(|s| EncodingEntry {
                    code: s.code,
                    glyph_name: aliases.get(&s.glyph_name).unwrap_or(&s.glyph_name).clone(),
                })
                .collect(),
        };
        let bound = BoundCffTfmFont::new(tfm, font, &manifest)?;
        let bytes = serde_json::to_vec(declarations)
            .map_err(|_| EncError::Binding(crate::invalid("encoding declaration serialization")))?;
        let resolved = bound
            .encoding()
            .as_ref()
            .clone()
            .preserve_literal_names(&self.slots, &sha256(&bytes))?;
        Ok(EncodedCff {
            source: self.source.clone(),
            font: BoundCffTfmFont::from_resolved(tfm, std::sync::Arc::new(resolved))?,
            declarations: Some(declarations.clone()),
        })
    }
    pub fn bind_cff<'a>(
        &self,
        tfm: &'a Tfm,
        font: &CffOutlineCache,
    ) -> Result<EncodedCff<'a>, EncError> {
        let identity = font.identity();
        let manifest = CffEncodingManifest {
            tfm_sha256: tfm.source_sha256.clone(),
            font_sha256: identity.font_sha256.clone(),
            cff_sha256: identity.cff_sha256.clone(),
            face_index: identity.face_index,
            encoding: self.slots.clone(),
        };
        Ok(EncodedCff {
            source: self.source.clone(),
            font: BoundCffTfmFont::new(tfm, font, &manifest)?,
            declarations: None,
        })
    }
}
pub struct EncodedTrueType<'a> {
    source: EncodingSource,
    font: BoundTfmFont<'a>,
}
impl<'a> EncodedTrueType<'a> {
    pub fn source(&self) -> &EncodingSource {
        &self.source
    }
    pub fn font(&self) -> &BoundTfmFont<'a> {
        &self.font
    }
}
pub struct EncodedCff<'a> {
    declarations: Option<CffMappingDeclarations>,
    source: EncodingSource,
    font: BoundCffTfmFont<'a>,
}
impl<'a> EncodedCff<'a> {
    pub fn declarations(&self) -> Option<&CffMappingDeclarations> {
        self.declarations.as_ref()
    }
    pub fn source(&self) -> &EncodingSource {
        &self.source
    }
    pub fn font(&self) -> &BoundCffTfmFont<'a> {
        &self.font
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(count: usize) -> String {
        format!(
            "%literal test\r\n/Test[{}] readonly def % end",
            vec!["/.notdef"; count].join("\n")
        )
    }
    #[test]
    fn literal_slots_comments_and_hash() {
        let s = fixture(256);
        let file = EncFile::parse(s.as_bytes(), &sha256(s.as_bytes())).unwrap();
        assert_eq!(file.slots.len(), 256);
        assert_eq!(file.slots[255].code, 255);
        assert_eq!(file.source.encoding_name, "Test");
        assert!(matches!(
            EncFile::parse(s.as_bytes(), "wrong"),
            Err(EncError::DigestMismatch)
        ));
    }
    #[test]
    fn no_executable_programs_or_duplicate_definitions() {
        for s in [
            fixture(255),
            fixture(257),
            format!("{}\n/Second[/.notdef] def", fixture(256)),
            "/E 256 array dup 0 /A put def".into(),
            fixture(256).replace("readonly def", "execute"),
            fixture(256).replace("/.notdef", "(name)"),
            fixture(256).replace("/.notdef", "//A"),
        ] {
            assert!(EncFile::parse(s.as_bytes(), &sha256(s.as_bytes())).is_err())
        }
        let big = vec![b' '; MAX_ENC_BYTES + 1];
        assert!(matches!(
            EncFile::parse(&big, &sha256(&big)),
            Err(EncError::SizeLimit)
        ));
    }
    #[test]
    fn names_are_literal_not_decoded() {
        let s = fixture(256).replacen("/.notdef", "/A#20B", 1);
        let f = EncFile::parse(s.as_bytes(), &sha256(s.as_bytes())).unwrap();
        assert_eq!(f.slots[0].glyph_name, "A#20B");
    }
}
