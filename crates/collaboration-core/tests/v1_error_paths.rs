//! `collab-v1` error-path unit tests: hostile-input rejections of the wire
//! codec, the file map, text apply, and project receive — branches the
//! simulation and fixture suites only reach by chance, if at all.

use flashtex_collaboration_core::v1::{
    filemap::is_valid_path,
    wire::{self, Message},
    Applied, BlobRef, CollabError, DocRef, FileId, FileKind, FileMap, FileOp, FileOpKind, Id,
    Project, Section, StateVector, TextDoc, TextOp,
};

fn round_trip(msg: &Message) -> Message {
    let bytes = wire::encode(msg);
    let (back, used) = wire::decode(&bytes).unwrap().unwrap();
    assert_eq!(used, bytes.len(), "one frame consumes exactly its bytes");
    back
}

fn frame(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(body.len() + 5);
    v.extend_from_slice(&((body.len() + 1) as u32).to_le_bytes());
    v.push(kind);
    v.extend_from_slice(body);
    v
}

#[test]
fn wire_messages_round_trip_through_encode_decode() {
    let f = FileId([7; 16]);
    let mut fm = FileMap::new(1);
    let create = fm.create(f, FileKind::Text, "main.tex").unwrap();
    let mut td = TextDoc::new(1);
    let ins = td.insert(0, "héllo 😀").unwrap();
    let del = td.delete(0, 1).pop().unwrap();
    let mut sv = StateVector::default();
    sv.set(1, 4);
    sv.set(9, 1);
    let cases = [
        Message::SyncRequest(vec![
            (DocRef::FileMap, sv),
            (DocRef::Text(f), StateVector::default()),
        ]),
        Message::SyncReply(vec![
            Section::FileMap(vec![create]),
            Section::Text(f, vec![ins, del]),
        ]),
        Message::Update {
            seq: 0,
            sections: vec![],
        },
        Message::Update {
            seq: u64::MAX,
            sections: vec![Section::Text(f, vec![td.insert(0, "y").unwrap()])],
        },
        Message::BlobChunk {
            sha256: [3; 32],
            offset: 1 << 40,
            data: vec![1, 2, 3],
        },
        Message::PreviewFrame(vec![0xff, 0, 0x80]),
        Message::Json {
            kind: wire::kind::JOIN,
            body: b"{}".to_vec(),
        },
        Message::Unknown {
            kind: 0x7f,
            body: vec![9],
        },
    ];
    for m in &cases {
        assert_eq!(&round_trip(m), m);
    }
}

#[test]
fn wire_decode_rejects_hostile_frames_with_typed_errors() {
    use wire::{kind, WireError};
    let dec = wire::decode;
    assert_eq!(
        dec(&frame(kind::SYNC_REQUEST, &[1, 0])),
        Err(WireError::Truncated)
    );
    assert_eq!(
        dec(&frame(kind::SYNC_REQUEST, &[1, 9])),
        Err(WireError::BadTag(9))
    );
    // One Text section for file 0xAA.. with caller-supplied op bytes.
    let text_frame = |op: &[u8]| {
        let mut body = vec![1, 1, 1];
        body.extend_from_slice(&[0xaa; 16]);
        body.push(1);
        body.extend_from_slice(op);
        frame(kind::UPDATE, &body)
    };
    assert_eq!(dec(&text_frame(&[7])), Err(WireError::BadTag(7)));
    assert_eq!(dec(&text_frame(&[1, 7, 0, 2])), Err(WireError::BadTag(2)));
    assert_eq!(
        dec(&text_frame(&[1, 7, 0, 0, 0, 2, 0xff, 0xfe])),
        Err(WireError::BadUtf8)
    );
    assert_eq!(
        dec(&text_frame(&[1, 7, 0, 0, 0, 0])),
        Err(WireError::Invalid("empty insert"))
    );
    assert_eq!(
        dec(&text_frame(&[2, 7, 0, 7, 0, 0])),
        Err(WireError::Invalid("empty delete"))
    );
    // One FileMap section with caller-supplied op bytes.
    let file_frame = |op: &[u8]| {
        let mut body = vec![1, 1, 0, 1, 1, 0, 1];
        body.extend_from_slice(&[0xbb; 16]);
        body.extend_from_slice(op);
        frame(kind::UPDATE, &body)
    };
    assert_eq!(dec(&file_frame(&[9])), Err(WireError::BadTag(9)));
    assert_eq!(dec(&file_frame(&[1, 5])), Err(WireError::BadTag(5)));
    assert_eq!(dec(&file_frame(&[4, 2])), Err(WireError::BadTag(2)));
    assert_eq!(
        dec(&frame(kind::UPDATE, &[1, 0, 0])),
        Err(WireError::TrailingBytes)
    );
}

#[test]
fn file_paths_are_validated_by_shape_not_just_length() {
    let long_ok = "a".repeat(1024);
    let long_bad = "a".repeat(1025);
    let cases: Vec<(&str, bool)> = vec![
        ("", false),
        ("main.tex", true),
        ("a/b/c.tex", true),
        ("/main.tex", false),
        ("main.tex/", false),
        ("a//b", false),
        (".", false),
        ("..", false),
        ("a/./b", false),
        ("a/../b", false),
        (".hidden", true),
        ("a b.tex", true),
        ("a\\b", false),
        ("a\0b", false),
        (&long_ok, true),
        (&long_bad, false),
    ];
    for (p, want) in cases {
        assert_eq!(is_valid_path(p), want, "{p:?}");
    }
}

#[test]
fn file_map_rejects_gaps_replays_and_invalid_ops() {
    let f1 = FileId([1; 16]);
    let ghost = FileId([9; 16]);
    let mut m = FileMap::new(1);
    let op = |id, lamport, file, kind| FileOp {
        id,
        lamport,
        file,
        kind,
    };
    let renamed = FileOpKind::SetPath("x".into());
    assert_eq!(
        m.apply(&op(Id::new(9, 0), 1, ghost, renamed)),
        Err(CollabError::UnknownFile(ghost))
    );
    assert_eq!(
        m.apply(&op(Id::new(9, 0), 0, ghost, FileOpKind::SetDeleted(true))),
        Err(CollabError::Malformed("lamport 0"))
    );
    assert_eq!(
        m.apply(&op(Id::new(9, 5), 5, ghost, FileOpKind::SetDeleted(true))),
        Err(CollabError::MissingDependency(Id::new(9, 0)))
    );
    let abs = FileOpKind::Create {
        kind: FileKind::Text,
        path: "/abs".into(),
    };
    assert_eq!(
        m.apply(&op(Id::new(9, 0), 1, f1, abs)),
        Err(CollabError::Malformed("invalid path"))
    );
    let c1 = m.create(f1, FileKind::Text, "a.tex").unwrap();
    assert_eq!(m.apply(&c1), Ok(Applied::Duplicate));
    let evil = FileOpKind::SetPath("evil".into());
    assert_eq!(
        m.apply(&op(c1.id, 99, f1, evil)),
        Err(CollabError::IdConflict(c1.id))
    );
    let dup = FileOpKind::Create {
        kind: FileKind::Text,
        path: "b.tex".into(),
    };
    // A second create of a file id merges (collab-v1 §2.6), and a blob on a
    // text file is stored but not shown: both apply, so neither blocks its
    // replica (`v1_conflicting_creates.rs`). This replica makes neither.
    assert_eq!(m.apply(&op(Id::new(2, 0), 2, f1, dup)), Ok(Applied::New));
    assert_eq!(
        m.create(f1, FileKind::Blob, "c.bin"),
        Err(CollabError::IdConflict(Id::new(1, 1)))
    );
    let blob = BlobRef {
        sha256: [0; 32],
        bytes: 1,
        media_type: "image/png".into(),
    };
    assert_eq!(
        m.apply(&op(Id::new(2, 1), 2, f1, FileOpKind::SetBlob(blob.clone()))),
        Ok(Applied::New)
    );
    assert_eq!(
        m.set_blob(f1, blob),
        Err(CollabError::Malformed("blob on a text file"))
    );
    let fb = FileId([3; 16]);
    m.create(fb, FileKind::Blob, "fig.bin").unwrap();
    let big = BlobRef {
        sha256: [0; 32],
        bytes: 4,
        media_type: "m".repeat(256),
    };
    assert_eq!(
        m.set_blob(fb, big),
        Err(CollabError::Malformed("media type too long"))
    );
    let ok = BlobRef {
        sha256: [0; 32],
        bytes: 4,
        media_type: "m".repeat(255),
    };
    m.set_blob(fb, ok).unwrap();
}

#[test]
fn text_apply_rejects_empty_gapped_and_misanchored_ops() {
    let mut d = TextDoc::new(1);
    assert!(d.insert(0, "").is_none());
    assert!(d.insert(1, "x").is_none());
    assert!(d.delete(0, 1).is_empty());
    d.insert(0, "ab").unwrap();
    assert!(d.delete(0, 0).is_empty());
    assert_eq!(d.raw_len(), 2);
    assert_eq!(d.is_deleted(Id::new(9, 9)), None);
    let ins = |id, left, right, content: &str| TextOp::Insert {
        id,
        origin_left: left,
        origin_right: right,
        content: content.into(),
    };
    assert_eq!(
        d.apply(&ins(Id::new(2, 0), None, None, "")),
        Err(CollabError::Malformed("empty operation"))
    );
    let empty_del = TextOp::Delete {
        id: Id::new(2, 0),
        target: Id::new(1, 0),
        len: 0,
    };
    assert_eq!(
        d.apply(&empty_del),
        Err(CollabError::Malformed("empty operation"))
    );
    assert_eq!(
        d.apply(&ins(Id::new(2, 9), None, None, "x")),
        Err(CollabError::MissingDependency(Id::new(2, 0)))
    );
    assert_eq!(
        d.apply(&ins(Id::new(2, 0), Some(Id::new(8, 8)), None, "x")),
        Err(CollabError::MissingDependency(Id::new(8, 8)))
    );
    assert_eq!(d.text(), "ab");
}

#[test]
fn project_does_not_park_the_same_op_twice() {
    // Resending an op for an unknown file must not take a second pending slot,
    // or a peer could fill the pending buffer by repeating one op.
    let f = FileId([4; 16]);
    let mut a = Project::new(1);
    let mut b = Project::new(2);
    b.file_op(|m| m.create(f, FileKind::Text, "n.tex")).unwrap();
    let t = b.insert(f, 0, "hi").unwrap();
    assert!(a.receive(std::slice::from_ref(&t)).is_empty());
    assert_eq!(a.pending_len(), 1);
    assert!(a.receive(std::slice::from_ref(&t)).is_empty());
    assert_eq!(a.pending_len(), 1);
}
