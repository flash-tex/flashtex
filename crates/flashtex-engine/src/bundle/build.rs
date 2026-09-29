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
        "cnf" if name == "fmtutil.cnf" || name == "texmf.cnf" => Format::Cnf,
        _ => Format::Tex,
    }
}

/// What to put in a bundle.
pub struct Selection {
    pub files: Vec<PackFile>,
    /// Files read that are not TeX Live's (outside TEXMFROOT): left out.
    pub outside: Vec<PathBuf>,
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
    let mut touched: BTreeSet<String> = BTreeSet::new();
    for p in read {
        let Ok(rel) = p.strip_prefix(root) else {
            outside.push(p.clone());
            continue;
        };
        let rel = rel.to_string_lossy().into_owned();
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
                if root.join(rel).is_file() {
                    chosen.entry(rel.clone()).or_insert_with(|| pkg.clone());
                }
            }
        }
    }
    let mut files = vec![];
    for (rel, pkg) in chosen {
        let data = std::fs::read(root.join(&rel))?;
        files.push(PackFile {
            path: rel,
            data,
            package: pkg,
        });
    }
    Ok(Selection { files, outside })
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
}
