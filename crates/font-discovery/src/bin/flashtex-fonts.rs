//! `flashtex-fonts`: lists what the discovery index sees, for checking a
//! machine or a project without a document.
//!
//!   flashtex-fonts [--project DIR] [--dir DIR]... families
//!   flashtex-fonts [...] math
//!   flashtex-fonts [...] find "Family" [weight] [italic]
//!   flashtex-fonts [...] faces "Family"
//!
//! Directories: `--dir` ones, then `FLASHTEX_FONT_DIRS`, the project's
//! `fonts/`, then the OS defaults (`scan_dirs`). Timing goes to stderr.

use std::path::PathBuf;

use flashtex_font_discovery::{scan_dirs, FontIndex};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut dirs = Vec::new();
    let mut project = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" if i + 1 < args.len() => {
                dirs.push(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--project" if i + 1 < args.len() => {
                project = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            a => {
                rest.push(a.to_string());
                i += 1;
            }
        }
    }
    dirs.extend(scan_dirs(project.as_deref()));
    let started = std::time::Instant::now();
    let index = FontIndex::scan(&dirs);
    eprintln!(
        "scanned {} faces in {} directories ({} files skipped) in {:.1} ms",
        index.files().len(),
        dirs.len(),
        index.errors().len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    match rest.first().map(String::as_str) {
        Some("families") | None => {
            for f in index.families() {
                println!("{f}");
            }
        }
        Some("math") => {
            for f in index.math_fonts() {
                println!("{}\t{}\t{}", f.info.family, f.info.postscript_name, f.path.display());
            }
        }
        Some("faces") => {
            let family = rest.get(1).map(String::as_str).unwrap_or("");
            for f in index.files().iter().filter(|f| flashtex_font_discovery::normalize(&f.info.family) == flashtex_font_discovery::normalize(family)) {
                println!("{}\t{}\tw{} {}{}\t{}#{}", f.info.subfamily, f.info.postscript_name, f.info.weight, if f.info.italic { "italic" } else { "upright" }, if f.info.has_math { " MATH" } else { "" }, f.path.display(), f.face_index);
            }
        }
        Some("find") => {
            let family = rest.get(1).map(String::as_str).unwrap_or("");
            let weight = rest.get(2).and_then(|w| w.parse().ok()).unwrap_or(400);
            let italic = rest.get(3).is_some_and(|s| s == "italic" || s == "true");
            match index.find_match(family, weight, italic) {
                Some(m) => println!(
                    "{}#{}\t{} ({})\tweight {} ({})\t{}",
                    m.file.path.display(),
                    m.file.face_index,
                    m.file.info.postscript_name,
                    m.file.info.subfamily,
                    m.file.info.weight,
                    if m.exact_weight { "exact" } else { "nearest" },
                    if m.exact_style { "style exact" } else { "style substituted" }
                ),
                None => {
                    eprintln!("no face matches {family:?}");
                    std::process::exit(1);
                }
            }
        }
        Some(other) => {
            eprintln!("unknown command {other}; try families, math, faces, find");
            std::process::exit(2);
        }
    }
}
