//! L1: the resident engine and the begin-document snapshot S₀ (DESIGN.md
//! §5.1).
//!
//! A [`Session`] keeps one engine per document in the process. Its first
//! compile is a full run that takes S₀ on the way (`changes/checkpoint.ch`:
//! the first `big_switch` after the expansion of `\document` has been
//! consumed) and records the run's read-set up to that point. A later
//! compile checks S₀'s [`Key`]; if nothing S₀ depends on has changed it
//! restores S₀ and re-runs only the rest of the job, else it runs in full
//! and takes a new S₀.
//!
//! **Why S₀'s point is safe.** It is a checkpoint like any other (all engine
//! state, `big_switch`, nothing on the Rust stack but `main_control`), so
//! restoring it and running on is the uninterrupted run, provided every
//! input the run had consumed by then is unchanged. The key says what that
//! input was: every file opened (content hash at open, so the `.aux` is the
//! one `\document` read, before it rewrites it), every lookup (a file that
//! appears later changes a run as surely as an edit), and, for each input
//! file still open, the bytes consumed so far -- whole lines, since TeX
//! holds the current line in its buffer. The pinned clock, the first line,
//! the job name and the engine build complete it. Anything no key can
//! describe (a shell command, a pipe, a font made by mktex) makes S₀
//! unusable. The point is also the earliest after `\document` that needs no
//! LaTeX-specific knowledge: `\document` ends with `\ignorespaces`, which
//! may already expand the first body material, so S₀ can sit a few tokens
//! into the body; the key then covers those lines too.
//!
//! S₀ persists to a file ([`Session::save_s0`]) and opens in a fresh
//! process ([`Session::open_s0`]): a header (the key, the host record, the
//! output files' prefixes, the list of the word space's nonzero chunks) and
//! those chunks, 16 KB-aligned, which are mapped and copied in.

pub mod crash;
pub mod diag;
pub mod external;
mod resident;
pub mod server;
pub mod tools;

use crate::arena::{CheckpointId, CHUNK_BYTES};
use crate::checkpoint::ExtRecord;
use crate::generated::Globals;
use crate::persist::{Codec, Reader};
use crate::resolver::Format;
use crate::system::{self, Lookup, RunOptions, StatSig, Stream};
use std::collections::HashMap;
use std::time::Instant;

/// What S₀ depends on.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// The engine build: a hash of the running executable.
    pub build: [u64; 2],
    /// The pinned clock (seconds, microseconds).
    pub clock: (i64, i32),
    pub source_date_epoch: Option<String>,
    pub force_source_date: Option<String>,
    /// The first line (the command line's file name or TeX code).
    pub first_line: Vec<u8>,
    pub job_name: String,
    /// Files opened and read: path, content hash at open, file identity.
    pub files: Vec<(String, [u64; 2], StatSig)>,
    /// Input files open at S₀: path, bytes consumed, their hash, identity.
    pub prefixes: Vec<(String, u64, [u64; 2], StatSig)>,
    /// Every lookup: name, format, `find_ex`'s flag, what it found.
    pub lookups: Vec<(String, u8, Option<bool>, Option<String>)>,
    /// External effects before S₀ (none, or S₀ is not used).
    pub barriers: Vec<String>,
    /// Files the run wrote and closed before S₀, with their contents then:
    /// a restore writes them again (a later part of the old run may have
    /// rewritten them, and a full run would write them first).
    pub written: Vec<(String, Vec<u8>)>,
    /// The directories whose listing a lookup depends on and that the
    /// TeX distribution does not own: the working directory and the
    /// directory of every user file found (`system::is_user_file`), with
    /// their stat signatures. While none has changed (a file added,
    /// removed or renamed changes its directory's), the lookups are not run
    /// again: the distribution's trees are taken as unchanged for the
    /// session, as kpathsea's own `ls-R` cache takes them.
    pub dirs: Vec<(String, StatSig)>,
}

impl Codec for StatSig {
    fn enc(&self, w: &mut Vec<u8>) {
        self.len.enc(w);
        (self.mtime_ns as i64).enc(w);
        ((self.mtime_ns >> 64) as i64).enc(w);
        self.ino.enc(w);
        (self.racy as u64).enc(w);
    }
    fn dec(r: &mut Reader) -> Result<Self, String> {
        let len = u64::dec(r)?;
        let lo = i64::dec(r)? as u64 as i128;
        let hi = i64::dec(r)? as i128;
        Ok(StatSig {
            len,
            mtime_ns: (hi << 64) | lo,
            ino: u64::dec(r)?,
            racy: u64::dec(r)? != 0,
        })
    }
}

crate::codec_struct!(Key {
    build,
    clock,
    source_date_epoch,
    force_source_date,
    first_line,
    job_name,
    files,
    prefixes,
    lookups,
    barriers,
    written,
    dirs
});

fn format_index(f: Format) -> u8 {
    Format::all().iter().position(|&x| x == f).unwrap_or(0) as u8
}

/// `FLASHTEX_PIN_CLOCK=SECONDS.MICROSECONDS`: pin the clock for this
/// process (both binaries), so that runs in different processes seed
/// `\pdfuniformdeviate` alike (the tests). Returns the pin.
pub fn pin_clock_from_env() -> Option<(i64, i32)> {
    let v = std::env::var("FLASHTEX_PIN_CLOCK").ok()?;
    let (s, m) = v.split_once('.').unwrap_or((&v, "0"));
    let t = (s.parse().ok()?, format!("{m:0<6}")[..6].parse().ok()?);
    crate::pdftex::utils::pin_clock(Some(t));
    crate::pdftex::utils::arm_pinned_seed();
    Some(t)
}

/// The running executable's hash, once per process.
pub fn engine_build() -> [u64; 2] {
    static BUILD: std::sync::OnceLock<[u64; 2]> = std::sync::OnceLock::new();
    *BUILD.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| crate::persist::hash128_file(p.to_str()?, None).ok())
            .map_or([0, 0], |(h, _)| h)
    })
}

fn hash_prefix(path: &str, len: u64) -> Result<[u64; 2], String> {
    let (h, n) =
        crate::persist::hash128_file(path, Some(len)).map_err(|e| format!("{path}: {e}"))?;
    if n < len {
        return Err(format!("{path} is shorter than the {len} bytes read"));
    }
    Ok(h)
}

impl Key {
    /// Whether S₀ is still what a full run would reach: `Err` says why not.
    pub fn check(&self, session_clock: (i64, i32), first_line: &[u8]) -> Result<(), String> {
        self.check_fresh(session_clock, first_line).map(|_| ())
    }

    /// [`check`](Self::check), then keep the signatures it verified another
    /// way (by content, or by running the lookups again): taken before that
    /// test, they stand for the state it found unchanged. A signature taken
    /// within the modification-time tick of its file (`StatSig::racy`)
    /// proves nothing, so a key taken just after the run wrote its output
    /// directory would otherwise test by content, and run every lookup
    /// again, at every compile; with the fresh signatures, once the tick has
    /// passed, it does not.
    pub fn check_refresh(
        &mut self,
        session_clock: (i64, i32),
        first_line: &[u8],
    ) -> Result<(), String> {
        let fresh = self.check_fresh(session_clock, first_line)?;
        for (i, s) in fresh.files {
            self.files[i].2 = s;
        }
        for (i, s) in fresh.prefixes {
            self.prefixes[i].3 = s;
        }
        if let Some(d) = fresh.dirs {
            for (i, s) in d.into_iter().enumerate() {
                self.dirs[i].1 = s;
            }
        }
        Ok(())
    }

    /// The test, and the signatures it verified by other means: (index,
    /// signature now) of files and prefixes compared by content, and every
    /// directory's signature when the lookups ran again and all held.
    // (merging #1551 into #1552's test: #1551's `check_run`, which a
    // preamble restart uses alone, and `check_files`; #1552's `Fresh`
    // signatures come out of `check_files`)
    fn check_fresh(&self, session_clock: (i64, i32), first_line: &[u8]) -> Result<Fresh, String> {
        self.check_run(session_clock, first_line)?;
        if let Some(b) = self.barriers.first() {
            return Err(format!("the preamble ran an external command ({b})"));
        }
        self.check_files()
    }

    /// The part of `check` that is not about what the run read: the engine
    /// build, the clock, the date variables, the first line.
    pub fn check_run(&self, session_clock: (i64, i32), first_line: &[u8]) -> Result<(), String> {
        if self.build != engine_build() {
            return Err("the engine build changed".into());
        }
        if self.clock != session_clock {
            return Err("the pinned clock changed".into());
        }
        if self.source_date_epoch != std::env::var("SOURCE_DATE_EPOCH").ok()
            || self.force_source_date != std::env::var("FORCE_SOURCE_DATE").ok()
        {
            return Err("SOURCE_DATE_EPOCH or FORCE_SOURCE_DATE changed".into());
        }
        if self.first_line != first_line {
            return Err("the first line changed".into());
        }
        Ok(())
    }

    /// The part of `check` about the files and lookups the run read.
    fn check_files<'a>(&'a self) -> Result<Fresh, String> {
        let mut fresh = Fresh::default();
        // One signature and one content hash per path: a preamble opens
        // many files more than once (beamer's: 432 reads of 189 files), and
        // the key lists every read. Each read is still compared with what
        // it recorded, against the file as this check found it.
        let mut sigs: HashMap<&str, Option<StatSig>> = HashMap::new();
        let mut sig_of = |path: &'a str| *sigs.entry(path).or_insert_with(|| StatSig::of(path));
        let mut hashes: HashMap<&str, Option<[u64; 2]>> = HashMap::new();
        // A file both written before S₀ and read before it is keyed by
        // what was read; `rewrite_outputs` puts back what was written.
        for (i, (path, hash, stat)) in self.files.iter().enumerate() {
            let sig = sig_of(path);
            if sig.as_ref() == Some(stat) {
                continue;
            }
            let now = *hashes.entry(path).or_insert_with(|| {
                crate::persist::hash128_file(path, None)
                    .ok()
                    .map(|(h, _)| h)
            });
            if now != Some(*hash) {
                return Err(format!("{path} changed"));
            }
            if let Some(s) = sig {
                fresh.files.push((i, s));
            }
        }
        for (i, (path, len, hash, stat)) in self.prefixes.iter().enumerate() {
            let sig = sig_of(path);
            if sig.as_ref() == Some(stat) {
                continue;
            }
            if hash_prefix(path, *len).ok() != Some(*hash) {
                return Err(format!("{path} changed in the {len} bytes read before S0"));
            }
            if let Some(s) = sig {
                fresh.prefixes.push((i, s));
            }
        }
        // (taken before the lookups: a directory changed while they run
        // shows as changed next time)
        let now: Vec<Option<StatSig>> = self.dirs.iter().map(|(d, _)| system::dep_sig(d)).collect();
        let dirs_same = !self.dirs.is_empty()
            && self
                .dirs
                .iter()
                .zip(&now)
                .all(|((_, s), n)| n.as_ref() == Some(s));
        // Whether the answers depend on nothing the key does not watch (a
        // lookup that finds the same may depend on more than when S₀ was
        // taken, #1562): if they do, the signatures stay as they were, and
        // every check makes the lookups again.
        let mut covered = true;
        for (name, fmt, must, found) in self.lookups.iter().filter(|_| !dirs_same) {
            let l = Lookup {
                name: name.clone(),
                format: Format::all()[*fmt as usize],
                must_exist: *must,
                found: found.clone(),
            };
            let (again, deps) = system::lookup_again_deps(&l);
            if again != *found {
                return Err(format!("looking up {name} finds another file now"));
            }
            covered &= deps
                .iter()
                .all(|(d, _)| self.dirs.iter().any(|(x, _)| x == d));
        }
        if !dirs_same && covered && now.iter().all(Option::is_some) {
            fresh.dirs = Some(now.into_iter().flatten().collect());
        }
        Ok(fresh)
    }
}

/// What [`Key::check_fresh`] verified other than by signature.
#[derive(Default)]
struct Fresh {
    files: Vec<(usize, StatSig)>,
    prefixes: Vec<(usize, StatSig)>,
    dirs: Option<Vec<StatSig>>,
}

impl Key {
    /// Write back the files the run wrote and closed before S₀, where they
    /// differ (before resuming from S₀).
    pub fn rewrite_outputs(&self) -> Result<(), String> {
        for (p, d) in &self.written {
            if std::fs::read(p).ok().as_deref() != Some(&d[..]) {
                std::fs::write(p, d).map_err(|e| format!("{p}: {e}"))?;
            }
            // what S₀'s streams recorded: the engine's again
            crate::system::stamp_output(p);
        }
        Ok(())
    }
}

/// S₀ of a session: the checkpoint and its key.
pub struct S0 {
    pub id: CheckpointId,
    pub key: Key,
}

/// How one compile went.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// `cold` (full run), `s0` (restored S₀) or `cold-after-invalid`.
    pub mode: String,
    pub status: i32,
    /// Why S₀ could not be used, if it could not.
    pub invalid: Option<String>,
    pub validate_s: f64,
    pub restore_s: f64,
    pub run_s: f64,
    pub total_s: f64,
    /// Time from the start of a cold run to S₀.
    pub s0_at_s: f64,
    /// Time the cold run spent taking checkpoints.
    pub checkpoint_s: f64,
    /// Time from the start of the compile to the end of the first shipout.
    pub first_page_s: f64,
    pub pages: usize,
    /// Bytes of undo log S₀ held when it was restored.
    pub log_bytes: usize,
    /// The checkpoints of a cold run (`Stats::json`).
    pub checkpoint_stats: Option<String>,
}

impl Report {
    pub fn json(&self) -> String {
        format!(
            "{{\"mode\":\"{}\",\"status\":{},\"invalid\":{},\"validate_s\":{:.6},\"restore_s\":{:.6},\"run_s\":{:.6},\"total_s\":{:.6},\"first_page_s\":{:.6},\"pages\":{},\"s0_at_s\":{:.6},\"checkpoint_s\":{:.6},\"log_bytes\":{}}}",
            self.mode,
            self.status,
            self.invalid
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "null".into()),
            self.validate_s,
            self.restore_s,
            self.run_s,
            self.total_s,
            self.first_page_s,
            self.pages,
            self.s0_at_s,
            self.checkpoint_s,
            self.log_bytes
        )
        .trim_end_matches('}')
        .to_string()
            + &format!(
                ",\"checkpoint_stats\":{}}}",
                self.checkpoint_stats.as_deref().unwrap_or("null")
            )
    }
}

/// One document's resident engine.
pub struct Session {
    pub g: Option<Box<Globals>>,
    first_line: Vec<u8>,
    clock: (i64, i32),
    pub s0: Option<S0>,
    /// Take a checkpoint after every shipout too (L2; off for L1).
    pub every_shipout: bool,
    /// Hash the state at every checkpoint (the bit-identity tests).
    pub hash_states: bool,
    /// S₀ was checked just now (by `open_s0`): the next compile need not.
    fresh: bool,
}

/// The first line `system::configure` makes of the arguments (texmfmp.c's
/// `topenin`).
pub fn first_line_of(o: &RunOptions) -> Vec<u8> {
    let mut line: Vec<u8> = vec![];
    for a in &o.args {
        line.extend_from_slice(a.as_bytes());
        line.push(b' ');
    }
    while matches!(line.last(), Some(b' ' | b'\r' | b'\n')) {
        line.pop();
    }
    line
}

impl Session {
    /// Set the process up for the job `o` describes (pdfTeX's command line)
    /// and pin the clock for the session.
    pub fn new(o: RunOptions, clock: Option<(i64, i32)>) -> Session {
        let clock = clock.or_else(pin_clock_from_env).unwrap_or_else(|| {
            let d = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (d.as_secs() as i64, d.subsec_micros() as i32)
        });
        crate::pdftex::utils::pin_clock(Some(clock));
        let first_line = first_line_of(&o);
        system::configure(o);
        system::capture_terminal(true);
        Session {
            g: None,
            first_line,
            clock,
            s0: None,
            every_shipout: false,
            hash_states: false,
            fresh: false,
        }
    }

    pub fn clock(&self) -> (i64, i32) {
        self.clock
    }

    /// The captured terminal of the last compile.
    pub fn terminal(&self) -> Vec<u8> {
        system::terminal_bytes()
    }

    /// Compile: from S₀ when its key still holds, else from the start.
    pub fn compile(&mut self) -> Result<Report, String> {
        let t0 = Instant::now();
        if let Some(s0) = &self.s0 {
            let v = if std::mem::take(&mut self.fresh) {
                Ok(())
            } else {
                s0.key.check(self.clock, &self.first_line)
            };
            let validate_s = t0.elapsed().as_secs_f64();
            match v {
                Ok(()) => {
                    let id = s0.id;
                    let t1 = Instant::now();
                    s0.key.rewrite_outputs()?;
                    let g = self.g.as_mut().unwrap();
                    let log_bytes = g.arena.log_bytes();
                    g.restore_discard(id)?;
                    g.note_shipouts(true);
                    let restore_s = t1.elapsed().as_secs_f64();
                    let t2 = Instant::now();
                    let status = g.resume_to_end()?;
                    let run_s = t2.elapsed().as_secs_f64();
                    let times = g.layer().shipout_times.clone();
                    return Ok(Report {
                        first_page_s: times.first().map_or(0.0, |t| t + validate_s + restore_s),
                        pages: times.len(),
                        mode: "s0".into(),
                        status,
                        validate_s,
                        restore_s,
                        run_s,
                        total_s: t0.elapsed().as_secs_f64(),
                        log_bytes,
                        ..Report::default()
                    });
                }
                Err(why) => {
                    let mut r = self.cold()?;
                    r.mode = "cold-after-invalid".into();
                    r.invalid = Some(why);
                    r.validate_s = validate_s;
                    r.total_s = t0.elapsed().as_secs_f64();
                    return Ok(r);
                }
            }
        }
        self.cold()
    }

    /// A full run from the format, taking a new S₀.
    pub fn cold(&mut self) -> Result<Report, String> {
        let t0 = Instant::now();
        self.s0 = None;
        self.g = None;
        crate::pdftex::reset_state();
        crate::pdftex::utils::arm_pinned_seed();
        system::truncate_terminal(0);
        crate::diag::reset();
        system::truncate_external_effects(0);
        system::record_reads(true);
        system::set_command_line(vec![self.first_line.clone()]);
        let mut g = Globals::new();
        g.arm_begin_document();
        g.checkpoint_every_shipout(self.every_shipout);
        g.note_shipouts(true);
        g.layer().hash_states = self.hash_states;
        let status = g.run_to_end();
        system::record_reads(false);
        let status = status?;
        let run_s = t0.elapsed().as_secs_f64();
        let (s0_id, reads, checkpoint_s, errors) = {
            let l = g.layer();
            (l.s0, l.s0_reads.take(), l.seconds, l.errors.clone())
        };
        let times = g.layer().shipout_times.clone();
        let stats = g.layer().stats.json();
        let mut s0_at_s = 0.0;
        if let (Some(id), Some(reads)) = (s0_id, reads) {
            s0_at_s = g.layer().s0_elapsed;
            let rec = g.record_of(id)?;
            match self.make_key(&mut g, id, &rec, reads) {
                Ok(key) => self.s0 = Some(S0 { id, key }),
                Err(e) => eprintln!("flashtex-host: no S0: {e}"),
            }
        } else if let Some(e) = errors.first() {
            eprintln!("flashtex-host: no S0: {e}");
        }
        self.g = Some(g);
        Ok(Report {
            mode: "cold".into(),
            status,
            first_page_s: times.first().copied().unwrap_or(0.0),
            pages: times.len(),
            checkpoint_stats: Some(stats),
            run_s,
            total_s: t0.elapsed().as_secs_f64(),
            s0_at_s,
            checkpoint_s,
            ..Report::default()
        })
    }

    fn make_key(
        &self,
        g: &mut Globals,
        id: CheckpointId,
        rec: &ExtRecord,
        reads: system::ReadLog,
    ) -> Result<Key, String> {
        make_key(g, id, rec, &reads, self.clock, &self.first_line)
    }
}

/// Files with their content (S₀'s key's `written`).
pub type Written = Vec<(String, Vec<u8>)>;

/// The files a run had written and closed at checkpoint `rec` (`reads`:
/// what it had read and written so far, or more), with their content on
/// disk now: S₀'s key's `written`, read when the checkpoint is taken
/// (`Layer::written_at`).
pub fn written_before(rec: &ExtRecord, reads: &system::ReadLog) -> Result<Written, String> {
    let no = if rec.reads == (0, 0, 0) {
        reads.outputs.len()
    } else {
        rec.reads.2
    };
    let mut written = vec![];
    for p in &reads.outputs[..no.min(reads.outputs.len())] {
        let open = rec
            .files
            .iter()
            .any(|f| matches!(&f.stream, Stream::Out { path, .. } if path == p));
        if !open {
            let d = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
            written.push((p.clone(), d));
        }
    }
    Ok(written)
}

/// S₀'s key: what the run read before S₀ (`reads`, whose files and lookups
/// may run past S₀: only the first `rec.reads` count), with S₀'s host
/// record `rec`.
/// A whole read (`system::note_whole_read`) the run made after
/// `\document`'s body was pushed (`Layer::arm_reads`), before S₀'s anchor:
/// `\document`'s `\IfFileExists{\jobname.aux}` (l3's lookup takes the
/// file's `\pdffilesize`). The anchor is the `.aux` point, and the `.aux`
/// is the journal's (L5, `crate::readset`), not S₀'s key's: such a read is
/// left out of the key, and the session checks it with the journal
/// (`incr::Session::take_s0`'s list of files not keyed whole).
pub fn whole_after_arm(arm: Option<usize>, i: usize, f: &system::FileRead) -> bool {
    f.closed_at == Some(u64::MAX) && arm.is_some_and(|a| i >= a)
}

pub fn make_key(
    g: &mut Globals,
    id: CheckpointId,
    rec: &ExtRecord,
    reads: &system::ReadLog,
    clock: (i64, i32),
    first_line: &[u8],
) -> Result<Key, String> {
    {
        let mut prefixes = vec![];
        let mut open_paths = vec![];
        for f in &rec.files {
            if let Stream::In { path, offset } = &f.stream {
                prefixes.push((
                    path.clone(),
                    *offset,
                    hash_prefix(path, *offset)?,
                    StatSig::of(path).unwrap_or_default(),
                ));
                open_paths.push(path.clone());
            }
        }
        let (nf, nl, _) = if rec.reads == (0, 0, 0) {
            (reads.files.len(), reads.lookups.len(), reads.outputs.len())
        } else {
            rec.reads
        };
        // As they were at the checkpoint (`written_at`, taken there), not
        // as the end of the run left them.
        let wa = &mut g.layer().written_at;
        let written = match wa.iter().position(|(i, _)| *i == id) {
            Some(k) => wa.swap_remove(k).1,
            None => written_before(rec, reads)?,
        };
        // A file open at S₀ is keyed by the prefix consumed, unless the run
        // also read all of it at once (`\pdffilesize{\jobname.tex}`:
        // `system::note_whole_read`): then by all of it. A read of its
        // modification time (`\pdffilemoddate`, `FileRead::stamp`) is keyed
        // by the time too: mixed into the hash, so that a file whose time
        // changed fails the test by content (and the session finds the
        // read in `changes`).
        // (a whole read after `\document`'s body was pushed, before the
        // anchor -- its `\IfFileExists{\jobname.aux}` -- is the journal's:
        // `whole_after_arm`)
        let arm = g.layer().arm_reads;
        let files = reads.files[..nf.min(reads.files.len())]
            .iter()
            .enumerate()
            .filter(|(i, f)| !whole_after_arm(arm, *i, f))
            .map(|(_, f)| f)
            .filter(|f| !open_paths.contains(&f.path) || f.closed_at == Some(u64::MAX))
            .map(|f| {
                let hash = match (&f.content, f.hash) {
                    (Some(c), [0, 0]) => crate::persist::hash128(c),
                    (_, h) => h,
                };
                let hash = match f.stamp {
                    Some(t) => [
                        hash[0] ^ (t as u64).rotate_left(17) ^ 0x5354_414d_5000_0000,
                        hash[1],
                    ],
                    None => hash,
                };
                (f.path.clone(), hash, f.stat)
            })
            .collect();
        // (the journal lists a repeated lookup each time; the key once)
        let mut lookup_seen = std::collections::HashSet::new();
        let lookups: Vec<(String, u8, Option<bool>, Option<String>)> = reads.lookups
            [..nl.min(reads.lookups.len())]
            .iter()
            .filter(|l| {
                lookup_seen.insert((
                    l.name.clone(),
                    format_index(l.format),
                    l.must_exist,
                    l.found.clone(),
                ))
            })
            .map(|l| {
                (
                    l.name.clone(),
                    format_index(l.format),
                    l.must_exist,
                    l.found.clone(),
                )
            })
            .collect();
        let job_name = if g.job_name > 0 {
            String::from_utf8_lossy(&g.str_bytes(g.job_name)).into_owned()
        } else {
            String::new()
        };
        let dirs = reads.dirs.clone();
        Ok(Key {
            build: engine_build(),
            clock,
            source_date_epoch: std::env::var("SOURCE_DATE_EPOCH").ok(),
            force_source_date: std::env::var("FORCE_SOURCE_DATE").ok(),
            first_line: first_line.to_vec(),
            job_name,
            files,
            prefixes,
            lookups,
            // Only the commands run before S₀, as files and lookups are cut
            // to `rec.reads`: a `\write18` in the body (imakeidx's makeindex
            // at `\printindex`) is the body's, which every run from S₀ runs
            // again, and leaves S₀ what a full run would reach. Counted by
            // S₀'s own record (`rec.effects_len`, the effects a cold run had
            // executed when it took S₀), not by the journal, which notes a
            // barrier only while a read log is active; named from the
            // effects list where it still holds them.
            barriers: {
                let fx = system::external_effects();
                (0..rec.effects_len)
                    .map(|i| fx.get(i).map_or("external", |e| e.kind).to_string())
                    .collect()
            },
            written,
            dirs,
        })
    }
}

impl Session {
    // ---- persistence -----------------------------------------------------

    /// Write S₀ to `path`. Returns (bytes of the file, bytes allocated on
    /// disk).
    pub fn save_s0(&mut self, path: &str) -> Result<(u64, u64), String> {
        let s0 = self.s0.as_ref().ok_or("no S0 to save")?;
        let (id, key) = (s0.id, s0.key.clone());
        let g = self.g.as_mut().unwrap();
        write_s0(g, id, &key, path)
    }

    /// A session whose S₀ comes from `path` (written by `save_s0`, in this
    /// or another process). `Err` if the file does not fit this engine or
    /// S₀'s key no longer holds; the caller then starts with `cold`.
    pub fn open_s0(o: RunOptions, path: &str) -> Result<(Session, OpenReport), String> {
        let mut s: Option<Session> = None;
        let (g, s0, rep) = read_s0(path, &mut |key| {
            let t = Session::new(o.clone(), Some(key.clock));
            key.check(t.clock, &t.first_line)?;
            s = Some(t);
            Ok(())
        })?;
        let mut s = s.ok_or("S0 was not opened")?;
        s.g = Some(g);
        s.s0 = Some(s0);
        s.fresh = true;
        Ok((s, rep))
    }
}

/// Write S₀ (checkpoint `id` of `g`, with its key) to `path`. Returns (bytes
/// of the file, bytes allocated on disk).
pub fn write_s0(
    g: &mut Globals,
    id: CheckpointId,
    key: &Key,
    path: &str,
) -> Result<(u64, u64), String> {
    {
        let rec = g.record_of(id)?;
        // The output files' contents up to their length at S₀, and the
        // terminal's.
        let mut outputs: Vec<(String, Vec<u8>)> = vec![];
        for f in &rec.files {
            if let Stream::Out { path, len, .. } = &f.stream {
                let d = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
                let p = d
                    .get(..*len as usize)
                    .ok_or_else(|| format!("{path} is shorter than at S0"))?;
                outputs.push((path.clone(), p.to_vec()));
            }
        }
        let terminal = system::terminal_bytes();
        let terminal = terminal
            .get(..rec.terminal_len)
            .ok_or("the terminal is shorter than at S0")?
            .to_vec();
        let view = g.arena.view_at(id)?;
        // The display list's side table (changes/displaylist.ch) is left out,
        // as it always was: its entries name source spans of this process
        // (`crate::displaylist`), which a new process numbers afresh. Nodes
        // made before S₀ are restored without a source span.
        let side = g
            .arena
            .regions
            .iter()
            .find(|r| r.name == "dl_side")
            .map_or(0..0, |r| r.off..r.off + r.bytes);
        let chunk = |c: usize| -> std::borrow::Cow<'_, [u8]> {
            let (lo, hi) = (c * CHUNK_BYTES, (c + 1) * CHUNK_BYTES);
            let d = view.chunk(c);
            if hi <= side.start || lo >= side.end {
                return std::borrow::Cow::Borrowed(d);
            }
            let mut v = d.to_vec();
            v[side.start.max(lo) - lo..side.end.min(hi) - lo].fill(0);
            std::borrow::Cow::Owned(v)
        };
        let mut present: Vec<u32> = vec![];
        for c in 0..g.arena.chunks() {
            if g.arena.touched(c) && chunk(c).iter().any(|&b| b != 0) {
                present.push(c as u32);
            }
        }
        let mut head = vec![];
        MAGIC.to_vec().enc(&mut head);
        key.enc(&mut head);
        rec.enc(&mut head);
        outputs.enc(&mut head);
        terminal.enc(&mut head);
        // The diagnostics side channel's notes up to S₀ and the definition
        // sites (`crate::diag`), so that a reopened document reports what
        // a full run reports.
        let notes = crate::diag::notes();
        notes
            .get(..rec.notes)
            .unwrap_or(&notes[..])
            .iter()
            .map(|n| (**n).clone())
            .collect::<Vec<crate::diag::Note>>()
            .enc(&mut head);
        crate::diag::sites().enc(&mut head);
        // The files opened for output before S₀, by name: a process that
        // opens S₀ must know them (`system::rewritten_at`: a file the
        // preamble wrote and the body writes again, #1348).
        let opens = system::opens_since(0);
        opens
            .get(..rec.opens)
            .ok_or("the output opens are fewer than at S0")?
            .to_vec()
            .enc(&mut head);
        (g.arena.len_bytes() as u64).enc(&mut head);
        (g.arena.scalar_bytes() as u64).enc(&mut head);
        present.enc(&mut head);
        let data_off = (8 + head.len()).next_multiple_of(CHUNK_BYTES) as u64;
        use std::io::Write;
        let tmp = format!("{path}.tmp");
        let f = std::fs::File::create(&tmp).map_err(|e| format!("{tmp}: {e}"))?;
        let mut f = std::io::BufWriter::with_capacity(1 << 20, f);
        let pad = vec![0u8; data_off as usize - 8 - head.len()];
        f.write_all(&(head.len() as u64).to_le_bytes())
            .and_then(|_| f.write_all(&head))
            .and_then(|_| f.write_all(&pad))
            .map_err(|e| format!("{tmp}: {e}"))?;
        // The present chunks, densely, in index order.
        for &c in &present {
            f.write_all(&chunk(c as usize))
                .map_err(|e| format!("{tmp}: {e}"))?;
        }
        let f = f.into_inner().map_err(|e| format!("{tmp}: {e}"))?;
        f.sync_all().ok();
        drop(f);
        std::fs::rename(&tmp, path).map_err(|e| format!("{path}: {e}"))?;
        let m = std::fs::metadata(path).map_err(|e| format!("{path}: {e}"))?;
        #[cfg(unix)]
        let on_disk = std::os::unix::fs::MetadataExt::blocks(&m) * 512;
        #[cfg(not(unix))]
        let on_disk = m.len();
        Ok((m.len(), on_disk))
    }
}

/// Open the S₀ file at `path` into a new engine. `prepare` runs once the
/// header is read, with S₀'s key: it sets the process up for the job (the
/// first time, kpathsea's start-up) and checks the key; its time is the
/// report's `config_s` and `validate_s`.
pub fn read_s0(
    path: &str,
    prepare: &mut dyn FnMut(&Key) -> Result<(), String>,
) -> Result<(Box<Globals>, S0, OpenReport), String> {
    {
        let t0 = Instant::now();
        let map = MappedFile::open(path)?;
        let bytes = map.bytes();
        let hlen = u64::from_le_bytes(bytes.get(..8).ok_or("not an S0 file")?.try_into().unwrap())
            as usize;
        let mut r = Reader::new(bytes.get(8..8 + hlen).ok_or("S0 file truncated")?);
        if Vec::<u8>::dec(&mut r)? != MAGIC {
            return Err("not an S0 file".into());
        }
        let key = Key::dec(&mut r)?;
        if key.build != engine_build() {
            return Err("S0 was saved by another engine build".into());
        }
        let rec = ExtRecord::dec(&mut r)?;
        let outputs = Vec::<(String, Vec<u8>)>::dec(&mut r)?;
        let terminal = Vec::<u8>::dec(&mut r)?;
        let notes = Vec::<crate::diag::Note>::dec(&mut r)?;
        let sites = Vec::<(i32, crate::diag::Site)>::dec(&mut r)?;
        let opens = Vec::<String>::dec(&mut r)?;
        if opens.len() != rec.opens {
            return Err("S0 file: its output opens do not match its record".into());
        }
        let arena_len = u64::dec(&mut r)? as usize;
        let scalar_bytes = u64::dec(&mut r)? as usize;
        let present = Vec::<u32>::dec(&mut r)?;
        let data_off = (8 + hlen).next_multiple_of(CHUNK_BYTES);
        let t_header = t0.elapsed().as_secs_f64();

        let tc = Instant::now();
        prepare(&key)?;
        let config_s = tc.elapsed().as_secs_f64();
        let validate_s = 0.0;

        let t2 = Instant::now();
        crate::pdftex::reset_state();
        let mut g = Globals::new();
        if g.arena.len_bytes() != arena_len || g.arena.scalar_bytes() != scalar_bytes {
            return Err("S0 was saved with another word-space layout".into());
        }
        for (i, &c) in present.iter().enumerate() {
            let off = data_off + i * CHUNK_BYTES;
            let d = bytes
                .get(off..off + CHUNK_BYTES)
                .ok_or("S0 file truncated")?;
            g.arena.load_chunk(c as usize, d);
        }
        g.fill_scalars();
        let load_s = t2.elapsed().as_secs_f64();

        let t3 = Instant::now();
        for (p, d) in &outputs {
            std::fs::write(p, d).map_err(|e| format!("{p}: {e}"))?;
            system::stamp_output(p);
        }
        system::truncate_terminal(0);
        system::append_terminal(&terminal);
        crate::diag::reset();
        let notes: Vec<std::sync::Arc<crate::diag::Note>> =
            notes.into_iter().map(std::sync::Arc::new).collect();
        crate::diag::append(&notes, 0);
        crate::diag::set_sites(sites);
        system::truncate_external_effects(0);
        g.restore_ext(&rec)?;
        // The images S₀ holds: read again and compared with what it stored
        // (`images::State`'s codec), or S₀ is not used.
        g.verify_persisted_images()?;
        // (`restore_ext` left placeholders for the opens)
        system::truncate_opens(0);
        system::append_opens(&opens);
        let id = g.checkpoint()?;
        let ext_s = t3.elapsed().as_secs_f64();
        let rep = OpenReport {
            config_s,
            header_s: t_header,
            validate_s,
            load_s,
            ext_s,
            total_s: t0.elapsed().as_secs_f64(),
            chunks: present.len(),
        };
        Ok((g, S0 { id, key }, rep))
    }
}

const MAGIC: &[u8] = b"flashtex S0 v3";

/// How opening a persisted S₀ went.
#[derive(Clone, Debug, Default)]
pub struct OpenReport {
    /// `system::configure` (texmf.cnf, kpathsea): every process pays it.
    pub config_s: f64,
    pub header_s: f64,
    pub validate_s: f64,
    pub load_s: f64,
    pub ext_s: f64,
    pub total_s: f64,
    pub chunks: usize,
}

impl OpenReport {
    pub fn json(&self) -> String {
        format!(
            "{{\"config_s\":{:.6},\"header_s\":{:.6},\"validate_s\":{:.6},\"load_s\":{:.6},\"ext_s\":{:.6},\"total_s\":{:.6},\"chunks\":{}}}",
            self.config_s, self.header_s, self.validate_s, self.load_s, self.ext_s, self.total_s, self.chunks
        )
    }
}

use crate::os::MappedFile;

#[cfg(test)]
mod key_tests {
    use super::*;

    /// A directory opened so that its times can be set: a plain open does
    /// on Unix; Windows wants write access and FILE_FLAG_BACKUP_SEMANTICS
    /// (`CreateFileW` opens a directory only with it).
    fn open_dir_for_times(d: &std::path::Path) -> std::fs::File {
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
            std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                .open(d)
                .unwrap()
        }
        #[cfg(not(windows))]
        {
            std::fs::File::open(d).unwrap()
        }
    }

    /// After a check that verified racy signatures by content and by the
    /// lookups, the key keeps fresh ones: once the tick has passed, the
    /// steady state is not racy (else every compile compared the preamble's
    /// files and ran its lookups again; review of #1552).
    #[test]
    fn a_checked_key_keeps_fresh_signatures() {
        let d = std::env::temp_dir().join(format!("flashtex-key-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("pre.sty");
        std::fs::write(&f, "\\def\\x{1}").unwrap();
        let (fp, dp) = (
            f.to_str().unwrap().to_string(),
            d.to_str().unwrap().to_string(),
        );
        let (fs, ds) = (StatSig::of(&fp).unwrap(), StatSig::of(&dp).unwrap());
        assert!(fs.racy && ds.racy, "just written");
        let mut key = Key {
            build: engine_build(),
            clock: (0, 0),
            source_date_epoch: std::env::var("SOURCE_DATE_EPOCH").ok(),
            force_source_date: std::env::var("FORCE_SOURCE_DATE").ok(),
            first_line: b"main".to_vec(),
            job_name: "main".into(),
            files: vec![(
                fp.clone(),
                crate::persist::hash128(&std::fs::read(&f).unwrap()),
                fs,
            )],
            prefixes: vec![],
            lookups: vec![],
            barriers: vec![],
            written: vec![],
            dirs: vec![(dp.clone(), ds)],
        };
        // the tick passes (the files' times put back a minute)
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(old)
            .unwrap();
        open_dir_for_times(&d).set_modified(old).unwrap();
        key.check_refresh((0, 0), b"main").unwrap();
        assert!(!key.files[0].2.racy, "the file's signature is still racy");
        assert!(
            !key.dirs[0].1.racy,
            "the directory's signature is still racy"
        );
        // and the next check is by signature alone: a changed file still fails
        key.check_refresh((0, 0), b"main").unwrap();
        std::fs::write(&f, "\\def\\x{2}").unwrap();
        assert!(key.check((0, 0), b"main").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
