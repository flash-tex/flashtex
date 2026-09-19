//! The oracle: run each package's `.ins` over the `.dtx` sources TeX Live
//! ships in `source/latex/<pkg>/` and compare every generated package
//! file byte for byte with the installed one in `tex/latex/<pkg>/`, which
//! TeX Live produced with the real docstrip. Skipped, loudly, when no
//! TeX Live source tree is found (`FLASHTEX_TEXMF_DIST` overrides the
//! search; the usual MacTeX and Linux roots are tried).
//!
//! TeX is never run here: only its output is read.

use std::path::{Path, PathBuf};

use flashtex_docstrip::{run, DirSources};

fn texmf_dist() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FLASHTEX_TEXMF_DIST") {
        return Some(PathBuf::from(p));
    }
    for candidate in ["/usr/local/texlive/2026/texmf-dist", "/usr/local/texlive/2025/texmf-dist", "/usr/share/texlive/texmf-dist", "/usr/share/texmf-dist"] {
        if Path::new(candidate).join("source/latex").is_dir() {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// A package: its source directory under `source/latex`, its batch file,
/// its installed directory under `tex/latex`, and whether every generated
/// package file must be identical (the claim this crate makes) or the
/// differences are only reported.
struct Case {
    source: &'static str,
    ins: &'static str,
    installed: &'static str,
    must_match: bool,
}

const CASES: &[Case] = &[
    Case { source: "lipsum", ins: "lipsum.ins", installed: "lipsum", must_match: true },
    Case { source: "booktabs", ins: "booktabs.ins", installed: "booktabs", must_match: true },
    Case { source: "siunitx", ins: "siunitx.ins", installed: "siunitx", must_match: true },
    Case { source: "float", ins: "float.ins", installed: "float", must_match: true },
    Case { source: "microtype", ins: "microtype.ins", installed: "microtype", must_match: true },
    Case { source: "xcolor", ins: "xcolor.ins", installed: "xcolor", must_match: true },
    // TeX Live's installed caption-light.sty, caption2.sty, the .sto files
    // and the fallback versions were generated from an older revision of
    // the sources it ships (their copyright line says 2022 where the .dtx
    // says 2023; caption3.sty lacks a trailing comment the .dtx has), so
    // only caption.sty, subcaption.sty and bicaption.sty can match.
    Case { source: "caption", ins: "caption.ins", installed: "caption", must_match: false },
    Case { source: "hyperref", ins: "hyperref.ins", installed: "hyperref", must_match: true },
    Case { source: "fancyhdr", ins: "fancyhdr.ins", installed: "fancyhdr", must_match: true },
    Case { source: "cleveref", ins: "cleveref.ins", installed: "cleveref", must_match: true },
    Case { source: "fontspec", ins: "fontspec.ins", installed: "fontspec", must_match: true },
    Case { source: "amsmath", ins: "amsmath.ins", installed: "amsmath", must_match: true },
    Case { source: "graphics", ins: "graphics.ins", installed: "graphics", must_match: true },
    Case { source: "tools", ins: "tools.ins", installed: "tools", must_match: true },
    // docstrip's own batch file: docstrip.tex, ltxdoc.cls, doc.sty, shortvrb.sty, gind.ist, gglo.ist.
    Case { source: "base", ins: "docstrip.ins", installed: "base", must_match: true },
];

/// Where TeX Live installed a generated file: `tex/latex/<pkg>/` (or a
/// subdirectory of it), else `makeindex/<pkg>/` for `.ist` files.
fn find_installed(dist: &Path, installed: &str, name: &str) -> Option<PathBuf> {
    let name = Path::new(name).file_name()?;
    let direct = dist.join("tex/latex").join(installed).join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let ist = dist.join("makeindex").join(installed).join(name);
    if ist.is_file() {
        return Some(ist);
    }
    fn walk(dir: &Path, name: &std::ffi::OsStr, depth: usize) -> Option<PathBuf> {
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 3 {
                if let Some(f) = walk(&p, name, depth + 1) {
                    return Some(f);
                }
            } else if p.file_name() == Some(name) {
                return Some(p);
            }
        }
        None
    }
    walk(&dist.join("tex/latex").join(installed), name, 0)
}

/// The first differing line, with a little context, for the report.
fn first_difference(ours: &[u8], theirs: &[u8]) -> String {
    let a: Vec<&[u8]> = ours.split(|&b| b == b'\n').collect();
    let b: Vec<&[u8]> = theirs.split(|&b| b == b'\n').collect();
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            return format!("line {}: ours {:?} vs TeX Live {:?}", i + 1, String::from_utf8_lossy(x), String::from_utf8_lossy(y));
        }
    }
    format!("same first {} lines; ours has {} lines, TeX Live {}", a.len().min(b.len()), a.len(), b.len())
}

#[test]
fn generated_files_match_tex_live() {
    let Some(dist) = texmf_dist() else {
        eprintln!("SKIPPED: no TeX Live texmf-dist found (set FLASHTEX_TEXMF_DIST); the docstrip oracle comparison did not run");
        return;
    };
    let mut failures = Vec::new();
    let mut identical = 0;
    for case in CASES {
        let src_dir = dist.join("source/latex").join(case.source);
        let ins_path = src_dir.join(case.ins);
        let Ok(ins) = std::fs::read(&ins_path) else {
            eprintln!("SKIPPED {}: {} not present", case.source, ins_path.display());
            continue;
        };
        let outcome = run(case.ins, &ins, &DirSources(src_dir.clone()));
        for d in &outcome.diagnostics {
            eprintln!("{}: note: {d}", case.source);
        }
        assert!(outcome.completed, "{}: the run did not complete", case.source);
        let mut compared = 0;
        for f in &outcome.files {
            let installed = find_installed(&dist, case.installed, &f.name);
            let Some(theirs) = installed.as_deref().and_then(|p| std::fs::read(p).ok()) else {
                eprintln!("{}: {} generated ({} bytes), not installed under tex/latex/{} or makeindex/, not compared", case.source, f.name, f.bytes.len(), case.installed);
                continue;
            };
            compared += 1;
            if f.bytes == theirs {
                identical += 1;
                eprintln!("{}: {} identical ({} bytes)", case.source, f.name, theirs.len());
            } else {
                let msg = format!("{}: {} differs: {}", case.source, f.name, first_difference(&f.bytes, &theirs));
                eprintln!("{msg}");
                if case.must_match {
                    failures.push(msg);
                }
            }
        }
        assert!(compared > 0, "{}: nothing generated to compare ({:?})", case.source, outcome.diagnostics);
    }
    eprintln!("{identical} generated files identical to TeX Live's");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Opt-in (`FLASHTEX_DOCSTRIP_SWEEP=1`): every `.ins` under
/// `source/latex/<pkg>/`, each generated file compared with the copy TeX
/// Live installed under `tex/latex/<pkg>/` when there is one. Prints a
/// summary and never fails: it measures coverage, it does not gate.
#[test]
fn sweep_every_batch_file_in_tex_live() {
    if std::env::var("FLASHTEX_DOCSTRIP_SWEEP").as_deref() != Ok("1") {
        eprintln!("SKIPPED: set FLASHTEX_DOCSTRIP_SWEEP=1 to run every .ins in TeX Live");
        return;
    }
    let Some(dist) = texmf_dist() else {
        eprintln!("SKIPPED: no TeX Live texmf-dist found");
        return;
    };
    let (mut batches, mut identical, mut differs, mut not_installed, mut incomplete, mut no_files) = (0, 0, 0, 0, 0, 0);
    let mut differing: Vec<String> = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dist.join("source/latex")).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for dir in dirs {
        let pkg = dir.file_name().unwrap().to_string_lossy().into_owned();
        let mut ins_files: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "ins")).collect();
        ins_files.sort();
        for ins_path in ins_files {
            let ins_name = ins_path.file_name().unwrap().to_string_lossy().into_owned();
            let Ok(ins) = std::fs::read(&ins_path) else { continue };
            batches += 1;
            if std::env::var("FLASHTEX_DOCSTRIP_SWEEP_VERBOSE").is_ok() {
                eprintln!("running {pkg}/{ins_name}");
            }
            let outcome = run(&ins_name, &ins, &DirSources(dir.clone()));
            if !outcome.completed {
                incomplete += 1;
                eprintln!("{pkg}/{ins_name}: did not complete: {:?}", outcome.diagnostics.last());
            }
            if outcome.files.is_empty() {
                no_files += 1;
                eprintln!("{pkg}/{ins_name}: no files generated; {}", outcome.diagnostics.first().map(|d| d.to_string()).unwrap_or_default());
                continue;
            }
            for f in &outcome.files {
                match find_installed(&dist, &pkg, &f.name).and_then(|p| std::fs::read(p).ok()) {
                    None => not_installed += 1,
                    Some(theirs) if theirs == f.bytes => identical += 1,
                    Some(theirs) => {
                        differs += 1;
                        differing.push(format!("{pkg}/{ins_name} -> {}: {}", f.name, first_difference(&f.bytes, &theirs)));
                    }
                }
            }
        }
    }
    for d in &differing {
        eprintln!("DIFFERS {d}");
    }
    eprintln!("SWEEP: {batches} batch files; generated files: {identical} identical, {differs} differ, {not_installed} not installed (not compared); {no_files} batch files generated nothing, {incomplete} did not complete");
}
