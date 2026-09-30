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
    openout(&mut g, 0, &p);
    openout(&mut g, 1, &p);
    write(&mut g, 1, "a1");
    let k = g.checkpoint().unwrap();
    write(&mut g, 1, "b22");
    let j = g.checkpoint().unwrap();
    write(&mut g, 1, "c333");
    g.checkpoint().unwrap();
    g.restore(k).unwrap();
    write(&mut g, 1, "B-edited");
    g.redo_to(j).unwrap();
    write(&mut g, 1, "!");
    close(&mut g, 1);
    close(&mut g, 0);
    assert_eq!(read(&p), "a1B-editedc333!");
}
