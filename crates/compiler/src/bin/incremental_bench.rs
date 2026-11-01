//! Reproducible std-only scaling measurement for incremental compilation.

use flashtex_compiler::incremental::{compile_full, CompileOutput, Session};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_font_engine::sha256;
use std::hint::black_box;
use std::time::{Duration, Instant};

const SAMPLES: usize = 30;
const FIXTURES: &[(&str, usize)] = &[("5 KB", 5_000), ("50 KB", 50_000), ("500 KB", 500_000)];
const WORDS: &[&str] = &[
    "layout", "preview", "source", "stable", "quick", "exact", "document", "compiler",
];

/// Generates complete paragraphs until the requested approximate byte size is
/// reached. The fixed xorshift seed makes every input byte reproducible.
fn fixture(target_bytes: usize) -> String {
    let mut source = String::from("\\newcommand{\\term}{FlashTeX}\n\n");
    let mut seed = 0x4d59_5df4_d0f3_3173_u64;
    let mut paragraph = 0;
    while source.len() < target_bytes {
        let mut prose = String::new();
        for _ in 0..12 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            prose.push_str(WORDS[seed as usize % WORDS.len()]);
            prose.push(' ');
        }
        source.push_str(&format!(
            "Paragraph {paragraph} uses \\term and {prose}with inline math $x_{paragraph}^2 + \\alpha$ and a nested fraction $\\frac{{a_{paragraph}}}{{b+1}}$.\n\n"
        ));
        paragraph += 1;
    }
    source
}

fn edited_fixture(source: &str) -> String {
    let middle = source.len() / 2;
    let paragraph = source[middle..]
        .find("Paragraph ")
        .map(|offset| middle + offset)
        .expect("fixture has a paragraph after its midpoint");
    let word = source[paragraph..]
        .find("layout")
        .or_else(|| source[paragraph..].find("preview"))
        .map(|offset| paragraph + offset)
        .expect("fixture has an editable generated word");
    let end = source[word..]
        .find(' ')
        .map(|offset| word + offset)
        .expect("generated word is space terminated");
    let mut edited = source.to_string();
    edited.replace_range(word..end, "changed");
    edited
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn percentile(sorted: &[Duration], percentile: f64) -> Duration {
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index]
}

fn summary(name: &str, mut samples: Vec<Duration>) {
    samples.sort_unstable();
    println!(
        "{name}: p50={:.3} ms p95={:.3} ms p99={:.3} ms",
        milliseconds(percentile(&samples, 0.50)),
        milliseconds(percentile(&samples, 0.95)),
        milliseconds(percentile(&samples, 0.99)),
    );
}

fn hex_sha256(bytes: &[u8]) -> String {
    sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn output_bytes(output: &CompileOutput) -> Vec<u8> {
    format!("{output:#?}").into_bytes()
}

fn assert_clean_equality(label: &str, incremental: &CompileOutput, clean: &CompileOutput) {
    assert_eq!(
        output_bytes(incremental),
        output_bytes(clean),
        "{label}: incremental output differs from a clean full build"
    );
}

fn measure(size: &str, target_bytes: usize) {
    let base = fixture(target_bytes);
    let edited = edited_fixture(&base);
    let macro_edited = base.replacen("{FlashTeX}", "{QuickTeX}", 1);
    let constraints = LayoutConstraints::default();
    let clean_base = compile_full(&base, constraints);
    let clean_edited = compile_full(&edited, constraints);
    let clean_macro_edited = compile_full(&macro_edited, constraints);

    println!();
    println!(
        "fixture {size}: bytes={} blocks={} sha256={}",
        base.len(),
        clean_base.blocks.len(),
        hex_sha256(base.as_bytes())
    );

    let mut cold_first = Vec::with_capacity(SAMPLES);
    let mut cold = Vec::with_capacity(SAMPLES);
    let mut warm = Vec::with_capacity(SAMPLES);
    let mut edit = Vec::with_capacity(SAMPLES);
    let mut macro_edit = Vec::with_capacity(SAMPLES);
    let mut edit_stats = None;
    let mut macro_stats = None;

    for _ in 0..SAMPLES {
        let mut first_session = Session::new();
        let started = Instant::now();
        let first = black_box(first_session.compile(black_box(&base), constraints));
        cold_first.push(started.elapsed());
        black_box(first);

        let started = Instant::now();
        let full = black_box(compile_full(black_box(&base), constraints));
        cold.push(started.elapsed());
        black_box(full);

        let started = Instant::now();
        let unchanged = black_box(first_session.compile(black_box(&base), constraints));
        warm.push(started.elapsed());
        black_box(unchanged);

        let started = Instant::now();
        let changed = black_box(first_session.compile(black_box(&edited), constraints));
        edit.push(started.elapsed());
        edit_stats = Some(changed.stats);

        let mut macro_session = Session::new();
        black_box(macro_session.compile(black_box(&base), constraints));
        let started = Instant::now();
        let changed = black_box(macro_session.compile(black_box(&macro_edited), constraints));
        macro_edit.push(started.elapsed());
        macro_stats = Some(changed.stats);
    }

    summary("cold first compile", cold_first);
    summary("cold", cold);
    summary("warm unchanged", warm);
    summary("one-word edit", edit);
    println!("edit stats: {:?}", edit_stats.expect("sample"));
    summary("global macro edit", macro_edit);
    println!("macro edit stats: {:?}", macro_stats.expect("sample"));

    // Equality checks are intentionally outside the timed intervals. Measuring
    // compiler work must not include formatting comparison evidence.
    let mut equality_session = Session::new();
    let cold_first = equality_session.compile(&base, constraints);
    assert_clean_equality("cold first compile", &cold_first.output, &clean_base);
    let cold = compile_full(&base, constraints);
    assert_clean_equality("cold compile", &cold, &clean_base);
    let unchanged = equality_session.compile(&base, constraints);
    assert_clean_equality("warm unchanged", &unchanged.output, &clean_base);
    let changed = equality_session.compile(&edited, constraints);
    assert_clean_equality("one-word edit", &changed.output, &clean_edited);
    let mut macro_session = Session::new();
    macro_session.compile(&base, constraints);
    let changed = macro_session.compile(&macro_edited, constraints);
    assert_clean_equality("global macro edit", &changed.output, &clean_macro_edited);
    println!("incremental-clean exact equality: PASS (all five cases)");
}

fn main() {
    let binary = std::env::current_exe()
        .and_then(std::fs::read)
        .expect("read the running benchmark binary");
    println!("binary sha256={}", hex_sha256(&binary));
    println!("samples per case={SAMPLES}");
    for &(size, target_bytes) in FIXTURES {
        measure(size, target_bytes);
    }

    // FT-002 acceptance: these timings include compiler work only and leave the
    // native paint and raw PDF byte gates EXPLICITLY SEPARATE. The product target
    // is keystroke to visible matching output, which this binary cannot measure.
    println!();
    println!("--- what these numbers are, and are not ---");
    println!("measured here:     COMPILER WORK ONLY, source text to laid-out result");
    println!("NOT measured here: UI paint, scheduling, IPC transport, PDF writing,");
    println!("                   or the native shell's own work");
    println!("SEPARATE GATES:    native paint equality and raw PDF byte equality");
    println!("PRODUCT TARGET:    <200 ms keystroke-to-visible-output remains UNPROVEN.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashtex_compiler::incremental::compile_full_project;
    use flashtex_compiler::parser::SourceDocument;

    fn assert_project_clean(
        size: &str,
        session: &mut Session,
        documents: &[SourceDocument<'_>],
        entry_path: &str,
        constraints: LayoutConstraints,
    ) {
        let incremental = session.compile_project(documents, entry_path, constraints);
        let clean = compile_full_project(documents, entry_path, constraints);
        assert_clean_equality(size, &incremental.output, &clean);
    }

    #[test]
    fn all_scaling_fixtures_match_clean_build_after_each_edit() {
        let constraints = LayoutConstraints::default();
        for &(size, target_bytes) in FIXTURES {
            let base = fixture(target_bytes);
            let edited = edited_fixture(&base);
            let macro_edited = base.replacen("{FlashTeX}", "{QuickTeX}", 1);
            let mut session = Session::new();

            let cold = session.compile(&base, constraints);
            assert_clean_equality(size, &cold.output, &compile_full(&base, constraints));
            let unchanged = session.compile(&base, constraints);
            assert_clean_equality(size, &unchanged.output, &compile_full(&base, constraints));
            let changed = session.compile(&edited, constraints);
            assert_clean_equality(size, &changed.output, &compile_full(&edited, constraints));
            assert!(
                changed.stats.candidate_comparisons <= changed.stats.blocks_total,
                "{size}: indexed lookup regressed to multiple comparisons per block"
            );

            let mut macro_session = Session::new();
            macro_session.compile(&base, constraints);
            let changed = macro_session.compile(&macro_edited, constraints);
            assert_clean_equality(
                size,
                &changed.output,
                &compile_full(&macro_edited, constraints),
            );
            assert!(
                changed.stats.candidate_comparisons <= changed.stats.blocks_total,
                "{size}: macro invalidation scanned candidate buckets superlinearly"
            );

            let body = base
                .strip_prefix("\\newcommand{\\term}{FlashTeX}\n\n")
                .expect("fixture prelude");
            let edited_body = edited_fixture(body);
            let main = "\\newcommand{\\term}{FlashTeX}\n\\input{body.tex}\n";
            let original_project = [
                SourceDocument {
                    path: "main.tex",
                    text: main,
                },
                SourceDocument {
                    path: "body.tex",
                    text: body,
                },
            ];
            let edited_project = [
                SourceDocument {
                    path: "main.tex",
                    text: main,
                },
                SourceDocument {
                    path: "body.tex",
                    text: &edited_body,
                },
            ];
            let mut project_session = Session::new();
            assert_project_clean(
                size,
                &mut project_session,
                &original_project,
                "main.tex",
                constraints,
            );
            assert_project_clean(
                size,
                &mut project_session,
                &edited_project,
                "main.tex",
                constraints,
            );

            let references = format!(
                "\\section{{Start}}\nSee \\ref{{tail}}.\n\n{base}\n\\section{{Tail}}\\label{{tail}}\n"
            );
            let edited_references = references.replacen("See", "Read", 1);
            let mut reference_session = Session::new();
            let original = reference_session.compile(&references, constraints);
            assert_clean_equality(
                size,
                &original.output,
                &compile_full(&references, constraints),
            );
            let changed = reference_session.compile(&edited_references, constraints);
            assert_clean_equality(
                size,
                &changed.output,
                &compile_full(&edited_references, constraints),
            );
        }
    }
}
