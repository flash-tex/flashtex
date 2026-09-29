//! tex.web's `expand_depth` guard (TeX Live: 10000). Scanners that fully
//! expand their operand (`\csname`, `\number`, `\romannumeral`, `\the`,
//! `\if`, `\ifnum`, `\ifcsname`, `\expandafter`) re-enter the expander once
//! per nesting level. Before the guard, a few thousand levels overflowed the
//! host thread's stack, which aborts the process: a document could kill the
//! preview worker, and the request after it was never answered.
//!
//! pdflatex (TeX Live 2026), measured:
//! * 5 000 nested `\csname`: `! Missing \endcsname inserted.` and a PDF.
//! * 20 000 nested `\csname`: `! TeX capacity exceeded, sorry [expansion
//!   depth=10000].` and no PDF.

use flashtex_tex_expansion::expand_str;

const CAPACITY: &str = "TeX capacity exceeded, sorry [expansion depth=10000].";

/// Runs `f` on a thread whose stack holds 10 000 nested expansions: about
/// 64 MiB in a release build and 256 MiB in a debug build (measured on
/// aarch64-apple-darwin), so the default 2 MiB test thread cannot. The
/// worker binaries run their compiles on a stack of this size.
fn on_deep_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("no panic")
}

const STACK_BYTES: usize = 512 << 20;

fn messages(src: String) -> Vec<String> {
    on_deep_stack(move || {
        expand_str(&src)
            .diagnostics
            .into_iter()
            .map(|d| d.message)
            .collect()
    })
}

fn nested(open: &str, n: usize, middle: &str, close: &str) -> String {
    format!("{}{middle}{}", open.repeat(n), close.repeat(n))
}

#[test]
fn nesting_past_the_expand_depth_is_one_capacity_error_not_a_crash() {
    let shapes = [
        ("csname", nested(r"\csname", 20_000, " x", r"\endcsname")),
        ("number", nested(r"\number", 20_000, " 1 ", "")),
        ("romannumeral", nested(r"\romannumeral", 20_000, " 1 ", "")),
        (
            "the",
            format!(r"\count0={}\count1 ", r"\the".repeat(20_000)),
        ),
        ("if", nested(r"\if", 20_000, "aa", "")),
        ("ifnum", nested(r"\ifnum", 20_000, " 1=1 ", "")),
        (
            "ifcsname",
            nested(r"\ifcsname", 20_000, r" x\endcsname", r"\fi"),
        ),
        ("expandafter", nested(r"\expandafter", 40_000, "x", "")),
    ];
    for (name, src) in shapes {
        let started = std::time::Instant::now();
        let got = messages(src);
        assert_eq!(
            got,
            [CAPACITY],
            "{name}: the overflow is fatal and says so once"
        );
        assert!(
            started.elapsed().as_secs() < 30,
            "{name}: {:?}",
            started.elapsed()
        );
    }
}

#[test]
fn nesting_below_the_expand_depth_is_not_a_capacity_error() {
    // pdflatex: 5 000 nested \csname only reports "Missing \endcsname
    // inserted." (the inner \csname x\endcsname is \relax, which is not a
    // character inside the next \csname).
    let got = messages(nested(r"\csname", 5_000, " x", r"\endcsname"));
    assert!(
        !got.iter().any(|m| m == CAPACITY),
        "{:?}",
        &got[..got.len().min(3)]
    );
    assert!(
        got.iter().any(|m| m == r"Missing \endcsname inserted."),
        "{:?}",
        &got[..got.len().min(3)]
    );

    // A deep chain that resolves: \number\number...1 is 1.
    let got = messages(format!(r"\count0={} 1 x", r"\number".repeat(5_000)));
    assert_eq!(got, Vec::<String>::new());
}
