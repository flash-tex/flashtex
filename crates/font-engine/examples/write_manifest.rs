//! Writes the pinned Latin Modern manifest JSON to stdout (or to the path
//! given as the first argument). `fonts/manifest.json` is this output.
use flashtex_font_engine::manifest::pinned_latin_modern;

fn main() {
    let json = pinned_latin_modern().to_json();
    match std::env::args().nth(1) {
        Some(path) => std::fs::write(&path, json).expect("write manifest"),
        None => print!("{json}"),
    }
}
