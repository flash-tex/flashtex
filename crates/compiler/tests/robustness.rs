//! Panic hunting.
//!
//! A panic in the compiler is the worst failure the product can have: the worker
//! dies mid-keystroke and the author's preview stops responding. Recovery
//! diagnostics only help if the process survives to emit them.
//!
//! These tests do not check that output is *correct* — the acceptance and
//! recovery suites do that. They check that no input, however malformed, takes
//! the process down or makes a span that cannot be sliced.

use flashtex_compiler::incremental::Session;
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::protocol::handle_line;
use flashtex_compiler::{layout, math, parser};

/// Deterministic xorshift: a fixed seed means a failure is always reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Fragments chosen to collide: openers without closers, math shifts, macro
/// definitions, multi-byte characters, and the commands that take arguments.
const FRAGMENTS: &[&str] = &[
    "{",
    "}",
    "$",
    "$$",
    "\\",
    "\\\\",
    "%",
    "#1",
    "^",
    "_",
    "&",
    "~",
    "\\section",
    "\\section{",
    "\\subsection{x}",
    "\\textbf{",
    "\\emph{}",
    "\\begin{document}",
    "\\end{document}",
    "\\begin{align}",
    "\\end{equation}",
    "\\newcommand",
    "\\newcommand{\\a}",
    "\\newcommand{\\a}[9]{#9}",
    "\\a",
    "\\renewcommand{\\a}{\\a}",
    "\\frac",
    "\\frac{1}",
    "\\sqrt{",
    "\\alpha",
    "\\documentclass",
    "\\documentclass{}",
    "\\usepackage{}",
    "\\input{x}",
    "héllo",
    "naïve",
    "café",
    "日本語",
    "\u{1F600}",
    "a\u{0301}",
    "word",
    " ",
    "\n",
    "\n\n",
    "\n\n\n",
    "x^2",
    "x_{i}",
    "$x",
    "${",
    "}$",
];

fn fuzz_source(rng: &mut Rng, pieces: usize) -> String {
    let mut s = String::new();
    for _ in 0..pieces {
        s.push_str(FRAGMENTS[rng.below(FRAGMENTS.len())]);
    }
    s
}

/// Every span the compiler emits must be sliceable from the text it describes.
/// A span on a non-character boundary would panic the moment anyone navigated to
/// it, so assert it here rather than discovering it in the editor.
fn assert_spans_slice(text: &str, blocks: &[parser::Block]) {
    for page in layout::layout(blocks) {
        for item in page.items {
            let (a, b) = (item.span.start, item.span.end);
            assert!(a <= b, "inverted span {a}..{b} for {:?}", item.text);
            assert!(b <= text.len(), "span {a}..{b} past end {}", text.len());
            assert!(
                text.is_char_boundary(a) && text.is_char_boundary(b),
                "span {a}..{b} is not on a character boundary in {text:?}"
            );
            let _ = &text[a..b];
        }
    }
}

#[test]
fn no_generated_source_panics_the_parser_or_layout() {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    for case in 0..3000 {
        let pieces = 1 + rng.below(24);
        let text = fuzz_source(&mut rng, pieces);
        let parsed = parser::parse(&text);
        assert_spans_slice(&text, &parsed.blocks);
        for d in &parsed.diagnostics {
            if let Some(s) = d.span {
                assert!(
                    text.is_char_boundary(s.start) && text.is_char_boundary(s.end),
                    "case {case}: diagnostic span off a boundary in {text:?}"
                );
                let _ = &text[s.start..s.end];
            }
        }
    }
}

#[test]
fn no_generated_source_panics_incremental_reuse() {
    // Reuse is where state persists across inputs, so it is where a bad
    // interaction is most likely to hide.
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let mut session = Session::new();
    let constraints = LayoutConstraints::default();
    let mut previous = String::new();
    for _ in 0..600 {
        let pieces = 1 + rng.below(16);
        let text = fuzz_source(&mut rng, pieces);
        let result = session.compile(&text, constraints);
        // Whatever reuse decided, it must still equal a clean build.
        let full = flashtex_compiler::incremental::compile_full(&text, constraints);
        assert_eq!(
            format!("{:#?}", result.output),
            format!("{full:#?}"),
            "incremental diverged from clean build\nprevious: {previous:?}\ncurrent: {text:?}"
        );
        previous = text;
    }
}

#[test]
fn no_malformed_request_line_panics_the_transport() {
    let mut rng = Rng(0xDEAD_BEEF_0000_0007);
    let shards = [
        "{",
        "}",
        "[",
        "]",
        "\"",
        ":",
        ",",
        "null",
        "true",
        "1e999",
        "-",
        "\\u",
        "\"protocol_version\"",
        "\"id\"",
        "\"type\"",
        "\"payload\"",
        "\"compile\"",
        "\"documents\"",
        "\"text\"",
        "\"path\"",
        "\u{1F600}",
        "\\\"",
        "0".repeat(40).leak(),
    ];
    for _ in 0..4000 {
        let mut line = String::new();
        for _ in 0..1 + rng.below(18) {
            line.push_str(shards[rng.below(shards.len())]);
        }
        // Must return a reply and must never panic.
        let out = handle_line(&line);
        assert!(!out.is_empty(), "empty reply for {line:?}");
        let parsed = json::parse(&out).expect("every reply must be valid JSON");
        assert!(
            matches!(
                parsed.get("type").and_then(|t| t.as_str()),
                Some("error") | Some("compile_result")
            ),
            "unexpected reply type for {line:?}: {out}"
        );
    }
}

#[test]
fn deeply_nested_input_does_not_blow_the_stack() {
    // Unbounded recursion on nesting is a classic parser crash.
    for depth in [64usize, 512, 4096, 20_000] {
        let text = format!("{}x{}", "{".repeat(depth), "}".repeat(depth));
        let parsed = parser::parse(&text);
        assert_spans_slice(&text, &parsed.blocks);

        let math = format!(
            "${}x{}$",
            "\\frac{".repeat(depth.min(2000)),
            "}{1}".repeat(depth.min(2000))
        );
        let parsed = parser::parse(&math);
        assert_spans_slice(&math, &parsed.blocks);
    }
}

#[test]
fn math_nesting_past_the_limit_diagnoses_once_and_recovers() {
    // `MAX_MATH_DEPTH` is the deepest math nesting that parses cleanly;
    // past it (`\frac`/`\sqrt` recurse through several large frames per
    // level) the debug stack runs out, so parsing must stop with one honest
    // diagnostic — not a stack overflow — and every span must still slice.
    let limit_message = format!(
        "math nesting deeper than {} levels is not supported",
        math::MAX_MATH_DEPTH
    );
    let over = math::MAX_MATH_DEPTH + 1;
    for text in [
        format!("${}x{}$", "{".repeat(over), "}".repeat(over)),
        format!("${}x{}$", "\\frac{".repeat(over), "}{1}".repeat(over)),
    ] {
        let parsed = parser::parse(&text);
        assert_eq!(
            parsed
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message == limit_message)
                .count(),
            1,
            "expected exactly one depth diagnostic for {text:?}"
        );
        assert_spans_slice(&text, &parsed.blocks);
    }

    // At or below the limit, nesting parses exactly as before: no
    // diagnostic, and the content still typesets.
    for depth in [1usize, math::MAX_MATH_DEPTH] {
        for text in [
            format!("${}x{}$", "{".repeat(depth), "}".repeat(depth)),
            format!("${}x{}$", "\\frac{".repeat(depth), "}{1}".repeat(depth)),
        ] {
            let parsed = parser::parse(&text);
            assert!(
                parsed.diagnostics.is_empty(),
                "unexpected diagnostics at depth {depth}: {:?}",
                parsed
                    .diagnostics
                    .iter()
                    .map(|diagnostic| &diagnostic.message)
                    .collect::<Vec<_>>()
            );
            assert!(
                !parsed.blocks.is_empty(),
                "nesting at depth {depth} typeset nothing"
            );
            assert_spans_slice(&text, &parsed.blocks);
        }
    }
}

#[test]
fn a_request_is_answered_even_when_the_document_is_pathological() {
    for text in [
        "\\newcommand{\\a}{\\a}\\a\n",
        "\\newcommand{\\a}[1]{\\a{#1}}\\a{x}\n",
        &"$".repeat(5000),
        &"{".repeat(10_000),
        &"\\section{".repeat(500),
        "\u{FEFF}\\section{bom}\n",
        "\0\0\0 nulls \0\n",
    ] {
        let mut doc = Value::obj();
        doc.set("path", json::str_("m.tex"));
        doc.set("text", json::str_(text.to_string()));
        let mut payload = Value::obj();
        payload.set("project_id", json::str_("fuzz"));
        payload.set("revision", Value::Num(1.0));
        payload.set("entry_path", json::str_("m.tex"));
        payload.set("documents", Value::Arr(vec![doc]));
        let mut env = Value::obj();
        env.set("protocol_version", Value::Num(1.0));
        env.set("id", json::str_("p"));
        env.set("type", json::str_("compile"));
        env.set("payload", payload);

        let out = handle_line(&json::write(&env));
        let parsed = json::parse(&out).expect("reply must be valid JSON");
        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("p"));
    }
}

// ---------------------------------------------------------------------------
// Minimised findings of the mutation fuzzer (`examples/fuzz_compile.rs`). Each
// input panicked, overflowed the stack or hung before its fix.

fn compile_messages(text: &str) -> Vec<String> {
    flashtex_compiler::incremental::compile_full(text, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn lists_nested_past_255_levels_are_too_deeply_nested_not_an_overflow() {
    // The per-kind and total list depths were `count() as u8 + 1`: 255
    // enclosing lists overflowed. LaTeX stops at `\@toodeep` long before.
    for env in ["itemize", "enumerate", "quote"] {
        let text = format!("\\begin{{document}}{}\\item x\n\\end{{document}}\n", format!("\\begin{{{env}}}").repeat(300));
        let messages = compile_messages(&text);
        assert!(
            messages.iter().any(|m| m == "LaTeX Error: Too deeply nested."),
            "{env}: {:?}",
            &messages[..messages.len().min(5)]
        );
    }
    // Within LaTeX's limits (four itemize levels) there is no such error.
    let ok = "\\begin{document}\\begin{itemize}\\item a\\begin{itemize}\\item b\\begin{itemize}\\item c\\begin{itemize}\\item d\\end{itemize}\\end{itemize}\\end{itemize}\\end{itemize}\\end{document}\n";
    assert!(!compile_messages(ok).iter().any(|m| m.contains("Too deeply nested")));
}

fn compile_project_messages(documents: &[(&str, &str)]) -> Vec<String> {
    let documents: Vec<parser::SourceDocument<'_>> =
        documents.iter().map(|&(path, text)| parser::SourceDocument { path, text }).collect();
    let out = flashtex_compiler::incremental::compile_full_project(
        &documents,
        documents[0].path,
        LayoutConstraints::default(),
    );
    for page in &out.pages {
        for item in &page.items {
            assert!(item.span.start <= item.span.end, "inverted span {:?}", item.span);
        }
    }
    out.diagnostics.into_iter().map(|d| d.message).collect()
}

#[test]
fn an_unclosed_math_span_never_inverts() {
    // `\setlength{` re-reads its argument, so the math that `$` opens sees
    // content tokens from before the opener: the span ended before it began
    // ("span start must not exceed end").
    let messages = compile_messages("\\setlength{\\begin{}$");
    assert!(!messages.is_empty());
}

#[test]
fn an_alignment_that_inputs_another_document_keeps_its_spans_in_one_document() {
    // A row's span merged a token of `sub.tex` with one of `main.tex`
    // ("cannot merge spans from different documents").
    compile_project_messages(&[("main.tex", "\\begin{align}a\\include{sub}"), ("sub.tex", "x\\input{sub}\n")]);
    compile_project_messages(&[("main.tex", "\\begin{align}a\\input{sub}"), ("sub.tex", "b\\end{align}\n")]);
}

#[test]
fn nested_sub_parses_hit_tex_grouping_capacity_instead_of_the_stack() {
    // Every nested table cell, box or footnote re-enters the parser on its
    // own token stream: 3000 nested tabulars overflowed an 8 MiB stack, and
    // 10k took minutes (each level copies its cell). TeX stops at 255
    // grouping levels. The limit is sized for release stacks (~3 KiB a
    // level); an unoptimised tabular level takes ~50 KiB, so the debug test
    // runs on a large stack.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(nested_sub_parses_body)
        .unwrap()
        .join()
        .unwrap();
}

fn nested_sub_parses_body() {
    let capacity = "TeX capacity exceeded, sorry [grouping levels=255].";
    for (open, close) in [
        ("\\begin{tabular}{c}", "\\end{tabular}"),
        ("\\footnote{", "}"),
        ("\\colorbox{red}{", "}"),
        ("\\rotatebox{90}{", "}"),
        ("\\uline{", "}"),
    ] {
        let depth = 3000;
        let text = format!(
            "\\documentclass{{article}}\\usepackage{{xcolor,graphicx,ulem}}\\begin{{document}}{}x{}\\end{{document}}\n",
            open.repeat(depth),
            close.repeat(depth)
        );
        let started = std::time::Instant::now();
        let messages = compile_messages(&text);
        assert_eq!(messages.iter().filter(|m| *m == capacity).count(), 1, "{open}: {:?}", &messages[..messages.len().min(4)]);
        assert!(started.elapsed().as_secs() < 30, "{open}: {:?}", started.elapsed());
    }
    // Well inside the limit nothing is reported.
    let text = format!("\\begin{{document}}{}x{}\\end{{document}}\n", "\\begin{tabular}{c}".repeat(20), "\\end{tabular}".repeat(20));
    assert!(!compile_messages(&text).iter().any(|m| m.contains("capacity")));
}

#[test]
fn a_runaway_loop_of_unknown_commands_is_diagnosed_in_bounded_time() {
    // Each of the ~666k `\n` the loop emits before the expansion limit got
    // an unknown-command diagnostic that scanned the whole vocabulary
    // (several times, with allocations): minutes to compile. Suggestions
    // are memoised and known names are a set lookup.
    let started = std::time::Instant::now();
    let messages = compile_messages("\\def\\a{\\n\\a}\\a");
    let elapsed = started.elapsed();
    assert!(messages.iter().any(|m| m.contains("expansion step limit exceeded")));
    assert!(elapsed.as_secs() < 60, "took {elapsed:?}");
}
