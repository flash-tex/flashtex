//! `collab-v1` edge-case unit tests: the deterministic small cases the
//! simulation and fixture suites only reach by chance, if at all —
//! frame-boundary decode behaviour, file-map last-writer-wins, diff
//! trimming, relative-position resolution, undo basics.

use flashtex_collaboration_core::v1::wire::{self, Message};
use flashtex_collaboration_core::v1::{
    Assoc, DocRef, FileId, FileKind, FileMap, FileOp, FileOpKind, Id, Project, RelativePosition,
    Section, StateVector, TextDoc, TextOp, UndoManager,
};

#[test]
fn wire_decode_waits_for_a_full_frame_and_stops_at_its_end() {
    let first = Message::Update {
        seq: 1,
        sections: vec![],
    };
    let f = FileId([6; 16]);
    let mut td = TextDoc::new(1);
    let second = Message::Update {
        seq: 2,
        sections: vec![Section::Text(f, vec![td.insert(0, "hi").unwrap()])],
    };
    let a = wire::encode(&first);
    let b = wire::encode(&second);

    // Nothing complete yet: every strict prefix waits, including a bare
    // header that already names its length.
    assert_eq!(wire::decode(&[]), Ok(None));
    assert_eq!(wire::decode(&a[..1]), Ok(None));
    assert_eq!(wire::decode(&a[..3]), Ok(None));
    assert_eq!(wire::decode(&a[..4]), Ok(None));
    assert_eq!(wire::decode(&a[..a.len() - 1]), Ok(None));

    // A header naming exactly MAX_FRAME is not too large on its own.
    assert_eq!(wire::decode(&[0, 0, 0, 0x01]), Ok(None));

    // Trailing bytes are left for the next call, not an error.
    let both = [a.clone(), b.clone()].concat();
    let (msg, used) = wire::decode(&both).unwrap().unwrap();
    assert_eq!(msg, first);
    assert_eq!(used, a.len());
    let (msg2, used2) = wire::decode(&both[used..]).unwrap().unwrap();
    assert_eq!(msg2, second);
    assert_eq!(used2, b.len());
}

#[test]
fn file_map_registers_keep_the_highest_stamp_whichever_arrives_first() {
    let f = FileId([1; 16]);
    let op = |id, lamport, kind| FileOp {
        id,
        lamport,
        file: f,
        kind,
    };
    let mut m = FileMap::new(1);
    m.create(f, FileKind::Text, "a.tex").unwrap();

    // A stale rename (lower lamport, later counter) applies but changes nothing.
    m.apply(&op(
        Id::new(2, 0),
        5,
        FileOpKind::SetPath("winner.tex".into()),
    ))
    .unwrap();
    m.apply(&op(
        Id::new(2, 1),
        3,
        FileOpKind::SetPath("stale.tex".into()),
    ))
    .unwrap();
    assert_eq!(m.files()[0].path, "winner.tex");

    // Equal lamports: the higher replica wins, in either arrival order.
    m.apply(&op(
        Id::new(3, 0),
        5,
        FileOpKind::SetPath("tie-high.tex".into()),
    ))
    .unwrap();
    assert_eq!(m.files()[0].path, "tie-high.tex");
    let mut n = FileMap::new(1);
    n.create(f, FileKind::Text, "a.tex").unwrap();
    n.apply(&op(
        Id::new(3, 0),
        5,
        FileOpKind::SetPath("tie-high.tex".into()),
    ))
    .unwrap();
    n.apply(&op(
        Id::new(2, 0),
        5,
        FileOpKind::SetPath("winner.tex".into()),
    ))
    .unwrap();
    assert_eq!(n.files()[0].path, "tie-high.tex");

    // The deleted flag is a register too: stale writes lose both ways.
    m.apply(&op(Id::new(4, 0), 9, FileOpKind::SetDeleted(true)))
        .unwrap();
    assert!(m.files().is_empty());
    m.apply(&op(Id::new(4, 1), 2, FileOpKind::SetDeleted(false)))
        .unwrap();
    assert!(m.files().is_empty());
    m.apply(&op(Id::new(5, 0), 20, FileOpKind::SetDeleted(false)))
        .unwrap();
    assert_eq!(m.files().len(), 1);

    // The lamport clock followed the highest stamp seen, even a remote one.
    let local = m.rename(f, "local.tex").unwrap();
    assert_eq!(local.lamport, 21);
    assert_eq!(m.files()[0].path, "local.tex");
}

#[test]
fn text_diff_trims_partially_covered_runs_unit_by_unit() {
    let mut d = TextDoc::new(1);
    let op = d.insert(0, "abcdef").unwrap();
    assert_eq!(d.diff(&StateVector::default()), vec![op.clone()]);
    assert_eq!(d.diff(&d.state_vector()), Vec::new());

    let mut sv = StateVector::default();
    sv.set(1, 2);
    let trimmed = d.diff(&sv);
    assert_eq!(trimmed, vec![op.without_prefix(2)]);
    match &trimmed[0] {
        TextOp::Insert {
            id,
            origin_left,
            origin_right,
            content,
        } => {
            assert_eq!(*id, Id::new(1, 2));
            assert_eq!(*origin_left, Some(Id::new(1, 1)));
            assert_eq!(*origin_right, None);
            assert_eq!(content, "cdef");
        }
        TextOp::Delete { .. } => panic!("an insert trims to an insert"),
    }

    // Trimming counts scalars, not bytes.
    let wide = d.insert(6, "é😀").unwrap();
    assert_eq!(wide.without_prefix(1).len(), 1);
    match wide.without_prefix(1) {
        TextOp::Insert { id, content, .. } => {
            assert_eq!(id, Id::new(1, 7));
            assert_eq!(content, "😀");
        }
        TextOp::Delete { .. } => panic!("an insert trims to an insert"),
    }

    // Deletes trim the same way: id and target both move.
    let dels = d.delete(0, 2);
    assert_eq!(dels.len(), 1);
    let mut sv = StateVector::default();
    sv.set(1, dels[0].id().counter + 1);
    let trimmed = d.diff(&sv);
    assert_eq!(trimmed.len(), 1);
    assert_eq!(
        trimmed[0],
        TextOp::Delete {
            id: dels[0].id().offset(1),
            target: match &dels[0] {
                TextOp::Delete { target, .. } => target.offset(1),
                TextOp::Insert { .. } => unreachable!(),
            },
            len: 1,
        }
    );
}

#[test]
fn project_diff_sends_the_file_map_first_and_only_what_is_missing() {
    let f = FileId([8; 16]);
    let mut a = Project::new(1);
    a.file_op(|m| m.create(f, FileKind::Text, "n.tex")).unwrap();
    a.insert(f, 0, "hi").unwrap();

    // A peer that has nothing gets the file map first (it creates the texts).
    let all = a.diff(&[]);
    assert_eq!(all.len(), 2);
    assert!(matches!(all[0], Section::FileMap(_)));
    assert!(matches!(all[1], Section::Text(id, _) if id == f));

    // A current peer gets nothing.
    assert!(a.diff(&a.state_vectors()).is_empty());

    // A peer that has only the file map gets only the text.
    let file_only = vec![(DocRef::FileMap, a.files().state_vector())];
    let rest = a.diff(&file_only);
    assert_eq!(rest.len(), 1);
    assert!(matches!(rest[0], Section::Text(id, _) if id == f));

    // State for an unknown file suppresses nothing.
    let mut sv = StateVector::default();
    sv.set(1, u64::MAX / 2);
    let ghost = vec![(DocRef::Text(FileId([9; 16])), sv)];
    assert_eq!(a.diff(&ghost).len(), 2);
}

#[test]
fn relative_positions_resolve_edges_without_edits() {
    let mut d = TextDoc::new(1);
    // Empty document: both associations anchor nowhere.
    assert_eq!(d.relative_position(0, Assoc::After).anchor, None);
    assert_eq!(d.relative_position(0, Assoc::Before).anchor, None);
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: None,
            assoc: Assoc::After
        }),
        Some(0)
    );
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: None,
            assoc: Assoc::Before
        }),
        Some(0)
    );

    d.insert(0, "abc").unwrap();
    // Document ends anchor nowhere and resolve to the ends.
    assert_eq!(d.relative_position(0, Assoc::After).anchor, None);
    assert_eq!(d.relative_position(3, Assoc::Before).anchor, None);
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: None,
            assoc: Assoc::After
        }),
        Some(0)
    );
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: None,
            assoc: Assoc::Before
        }),
        Some(3)
    );
    // Document ends also resolve through their neighbour's anchor.
    assert_eq!(d.resolve(d.relative_position(3, Assoc::After)), Some(3));
    assert_eq!(d.resolve(d.relative_position(0, Assoc::Before)), Some(0));
    assert_eq!(d.resolve(d.relative_position(1, Assoc::Before)), Some(1));
    assert_eq!(d.resolve(d.relative_position(1, Assoc::After)), Some(1));

    // Unknown anchors never resolve.
    for assoc in [Assoc::Before, Assoc::After] {
        assert_eq!(
            d.resolve(RelativePosition {
                anchor: Some(Id::new(9, 9)),
                assoc
            }),
            None
        );
    }

    // A deleted anchor resolves to where it was, from either side.
    let b = Id::new(1, 1);
    d.delete(1, 1).pop().unwrap();
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: Some(b),
            assoc: Assoc::Before
        }),
        Some(1)
    );
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: Some(b),
            assoc: Assoc::After
        }),
        Some(1)
    );
    assert_eq!(
        d.resolve(RelativePosition {
            anchor: Some(Id::new(1, 0)),
            assoc: Assoc::After
        }),
        Some(1)
    );
}

#[test]
fn undo_redo_single_steps_and_a_new_edit_clears_redo() {
    let mut d = TextDoc::new(1);
    let mut u = UndoManager::new();
    // Nothing recorded: nothing to walk.
    assert!(u.undo(&mut d).is_empty());
    assert!(u.redo(&mut d).is_empty());
    // Recording no operations is not a step.
    u.record(&[]);
    assert!(u.undo(&mut d).is_empty());

    let op = d.insert(0, "hello").unwrap();
    u.record(&[op]);
    assert_eq!(d.text(), "hello");
    assert!(!u.undo(&mut d).is_empty());
    assert_eq!(d.text(), "");
    assert!(!u.redo(&mut d).is_empty());
    assert_eq!(d.text(), "hello");

    let dels = d.delete(0, 2);
    u.record(&dels);
    assert_eq!(d.text(), "llo");
    u.undo(&mut d);
    assert_eq!(d.text(), "hello");
    u.redo(&mut d);
    assert_eq!(d.text(), "llo");

    // A new edit after an undo drops the redo stack.
    u.undo(&mut d);
    assert_eq!(d.text(), "hello");
    let op = d.insert(5, "!").unwrap();
    u.record(&[op]);
    assert_eq!(d.text(), "hello!");
    assert!(u.redo(&mut d).is_empty());
}
