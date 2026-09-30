//! The format cache (src/formats.rs, DESIGN.md 4.4): miss, hit, a touched
//! file (rehash, still a hit), a changed file and a new file that a lookup
//! now finds (rebuild), and two processes asking at once (one build).
//!
//! Hermetic: a tiny TeX tree with its own `fmtutil.cnf` and ini file, read
//! through kpathsea over one directory (`FLASHTEX_RESOLVER=bundle`,
//! `FLASHTEX_BUNDLE`), so CI runs it without TeX Live. The INITEX runs are
//! this crate's `flashtex-initex`. Its own test binary, because kpathsea's
//! configuration is process-wide.
#![cfg(feature = "distribution")]

use flashtex_engine::formats::FormatCache;
use flashtex_engine::resolver::KpathseaResolver;
use std::path::{Path, PathBuf};
use std::process::Command;

/// kpathsea keeps its configuration in the process environment
/// (`kpathsea_xputenv` is `putenv`), which is not safe against another
/// thread reading the environment at the same time (spawning a process
/// does): with the tests of this binary on parallel threads, glibc crashed
/// (SIGSEGV in CI, merge-queue runs of #1216 and #1217). They run one at a
/// time.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

fn tree(name: &str) -> (PathBuf, PathBuf) {
    let d = std::env::temp_dir().join(format!("flashtex-fmtcache-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let tex = d.join("tree");
    std::fs::create_dir_all(&tex).unwrap();
    std::fs::write(
        tex.join("fmtutil.cnf"),
        "# test\nmini pdftex - *mini.ini\n#! off pdftex - *mini.ini\nother etex - mini.ini\n",
    )
    .unwrap();
    std::fs::write(
        tex.join("mini.ini"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\input minidep \
         \\openin1=optional.cfg \\ifeof1 \\else \\message{optional}\\fi \\dump\n",
    )
    .unwrap();
    std::fs::write(tex.join("minidep.tex"), "\\def\\hello{hi}\n").unwrap();
    std::fs::write(
        tex.join("story.tex"),
        "\\immediate\\write16{[\\hello]}\\end\n",
    )
    .unwrap();
    (d.clone(), tex)
}

fn cache(d: &Path) -> FormatCache {
    let mut c = FormatCache::new(d.join("cache"));
    c.engine_exe = Some(PathBuf::from(env!("CARGO_BIN_EXE_flashtex-initex")));
    c.env = vec![
        (
            "FLASHTEX_POOL".into(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool").into(),
        ),
        ("FLASHTEX_RESOLVER".into(), "bundle".into()),
    ];
    c
}

#[test]
fn miss_hit_touch_change_and_shadow() {
    let _serial = serial();
    let (d, tex) = tree("seq");
    let mut r = KpathseaResolver::for_bundle(&tex, "mini", "");
    let mut c = cache(&d);
    c.env
        .push(("FLASHTEX_BUNDLE".into(), tex.display().to_string()));

    // Only enabled pdftex lines are this engine's.
    assert!(matches!(
        c.ensure("off", "off", &mut r),
        Err(flashtex_engine::formats::FormatError::NotInFmtutil(_))
    ));
    assert!(matches!(
        c.ensure("other", "other", &mut r),
        Err(flashtex_engine::formats::FormatError::NotInFmtutil(_))
    ));

    // Miss: built.
    let p1 = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(c.last.built, "first use builds");
    assert!(std::fs::metadata(&p1).unwrap().len() > 0);
    let manifest = std::fs::read_to_string(p1.parent().unwrap().join("manifest")).unwrap();
    assert!(manifest.contains("/minidep.tex\n"), "{manifest}");
    assert!(manifest.contains("/mini.ini\n"), "{manifest}");
    assert!(
        manifest.contains("\toptional.cfg\t\n"),
        "the failed lookup is recorded: {manifest}"
    );

    // Hit.
    let p2 = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(!c.last.built);
    assert_eq!(p1, p2);
    assert_eq!(c.last.files_rehashed, 0);
    assert!(c.last.lookups_checked > 0);

    // Touched, same content: rehashed once, still a hit; then stat matches again.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(tex.join("minidep.tex"), "\\def\\hello{hi}\n").unwrap();
    let p3 = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(!c.last.built);
    assert_eq!(p3, p1);
    assert_eq!(c.last.files_rehashed, 1);
    c.ensure("mini", "mini", &mut r).unwrap();
    assert_eq!(
        c.last.files_rehashed, 0,
        "the manifest's stat signature was refreshed"
    );

    // Changed content: rebuilt under a new key; the previous format is kept.
    std::fs::write(tex.join("minidep.tex"), "\\def\\hello{changed}\n").unwrap();
    let p4 = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(c.last.built);
    assert!(c
        .last
        .stale_reason
        .as_deref()
        .unwrap()
        .contains("minidep.tex changed"));
    assert_ne!(p4, p1);
    assert!(p1.exists(), "one previous generation stays for readers");

    // A file a lookup now finds: rebuilt.
    std::fs::write(tex.join("optional.cfg"), "\\relax\n").unwrap();
    let p5 = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(c.last.built);
    assert!(
        c.last
            .stale_reason
            .as_deref()
            .unwrap()
            .contains("optional.cfg"),
        "{:?}",
        c.last.stale_reason
    );
    assert_ne!(p5, p4);
    assert!(!p1.exists(), "two generations back is removed");
    assert!(p4.exists());

    // Removed again: the old key comes back (content-addressed).
    std::fs::remove_file(tex.join("optional.cfg")).unwrap();
    let p6 = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(c.last.built);
    assert_eq!(p6, p4);

    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn two_processes_build_once() {
    let _serial = serial();
    let (d, tex) = tree("conc");
    let dist = env!("CARGO_BIN_EXE_flashtex-dist");
    let run = || {
        Command::new(dist)
            .args(["format", "mini"])
            .env(
                "FLASHTEX_POOL",
                concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool"),
            )
            .env("FLASHTEX_RESOLVER", "bundle")
            .env("FLASHTEX_BUNDLE", &tex)
            .env("FLASHTEX_FORMAT_CACHE_DIR", d.join("cache"))
            .output()
            .unwrap()
    };
    let (a, b) = std::thread::scope(|s| {
        let a = s.spawn(run);
        let b = s.spawn(run);
        (a.join().unwrap(), b.join().unwrap())
    });
    let outs: Vec<String> = [a, b]
        .iter()
        .map(|o| {
            assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
            String::from_utf8_lossy(&o.stdout).into_owned()
        })
        .collect();
    let built = outs.iter().filter(|o| o.contains("mini: built")).count();
    let hits = outs
        .iter()
        .filter(|o| o.contains("mini: cache hit"))
        .count();
    assert_eq!((built, hits), (1, 1), "{outs:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The cache is keyed by the engine's build id, not by the binary: a format
/// `flashtex-dist` prepared is a hit for `flashtex-initex` (as for the host),
/// and the other way round.
#[test]
fn a_format_prepared_by_one_binary_is_a_hit_for_another() {
    let _serial = serial();
    let (d, tex) = tree("xbin");
    let pool = concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool");
    let dist = |cache: &str| {
        Command::new(env!("CARGO_BIN_EXE_flashtex-dist"))
            .args(["format", "mini"])
            .env("FLASHTEX_POOL", pool)
            .env("FLASHTEX_RESOLVER", "bundle")
            .env("FLASHTEX_BUNDLE", &tex)
            .env("FLASHTEX_FORMAT_CACHE_DIR", d.join(cache))
            .output()
            .unwrap()
    };
    let engine = |cache: &str| {
        Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
            .args(["-fmt=mini", "-interaction=nonstopmode", "story"])
            .current_dir(&d)
            .env("FLASHTEX_POOL", pool)
            .env("FLASHTEX_RESOLVER", "bundle")
            .env("FLASHTEX_BUNDLE", &tex)
            .env("FLASHTEX_FORMAT_CACHE_DIR", d.join(cache))
            .env("FLASHTEX_DEBUG_FORMATS", "1")
            .env_remove("FLASHTEX_FORMATS")
            .output()
            .unwrap()
    };
    let text = |o: &std::process::Output| {
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    };
    // flashtex-dist prepares, the engine hits.
    let o = dist("c1");
    assert!(text(&o).contains("mini: built"), "{}", text(&o));
    let o = engine("c1");
    let t = text(&o);
    assert!(t.contains("[formats] mini: cache hit"), "{t}");
    assert!(t.contains("[hi]"), "{t}");
    // A third binary -- this test, through the library, as the host does --
    // hits too. (Keyed by the running executable, it missed.)
    let mut r = KpathseaResolver::for_bundle(&tex, "mini", "");
    let mut c = FormatCache::new(d.join("c1"));
    let p = c.ensure("mini", "mini", &mut r).unwrap();
    assert!(!c.last.built, "the library in another binary must hit");
    assert!(p.starts_with(d.join("c1")));
    // The engine prepares, flashtex-dist hits.
    let o = engine("c2");
    assert!(text(&o).contains("[formats] mini: built"), "{}", text(&o));
    let o = dist("c2");
    assert!(text(&o).contains("mini: cache hit"), "{}", text(&o));
    let _ = std::fs::remove_dir_all(&d);
}
