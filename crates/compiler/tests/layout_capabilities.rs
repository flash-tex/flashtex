use flashtex_compiler::json::{self, Value};
use flashtex_compiler::math::FRACTION_RULE_CHAR;
use flashtex_compiler::protocol::handle_line;

fn request(
    project: &str,
    id: &str,
    revision: i64,
    text: &str,
    capabilities: Option<Value>,
) -> String {
    let mut document = Value::obj();
    document.set("path", json::str_("main.tex"));
    document.set("text", json::str_(text));

    let mut payload = Value::obj();
    payload.set("project_id", json::str_(project));
    payload.set("revision", Value::Num(revision as f64));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![document]));
    if let Some(capabilities) = capabilities {
        payload.set("layout_capabilities", capabilities);
    }

    let mut envelope = Value::obj();
    envelope.set("protocol_version", Value::Num(1.0));
    envelope.set("id", json::str_(id));
    envelope.set("type", json::str_("compile"));
    envelope.set("payload", payload);
    json::write(&envelope)
}

fn capability_list(capabilities: &[&str]) -> Value {
    Value::Arr(
        capabilities
            .iter()
            .map(|capability| json::str_(*capability))
            .collect(),
    )
}

fn reply(line: &str) -> Value {
    json::parse(&handle_line(line)).expect("compiler response must be valid JSON")
}

fn items(response: &Value) -> Vec<&Value> {
    response
        .get("payload")
        .and_then(|payload| payload.get("pages"))
        .and_then(Value::as_arr)
        .into_iter()
        .flatten()
        .flat_map(|page| {
            page.get("items")
                .and_then(Value::as_arr)
                .into_iter()
                .flatten()
        })
        .collect()
}

fn diagnostic_messages(response: &Value) -> Vec<&str> {
    response
        .get("payload")
        .and_then(|payload| payload.get("diagnostics"))
        .and_then(Value::as_arr)
        .into_iter()
        .flatten()
        .filter_map(|diagnostic| diagnostic.get("message").and_then(Value::as_str))
        .collect()
}

fn num(value: &Value, key: &str) -> f64 {
    match value.get(key) {
        Some(Value::Num(number)) => *number,
        _ => panic!("{key} must be a number"),
    }
}

#[test]
fn omitted_capabilities_preserve_the_legacy_fraction_route() {
    let response = reply(&request(
        "layout-caps-legacy",
        "legacy",
        1,
        "$\\frac{1}{2}$",
        None,
    ));
    let payload = response.get("payload").unwrap();
    assert!(payload.get("layout_capabilities").is_none());
    assert!(items(&response).iter().all(|item| {
        item.get("kind").and_then(Value::as_str) == Some("text") && item.get("font").is_none()
    }));
    assert!(items(&response).iter().any(|item| {
        item.get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains(FRACTION_RULE_CHAR))
    }));
    assert!(diagnostic_messages(&response)
        .iter()
        .any(|message| message.contains("will not survive PDF export")));
}

#[test]
fn rules_v1_emits_a_sourced_top_left_rectangle_without_the_legacy_glyph() {
    let source = "$\\frac{1}{2}$";
    let legacy = reply(&request(
        "layout-caps-rule-legacy",
        "legacy-geometry",
        1,
        source,
        None,
    ));
    let legacy_rule = items(&legacy)
        .into_iter()
        .find(|item| {
            item.get("text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains(FRACTION_RULE_CHAR))
        })
        .unwrap();

    let response_line = handle_line(&request(
        "layout-caps-rule",
        "typed-rule",
        1,
        source,
        Some(capability_list(&["rules-v1"])),
    ));
    assert!(!response_line.contains(FRACTION_RULE_CHAR));
    let response = json::parse(&response_line).unwrap();
    let rule = items(&response)
        .into_iter()
        .find(|item| item.get("kind").and_then(Value::as_str) == Some("rule"))
        .expect("fraction must emit a typed rule");

    let x = num(rule, "x_pt");
    let y = num(rule, "y_pt");
    let width = num(rule, "width_pt");
    let height = num(rule, "height_pt");
    for coordinate in [x, y, width, height] {
        assert!(coordinate.is_finite());
        assert!(coordinate.abs() <= 1_000_000.0);
    }
    assert!(width > 0.0 && height > 0.0);
    assert_eq!(x, num(legacy_rule, "x_pt"));
    assert!((y + height - num(legacy_rule, "baseline_y_pt")).abs() < 0.011);

    let source_range = rule.get("source").unwrap();
    let start = num(source_range, "start_byte") as usize;
    let end = num(source_range, "end_byte") as usize;
    assert_eq!(&source[start..end], "\\frac");
    assert!(!diagnostic_messages(&response)
        .iter()
        .any(|message| message.contains("fraction rules")));
}

#[test]
fn font_hints_report_the_faces_selected_by_layout() {
    let response = reply(&request(
        "layout-caps-font",
        "fonts",
        1,
        "\\section{Heading}\nBody",
        Some(capability_list(&["font-hints-v1"])),
    ));
    for item in items(&response) {
        assert!(item.get("font").is_some(), "every text item needs a hint");
    }

    let heading = items(&response)
        .into_iter()
        .find(|item| item.get("text").and_then(Value::as_str) == Some("Heading"))
        .unwrap();
    let heading_font = heading.get("font").unwrap();
    assert_eq!(
        heading_font.get("family").and_then(Value::as_str),
        Some("Times-Bold")
    );
    assert_eq!(
        heading_font.get("weight").and_then(Value::as_str),
        Some("bold")
    );
    assert_eq!(
        heading_font.get("style").and_then(Value::as_str),
        Some("normal")
    );

    let body = items(&response)
        .into_iter()
        .find(|item| item.get("text").and_then(Value::as_str) == Some("Body"))
        .unwrap();
    assert_eq!(
        body.get("font")
            .unwrap()
            .get("family")
            .and_then(Value::as_str),
        Some("Times-Roman")
    );
}

#[test]
fn unknown_and_unrequested_capabilities_never_change_shapes() {
    let unknown = reply(&request(
        "layout-caps-unknown",
        "unknown",
        1,
        "$\\frac{1}{2}$",
        Some(capability_list(&["future-shapes-v9"])),
    ));
    assert_eq!(
        unknown
            .get("payload")
            .unwrap()
            .get("layout_capabilities")
            .and_then(Value::as_arr)
            .map(Vec::len),
        Some(0)
    );
    assert!(items(&unknown)
        .iter()
        .all(|item| item.get("kind").and_then(Value::as_str) == Some("text")));

    let rules_only = reply(&request(
        "layout-caps-no-font",
        "rules-no-font",
        1,
        "text $\\frac{1}{2}$",
        Some(capability_list(&["rules-v1"])),
    ));
    assert!(items(&rules_only)
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("text"))
        .all(|item| item.get("font").is_none()));
}

#[test]
fn malformed_capability_fields_each_produce_an_explicit_diagnostic() {
    let too_many = Value::Arr(
        (0..17)
            .map(|index| json::str_(format!("unknown-{index}")))
            .collect(),
    );
    let sixty_five_bytes = format!("{}é", "a".repeat(63));
    let malformed = [
        Value::Bool(true),
        too_many,
        capability_list(&[&sixty_five_bytes]),
        capability_list(&[""]),
        capability_list(&["rules-v1", "rules-v1"]),
    ];

    for (index, capabilities) in malformed.into_iter().enumerate() {
        let response = reply(&request(
            &format!("layout-caps-malformed-{index}"),
            &format!("malformed-{index}"),
            1,
            "text",
            Some(capabilities),
        ));
        let payload = response.get("payload").unwrap();
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some("failed")
        );
        assert!(
            !diagnostic_messages(&response).is_empty(),
            "malformed case {index} must explain its rejection"
        );
    }
}

#[test]
fn capability_switching_on_one_session_cannot_leak_typed_rules() {
    let source = "$\\frac{1}{2}$";
    let extended = reply(&request(
        "layout-caps-switch",
        "switch-extended",
        1,
        source,
        Some(capability_list(&["rules-v1"])),
    ));
    assert!(items(&extended)
        .iter()
        .any(|item| item.get("kind").and_then(Value::as_str) == Some("rule")));

    let legacy = reply(&request(
        "layout-caps-switch",
        "switch-legacy",
        2,
        source,
        None,
    ));
    assert!(items(&legacy)
        .iter()
        .all(|item| item.get("kind").and_then(Value::as_str) == Some("text")));
    assert!(items(&legacy).iter().any(|item| {
        item.get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains(FRACTION_RULE_CHAR))
    }));
    assert!(diagnostic_messages(&legacy)
        .iter()
        .any(|message| message.contains("will not survive PDF export")));
}

#[test]
fn incremental_and_clean_protocol_output_match_for_the_same_capability_set() {
    let project = "layout-caps-incremental-clean";
    let capabilities = capability_list(&["rules-v1", "font-hints-v1"]);
    let _old = handle_line(&request(
        project,
        "same-output",
        1,
        "First paragraph.\n\n$\\frac{1}{2}$ old tail.",
        Some(capabilities.clone()),
    ));
    let final_request = request(
        project,
        "same-output",
        2,
        "First paragraph.\n\n$\\frac{1}{2}$ changed tail.",
        Some(capabilities.clone()),
    );
    let incremental = handle_line(&final_request);

    // Force this least-recently-used key out of the bounded warm cache, making
    // the identical request below a clean compile without a test-only cache API.
    for index in 0..20 {
        let filler = request(
            &format!("layout-caps-evict-{index}"),
            "filler",
            1,
            "filler text",
            Some(capabilities.clone()),
        );
        let _ = handle_line(&filler);
    }
    let clean = handle_line(&final_request);
    assert_eq!(incremental.as_bytes(), clean.as_bytes());
}

#[test]
fn typed_rules_remove_only_the_fraction_export_warning() {
    let response = reply(&request(
        "layout-caps-other-glyph",
        "other-glyph",
        1,
        "日 $\\frac{1}{2}$",
        Some(capability_list(&["rules-v1"])),
    ));
    let messages = diagnostic_messages(&response);
    assert!(messages.iter().any(|message| message.contains("U+65E5")));
    assert!(!messages.iter().any(|message| message.contains("U+2500")));
}
