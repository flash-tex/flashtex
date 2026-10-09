//! The font map's disk cache (src/pdftex/mapfile.rs `disk`): a run that
//! takes a map file's parse from the cache writes the same PDF and log as a
//! run that parses it, a changed map file is parsed again, and a parse that
//! warned is never cached, so its warning is printed every time. Plain TeX
//! with a map file of the test's own (`\pdfmapfile`), the format built with
//! this engine. Skips where there is no TeX Live (the fonts come from it).
#![cfg(all(feature = "kpathsea", feature = "distribution"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const DOC: &str = "\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfmapfile{test.map}\n\
    Hello, world! {\\bf Bold}\n\\bye\n";

/// Run the engine in `dir`; `cache`: the font map cache's directory, or
/// none (off). Returns its stderr.
fn run(dir: &Path, args: &[&str], cache: Option<&Path>) -> String {
    let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    c.args(args)
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_DEBUG_FONTMAP", "1")
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    match cache {
        Some(d) => c
            .env("FLASHTEX_FORMAT_CACHE_DIR", d)
            .env_remove("FLASHTEX_FORMAT_CACHE"),
        None => c.env("FLASHTEX_FORMAT_CACHE", "off"),
    };
    let o = c.output().unwrap();
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn outputs(dir: &Path) -> (Vec<u8>, String) {
    (
        std::fs::read(dir.join("doc.pdf")).unwrap_or_default(),
        std::fs::read_to_string(dir.join("doc.log")).unwrap_or_default(),
    )
}

fn entries(cache: &Path) -> usize {
    std::fs::read_dir(cache.join("fontmaps")).map_or(0, |d| {
        d.flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "bin"))
            .count()
    })
}

#[test]
fn a_cached_parse_writes_what_a_parse_writes() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let base = common::fresh_dir("flashtex-fontmap-cache");
    let cache = base.join("cache");
    let job = base.join("job");
    std::fs::create_dir_all(&job).unwrap();
    std::fs::write(job.join("doc.tex"), DOC).unwrap();
    run(&job, &["-ini", "\\input plain \\dump"], None);

    // 1. Parsed (cache off), parsed and stored, then taken from the disk.
    std::fs::write(
        job.join("test.map"),
        "cmr10 CMR10 <cmr10.pfb\ncmbx10 CMBX10 <cmbx10.pfb\n",
    )
    .unwrap();
    let say = run(&job, &["&plain doc"], None);
    assert!(!say.contains("[fontmap]"), "the cache is off: {say}");
    let parsed = outputs(&job);
    assert!(!parsed.0.is_empty(), "no PDF");
    let say = run(&job, &["&plain doc"], Some(&cache));
    assert!(say.contains("[fontmap] stored"), "{say}");
    assert_eq!(entries(&cache), 1);
    assert!(
        outputs(&job) == parsed,
        "a storing run differs from a parse"
    );
    let say = run(&job, &["&plain doc"], Some(&cache));
    assert!(say.contains("[fontmap] disk hit"), "{say}");
    assert!(
        outputs(&job) == parsed,
        "a run from the cache differs from a parse"
    );

    // 2. The map file changed: parsed again, and what the new file says.
    std::fs::write(
        job.join("test.map"),
        "cmr10 CMR10 \"0.167 SlantFont\" <cmr10.pfb\ncmbx10 CMBX10 <cmbx10.pfb\n",
    )
    .unwrap();
    let say = run(&job, &["&plain doc"], Some(&cache));
    assert!(
        !say.contains("[fontmap] disk hit"),
        "a changed map file was taken from the cache: {say}"
    );
    let changed = outputs(&job);
    run(&job, &["&plain doc"], None);
    assert!(
        outputs(&job) == changed,
        "the changed map's run differs from a parse"
    );
    assert!(changed.0 != parsed.0, "the slant did not change the PDF");

    // 3. A parse that warns (a duplicate entry) is not cached: the warning
    // is printed by every run.
    std::fs::write(
        job.join("test.map"),
        "cmr10 CMR10 <cmr10.pfb\ncmr10 CMR10 <cmr10.pfb\ncmbx10 CMBX10 <cmbx10.pfb\n",
    )
    .unwrap();
    let before = entries(&cache);
    for _ in 0..2 {
        let say = run(&job, &["&plain doc"], Some(&cache));
        assert!(!say.contains("[fontmap] disk hit"), "{say}");
        assert!(
            !say.contains("[fontmap] stored"),
            "a parse that warned was stored: {say}"
        );
        let (_, log) = outputs(&job);
        assert!(
            log.contains("duplicates ignored"),
            "no duplicate warning in the log:\n{log}"
        );
    }
    assert_eq!(entries(&cache), before);
    let _ = std::fs::remove_dir_all(&base);
}
