//! `flashtex-tikz-pdf in.tex out.pdf [--border PT] [--size PT]`: compiles
//! the first `tikzpicture` of a standalone document into a one-page PDF
//! (the oracle harness's FlashTeX side). Diagnostics go to stderr, one per
//! line, as `severity:start-end: message`.

use flashtex_render_pipeline::fonts::FontSet;
use flashtex_render_pipeline::tikz::standalone_pdf;
use flashtex_vector_graphics::tikz::Severity;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut border = 5.0;
    let mut size = 10.0;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--border" => {
                border = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(border);
                i += 1;
            }
            "--size" => {
                size = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(size);
                i += 1;
            }
            a => positional.push(a.to_string()),
        }
        i += 1;
    }
    if positional.len() != 2 {
        eprintln!("usage: flashtex-tikz-pdf in.tex out.pdf [--border PT] [--size PT]");
        std::process::exit(2);
    }
    let doc = match std::fs::read_to_string(&positional[0]) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", positional[0]);
            std::process::exit(1);
        }
    };
    let fonts = FontSet::with_default_dirs(&[]);
    match standalone_pdf(&fonts, &doc, border, size) {
        Ok(out) => {
            for d in &out.picture.diagnostics {
                let sev = if d.severity == Severity::Error { "error" } else { "warning" };
                eprintln!("{sev}:{}-{}: {}", d.start, d.end, d.message);
            }
            if let Err(e) = std::fs::write(&positional[1], &out.bytes) {
                eprintln!("error: cannot write {}: {e}", positional[1]);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
