//! A file added where kpathsea searches on disk shadows the one a lookup
//! found before, at once, in this process: the resolver's kept lookups
//! (src/resolver.rs `Found`) depend on every directory kpathsea reads on
//! disk, not only the working directory (#1493 review; on main through the
//! lookup memo, #1561, and #1680). One process (TEXMFHOME and TEXINPUTS are
//! set before kpathsea starts); between steps the resolver refreshes what it
//! cached of the disk (`FileResolver::refresh_disk_dirs`), as every compile
//! starts by doing:
//!
//! 1. a package added to `$TEXMFHOME/tex/latex`;
//! 2. a class added to a subdirectory of a user TEXINPUTS entry (`dir//`);
//! 3. `hyphen.cfg` added to TEXMFHOME: the format cache must rebuild
//!    `pdflatex.fmt` (src/formats.rs, "What the cache is keyed by");
//! 4. a resident host reads a TEXMFHOME package added after S₀;
//! 5. a package in a subdirectory of TEXMFHOME made after kpathsea expanded
//!    it, and 6. one under a TEXINPUTS `dir//` whose `dir` did not exist at
//!    the start: kpathsea caches each `//` expansion for the process, so
//!    these were never found before (#1493 re-review); a fresh process
//!    finds them.
//!
//! 7. TEXINPUTS `A:B:`, a lookup of `Sub/x.sty` that finds `B/Sub/x.sty`
//!    while `A/sub/` exists: on a case-insensitive file system (APFS, NTFS)
//!    `A/sub/x.sty`, added later, comes first (#1493 final review: the
//!    directory listing compared names exactly and left `A/sub/` out).
//!
//! 1-3 fail with a cache that depends on the working directory alone, 4
//! without the read set's directories, 5-6 without forgetting kpathsea's
//! expansions, 7 with an exact-name listing. Skips without TeX Live.
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
    let later = d.join("later");
    let (ca, cb) = (d.join("ca"), d.join("cb"));
    for s in [
        ca.join("sub"),
        cb.join("Sub"),
        home.join("tex/latex"),
        home.join("tex/generic"),
        styles.join("sub"),
        d.join("cache"),
    ] {
        std::fs::create_dir_all(s).unwrap();
    }
    // Before any kpathsea instance starts (one test in this binary).
    std::env::set_var("TEXMFHOME", &home);
    std::env::set_var(
        "TEXINPUTS",
        format!(
            "{}:{}:{}//:{}//:",
            ca.display(),
            cb.display(),
            styles.display(),
            later.display()
        ),
    );
    std::env::remove_var("FLASHTEX_FORMATS");
    let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "flashtex");
    let canon = |p: PathBuf| std::fs::canonicalize(p).unwrap();

    // 1. TEXMFHOME.
    let dist = r.find_ex("verbatim.sty", Format::Tex, true).0.unwrap();
    assert!(!dist.starts_with(&home), "{}", dist.display());
    assert_eq!(r.find("verbatim.sty", Format::Tex), Some(dist.clone()));
    let mine = home.join("tex/latex/verbatim.sty");
    std::fs::copy(&dist, &mine).unwrap();
    r.refresh_disk_dirs();
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
    r.refresh_disk_dirs();
    let now = r.find("article.cls", Format::Tex).unwrap();
    assert_eq!(canon(now.clone()), canon(mine.clone()), "{}", now.display());
    std::fs::remove_file(&mine).unwrap();
    std::fs::remove_file(home.join("tex/latex/verbatim.sty")).unwrap();
    r.refresh_disk_dirs();
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
    // (the format cache refreshes the disk itself: FormatCache::validate)
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

    // 5. A subdirectory of TEXMFHOME made after kpathsea expanded it.
    assert_eq!(r.find("flashtex-later-a.sty", Format::Tex), None);
    std::fs::create_dir_all(home.join("tex/latex/newpkg")).unwrap();
    std::fs::write(
        home.join("tex/latex/newpkg/flashtex-later-a.sty"),
        "\\relax\n",
    )
    .unwrap();
    r.refresh_disk_dirs();
    let now = r.find("flashtex-later-a.sty", Format::Tex);
    assert_eq!(
        now.map(canon),
        Some(canon(home.join("tex/latex/newpkg/flashtex-later-a.sty"))),
        "a new TEXMFHOME subdirectory is searched"
    );
    // 6. A TEXINPUTS `dir//` whose `dir` was missing at the start.
    assert_eq!(r.find("flashtex-later-b.sty", Format::Tex), None);
    std::fs::create_dir_all(later.join("sub")).unwrap();
    std::fs::write(later.join("sub/flashtex-later-b.sty"), "\\relax\n").unwrap();
    r.refresh_disk_dirs();
    let now = r.find("flashtex-later-b.sty", Format::Tex);
    assert_eq!(
        now.map(canon),
        Some(canon(later.join("sub/flashtex-later-b.sty"))),
        "a TEXINPUTS directory made after the start is searched"
    );

    // 7. Case-insensitive names: `Sub/x.sty` with `ca/sub/` present.
    std::fs::write(cb.join("Sub/flashtex-case.sty"), "\\relax\n").unwrap();
    r.refresh_disk_dirs();
    let first = r.find("Sub/flashtex-case.sty", Format::Tex).unwrap();
    assert_eq!(canon(first), canon(cb.join("Sub/flashtex-case.sty")));
    std::fs::write(ca.join("sub/flashtex-case.sty"), "\\relax\n").unwrap();
    r.refresh_disk_dirs();
    let now = r.find("Sub/flashtex-case.sty", Format::Tex).unwrap();
    // What a fresh resolver finds is the reference: `ca/sub/` on a
    // case-insensitive file system, `cb/Sub/` on a case-sensitive one.
    let fresh = KpathseaResolver::for_texlive(&bin, "pdflatex", "flashtex")
        .find("Sub/flashtex-case.sty", Format::Tex)
        .unwrap();
    assert_eq!(
        canon(now),
        canon(fresh),
        "a kept lookup and a fresh one agree"
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
