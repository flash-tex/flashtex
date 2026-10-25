//! PROBE (lane COLD-OPEN, 2026-10-24; scratch branch, never merged):
//! "convergence modulo the L5 patch". With `FLASHTEX_MODPATCH_PROBE=N` set,
//! every N-th page checkpoint of an incremental run is compared with the old
//! run's checkpoint after the same page (aligned by input position), with
//! the patched names' `eqtb`/`hash` entries checked against their patch
//! meanings and their token lists left out, and everything else compared as
//! the convergence test compares it. One `[modpatch]` JSON line per page on
//! stderr. Unset, nothing here runs.
//!
//! Two comparisons per page:
//! * strict: modulo the run's L5 patch (or, without one, the meanings that
//!   differ at the first aligned page), `CState::same_as` as the test has it;
//! * relaxed (an upper bound): modulo the extended patch (the L5 patch and
//!   every other meaning that differs at the first aligned page), the image
//!   table compared by its encoding, and string numbers compared through the
//!   renumbering that inserting the patch's new names' strings makes.

use super::*;
use crate::readset::{Meaning, Name, Patch, View};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// The probe's stride (0: off).
pub(super) fn stride() -> usize {
    static S: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        std::env::var("FLASHTEX_MODPATCH_PROBE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    })
}

/// The stride of a run without an L5 patch (`FLASHTEX_MODPATCH_STRIDE_NOL5`).
fn stride_nol5() -> usize {
    static S: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        std::env::var("FLASHTEX_MODPATCH_STRIDE_NOL5")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(stride)
            .max(1)
    })
}

pub(super) fn on() -> bool {
    stride() > 0
}

static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub(super) struct Probe {
    run: usize,
    /// The L5 patch of this run (`None`: computed at the first aligned page).
    patch: Option<Patch>,
    /// The patch and the meanings that differ besides at the first aligned page.
    patch_ext: Option<Patch>,
    from_l5: bool,
    /// input key -> old page indices
    index: Option<HashMap<u64, Vec<usize>>>,
    last_off: i64,
}

impl Probe {
    pub(super) fn new(patch: Option<Patch>) -> Probe {
        let run = RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        Probe {
            run,
            from_l5: patch.as_ref().is_some_and(|p| !p.is_empty()),
            patch: patch.filter(|p| !p.is_empty()),
            patch_ext: None,
            index: None,
            last_off: 0,
        }
    }
}

fn input_key(r: &ExtRecord) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for f in &r.files {
        (f.buf, &f.line, f.pos, f.have_line, f.at_eof, f.err).hash(&mut h);
        match &f.stream {
            Stream::In { path, offset } => (1u8, path, offset).hash(&mut h),
            Stream::Out { path, .. } => (2u8, path).hash(&mut h),
            Stream::None => 3u8.hash(&mut h),
            Stream::Stdin => 4u8.hash(&mut h),
            _ => 5u8.hash(&mut h),
        }
    }
    r.files.len().hash(&mut h);
    h.finish()
}

fn js(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn jl(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| js(s)).collect::<Vec<_>>().join(",")
    )
}

#[derive(Default)]
struct Outcome {
    input: bool,
    effects: bool,
    cparts: Vec<&'static str>,
    unexpl: Vec<String>,
    unexpl_n: usize,
    delta_err: Option<String>,
    raw_meanings: usize,
    uncov: Vec<String>,
    uncov_n: usize,
    uncov_sum: String,
    iso: Option<Result<usize, String>>,
    char_or: usize,
    differing_chunks: usize,
    p_mismatch: Vec<String>,
    strs: String,
}

impl Outcome {
    fn cstate_ok(&self, relax: bool) -> bool {
        self.cparts.iter().all(|p| relax && *p == "img~")
    }

    fn eq(&self, relax: bool) -> bool {
        self.input
            && self.effects
            && self.cstate_ok(relax)
            && self.delta_err.is_none()
            && self.unexpl_n == 0
            && self.uncov_n == 0
            && matches!(self.iso, Some(Ok(_)) | None)
    }

    fn json(&self, relax: bool) -> String {
        format!(
            "{{\"eq\":{},\"cparts\":{},\"strs\":{},\"unexpl_n\":{},\"unexpl\":{},\"p_mismatch\":{},\"delta_err\":{},\"uncov_n\":{},\"uncov_sum\":{},\"uncov\":{},\"iso\":{}}}",
            self.eq(relax),
            jl(&self.cparts.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
            js(&self.strs),
            self.unexpl_n,
            jl(&self.unexpl),
            jl(&self.p_mismatch),
            self.delta_err.as_deref().map_or("null".into(), js),
            self.uncov_n,
            js(&self.uncov_sum),
            jl(&self.uncov),
            match &self.iso {
                None => "null".into(),
                Some(Ok(n)) => format!("{{\"ok\":{n}}}"),
                Some(Err(e)) => format!(
                    "{{\"err\":{}}}",
                    js(&e.chars().take(400).collect::<String>())
                ),
            },
        )
    }
}

impl Obs {
    /// At page `j`'s checkpoint (record `rec`): the probe's comparison.
    pub(super) fn modpatch_page(&mut self, g: &mut Globals, rec: &ExtRecord, j: usize) {
        let Some(mut mp) = self.mp.take() else { return };
        let k = if mp.from_l5 { stride() } else { stride_nol5() };
        if (j - self.base) % k == 0 || j == self.base + 1 {
            self.modpatch_page_with(g, rec, j, &mut mp);
        }
        self.mp = Some(mp);
    }

    fn modpatch_page_with(&mut self, g: &mut Globals, rec: &ExtRecord, j: usize, mp: &mut Probe) {
        let t = Instant::now();
        let i0 = crate::os::thread_counts();
        let pending: HashSet<CheckpointId> = g.pending_ids().into_iter().collect();
        if mp.index.is_none() {
            let mut m: HashMap<u64, Vec<usize>> = HashMap::new();
            for (i, p) in self.old_pages.iter().enumerate() {
                if let Some(c) = p.ckpt.filter(|c| pending.contains(c)) {
                    if let Some(r) = g.pending_record(c) {
                        m.entry(input_key(&r)).or_default().push(i);
                    }
                }
            }
            eprintln!(
                "[modpatch] {{\"run\":{},\"start\":true,\"base\":{},\"old_pages\":{},\"old_kept\":{},\"patch\":{},\"from_l5\":{},\"patch_names\":{}}}",
                mp.run,
                self.base,
                self.old_pages.len(),
                m.values().map(|v| v.len()).sum::<usize>(),
                mp.patch.as_ref().map_or(0, |p| p.defs.len()),
                mp.from_l5,
                jl(&mp
                    .patch
                    .as_ref()
                    .map(|p| p
                        .defs
                        .iter()
                        .take(12)
                        .map(|(n, _)| n.to_string())
                        .collect::<Vec<String>>())
                    .unwrap_or_default())
            );
            mp.index = Some(m);
        }
        let same = j - self.base - 1;
        let key = input_key(rec);
        let cands = mp
            .index
            .as_ref()
            .unwrap()
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let same_kept = self
            .old_pages
            .get(same)
            .and_then(|p| p.ckpt)
            .is_some_and(|c| pending.contains(&c));
        let pick = if cands.contains(&same) {
            Some(same)
        } else {
            let want = same as i64 + mp.last_off;
            cands
                .iter()
                .copied()
                .min_by_key(|&i| (i as i64 - want).abs())
        };
        let mut line = format!(
            "{{\"run\":{},\"page\":{},\"same_kept\":{}",
            mp.run, j, same_kept
        );
        let Some(idx) = pick else {
            line.push_str(&format!(
                ",\"aligned\":false,\"eq\":false,\"r_eq\":false,\"ms\":{:.2}}}",
                t.elapsed().as_secs_f64() * 1e3
            ));
            eprintln!("[modpatch] {line}");
            return;
        };
        mp.last_off = idx as i64 - same as i64;
        let old = self.old_pages[idx].ckpt.unwrap();
        let first = mp.patch_ext.is_none();
        let d = match g.diff_pending(old) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[modpatch] {line},\"diff_err\":{}}}", js(&e));
                return;
            }
        };
        // the patch (none for a run without an L5 patch: the meanings that
        // differ at the first aligned page), and the extended patch: the
        // patch and every other meaning that differs at the first aligned
        // page (names the `\document` hooks derive from the `.aux`'s)
        if first {
            let base = mp.patch.clone().unwrap_or_default();
            let extra = delta(g, &d, &base).map(|(p, _)| p).unwrap_or_default();
            eprintln!(
                "[modpatch] {{\"run\":{},\"patch_ext_extra\":{},\"names\":{}}}",
                mp.run,
                extra.defs.len(),
                jl(&extra
                    .defs
                    .iter()
                    .take(40)
                    .map(|(n, m)| describe(n, m))
                    .collect::<Vec<_>>())
            );
            mp.patch_ext = Some(base.then(&extra));
            if mp.patch.is_none() {
                mp.patch = mp.patch_ext.clone();
            }
        }
        let p_strict = mp.patch.clone().unwrap_or_default();
        let p_ext = mp.patch_ext.clone().unwrap_or_default();
        let strict = self.modpatch_compare(g, rec, old, &d, &p_strict, false);
        let relaxed = self.modpatch_compare(g, rec, old, &d, &p_ext, true);
        drop(d);
        // the old run's reads of patched names in its next page interval
        let (reads_n, reads) = match g.pending_record(old) {
            Some(o) => {
                let keys: HashMap<u64, String> = p_ext
                    .defs
                    .iter()
                    .map(|(n, _)| (n.key(), n.to_string()))
                    .collect();
                let next = self
                    .old_pages
                    .get(idx + 1)
                    .and_then(|q| q.ckpt)
                    .and_then(|c| g.pending_record(c))
                    .map(|r| r.rs);
                match g.pending_rs() {
                    Some(rs) if !rs.is_empty() => {
                        let hi = next.unwrap_or(rs.len()).min(rs.len());
                        let lo = o.rs.min(hi);
                        let mut names: Vec<String> = vec![];
                        let mut seen = HashSet::new();
                        for e in &rs.events[lo..hi] {
                            if let Some(n) = keys.get(&e.name) {
                                if seen.insert(e.name) {
                                    names.push(n.clone());
                                }
                            }
                        }
                        (names.len() as i64, names)
                    }
                    _ => (-1, vec![]),
                }
            }
            None => (-1, vec![]),
        };
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let minstr = i0
            .zip(crate::os::thread_counts())
            .map_or(0.0, |(a, b)| (b.0 - a.0) as f64 / 1e6);
        line.push_str(&format!(
            ",\"aligned\":true,\"old_page\":{},\"off\":{},\"eq\":{},\"r_eq\":{},\"input\":{},\"effects\":{},\"chunks\":{},\"raw_meanings\":{},\"char_or\":{},\"first_patch\":{},\"strict\":{},\"relaxed\":{},\"reads_n\":{},\"reads\":{},\"ms\":{:.2},\"minstr\":{:.1}}}",
            self.base + idx + 1,
            idx as i64 - same as i64,
            strict.eq(false),
            relaxed.eq(true),
            strict.input,
            strict.effects,
            strict.differing_chunks,
            strict.raw_meanings,
            strict.char_or,
            first,
            strict.json(false),
            relaxed.json(true),
            reads_n,
            jl(&reads.into_iter().take(8).collect::<Vec<_>>()),
            ms,
            minstr
        ));
        eprintln!("[modpatch] {line}");
    }

    /// The comparison modulo `patch`; `relax`: the image table by its
    /// encoding, and string numbers through a pure insertion of the
    /// patch's new names' strings.
    fn modpatch_compare(
        &mut self,
        g: &mut Globals,
        rec: &ExtRecord,
        old: CheckpointId,
        d: &crate::arena::ChunkDiff,
        patch: &Patch,
        relax: bool,
    ) -> Outcome {
        let mut out = Outcome::default();
        let Some(o) = g.pending_record(old) else {
            out.delta_err = Some("no old record".into());
            return out;
        };
        out.input = input_key(&o) == input_key(rec);
        out.effects = o.effects_len == rec.effects_len && o.tex_input_type == rec.tex_input_type;
        out.cparts = rec.cstate.probe_parts(&o.cstate);
        out.differing_chunks = d.differing.len();
        if !relax {
            if let Ok((raw, _)) = delta(g, d, &Patch::default()) {
                out.raw_meanings = raw.defs.len();
            }
        }
        match delta(g, d, patch) {
            Ok((un, _)) => {
                out.unexpl_n = un.defs.len();
                let pk = patch.keys();
                for (n, _) in &un.defs {
                    if pk.contains(&n.key()) {
                        out.p_mismatch.push(n.to_string());
                    }
                }
                out.p_mismatch.truncate(8);
                out.unexpl = un
                    .defs
                    .iter()
                    .take(10)
                    .map(|(n, m)| describe(n, m))
                    .collect();
            }
            Err(e) => out.delta_err = Some(format!("aux_delta: {e}")),
        }
        // the slots of the patched names, in either state; the string
        // renumbering
        let mut skip: HashSet<i32> = HashSet::new();
        let mut smap: Option<std::sync::Arc<Vec<i32>>> = None;
        {
            let ov = View::old(g, d);
            let nv = View::live(g);
            if let (Ok(ov), Ok(nv)) = (ov, nv) {
                for (n, _) in &patch.defs {
                    skip.extend(ov.lookup(n));
                    skip.extend(nv.lookup(n));
                }
                if relax {
                    let ins: HashSet<Vec<u8>> = patch
                        .defs
                        .iter()
                        .filter_map(|(n, _)| match n {
                            Name::Multi(b) if ov.lookup(n).is_none() => Some(b.clone()),
                            _ => None,
                        })
                        .collect();
                    match str_map(&ov, &nv, &ins) {
                        Ok(None) => out.strs = "identity".into(),
                        Ok(Some((m, n))) => {
                            out.strs = format!("insert {n}");
                            smap = Some(std::sync::Arc::new(m));
                        }
                        Err(e) => out.strs = format!("err: {e}"),
                    }
                }
            }
        }
        let strs_ok = smap.is_some();
        let side_written = g.pending_side_written(old).unwrap_or_default();
        let side_any = side_written.iter().any(|&w| w != 0);
        let words = crate::statediff::words(g, d);
        let layout = crate::statediff::scalar_layout(g);
        let g: &Globals = g;
        let mut char_or = 0;
        let words: Vec<_> = words
            .into_iter()
            .filter(|w| {
                if w.region == "pdf_char_used" && w.new & w.old == w.old {
                    char_or += 1;
                    false
                } else {
                    true
                }
            })
            .collect();
        out.char_or = char_or;
        let hash_base = crate::readset::HASH_BASE;
        let map = |x: i32| -> i32 {
            match &smap {
                Some(m) if x >= 0 && (x as usize) < m.len() => m[x as usize],
                _ => x,
            }
        };
        let halves = |v: u64| (v as u32 as i32, (v >> 32) as u32 as i32);
        let renumbered = |w: &crate::statediff::WordDiff| -> bool {
            if !strs_ok {
                return false;
            }
            if matches!(w.region, "str_pool" | "str_start")
                || matches!(
                    w.scalar,
                    Some("pool_ptr" | "str_ptr" | "cs_count" | "hash_used" | "hash_high")
                )
            {
                return true;
            }
            let (a, b) = (halves(w.old), halves(w.new));
            if w.region == "hash" && w.scalar.is_none() {
                // (text: the low half; the chains of new names: lenient)
                return b.0 == map(a.0) || a.0 == 0 || b.0 == 0;
            }
            (b.0 == a.0 || b.0 == map(a.0)) && (b.1 == a.1 || b.1 == map(a.1))
        };
        let left: Vec<_> = words
            .into_iter()
            .filter(|w| !dead_word(g, w))
            .filter(|w| !position_only(g, w))
            .filter(|w| w.region != "dl_side")
            .filter(|w| match (w.region, w.scalar) {
                ("eqtb", None) => !skip.contains(&(w.index as i32 + 1)),
                ("hash", None) => !skip.contains(&(w.index as i32 + hash_base)),
                _ => true,
            })
            .filter(|w| !(relax && w.region != "mem" && renumbered(w)))
            .collect();
        let (left, (free_o, free_n)) = drop_free_mem(g, d, &layout, left);
        let uncov: Vec<_> = left
            .iter()
            .filter(|w| w.region != "mem" && !iso_covers(g, w))
            .cloned()
            .collect();
        out.uncov_n = uncov.len();
        out.uncov_sum = crate::statediff::summary(&uncov);
        out.uncov = uncov.iter().take(6).map(|w| format!("{w}")).collect();
        let bad_mem: Vec<usize> = left
            .iter()
            .filter(|w| w.region == "mem")
            .map(|w| w.index)
            .collect();
        if !left.is_empty() || side_any {
            let mut fetch = |cs: &[u32]| {
                let mut never = || false;
                g.pending_old_chunks(old, cs, &mut never)
            };
            let mut never = || false;
            let r = crate::iso::with_str_map(smap.clone(), || {
                crate::iso::with_skip(skip, || {
                    crate::iso::Iso::check(
                        g,
                        d,
                        &layout,
                        free_o.as_deref(),
                        free_n.as_deref(),
                        &bad_mem,
                        side_any.then_some(crate::iso::SideOld {
                            written: &side_written,
                            fetch: &mut fetch,
                        }),
                        g.hyph_list.len(),
                        false,
                        &mut never,
                    )
                })
            });
            out.iso = Some(r);
        }
        out
    }
}

/// The string renumbering from the old state to the live one, when the live
/// one's strings are the old one's with strings of `ins` (the patch's new
/// names) inserted: `None` when the numbering is the same, else the map
/// (old -> new) and how many were inserted. Strings below the first
/// `str_start` that differs are taken as equal (they precede the `.aux`
/// read; checked for the 64 before it).
#[allow(clippy::type_complexity)]
fn str_map(
    ov: &View,
    nv: &View,
    ins: &HashSet<Vec<u8>>,
) -> Result<Option<(Vec<i32>, usize)>, String> {
    let (so, sn) = (ov.str_ptr(), nv.str_ptr());
    let lim = so.min(sn);
    let mut s0 = lim;
    for s in 0..=lim {
        if ov.str_start(s) != nv.str_start(s) {
            s0 = (s - 1).max(0);
            break;
        }
    }
    if s0 == lim && so == sn {
        return Ok(None);
    }
    for s in (s0 - 64).max(1)..s0 {
        if ov.string_bytes(s) != nv.string_bytes(s) {
            return Err(format!("string {s} differs below the first moved one"));
        }
    }
    let mut m: Vec<i32> = (0..so.max(0) + 1).collect();
    let (mut i, mut j) = (s0, s0);
    let mut n = 0;
    while i < so {
        let a = ov.string_bytes(i);
        let b = if j < sn { nv.string_bytes(j) } else { vec![] };
        if j < sn && a == b {
            m[i as usize] = j;
            i += 1;
            j += 1;
        } else if j < sn && ins.contains(&b) {
            j += 1;
            n += 1;
        } else {
            return Err(format!(
                "old string {i} {:?} vs new {j} {:?}",
                String::from_utf8_lossy(&a).chars().take(40).collect::<String>(),
                String::from_utf8_lossy(&b).chars().take(40).collect::<String>()
            ));
        }
    }
    while j < sn {
        let b = nv.string_bytes(j);
        if !ins.contains(&b) {
            return Err(format!(
                "new string {j} {:?} past the old ones",
                String::from_utf8_lossy(&b).chars().take(40).collect::<String>()
            ));
        }
        j += 1;
        n += 1;
    }
    m[so.max(0) as usize] = sn;
    Ok(Some((m, n)))
}

/// `aux_delta`, a panic (a garbage pointer `View::meaning` follows, in a
/// state far from the other) as an error.
fn delta(
    g: &mut Globals,
    d: &crate::arena::ChunkDiff,
    before: &Patch,
) -> Result<(Patch, Patch), String> {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::readset::aux_delta(g, d, before, &|g, w| dead_word(g, w))
    }));
    match r {
        Ok(r) => r,
        Err(_) => Err("aux_delta panicked (a pointer outside mem)".into()),
    }
}

fn describe(n: &Name, m: &Meaning) -> String {
    let s = match m {
        Meaning::Undefined => "undefined".to_string(),
        Meaning::Macro { toks, .. } => format!("macro({} toks)", toks.len()),
        Meaning::Value { ty, equiv, .. } => format!("value(ty {ty}, equiv {equiv})"),
    };
    format!("{n}={s}")
}
