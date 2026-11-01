//! `collab-v1` oracle: the in-process simulation harness and property tests
//! (proposal §7.2). Thousands of seeded scenarios with two to five peers,
//! hostile delivery (reorder, duplicate, drop then resync, partition then
//! heal) and every frame through the codec; all peers must end
//! byte-identical. `FLASHTEX_COLLAB_FUZZ_CASES=1000000 cargo test --release
//! --test v1_sim` is the P0 gate run; the default keeps `cargo test` quick.

mod common;

use common::{random_text, Rng, Sim};
use flashtex_collaboration_core::v1::UndoManager;
use flashtex_collaboration_core::v1::{
    wire, Assoc, CollabError, FileId, FileKind, Id, Project, Section, TextDoc, TextOp,
};

fn cases(default: u64) -> u64 {
    std::env::var("FLASHTEX_COLLAB_FUZZ_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// One random scenario (`common::random_scenario`: edits, undo and redo,
/// file and blob ops, attacks, partitions); returns the updates it sent.
pub fn run_scenario(seed: u64) -> usize {
    let sim = common::random_scenario(seed, 40);
    sim.assert_converged(&format!("seed {seed:#x}"));
    sim.frames.len()
}

#[test]
fn random_interleavings_converge() {
    let n = cases(3_000);
    // Shards of a long run: FLASHTEX_COLLAB_FUZZ_FIRST=250000 etc.
    let first: u64 = std::env::var("FLASHTEX_COLLAB_FUZZ_FIRST")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut ops = 0;
    for seed in first..first + n {
        ops += run_scenario(seed);
    }
    eprintln!(
        "seeds {first}..{}: {n} scenarios, {ops} broadcast updates, 0 divergences",
        first + n
    );
}

/// With causal, non-concurrent delivery (every edit reaches everyone
/// before the next one) the text equals a plain string edited the same way.
#[test]
fn sequential_delivery_matches_a_plain_string() {
    for seed in 0..cases(2_000).min(5_000) {
        let mut rng = Rng(seed);
        let n = 2 + rng.below(3);
        let mut peers: Vec<TextDoc> = (0..n as u64).map(|r| TextDoc::new(r + 1)).collect();
        let mut reference: Vec<char> = Vec::new();
        for _ in 0..40 {
            let p = rng.below(n);
            let len = reference.len();
            let ops = if len > 0 && rng.chance(35) {
                let pos = rng.below(len);
                let k = 1 + rng.below((len - pos).min(5));
                reference.drain(pos..pos + k);
                peers[p].delete(pos, k)
            } else {
                let pos = rng.below(len + 1);
                let t = random_text(&mut rng, 5);
                for (i, c) in t.chars().enumerate() {
                    reference.insert(pos + i, c);
                }
                vec![peers[p].insert(pos, &t).unwrap()]
            };
            for (q, peer) in peers.iter_mut().enumerate() {
                if q != p {
                    for op in &ops {
                        peer.apply(op).unwrap();
                    }
                }
            }
            let want: String = reference.iter().collect();
            for peer in &peers {
                assert_eq!(peer.text(), want, "seed {seed}");
            }
        }
    }
}

/// Concurrent typing at one spot never interleaves: each peer's word
/// stays contiguous, typed forwards (each key after the last) or backwards
/// (each key before the last), one operation per key.
#[test]
fn concurrent_typing_at_one_spot_does_not_interleave() {
    for seed in 0..cases(2_000).min(4_000) {
        let mut rng = Rng(seed);
        let n = 2 + rng.below(3);
        let base = TextDoc::new(999);
        let mut base = base;
        base.insert(0, "[]").unwrap();
        let base_ops = base.log().to_vec();
        let mut peers: Vec<TextDoc> = (0..n as u64)
            .map(|r| {
                let mut d = TextDoc::new(rng.next() | 1);
                let _ = r;
                for op in &base_ops {
                    d.apply(op).unwrap();
                }
                d
            })
            .collect();
        let backwards = rng.chance(50);
        let mut words = Vec::new();
        let mut all_ops: Vec<Vec<TextOp>> = Vec::new();
        for (k, peer) in peers.iter_mut().enumerate() {
            let letter = (b'a' + k as u8) as char;
            let len = 1 + rng.below(6);
            let word: String = std::iter::repeat_n(letter, len).collect();
            let mut ops = Vec::new();
            for i in 0..len {
                let pos = if backwards { 1 } else { 1 + i };
                ops.push(peer.insert(pos, &letter.to_string()).unwrap());
            }
            words.push(word);
            all_ops.push(ops);
        }
        // Deliver everyone's keys to everyone, interleaved at random but in
        // each peer's own order.
        for (q, peer) in peers.iter_mut().enumerate() {
            let mut cursors = vec![0; n];
            loop {
                let ready: Vec<usize> = (0..n)
                    .filter(|&k| k != q && cursors[k] < all_ops[k].len())
                    .collect();
                if ready.is_empty() {
                    break;
                }
                let k = ready[rng.below(ready.len())];
                peer.apply(&all_ops[k][cursors[k]]).unwrap();
                cursors[k] += 1;
            }
        }
        let text = peers[0].text();
        for peer in &peers {
            assert_eq!(peer.text(), text, "seed {seed}");
        }
        let inner = &text[1..text.len() - 1];
        for w in &words {
            assert!(
                inner.contains(w.as_str()),
                "seed {seed} backwards={backwards}: {w:?} interleaved in {inner:?}"
            );
        }
    }
}

/// Insertions inside a range another peer deletes concurrently survive;
/// the deleted text goes, once, however often it is deleted.
#[test]
fn concurrent_delete_and_insert_keep_both_intentions() {
    let mut a = TextDoc::new(1);
    let op0 = a.insert(0, "hello world").unwrap();
    let mut b = TextDoc::new(2);
    let mut c = TextDoc::new(3);
    b.apply(&op0).unwrap();
    c.apply(&op0).unwrap();
    let da = a.delete(0, 6); // "hello "
    let ib = b.insert(3, "XY").unwrap(); // "helXYlo world"
    let dc = c.delete(2, 4); // "he" + "world"
    for op in da.iter().chain([&ib]).chain(dc.iter()) {
        for d in [&mut a, &mut b, &mut c] {
            let _ = d.apply(op).unwrap();
        }
    }
    assert_eq!(a.text(), "XYworld");
    assert_eq!(a.text(), b.text());
    assert_eq!(a.digest(), c.digest());
}

/// A caret anchored to a scalar stays next to it through concurrent edits
/// on either side, and a deleted anchor resolves to where it was.
#[test]
fn relative_positions_follow_their_anchor() {
    for seed in 0..cases(1_000).min(2_000) {
        let mut rng = Rng(seed);
        let mut a = TextDoc::new(1);
        let mut b = TextDoc::new(2);
        let t = random_text(&mut rng, 12);
        let op = a.insert(0, &t).unwrap();
        b.apply(&op).unwrap();
        let len = a.len();
        let pos = rng.below(len + 1);
        let assoc = if rng.chance(50) {
            Assoc::Before
        } else {
            Assoc::After
        };
        let rp = a.relative_position(pos, assoc);
        let anchor_char = match assoc {
            Assoc::Before => a.text().chars().nth(pos),
            Assoc::After => pos.checked_sub(1).and_then(|p| a.text().chars().nth(p)),
        };
        for _ in 0..10 {
            let l = b.len();
            let ops = if l > 0 && rng.chance(30) {
                let p = rng.below(l);
                b.delete(p, 1)
            } else {
                let p = rng.below(l + 1);
                vec![b.insert(p, &random_text(&mut rng, 3)).unwrap()]
            };
            for op in &ops {
                a.apply(op).unwrap();
            }
        }
        let at = a.resolve(rp).unwrap();
        assert!(at <= a.len());
        let anchor_visible = rp.anchor.is_none_or(|id| a.visible_ids().contains(&id));
        if anchor_visible {
            let chars: Vec<char> = a.text().chars().collect();
            match (assoc, anchor_char) {
                (Assoc::Before, Some(ch)) => assert_eq!(chars[at], ch, "seed {seed}"),
                (Assoc::After, Some(ch)) => assert_eq!(chars[at - 1], ch, "seed {seed}"),
                (Assoc::Before, None) => assert_eq!(at, a.len()),
                (Assoc::After, None) => assert_eq!(at, 0),
            }
        }
    }
}

#[test]
fn duplicates_and_partial_overlaps_are_idempotent() {
    let mut a = TextDoc::new(7);
    let op = a.insert(0, "abcdef").unwrap();
    let mut b = TextDoc::new(8);
    let TextOp::Insert { .. } = op else { panic!() };
    let tail = op.without_prefix(2);
    // The tail alone cannot apply: it depends on the head.
    assert_eq!(
        b.apply(&tail),
        Err(CollabError::MissingDependency(Id::new(7, 0)))
    );
    b.apply(&op).unwrap();
    assert_eq!(
        b.apply(&op),
        Ok(flashtex_collaboration_core::v1::Applied::Duplicate)
    );
    assert_eq!(
        b.apply(&tail),
        Ok(flashtex_collaboration_core::v1::Applied::Duplicate)
    );
    assert_eq!(b.text(), "abcdef");
    assert_eq!(a.digest(), b.digest());
}

/// `id_reuse_divergence.rs`'s lesson, in v1: reusing an id with different
/// content is refused, never merged.
#[test]
fn reused_ids_are_refused() {
    let mut a = TextDoc::new(7);
    let op = a.insert(0, "abc").unwrap();
    let TextOp::Insert {
        id,
        origin_left,
        origin_right,
        ..
    } = op.clone()
    else {
        panic!()
    };
    let forged = TextOp::Insert {
        id,
        origin_left,
        origin_right,
        content: "abX".into(),
    };
    let mut b = TextDoc::new(8);
    b.apply(&op).unwrap();
    assert_eq!(
        b.apply(&forged),
        Err(CollabError::IdConflict(Id::new(7, 2)))
    );
    assert_eq!(b.text(), "abc");
}

#[test]
fn file_map_renames_collisions_and_deletes() {
    let (f1, f2) = (FileId([1; 16]), FileId([2; 16]));
    let mut a = Project::new(1);
    let mut b = Project::new(2);
    let s1 = a
        .file_op(|m| m.create(f1, FileKind::Text, "main.tex"))
        .unwrap();
    b.receive(std::slice::from_ref(&s1));
    let e = a.insert(f1, 0, "\\section{A}").unwrap();
    b.receive(&[e]);
    // Concurrently: a renames main.tex, b edits it and creates another file
    // at the new name.
    let r = a.file_op(|m| m.rename(f1, "intro.tex")).unwrap();
    let e2 = b.insert(f1, 0, "% b\n").unwrap();
    let c2 = b
        .file_op(|m| m.create(f2, FileKind::Text, "intro.tex"))
        .unwrap();
    a.receive(&[e2, c2]);
    b.receive(std::slice::from_ref(&r));
    let fa = a.files().files();
    assert_eq!(fa, b.files().files());
    assert_eq!(a.text(f1).unwrap().text(), "% b\n\\section{A}");
    let paths: Vec<&str> = fa.iter().map(|e| e.path.as_str()).collect();
    // b's create has lamport 2 like a's rename (both after one op); the
    // replica breaks the tie: a (1) keeps the name.
    assert_eq!(paths.len(), 2);
    assert!(paths.contains(&"intro.tex"));
    assert!(fa.iter().all(|e| e.conflict));
    // Delete versus concurrent edit: delete wins, content is kept.
    let d = a.file_op(|m| m.set_deleted(f1, true)).unwrap();
    let e3 = b.insert(f1, 0, "x").unwrap();
    a.receive(&[e3]);
    b.receive(&[d]);
    assert_eq!(a.files().files(), b.files().files());
    assert_eq!(a.files().files().len(), 1);
    assert_eq!(a.text(f1).unwrap().text(), b.text(f1).unwrap().text());
    assert_eq!(a.digest(), b.digest());
}

#[test]
fn codec_refuses_hostile_frames() {
    use wire::{decode, WireError};
    assert_eq!(decode(&[0, 0, 0, 0, 5]), Err(WireError::EmptyFrame));
    assert_eq!(decode(&[0, 0, 0, 2, 5]), Err(WireError::FrameTooLarge));
    assert_eq!(decode(&[3, 0, 0]), Ok(None));
    // update, seq 1, 1 section claiming 200 ops in 3 bytes
    assert_eq!(
        decode(&[5, 0, 0, 0, 5, 1, 1, 0, 200]),
        Err(WireError::Truncated)
    );
    // overlong varint
    assert_eq!(
        decode(&[3, 0, 0, 0, 5, 0x80, 0x00]),
        Err(WireError::BadVarint)
    );
    // unknown kind: not fatal
    assert!(matches!(
        decode(&[2, 0, 0, 0, 0x7f, 9]),
        Ok(Some((wire::Message::Unknown { kind: 0x7f, .. }, 6)))
    ));
    // random bytes never panic
    let mut rng = Rng(42);
    for _ in 0..20_000 {
        let n = rng.below(40);
        let mut b: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        if b.len() >= 5 {
            let len = (b.len() - 4) as u32;
            b[..4].copy_from_slice(&len.to_le_bytes());
            b[4] = [3, 4, 5, 9][rng.below(4)];
        }
        let _ = decode(&b);
    }
}

#[test]
fn sections_round_trip() {
    let mut sim = Sim::new(9, 2);
    let f = sim.rng.file_id();
    let s = sim.peers[0]
        .file_op(|m| m.create(f, FileKind::Text, "main.tex"))
        .unwrap();
    let t = sim.peers[0].insert(f, 0, "a😀é").unwrap();
    let msg = wire::Message::Update {
        seq: 300,
        sections: vec![s, t],
    };
    let bytes = wire::encode(&msg);
    let (back, used) = wire::decode(&bytes).unwrap().unwrap();
    assert_eq!(used, bytes.len());
    assert_eq!(back, msg);
    let _: Option<Section> = None;
}

/// Undo never removes another replica's scalars; one user's undo and redo
/// walk back and forth through every state; pruning keeps copies bounded.
#[test]
fn undo_touches_only_own_scalars_and_round_trips() {
    for seed in 0..cases(300).min(2_000) {
        let mut rng = Rng(seed);
        let mut a = TextDoc::new(1);
        let mut b = TextDoc::new(2);
        let mut ua = UndoManager::new();
        let mut states = vec![a.text()];
        for _ in 0..(1 + rng.below(20)) {
            let len = a.len();
            let ops = if len > 0 && rng.chance(40) {
                let pos = rng.below(len);
                a.delete(pos, 1 + rng.below((len - pos).min(5)))
            } else {
                vec![a
                    .insert(rng.below(len + 1), &random_text(&mut rng, 5))
                    .unwrap()]
            };
            ua.record(&ops);
            states.push(a.text());
        }
        for want in states.iter().rev().skip(1) {
            ua.undo(&mut a);
            assert_eq!(&a.text(), want, "seed {seed}: undo");
        }
        for want in states.iter().skip(1) {
            ua.redo(&mut a);
            assert_eq!(&a.text(), want, "seed {seed}: redo");
        }
        // Another replica types inside; undoing everything keeps its text.
        for op in a.log().to_vec() {
            b.apply(&op).unwrap();
        }
        let len = b.len();
        let theirs = b.insert(rng.below(len + 1), "OTHER").unwrap();
        a.apply(&theirs).unwrap();
        let others: Vec<Id> = a
            .visible_ids()
            .into_iter()
            .filter(|i| i.replica != 1)
            .collect();
        while !ua.undo(&mut a).is_empty() {}
        let after: Vec<Id> = a
            .visible_ids()
            .into_iter()
            .filter(|i| i.replica != 1)
            .collect();
        assert_eq!(
            others, after,
            "seed {seed}: undo removed another replica's text"
        );
        assert!(a.text().contains("OTHER"));
    }
    let mut d = TextDoc::new(3);
    let mut u = UndoManager::new();
    u.limit = 8;
    for k in 0..400 {
        let op = d.insert(0, "ab").unwrap();
        u.record(&[op]);
        if k % 3 == 0 {
            u.undo(&mut d);
            u.redo(&mut d);
        }
    }
    assert!(
        u.copies_len() <= 64,
        "copies not pruned: {}",
        u.copies_len()
    );
}

/// The codec and integration under mutation (the in-tree stand-in for a
/// cargo-fuzz target; cargo-fuzz is not installed on the agents' machines):
/// valid frames from random scenarios, mutated at random, must decode or be
/// refused without a panic, and whatever decodes must apply or be refused
/// identically by two replicas that share the history.
#[test]
fn mutated_frames_never_panic_and_refusals_agree() {
    let rounds = cases(200).min(5_000);
    for seed in 0..rounds {
        let sim = common::random_scenario(seed ^ 0xf022, 15);
        let mut rng = Rng(seed);
        let mut a = Project::new(u64::MAX - 1);
        let mut b = Project::new(u64::MAX - 2);
        for f in &sim.frames {
            if let Ok(Some((wire::Message::Update { sections, .. }, _))) = wire::decode(f) {
                a.receive(&sections);
                b.receive(&sections);
            }
        }
        for _ in 0..50 {
            let mut f = sim.frames[rng.below(sim.frames.len())].clone();
            for _ in 0..1 + rng.below(4) {
                let i = 4 + rng.below(f.len() - 4);
                match rng.below(3) {
                    0 => f[i] ^= 1 << rng.below(8),
                    1 => f[i] = rng.next() as u8,
                    _ => f[i] = f[i].wrapping_add(1),
                }
            }
            if let Ok(Some((wire::Message::Update { sections, .. }, _))) = wire::decode(&f) {
                let ea: Vec<_> = a
                    .receive(&sections)
                    .into_iter()
                    .map(|e| (e.op, e.error.kind_name()))
                    .collect();
                let eb: Vec<_> = b
                    .receive(&sections)
                    .into_iter()
                    .map(|e| (e.op, e.error.kind_name()))
                    .collect();
                assert_eq!(ea, eb, "seed {seed}");
                assert_eq!(a.digest(), b.digest(), "seed {seed}");
            }
        }
    }
}
