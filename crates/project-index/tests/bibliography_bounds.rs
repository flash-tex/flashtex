use flashtex_project_index::{MetadataStatus, ProjectIndex};

fn index(source: &str) -> ProjectIndex {
    let mut index = ProjectIndex::new("bibliography-bounds").unwrap();
    index
        .replace_bibliography_document("refs.bib", 1, source)
        .unwrap();
    index
}

fn fields_source(count: usize) -> String {
    let mut source = String::from("@book{k,");
    for n in 0..count {
        source.push_str(&format!("f{n}={{x}},"));
    }
    source.push('}');
    source
}

#[test]
fn exactly_128_fields_resolve() {
    let index = index(&fields_source(128));
    let metadata = index.citation_metadata(&index.snapshot(), "k").unwrap();
    assert_eq!(metadata.status, MetadataStatus::Resolved);
    assert_eq!(metadata.fields.len(), 128);
    assert_eq!(metadata.fields["f127"].value.as_deref(), Some("x"));
}

#[test]
fn field_129_is_rejected_with_too_many_fields() {
    let index = index(&fields_source(129));
    let metadata = index.citation_metadata(&index.snapshot(), "k").unwrap();
    assert_eq!(metadata.status, MetadataStatus::Malformed);
    assert!(metadata.fields.is_empty());
    assert!(
        metadata.records[0]
            .diagnostics
            .iter()
            .any(|d| d.message == "Too many fields")
    );
}

#[test]
fn exactly_256_concatenated_atoms_resolve() {
    let atoms = vec!["{a}"; 256].join(" # ");
    let index = index(&format!("@book{{k,title={atoms}}}"));
    let metadata = index.citation_metadata(&index.snapshot(), "k").unwrap();
    assert_eq!(metadata.status, MetadataStatus::Resolved);
    assert_eq!(metadata.records[0].fields[0].parts.len(), 256);
    assert_eq!(
        metadata.fields["title"].value.as_deref(),
        Some("a".repeat(256).as_str())
    );
}

#[test]
fn literal_between_expansion_and_lexical_bounds_is_incomplete() {
    // 300 KiB parses (under the 1 MiB atom bound) but cannot expand
    // (over the 256 KiB expanded-value bound).
    let value = "é".repeat(150_000);
    let index = index(&format!("@book{{k,title={{{value}}}}}"));
    let metadata = index.citation_metadata(&index.snapshot(), "k").unwrap();
    assert_eq!(metadata.status, MetadataStatus::Incomplete);
    assert!(metadata.fields["title"].value.is_none());
    assert_eq!(
        metadata.fields["title"].issues,
        ["Expanded value byte bound exceeded"]
    );
}

#[test]
fn literal_beyond_one_mebibyte_is_malformed_and_excludes_the_tail() {
    let value = "x".repeat(1024 * 1024 + 4096);
    let index = index(&format!(
        "@book{{k,title={{{value}}}}}\n@book{{tail,title={{T}}}}"
    ));
    let snapshot = index.snapshot();
    let metadata = index.citation_metadata(&snapshot, "k").unwrap();
    assert_eq!(metadata.status, MetadataStatus::Malformed);
    assert!(
        metadata.records[0]
            .diagnostics
            .iter()
            .any(|d| d.message == "Value exceeds lexical bounds")
    );
    assert_eq!(
        index.citation_metadata(&snapshot, "tail").unwrap().status,
        MetadataStatus::Missing
    );
}
