//! Making a bundle from a TeX Live installation: which files, under which
//! package, at which path. The files are TeX Live's, byte for byte, at their
//! paths relative to TEXMFROOT (`texmf-dist/...`, and `texmf-var/...` for
//! what TeX Live generates at install time: `pdftex.map`, `language.dat`).
//! Nothing is patched (DESIGN.md 4.4), so LPPL clause 6 never comes into it.
//!
//! A bundle carries TeX Live's own `texmf-dist/web2c/texmf.cnf`, and the
//! bundle resolver runs kpathsea with it over the bundle's tree, so lookups
//! in a bundle are TeX Live's own: a file of a package that TeX Live's search
//! paths would not find is not found in the bundle either.
//!
//! Only the trees the bundle resolver searches are packed
//! ([`BUNDLE_TREES`]); everything else under TEXMFROOT is left out
//! ([`excluded`]), notably:
//!
//! * `TEXMFROOT/texmf.cnf`, the installation's own settings (the installer
//!   writes `TEXMFHOME = ~/Library/texmf` and the like there): it is no
//!   package's file, and the bundle resolver sets those trees itself
//!   (`KpathseaResolver::for_bundle_tree`), with `TEXMFCNF` at the bundle's
//!   `texmf-dist/web2c`, so TeX Live's texmf.cnf is the one at
//!   `texmf-dist/web2c/texmf.cnf`. kpathsea and pdflatex's `.fls` record the
//!   top-level file as read, so read lists name it.
//! * installed formats (`texmf-var/web2c/pdftex/pdflatex.fmt`): the engine
//!   builds its own format from the bundle's files (DESIGN.md 4.4).
//! * `tlpkg/`, the installer's top-level files and the like, which whole
//!   packages (`texlive.infra`) list as runfiles.

use super::ttb::PackFile;
use crate::resolver::Format;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// `texlive.tlpdb`: each package's `runfiles`, relative to TEXMFROOT.
pub fn parse_tlpdb_runfiles(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut pkg = String::new();
    let mut in_run = false;
    for l in text.lines() {
        if let Some(n) = l.strip_prefix("name ") {
            pkg = n.trim().to_string();
            in_run = false;
        } else if let Some(f) = l.strip_prefix(' ') {
            if in_run && !pkg.is_empty() {
                let f = f.split(' ').next().unwrap_or(f);
                out.entry(pkg.clone()).or_default().push(f.to_string());
            }
        } else {
            in_run = l.starts_with("runfiles");
        }
    }
    out
}

/// The kpathsea format a file of this name is looked up in.
pub fn format_for(name: &str) -> Format {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    match ext {
        "tfm" => Format::Tfm,
        "pfb" | "pfa" => Format::Type1,
        "enc" => Format::Enc,
        "map" => Format::Map,
        "vf" => Format::Vf,
        "bib" => Format::Bib,
        "bst" => Format::Bst,
        "pk" => Format::Pk,
        "tcx" => Format::Web2c,
        "ttf" | "ttc" => Format::TrueType,
        "otf" => Format::OpenType,
        "sfd" => Format::Sfd,
        "ist" => Format::Ist,
        "pgc" => Format::MiscFonts,
        "cnf" if name == "fmtutil.cnf" || name == "texmf.cnf" => Format::Cnf,
        _ => Format::Tex,
    }
}

/// What to put in a bundle.
pub struct Selection {
    pub files: Vec<PackFile>,
    /// Files read that are not TeX Live's (outside TEXMFROOT): left out.
    pub outside: Vec<PathBuf>,
    /// Files under TEXMFROOT that a bundle never carries, relative to it,
    /// with the reason ([`excluded`]).
    pub excluded: Vec<(String, &'static str)>,
}

/// The trees under TEXMFROOT a bundle carries: the ones the bundle
/// resolver gives kpathsea at the same place (`TEXMFDIST`, `TEXMFSYSVAR`,
/// `TEXMFSYSCONFIG`; `KpathseaResolver::for_bundle_tree`). `texmf-config`
/// holds what `tlmgr paper` and the installer configured, e.g. MacTeX's
/// `pdftexconfig.tex` with the paper size, which the format build reads.
pub const BUNDLE_TREES: [&str; 3] = ["texmf-dist", "texmf-var", "texmf-config"];

/// Why the file at `rel` (relative to TEXMFROOT, `/`-separated) never goes
/// into a bundle, or `None` when it may. Whatever this lets through, the
/// bundle reader accepts ([`super::ttb::check_member_path`]).
pub fn excluded(rel: &str) -> Option<&'static str> {
    if rel == "texmf.cnf" {
        return Some(
            "the installation's own settings (TeX Live's texmf.cnf is texmf-dist/web2c/texmf.cnf)",
        );
    }
    if super::ttb::check_member_path(rel).is_err() {
        return Some("not a plain relative path inside a tree");
    }
    if !BUNDLE_TREES.contains(&rel.split('/').next().unwrap_or("")) {
        return Some(
            "outside the trees the bundle resolver searches (texmf-dist, texmf-var, texmf-config)",
        );
    }
    if rel.ends_with(".fmt") {
        return Some("an installed format (the engine builds its own)");
    }
    None
}

/// The package of files TeX Live makes at install time (`texmf-var`).
pub const GENERATED: &str = "texlive.generated";

/// Select the files of a bundle.
///
/// * `root`: TEXMFROOT (`/usr/local/texlive/2026`); bundle paths are relative to it.
/// * `read`: files runs actually read (absolute); always included.
/// * `packages`: the tlpdb's runfiles; with `whole_packages`, every package
///   that holds a file of `read` is included whole. Files in no package go
///   in [`GENERATED`].
pub fn select(
    root: &Path,
    read: &[PathBuf],
    packages: &BTreeMap<String, Vec<String>>,
    whole_packages: bool,
) -> std::io::Result<Selection> {
    let mut package_of: HashMap<&str, &str> = HashMap::new();
    for (p, fs) in packages {
        for f in fs {
            package_of.insert(f.as_str(), p.as_str());
        }
    }
    let mut chosen: BTreeMap<String, String> = BTreeMap::new(); // rel -> pkg
    let mut outside = vec![];
    let mut left_out: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut touched: BTreeSet<String> = BTreeSet::new();
    for p in read {
        let Ok(rel) = p.strip_prefix(root) else {
            outside.push(p.clone());
            continue;
        };
        // Bundle paths are `/`-separated whatever the host's separator.
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if let Some(why) = excluded(&rel) {
            left_out.insert(rel, why);
            continue;
        }
        let pkg = package_of
            .get(rel.as_str())
            .copied()
            .unwrap_or(GENERATED)
            .to_string();
        touched.insert(pkg.clone());
        chosen.insert(rel, pkg);
    }
    if whole_packages {
        for pkg in &touched {
            for rel in packages.get(pkg).map(Vec::as_slice).unwrap_or(&[]) {
                if let Some(why) = excluded(rel) {
                    left_out.insert(rel.clone(), why);
                } else if root.join(rel).is_file() {
                    chosen.entry(rel.clone()).or_insert_with(|| pkg.clone());
                }
            }
        }
    }
    super::ttb::check_case_collisions(chosen.keys().map(String::as_str))
        .map_err(std::io::Error::other)?;
    let mut files = vec![];
    for (rel, pkg) in chosen {
        let data = std::fs::read(root.join(&rel))?;
        files.push(PackFile {
            path: rel,
            data,
            package: pkg,
        });
    }
    Ok(Selection {
        files,
        outside,
        excluded: left_out.into_iter().collect(),
    })
}

/// The TTBv1 search order for Tectonic's reader (FlashTeX's own lookups
/// are kpathsea's with TeX Live's texmf.cnf and do not use it).
pub fn default_search() -> Vec<String> {
    vec!["/texmf-dist//".into(), "/texmf-var//".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tlpdb_runfiles_only() {
        let t = "name latex\ncategory Package\nrunfiles size=2\n texmf-dist/tex/latex/base/article.cls\n texmf-dist/tex/latex/base/size10.clo\ndocfiles size=1\n texmf-dist/doc/latex/base/x.pdf details=\"d\"\n\nname cm\nrunfiles size=1\n texmf-dist/fonts/tfm/public/cm/cmr10.tfm\n";
        let m = parse_tlpdb_runfiles(t);
        assert_eq!(m["latex"].len(), 2);
        assert_eq!(m["cm"], ["texmf-dist/fonts/tfm/public/cm/cmr10.tfm"]);
    }

    /// Two members whose paths differ only in case would be one file in a
    /// case-insensitive cache: the packer refuses them, and so does the
    /// reader (a bundle packed by something else).
    #[test]
    fn case_colliding_members_are_refused() {
        use crate::bundle::{ttb, Bundle, BundleSpec};
        use crate::formats::hex;
        assert!(ttb::check_case_collisions(["texmf-dist/a/X.sty", "texmf-dist/a/x.sty"]).is_err());
        assert!(ttb::check_case_collisions(["texmf-dist/A/x.sty", "texmf-dist/a/x.sty"]).is_err());
        assert!(ttb::check_case_collisions(["texmf-dist/a/x.sty", "texmf-dist/b/x.sty"]).is_ok());
        let f = |p: &str, d: &[u8]| ttb::PackFile {
            path: p.into(),
            data: d.to_vec(),
            package: "p".into(),
        };
        let (bytes, _) = ttb::pack(
            vec![
                f("texmf-dist/tex/latex/x/Foo.sty", b"one"),
                f("texmf-dist/tex/latex/x/foo.sty", b"two"),
            ],
            &default_search(),
            &[],
        );
        let base = std::env::temp_dir().join(format!(
            "flashtex-case-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let p = base.join("c.ttb");
        std::fs::write(&p, &bytes).unwrap();
        let spec = BundleSpec {
            url: format!("file://{}", p.display()),
            digest: hex(&ttb::Header::parse(&bytes).unwrap().digest),
            offline: false,
        };
        let e = Bundle::open(spec, &base.join("cache")).err().unwrap();
        assert!(e.contains("differ only in case"), "{e}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn exclusion_rules() {
        assert!(excluded("texmf.cnf").is_some());
        assert!(excluded("texmf-var/web2c/pdftex/pdflatex.fmt").is_some());
        assert!(excluded("texmf-dist/web2c/pdftex/x.fmt").is_some());
        assert!(excluded("tlpkg/TeXLive/TLUtils.pm").is_some());
        assert!(excluded("release-texlive.txt").is_some());
        assert!(excluded("texmf-config/tex/generic/tex-ini-files/pdftexconfig.tex").is_none());
        assert!(excluded("texmf-local/tex/latex/x.sty").is_some());
        assert!(excluded("texmf-dist/ls-R").is_some());
        assert!(excluded("texmf-dist/web2c/texmf.cnf").is_none());
        assert!(excluded("texmf-dist/web2c/fmtutil.cnf").is_none());
        assert!(excluded("texmf-var/fonts/map/pdftex/updmap/pdftex.map").is_none());
        assert!(excluded("texmf-dist/tex/latex/base/article.cls").is_none());
    }

    /// The packer and the reader agree: a TeX Live root whose read list and
    /// whole packages name the installation's own `texmf.cnf`, an installed
    /// format and `tlpkg/` files packs into a bundle that opens, holding
    /// TeX Live's `texmf-dist/web2c/texmf.cnf` and none of those.
    #[test]
    fn packed_bundle_opens_without_top_level_files_or_formats() {
        use crate::bundle::{ttb, Bundle, BundleSpec};
        use crate::formats::hex;
        let base = std::env::temp_dir().join(format!(
            "flashtex-pack-reader-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = base.join("texlive/2026");
        let files: &[(&str, &[u8])] = &[
            ("texmf.cnf", b"TEXMFHOME = ~/Library/texmf\n"),
            ("texmf-dist/web2c/texmf.cnf", b"% TeX Live's texmf.cnf\n"),
            (
                "texmf-dist/web2c/fmtutil.cnf",
                b"pdflatex pdftex - *pdflatex.ini\n",
            ),
            ("texmf-dist/tex/latex/base/article.cls", b"% article\n"),
            ("texmf-dist/tex/latex/base/size10.clo", b"% size10\n"),
            ("texmf-var/web2c/pdftex/pdflatex.fmt", &[0u8; 4096]),
            (
                "texmf-var/fonts/map/pdftex/updmap/pdftex.map",
                b"cmr10 CMR10 <cmr10.pfb\n",
            ),
            ("tlpkg/TeXLive/TLUtils.pm", b"1;\n"),
            ("release-texlive.txt", b"TeX Live 2026\n"),
        ];
        for (rel, data) in files {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, data).unwrap();
        }
        let packages: BTreeMap<String, Vec<String>> = [
            (
                "latex".to_string(),
                vec![
                    "texmf-dist/tex/latex/base/article.cls".to_string(),
                    "texmf-dist/tex/latex/base/size10.clo".to_string(),
                ],
            ),
            (
                "texlive.infra".to_string(),
                vec![
                    "texmf-dist/web2c/fmtutil.cnf".to_string(),
                    "tlpkg/TeXLive/TLUtils.pm".to_string(),
                    "release-texlive.txt".to_string(),
                ],
            ),
        ]
        .into_iter()
        .collect();
        // What pdflatex's .fls lists (every file it and kpathsea read).
        let read: Vec<PathBuf> = [
            "texmf.cnf",
            "texmf-dist/web2c/texmf.cnf",
            "texmf-dist/web2c/fmtutil.cnf",
            "texmf-var/web2c/pdftex/pdflatex.fmt",
            "texmf-dist/tex/latex/base/article.cls",
            "texmf-var/fonts/map/pdftex/updmap/pdftex.map",
        ]
        .iter()
        .map(|r| root.join(r))
        .collect();
        let sel = select(&root, &read, &packages, true).unwrap();
        let mut packed: Vec<&str> = sel.files.iter().map(|f| f.path.as_str()).collect();
        packed.sort();
        assert_eq!(
            packed,
            [
                "texmf-dist/tex/latex/base/article.cls",
                "texmf-dist/tex/latex/base/size10.clo",
                "texmf-dist/web2c/fmtutil.cnf",
                "texmf-dist/web2c/texmf.cnf",
                "texmf-var/fonts/map/pdftex/updmap/pdftex.map",
            ]
        );
        let left: Vec<&str> = sel.excluded.iter().map(|(r, _)| r.as_str()).collect();
        assert_eq!(
            left,
            [
                "release-texlive.txt",
                "texmf-var/web2c/pdftex/pdflatex.fmt",
                "texmf.cnf",
                "tlpkg/TeXLive/TLUtils.pm",
            ]
        );
        let (bytes, _) = ttb::pack(sel.files, &default_search(), &[]);
        let ttb_path = base.join("b.ttb");
        std::fs::write(&ttb_path, &bytes).unwrap();
        let digest = hex(&ttb::Header::parse(&bytes).unwrap().digest);
        let spec = BundleSpec {
            url: format!("file://{}", ttb_path.display()),
            digest,
            offline: false,
        };
        let mut b = Bundle::open(spec, &base.join("cache")).unwrap();
        let cnf = b.entry_at("texmf-dist/web2c/texmf.cnf").unwrap();
        let p = b.materialize(cnf).unwrap();
        assert_eq!(std::fs::read(p).unwrap(), b"% TeX Live's texmf.cnf\n");
        assert!(b.names().all(|n| !n.ends_with(".fmt")));
        let _ = std::fs::remove_dir_all(&base);
    }
}
