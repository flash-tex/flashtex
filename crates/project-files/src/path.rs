//! Project-relative path normalization.
//!
//! A [`ProjectPath`] is always relative, forward-slash separated, free of `.`
//! and empty segments, and never escapes the project root. This mirrors and
//! tightens the runtime-v1 rule (relative, no parent traversal) and the
//! transfer-v1 rule (no backslashes, colons, NUL).

use std::fmt;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

/// A normalized project-relative path such as `chapters/intro.tex`.
///
/// Equality, ordering and hashing compare the Unicode NFC-normalized form
/// (`key`), not the raw stored bytes (`raw`): two different normalizations
/// of the same on-disk name (e.g. `"café.tex"` with `'é'` precomposed vs.
/// `'e'` + a combining acute accent) are the *same* project-relative
/// identity on a normalization-insensitive filesystem, and must dedupe,
/// cycle-detect and revision-track as one file, not two (issue #45 finding
/// 3). `raw` is what's displayed and used to build OS paths, so filesystem
/// access for any single reference is unaffected by this normalization —
/// only how two different-spelled references compare changes.
#[derive(Clone)]
pub struct ProjectPath {
    raw: String,
    key: String,
}

impl ProjectPath {
    fn from_raw(raw: String) -> Self {
        let key = raw.nfc().collect();
        ProjectPath { raw, key }
    }
}

impl PartialEq for ProjectPath {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for ProjectPath {}

impl PartialOrd for ProjectPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ProjectPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}
impl std::hash::Hash for ProjectPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

/// Why a raw path was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// Empty after normalization (`""`, `"."`, `"./"`).
    Empty,
    /// Starts with `/`, `~`, or a drive prefix such as `C:`.
    Absolute,
    /// `..` segments would leave the project root.
    EscapesRoot,
    /// Contains a backslash, colon, NUL, or control character.
    ForbiddenCharacter(char),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::Empty => write!(f, "path is empty"),
            PathError::Absolute => write!(f, "path must be project-relative, not absolute"),
            PathError::EscapesRoot => write!(f, "path escapes the project root via '..'"),
            PathError::ForbiddenCharacter(c) => {
                write!(f, "path contains forbidden character {c:?}")
            }
        }
    }
}

impl std::error::Error for PathError {}

impl ProjectPath {
    /// Normalizes `raw` against the project root. `.` and empty segments are
    /// dropped; `..` pops the previous segment and is an error if there is
    /// nothing left to pop.
    pub fn normalize(raw: &str) -> Result<Self, PathError> {
        Self::resolve_in("", raw)
    }

    /// Normalizes `raw` relative to `base_dir` (itself a project-relative
    /// directory, `""` for the root). Used when a reference must be resolved
    /// relative to the referencing file's directory rather than the root.
    pub fn resolve_in(base_dir: &str, raw: &str) -> Result<Self, PathError> {
        if let Some(c) = raw
            .chars()
            .find(|c| matches!(c, '\\' | ':' | '\0') || c.is_control())
        {
            return Err(PathError::ForbiddenCharacter(c));
        }
        if raw.starts_with('/') || raw.starts_with('~') {
            return Err(PathError::Absolute);
        }
        let mut segments: Vec<&str> = Vec::new();
        for seg in base_dir.split('/').chain(raw.split('/')) {
            match seg {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(PathError::EscapesRoot);
                    }
                }
                s => segments.push(s),
            }
        }
        if segments.is_empty() {
            return Err(PathError::Empty);
        }
        Ok(ProjectPath::from_raw(segments.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The directory part (`""` for a root-level file).
    pub fn parent_dir(&self) -> &str {
        match self.raw.rfind('/') {
            Some(i) => &self.raw[..i],
            None => "",
        }
    }

    /// The last segment.
    pub fn file_name(&self) -> &str {
        match self.raw.rfind('/') {
            Some(i) => &self.raw[i + 1..],
            None => &self.raw,
        }
    }

    /// The extension of the last segment without the dot, if any.
    pub fn extension(&self) -> Option<&str> {
        let name = self.file_name();
        let dot = name.rfind('.')?;
        if dot == 0 {
            None
        } else {
            Some(&name[dot + 1..])
        }
    }

    /// Returns a path with `ext` appended (`foo` -> `foo.tex`).
    pub fn with_appended_extension(&self, ext: &str) -> ProjectPath {
        ProjectPath::from_raw(format!("{}.{}", self.raw, ext))
    }

    /// Returns `self/leaf` for a `leaf` that is one directory entry name
    /// (no separator, not `.`/`..`), such as a name read from a listing.
    pub fn with_appended_leaf(&self, leaf: &str) -> ProjectPath {
        ProjectPath::from_raw(format!("{}/{}", self.raw, leaf))
    }

    /// Joins onto an OS root directory.
    pub fn to_os_path(&self, root: &Path) -> PathBuf {
        let mut p = root.to_path_buf();
        for seg in self.raw.split('/') {
            p.push(seg);
        }
        p
    }
}

impl fmt::Display for ProjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl fmt::Debug for ProjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProjectPath({:?})", self.raw)
    }
}

impl AsRef<str> for ProjectPath {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dots_and_slashes() {
        assert_eq!(
            ProjectPath::normalize("./a/./b//c.tex").unwrap().as_str(),
            "a/b/c.tex"
        );
        assert_eq!(
            ProjectPath::normalize("a/b/../c.tex").unwrap().as_str(),
            "a/c.tex"
        );
        assert_eq!(
            ProjectPath::resolve_in("ch", "../x.tex").unwrap().as_str(),
            "x.tex"
        );
    }

    #[test]
    fn rejects_bad_paths() {
        assert_eq!(
            ProjectPath::normalize("../x.tex"),
            Err(PathError::EscapesRoot)
        );
        assert_eq!(
            ProjectPath::normalize("a/../../x.tex"),
            Err(PathError::EscapesRoot)
        );
        assert_eq!(
            ProjectPath::normalize("/etc/passwd"),
            Err(PathError::Absolute)
        );
        assert_eq!(ProjectPath::normalize("~/x"), Err(PathError::Absolute));
        assert_eq!(
            ProjectPath::normalize("C:/x"),
            Err(PathError::ForbiddenCharacter(':'))
        );
        assert_eq!(
            ProjectPath::normalize("a\\b"),
            Err(PathError::ForbiddenCharacter('\\'))
        );
        assert_eq!(ProjectPath::normalize(""), Err(PathError::Empty));
        assert_eq!(ProjectPath::normalize("."), Err(PathError::Empty));
    }

    #[test]
    fn parts() {
        let p = ProjectPath::normalize("a/b/c.tex").unwrap();
        assert_eq!(p.parent_dir(), "a/b");
        assert_eq!(p.file_name(), "c.tex");
        assert_eq!(p.extension(), Some("tex"));
        assert_eq!(ProjectPath::normalize(".hidden").unwrap().extension(), None);
        assert_eq!(p.to_os_path(Path::new("/r")), PathBuf::from("/r/a/b/c.tex"));
    }

    /// Issue #45 finding 3: 'é' as a single precomposed codepoint (NFC) and
    /// 'e' + combining acute accent (NFD) are different byte sequences that
    /// name the same file on a normalization-insensitive filesystem. Two
    /// references using different normalizations of the same on-disk name
    /// must be the *same* `ProjectPath` identity — equal, equally ordered,
    /// and equally hashed — or graph dedup, cycle detection, and the
    /// revision tracker (all keyed by `ProjectPath`) double-count a single
    /// physical file. The raw bytes must stay distinguishable for display
    /// and for building OS paths (filesystem access for a single reference
    /// is unaffected by this).
    #[test]
    fn nfc_and_nfd_spellings_of_the_same_name_share_one_identity() {
        let nfc = ProjectPath::normalize("caf\u{e9}.tex").unwrap();
        let nfd = ProjectPath::normalize("cafe\u{301}.tex").unwrap();
        assert_ne!(
            nfc.as_str(),
            nfd.as_str(),
            "raw bytes are still distinct spellings"
        );
        assert_eq!(nfc, nfd, "identity must be normalization-insensitive");
        assert_eq!(nfc.cmp(&nfd), std::cmp::Ordering::Equal);

        let mut set = std::collections::BTreeSet::new();
        set.insert(nfc.clone());
        set.insert(nfd.clone());
        assert_eq!(set.len(), 1, "BTreeSet must treat them as one entry");

        use std::hash::{Hash, Hasher};
        let mut ha = std::collections::hash_map::DefaultHasher::new();
        let mut hb = std::collections::hash_map::DefaultHasher::new();
        nfc.hash(&mut ha);
        nfd.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish(), "hash must agree too");
    }
}
