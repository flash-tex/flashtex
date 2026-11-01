//! Runs the visual-corpus text (tests/visual-corpus/fixtures/*.tex, text
//! extracted from the `\mbox{}` and paragraph lines) through every consumer
//! route that reads this engine and prints the width each route obtains, so
//! docs/consumers.md can quantify what still differs:
//!
//!   engine      shape() with the pinned Latin Modern Roman 10 face at 12 pt
//!   paragraph   paragraph-layout `shape_run` over `FaceMetrics` (char route)
//!   paragraph*  paragraph-layout `GlyphRun::from_shaped` (engine route)
//!   pdf         width implied by the PDF /W array (1/1000 em rounding) plus
//!               the kerning carried as TJ adjustments — what a PDF reader
//!               advances by
//!   compiler    crates/compiler's own Times-Roman table (same AFM values as
//!               this crate's Core 14 table, proven equal in tests) — the
//!               source the compiler still uses until it adopts the adapter
//!
//! Also writes `<out dir>/face_metrics.json` (adapters::preview export) for
//! `tools/preview_positions_check.swift`.
//!
//!   cargo run --example corpus_evidence -- <out dir>

use std::path::Path;

use flashtex_font_engine::Face;
use flashtex_font_engine::adapters::paragraph::{FaceMetrics, glyph_run};
use flashtex_font_engine::adapters::preview::{ExportRun, face_metrics_json};
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::embed::EmbedPlan;
use flashtex_font_engine::manifest::{BASICTEX_OPENTYPE_ROOT, PinnedFontSet, pinned_latin_modern};
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_paragraph_layout::items::shape_run;

/// (fixture id, text) — TeX input ligatures already resolved to Unicode
/// (`` `` `` → “, `''` → ”, `---` → —, `--` → –); the `\kern0pt` line of
/// kerning-ligatures is the suppression control and is listed with the
/// kerns removed, which the engine cannot express (see consumers.md).
const CORPUS: &[(&str, &str)] = &[
    (
        "kerning-ligatures",
        "AVATAR WA To Ta Yo VA ff fi fl ffi ffl office affinity",
    ),
    ("kerning-ligatures", "AVATAR AVATAR ToToTo office office"),
    ("kerning-ligatures", "(AV) [To] “office” — – - 0123456789"),
    ("font-faces", "Roman: AVATAR office 0123456789."),
    (
        "paragraph-linebreaks",
        "A student writes a careful explanation of an equation.",
    ),
    (
        "paragraph-linebreaks",
        "Repeated words expose changes in font widths: minimum maximum minimum maximum.",
    ),
    (
        "paragraph-linebreaks",
        "Hyphenation illustrates international collaboration and computational mathematics.",
    ),
    (
        "paragraph-linebreaks",
        "The same font is constrained to a narrower measure.",
    ),
];

const SIZE: f64 = 12.0;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let set = PinnedFontSet::load(
        &pinned_latin_modern(),
        Path::new(BASICTEX_OPENTYPE_ROOT),
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"),
    )
    .expect("pinned Latin Modern");
    let roman = set.face("lm.roman10.regular").unwrap();
    let times = Core14Face::new(Core14::TimesRoman);
    let opts = ShapeOptions::default();

    println!(
        "| fixture | text | engine pt | paragraph (char) Δ | paragraph (shaped) Δ | pdf /W+TJ Δ | compiler Times-Roman table pt (Δ) | ligatures | kern src |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    let mut shaped_runs = Vec::new();
    for (fixture, text) in CORPUS {
        let s = shape(roman, text, &opts).expect("shape");
        let engine = s.width_pt(SIZE);
        let char_route = shape_run(&FaceMetrics::new(roman), SIZE, text, 0).width;
        let (run, _) = glyph_run(roman, SIZE, text, 0, &opts).expect("run");
        // PDF: /W widths are round(units*1000/upem); TJ carries the kerns.
        let mut plan = EmbedPlan::new();
        plan.add_shaped(&s);
        let program = plan.finish(roman).expect("embed");
        let upem = f64::from(roman.units_per_em());
        let mut pdf = 0.0;
        for g in s.glyphs() {
            let cid = program.cid(g.gid).unwrap();
            let w = f64::from(program.width(cid).unwrap()) / 1000.0 * SIZE;
            let kern_units = f64::from(g.advance) - f64::from(roman.advance(g.gid).unwrap());
            pdf += w + kern_units * SIZE / upem;
        }
        let compiler = shape(&times, text, &ShapeOptions::PLAIN)
            .map(|t| t.width_pt(SIZE))
            .unwrap_or(f64::NAN);
        println!(
            "| {fixture} | {text} | {engine:.4} | {:+.4} | {:+.4} | {:+.4} | {compiler:.4} ({:+.3}) | {} | {:?} |",
            char_route - engine,
            run.width - engine,
            pdf - engine,
            compiler - engine,
            s.ligatures_applied,
            s.kerning_source
        );
        shaped_runs.push((*text, s));
    }
    let runs: Vec<ExportRun<'_>> = shaped_runs
        .iter()
        .map(|(t, s)| ExportRun { text: t, shaped: s })
        .collect();
    let json = face_metrics_json(roman, SIZE, &runs);
    let path = Path::new(&out).join("face_metrics.json");
    std::fs::write(&path, json).expect("write json");
    eprintln!("wrote {}", path.display());
}
