//! Two creates of one file id (#2059 findings 2 and 3). Random 128-bit ids
//! make an honest collision negligible, so only a faulty or hostile replica
//! makes one. The file map must still converge, and neither create may
//! leave its replica's later operations waiting for good.

use flashtex_collaboration_core::v1::{
    BlobRef, DocRef, FileId, FileKind, FileOp, FileOpKind, Id, Project, Section,
};

const A: u64 = 0xa;
const B: u64 = 0xb;

fn op(replica: u64, counter: u64, lamport: u64, file: FileId, kind: FileOpKind) -> Section {
    Section::FileMap(vec![FileOp {
        id: Id::new(replica, counter),
        lamport,
        file,
        kind,
    }])
}

fn create(replica: u64, lamport: u64, file: FileId, kind: FileKind, path: &str) -> Section {
    op(
        replica,
        0,
        lamport,
        file,
        FileOpKind::Create {
            kind,
            path: path.into(),
        },
    )
}

/// Every section delivered to a fresh observer in `order`; returns it and
/// the refusals it reported.
fn replay(sections: &[Section], order: &[usize]) -> (Project, Vec<String>) {
    let mut p = Project::new(0x0b5e);
    let mut refused = Vec::new();
    for &i in order {
        for e in p.receive(std::slice::from_ref(&sections[i])) {
            refused.push(format!("{:?} {:?} {}", e.doc, e.op, e.error.kind_name()));
        }
    }
    (p, refused)
}

/// Delivers in every order that keeps each replica's own operations in
/// counter order and checks that all reach the same state, with nothing
/// refused and nothing left waiting.
fn assert_converges(sections: &[Section], label: &str) -> Project {
    let fwd: Vec<usize> = (0..sections.len()).collect();
    let mut rev = fwd.clone();
    rev.reverse();
    let (p, refused) = replay(sections, &fwd);
    assert_eq!(refused, Vec::<String>::new(), "{label}: forward refusals");
    assert_eq!(
        p.pending_len(),
        0,
        "{label}: forward left operations waiting"
    );
    // Reversed delivery parks each replica's later operations until their
    // predecessors arrive; the outcome must not depend on it.
    let (q, refused_q) = replay(sections, &rev);
    assert_eq!(
        refused_q,
        Vec::<String>::new(),
        "{label}: reversed refusals"
    );
    assert_eq!(
        q.pending_len(),
        0,
        "{label}: reversed left operations waiting"
    );
    assert_eq!(p.files().files(), q.files().files(), "{label}: file views");
    assert_eq!(
        p.files().digest(),
        q.files().digest(),
        "{label}: file-map digest"
    );
    assert_eq!(p.digest(), q.digest(), "{label}: project digest");
    assert_eq!(
        p.state_vectors(),
        q.state_vectors(),
        "{label}: state vectors"
    );
    p
}

/// Finding 2: B's create collides with A's. Before the fix, whichever
/// arrived second was refused as an id conflict, so that replica's next
/// operations (here a create and a rename of another file) waited for
/// good, and the two orders kept different paths.
#[test]
fn conflicting_creates_converge_and_do_not_block_the_replica() {
    let f = FileId([0x2f; 16]);
    let g = FileId([0x30; 16]);
    let sections = vec![
        create(A, 1, f, FileKind::Text, "a.tex"),
        create(B, 1, f, FileKind::Text, "b.tex"),
        op(
            B,
            1,
            2,
            g,
            FileOpKind::Create {
                kind: FileKind::Text,
                path: "g.tex".into(),
            },
        ),
        op(B, 2, 3, g, FileOpKind::SetPath("h.tex".into())),
    ];
    let p = assert_converges(&sections, "same kind");
    let paths: Vec<String> = p.files().files().into_iter().map(|e| e.path).collect();
    // The path register's later write, (1, B) over (1, A), wins.
    assert_eq!(paths, ["b.tex", "h.tex"]);
    let sv = p.files().state_vector();
    assert_eq!((sv.get(A), sv.get(B)), (1, 3));
    assert!(p.text(f).is_some() && p.text(g).is_some());
}

/// Finding 3: the creates disagree on the kind, and B sets a blob on the
/// file it made as a blob. Before the fix, a replica that saw A's text
/// create first refused B's create and B's `set_blob`, while one that saw
/// B's first took both; they diverged for good.
#[test]
fn a_kind_conflict_keeps_the_earliest_create_and_still_takes_set_blob() {
    let f = FileId([0x3f; 16]);
    let blob = BlobRef {
        sha256: [7; 32],
        bytes: 3,
        media_type: "image/png".into(),
    };
    for (a_lamport, b_lamport, kind) in [(1, 2, FileKind::Text), (2, 1, FileKind::Blob)] {
        let sections = vec![
            create(A, a_lamport, f, FileKind::Text, "x.tex"),
            create(B, b_lamport, f, FileKind::Blob, "x.png"),
            op(B, 1, b_lamport + 1, f, FileOpKind::SetBlob(blob.clone())),
            op(B, 2, b_lamport + 2, f, FileOpKind::SetDeleted(true)),
            op(A, 1, 9, f, FileOpKind::SetDeleted(false)),
        ];
        let p = assert_converges(&sections, &format!("{kind:?} first"));
        assert_eq!(p.files().kind(f), Some(kind));
        let view = p.files().files();
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].kind, kind);
        // A text file shows no blob, even when one was set on it.
        assert_eq!(view[0].blob.is_some(), kind == FileKind::Blob);
        // A's text create made a text document on every replica alike.
        assert!(p.text(f).is_some());
        assert!(p.state_vectors().iter().any(|(d, _)| *d == DocRef::Text(f)));
    }
}
