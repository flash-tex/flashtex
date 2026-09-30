//! The bundle fallback (src/bundle/, DESIGN.md 4.4) against a local fixture
//! bundle served over HTTP from this process: no network, deterministic.
//! One test function, because kpathsea's configuration is process-wide.
#![cfg(feature = "distribution")]

use flashtex_engine::bundle::serve::FixtureServer;
use flashtex_engine::bundle::ttb::{self, PackFile};
use flashtex_engine::bundle::{BundleResolver, BundleSpec, PACKAGE_FETCH_LIMIT};
use flashtex_engine::formats::hex;
use flashtex_engine::resolver::{FileResolver, Format};
use std::path::{Path, PathBuf};
use std::process::Command;

fn f(path: &str, data: &[u8], package: &str) -> PackFile {
    PackFile {
        path: path.into(),
        data: data.to_vec(),
        package: package.into(),
    }
}

/// A pseudo-random, incompressible payload (so package sizes are real).
fn noise(n: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2654435761).max(1);
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

fn fixture() -> Vec<PackFile> {
    vec![
        // core
        f(
            "texmf-dist/tex/latex/base/article.cls",
            b"% article\n",
            "latex",
        ),
        f(
            "texmf-dist/tex/latex/base/size10.clo",
            b"% size10\n",
            "latex",
        ),
        f(
            "texmf-dist/fonts/tfm/public/cm/cmr10.tfm",
            &noise(1200, 1),
            "cm",
        ),
        // a small package: fetched whole
        f(
            "texmf-dist/tex/latex/amsmath/amsmath.sty",
            b"% amsmath\n",
            "amsmath",
        ),
        f(
            "texmf-dist/tex/latex/amsmath/amsgen.sty",
            b"% amsgen\n",
            "amsmath",
        ),
        f(
            "texmf-dist/tex/latex/amsmath/amsbsy.sty",
            b"% amsbsy\n",
            "amsmath",
        ),
        // a large package: fetched file by file
        f(
            "texmf-dist/fonts/type1/public/big/big1.pfb",
            &noise(PACKAGE_FETCH_LIMIT as usize, 2),
            "bigfonts",
        ),
        f(
            "texmf-dist/fonts/type1/public/big/big2.pfb",
            &noise(PACKAGE_FETCH_LIMIT as usize, 3),
            "bigfonts",
        ),
        // suffix rules
        f(
            "texmf-dist/tex/generic/story/story.tex",
            b"% story\n",
            "story",
        ),
        f("texmf-dist/tex/generic/story/noext", b"% noext\n", "story"),
        f("texmf-dist/tex/generic/empty/empty.tex", b"", "empty"),
    ]
}

fn content(p: &Path) -> Vec<u8> {
    std::fs::read(p).unwrap()
}

/// Where a fixture file lives in the bundle's tree: its TeX Live path.
fn at(tree: &Path, name: &str) -> PathBuf {
    let f = fixture()
        .into_iter()
        .find(|f| f.path.rsplit('/').next() == Some(name))
        .unwrap();
    tree.join(f.path)
}

#[test]
fn fetch_verify_packages_offline_and_engine() {
    let base = std::env::temp_dir().join(format!("flashtex-bundle-fetch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let (bytes, ix) = ttb::pack(
        fixture(),
        &["/texmf-dist//".into()],
        &["latex".into(), "cm".into()],
    );
    let digest = hex(&ix.digest());
    let server = FixtureServer::start(bytes.clone(), "fixture.ttb").unwrap();
    let spec = |offline| BundleSpec {
        url: server.url.clone(),
        digest: digest.clone(),
        offline,
    };

    // A wrong pin is refused before anything is stored.
    let mut wrong = spec(false);
    wrong.digest = "0".repeat(64);
    let e = BundleResolver::open(wrong, &base.join("c-wrong"), "pdflatex", "")
        .err()
        .unwrap();
    assert!(e.contains("not the pinned"), "{e}");

    // Offline with nothing cached: an error, not a fetch.
    let r0 = server.requests();
    let e = BundleResolver::open(spec(true), &base.join("c1"), "pdflatex", "")
        .err()
        .unwrap();
    assert!(e.contains("offline"), "{e}");
    assert_eq!(server.requests(), r0);

    // Cold open: header, index, and the core in one request.
    let cache = base.join("c1");
    let mut r = BundleResolver::open(spec(false), &cache, "pdflatex", "").unwrap();
    assert_eq!(r.bundle.stats.requests, 3, "{:?}", r.bundle.stats);
    assert_eq!(server.requests() - r0, 3);
    let files = r.bundle.files_dir.clone();
    assert_eq!(content(&at(&files, "article.cls")), b"% article\n");
    assert_eq!(content(&at(&files, "cmr10.tfm")), noise(1200, 1));
    assert!(r.describe().contains(&digest));

    // Core files: no request.
    let r1 = server.requests();
    let p = r.find("article.cls", Format::Tex).unwrap();
    assert_eq!(p, at(&files, "article.cls"));
    assert_eq!(
        r.find("cmr10", Format::Tfm).unwrap(),
        at(&files, "cmr10.tfm")
    );
    assert_eq!(server.requests(), r1);

    // A small package: one request brings all of it.
    let p = r.find("amsmath.sty", Format::Tex).unwrap();
    assert_eq!(content(&p), b"% amsmath\n");
    assert_eq!(server.requests(), r1 + 1);
    assert_eq!(
        content(&r.find("amsbsy.sty", Format::Tex).unwrap()),
        b"% amsbsy\n"
    );
    assert_eq!(server.requests(), r1 + 1);

    // A large package: file by file.
    let p = r.find("big1.pfb", Format::Type1).unwrap();
    assert_eq!(content(&p), noise(PACKAGE_FETCH_LIMIT as usize, 2));
    assert_eq!(server.requests(), r1 + 2);
    assert!(!r.bundle.is_present(
        r.bundle
            .index
            .files
            .iter()
            .position(|e| e.path.ends_with("big2.pfb"))
            .unwrap()
    ));

    // kpathsea's suffix rules, over placeholders.
    assert_eq!(
        r.find("story", Format::Tex).unwrap(),
        at(&files, "story.tex")
    );
    assert_eq!(content(&at(&files, "story.tex")), b"% story\n");
    assert_eq!(r.find("noext", Format::Tex).unwrap(), at(&files, "noext"));
    assert_eq!(r.find("story", Format::Tfm), None);
    assert_eq!(r.find("missing", Format::Tex), None);
    assert_eq!(
        r.find("empty", Format::Tex).unwrap(),
        at(&files, "empty.tex")
    );
    // A placeholder that was never fetched (big2) is not a file of the bundle's yet.
    let big2 = at(&files, "big2.pfb");
    r.bundle.prepare_lookup("big2.pfb");
    assert_eq!(std::fs::metadata(&big2).unwrap().len(), 0);
    let requests_online = server.requests();
    drop(r);

    // Offline on the same cache: what was fetched is there; nothing else is.
    let mut off = BundleResolver::open(spec(true), &cache, "pdflatex", "").unwrap();
    assert_eq!(
        content(&off.find("amsgen.sty", Format::Tex).unwrap()),
        b"% amsgen\n"
    );
    assert_eq!(off.find("big2.pfb", Format::Type1), None);
    assert_eq!(server.requests(), requests_online);

    // Online again on the same cache: no index refetch; big2 on demand.
    let mut on = BundleResolver::open(spec(false), &cache, "pdflatex", "").unwrap();
    assert_eq!(on.bundle.stats.requests, 0);
    let p = on.find("big2.pfb", Format::Type1).unwrap();
    assert_eq!(content(&p), noise(PACKAGE_FETCH_LIMIT as usize, 3));
    assert_eq!(server.requests(), requests_online + 1);

    // A corrupted member is refused (its SHA-256 does not match).
    let mut bad = bytes.clone();
    let e = ix
        .files
        .iter()
        .find(|e| e.path.ends_with("story.tex"))
        .unwrap();
    // Flip a byte of the gzip trailer's CRC and the payload: zlib or the hash rejects it.
    let at = (e.start + e.gzip_len as u64 - 5) as usize;
    bad[at] ^= 0xff;
    let bad_server = FixtureServer::start(bad, "bad.ttb").unwrap();
    let mut spec_bad = spec(false);
    spec_bad.url = bad_server.url.clone();
    let mut rb = BundleResolver::open(spec_bad, &base.join("c-bad"), "pdflatex", "").unwrap();
    assert_eq!(rb.find("story", Format::Tex), None);
    assert!(rb.find("article.cls", Format::Tex).is_some());

    // file:// works the same way.
    let ttb_path = base.join("fixture.ttb");
    std::fs::write(&ttb_path, &bytes).unwrap();
    let mut spec_file = spec(false);
    spec_file.url = format!("file://{}", ttb_path.display());
    let mut rf = BundleResolver::open(spec_file, &base.join("c-file"), "pdflatex", "").unwrap();
    assert_eq!(
        content(&rf.find("amsgen.sty", Format::Tex).unwrap()),
        b"% amsgen\n"
    );

    engine_builds_its_format_from_a_bundle(&base, false);
    engine_builds_its_format_from_a_bundle(&base, true);
    let _ = std::fs::remove_dir_all(&base);
}

/// The engine end to end on a bundle: its format is built by the format
/// cache from the bundle's fmtutil.cnf and ini file, fetched on demand.
/// With `cnf`, the bundle carries a texmf.cnf, which kpathsea then reads
/// (as it reads TeX Live's in a real bundle): its search paths are used and
/// its `max_print_line = 61` shows in the terminal output.
fn engine_builds_its_format_from_a_bundle(base: &Path, cnf: bool) {
    let mut files = vec![
        f("texmf-dist/web2c/fmtutil.cnf", b"mini pdftex - *mini.ini\n", "kpathsea"),
        f(
            "texmf-dist/tex/mini/mini.ini",
            b"\\catcode`\\{=1 \\catcode`\\}=2 \\input minidep \\dump\n",
            "mini",
        ),
        f("texmf-dist/tex/mini/minidep.tex", b"\\def\\hello{Hello from the bundle: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx}\n", "mini"),
        f("texmf-dist/tex/mini/story.tex", b"\\immediate\\write16{\\hello}\\end\n", "story"),
    ];
    if cnf {
        files.push(f(
            "texmf-dist/web2c/texmf.cnf",
            b"TEXMF = !!$TEXMFDIST\nTEXMFDBS = $TEXMFDIST\nTEXINPUTS = .;$TEXMF/tex//\n\
              TEXFORMATS = .;$TEXMF/web2c\nWEB2C = $TEXMF/web2c\nmax_print_line = 61\n",
            "kpathsea",
        ));
    }
    let (bytes, ix) = ttb::pack(files, &["/texmf-dist//".into()], &["kpathsea".into()]);
    let digest = hex(&ix.digest());
    let server = FixtureServer::start(bytes.clone(), "mini.ttb").unwrap();
    let work = base.join(if cnf { "engine-cnf" } else { "engine" });
    std::fs::create_dir_all(work.join("doc")).unwrap();
    std::fs::write(work.join("mini.ttb"), &bytes).unwrap();
    let bin: PathBuf = work.join("pdftex");
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_flashtex-initex"), &bin).unwrap();
    let run = |offline: &str| {
        Command::new(&bin)
            .args(["-fmt=mini", "-interaction=nonstopmode", "story"])
            // A fresh process environment, as the engine has in use: the
            // resolvers above put kpathsea variables into this process's.
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .current_dir(work.join("doc"))
            .env(
                "FLASHTEX_POOL",
                concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool"),
            )
            .env("FLASHTEX_RESOLVER", "bundle")
            .env("FLASHTEX_BUNDLE_URL", &server.url)
            .env("FLASHTEX_BUNDLE_DIGEST", &digest)
            .env("FLASHTEX_BUNDLE_OFFLINE", offline)
            .env("FLASHTEX_BUNDLE_CACHE_DIR", work.join("bundles"))
            .env("FLASHTEX_FORMAT_CACHE_DIR", work.join("formats"))
            .env_remove("FLASHTEX_FORMATS")
            .output()
            .unwrap()
    };
    let o = run("0");
    let out = String::from_utf8_lossy(&o.stdout);
    let x = "x".repeat(38);
    let hello = if cnf {
        format!("Hello from the bundle: {x}\nxxxxxxxxxxxx")
    } else {
        format!("Hello from the bundle: {x}xxxxxxxxxxxx")
    };
    assert!(
        out.contains(&hello),
        "{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let cold = server.requests();
    assert!(cold >= 3, "header, index, core and the rest: {cold}");
    // Warm and offline: the cached format and files, no requests.
    let o = run("1");
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains(&hello),
        "{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(server.requests(), cold);
}
