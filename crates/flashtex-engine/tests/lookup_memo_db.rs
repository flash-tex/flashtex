//! The session's lookup memo (`resolver`'s `Memo`) and ls-R databases:
//! kpathsea's database search checks each entry on disk (db.c), so an entry
//! listed in `ls-R` with no file under it is passed over, and found once the
//! file appears -- with no directory of the search changing. Such a lookup
//! is never remembered; one the database answers with a file on disk is,
//! and only where kpathsea's own database search found it. Through every
//! step the memo's answer equals kpathsea's own on the same instance with
//! the memo off (review of #1561, MED).
//!
//! Its own process: it sets `TEXINPUTS`, `TEXMFDBS` and the working
//! directory.

mod common;

use flashtex_engine::resolver::{FileResolver, Format, KpathseaResolver};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[test]
fn an_ls_r_entry_not_on_disk_is_never_remembered() {
    let Some(bin) = flashtex_engine::resolver::find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-lookup-memo-db");
    let _ = std::fs::remove_dir_all(&base);
    let (tree, home, cwd) = (base.join("tree"), base.join("home"), base.join("cwd"));
    for d in [tree.join("a"), home.clone(), cwd.clone()] {
        std::fs::create_dir_all(d).unwrap();
    }
    // the database lists a/flashdb.sty (not there) and a/flashok.sty
    std::fs::write(
        tree.join("ls-R"),
        "% ls-R -- filename database for kpathsea; do not change this line.\n\
         ./:\nls-R\na\n\n./a:\nflashdb.sty\nflashok.sty\n",
    )
    .unwrap();
    std::fs::write(tree.join("a/flashok.sty"), "% ok\n").unwrap();
    let old = SystemTime::now() - Duration::from_secs(3600);
    for d in [
        tree.join("a"),
        tree.clone(),
        home.clone(),
        cwd.clone(),
        base.clone(),
    ] {
        std::fs::File::open(&d).unwrap().set_modified(old).unwrap();
    }
    std::env::set_var(
        "TEXINPUTS",
        format!(".:!!{}//:{}//", tree.display(), home.display()),
    );
    std::env::set_var("TEXMFDBS", &tree);
    std::env::set_current_dir(&cwd).unwrap();
    let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex");
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
    let under = |p: &Option<PathBuf>, d: &Path| {
        p.as_ref()
            .and_then(|p| p.parent())
            .and_then(|q| std::fs::canonicalize(q).ok())
            == std::fs::canonicalize(d).ok()
    };
    // found in the database, on disk: remembered
    let (p, _) = look(&mut r, "flashok.sty", "in the database");
    assert!(under(&p, &tree.join("a")), "{p:?}");
    assert!(look(&mut r, "flashok.sty", "again").1, "not remembered");

    // listed, not on disk: not found, and not remembered
    assert_eq!(look(&mut r, "flashdb.sty", "listed, absent"), (None, false));
    assert_eq!(look(&mut r, "flashdb.sty", "again"), (None, false));
    // the home tree has it: found there, not remembered either
    std::fs::write(home.join("flashdb.sty"), "% home\n").unwrap();
    std::fs::File::open(&home)
        .unwrap()
        .set_modified(old + Duration::from_secs(1))
        .unwrap();
    let (p, hit) = look(&mut r, "flashdb.sty", "in the home tree");
    assert!(under(&p, &home) && !hit, "{p:?}");
    assert!(!look(&mut r, "flashdb.sty", "again").1, "remembered");
    // the database's file appears (its directory set back): kpathsea
    // prefers it, and so does the memo
    std::fs::write(tree.join("a/flashdb.sty"), "% tree\n").unwrap();
    std::fs::File::open(tree.join("a"))
        .unwrap()
        .set_modified(old)
        .unwrap();
    let (p, _) = look(&mut r, "flashdb.sty", "the database's file appeared");
    assert!(under(&p, &tree.join("a")), "{p:?}");
    let _ = std::fs::remove_dir_all(&base);
}
