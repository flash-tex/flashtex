//! Stage timing for one document: parse, adapt, typeset (shape + line
//! break), page build, display list, v1 fallback, JSON. Usage:
//! `cargo run --release --example stages -- file.tex [repeat]`.

use std::time::Instant;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{adapter, typeset, v1, FontSet, RenderOptions};

fn main() {
    let path = std::env::args().nth(1).expect("file.tex");
    let repeat: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let text = std::fs::read_to_string(&path).expect("read");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let cache = flashtex_render_pipeline::RenderCache::new();
    for _ in 0..repeat {
        let t0 = Instant::now();
        let docs = [SourceDocument { path: "main.tex", text: &text }];
        let parsed = flashtex_compiler::parser::parse_project(&docs, "main.tex");
        let t1 = Instant::now();
        let texts = [text.as_str()];
        let paths = ["main.tex"];
        let labels = adapter::Labels::from_parsed(&parsed);
        let doc = adapter::adapt_cached(&texts, 0, &parsed, &options, &labels, Some(&cache));
        eprintln!("packages {:?} family {:?}", parsed.packages, doc.style.family);
        let t2 = Instant::now();
        let mut ctx = typeset::Context::new(&fonts, &doc.style, &paths);
        let laid = typeset::build(&mut ctx, &doc, Some(&cache));
        let t3 = Instant::now();
        let diagnostics = ctx.take_diagnostics();
        let v2 = typeset::assemble("p", 1, &docs, &doc.style, &fonts, laid, diagnostics, Some(&cache), None, None);
        let t4 = Instant::now();
        let payload = v1::fallback(&v2, Capabilities { rules: true, font_hints: true, display_list: false, images: false, device_color: false, ..Capabilities::default() }, Some(vec![]));
        let t5 = Instant::now();
        let line = payload.write_envelope("stages");
        let t6 = Instant::now();
        let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1000.0;
        println!(
            "parse {:.1} ms | adapt {:.1} | typeset(shape+break+pages) {:.1} | assemble v2 {:.1} | v1 {:.1} | json {:.1} ({} bytes) | pages {}",
            ms(t0, t1),
            ms(t1, t2),
            ms(t2, t3),
            ms(t3, t4),
            ms(t4, t5),
            ms(t5, t6),
            line.len(),
            v2.pages.len()
        );
    }
    for f in fonts.loaded() {
        println!("loaded face: {} ({})", f.name, f.format);
    }
    for (file, why) in fonts.failures() {
        println!("failed: {file}: {why}");
    }
}
