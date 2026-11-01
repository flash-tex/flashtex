//! Debug helper: `cargo run --example probe -- <file>` expands each
//! non-empty line of `<file>` as a separate document and prints the
//! display string plus diagnostics.
use flashtex_tex_expansion::{expand_str, tokens_to_display_string};

fn main() {
    let path = std::env::args().nth(1).expect("usage: probe <file>");
    let text = std::fs::read_to_string(path).expect("read");
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let src = line.replace("<NL>", "\n");
        let r = expand_str(&src);
        println!("== {line}\nOUT: {:?}", tokens_to_display_string(&r.tokens));
        for d in &r.diagnostics {
            println!("DIAG {:?}: {}", d.severity, d.message.replace('\n', " / "));
        }
    }
}
