//! Regression tests for nested `\left`/`\right` performance and output stability.
//!
//! Measuring a `\left`/`\right` pair used to lay out its enclosed content with
//! a full recursive layout, so each nesting level duplicated all the work
//! inside it (exponential time in nesting depth). These tests pin the fixed
//! behavior: deep nesting compiles fast through the real protocol path, and
//! small-nesting display output is byte-identical to the pre-fix layout.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;
use std::time::Instant;

fn compile_text(text: &str) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("m.tex"));
    doc.set("text", json::str_(text.to_string()));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("leftright"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("m.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("lr"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    handle_line(&json::write(&env))
}

fn nested(n: usize) -> String {
    format!("${}x{}$", "\\left(".repeat(n), "\\right)".repeat(n))
}

fn compile_ms(text: &str) -> (f64, String) {
    let start = Instant::now();
    let out = compile_text(text);
    (start.elapsed().as_secs_f64() * 1000.0, out)
}

fn assert_compile_ok(out: &str) {
    let parsed = json::parse(out).expect("reply must be valid JSON");
    assert_eq!(
        parsed.get("id").and_then(|v| v.as_str()),
        Some("lr"),
        "reply id must round-trip"
    );
    assert_eq!(
        parsed.get("type").and_then(|v| v.as_str()),
        Some("compile_result"),
        "nested delimiters must compile, got: {out:.500}"
    );
}

/// Timing probe: logs per-depth compile time through the protocol path.
/// Before the fix this doubled per ~2 levels (exponential); after the fix
/// every depth here is milliseconds.
#[test]
fn left_right_nesting_timing_probe() {
    for n in [12usize, 16, 18, 20] {
        let (ms, out) = compile_ms(&nested(n));
        assert_compile_ok(&out);
        eprintln!("nested left-right n={n}: {ms:.1}ms");
    }
}

/// 30 nested pairs must compile in under a second in release builds.
/// (Debug builds get a looser bound; codegen there is not representative.)
#[test]
fn thirty_nested_left_right_levels_compile_quickly() {
    let (ms, out) = compile_ms(&nested(30));
    assert_compile_ok(&out);
    let budget_ms = if cfg!(debug_assertions) {
        10_000.0
    } else {
        1000.0
    };
    eprintln!("nested left-right n=30: {ms:.1}ms (budget {budget_ms:.0}ms)");
    assert!(
        ms < budget_ms,
        "30 nested \\left/\\right pairs took {ms:.1}ms, budget {budget_ms:.0}ms"
    );
}

/// Display output (delimiter scales and positions) for 1-5 nesting levels,
/// captured before the perf fix; the fix must not change it.
#[test]
fn small_nesting_display_output_is_stable() {
    for (n, golden) in GOLDENS {
        assert_eq!(&layout_debug(*n), golden, "layout changed at n={n}");
    }
}

fn layout_debug(n: usize) -> String {
    let mut diagnostics = Vec::new();
    let source = format!("{}{}{}", "\\left(".repeat(n), "x", "\\right)".repeat(n));
    let tokens = flashtex_compiler::lexer::tokenize(&source);
    let list = flashtex_compiler::math::parse_tokens(
        &tokens,
        flashtex_compiler::math::MathPackages::KERNEL,
        &mut diagnostics,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let boxed = flashtex_compiler::math::layout(&list, 10.0, &mut diagnostics);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    // Compact but exact: every item's text, position, size (delimiter scale
    // shows up in `size`) and span, plus the box extents. Floats use {:?}
    // so this is a byte-exact pin of the layout.
    let mut out = String::new();
    for item in &boxed.items {
        out.push_str(&format!(
            "{:?} x={:?} baseline={:?} size={:?} span={}..{}\n",
            item.text, item.x, item.baseline, item.size, item.span.start, item.span.end
        ));
    }
    out.push_str(&format!(
        "width={:?} ascent={:?} descent={:?}\n",
        boxed.width, boxed.ascent, boxed.descent
    ));
    out
}

const GOLDENS: &[(usize, &str)] = &[
    (
        1,
        "\"(\" x=0.0 baseline=0.0 size=10.0 span=0..6\n\
         \"x\" x=3.33 baseline=0.0 size=10.0 span=6..7\n\
         \")\" x=7.7700000000000005 baseline=0.0 size=10.0 span=7..14\n\
         width=11.100000000000001 ascent=10.0 descent=2.5\n",
    ),
    (
        2,
        "\"(\" x=0.0 baseline=0.0 size=10.0 span=0..6\n\
         \"(\" x=3.33 baseline=0.0 size=10.0 span=6..12\n\
         \"x\" x=6.66 baseline=0.0 size=10.0 span=12..13\n\
         \")\" x=11.100000000000001 baseline=0.0 size=10.0 span=13..20\n\
         \")\" x=14.430000000000001 baseline=0.0 size=10.0 span=20..27\n\
         width=17.76 ascent=10.0 descent=2.5\n",
    ),
    (
        3,
        "\"(\" x=0.0 baseline=0.0 size=10.0 span=0..6\n\
         \"(\" x=3.33 baseline=0.0 size=10.0 span=6..12\n\
         \"(\" x=6.66 baseline=0.0 size=10.0 span=12..18\n\
         \"x\" x=9.99 baseline=0.0 size=10.0 span=18..19\n\
         \")\" x=14.43 baseline=0.0 size=10.0 span=19..26\n\
         \")\" x=17.759999999999998 baseline=0.0 size=10.0 span=26..33\n\
         \")\" x=21.089999999999996 baseline=0.0 size=10.0 span=33..40\n\
         width=24.419999999999995 ascent=10.0 descent=2.5\n",
    ),
    (
        4,
        "\"(\" x=0.0 baseline=0.0 size=10.0 span=0..6\n\
         \"(\" x=3.33 baseline=0.0 size=10.0 span=6..12\n\
         \"(\" x=6.66 baseline=0.0 size=10.0 span=12..18\n\
         \"(\" x=9.99 baseline=0.0 size=10.0 span=18..24\n\
         \"x\" x=13.32 baseline=0.0 size=10.0 span=24..25\n\
         \")\" x=17.76 baseline=0.0 size=10.0 span=25..32\n\
         \")\" x=21.090000000000003 baseline=0.0 size=10.0 span=32..39\n\
         \")\" x=24.42 baseline=0.0 size=10.0 span=39..46\n\
         \")\" x=27.75 baseline=0.0 size=10.0 span=46..53\n\
         width=31.08 ascent=10.0 descent=2.5\n",
    ),
    (
        5,
        "\"(\" x=0.0 baseline=0.0 size=10.0 span=0..6\n\
         \"(\" x=3.33 baseline=0.0 size=10.0 span=6..12\n\
         \"(\" x=6.66 baseline=0.0 size=10.0 span=12..18\n\
         \"(\" x=9.99 baseline=0.0 size=10.0 span=18..24\n\
         \"(\" x=13.32 baseline=0.0 size=10.0 span=24..30\n\
         \"x\" x=16.65 baseline=0.0 size=10.0 span=30..31\n\
         \")\" x=21.09 baseline=0.0 size=10.0 span=31..38\n\
         \")\" x=24.42 baseline=0.0 size=10.0 span=38..45\n\
         \")\" x=27.75 baseline=0.0 size=10.0 span=45..52\n\
         \")\" x=31.08 baseline=0.0 size=10.0 span=52..59\n\
         \")\" x=34.41 baseline=0.0 size=10.0 span=59..66\n\
         width=37.739999999999995 ascent=10.0 descent=2.5\n",
    ),
];
