//! Debug helper: replay `tests/incremental_tests.rs`'s random oracle-doc
//! edits, printing each edited source *before* expanding it, so a hang
//! can be located. `cargo run --example inc_hunt`
use std::io::Write;

use flashtex_tex_expansion::{Edit, Engine, IncrementalExpander, Limits};

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
        (self.next() % n.max(1) as u64) as usize
    }
}

const SNIPPETS: &[&str] = &[
    "a", "Z", " ", "\n", "\n\n", "%", "{", "}", "\\relax ", "\\x", "\\def\\x{Q}", "\\newcommand{\\y}[1]{<#1>}",
    "\\y{v}", "\\begingroup", "\\endgroup", "\\iftrue", "\\fi", "\\else", "#", "\\par", "\\stepcounter{section}",
    "\\label{k}", "$x^2$", "\\section{S}",
];

fn floor(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn random_edit(rng: &mut Rng, src: &str) -> Edit {
    let start = floor(src, rng.below(src.len() + 1));
    match rng.below(3) {
        0 => Edit { start, end: start, replacement: SNIPPETS[rng.below(SNIPPETS.len())].to_string() },
        1 => Edit { start, end: floor(src, start + rng.below(12)), replacement: String::new() },
        _ => Edit { start, end: floor(src, start + rng.below(4)), replacement: SNIPPETS[rng.below(SNIPPETS.len())].to_string() },
    }
}

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle/manifest.json");
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let limits = Limits { max_expansion_steps: 200_000, ..Limits::default() };
    let filter = std::env::args().nth(1).unwrap_or_default();
    for (i, (id, case)) in manifest.as_object().unwrap().iter().enumerate() {
        if !id.starts_with(&filter) {
            continue;
        }
        let src = format!("{}{}", case["setup"].as_str().unwrap_or_default(), case["expr"].as_str().unwrap_or_default());
        println!("DOC {id} initial {src:?}");
        std::io::stdout().flush().unwrap();
        let mut inc = IncrementalExpander::with_options(&src, limits, 8);
        let mut rng = Rng((0x9E37_79B9 + i as u64) | 1);
        for k in 0..12 {
            let edit = random_edit(&mut rng, inc.source());
            let mut next = inc.source().to_string();
            next.replace_range(edit.start..edit.end, &edit.replacement);
            println!("  edit {k} {edit:?} -> {next:?} (incremental)");
            std::io::stdout().flush().unwrap();
            inc.edit(&edit);
            println!("  edit {k} (full)");
            std::io::stdout().flush().unwrap();
            let mut e = Engine::with_limits(inc.source(), limits);
            e.run();
        }
    }
}
