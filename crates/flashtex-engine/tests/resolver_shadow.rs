//! A file added where kpathsea searches on disk shadows the one a lookup
//! found before, at once, in this process: the resolver's kept lookups
//! (src/resolver.rs `Found`) depend on every directory kpathsea reads on
//! disk, not only the working directory (#1493 review). Three cases, one
//! process (TEXMFHOME and TEXINPUTS are set before kpathsea starts):
//!
//! 1. a package added to `$TEXMFHOME/tex/latex`;
//! 2. a class added to a subdirectory of a user TEXINPUTS entry (`dir//`);
//! 3. `hyphen.cfg` added to TEXMFHOME: the format cache must rebuild
//!    `pdflatex.fmt` (src/formats.rs, "What the cache is keyed by").
//!
//! Each fails with a cache that depends on the working directory alone.
//! Skips without TeX Live.
#![cfg(all(feature = "kpathsea", feature = "distribution"))]

mod common;

use flashtex_engine::formats::FormatCache;
use flashtex_engine::resolver::{find_texlive_bin, FileResolver, Format, KpathseaResolver};
use std::path::PathBuf;

#[test]
fn files_added_to_texmfhome_and_texinputs_shadow_at_once() {
    let Some(bin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let d = common::fresh_dir("flashtex-shadow");
    let home = d.join("texmf");
    let styles = d.join("styles");
    for s in [
        home.join("tex/latex"),
        home.join("tex/generic"),
        styles.join("sub"),
        d.join("cache"),
    ] {
        std::fs::create_dir_all(s).unwrap();
    }
    // Before any kpathsea instance starts (one test in this binary).
    std::env::set_var("TEXMFHOME", &home);
    std::env::set_var("TEXINPUTS", format!("{}//:", styles.display()));
    std::env::remove_var("FLASHTEX_FORMATS");
    let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "flashtex");
    let canon = |p: PathBuf| std::fs::canonicalize(p).unwrap();

    // 1. TEXMFHOME.
    let dist = r.find_ex("verbatim.sty", Format::Tex, true).0.unwrap();
    assert!(!dist.starts_with(&home), "{}", dist.display());
    assert_eq!(r.find("verbatim.sty", Format::Tex), Some(dist.clone()));
    assert!(
        r.depends_on("verbatim.sty", Format::Tex)
            .iter()
            .any(|x| canon(x.into()) == canon(home.join("tex/latex"))),
        "the read set records TEXMFHOME's directories: {:?}",
        r.depends_on("verbatim.sty", Format::Tex)
    );
    let mine = home.join("tex/latex/verbatim.sty");
    std::fs::copy(&dist, &mine).unwrap();
    for (what, now) in [
        ("find", r.find("verbatim.sty", Format::Tex)),
        ("find_ex", r.find_ex("verbatim.sty", Format::Tex, true).0),
    ] {
        let now = now.unwrap();
        assert_eq!(
            canon(now.clone()),
            canon(mine.clone()),
            "{what}: {}",
            now.display()
        );
    }

    // 2. A subdirectory of a user TEXINPUTS entry.
    let dist = r.find("article.cls", Format::Tex).unwrap();
    assert!(!dist.starts_with(&styles));
    let mine = styles.join("sub/article.cls");
    std::fs::copy(&dist, &mine).unwrap();
    let now = r.find("article.cls", Format::Tex).unwrap();
    assert_eq!(canon(now.clone()), canon(mine.clone()), "{}", now.display());
    std::fs::remove_file(&mine).unwrap();
    std::fs::remove_file(home.join("tex/latex/verbatim.sty")).unwrap();
    assert_eq!(r.find("article.cls", Format::Tex), Some(dist));

    // 3. The format cache and hyphen.cfg.
    let mut c = FormatCache::new(d.join("cache"));
    c.engine_exe = Some(PathBuf::from(env!("CARGO_BIN_EXE_flashtex-initex")));
    c.env = vec![(
        "FLASHTEX_POOL".into(),
        concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool").into(),
    )];
    let p1 = c.ensure("pdflatex", "pdflatex", &mut r).unwrap();
    assert!(c.last.built);
    let p2 = c.ensure("pdflatex", "pdflatex", &mut r).unwrap();
    assert!(!c.last.built, "a hit: {:?}", c.last.stale_reason);
    assert_eq!(p1, p2);
    let dist = r.find_ex("hyphen.cfg", Format::Tex, true).0.unwrap();
    std::fs::copy(&dist, home.join("tex/generic/hyphen.cfg")).unwrap();
    c.ensure("pdflatex", "pdflatex", &mut r).unwrap();
    assert!(
        c.last.built
            && c.last
                .stale_reason
                .as_deref()
                .is_some_and(|s| s.contains("hyphen.cfg")),
        "hyphen.cfg in TEXMFHOME rebuilds the format: built {}, {:?}",
        c.last.built,
        c.last.stale_reason
    );

    // 4. The resident host: a package added to TEXMFHOME after S₀ is read
    //    by the next compile, though only the body changed. Before, a lookup
    //    that found a distribution file recorded no directory in S₀'s key,
    //    so S₀ was kept and the old package stayed in.
    let fmt = d.join("fmt");
    let work = d.join("doc");
    std::fs::create_dir_all(&fmt).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::fs::copy(&p1, fmt.join("pdflatex.fmt")).unwrap_or_else(|_| {
        std::fs::copy(
            c.ensure("pdflatex", "pdflatex", &mut r).unwrap(),
            fmt.join("pdflatex.fmt"),
        )
        .unwrap()
    });
    let doc = |w: &str| {
        format!(
            "\\documentclass{{article}}\n\\usepackage{{verbatim}}\n\\begin{{document}}\n\
             {w} text.\\par\\newpage More.\n\\end{{document}}\n"
        )
    };
    std::fs::write(work.join("doc.tex"), doc("First")).unwrap();
    let mut h = std::process::Command::new(env!("CARGO_BIN_EXE_flashtex-host"))
        .args([
            "iserve",
            "--",
            "-fmt=pdflatex",
            "-interaction=nonstopmode",
            "doc.tex",
        ])
        .current_dir(&work)
        .env(
            "FLASHTEX_POOL",
            concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool"),
        )
        .env("FLASHTEX_FORMATS", &fmt)
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut input = h.stdin.take().unwrap();
    let mut output = std::io::BufReader::new(h.stdout.take().unwrap());
    let mut compile = || {
        use std::io::{BufRead, Write};
        writeln!(input, "compile").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        assert!(line.contains("\"status\":0"), "{line}");
        line
    };
    let log = || std::fs::read_to_string(work.join("doc.log")).unwrap();
    compile();
    compile();
    assert!(log().contains("verbatim.sty") && !log().contains("flashtex-shadow"));
    let dist = r.find("verbatim.sty", Format::Tex).unwrap();
    let text = std::fs::read_to_string(&dist).unwrap();
    std::fs::write(
        home.join("tex/latex/verbatim.sty"),
        format!("\\typeout{{flashtex-shadow}}\n{text}"),
    )
    .unwrap();
    std::fs::write(work.join("doc.tex"), doc("Second")).unwrap();
    let report = compile();
    assert!(
        log().contains("flashtex-shadow"),
        "the TEXMFHOME package was not read: {report}"
    );
    drop(compile);
    let _ = h.kill();
    let _ = h.wait();
    let _ = std::fs::remove_dir_all(&d);
}
