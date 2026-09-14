//! `protocol/fixtures/caret-context-v1.json` drives this suite, the Mac's
//! `CaretContextTests`, and the pdflatex oracle
//! (`scripts/caret_context_oracle.py`). One table, three consumers: the two
//! implementations cannot drift from each other, and neither can drift from
//! what pdflatex actually accepts.
use flashtex_bridge::caret::{self, CaretContext, Mode, Wrap};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol/fixtures/caret-context-v1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture readable")).expect("json")
}

fn mode(name: &str) -> Mode {
    match name {
        "text" => Mode::Text,
        "inline_math" => Mode::InlineMath,
        "display_math" => Mode::DisplayMath,
        "verbatim" => Mode::Verbatim,
        "comment" => Mode::Comment,
        other => panic!("unknown mode {other}"),
    }
}

fn wrap(name: &str) -> Wrap {
    match name {
        "display" => Wrap::Display,
        "inline" => Wrap::Inline,
        "already_math" => Wrap::AlreadyMath,
        "literal" => Wrap::Literal,
        other => panic!("unknown wrap {other}"),
    }
}

/// The caret context every fixture case expects is exactly what `derive`
/// produces from the document prefix.
#[test]
fn derives_every_fixture_context() {
    let data = fixture();
    for case in data["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let before = case["before"].as_str().unwrap();
        let after = case["after"].as_str().unwrap();
        let preamble = case
            .get("preamble")
            .and_then(Value::as_str)
            .unwrap_or_else(|| data["preamble"].as_str().unwrap());
        // The preamble is part of the document the caret lives in, so amsmath
        // is discovered the same way the bridge discovers it in a real project.
        let document = format!("{preamble}\\begin{{document}}\n{before}{after}");
        let caret = preamble.len() + "\\begin{document}\n".len() + before.len();
        let got = caret::derive(&document, caret);
        let want = &case["context"];

        assert_eq!(got.mode, mode(want["mode"].as_str().unwrap()), "{name}: mode");
        assert_eq!(got.wrap, wrap(want["wrap"].as_str().unwrap()), "{name}: wrap");
        assert_eq!(
            got.delimiter.as_deref(),
            want["delimiter"].as_str(),
            "{name}: delimiter"
        );
        // `document` is open in every real file; the fixture lists only the
        // environments inside the body.
        let got_environment = got.environment.as_deref().filter(|e| *e != "document");
        assert_eq!(
            got_environment,
            want["environment"].as_str(),
            "{name}: environment"
        );
        assert_eq!(
            got.amsmath,
            want["amsmath"].as_bool().unwrap(),
            "{name}: amsmath"
        );
        let want_envs: Vec<&str> = want["environments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let got_envs: Vec<&str> = got
            .environments
            .iter()
            .map(String::as_str)
            .filter(|e| *e != "document")
            .collect();
        assert_eq!(got_envs, want_envs, "{name}: environments");
    }
}

/// The text actually inserted for every fixture case, including the refusals.
#[test]
fn normalizes_every_fixture_proposal() {
    let data = fixture();
    for case in data["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let want = &case["context"];
        let context = CaretContext {
            mode: mode(want["mode"].as_str().unwrap()),
            delimiter: want["delimiter"].as_str().map(str::to_owned),
            environment: want["environment"].as_str().map(str::to_owned),
            environments: want["environments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect(),
            amsmath: want["amsmath"].as_bool().unwrap(),
            wrap: wrap(want["wrap"].as_str().unwrap()),
        };
        let got = caret::normalize(case["proposal"].as_str().unwrap(), &context);
        assert_eq!(
            got.text.as_deref(),
            case["insert"].as_str(),
            "{name}: inserted text"
        );
        let want_advisories: Vec<&str> = case["advisories"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for advisory in &want_advisories {
            assert!(
                got.advisories.iter().any(|a| a == advisory),
                "{name}: missing advisory {advisory:?}, got {:?}",
                got.advisories
            );
        }
        if want_advisories.is_empty() {
            assert!(
                got.advisories.is_empty(),
                "{name}: unexpected advisories {:?}",
                got.advisories
            );
        }
    }
}

/// The property the owner's report is really about, checked beyond the table:
/// whatever a recogniser returns, the text inserted at a math caret never
/// carries a math delimiter, and no insertion is ever unbalanced.
#[test]
fn no_insertion_ever_double_wraps_or_unbalances() {
    let proposals = [
        "$x^2$",
        "$$x^2$$",
        "\\( x^2 \\)",
        "\\[ x^2 \\]",
        "\\begin{equation} x^2 \\end{equation}",
        "$\\frac{a}{b}$",
        "x^2",
        "\\[\\[x\\]\\]",
        "$$ \\[ x \\] $$",
    ];
    let carets = [
        ("Let $a + ", " + b$.\n"),
        ("\\[ a + ", " + b \\]\n"),
        ("\\begin{equation}\n", "\n\\end{equation}\n"),
        ("Prose ", " prose.\n"),
        ("\\begin{tabular}{cc} a & ", " \\\\ \\end{tabular}\n"),
    ];
    for (before, after) in carets {
        let document = format!("\\documentclass{{article}}\n\\begin{{document}}\n{before}{after}");
        let caret = document.find(before).unwrap() + before.len();
        let context = caret::derive(&document, caret);
        for proposal in proposals {
            let result = caret::normalize(proposal, &context);
            let Some(text) = result.text else { continue };
            if context.wrap == Wrap::AlreadyMath {
                assert!(
                    !text.contains('$') && !text.contains("\\[") && !text.contains("\\("),
                    "double wrap at {before:?} from {proposal:?}: {text:?}"
                );
            }
            if context.wrap == Wrap::Inline {
                assert!(
                    !text.contains("\\["),
                    "display math in a cell at {before:?} from {proposal:?}: {text:?}"
                );
            }
            // Whatever came out, the document around it still balances.
            let spliced = format!("{before}{text}{after}");
            assert_eq!(
                spliced.matches('$').count() % 2,
                0,
                "unbalanced $ at {before:?} from {proposal:?}: {text:?}"
            );
        }
    }
}

/// Normalising twice changes nothing. `normalize` runs once (at conversion) and
/// `insertable` re-checks at insertion; this pins the invariant that makes that
/// split safe, and catches the \mbox{\mbox{…}} regression directly.
#[test]
fn normalizing_is_idempotent() {
    let data = fixture();
    for case in data["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let want = &case["context"];
        let context = CaretContext {
            mode: mode(want["mode"].as_str().unwrap()),
            delimiter: want["delimiter"].as_str().map(str::to_owned),
            environment: want["environment"].as_str().map(str::to_owned),
            environments: vec![],
            amsmath: want["amsmath"].as_bool().unwrap(),
            wrap: wrap(want["wrap"].as_str().unwrap()),
        };
        let once = caret::normalize(case["proposal"].as_str().unwrap(), &context);
        let Some(text) = once.text else { continue };
        let twice = caret::normalize(&text, &context);
        assert_eq!(twice.text.as_deref(), Some(text.as_str()), "{name}: not idempotent");
        assert_eq!(
            caret::insertable(&text, &context).as_deref(),
            Some(text.as_str()),
            "{name}: normalised text refused at insertion"
        );
    }
}

/// The sentence the recogniser is given must actually name the rule, because
/// that is the half of the fix that stops bad LaTeX being produced at all.
#[test]
fn the_prompt_sentence_states_the_rule_for_every_caret() {
    let cases = [
        ("Let $a + ", "NO delimiters"),
        ("\\begin{tabular}{cc} a & ", "NOT legal here"),
        ("\\begin{verbatim}\n", "literally"),
        ("Prose.\n\n", "\\["),
    ];
    for (before, expected) in cases {
        let document = format!("\\documentclass{{article}}\n\\begin{{document}}\n{before}\n");
        let caret = document.find(before).unwrap() + before.len();
        let sentence = caret::derive(&document, caret).describe();
        assert!(
            sentence.contains(expected),
            "caret {before:?}: {sentence:?} does not state {expected:?}"
        );
    }
}
