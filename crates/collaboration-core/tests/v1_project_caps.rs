//! collab-v1 §2.4/§2.6 project caps: a peer (a hostile hub, for a guest)
//! cannot grow a replica's memory without bound by creating files or by
//! filling many documents, each under the per-document limit. The Swift
//! core's `ProjectCapsTests` checks the same cases.
use flashtex_collaboration_core::v1::{
    limits, CollabError, FileId, FileKind, Project, ProjectCaps, Section,
};

fn fid(n: u32) -> FileId {
    let mut b = [0u8; 16];
    b[..4].copy_from_slice(&n.to_be_bytes());
    FileId(b)
}

fn uncapped(replica: u64) -> Project {
    Project::with_caps(
        replica,
        ProjectCaps {
            max_files: usize::MAX,
            max_text_bytes: usize::MAX,
        },
    )
}

fn kinds(errors: &[flashtex_collaboration_core::v1::ProjectError]) -> Vec<&'static str> {
    errors.iter().map(|e| e.error.kind_name()).collect()
}

#[test]
fn the_contract_caps_are_these() {
    assert_eq!(limits::MAX_FILES, 4096);
    assert_eq!(limits::MAX_PROJECT_TEXT_BYTES, 64 * 1024 * 1024);
    let d = ProjectCaps::default();
    assert_eq!(d.max_files, limits::MAX_FILES);
    assert_eq!(d.max_text_bytes, limits::MAX_PROJECT_TEXT_BYTES);
}

#[test]
fn a_create_past_the_file_cap_is_refused_remotely_and_locally() {
    // The real cap: 4096 creates are cheap.
    let mut hostile = uncapped(2);
    let mut sections = Vec::new();
    for n in 0..=limits::MAX_FILES as u32 {
        sections.push(
            hostile
                .file_op(|m| m.create(fid(n), FileKind::Text, &format!("f{n}.tex")))
                .unwrap(),
        );
    }
    let mut guest = Project::new(1);
    let errors = guest.receive(&sections);
    assert_eq!(kinds(&errors), vec!["project_full"]);
    assert_eq!(errors[0].op.counter, limits::MAX_FILES as u64);
    assert!(!guest.files().contains(fid(limits::MAX_FILES as u32)));
    assert!(guest.text(fid(limits::MAX_FILES as u32 - 1)).is_some());
    // A local create at the cap is refused the same way, and changes nothing.
    let before = guest.digest();
    assert_eq!(
        guest
            .file_op(|m| m.create(fid(9_999_999), FileKind::Blob, "x.png"))
            .unwrap_err(),
        CollabError::ProjectFull
    );
    assert_eq!(guest.digest(), before);
    // Redelivery is still a duplicate, never a refusal.
    assert!(guest.receive(&sections[..10]).is_empty());
}

#[test]
fn text_past_the_project_byte_cap_is_refused_remotely_and_locally() {
    let caps = ProjectCaps {
        max_files: 16,
        max_text_bytes: 100,
    };
    let mut hostile = uncapped(2);
    let mut sections: Vec<Section> = Vec::new();
    for n in 0..3 {
        sections.push(
            hostile
                .file_op(|m| m.create(fid(n), FileKind::Text, &format!("f{n}.tex")))
                .unwrap(),
        );
    }
    // 40 + 40 bytes fit; the third 40 would make 120 > 100.
    for n in 0..3 {
        sections.push(hostile.insert(fid(n), 0, &"é".repeat(20)).unwrap());
    }
    let mut guest = Project::with_caps(1, caps);
    let errors = guest.receive(&sections);
    assert_eq!(kinds(&errors), vec!["project_full"]);
    assert_eq!(guest.text(fid(2)).unwrap().text(), "");
    // Twenty bytes are left: a local 21-byte insert is refused, 20 fit.
    assert!(guest.insert(fid(2), 0, &"x".repeat(21)).is_none());
    assert!(guest.insert(fid(2), 0, &"x".repeat(20)).is_some());
    assert!(guest.insert(fid(0), 0, "y").is_none());
    // Deleting frees nothing (tombstones are stored, §2.4).
    assert!(guest.delete(fid(0), 0, 5).is_some());
    assert!(guest.insert(fid(0), 0, "y").is_none());
}

#[test]
fn an_uncapped_project_keeps_the_per_document_limit() {
    let caps = ProjectCaps {
        max_files: 16,
        max_text_bytes: 10,
    };
    let mut hostile = uncapped(2);
    let create = hostile
        .file_op(|m| m.create(fid(0), FileKind::Text, "a.tex"))
        .unwrap();
    let big = hostile
        .insert(fid(0), 0, &"x".repeat(limits::MAX_DOCUMENT_BYTES + 1))
        .is_none();
    // An uncapped project still keeps the per-document limit.
    assert!(big);
    let mut guest = Project::with_caps(1, caps);
    assert!(guest.receive(&[create]).is_empty());
    let over = hostile.insert(fid(0), 0, &"x".repeat(11)).unwrap();
    assert_eq!(kinds(&guest.receive(&[over])), vec!["project_full"]);
}
