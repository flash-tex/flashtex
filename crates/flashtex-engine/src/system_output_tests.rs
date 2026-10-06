//! Output files across checkpoints (issue #1294): what a restore, a
//! reattach and a convergence jump leave in a file must be exactly what the
//! engine's streams recorded -- with several streams on one file (LaTeX's
//! `\tableofcontents` twice opens the `.toc` twice), with bytes still in a
//! stream's buffer, and with files opened for output again (truncated)
//! after a checkpoint.

use super::*;
use crate::generated::Globals;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-out-restore-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `\immediate\openout` of `path` on write stream `k`.
fn openout(g: &mut Globals, k: usize, path: &str) {
    g.set_name_of_file(path);
    let mut f = AlphaFile::default();
    assert!(g.a_open_out(&mut f));
    g.write_file[k] = f;
}

fn write(g: &mut Globals, k: usize, s: &str) {
    wr_str(&mut g.write_file[k], s);
}

fn close(g: &mut Globals, k: usize) {
    let mut f = std::mem::take(&mut g.write_file[k]);
    g.a_close(&mut f);
}

fn read(p: &str) -> String {
    String::from_utf8_lossy(&std::fs::read(p).unwrap()).into_owned()
}

#[test]
fn two_streams_on_one_file() {
    let d = dir("two");
    let p = d.join("doc.toc").to_string_lossy().into_owned();
    let mut g = Globals::new();
    // The first `\tableofcontents` opens it on stream 0, the second on
    // stream 1 (truncating it again); only stream 1 is written, and what it
    // wrote is still in its buffer at the checkpoint.
    openout(&mut g, 0, &p);
    openout(&mut g, 1, &p);
    write(&mut g, 1, "\\contentsline {section}{Intro}{1}\n");
    let k = g.checkpoint().unwrap();
    write(&mut g, 1, "\\contentsline {section}{Later}{2}\n");
    g.checkpoint().unwrap();
    g.restore_discard(k).unwrap();
    write(&mut g, 1, "\\contentsline {section}{Again}{2}\n");
    close(&mut g, 1);
    close(&mut g, 0);
    assert_eq!(
        read(&p),
        "\\contentsline {section}{Intro}{1}\n\\contentsline {section}{Again}{2}\n"
    );
}

#[test]
fn an_idle_stream_keeps_its_position() {
    let d = dir("idle");
    let p = d.join("doc.toc").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    openout(&mut g, 1, &p);
    write(&mut g, 1, "abcdef");
    let k = g.checkpoint().unwrap();
    let rec = g.record_of(k).unwrap();
    let outs: Vec<(u64, u64)> = rec
        .files
        .iter()
        .filter_map(|f| match &f.stream {
            Stream::Out { path, len, at } if *path == p => Some((*len, *at)),
            _ => None,
        })
        .collect();
    // one length of the file for both; each stream where it writes next
    assert_eq!(outs, vec![(6, 0), (6, 6)]);
    write(&mut g, 1, "ghi");
    g.restore_discard(k).unwrap();
    // stream 0 writes at its own offset, the start (as C stdio's does)
    write(&mut g, 0, "XY");
    close(&mut g, 0);
    close(&mut g, 1);
    assert_eq!(read(&p), "XYcdef");
}

#[test]
fn a_file_opened_again_since_is_not_restored() {
    let d = dir("again");
    let p = d.join("doc.vrb").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "first frame");
    let k = g.checkpoint().unwrap();
    close(&mut g, 0);
    openout(&mut g, 0, &p);
    write(&mut g, 0, "2nd");
    close(&mut g, 0);
    // its bytes at `k` are gone: refuse (the caller compiles from scratch)
    // rather than cut the new file to the old length
    let e = g.restore_discard(k).unwrap_err();
    assert!(e.contains("opened for output again"), "{e}");
    let e = g.restore(k).unwrap_err();
    assert!(e.contains("opened for output again"), "{e}");
    assert_eq!(read(&p), "2nd");
}

#[test]
fn reattach_after_the_new_run_opened_a_file_again() {
    let d = dir("reattach");
    let p = d.join("doc.aux").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "abc");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, "def");
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    // the new run closes it and opens it again, then is abandoned
    close(&mut g, 0);
    openout(&mut g, 0, &p);
    write(&mut g, 0, "zz");
    close(&mut g, 0);
    g.reattach_pending().unwrap();
    // the old run's file and stream, whole
    write(&mut g, 0, "!");
    close(&mut g, 0);
    assert_eq!(read(&p), "abcdef!");
}

#[test]
fn reattach_drops_the_abandoned_runs_buffer() {
    let d = dir("buffer");
    let p = d.join("doc.aux").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "abc");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, "def");
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    // the new run's bytes, still in the stream's buffer
    write(&mut g, 0, "NEW");
    g.reattach_pending().unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), "abcdef");
}

#[test]
fn a_jump_keeps_what_only_the_new_run_wrote() {
    let d = dir("jump");
    let p = d.join("doc.vrb").to_string_lossy().into_owned();
    let mut g = Globals::new();
    let k = g.checkpoint().unwrap();
    // the old run writes the file whole before `j`
    openout(&mut g, 0, &p);
    write(&mut g, 0, "old frame");
    close(&mut g, 0);
    let j = g.checkpoint().unwrap();
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    // the new run writes it otherwise and converges at `j`; the old run
    // does not write it after `j`, so the new run's stays
    openout(&mut g, 0, &p);
    write(&mut g, 0, "edited frame");
    close(&mut g, 0);
    g.redo_to(j).unwrap();
    assert_eq!(read(&p), "edited frame");
}

#[test]
fn a_jump_takes_a_file_the_old_run_opened_again_whole() {
    let d = dir("jump2");
    let p = d.join("doc.vrb").to_string_lossy().into_owned();
    let mut g = Globals::new();
    let k = g.checkpoint().unwrap();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "frame 1");
    close(&mut g, 0);
    let j = g.checkpoint().unwrap();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "frame 2");
    close(&mut g, 0);
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "frame one, edited");
    close(&mut g, 0);
    g.redo_to(j).unwrap();
    assert_eq!(read(&p), "frame 2");
}

#[test]
fn a_jump_splices_a_stream_open_at_the_convergence_point() {
    let d = dir("splice");
    let p = d.join("doc.toc").to_string_lossy().into_owned();
    let mut g = Globals::new();
    // Both streams opened before anything is written: both at the end.
    openout(&mut g, 0, &p);
    openout(&mut g, 1, &p);
    let k = g.checkpoint().unwrap();
    write(&mut g, 1, "b22");
    let j = g.checkpoint().unwrap();
    write(&mut g, 1, "c333");
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    write(&mut g, 1, "B-edited");
    // stream 0 is behind the end now in both runs: no jump
    let e = g.redo_to(j).unwrap_err();
    assert!(e.contains("is at 0 of 8"), "{e}");

    let p = d.join("doc.aux").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "a1");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, "b22");
    let j = g.checkpoint().unwrap();
    write(&mut g, 0, "c333");
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    write(&mut g, 0, "B-edited");
    g.redo_to(j).unwrap();
    write(&mut g, 0, "!");
    close(&mut g, 0);
    assert_eq!(read(&p), "a1B-editedc333!");
}

#[test]
fn a_file_another_program_rewrote_is_not_restored() {
    let d = dir("outside");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "%PDF preview, stored streams");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, " and more pages");
    close(&mut g, 0);
    // an export of the same job in the same directory
    std::thread::sleep(std::time::Duration::from_millis(5));
    std::fs::write(&p, "%PDF exported, compressed, longer than the preview was").unwrap();
    for e in [g.restore(k).unwrap_err(), g.restore_discard(k).unwrap_err()] {
        assert!(e.contains("changed by another program"), "{e}");
    }
    assert_eq!(
        read(&p),
        "%PDF exported, compressed, longer than the preview was"
    );
}

/// The review's probe: the file is still open in the engine when another
/// program rewrites it (an export runs beside the resident engine).
#[test]
fn a_file_another_program_rewrote_while_open_is_not_restored() {
    let d = dir("outside-open");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "%PDF preview, stored streams");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, " and more pages");
    g.checkpoint().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    std::fs::write(&p, "%PDF exported, compressed, longer than the preview was").unwrap();
    for e in [g.restore(k).unwrap_err(), g.restore_discard(k).unwrap_err()] {
        assert!(e.contains("changed by another program"), "{e}");
    }
    // ... also once the engine has written more after it (the write that
    // follows another program's must not make the file the engine's)
    write(&mut g, 0, " page 3");
    g.checkpoint().unwrap();
    let e = g.restore_discard(k).unwrap_err();
    assert!(e.contains("changed by another program"), "{e}");
    close(&mut g, 0);
    assert!(read(&p).starts_with("%PDF exported"));
}

/// `\openout ./doc.vrb` truncates the `doc.vrb` a checkpoint holds open.
#[test]
fn a_file_opened_again_under_another_spelling_is_not_restored() {
    let d = dir("spelling");
    let p = d.join("doc.vrb").to_string_lossy().into_owned();
    let p2 = d.join(".").join("doc.vrb").to_string_lossy().into_owned();
    let sep = std::path::MAIN_SEPARATOR;
    assert!(p2.contains(&format!("{sep}.{sep}")));
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "first frame text");
    let k = g.checkpoint().unwrap();
    close(&mut g, 0);
    openout(&mut g, 0, &p2);
    write(&mut g, 0, "second frame, longer than the first");
    close(&mut g, 0);
    let e = g.restore_discard(k).unwrap_err();
    assert!(e.contains("opened for output again"), "{e}");
    assert_eq!(read(&p), "second frame, longer than the first");
}

/// A run that fails removes its PDF: the restore says so.
#[test]
fn a_removed_file_is_reported_as_gone() {
    let d = dir("gone");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "%PDF");
    let k = g.checkpoint().unwrap();
    close(&mut g, 0);
    std::fs::remove_file(&p).unwrap();
    let e = g.restore_discard(k).unwrap_err();
    assert!(e.contains("is gone"), "{e}");
}

/// A restore keeps the old run's tail in the file itself (lane
/// P4-PAGE-COST): nothing is cut until the stream closes, an abandon puts
/// back only what the new run wrote over, and a jump splices as before.
#[test]
fn a_kept_tail_stays_in_the_file_until_it_is_settled() {
    let d = dir("kept");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let tail = "0123456789".repeat(20);
    // the abandon: the new run writes over part of the tail, then more
    // than the whole old file
    for (new, more) in [
        ("XY", ""),
        ("edited", "and on past the old end".repeat(12).as_str()),
    ] {
        let mut g = Globals::new();
        openout(&mut g, 0, &p);
        write(&mut g, 0, "head");
        let k = g.checkpoint().unwrap();
        write(&mut g, 0, &tail);
        g.checkpoint().unwrap();
        g.restore(k).unwrap();
        // nothing cut: the file still holds the old run's bytes, its end
        // blanked (it reads as truncated, not as the old run's whole file)
        assert_eq!(read(&p), format!("head{}", " ".repeat(tail.len())));
        write(&mut g, 0, new);
        write(&mut g, 0, more);
        g.checkpoint().unwrap();
        g.reattach_pending().unwrap();
        close(&mut g, 0);
        assert_eq!(read(&p), format!("head{tail}"), "abandon after {new:?}");
    }
    // the jump: the new run's bytes, then the old run's later ones
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "head");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, "page");
    let j = g.checkpoint().unwrap();
    write(&mut g, 0, &tail);
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    write(&mut g, 0, "PAGE!");
    g.redo_to(j).unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), format!("headPAGE!{tail}"));
    // no jump and no abandon: the run's end cuts the file to what it wrote
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "head");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, &tail);
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    write(&mut g, 0, "short");
    g.checkpoint().unwrap();
    // the record has the logical length, not the file's
    let last = *g.checkpoints().last().unwrap();
    let len = g
        .record_of(last)
        .unwrap()
        .files
        .iter()
        .find_map(|f| match &f.stream {
            Stream::Out { path, len, .. } if *path == p => Some(*len),
            _ => None,
        });
    assert_eq!(len, Some(9));
    close(&mut g, 0);
    assert_eq!(read(&p), "headshort");
}

/// What another program reading a file sees while a restore keeps its
/// tail (the #1613 review's polling reader): never a zero byte, never the
/// file cut and then written on, and never the old run's end after the
/// new run's bytes. Of a tail longer than `BLANK_END`, only the end is
/// blanked, and the jump and the abandon give back every byte.
#[test]
fn a_kept_tail_is_blanked_at_its_end_only_and_comes_back_whole() {
    let d = dir("blank");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let n = BLANK_END as usize;
    let body: String = (0..3 * n)
        .map(|i| (b'a' + (i % 26) as u8) as char)
        .collect();
    let tail = format!("{body}\nstartxref\n123\n%%EOF\n");
    for jump in [false, true] {
        let mut g = Globals::new();
        openout(&mut g, 0, &p);
        write(&mut g, 0, "head");
        let k = g.checkpoint().unwrap();
        write(&mut g, 0, "page");
        let j = g.checkpoint().unwrap();
        write(&mut g, 0, &tail);
        g.checkpoint().unwrap();
        g.restore(k).unwrap();
        let old = format!("headpage{tail}");
        let now = read(&p);
        assert_eq!(now.len(), old.len());
        assert_eq!(now[..old.len() - n], old[..old.len() - n]);
        assert_eq!(now[old.len() - n..], " ".repeat(n));
        assert!(!now.contains('\0'));
        // the new run writes over part of the old bytes, not the end
        write(&mut g, 0, "PAGE!");
        g.checkpoint().unwrap();
        if jump {
            // (the old run's later bytes, the blanked ones too)
            g.redo_to(j).unwrap();
            close(&mut g, 0);
            assert_eq!(read(&p), format!("headPAGE!{tail}"));
        } else {
            g.reattach_pending().unwrap();
            close(&mut g, 0);
            assert_eq!(read(&p), old);
        }
    }
}

/// A restore that drops the later run (`restore_discard`) of a file longer
/// than the checkpoint had it: the file is not cut then written on (a
/// reader could see a hole); its end is blanked and the run's end cuts it.
#[test]
fn a_discarding_restore_cuts_a_longer_file_at_the_runs_end() {
    let d = dir("discard");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let tail = "0123456789".repeat(300);
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "head");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, &tail);
    g.checkpoint().unwrap();
    g.restore_discard(k).unwrap();
    let now = read(&p);
    let n = BLANK_END as usize;
    assert_eq!(now.len(), 4 + tail.len());
    assert_eq!(now[..now.len() - n], format!("head{tail}")[..now.len() - n]);
    assert_eq!(now[now.len() - n..], " ".repeat(n));
    // the record has the checkpoint's length, not the file's
    write(&mut g, 0, "short");
    g.checkpoint().unwrap();
    let last = *g.checkpoints().last().unwrap();
    let len = g
        .record_of(last)
        .unwrap()
        .files
        .iter()
        .find_map(|f| match &f.stream {
            Stream::Out { path, len, .. } if *path == p => Some(*len),
            _ => None,
        });
    assert_eq!(len, Some(9));
    close(&mut g, 0);
    assert_eq!(read(&p), "headshort");
}

/// An outside write to a tail the restore keeps in the file (an export in
/// the same directory) between the restore and the jump: the jump refuses
/// it (#1313's rule; the session then runs from scratch) rather than take
/// the other program's bytes for the old run's.
#[test]
fn a_jump_refuses_a_kept_tail_another_program_wrote() {
    let d = dir("jumpout");
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let mut g = Globals::new();
    openout(&mut g, 0, &p);
    write(&mut g, 0, "head");
    let k = g.checkpoint().unwrap();
    write(&mut g, 0, "page");
    let j = g.checkpoint().unwrap();
    write(&mut g, 0, "the old run's later pages");
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    write(&mut g, 0, "PAGE");
    g.checkpoint().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    std::fs::write(&p, "%PDF exported, compressed, longer than the preview was").unwrap();
    assert!(g.jump_blocked().is_some());
    let e = g.redo_to(j).unwrap_err();
    assert!(e.contains("changed by another program"), "{e}");
}

/// A large file the run does not read (a preview PDF): its path, and the
/// old run's text after a 4-byte head, `a` then `b` (#1608's cases, which
/// deferred such a tail at an abandon; kept tails, `system::keep_tail`,
/// leave it in the file instead).
fn old_pdf(name: &str) -> (String, String, String) {
    let d = dir(name);
    let p = d.join("doc.pdf").to_string_lossy().into_owned();
    let a: String = (0..100_000)
        .map(|i| (b'a' + (i % 26) as u8) as char)
        .collect();
    let b: String = (0..300_000)
        .map(|i| (b'A' + (i % 26) as u8) as char)
        .collect();
    (p, a, b)
}

/// An old run of `p`: "%PDF", checkpoint `k1`, `a`, checkpoint `k2`, `b`,
/// a last checkpoint.
fn old_run(g: &mut Globals, p: &str, a: &str, b: &str) -> (u64, u64) {
    openout(g, 0, p);
    write(g, 0, "%PDF");
    let k1 = g.checkpoint().unwrap();
    write(g, 0, a);
    let k2 = g.checkpoint().unwrap();
    write(g, 0, b);
    g.checkpoint().unwrap();
    (k1, k2)
}

#[test]
fn an_abandoned_runs_tail_is_the_next_restores_tail() {
    let (p, a, b) = old_pdf("defer-same");
    let mut g = Globals::new();
    let (k, _) = old_run(&mut g, &p, &a, &b);
    // a keystroke's run, stopped and abandoned
    g.restore(k).unwrap();
    write(&mut g, 0, "first edit");
    g.reattach_pending().unwrap();
    // the old run's file, whole (only the bytes the new run wrote over
    // were put back)
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
    // the next keystroke restores the same point: its tail is kept again
    g.restore(k).unwrap();
    write(&mut g, 0, "second edit");
    // abandoned too: the old run, whole
    g.reattach_pending().unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
}

#[test]
fn an_abandon_leaves_the_old_file() {
    let (p, a, b) = old_pdf("defer-flush");
    let mut g = Globals::new();
    let (k, _) = old_run(&mut g, &p, &a, &b);
    g.restore(k).unwrap();
    write(&mut g, 0, "an edit");
    g.reattach_pending().unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
}

#[test]
fn an_abandoned_runs_tail_serves_a_later_and_an_earlier_restore() {
    let (p, a, b) = old_pdf("defer-move");
    let mut g = Globals::new();
    let (k1, k2) = old_run(&mut g, &p, &a, &b);
    // abandoned from k1, then a restore at the later k2: the old bytes
    // between them are written, the rest is the tail
    g.restore(k1).unwrap();
    write(&mut g, 0, "x");
    g.reattach_pending().unwrap();
    g.restore(k2).unwrap();
    write(&mut g, 0, "y");
    g.reattach_pending().unwrap();
    // abandoned from k2, then a restore at the earlier k1: the old bytes
    // the disk holds between them, then the kept tail
    g.restore(k1).unwrap();
    write(&mut g, 0, "z");
    g.reattach_pending().unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
}

#[test]
fn a_run_after_an_abandon_writes_after_the_old_bytes() {
    let (p, a, b) = old_pdf("defer-run");
    let mut g = Globals::new();
    let (k1, k2) = old_run(&mut g, &p, &a, &b);
    g.restore(k1).unwrap();
    write(&mut g, 0, "x");
    g.reattach_pending().unwrap();
    g.restore(k2).unwrap();
    write(&mut g, 0, "the new end");
    close(&mut g, 0);
    assert_eq!(read(&p), format!("%PDF{a}the new end"));
}

#[test]
fn a_tail_the_run_reads_is_the_old_runs_after_an_abandon() {
    let (p, a, b) = old_pdf("defer-read");
    let mut g = Globals::new();
    let (k, _) = old_run(&mut g, &p, &a, &b);
    g.restore(k).unwrap();
    write(&mut g, 0, "an edit");
    // (the caller says the run reads it)
    g.reattach_pending().unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
}

#[test]
fn a_restore_before_the_file_was_opened_sees_the_old_file() {
    let (p, a, b) = old_pdf("defer-before-open");
    let mut g = Globals::new();
    let k0 = g.checkpoint().unwrap();
    let (k1, _) = old_run(&mut g, &p, &a, &b);
    g.restore(k1).unwrap();
    write(&mut g, 0, "x");
    g.reattach_pending().unwrap();
    // not open at k0: the new run may read the file before it writes it,
    // so the disk holds the old run's file again
    g.restore(k0).unwrap();
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
    g.reattach_pending().unwrap();
    close(&mut g, 0);
    assert_eq!(read(&p), format!("%PDF{a}{b}"));
}
