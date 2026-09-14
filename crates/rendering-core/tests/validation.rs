use flashtex_rendering_core::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn offer() -> Envelope {
    parse(include_bytes!("fixtures/capabilities.json")).unwrap()
}
fn value() -> Value {
    serde_json::from_slice(include_bytes!("fixtures/synthetic-display-list.json")).unwrap()
}
fn envelope(value: &Value) -> Result<Envelope> {
    parse(&serde_json::to_vec(value).unwrap())
}
fn valid(value: &Value) -> Result<()> {
    envelope(value)?.validate(Some(&offer()))
}
fn list(value: &Value) -> DisplayList {
    match envelope(value).unwrap().message {
        Message::DisplayList(list) => list,
        _ => panic!("fixture is display list"),
    }
}
fn capabilities() -> Capabilities {
    match offer().message {
        Message::Offer(caps) => caps,
        _ => unreachable!(),
    }
}

#[test]
fn synthetic_logical_clusters_are_lossless_not_a_rendered_font_claim() {
    let value = value();
    valid(&value).unwrap();
    let list = list(&value);
    if let Item::GlyphRun(run) = &list.pages[0].items[0] {
        let extracted: String = run
            .clusters
            .iter()
            .map(|c| &run.text[c.text_start_byte as usize..c.text_end_byte as usize])
            .collect();
        assert_eq!(extracted, "office e\u{301}");
        assert_eq!(run.glyphs.len(), 2);
        assert_eq!(run.clusters.len(), 1);
    } else {
        panic!("glyph fixture missing")
    }
}
#[test]
fn typed_model_roundtrip_preserves_glyphs_geometry_and_sources() {
    let list = list(&value());
    let roundtrip: DisplayList =
        serde_json::from_value(serde_json::to_value(&list).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&list).unwrap(),
        serde_json::to_value(&roundtrip).unwrap()
    );
    roundtrip.validate(&capabilities()).unwrap();
}
#[test]
fn no_implicit_negotiation_and_no_v1_switch() {
    let display = envelope(&value()).unwrap();
    assert!(display.validate(None).is_err());
    let mut old = offer();
    if let Message::Offer(c) = &mut old.message {
        c.render_formats = vec![RenderFormat::RuntimeV1];
    }
    assert!(display.validate(Some(&old)).is_err());
}
#[test]
fn selection_requires_offer_id() {
    let mut selection = json!({"protocol_version":2,"id":"hello","type":"render_format_selected","payload":{"render_format":"display-list-v2","required_features":["glyph_run"]}});
    envelope(&selection)
        .unwrap()
        .validate(Some(&offer()))
        .unwrap();
    selection["id"] = json!("different");
    assert!(envelope(&selection)
        .unwrap()
        .validate(Some(&offer()))
        .is_err());
}
#[test]
fn unsupported_item_message_and_extra_fields_fail_closed() {
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["kind"] = json!("image");
    assert!(envelope(&v).is_err());
    let mut v = value();
    v["type"] = json!("compile_result");
    assert!(envelope(&v).is_err());
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["guessed_font"] = json!("Times");
    assert!(envelope(&v).is_err());
    let mut v = value();
    v["extra"] = json!(true);
    assert!(envelope(&v).is_err());
}
#[test]
fn required_feature_and_actual_primitive_must_agree() {
    let mut v = value();
    v["payload"]["required_features"] = json!(["rgba-srgb", "cluster-actualtext"]);
    assert!(valid(&v).is_err());
    let mut caps = offer();
    if let Message::Offer(c) = &mut caps.message {
        c.features.retain(|f| *f != Feature::GlyphRun);
    }
    assert!(envelope(&value()).unwrap().validate(Some(&caps)).is_err());
}
#[test]
fn exact_ticks_reject_fractional_and_overflowing_values() {
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["glyphs"][0]["origin_x"] = json!(0.1);
    assert!(envelope(&v).is_err());
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["glyphs"][0]["origin_x"] = json!(MAX_EXACT_INTEGER);
    assert!(valid(&v).is_err());
    assert!(Tick(i64::MAX).checked_add(Tick(1)).is_err());
    assert!(Tick(-MAX_EXACT_INTEGER).checked_add(Tick(-1)).is_err());
}
#[test]
fn tex_scaled_points_convert_once_to_pdf_units() {
    // TeX's 72.27pt = exactly one inch = 72 PDF points.
    let tex_point = Tick::from_tex_sp(65536).unwrap();
    assert_eq!(tex_point.0, 1044659);
    assert_eq!(Tick::from_tex_sp(-65536).unwrap().0, -tex_point.0);
    assert!(Tick::from_tex_sp(i64::MAX).is_err());
}
#[test]
fn nonfinite_json_and_excessive_messages_are_rejected() {
    assert!(
        parse(br#"{"protocol_version":2,"id":"a","type":"display_list","payload":NaN}"#).is_err()
    );
    assert!(parse(&vec![b' '; MAX_MESSAGE_BYTES + 1]).is_err());
    let nested = format!("{}0{}", "[".repeat(200), "]".repeat(200));
    assert!(parse(nested.as_bytes()).is_err());
}
#[test]
fn font_identity_gid_and_counts_are_checked() {
    for gid in [0, 3, 65536] {
        let mut v = value();
        v["payload"]["pages"][0]["items"][0]["glyphs"][0]["gid"] = json!(gid);
        assert!(valid(&v).is_err());
    }
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["font_id"] = json!("missing");
    assert!(valid(&v).is_err());
    let mut v = value();
    let font = v["payload"]["fonts"][0].clone();
    v["payload"]["fonts"].as_array_mut().unwrap().push(font);
    assert!(valid(&v).is_err());
}
#[test]
fn clusters_cannot_split_utf8_or_drop_text() {
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["clusters"][0]["text_end_byte"] = json!(9);
    assert!(valid(&v).is_err());
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["glyphs"][0]["cluster"] = json!(4);
    assert!(valid(&v).is_err());
}
#[test]
fn generated_content_requires_unambiguous_provenance() {
    let mut v = value();
    let cluster = &mut v["payload"]["pages"][0]["items"][0]["clusters"][0];
    cluster.as_object_mut().unwrap().remove("sources");
    assert!(valid(&v).is_err());
    v["payload"]["pages"][0]["items"][0]["clusters"][0]["synthetic_reason"] =
        json!("equation number");
    valid(&v).unwrap();
}
#[test]
fn resource_callbacks_are_not_called_for_wrong_hash_and_do_not_claim_paintability() {
    struct FakeFont;
    impl FontValidator for FakeFont {
        fn validate_static_truetype(&self, bytes: &[u8]) -> Result<FontMetadata> {
            assert_eq!(bytes, b"synthetic-font-bytes");
            Ok(FontMetadata {
                units_per_em: 1000,
                glyph_count: 3,
            })
        }
    }
    let mut list = list(&value());
    let docs = BTreeMap::from([(
        "main.tex".into(),
        SourceSnapshot {
            revision: 1,
            text: "office e\u{301}".into(),
        },
    )]);
    let bytes = b"synthetic-font-bytes".to_vec();
    let fonts = BTreeMap::from([("synthetic".into(), bytes.clone())]);
    assert!(list
        .validate_resources(&capabilities(), &docs, &fonts, &FakeFont)
        .is_err());
    list.fonts[0].sha256 = digest(&bytes);
    list.fonts[0].byte_length = bytes.len() as u64;
    let evidence = list
        .validate_resources(&capabilities(), &docs, &fonts, &FakeFont)
        .unwrap();
    assert!(evidence.source_snapshots_verified && evidence.font_resources_verified);
    assert!(!evidence.paintable);
    if let Item::GlyphRun(run) = &mut list.pages[0].items[0] {
        run.clusters[0].sources.as_mut().unwrap()[0].end_byte = 9;
    }
    assert!(list
        .validate_resources(&capabilities(), &docs, &fonts, &FakeFont)
        .is_err());
}
#[test]
fn source_registry_rejects_unknown_paths_and_out_of_bounds_offsets() {
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["clusters"][0]["sources"][0]["path"] = json!("other.tex");
    assert!(valid(&v).is_err());
    let mut v = value();
    v["payload"]["pages"][0]["items"][0]["clusters"][0]["sources"][0]["end_byte"] = json!(11);
    assert!(valid(&v).is_err());
}
// PROPOSAL display-list-v2-window: fail closed. A page decodes only when it
// has `items` (resident) or an explicit `resident: false` and no `items`
// (elided) — never neither, and never both.
#[test]
fn page_without_items_or_explicit_resident_false_is_refused() {
    let mut v = value();
    v["payload"]["pages"][0]
        .as_object_mut()
        .unwrap()
        .remove("items");
    assert!(envelope(&v).is_err());
    // Also refused with both present.
    let mut v = value();
    v["payload"]["pages"][0]["resident"] = json!(false);
    assert!(envelope(&v).is_err());
    // And refused with an explicit `resident: true` (the wire form for a
    // resident page never carries a `resident` key at all).
    let mut v = value();
    v["payload"]["pages"][0]
        .as_object_mut()
        .unwrap()
        .remove("items");
    v["payload"]["pages"][0]["resident"] = json!(true);
    assert!(envelope(&v).is_err());
}
// PROPOSAL display-list-v2-window: residency must agree exactly with the
// negotiated `window`; disagreement is refused rather than silently trusted.
#[test]
fn residency_disagreeing_with_window_is_refused() {
    // Single resident page numbered 1, but the window claims pages 5..6 are
    // the resident ones.
    let mut v = value();
    v["payload"]["window"] = json!({"first_page": 5, "page_count": 1, "document_page_count": 1});
    assert!(valid(&v).is_err());
    // A two-page list where the second page is elided but the window claims
    // both pages are resident.
    let mut v = value();
    let mut elided_page = v["payload"]["pages"][0].clone();
    elided_page["number"] = json!(2);
    elided_page.as_object_mut().unwrap().remove("items");
    elided_page["resident"] = json!(false);
    v["payload"]["pages"].as_array_mut().unwrap().push(elided_page);
    v["payload"]["window"] = json!({"first_page": 1, "page_count": 2, "document_page_count": 2});
    assert!(valid(&v).is_err());
    // An elided page with no window at all is a refusal, not a blank paint.
    let mut v = value();
    let mut elided_page = v["payload"]["pages"][0].clone();
    elided_page.as_object_mut().unwrap().remove("items");
    elided_page["resident"] = json!(false);
    v["payload"]["pages"][0] = elided_page;
    assert!(valid(&v).is_err());
}
#[test]
fn rule_is_explicit_geometry_not_inferred_text() {
    let mut v = value();
    v["payload"]["pages"][0]["items"].as_array_mut().unwrap().push(json!({"kind":"rule","x":0,"top":1,"width":100,"height":2,"paint":{"r":0,"g":0,"b":0,"a":1},"synthetic_reason":"fraction bar"}));
    valid(&v).unwrap();
    v["payload"]["pages"][0]["items"][1]["height"] = json!(0);
    assert!(valid(&v).is_err());
}
