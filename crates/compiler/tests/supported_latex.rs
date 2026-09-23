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
        // A control-word arm is letters only; a package arm may carry a
        // digit (`CJKutf8`, `\usepackage{CJKutf8}`), never as its first
        // character.
        if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_alphabetic()) || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
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

/// Command names the tabular row scanner (`src/parser/tabular.rs`) handles
/// itself, outside `Parser::command`: the rule arms (`is_rule_command`),
/// the longtable/colortbl arms (`is_table_command`), the cell strippers
/// (`strip_cell_commands`, `extract_column_color`) and the `command == ".."`
/// guards of the row loop (`\multicolumn`, `\tabularnewline`; `\begin` and
/// `\end` only track nesting depth there and are inventoried with the
/// parser arms). Scanned from the source like the parser arms above, so a
/// new row-scanner command without an inventory entry fails here.
fn table_commands(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // `is_rule_command` and the `is_table_command` gate are free functions;
    // the cell strippers are methods, whose own closing brace is indented
    // one level. (The gate's arms mirror `table_command`, which consumes
    // what the gate admits, so scanning the gate covers both.)
    for (start, end) in [
        ("fn is_rule_command(", "\n}\n"),
        ("fn is_table_command(", "\n}\n"),
        ("fn strip_cell_commands(", "\n    }\n"),
        ("fn extract_column_color(", "\n    }\n"),
    ] {
        out.extend(quoted(region(text, start, end), false));
    }
    // `command == "multicolumn"` guards of the row loop. (`begin`/`end`
    // only track nesting depth there and are inventoried with the parser
    // arms, as are the other guards this scan picks up.) Only the row
    // loop names its token `command`; the `name == ".."` checks elsewhere
    // (environment and column-spec dispatch) are inventoried separately.
    for line in text.lines() {
        let mut rest = line;
        while let Some(found) = rest.find("command == \"") {
            rest = &rest[found + "command == \"".len()..];
            if let Some(end) = rest.find('"') {
                let name = &rest[..end];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic()) {
                    out.insert(name.to_string());
                }
                rest = &rest[end..];
            } else {
                break;
            }
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
    // Table rules, spans and colours never reach `Parser::command`: the
    // tabular row scanner (`src/parser/tabular.rs`) consumes them first.
    // They are inventoried with the text commands (see `TEXT_EXTRA_ARMS`),
    // so the expected set scans those arms too.
    expected.extend(table_commands(&read(
        crate_dir().join("src/parser/tabular.rs"),
    )));
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
    // `longtable` has no arm in `environment`: `package_table_environment`
    // in `src/parser/tabular.rs` takes it when the package is loaded.
    expected.extend(quoted(
        region(
            &read(crate_dir().join("src/parser/tabular.rs")),
            "fn package_table_environment(",
            "\n    }\n",
        ),
        true,
    ));
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
    let wrong_class = format!("\\{name} is defined by the letter");
    let beamer_class = format!("\\{name} is defined by the beamer");
    messages
        .iter()
        // letter.cls commands are refused by their own message when the
        // class is not `letter`. Without the second prefix this probe could
        // not fail for any of them: an `\opening` diagnosed as "defined by
        // the letter document class" sails past a match on "\opening is not
        // supported", and the inventory row keeps claiming `renders: true`.
        .find(|m| {
            m.starts_with(&needle) || m.starts_with(&wrong_class) || m.starts_with(&beamer_class)
        })
        .cloned()
}

/// A letter with everything `\opening` and `\closing` read already declared.
/// `#` marks where a probe's own command goes.
const LETTER_DOCUMENT: &str = "\\documentclass{letter}\n\\address{1 Example Street}\n\\signature{A. Author}\n\\begin{document}\n\\begin{letter}{A Name\\\\An Address}\n\\opening{Dear reader,}\nBody.\n#\n\\end{letter}\n\\end{document}\n";

/// Where a letter.cls command has to sit to be exercised for real: the
/// preamble declarations before `\begin{document}`, the rest inside an open
/// letter. `None` for anything that is not a letter.cls command.
fn letter_probe(name: &str, arguments: &str) -> Option<String> {
    let source = match name {
        "address" | "signature" | "name" | "location" | "telephone" | "makelabels" => {
            LETTER_DOCUMENT.replace(
                "\\begin{document}",
                &format!(
                    "{}\n\\begin{{document}}",
                    with_arguments(name, arguments, "1pt")
                ),
            )
        }
        "opening" | "closing" | "cc" | "encl" | "ps" | "startbreaks" | "stopbreaks"
        | "stopletter" => LETTER_DOCUMENT.replace('#', &with_arguments(name, arguments, "1pt")),
        _ => return None,
    };
    Some(source.replace('#', ""))
}

/// A minimal beamer deck: `#` marks where a probe's own command goes.
/// `\frametitle`/`\framesubtitle`/`\alert` exist only under beamer, so like
/// the letter.cls commands above they are exercised under their own class.
const BEAMER_DOCUMENT: &str = "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{Probe}\n#\n\\end{frame}\n\\end{document}\n";

/// Where a beamer command has to sit to be exercised for real: inside a
/// frame of a beamer deck. `None` for anything that is not beamer-gated.
fn beamer_probe(name: &str, arguments: &str) -> Option<String> {
    match name {
        n if supported::BEAMER_CLASS_COMMANDS.contains(&n) => {
            Some(BEAMER_DOCUMENT.replace('#', &with_arguments(name, arguments, "1pt")))
        }
        _ => None,
    }
}

/// The AMS classes' top-matter commands exist only under amsart/amsbook/
/// amsproc: exercised in a minimal amsart document, before `\maketitle`.
fn ams_probe(name: &str, arguments: &str) -> Option<String> {
    match name {
        "curraddr" | "email" | "urladdr" | "subjclass" | "keywords" | "dedicatory" => Some(format!(
            "\\documentclass{{amsart}}\n\\title{{T}}\\author{{A}}\n{}\n\\begin{{document}}\n\\maketitle\nBody.\n\\end{{document}}\n",
            with_arguments(name, arguments, "1pt")
        )),
        _ => None,
    }
}

/// A compilable use of `\name` built from its argument shape.
fn text_probe(name: &str, arguments: &str) -> String {
    if let Some(letter) = letter_probe(name, arguments) {
        return letter;
    }
    if let Some(ams) = ams_probe(name, arguments) {
        return ams;
    }
    if let Some(beamer) = beamer_probe(name, arguments) {
        return beamer;
    }
    match name {
        "\\" => "a\\\\b".into(),
        "begin" | "end" => "\\begin{center}x\\end{center}".into(),
        "newcommand" => "\\newcommand{\\foo}{x}\\foo".into(),
        "renewcommand" => "\\newcommand{\\foo}{x}\\renewcommand{\\foo}{y}\\foo".into(),
        "usepackage" => "\\usepackage{geometry}".into(),
        "setlength" => "\\setlength{\\parskip}{1pt}".into(),
        "addtolength" => "\\addtolength{\\textwidth}{1pt}".into(),
        "setlist" => "\\setlist{itemsep=1pt}".into(),
        "item" => "\\begin{itemize}\\item x\\end{itemize}".into(),
        // natbib's commands exist only once the package is loaded, and an
        // author-year citation needs an `[Author(Year)]` entry to resolve to
        // — exactly as `\item` needs its list around it.
        n if natbib_command(n) => format!(
            "\\usepackage{{natbib}}\\begin{{thebibliography}}{{9}}\\bibitem[Knuth(1984)]{{x}}A.\\end{{thebibliography}}{}",
            with_arguments(n, arguments, "1pt")
        ),
        "caption" => "\\begin{figure}\\caption{x}\\end{figure}".into(),
        "newfloat" => "\\usepackage{float}\\newfloat{program}{htbp}{lop}".into(),
        "floatname" => "\\usepackage{float}\\floatname{program}{Program}".into(),
        "floatstyle" => "\\usepackage{float}\\floatstyle{ruled}".into(),
        "floatplacement" => "\\usepackage{float}\\floatplacement{figure}{tbp}".into(),
        // A bare `{x}` test is not a valid `\ifthenelse` test (the engine
        // reports "Missing test"), so probe the real form instead.
        "ifthenelse" => "\\ifthenelse{\\equal{a}{a}}{yes}{no}".into(),
        // `\iftoggle` needs a declared toggle; probing it bare would
        // report the undefined-toggle marker instead of rendering.
        "iftoggle" => "\\newtoggle{x}\\toggletrue{x}\\iftoggle{x}{yes}{no}".into(),
        "captionof" => "\\captionof{figure}{x}".into(),
        "uline" => "\\usepackage{ulem}\\uline{x}".into(),
        "sout" => "\\usepackage{ulem}\\sout{x}".into(),
        "so" => "\\usepackage{soul}\\so{x}".into(),
        "hl" => "\\usepackage{soul}\\hl{x}".into(),
        // Table rules, spans and colours only exist inside a table: probe
        // each where TeX allows it, with the package that defines it.
        "hline" => "\\begin{tabular}{cc}a&b\\\\\\hline c&d\\end{tabular}".into(),
        "cline" => "\\begin{tabular}{cc}a&b\\\\\\cline{1-2}c&d\\end{tabular}".into(),
        "multicolumn" => "\\begin{tabular}{cc}\\multicolumn{2}{c}{x}\\\\a&b\\end{tabular}".into(),
        "tabularnewline" => "\\begin{tabular}{cc}a&b\\tabularnewline c&d\\end{tabular}".into(),
        "toprule" | "midrule" | "bottomrule" => format!(
            "\\usepackage{{booktabs}}\\begin{{tabular}}{{cc}}\\{name} a&b\\\\c&d\\\\\\bottomrule\\end{{tabular}}"
        ),
        "cmidrule" => "\\usepackage{booktabs}\\begin{tabular}{cc}\\toprule a&b\\\\\\cmidrule{1-1}c&d\\end{tabular}".into(),
        "addlinespace" => "\\usepackage{booktabs}\\begin{tabular}{cc}a&b\\\\\\addlinespace c&d\\end{tabular}".into(),
        "specialrule" => "\\usepackage{booktabs}\\begin{tabular}{cc}a&b\\\\\\specialrule{1pt}{0pt}{0pt}c&d\\end{tabular}".into(),
        "morecmidrules" => "\\usepackage{booktabs}\\begin{tabular}{cc}a&b\\\\\\cmidrule{1-1}\\morecmidrules\\cmidrule{2-2}c&d\\end{tabular}".into(),
        "multirow" => "\\usepackage{multirow}\\begin{tabular}{cc}\\multirow{2}{*}{x}&b\\\\c&d\\end{tabular}".into(),
        "rowcolor" => "\\usepackage{colortbl}\\begin{tabular}{cc}\\rowcolor{red}a&b\\\\c&d\\end{tabular}".into(),
        "cellcolor" => "\\usepackage{colortbl}\\begin{tabular}{cc}\\cellcolor{red}x&b\\\\c&d\\end{tabular}".into(),
        "columncolor" => "\\usepackage{colortbl}\\begin{tabular}{>{\\columncolor{red}}cc}a&b\\\\c&d\\end{tabular}".into(),
        "kill" => "\\usepackage{longtable}\\begin{longtable}{cc}a&b\\\\\\kill c&d\\end{longtable}".into(),
        "endfirsthead" | "endhead" | "endfoot" | "endlastfoot" => format!(
            "\\usepackage{{longtable}}\\begin{{longtable}}{{cc}}a&b\\\\\\{name}c&d\\end{{longtable}}"
        ),
        // Expansion-pass commands need their real argument shape.
        "arabic" | "roman" | "Roman" | "alph" => {
            format!("\\section{{S}}\\{name}{{section}}")
        }
        "newif" => "\\newif\\iffoo\\footrue\\iffoo x\\fi".into(),
        "verb" => "x\\verb|y|z".into(),
        // A text-command default needs a command to declare it for; probing
        // it bare would leave an unconsumed argument instead of rendering.
        "DeclareTextCommandDefault" => {
            "\\DeclareTextCommandDefault{\\textfoo}{FOO}A \\textfoo{} B".into()
        }
        "ProvideTextCommandDefault" => {
            "\\ProvideTextCommandDefault{\\textbaz}{BAZ}A \\textbaz{} B".into()
        }
        _ => with_arguments(name, arguments, "1pt"),
    }
}

/// The natbib citation commands, which `\usepackage{natbib}` defines.
fn natbib_command(name: &str) -> bool {
    name != "cite"
        && (name.starts_with("cite") || name.starts_with("Cite"))
        && name != "citation"
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
            // `letter` is the one text environment that exists in exactly
            // one class, and it takes a mandatory recipient argument.
            Mode::Text if e.name == "letter" => (
                LETTER_DOCUMENT.replace('#', ""),
                "environment 'letter' is not implemented".to_string(),
            ),
            // beamer's blocks and columns exist only under beamer, like the
            // class's commands: exercised inside a frame of a deck.
            Mode::Text if supported::BEAMER_CLASS_ENVIRONMENTS.contains(&e.name) => (
                BEAMER_DOCUMENT.replace(
                    '#',
                    &match e.name {
                        "columns" => "\\begin{columns}\\column{.5\\textwidth}a\\end{columns}".to_string(),
                        "column" => "\\begin{columns}\\begin{column}{.5\\textwidth}a\\end{column}\\end{columns}".to_string(),
                        name => format!("\\begin{{{name}}}{{T}}a\\end{{{name}}}"),
                    },
                ),
                format!("environment '{}' is", e.name),
            ),
            // beamer's overlay environments exist only inside a frame of a
            // beamer deck, like the class's commands (`beamer_probe`).
            Mode::Text if supported::BEAMER_OVERLAY_ENVIRONMENTS.contains(&e.name) => (
                BEAMER_DOCUMENT.replace('#', &format!("\\begin{{{0}}}<2>a\\end{{{0}}}", e.name)),
                format!("environment '{}' is not implemented", e.name),
            ),
            // `longtable` exists only with its package and takes a column
            // specification, like `tabular`.
            Mode::Text if e.name == "longtable" => (
                "\\usepackage{longtable}\\begin{longtable}{cc}a&b\\end{longtable}".into(),
                "environment 'longtable' is not implemented".to_string(),
            ),
            // `CJK`/`CJK*` exist only with CJKutf8 and take the encoding
            // and family arguments.
            Mode::Text if e.name == "CJK" || e.name == "CJK*" => (
                format!("\\usepackage{{CJKutf8}}\\begin{{{0}}}{{UTF8}}{{min}}a\\end{{{0}}}", e.name),
                format!("environment '{}' ", e.name),
            ),
            // `tabularx` likewise exists only with its package, and takes a
            // target width before the column specification (#901).
            Mode::Text if e.name == "tabularx" => (
                "\\usepackage{tabularx}\\begin{tabularx}{\\linewidth}{cX}a&b\\end{tabularx}".into(),
                "environment 'tabularx' is not implemented".to_string(),
            ),
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

// ---- class scope (completion gating) --------------------------------------
// Adding a class-scoped family to the inventory (beamer's `\frametitle`,
// `\alert`, ...) changed completion for every document: `\fra` offered
// `\frametitle` ahead of `\frac`, `\a` offered `\alert` ahead of `\alpha`.
// The inventory records the dependency (`requires_class`); completion has to
// respect it. These tests pin the data that gate reads.

/// The scope list is exactly the parser's real gate: every name here is
/// diagnosed outside `\documentclass{letter}` (`letter_probe` builds its
/// document for the same set), and the kernel neighbour `hangfrom` — which
/// sits beside them in `BUILT_INS` — stays universal.
#[test]
fn class_scope_matches_the_parser_gate() {
    let inventory = supported::inventory();
    let scoped: BTreeSet<String> = inventory
        .commands
        .iter()
        .filter(|c| c.requires_class.is_some())
        .map(|c| c.name.to_string())
        .collect();
    let listed: BTreeSet<String> = supported::LETTER_CLASS_COMMANDS
        .iter()
        .chain(supported::BEAMER_CLASS_COMMANDS)
        .map(|s| s.to_string())
        .collect();
    assert_eq!(scoped, listed, "scope must be exactly the letter and beamer gates");
    for name in supported::BEAMER_CLASS_COMMANDS {
        let command = inventory
            .commands
            .iter()
            .find(|c| c.name == *name)
            .unwrap_or_else(|| panic!("\\{name} is not in the inventory"));
        assert_eq!(
            command.requires_class,
            Some("beamer"),
            "\\{name} is gated on the beamer class"
        );
        assert!(
            beamer_probe(name, "{}").is_some(),
            "\\{name} must go through the parser's beamer gate"
        );
    }
    for name in supported::LETTER_CLASS_COMMANDS {
        let command = inventory
            .commands
            .iter()
            .find(|c| c.name == *name)
            .unwrap_or_else(|| panic!("\\{name} is not in the inventory"));
        assert_eq!(
            command.requires_class,
            Some("letter"),
            "\\{name} is gated on the letter class"
        );
        assert!(
            letter_probe(name, "{}").is_some(),
            "\\{name} must go through the parser's letter gate"
        );
    }
    // The everyday commands — and the kernel neighbour — stay universal.
    for name in [
        "frac",
        "alpha",
        "aleph",
        "allowdisplaybreaks",
        "allowbreak",
        "section",
        "hangfrom",
        "today",
    ] {
        let command = inventory
            .commands
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("\\{name} is not in the inventory"));
        assert_eq!(command.requires_class, None, "\\{name} is universal");
    }
}

/// The offer rule completion mirrors: a scoped command is hidden only when
/// the document class is known and different. An unknown class (a fragment
/// with no `\documentclass`, as in the editor's prefix tests) keeps today's
/// table order untouched.
#[test]
fn offer_rule_hides_scoped_commands_only_under_another_class() {
    let inventory = supported::inventory();
    let by_name = |name: &str| {
        inventory
            .commands
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("\\{name} is not in the inventory"))
    };
    let (frac, opening) = (by_name("frac"), by_name("opening"));
    assert!(frac.offered_in_class(None));
    assert!(frac.offered_in_class(Some("article")));
    assert!(frac.offered_in_class(Some("letter")));
    assert!(opening.offered_in_class(None), "unknown class gates nothing");
    assert!(
        !opening.offered_in_class(Some("article")),
        "\\opening must not be offered in an article"
    );
    assert!(opening.offered_in_class(Some("letter")));
}

/// Environments scope the same way, on the same probes: `letter` is
/// letter.cls's, the blocks, columns and overlay environments are beamer's,
/// and the ones that merely behave differently under beamer (`frame`,
/// `figure`, `table`) stay universal. Same offer rule as the commands.
#[test]
fn environment_class_scope_matches_the_parser_gate() {
    let inventory = supported::inventory();
    let scoped: BTreeSet<String> = inventory
        .environments
        .iter()
        .filter(|e| e.requires_class.is_some())
        .map(|e| e.name.to_string())
        .collect();
    let listed: BTreeSet<String> = ["letter"]
        .iter()
        .chain(supported::BEAMER_CLASS_ENVIRONMENTS)
        .chain(supported::BEAMER_OVERLAY_ENVIRONMENTS)
        .map(|s| s.to_string())
        .collect();
    assert_eq!(scoped, listed, "scope must be exactly the letter and beamer gates");
    let by_name = |name: &str| {
        inventory
            .environments
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("environment {name} is not in the inventory"))
    };
    assert_eq!(by_name("letter").requires_class, Some("letter"));
    for name in supported::BEAMER_CLASS_ENVIRONMENTS
        .iter()
        .chain(supported::BEAMER_OVERLAY_ENVIRONMENTS)
    {
        assert_eq!(by_name(name).requires_class, Some("beamer"), "{name} is gated on the beamer class");
    }
    for name in ["frame", "figure", "table", "itemize", "equation"] {
        assert_eq!(by_name(name).requires_class, None, "{name} is universal");
    }
    let (itemize, invisibleenv) = (by_name("itemize"), by_name("invisibleenv"));
    for class in [None, Some("article"), Some("beamer")] {
        assert!(itemize.offered_in_class(class), "itemize is universal: offered under {class:?}");
    }
    assert!(invisibleenv.offered_in_class(None), "unknown class gates nothing");
    assert!(!invisibleenv.offered_in_class(Some("article")));
    assert!(invisibleenv.offered_in_class(Some("beamer")));
}

/// The `requires_class` field reaches `--supported json`, the Mac completion
/// vocabulary's data source, without moving anything else: the schema marker
/// stays `flashtex-supported-latex/1` (the sync script greps for it and the
/// Swift decoder asserts it), universal entries gain no key, and every scoped
/// entry carries its class.
#[test]
fn requires_class_is_emitted_in_supported_json() {
    let inventory = supported::inventory();
    let parsed =
        json::parse(&supported::render_json(&inventory)).expect("--supported json is valid JSON");
    assert_eq!(
        parsed.get("schema").and_then(|v| v.as_str()),
        Some("flashtex-supported-latex/1"),
        "schema bump would break the Swift decoder and the sync script"
    );
    let commands = parsed
        .get("commands")
        .and_then(|v| v.as_arr())
        .expect("commands array");
    assert_eq!(commands.len(), inventory.commands.len());
    for entry in commands {
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .expect("command name");
        let model = inventory
            .commands
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("JSON-only command \\{name}"));
        match model.requires_class {
            Some(class) => assert_eq!(
                entry.get("requires_class").and_then(|v| v.as_str()),
                Some(class),
                "\\{name} must carry its class in JSON"
            ),
            None => assert!(
                entry.get("requires_class").is_none(),
                "\\{name} is universal and must not gain the key"
            ),
        }
    }
    let environments = parsed
        .get("environments")
        .and_then(|v| v.as_arr())
        .expect("environments array");
    assert_eq!(environments.len(), inventory.environments.len());
    for entry in environments {
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .expect("environment name");
        let model = inventory
            .environments
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("JSON-only environment {name}"));
        match model.requires_class {
            Some(class) => assert_eq!(
                entry.get("requires_class").and_then(|v| v.as_str()),
                Some(class),
                "environment {name} must carry its class in JSON"
            ),
            None => assert!(
                entry.get("requires_class").is_none(),
                "environment {name} is universal and must not gain the key"
            ),
        }
    }
}

/// The article-visibility contract the completion gate implements: under
/// `article`, every prefix-visible name is universal, so a future
/// class-scoped family can never outrank `\frac` on `\fra` or `\alpha` on
/// `\a` again. (`\address` is the letter-scoped witness for the `\a`
/// prefix; beamer's `\alert`/`\frametitle` register the same way.)
#[test]
fn article_prefix_completion_has_no_class_scoped_command() {
    let inventory = supported::inventory();
    let visible = |class: Option<&str>, prefix: &str| -> Vec<&str> {
        inventory
            .commands
            .iter()
            .filter(|c| c.renders && c.offered_in_class(class))
            .map(|c| c.name)
            .filter(|name| name.starts_with(prefix))
            .collect()
    };
    for prefix in ["a", "c", "o", "s", "fra", "al"] {
        assert!(
            visible(Some("article"), prefix)
                .iter()
                .all(|name| visible(None, prefix).contains(name)),
            "gating only removes, never adds, for prefix {prefix:?}"
        );
        assert!(
            !visible(Some("article"), prefix).contains(&"opening"),
            "prefix {prefix:?}"
        );
    }
    assert!(
        !visible(Some("article"), "a").contains(&"address"),
        "the letter-scoped \\address must not be offered in an article"
    );
    for name in ["allowdisplaybreaks", "allowbreak", "alpha", "aleph"] {
        assert!(
            visible(Some("article"), "a").contains(&name),
            "\\{name} stays offered in an article"
        );
    }
    assert!(
        visible(Some("article"), "fra").contains(&"frac"),
        "\\frac stays offered in an article"
    );
    assert!(
        visible(Some("letter"), "o").contains(&"opening"),
        "the gate opens under the owning class"
    );
}
