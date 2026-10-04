//! The project's FlashTeX Typst lock, `flashtex-typst.lock` in the project
//! root (DESIGN.md §15.2): what the document was set with, so another
//! machine (or a later session) notices when it differs.
//!
//! * **`[packages]`**: `"@ns/name:version" = "sha256:<hex>"`, the SHA-256 of
//!   the package's tarball as it was first fetched. Every later fetch, and
//!   every use of a cached copy, is checked against it; a mismatch is an
//!   error, never silently accepted (typst-kit verifies nothing but TLS, and
//!   the Universe index has no hash field: Track A §6, Track C §2.6).
//! * **`[fonts]`**: `"family|style|weight|stretch" = "sha256:<hex>"`, the
//!   SHA-256 of each font file the document's text uses, recorded on its
//!   first successful compile. A missing or different file is a prominent
//!   warning: Typst itself only warns about an unknown family, and a changed
//!   file silently reflows (Track C §2.5).
//!
//! **Why a separate file in the project root.** It is the project's, like a
//! `Cargo.lock`: committed with the sources so collaborators and CI get the
//! same packages and are told about different fonts; the host writes it
//! (never `flashtex.toml`, which the user edits), and one file keeps both
//! lists in step. The format is a strict TOML subset (one `key = "value"`
//! per line, two tables), readable by any TOML parser and diffable, written
//! sorted so it changes only where an entry does. It is read and written
//! only through the project root's confined, no-symlink paths
//! ([`crate::world::open_for_write`]).

use std::collections::BTreeMap;
use std::path::Path;

/// The lock's file name, in the project root.
pub const FILE: &str = "flashtex-typst.lock";

/// The lock's contents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lock {
    /// `@ns/name:version` → tarball SHA-256 (hex).
    pub packages: BTreeMap<String, String>,
    /// `family|style|weight|stretch` → (font file SHA-256 (hex), file name).
    pub fonts: BTreeMap<String, (String, String)>,
}

impl Lock {
    /// Parse the lock's text. Unknown tables and keys are kept out (a later
    /// version's), malformed lines are an error with their line number.
    pub fn parse(text: &str) -> Result<Lock, String> {
        let mut lock = Lock::default();
        let mut table = String::new();
        let mut comment_file = String::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(c) = line.strip_prefix('#') {
                // `# file: NAME` before a font entry names its file.
                comment_file = c
                    .trim()
                    .strip_prefix("file:")
                    .map(|f| f.trim().to_string())
                    .unwrap_or_default();
                continue;
            }
            if let Some(t) = line.strip_prefix('[') {
                table = t
                    .strip_suffix(']')
                    .ok_or_else(|| format!("{FILE}:{}: unclosed table header", n + 1))?
                    .trim()
                    .to_string();
                continue;
            }
            let (key, rest) = if line.starts_with('"') {
                let (k, used) =
                    unquote(line).ok_or_else(|| format!("{FILE}:{}: a malformed key", n + 1))?;
                (k, line[used..].trim_start())
            } else {
                let at = line
                    .find('=')
                    .ok_or_else(|| format!("{FILE}:{}: expected `key = value`", n + 1))?;
                (line[..at].trim().to_string(), &line[at..])
            };
            let value = rest
                .strip_prefix('=')
                .ok_or_else(|| format!("{FILE}:{}: expected `=`", n + 1))?
                .trim();
            if table.is_empty() && key == "version" {
                if value != "1" {
                    return Err(format!(
                        "{FILE}:{}: lock version {value} is newer than this host reads (1)",
                        n + 1
                    ));
                }
                continue;
            }
            let (value, used) =
                unquote(value).ok_or_else(|| format!("{FILE}:{}: a malformed value", n + 1))?;
            let _ = used;
            let sha = |v: &str| -> Result<String, String> {
                let h = v
                    .strip_prefix("sha256:")
                    .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or_else(|| format!("{FILE}:{}: expected \"sha256:<64 hex>\"", n + 1))?;
                Ok(h.to_ascii_lowercase())
            };
            match table.as_str() {
                "packages" => {
                    lock.packages.insert(key, sha(&value)?);
                }
                "fonts" => {
                    lock.fonts
                        .insert(key, (sha(&value)?, std::mem::take(&mut comment_file)));
                }
                _ => {}
            }
        }
        Ok(lock)
    }

    /// The lock's text, sorted (so it changes only where an entry does).
    pub fn to_text(&self) -> String {
        let mut o = String::from(
            "# flashtex-typst.lock: written by FlashTeX's Typst host (DESIGN.md §15.2).\n\
             # Commit it with the project. [packages]: the SHA-256 of each package's\n\
             # tarball, checked on every later fetch. [fonts]: the font files the\n\
             # document was set with; a missing or different file is reported.\n\
             version = 1\n",
        );
        o.push_str("\n[packages]\n");
        for (k, v) in &self.packages {
            o.push_str(&format!("{} = \"sha256:{v}\"\n", quote(k)));
        }
        o.push_str("\n[fonts]\n");
        for (k, (v, file)) in &self.fonts {
            if !file.is_empty() {
                o.push_str(&format!("# file: {}\n", file.replace('\n', " ")));
            }
            o.push_str(&format!("{} = \"sha256:{v}\"\n", quote(k)));
        }
        o
    }

    /// Read the lock of the project at `root` (canonical): `None` when there
    /// is none. A lock that is a symlink, or that leaves the root, is refused.
    pub fn read(root: &Path) -> Result<Option<Lock>, String> {
        let path = root.join(FILE);
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{FILE}: {e}")),
            Ok(m) if !m.file_type().is_file() => {
                return Err(format!(
                    "{FILE} is refused: it is a symlink or not a regular file"
                ))
            }
            Ok(_) => {}
        }
        let path = crate::world::confine(root, Path::new(FILE))?;
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{FILE}: {e}"))?;
        Lock::parse(&text).map(Some)
    }

    /// Write the lock into the project at `root`, never through a symlink.
    pub fn write(&self, root: &Path) -> Result<(), String> {
        use std::io::Write;
        let mut f = crate::world::open_for_write(root, Path::new(FILE), true)?;
        f.set_len(0).map_err(|e| format!("{FILE}: {e}"))?;
        f.write_all(self.to_text().as_bytes())
            .map_err(|e| format!("{FILE}: {e}"))
    }
}

/// A TOML basic string.
fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => o.push_str(&format!("\\u{:04X}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Read a TOML basic string at the start of `s`: (text, bytes used).
fn unquote(s: &str) -> Option<(String, usize)> {
    let mut chars = s.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut o = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((o, i + 1)),
            '\\' => match chars.next()?.1 {
                '"' => o.push('"'),
                '\\' => o.push('\\'),
                'n' => o.push('\n'),
                't' => o.push('\t'),
                'u' => {
                    let hex: String = (0..4).filter_map(|_| chars.next().map(|x| x.1)).collect();
                    o.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                _ => return None,
            },
            c => o.push(c),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_strictness() {
        let mut l = Lock::default();
        l.packages
            .insert("@preview/cetz:0.3.1".into(), "ab".repeat(32));
        l.fonts.insert(
            "Weird \"Family\"\\|Normal|400|1000".into(),
            ("cd".repeat(32), "W.otf".into()),
        );
        let t = l.to_text();
        assert_eq!(Lock::parse(&t).unwrap(), l);
        assert!(Lock::parse("version = 2\n").is_err());
        assert!(Lock::parse("[packages]\n\"x\" = \"sha256:12\"\n").is_err());
        assert!(Lock::parse("[packages]\n\"x\" = \"md5:00\"\n").is_err());
        // A later version's table is skipped.
        let later = format!("{t}\n[other]\nk = \"v\"\n");
        assert_eq!(Lock::parse(&later).unwrap(), l);
    }
}
