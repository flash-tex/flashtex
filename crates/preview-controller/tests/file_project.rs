use flashtex_preview_controller::file_project::{DiskState, FileProject};
#[test]
fn disk_import_reopen_and_external_changes_preserve_durable_source() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "\\input{chapter}").unwrap();
    std::fs::write(root.path().join("chapter.tex"), "original").unwrap();
    let (files, mut controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    assert_eq!(controller.index().snapshot().documents.len(), 2);
    assert!(matches!(
        files.inspect(&controller, "chapter.tex").unwrap(),
        DiskState::MatchesSource { .. }
    ));
    let old = controller.document("chapter.tex").unwrap().clone();
    controller
        .replace_document(
            "chapter.tex",
            old.revision,
            &old.source_sha256,
            "durable unsaved".into(),
        )
        .unwrap();
    std::fs::write(root.path().join("chapter.tex"), "external edit").unwrap();
    assert!(matches!(
        files.inspect(&controller, "chapter.tex").unwrap(),
        DiskState::DiffersFromSource { .. }
    ));
    assert!(files
        .export(
            &controller,
            "chapter.tex",
            2,
            &controller.document("chapter.tex").unwrap().source_sha256,
            Some(&old.source_sha256)
        )
        .is_err());
    drop(controller);
    std::fs::remove_file(root.path().join("main.tex")).unwrap();
    let (files, controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    assert_eq!(
        controller.document("chapter.tex").unwrap().text,
        "durable unsaved"
    );
    assert_eq!(
        files.inspect(&controller, "main.tex").unwrap(),
        DiskState::Missing
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("chapter.tex")).unwrap(),
        "external edit"
    );
}
#[test]
fn same_project_id_under_different_roots_has_separate_ledgers() {
    let private = tempfile::tempdir().unwrap();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    std::fs::write(a.path().join("main.tex"), "first").unwrap();
    std::fs::write(b.path().join("main.tex"), "second").unwrap();
    let (_, first) = FileProject::open(a.path(), private.path(), "p", "main.tex").unwrap();
    let (_, second) = FileProject::open(b.path(), private.path(), "p", "main.tex").unwrap();
    assert_eq!(first.document("main.tex").unwrap().text, "first");
    assert_eq!(second.document("main.tex").unwrap().text, "second");
}
#[test]
#[cfg(unix)]
fn outside_symlink_entry_is_refused_before_shared_graph_discovery() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("external.tex"), "outside").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("external.tex"),
        root.path().join("main.tex"),
    )
    .unwrap();
    assert!(FileProject::open(root.path(), private.path(), "p", "main.tex").is_err());
}

#[test]
fn rooted_export_checks_source_disk_and_new_file_expectations() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "old").unwrap();
    let (files, mut controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    let old = controller.document("main.tex").unwrap().clone();
    controller
        .replace_document("main.tex", old.revision, &old.source_sha256, "new α".into())
        .unwrap();
    let new = controller.document("main.tex").unwrap().clone();
    assert!(files
        .export(
            &controller,
            "main.tex",
            old.revision,
            &old.source_sha256,
            Some(&old.source_sha256)
        )
        .is_err());
    assert!(files
        .export(
            &controller,
            "main.tex",
            new.revision,
            &new.source_sha256,
            None
        )
        .is_err());
    let receipt = files
        .export(
            &controller,
            "main.tex",
            new.revision,
            &new.source_sha256,
            Some(&old.source_sha256),
        )
        .unwrap();
    assert_eq!(receipt.sha256_hex(), new.source_sha256);
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.tex")).unwrap(),
        new.text
    );
    std::fs::remove_file(root.path().join("main.tex")).unwrap();
    assert!(files
        .export(
            &controller,
            "main.tex",
            new.revision,
            &new.source_sha256,
            Some(&new.source_sha256)
        )
        .is_err());
    files
        .export(
            &controller,
            "main.tex",
            new.revision,
            &new.source_sha256,
            None,
        )
        .unwrap();
}

#[test]
#[cfg(unix)]
fn export_refuses_parent_swapped_to_outside_symlink() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("chapter")).unwrap();
    std::fs::write(root.path().join("main.tex"), "\\input{chapter/body}").unwrap();
    std::fs::write(root.path().join("chapter/body.tex"), "inside").unwrap();
    std::fs::write(outside.path().join("body.tex"), "victim").unwrap();
    let (files, controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    let doc = controller.document("chapter/body.tex").unwrap();
    std::fs::rename(root.path().join("chapter"), root.path().join("retained")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("chapter")).unwrap();
    assert!(files
        .export(
            &controller,
            "chapter/body.tex",
            doc.revision,
            &doc.source_sha256,
            Some(&flashtex_project_files::sha256_hex(b"victim"))
        )
        .is_err());
    assert!(matches!(
        files.inspect(&controller, "chapter/body.tex").unwrap(),
        DiskState::Unavailable { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(outside.path().join("body.tex")).unwrap(),
        "victim"
    );
}

#[test]
fn explicit_reload_checks_both_versions_and_keeps_dirty_source_in_undo() {
    use flashtex_edit_ledger::history::HistoryMove;
    use flashtex_preview_controller::HistoryAction;
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "original").unwrap();
    let (files, mut controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    let old = controller.document("main.tex").unwrap().clone();
    controller
        .replace_document("main.tex", 1, &old.source_sha256, "unsaved α".into())
        .unwrap();
    let dirty = controller.document("main.tex").unwrap().clone();
    std::fs::write(root.path().join("main.tex"), "external β").unwrap();
    let hash = flashtex_project_files::sha256_hex("external β".as_bytes());
    assert!(files
        .reload_explicitly(&mut controller, "main.tex", 1, &old.source_sha256, &hash)
        .is_err());
    assert!(files
        .reload_explicitly(
            &mut controller,
            "main.tex",
            2,
            &dirty.source_sha256,
            &old.source_sha256
        )
        .is_err());
    let result = files
        .reload_explicitly(&mut controller, "main.tex", 2, &dirty.source_sha256, &hash)
        .unwrap();
    assert_eq!(result.document.text, "external β");
    drop(controller);
    let (_, mut controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    let result = controller
        .apply_history(
            "main.tex",
            HistoryAction::Undo(HistoryMove {
                command_id: "undo-reload".into(),
                expected_revision: 3,
                expected_sha256: hash,
            }),
        )
        .unwrap();
    assert_eq!(result.history.document.text, "unsaved α");
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.tex")).unwrap(),
        "external β"
    );
}

#[test]
fn dynamic_open_detach_and_reopen_preserve_source_and_navigation() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "main").unwrap();
    let (files, mut controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    std::fs::write(root.path().join("chapter.tex"), "\\label{chapter}").unwrap();
    let initial = controller.index().snapshot();
    files
        .open_document(&mut controller, &initial, "chapter.tex")
        .unwrap();
    let added = controller.index().snapshot();
    assert!(files
        .open_document(&mut controller, &initial, "chapter.tex")
        .is_err());
    assert!(controller.detach_document(&added, "main.tex").is_err());
    controller.detach_document(&added, "chapter.tex").unwrap();
    assert!(controller.document("chapter.tex").is_err());
    std::fs::remove_file(root.path().join("chapter.tex")).unwrap();
    let removed = controller.index().snapshot();
    files
        .open_document(&mut controller, &removed, "chapter.tex")
        .unwrap();
    assert_eq!(
        controller.document("chapter.tex").unwrap().text,
        "\\label{chapter}"
    );
    assert!(controller.index().snapshot().generation > added.generation);
    assert!(controller.index().symbols(&added).is_err());
    assert!(!root.path().join("chapter.tex").exists());
}

#[test]
fn bibliography_declarations_refuse_escape_and_do_not_infer_extensions() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "\\cite{old}").unwrap();
    std::fs::write(root.path().join("refs.bib"), "@article{old,title={T}}").unwrap();
    for paths in [
        vec!["../outside".into()],
        vec!["main.tex".into()],
        vec!["refs.bib".into(), "refs.bib".into()],
    ] {
        assert!(FileProject::open_with_bibliography(
            root.path(),
            private.path(),
            "p",
            "main.tex",
            &paths
        )
        .is_err());
    }
    let (files, mut controller) =
        FileProject::open(root.path(), private.path(), "p", "main.tex").unwrap();
    let snapshot = controller.index().snapshot();
    files
        .open_document(&mut controller, &snapshot, "refs.bib")
        .unwrap();
    let snapshot = controller.index().snapshot();
    assert_eq!(
        controller
            .index()
            .document_kind(&snapshot, "refs.bib")
            .unwrap(),
        flashtex_project_index::DocumentKind::Latex
    );
    assert!(controller
        .index()
        .plan_citation_rename(&snapshot, "old", "new")
        .is_err());
}
