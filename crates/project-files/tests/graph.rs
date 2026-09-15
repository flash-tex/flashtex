mod common;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use common::{TempDir, pp};
use flashtex_project_files::{
    DiagnosticKind, DiscoverError, FileKind, FileSource, Overlay, PathError, ProjectGraph,
    ProjectPath, ReferenceKind, Severity, json::Json, sha256,
};

fn paths(g: &ProjectGraph) -> Vec<&str> {
    g.files().iter().map(|f| f.path.as_str()).collect()
}

#[test]
fn nested_includes_diamond_and_cycle() {
    let t = TempDir::new("graph");
    t.write("main.tex", "\\documentclass{article}\n\\input{chapters/one}\n\\include{chapters/two}\n\\input{shared}\n");
    t.write(
        "chapters/one.tex",
        "One. \\input{shared} \\input{chapters/deep/three}",
    );
    t.write(
        "chapters/two.tex",
        "Two. \\input{main} % cycle back to the entry",
    );
    t.write("chapters/deep/three.tex", "Three.");
    t.write("shared.tex", "Shared.");

    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    // Depth-first in reference order; shared appears once (diamond).
    assert_eq!(
        paths(&g),
        [
            "main.tex",
            "chapters/one.tex",
            "shared.tex",
            "chapters/deep/three.tex",
            "chapters/two.tex"
        ]
    );
    assert_eq!(
        g.edges().len(),
        6,
        "every resolved reference is an edge, including the cycle-closing one"
    );

    let cycles: Vec<_> = g
        .diagnostics()
        .iter()
        .filter(|d| matches!(d.kind, DiagnosticKind::Cycle { .. }))
        .collect();
    assert_eq!(cycles.len(), 1);
    let c = cycles[0];
    assert_eq!(c.severity, Severity::Error);
    assert_eq!(c.path, pp("chapters/two.tex"));
    let two = t.read("chapters/two.tex");
    let span = c.span.unwrap();
    assert_eq!(&two[span.start..span.end], "\\input{main}");
    match &c.kind {
        DiagnosticKind::Cycle { chain } => {
            assert_eq!(chain, &[pp("main.tex"), pp("chapters/two.tex")])
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(g.has_errors());
}

#[test]
fn path_escape_is_rejected_with_span() {
    let t = TempDir::new("escape");
    t.write("main.tex", "x \\input{../outside} y \\include{/abs/file} z");
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert_eq!(paths(&g), ["main.tex"]);
    let diags = g.diagnostics();
    assert_eq!(diags.len(), 2);
    assert!(
        matches!(&diags[0].kind, DiagnosticKind::InvalidPath { target, error: PathError::EscapesRoot } if target == "../outside")
    );
    assert!(matches!(
        &diags[1].kind,
        DiagnosticKind::InvalidPath {
            error: PathError::Absolute,
            ..
        }
    ));
    let text = t.read("main.tex");
    let s = diags[0].argument_span.unwrap();
    assert_eq!(&text[s.start..s.end], "../outside");
    assert!(g.edges().is_empty());
}

#[test]
fn missing_file_diagnostic_has_multibyte_spans() {
    let t = TempDir::new("missing");
    let text =
        "Café résumé — \\input{naïve/chapitre} fin. \\includegraphics{fig} \\bibliography{refs}";
    t.write("main.tex", text);
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    let diags = g.diagnostics();
    assert_eq!(diags.len(), 3);

    let d = &diags[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.path, pp("main.tex"));
    let span = d.span.unwrap();
    let arg = d.argument_span.unwrap();
    assert_eq!(&text[span.start..span.end], "\\input{naïve/chapitre}");
    assert_eq!(&text[arg.start..arg.end], "naïve/chapitre");
    assert_eq!(
        span.start,
        "Café résumé — ".len(),
        "byte offset, not char offset"
    );
    match &d.kind {
        DiagnosticKind::MissingFile { target, tried } => {
            assert_eq!(target, "naïve/chapitre");
            assert_eq!(tried, &[pp("naïve/chapitre.tex"), pp("naïve/chapitre")]);
        }
        other => panic!("unexpected {other:?}"),
    }
    // Missing graphics are warnings; missing bibliographies are errors.
    assert_eq!(diags[1].severity, Severity::Warning);
    assert!(
        matches!(&diags[1].kind, DiagnosticKind::MissingFile { tried, .. } if tried.len() == 6 && tried[0] == pp("fig.pdf"))
    );
    assert_eq!(diags[2].severity, Severity::Error);
    assert!(
        matches!(&diags[2].kind, DiagnosticKind::MissingFile { tried, .. } if tried == &[pp("refs.bib")])
    );
}

#[test]
fn documents_export_entry_first_tex_only() {
    let t = TempDir::new("docs");
    t.write(
        "main.tex",
        "\\input{b}\\input{a}\\bibliography{refs}\\includegraphics{pic.png}",
    );
    t.write("a.tex", "A");
    t.write("b.tex", "B \\input{a}");
    t.write("refs.bib", "@book{k, title={T}}");
    t.write("pic.png", "not really a png");
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert!(g.diagnostics().is_empty(), "{:?}", g.diagnostics());
    assert_eq!(
        paths(&g),
        ["main.tex", "b.tex", "a.tex", "refs.bib", "pic.png"]
    );
    assert_eq!(
        g.file(&pp("refs.bib")).unwrap().kind,
        FileKind::Bibliography
    );
    let pic = g.file(&pp("pic.png")).unwrap();
    assert_eq!(pic.kind, FileKind::Graphic);
    assert!(pic.text.is_none());
    assert_eq!(pic.bytes, 16);

    let docs = g.documents();
    let doc_paths: Vec<&str> = docs.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(doc_paths, ["main.tex", "b.tex", "a.tex"]);
    assert_eq!(docs[2].text, "A");
    let with_bib: Vec<String> = g
        .documents_including_bibliography()
        .into_iter()
        .map(|d| d.path)
        .collect();
    assert_eq!(with_bib, ["main.tex", "b.tex", "a.tex", "refs.bib"]);

    let payload = g.compile_payload("demo", 7);
    assert_eq!(
        payload.get("entry_path").unwrap().as_str(),
        Some("main.tex")
    );
    assert_eq!(payload.get("revision").unwrap().as_u64(), Some(7));
    let line = g.compile_envelope("req-1", "demo", 7);
    let parsed = Json::parse(&line).unwrap();
    assert_eq!(parsed.get("type").unwrap().as_str(), Some("compile"));
    assert_eq!(parsed.get("protocol_version").unwrap().as_u64(), Some(1));
    let docs_json = parsed.get("payload").unwrap().get("documents").unwrap();
    match docs_json {
        Json::Array(items) => assert_eq!(items[0].get("path").unwrap().as_str(), Some("main.tex")),
        _ => panic!("documents must be an array"),
    }
    assert!(!line.contains('\n'));
}

#[test]
fn overlay_buffers_take_precedence_and_are_hashed() {
    let t = TempDir::new("overlay");
    t.write("main.tex", "disk \\input{a}");
    t.write("a.tex", "disk a");
    let mut overlay = Overlay::new();
    overlay.insert(pp("main.tex"), "buffer \\input{a} \\input{unsaved}");
    overlay.insert(pp("unsaved.tex"), "never saved");
    let g = ProjectGraph::discover_with(t.root(), &pp("main.tex"), &overlay).unwrap();
    assert!(g.diagnostics().is_empty(), "{:?}", g.diagnostics());
    assert_eq!(paths(&g), ["main.tex", "a.tex", "unsaved.tex"]);
    let main = g.file(&pp("main.tex")).unwrap();
    assert_eq!(main.source, FileSource::Overlay);
    assert_eq!(
        main.sha256,
        flashtex_project_files::sha256(b"buffer \\input{a} \\input{unsaved}")
    );
    assert_eq!(g.file(&pp("a.tex")).unwrap().source, FileSource::Disk);
    assert_eq!(g.documents()[2].text, "never saved");
}

#[test]
fn discovery_is_deterministic() {
    let t = TempDir::new("determinism");
    t.write("main.tex", "\\input{z}\\input{y}\\input{x}");
    t.write("x.tex", "\\input{y}");
    t.write("y.tex", "y");
    t.write("z.tex", "\\input{x}");
    let a = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    let b = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert_eq!(paths(&a), paths(&b));
    assert_eq!(paths(&a), ["main.tex", "z.tex", "x.tex", "y.tex"]);
    assert_eq!(
        a.compile_envelope("id", "p", 1),
        b.compile_envelope("id", "p", 1)
    );
    assert_eq!(a.edges(), b.edges());
    assert_eq!(a.diagnostics(), b.diagnostics());
}

#[test]
fn unresolvable_macro_argument_and_bare_input() {
    let t = TempDir::new("macro");
    t.write("main.tex", "\\input{\\jobname-extra} \\input bare.tex\n");
    t.write("bare.tex", "bare");
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert_eq!(paths(&g), ["main.tex", "bare.tex"]);
    assert_eq!(g.diagnostics().len(), 1);
    assert_eq!(g.diagnostics()[0].severity, Severity::Warning);
    assert!(matches!(
        &g.diagnostics()[0].kind,
        DiagnosticKind::UnresolvableReference { .. }
    ));
    assert_eq!(g.edges()[0].reference.kind, ReferenceKind::Input);
}

#[test]
fn entry_errors() {
    let t = TempDir::new("entry");
    assert!(matches!(
        ProjectGraph::discover(t.root(), &pp("nope.tex")),
        Err(DiscoverError::EntryMissing(_))
    ));
    assert!(matches!(
        ProjectGraph::discover(&t.root().join("missing"), &pp("main.tex")),
        Err(DiscoverError::RootNotDirectory(_))
    ));
    std::fs::write(t.root().join("bad.tex"), [0xff, 0xfe, b'x']).unwrap();
    assert!(matches!(
        ProjectGraph::discover(t.root(), &pp("bad.tex")),
        Err(DiscoverError::EntryNotUtf8(_))
    ));
}

#[cfg(unix)]
#[test]
fn symlink_escaping_root_is_rejected() {
    let outside = TempDir::new("outside");
    outside.write("secret.tex", "secret");
    let t = TempDir::new("symlink");
    t.write("main.tex", "\\input{link}");
    std::os::unix::fs::symlink(outside.root().join("secret.tex"), t.root().join("link.tex"))
        .unwrap();
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert_eq!(paths(&g), ["main.tex"]);
    assert!(
        matches!(&g.diagnostics()[0].kind, DiagnosticKind::EscapesRootViaSymlink { target } if target == &pp("link.tex"))
    );
    assert_eq!(
        g.diagnostics()[0].message,
        "\\input{link}: link.tex is a symbolic link; project files are read without following symlinks"
    );
}

/// Symlinks are refused wherever they point, so the diagnostic must not
/// claim an in-root link leaves the root; an ancestor link is named.
#[cfg(unix)]
#[test]
fn symlink_inside_root_is_refused_with_truthful_wording() {
    let t = TempDir::new("symlink-inside");
    t.write("main.tex", "\\input{link}\n\\input{alias/one}");
    t.write("real.tex", "Real.");
    t.write("chapters/one.tex", "One.");
    std::os::unix::fs::symlink(t.root().join("real.tex"), t.root().join("link.tex")).unwrap();
    std::os::unix::fs::symlink(t.root().join("chapters"), t.root().join("alias")).unwrap();
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert_eq!(paths(&g), ["main.tex"]);
    let messages: Vec<&str> = g.diagnostics().iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "\\input{link}: link.tex is a symbolic link; project files are read without following symlinks",
            "\\input{alias/one}: alias/one.tex: `alias` is a symbolic link; project files are read without following symlinks",
        ]
    );
    assert!(
        g.diagnostics()
            .iter()
            .all(|d| matches!(d.kind, DiagnosticKind::EscapesRootViaSymlink { .. }))
    );
}

/// Issue #45 finding 1: `escapes_via_symlink` (canonicalize) and `load`
/// (`fs::read`) used to be two independent syscall sequences against the
/// same path string, with no file descriptor pinned between them. A thread
/// that keeps swapping `secret.tex` between an in-root regular file and a
/// symlink to an outside file must never get its outside content read into
/// the graph: every `discover` outcome must be either the safe in-root
/// content or a refusal (`EscapesRootViaSymlink`/`MissingFile`), never a
/// leak. This must hold for every interleaving, not just probabilistically,
/// so the assertion is unconditional rather than "usually passes".
#[cfg(unix)]
#[test]
fn toctou_symlink_race_never_leaks_outside_content_into_graph() {
    let outside = TempDir::new("toctou-outside");
    let victim = outside.write("secret-data.txt", "OUTSIDE-SECRET-CONTENT");
    let t = TempDir::new("toctou-root");
    t.write("main.tex", "\\input{secret}");
    let target = t.root().join("secret.tex");
    fs::write(&target, "safe-inroot-content").unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let racer = {
        let stop = stop.clone();
        let target = target.clone();
        let victim = victim.clone();
        // A sibling temp path used to stage the "safe" content so it can be
        // installed with a single atomic `rename`, never a `fs::write`
        // (open + truncate + write) directly over `target`. `fs::write`
        // there would let a concurrent reader observe a transiently
        // truncated (empty) file -- a torn read of the racer's *own*
        // in-root content that has nothing to do with the symlink race
        // under test, but which the loop below cannot tell apart from a
        // real leak by hash alone. Only the symlink arm still needs a
        // separate `remove_file` (symlink creation fails over an existing
        // path); that produces a "missing" window, which the assertions
        // already treat as a valid outcome.
        let tmp = target.with_extension("tmp-racer");
        thread::spawn(move || {
            let mut flips = 0u32;
            while !stop.load(Ordering::Relaxed) {
                if flips.is_multiple_of(2) {
                    let _ = fs::remove_file(&target);
                    let _ = std::os::unix::fs::symlink(&victim, &target);
                } else {
                    fs::write(&tmp, "safe-inroot-content").unwrap();
                    let _ = fs::rename(&tmp, &target);
                }
                flips += 1;
            }
            flips
        })
    };

    let safe_hash = sha256(b"safe-inroot-content");
    let mut leaked = false;
    let mut safe_content_seen = 0u32;
    let mut flagged_escape_seen = 0u32;
    let mut missing_seen = 0u32;
    for _ in 0..600 {
        let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
        match g.file(&pp("secret.tex")) {
            Some(f) => {
                if f.sha256 != safe_hash {
                    leaked = true;
                }
                safe_content_seen += 1;
            }
            None => {
                let escapes = g.diagnostics().iter().any(|d| {
                    matches!(&d.kind, DiagnosticKind::EscapesRootViaSymlink { target } if target == &pp("secret.tex"))
                });
                let missing = g.diagnostics().iter().any(|d| {
                    matches!(&d.kind, DiagnosticKind::MissingFile { target, .. } if target == "secret")
                });
                assert!(
                    escapes || missing,
                    "secret.tex absent from the graph but no escape/missing diagnostic explains it: {:?}",
                    g.diagnostics()
                );
                if escapes {
                    flagged_escape_seen += 1;
                } else {
                    missing_seen += 1;
                }
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    let flips = racer.join().unwrap();
    assert!(flips > 0);
    eprintln!(
        "toctou outcomes: safe={safe_content_seen} escaped={flagged_escape_seen} missing={missing_seen}, racer flips={flips}"
    );
    assert!(
        !leaked,
        "TOCTOU RACE WON: outside file content was read into the project graph as secret.tex"
    );
}

/// Same property as
/// [`toctou_symlink_race_never_leaks_outside_content_into_graph`], but for a
/// file reached through an intermediate directory (`a/b/secret.tex`) rather
/// than a direct root child. `ProjectRoot::walk` opens each intermediate
/// component with `openat(O_NOFOLLOW)` and re-verifies it against the
/// handle it was reached from before continuing (see `save.rs`); this test
/// exercises that walk under the same kind of concurrent leaf-swapping,
/// added because the original regression only ever covered a root-level
/// target.
#[cfg(unix)]
#[test]
fn toctou_symlink_race_never_leaks_outside_content_nested_path() {
    let outside = TempDir::new("toctou-nested-outside");
    let victim = outside.write("secret-data.txt", "OUTSIDE-SECRET-CONTENT");
    let t = TempDir::new("toctou-nested-root");
    t.write("main.tex", "\\input{a/b/secret}");
    t.write("a/b/.keep", "");
    let target = t.root().join("a/b/secret.tex");
    fs::write(&target, "safe-inroot-content").unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let racer = {
        let stop = stop.clone();
        let target = target.clone();
        let victim = victim.clone();
        let tmp = target.with_extension("tmp-racer");
        thread::spawn(move || {
            let mut flips = 0u32;
            while !stop.load(Ordering::Relaxed) {
                if flips.is_multiple_of(2) {
                    let _ = fs::remove_file(&target);
                    let _ = std::os::unix::fs::symlink(&victim, &target);
                } else {
                    fs::write(&tmp, "safe-inroot-content").unwrap();
                    let _ = fs::rename(&tmp, &target);
                }
                flips += 1;
            }
            flips
        })
    };

    let safe_hash = sha256(b"safe-inroot-content");
    let mut leaked = false;
    for _ in 0..600 {
        let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
        if let Some(f) = g.file(&pp("a/b/secret.tex"))
            && f.sha256 != safe_hash
        {
            leaked = true;
        }
    }
    stop.store(true, Ordering::Relaxed);
    let flips = racer.join().unwrap();
    assert!(flips > 0);
    assert!(
        !leaked,
        "TOCTOU RACE WON: outside file content was read into the project graph as a/b/secret.tex"
    );
}

/// Issue #45 finding 3, exercised end to end through discovery: one
/// physical file referenced once as NFC ('é' precomposed) and once as NFD
/// ('e' + combining acute accent) must produce exactly one graph entry, not
/// two — on a normalization-insensitive filesystem (confirmed for APFS)
/// both spellings independently resolve to the same on-disk file, so
/// `ProjectPath`'s identity, not just its raw bytes, must recognize them as
/// the same target.
#[test]
fn nfc_nfd_reference_collision_does_not_duplicate_the_file() {
    let t = TempDir::new("nfc-nfd-graph");
    let nfc_stem = "caf\u{e9}"; // "café", 'é' precomposed (NFC)
    let nfd_stem = "cafe\u{301}"; // "café", 'e' + combining acute (NFD)
    t.write(&format!("{nfc_stem}.tex"), "Cafe content.");
    t.write(
        "main.tex",
        &format!("\\input{{{nfc_stem}}} \\input{{{nfd_stem}}}"),
    );
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    let cafe_files: Vec<&str> = g
        .files()
        .iter()
        .filter(|f| f.path.as_str() != "main.tex")
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(
        cafe_files.len(),
        1,
        "one physical file must be one graph entry, got {cafe_files:?}"
    );
    assert_eq!(g.edges().len(), 2, "both references still resolve");
}

/// Issue #45 finding 3, the ext4 half of the story. APFS is normalization
/// *insensitive*: a literal byte-for-byte lookup of the NFD spelling against
/// an NFC file on disk already succeeds at the OS level (discovery's literal
/// check is tried first, so it never even reaches the directory-listing
/// fallback there), so a test that only exercises that would stay green on
/// macOS whether or not discovery has any normalization awareness of its own
/// — which is exactly how this regressed (green on macOS, `left: 1, right:
/// 2` on the ubuntu runner: ext4 performs a literal byte comparison, so the
/// NFD reference found no file at all and never resolved). This test creates
/// *only* the NFC-spelled file and resolves it through a single NFD-spelled
/// reference, deterministically (no threads, no probabilistic outcome): on a
/// normalization-sensitive filesystem this can only succeed through the
/// directory-listing fallback; see
/// `graph::tests::directory_listing_fallback_finds_nfd_reference_against_nfc_file`
/// in `src/graph.rs` for a unit test that exercises that exact fallback
/// function directly, independent of the host filesystem's own behavior.
#[test]
fn nfd_only_reference_resolves_to_the_nfc_file_on_disk() {
    let t = TempDir::new("nfc-nfd-ext4-sim");
    let nfc_stem = "caf\u{e9}"; // "café", 'é' precomposed (NFC) -- the only file written to disk
    let nfd_stem = "cafe\u{301}"; // "café", 'e' + combining acute (NFD) -- how it's referenced
    t.write(&format!("{nfc_stem}.tex"), "Cafe content.");
    t.write("main.tex", &format!("\\input{{{nfd_stem}}}"));

    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert!(g.diagnostics().is_empty(), "{:?}", g.diagnostics());

    let cafe_files: Vec<&str> = g
        .files()
        .iter()
        .filter(|f| f.path.as_str() != "main.tex")
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(
        cafe_files.len(),
        1,
        "the single NFC file on disk must resolve exactly once, got {cafe_files:?}"
    );
    // Identity (not raw-byte) equality: whichever spelling the resolver kept
    // as the winning candidate, it must be recognized as the same physical
    // file as the on-disk NFC name.
    assert_eq!(
        ProjectPath::normalize(cafe_files[0]).unwrap(),
        pp(&format!("{nfc_stem}.tex"))
    );
    assert_eq!(g.edges().len(), 1, "the NFD reference must resolve");
}

#[test]
fn project_path_display_and_ordering() {
    let mut v = [pp("b/a.tex"), pp("a.tex"), pp("a/z.tex")];
    v.sort();
    assert_eq!(
        v.iter().map(ProjectPath::as_str).collect::<Vec<_>>(),
        ["a.tex", "a/z.tex", "b/a.tex"]
    );
    assert_eq!(format!("{}", pp("./x/../y.tex")), "y.tex");
}

/// Review finding 3: existence is probed through the pinned root without
/// following anything, so a broken symlink, a symlink to a directory, and a
/// reference under a symlinked directory that has no such file are all
/// refused as symlinks rather than reported as missing files.
#[cfg(unix)]
#[test]
fn broken_and_directory_symlinks_are_refused_not_missing() {
    let outside = TempDir::new("probe-outside");
    outside.write("sub/keep.txt", "x");
    let t = TempDir::new("probe-symlinks");
    t.write(
        "main.tex",
        "\\input{broken}\n\\input{dirlink}\n\\input{linked/absent}\n\\input{really-missing}",
    );
    let link = std::os::unix::fs::symlink;
    link(t.root().join("nowhere.tex"), t.root().join("broken.tex")).unwrap();
    link(outside.root().join("sub"), t.root().join("dirlink.tex")).unwrap();
    link(outside.root().to_path_buf(), t.root().join("linked")).unwrap();
    let g = ProjectGraph::discover(t.root(), &pp("main.tex")).unwrap();
    assert_eq!(paths(&g), ["main.tex"]);
    let got: Vec<(&str, bool)> = g
        .diagnostics()
        .iter()
        .map(|d| {
            (
                d.message.as_str(),
                matches!(d.kind, DiagnosticKind::EscapesRootViaSymlink { .. }),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (
                "\\input{broken}: broken.tex is a symbolic link; project files are read without following symlinks",
                true
            ),
            (
                "\\input{dirlink}: dirlink.tex is a symbolic link; project files are read without following symlinks",
                true
            ),
            (
                "\\input{linked/absent}: linked/absent.tex: `linked` is a symbolic link; project files are read without following symlinks",
                true
            ),
            (
                "\\input{really-missing}: no file found (tried really-missing.tex, really-missing)",
                false
            ),
        ]
    );
}
