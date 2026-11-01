//! Gated live round trip against the real xAI Responses API.
//!
//! This test is intentionally excluded from every normal `cargo test` run and
//! from CI: it spends real API credits and requires a live `XAI_API_KEY`. It
//! is `#[ignore]`d rather than silently skipped-and-passed, so a run without a
//! key reports it as `ignored`, never as a false `passed`. To actually
//! exercise it:
//!
//!   XAI_API_KEY=... cargo test --test live_grok -- --ignored --nocapture
//!
//! Optionally set `FLASHTEX_GROK_MODEL` to override the model under test.
//!
//! The key is read from the environment only at runtime here; it is never
//! logged, printed, or written to any file or fixture.
//!
//! What this proves (when it passes): the request/response plumbing --
//! bearer auth, the strict JSON schema, image upload, response parsing, and
//! `Proposal::validate` -- round-trips against the real API for a bounded
//! synthetic image. What it does NOT prove: handwriting transcription
//! quality, photo-of-paper conditions, or any particular LaTeX content --
//! model output is not deterministic, so every assertion below is about
//! structure, never an exact string.

use base64::{engine::general_purpose::STANDARD, Engine};
use flashtex_bridge::{
    grok::{GrokClient, DEFAULT_MODEL},
    CaptureImage, CaptureSubmit, Context, Converter,
};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

/// A tiny synthetic "+" drawn as two solid black bars on white: not
/// handwriting, not a photo, just enough deterministic pixel content to give
/// the model something unambiguous to transcribe.
fn synthetic_plus_sign_png() -> Vec<u8> {
    let (w, h) = (64u32, 64u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb([255, 255, 255]));
    for y in 26..38 {
        for x in 8..56 {
            img.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }
    for y in 8..56 {
        for x in 26..38 {
            img.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(img)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("encoding a synthetic in-memory PNG cannot fail");
    bytes
}

/// Rough structural sanity check: every `{` is eventually closed by a `}`,
/// ignoring escaped braces (`\{`, `\}`), which LaTeX uses for literal
/// symbols. This is not a real parser -- see crates/compiler for that -- but
/// it catches a truncated or mismatched proposal without asserting on exact
/// content.
fn braces_are_balanced(latex: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = latex.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

#[test]
#[ignore = "spends real xAI API credits; run explicitly with XAI_API_KEY set: \
            XAI_API_KEY=... cargo test --test live_grok -- --ignored --nocapture"]
fn live_grok_round_trip_produces_structurally_valid_proposal() {
    let key = std::env::var("XAI_API_KEY")
        .expect("set XAI_API_KEY to run this live test (read only at runtime here, never logged)");
    let model = std::env::var("FLASHTEX_GROK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let mut capture = CaptureSubmit {
        capture_id: "live-test-capture".into(),
        destination_id: "live-test-destination".into(),
        base_revision: 1,
        image: CaptureImage {
            mime_type: "image/png".into(),
            data_base64: STANDARD.encode(synthetic_plus_sign_png()),
        },
        instructions: "This is a synthetic calibration image, not real handwriting: a single \
                       plus sign drawn as two solid black bars on a white background. \
                       Transcribe it as LaTeX math containing the plus operator, for example \
                       `$+$`. This call exists only to validate request/response plumbing, not \
                       transcription quality."
            .into(),
    };
    capture
        .validate()
        .expect("synthetic capture must satisfy the bridge's own bounds");

    let context = Context {
            caret_context: Default::default(),
        project_id: "live-test-project".into(),
        path: "main.tex".into(),
        revision: 1,
        source_before: String::new(),
        selected_source: String::new(),
        source_after: String::new(),
        definitions: vec![],
        supported_features: vec![],
        dependencies: vec![],
    };

    let client = GrokClient::new(key, model.clone())
        .expect("GrokClient::new should accept a non-empty key and model");
    let proposal = client
        .convert(&capture, &context)
        .expect("live Grok round trip should return a completed, schema-conformant proposal");

    // Structure, not content. parse_response already calls Proposal::validate()
    // internally before returning Ok, so this is a second, explicit witness of
    // the same contract at the test boundary rather than new coverage.
    proposal
        .validate()
        .expect("returned proposal must satisfy the bridge's own bounds a second time");
    assert!(!proposal.latex.is_empty(), "latex must be non-empty");
    assert!(!proposal.latex.contains('\0'), "latex must not contain NUL");
    assert!(
        braces_are_balanced(&proposal.latex),
        "latex braces should balance: {:?}",
        proposal.latex
    );
    assert!(
        proposal.ambiguities.len() <= 32,
        "ambiguities list must be bounded"
    );
    assert!(
        proposal.required_dependencies.len() <= 32,
        "required_dependencies list must be bounded"
    );
    for note in proposal
        .ambiguities
        .iter()
        .chain(&proposal.required_dependencies)
    {
        assert!(note.len() <= 2048, "annotation entries must be bounded");
        assert!(
            !note.contains('\0'),
            "annotation entries must not contain NUL"
        );
    }

    eprintln!("live Grok proposal (model={model}):");
    eprintln!("  latex: {}", proposal.latex);
    eprintln!("  ambiguities: {:?}", proposal.ambiguities);
    eprintln!(
        "  required_dependencies: {:?}",
        proposal.required_dependencies
    );
}
