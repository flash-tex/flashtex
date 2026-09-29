//! Making a bundle from a TeX Live installation: which files, under which
//! package, at which path. The files are TeX Live's, byte for byte (DESIGN.md
//! 4.4: nothing is patched, so LPPL clause 6 never comes into it).
//!
//! The bundle's namespace is flat, so it can hold one file per basename. The
//! file it holds is the one kpathsea finds in the source TeX Live for that
//! basename, under the program name `pdflatex` and the format its extension
//! implies: a file kpathsea would not find there (another directory's
//! `README.cfg`, a ConTeXt-only file) is left out, so a lookup that fails in
//! TeX Live fails in the bundle too.

use super::ttb::PackFile;
use crate::resolver::{FileResolver, Format};
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
    /// Package files kpathsea would not find under their basename: left out.
    pub hidden: usize,
}

/// Select the files of a bundle.
///
/// * `root`: TEXMFROOT (`/usr/local/texlive/2026`); bundle paths are relative to it.
/// * `read`: files runs actually read (absolute); always included.
/// * `packages`: the tlpdb's runfiles; with `whole_packages`, every package
///   that holds a file of `read` is included whole (subject to the rule in
///   the module documentation). Files in no package (TEXMFSYSVAR's generated
///   `pdftex.map`, `language.dat`, ...) go in the package `texlive.generated`.
/// * `r`: kpathsea over the source TeX Live, as `pdflatex`.
pub fn select(
    root: &Path,
    read: &[PathBuf],
    packages: &BTreeMap<String, Vec<String>>,
    whole_packages: bool,
    r: &mut dyn FileResolver,
) -> std::io::Result<Selection> {
    let mut package_of: HashMap<&str, &str> = HashMap::new();
    for (p, fs) in packages {
        for f in fs {
            package_of.insert(f.as_str(), p.as_str());
        }
    }
    let mut chosen: BTreeMap<String, (String, String)> = BTreeMap::new(); // basename -> (rel, pkg)
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
            .unwrap_or("texlive.generated")
            .to_string();
        let base = rel.rsplit('/').next().unwrap_or(&rel).to_string();
        touched.insert(pkg.clone());
        chosen.entry(base).or_insert((rel, pkg));
    }
    let mut hidden = 0;
    if whole_packages {
        for pkg in &touched {
            for rel in packages.get(pkg).map(Vec::as_slice).unwrap_or(&[]) {
                let base = rel.rsplit('/').next().unwrap_or(rel).to_string();
                if chosen.contains_key(&base) {
                    continue;
                }
                let abs = root.join(rel);
                if !abs.is_file() {
                    continue;
                }
                // Only the file kpathsea itself would return for this name.
                let found = r.find(&base, format_for(&base));
                if found.as_deref() != Some(abs.as_path()) {
                    hidden += 1;
                    continue;
                }
                chosen.insert(base, (rel.clone(), pkg.clone()));
            }
        }
    }
    let mut files = vec![];
    for (rel, pkg) in chosen.into_values() {
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
        hidden,
    })
}

/// The TTBv1 search order for Tectonic's reader (FlashTeX's own lookups
/// are kpathsea's over the flat namespace and do not use it).
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
