//! Convergence (DESIGN.md §5.3, docs/design/xetex/INCREMENTAL.md §5 item
//! 5): a run restarted from S₀ or a page checkpoint compares its state
//! after each page with the last complete run's after the same page; where
//! they are the same up to where dynamic memory put things, the rest of the
//! old run stands.
//!
//! Step 1, measurement only (`FLASHTEX_UNICODE_CONV_DEBUG=1`): the test
//! runs at each page checkpoint of a restarted run until it passes (where
//! a jump would be made), and the run goes on to its end as without it.
//! At the end the claim is checked against what the run made: every page
//! after the convergence point must be the old run's page (its display
//! list frame). Notes go to standard error, prefixed `[conv]`.
//!
//! The test, after the pdfTeX engine's (`crates/flashtex-engine/src/incr.rs`
//! `IncrRun::test` and `same_words`):
//! 1. the file globals: the same files at the same places (an edited
//!    file's offset after the edit, moved by it), the same external
//!    effects and run state;
//! 2. the reads: the old run reads no changed file from here on, makes no
//!    lookup whose answer changed, and reads no file either run writes (up
//!    to its last page checkpoint; `\end{document}`'s reads would run live);
//! 3. the state outside the word space ([`Globals::host_vs`]);
//! 4. the word space: the differing words, less dead ones ([`dead_word`])
//!    and positions in the XDV stream (which only feeds the display list,
//!    page by page, in PDF mode), must be free cells or lie in what the
//!    structural comparison (`crate::iso`) walks, and that comparison must
//!    find the two states alike.

use crate::ckpt::Record;
use crate::generated::Globals;
use crate::iso::{EqtbScan, HostSide, Iso};
use crate::statediff::{ScalarSlot, WordDiff};
use crate::system::Stream;
use flashtex_engine::arena::{Branch, CheckpointId, ChunkDiff, CHUNK_BYTES};
use flashtex_engine::system::ReadLog;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

/// An old page checkpoint after the restart point.
pub struct OldPage {
    pub pages: usize,
    /// How many output opens the old run had made (`PageCk::opens`).
    pub opens: usize,
    pub id: CheckpointId,
    pub rec: Record,
    /// Its output's state (the document then).
    pub out: Option<Arc<crate::ckpt::OutSnap>>,
    pub reads: (usize, usize, usize),
    /// READ-TRACK (`crate::dormant`): its record of the counts.
    pub dorm: flashtex_engine::dormant::Dorm,
}

/// An edit of a user's file: bytes `prefix..old_end` of the old text
/// became `prefix..old_end + delta` of the new.
#[derive(Clone, Debug)]
pub struct Edit {
    pub path: String,
    pub prefix: u64,
    pub old_end: u64,
    pub delta: i64,
}

impl Edit {
    /// The edit between `old` and `new`.
    pub fn between(path: &str, old: &[u8], new: &[u8]) -> Edit {
        let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let room = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(room)
            .take_while(|(a, b)| a == b)
            .count();
        Edit {
            path: path.to_string(),
            prefix: prefix as u64,
            old_end: (old.len() - suffix) as u64,
            delta: new.len() as i64 - old.len() as i64,
        }
    }
}

/// What the restarted run needs of the last complete one.
pub struct Old {
    /// (taken by the jump, which keeps the old run's checkpoints)
    pub branch: Option<Branch>,
    pub pages: Vec<OldPage>,
    /// The old run's pages as it built them (its newest page checkpoint's
    /// document), for the jump to build again ([`crate::out::replay_page`]).
    pub doc_pages: Vec<Arc<crate::out::doc::Built>>,
    /// That document's resource keys (`out::replay_page` sends a page's
    /// frame again where its resources are named alike).
    pub doc_keys: Option<crate::out::doc::DocKeys>,
    /// The old run's output files as it left them (those open at one of
    /// its page checkpoints), and its terminal.
    pub old_files: std::collections::HashMap<String, Arc<Vec<u8>>>,
    pub old_terminal: Arc<Vec<u8>>,
    /// Where the jump went: (the claim's page, the page it went on from).
    pub jumped: Option<(usize, usize)>,
    /// Why the claim's jump was not made, if it was not.
    pub refused: Option<String>,
    /// The read journal's lengths at the restore target (files, lookups,
    /// outputs).
    pub from_reads: (usize, usize, usize),
    /// The user's files' edits, and every file read since S₀ that changed
    /// (an edit's or another's).
    pub edits: Vec<Edit>,
    pub changed: HashSet<String>,
    /// The first lookup since S₀ whose answer changed (its index).
    pub lookup: Option<usize>,
    /// The old run's read journal.
    pub journal: Option<ReadLog>,
    /// Its source map's span and file tables.
    pub tables: crate::dlsource::Tables,
    /// The `eqtb` pointer entries of the restore target.
    pub eqtb_pointers: Arc<Vec<i32>>,
    /// Where the test passed: (pages shipped, index into `pages`).
    pub claim: Option<(usize, usize)>,
    /// Each test: pages shipped, the answer, milliseconds.
    pub tests: Vec<(usize, Result<usize, String>, f64)>,
    /// READ-TRACK (`crate::dormant`): the claim's test passed but for a
    /// dormant line; what the jump puts in, and where it goes on from.
    pub dormant: Option<crate::dormant::Jump>,
}

impl Old {
    /// Where a jump from the old page checkpoint `i` goes on from: the
    /// newest old page checkpoint with its output opens and external
    /// effects (both only grow). The old run's later reads, lookups and
    /// opens run live again, so the test reads only up to there.
    pub fn target(&self, i: usize) -> usize {
        let op = &self.pages[i];
        (i..self.pages.len())
            .rev()
            .find(|&j| {
                self.pages[j].opens == op.opens && self.pages[j].rec.effects == op.rec.effects
            })
            .unwrap_or(i)
    }

    /// Where the jump from old page checkpoint `i` goes on from: [`target`](Self::target),
    /// or before the old run reads a dormant line (`crate::dormant`).
    pub fn jump_target(&self, i: usize) -> usize {
        match &self.dormant {
            Some(d) => self.target(i).min(d.to),
            None => self.target(i),
        }
    }

    pub fn summary(&self) -> String {
        let ms: f64 = self.tests.iter().map(|t| t.2).sum();
        let fails: Vec<String> = self
            .tests
            .iter()
            .filter_map(|(p, r, ms)| r.as_ref().err().map(|e| format!("p{p} {ms:.1}ms: {e}")))
            .take(6)
            .collect();
        format!(
            "{} tests, {:.1} ms; claim {:?}; {}",
            self.tests.len(),
            ms,
            self.claim.map(|c| c.0),
            fails.join(" | ")
        )
    }
}

/// Whether the measurement's notes are printed, and its claims checked
/// at the end of the run (`FLASHTEX_UNICODE_CONV_DEBUG=1`).
pub fn on() -> bool {
    std::env::var_os("FLASHTEX_UNICODE_CONV_DEBUG").is_some_and(|v| v != "0")
}

/// Whether a restarted run tests for convergence and jumps
/// (`FLASHTEX_UNICODE_CONV=0` turns it off; `=measure` tests without
/// jumping).
pub fn enabled() -> bool {
    std::env::var("FLASHTEX_UNICODE_CONV").map_or(true, |v| v != "0")
}

fn jumps() -> bool {
    std::env::var("FLASHTEX_UNICODE_CONV").map_or(true, |v| v != "0" && v != "measure")
}

impl Globals {
    /// After the page checkpoint of page `pages`: the test against the old
    /// run's, until one passes.
    pub(crate) fn conv_test(&mut self, pages: usize) {
        let Some(c) = self.host.resident.as_mut().and_then(|r| r.conv.take()) else {
            return;
        };
        let mut c = c;
        // (a test costs a few milliseconds: none where a jump could save
        // less than two pages, and none after eight have failed)
        let worth = c.pages.last().is_some_and(|l| l.pages >= pages + 2) && c.tests.len() < 8;
        if c.claim.is_none() && worth {
            if let Some(i) = c.pages.iter().position(|p| p.pages == pages) {
                let t = Instant::now();
                let mut dormant = None;
                let r = self.conv_check(&c, i, &mut dormant);
                let ms = t.elapsed().as_secs_f64() * 1e3;
                if on() {
                    eprintln!("[conv] page {pages}: {r:?} ({ms:.2} ms)");
                    if let Some(d) = &dormant {
                        eprintln!(
                            "[conv]   a dormant line ({} words): kept to page {}",
                            d.words.len(),
                            c.pages[d.to].pages
                        );
                    }
                }
                let ok = r.is_ok();
                c.tests.push((pages, r, ms));
                if ok {
                    c.claim = Some((pages, i));
                    c.dormant = dormant;
                    if jumps() {
                        let t = Instant::now();
                        match self.conv_jump(&mut c, i) {
                            Ok(to) => {
                                c.jumped = Some((pages, to));
                                if on() {
                                    eprintln!(
                                        "[conv] jumped at page {pages} to page {to} ({:.2} ms)",
                                        t.elapsed().as_secs_f64() * 1e3
                                    );
                                }
                            }
                            Err(e) => {
                                if on() {
                                    eprintln!("[conv] no jump at page {pages}: {e}");
                                }
                                c.refused = Some(e);
                            }
                        }
                    }
                }
            }
        }
        if let Some(r) = self.host.resident.as_mut() {
            r.conv = Some(c);
        }
    }

    /// `FLASHTEX_UNICODE_CHECKMEM=1`: walk the live state from the roots
    /// (`Iso::walk_one`) and print the allocated cells nothing reaches: a
    /// root or node field the structural comparison misses (the pdfTeX
    /// engine's `checkmem`).
    pub(crate) fn check_mem(&mut self, page: usize) {
        if std::env::var_os("FLASHTEX_UNICODE_CHECKMEM").is_none() {
            return;
        }
        let layout = crate::statediff::scalar_layout(self);
        let g: &Globals = self;
        let w = |p: usize| g.mem[p].to_bits();
        let Some(free) = free_cells(&w, g.avail, g.rover, g.lo_mem_max, g.hi_mem_min, g.mem_end)
        else {
            eprintln!("[checkmem] page {page}: the free lists cannot be read");
            return;
        };
        if let Ok((nodes, missed, n_missed, n_leaked)) =
            Iso::classify_unreached(g, &layout, &free, g.hyph_list.len(), 12)
        {
            eprintln!(
                "[checkmem] page {page}: {n_missed} cells referenced but unreached {missed:?}, {n_leaked} leaked ({nodes} nodes)"
            );
        }
        match Iso::walk_one(g, &layout, &free, g.hyph_list.len(), 12) {
            Err(e) => eprintln!("[checkmem] page {page}: walk failed: {e}"),
            Ok((nodes, missed, count)) => {
                let cells: Vec<String> = missed
                    .iter()
                    .map(|&p| format!("{p}:{:#018x}", g.mem[p as usize].to_bits()))
                    .collect();
                eprintln!(
                    "[checkmem] page {page}: {count} allocated cells unreached ({nodes} nodes): {}",
                    cells.join(" ")
                );
                if count > 0 {
                    for &m in missed.iter().take(1) {
                        let who = crate::iso::who_points(g, &layout, &[m], 60);
                        eprintln!("[checkmem]   {m} pointed at from: {}", who.join(" "));
                    }
                }
            }
        }
    }

    /// The convergence jump (DESIGN.md §5.3): the live state is the old
    /// run's at its page checkpoint `c.pages[i]` (the test passed). Take the
    /// old run's state over and go on from its newest page checkpoint `L`
    /// with the same output files open and no external effect since (its
    /// later pages, and `\end{document}`, run live again):
    ///
    /// * the word space: the old run's state at `c.pages[i]` adopted whole
    ///   (the test allowed dead words, free cells and node addresses to
    ///   differ), the old run's checkpoints from there on kept
    ///   (`Arena::converge`), and `L` restored;
    /// * each output file open there: the new run's bytes so far, then the
    ///   old run's from its length at the convergence point; the terminal
    ///   likewise; the read journal likewise;
    /// * the source map: the old run's span and file numbering (the test
    ///   required this run's to be the same so far);
    /// * the document: the old run's pages after the convergence point
    ///   built again into this run's document from their XDV bytes
    ///   ([`crate::out::replay_page`]), and sent;
    /// * the old run's checkpoints after the convergence point up to `L`,
    ///   re-based (output lengths, edited files' offsets, the terminal, the
    ///   journal's counts, output opens; each with the document as
    ///   rebuilt), are this run's.
    ///
    /// `Err` before anything changed: no jump (the run goes on). A failure
    /// after the state began to change ends the run (a panic, which the
    /// resident engine reports as a failure and recovers from).
    fn conv_jump(&mut self, c: &mut Old, i: usize) -> Result<usize, String> {
        let op = &c.pages[i];
        // `L`: the newest old page checkpoint with the convergence point's
        // output opens and external effects (and, past a dormant line, before
        // the old run reads it: `crate::dormant`)
        let j = c.jump_target(i);
        if j == i {
            return Err("no later page checkpoint to go on from".into());
        }
        let l = &c.pages[j];
        let (k, lp) = (op.pages, l.pages);
        let old_built = c
            .doc_pages
            .get(k..lp)
            .ok_or("the old run's pages are not kept")?;
        if old_built.iter().any(|b| b.replay.is_none()) {
            return Err("an old page's XDV is not kept".into());
        }
        if !self.has_saved(l.id) {
            return Err("no host state saved with the old checkpoint".into());
        }
        if !c.branch.as_ref().is_some_and(|b| b.ids().contains(&op.id)) {
            return Err("the old checkpoint is not in the branch".into());
        }
        let r = self.host.resident.as_ref().ok_or("not resident")?;
        let new_ck = r.pages.last().ok_or("no page checkpoint")?;
        let (new_rec, new_reads, new_opens) = (new_ck.rec.clone(), new_ck.reads, new_ck.opens);
        let out = self.host.out.as_ref().ok_or("no output")?;
        if out.doc.pages.len() != k || new_ck.pages != k {
            return Err("the document's pages are not the run's".into());
        }
        let (pdf_name, pdf_open) = (out.pdf_name.clone(), out.pdf.is_some());
        // READ-TRACK-3: where this run's document at the convergence point
        // is the old run's there (`out::same_doc`), building the old pages
        // again would make the old run's later documents. Then those are
        // taken over (`out::adopt_doc`), with the pages' frames, and the
        // carried checkpoints keep the old run's documents.
        let take_docs: Option<Vec<Arc<crate::ckpt::OutSnap>>> = op
            .out
            .as_ref()
            .filter(|at_k| crate::out::same_doc(&out.doc, &at_k.doc))
            .and_then(|_| {
                c.pages[i + 1..=j]
                    .iter()
                    .map(|p| {
                        p.out
                            .clone()
                            .filter(|s| s.pdf_name == pdf_name && s.pdf_open == pdf_open)
                    })
                    .collect()
            });
        if self.host.dl.on() {
            let (_, old_dl) = self.host_vs(op.id)?;
            if !self.host.dl.numbered_like(&c.tables, old_dl.lens()) {
                return Err("the source map numbers spans or files otherwise".into());
            }
        }
        // the output files open at the convergence point: (path, new
        // length, old length), and their bytes after the jump
        let mut splice: Vec<(String, u64, u64)> = vec![];
        for (a, b) in op.rec.files.iter().zip(&new_rec.files) {
            if let (Stream::Out { path, len: ol, .. }, Stream::Out { len: nl, .. }) =
                (&a.stream, &b.stream)
            {
                if !splice.iter().any(|s| s.0 == *path) {
                    splice.push((path.clone(), *nl, *ol));
                }
            }
        }
        let mut contents = vec![];
        for (p, nl, ol) in &splice {
            let now = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
            let old = c
                .old_files
                .get(p)
                .ok_or_else(|| format!("{p}: the old run's bytes are not kept"))?;
            if (now.len() as u64) < *nl || (old.len() as u64) < *ol {
                return Err(format!("{p} is shorter than the runs wrote"));
            }
            let mut v = now[..*nl as usize].to_vec();
            v.extend_from_slice(&old[*ol as usize..]);
            contents.push((p.clone(), v));
        }
        let term_delta = new_rec.terminal_len as i64 - op.rec.terminal_len as i64;
        let reads_delta = (
            new_reads.0 as i64 - op.reads.0 as i64,
            new_reads.1 as i64 - op.reads.1 as i64,
            new_reads.2 as i64 - op.reads.2 as i64,
        );
        let opens_delta = new_opens as i64 - op.opens as i64;
        let remap = |rec: &Record| -> Result<Record, String> {
            let mut r = rec.clone();
            for f in r.files.iter_mut() {
                match &mut f.stream {
                    Stream::Out { path, len, prefix } => {
                        let Some(s) = splice.iter().find(|s| s.0 == *path) else {
                            return Err(format!("{path} was opened after the convergence point"));
                        };
                        if prefix.is_some() || *len < s.2 {
                            return Err(format!("{path}: not a page checkpoint's stream"));
                        }
                        *len = *len - s.2 + s.1;
                    }
                    Stream::In { path, offset } => {
                        if let Some(e) = c.edits.iter().find(|e| e.path == *path) {
                            if *offset < e.old_end {
                                return Err(format!("{path} is read before its edit later"));
                            }
                            *offset = (*offset as i64 + e.delta) as u64;
                        }
                    }
                    _ => {}
                }
            }
            r.terminal_len = (r.terminal_len as i64 + term_delta) as usize;
            Ok(r)
        };
        let shift = |n: usize, d: i64| (n as i64 + d) as usize;
        let mut carried = vec![];
        for p in &c.pages[i + 1..=j] {
            let rec = remap(&p.rec)?;
            carried.push(crate::ckpt::PageCk {
                id: p.id,
                pages: p.pages,
                rec,
                out: None,
                reads: (
                    shift(p.reads.0, reads_delta.0),
                    shift(p.reads.1, reads_delta.1),
                    shift(p.reads.2, reads_delta.2),
                ),
                opens: shift(p.opens, opens_delta),
                dorm: p.dorm,
            });
        }
        let op_dorm = op.dorm;
        let dormant_words: Vec<(usize, u64)> = c
            .dormant
            .as_ref()
            .map(|d| d.words.clone())
            .unwrap_or_default();
        let l_rec = carried
            .last()
            .map(|p| p.rec.clone())
            .ok_or("nothing carried")?;
        let mut term = self.term_out.captured_bytes().to_vec();
        if term.len() != new_rec.terminal_len
            || c.old_terminal.len() < op.rec.terminal_len
            || c.old_terminal.len() < l.rec.terminal_len
        {
            return Err("the terminal is not as the records say".into());
        }
        term.extend_from_slice(&c.old_terminal[op.rec.terminal_len..]);
        let jo = c.journal.as_ref().ok_or("no journal of the old run")?;
        if flashtex_engine::system::reads_len() != new_reads
            || jo.files.len() < l.reads.0
            || jo.lookups.len() < l.reads.1
            || jo.outputs.len() < l.reads.2
        {
            return Err("the read journals are not as the records say".into());
        }
        // The old run's state at the convergence point, whole (copied out
        // before the space changes).
        let branch = c.branch.as_ref().ok_or("no branch")?;
        let d = self.arena.diff_branch_all(branch, op.id)?;
        let adopt: Vec<(usize, Vec<u8>)> = d
            .differing
            .iter()
            .map(|&(ch, old, _)| {
                // SAFETY: `old` points at a whole chunk of the old run's,
                // unchanged until `d` is dropped (below, before any write).
                let b = unsafe { std::slice::from_raw_parts(old as *const u8, CHUNK_BYTES) };
                (ch as usize * CHUNK_BYTES, b.to_vec())
            })
            .collect();
        drop(d);
        let (op_id, l_id) = (op.id, l.id);
        let (op_reads, l_reads) = (op.reads, l.reads);
        let branch = c.branch.take().expect("checked");

        // ---- from here on the state changes: a failure ends the run ----
        let t_jump = Instant::now();
        let fail = |what: &str, e: String| -> ! { panic!("the convergence jump: {what}: {e}") };
        for (off, b) in &adopt {
            self.arena.write_through(*off, b);
        }
        self.arena.checkpoint();
        if let Err(e) = self.arena.converge(branch, op_id) {
            fail("converge", e);
        }
        // READ-TRACK (`crate::dormant`): the dormant line's new words in every
        // kept state from the convergence point on, before the restore of `L`
        if let Err(e) = self.arena.set_from(op_id, &dormant_words) {
            fail("the dormant line", e);
        }
        for (p, v) in contents {
            if let Err(e) = std::fs::write(&p, v) {
                fail(&p, e.to_string());
            }
        }
        self.host.dl.set_tables(c.tables.clone());
        if let Err(e) = self.restore(l_id) {
            fail("restore", e);
        }
        if let Err(e) = self.restore_record(&l_rec, Some(&term)) {
            fail("restore the files", e);
        }
        let mut jn = flashtex_engine::system::record_reads_into(None).unwrap_or_default();
        for f in &jo.files[op_reads.0..l_reads.0] {
            jn.files_mut().push(f.clone());
            jn.mark_seen(&f.path);
        }
        jn.lookups_mut()
            .extend_from_slice(&jo.lookups[op_reads.1..l_reads.1]);
        jn.outputs
            .extend_from_slice(&jo.outputs[op_reads.2..l_reads.2]);
        flashtex_engine::system::record_reads_into(Some(jn));
        // the old run's pages, taken over or built again into this run's
        // document
        let t_state = t_jump.elapsed();
        let (mut t_replay, mut t_snap) = (0.0, 0.0);
        let adopted = take_docs.is_some();
        if let Some(snaps) = take_docs {
            let t = Instant::now();
            let at_l = snaps.last().expect("a page carried");
            if let Err(e) = crate::out::adopt_doc(self, &at_l.doc, k) {
                fail("the old document", e);
            }
            for (ck, s) in carried.iter_mut().zip(snaps) {
                ck.out = Some(s);
            }
            t_replay = t.elapsed().as_secs_f64() * 1e3;
        }
        for (n, b) in old_built.to_vec().iter().enumerate() {
            if adopted {
                break;
            }
            let page_no = k + n + 1;
            let t = Instant::now();
            if let Err(e) = crate::out::replay_page(self, b, page_no as u32, c.doc_keys.as_ref()) {
                fail("replay", e);
            }
            t_replay += t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            if let Some(ck) = carried.iter_mut().find(|p| p.pages == page_no) {
                ck.out = self.host.out.as_ref().map(|o| {
                    Arc::new(crate::ckpt::OutSnap {
                        doc: o.doc.clone(),
                        pdf_name: pdf_name.clone(),
                        pdf_open,
                    })
                });
            }
            t_snap += t.elapsed().as_secs_f64() * 1e3;
        }
        if on() {
            eprintln!(
                "[conv]   jump: state {:.2} ms, {} pages {} {t_replay:.2} ms, their documents kept {t_snap:.2} ms",
                t_state.as_secs_f64() * 1e3,
                old_built.len(),
                if adopted { "taken over" } else { "built again" }
            );
        }
        if let Some(r) = self.host.resident.as_mut() {
            // (the new run's checkpoint at the convergence point is that
            // point too: the old run's counts there are the later records'
            // baseline, `crate::dormant`)
            if let Some(last) = r.pages.last_mut() {
                last.dorm = op_dorm;
            }
            r.pages.extend(carried);
        }
        Ok(lp)
    }

    fn conv_check(
        &mut self,
        c: &Old,
        i: usize,
        dormant: &mut Option<crate::dormant::Jump>,
    ) -> Result<usize, String> {
        let op = &c.pages[i];
        let new = &self
            .host
            .resident
            .as_ref()
            .and_then(|r| r.pages.last())
            .ok_or("no page checkpoint")?
            .rec;
        // READ-TRACK (`crate::dormant`): a dormant line may differ in its
        // unread rest (the edited file's reader keeps no copy of the line:
        // its record must be equal, as for any convergence)
        let level = if flashtex_engine::dormant::enabled() {
            crate::dormant::Level::of(self)
        } else {
            None
        };
        // 1. the file globals
        if op.rec.files.len() != new.files.len() {
            return Err("file globals differ".into());
        }
        for (a, b) in op.rec.files.iter().zip(&new.files) {
            if !a.alike_but_stream(b) {
                let mut x = format!("{a:?} vs {b:?}");
                x.truncate(400);
                return Err(format!("a file global differs: {x}"));
            }
            match (&a.stream, &b.stream) {
                (Stream::In { path, offset: x }, Stream::In { path: p, offset: y }) => {
                    if path != p {
                        return Err(format!("reading {p}, the old run {path}"));
                    }
                    match c.edits.iter().find(|e| e.path == *path) {
                        Some(e) => {
                            if *x < e.old_end || *x as i64 + e.delta != *y as i64 {
                                return Err(format!(
                                    "{p}: at {y}, the old run at {x} (edit {e:?})"
                                ));
                            }
                        }
                        None if c.changed.contains(path) => {
                            return Err(format!("reading the changed {p}"));
                        }
                        None => {
                            if x != y {
                                return Err(format!("{p}: at {y}, the old run at {x}"));
                            }
                        }
                    }
                }
                (Stream::Out { path, .. }, Stream::Out { path: p, .. }) => {
                    if path != p {
                        return Err(format!("writing {p}, the old run {path}"));
                    }
                }
                (Stream::Mem(_), Stream::Mem(_)) => {}
                (Stream::None, Stream::None) | (Stream::Terminal, Stream::Terminal) => {}
                _ => return Err("a file global's stream differs".into()),
            }
        }
        if op.rec.effects != new.effects {
            return Err("an external effect since the restart".into());
        }
        if op.rec.run != new.run {
            return Err("the run state differs".into());
        }
        let read = |e: &[(i32, (Option<String>, u32))]| -> Vec<(i32, (Option<String>, bool))> {
            e.iter()
                .map(|(h, s)| {
                    (
                        *h,
                        crate::native::layout::LayoutEngine::shaping_state_read(s),
                    )
                })
                .collect()
        };
        if read(&op.rec.engines) != read(&new.engines) {
            let (a, b) = (read(&op.rec.engines), read(&new.engines));
            let d: Vec<_> = a.iter().zip(&b).filter(|(x, y)| x != y).take(3).collect();
            return Err(format!(
                "a layout engine's shaping state differs ({} vs {}: {d:?})",
                a.len(),
                b.len()
            ));
        }
        // 2. the reads
        // (up to the jump's target, `Old::target`)
        let last = c.pages[c.target(i)].reads;
        if c.lookup.is_some_and(|l| l >= op.reads.1 && l < last.1) {
            return Err("the old run makes a lookup later whose answer changed".into());
        }
        // (only reads before the jump's target: the jump goes on from
        // there, and later reads run live again)
        let norm = |p: &str| p.strip_prefix("./").unwrap_or(p).to_string();
        if let Some(j) = &c.journal {
            let written: HashSet<String> = j.outputs.iter().map(|p| norm(p)).collect();
            let later = &j.files[op.reads.0.min(j.files.len())
                ..last.0.min(j.files.len()).max(op.reads.0.min(j.files.len()))];
            // A file the runs write that the old run reads later (a
            // `\tableofcontents` at the document's end reads the `.toc`
            // the last run's `\end{document}` wrote) is no barrier where
            // neither run opened it for output since the restore target,
            // and it holds now the bytes the old run read: then this run
            // reads it as the old run did (the pdfTeX engine's #1729 bound,
            // with its `written_same`).
            let mut opened: Option<HashSet<String>> = None;
            for f in later {
                if c.changed.contains(&f.path) {
                    return Err(format!("the old run reads the changed {} later", f.path));
                }
                let p = norm(&f.path);
                if written.contains(&p) {
                    let opened = opened.get_or_insert_with(|| {
                        let lo = c.from_reads.2.min(j.outputs.len());
                        let mut o: HashSet<String> = j.outputs
                            [lo..last.2.min(j.outputs.len()).max(lo)]
                            .iter()
                            .map(|p| norm(p))
                            .collect();
                        if let Some(n) = flashtex_engine::system::record_reads_into(None) {
                            let lo = c.from_reads.2.min(n.outputs.len());
                            o.extend(n.outputs[lo..].iter().map(|p| norm(p)));
                            flashtex_engine::system::record_reads_into(Some(n));
                        }
                        for r in [&op.rec, new] {
                            for s in &r.files {
                                if let Stream::Out { path, .. } = &s.stream {
                                    o.insert(norm(path));
                                }
                            }
                        }
                        o
                    });
                    let same = !opened.contains(&p)
                        && flashtex_engine::hashcache::hash_file(&f.path).ok() == Some(f.hash);
                    if !same {
                        return Err(format!(
                            "the old run reads {} later, which the runs write",
                            f.path
                        ));
                    }
                }
            }
        } else {
            return Err("no journal of the old run".into());
        }
        // 3. outside the word space, and 4. the word space
        let (old_handles, old_dl) = self.host_vs(op.id).map(|(h, d)| (h.clone(), d.clone()))?;
        let t0 = Instant::now();
        let d = self
            .arena
            .diff_branch(c.branch.as_ref().ok_or("no branch")?, op.id)?;
        let t_diff = t0.elapsed().as_secs_f64() * 1e3;
        let words = crate::statediff::words(self, &d);
        let n_words = words.len();
        let layout = crate::statediff::scalar_layout(self);
        let g: &Globals = self;
        let left: Vec<WordDiff> = words
            .into_iter()
            .filter(|w| !dead_word(g, w) && !position_only(w))
            .collect();
        // (the unread rest of a dormant line, set aside)
        let (aside, left): (Vec<WordDiff>, Vec<WordDiff>) = match level {
            Some(l) => left.into_iter().partition(|w| l.set_aside(g, &layout, w)),
            None => (vec![], left),
        };
        // (past a dormant line no state is equal, so an `eqtb` entry whose
        // type or level differs -- the structural comparison compares both
        // exactly -- ends the test before the walk)
        if !aside.is_empty() {
            if let Some(w) = left
                .iter()
                .find(|w| w.region == "eqtb" && (w.old ^ w.new) >> 32 != 0)
            {
                return Err(format!("a dormant line, and a meaning differs: {w}"));
            }
        }
        let (left, (free_o, free_n)) = drop_free_mem(g, &d, &layout, left);
        if let Some(w) = left.iter().find(|w| w.region != "mem" && !iso_covers(g, w)) {
            let n = left
                .iter()
                .filter(|w| w.region != "mem" && !iso_covers(g, w))
                .count();
            let some: Vec<String> = left
                .iter()
                .filter(|w| w.region != "mem" && !iso_covers(g, w))
                .take(6)
                .map(|w| w.to_string())
                .collect();
            let _ = w;
            return Err(format!(
                "{n} words differ outside what the structural comparison reads: {}",
                some.join(", ")
            ));
        }
        let bad_mem: Vec<usize> = left
            .iter()
            .filter(|w| w.region == "mem")
            .map(|w| w.index)
            .collect();
        if on() {
            eprintln!(
                "[conv]   left for the walk: {} words ({} set aside): {}",
                left.len(),
                aside.len(),
                crate::statediff::summary(&left)
            );
            for w in left.iter().take(12) {
                eprintln!("[conv]     {w}");
            }
        }
        let written = d.written();
        let mut written_bits = vec![0u64; g.arena.chunks().div_ceil(64) + 1];
        for &ch in &written {
            written_bits[ch as usize >> 6] |= 1 << (ch & 63);
        }
        let glyphs = |a: i32, b: i32| -> bool {
            match (old_handles.glyph_info(a), g.host.handles.glyph_info(b)) {
                (None, None) => a == 0 && b == 0,
                (Some(x), Some(y)) => Arc::ptr_eq(x, y) || **x == **y,
                _ => false,
            }
        };
        let side = |a: i32, b: i32| -> bool {
            let (x, y) = (old_dl.at(a), g.host.dl.at(b));
            x == y && x == 0 || c.tables.place(x) == g.host.dl.place(y)
        };
        let host = HostSide {
            glyphs: Some(&glyphs),
            side_eq: g.host.dl.on().then_some(&side as &dyn Fn(i32, i32) -> bool),
            eqtb_scan: Some(EqtbScan {
                written: &written,
                written_bits: &written_bits,
                pointers: &c.eqtb_pointers,
            }),
            dormant: level.map_or(-1, |l| l.j),
        };
        let mut never = || false;
        let t1 = Instant::now();
        // An equal state needs no walk (READ-TRACK-2). With no word left,
        // every word the walk reads is the same in both states, so it pairs
        // each node with itself, and all it could still find differs outside
        // the word space: a native word's glyphs (its handle's array) or a
        // node's source position (the side table's entry). Where both tables
        // agree entry for entry, it finds nothing; else it walks.
        let equal = left.is_empty()
            && match same_outside(g, c, &old_handles, &old_dl) {
                Ok(()) => true,
                Err(e) => {
                    if on() {
                        eprintln!("[conv]   no word left, but {e}: the walk");
                    }
                    false
                }
            };
        let r = if equal {
            Ok(0)
        } else {
            Iso::check(
                g,
                &d,
                &layout,
                free_o.as_deref(),
                free_n.as_deref(),
                &bad_mem,
                None,
                g.hyph_list.len(),
                host,
                &mut never,
            )
            .map_err(|e| format!("structures differ: {e}"))
        };
        if on() {
            eprintln!(
                "[conv]   diff {t_diff:.2} ms ({} chunks, {n_words} words), words {:.2} ms, {} {:.2} ms",
                d.differing.len(),
                (t1 - t0).as_secs_f64() * 1e3 - t_diff,
                if equal { "equal, no walk" } else { "walk" },
                t1.elapsed().as_secs_f64() * 1e3
            );
        }
        // READ-TRACK: equal but for a dormant line: the old run's pages are
        // the new run's up to its last page checkpoint before it reads the
        // line, prints one or opens a level (`dormant::target`)
        let r = match (r, level) {
            (Ok(n), Some(l)) if !aside.is_empty() => {
                let words = flashtex_engine::dormant::jump_words(
                    aside.iter().map(|w| (w.off, w.new)),
                    &self.arena,
                    std::mem::size_of_val(&self.buffer[0]),
                    l.loc,
                    self.first,
                );
                match crate::dormant::target(&c.pages, i, l.j, crate::dormant::grow(&aside)) {
                    _ if words.is_none() => Err("a dormant line: no buffer region".into()),
                    None => Err(format!(
                        "a dormant line ({} words): the old run uses it before its next page",
                        aside.len()
                    )),
                    // (the jump goes on live from the earlier of the two:
                    // worth it only where it keeps at least as many pages as
                    // it typesets again, `flashtex_engine::dormant::worth`)
                    Some(to)
                        if !flashtex_engine::dormant::worth(
                            c.pages[i].pages,
                            c.pages[c.target(i).min(to)].pages,
                            c.doc_pages.len(),
                        ) =>
                    {
                        Err(format!(
                            "a dormant line ({} words): the jump would typeset more pages \
                             than it keeps",
                            aside.len()
                        ))
                    }
                    Some(to) => {
                        *dormant = Some(crate::dormant::Jump {
                            words: words.unwrap_or_default(),
                            to,
                        });
                        Ok(n)
                    }
                }
            }
            (r, _) => r,
        };
        r
    }
}

/// What the structural comparison reads outside the word space, for a state
/// whose words are all equal (`Globals::conv_check`): every glyph-info
/// handle the same array in both runs' tables (`Handles::same_glyphs`), and,
/// with a source map, every side-table entry the same place
/// (`DlSnap::side_diffs`, with the live run numbering its spans and files as
/// the old run had at the checkpoint; an entry that differs is compared as
/// the walk compares it, by place in each run's tables). `Err`: what may
/// differ, for the walk to decide.
fn same_outside(
    g: &Globals,
    c: &Old,
    old_handles: &crate::state::Handles,
    old_dl: &crate::dlsource::DlSnap,
) -> Result<(), String> {
    if !old_handles.same_glyphs(&g.host.handles) {
        return Err("a glyph-info handle differs".into());
    }
    let dl = &g.host.dl;
    if !dl.on() {
        return Ok(());
    }
    if !dl.numbered_like(&c.tables, old_dl.lens()) {
        return Err("the source map numbers spans or files otherwise".into());
    }
    let diffs = old_dl
        .side_diffs(dl, 256)
        .ok_or("over 256 side-table entries differ")?;
    for p in diffs {
        let (x, y) = (old_dl.at(p as i32), dl.at(p as i32));
        if !(x == y && x == 0 || c.tables.place(x) == dl.place(y)) {
            return Err(format!(
                "the side table's entry {p} differs ({x:#x} vs {y:#x})"
            ));
        }
    }
    Ok(())
}

/// Arrays whose elements from a pointer on are dead between two commands
/// (the pdfTeX engine's `live_len`, with XeTeX's arrays).
fn live_len(g: &Globals, region: &str) -> Option<usize> {
    let n = match region {
        "str_pool" => g.pool_ptr,
        "buffer" => g.first,
        // (`str_start` is indexed from the first string after the 65536
        // characters)
        "str_start" => g.str_ptr - 65536 + 1,
        "save_stack" => g.save_ptr,
        "input_stack" => g.input_ptr,
        "nest" => g.nest_ptr,
        "param_stack" => g.param_ptr,
        "pstack" | "trick_buf" => 0,
        "active_width" | "cur_active_width" | "background" | "break_width" | "minimal_demerits"
        | "best_place" | "best_pl_line" | "disc_width" => 0,
        // (`hyf`: set by `hyphenate`, l. 22260, and by `\patterns`
        // and `\hyphenation` as they scan, before they read it)
        "hc" | "hu" | "hyf" => 0,
        // XeTeX: the text of the native word being collected
        "native_text" => g.native_len,
        // changes/ext.ch: `apply_mapping`'s result, read by its caller
        // right after it (xetex.web's `new_native_character` and the main
        // loop's `collected:`), in the same command
        "mapped_text" => 0,
        // the XDV stream: in PDF mode it only feeds the display list, page
        // by page (`out::page_done` takes each page's bytes), and
        // `xdv_buffer` is `ship_out`'s scratch
        "dvi_buf" | "xdv_buffer" => 0,
        _ => return None,
    };
    Some(n.max(0) as usize)
}

/// Whether a differing word is dead between two commands.
fn dead_word(g: &Globals, w: &WordDiff) -> bool {
    match w.scalar {
        // READ-TRACK's watch (`crate::dormant`, `changes/checkpoint.ch`): set
        // by the host at each page checkpoint and counted by `pop_input`,
        // read only by the host
        Some("dorm_level" | "pop_watch" | "dorm_fired") => return true,
        Some(
            "max_buf_stack"
            | "max_in_stack"
            | "max_nest_stack"
            | "max_param_stack"
            | "max_save_stack"
            | "best_height_plus_depth"
            | "long_state",
        ) => return true,
        // xetex.web: `line_break`'s copy of `prev_p` for `total_pw`, set by
        // `update_prev_p` as "Find optimal breakpoints" begins, before any
        // `try_break`; the main loop's list registers for native words, set
        // (`main_pp:=tail`, `main_ppp:=head`) before each use, in the
        // command that collects the word
        Some("global_prev_p" | "main_pp" | "main_ppp") => return true,
        // xetex.web (as tex.web): `line_break`'s globals, set as it starts
        // (`init_l_hyf`, `init_r_hyf`, `init_cur_lang` from `prev_graf`
        // and the language, l. 19750) or as each pass starts (`threshold`,
        // `second_pass`, `final_pass`, l. 20828; `l_hyf`, `r_hyf`,
        // l. 21462) before they are read; `disc_width`, set before each
        // `try_break` after a discretionary (l. 20977); `show_context`'s
        // `trick_count` and `first_count` (l. 8092)
        Some(
            "init_l_hyf" | "init_r_hyf" | "init_cur_lang" | "threshold" | "second_pass"
            | "final_pass" | "l_hyf" | "r_hyf" | "disc_width" | "trick_count" | "first_count",
        ) => return true,
        Some(_) => return false,
        None => {}
    }
    let Some(live) = live_len(g, w.region) else {
        return false;
    };
    let (r, rel) = g.arena.region_at(w.off);
    let elem = r.elem.max(1);
    let first = rel / elem;
    if first >= live {
        return true;
    }
    if elem >= 8 {
        return false;
    }
    let per = 8 / elem;
    let mask_bits = (live - first).min(per) * elem * 8;
    let mask = if mask_bits >= 64 {
        u64::MAX
    } else {
        (1u64 << mask_bits) - 1
    };
    (w.old ^ w.new) & mask == 0
}

/// Positions in the XDV stream (PDF mode: the display list takes each page
/// from `last_bop` to `dvi_offset + dvi_ptr` as it ships; no byte count is
/// printed), and the page-local state of `ship_out` (set as each page
/// starts).
fn position_only(w: &WordDiff) -> bool {
    matches!(
        w.scalar,
        Some(
            "dvi_ptr"
                | "dvi_offset"
                | "dvi_gone"
                | "last_bop"
                | "dvi_limit"
                | "half_buf"
                | "max_h"
                | "max_v"
                | "max_push"
                | "dvi_h"
                | "dvi_v"
                | "dvi_f"
        )
    )
}

/// The structural comparison's share (`crate::iso`).
fn iso_covers(g: &Globals, w: &WordDiff) -> bool {
    match w.scalar {
        Some(n) => crate::iso::scalar_covered(n),
        None => {
            matches!(
                w.region,
                "save_stack"
                    | "nest"
                    | "input_stack"
                    | "param_stack"
                    | "cur_mark"
                    | "font_glue"
                    | "hyph_list"
                    | "disc_ptr"
                    | "sa_root"
                    | "if_stack"
            ) || (w.region == "eqtb" && {
                let p = w.index as i32 + 1;
                let high = crate::iso::EQTB_SIZE + 1..=crate::iso::EQTB_SIZE + g.hash_high;
                p < crate::iso::INT_BASE || high.contains(&p)
            })
        }
    }
}

type FreeSets = (Option<Vec<u64>>, Option<Vec<u64>>);

/// The free cells of a state (the pdfTeX engine's `free_cells`).
fn free_cells(
    word: &dyn Fn(usize) -> u64,
    avail: i32,
    rover: i32,
    lo_mem_max: i32,
    hi_mem_min: i32,
    mem_end: i32,
) -> Option<Vec<u64>> {
    const EMPTY_FLAG: u32 = 0x3FFF_FFFF;
    let n = crate::generated::consts::mem_max as usize + 1;
    let mut bits = vec![0u64; n.div_ceil(64)];
    let set = |p: usize, bits: &mut Vec<u64>| bits[p >> 6] |= 1 << (p & 63);
    let rh = |w: u64| (w & 0xFFFF_FFFF) as u32 as i32;
    let lh = |w: u64| (w >> 32) as u32 as i32;
    let in_range = |p: i32| p > 0 && (p as usize) < n;
    let mut p = avail;
    let mut steps = 0usize;
    while p != -0x0FFF_FFFF {
        if !in_range(p) || p < hi_mem_min || p > mem_end || steps > n {
            return None;
        }
        set(p as usize, &mut bits);
        p = rh(word(p as usize));
        steps += 1;
    }
    if rover != 0 {
        let mut p = rover;
        let mut steps = 0usize;
        loop {
            if !in_range(p) || p >= lo_mem_max || steps > n {
                return None;
            }
            let w = word(p as usize);
            if rh(w) as u32 != EMPTY_FLAG {
                return None;
            }
            let size = lh(w);
            if size <= 0 || p + size > lo_mem_max {
                return None;
            }
            set_range(&mut bits, p as usize, (p + size) as usize);
            p = rh(word(p as usize + 1));
            steps += 1;
            if p == rover {
                break;
            }
        }
    }
    set_range(
        &mut bits,
        (lo_mem_max + 1).max(0) as usize,
        hi_mem_min.clamp(0, n as i32) as usize,
    );
    set_range(&mut bits, (mem_end + 1).max(0) as usize, n);
    Some(bits)
}

fn set_range(bits: &mut [u64], lo: usize, hi: usize) {
    let mut i = lo;
    while i < hi {
        if i & 63 == 0 && i + 64 <= hi {
            bits[i >> 6] = u64::MAX;
            i += 64;
        } else {
            bits[i >> 6] |= 1 << (i & 63);
            i += 1;
        }
    }
}

/// Drop the differing `mem` words free in both states, and `avail` and
/// `rover` (the pdfTeX engine's `drop_free_mem`).
fn drop_free_mem(
    g: &Globals,
    d: &ChunkDiff,
    layout: &[ScalarSlot],
    left: Vec<WordDiff>,
) -> (Vec<WordDiff>, FreeSets) {
    if !left
        .iter()
        .any(|w| w.region == "mem" || matches!(w.scalar, Some("avail" | "rover")))
    {
        return (left, (None, None));
    }
    let Some(mem_off) = g
        .arena
        .regions
        .iter()
        .find(|r| r.name == "mem")
        .map(|r| r.off)
    else {
        return (left, (None, None));
    };
    let scalar = |name: &str| -> Option<i32> {
        let s = layout.iter().find(|s| s.name == name)?;
        let w = d.old_word(&g.arena, s.off & !7);
        Some(((w >> ((s.off & 7) * 8)) & 0xFFFF_FFFF) as u32 as i32)
    };
    let (Some(av), Some(ro), Some(lo), Some(hi), Some(me)) = (
        scalar("avail"),
        scalar("rover"),
        scalar("lo_mem_max"),
        scalar("hi_mem_min"),
        scalar("mem_end"),
    ) else {
        return (left, (None, None));
    };
    let ow = d.old_words(&g.arena);
    let old_word = |p: usize| ow.word(mem_off + p * 8);
    let new_word = |p: usize| g.mem[p].to_bits();
    let Some(fo) = free_cells(&old_word, av, ro, lo, hi, me) else {
        return (left, (None, None));
    };
    let Some(fnw) = free_cells(
        &new_word,
        g.avail,
        g.rover,
        g.lo_mem_max,
        g.hi_mem_min,
        g.mem_end,
    ) else {
        return (left, (Some(fo), None));
    };
    let free_both = |p: usize| (fo[p >> 6] & fnw[p >> 6]) >> (p & 63) & 1 == 1;
    let left = left
        .into_iter()
        .filter(|w| {
            if matches!(w.scalar, Some("avail" | "rover")) {
                return false;
            }
            !(w.region == "mem" && free_both(w.index))
        })
        .collect();
    (left, (Some(fo), Some(fnw)))
}

/// At the end of a measured run: whether the claim holds. `new` are the
/// run's pages, `old` the last complete run's.
pub fn truth(
    c: &Old,
    new: &[Arc<crate::out::doc::Built>],
    old: &[Arc<crate::out::doc::Built>],
) -> String {
    let frame = |b: &crate::out::doc::Built| b.frame.get().cloned();
    // the first page from which on every page is the old run's
    let mut from = new.len();
    if new.len() == old.len() {
        while from > 0
            && frame(&new[from - 1]).is_some()
            && frame(&new[from - 1]) == frame(&old[from - 1])
        {
            from -= 1;
        }
    }
    if new.iter().chain(old).any(|b| b.frame.get().is_none()) {
        return format!(
            "claim {:?}, unchecked (no display list frames)",
            c.claim.map(|c| c.0)
        );
    }
    let verdict = match c.claim {
        None => "no claim".to_string(),
        Some((k, i)) => {
            let lp = c.pages[c.jump_target(i)].pages;
            let held = new.len() == old.len()
                && (k..lp.min(new.len())).all(|m| frame(&new[m]) == frame(&old[m]));
            if from > k && c.jumped.is_none() && c.refused.is_some() {
                // (a refused jump's claim: the old run's pages after it can
                // differ where the jump checks what the test does not, as
                // the source map's numbering)
                format!(
                    "claim at page {k}, jump refused ({}); the pages from {from} are not the old run's",
                    c.refused.as_deref().unwrap_or("")
                )
            } else if new.len() != old.len() {
                format!(
                    "UNSOUND: claim at page {k}, but {} pages vs the old run's {}",
                    new.len(),
                    old.len()
                )
            } else if held {
                // (pages after the jump's target are typeset live again)
                format!("claim at page {k} holds (to page {lp})")
            } else if from <= k {
                format!("claim at page {k} holds")
            } else {
                let (a, b) = (frame(&new[from - 1]), frame(&old[from - 1]));
                let at = match (&a, &b) {
                    (Some(a), Some(b)) => {
                        let i = a.iter().zip(b).take_while(|(x, y)| x == y).count();
                        format!(
                            "{} vs {} bytes, first difference at {i}: {:?} vs {:?}",
                            a.len(),
                            b.len(),
                            String::from_utf8_lossy(
                                &a[i.saturating_sub(40)..(i + 40).min(a.len())]
                            ),
                            String::from_utf8_lossy(
                                &b[i.saturating_sub(40)..(i + 40).min(b.len())]
                            )
                        )
                    }
                    _ => "no frame".into(),
                };
                format!("UNSOUND: claim at page {k}, but page {from} differs ({at})")
            }
        }
    };
    format!(
        "{verdict}; the old run's pages stand from page {} of {}",
        from + 1,
        new.len()
    )
}
