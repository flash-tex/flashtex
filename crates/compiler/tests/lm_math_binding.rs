//! `\mathbb`, `\setminus` and `\Longrightarrow` draw real glyphs from the
//! pinned Latin Modern Math resource, and the compiler says exactly that.
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::lm_math;
use flashtex_compiler::protocol::handle_line;
use flashtex_font_engine::manifest::{pinned_latin_modern, EmbeddingPermission};
use flashtex_font_engine::{sha256, GlyphId, TrueTypeFace};

const BUNDLED_FONT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/mac/Fonts/latinmodern-math.otf"
);

/// The table is bound to one exact font program: the digest the font-engine
/// manifest pins, and the advances that program actually carries.
#[test]
fn advances_are_read_from_the_pinned_font_program() {
    let entry = pinned_latin_modern()
        .resources
        .into_iter()
        .find(|entry| entry.font.font_id == lm_math::FONT_ID)
        .expect("font-engine manifest pins lm.math");
    assert_eq!(entry.font.sha256, lm_math::SHA256);
    assert_eq!(f64::from(entry.font.units_per_em), lm_math::UNITS_PER_EM);
    assert_eq!(
        entry.license.embedding_permission,
        EmbeddingPermission::Allowed
    );

    let bytes = std::fs::read(BUNDLED_FONT).expect("bundled Latin Modern Math");
    assert_eq!(sha256::hex(&sha256::digest(&bytes)), lm_math::SHA256);
    let face = TrueTypeFace::parse(bytes).expect("parse Latin Modern Math");
    // amssymb/amsfonts symbols bound to the same program (`amssymb::LM_ADVANCES`).
    for (c, advance) in lm_math::ADVANCES.iter().chain(flashtex_compiler::amssymb::LM_ADVANCES) {
        let gid = face
            .char_map()
            .get(&(*c as u32))
            .unwrap_or_else(|| panic!("Latin Modern Math has no glyph for {c:?}"));
        let (actual, _) = face.hmetric(GlyphId(*gid)).expect("hmtx entry");
        assert_eq!(actual, *advance, "advance of {c:?} (U+{:04X})", *c as u32);
    }
}

fn compile(text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("lm-math"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    payload.set(
        "layout_capabilities",
        Value::Arr(vec![json::str_("font-hints-v1")]),
    );
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("lm"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply")
}

fn at<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |v, key| {
        v.get(key).unwrap_or_else(|| panic!("missing {key}"))
    })
}

fn page_items(reply: &Value) -> Vec<&Value> {
    at(reply, &["payload", "pages"])
        .as_arr()
        .unwrap()
        .iter()
        .flat_map(|page| at(page, &["items"]).as_arr().unwrap().iter())
        .collect()
}

fn items(reply: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    {
        for item in page_items(reply) {
            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                let family = item
                    .get("font")
                    .and_then(|f| f.get("family"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                out.push((text.to_string(), family));
            }
        }
    }
    out
}

fn messages(reply: &Value) -> Vec<String> {
    at(reply, &["payload", "diagnostics"])
        .as_arr()
        .unwrap()
        .iter()
        .map(|d| at(d, &["message"]).as_str().unwrap().to_string())
        .collect()
}

#[test]
fn real_glyphs_are_emitted_with_the_latin_modern_math_hint() {
    // `\mathbb` is an `amsfonts.sty` alphabet (`amsfonts.sty` 108), undefined
    // in base LaTeX2e, so the document has to load the package.
    let reply = compile(
        "\\usepackage{amsfonts}\n\
         $\\mathbb{Z} \\subset \\mathbb{Q}$ and $\\mathbb{R}\\setminus\\mathbb{Q}$ \
         $A \\Longrightarrow B$ $\\mathbb{N}$\n",
    );
    let items = items(&reply);
    for glyph in ["ℤ", "ℚ", "ℝ", "∖", "⟹", "ℕ"] {
        let (_, family) = items
            .iter()
            .find(|(text, _)| text == glyph)
            .unwrap_or_else(|| panic!("{glyph} not emitted: {items:?}"));
        assert_eq!(family, lm_math::FAMILY, "{glyph}");
    }
    assert_eq!(
        items.iter().find(|(text, _)| text == "A").unwrap().1,
        "Times-Italic",
        "ordinary math letters keep a base-14 (math italic) hint"
    );

    let messages = messages(&reply);
    for command in ["mathbb", "setminus", "Longrightarrow"] {
        assert!(
            !messages
                .iter()
                .any(|m| m.contains(&format!("\\{command} is not supported"))),
            "{messages:#?}"
        );
    }
    // No base-14 "no glyph" width fallback for these glyphs.
    assert!(
        !messages.iter().any(|m| m.contains("has no glyph")),
        "{messages:#?}"
    );
    // The PDF writer embeds Latin Modern Math (and warns itself when the font
    // is not installed), so the compiler must not predict an export loss.
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("will not survive PDF export")),
        "{messages:#?}"
    );
    // And exactly one fidelity limitation for the document.
    let fidelity: Vec<_> = messages
        .iter()
        .filter(|m| m.contains("widths differ from pdfLaTeX"))
        .collect();
    assert_eq!(fidelity.len(), 1, "{messages:#?}");
}

/// Issue #62: amssymb/latexsym symbols with no base-14 Symbol glyph also draw
/// from the pinned Latin Modern Math resource, exactly like `\mathbb`.
#[test]
fn amssymb_symbols_are_emitted_with_the_latin_modern_math_hint() {
    let reply = compile(
        r"\usepackage{amssymb}
          $a \mp b$ $a \ll b$ $a \gg b$ $a \simeq b$ $\vdots$ $\ddots$
          $\lfloor x \rfloor$ $\lceil x \rceil$ $\oint_C f$ $a \mapsto b$
          $\ell$ $\hbar$ $a \circ b$ $a \parallel b$ $a \nmid b$
          $a \nleq b$ $a \ngeq b$ $a \subsetneq b$ $a \supsetneq b$
          $a \lesssim b$ $a \gtrsim b$ $a \triangleq b$ $a \coloneqq b$
          $\nexists x$ $\complement A$ $a \rightsquigarrow b$
          $a \hookrightarrow b$ $a \leftrightarrows b$ $a \models b$
          $a \vdash b$ $a \dashv b$ $\top$ $\measuredangle$ $\square$
          $\blacksquare$ $\lozenge$ $\checkmark$
",
    );
    let items = items(&reply);
    let glyphs = [
        "∓", "≪", "≫", "≃", "⋮", "⋱", "⌊", "⌋", "⌈", "⌉", "∮", "↦", "ℓ", "ℏ", "∘", "∥", "∤", "≰",
        "≱", "⊊", "⊋", "≲", "≳", "≜", "≔", "∄", "∁", "⇝", "↪", "⇆", "⊨", "⊢", "⊣", "⊤", "∡", "□",
        "■", "◊", "✓",
    ];
    for glyph in glyphs {
        let (_, family) = items
            .iter()
            .find(|(text, _)| text == glyph)
            .unwrap_or_else(|| panic!("{glyph} not emitted: {items:?}"));
        assert_eq!(family, lm_math::FAMILY, "{glyph}");
    }

    let messages = messages(&reply);
    let commands = [
        "mp",
        "ll",
        "gg",
        "simeq",
        "vdots",
        "ddots",
        "lfloor",
        "rfloor",
        "lceil",
        "rceil",
        "oint",
        "mapsto",
        "ell",
        "hbar",
        "circ",
        "parallel",
        "nmid",
        "nleq",
        "ngeq",
        "subsetneq",
        "supsetneq",
        "lesssim",
        "gtrsim",
        "triangleq",
        "coloneqq",
        "nexists",
        "complement",
        "rightsquigarrow",
        "hookrightarrow",
        "leftrightarrows",
        "models",
        "vdash",
        "dashv",
        "top",
        "measuredangle",
        "square",
        "blacksquare",
        "lozenge",
        "checkmark",
    ];
    for command in commands {
        assert!(
            !messages
                .iter()
                .any(|m| m.contains(&format!("\\{command} is not supported"))),
            "{messages:#?}"
        );
    }
    // No base-14 "no glyph" width fallback, and no false export-loss warning,
    // for any of these glyphs (same guarantee as `\mathbb`/`\setminus`).
    assert!(
        !messages.iter().any(|m| m.contains("has no glyph")),
        "{messages:#?}"
    );
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("will not survive PDF export")),
        "{messages:#?}"
    );
}

/// Issue #62 HW2 follow-up: long arrows, `\triangle`/`\bigtriangleup`/
/// `\bigtriangledown`, `\bot`, and the `\mathbin`-family class overrides all
/// draw from the pinned Latin Modern Math resource with no diagnostic loss.
#[test]
fn hw2_math_follow_up_commands_render_with_no_diagnostics() {
    let reply = compile(
        r"$A \Longleftrightarrow B$ $A \longrightarrow B$ $A \longleftarrow B$
          $A \Longleftarrow B$ $A \longleftrightarrow B$ $A \longmapsto B$
          $A \iff B$ $A \implies B$ $A \impliedby B$
          $\triangle$ $A \bigtriangleup B$ $A \bigtriangledown B$
          $a \bot b$ $A \mathbin{\triangle} B$ $a \mathrel{+} b$
",
    );
    let items = items(&reply);
    for glyph in ["⟺", "⟶", "⟵", "⟸", "⟷", "⟼", "△", "▽"] {
        let (_, family) = items
            .iter()
            .find(|(text, _)| text == glyph)
            .unwrap_or_else(|| panic!("{glyph} not emitted: {items:?}"));
        assert_eq!(family, lm_math::FAMILY, "{glyph}");
    }
    // `\bot` renders base-14 Symbol's `⊥`, same as `\perp`, not Latin Modern
    // Math, so it is checked separately for presence rather than family.
    assert!(items.iter().any(|(text, _)| text == "⊥"), "{items:?}");

    let messages = messages(&reply);
    for command in [
        "Longleftrightarrow",
        "longrightarrow",
        "longleftarrow",
        "Longleftarrow",
        "longleftrightarrow",
        "longmapsto",
        "iff",
        "implies",
        "impliedby",
        "triangle",
        "bigtriangleup",
        "bigtriangledown",
        "bot",
        "mathbin",
        "mathrel",
    ] {
        assert!(
            !messages
                .iter()
                .any(|m| m.contains(&format!("\\{command} is not supported"))),
            "{messages:#?}"
        );
    }
    assert!(
        !messages.iter().any(|m| m.contains("has no glyph")),
        "{messages:#?}"
    );
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("will not survive PDF export")),
        "{messages:#?}"
    );
}

#[test]
fn mathbb_rejects_what_amsfonts_does_not_provide() {
    let reply = compile("\\usepackage{amsfonts}\n$\\mathbb{1}$ $\\mathbb{x}$\n");
    let messages = messages(&reply);
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.contains("\\mathbb supports only capital letters"))
            .count(),
        2,
        "{messages:#?}"
    );
    assert!(!messages.iter().any(|m| m.contains("widths differ")));
}

#[test]
fn blackboard_bold_is_measured_from_the_font_not_times() {
    let wide = compile("$\\mathbb{W}x$\n");
    let narrow = compile("$\\mathbb{I}x$\n");
    let x_of = |reply: &Value| match page_items(reply)
        .into_iter()
        .find(|item| item.get("text").and_then(|v| v.as_str()) == Some("x"))
        .and_then(|item| item.get("x_pt"))
    {
        Some(Value::Num(x)) => *x,
        other => panic!("no x position: {other:?}"),
    };
    let size = 12.0;
    let expected =
        f64::from(lm_math::advance('\u{1D54E}').unwrap() - lm_math::advance('\u{1D540}').unwrap())
            * size
            / lm_math::UNITS_PER_EM;
    assert!(
        (x_of(&wide) - x_of(&narrow) - expected).abs() < 0.02,
        "W is {expected}pt wider than I in Latin Modern Math"
    );
}
