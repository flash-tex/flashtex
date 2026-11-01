//! Debug helper: run every oracle case one by one, printing the id
//! *before* expanding it (so a hang/blow-up can be located), with the
//! elapsed time after. `cargo run --example oracle_scan [-- <id-prefix>]`
use std::io::Write;
use std::time::Instant;

use flashtex_tex_expansion::{tokens_to_display_string, Engine, Limits};

fn main() {
    let filter = std::env::args().nth(1).unwrap_or_default();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle/manifest.json");
    let text = std::fs::read_to_string(path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    for (id, case) in v.as_object().unwrap() {
        if !id.starts_with(&filter) {
            continue;
        }
        let mode = case["mode"].as_str().unwrap_or("tex");
        let setup = case["setup"].as_str().unwrap_or_default();
        let expr = case["expr"].as_str().unwrap_or_default();
        // Same construction as tests/oracle_tests.rs `case_source`.
        let src = match mode {
            "latex-doc" => format!("{setup}\\begin{{document}}\\relax {expr}"),
            "latex-render" => format!("\\begin{{document}}{setup}\n{expr}"),
            "latex-err" => format!("\\begin{{document}}{setup}{expr}"),
            "latex-write" => format!("\\begin{{document}}{setup}\\relax {expr}"),
            m if m.ends_with("-err") => format!("{setup}{expr}"),
            _ => format!("{setup}\\relax {expr}"),
        };
        print!("{id} ... ");
        std::io::stdout().flush().unwrap();
        let t = Instant::now();
        let limits = Limits { max_expansion_steps: 100_000, max_output_tokens: 100_000, ..Limits::default() };
        let mut e = Engine::with_limits(&src, limits);
        let toks = e.run();
        let s = tokens_to_display_string(&toks);
        println!("{:.1}ms {:?}", t.elapsed().as_secs_f64() * 1e3, s.chars().take(60).collect::<String>());
    }
}
