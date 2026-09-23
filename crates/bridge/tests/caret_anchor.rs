//! The automatic destination (`AnchorMode::Caret`, transfer-v1 additive) and
//! the approval-time `wrap` on `capture_prepare_insert`.
//!
//! The owner's report: the Mac pinned the caret for the iPad, the owner kept
//! typing, and the capture was later refused with
//! `destination_reselection_required` although nothing was pinned or edited
//! "at the pin" from their point of view. A zero-width fixed anchor is
//! invalidated by the very keystrokes a caret receives; a caret anchor must
//! not be. Every case here runs through `Bridge` as a library and, for the
//! wire shape, through the compiled binary in a separate process.
use base64::{engine::general_purpose::STANDARD, Engine};
use flashtex_bridge::{caret, store::Store, *};
use serde_json::{json, Value};
use std::io::{Cursor, Write};
use std::path::Path;
use std::process::{Command, Stdio};

fn image_base64() -> String {
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1, 1)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    STANDARD.encode(bytes.into_inner())
}
fn capture(capture_id: &str, destination_id: &str, base_revision: u64) -> CaptureSubmit {
    CaptureSubmit {
        capture_id: capture_id.into(),
        destination_id: destination_id.into(),
        base_revision,
        image: CaptureImage {
            mime_type: "image/png".into(),
            data_base64: image_base64(),
        },
        instructions: "keep the notation".into(),
    }
}
struct Fixed(&'static str);
impl Converter for Fixed {
    fn convert(&self, _: &CaptureSubmit, _: &Context) -> Result<Proposal> {
        Ok(Proposal {
            latex: self.0.into(),
            ambiguities: vec![],
            required_dependencies: vec![],
        })
    }
}
fn open(b: &mut Bridge, revision: u64, text: &str) {
    b.open_document(Document {
        project_id: "project".into(),
        path: "main.tex".into(),
        revision,
        text: text.into(),
    })
    .unwrap();
}
fn edit(b: &mut Bridge, base: u64, start: usize, end: usize, replacement: &str) {
    b.edit(&EditRequest {
        project_id: "project".into(),
        path: "main.tex".into(),
        base_revision: base,
        revision: base + 1,
        start_byte: start,
        end_byte: end,
        replacement: replacement.into(),
    })
    .unwrap();
}
fn pin(b: &mut Bridge, id: &str, revision: u64, at: usize, mode: AnchorMode) -> Anchor {
    b.pin_with_mode(id, "project", "main.tex", revision, at, at, mode)
        .unwrap()
}
fn wrap(prefix: &str, suffix: &str, kind: &str) -> InsertionWrap {
    InsertionWrap {
        prefix: prefix.into(),
        suffix: suffix.into(),
        kind: Some(kind.into()),
    }
}

// MARK: edits

#[test]
fn typing_at_a_caret_anchor_shifts_it_and_never_invalidates_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Bridge::new(Store::open(dir.path()).unwrap());
    open(&mut b, 1, "Hello world");
    pin(&mut b, "mac-caret-1", 1, 5, AnchorMode::Caret);
    pin(&mut b, "mac-anchor-1", 1, 5, AnchorMode::Fixed);

    // The owner types "!!" exactly where the caret (and both pins) sit.
    edit(&mut b, 1, 5, 5, "!!");
    let refused = b.receive(capture("c-fixed", "mac-anchor-1", 1)).unwrap_err();
    assert_eq!(
        refused.code, "destination_reselection_required",
        "a fixed zero-width pin is still invalidated by an insertion at it"
    );
    b.receive(capture("c-caret", "mac-caret-1", 1)).unwrap();
    let ctx = b
        .context(&capture("c-caret", "mac-caret-1", 1), vec![])
        .unwrap();
    assert_eq!(ctx.source_before, "Hello!!", "the caret pin moved past the typed text");

    // An edit spanning the caret collapses it to the end of the replacement.
    edit(&mut b, 2, 2, 9, "X");
    let ctx = b
        .context(&capture("c-caret", "mac-caret-1", 1), vec![])
        .unwrap();
    assert_eq!(ctx.source_before, "HeX", "anchor now at byte 3");
    assert_eq!(ctx.revision, 3);

    // An edit after it leaves it alone; one before it shifts it.
    edit(&mut b, 3, 4, 5, "");
    edit(&mut b, 4, 0, 0, "--");
    let ctx = b
        .context(&capture("c-caret", "mac-caret-1", 1), vec![])
        .unwrap();
    assert_eq!(ctx.source_before, "--HeX");
}

#[test]
fn a_caret_anchor_may_be_repinned_anywhere_and_base_revision_is_informational() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Bridge::new(Store::open(dir.path()).unwrap());
    open(&mut b, 1, "Hello world");
    let first = pin(&mut b, "mac-caret-1", 1, 5, AnchorMode::Caret);
    assert_eq!(first.mode, AnchorMode::Caret);
    edit(&mut b, 1, 11, 11, "!");
    // Same id, another place, a later revision: allowed for the caret.
    let again = pin(&mut b, "mac-caret-1", 2, 0, AnchorMode::Caret);
    assert_eq!((again.start_byte, again.pinned_revision), (0, 2));
    // The companion still names the revision it read from hello_ack.
    b.receive(capture("c1", "mac-caret-1", 1)).unwrap();

    // A fixed pin keeps today's contract on both counts.
    pin(&mut b, "mac-anchor-1", 2, 3, AnchorMode::Fixed);
    let moved = b
        .pin_with_mode("mac-anchor-1", "project", "main.tex", 2, 4, 4, AnchorMode::Fixed)
        .unwrap_err();
    assert_eq!(moved.code, "destination_conflict");
    let stale = b.receive(capture("c2", "mac-anchor-1", 1)).unwrap_err();
    assert_eq!(stale.code, "revision_conflict");

    // An invalidated fixed pin can be taken over as a caret pin (the Mac's
    // "Insert at caret" recovery), and a caret pin can be re-pinned fixed.
    edit(&mut b, 2, 3, 3, "x"); // invalidates mac-anchor-1
    let taken = pin(&mut b, "mac-anchor-1", 3, 9, AnchorMode::Caret);
    assert!(taken.valid && taken.mode == AnchorMode::Caret);
    let fixed_again = pin(&mut b, "mac-caret-1", 3, 1, AnchorMode::Fixed);
    assert_eq!(fixed_again.mode, AnchorMode::Fixed);
}

// MARK: preparation

#[test]
fn prepare_lands_at_the_current_caret_journals_the_wrap_and_needs_no_reconversion() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Bridge::new(Store::open(dir.path()).unwrap());
    open(&mut b, 1, "Intro.\n\nEnd.\n");
    pin(&mut b, "mac-caret-1", 1, 7, AnchorMode::Caret);
    b.receive(capture("c1", "mac-caret-1", 1)).unwrap();
    // A single letter is neither marked nor formula-shaped: the bridge leaves
    // it bare at conversion, so what the Mac wraps at approval is visible.
    b.convert("c1", vec![], &Fixed("x")).unwrap();

    // The owner keeps typing after the conversion, then moves the caret.
    edit(&mut b, 1, 6, 6, " More prose"); // before the anchor: shifts it
    pin(&mut b, "mac-caret-1", 2, 22, AnchorMode::Caret); // between "End" and "."
    let edit = b
        .prepare_insert_wrapped("c1", 2, true, Some(wrap("$", "$", "inline_math")))
        .unwrap();
    assert_eq!((edit.start_byte, edit.end_byte), (22, 22));
    assert_eq!(edit.replacement, "$x$");
    assert_eq!(edit.wrap.as_ref().map(|w| w.kind.as_deref()), Some(Some("inline_math")));
    let journaled = b.store.require("c1").unwrap().prepared.unwrap();
    assert_eq!(journaled, edit, "the wrap is journaled with the prepared edit");
    let json = serde_json::to_value(&journaled).unwrap();
    assert_eq!(json["wrap"]["prefix"], "$");

    // Idempotent replay ignores a different wrap: the journaled edit is the truth.
    let replay = b
        .prepare_insert_wrapped("c1", 2, true, Some(wrap("\\[ ", " \\]", "display_math")))
        .unwrap();
    assert_eq!(replay, edit);

    // Confirming moves the caret past the insertion, like the editor's caret.
    b.confirm_insert("c1", &edit.edit_id, 3).unwrap();
    let after = pin(&mut b, "mac-caret-2", 3, 0, AnchorMode::Caret);
    assert_eq!(after.pinned_revision, 3);
    assert_eq!(
        b.document("project", "main.tex").unwrap().text,
        "Intro. More prose\n\nEnd$x$.\n"
    );
}

#[test]
fn a_fixed_pin_still_requires_fresh_context_and_a_matching_binding() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Bridge::new(Store::open(dir.path()).unwrap());
    open(&mut b, 1, "Intro.\n\nEnd.\n");
    pin(&mut b, "mac-anchor-1", 1, 8, AnchorMode::Fixed);
    b.receive(capture("c1", "mac-anchor-1", 1)).unwrap();
    b.convert("c1", vec![], &Fixed("x")).unwrap();
    edit(&mut b, 1, 0, 0, "% note\n"); // before the pin: still valid, context stale
    let stale = b.prepare_insert("c1", 2, true).unwrap_err();
    assert_eq!(stale.code, "proposal_context_stale");
}

#[test]
fn the_wrapped_text_is_gated_against_the_caret_it_lands_in() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = Bridge::new(Store::open(dir.path()).unwrap());
    open(&mut b, 1, "Let $a + $ hold.\n");
    pin(&mut b, "mac-caret-1", 1, 9, AnchorMode::Caret); // inside $…$
    b.receive(capture("c1", "mac-caret-1", 1)).unwrap();
    b.convert("c1", vec![], &Fixed("x^2")).unwrap();
    // A stale wrap (computed while the caret was in text) would nest math.
    let nested = b
        .prepare_insert_wrapped("c1", 1, true, Some(wrap("$", "$", "inline_math")))
        .unwrap_err();
    assert_eq!(nested.code, "unsupported_construct_requires_confirmation");
    let too_long = b
        .prepare_insert_wrapped("c1", 1, true, Some(wrap(&"x".repeat(1025), "", "as_is")))
        .unwrap_err();
    assert_eq!(too_long.code, "invalid_wrap");
    // Bare at a math caret is exactly right; an empty wrap with a label is kept.
    let edit = b
        .prepare_insert_wrapped("c1", 1, true, Some(wrap("", "", "as_is")))
        .unwrap();
    assert_eq!(edit.replacement, "x^2");
    assert_eq!(edit.wrap.unwrap().kind.as_deref(), Some("as_is"));
}

#[test]
fn a_bare_formula_is_wrapped_at_conversion_even_without_a_structural_command() {
    assert_eq!(caret::shape("x = 2y + 1"), caret::Shape::BareMath);
    assert_eq!(caret::shape("f(x) = 3x - 1"), caret::Shape::BareMath);
    assert_eq!(caret::shape("sin(x) + cos(x) = 1"), caret::Shape::BareMath);
    assert_eq!(caret::shape("\\mathbb{R} = X"), caret::Shape::BareMath);
    assert_eq!(caret::shape("see figure 2"), caret::Shape::Prose);
    assert_eq!(caret::shape("<F>"), caret::Shape::Prose, "a placeholder is not a formula");
    assert_eq!(caret::shape("a < b"), caret::Shape::BareMath);
    assert_eq!(caret::shape("12"), caret::Shape::Prose);
    assert_eq!(caret::shape("x"), caret::Shape::Prose);
    assert_eq!(caret::shape("A = the set"), caret::Shape::Prose);
    let context = caret::CaretContext::default();
    assert_eq!(
        caret::normalize("x = 2y + 1", &context).text.as_deref(),
        Some("\\[ x = 2y + 1 \\]")
    );
}

// MARK: restart

#[test]
fn after_a_restart_the_caret_id_is_repinned_wherever_the_caret_is_and_prepare_works() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut b = Bridge::new(Store::open(dir.path()).unwrap());
        open(&mut b, 1, "Hello world\n");
        pin(&mut b, "mac-caret-1", 1, 5, AnchorMode::Caret);
        pin(&mut b, "mac-anchor-1", 1, 5, AnchorMode::Fixed);
        b.receive(capture("c-caret", "mac-caret-1", 1)).unwrap();
        b.receive(capture("c-fixed", "mac-anchor-1", 1)).unwrap();
        b.convert("c-caret", vec![], &Fixed("$x$")).unwrap();
        b.convert("c-fixed", vec![], &Fixed("$x$")).unwrap();
    }
    // The app relaunched; the document moved on; the bridge remembers nothing but the journal.
    let mut b = Bridge::new(Store::open(dir.path()).unwrap());
    open(&mut b, 4, "Hello brave new world\n");
    pin(&mut b, "mac-caret-1", 4, 21, AnchorMode::Caret);
    let edit = b.prepare_insert("c-caret", 4, true).unwrap();
    assert_eq!(edit.start_byte, 21);
    assert_eq!(edit.replacement, "$x$");
    // The explicit pin cannot be restored identically: still a reselection
    // (the capture names revision 1, the restored pin is at 4).
    pin(&mut b, "mac-anchor-1", 4, 5, AnchorMode::Fixed);
    let refused = b.prepare_insert("c-fixed", 4, true).unwrap_err();
    assert_eq!(refused.code, "revision_conflict");
}

// MARK: wire shape through the compiled binary

fn request(id: &str, kind: &str, payload: Value) -> String {
    format!(
        "{}\n",
        json!({"protocol_version":1,"id":id,"type":kind,"payload":payload})
    )
}
fn spawn(store: &Path, input: &str) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-bridge"))
        .arg("--store")
        .arg(store)
        .env_remove("XAI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

#[test]
fn wire_mode_and_wrap_are_additive_and_round_trip_through_status_and_receipt() {
    let dir = tempfile::tempdir().unwrap();
    // A proposal is attached as capture_convert would (no provider offline).
    {
        let mut b = Bridge::new(Store::open(dir.path()).unwrap());
        open(&mut b, 1, "Hello world\n");
        pin(&mut b, "mac-caret-1", 1, 5, AnchorMode::Caret);
        b.receive(capture("wire-1", "mac-caret-1", 1)).unwrap();
        b.convert("wire-1", vec![], &Fixed("x")).unwrap();
    }
    let input = [
        request("open", "document_open", json!({"project_id":"project","path":"main.tex","revision":2,"text":"Hello there world\n"})),
        request("pin-default", "destination_pin", json!({"destination_id":"mac-anchor-9","project_id":"project","path":"main.tex","revision":2,"start_byte":0,"end_byte":0})),
        request("pin-caret", "destination_pin", json!({"destination_id":"mac-caret-1","project_id":"project","path":"main.tex","revision":2,"start_byte":11,"end_byte":11,"mode":"caret"})),
        request("type", "document_edit", json!({"project_id":"project","path":"main.tex","base_revision":2,"revision":3,"start_byte":11,"end_byte":11,"replacement":"big "})),
        request("prepare", "capture_prepare_insert", json!({"capture_id":"wire-1","expected_revision":3,"approved":true,"wrap":{"prefix":"$","suffix":"$","kind":"inline_math"}})),
        request("status", "capture_status", json!({"capture_id":"wire-1"})),
        request("applied", "capture_applied", json!({"capture_id":"wire-1","edit_id":"capture-wire-1","new_revision":4})),
    ]
    .concat();
    let replies = spawn(dir.path(), &input);
    let by_id = |id: &str| replies.iter().find(|r| r["id"] == id).unwrap().clone();
    assert_eq!(by_id("pin-default")["type"], "destination_pinned");
    assert_eq!(by_id("pin-default")["payload"]["mode"], "fixed", "absent mode is the old contract");
    assert_eq!(by_id("pin-caret")["payload"]["mode"], "caret");
    let prepared = by_id("prepare");
    assert_eq!(prepared["type"], "capture_edit", "{prepared}");
    assert_eq!(prepared["payload"]["start_byte"], 15, "typed text before the caret shifted it");
    assert_eq!(prepared["payload"]["replacement"], "$x$");
    assert_eq!(prepared["payload"]["wrap"]["kind"], "inline_math");
    assert_eq!(by_id("status")["payload"]["prepared"]["wrap"]["suffix"], "$");
    let receipt = by_id("applied");
    assert_eq!(receipt["type"], "capture_application_received", "{receipt}");
    assert_eq!(receipt["payload"]["wrap"]["prefix"], "$");
}
