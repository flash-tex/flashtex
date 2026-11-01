//! Debug helper: `cargo run --example dbg -- '<tex source>'` prints the
//! expanded display string, every diagnostic, and recorded labels.
use flashtex_tex_expansion::{expand_str, tokens_to_display_string};

fn main() {
    let src = std::env::args().nth(1).unwrap_or_default();
    let src = if let Some(path) = src.strip_prefix('@') { std::fs::read_to_string(path).unwrap() } else { src };
    let r = expand_str(&src);
    println!("OUT: {:?}", tokens_to_display_string(&r.tokens));
    for d in &r.diagnostics {
        println!("DIAG {:?} @{}..{}: {}", d.severity, d.span.start, d.span.end, d.message);
    }
    for l in &r.labels {
        println!("LABEL {} = {:?}", l.key, l.current_label);
    }
}
