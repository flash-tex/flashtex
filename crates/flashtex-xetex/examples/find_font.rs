//! Which face XeTeX's font lookup (`flashtex_xetex::fontmgr`) selects for
//! each `\font` name, as `tools/xetex-lockstep/fontmatch.py` compares with
//! TeX Live's `xetex`.
//!
//! ```sh
//! cargo run --release --example find_font -- "Helvetica/B" "Avenir Next"
//! printf 'Times\t12\n' | cargo run --release --example find_font
//! ```
//!
//! Input: names as arguments, or one per line on stdin, each optionally
//! followed by a tab and a size: `12` (points, `at 12pt`) or `scaled 1200`
//! (default: `scaled 1000`, a plain `\font\x="NAME"`). Every name gets a
//! fresh font manager, as every `xetex` run does. Output, one line each,
//! tab-separated: the name, `path#index` (or `-`), the `name_of_file`
//! XeTeX leaves (`\fontname` without the size; `-` for a `[file]`), the
//! requested engine (`-` for none), the loaded design size in points, and
//! the size the font is loaded at in scaled points (what XDV's
//! define_native_font records).
//! With `--list`, every catalog face is printed instead: `path#index`, the
//! PostScript name, then readNames' full, family and style names.
//! The catalog is `FLASHTEX_FONT_DIRS` then the system's font directories
//! (font-discovery's `scan_dirs`); `[file]` names go through the engine's
//! kpathsea.

use flashtex_xetex::fontmgr::{FontCatalog, FontMgr};
use std::io::BufRead;
use std::sync::Arc;

fn parse_size(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Some(-1000);
    }
    if let Some(n) = s.strip_prefix("scaled") {
        return n.trim().parse::<i32>().ok().map(|n| -n);
    }
    let pt: f64 = s.trim_end_matches("pt").parse().ok()?;
    Some((pt * 65536.0).round() as i32)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lines: Vec<String> = if args.is_empty() {
        std::io::stdin()
            .lock()
            .lines()
            .map_while(Result::ok)
            .collect()
    } else {
        args
    };
    let t = std::time::Instant::now();
    let catalog = Arc::new(FontCatalog::system_cached(None));
    eprintln!(
        "catalog: {} faces in {:.0} ms",
        catalog.len(),
        t.elapsed().as_secs_f64() * 1000.0
    );
    if lines.first().map(String::as_str) == Some("--list") {
        for f in catalog.faces() {
            let n = &f.names;
            println!(
                "{}#{}\t{}\t{}\t{}\t{}",
                f.path.display(),
                f.index,
                n.ps_name,
                n.full_names.join("|"),
                n.family_names.join("|"),
                n.style_names.join("|")
            );
        }
        return;
    }
    for line in lines {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, size) = line.split_once('\t').unwrap_or((line.as_str(), ""));
        let Some(size) = parse_size(size) else {
            eprintln!("bad size in {line:?}");
            std::process::exit(2);
        };
        let mut mgr = FontMgr::new(Arc::clone(&catalog));
        match mgr.locate(name, size) {
            Some(l) => {
                let engine = if l.req_engine == 0 {
                    "-".to_string()
                } else {
                    (l.req_engine as char).to_string()
                };
                println!(
                    "{name}\t{}#{}\t{}\t{engine}\t{:.4}\t{}",
                    l.path,
                    l.face_index,
                    l.name_of_file.as_deref().unwrap_or("-"),
                    f64::from(l.loaded_font_design_size) / 65536.0,
                    l.scaled_size
                );
            }
            None => println!("{name}\t-\t-\t-\t-\t-"),
        }
    }
}
