//! Edge-case tests for APIs with no coverage elsewhere: JSON error paths,
//! path/scan edges, the reference-candidate table, and discovery diagnostics.

mod common;

use common::{TempDir, pp};
use flashtex_project_files::graph::{GRAPHIC_EXTENSIONS, MAX_DEPTH, candidates};
use flashtex_project_files::json::{Json, write_string};
use flashtex_project_files::{
    DiagnosticKind, FileKind, PathError, ProjectGraph, ProjectPath, ReferenceKind, RevisionTracker,
    Severity, scan_references, sha256, sha256_from_hex, sha256_to_hex,
};

#[test]
fn json_parse_errors_are_typed() {
    let cases = [
        ("", "unexpected end"),
        ("[", "unexpected end"),
        ("{", "expected '\"'"),
        ("-", "invalid number"),
        ("1e", "invalid number"),
        ("tru", "invalid literal"),
        ("nul", "invalid literal"),
        ("\"abc", "unterminated string"),
        ("\"abc\\", "unterminated escape"),
        ("\"a\x01b\"", "control character in string"),
        ("\"\\q\"", "invalid escape"),
        ("\"\\u12\"", "short \\u escape"),
        ("\"\\ud83d\\u0041\"", "invalid low surrogate"),
        ("\"\\ude00\"", "invalid code point"),
        ("[1,]", "unexpected character"),
        ("{\"a\":1} trailing", "trailing characters"),
    ];
    for (input, fragment) in cases {
        let err = Json::parse(input).unwrap_err();
        assert!(
            err.message.contains(fragment),
            "{input:?} -> {err:?}, want {fragment:?}"
        );
    }
    // Nesting deeper than the parser allows is rejected, not a stack overflow.
    let err = Json::parse(&"[".repeat(200)).unwrap_err();
    assert!(err.message.contains("nesting too deep"), "{err:?}");
}

#[test]
fn path_edge_cases() {
    // NUL and control characters are forbidden, like backslash and colon.
    assert_eq!(
        ProjectPath::normalize("a\x00b"),
        Err(PathError::ForbiddenCharacter('\0'))
    );
    assert_eq!(
        ProjectPath::normalize("a\nb"),
        Err(PathError::ForbiddenCharacter('\n'))
    );
    assert_eq!(ProjectPath::normalize("./"), Err(PathError::Empty));
    // resolve_in folds dot segments in the base directory too.
    let r = |b: &str, p: &str| ProjectPath::resolve_in(b, p).unwrap().as_str().to_string();
    assert_eq!(r("a/./b", "c.tex"), "a/b/c.tex");
    assert_eq!(r("a/b", ".."), "a");
    assert_eq!(
        ProjectPath::resolve_in("", ".."),
        Err(PathError::EscapesRoot)
    );
    // Extension edges: a trailing dot yields an empty extension, not None.
    assert_eq!(pp("a.b.c").extension(), Some("c"));
    assert_eq!(pp("foo.").extension(), Some(""));
    assert_eq!(pp("noext").extension(), None);
    assert_eq!(pp("a.tex").parent_dir(), "");
    // Builders used by discovery for candidate probing.
    assert_eq!(
        pp("ch/a").with_appended_extension("tex").as_str(),
        "ch/a.tex"
    );
    assert_eq!(pp("ch/a").with_appended_leaf("x").as_str(), "ch/a/x");
}

#[test]
fn scan_edge_cases() {
    // `#` (like `\`) marks an argument as needing expansion.
    let refs = scan_references("\\input{a#b}");
    assert_eq!(refs.len(), 1);
    assert!(!refs[0].literal);
    // Braces nest; a backslash escapes the closing brace (making the
    // argument non-literal) instead of ending the argument.
    let refs = scan_references("\\input{a{b}c}");
    assert_eq!(refs[0].argument, "a{b}c");
    assert!(refs[0].literal);
    let refs = scan_references("\\input{a\\}b}");
    assert_eq!(refs[0].argument, "a\\}b");
    assert!(!refs[0].literal);
    // Empty arguments (and empty bibliography items) are skipped.
    assert!(scan_references("\\input{}").is_empty());
    assert!(scan_references("\\input{   }").is_empty());
    assert!(scan_references("\\bibliography{,}").is_empty());
    let args: Vec<_> = scan_references("\\bibliography{a,,b}")
        .into_iter()
        .map(|r| r.argument)
        .collect();
    assert_eq!(args, ["a", "b"]);
    // An unbalanced brace consumes to end of input without hanging or emitting.
    assert!(scan_references("\\input{abc").is_empty());
    // `\begin` of a non-verbatim env (or without braces) hides nothing.
    assert_eq!(
        scan_references("\\begin{document}\\input{x}\\end{document}")[0].argument,
        "x"
    );
    assert_eq!(scan_references("\\begin \\input{x}")[0].argument, "x");
    // Starred and unterminated `\verb` still hide their contents.
    assert!(scan_references("\\verb*|\\input{x}|").is_empty());
    assert!(scan_references("\\verb|\\input{x}").is_empty());
    // Bare `\input` with no name, and a comment ending at EOF, emit nothing.
    assert!(scan_references("\\input   ").is_empty());
    assert!(scan_references("% \\input{x}").is_empty());
    // Braces inside the optional argument do not confuse includegraphics.
    let refs = scan_references("\\includegraphics[alt={x}]{fig}");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].argument, "fig");
}

#[test]
fn file_kind_names_and_reference_candidates() {
    use FileKind as K;
    use ReferenceKind as R;
    let table = [
        ("a.tex", Some(K::Tex)),
        ("a.ltx", Some(K::Tex)),
        ("a.bib", Some(K::Bibliography)),
        ("a.sty", Some(K::Package)),
        ("a.def", Some(K::Package)),
        ("a.cls", Some(K::Class)),
        ("a.clo", Some(K::Class)),
        ("a.pdf", Some(K::Graphic)),
        ("a.png", Some(K::Graphic)),
        ("a.jpg", Some(K::Graphic)),
        ("a.jpeg", Some(K::Graphic)),
        ("a.eps", Some(K::Graphic)),
        ("a.svg", Some(K::Graphic)),
        ("README", None),
        ("a.log", None),
        (".tex", None),
        ("noext.", None),
        ("a.TEX", None),
        ("a.PNG", None),
    ];
    for (name, want) in table {
        assert_eq!(FileKind::from_name(name), want, "{name}");
    }

    // Candidate probing order per reference kind.
    let probed = |kind: ReferenceKind, base: &str| {
        let (found, paths) = candidates(kind, &pp(base));
        (
            found,
            paths
                .iter()
                .map(|p| p.as_str().to_string())
                .collect::<Vec<_>>(),
        )
    };
    let v = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(probed(R::Input, "ch/a"), (K::Tex, v(&["ch/a.tex", "ch/a"])));
    assert_eq!(probed(R::Input, "a.tex"), (K::Tex, v(&["a.tex"])));
    assert_eq!(probed(R::Include, "ch"), (K::Tex, v(&["ch.tex", "ch"])));
    assert_eq!(
        probed(R::Bibliography, "refs"),
        (K::Bibliography, v(&["refs.bib", "refs.bbl"]))
    );
    assert_eq!(
        probed(R::Bibliography, "refs.bib"),
        (K::Bibliography, v(&["refs.bib"]))
    );
    assert_eq!(
        probed(R::Bibliography, "refs.bbl"),
        (K::Bibliography, v(&["refs.bbl"]))
    );
    assert_eq!(
        probed(R::AddBibResource, "r"),
        (K::Bibliography, v(&["r.bib"]))
    );
    assert_eq!(
        probed(R::AddBibResource, "r.bib"),
        (K::Bibliography, v(&["r.bib"]))
    );
    // Graphics: a known extension (any ASCII case) is used as-is,
    // otherwise every supported extension is tried in order.
    let all_graphics: Vec<String> = GRAPHIC_EXTENSIONS
        .iter()
        .map(|e| format!("fig.{e}"))
        .collect();
    assert_eq!(
        probed(R::IncludeGraphics, "fig"),
        (K::Graphic, all_graphics)
    );
    assert_eq!(
        probed(R::IncludeGraphics, "fig.pdf"),
        (K::Graphic, v(&["fig.pdf"]))
    );
    assert_eq!(
        probed(R::IncludeGraphics, "fig.PNG"),
        (K::Graphic, v(&["fig.PNG"]))
    );
    let (found, unknown) = probed(R::IncludeGraphics, "fig.txt");
    assert_eq!(found, K::Graphic);
    assert_eq!(unknown.len(), GRAPHIC_EXTENSIONS.len());
    assert!(unknown[0].ends_with(".pdf"));
}

#[test]
fn discovery_reports_depth_and_utf8_errors() {
    // A chain longer than MAX_DEPTH: main -> f1 -> ... -> f70.
    let t = TempDir::new("edge-diag");
    t.write("main.tex", "\\input{f1}");
    for i in 1..=70 {
        let next = if i < 70 {
            format!("\\input{{f{}}}", i + 1)
        } else {
            "end.".to_string()
        };
        t.write(&format!("f{i}.tex"), &next);
    }
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    let depth: Vec<_> = g
        .diagnostics()
        .iter()
        .filter(|d| matches!(d.kind, DiagnosticKind::DepthExceeded { .. }))
        .collect();
    assert_eq!(depth.len(), 1);
    assert_eq!(depth[0].severity, Severity::Error);
    assert!(
        matches!(&depth[0].kind, DiagnosticKind::DepthExceeded { target } if target == &pp("f64.tex")),
        "{:?}",
        depth[0].kind
    );
    assert!(g.has_errors());
    // The chain stops at 64 files (entry + 63 descended); the edge to the
    // un-descended target is still recorded.
    assert_eq!(g.files().len(), MAX_DEPTH);
    assert!(g.edges().iter().any(|e| e.to == pp("f64.tex")));

    // A non-UTF-8 TeX input is an InvalidUtf8 error (spans unset); the same
    // bytes as a graphic are fine — graphics are hashed, never decoded.
    let u = TempDir::new("edge-utf8");
    u.write("main.tex", "\\input{bad}\n\\includegraphics{img}");
    std::fs::write(u.root().join("bad.tex"), b"\xff\xfe invalid").unwrap();
    std::fs::write(u.root().join("img.png"), b"\xff\xfe binary").unwrap();
    let g = ProjectGraph::discover(u.root(), &pp("main.tex")).unwrap();
    let bad: Vec<_> = g
        .diagnostics()
        .iter()
        .filter(|d| matches!(d.kind, DiagnosticKind::InvalidUtf8 { .. }))
        .collect();
    assert_eq!(bad.len(), 1);
    assert_eq!(bad[0].severity, Severity::Error);
    assert_eq!(bad[0].path, pp("bad.tex"));
    assert!(bad[0].span.is_none());
    assert_eq!(g.file(&pp("bad.tex")).unwrap().text, None);
    assert!(g.file(&pp("img.png")).is_some());
}

#[test]
fn json_write_string_escape_edges() {
    // Short escapes plus \u00xx for other C0 controls; DEL and non-ASCII
    // pass through raw.
    let cases = [
        ("a\"b\\c", "\"a\\\"b\\\\c\""),
        ("\r\n\t", "\"\\r\\n\\t\""),
        ("\u{08}\u{0c}", "\"\\b\\f\""),
        ("\u{1b}", "\"\\u001b\""),
        ("\u{0}", "\"\\u0000\""),
        ("\u{7f}", "\"\u{7f}\""),
        ("é😀", "\"é😀\""),
    ];
    for (input, want) in cases {
        assert_eq!(Json::from(input).to_string_compact(), want, "{input:?}");
    }
    // The public helper matches the compact writer exactly.
    let mut direct = String::new();
    write_string(&mut direct, "\u{8}q\u{1b}");
    assert_eq!(direct, "\"\\bq\\u001b\"");
}

#[test]
fn sha256_parse_hex_edges() {
    let digest = sha256(b"abc");
    let lower = sha256_to_hex(&digest);
    // Either case parses (the lower round-trip itself is covered in sha256.rs).
    assert_eq!(sha256_from_hex(&lower.to_ascii_uppercase()), Some(digest));
    let mut mixed = lower.to_ascii_uppercase();
    mixed.replace_range(..1, &lower[..1]);
    assert_eq!(sha256_from_hex(&mixed), Some(digest));
    // Length must be exactly 64 hex chars.
    assert_eq!(sha256_from_hex(""), None);
    assert_eq!(sha256_from_hex(&lower[..63]), None);
    assert_eq!(sha256_from_hex(&format!("{lower}0")), None);
    // 64 chars with a non-hex digit anywhere is malformed, not truncated.
    for i in [0, 31, 63] {
        let mut bad = lower.clone();
        bad.replace_range(i..i + 1, "g");
        assert_eq!(sha256_from_hex(&bad), None, "bad char at {i}");
    }
}

#[test]
fn revision_tracker_observe_bytes_and_files_edges() {
    let mut t = RevisionTracker::new();
    assert_eq!(t.files().count(), 0);
    assert_eq!(t.project_revision(), 0);
    // observe_bytes hashes raw bytes: non-UTF-8 content works, which
    // observe(&str) cannot express.
    let raw = b"\xff\xfe\0binary";
    let (fr, changed) = t.observe_bytes(&pp("img.png"), raw);
    assert!(changed);
    assert_eq!(fr.revision, 1);
    assert_eq!(fr.bytes, raw.len() as u64);
    assert_eq!(fr.sha256, sha256(raw));
    // Same bytes again: no change, no project bump.
    let (fr2, changed2) = t.observe_bytes(&pp("img.png"), raw);
    assert!(!changed2);
    assert_eq!(fr2, fr);
    assert_eq!(t.project_revision(), 1);
    // observe_bytes and observe agree on the same content.
    let (g1, c1) = t.observe_bytes(&pp("a.tex"), b"hello");
    assert!(c1);
    let (g2, c2) = t.observe(&pp("a.tex"), "hello");
    assert!(!c2);
    assert_eq!(g1, g2);
    // Changed bytes bump the file revision and the project revision.
    let (g3, c3) = t.observe_bytes(&pp("a.tex"), b"hello!");
    assert!(c3);
    assert_eq!(g3.revision, 2);
    assert_eq!(t.project_revision(), 3);
    // files() iterates tracked entries in path order; remove() drops one.
    let paths: Vec<String> = t.files().map(|(p, _)| p.as_str().to_string()).collect();
    assert_eq!(paths, ["a.tex", "img.png"]);
    t.remove(&pp("a.tex"));
    let paths: Vec<String> = t.files().map(|(p, _)| p.as_str().to_string()).collect();
    assert_eq!(paths, ["img.png"]);
}
