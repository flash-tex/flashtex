//! Resolve a corpus of names and time it; the measuring half of
//! docs/evidence/file-resolver-2026-09-29/run.py.
//!
//!     resolver_corpus CORPUS.tsv OUT.tsv texlive
//!     resolver_corpus CORPUS.tsv OUT.tsv bundle DIR
//!
//! CORPUS.tsv has `format<TAB>name` lines (kpathsea format names). OUT.tsv gets
//! `format<TAB>name<TAB>path-or-empty<TAB>ns-first<TAB>ns-second`, and a
//! `#` header line with the resolver's set-up time. "first" is the first pass
//! over the corpus in a fresh process (it includes kpathsea reading texmf.cnf
//! and ls-R on the first lookup of each format); "second" repeats every
//! lookup, i.e. the resident engine's steady state.

#[cfg(feature = "kpathsea")]
fn main() {
    use flashtex_engine::resolver::{find_texlive_bin, FileResolver, Format, KpathseaResolver};
    use std::io::Write;
    use std::time::Instant;

    let args: Vec<String> = std::env::args().collect();
    let corpus = std::fs::read_to_string(&args[1]).expect("corpus");
    let entries: Vec<(Format, String)> = corpus
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (f, n) = l.split_once('\t').expect("format<TAB>name");
            (Format::from_kpse_name(f).unwrap_or_else(|| panic!("format {f}")), n.to_string())
        })
        .collect();

    let t0 = Instant::now();
    let mut r: Box<dyn FileResolver> = match args[3].as_str() {
        "texlive" => {
            let bin = find_texlive_bin().expect("no TeX Live found");
            Box::new(KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex"))
        }
        "bundle" => Box::new(KpathseaResolver::for_bundle(
            std::path::Path::new(&args[4]),
            "pdflatex",
            "pdftex",
        )),
        m => panic!("mode {m}"),
    };
    let setup = t0.elapsed().as_nanos();

    let mut rows = vec![];
    for (f, n) in &entries {
        let t = Instant::now();
        let p = r.find(n, *f);
        rows.push((f, n, p, t.elapsed().as_nanos()));
    }
    let mut second = vec![];
    for (f, n) in &entries {
        let t = Instant::now();
        let _ = r.find(n, *f);
        second.push(t.elapsed().as_nanos());
    }

    let mut out = std::fs::File::create(&args[2]).expect("out");
    writeln!(out, "# resolver={} setup_ns={setup}", r.describe()).unwrap();
    for ((f, n, p, t1), t2) in rows.into_iter().zip(second) {
        let p = p.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        writeln!(out, "{}\t{n}\t{p}\t{t1}\t{t2}", f.kpse_name()).unwrap();
    }
}

#[cfg(not(feature = "kpathsea"))]
fn main() {
    eprintln!("resolver_corpus needs the `kpathsea` feature");
}
