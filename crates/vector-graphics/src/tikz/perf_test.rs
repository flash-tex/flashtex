//! Manual perf comparison for issue #650.
//!
//! `Tikz::render` used to call `blank_comments` on the *entire* document once
//! per picture (`old_render` below reproduces that exactly), so a document
//! with N pictures paid O(N * document_size) instead of O(document_size).
//! The fix blanks only that picture's byte slice.
//!
//! This does NOT go through `flashtex-perf-bench` (`--only tikz-heavy`),
//! because that bench renders through the frozen
//! `crates/render-pipeline/vendor/vector-graphics` mirror and would not see
//! this change until that mirror is re-pinned. Instead this times both the
//! old and the new implementation in the same process, on the same input,
//! in the same run, which removes machine-load bias.
//!
//! Run with (prefer `--release`; the machine is shared, keep `CARGO_BUILD_JOBS`
//! low):
//!   CARGO_BUILD_JOBS=4 \
//!     cargo test -p flashtex-vector-graphics --release \
//!     tikz::perf_test -- --ignored --nocapture

use super::*;
use std::time::{Duration, Instant};

// --- Baseline: the pre-#650 `blank_comments` (char-by-char, always owned). ---
fn old_blank_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_comment = false;
    let mut prev_backslash = false;
    for ch in s.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                out.push('\n');
            } else {
                for _ in 0..ch.len_utf8() {
                    out.push(' ');
                }
            }
            continue;
        }
        if ch == '%' && !prev_backslash {
            in_comment = true;
            out.push(' ');
            continue;
        }
        prev_backslash = ch == '\\' && !prev_backslash;
        out.push(ch);
    }
    out
}

// --- Baseline: the pre-#650 `Tikz::render` body, which blanked the whole
// document on every call and indexed with absolute offsets. ---
fn old_render(tikz: &Tikz, source: &str, picture: &PictureSource, measurer: &dyn TextMeasurer) -> Picture {
    let clean = old_blank_comments(source);
    let mut it = interp::Interp::new(tikz, measurer, picture.start);
    let options = picture.options.map(|(a, b)| &clean[a..b]).unwrap_or("");
    it.picture(options, &clean[picture.body_start..picture.body_end], picture.body_start);
    it.finish()
}

// --- Fixture: byte-for-byte the generator flashtex-perf-bench's
// "tikz-heavy" case uses (crates/perf-bench/src/corpus.rs::tikz_heavy_document).
// Copied rather than depended-on because flashtex-vector-graphics cannot
// depend on flashtex-perf-bench; keep this mirrored if that generator changes. ---
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
}

fn tikz_heavy_document(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 1024);
    out.push_str("\\documentclass{article}\n\\usepackage{tikz}\n\\begin{document}\n");
    let mut n = 0usize;
    let mut rng = Rng(0xB504_F333_F9DE_6484);
    while out.len() < target_bytes {
        let seed = rng.next();
        let a = (n % 4) as f64 * 0.5 + 1.0;
        let b = (n % 3) as f64 * 0.4 + 0.8;
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\draw (0,0) -- ({a},0) -- ({a},{b}) -- cycle;\n\\draw[thick] (0,{b}) -- ({a},{b});\n\\draw[dashed] (0,0) -- ({a},{b});\n\\end{{tikzpicture}}\n\n"
            )),
            1 => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\draw[step=0.5,gray,very thin] (0,0) grid ({a},{b});\n\\draw[->] (0,0) -- ({a},0);\n\\draw[->] (0,0) -- (0,{b});\n\\end{{tikzpicture}}\n\n"
            )),
            2 => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\draw ({a},{b}) circle ({b});\n\\fill[black!20] (0,0) rectangle ({a},{b});\n\\draw[very thick] (0,0) -- ({a},{b});\n\\end{{tikzpicture}}\n\n"
            )),
            _ => out.push_str(&format!(
                "\\begin{{tikzpicture}}\n\\node (a{n}) at (0,0) {{Start {n}}};\n\\node (b{n}) at ({a},{b}) {{End {n}}};\n\\draw[->] (a{n}) -- (b{n});\n\\end{{tikzpicture}}\n\n"
            )),
        }
        out.push_str(&format!(
            "Figure {n} shows the construction described above, drawn to the same scale as the preceding one so the two can be compared directly.\n\n"
        ));
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

fn median(mut xs: Vec<Duration>) -> Duration {
    xs.sort();
    xs[xs.len() / 2]
}

fn time_old(tikz: &Tikz, doc: &str, pictures: &[PictureSource], measurer: &dyn TextMeasurer) -> Duration {
    let start = Instant::now();
    for p in pictures {
        std::hint::black_box(old_render(tikz, doc, p, measurer));
    }
    start.elapsed()
}

fn time_new(tikz: &Tikz, doc: &str, pictures: &[PictureSource], measurer: &dyn TextMeasurer) -> Duration {
    let start = Instant::now();
    for p in pictures {
        std::hint::black_box(tikz.render(doc, p, measurer));
    }
    start.elapsed()
}

#[test]
#[ignore = "manual perf comparison; run with --release -- --ignored --nocapture"]
fn per_picture_blanking_old_vs_new() {
    const RUNS: usize = 9;
    // 120_000 bytes matches flashtex-perf-bench's actual tikz-heavy corpus
    // size; 2_000_000 bytes is synthesized at larger scale (same generator,
    // just run longer) purely to make the per-picture-count scaling visible.
    for target_bytes in [120_000usize, 2_000_000] {
        let doc = tikz_heavy_document(target_bytes);
        let pictures = find_pictures(&doc);
        let tikz = Tikz::new(10.0);
        let measurer = ApproxMeasurer;

        // Warm up (page-in, allocator warmup) before any measured run.
        time_old(&tikz, &doc, &pictures, &measurer);
        time_new(&tikz, &doc, &pictures, &measurer);

        let old_times: Vec<Duration> = (0..RUNS).map(|_| time_old(&tikz, &doc, &pictures, &measurer)).collect();
        let new_times: Vec<Duration> = (0..RUNS).map(|_| time_new(&tikz, &doc, &pictures, &measurer)).collect();

        let old_median = median(old_times);
        let new_median = median(new_times);
        eprintln!(
            "tikz-heavy {target_bytes}B doc, {} pictures, {RUNS} runs each: old median {old_median:?}, new median {new_median:?} ({:.2}x)",
            pictures.len(),
            old_median.as_secs_f64() / new_median.as_secs_f64().max(1e-12),
        );
    }
}
