//! `id_lookup` with `changes/throughput.ch` [9]'s index (`src/csindex.rs`)
//! against `id_lookup` without it, step for step: the same location for
//! every name, found, entered or absent, and the same hash table and string
//! pool after every step. Two engines get the same random mix of names:
//! - enough names, over a small alphabet, that the 8,501 chains are long and
//!   the index's 64 Ki slots collide;
//! - lookups that may enter a name (`no_new_control_sequence` false) and
//!   lookups that may not;
//! - restores: both engines' hash and strings go back to an earlier state,
//!   while the indexed one keeps an index filled by the later lookups;
//! - names taken out of their chains, as `readset.rs` takes them out.

use flashtex_engine::generated::consts::{
    eqtb_size, frozen_control_sequence, hash_base, undefined_control_sequence,
};
use flashtex_engine::Globals;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn engine(index: bool) -> Box<Globals> {
    let mut g = Globals::new();
    // strings 0..255 (empty here) exist, as after INITEX's pool
    g.str_ptr = 256;
    g.pool_ptr = 0;
    g.hash_used = frozen_control_sequence;
    g.hash_high = 0;
    if index {
        g.host.set_verify();
    } else {
        g.host.set_enabled(false);
    }
    g
}

fn lookup(g: &mut Globals, name: &[u8], enter: bool) -> i32 {
    for (i, &c) in name.iter().enumerate() {
        g.buffer[i] = c as i32;
    }
    g.no_new_control_sequence = !enter;
    g.id_lookup(0, name.len() as i32)
}

/// What a restore puts back: the hash, the strings and their counters.
struct State {
    hash: Vec<(i32, i32)>,
    start: Vec<i32>,
    pool: Vec<i32>,
    scalars: [i32; 5],
}

fn save(g: &Globals) -> State {
    State {
        hash: g.hash.iter().map(|w| (w.lh(), w.rh())).collect(),
        start: g.str_start[..=g.str_ptr as usize].to_vec(),
        pool: g.str_pool[..g.pool_ptr as usize].to_vec(),
        scalars: [g.str_ptr, g.pool_ptr, g.hash_used, g.hash_high, g.cs_count],
    }
}

fn restore(g: &mut Globals, s: &State) {
    for (i, &(lh, rh)) in s.hash.iter().enumerate() {
        g.hash[i].set_lh(lh);
        g.hash[i].set_rh(rh);
    }
    for (i, &v) in s.start.iter().enumerate() {
        g.str_start[i] = v;
    }
    for (i, &v) in s.pool.iter().enumerate() {
        g.str_pool[i] = v;
    }
    [g.str_ptr, g.pool_ptr, g.hash_used, g.hash_high, g.cs_count] = s.scalars;
}

/// Takes the name at `p` out of its chain, as `readset.rs` does: its
/// predecessor links past it, and its text is cleared. Only a name that does
/// not head its chain (`hash_extra`'s locations never do).
fn take_out(g: &mut Globals, p: i32) {
    let idx = |q: i32| (q - hash_base) as usize;
    for q in hash_base..hash_base + g.hash.len() as i32 {
        if g.hash[idx(q)].lh() == p {
            let after = g.hash[idx(p)].lh();
            g.hash[idx(q)].set_lh(after);
            g.hash[idx(p)].set_lh(0);
            g.hash[idx(p)].set_rh(0);
            return;
        }
    }
}

fn same_tables(a: &Globals, b: &Globals, step: usize) {
    assert_eq!(a.str_ptr, b.str_ptr, "step {step}: str_ptr");
    assert_eq!(a.hash_high, b.hash_high, "step {step}: hash_high");
    assert!(
        a.hash
            .iter()
            .zip(b.hash.iter())
            .all(|(x, y)| x.lh() == y.lh() && x.rh() == y.rh()),
        "step {step}: the hash tables differ"
    );
    let n = a.pool_ptr as usize;
    assert_eq!(
        &a.str_pool[..n],
        &b.str_pool[..n],
        "step {step}: the pools differ"
    );
}

#[test]
fn the_index_answers_what_the_walk_answers() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let names: Vec<Vec<u8>> = (0..90_000)
        .map(|_| {
            let len = 2 + rng.below(11);
            (0..len).map(|_| b"abcdefghij@:_"[rng.below(13)]).collect()
        })
        .collect();
    let (mut g, mut r) = (engine(true), engine(false));
    let mut saved: Option<State> = None;
    let mut entered: Vec<usize> = vec![];
    for step in 0..400_000 {
        let k = match rng.below(10) {
            // a name entered before (the common case: a hit)
            0..=5 if !entered.is_empty() => entered[rng.below(entered.len())],
            _ => rng.below(names.len()),
        };
        let enter = rng.below(4) != 0;
        let (a, b) = (
            lookup(&mut g, &names[k], enter),
            lookup(&mut r, &names[k], enter),
        );
        assert_eq!(
            a,
            b,
            "step {step}: {:?}",
            String::from_utf8_lossy(&names[k])
        );
        if b != undefined_control_sequence && enter {
            entered.push(k);
        }
        match step % 50_000 {
            10_000 => saved = Some(save(&r)),
            // back to the saved state; the index keeps what came after
            30_000 => {
                let s = saved.take().unwrap();
                restore(&mut g, &s);
                restore(&mut r, &s);
                same_tables(&g, &r, step);
            }
            // a name taken out, by both
            40_000 if r.hash_high > 0 => {
                let p = eqtb_size + 1 + rng.below(r.hash_high as usize) as i32;
                take_out(&mut g, p);
                take_out(&mut r, p);
            }
            _ => {}
        }
        if step % 20_000 == 0 {
            same_tables(&g, &r, step);
        }
    }
    same_tables(&g, &r, 400_000);
    assert!(
        g.cs_count > 30_000,
        "chains too short to test: {} names",
        g.cs_count
    );
    assert!(
        g.host.hits > 100_000 && g.host.misses > 50_000,
        "hits {}, misses {}",
        g.host.hits,
        g.host.misses
    );
}
