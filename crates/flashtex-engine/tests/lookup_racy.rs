//! Racy directories and the resolver's caches (TITLE-WARMUP).
//!
//! A directory changed within the file system's time granularity has a racy
//! signature, which equals nothing (`StatSig`). The resolver used to take a
//! racy directory as changed: every compile made kpathsea forget its `//`
//! expansions, and the lookup memo answered nothing that depended on it.
//! A host opened on a project whose output directory or working directory
//! had just been written (the first compile's outputs, the app's save by
//! rename) paid 0.9 G instructions more per keystroke for two seconds.
//! Now a racy directory is listed again instead, and its listing, taken
//! after a fresh signature, decides:
//! - `refresh_disk_dirs` keeps the expansions while every racy directory's
//!   subdirectories (what the expansion reads of it) are the ones listed at
//!   the last check;
//! - the memo answers while every directory whose signature differs holds
//!   nothing new under a name kpathsea tries, and kpathsea expands the
//!   format's path to the directories the entry was made with.
//!
//! The changes below are made so that no signature shows them (renamed in
//! place, the directory's time put back), as a change within the same tick
//! would be. Each fails if a racy signature is trusted, in the memo (2) or
//! in `refresh_disk_dirs` (4), or if the memo ignores a new expansion (4).
//! From the review of #1776: 5 fails if a listing taken after kpathsea's
//! walk validates it, and 6 if the memo compares names by `to_lowercase`
//! (on a case-insensitive file system) or lists again for a name that is
//! not ASCII.
//! Through every step the memo's answer is kpathsea's own on the same
//! instance with the memo off. One test in its own process: it sets
//! `FLASHTEX_RACY_MS` (every signature racy), `TEXMFHOME` and the working
//! directory.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::{FileResolver, Format, KpathseaResolver};
use flashtex_engine::system::StatSig;
use std::path::{Path, PathBuf};

#[test]
fn racy_directories_are_listed_again() {
    // A one-hour tick: every signature taken here is racy (set before the
    // first one is taken).
    std::env::set_var("FLASHTEX_RACY_MS", "3600000");
    let Some(bin) = flashtex_engine::resolver::find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-lookup-racy");
    let (home, cwd) = (base.join("home"), base.join("cwd"));
    let sub = home.join("tex/latex/flashrace");
    for d in [&sub, &cwd, &sub.join(".stash")] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(sub.join("flashrace.sty"), "% race\n").unwrap();
    // stand-ins, renamed into place later: kpathsea never tries the first
    // name, and never expands into a directory whose name begins with `.`
    std::fs::write(cwd.join("flashpad.tmp"), "% pad\n").unwrap();
    std::fs::write(sub.join(".stash/flashdeep.sty"), "% deep\n").unwrap();
    std::env::set_var("TEXMFHOME", &home);
    std::env::set_current_dir(&cwd).unwrap();
    let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex");

    // The memo's answer and kpathsea's (memo off, same instance), equal;
    // and whether the memo answered.
    let look = |r: &mut KpathseaResolver, name: &str, what: &str| -> (Option<PathBuf>, bool) {
        let (h0, _) = KpathseaResolver::memo_counts();
        let a = r.find_ex(name, Format::Tex, false).0;
        let hit = KpathseaResolver::memo_counts().0 > h0;
        r.set_memo(false);
        let b = r.find_ex(name, Format::Tex, false).0;
        r.set_memo(true);
        assert_eq!(a, b, "{what}: the memo's answer is not kpathsea's");
        (a, hit)
    };
    let found_in = |p: &Option<PathBuf>, d: &Path| {
        p.as_ref()
            .and_then(|p| p.parent())
            .and_then(|q| std::fs::canonicalize(q).ok())
            == std::fs::canonicalize(d).ok()
    };
    // A rename in `d` that its signature does not show: as a change within
    // the tick in which the signature was taken.
    let quietly = |d: &Path, from: &str, to: &str| {
        let s = d.to_str().unwrap();
        let before = StatSig::of(s).unwrap();
        let t = std::fs::metadata(d).unwrap().modified().unwrap();
        std::fs::rename(d.join(from), d.join(to)).unwrap();
        std::fs::File::open(d).unwrap().set_modified(t).unwrap();
        let after = StatSig::of(s).unwrap();
        assert!(
            before.racy && after.racy && before.same_fields(&after),
            "the signature shows the rename: {before:?} -> {after:?}"
        );
    };

    // 1. A racy directory, unchanged: listed again, the memo answers, and
    // kpathsea keeps its expansions.
    let (p, hit) = look(&mut r, "flashrace.sty", "in the subtree");
    assert!(found_in(&p, &sub) && !hit, "{p:?}");
    assert!(
        look(&mut r, "flashrace.sty", "again").1,
        "racy directories, listed again, did not answer"
    );
    // (the first check trusts no listing, so the second forgets once and
    // lists the racy directories before kpathsea walks them again)
    r.refresh_disk_dirs();
    r.refresh_disk_dirs();
    let f0 = KpathseaResolver::disk_forgets();
    r.refresh_disk_dirs();
    r.refresh_disk_dirs();
    assert_eq!(
        KpathseaResolver::disk_forgets(),
        f0,
        "unchanged racy directories made kpathsea forget its expansions"
    );
    assert!(look(&mut r, "flashrace.sty", "after the checks").1);

    // 2. The name appears in the working directory, searched first.
    quietly(&cwd, "flashpad.tmp", "flashrace.sty");
    r.refresh_disk_dirs();
    let (p, hit) = look(&mut r, "flashrace.sty", "appeared in the working directory");
    assert!(found_in(&p, &cwd) && !hit, "{p:?}");
    // 3. ... and goes again.
    quietly(&cwd, "flashrace.sty", "flashpad.tmp");
    r.refresh_disk_dirs();
    let (p, hit) = look(&mut r, "flashrace.sty", "went from the working directory");
    assert!(found_in(&p, &sub) && !hit, "{p:?}");
    assert!(look(&mut r, "flashrace.sty", "remembered again").1);

    // 4. A subdirectory appears in the subtree: kpathsea's expansion is
    // stale, so it is forgotten, and the memo's answer, made under the old
    // expansion, is not taken.
    assert_eq!(look(&mut r, "flashdeep.sty", "absent"), (None, false));
    assert_eq!(look(&mut r, "flashdeep.sty", "absent again"), (None, true));
    let f0 = KpathseaResolver::disk_forgets();
    quietly(&sub, ".stash", "deeper");
    r.refresh_disk_dirs();
    assert!(
        KpathseaResolver::disk_forgets() > f0,
        "a new subdirectory of a racy directory left kpathsea's expansion as it was"
    );
    let (p, hit) = look(&mut r, "flashdeep.sty", "in a new subdirectory");
    assert!(found_in(&p, &sub.join("deeper")) && !hit, "{p:?}");
    let fresh = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex")
        .find_ex("flashdeep.sty", Format::Tex, false)
        .0;
    assert_eq!(p, fresh, "a fresh process finds another file");

    // 5. A subdirectory made after kpathsea walked the subtree, before the
    // resolver's first check: a listing taken at that check already shows
    // it, so it proves nothing about the walk (review of #1776, HIGH 1).
    let mut r2 = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex");
    assert_eq!(r2.find_ex("flashdeep2.sty", Format::Tex, false).0, None);
    std::fs::create_dir_all(sub.join("deeper2")).unwrap();
    std::fs::write(sub.join("deeper2/flashdeep2.sty"), "% deep\n").unwrap();
    r2.refresh_disk_dirs(); // the first check
    r2.refresh_disk_dirs();
    let (p, _) = look(&mut r2, "flashdeep2.sty", "made before the first check");
    assert!(found_in(&p, &sub.join("deeper2")), "{p:?}");

    // 6. A name a case-insensitive file system matches beyond ASCII case
    // (U+FB01, the `fi` ligature: APFS opens `\u{FB01}xa.sty` as
    // `fixa.sty`) appears in the working directory, searched first (review
    // of #1776, HIGH 2). Whatever kpathsea finds, the memo does not answer.
    std::fs::write(sub.join("fixa.sty"), "% fixa\n").unwrap();
    std::fs::write(cwd.join("flashpad2.tmp"), "% pad\n").unwrap();
    r.refresh_disk_dirs();
    let (p, _) = look(&mut r, "fixa.sty", "fixa");
    assert!(found_in(&p, &sub), "{p:?}");
    assert!(look(&mut r, "fixa.sty", "fixa again").1);
    quietly(&cwd, "flashpad2.tmp", "\u{FB01}xa.sty");
    r.refresh_disk_dirs();
    assert!(
        !look(&mut r, "fixa.sty", "a ligature in the working directory").1,
        "the memo answered past an entry a case-insensitive file system may match"
    );
    // ... and a name that is not ASCII is left to the signature. (Any entry
    // that could match it is not ASCII either, so the entry rule alone
    // also holds this; the name rule is defence in depth.)
    std::fs::write(sub.join("flash\u{e9}.sty"), "% e\n").unwrap();
    look(&mut r, "flash\u{e9}.sty", "not ASCII");
    assert!(
        !look(&mut r, "flash\u{e9}.sty", "not ASCII again").1,
        "a name that is not ASCII answered from a racy directory"
    );

    let _ = std::fs::remove_dir_all(&base);
}
