//! Differential fixtures (proposal §7.2): `collab-v1` op logs that both
//! implementations replay. Each fixture is a list of `update` frames, some
//! delivery orders (reordered, duplicated), and the state a fresh replica
//! must reach: every live file's materialised path and text, and the
//! structure digests. The Swift core (`FlashTeXCollabCoreTests`) replays
//! every file here, including the `swift-*.json` ones it recorded itself,
//! and this test replays them all with the oracle. Either side drifting
//! fails both.
//!
//! Regenerate the oracle's fixtures with
//! `FLASHTEX_COLLAB_RECORD=1 cargo test -p flashtex-collaboration-core --test v1_fixtures`.

mod common;

use std::path::{Path, PathBuf};

use common::{hex, unhex, Json, Rng, Sim};
use flashtex_collaboration_core::v1::wire::{self, Message};
use flashtex_collaboration_core::v1::{BlobRef, FileId, FileKind, Project};

const OBSERVER: u64 = u64::MAX;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collab-v1")
}

fn replay(frames: &[Vec<u8>], order: &[usize]) -> Project {
    let mut p = Project::new(OBSERVER);
    for &i in order {
        let (msg, _) = wire::decode(&frames[i]).unwrap().unwrap();
        let Message::Update { sections, .. } = msg else {
            panic!("fixture frames are updates")
        };
        let errors = p.receive(&sections);
        assert!(errors.is_empty(), "{errors:?}");
    }
    assert_eq!(p.pending_len(), 0, "ops left pending");
    p
}

fn expected(p: &Project) -> Json {
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
    ])
}

fn orders(rng: &mut Rng, n: usize) -> Vec<Vec<usize>> {
    let fwd: Vec<usize> = (0..n).collect();
    let rev: Vec<usize> = (0..n).rev().collect();
    let mut shuffled = fwd.clone();
    for i in (1..n).rev() {
        shuffled.swap(i, rng.below(i + 1));
    }
    for _ in 0..n / 4 {
        let i = rng.below(n);
        shuffled.push(i); // duplicates
    }
    vec![fwd, rev, shuffled]
}

fn write_fixture(name: &str, description: &str, frames: Vec<Vec<u8>>, seed: u64) {
    let mut rng = Rng(seed);
    let orders = orders(&mut rng, frames.len());
    let p = replay(&frames, &orders[0]);
    let json = Json::Obj(vec![
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
        ("expected".into(), expected(&p)),
    ]);
    let mut out = String::new();
    json.write(&mut out, 0);
    out.push('\n');
    std::fs::write(dir().join(format!("{name}.json")), out).unwrap();
}

fn new_sim_with_main(seed: u64, n: usize) -> (Sim, FileId) {
    let mut sim = Sim::new(seed, n);
    let main = sim.rng.file_id();
    let s = sim.peers[0]
        .file_op(|m| m.create(main, FileKind::Text, "main.tex"))
        .unwrap();
    sim.broadcast(0, vec![s]);
    while !sim.queue.is_empty() {
        let m = sim.queue.remove(0);
        sim.receive_frame(m.to, &m.frame);
    }
    (sim, main)
}

fn flush(sim: &mut Sim) {
    while !sim.queue.is_empty() {
        let m = sim.queue.remove(0);
        sim.receive_frame(m.to, &m.frame);
    }
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
    assert!(names.len() >= 12, "fixtures missing: {names:?}");
    for path in names {
        let fx = Json::parse(&std::fs::read_to_string(&path).unwrap());
        let frames: Vec<Vec<u8>> = fx
            .get("frames")
            .arr()
            .iter()
            .map(|f| unhex(f.str()))
            .collect();
        for f in &frames {
            let (msg, used) = wire::decode(f).unwrap().unwrap();
            assert_eq!(used, f.len());
            assert_eq!(&wire::encode(&msg), f, "{path:?}: non-canonical frame");
        }
        let want = fx.get("expected");
        for order in fx.get("orders").arr() {
            let order: Vec<usize> = order.arr().iter().map(|n| n.num() as usize).collect();
            let p = replay(&frames, &order);
            let got = expected(&p);
            assert_eq!(got.sorted(), want.sorted(), "{path:?}: order {order:?}");
        }
    }
}
