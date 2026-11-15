//! The public behaviour of `lookupproof` (LOOKUP-SKIP), which had no test
//! coverage: a proof holds while the searched directories say kpathsea
//! finds the same answer, and fails on every change that could move it.
//!
//! The proofs live in process-global tables, so each scenario runs under
//! one lock and in a directory of its own.

use flashtex_runtime::lookupproof;
use flashtex_runtime::resolver::{Format, LookupDirs};
use std::sync::Mutex;

static SER: Mutex<()> = Mutex::new(());

fn scratch(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("flashtex-rt-lp-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn deps(tries: &[&str], listed: &[String]) -> Result<LookupDirs, &'static str> {
    Ok(LookupDirs {
        tries: tries.iter().map(|s| s.to_string()).collect(),
        listed: listed.to_vec(),
        above: vec![],
    })
}

/// Two searched directories, the answer in the second: the proof holds and
/// names the answer, with one dependency per directory and one for the
/// answer staying readable.
#[test]
fn proof_holds_when_nothing_changed() {
    let _g = SER.lock().unwrap();
    let d = scratch("holds");
    let (d1, d2) = (d.join("first"), d.join("second"));
    std::fs::create_dir_all(&d1).unwrap();
    std::fs::create_dir_all(&d2).unwrap();
    let (d1, d2) = (
        d1.to_str().unwrap().to_string(),
        d2.to_str().unwrap().to_string(),
    );
    let found = format!("{d2}/wanted-holds.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wanted-holds.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wanted-holds.tex"], &[d1.clone(), d2.clone()]),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    let (back, dd) = v
        .proven_again("wanted-holds.tex", Format::Tex, None)
        .expect("the proof holds");
    assert_eq!(back.as_deref(), Some(found.as_str()));
    assert_eq!((v.held, v.made), (1, 0));
    assert_eq!(dd.len(), 3);
    assert_eq!((&dd[0].0, &dd[1].0), (&d1, &d2));
    assert_eq!(dd[2].0, lookupproof::readable_dep(&found));
    let _ = std::fs::remove_dir_all(&d);
}

/// A file under a tried name appears in a directory searched before the
/// answer's: kpathsea would find it instead, so the proof fails.
#[test]
fn proof_fails_when_a_competing_file_appears() {
    let _g = SER.lock().unwrap();
    let d = scratch("competes");
    let (d1, d2) = (d.join("first"), d.join("second"));
    std::fs::create_dir_all(&d1).unwrap();
    std::fs::create_dir_all(&d2).unwrap();
    let (d1, d2) = (
        d1.to_str().unwrap().to_string(),
        d2.to_str().unwrap().to_string(),
    );
    let found = format!("{d2}/wanted-competes.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wanted-competes.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wanted-competes.tex"], &[d1.clone(), d2]),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    assert!(v
        .proven_again("wanted-competes.tex", Format::Tex, None)
        .is_some());
    std::fs::write(format!("{d1}/wanted-competes.tex"), b"shadow").unwrap();
    // (a verifier takes each listing once: the next pass gets a new one)
    let mut v = lookupproof::Verifier::new(None);
    assert_eq!(
        v.proven_again("wanted-competes.tex", Format::Tex, None),
        None
    );
    assert_eq!((v.held, v.made), (0, 1));
    let _ = std::fs::remove_dir_all(&d);
}

/// The answer goes: the proof fails.
#[test]
fn proof_fails_when_the_answer_goes() {
    let _g = SER.lock().unwrap();
    let d = scratch("gone");
    let d1 = d.join("only");
    std::fs::create_dir_all(&d1).unwrap();
    let d1 = d1.to_str().unwrap().to_string();
    let found = format!("{d1}/wanted-gone.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wanted-gone.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wanted-gone.tex"], &[d1]),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    assert!(v
        .proven_again("wanted-gone.tex", Format::Tex, None)
        .is_some());
    std::fs::remove_file(&found).unwrap();
    let mut v = lookupproof::Verifier::new(None);
    assert_eq!(v.proven_again("wanted-gone.tex", Format::Tex, None), None);
    let _ = std::fs::remove_dir_all(&d);
}

/// The answer stays listed but stops being readable: the proof fails (the
/// listing alone cannot see that).
#[cfg(unix)]
#[test]
fn proof_fails_when_the_answer_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let _g = SER.lock().unwrap();
    let d = scratch("unreadable");
    let d1 = d.join("only");
    std::fs::create_dir_all(&d1).unwrap();
    let d1 = d1.to_str().unwrap().to_string();
    let found = format!("{d1}/wanted-dark.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wanted-dark.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wanted-dark.tex"], &[d1]),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    assert!(v
        .proven_again("wanted-dark.tex", Format::Tex, None)
        .is_some());
    std::fs::set_permissions(&found, std::fs::Permissions::from_mode(0o000)).unwrap();
    // (as root the mode changes nothing: nothing to check)
    if std::fs::File::open(&found).is_ok() {
        std::fs::set_permissions(&found, std::fs::Permissions::from_mode(0o644)).unwrap();
        let _ = std::fs::remove_dir_all(&d);
        return;
    }
    let mut v = lookupproof::Verifier::new(None);
    assert_eq!(v.proven_again("wanted-dark.tex", Format::Tex, None), None);
    std::fs::set_permissions(&found, std::fs::Permissions::from_mode(0o644)).unwrap();
    let _ = std::fs::remove_dir_all(&d);
}

/// Unknown lookup dependencies keep no proof: the lookup is made again.
#[test]
fn unknown_dependencies_are_made_again() {
    let _g = SER.lock().unwrap();
    lookupproof::note(
        "wanted-unknown.tex",
        Format::Tex,
        None,
        None,
        &Err("not known"),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    assert_eq!(
        v.proven_again("wanted-unknown.tex", Format::Tex, None),
        None
    );
    assert_eq!((v.held, v.made), (0, 1));
}

/// Tried names that are not ASCII keep no proof (only ASCII names are
/// compared against the listings' case-folding).
#[test]
fn non_ascii_tries_keep_no_proof() {
    let _g = SER.lock().unwrap();
    let d = scratch("nonascii");
    let d1 = d.join("only");
    std::fs::create_dir_all(&d1).unwrap();
    let d1 = d1.to_str().unwrap().to_string();
    let found = format!("{d1}/wänted.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wänted.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wänted.tex"], &[d1]),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    assert_eq!(v.proven_again("wänted.tex", Format::Tex, None), None);
    let _ = std::fs::remove_dir_all(&d);
}

/// Replacing the resolver invalidates every proof made with the old one.
#[test]
fn resolver_changed_invalidates_proofs() {
    let _g = SER.lock().unwrap();
    let d = scratch("generation");
    let d1 = d.join("only");
    std::fs::create_dir_all(&d1).unwrap();
    let d1 = d1.to_str().unwrap().to_string();
    let found = format!("{d1}/wanted-gen.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wanted-gen.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wanted-gen.tex"], &[d1]),
        None,
    );
    let mut v = lookupproof::Verifier::new(None);
    assert!(v
        .proven_again("wanted-gen.tex", Format::Tex, None)
        .is_some());
    lookupproof::resolver_changed();
    let mut v = lookupproof::Verifier::new(None);
    assert_eq!(v.proven_again("wanted-gen.tex", Format::Tex, None), None);
    let _ = std::fs::remove_dir_all(&d);
}

/// With `-output-directory`, tried first: the proof holds while the name
/// is not a file there, and fails once it is.
#[test]
fn output_directory_is_tried_first() {
    let _g = SER.lock().unwrap();
    let d = scratch("outdir");
    let (od, tex) = (d.join("out"), d.join("tex"));
    std::fs::create_dir_all(&od).unwrap();
    std::fs::create_dir_all(&tex).unwrap();
    let (od, tex) = (
        od.to_str().unwrap().to_string(),
        tex.to_str().unwrap().to_string(),
    );
    let found = format!("{tex}/wanted-od.tex");
    std::fs::write(&found, b"x").unwrap();
    lookupproof::note(
        "wanted-od.tex",
        Format::Tex,
        None,
        Some(&found),
        &deps(&["wanted-od.tex"], &[tex]),
        Some(&od),
    );
    let mut v = lookupproof::Verifier::new(Some(od.clone()));
    assert!(v.proven_again("wanted-od.tex", Format::Tex, None).is_some());
    std::fs::write(format!("{od}/wanted-od.tex"), b"local").unwrap();
    let mut v = lookupproof::Verifier::new(Some(od.clone()));
    assert_eq!(v.proven_again("wanted-od.tex", Format::Tex, None), None);
    let _ = std::fs::remove_dir_all(&d);
}

/// `folded_listing`: entries as an ASCII tried name could match them,
/// sorted; `None` where the directory cannot be listed.
#[test]
fn folded_listing_folds_and_sorts() {
    let _g = SER.lock().unwrap();
    let d = scratch("folded");
    std::fs::write(d.join("B.TEX"), b"x").unwrap();
    std::fs::write(d.join("a.tex"), b"x").unwrap();
    std::fs::create_dir_all(d.join("Sub")).unwrap();
    let l = lookupproof::folded_listing(d.to_str().unwrap()).unwrap();
    assert_eq!(l, vec!["a.tex", "b.tex", "sub"]);
    assert_eq!(
        lookupproof::folded_listing(d.join("missing").to_str().unwrap()),
        None
    );
    let _ = std::fs::remove_dir_all(&d);
}
