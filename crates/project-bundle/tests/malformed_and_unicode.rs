//! Bounded malformed-input and Unicode-filename coverage.
//!
//! Path syntax is now validated by `flashtex_project_files::ProjectPath`,
//! which normalizes `.`  segments, empty (`//`) segments and a trailing `/`
//! away rather than rejecting them outright (rev 1's own hand-rolled
//! validator treated all three as hard `MalformedPath` errors). This is a
//! relaxation of input *hygiene*, not a security property: every one of
//! those forms still resolves, after normalization, to a plain
//! project-relative path that cannot leave the root — the escape- and
//! absolute-path checks below are unchanged and still fire before any
//! filesystem access.

mod common;

use common::TempDir;
use flashtex_project_bundle::{BundleEntry, BundleError, ProjectRoot, build_bundle};

#[test]
fn empty_path_is_rejected() {
    let dir = TempDir::new("malformed-empty");
    let root = ProjectRoot::new(dir.path()).unwrap();
    assert_eq!(root.read_rooted("").unwrap_err(), BundleError::EmptyPath);
}

#[test]
fn null_byte_in_path_is_rejected() {
    let dir = TempDir::new("malformed-nul");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let err = root.read_rooted("foo\0bar").unwrap_err();
    assert!(matches!(err, BundleError::MalformedPath(_)));
}

#[test]
fn double_slash_empty_component_is_normalized_away() {
    let dir = TempDir::new("malformed-double-slash");
    dir.write("foo/bar.tex", b"x");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let bytes = root.read_rooted("foo//bar.tex").unwrap();
    assert_eq!(bytes, b"x");
}

#[test]
fn trailing_slash_is_normalized_away() {
    let dir = TempDir::new("malformed-trailing-slash");
    dir.write("foo.tex", b"x");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let bytes = root.read_rooted("foo.tex/").unwrap();
    assert_eq!(bytes, b"x");
}

#[test]
fn single_dot_component_is_normalized_away() {
    let dir = TempDir::new("malformed-dot");
    dir.write("foo.tex", b"x");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let bytes = root.read_rooted("./foo.tex").unwrap();
    assert_eq!(bytes, b"x");
}

#[test]
fn internal_dot_dot_that_stays_inside_the_root_is_normalized_not_rejected() {
    // "a/../foo.tex" never leaves the root once normalized ("foo.tex"), so
    // it is accepted — unlike rev 1, which rejected any ".." component on
    // sight. A ".." that would actually leave the root is still rejected
    // (see `dot_dot_traversal_is_rejected*` in tests/rooted.rs).
    let dir = TempDir::new("malformed-internal-dotdot");
    dir.write("foo.tex", b"x");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let bytes = root.read_rooted("a/../foo.tex").unwrap();
    assert_eq!(bytes, b"x");
}

#[test]
fn bare_dot_dot_is_traversal_not_malformed() {
    let dir = TempDir::new("malformed-bare-dotdot");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let err = root.read_rooted("..").unwrap_err();
    assert_eq!(err, BundleError::PathTraversal("..".to_string()));
}

#[test]
fn duplicate_entries_are_rejected() {
    let dir = TempDir::new("malformed-duplicate");
    dir.write("main.tex", b"x");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let entries = [BundleEntry::new("main.tex"), BundleEntry::new("main.tex")];
    let err = build_bundle(&root, &entries).unwrap_err();
    assert_eq!(err, BundleError::DuplicatePath("main.tex".to_string()));
}

#[test]
fn non_ascii_filenames_round_trip_through_the_bundle() {
    let dir = TempDir::new("unicode");
    dir.write("café.tex", "contenu français".as_bytes());
    dir.write("日本語のファイル.tex", "内容".as_bytes());
    dir.write("emoji-\u{1F4C4}.txt", b"page emoji");

    let root = ProjectRoot::new(dir.path()).unwrap();
    let entries = [
        BundleEntry::new("café.tex"),
        BundleEntry::new("日本語のファイル.tex"),
        BundleEntry::new("emoji-\u{1F4C4}.txt"),
    ];
    let bundle = build_bundle(&root, &entries).unwrap();
    assert_eq!(bundle.files.len(), 3);

    let cafe = bundle
        .files
        .iter()
        .find(|f| f.path == "café.tex")
        .expect("café.tex present");
    assert_eq!(cafe.contents, "contenu français".as_bytes());

    // Ordering must be a plain byte sort of the UTF-8 path, so building
    // again from a shuffled entry list gives the identical manifest.
    let shuffled = [
        BundleEntry::new("emoji-\u{1F4C4}.txt"),
        BundleEntry::new("café.tex"),
        BundleEntry::new("日本語のファイル.tex"),
    ];
    let bundle2 = build_bundle(&root, &shuffled).unwrap();
    assert_eq!(bundle.manifest_bytes(), bundle2.manifest_bytes());
}

#[test]
fn non_ascii_directory_and_file_names_are_rooted_normally() {
    let dir = TempDir::new("unicode-nested");
    dir.write("章/一.tex", "第一章".as_bytes());
    let root = ProjectRoot::new(dir.path()).unwrap();
    let bytes = root.read_rooted("章/一.tex").unwrap();
    assert_eq!(bytes, "第一章".as_bytes());
}

#[test]
fn path_of_only_separators_is_rejected_as_absolute() {
    // No content at all, just repeated "/" — must not be silently
    // normalized down to the empty path (a different, also-rejected case)
    // or accepted as some kind of root reference. `ProjectPath::normalize`
    // checks `starts_with('/')` before it ever splits into segments, so
    // this is `AbsolutePath`, not `EmptyPath` — pinned here so the two
    // stay distinguishable.
    let dir = TempDir::new("malformed-only-separators");
    let root = ProjectRoot::new(dir.path()).unwrap();
    let err = root.read_rooted("///").unwrap_err();
    assert_eq!(err, BundleError::AbsolutePath("///".to_string()));
}

#[test]
fn unicode_normalization_collision_is_rejected_not_silently_admitted() {
    // "café.tex" written with a precomposed é (U+00E9, NFC) vs. the same
    // visual name spelled with "e" + a combining acute accent (U+0301,
    // NFD): different UTF-8 byte sequences, but the APFS hazard documented
    // at the crate level means a normalization-insensitive volume (the
    // default for macOS APFS, which this test runs on) resolves both to
    // the very same directory entry. Declaring both as separate bundle
    // entries must be a typed error, not a bundle that silently ends up
    // with two "different" files that are actually one, or a panic from
    // whatever tries to treat them as independent.
    let nfc = "caf\u{e9}.tex".to_string(); // "café.tex", precomposed é (U+00E9)
    let nfd = "cafe\u{301}.tex".to_string(); // "café.tex", "e" + combining acute (U+0301)

    let dir = TempDir::new("unicode-normalization-collision");
    dir.write(&nfc, b"content");
    let root = ProjectRoot::new(dir.path()).unwrap();

    // Sanity check on this machine's actual filesystem: reading the NFD
    // spelling must find the very same file the NFC spelling wrote, or
    // the rest of this test would not be exercising the hazard it claims
    // to. Only macOS (APFS) is normalization-insensitive by default; on
    // Linux (ext4) the NFD spelling is simply a different, absent name, and
    // the bundle must reject the pair all the same, which is asserted
    // below on every platform.
    #[cfg(target_os = "macos")]
    assert_eq!(
        root.read_rooted(&nfd).unwrap(),
        b"content",
        "this test assumes a normalization-insensitive filesystem (default macOS APFS); \
         the NFD spelling must resolve to the file the NFC spelling created"
    );

    let entries = [BundleEntry::new(nfc.clone()), BundleEntry::new(nfd.clone())];
    let err = build_bundle(&root, &entries).unwrap_err();
    assert_eq!(
        err,
        BundleError::AmbiguousPath {
            first: nfc,
            second: nfd
        }
    );
}

#[test]
fn unicode_normalization_collision_is_detected_before_the_second_path_is_ever_read() {
    // Same NFC/NFD pair as above, but this time only the *first* declared
    // spelling is ever written to disk; the second is not created at all.
    // Detection is now Unicode-canonical-equivalence on the declared
    // strings themselves, checked before either entry is read — so the
    // collision is caught without this crate ever calling
    // `read_rooted_optional` for the second path, whose backing file does
    // not exist. This is exactly the capability an `fs::canonicalize`-based
    // check structurally cannot have: canonicalize requires its argument to
    // already exist, so a check built on it could only ever fire once both
    // colliding paths are already real files — never for the import-preview
    // case of validating a bundle before a target has anything written to
    // it yet. Without this fix, the second path is read *before* the
    // identity check runs, so a missing second file surfaces as
    // `NotFound`, silently masking the collision instead of reporting it.
    let nfc = "caf\u{e9}.tex".to_string(); // "café.tex", precomposed é (U+00E9)
    let nfd = "cafe\u{301}.tex".to_string(); // "café.tex", "e" + combining acute (U+0301)

    let dir = TempDir::new("unicode-normalization-collision-second-missing");
    dir.write(&nfc, b"content");
    // Deliberately: the NFD spelling is never created.
    let root = ProjectRoot::new(dir.path()).unwrap();

    let entries = [BundleEntry::new(nfc.clone()), BundleEntry::new(nfd.clone())];
    let err = build_bundle(&root, &entries).unwrap_err();
    assert_eq!(
        err,
        BundleError::AmbiguousPath {
            first: nfc,
            second: nfd
        },
        "must be caught by name alone before the second path is ever read — its file does not exist"
    );
}
