//! Scaling benchmark for FT-002 rev 7.
//!
//! Measures 5 KB, 50 KB and 500 KB documents at p50, p95 and p99, and prints the
//! SHA-256 of every generated input and of this binary, so any number here can be
//! tied to exactly the input and build that produced it.
//!
//! It measures COMPILER WORK ONLY: from having the source text to having the
//! laid-out result. UI paint, scheduling, IPC transport and PDF writing are
//! outside this crate. Native paint parity and raw PDF byte equality are separate
//! gates and are deliberately NOT claimed or estimated here.

use flashtex_compiler::incremental::{compile_full, Session};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_font_engine::sha256;
use std::hint::black_box;
use std::time::{Duration, Instant};

const SAMPLES: usize = 30;

/// Deterministic document generator: same bytes on every machine and run.
fn generate(target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + 512);
    out.push_str("\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n");
    let mut n = 0usize;
    // A seeded LCG keeps the shape varied but reproducible.
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    while out.len() < target_bytes {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let pick = (seed >> 33) % 4;
        match pick {
            0 => out.push_str(&format!("\\section{{Section {n}}}\n")),
            1 => out.push_str(&format!(
                "Paragraph {n} of \\proj{{}} with inline $x^{{{n}}} + \\alpha$ maths.\n\n"
            )),
            2 => out.push_str(&format!(
                "Paragraph {n} discusses results in some detail, with enough words to wrap \
                 across a line and exercise the paragraph breaker properly.\n\n"
            )),
            _ => out.push_str(&format!("Displayed: $$\\frac{{a_{{{n}}}}}{{b}}$$\n\n")),
        }
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

fn percentile(samples: &mut [Duration], q: f64) -> Duration {
    samples.sort_unstable();
    let idx = (((samples.len() - 1) as f64) * q).round() as usize;
    samples[idx]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn report(label: &str, mut samples: Vec<Duration>) {
    let p50 = ms(percentile(&mut samples.clone(), 0.50));
    let p95 = ms(percentile(&mut samples.clone(), 0.95));
    let p99 = ms(percentile(&mut samples, 0.99));
    println!("  {label:<22} p50={p50:>9.3} ms  p95={p95:>9.3} ms  p99={p99:>9.3} ms");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn measure(size_label: &str, target: usize) {
    let text = generate(target);
    let constraints = LayoutConstraints::default();

    println!();
    println!(
        "=== {size_label} === {} bytes  sha256={}",
        text.len(),
        &hex(&sha256::digest(text.as_bytes()))[..16]
    );

    // Cold first: a brand-new session each time.
    let mut cold_first = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES.min(5) {
        let start = Instant::now();
        let mut s = Session::new();
        black_box(s.compile(black_box(&text), constraints));
        cold_first.push(start.elapsed());
    }
    report("cold (fresh session)", cold_first);

    // Warm unchanged and one-word edit on a live session.
    let mut session = Session::new();
    session.compile(&text, constraints);

    let mut warm = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        black_box(session.compile(black_box(&text), constraints));
        warm.push(start.elapsed());
    }
    report("warm unchanged", warm);

    // Edit near the middle so reuse has work on both sides.
    let anchor = text.len() / 2;
    let cut = text[..anchor].rfind("Paragraph ").unwrap_or(0);
    let mut edit_samples = Vec::with_capacity(SAMPLES);
    let mut reuse_note = String::new();
    for i in 0..SAMPLES {
        let edited = format!(
            "{}Para{i}graph {}",
            &text[..cut],
            &text[cut + "Paragraph ".len()..]
        );
        let start = Instant::now();
        let r = session.compile(black_box(&edited), constraints);
        edit_samples.push(start.elapsed());
        if i == 0 {
            reuse_note = format!(
                "{} of {} blocks reused",
                r.stats.blocks_reused, r.stats.blocks_total
            );
        }
    }
    report("one-word edit", edit_samples);
    println!("  {reuse_note}");

    // Global macro edit: every block that reads \proj must be recomputed.
    let macro_edited = text.replacen("{FlashTeX}", "{QuickTeX}", 1);
    let mut macro_samples = Vec::with_capacity(SAMPLES);
    let mut macro_session = Session::new();
    macro_session.compile(&text, constraints);
    for _ in 0..SAMPLES.min(10) {
        let start = Instant::now();
        black_box(macro_session.compile(black_box(&macro_edited), constraints));
        macro_samples.push(start.elapsed());
        macro_session.compile(&text, constraints);
    }
    report("global macro edit", macro_samples);

    // Where does the time actually go? Parse is unavoidable today: there is no
    // incremental parsing, so every keystroke reparses the whole document.
    let mut parse_only = Vec::with_capacity(10);
    for _ in 0..10 {
        let start = Instant::now();
        black_box(flashtex_compiler::parser::parse(black_box(&text)));
        parse_only.push(start.elapsed());
    }
    report("  of which: parse", parse_only);

    // Acceptance 2: incremental must equal a clean build at every size.
    let incremental = format!("{:#?}", session.compile(&text, constraints).output);
    let clean = format!("{:#?}", compile_full(&text, constraints));
    println!(
        "  incremental == clean full build: {}",
        if incremental == clean {
            "YES"
        } else {
            "NO — DIVERGED"
        }
    );
    assert_eq!(
        incremental, clean,
        "{size_label}: incremental diverged from a clean build"
    );
}

fn main() {
    let exe = std::env::current_exe().ok();
    let binary_hash = exe
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| hex(&sha256::digest(&b))[..16].to_string())
        .unwrap_or_else(|| "unavailable".into());

    println!("FlashTeX compiler scaling benchmark");
    println!("binary sha256 (first 16): {binary_hash}");
    println!("samples per measurement: {SAMPLES}");

    measure("5 KB", 5_000);
    measure("50 KB", 50_000);
    measure("500 KB", 500_000);

    println!();
    println!("--- what these numbers are, and are not ---");
    println!("measured:     compiler work only, source text to laid-out result");
    println!("NOT measured: UI paint, scheduling, IPC transport, PDF writing");
    println!("SEPARATE GATES, not claimed here: native paint parity, raw PDF byte equality");
    println!("PRODUCT TARGET <200 ms keystroke-to-visible-output REMAINS UNPROVEN");
}
