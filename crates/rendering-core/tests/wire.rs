use flashtex_rendering_core::{wire::*, *};
use serde_json::json;
fn offer() -> Envelope {
    parse_validated(include_bytes!("fixtures/capabilities.json"), None).unwrap()
}
#[test]
fn validated_glyph_and_source_roundtrip_is_canonical_and_lossless() {
    let offer = offer();
    let envelope = parse_validated(
        include_bytes!("fixtures/synthetic-display-list.json"),
        Some(&offer),
    )
    .unwrap();
    let bytes = serialize_validated(&envelope, Some(&offer), MAX_MESSAGE_BYTES).unwrap();
    let decoded = parse_validated(&bytes, Some(&offer)).unwrap();
    assert_eq!(
        bytes,
        serialize_validated(&decoded, Some(&offer), MAX_MESSAGE_BYTES).unwrap()
    );
    if let Message::DisplayList(list) = decoded.message {
        if let Item::GlyphRun(run) = &list.pages[0].items[0] {
            assert_eq!(run.text, "office e\u{301}");
            assert_eq!(run.clusters[0].sources.as_ref().unwrap()[0].end_byte, 10);
            assert_eq!(run.glyphs[0].gid, 1);
        } else {
            panic!()
        }
    } else {
        panic!()
    }
}
#[test]
fn unknown_primitives_and_messages_have_typed_errors() {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/synthetic-display-list.json")).unwrap();
    value["payload"]["pages"][0]["items"][0]["kind"] = json!("image");
    assert_eq!(
        parse_validated(&serde_json::to_vec(&value).unwrap(), Some(&offer())).unwrap_err(),
        WireError::UnsupportedPrimitive {
            page_index: 0,
            item_index: 0,
            kind: "image".into()
        }
    );
    value["type"] = json!("future_frame");
    assert!(matches!(
        parse_validated(&serde_json::to_vec(&value).unwrap(), Some(&offer())),
        Err(WireError::UnsupportedMessage { .. })
    ));
}
#[test]
fn unsupported_versions_are_not_decoded_as_current() {
    let value = json!({"protocol_version":1,"id":"a","type":"render_capabilities","payload":{}});
    assert_eq!(
        parse_validated(&serde_json::to_vec(&value).unwrap(), None).unwrap_err(),
        WireError::UnsupportedVersion { version: 1 }
    );
}
#[test]
fn serializers_reject_invalid_inmemory_fields_and_never_return_partial_bytes() {
    let offer = offer();
    let mut envelope = parse_validated(
        include_bytes!("fixtures/synthetic-display-list.json"),
        Some(&offer),
    )
    .unwrap();
    assert_eq!(
        serialize_validated(&envelope, Some(&offer), 10).unwrap_err(),
        WireError::TooLarge { limit: 10 }
    );
    if let Message::DisplayList(list) = &mut envelope.message {
        list.pages[0].width = Tick(-1);
    }
    assert!(matches!(
        serialize_validated(&envelope, Some(&offer), MAX_MESSAGE_BYTES),
        Err(WireError::Semantic { .. })
    ));
}
#[test]
fn malformed_truncated_and_missing_offer_are_distinct_failures() {
    assert!(matches!(
        parse_validated(b"{", None),
        Err(WireError::Malformed { .. })
    ));
    assert!(matches!(
        parse_validated(include_bytes!("fixtures/synthetic-display-list.json"), None),
        Err(WireError::Semantic { .. })
    ));
}
#[test]
fn offers_selections_and_explicit_rejections_roundtrip() {
    let offer = offer();
    for (kind, payload) in [
        (
            "render_format_selected",
            json!({"render_format":"display-list-v2","required_features":["glyph_run"]}),
        ),
        (
            "render_format_rejected",
            json!({"code":"unsupported_feature","message":"image is not negotiated"}),
        ),
    ] {
        let bytes = serde_json::to_vec(
            &json!({"protocol_version":2,"id":"hello","type":kind,"payload":payload}),
        )
        .unwrap();
        let message = parse_validated(&bytes, Some(&offer)).unwrap();
        let encoded = serialize_validated(&message, Some(&offer), MAX_MESSAGE_BYTES).unwrap();
        parse_validated(&encoded, Some(&offer)).unwrap();
    }
    let bytes = serialize_validated(&offer, None, MAX_MESSAGE_BYTES).unwrap();
    parse_validated(&bytes, None).unwrap();
}
// PROPOSAL display-list-v2-window: a windowed display list round-trips
// through the wire boundary losslessly — `window` and `resident: false`
// survive, and resident pages gain no `resident` key.
#[test]
fn windowed_display_list_roundtrips_byte_preserving() {
    let offer = offer();
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/synthetic-display-list.json")).unwrap();
    let mut elided_page = value["payload"]["pages"][0].clone();
    elided_page["number"] = json!(2);
    elided_page.as_object_mut().unwrap().remove("items");
    elided_page["resident"] = json!(false);
    value["payload"]["pages"]
        .as_array_mut()
        .unwrap()
        .push(elided_page);
    value["payload"]["window"] =
        json!({"first_page": 1, "page_count": 1, "document_page_count": 2});
    let envelope = parse_validated(&serde_json::to_vec(&value).unwrap(), Some(&offer)).unwrap();
    let bytes = serialize_validated(&envelope, Some(&offer), MAX_MESSAGE_BYTES).unwrap();
    let decoded = parse_validated(&bytes, Some(&offer)).unwrap();
    assert_eq!(
        bytes,
        serialize_validated(&decoded, Some(&offer), MAX_MESSAGE_BYTES).unwrap()
    );
    let round_tripped: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        round_tripped["payload"]["window"],
        json!({"first_page": 1, "page_count": 1, "document_page_count": 2})
    );
    let pages = round_tripped["payload"]["pages"].as_array().unwrap();
    assert!(pages[0]["items"].is_array());
    assert!(pages[0].get("resident").is_none());
    assert!(pages[1].get("items").is_none());
    assert_eq!(pages[1]["resident"], json!(false));
    if let Message::DisplayList(list) = decoded.message {
        assert!(list.pages[0].is_resident());
        assert!(list.pages[0].items.len() == 1);
        assert!(!list.pages[1].is_resident());
        assert!(list.pages[1].items.is_empty());
        assert_eq!(list.window.unwrap().document_page_count, 2);
    } else {
        panic!("fixture is display list")
    }
}
// PROPOSAL display-list-v2-window: an unwindowed list's serialization stays
// byte-identical to before — no `window` or `resident` key is ever emitted.
#[test]
fn unwindowed_display_list_serialization_never_gains_window_or_resident_keys() {
    let offer = offer();
    let envelope = parse_validated(
        include_bytes!("fixtures/synthetic-display-list.json"),
        Some(&offer),
    )
    .unwrap();
    let bytes = serialize_validated(&envelope, Some(&offer), MAX_MESSAGE_BYTES).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(!text.contains("\"window\""));
    assert!(!text.contains("\"resident\""));
}
