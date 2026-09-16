//! Generates UNSUPPORTED.md: what this compiler does not implement, with a
//! reproducible case and the exact diagnostic for each.
//!
//! The master plan requires reporting in three labels — implemented and tested,
//! implemented but unverified, and required but outstanding. This file produces
//! the third list from the real compiler, so the honest inventory cannot drift
//! away from what the code actually does.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;
use std::fmt::Write as _;

struct Case {
    feature: &'static str,
    input: &'static str,
    /// Why this matters to a student's document, not just to the compiler.
    consequence: &'static str,
}

const CASES: &[Case] = &[
    Case {
        feature: r"\usepackage — package implementations",
        // Not amsmath: `parser::package_matches_layout` accepts it, because
        // this crate parses and sets the amsmath constructs and reports the
        // ones it does not at their own span. `fancyhdr` is modelled by
        // neither this crate nor the render pipeline, so it is the honest
        // example of a package that is only recognised.
        input: "\\usepackage{fancyhdr}\nText.\n",
        consequence:
            "Any document relying on package-defined commands will report them as unsupported.",
    },
    Case {
        feature: "TikZ diagrams",
        input: "\\begin{tikzpicture}\\draw (0,0) -- (1,1);\\end{tikzpicture}\n",
        consequence: "The demo contract names a small TikZ diagram; it is not implemented.",
    },
    Case {
        feature: r"\includegraphics — image loading",
        input: "\\begin{figure}\\includegraphics{plot.png}\\caption{P}\\end{figure}\n",
        consequence: "Figures lay out and number, but no image is loaded or drawn.",
    },
    Case {
        feature: "longtable — multi-page tables",
        input: "\\begin{longtable}{ll}a & b \\\\ c & d\\end{longtable}\n",
        consequence:
            "tabular is laid out, but longtable's page-breaking tables are typeset as plain text.",
    },
    Case {
        feature: r"\cite and bibliographies",
        input: "Text \\cite{knuth1984}.\n",
        consequence: "Citations do not resolve; .bib files are not read.",
    },
];

fn compile(text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("inventory"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("inv"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply")
}

/// The guarantee that matters: an unsupported feature is never silent.
#[test]
fn every_unsupported_feature_is_reported_explicitly() {
    for case in CASES {
        let reply = compile(case.input);
        let payload = reply.get("payload").expect("payload");
        let diags = payload
            .get("diagnostics")
            .and_then(|v| v.as_arr())
            .cloned()
            .unwrap_or_default();
        assert!(
            !diags.is_empty(),
            "{}: produced no diagnostic — an unsupported feature must never be silent",
            case.feature
        );
        // And the document is still usable: something was typeset.
        let items: usize = payload
            .get("pages")
            .and_then(|v| v.as_arr())
            .map(|p| {
                p.iter()
                    .map(|pg| {
                        pg.get("items")
                            .and_then(|i| i.as_arr())
                            .map_or(0, |i| i.len())
                    })
                    .sum()
            })
            .unwrap_or(0);
        assert!(
            items > 0 || payload.get("status").and_then(|v| v.as_str()) == Some("failed"),
            "{}: produced neither output nor an explicit failure",
            case.feature
        );
    }
}

#[test]
#[ignore = "regenerates the committed unsupported-feature inventory"]
fn generate_unsupported_inventory() {
    let mut out = String::new();
    out.push_str(
        "# What this compiler does not implement\n\n\
         Generated from the real compiler by \
         `cargo test --test unsupported_inventory generate_unsupported_inventory -- --ignored --exact`.\n\n\
         Each entry is a required-but-outstanding item in the master plan's sense. \
         Every one is reported explicitly by the compiler; none fails silently. \
         Documents using these still compile and still produce readable output.\n\n",
    );
    for case in CASES {
        let reply = compile(case.input);
        let payload = reply.get("payload").unwrap();
        let _ = writeln!(out, "## {}\n", case.feature);
        let _ = writeln!(out, "{}\n", case.consequence);
        let _ = writeln!(out, "Input:\n\n```text\n{}```\n", case.input);
        let _ = writeln!(
            out,
            "Status: `{}`\n",
            payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
        );
        out.push_str("Diagnostics:\n\n");
        for d in payload
            .get("diagnostics")
            .and_then(|v| v.as_arr())
            .cloned()
            .unwrap_or_default()
        {
            let msg = d.get("message").and_then(|v| v.as_str()).unwrap_or("");
            let rec = d
                .get("recovery")
                .and_then(|v| v.as_str())
                .unwrap_or("no provisional rendering");
            let _ = writeln!(out, "- `{msg}` — recovery: {rec}");
        }
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/UNSUPPORTED.md"), out)
        .expect("write inventory");
}
