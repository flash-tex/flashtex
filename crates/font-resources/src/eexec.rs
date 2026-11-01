//! Passive Adobe Type1 binary-eexec byte inspection; never PostScript execution.
use crate::{
    pfb::{Identity, Resource, SegmentKind},
    sha256, MAX_FONT_BYTES,
};
use sha2::{Digest, Sha256};
use std::{ops::Range, sync::Arc};
pub const EEXEC_SEED: u16 = 55665;
pub const RANDOM_PREFIX_BYTES: usize = 4;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    OutputLimit,
    TruncatedPrefix,
    UnsupportedBinaryPrefix,
}
pub struct Inspection<'a> {
    resource: &'a Resource,
    encrypted_ranges: Vec<Range<usize>>,
    encrypted_sha256: String,
    random_prefix: [u8; 4],
    plaintext: Arc<[u8]>,
    plaintext_sha256: String,
}
impl Inspection<'_> {
    pub fn resource_identity(&self) -> &Identity {
        self.resource.identity()
    }
    pub fn encrypted_ranges(&self) -> &[Range<usize>] {
        &self.encrypted_ranges
    }
    pub fn encrypted_sha256(&self) -> &str {
        &self.encrypted_sha256
    }
    /// Decrypted four random prefix bytes, excluded from plaintext and its hash.
    pub fn random_prefix(&self) -> [u8; 4] {
        self.random_prefix
    }
    pub fn plaintext(&self) -> &[u8] {
        &self.plaintext
    }
    pub fn plaintext_sha256(&self) -> &str {
        &self.plaintext_sha256
    }
}
/// Explicit caller assertion that the PFB binary records contain binary eexec.
/// Container validation alone cannot establish this PostScript-level meaning.
/// State spans consecutive binary records; ASCII records are never decrypted.
/// Fixed seed55665 and exactly4 prefix bytes, not the charstring seed/lenIV policy.
pub fn inspect_binary_eexec(
    resource: &Resource,
    max_plaintext_bytes: usize,
) -> Result<Inspection<'_>, Error> {
    if max_plaintext_bytes > MAX_FONT_BYTES {
        return Err(Error::OutputLimit);
    }
    let ranges: Vec<_> = resource
        .segments()
        .iter()
        .filter(|s| s.kind == SegmentKind::Binary)
        .map(|s| s.payload.clone())
        .collect();
    let total = ranges
        .iter()
        .try_fold(0usize, |n, r| n.checked_add(r.len()))
        .ok_or(Error::OutputLimit)?;
    if total < RANDOM_PREFIX_BYTES {
        return Err(Error::TruncatedPrefix);
    }
    if total - RANDOM_PREFIX_BYTES > max_plaintext_bytes {
        return Err(Error::OutputLimit);
    }
    let ciphertext = || {
        ranges
            .iter()
            .flat_map(|r| resource.bytes()[r.clone()].iter().copied())
    };
    let first: Vec<_> = ciphertext().take(4).collect();
    if matches!(first[0], b' ' | b'\t' | b'\r' | b'\n') || first.iter().all(u8::is_ascii_hexdigit) {
        return Err(Error::UnsupportedBinaryPrefix);
    }
    let mut random_prefix = [0; 4];
    let mut plain = Vec::with_capacity(total - 4);
    let mut seed = EEXEC_SEED;
    let mut digest = Sha256::new();
    for (index, cipher) in ciphertext().enumerate() {
        digest.update([cipher]);
        let value = decrypt_byte(cipher, &mut seed);
        if index < 4 {
            random_prefix[index] = value
        } else {
            plain.push(value)
        }
    }
    let plaintext_sha256 = sha256(&plain);
    Ok(Inspection {
        resource,
        encrypted_ranges: ranges,
        encrypted_sha256: format!("{:x}", digest.finalize()),
        random_prefix,
        plaintext: Arc::from(plain),
        plaintext_sha256,
    })
}
pub(crate) fn decrypt_byte(cipher: u8, seed: &mut u16) -> u8 {
    let value = cipher ^ (*seed >> 8) as u8;
    *seed = u16::from(cipher)
        .wrapping_add(*seed)
        .wrapping_mul(52845)
        .wrapping_add(22719);
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    fn resource(chunks: &[&[u8]]) -> Resource {
        fn segment(kind: u8, b: &[u8], out: &mut Vec<u8>) {
            out.extend([128, kind]);
            out.extend((b.len() as u32).to_le_bytes());
            out.extend(b)
        }
        let mut b = Vec::new();
        segment(1, b"explicit synthetic header", &mut b);
        for c in chunks {
            segment(2, c, &mut b)
        }
        segment(1, b"trailer", &mut b);
        b.extend([128, 3]);
        let license = b"synthetic license";
        let identity = Identity {
            resource_id: "synthetic".into(),
            sha256: sha256(&b),
            byte_length: b.len() as u64,
            license: crate::LicenseMetadata {
                identifier: "LicenseRef-test".into(),
                copyright: "test".into(),
                source: "original vector".into(),
                text_path: "LICENSE".into(),
                text_sha256: sha256(license),
                embedding_permission: crate::EmbeddingPermission::Unknown,
            },
        };
        Resource::from_bytes(&identity, &b, license).unwrap()
    }
    #[test]
    fn fixed_known_vector_and_split_record_equivalence() {
        let hex = "d9d73f4a50b9fa428d59a36bd8f46f9cb9c47adfb0f3297e";
        let bytes: Vec<_> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        for split in 1..bytes.len() {
            let r = resource(&[&bytes[..split], &bytes[split..]]);
            let out = inspect_binary_eexec(&r, 20).unwrap();
            assert_eq!(out.random_prefix(), [0, 1, 2, 3]);
            assert_eq!(out.plaintext(), b"/Private 1 dict def\n");
            assert_eq!(
                out.plaintext_sha256(),
                "5cebe2e70121905b7678ddb52f9cdddc2d2af251ed5b5801fa3fc8012a2ee0e5"
            );
            assert_eq!(out.encrypted_sha256(), sha256(&bytes));
            assert_eq!(out.resource_identity(), r.identity());
            assert_eq!(out.encrypted_ranges().len(), 2);
            assert!(matches!(
                inspect_binary_eexec(&r, 19),
                Err(Error::OutputLimit)
            ));
        }
    }
    #[test]
    fn malformed_short_ambiguous_and_budget_are_explicit() {
        assert!(matches!(
            inspect_binary_eexec(&resource(&[b"abc"]), 10),
            Err(Error::TruncatedPrefix)
        ));
        for bytes in [b"abcd".as_slice(), b" 123", b"\nxyz"] {
            assert!(matches!(
                inspect_binary_eexec(&resource(&[bytes]), 10),
                Err(Error::UnsupportedBinaryPrefix)
            ))
        }
        assert!(matches!(
            inspect_binary_eexec(&resource(&[b"xyz!"]), MAX_FONT_BYTES + 1),
            Err(Error::OutputLimit)
        ));
    }
}
