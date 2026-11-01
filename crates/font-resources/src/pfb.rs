//! Passive PFB framing only. No PostScript, eexec, charstring, glyph or font-name decoding.
use crate::{
    check_license_metadata, sha256, valid_hash, LicenseMetadata, MAX_FONT_BYTES, MAX_LICENSE_BYTES,
};
use std::{ops::Range, sync::Arc};
pub const MAX_SEGMENTS: usize = 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Ascii,
    Binary,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentKind,
    pub header: Range<usize>,
    pub payload: Range<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    SizeLimit,
    SegmentLimit,
    Truncated,
    Marker,
    UnknownType(u8),
    EmptySegment,
    UnsupportedOrder,
    MissingEnd,
    TrailingData,
    Identity,
    License,
    EncryptedOutlinesUnsupported,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub resource_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub license: LicenseMetadata,
}
/// Supported strict profile: one or more ASCII records, one or more binary
/// records, one or more ASCII trailer records, then the two-byte end marker.
/// This is a container profile, not proof its payload is a Type1 font program.
pub fn inspect(bytes: &[u8]) -> Result<Vec<Segment>, Error> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(Error::SizeLimit);
    }
    let mut segments = Vec::new();
    let mut at = 0usize;
    let mut phase = 0;
    loop {
        if at == bytes.len() {
            return Err(Error::MissingEnd);
        }
        if bytes.get(at) != Some(&0x80) {
            return Err(Error::Marker);
        }
        let kind = *bytes.get(at + 1).ok_or(Error::Truncated)?;
        if kind == 3 {
            if at + 2 != bytes.len() {
                return Err(Error::TrailingData);
            }
            if phase != 2 {
                return Err(Error::UnsupportedOrder);
            }
            return Ok(segments);
        }
        if kind != 1 && kind != 2 {
            return Err(Error::UnknownType(kind));
        }
        if segments.len() == MAX_SEGMENTS {
            return Err(Error::SegmentLimit);
        }
        let header_end = at.checked_add(6).ok_or(Error::SizeLimit)?;
        let length = bytes.get(at + 2..header_end).ok_or(Error::Truncated)?;
        let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
        if length == 0 {
            return Err(Error::EmptySegment);
        }
        let end = header_end.checked_add(length).ok_or(Error::SizeLimit)?;
        if end > bytes.len() {
            return Err(Error::Truncated);
        }
        phase = match (phase, kind, segments.is_empty()) {
            (0, 1, _) => 0,
            (0, 2, false) => 1,
            (1, 2, _) => 1,
            (1, 1, _) => 2,
            (2, 1, _) => 2,
            _ => return Err(Error::UnsupportedOrder),
        };
        segments.push(Segment {
            kind: if kind == 1 {
                SegmentKind::Ascii
            } else {
                SegmentKind::Binary
            },
            header: at..header_end,
            payload: header_end..end,
        });
        at = end;
    }
}
pub struct Resource {
    identity: Identity,
    bytes: Arc<[u8]>,
    license: Arc<[u8]>,
    segments: Vec<Segment>,
}
impl Resource {
    pub fn from_bytes(identity: &Identity, bytes: &[u8], license: &[u8]) -> Result<Self, Error> {
        if identity.resource_id.is_empty()
            || identity.resource_id.len() > 256
            || !valid_hash(&identity.sha256)
            || identity.byte_length != bytes.len() as u64
        {
            return Err(Error::Identity);
        }
        if bytes.len() > MAX_FONT_BYTES || license.len() > MAX_LICENSE_BYTES {
            return Err(Error::SizeLimit);
        }
        check_license_metadata(&identity.license).map_err(|_| Error::License)?;
        if identity.license.text_path.is_empty() || identity.license.text_path.len() > 4096 {
            return Err(Error::License);
        }
        if sha256(bytes) != identity.sha256 {
            return Err(Error::Identity);
        }
        if sha256(license) != identity.license.text_sha256 {
            return Err(Error::License);
        }
        let segments = inspect(bytes)?;
        Ok(Self {
            identity: identity.clone(),
            bytes: Arc::from(bytes),
            license: Arc::from(license),
            segments,
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn license_text(&self) -> &[u8] {
        &self.license
    }
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    pub fn require_outlines(&self) -> Result<(), Error> {
        Err(Error::EncryptedOutlinesUnsupported)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn segment(kind: u8, data: &[u8]) -> Vec<u8> {
        let mut b = vec![128, kind];
        b.extend((data.len() as u32).to_le_bytes());
        b.extend(data);
        b
    }
    fn fixture() -> Vec<u8> {
        let mut b = segment(1, b"header");
        b.extend(segment(2, &[0, 128, 3, 255]));
        b.extend(segment(1, b"trailer"));
        b.extend([128, 3]);
        b
    }
    #[test]
    fn framing_and_little_endian_lengths() {
        let b = fixture();
        let s = inspect(&b).unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(&b[s[1].payload.clone()], &[0, 128, 3, 255]);
        for n in 0..b.len() {
            assert!(inspect(&b[..n]).is_err(), "{n}")
        }
        let mut long = segment(1, &vec![b'A'; 256]);
        long.extend(segment(2, b"b"));
        long.extend(segment(1, b"c"));
        long.extend([128, 3]);
        assert_eq!(inspect(&long).unwrap()[0].payload.len(), 256);
    }
    #[test]
    fn order_marker_trailing_and_budgets() {
        let mut b = fixture();
        b.push(0);
        assert_eq!(inspect(&b), Err(Error::TrailingData));
        for bad in [
            vec![0],
            vec![128, 4],
            segment(1, b""),
            vec![128, 3],
            segment(2, b"x"),
        ] {
            assert!(inspect(&bad).is_err())
        }
        let mut b = segment(1, b"a");
        b.extend(segment(2, b"b"));
        b.extend(segment(1, b"c"));
        b.extend(segment(2, b"d"));
        b.extend([128, 3]);
        assert_eq!(inspect(&b), Err(Error::UnsupportedOrder));
        let mut b = vec![];
        for _ in 0..=MAX_SEGMENTS {
            b.extend(segment(1, b"a"))
        }
        assert_eq!(inspect(&b), Err(Error::SegmentLimit));
        assert_eq!(
            inspect(&[128, 1, 255, 255, 255, 255]),
            Err(Error::Truncated)
        );
    }
    #[test]
    fn immutable_full_identity_and_license_gate() {
        let mut bytes = fixture();
        let license = b"synthetic license";
        let id = Identity {
            resource_id: "synthetic".into(),
            sha256: sha256(&bytes),
            byte_length: bytes.len() as u64,
            license: LicenseMetadata {
                identifier: "LicenseRef-test".into(),
                copyright: "test".into(),
                source: "original test fixture".into(),
                text_path: "LICENSE".into(),
                text_sha256: sha256(license),
                embedding_permission: crate::EmbeddingPermission::Unknown,
            },
        };
        let r = Resource::from_bytes(&id, &bytes, license).unwrap();
        bytes[6] = b'Z';
        assert_ne!(r.bytes(), bytes);
        assert_eq!(r.identity(), &id);
        assert_eq!(r.license_text(), license);
        assert_eq!(
            r.require_outlines(),
            Err(Error::EncryptedOutlinesUnsupported)
        );
        assert!(matches!(
            Resource::from_bytes(&id, &bytes, license),
            Err(Error::Identity)
        ));
        assert!(matches!(
            Resource::from_bytes(&id, r.bytes(), b"wrong"),
            Err(Error::License)
        ));
    }
}
