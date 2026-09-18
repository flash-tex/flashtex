//! Output-identity digests for performance work (issue #65).
//!
//! For every `.tex` file under the given directories, prints SHA-256 digests of
//! (1) the `{:#?}` dump of a clean `compile_full_project` (parsed blocks, i.e.
//! the AST, plus diagnostics and the laid-out pages / display list), (2) the
//! runtime-v1 `protocol::handle_line` reply bytes, and (3) the warm
//! `Session` output after inserting one character mid-document. Files under
//! `fixtures/real-world/<name>/` are compiled together with every other `.tex`
//! file of that fixture, so `\input` resolves. Two builds whose outputs are
//! byte-identical print identical lines; `diff` the two listings.
//!
//! Usage: fixture_digest <dir>... > digests.txt

use flashtex_compiler::incremental::{compile_full_project, Session};
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::protocol::handle_line;
use flashtex_font_engine::sha256;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "tex") {
            out.push(path);
        }
    }
}

/// Root of the project a file belongs to: `fixtures/real-world/<name>` or the file itself.
fn project_root(path: &Path) -> Option<PathBuf> {
    let components: Vec<_> = path.components().collect();
    let at = components
        .windows(2)
        .position(|w| w[0].as_os_str() == "fixtures" && w[1].as_os_str() == "real-world")?;
    let root: PathBuf = components.get(..at + 3)?.iter().collect();
    root.is_dir().then_some(root)
}

fn hex16(bytes: &[u8]) -> String {
    sha256::digest(bytes)[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn request(project: &str, entry: &str, documents: &[(String, String)]) -> String {
    let docs = documents
        .iter()
        .map(|(path, text)| {
            let mut doc = Value::obj();
            doc.set("path", json::str_(path.as_str()));
            doc.set("text", json::str_(text.as_str()));
            doc
        })
        .collect();
    let mut payload = Value::obj();
    payload.set("project_id", json::str_(project));
    payload.set("revision", Value::Num(0.0));
    payload.set("entry_path", json::str_(entry));
    payload.set("documents", Value::Arr(docs));
    payload.set(
        "layout_capabilities",
        Value::Arr(vec![json::str_("font-hints-v1"), json::str_("rules-v1")]),
    );
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("digest"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

fn main() {
    let constraints = LayoutConstraints::default();
    let mut files = Vec::new();
    for dir in std::env::args().skip(1) {
        walk(Path::new(&dir), &mut files);
    }
    for (index, file) in files.iter().enumerate() {
        let (root, members) = match project_root(file) {
            Some(root) => {
                let mut members = Vec::new();
                walk(&root, &mut members);
                (root, members)
            }
            None => (
                file.parent().expect("parent").to_path_buf(),
                vec![file.clone()],
            ),
        };
        let relative = |path: &Path| {
            path.strip_prefix(&root)
                .expect("member under root")
                .to_string_lossy()
                .into_owned()
        };
        let entry = relative(file);
        let owned: Vec<(String, String)> = members
            .iter()
            .map(|member| {
                let text = std::fs::read(member).expect("read tex");
                (
                    relative(member),
                    String::from_utf8_lossy(&text).into_owned(),
                )
            })
            .collect();
        let documents: Vec<SourceDocument<'_>> = owned
            .iter()
            .map(|(path, text)| SourceDocument { path, text })
            .collect();

        let clean = compile_full_project(&documents, &entry, constraints);
        let clean_digest = hex16(format!("{clean:#?}").as_bytes());
        let reply = handle_line(&request(&format!("digest-{index}"), &entry, &owned));
        let reply_digest = hex16(reply.as_bytes());

        let mut edited = owned.clone();
        let target = edited
            .iter_mut()
            .find(|(path, _)| *path == entry)
            .expect("entry present");
        let mut mid = target.1.len() / 2;
        while !target.1.is_char_boundary(mid) {
            mid += 1;
        }
        target.1.insert(mid, 'x');
        let edited_docs: Vec<SourceDocument<'_>> = edited
            .iter()
            .map(|(path, text)| SourceDocument { path, text })
            .collect();
        let mut session = Session::new();
        session.compile_project(&documents, &entry, constraints);
        let warm = session.compile_project(&edited_docs, &entry, constraints);
        let warm_digest = hex16(format!("{:#?}", warm.output).as_bytes());

        println!(
            "{clean_digest} {reply_digest} {warm_digest} pages={} diags={} {}",
            clean.pages.len(),
            clean.diagnostics.len(),
            file.display()
        );
    }
}
