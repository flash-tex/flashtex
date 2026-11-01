use super::{Cff, HintPolicy, MatrixOutline};
use crate::{invalid, Error, Result, MAX_FONT_BYTES};
use std::{collections::BTreeMap, ops::Range, sync::Arc};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CffIdentity {
    pub font_sha256: String,
    pub cff_sha256: String,
    pub face_index: u32,
    pub table_range: Range<usize>,
}
#[derive(Debug, Clone, Copy)]
pub struct CacheLimits {
    pub max_entries: usize,
    pub max_bytes: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    Hit,
    Stored,
    BypassedOversize,
}
#[derive(Debug, Clone)]
pub struct CacheOutcome {
    pub result: Result<Arc<MatrixOutline>>,
    pub status: CacheStatus,
}
struct Entry {
    value: Result<Arc<MatrixOutline>>,
    bytes: usize,
    last_used: u64,
}
pub struct CffOutlineCache {
    identity: CffIdentity,
    cff: Cff,
    limits: CacheLimits,
    entries: BTreeMap<(u16, HintPolicy), Entry>,
    bytes: usize,
    clock: u64,
    names: std::sync::OnceLock<Result<Arc<super::CffGlyphNames>>>,
}
impl CffOutlineCache {
    /// Table range must come from the caller's validated OpenType accessor. This
    /// constructor verifies byte containment/identity, not sfnt directory semantics.
    pub fn from_font_table(
        font_bytes: &[u8],
        face_index: u32,
        range: Range<usize>,
        limits: CacheLimits,
    ) -> Result<Self> {
        if font_bytes.len() > MAX_FONT_BYTES
            || limits.max_entries > 4096
            || limits.max_bytes > 256 * 1024 * 1024
        {
            return Err(invalid("CFF cache resource limit"));
        }
        if face_index != 0 || font_bytes.get(..4) != Some(b"OTTO") {
            return Err(crate::Error::UnsupportedFont(
                "CFF cache requires single-face OTTO font".into(),
            ));
        }
        let table = font_bytes
            .get(range.clone())
            .ok_or_else(|| invalid("CFF cache table range"))?;
        let cff = Cff::parse(table)?;
        Ok(Self {
            identity: CffIdentity {
                font_sha256: crate::sha256(font_bytes),
                cff_sha256: cff.sha256.clone(),
                face_index,
                table_range: range,
            },
            cff,
            limits,
            entries: BTreeMap::new(),
            bytes: 0,
            clock: 0,
            names: std::sync::OnceLock::new(),
        })
    }
    pub fn glyph_count(&self) -> usize {
        self.cff.glyph_count()
    }
    pub fn glyph_names(&self) -> Result<Arc<super::CffGlyphNames>> {
        self.names
            .get_or_init(|| self.cff.glyph_names().map(Arc::new))
            .clone()
    }
    pub fn identity(&self) -> &CffIdentity {
        &self.identity
    }
    pub fn retained_bytes(&self) -> usize {
        self.bytes
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    pub fn lookup(&mut self, gid: u16, policy: HintPolicy) -> CacheOutcome {
        if self.clock == u64::MAX {
            self.clear();
            self.clock = 0;
        }
        self.clock += 1;
        let key = (gid, policy);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.clock;
            return CacheOutcome {
                result: entry.value.clone(),
                status: CacheStatus::Hit,
            };
        }
        let value = self.cff.matrix_outline(gid, policy).map(Arc::new);
        let bytes = retained_size(&value);
        if self.limits.max_entries == 0 || bytes > self.limits.max_bytes {
            return CacheOutcome {
                result: value,
                status: CacheStatus::BypassedOversize,
            };
        }
        while self.entries.len() >= self.limits.max_entries
            || self.bytes > self.limits.max_bytes - bytes
        {
            let key = *self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .unwrap()
                .0;
            self.bytes -= self.entries.remove(&key).unwrap().bytes;
        }
        self.entries.insert(
            key,
            Entry {
                value: value.clone(),
                bytes,
                last_used: self.clock,
            },
        );
        self.bytes += bytes;
        CacheOutcome {
            result: value,
            status: CacheStatus::Stored,
        }
    }
}
fn retained_size(value: &Result<Arc<MatrixOutline>>) -> usize {
    // Conservative fixed charge covers Arc counters, key, Entry and tree slot.
    let base = 512;
    match value {
        Err(error) => {
            base + std::mem::size_of::<Error>()
                + match error {
                    Error::InvalidManifest(s)
                    | Error::MissingResource(s)
                    | Error::MissingLicense(s)
                    | Error::Io(s)
                    | Error::UnsupportedFont(s)
                    | Error::InvalidFont(s) => s.capacity(),
                    _ => 0,
                }
        }
        Ok(out) => {
            base + std::mem::size_of::<MatrixOutline>()
                + out.cff_sha256.capacity()
                + out.commands.capacity() * std::mem::size_of::<super::MatrixCommand>()
                + out.hints.stems.capacity() * std::mem::size_of::<super::StemHint>()
                + out.hints.masks.capacity() * std::mem::size_of::<super::HintMask>()
                + out.hints.flex_depths.capacity() * std::mem::size_of::<crate::Coordinate>()
                + out
                    .hints
                    .masks
                    .iter()
                    .map(|m| m.bytes.capacity())
                    .sum::<usize>()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn font() -> Vec<u8> {
        let mut bytes = b"OTTOxxxx".to_vec();
        bytes.extend(super::super::tests::fixture());
        bytes
    }
    fn cache(bytes: &[u8], entries: usize, budget: usize) -> CffOutlineCache {
        CffOutlineCache::from_font_table(
            bytes,
            0,
            8..bytes.len(),
            CacheLimits {
                max_entries: entries,
                max_bytes: budget,
            },
        )
        .unwrap()
    }
    #[test]
    fn immutable_identity_shared_arc_and_policy_separation() {
        let mut bytes = font();
        let mut cache = cache(&bytes, 3, 10000);
        let sha = cache.identity().font_sha256.clone();
        bytes[4] = b'z';
        assert_ne!(sha, crate::sha256(&bytes));
        let first = cache.lookup(0, HintPolicy::Unhinted);
        assert_eq!(first.status, CacheStatus::Stored);
        let next = cache.lookup(0, HintPolicy::Unhinted);
        assert_eq!(next.status, CacheStatus::Hit);
        assert!(Arc::ptr_eq(
            first.result.as_ref().unwrap(),
            next.result.as_ref().unwrap()
        ));
        assert_eq!(
            cache.lookup(0, HintPolicy::Reject).status,
            CacheStatus::Stored
        );
    }
    #[test]
    fn byte_entry_caps_eviction_and_negative_hits() {
        let bytes = font();
        let mut cache = cache(&bytes, 1, 10000);
        assert!(cache.lookup(1, HintPolicy::Unhinted).result.is_err());
        assert_eq!(
            cache.lookup(1, HintPolicy::Unhinted).status,
            CacheStatus::Hit
        );
        cache.lookup(0, HintPolicy::Unhinted);
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.lookup(1, HintPolicy::Unhinted).status,
            CacheStatus::Stored
        );
        assert!(cache.retained_bytes() <= 10000);
        let mut cache = super::tests::cache(&bytes, 2, 1);
        assert_eq!(
            cache.lookup(0, HintPolicy::Unhinted).status,
            CacheStatus::BypassedOversize
        );
        assert!(cache.is_empty());
    }
    #[test]
    fn whole_font_identity_not_only_cff_and_bad_ranges() {
        let a = font();
        let mut b = a.clone();
        b[4] = b'z';
        let left = cache(&a, 1, 10000);
        let right = cache(&b, 1, 10000);
        assert_eq!(left.identity().cff_sha256, right.identity().cff_sha256);
        assert_ne!(left.identity().font_sha256, right.identity().font_sha256);
        assert!(CffOutlineCache::from_font_table(
            &a,
            0,
            8..a.len() + 1,
            CacheLimits {
                max_entries: 1,
                max_bytes: 10000
            }
        )
        .is_err());
    }
}
