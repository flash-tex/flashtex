//! GH-75: a compile request that names its `project_root` gets the whole
//! `\input`/`\include` closure, not just the buffers the client had open.
//!
//! The IDE sends the documents its windows hold. For the ordinary shape of a
//! LaTeX project — a `main.tex` that `\input`s files in subdirectories — that
//! used to mean the compile saw only `main.tex`, reported
//! `included file not found: looked for 'sections/intro' …` for a file sitting
//! right there on disk, and rendered every `\ref` into it as `??`. The
//! document was unusable until the user opened each include by hand.
//!
//! The rest of the closure is now read through project-files' rooted,
//! symlink-refusing discovery — the same walk `flashtex build` uses — with the
//! request's own documents overlaid, so:
//!
//! * an unsaved buffer always beats the bytes on disk, and is what gets
//!   scanned for further includes;
//! * resolution stays TeX's: every path is relative to the job's root, never
//!   to the directory of the including file;
//! * an include that escapes the root (`..`, or a symlink pointing out) is
//!   refused, and says so;
//! * a genuinely missing include still gets the compiler's diagnostic;
//! * a request with no `project_root` reads nothing from disk at all.

use flashtex_render_pipeline::{FontSet, RenderOptions};

/// A project staged in a per-test temp directory, removed on drop.
struct Project(std::path::PathBuf);

impl Project {
    fn new(tag: &str) -> Project {
        let dir = std::env::temp_dir().join(format!("flashtex-gh75-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("stage a project directory");
        Project(std::fs::canonicalize(&dir).expect("canonicalize the project directory"))
    }

    /// Writes `text` at project-relative `path`, creating parent directories.
    fn write(&self, path: &str, text: &str) -> &Project {
        let full = self.0.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("stage a subdirectory");
        }
        std::fs::write(&full, text).expect("write a project file");
        self
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One `compile` request carrying `documents` and, when `root` is set, the
/// `project_root` that turns on closure completion.
fn reply(root: Option<&std::path::Path>, documents: &[(&str, &str)]) -> String {
    let docs: Vec<String> = documents
        .iter()
        .map(|(p, t)| format!(r#"{{"path":{},"text":{}}}"#, json_string(p), json_string(t)))
        .collect();
    let root_field = match root {
        Some(r) => format!(r#","project_root":{}"#, json_string(&r.display().to_string())),
        None => String::new(),
    };
    let line = format!(
        r#"{{"protocol_version":1,"id":"g75","type":"compile","payload":{{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{}]{root_field}}}}}"#,
        docs.join(",")
    );
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    flashtex_render_pipeline::protocol::handle_line(&line, &fonts, &options, None).line
}

const PREAMBLE: &str = "\\documentclass{article}\\begin{document}";
const POSTAMBLE: &str = "\\end{document}";

fn main_tex(body: &str) -> String {
    format!("{PREAMBLE}{body}{POSTAMBLE}")
}

/// Every `"text"` value the reply's pages carry, in order — what the reader
/// would see on the page.
fn page_text(reply: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = reply;
    // The reply is the v1 fallback: page items are the only objects with a
    // `"text"` field, and diagnostics use `"message"`.
    while let Some(at) = rest.find("\"text\":") {
        rest = rest[at + "\"text\":".len()..].trim_start();
        if !rest.starts_with('"') {
            continue;
        }
        let mut value = String::new();
        let mut chars = rest[1..].chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        value.push(escaped);
                    }
                }
                c => value.push(c),
            }
        }
        out.push(value);
    }
    out
}

// -- the reported symptom -------------------------------------------------

/// The filed bug, end to end: only `main.tex` is open, `sections/intro.tex`
/// is on disk, and the compile must read it and resolve the `\ref` into it.
#[test]
fn an_unopened_include_in_a_subdirectory_is_read_from_disk() {
    let project = Project::new("subdir-include");
    let body = "\\input{sections/intro}See \\ref{sec:intro}.";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "\\section{Introduction}\\label{sec:intro}Hello from the include.");

    let reply = reply(Some(project.path()), &[("main.tex", &main_tex(body))]);
    assert!(!reply.contains("included file not found"), "the include is on disk and must be found: {reply}");
    let text = page_text(&reply);
    assert!(text.iter().any(|t| t == "Introduction"), "the include's own text must be typeset: {text:?}");
    assert!(text.iter().any(|t| t == "include."), "the include's body must be typeset: {text:?}");
}

/// The other half of the symptom: `\ref` across the include boundary renders
/// the number, not `??`.
#[test]
fn a_ref_across_the_include_boundary_resolves() {
    let project = Project::new("cross-file-ref");
    let body = "\\input{sections/intro}See \\ref{sec:intro}.";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "\\section{Introduction}\\label{sec:intro}Body.");

    let text = page_text(&reply(Some(project.path()), &[("main.tex", &main_tex(body))]));
    assert!(!text.iter().any(|t| t.contains("??")), "\\ref into the include must resolve, got {text:?}");
    assert!(text.iter().any(|t| t == "1."), "\\ref must render the section number: {text:?}");
}

/// A `\ref` into a file the closure never reaches is still `??` — the fix
/// resolves references, it does not invent them.
#[test]
fn a_ref_to_a_label_that_exists_nowhere_is_still_unresolved() {
    let project = Project::new("dangling-ref");
    let body = "See \\ref{sec:nowhere}.";
    project.write("main.tex", &main_tex(body));

    let text = page_text(&reply(Some(project.path()), &[("main.tex", &main_tex(body))]));
    assert!(text.iter().any(|t| t.contains("??")), "an unresolvable \\ref stays ??: {text:?}");
}

// -- what distinguishes a correct implementation --------------------------

/// A nested include: `sections/intro.tex` itself `\input`s a deeper file.
/// TeX resolves that path against the job's root, not against the directory
/// of the file doing the including, and so must this.
#[test]
fn a_nested_include_resolves_against_the_root() {
    let project = Project::new("nested-include");
    let body = "\\input{sections/intro}See \\ref{sec:deep}.";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "\\section{Introduction}\\input{sections/sub/deep}")
        .write("sections/sub/deep.tex", "\\subsection{Deep}\\label{sec:deep}Deep text.");

    let reply = reply(Some(project.path()), &[("main.tex", &main_tex(body))]);
    let text = page_text(&reply);
    assert!(text.iter().any(|t| t == "Deep"), "the second-level include must be read: {text:?}");
    assert!(text.iter().any(|t| t == "1.1."), "\\ref into the nested include resolves: {text:?}");
}

/// The mirror image: a path relative to the *including* file's own directory
/// is not how TeX resolves, so `sections/sibling.tex` must stay unfound even
/// though it sits next to the file naming it. Completing the closure from
/// disk must not quietly widen resolution beyond TeX's rule.
#[test]
fn an_include_relative_to_its_own_directory_is_not_resolved() {
    let project = Project::new("sibling-relative");
    let body = "\\input{sections/intro}";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "\\section{Intro}\\input{sibling}")
        .write("sections/sibling.tex", "SIBLINGTEXT");

    let reply = reply(Some(project.path()), &[("main.tex", &main_tex(body))]);
    assert!(reply.contains("included file not found"), "TeX resolves from the root, so this is missing: {reply}");
    assert!(!page_text(&reply).iter().any(|t| t == "SIBLINGTEXT"), "the sibling file must not be pulled in");
}

/// An include that already names the extension resolves the same way.
#[test]
fn an_include_with_an_explicit_tex_extension_resolves() {
    let project = Project::new("explicit-extension");
    let body = "\\input{sections/intro.tex}";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "EXPLICITEXTENSION");

    let text = page_text(&reply(Some(project.path()), &[("main.tex", &main_tex(body))]));
    assert!(text.iter().any(|t| t == "EXPLICITEXTENSION"), "an explicit .tex must resolve: {text:?}");
}

/// `\include` (the `\clearpage`-ing one), not just `\input`.
#[test]
fn an_unopened_include_command_target_is_read_from_disk() {
    let project = Project::new("include-command");
    let body = "\\include{chapters/one}See \\ref{ch:one}.";
    project
        .write("main.tex", &main_tex(body))
        .write("chapters/one.tex", "\\section{One}\\label{ch:one}Chapter one.");

    let reply = reply(Some(project.path()), &[("main.tex", &main_tex(body))]);
    assert!(!reply.contains("included file not found"), "\\include resolves too: {reply}");
    assert!(page_text(&reply).iter().any(|t| t == "One"), "the \\include'd file is typeset");
}

// -- containment ----------------------------------------------------------

/// A parent-traversing include is refused, and says why. The compiler's own
/// "rejected include path" diagnostic covers the path; discovery adds the
/// reason it refused to read it.
#[test]
fn an_include_escaping_the_root_with_dot_dot_is_refused() {
    let outer = Project::new("dotdot-outer");
    outer.write("secret.tex", "SECRETCONTENT");
    let root = outer.path().join("proj");
    std::fs::create_dir_all(&root).expect("stage the project root");
    let body = "\\input{../secret}";
    std::fs::write(root.join("main.tex"), main_tex(body)).expect("write the entry");

    let reply = reply(Some(&root), &[("main.tex", &main_tex(body))]);
    assert!(!page_text(&reply).iter().any(|t| t == "SECRETCONTENT"), "a file outside the root must never be read: {reply}");
    assert!(reply.contains("path escapes the project root"), "and the refusal must be explained: {reply}");
}

/// A symlink pointing outside the root is refused by the same walk — the
/// project's deliberate refuse-all-symlinks discovery, not a second rule
/// written here.
#[cfg(unix)]
#[test]
fn an_include_reached_through_a_symlink_out_of_the_root_is_refused() {
    let outer = Project::new("symlink-outer");
    outer.write("secret.tex", "SECRETCONTENT");
    let root = outer.path().join("proj");
    std::fs::create_dir_all(root.join("sections")).expect("stage the project root");
    std::os::unix::fs::symlink(outer.path().join("secret.tex"), root.join("sections/secret.tex"))
        .expect("stage the escaping symlink");
    let body = "\\input{sections/secret}";
    std::fs::write(root.join("main.tex"), main_tex(body)).expect("write the entry");

    let reply = reply(Some(&root), &[("main.tex", &main_tex(body))]);
    assert!(!page_text(&reply).iter().any(|t| t == "SECRETCONTENT"), "a symlink out of the root must never be read: {reply}");
    assert!(reply.contains("symlink outside the project root"), "and the refusal must be explained: {reply}");
}

/// A genuinely missing include keeps the compiler's diagnostic, with the
/// names it looked for — the message that tells the user what to fix.
#[test]
fn a_genuinely_missing_include_still_reports_it() {
    let project = Project::new("missing-include");
    let body = "\\input{sections/nope}";
    project.write("main.tex", &main_tex(body));

    let reply = reply(Some(project.path()), &[("main.tex", &main_tex(body))]);
    assert!(
        reply.contains("included file not found") && reply.contains("sections/nope.tex"),
        "a missing include must still be reported, with what was tried: {reply}"
    );
}

// -- precedence and opt-in ------------------------------------------------

/// An open, edited buffer is the truth: the file on disk must not overwrite
/// what the user is typing.
#[test]
fn an_unsaved_buffer_beats_the_file_on_disk() {
    let project = Project::new("unsaved-buffer");
    let body = "\\input{sections/intro}";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "ONDISK");

    let text = page_text(&reply(
        Some(project.path()),
        &[("main.tex", &main_tex(body)), ("sections/intro.tex", "UNSAVEDBUFFER")],
    ));
    assert!(text.iter().any(|t| t == "UNSAVEDBUFFER"), "the open buffer wins: {text:?}");
    assert!(!text.iter().any(|t| t == "ONDISK"), "the disk copy must not be used: {text:?}");
}

/// A further include reached only through an unsaved buffer is discovered
/// from that buffer, not from the stale file.
#[test]
fn an_include_added_in_an_unsaved_buffer_is_followed() {
    let project = Project::new("unsaved-new-include");
    let body = "\\input{sections/intro}";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "OLDTEXT")
        .write("sections/added.tex", "ADDEDTEXT");

    let text = page_text(&reply(
        Some(project.path()),
        &[("main.tex", &main_tex(body)), ("sections/intro.tex", "\\input{sections/added}")],
    ));
    assert!(text.iter().any(|t| t == "ADDEDTEXT"), "the buffer's new include is followed: {text:?}");
}

/// Without `project_root` the worker reads nothing from disk — the request is
/// the whole project, exactly as before this existed.
#[test]
fn without_a_project_root_nothing_is_read_from_disk() {
    let project = Project::new("no-project-root");
    let body = "\\input{sections/intro}";
    project
        .write("main.tex", &main_tex(body))
        .write("sections/intro.tex", "ONDISK");

    let reply = reply(None, &[("main.tex", &main_tex(body))]);
    assert!(reply.contains("included file not found"), "no root means no disk read: {reply}");
    assert!(!page_text(&reply).iter().any(|t| t == "ONDISK"), "and nothing from the directory is used");
}

/// A `project_root` that is not a usable directory is not an error: the
/// compile proceeds on the documents the request sent.
#[test]
fn an_unusable_project_root_falls_back_to_the_sent_documents() {
    let project = Project::new("unusable-root");
    let body = "\\section{Only}Only this.";
    project.write("main.tex", &main_tex(body));
    let missing = project.path().join("does-not-exist");

    let text = page_text(&reply(Some(&missing), &[("main.tex", &main_tex(body))]));
    assert!(text.iter().any(|t| t == "Only"), "the sent documents still compile: {text:?}");
}
