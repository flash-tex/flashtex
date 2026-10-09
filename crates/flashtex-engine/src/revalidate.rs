//! READ-REVALIDATE: a read whose result left no trace no longer holds the
//! restart point back (DESIGN.md §5.3; docs/design/engine-v2/REVALIDATE.md).
//!
//! A restart point must have consumed nothing that changed
//! (`incr::consumed_nothing_changed`). Some reads consume a whole file at
//! once and keep nothing of it: LaTeX's `\include` and `\includeonly` name
//! each chapter with expl3's `\file_full_name:n`, which takes
//! `\pdffilesize` of the chapter only to test that it is not blank and that
//! two names give the same size (#1724 made that size a read of the file).
//! Every checkpoint after such a read has "consumed" the chapter, so an edit
//! deep in the chapter restarted at the chapter's start.
//!
//! The window. `P0` is the restart point the ordinary test finds, `P1` the
//! old run's first page checkpoint after it (at most [`MAX_PAGES`] on), and
//! `P2` the newest checkpoint that passes the ordinary test once the reads
//! the old run made between `P0` and `P1` are left out of it
//! ([`window_journal`]), with every stream on a changed file at `P2` before
//! its change ([`streams_before_edits`]). The run restarts at `P0` as
//! before, and at page `P1` compares itself with the old run
//! ([`same_since`] and the convergence test's `cstate.same_as` and
//! `same_words`, the full state). Equal: what the reads in the window did
//! left nothing behind -- in the state, in the output files, the terminal,
//! the diagnostics or the read journal -- so the old run from `P1` on is the
//! new run's own until it reads something changed, and `P2` is a restart
//! point: the run is abandoned (`Globals::reattach_pending`) and restarts at
//! `P2`. Not equal: the run goes on from `P1` as the ordinary restart.
//!
//! The cost is the comparison (the convergence test's, 2-10 ms) and, when it
//! holds, the pages from `P0` to `P1` typeset twice; the window is at most
//! [`MAX_PAGES`] pages, and the restart at `P2` must save at least one page
//! more than that. `FLASHTEX_REVALIDATE=off` disables it.

use crate::arena::CheckpointId;
use crate::checkpoint::ExtRecord;
use crate::generated::Globals;
use crate::incr::Edit;
use crate::system::{self, ReadLog, Stream};
use std::collections::HashMap;
use std::sync::Arc;

/// The window `P0..P1` spans at most this many pages.
pub const MAX_PAGES: usize = 2;

/// Revalidation is on (`FLASHTEX_REVALIDATE=off`: not).
pub fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FLASHTEX_REVALIDATE").as_deref() != Ok("off"))
}

/// A run's revalidation: compare at the new run's page `page` with the old
/// run's checkpoint `p1` there; equal, restart at `p2`.
#[derive(Clone)]
pub struct Probe {
    /// The windows left out so far, this one last (journal indices), and
    /// the first lookup whose answer changed.
    pub windows: Vec<(usize, usize)>,
    pub bad_lookup: Option<usize>,
    pub page: usize,
    pub p1: CheckpointId,
    pub p2: CheckpointId,
    /// The restart point's record (`P0`).
    pub p0: ExtRecord,
    /// The old run's journal from `P0` to `P1`: files, lookups, outputs.
    pub old_files: Vec<system::FileRead>,
    pub old_lookups: Vec<system::Lookup>,
    pub old_outputs: Vec<String>,
    /// The files the old run opens for output after `P1`.
    pub later_outputs: Vec<String>,
    /// The changed files as they are now.
    pub now: HashMap<String, Arc<Vec<u8>>>,
}

/// `j` with the reads of each window `from..to` left out (their paths
/// blanked, so that the indices of the others hold).
pub fn window_journal(j: &ReadLog, windows: &[(usize, usize)]) -> ReadLog {
    let mut w = j.clone();
    for &(from, to) in windows {
        let to = to.min(w.files.len());
        for f in w.files[from.min(to)..to].iter_mut() {
            f.path = String::new();
        }
    }
    w
}

/// At most this many windows in a row in one compile: each restart at a
/// `P2` may meet the next read that holds it back (a focused chapter's
/// `\includeonly` lookup, then `\include`'s own), and revalidate again.
pub const MAX_CHAIN: usize = 4;

/// Every stream `rec` has open on a changed file is before its change (has
/// read nothing from the change on, `incr::read_through`).
pub fn streams_before_edits(
    rec: &ExtRecord,
    edits: &[Edit],
    now: impl Fn(&str) -> Option<Arc<Vec<u8>>>,
) -> bool {
    rec.files.iter().all(|f| match &f.stream {
        Stream::In { path, offset } => edits.iter().filter(|e| e.path == *path).all(|e| {
            let n = now(path);
            crate::incr::read_through(*offset, e, n.as_deref().map(|v| v.as_slice())) <= e.prefix
        }),
        _ => true,
    })
}

fn read_range(path: &str, from: u64, to: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek};
    let mut f = std::fs::File::open(path).ok()?;
    f.seek(std::io::SeekFrom::Start(from)).ok()?;
    let mut b = vec![0u8; to.saturating_sub(from) as usize];
    f.read_exact(&mut b).ok()?;
    Some(b)
}

/// Everything but the engine's state, at the new run's checkpoint `new`
/// against the old run's `old` (both at the same page, the old one from
/// the branch the restore at `P0` detached): the same input positions,
/// before every change; the output files and the terminal the same bytes
/// since `P0`; the same diagnostics count, external effects and read
/// journal since `P0` (the journal the restart at `P2` keeps is the old
/// run's). The state is the caller's (`cstate`, `same_words`).
pub fn same_since(
    g: &Globals,
    pr: &Probe,
    old: &ExtRecord,
    new: &ExtRecord,
    edits: &[Edit],
) -> Result<(), String> {
    if old.files.len() != new.files.len() {
        return Err("file globals differ".into());
    }
    for (a, b) in old.files.iter().zip(&new.files) {
        if (a.buf, &a.line, a.pos, a.have_line, a.at_eof, a.err)
            != (b.buf, &b.line, b.pos, b.have_line, b.at_eof, b.err)
        {
            return Err("an input file's lookahead differs".into());
        }
        match (&a.stream, &b.stream) {
            (Stream::In { path, offset: x }, Stream::In { path: p, offset: y }) => {
                if path != p || x != y {
                    return Err(format!("{p}: at {y}, the old run {path} at {x}"));
                }
            }
            (
                Stream::Out { path, len, at },
                Stream::Out {
                    path: p,
                    len: l,
                    at: t,
                },
            ) => {
                if path != p || len != l || at != t || at != len {
                    return Err(format!("{p}: {t} of {l}, the old run's {at} of {len}"));
                }
                if let Some(why) = system::outside_change(p) {
                    return Err(why);
                }
                let from = pr
                    .p0
                    .files
                    .iter()
                    .find_map(|f| match &f.stream {
                        Stream::Out { path: q, len, .. } if q == p => Some(*len),
                        _ => None,
                    })
                    .unwrap_or(0);
                if from < *l {
                    let mine = read_range(p, from, *l);
                    let theirs = g.pending_old_bytes(p, from, *l);
                    if mine.is_none() || mine != theirs {
                        return Err(format!("{p}: other bytes since the restart point"));
                    }
                }
            }
            (x, y) => {
                if x != y {
                    return Err("a file global's stream differs".into());
                }
            }
        }
    }
    // Every output file closed since `P0` (open there, or opened since,
    // and not open now): its bytes as the old run left them. On success the
    // abandoned run's files are put back as the old run left them
    // (`Globals::reattach_pending`): a file written and closed in the window
    // (`\immediate\write` of a size) must hold the same bytes in both.
    let open_now: Vec<String> = new
        .files
        .iter()
        .filter_map(|f| match &f.stream {
            Stream::Out { path, .. } => Some(system::out_key(path)),
            _ => None,
        })
        .collect();
    let mut closed: Vec<&str> = vec![];
    let opened_p0 = pr.p0.files.iter().filter_map(|f| match &f.stream {
        Stream::Out { path, .. } => Some(path.as_str()),
        _ => None,
    });
    for p in opened_p0.chain(pr.old_outputs.iter().map(|s| s.as_str())) {
        let k = system::out_key(p);
        if !open_now.contains(&k) && !closed.iter().any(|q| system::out_key(q) == k) {
            closed.push(p);
        }
    }
    for p in closed {
        let k = system::out_key(p);
        // (the old run's copy is of its last opening: one it opens again
        // later is not the one closed here)
        if pr.later_outputs.iter().any(|o| system::out_key(o) == k) {
            return Err(format!(
                "{p} is closed in the window and written again later"
            ));
        }
        let (base, theirs) = g
            .pending_old_tail(p)
            .ok_or_else(|| format!("{p}: the old run's bytes are not kept"))?;
        let mine = system::read_logical(p).map_err(|e| format!("{p}: {e}"))?;
        if mine.get(base as usize..) != Some(&theirs[..]) {
            return Err(format!("{p}: other bytes, closed since the restart point"));
        }
    }
    if !streams_before_edits(new, edits, |p| pr.now.get(p).cloned()) {
        return Err("a stream has read a change".into());
    }
    if old.effects_len != new.effects_len || old.tex_input_type != new.tex_input_type {
        return Err("an external effect since the restart point".into());
    }
    if old.terminal_len != new.terminal_len || old.notes != new.notes {
        return Err("the terminal or the diagnostics differ".into());
    }
    let t0 = pr.p0.terminal_len;
    if t0 < new.terminal_len
        && g.pending_old_terminal(t0, new.terminal_len)
            != Some(system::terminal_slice(t0, new.terminal_len))
    {
        return Err("the terminal differs since the restart point".into());
    }
    if old.reads != new.reads {
        return Err("another number of reads since the restart point".into());
    }
    let live = system::reads_so_far().ok_or("no read journal")?;
    let (f0, l0, o0) = pr.p0.reads;
    let key = |f: &system::FileRead| (f.path.clone(), f.closed_at, f.written_before, f.stamp);
    let same_files = live.files.get(f0..new.reads.0).is_some_and(|n| {
        n.len() == pr.old_files.len() && n.iter().zip(&pr.old_files).all(|(a, b)| key(a) == key(b))
    });
    if !same_files
        || live.lookups.get(l0..new.reads.1) != Some(&pr.old_lookups[..])
        || live.outputs.get(o0..new.reads.2) != Some(&pr.old_outputs[..])
    {
        return Err("other reads since the restart point".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_is_left_out_and_the_rest_keeps_its_place() {
        let mut j = ReadLog::default();
        for p in ["a", "b", "c", "d"] {
            j.files.push(system::FileRead {
                path: p.into(),
                hash: [0, 0],
                stat: Default::default(),
                content: None,
                closed_at: None,
                written_before: false,
                stamp: None,
            });
        }
        let w = window_journal(&j, &[(1, 2), (2, 3)]);
        let paths: Vec<&str> = w.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["a", "", "", "d"]);
        assert_eq!(window_journal(&j, &[(3, 9)]).files.len(), 4);
    }
}
