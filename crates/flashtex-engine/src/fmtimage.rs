//! The loaded format kept in memory (lane COLD-FIXED; `changes/fmtimage.ch`).
//!
//! A resident host runs from the format at every cold compile and every
//! preamble edit, and `load_fmt_file` undumps the format a word at a time:
//! about 116 M instructions for LaTeX's 15 MB format, a quarter of a
//! preamble keystroke on a one-page article.
//!
//! **Why an image may stand for the load.** `load_fmt_file` reads the state
//! it starts from and the format file, and writes the word space (every
//! array and, once spilled, every scalar global: `Globals` keeps nothing
//! else but file handles, and the load uses only `fmt_file` and, on its
//! failure paths, `term_out`) and the C parts' state (`undumptounicode`,
//! `undumpimagemeta`). So two loads of the same file from the same state
//! leave the same state. The first load in a thread keeps an image of what
//! it left: every nonzero chunk of the word space, the scalars spilled, and
//! the C state. A later load installs the image instead when
//!
//! * the format file is the same file, unchanged: the same path and identity
//!   (size, modification and status-change times, inode, device), neither
//!   time within the racy window (`system::racy_ns`, as for `StatSig`), and
//!   the identity the same before and after the image was taken; and
//! * the state before it is the same: every nonzero chunk of the word space
//!   (scalars spilled), compared word for word with a copy of the state
//!   before the first load (5 MB for LaTeX), and the C state, by its
//!   persisted encoding.
//!
//! Anything else (another document's first line in the buffer, another
//! clock, a changed format) is a miss: the format is loaded as before and
//! the image replaced -- but only by a load whose first line (the job's
//! command line) is the thread's previous load's: the job is loading again.
//! So a host's warm-up and a document's first compile, which nothing could
//! reuse yet, pay nothing; the first run of a document from the format after
//! its first (a class-line edit, a cold restart) takes the image, and the
//! next ones install it. Installing writes through the barrier only the chunks
//! that differ, so checkpoints see it as they see a load.
//!
//! The image is the size of the loaded word space (17 MB for LaTeX's format)
//! plus the state before the load (5 MB), one per engine thread.
//! `FLASHTEX_FMT_IMAGE=0` turns it off; `FLASHTEX_FMT_IMAGE_DEBUG=1` says on
//! stderr why a load missed (the first array that differs).
//! `FLASHTEX_FMT_IMAGE=verify` loads the format even where the image would
//! stand for the load, then compares what the load left with the image, word
//! for word and the C state by its encoding, and says on stderr
//! `fmtimage: verified` or `fmtimage: MISMATCH` (the tests).

use crate::arena::CHUNK_BYTES;
use crate::generated::Globals;
use crate::pdftex::CState;
use std::cell::RefCell;

/// The format file's identity (`os::file_stat`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Ident {
    size: u64,
    mtime_ns: i128,
    ctime_ns: i128,
    ino: u64,
    dev: u64,
}

impl Ident {
    /// `None` when the file cannot be read or is racy, and off Unix, where
    /// `os::file_stat` has no inode and no status-change time (Windows gives
    /// the creation time, which NTFS carries over to a file re-created under
    /// the same name): a format rewritten to the same size with its
    /// modification time put back would keep its identity there.
    fn of(path: &str) -> Option<Ident> {
        if !cfg!(unix) {
            return None;
        }
        let m = std::fs::metadata(path).ok()?;
        let s = crate::os::file_stat(&m);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i128);
        let w = crate::system::racy_ns();
        if (now - s.mtime_ns).abs() < w || (now - s.ctime_ns).abs() < w {
            return None;
        }
        Some(Ident {
            size: s.size,
            mtime_ns: s.mtime_ns,
            ctime_ns: s.ctime_ns,
            ino: s.ino,
            dev: s.dev,
        })
    }
}

/// The state before a load: the format, the nonzero chunks of the word space
/// (index and bytes, scalars spilled) and the C state's encoding.
struct Before {
    path: String,
    ident: Ident,
    chunks: Vec<u32>,
    data: Vec<u8>,
    c: Vec<u8>,
}

struct Image {
    before: Before,
    /// The nonzero chunks after the load, by index, and their bytes.
    chunks: Vec<u32>,
    data: Vec<u8>,
    c: CState,
}

thread_local! {
    static IMAGE: RefCell<Option<Image>> = const { RefCell::new(None) };
    /// The state before the load now going, for its image.
    static PENDING: RefCell<Option<Before>> = const { RefCell::new(None) };
    /// The load now going is one the image stands for (`verify`).
    static VERIFY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Off,
    On,
    Verify,
}

fn mode() -> Mode {
    static M: std::sync::OnceLock<Mode> = std::sync::OnceLock::new();
    *M.get_or_init(|| match std::env::var("FLASHTEX_FMT_IMAGE").as_deref() {
        Ok("0") => Mode::Off,
        Ok("verify") => Mode::Verify,
        _ => Mode::On,
    })
}

fn debug() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FLASHTEX_FMT_IMAGE_DEBUG").is_some())
}

fn chunk_bytes(g: &Globals, c: usize) -> &[u8] {
    let w = g.arena.chunk(c);
    // SAFETY: a chunk of u64 words viewed as bytes.
    unsafe { std::slice::from_raw_parts(w.as_ptr() as *const u8, CHUNK_BYTES) }
}

/// The nonzero chunks of the word space, in index order.
fn nonzero(g: &Globals) -> impl Iterator<Item = usize> + '_ {
    (0..g.arena.chunks())
        .filter(|&c| g.arena.touched(c) && g.arena.chunk(c).iter().any(|&w| w != 0))
}

/// The nonzero chunks and their bytes, into `chunks` and `data` (emptied
/// first; their memory is reused).
fn take_chunks_into(g: &Globals, chunks: &mut Vec<u32>, data: &mut Vec<u8>) {
    chunks.clear();
    data.clear();
    chunks.extend(nonzero(g).map(|c| c as u32));
    data.reserve(chunks.len() * CHUNK_BYTES);
    for &c in chunks.iter() {
        data.extend_from_slice(chunk_bytes(g, c as usize));
    }
}

fn take_chunks(g: &Globals) -> (Vec<u32>, Vec<u8>) {
    let (mut chunks, mut data) = (vec![], vec![]);
    take_chunks_into(g, &mut chunks, &mut data);
    (chunks, data)
}

/// Buffers for a new image: the old image's, whose memory is in place
/// already (a miss replaces it; a fresh 22 MB costs a page fault per page).
#[derive(Default)]
struct Spare {
    before: (Vec<u32>, Vec<u8>),
    after: (Vec<u32>, Vec<u8>),
}

thread_local! {
    static SPARE: RefCell<Spare> = RefCell::new(Spare::default());
    /// The first line of the thread's last load (the job's command line).
    static LAST_LINE: RefCell<Option<Vec<i32>>> = const { RefCell::new(None) };
}

fn c_state_bytes() -> Vec<u8> {
    crate::pdftex::with_state(|s| {
        let mut w = vec![];
        crate::persist::Codec::enc(&*s, &mut w);
        w
    })
}

/// Whether the state now is `b`, word for word (scalars spilled), and the
/// first chunk where it is not.
fn same_as(g: &Globals, b: &Before) -> Result<(), Option<usize>> {
    let mut k = 0;
    for c in nonzero(g) {
        if b.chunks.get(k) != Some(&(c as u32))
            || b.data[k * CHUNK_BYTES..(k + 1) * CHUNK_BYTES] != *chunk_bytes(g, c)
        {
            return Err(Some(c));
        }
        k += 1;
    }
    if k != b.chunks.len() {
        return Err(b.chunks.get(k).map(|&c| c as usize));
    }
    if c_state_bytes() != b.c {
        return Err(None);
    }
    Ok(())
}

impl Globals {
    /// `changes/fmtimage.ch`: after `open_fmt_file`, install the image of
    /// an earlier load from the same state, or note the state for this
    /// load's image. `true`: installed (`load_fmt_file` is not called).
    pub fn flashtex_fmt_restore(&mut self) -> bool {
        PENDING.with(|p| p.borrow_mut().take());
        VERIFY.with(|v| v.set(false));
        if mode() == Mode::Off {
            return false;
        }
        let Some(path) = self.fmt_file.path().map(str::to_string) else {
            return false;
        };
        let Some(ident) = Ident::of(&path) else {
            return false;
        };
        self.spill_scalars();
        // Only a job that loads the format again keeps an image: a host's
        // warm-up and a document's first compile (each the first load of its
        // command line) would pay for an image nothing uses yet.
        let line: Vec<i32> = (0..=self.last.max(0) as usize)
            .map(|k| self.buffer[k])
            .collect();
        let again = LAST_LINE.with(|l| l.replace(Some(line.clone()))) == Some(line);
        let same = IMAGE.with(|i| {
            let i = i.borrow();
            let img = i.as_ref()?;
            if img.before.path != path || img.before.ident != ident {
                if debug() {
                    eprintln!("fmtimage: another format file than {}", img.before.path);
                }
                return None;
            }
            match same_as(self, &img.before) {
                Ok(()) => Some(()),
                Err(at) => {
                    if debug() {
                        let r = at.map(|c| self.arena.region_at(c * CHUNK_BYTES).0.name);
                        eprintln!("fmtimage: the state before the load differs, first in {r:?}");
                    }
                    None
                }
            }
        });
        if same.is_none() {
            if !again {
                return false;
            }
            // The image will be replaced: its memory takes the new one.
            if let Some(old) = IMAGE.with(|i| i.borrow_mut().take()) {
                SPARE.with(|s| {
                    *s.borrow_mut() = Spare {
                        before: (old.before.chunks, old.before.data),
                        after: (old.chunks, old.data),
                    }
                });
            }
            let (mut chunks, mut data) = SPARE.with(|s| std::mem::take(&mut s.borrow_mut().before));
            take_chunks_into(self, &mut chunks, &mut data);
            let before = Before {
                path,
                ident,
                chunks,
                data,
                c: c_state_bytes(),
            };
            PENDING.with(|p| *p.borrow_mut() = Some(before));
            return false;
        }
        if mode() == Mode::Verify {
            VERIFY.with(|v| v.set(true));
            return false;
        }
        IMAGE.with(|i| {
            let i = i.borrow();
            let img = i.as_ref().expect("the image just compared");
            // The chunks the load leaves zero, then the image's.
            for &c in &img.before.chunks {
                if img.chunks.binary_search(&c).is_err() {
                    self.arena
                        .write_through(c as usize * CHUNK_BYTES, &[0u8; CHUNK_BYTES]);
                }
            }
            for (d, &c) in img
                .data
                .as_chunks::<CHUNK_BYTES>()
                .0
                .iter()
                .zip(&img.chunks)
            {
                self.arena.write_through(c as usize * CHUNK_BYTES, d);
            }
            let c = img.c.clone();
            crate::pdftex::with_state(|s| *s = c);
        });
        self.fill_scalars();
        if debug() {
            eprintln!("fmtimage: installed");
        }
        true
    }

    /// `changes/fmtimage.ch`: a load succeeded; keep its image.
    pub fn flashtex_fmt_loaded(&mut self) {
        if VERIFY.with(|v| v.replace(false)) {
            self.spill_scalars();
            let (chunks, data) = take_chunks(self);
            let c = c_state_bytes();
            let same = IMAGE.with(|i| {
                let i = i.borrow();
                let img = i.as_ref().expect("the image verified");
                let mut w = vec![];
                crate::persist::Codec::enc(&img.c, &mut w);
                img.chunks == chunks && img.data == data && w == c
            });
            eprintln!(
                "fmtimage: {}",
                if same {
                    "verified: the load left the image"
                } else {
                    "MISMATCH: the load did not leave the image"
                }
            );
            return;
        }
        let Some(before) = PENDING.with(|p| p.borrow_mut().take()) else {
            return;
        };
        // (a format changed while it was read is not kept)
        if Ident::of(&before.path) != Some(before.ident) {
            return;
        }
        self.spill_scalars();
        let (mut chunks, mut data) = SPARE.with(|s| std::mem::take(&mut s.borrow_mut().after));
        take_chunks_into(self, &mut chunks, &mut data);
        let c = crate::pdftex::with_state(|s| s.clone());
        IMAGE.with(|i| {
            *i.borrow_mut() = Some(Image {
                before,
                chunks,
                data,
                c,
            })
        });
    }
}

/// Forget the image (the tests).
pub fn forget() {
    IMAGE.with(|i| i.borrow_mut().take());
    PENDING.with(|p| p.borrow_mut().take());
    SPARE.with(|s| s.take());
}

/// The bytes the image holds (both copies of the word space).
pub fn bytes() -> usize {
    IMAGE.with(|i| {
        i.borrow()
            .as_ref()
            .map_or(0, |i| i.data.len() + i.before.data.len())
    })
}
