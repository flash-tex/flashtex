//! The format cache's validation through the process's resolver
//! (src/formats.rs, `lookupproof`): within one process, a hit checks each
//! recorded lookup against the directories it searched instead of making
//! it again, and is still stale when a file a lookup now finds appears
//! (`optional.cfg`, which the ini file tests for), and when it goes again.
//!
//! Hermetic, as tests/format_cache.rs: kpathsea over one directory
//! (`FLASHTEX_RESOLVER=bundle`, `FLASHTEX_BUNDLE`), searched on disk. Its
//! own test binary: it sets the process's resolver.
#![cfg(feature = "distribution")]

mod common;

use flashtex_engine::formats::FormatCache;
use std::path::PathBuf;

#[test]
fn a_hit_checks_lookups_against_their_directories_and_sees_a_new_file() {
    let d = common::fresh_dir("flashtex-fmtcache-proofs");
    let _ = std::fs::remove_dir_all(&d);
    let tex = d.join("tree");
    std::fs::create_dir_all(&tex).unwrap();
    std::fs::write(tex.join("fmtutil.cnf"), "mini pdftex - *mini.ini\n").unwrap();
    std::fs::write(
        tex.join("mini.ini"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\input minidep \
         \\openin1=optional.cfg \\ifeof1 \\else \\message{optional}\\fi \\dump\n",
    )
    .unwrap();
    std::fs::write(tex.join("minidep.tex"), "\\def\\hello{hi}\n").unwrap();
    // (well in the past: a signature that is not racy, so a hit can take
    // the directory's listing from the last check)
    let set_back = |s: u64| {
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(s);
        std::fs::File::open(&tex).unwrap().set_modified(t).unwrap();
    };
    set_back(900);
    std::env::set_var("FLASHTEX_RESOLVER", "bundle");
    std::env::set_var("FLASHTEX_BUNDLE", &tex);
    let mut c = FormatCache::new(d.join("cache"));
    c.engine_exe = Some(PathBuf::from(env!("CARGO_BIN_EXE_flashtex-initex")));
    c.env = vec![
        (
            "FLASHTEX_POOL".into(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool").into(),
        ),
        ("FLASHTEX_RESOLVER".into(), "bundle".into()),
        ("FLASHTEX_BUNDLE".into(), tex.display().to_string()),
    ];
    let ensure = |c: &mut FormatCache| {
        flashtex_engine::system::with_resolver_for("mini", |r| c.ensure("mini", "mini", r)).unwrap()
    };
    let p1 = ensure(&mut c);
    assert!(c.last.built, "first use builds");
    // a hit, its lookups made again (and their proofs kept)
    assert_eq!(ensure(&mut c), p1);
    assert!(!c.last.built);
    assert!(c.last.lookups_checked > 0);
    // a hit from the proofs
    assert_eq!(ensure(&mut c), p1);
    assert!(!c.last.built);
    assert_eq!(
        c.last.lookups_held, c.last.lookups_checked,
        "every lookup held by its proof: {:?}",
        c.last
    );
    // a file the ini file tests for appears: stale, rebuilt
    std::fs::write(tex.join("optional.cfg"), "\\relax\n").unwrap();
    set_back(800);
    let p2 = ensure(&mut c);
    assert!(c.last.built, "{:?}", c.last);
    assert!(
        c.last
            .stale_reason
            .as_deref()
            .unwrap_or_default()
            .contains("optional.cfg"),
        "{:?}",
        c.last.stale_reason
    );
    assert_ne!(p2, p1);
    assert_eq!(ensure(&mut c), p2);
    assert_eq!(ensure(&mut c), p2);
    assert!(!c.last.built);
    // ... and goes again: the first key back
    std::fs::remove_file(tex.join("optional.cfg")).unwrap();
    set_back(700);
    assert_eq!(ensure(&mut c), p1);
    assert!(c.last.built, "{:?}", c.last);
    let _ = std::fs::remove_dir_all(&d);
}
