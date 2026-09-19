//! Debug helper for the package/class kernel (`src/latex_packages.rs`):
//! `cargo run --example package_probe -- <main.tex>` expands the document
//! with a package reader that serves `name.sty`/`name.cls` from the
//! document's directory and prints the display string, the opened files
//! and the diagnostics.
use std::path::Path;
use std::rc::Rc;

use flashtex_tex_expansion::{tokens_to_display_string, Engine};

fn main() {
    let path = std::env::args().nth(1).expect("usage: package_probe <main.tex>");
    let text = std::fs::read_to_string(&path).expect("read");
    let dir = Path::new(&path).parent().map(Path::to_path_buf).unwrap_or_default();
    let mut engine = Engine::new(&text);
    engine.set_package_reader(Rc::new(move |name, ext| std::fs::read_to_string(dir.join(format!("{name}.{ext}"))).ok()));
    let tokens = engine.run();
    println!("OUT: {:?}", tokens_to_display_string(&tokens));
    for file in engine.opened_package_files() {
        println!("OPENED {} as source {} (loaded at {:?})", file.name, file.source_id, file.loaded_at);
    }
    for d in engine.take_diagnostics() {
        println!("DIAG {:?} {:?}: {}", d.severity, d.span, d.message.replace('\n', " / "));
    }
}
