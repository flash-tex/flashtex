//! FT-067: the supported-LaTeX inventory is the compiler's truth.
//!
//! 1. Generated artefacts (`docs/user/compiler.md` section, `supported/*`)
//!    equal what `flashtex-compiler --supported` prints now.
//! 2. The inventory equals the literal `match` arms in `src/parser.rs` and
//!    `src/math.rs` (both directions).
//! 3. Every inventory entry compiles without a "not supported" diagnostic,
//!    and every canonical name outside the inventory is diagnosed, so an
//!    unlisted working command cannot hide.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;
use flashtex_compiler::supported::{self, Mode, Origin};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

const REGENERATE: &str = "run crates/compiler/scripts/render_supported_latex.sh";

#[test]
fn generated_artifacts_are_current() {
    let inventory = supported::inventory();
    let docs = read(crate_dir().join("../../docs/user/compiler.md"));
    let begin = docs.find(supported::DOC_BEGIN).unwrap_or_else(|| {
        panic!("docs/user/compiler.md lacks the generated marker; {REGENERATE}")
    });
    let end = docs[begin..]
        .find(supported::DOC_END)
        .map(|i| begin + i + supported::DOC_END.len() + 1)
        .expect("end marker");
    assert!(
        docs[begin..end.min(docs.len())] == supported::render_markdown(&inventory),
        "docs/user/compiler.md 'Supported LaTeX' is stale; {REGENERATE}"
    );
    assert!(
        read(crate_dir().join("supported/supported-latex.json"))
            == supported::render_json(&inventory),
        "supported/supported-latex.json is stale; {REGENERATE}"
    );
    assert!(
        read(crate_dir().join("supported/coverage.md"))
            == supported::render_coverage_markdown(&inventory),
        "supported/coverage.md is stale; {REGENERATE}"
    );
    json::parse(&supported::render_json(&inventory)).expect("--supported json is valid JSON");
}

#[test]
fn inventory_has_no_duplicates_and_every_entry_is_described() {
    let inventory = supported::inventory();
    let mut seen = BTreeSet::new();
    for c in &inventory.commands {
        assert!(
            seen.insert((c.name, c.mode.as_str())),
            "duplicate {:?} {}",
            c.mode,
            c.name
        );
        assert!(!c.description.is_empty(), "\\{} has no description", c.name);
        assert!(!c.description.contains('\n'), "\\{}", c.name);
    }
    let mut envs = BTreeSet::new();
    for e in &inventory.environments {
        assert!(envs.insert(e.name), "duplicate environment {}", e.name);
    }
}

// ---- source arms ---------------------------------------------------------

enum ArmLine {
    /// `"a" | "b" =>`: the head of an arm.
    Head(Vec<String>),
    /// `"a" | "b"` or `| "c"` with no `=>`: part of a multi-line head.
    Part(Vec<String>),
}

/// Parses one line of a `"a" | "b" => ...` arm head, which rustfmt may split
/// across lines (`"mathrm" | … | "bm"` / `| "mbox" | … =>`).
fn arm_line(line: &str) -> Option<ArmLine> {
    let mut rest = line.trim();
    if let Some(after) = rest.strip_prefix('|') {
        rest = after.trim_start();
    }
    let mut names = Vec::new();
    loop {
        rest = rest.strip_prefix('"')?;
        let end = rest.find('"')?;
        let name = &rest[..end];
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        names.push(name.to_string());
        rest = rest[end + 1..].trim_start();
        if let Some(after) = rest.strip_prefix('|') {
            rest = after.trim_start();
        } else if rest.starts_with("=>") {
            return Some(ArmLine::Head(names));
        } else if rest.is_empty() {
            return Some(ArmLine::Part(names));
        } else {
            return None;
        }
    }
}

fn region<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let s = text
        .find(start)
        .unwrap_or_else(|| panic!("source marker {start:?} moved"));
    let e = text[s..]
        .find(end)
        .unwrap_or_else(|| panic!("source marker {end:?} moved"));
    &text[s..s + e]
}

fn arms(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut pending: Vec<String> = Vec::new();
    for line in text.lines() {
        match arm_line(line) {
            Some(ArmLine::Head(names)) => {
                out.extend(pending.drain(..));
                out.extend(names);
            }
            Some(ArmLine::Part(names)) => pending.extend(names),
            None => pending.clear(),
        }
    }
    out
}

fn quoted(text: &str, allow_star: bool) -> BTreeSet<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .filter(|s| {
            !s.is_empty()
                && s.trim_end_matches(|c| allow_star && c == '*')
                    .chars()
                    .all(|c| c.is_ascii_alphabetic())
        })
        .map(str::to_string)
        .collect()
}

fn names(filter: impl Fn(&supported::Command) -> bool) -> BTreeSet<String> {
    supported::inventory()
        .commands
        .iter()
        .filter(|c| filter(c))
        .map(|c| c.name.to_string())
        .collect()
}

#[test]
fn text_inventory_equals_the_parser_arms() {
    let parser = read(crate_dir().join("src/parser.rs"));
    let mut expected = arms(region(
        &parser,
        "        match name {\n            \"documentclass\"",
        "other => self.unsupported(other, span)",
    ));
    expected.extend(quoted(region(&parser, "fn style_command(", "\n}\n"), false));
    expected.extend(quoted(
        region(&parser, "fn style_declaration(", "\n}\n"),
        false,
    ));
    for diagnostic_only in supported::TEXT_DIAGNOSTIC_ONLY {
        assert!(
            expected.remove(*diagnostic_only),
            "{diagnostic_only} arm moved"
        );
    }
    // Expansion-pass commands are executed by `crate::expansion`'s engine and
    // never reach a parser arm; the behaviour probes below still compile each
    // one and require that it is not diagnosed as unsupported.
    let actual = names(|c| {
        c.mode == Mode::Text && c.origin != Origin::ControlSymbol && c.origin != Origin::Expansion
    });
    assert_eq!(
        actual, expected,
        "text inventory != parser dispatch/style arms"
    );
}

#[test]
fn math_inventory_equals_the_math_arms() {
    let math = read(crate_dir().join("src/math.rs"));
    let mut expected = arms(region(
        &math,
        "fn command_atom(",
        "            _ => match (crate::amssymb::by_name(&name), command_glyph(&name)) {",
    ));
    for line in region(&math, "fn list_inner(", "fn script_argument(").lines() {
        if line.contains("TokenKind::Command(ref ") {
            expected.extend(quoted(line, false));
        }
    }
    let actual = names(|c| c.origin == Origin::MathStructure);
    assert_eq!(
        actual, expected,
        "math structure inventory != command_atom arms"
    );
}

#[test]
fn environment_and_package_inventory_equals_the_parser_arms() {
    let parser = read(crate_dir().join("src/parser.rs"));
    let mut expected = quoted(
        region(
            &parser,
            "    fn environment(",
            "let popped = self.env_stack.pop();",
        ),
        true,
    );
    expected.remove("begin");
    // `thebibliography`'s arm sets its heading text, not an environment name.
    expected.remove("References");
    expected.extend(arms(region(&parser, "fn paragraph_style(", "\n}\n")));
    let inventory = supported::inventory();
    let actual: BTreeSet<String> = inventory
        .environments
        .iter()
        .filter(|e| e.mode == Mode::Text)
        .map(|e| e.name.to_string())
        .collect();
    assert_eq!(
        actual, expected,
        "text environments != parser environment arms"
    );

    let packages = arms(region(&parser, "fn package_matches_layout(", "\n}\n"));
    let actual: BTreeSet<String> = inventory
        .packages
        .iter()
        .map(|p| p.name.to_string())
        .collect();
    assert_eq!(actual, packages, "packages != package_matches_layout arms");
}

// ---- behaviour probes ----------------------------------------------------

fn diagnostics(text: &str) -> Vec<String> {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("supported"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("probe"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    let reply = json::parse(&handle_line(&json::write(&env))).expect("JSON reply");
    let payload = reply.get("payload").expect("payload");
    assert!(
        payload.get("code").is_none(),
        "protocol error for {text:?}: {payload:?}"
    );
    payload
        .get("diagnostics")
        .and_then(|d| d.as_arr())
        .map(|d| {
            d.iter()
                .filter_map(|d| {
                    d.get("message")
                        .and_then(|m| m.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn not_supported(messages: &[String], name: &str) -> Option<String> {
    let needle = format!("\\{name} is not supported");
    messages.iter().find(|m| m.starts_with(&needle)).cloned()
}

/// A compilable use of `\name` built from its argument shape.
fn text_probe(name: &str, arguments: &str) -> String {
    match name {
        "\\" => "a\\\\b".into(),
        "begin" | "end" => "\\begin{center}x\\end{center}".into(),
        "newcommand" => "\\newcommand{\\foo}{x}\\foo".into(),
        "renewcommand" => "\\newcommand{\\foo}{x}\\renewcommand{\\foo}{y}\\foo".into(),
        "usepackage" => "\\usepackage{geometry}".into(),
        "setlength" => "\\setlength{\\parskip}{1pt}".into(),
        "setlist" => "\\setlist{itemsep=1pt}".into(),
        "item" => "\\begin{itemize}\\item x\\end{itemize}".into(),
        "caption" => "\\begin{figure}\\caption{x}\\end{figure}".into(),
        "uline" => "\\usepackage{ulem}\\uline{x}".into(),
        _ => with_arguments(name, arguments, "1pt"),
    }
}

fn with_arguments(name: &str, arguments: &str, dimension: &str) -> String {
    let mut out = format!("\\{name}");
    let mut rest = arguments;
    while let Some(open) = rest.find('{') {
        let close = rest[open..]
            .find('}')
            .map(|i| open + i)
            .unwrap_or(rest.len());
        let shape = &rest[open + 1..close];
        let value = if shape.contains("dimension") {
            dimension
        } else if shape == "A-Z" {
            "R"
        } else if shape == "env" {
            "matrix"
        } else {
            "x"
        };
        out.push_str(&format!("{{{value}}}"));
        rest = &rest[(close + 1).min(rest.len())..];
    }
    out
}

fn math_probe(name: &str, arguments: &str) -> String {
    let body = match name {
        "begin" => "\\begin{matrix}a\\end{matrix}".to_string(),
        "left" => "\\left( a \\right)".to_string(),
        "right" => "\\left( a \\right)".to_string(),
        "limits" | "nolimits" => format!("\\sum\\{name}_a"),
        "choose" | "over" => format!("{{a \\{name} b}}"),
        n if n.starts_with("big") || n.starts_with("Big") => format!("\\{n}( a"),
        n if n.len() == 1 && !n.chars().all(|c| c.is_ascii_alphabetic()) => format!("a\\{n}b"),
        _ => with_arguments(name, arguments, "1pt"),
    };
    format!("${body}$")
}

#[test]
fn every_inventory_entry_compiles_without_an_unsupported_diagnostic() {
    let inventory = supported::inventory();
    let mut failures = Vec::new();
    for c in &inventory.commands {
        let source = match c.mode {
            Mode::Text => text_probe(c.name, c.arguments),
            Mode::Math => math_probe(c.name, c.arguments),
        };
        let messages = diagnostics(&source);
        if let Some(message) = not_supported(&messages, c.name) {
            failures.push(format!("{source:?}: {message}"));
        }
        if c.mode == Mode::Math
            && messages
                .iter()
                .any(|m| m.contains("script marker used outside math"))
        {
            failures.push(format!("{source:?}: probe left math mode: {messages:?}"));
        }
    }
    for e in &inventory.environments {
        let (source, needle) = match e.mode {
            Mode::Text => (
                format!(
                    "\\begin{{{0}}}{1}a\\end{{{0}}}",
                    e.name,
                    if e.name.starts_with("alignat") {
                        "{1}"
                    } else {
                        ""
                    }
                ),
                format!("environment '{}' is not implemented", e.name),
            ),
            Mode::Math => (
                format!(
                    "$\\begin{{{0}}}{1}a\\end{{{0}}}$",
                    e.name,
                    match e.name {
                        "array" => "{c}",
                        "alignedat" => "{1}",
                        _ => "",
                    }
                ),
                format!("\\begin{{{}}} is not supported", e.name),
            ),
        };
        let messages = diagnostics(&source);
        if let Some(m) = messages.iter().find(|m| m.starts_with(&needle)) {
            failures.push(format!("{source:?}: {m}"));
        }
    }
    for p in &inventory.packages {
        let options = p.options.replace(' ', "");
        let source = format!("\\documentclass{{article}}\\usepackage[{options}]{{{}}}\\begin{{document}}x\\end{{document}}", p.name);
        let messages = diagnostics(&source);
        if let Some(m) = messages
            .iter()
            .find(|m| m.contains("recognised but not implemented"))
        {
            failures.push(format!("{source:?}: {m}"));
        }
    }
    assert!(
        failures.is_empty(),
        "listed as supported but diagnosed:\n{}",
        failures.join("\n")
    );

    // The probe itself must be able to see a failure.
    assert!(not_supported(
        &diagnostics("\\flashtexnosuchcommand"),
        "flashtexnosuchcommand"
    )
    .is_some());
    assert!(not_supported(
        &diagnostics("$\\flashtexnosuchcommand$"),
        "flashtexnosuchcommand"
    )
    .is_some());
}

#[test]
fn canonical_names_outside_the_inventory_are_diagnosed() {
    let inventory = supported::inventory();
    let mut silent = BTreeSet::new();
    for (set, kind, name) in supported::canonical_entries() {
        if kind == "command" {
            if inventory
                .commands
                .iter()
                .any(|c| c.name == name && c.renders)
            {
                continue;
            }
            let text = diagnostics(&format!("\\{name}"));
            let math = diagnostics(&format!("$\\{name}$"));
            let named = |ms: &[String]| ms.iter().any(|m| m.contains(&format!("\\{name}")));
            if !named(&text) || !named(&math) {
                silent.insert(format!("{set} \\{name}"));
            }
        } else if !inventory.environments.iter().any(|e| e.name == name) {
            let text = diagnostics(&format!("\\begin{{{name}}}a\\end{{{name}}}"));
            if !text.iter().any(|m| m.contains(name)) {
                silent.insert(format!("{set} env {name}"));
            }
        }
    }
    assert!(
        silent.is_empty(),
        "canonical names handled without a diagnostic but missing from the inventory:\n{}",
        silent.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn canonical_list_is_well_formed_and_sourced() {
    let entries = supported::canonical_entries();
    let sources = supported::canonical_sources();
    for set in supported::CANONICAL_SETS {
        assert!(
            entries.iter().any(|(s, ..)| s == set),
            "canonical set {set} is empty"
        );
        assert!(
            sources
                .iter()
                .any(|line| line.starts_with(&format!("Source {set}:"))),
            "canonical set {set} has no source line"
        );
    }
    let mut seen = BTreeSet::new();
    for (set, kind, name) in &entries {
        assert!(
            matches!(*kind, "command" | "environment"),
            "{set} {kind} {name}"
        );
        assert!(
            seen.insert((set, kind, name)),
            "duplicate canonical {set} {kind} {name}"
        );
        assert!(!name.contains('@'), "{name}");
    }
}
