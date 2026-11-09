use flashtex_project_index::{Category, IndexError, ProjectIndex, SearchRequest, SourceSpan};

fn project() -> ProjectIndex {
    let mut index = ProjectIndex::new("guards").unwrap();
    index
        .replace_document(
            "main.tex",
            1,
            "\\label{sec}\\cite{key}\\newcommand{\\foo}{x}\\foo",
        )
        .unwrap();
    index
        .replace_bibliography_document("refs.bib", 1, "@book{key,title={T}}")
        .unwrap();
    index
}

#[test]
fn blank_project_ids_are_rejected() {
    for id in ["", "   ", "\t\n "] {
        assert!(matches!(
            ProjectIndex::new(id),
            Err(IndexError::InvalidProject)
        ));
    }
    assert!(ProjectIndex::new("guards").is_ok());
}

#[test]
fn every_snapshot_query_rejects_a_stale_view() {
    let mut index = project();
    let old = index.snapshot();
    let span = index
        .definitions(&old, Category::Label, "sec")
        .unwrap()
        .remove(0)
        .source;
    let search = index
        .search_literal(&old, &SearchRequest::literal("sec"), || false)
        .unwrap();
    index
        .replace_document("main.tex", 2, "\\label{sec}")
        .unwrap();
    assert_eq!(index.symbols(&old), Err(IndexError::StaleSnapshot));
    assert_eq!(index.diagnostics(&old), Err(IndexError::StaleSnapshot));
    assert_eq!(
        index.unresolved_references(&old),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.last_reindex_metrics(&old),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.source_text(&old, &span),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.occurrences(&old, Category::Label, "sec"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.definitions(&old, Category::Label, "sec"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.navigate(&old, "main.tex", 0),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.plan_label_rename(&old, "sec", "new"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.plan_label_rename_at(&old, &span, "new"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.complete(&old, Category::Label, "", 10),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.citation_metadata(&old, "key"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.complete_citations(&old, "", 10),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.metadata_cache_metrics(&old),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.document_kind(&old, "main.tex"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.plan_citation_rename(&old, "key", "new"),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.search_literal(&old, &SearchRequest::literal("sec"), || false),
        Err(IndexError::StaleSnapshot)
    );
    assert_eq!(
        index.plan_literal_replacement(&search, "x"),
        Err(IndexError::StaleSnapshot)
    );
}

#[test]
fn unknown_and_malformed_paths_are_rejected_at_every_entry_point() {
    let mut index = project();
    let snapshot = index.snapshot();
    assert_eq!(
        index.document_kind(&snapshot, "missing.tex"),
        Err(IndexError::MissingDocument)
    );
    assert_eq!(
        index.document_kind(&snapshot, "../main.tex"),
        Err(IndexError::InvalidPath)
    );
    assert_eq!(
        index.navigate(&snapshot, "../main.tex", 0),
        Err(IndexError::InvalidPath)
    );
    let missing = SourceSpan {
        file: "missing.tex".into(),
        revision: 0,
        start_byte: 0,
        end_byte: 1,
    };
    assert_eq!(
        index.source_text(&snapshot, &missing),
        Err(IndexError::MissingDocument)
    );
    let malformed = SourceSpan {
        file: "../main.tex".into(),
        revision: 1,
        start_byte: 0,
        end_byte: 1,
    };
    assert_eq!(
        index.source_text(&snapshot, &malformed),
        Err(IndexError::InvalidPath)
    );
    assert_eq!(
        index.remove_document("missing.tex", 0),
        Err(IndexError::MissingDocument)
    );
    assert_eq!(
        index.remove_document("../main.tex", 0),
        Err(IndexError::InvalidPath)
    );
    index.remove_document("main.tex", 2).unwrap();
    assert!(matches!(
        index.remove_document("main.tex", 2),
        Err(IndexError::StaleDocument { .. })
    ));
    assert_eq!(
        index.remove_document("main.tex", 3),
        Err(IndexError::MissingDocument)
    );
}

#[test]
fn rename_by_selection_rejects_foreign_categories_and_noop_requires_a_definition() {
    let index = project();
    let snapshot = index.snapshot();
    let citation = index
        .occurrences(&snapshot, Category::Citation, "key")
        .unwrap()
        .remove(0)
        .source;
    assert_eq!(
        index.plan_label_rename_at(&snapshot, &citation, "new"),
        Err(IndexError::InvalidRenamePlan)
    );
    let command = index
        .occurrences(&snapshot, Category::Command, "foo")
        .unwrap()
        .remove(0)
        .source;
    assert_eq!(
        index.plan_label_rename_at(&snapshot, &command, "new"),
        Err(IndexError::InvalidRenamePlan)
    );
    let label = index
        .definitions(&snapshot, Category::Label, "sec")
        .unwrap()
        .remove(0)
        .source;
    assert_eq!(
        index.plan_citation_rename_at(&snapshot, &label, "new"),
        Err(IndexError::InvalidCitationRenamePlan)
    );
    assert_eq!(
        index.plan_label_rename(&snapshot, "missing", "missing"),
        Err(IndexError::MissingLabelDefinition)
    );
}

#[test]
fn citation_key_length_bound_is_enforced_at_the_boundary() {
    let index = project();
    let snapshot = index.snapshot();
    assert_eq!(
        index.plan_citation_rename(&snapshot, "key", &"x".repeat(4097)),
        Err(IndexError::InvalidCitationKey)
    );
    assert_eq!(
        index.plan_citation_rename(&snapshot, "key", "a\n"),
        Err(IndexError::InvalidCitationKey)
    );
    assert_eq!(
        index.plan_citation_rename(&snapshot, "no-such-key", &"x".repeat(4096)),
        Err(IndexError::MissingBibliographyDefinition)
    );
}

#[test]
fn empty_membership_atomically_clears_the_project() {
    let mut index = project();
    let old = index.snapshot();
    assert!(index.last_reindex_metrics(&old).unwrap().is_some());
    index.replace_membership(&old, &[]).unwrap();
    let cleared = index.snapshot();
    assert_eq!(cleared.generation, old.generation + 1);
    assert!(cleared.documents.is_empty());
    assert!(index.symbols(&cleared).unwrap().is_empty());
    assert!(index.diagnostics(&cleared).unwrap().is_empty());
    assert!(index.unresolved_references(&cleared).unwrap().is_empty());
    assert_eq!(index.last_reindex_metrics(&cleared).unwrap(), None);
    assert_eq!(index.symbols(&old), Err(IndexError::StaleSnapshot));
    assert!(matches!(
        index.replace_document("main.tex", 1, "\\label{sec}"),
        Err(IndexError::StaleDocument { .. })
    ));
}
