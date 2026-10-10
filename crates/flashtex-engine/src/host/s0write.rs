//! Writing persisted S₀ files off the engine thread (lane COLD-FIXED).
//!
//! The resident host saves S₀ after a full run (`--s0-cache`, DESIGN.md
//! §5.1). Writing it (scanning the word space's chunks for zeros, the file,
//! `fsync`, the rename) took 75–120 M instructions on the engine thread after
//! `DONE`, so a keystroke that came then waited for it. The engine thread now
//! only takes S₀ out of the engine (`super::prepare_s0`: the header and a copy
//! of the chunks, a few milliseconds); this thread does the rest.
//!
//! **Order.** One thread writes, in the order the saves were asked for. A
//! save of a path supersedes that path's pending one (it is dropped unwritten)
//! and the one being written (which stops between chunks and removes its
//! temporary file); so the newest S₀ of a path is the one left on disk.
//!
//! **Crash safety.** A write goes to `PATH.PID.tmp` and is renamed over `PATH`
//! once complete and synced, so `PATH` is always a whole S₀ or the previous
//! one. A host killed with a write pending leaves the previous S₀ (or none):
//! S₀ is a cache, and its key is checked whenever it is opened.
//!
//! **Memory.** The image is a copy of the word space's chunks (17–28 MB on
//! the benchmark documents). A newer save of a path drops the pending one
//! and stops the one being written, so at most two are alive, briefly. A
//! `lean` performance mode (Low Memory) does not use this thread: it writes
//! each S₀ on the engine thread, as before, and frees the copy at once.
//!
//! **Readers.** Before the host opens a persisted S₀ it waits for that
//! path's pending write ([`flush`]), so a reopen in this process finds what
//! the last save wrote. A host that ends does not wait: as when the save ran
//! on the engine thread, a save cut off at exit leaves the previous S₀ (the
//! rename never happened), and the host's lifetime stays what the app
//! expects.

use super::S0Image;
use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Instant;

/// What a finished save reports: the result of the write (bytes of the
/// file, bytes on disk) and its seconds on the writer thread.
pub type Done = Box<dyn FnOnce(Result<(u64, u64), String>, f64) + Send>;

struct Job {
    path: String,
    image: S0Image,
    gen: u64,
    done: Done,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    /// The path being written, if any.
    writing: Option<String>,
    /// Each path's newest save.
    newest: HashMap<String, u64>,
    gen: u64,
}

struct Writer {
    q: Mutex<Queue>,
    cv: Condvar,
}

fn writer() -> &'static Writer {
    static W: OnceLock<&'static Writer> = OnceLock::new();
    W.get_or_init(|| {
        let w: &'static Writer = Box::leak(Box::new(Writer {
            q: Mutex::new(Queue::default()),
            cv: Condvar::new(),
        }));
        let spawned = std::thread::Builder::new()
            .name("s0-writer".into())
            .spawn(move || run(w));
        if let Err(e) = spawned {
            eprintln!("flashtex-host: cannot start the S0 writer: {e}");
        }
        w
    })
}

fn run(w: &'static Writer) {
    loop {
        let job = {
            let mut q = w.q.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if let Some(j) = q.jobs.pop_front() {
                    q.writing = Some(j.path.clone());
                    break j;
                }
                q = w.cv.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let t = Instant::now();
        let (path, gen) = (job.path.clone(), job.gen);
        let superseded = || {
            let q = w.q.lock().unwrap_or_else(|e| e.into_inner());
            q.newest.get(&path) != Some(&gen)
        };
        let r = job.image.write(&job.path, &superseded);
        // (a superseded write reports nothing: the newer one will)
        if !(r.is_err() && superseded()) {
            (job.done)(r, t.elapsed().as_secs_f64());
        }
        let mut q = w.q.lock().unwrap_or_else(|e| e.into_inner());
        q.writing = None;
        if q.newest.get(&job.path) == Some(&gen) {
            q.newest.remove(&job.path);
        }
        w.cv.notify_all();
    }
}

/// Write `image` to `path` on the writer thread; `done` runs there once it
/// is written (or failed), unless a newer save of `path` superseded it.
pub fn submit(path: &str, image: S0Image, done: Done) {
    let w = writer();
    let mut q = w.q.lock().unwrap_or_else(|e| e.into_inner());
    q.gen += 1;
    let gen = q.gen;
    q.jobs.retain(|j| j.path != path);
    q.newest.insert(path.to_string(), gen);
    q.jobs.push_back(Job {
        path: path.to_string(),
        image,
        gen,
        done,
    });
    w.cv.notify_all();
}

/// Wait until no save of `path` (of any path, for `None`) is pending or
/// being written, or `limit` has passed. Returns whether none is left.
pub fn flush(path: Option<&str>, limit: std::time::Duration) -> bool {
    let w = writer();
    let end = Instant::now() + limit;
    let mut q = w.q.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        let busy = match path {
            Some(p) => q.writing.as_deref() == Some(p) || q.jobs.iter().any(|j| j.path == p),
            None => q.writing.is_some() || !q.jobs.is_empty(),
        };
        if !busy {
            return true;
        }
        let now = Instant::now();
        if now >= end {
            return false;
        }
        q =
            w.cv.wait_timeout(q, end - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::CHUNK_BYTES;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    /// An image of `n` chunks: every third one zero, the others filled
    /// with `fill` and their index.
    fn image(n: u32, fill: u8) -> S0Image {
        let chunks: Vec<u32> = (0..n).map(|c| c * 2 + 1).collect();
        let mut data = vec![0u8; n as usize * CHUNK_BYTES];
        for (i, d) in data.as_chunks_mut::<CHUNK_BYTES>().0.iter_mut().enumerate() {
            if i % 3 != 0 {
                d.fill(fill);
                d[..4].copy_from_slice(&(i as u32).to_le_bytes());
            }
        }
        S0Image {
            head: b"head of an S0 for the tests".to_vec(),
            tail: Box::new(|h: &mut Vec<u8>| h.extend_from_slice(b", its tail")),
            chunks,
            data: crate::host::Scratch::from_slice(&data),
        }
    }

    /// No temporary file is left in `d`.
    fn no_tmp(d: &std::path::Path) -> bool {
        std::fs::read_dir(d)
            .unwrap()
            .flatten()
            .all(|e| !e.file_name().to_string_lossy().ends_with(".tmp"))
    }

    fn dir(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("flashtex-s0write-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A save written by the writer thread is the file a write on the
    /// caller's thread makes, and leaves no temporary file.
    #[test]
    fn a_save_writes_what_a_direct_write_writes() {
        let d = dir("same");
        let (a, b) = (d.join("a.s0"), d.join("b.s0"));
        image(300, 7).write(a.to_str().unwrap(), &|| false).unwrap();
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        submit(
            b.to_str().unwrap(),
            image(300, 7),
            Box::new(move |r, _| {
                r.unwrap();
                n2.fetch_add(1, Ordering::SeqCst);
            }),
        );
        assert!(flush(Some(b.to_str().unwrap()), Duration::from_secs(60)));
        assert_eq!(n.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert!(no_tmp(&d));
    }

    /// Saves of one path in a row: the newest is the file left, whatever
    /// the writer had started; the superseded ones report nothing or
    /// succeeded before the next came.
    #[test]
    fn the_newest_save_of_a_path_wins() {
        let d = dir("newest");
        let (want, p) = (d.join("want.s0"), d.join("doc.s0"));
        let ps = p.to_str().unwrap();
        image(2000, 9)
            .write(want.to_str().unwrap(), &|| false)
            .unwrap();
        let reported = Arc::new(AtomicUsize::new(0));
        for fill in 1..=8u8 {
            let r = reported.clone();
            submit(
                ps,
                image(2000, fill),
                Box::new(move |res, _| {
                    res.unwrap();
                    r.fetch_add(1, Ordering::SeqCst);
                }),
            );
        }
        submit(
            ps,
            image(2000, 9),
            Box::new(|res, _| {
                res.unwrap();
            }),
        );
        assert!(flush(Some(ps), Duration::from_secs(60)));
        assert_eq!(std::fs::read(&want).unwrap(), std::fs::read(&p).unwrap());
        assert!(no_tmp(&d));
        assert!(reported.load(Ordering::SeqCst) <= 8);
    }

    /// A write told to stop leaves the file as it was and no temporary file.
    #[test]
    fn a_cancelled_write_leaves_the_old_file() {
        let d = dir("cancel");
        let p = d.join("doc.s0");
        let ps = p.to_str().unwrap();
        image(10, 1).write(ps, &|| false).unwrap();
        let before = std::fs::read(&p).unwrap();
        let r = image(5000, 2).write(ps, &|| true);
        assert!(r.is_err());
        assert_eq!(std::fs::read(&p).unwrap(), before);
        assert!(no_tmp(&d));
    }
}
