//! Pure unit tests for untested public helpers: [`Edit::between`]
//! (conv), the `Loc` bit helpers and empty-map edges (dlsource), [`grow`]
//! (dormant), and the error paths of [`base_of`] (s0file). None needs TeX
//! Live, a display, or an engine.

use flashtex_xetex::conv::Edit;
use flashtex_xetex::dlsource::{loc_col, loc_span, DlSource, Tables, NO_COLUMN};
use flashtex_xetex::dormant::grow;
use flashtex_xetex::s0file::base_of;
use flashtex_xetex::statediff::WordDiff;
use std::path::PathBuf;

fn wd(
    old: u64,
    new: u64,
    region: &'static str,
    index: usize,
    scalar: Option<&'static str>,
) -> WordDiff {
    WordDiff {
        off: 0,
        old,
        new,
        region,
        index,
        scalar,
    }
}

/// The edit's contract: old[..prefix] + inserted + old[old_end..] == new.
fn applies(e: &Edit, old: &[u8], new: &[u8]) {
    let (p, oe) = (e.prefix as usize, e.old_end as usize);
    let inserted = &new[p..(oe as i64 + e.delta) as usize];
    let mut rebuilt = vec![];
    rebuilt.extend_from_slice(&old[..p]);
    rebuilt.extend_from_slice(inserted);
    rebuilt.extend_from_slice(&old[oe..]);
    assert_eq!(rebuilt, new, "edit {e:?} does not turn old into new");
}

#[test]
fn edit_between_cases() {
    // (old, new, prefix, old_end, delta)
    type Case<'a> = (&'a [u8], &'a [u8], u64, u64, i64);
    let cases: &[Case] = &[
        (b"hello", b"hello", 5, 5, 0),  // identical
        (b"", b"", 0, 0, 0),            // both empty
        (b"abc", b"abcdef", 3, 3, 3),   // pure append
        (b"abc", b"Xabc", 0, 0, 1),     // pure prepend
        (b"aXb", b"aYb", 1, 2, 0),      // middle change
        (b"abcdef", b"abef", 2, 4, -2), // middle delete
        (b"aaa", b"bbb", 0, 3, 0),      // nothing in common
        (b"", b"hi", 0, 0, 2),          // old empty
        (b"hi", b"", 0, 2, -2),         // new empty
        (b"ab", b"ac", 1, 2, 0),        // last byte differs
        // The `room` cap: the suffix match must not overlap the prefix.
        (b"aaa", b"aaaaa", 3, 3, 2),
        (b"aaaaa", b"aaa", 3, 5, -2),
    ];
    for (old, new, prefix, old_end, delta) in cases {
        let e = Edit::between("f.tex", old, new);
        assert_eq!(e.path, "f.tex");
        assert_eq!(
            (e.prefix, e.old_end, e.delta),
            (*prefix, *old_end, *delta),
            "old={old:?} new={new:?}"
        );
        applies(&e, old, new);
    }
}

#[test]
fn loc_bits() {
    assert_eq!((loc_span(0), loc_col(0)), (0, 0));
    assert_eq!((loc_span((5 << 16) | 7), loc_col((5 << 16) | 7)), (5, 7));
    // The documented layout: span in bits 16..48, column in 0..16.
    assert_eq!(loc_span(0xABCD_1234_5678), 0xABCD_1234);
    assert_eq!(loc_col(0xABCD_1234_5678), 0x5678);
    assert_eq!(loc_span(0xFFFF_FFFFu64 << 16), u32::MAX);
    // The `NO_COLUMN` sentinel survives as a column.
    assert_eq!(loc_col((2 << 16) | u64::from(NO_COLUMN)), NO_COLUMN);
    // Bit 63 (a native word's start-found flag) changes neither field.
    let l = (1 << 63) | (3 << 16) | 9;
    assert_eq!((loc_span(l), loc_col(l)), (3, 9));
}

#[test]
fn empty_source_map() {
    let mut dl = DlSource::default();
    assert!(!dl.on());
    assert_eq!(dl.at(0), 0);
    assert_eq!(dl.at(1000), 0);
    assert_eq!(dl.at(-1), 0);
    // No spans or files named: every lookup misses, none panics.
    assert_eq!(dl.place(0), None);
    assert_eq!(dl.place(1 << 16), None);
    assert_eq!(dl.place(5), None); // span id 0 with a column
    assert_eq!(dl.place(u64::MAX), None);
    assert_eq!(dl.span(0), None);
    assert_eq!(dl.span(1), None);
    assert_eq!(dl.file(0), None);
    assert_eq!(dl.file(1), None);
    assert_eq!(Tables::default().place(0), None);
    assert_eq!(Tables::default().place(u64::MAX), None);
    // Numbering checks against empty tables.
    assert!(dl.numbered_like(&Tables::default(), (0, 0)));
    assert!(!dl.numbered_like(&Tables::default(), (1, 0)));
    assert!(!dl.numbered_like(&Tables::default(), (0, 1)));
    dl.set_tables(Tables::default());
    assert!(dl.numbered_like(&Tables::default(), (0, 0)));
    dl.page_done();
    assert!(!dl.on());
}

#[test]
fn dormant_grow() {
    assert_eq!(grow(&[]), 0);
    // No `first` word among the differences.
    assert_eq!(grow(&[wd(1, 2, "mem", 0, Some("last"))]), 0);
    assert_eq!(grow(&[wd(1, 2, "buffer", 0, None)]), 0);
    assert_eq!(grow(&[wd(100, 110, "mem", 0, Some("first"))]), 10);
    assert_eq!(grow(&[wd(110, 100, "mem", 0, Some("first"))]), -10);
    // Found wherever it sits in the list.
    let ws = vec![
        wd(1, 2, "buffer", 0, None),
        wd(7, 9, "mem", 0, Some("first")),
    ];
    assert_eq!(grow(&ws), 2);
    // Only the low 32 bits count, signed.
    assert_eq!(
        grow(&[wd(
            0xDEAD_BEEF_0000_0064,
            0x1234_5678_0000_006E,
            "mem",
            0,
            Some("first")
        )]),
        10
    );
    assert_eq!(grow(&[wd(u64::MAX, 5, "mem", 0, Some("first"))]), 6);
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "flashtex-xetex-pure-units-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn base_of_rejects() {
    let dir = scratch("baseof");
    assert_eq!(base_of(&dir.join("missing.xs0")), None);
    let empty = dir.join("empty.xs0");
    std::fs::write(&empty, []).unwrap();
    assert_eq!(base_of(&empty), None);
    let tiny = dir.join("tiny.xs0");
    std::fs::write(&tiny, [1, 2, 3]).unwrap();
    assert_eq!(base_of(&tiny), None);
    // Eight zero bytes: header length 0, nothing decodes.
    let zeros = dir.join("zeros.xs0");
    std::fs::write(&zeros, [0u8; 64]).unwrap();
    assert_eq!(base_of(&zeros), None);
    // A framed header with the wrong magic.
    let mut bad = 64u64.to_le_bytes().to_vec();
    bad.extend_from_slice(&[0x41; 64]);
    let wrong = dir.join("wrong.xs0");
    std::fs::write(&wrong, bad).unwrap();
    assert_eq!(base_of(&wrong), None);
    let _ = std::fs::remove_dir_all(&dir);
}
