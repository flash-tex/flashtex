//! Prints the engine's measurement of a string so it can be compared with
//! another engine (see `examples/compare_coretext.swift`).
//!
//! Usage:
//!   cargo run --quiet --example measure -- <face> <size-pt> [--stdin | text...]
//!
//! `<face>` is a Core 14 name (`Times-Roman`, `Helvetica`, `Symbol`, ...) or
//! a path to a `.ttf`/`.ttc` (`path#index` selects a collection face). Text
//! read from stdin is used verbatim (macOS Process rewrites argv to NFD).
//!
//! Output (one line per mode, tab-separated):
//!   plain   <width-pt>  <glyph-count>  <missing-count>
//!   shaped  <width-pt>  <glyph-count>  <missing-count>  <ligatures>  <kerning-source>

use std::io::Read;
use std::path::Path;

use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_font_engine::{Face, load_from_path_index};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: measure <face|path[#index]> <size-pt> (--stdin | text...)");
        std::process::exit(2);
    }
    let size: f64 = args[1].parse().expect("size in points");
    let text = if args[2] == "--stdin" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).unwrap();
        s.trim_end_matches('\n').to_string()
    } else {
        args[2..].join(" ")
    };
    let face: Box<dyn Face> = match Core14::from_name(&args[0]) {
        Some(which) => Box::new(Core14Face::new(which)),
        None => {
            let (path, index) = match args[0].rsplit_once('#') {
                Some((p, i)) => (p, i.parse().expect("face index")),
                None => (args[0].as_str(), 0),
            };
            Box::new(
                load_from_path_index(Path::new(path), index).unwrap_or_else(|e| {
                    eprintln!("{path}: {e}");
                    std::process::exit(1);
                }),
            )
        }
    };
    for (name, opts) in [
        ("plain", ShapeOptions::PLAIN),
        ("shaped", ShapeOptions::default()),
    ] {
        match shape(face.as_ref(), &text, &opts) {
            Ok(s) => {
                let glyphs = s.glyphs().count();
                if name == "plain" {
                    println!(
                        "{name}\t{:.4}\t{glyphs}\t{}",
                        s.width_pt(size),
                        s.missing.len()
                    );
                } else {
                    println!(
                        "{name}\t{:.4}\t{glyphs}\t{}\t{}\t{:?}",
                        s.width_pt(size),
                        s.missing.len(),
                        s.ligatures_applied,
                        s.kerning_source
                    );
                }
            }
            Err(e) => println!("{name}\terror\t{e}"),
        }
    }
}
