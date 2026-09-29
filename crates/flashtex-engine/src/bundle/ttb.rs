//! Tectonic's bundle format, TTBv1 (tectonic-typesetting/tectonic,
//! `bundles/format-v1.md`; MIT), read and written as specified, so the
//! same file works with Tectonic's own tools:
//!
//! * a 66-byte header: `tectonicbundle`, version `1` (u32), the index's
//!   offset (u64), gzipped and real length (u32, u32), and the bundle's
//!   SHA-256 digest (32 bytes), little-endian;
//! * gzip members, one per file, at the offsets the index gives;
//! * the index, itself a gzip member: `[DEFAULTSEARCH]`, `[SEARCH:MAIN]`,
//!   `[FILELIST]` lines `<start> <gzip_len> <real_len> <sha256|nohash> <path>`.
//!
//! The digest is Tectonic's: the SHA-256 of the bundle's `FILELIST` file,
//! whose lines are `<sha256|nohash> <path>` sorted by path (component-wise,
//! as Rust's `Path` orders them), listing every member including `SEARCH`
//! (hashed) and `SHA256SUM` and `FILELIST` themselves (`nohash`). So the
//! digest pins every file's content hash, and a reader recomputes it from
//! the index to check the index it was given.
//!
//! FlashTeX adds two sections, which TTBv1 readers ignore ("all others are
//! ignored"): `[FLASHTEX:PACKAGES]`, lines `<start> <len> <name>`, the byte
//! range of each TeX Live package's members (they are stored contiguously),
//! and `[FLASHTEX:CORE]`, the names of the packages fetched up front.

use super::gz;
use crate::formats::hex;
use sha2::{Digest, Sha256};
use std::path::Path;

pub const MAGIC: &[u8; 14] = b"tectonicbundle";
pub const HEADER_SIZE: u64 = 66;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u32,
    pub index_start: u64,
    pub index_gzip_len: u32,
    pub index_real_len: u32,
    pub digest: [u8; 32],
}

impl Header {
    pub fn parse(b: &[u8]) -> Result<Header, String> {
        if b.len() < HEADER_SIZE as usize || &b[..14] != MAGIC {
            return Err("not a Tectonic bundle (bad magic)".into());
        }
        let u32_at = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        let version = u32_at(14);
        if version != 1 {
            return Err(format!("bundle format version {version}, not 1"));
        }
        Ok(Header {
            version,
            index_start: u64::from_le_bytes(b[18..26].try_into().unwrap()),
            index_gzip_len: u32_at(26),
            index_real_len: u32_at(30),
            digest: b[34..66].try_into().unwrap(),
        })
    }

    pub fn encode(&self) -> [u8; 66] {
        let mut h = [0u8; 66];
        h[..14].copy_from_slice(MAGIC);
        h[14..18].copy_from_slice(&self.version.to_le_bytes());
        h[18..26].copy_from_slice(&self.index_start.to_le_bytes());
        h[26..30].copy_from_slice(&self.index_gzip_len.to_le_bytes());
        h[30..34].copy_from_slice(&self.index_real_len.to_le_bytes());
        h[34..66].copy_from_slice(&self.digest);
        h
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub start: u64,
    pub gzip_len: u32,
    pub real_len: u32,
    /// Hex SHA-256 of the decompressed content; `None` for `nohash`.
    pub sha256: Option<String>,
}

impl Entry {
    pub fn basename(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub start: u64,
    pub len: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Index {
    pub search: Vec<String>,
    pub files: Vec<Entry>,
    pub packages: Vec<Package>,
    pub core: Vec<String>,
}

/// TTBv1's meta-files, which are members but not TeX files.
pub const META_FILES: [&str; 3] = ["FILELIST", "SEARCH", "SHA256SUM"];

impl Index {
    pub fn parse(text: &str) -> Result<Index, String> {
        let mut ix = Index::default();
        let mut section = "";
        for (n, line) in text.lines().enumerate() {
            if line.starts_with('[') && line.ends_with(']') {
                section = &line[1..line.len() - 1];
                continue;
            }
            let bad = || format!("index line {}: {line:?}", n + 1);
            match section {
                "SEARCH:MAIN" => ix.search.push(line.to_string()),
                "FILELIST" => {
                    let mut it = line.splitn(5, ' ');
                    let mut next = || it.next().ok_or_else(bad);
                    let start = next()?.parse().map_err(|_| bad())?;
                    let gzip_len = next()?.parse().map_err(|_| bad())?;
                    let real_len = next()?.parse().map_err(|_| bad())?;
                    let hash = next()?;
                    let path = next()?.to_string();
                    let sha256 = (hash != "nohash").then(|| hash.to_string());
                    if sha256.as_ref().is_some_and(|h| h.len() != 64) {
                        return Err(bad());
                    }
                    ix.files.push(Entry {
                        path,
                        start,
                        gzip_len,
                        real_len,
                        sha256,
                    });
                }
                "FLASHTEX:PACKAGES" => {
                    let mut it = line.splitn(3, ' ');
                    let mut next = || it.next().ok_or_else(bad);
                    let start = next()?.parse().map_err(|_| bad())?;
                    let len = next()?.parse().map_err(|_| bad())?;
                    let name = next()?.to_string();
                    ix.packages.push(Package { name, start, len });
                }
                "FLASHTEX:CORE" => ix.core.push(line.to_string()),
                _ => {}
            }
        }
        Ok(ix)
    }

    pub fn encode(&self) -> String {
        let mut s = String::from("[DEFAULTSEARCH]\nMAIN\n[SEARCH:MAIN]\n");
        for l in &self.search {
            s += l;
            s.push('\n');
        }
        s += "[FILELIST]\n";
        for e in &self.files {
            s += &format!(
                "{} {} {} {} {}\n",
                e.start,
                e.gzip_len,
                e.real_len,
                e.sha256.as_deref().unwrap_or("nohash"),
                e.path
            );
        }
        s += "[FLASHTEX:PACKAGES]\n";
        for p in &self.packages {
            s += &format!("{} {} {}\n", p.start, p.len, p.name);
        }
        s += "[FLASHTEX:CORE]\n";
        for c in &self.core {
            s += c;
            s.push('\n');
        }
        s
    }

    /// The bundle's `FILELIST` file as Tectonic writes it.
    pub fn filelist_text(entries: &[(String, Option<String>)]) -> String {
        let mut v: Vec<&(String, Option<String>)> = entries.iter().collect();
        v.sort_by(|a, b| Path::new(&a.0).cmp(Path::new(&b.0)));
        v.iter()
            .map(|(p, h)| format!("{} {p}\n", h.as_deref().unwrap_or("nohash")))
            .collect()
    }

    /// The digest this index implies (see the module documentation).
    pub fn digest(&self) -> [u8; 32] {
        let e: Vec<(String, Option<String>)> = self
            .files
            .iter()
            .map(|e| (e.path.clone(), e.sha256.clone()))
            .collect();
        Sha256::digest(Self::filelist_text(&e).as_bytes()).into()
    }
}

/// A file to pack: its path in the bundle, content, and TeX Live package.
pub struct PackFile {
    pub path: String,
    pub data: Vec<u8>,
    pub package: String,
}

/// Write a TTBv1 bundle. Members are laid out package by package, the
/// `core` packages first, so that each package is one byte range and the
/// core is one range at the front. Returns the bytes and the index.
pub fn pack(mut files: Vec<PackFile>, search: &[String], core: &[String]) -> (Vec<u8>, Index) {
    let rank = |p: &str| core.iter().position(|c| c == p).unwrap_or(usize::MAX);
    files.sort_by(|a, b| {
        (rank(&a.package), &a.package, &a.path).cmp(&(rank(&b.package), &b.package, &b.path))
    });
    let search_text: String = search.iter().map(|l| format!("{l}\n")).collect();
    let mut listed: Vec<(String, Option<String>)> = files
        .iter()
        .map(|f| (f.path.clone(), Some(hex(&Sha256::digest(&f.data)))))
        .collect();
    listed.push((
        "SEARCH".into(),
        Some(hex(&Sha256::digest(search_text.as_bytes()))),
    ));
    listed.push(("SHA256SUM".into(), None));
    listed.push(("FILELIST".into(), None));
    let filelist = Index::filelist_text(&listed);
    let digest: [u8; 32] = Sha256::digest(filelist.as_bytes()).into();
    let sha256sum = format!("{}\n", hex(&digest));

    let mut out = vec![0u8; HEADER_SIZE as usize];
    let mut ix = Index {
        search: search.to_vec(),
        core: core.to_vec(),
        ..Index::default()
    };
    let add = |out: &mut Vec<u8>, path: &str, data: &[u8], hashed: bool| {
        let gz = gz::gzip(data, 9);
        let e = Entry {
            path: path.into(),
            start: out.len() as u64,
            gzip_len: gz.len() as u32,
            real_len: data.len() as u32,
            sha256: hashed.then(|| hex(&Sha256::digest(data))),
        };
        out.extend_from_slice(&gz);
        e
    };
    let mut i = 0;
    while i < files.len() {
        let pkg = files[i].package.clone();
        let start = out.len() as u64;
        while i < files.len() && files[i].package == pkg {
            let e = add(&mut out, &files[i].path, &files[i].data, true);
            ix.files.push(e);
            i += 1;
        }
        ix.packages.push(Package {
            name: pkg,
            start,
            len: out.len() as u64 - start,
        });
    }
    let e = add(&mut out, "SEARCH", search_text.as_bytes(), true);
    ix.files.push(e);
    let e = add(&mut out, "SHA256SUM", sha256sum.as_bytes(), false);
    ix.files.push(e);
    let e = add(&mut out, "FILELIST", filelist.as_bytes(), false);
    ix.files.push(e);
    let text = ix.encode();
    let gz = gz::gzip(text.as_bytes(), 9);
    let h = Header {
        version: 1,
        index_start: out.len() as u64,
        index_gzip_len: gz.len() as u32,
        index_real_len: text.len() as u32,
        digest,
    };
    out.extend_from_slice(&gz);
    out[..HEADER_SIZE as usize].copy_from_slice(&h.encode());
    debug_assert_eq!(ix.digest(), digest);
    (out, ix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_parse_round_trip_and_digest() {
        let files = vec![
            PackFile {
                path: "texmf-dist/tex/latex/base/article.cls".into(),
                data: b"cls".to_vec(),
                package: "latex".into(),
            },
            PackFile {
                path: "texmf-dist/tex/latex/base/size10.clo".into(),
                data: b"clo".to_vec(),
                package: "latex".into(),
            },
            PackFile {
                path: "texmf-dist/fonts/tfm/public/cm/cmr10.tfm".into(),
                data: vec![0; 1000],
                package: "cm".into(),
            },
            PackFile {
                path: "texmf-dist/tex/latex/amsmath/amsmath.sty".into(),
                data: b"sty".to_vec(),
                package: "amsmath".into(),
            },
        ];
        let (bytes, ix) = pack(
            files,
            &["/texmf-dist//".into()],
            &["latex".into(), "cm".into()],
        );
        let h = Header::parse(&bytes).unwrap();
        let s = h.index_start as usize;
        let text = gz::gunzip(&bytes[s..s + h.index_gzip_len as usize], 0).unwrap();
        assert_eq!(text.len(), h.index_real_len as usize);
        let parsed = Index::parse(std::str::from_utf8(&text).unwrap()).unwrap();
        assert_eq!(parsed, ix);
        assert_eq!(parsed.digest(), h.digest);
        // Core packages first, in core order; one byte range each.
        let names: Vec<&str> = parsed.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["latex", "cm", "amsmath"]);
        assert_eq!(parsed.packages[0].start, HEADER_SIZE);
        // SHA256SUM holds the digest, as Tectonic's does.
        let e = parsed.files.iter().find(|e| e.path == "SHA256SUM").unwrap();
        let d = gz::gunzip(&bytes[e.start as usize..][..e.gzip_len as usize], 0).unwrap();
        assert_eq!(String::from_utf8(d).unwrap().trim(), hex(&h.digest));
    }

    #[test]
    fn filelist_sorts_by_path_components() {
        let t = Index::filelist_text(&[("a.b".into(), None), ("a/b".into(), None)]);
        // Component-wise, `a` < `a.b`; bytewise it would be the other way.
        assert_eq!(t, "nohash a/b\nnohash a.b\n");
    }
}
