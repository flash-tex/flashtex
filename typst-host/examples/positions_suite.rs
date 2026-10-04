//! The T1 positions gate over Typst's own test suite (DESIGN.md §15.10):
//! every snippet of `tests/suite` (Typst 0.15.1) that compiles goes
//! through the host's conversion (`convert::page` with
//! `pdfpos`-derived positions, exactly what the host sends a 3.3 client),
//! and every glyph's origin and glyph matrix, and every page box, must
//! equal what the independent checker (`tests/checker`) computes from
//! typst-pdf's own export of the document.
//!
//! ```sh
//! cargo run --release --example positions_suite -- \
//!     --typst <typst checkout at v0.15.1> --assets <typst-dev-assets at v0.15.1> \
//!     --fonts <typst-assets fonts dir> --work <scratch dir> [--json out.jsonl] [--only substr]
//! ```
//!
//! A snippet is run as the test runner runs it, nearly: the page and text
//! defaults of Typst's test library (120 pt wide, auto height, 10 pt
//! margins and text) and its helpers (`test`, `test-repr`, `print`,
//! `lines`, `bounds`, `conifer`, `forest`) are prepended as Typst code. A
//! snippet that does not compile here (errors it tests on purpose, `@test`
//! packages, HTML) is counted and skipped: the gate is over the snippets
//! that compile.

#[path = "../tests/checker/mod.rs"]
mod checker;

use std::io::Write;
use std::path::{Path, PathBuf};

use flashtex_display_list::json::Json;
use flashtex_display_list::page::{Item, Page, StreamKind};
use flashtex_display_list::resource::ImageData;
use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
use flashtex_typst_host::pdfpos;
use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};
use typst_layout::PagedDocument;

const PRELUDE: &str = "#set page(width: 120pt, height: auto, margin: 10pt)\n\
#set text(size: 10pt)\n\
#let test(lhs, rhs) = assert(lhs == rhs, message: \"Assertion failed\")\n\
#let test-repr(lhs, rhs) = assert(repr(lhs) == repr(rhs), message: \"Assertion failed\")\n\
#let print(..args) = none\n\
#let lines(count, numbering: \"A\") = range(1, count + 1).map(n => std.numbering(numbering, n)).join(\"\\n\")\n\
#let bounds(body) = body\n";

/// Typst's test library's globals that the suite's modules use too.
fn library() -> typst::Library {
    use typst::LibraryExt;
    let mut lib = typst::Library::builder().build();
    let scope = lib.global.scope_mut();
    scope.define(
        "conifer",
        typst::visualize::Color::from_u8(0x9f, 0xEB, 0x52, 0xFF),
    );
    scope.define(
        "forest",
        typst::visualize::Color::from_u8(0x43, 0xA1, 0x27, 0xFF),
    );
    lib
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let p = e.path();
        let t = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&p, &t);
        } else if e.file_type().unwrap().is_file() {
            std::fs::copy(&p, &t).unwrap();
        }
    }
}

fn typ_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            typ_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "typ") {
            out.push(p);
        }
    }
}

/// `--- name attrs ---` headers split a test file into snippets.
fn snippets(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.split_inclusive('\n') {
        let t = line.trim_end();
        if t.starts_with("--- ") && t.ends_with(" ---") && t.len() > 8 {
            let name = t[4..t.len() - 4]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            out.push((name, String::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push_str(line);
        }
    }
    out
}

#[derive(Default)]
struct Tally {
    snippets: usize,
    compiled: usize,
    not_compiled: usize,
    export_failed: usize,
    pages: usize,
    glyphs: usize,
    glyphs_drawn: usize,
    mismatched_glyphs: usize,
    mismatched_boxes: usize,
    paths: usize,
    mismatched_paths: usize,
    /// Raster image `Do`s in the PDFs (not refused), images the host drew,
    /// drawn images not the PDF's, and images the host could not place or
    /// decode ("image: ..." UNSUPPORTED entries).
    pdf_images: usize,
    images: usize,
    mismatched_images: usize,
    image_failures: usize,
    /// PDF islands (E5): drawn, and those whose content is not the page's
    /// (`checker::check_island`); the glyphs and paths they carry.
    islands: usize,
    island_mismatches: usize,
    island_glyphs: usize,
    island_paths: usize,
    /// Pages per UNSUPPORTED entry (what still makes pages INCOMPLETE).
    reasons: std::collections::BTreeMap<String, usize>,
    incomplete_pages: usize,
    positions_failed_pages: usize,
    mismatched_snippets: Vec<String>,
    export_failed_snippets: Vec<String>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut typst, mut assets, mut fonts_dir, mut work) = (None, None, None, None);
    let (mut json, mut only) = (None, None);
    let (mut accept_e3, mut accept_images, mut ungated) = (false, false, false);
    while let Some(a) = args.next() {
        let v = args.next().expect("a value");
        match a.as_str() {
            "--typst" => typst = Some(PathBuf::from(v)),
            "--assets" => assets = Some(PathBuf::from(v)),
            "--fonts" => fonts_dir = Some(PathBuf::from(v)),
            "--work" => work = Some(PathBuf::from(v)),
            "--json" => json = Some(PathBuf::from(v)),
            "--only" => only = Some(v),
            // `--accept colour`: the client accepts `color-spaces` and
            // `line-state` (spec §11.3, §11.4).
            // `--accept colour[,images][,ungated]`: `images` is `image-data`
            // (§11.5); `ungated` draws what has no pixel gate row yet as
            // complete (`--draw-ungated`), so that every number is compared.
            "--accept" => {
                accept_e3 = v.split(',').any(|t| t == "colour");
                accept_images = v.split(',').any(|t| t == "images");
                ungated = v.split(',').any(|t| t == "ungated");
            }
            _ => panic!("unknown argument {a}"),
        }
    }
    let typst = typst.expect("--typst");
    let assets = assets.expect("--assets");
    let work = work.expect("--work");
    let fonts = Fonts::load(&FontOptions {
        paths: vec![fonts_dir.expect("--fonts"), assets.join("files/fonts")],
        system: false,
    });
    // The project root: the suite and the assets, as the test world sees
    // them (`tests/...`, `/assets/...`).
    let _ = std::fs::remove_dir_all(&work);
    copy_dir(&typst.join("tests/suite"), &work.join("tests/suite"));
    for sub in [
        "bib", "data", "icons", "images", "plugins", "syntaxes", "text", "themes", "latex",
    ] {
        let from = assets.join("files").join(sub);
        if from.is_dir() {
            copy_dir(&from, &work.join("assets").join(sub));
        }
    }
    let work = work.canonicalize().unwrap();
    let mut files = Vec::new();
    typ_files(&work.join("tests/suite"), &mut files);
    let mut out = json.map(|p| std::io::BufWriter::new(std::fs::File::create(p).unwrap()));
    let mut t = Tally::default();
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        program_budget: None,
        color_spaces: accept_e3,
        line_state: accept_e3,
        image_data: accept_images,
        image_budget: None,
        ungated,
    };
    let t0 = std::time::Instant::now();
    for f in &files {
        let rel_dir = f
            .parent()
            .unwrap()
            .strip_prefix(&work)
            .unwrap()
            .to_path_buf();
        let text = std::fs::read_to_string(f).unwrap();
        for (name, body) in snippets(&text) {
            if only.as_ref().is_some_and(|o| !name.contains(o.as_str())) {
                continue;
            }
            t.snippets += 1;
            let main_rel = rel_dir.join("__snippet__.typ");
            std::fs::write(work.join(&main_rel), format!("{PRELUDE}{body}")).unwrap();
            let main_str = main_rel.to_string_lossy().replace('\\', "/");
            let mut world = HostWorld::new(&work, &main_str, &fonts).unwrap();
            world.set_library(library());
            let doc = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                typst::compile::<PagedDocument>(&world).output
            })) {
                Ok(Ok(d)) => d,
                r => {
                    t.not_compiled += 1;
                    if let Some(o) = out.as_mut() {
                        let why = match r {
                            Ok(Err(e)) => {
                                e.first().map(|d| d.message.to_string()).unwrap_or_default()
                            }
                            _ => "panic".into(),
                        };
                        writeln!(
                            o,
                            r#"{{"snippet":{:?},"not_compiled":{:?}}}"#,
                            format!("{}/{name}", rel_dir.display()),
                            why
                        )
                        .unwrap();
                    }
                    continue;
                }
            };
            t.compiled += 1;
            let bytes = match typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                Ok(b) => b,
                Err(errs) => {
                    // typst-pdf cannot export it, so there is no PDF to
                    // compare with: counted and named, never silent.
                    t.export_failed += 1;
                    let why = errs
                        .first()
                        .map(|d| d.message.to_string())
                        .unwrap_or_default();
                    t.export_failed_snippets
                        .push(format!("{}/{name}: {why}", rel_dir.display()));
                    continue;
                }
            };
            // The PDF's own numbers (islands are compared with these), and
            // as a client without `color-spaces` gets them drawn.
            let (reference, raw_reference) =
                match std::panic::catch_unwind(|| checker::reference(&bytes)) {
                    Ok(r) => {
                        let raw = r.clone();
                        let mut r = r;
                        if !caps.color_spaces {
                            checker::device_only(&mut r);
                        }
                        (r, raw)
                    }
                    Err(_) => {
                        t.mismatched_snippets
                            .push(format!("{name}: the checker failed"));
                        continue;
                    }
                };
            // As the host does: the first page sent alone, the rest in one
            // export (here every page alone, then all together, alternately
            // by snippet, so both paths are compared).
            let all: Vec<usize> = (0..doc.pages().len()).collect();
            let derived: Result<Vec<pdfpos::PagePos>, String> = if t.compiled % 2 == 0 {
                all.iter()
                    .map(|&i| pdfpos::derive(&doc, &[i]).map(|mut v| v.remove(0)))
                    .collect()
            } else {
                pdfpos::derive(&doc, &all)
            };
            let mut tables = Tables::new();
            // IMAGE and IMAGE_DATA by id, as a client receives them.
            let mut image_info = std::collections::HashMap::new();
            let mut bad = 0usize;
            let mut why = String::new();
            for (i, rp) in reference.iter().enumerate() {
                t.pages += 1;
                let positions = match &derived {
                    Ok(v) => Positions::Pdf(&v[i]),
                    Err(e) => Positions::Failed(e),
                };
                let page = match convert::page(&world, &doc, i, &mut tables, caps, &[], positions) {
                    Ok(o) => {
                        for (info, data) in &o.images {
                            let j = Json::parse(std::str::from_utf8(info).unwrap()).unwrap();
                            let d = ImageData::decode(data).unwrap();
                            image_info.insert(d.id, (j, d.parts));
                        }
                        Page::decode(StreamKind::Page, &o.body).unwrap()
                    }
                    Err(e) => {
                        why = format!("convert: {e}");
                        bad += 1;
                        continue;
                    }
                };
                if page
                    .unsupported
                    .iter()
                    .any(|u| u.starts_with("glyph positions") || u.starts_with("paths:"))
                {
                    t.positions_failed_pages += 1;
                    why = page
                        .unsupported
                        .iter()
                        .find(|u| u.starts_with("glyph positions") || u.starts_with("paths:"))
                        .unwrap()
                        .clone();
                    bad += 1;
                    continue;
                }
                // Paths and clips: the PDF's own numbers, in order.
                let hp = checker::host_paths(&page);
                let missing = checker::unmatched_paths(&hp, &rp.paths);
                t.paths += hp.len();
                if missing > 0 {
                    t.mismatched_paths += missing;
                    if why.is_empty() {
                        why = format!(
                            "{missing} of {} paths are not the PDF's (first host path: fill {:?} {:?})",
                            hp.len(),
                            hp.first().map(|p| &p.fill),
                            hp.first().map(|p| &p.fill_space)
                        );
                    }
                    bad += 1;
                }
                // Images: the PDF's samples and CTM, bit for bit (§11.5).
                let hi = checker::host_images(&page);
                t.images += hi.len();
                t.pdf_images += rp.images.iter().filter(|r| !r.form && !r.refused).count();
                let missing = checker::unmatched_images(&hi, &image_info, &rp.images);
                for (m, id) in &hi {
                    let Some((j, parts)) = image_info.get(id) else {
                        continue;
                    };
                    if j.get("island").and_then(Json::as_bool) != Some(true) {
                        continue;
                    }
                    t.islands += 1;
                    match checker::check_island(*m, &parts[0], &raw_reference[i]) {
                        Ok((g, p, _)) => {
                            t.island_glyphs += g;
                            t.island_paths += p;
                        }
                        Err(e) => {
                            t.island_mismatches += 1;
                            if why.is_empty() {
                                why = format!("island: {e}");
                            }
                            bad += 1;
                        }
                    }
                }
                if missing > 0 {
                    t.mismatched_images += missing;
                    if why.is_empty() {
                        why = format!("{missing} of {} images are not the PDF's", hi.len());
                    }
                    bad += 1;
                }
                for u in &page.unsupported {
                    *t.reasons.entry(u.clone()).or_default() += 1;
                }
                if page.flags & 1 != 0 {
                    t.incomplete_pages += 1;
                }
                if let Some(u) = page.unsupported.iter().find(|u| u.starts_with("image: ")) {
                    t.image_failures += 1;
                    if why.is_empty() {
                        why = u.clone();
                    }
                    bad += 1;
                }
                if page.pdf_box.map(f64::to_bits) != rp.media_box.map(f64::to_bits) {
                    t.mismatched_boxes += 1;
                    why = format!("box {:?} vs {:?}", page.pdf_box, rp.media_box);
                    bad += 1;
                }
                t.glyphs += rp.glyphs.len();
                // Every GLYPH item, with the glyph matrix in effect.
                let mut lin = [0.0; 4];
                let mut drawn = Vec::new();
                for it in &page.items {
                    match it {
                        Item::Matrix(m) => {
                            let m = page.matrix(*m);
                            lin = [m[0], m[1], m[2], m[3]];
                        }
                        Item::Glyph { x, y, .. } => drawn.push((*x, *y, lin)),
                        _ => {}
                    }
                }
                t.glyphs_drawn += drawn.len();
                // The PDF shows every glyph of every run; the host draws
                // those whose fill is a solid process colour (spec §11.3,
                // §11.5: the others make the page INCOMPLETE). The frames,
                // walked in typst-pdf's order, say which.
                let mut evs = Vec::new();
                let tp = &doc.pages()[i];
                let ts = typst::layout::Transform::translate(tp.bleed.left, tp.bleed.top);
                checker::events_with(&tp.frame, ts, rp.media_box[3], caps.image_data, &mut evs);
                let expected = match checker::expected(&evs, &rp.glyphs) {
                    Ok(e) => e,
                    Err(e) => {
                        why = format!("the checker cannot place the runs: {e}");
                        bad += 1;
                        continue;
                    }
                };
                if drawn.len() != expected.len() || page.origins.len() != drawn.len() {
                    t.mismatched_glyphs += expected.len().abs_diff(drawn.len());
                    why = format!("{} glyphs drawn, {} expected", drawn.len(), expected.len());
                    bad += 1;
                    continue;
                }
                let h = page.pdf_box[3];
                let sp = |v: f64| (v * 65_781.76).round() as i32;
                let colours = checker::host_glyph_paints(&page);
                for (gi, (r, g)) in expected.iter().zip(&page.origins).enumerate() {
                    let (x, y, l) = drawn[gi];
                    // The fill colour too (alpha only where the page is
                    // complete: without `color-spaces` the host does not
                    // draw it and flags the page).
                    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
                    // On a complete page every part of the paint state is
                    // the PDF's (colour, space, alphas, render mode, line
                    // state); on an INCOMPLETE one the fill colour.
                    let hc = &colours[gi];
                    let differs = if page.flags & 1 == 0 {
                        checker::glyph_paint_mismatch(hc, r)
                    } else if bits(&hc.fill) != bits(&r.fill) || hc.fill_space != r.fill_space {
                        Some(format!(
                            "fill {:?} {}, PDF {:?} {}",
                            hc.fill, hc.fill_space, r.fill, r.fill_space
                        ))
                    } else {
                        None
                    };
                    if let Some(d) = differs {
                        t.mismatched_glyphs += 1;
                        if why.is_empty() {
                            why = format!("glyph {gi}: {d}");
                        }
                        bad += 1;
                        continue;
                    }
                    let ok = r.origin.map(f64::to_bits) == g.map(f64::to_bits)
                        && r.matrix.map(f64::to_bits) == l.map(f64::to_bits)
                        && x == sp(r.origin[0])
                        && y == sp(h - r.origin[1]);
                    if !ok {
                        t.mismatched_glyphs += 1;
                        if why.is_empty() {
                            why = format!(
                                "glyph {gi}: host {:?} {:?} ({x}, {y}), PDF {:?} {:?}",
                                g, l, r.origin, r.matrix
                            );
                        }
                        bad += 1;
                    }
                }
            }
            if bad > 0 {
                t.mismatched_snippets
                    .push(format!("{}/{name}: {why}", rel_dir.display()));
            }
            if let Some(o) = out.as_mut() {
                writeln!(
                    o,
                    "{{\"snippet\":{:?},\"pages\":{},\"bad\":{bad},\"why\":{:?}}}",
                    format!("{}/{name}", rel_dir.display()),
                    doc.pages().len(),
                    why
                )
                .unwrap();
            }
            comemo::evict(30);
        }
    }
    println!(
        "{{\"snippets\":{},\"compiled\":{},\"not_compiled\":{},\"export_failed\":{},\"pages\":{},\"pdf_glyphs\":{},\"drawn_glyphs\":{},\"mismatched_glyphs\":{},\"mismatched_boxes\":{},\"paths\":{},\"mismatched_paths\":{},\"pdf_images\":{},\"images\":{},\"mismatched_images\":{},\"image_failures\":{},\"islands\":{},\"island_mismatches\":{},\"island_glyphs\":{},\"island_paths\":{},\"positions_failed_pages\":{},\"mismatched_snippets\":{},\"seconds\":{:.1}}}",
        t.snippets,
        t.compiled,
        t.not_compiled,
        t.export_failed,
        t.pages,
        t.glyphs,
        t.glyphs_drawn,
        t.mismatched_glyphs,
        t.mismatched_boxes,
        t.paths,
        t.mismatched_paths,
        t.pdf_images,
        t.images,
        t.mismatched_images,
        t.image_failures,
        t.islands,
        t.island_mismatches,
        t.island_glyphs,
        t.island_paths,
        t.positions_failed_pages,
        t.mismatched_snippets.len(),
        t0.elapsed().as_secs_f64()
    );
    eprintln!("INCOMPLETE pages: {} of {}", t.incomplete_pages, t.pages);
    let (n, ms, ms_max, bytes, bytes_max) = convert::island_stats();
    if n > 0 {
        eprintln!(
            "ISLANDS exported: {n}, {:.2} ms each on average (max {ms_max:.1}), {} bytes each on average (max {bytes_max})",
            ms / n as f64,
            bytes / n
        );
    }
    let mut reasons: Vec<_> = t.reasons.iter().collect();
    reasons.sort_by(|a, b| b.1.cmp(a.1));
    for (r, n) in reasons.iter().take(30) {
        eprintln!("UNSUPPORTED on {n} pages: {r}");
    }
    for s in &t.export_failed_snippets {
        eprintln!("SKIPPED (typst-pdf cannot export it, nothing to compare) {s}");
    }
    for s in t.mismatched_snippets.iter().take(60) {
        eprintln!("MISMATCH {s}");
    }
}
