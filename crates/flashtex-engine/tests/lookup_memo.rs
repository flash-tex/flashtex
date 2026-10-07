//! The session's lookup memo (`resolver`'s `Memo`): through every change
//! to what a lookup depends on, the memo's answer is kpathsea's own answer
//! on the same instance with the memo off, and the memo answers (a hit)
//! only where nothing it depends on changed or could change unseen.
//!
//! One test in its own process: it sets `TEXMFHOME` (a `//` subtree with no
//! `ls-R`, which kpathsea searches on disk) and the working directory.

mod common;

use flashtex_engine::resolver::{FileResolver, Format, KpathseaResolver};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[test]
fn the_memo_answers_as_kpathsea_does() {
    let Some(bin) = flashtex_engine::resolver::find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-lookup-memo");
    let _ = std::fs::remove_dir_all(&base);
    let (home, cwd, ext) = (base.join("home"), base.join("cwd"), base.join("ext"));
    let sub = home.join("tex/latex/flashprobe");
    for d in [&sub, &cwd, &ext] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(sub.join("flashother.sty"), "% other\n").unwrap();
    // the same name in two directories of the subtree (kpathsea's answer
    // depends on its own search order there)
    let sub2 = home.join("tex/latex/flashprobe2");
    std::fs::create_dir_all(&sub2).unwrap();
    std::fs::write(sub.join("flashdup.sty"), "% one\n").unwrap();
    std::fs::write(sub2.join("flashdup.sty"), "% two\n").unwrap();
    std::env::set_var("TEXMFHOME", &home);
    std::env::set_current_dir(&cwd).unwrap();
    // (the subtree's directories made just now are racy: set back once)
    let old = SystemTime::now() - Duration::from_secs(3600);
    for d in [
        home.join("tex"),
        home.join("tex/latex"),
        home.clone(),
        base.clone(),
        sub2.clone(),
    ] {
        std::fs::File::open(&d).unwrap().set_modified(old).unwrap();
    }
    let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex");
    // Directory times well in the past (no racy signature), a new one each
    // time: the memo, not the racy rule, is what is tested.
    let mut tick = 0u64;
    let mut set_back = |d: &Path| {
        tick += 1;
        let t = SystemTime::now() - Duration::from_secs(600 - tick);
        std::fs::File::open(d).unwrap().set_modified(t).unwrap();
    };
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
        // (the directory the name is in: a link is not followed)
        p.as_ref()
            .and_then(|p| p.parent())
            .and_then(|q| std::fs::canonicalize(q).ok())
            == std::fs::canonicalize(d).ok()
    };
    set_back(&sub);
    set_back(&cwd);

    // not found, remembered
    assert_eq!(look(&mut r, "flashprobe.sty", "absent"), (None, false));
    assert_eq!(look(&mut r, "flashprobe.sty", "absent again"), (None, true));
    // the file appears in the subtree
    std::fs::write(sub.join("flashprobe.sty"), "% probe\n").unwrap();
    set_back(&sub);
    let (p, hit) = look(&mut r, "flashprobe.sty", "appeared in the subtree");
    assert!(found_in(&p, &sub) && !hit, "{p:?}");
    assert!(look(&mut r, "flashprobe.sty", "again").1, "not remembered");
    // ... and in the working directory, which is searched first
    std::fs::write(cwd.join("flashprobe.sty"), "% probe\n").unwrap();
    set_back(&cwd);
    let (p, hit) = look(
        &mut r,
        "flashprobe.sty",
        "appeared in the working directory",
    );
    assert!(found_in(&p, &cwd) && !hit, "{p:?}");
    // both go
    std::fs::remove_file(cwd.join("flashprobe.sty")).unwrap();
    std::fs::remove_file(sub.join("flashprobe.sty")).unwrap();
    set_back(&cwd);
    set_back(&sub);
    assert_eq!(look(&mut r, "flashprobe.sty", "both went"), (None, false));

    // a dangling link: kpathsea passes over it; its target appears without
    // a change to the link's directory, so the lookup is never remembered
    let target = ext.join("flashlink-target.sty");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, sub.join("flashlink.sty")).unwrap();
        set_back(&sub);
        assert_eq!(look(&mut r, "flashlink.sty", "dangling").0, None);
        assert_eq!(
            look(&mut r, "flashlink.sty", "dangling again"),
            (None, false)
        );
        std::fs::write(&target, "% target\n").unwrap();
        let (p, _) = look(&mut r, "flashlink.sty", "the target appeared");
        assert!(found_in(&p, &sub), "{p:?}");
    }

    // the file found becomes unreadable (its directory does not change)
    let locked = sub.join("flashlocked.sty");
    std::fs::write(&locked, "% locked\n").unwrap();
    set_back(&sub);
    assert!(look(&mut r, "flashlocked.sty", "readable").0.is_some());
    assert!(look(&mut r, "flashlocked.sty", "readable again").1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // (as root everything is readable: kpathsea's answer, whatever it is,
        // is still compared)
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let (_, hit) = look(&mut r, "flashlocked.sty", "unreadable");
        assert!(!hit, "an unreadable file answered from the memo");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(look(&mut r, "flashlocked.sty", "readable once more")
            .0
            .is_some());
    }

    // the same name in two directories of the subtree: not remembered
    look(&mut r, "flashdup.sty", "in two directories");
    assert!(!look(&mut r, "flashdup.sty", "in two directories again").1);

    // a directory made in the subtree after kpathsea expanded it: kpathsea
    // does not search it (it expands a subtree once per process), and the
    // memo gives kpathsea's answer
    let deeper = sub.join("deeper");
    std::fs::create_dir_all(&deeper).unwrap();
    std::fs::write(deeper.join("flashother.sty"), "% twice\n").unwrap();
    // (a new directory: kpathsea expanded the subtree before, and does not
    // see it -- nor does the memo pretend to)
    set_back(&deeper);
    set_back(&sub);
    look(&mut r, "flashother.sty", "twice");
    look(&mut r, "flashother.sty", "twice again");

    // a racy directory (changed within the file system's time granularity)
    // proves nothing: no answer from the memo until it is old enough
    std::fs::write(sub.join("flashfresh.sty"), "% fresh\n").unwrap();
    let (p, _) = look(&mut r, "flashfresh.sty", "fresh");
    assert!(found_in(&p, &sub), "{p:?}");
    assert!(
        !look(&mut r, "flashfresh.sty", "fresh again").1,
        "a racy directory answered"
    );
    set_back(&sub);
    look(&mut r, "flashfresh.sty", "no longer racy");
    assert!(
        look(&mut r, "flashfresh.sty", "and again").1,
        "not remembered once old"
    );

    // names the memo leaves to kpathsea
    for n in [
        "./flashprobe.sty",
        "flashprobe/flashother.sty",
        "$HOME/x.sty",
        "~/x.sty",
    ] {
        let (_, hit) = look(&mut r, n, n);
        let (_, hit2) = look(&mut r, n, n);
        assert!(!hit && !hit2, "{n} answered from the memo");
    }
    let _ = std::fs::remove_dir_all(&base);
}
