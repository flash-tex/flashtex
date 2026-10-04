//! Differential fixtures (proposal §7.2): `collab-v1` op logs that both
//! implementations replay. Each fixture is a list of `update` frames, some
//! delivery orders (reordered, duplicated), and the state a fresh replica
//! must reach: every live file's materialised path and text, the structure
//! digests, the operations it refuses (and with which error kind), and how
//! many wait forever. The Swift core (`FlashTeXCollabCoreTests`) replays
//! every file here, including the `swift-*.json` ones it recorded itself,
//! and this test replays them all with the oracle. Either side drifting
//! fails both.
//!
//! `collab-v1-bulk/bulk-200.json` is the bulk differential: 200 random
//! simulation scenarios (2-5 peers, edits, undo/redo, file and blob ops,
//! attacks) the Swift suite replays in all three orders. This test checks
//! the committed file is exactly what the oracle generates today.
//!
//! Regenerate everything the oracle writes with
//! `FLASHTEX_COLLAB_RECORD=1 cargo test -p flashtex-collaboration-core --test v1_fixtures`.
//! A larger bulk run on demand:
//! `FLASHTEX_COLLAB_BULK=5000 FLASHTEX_COLLAB_BULK_OUT=/tmp/bulk.json cargo test --release
//! -p flashtex-collaboration-core --test v1_fixtures bulk`, then
//! `FLASHTEX_COLLAB_BULK_FILE=/tmp/bulk.json swift test --filter FixtureTests` in `apps/mac`.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::{hex, hostile_paths, random_scenario, unhex, Json, Rejection, Rng, Sim, TRICKY_PATHS};
use flashtex_collaboration_core::v1::wire::{self, Message};
use flashtex_collaboration_core::v1::{
    BlobRef, DocRef, FileId, FileKind, FileOp, FileOpKind, Id, Project, Section, TextOp,
};

const OBSERVER: u64 = u64::MAX;
const BULK_DEFAULT: u64 = 200;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collab-v1")
}

fn bulk_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collab-v1-bulk/bulk-200.json")
}

fn replay(frames: &[Vec<u8>], order: &[usize]) -> (Project, BTreeSet<Rejection>) {
    let mut p = Project::new(OBSERVER);
    let mut rejected = BTreeSet::new();
    for &i in order {
        let (msg, _) = wire::decode(&frames[i]).unwrap().unwrap();
        let Message::Update { sections, .. } = msg else {
            panic!("fixture frames are updates")
        };
        for e in p.receive(&sections) {
            rejected.insert((e.doc, e.op, e.error.kind_name()));
        }
    }
    (p, rejected)
}

fn doc_name(d: DocRef) -> String {
    match d {
        DocRef::FileMap => "files".into(),
        DocRef::Text(f) => hex(&f.0),
    }
}

fn expected(p: &Project, rejected: &BTreeSet<Rejection>) -> Json {
    let files = p
        .files()
        .files()
        .into_iter()
        .map(|e| {
            let mut kv = vec![
                ("file".to_owned(), Json::Str(hex(&e.file.0))),
                ("path".to_owned(), Json::Str(e.path.clone())),
                (
                    "kind".to_owned(),
                    Json::Str(match e.kind {
                        FileKind::Text => "text".into(),
                        FileKind::Blob => "blob".into(),
                    }),
                ),
            ];
            if let Some(t) = p.text(e.file) {
                kv.push(("text".into(), Json::Str(t.text())));
                kv.push(("digest".into(), Json::Str(format!("{:016x}", t.digest()))));
            }
            if let Some(b) = &e.blob {
                kv.push(("sha256".into(), Json::Str(hex(&b.sha256))));
            }
            Json::Obj(kv)
        })
        .collect();
    let rejected = rejected
        .iter()
        .map(|(d, id, kind)| {
            Json::Obj(vec![
                ("doc".into(), Json::Str(doc_name(*d))),
                ("replica".into(), Json::Str(format!("{:016x}", id.replica))),
                ("counter".into(), Json::Num(id.counter)),
                ("kind".into(), Json::Str((*kind).into())),
            ])
        })
        .collect();
    Json::Obj(vec![
        (
            "project_digest".into(),
            Json::Str(format!("{:016x}", p.digest())),
        ),
        (
            "filemap_digest".into(),
            Json::Str(format!("{:016x}", p.files().digest())),
        ),
        ("files".into(), Json::Arr(files)),
        ("rejected".into(), Json::Arr(rejected)),
        ("pending".into(), Json::Num(p.pending_len() as u64)),
    ])
}

/// Forward, reversed, and shuffled with duplicates.
fn all_orders(rng: &mut Rng, n: usize) -> Vec<Vec<usize>> {
    let fwd: Vec<usize> = (0..n).collect();
    let rev: Vec<usize> = (0..n).rev().collect();
    let mut shuffled = fwd.clone();
    for i in (1..n).rev() {
        shuffled.swap(i, rng.below(i + 1));
    }
    for _ in 0..n / 4 {
        let i = rng.below(n);
        shuffled.push(i);
    }
    vec![fwd, rev, shuffled]
}

/// Forward, and forward replayed twice (for fixtures whose outcome depends
/// on which of two conflicting operations arrives first).
fn forward_orders(n: usize) -> Vec<Vec<usize>> {
    let fwd: Vec<usize> = (0..n).collect();
    let twice: Vec<usize> = fwd.iter().chain(fwd.iter()).copied().collect();
    vec![fwd, twice]
}

fn fixture_json(
    name: &str,
    description: &str,
    frames: &[Vec<u8>],
    orders: Vec<Vec<usize>>,
) -> Json {
    let (p, rejected) = replay(frames, &orders[0]);
    Json::Obj(vec![
        ("format".into(), Json::Str("collab-v1-fixture/1".into())),
        ("name".into(), Json::Str(name.into())),
        ("description".into(), Json::Str(description.into())),
        (
            "generator".into(),
            Json::Str("rust oracle (crates/collaboration-core)".into()),
        ),
        (
            "frames".into(),
            Json::Arr(frames.iter().map(|f| Json::Str(hex(f))).collect()),
        ),
        (
            "orders".into(),
            Json::Arr(
                orders
                    .iter()
                    .map(|o| Json::Arr(o.iter().map(|&i| Json::Num(i as u64)).collect()))
                    .collect(),
            ),
        ),
        ("expected".into(), expected(&p, &rejected)),
    ])
}

fn write_json(path: &Path, json: &Json) {
    let mut out = String::new();
    json.write(&mut out, 0);
    out.push('\n');
    std::fs::write(path, out).unwrap();
}

fn write_fixture(name: &str, description: &str, frames: Vec<Vec<u8>>, seed: u64) {
    let mut rng = Rng(seed);
    let orders = all_orders(&mut rng, frames.len());
    write_json(
        &dir().join(format!("{name}.json")),
        &fixture_json(name, description, &frames, orders),
    );
}

fn new_sim_with_main(seed: u64, n: usize) -> (Sim, FileId) {
    let mut sim = Sim::new(seed, n);
    let main = sim.rng.file_id();
    let s = sim.peers[0]
        .file_op(|m| m.create(main, FileKind::Text, "main.tex"))
        .unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    (sim, main)
}

fn flush(sim: &mut Sim) {
    while !sim.queue.is_empty() {
        let m = sim.queue.remove(0);
        sim.receive_frame(m.to, &m.frame);
    }
}

fn update(seq: u64, sections: Vec<Section>) -> Vec<u8> {
    wire::encode(&Message::Update { seq, sections })
}

/// The bulk differential: `n` random scenarios as one JSON document.
fn bulk(n: u64) -> Json {
    let scenarios = (0..n)
        .map(|k| {
            let seed = 0xb01c_0000 + k;
            let sim = random_scenario(seed, 30);
            sim.assert_converged(&format!("bulk {seed:#x}"));
            let mut rng = Rng(seed);
            let orders = all_orders(&mut rng, sim.frames.len());
            fixture_json(
                &format!("bulk-{k}"),
                &format!("seed {seed:#x}, {} peers", sim.peers.len()),
                &sim.frames,
                orders,
            )
        })
        .collect();
    Json::Obj(vec![
        ("format".into(), Json::Str("collab-v1-bulk/1".into())),
        ("scenarios".into(), Json::Arr(scenarios)),
    ])
}

fn record() {
    std::fs::create_dir_all(dir()).unwrap();

    // Three peers type at the same gap at once.
    let (mut sim, f) = new_sim_with_main(1, 3);
    let s = sim.peers[0].insert(f, 0, "\\begin{x}\\end{x}").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    for (p, word) in ["alpha", "beta", "gamma"].iter().enumerate() {
        let s = sim.peers[p].insert(f, 9, word).unwrap();
        sim.broadcast(p, vec![s]);
    }
    flush(&mut sim);
    sim.assert_converged("same-spot");
    write_fixture(
        "same-spot",
        "three peers insert a word at the same gap concurrently",
        sim.frames,
        11,
    );

    // Key-by-key typing, forwards and backwards, concurrently.
    let (mut sim, f) = new_sim_with_main(2, 2);
    let s = sim.peers[0].insert(f, 0, "()").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    for i in 0..6 {
        let s = sim.peers[0].insert(f, 1 + i, "f").unwrap();
        sim.broadcast(0, vec![s]);
        let s = sim.peers[1].insert(f, 1, "b").unwrap();
        sim.broadcast(1, vec![s]);
    }
    flush(&mut sim);
    sim.assert_converged("typing");
    write_fixture(
        "typing-forwards-backwards",
        "one peer types forwards and one backwards at the same spot, a key per op",
        sim.frames,
        12,
    );

    // Overlapping deletes, an insert inside a deleted range, multi-byte text.
    let (mut sim, f) = new_sim_with_main(3, 3);
    let s = sim.peers[0].insert(f, 0, "a😀b中c e\u{301}d").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    let s = sim.peers[0].delete(f, 1, 4).unwrap();
    sim.broadcast(0, vec![s]);
    let s = sim.peers[1].delete(f, 3, 4).unwrap();
    sim.broadcast(1, vec![s]);
    let s = sim.peers[2].insert(f, 2, "é😀").unwrap();
    sim.broadcast(2, vec![s]);
    flush(&mut sim);
    sim.assert_converged("deletes");
    write_fixture(
        "deletes-multibyte",
        "overlapping concurrent deletes, an insert inside a deleted range, 2-4 byte scalars and a combining mark",
        sim.frames,
        13,
    );

    // File map: rename races, a path collision, delete vs edit, a blob.
    let (mut sim, f) = new_sim_with_main(4, 3);
    let s = sim.peers[1].insert(f, 0, "\\input{a}").unwrap();
    sim.broadcast(1, vec![s]);
    flush(&mut sim);
    let s = sim.peers[0].file_op(|m| m.rename(f, "a.tex")).unwrap();
    sim.broadcast(0, vec![s]);
    let g = sim.rng.file_id();
    let s = sim.peers[1]
        .file_op(|m| m.create(g, FileKind::Text, "a.tex"))
        .unwrap();
    sim.broadcast(1, vec![s]);
    let s = sim.peers[1].insert(g, 0, "chapter").unwrap();
    sim.broadcast(1, vec![s]);
    let img = sim.rng.file_id();
    let s = sim.peers[2]
        .file_op(|m| m.create(img, FileKind::Blob, "fig/plot.png"))
        .unwrap();
    sim.broadcast(2, vec![s]);
    let s = sim.peers[2]
        .file_op(|m| {
            m.set_blob(
                img,
                BlobRef {
                    sha256: [7; 32],
                    bytes: 1234,
                    media_type: "image/png".into(),
                },
            )
        })
        .unwrap();
    sim.broadcast(2, vec![s]);
    flush(&mut sim);
    let s = sim.peers[0].file_op(|m| m.set_deleted(g, true)).unwrap();
    sim.broadcast(0, vec![s]);
    let s = sim.peers[1].insert(g, 7, " one").unwrap();
    sim.broadcast(1, vec![s]);
    flush(&mut sim);
    sim.assert_converged("filemap");
    write_fixture(
        "file-map",
        "rename racing a create at the same path, delete racing an edit, a blob file",
        sim.frames,
        14,
    );

    // Long runs: inserts longer than the Swift core's storage chunk, and
    // deletes that cut across many runs.
    let (mut sim, f) = new_sim_with_main(5, 3);
    let mut rng = Rng(55);
    for step in 0..40 {
        let p = rng.below(3);
        let len = sim.peers[p].text(f).unwrap().len();
        let s = if len > 400 && rng.chance(40) {
            let pos = rng.below(len - 300);
            sim.peers[p].delete(f, pos, 50 + rng.below(250))
        } else {
            let pos = rng.below(len + 1);
            let t: String = (0..300 + rng.below(400))
                .map(|_| common::PIECES[rng.below(common::PIECES.len())])
                .collect();
            sim.peers[p].insert(f, pos, &t)
        };
        sim.broadcast(p, vec![s.unwrap()]);
        if step % 3 == 2 {
            flush(&mut sim);
        }
    }
    flush(&mut sim);
    sim.assert_converged("long-runs");
    write_fixture(
        "long-runs",
        "inserts of 300-700 scalars and deletes across many runs, partly concurrent",
        sim.frames,
        15,
    );

    record_hostile();
    record_legacy();

    // Random scenarios from the simulation harness.
    for k in 0..8u64 {
        let seed = 0xf1c5 + k;
        let mut rng = Rng(seed);
        let n = 2 + rng.below(4);
        let (mut sim, _) = new_sim_with_main(seed, n);
        let steps = if k == 7 { 2_000 } else { 60 };
        for _ in 0..steps {
            let p = rng.below(n);
            match rng.below(20) {
                0 if k != 7 => sim.random_file_op(p),
                1..=6 => {
                    for _ in 0..rng.below(6) {
                        sim.deliver_one();
                    }
                }
                7 => sim.random_undo(p),
                _ => sim.random_text_edit(p),
            }
        }
        sim.heal();
        sim.assert_converged("random");
        write_fixture(
            &format!("random-{k}"),
            &format!("simulation seed {seed:#x}, {n} peers, {steps} steps"),
            sim.frames,
            seed,
        );
    }

    std::fs::create_dir_all(bulk_path().parent().unwrap()).unwrap();
    write_json(&bulk_path(), &bulk(BULK_DEFAULT));
}

/// Refusals every replica must make alike (contract §2.4, §2.6): hostile
/// paths (separators and dot segments hidden behind a combining mark among
/// them), malformed operations, and operations that wait forever.
fn record_hostile() {
    let (mut sim, f) = new_sim_with_main(6, 2);
    let s = sim.peers[0].insert(f, 0, "abcdef").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    let mut next_attacker = 0xa77a_c000_u64;
    let mut attacker = |sim: &mut Sim| {
        next_attacker += 1;
        sim.attackers.push(next_attacker);
        Id::new(next_attacker, 0)
    };
    for path in hostile_paths() {
        let file = sim.rng.file_id();
        let id = attacker(&mut sim);
        sim.inject(vec![Section::FileMap(vec![FileOp {
            id,
            lamport: 5,
            file,
            kind: FileOpKind::Create {
                kind: FileKind::Text,
                path: path.clone(),
            },
        }])]);
        let id = attacker(&mut sim);
        sim.inject(vec![Section::FileMap(vec![FileOp {
            id,
            lamport: 5,
            file: f,
            kind: FileOpKind::SetPath(path),
        }])]);
    }
    for path in TRICKY_PATHS {
        let id = sim.rng.file_id();
        let s = sim.peers[1]
            .file_op(|m| m.create(id, FileKind::Text, path))
            .unwrap();
        sim.broadcast(1, vec![s]);
    }
    flush(&mut sim);
    sim.assert_converged("hostile-paths");
    write_fixture(
        "hostile-paths",
        "creates and renames to paths contract §2.6 refuses, including `../\u{301}etc` and `a\\\u{301}b` (a separator or dot segment behind a combining mark), next to odd but valid paths",
        sim.frames,
        16,
    );

    // Malformed operations, and ones that can never apply.
    let (mut sim, f) = new_sim_with_main(7, 2);
    let s = sim.peers[0].insert(f, 0, "hello world").unwrap();
    sim.broadcast(0, vec![s]);
    let s = sim.peers[0].delete(f, 0, 2).unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    let vis = sim.peers[0].text(f).unwrap().visible_ids();
    let del_unit = sim.peers[0]
        .text(f)
        .unwrap()
        .log()
        .iter()
        .find_map(|op| match op {
            TextOp::Delete { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    let mut n = 0xbad0_0000_u64;
    let mut fresh = |sim: &mut Sim| {
        n += 1;
        sim.attackers.push(n);
        Id::new(n, 0)
    };
    let text = |f, op| vec![Section::Text(f, vec![op])];
    let file_op = |id, lamport, file, kind| {
        vec![Section::FileMap(vec![FileOp {
            id,
            lamport,
            file,
            kind,
        }])]
    };
    let id = fresh(&mut sim);
    sim.inject(text(
        f,
        TextOp::Insert {
            id,
            origin_left: Some(vis[3]),
            origin_right: Some(vis[1]),
            content: "x".into(),
        },
    ));
    let id = fresh(&mut sim);
    sim.inject(text(
        f,
        TextOp::Insert {
            id,
            origin_left: Some(vis[0]),
            origin_right: Some(vis[0]),
            content: "x".into(),
        },
    ));
    let id = fresh(&mut sim);
    sim.inject(text(
        f,
        TextOp::Delete {
            id,
            target: del_unit,
            len: 1,
        },
    ));
    let id = fresh(&mut sim);
    sim.inject(text(
        f,
        TextOp::Insert {
            id,
            origin_left: Some(del_unit),
            origin_right: None,
            content: "x".into(),
        },
    ));
    let id = fresh(&mut sim);
    sim.inject(file_op(
        id,
        3,
        f,
        FileOpKind::SetBlob(BlobRef {
            sha256: [1; 32],
            bytes: 9,
            media_type: "image/png".into(),
        }),
    ));
    let img = sim.rng.file_id();
    let id = fresh(&mut sim);
    sim.inject(file_op(
        id,
        0,
        img,
        FileOpKind::Create {
            kind: FileKind::Blob,
            path: "fig/a.png".into(),
        },
    ));
    let s = sim.peers[0]
        .file_op(|m| m.create(img, FileKind::Blob, "fig/a.png"))
        .unwrap();
    sim.broadcast(0, vec![s]);
    let id = fresh(&mut sim);
    sim.inject(file_op(
        id,
        9,
        img,
        FileOpKind::SetBlob(BlobRef {
            sha256: [2; 32],
            bytes: 9,
            media_type: "x".repeat(256),
        }),
    ));
    flush(&mut sim);
    sim.assert_converged("rejected-ops");
    // Operations that never apply: they wait (bounded), they are not
    // refused. A dependency on a replica nobody has heard from, a file no
    // one created, and the operation after a refused one.
    let ghost = Id::new(0x6057, 0);
    let mut frames = sim.frames.clone();
    frames.push(update(
        900,
        text(
            f,
            TextOp::Insert {
                id: Id::new(0x6058, 0),
                origin_left: Some(ghost),
                origin_right: None,
                content: "never".into(),
            },
        ),
    ));
    frames.push(update(
        901,
        text(
            f,
            TextOp::Delete {
                id: Id::new(0x6059, 0),
                target: ghost,
                len: 3,
            },
        ),
    ));
    frames.push(update(
        902,
        file_op(
            Id::new(0x605a, 0),
            4,
            FileId([0xee; 16]),
            FileOpKind::SetPath("nowhere.tex".into()),
        ),
    ));
    frames.push(update(
        903,
        text(
            FileId([0xef; 16]),
            TextOp::Insert {
                id: Id::new(0x605b, 0),
                origin_left: None,
                origin_right: None,
                content: "no such file".into(),
            },
        ),
    ));
    let blocked = Id::new(0xbad0_0001, 1); // behind the first refused op
    frames.push(update(
        904,
        text(
            f,
            TextOp::Insert {
                id: blocked,
                origin_left: None,
                origin_right: None,
                content: "blocked".into(),
            },
        ),
    ));
    let mut rng = Rng(17);
    let orders = all_orders(&mut rng, frames.len());
    let json = fixture_json(
        "rejected-ops",
        "malformed operations every replica refuses alike (origins out of order or equal, a delete or origin naming a deletion unit, a blob on a text file, lamport 0, an overlong media type), and five that wait forever",
        &frames,
        orders,
    );
    let (p, rejected) = replay(&frames, &(0..frames.len()).collect::<Vec<_>>());
    assert_eq!(p.pending_len(), 5);
    assert_eq!(rejected.len(), 7);
    write_json(&dir().join("rejected-ops.json"), &json);
}

/// The legacy suites' cases (`adversarial.rs`, `interrupted_delivery.rs`,
/// `id_reuse_divergence.rs`) restated in collab-v1.
fn record_legacy() {
    // adversarial.rs: an insert anchored to a tombstone; the same scalar
    // deleted by two replicas; delete-then-insert at one element.
    let (mut sim, f) = new_sim_with_main(8, 3);
    let s = sim.peers[0].insert(f, 0, "abc").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    let s = sim.peers[1].delete(f, 1, 1).unwrap(); // b
    sim.broadcast(1, vec![s]);
    let s = sim.peers[2].delete(f, 1, 1).unwrap(); // b again
    sim.broadcast(2, vec![s]);
    let s = sim.peers[0].insert(f, 2, "X").unwrap(); // after b, before c
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    let s = sim.peers[1].insert(f, 1, "Y").unwrap(); // next to b's tombstone
    sim.broadcast(1, vec![s]);
    let s = sim.peers[2].delete(f, 0, 1).unwrap();
    sim.broadcast(2, vec![s]);
    let s = sim.peers[0].insert(f, 1, "Z").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    sim.assert_converged("legacy-adversarial");
    write_fixture(
        "legacy-adversarial",
        "adversarial.rs in collab-v1: inserts anchored to a tombstone, a scalar deleted twice concurrently, delete and insert at one element",
        sim.frames,
        18,
    );

    // adversarial.rs: five replicas insert at the identical anchor.
    let (mut sim, f) = new_sim_with_main(9, 5);
    let s = sim.peers[0].insert(f, 0, "[]").unwrap();
    sim.broadcast(0, vec![s]);
    flush(&mut sim);
    for p in 0..5 {
        let s = sim.peers[p].insert(f, 1, &format!("{p}")).unwrap();
        sim.broadcast(p, vec![s]);
    }
    flush(&mut sim);
    sim.assert_converged("legacy-five");
    write_fixture(
        "legacy-five-replicas-one-anchor",
        "adversarial.rs in collab-v1: five replicas insert at the identical anchor; one total order",
        sim.frames,
        19,
    );

    // interrupted_delivery.rs: a peer offline for a backlog of edits from
    // two others catches up out of order (the reversed and shuffled orders).
    let (mut sim, f) = new_sim_with_main(10, 3);
    sim.partition = Some(vec![false, false, true]);
    let mut rng = Rng(20);
    for _ in 0..30 {
        let p = rng.below(2);
        sim.random_text_edit(p);
        while sim.deliver_one() {}
    }
    let _ = f;
    sim.heal();
    sim.assert_converged("legacy-interrupted");
    write_fixture(
        "legacy-interrupted-delivery",
        "interrupted_delivery.rs in collab-v1: a backlog of 30 edits from two peers reaching an offline third in any order",
        sim.frames,
        21,
    );

    // id_reuse_divergence.rs: an id reused with different content is
    // refused, never merged; an identical replay is a duplicate. Which of
    // two conflicting operations wins depends on arrival, so this fixture's
    // orders are forward only (then the whole log again: every op a replay).
    let r = 0x1d_u64;
    let f = FileId([0x1d; 16]);
    let ins = |id: Id, l: Option<Id>, c: &str| TextOp::Insert {
        id,
        origin_left: l,
        origin_right: None,
        content: c.into(),
    };
    let frames = vec![
        update(
            1,
            vec![Section::FileMap(vec![FileOp {
                id: Id::new(r, 0),
                lamport: 1,
                file: f,
                kind: FileOpKind::Create {
                    kind: FileKind::Text,
                    path: "main.tex".into(),
                },
            }])],
        ),
        update(
            2,
            vec![Section::Text(f, vec![ins(Id::new(r, 0), None, "abc")])],
        ),
        // Same id, different scalar.
        update(
            3,
            vec![Section::Text(f, vec![ins(Id::new(r, 0), None, "aXc")])],
        ),
        // A delete reusing an insert's ids.
        update(
            4,
            vec![Section::Text(
                f,
                vec![TextOp::Delete {
                    id: Id::new(r, 1),
                    target: Id::new(r, 0),
                    len: 1,
                }],
            )],
        ),
        update(
            5,
            vec![Section::Text(
                f,
                vec![TextOp::Delete {
                    id: Id::new(r, 3),
                    target: Id::new(r, 1),
                    len: 2,
                }],
            )],
        ),
        // Same delete id, different target.
        update(
            6,
            vec![Section::Text(
                f,
                vec![TextOp::Delete {
                    id: Id::new(r, 4),
                    target: Id::new(r, 0),
                    len: 1,
                }],
            )],
        ),
        // An insert reusing a delete's id.
        update(
            7,
            vec![Section::Text(f, vec![ins(Id::new(r, 3), None, "q")])],
        ),
        // Another replica creating the same file id.
        update(
            8,
            vec![Section::FileMap(vec![FileOp {
                id: Id::new(r + 1, 0),
                lamport: 1,
                file: f,
                kind: FileOpKind::Create {
                    kind: FileKind::Text,
                    path: "other.tex".into(),
                },
            }])],
        ),
        // The same file-map op id reused with a different path.
        update(
            9,
            vec![Section::FileMap(vec![FileOp {
                id: Id::new(r, 0),
                lamport: 1,
                file: f,
                kind: FileOpKind::Create {
                    kind: FileKind::Text,
                    path: "main2.tex".into(),
                },
            }])],
        ),
    ];
    let orders = forward_orders(frames.len());
    let json = fixture_json(
        "legacy-id-reuse",
        "id_reuse_divergence.rs in collab-v1: ids reused with different content (scalar, delete target, insert vs delete, file op, file id) are refused, identical replays are duplicates; forward orders only",
        &frames,
        orders,
    );
    let (_, rejected) = replay(&frames, &(0..frames.len()).collect::<Vec<_>>());
    assert_eq!(rejected.len(), 6, "{rejected:?}");
    assert!(rejected.iter().all(|r| r.2 == "id_conflict"));
    write_json(&dir().join("legacy-id-reuse.json"), &json);
}

fn check_fixture(name: &str, fx: &Json) {
    let frames: Vec<Vec<u8>> = fx
        .get("frames")
        .arr()
        .iter()
        .map(|f| unhex(f.str()))
        .collect();
    for f in &frames {
        let (msg, used) = wire::decode(f).unwrap().unwrap();
        assert_eq!(used, f.len());
        assert_eq!(&wire::encode(&msg), f, "{name}: non-canonical frame");
    }
    let want = fx.get("expected");
    for order in fx.get("orders").arr() {
        let order: Vec<usize> = order.arr().iter().map(|n| n.num() as usize).collect();
        let (p, rejected) = replay(&frames, &order);
        let got = expected(&p, &rejected);
        assert_eq!(
            got.sorted(),
            want.sorted(),
            "{name}: order {:?}...",
            &order[..order.len().min(8)]
        );
    }
}

#[test]
fn fixtures_replay_identically() {
    if std::env::var_os("FLASHTEX_COLLAB_RECORD").is_some() {
        record();
    }
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    names.sort();
    assert!(names.len() >= 23, "fixtures missing: {names:?}");
    for path in names {
        let fx = Json::parse(&std::fs::read_to_string(&path).unwrap());
        check_fixture(&format!("{path:?}"), &fx);
    }
}

/// The committed bulk file is exactly what the oracle generates now, and
/// replays identically in every order. On demand, a larger one.
#[test]
fn bulk_differential_is_current() {
    if let (Some(n), Some(out)) = (
        std::env::var("FLASHTEX_COLLAB_BULK")
            .ok()
            .and_then(|s| s.parse::<u64>().ok()),
        std::env::var_os("FLASHTEX_COLLAB_BULK_OUT"),
    ) {
        write_json(Path::new(&out), &bulk(n));
        eprintln!("wrote {n} bulk scenarios to {out:?}");
        return;
    }
    let committed = std::fs::read_to_string(bulk_path()).unwrap();
    let mut regenerated = String::new();
    bulk(BULK_DEFAULT).write(&mut regenerated, 0);
    regenerated.push('\n');
    assert!(
        committed == regenerated,
        "bulk-200.json is stale: re-record with FLASHTEX_COLLAB_RECORD=1"
    );
    let parsed = Json::parse(&committed);
    for sc in parsed.get("scenarios").arr() {
        check_fixture(sc.get("name").str(), sc);
    }
}
