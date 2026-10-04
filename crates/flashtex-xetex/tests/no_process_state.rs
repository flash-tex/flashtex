//! The crate keeps no process-wide state (docs/design/xetex/PLAN.md §4.7):
//! what TeX Live's XeTeX keeps in C globals is per engine, in
//! `Globals::host` (src/state.rs), so that a resident host can run several
//! engines and restore one. A `static` item or a `thread_local!` in the
//! crate's sources fails this test; a constant is a `const`.

use std::path::Path;

fn scan(dir: &Path, hits: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            scan(&p, hits);
        } else if p.extension().is_some_and(|x| x == "rs") {
            let text = std::fs::read_to_string(&p).unwrap();
            for (i, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("").trim_start();
                let item = code
                    .strip_prefix("pub(crate) ")
                    .or_else(|| code.strip_prefix("pub "))
                    .unwrap_or(code);
                if item.starts_with("static ") || code.contains("thread_local!") {
                    hits.push(format!("{}:{}: {}", p.display(), i + 1, line.trim()));
                }
            }
        }
    }
}

#[test]
fn no_static_or_thread_local_state() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = vec![];
    scan(&src, &mut hits);
    assert!(
        hits.is_empty(),
        "process-wide state; move it into src/state.rs's Host:\n{}",
        hits.join("\n")
    );
}
