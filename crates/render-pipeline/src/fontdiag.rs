//! The font/metric diagnostic codes, and what each one means for a harness
//! that is judging a run — the single source of truth for every acceptance
//! gate and oracle that has to answer "did this run get the fonts it asked
//! for?" or "are these positions still the reference geometry?".
//!
//! Three scripts used to keep their own hand-written copy of this list, and
//! they had drifted apart in both directions:
//!
//! * `apps/mac/scripts/texmf-acceptance.sh` listed three codes, omitting
//!   `ec_metrics_unavailable`, `font_outline_substituted` and
//!   `math_font_unavailable`. A run emitting only those printed
//!   "0 missing-metric diagnostics = ok" while a real substitution had
//!   happened, on the acceptance gate for the shipped app's font discovery.
//! * `apps/mac/scripts/faces-acceptance.py` had six, missing
//!   `math_metrics_opentype`.
//! * `tools/visual-oracle/fontenv.py` had a different six, because it asks a
//!   different question (see the two flags below).
//!
//! [`render_json`] writes `supported/font-diagnostics.json`, which all three
//! now read; `tests/fontdiag.rs` fails if that file is stale, and fails if
//! `src/` emits a diagnostic code this table does not classify. Adding a code
//! is therefore a decision someone has to record, not something that silently
//! lands outside every gate.

/// One diagnostic code, classified for the two questions harnesses ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontDiagnostic {
    /// The `code` field of the emitted [`crate::display::Diagnostic`].
    pub code: &'static str,
    /// The document did not get the face or the metrics it asked for, so the
    /// output is not the document that was requested. Font-discovery and
    /// packaging gates (`texmf-acceptance.sh`, `faces-acceptance.py`) count
    /// these: seeing one means the bundle failed to provide a resource.
    pub substitution: bool,
    /// The glyph positions are no longer pdflatex's reference geometry, so
    /// comparing them against a pdflatex reference is meaningless. Position
    /// oracles (`tools/visual-oracle/fontenv.py`) must fail rather than score
    /// a run that emitted one of these.
    pub geometry_void: bool,
    /// Why it is classified this way. Free of `"` and `\` so the generated
    /// JSON needs no escaping; `notes_need_no_escaping` enforces that.
    pub note: &'static str,
}

const fn d(
    code: &'static str,
    substitution: bool,
    geometry_void: bool,
    note: &'static str,
) -> FontDiagnostic {
    FontDiagnostic { code, substitution, geometry_void, note }
}

/// Every font/metric diagnostic the render pipeline emits.
///
/// The `substitution` set is the union of the two copies that were correct
/// when this table was written (`faces-acceptance.py` and `fontenv.py`,
/// including the latter's advisory code). `geometry_void` is that set minus
/// `font_outline_substituted`, whose metrics — and so whose positions — are
/// still the reference ones.
pub const FONT_DIAGNOSTICS: [FontDiagnostic; 8] = [
    d(
        "font_unavailable",
        true,
        true,
        "Latin Modern face unavailable; Times metrics substituted, so this is not the requested document",
    ),
    d(
        "required_metrics_unavailable",
        true,
        true,
        "the pinned Latin Modern 2.004 metrics this size requires were not found; OpenType advances were used",
    ),
    d(
        "tfm_missing",
        true,
        true,
        "a .tfm the face needs was not found; OpenType advances are used instead of TeX metrics",
    ),
    d(
        "ec_metrics_unavailable",
        true,
        true,
        "T1/EC metrics missing for this face, e.g. no ectt* for a texttt fixture",
    ),
    d(
        "math_font_unavailable",
        true,
        true,
        "Latin Modern Math unavailable; math is not typeset at all",
    ),
    d(
        "math_metrics_opentype",
        true,
        true,
        "rm-lmr*.tfm unavailable; math is laid out from the OpenType MATH table rather than TeX metrics",
    ),
    d(
        "font_outline_substituted",
        true,
        false,
        "the outline differs but the metrics are the reference ones, so positions still compare",
    ),
    d(
        "font_shape_substituted",
        false,
        false,
        "LaTeX's own NFSS shape substitution; pdflatex substitutes identically, so nothing is missing and the geometry is the reference geometry",
    ),
];

/// Codes emitted by `src/` that are deliberately not font-resource
/// diagnostics. Listed so that `tests/fontdiag.rs` can require every emitted
/// code to be classified: a new code lands in neither list and fails the test
/// until someone decides which it is.
///
/// `missing_glyph`, `math_glyph_unmapped` and `tfm_run_error` are the close
/// calls. They are font-related but they are *coverage and read* failures of a
/// face that was found, not a failure to find the face or its metrics, so the
/// discovery gates do not count them. Revisit that if a gate needs them.
pub const NON_FONT_DIAGNOSTICS: [&str; 25] = [
    "display_list_declined",
    "float_content_unsupported",
    "float_placement",
    "float_too_large",
    "graphics_option",
    "image_unavailable",
    "labels_unstable",
    "math_glyph_unmapped",
    "math_limitation",
    "math_resource_profile",
    "math_text_overflow",
    "microtype_unsupported",
    "missing_glyph",
    "multicol",
    "overfull_display",
    "overfull_hbox",
    "overfull_vbox",
    "paragraph_layout_error",
    "tfm_run_error",
    "tikz_display_list_only",
    "tikz_error",
    "tikz_text_transform",
    "tikz_unsupported",
    "unsupported_block",
    "unsupported_script",
];

/// The codes a font-discovery gate must treat as "a resource was missing".
pub fn substitution_codes() -> impl Iterator<Item = &'static str> {
    FONT_DIAGNOSTICS.iter().filter(|d| d.substitution).map(|d| d.code)
}

/// The codes that void a position comparison against a pdflatex reference.
pub fn geometry_void_codes() -> impl Iterator<Item = &'static str> {
    FONT_DIAGNOSTICS.iter().filter(|d| d.geometry_void).map(|d| d.code)
}

/// Regenerate with `flashtex-render --font-diagnostics json`.
pub const SCHEMA: &str = "flashtex-font-diagnostics/1";

/// The generated `supported/font-diagnostics.json`. Deterministic: table
/// order, two-space indent, trailing newline.
pub fn render_json() -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{SCHEMA}\",\n"));
    s.push_str("  \"generated_by\": \"flashtex-render --font-diagnostics json\",\n");
    s.push_str("  \"source\": \"crates/render-pipeline/src/fontdiag.rs\",\n");
    s.push_str("  \"codes\": [\n");
    for (i, e) in FONT_DIAGNOSTICS.iter().enumerate() {
        s.push_str("    {\n");
        s.push_str(&format!("      \"code\": \"{}\",\n", e.code));
        s.push_str(&format!("      \"substitution\": {},\n", e.substitution));
        s.push_str(&format!("      \"geometry_void\": {},\n", e.geometry_void));
        s.push_str(&format!("      \"note\": \"{}\"\n", e.note));
        s.push_str(if i + 1 == FONT_DIAGNOSTICS.len() { "    }\n" } else { "    },\n" });
    }
    s.push_str("  ],\n");
    s.push_str("  \"not_font_diagnostics\": [\n");
    for (i, c) in NON_FONT_DIAGNOSTICS.iter().enumerate() {
        s.push_str(&format!(
            "    \"{c}\"{}\n",
            if i + 1 == NON_FONT_DIAGNOSTICS.len() { "" } else { "," }
        ));
    }
    s.push_str("  ]\n");
    s.push_str("}\n");
    s
}
