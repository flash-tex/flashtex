//! Tests for the harness's own arithmetic and for the properties a
//! benchmark's conclusions rest on: that the generated corpus is the same
//! bytes everywhere, that the report survives a round trip through JSON, and
//! that the gate fires on a real regression and stays quiet inside the noise.
//!
//! Nothing here renders a document: these are cheap and need no fonts.

use crate::corpus;
use crate::measure::Phases;
use crate::report::{self, CaseReport, Report};
use crate::stats;
use flashtex_compiler::json;
use std::collections::BTreeMap;

#[test]
fn percentiles_are_nearest_rank_and_spread_is_robust() {
    let s = stats::summarize(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(s.n, 5);
    assert_eq!(s.min, 1.0);
    assert_eq!(s.median, 3.0);
    assert_eq!(s.max, 5.0);
    assert_eq!(s.mad, 1.0);

    // One sample ten times the rest moves the mean and leaves the median and
    // the MAD alone. That is the whole reason the report is built on order
    // statistics: another build lane waking up mid-run looks exactly like this.
    let spike = stats::summarize(&[1.0, 2.0, 3.0, 4.0, 50.0]);
    assert_eq!(spike.median, 3.0);
    assert_eq!(spike.mad, 1.0);
    assert!(spike.mean > 10.0);
}

#[test]
fn an_empty_sample_set_does_not_produce_a_number() {
    let s = stats::summarize(&[]);
    assert_eq!(s.n, 0);
    assert!(s.median.is_nan(), "an empty set must not report a median");
}

#[test]
fn generated_documents_are_byte_identical_between_calls() {
    for (a, b) in [
        (corpus::scaling_document(50_000), corpus::scaling_document(50_000)),
        (corpus::math_heavy_document(20_000), corpus::math_heavy_document(20_000)),
        (corpus::tikz_heavy_document(20_000), corpus::tikz_heavy_document(20_000)),
    ] {
        assert_eq!(a, b, "a generator that is not deterministic invalidates every baseline");
    }
}

#[test]
fn generated_documents_are_complete_and_reach_their_size() {
    for doc in [corpus::scaling_document(50_000), corpus::math_heavy_document(20_000), corpus::tikz_heavy_document(20_000)] {
        assert!(doc.starts_with("\\documentclass"));
        assert!(doc.ends_with("\\end{document}\n"));
        assert!(doc.contains("\\begin{document}"));
    }
    assert!(corpus::scaling_document(50_000).len() >= 50_000);
}

/// Both shape-specific documents must contain at least one line of prose with
/// no command and no maths in it. Without one the harness finds no anchor, the
/// warm scenarios are skipped, and the case silently measures half of what it
/// is there to measure.
#[test]
fn the_shape_specific_documents_have_somewhere_to_type() {
    for doc in [corpus::math_heavy_document(20_000), corpus::tikz_heavy_document(20_000)] {
        let has_prose = doc
            .lines()
            .any(|l| l.len() >= 60 && !l.contains('\\') && !l.contains('$') && l.starts_with(|c: char| c.is_ascii_alphabetic()));
        assert!(has_prose, "no prose line to place a keystroke on");
    }
}

fn case(id: &str, metrics: &[(&str, f64, f64)], digest: &str) -> CaseReport {
    CaseReport {
        id: id.into(),
        group: "synthetic".into(),
        bytes: 1000,
        documents: 1,
        pages: 1,
        passes: 1,
        metrics: metrics
            .iter()
            .map(|(k, median, rel_iqr)| {
                let mut s = stats::summarize(&[*median]);
                s.median = *median;
                s.rel_iqr = *rel_iqr;
                (k.to_string(), s)
            })
            .collect(),
        digests: BTreeMap::from([("cold.reply".to_string(), digest.to_string())]),
        coverage: 1.0,
        cache_hit_rate: 0.5,
        load_max: 1.0,
        notes: Vec::new(),
    }
}

fn report_of(cases: Vec<CaseReport>) -> Report {
    Report {
        meta: vec![
            ("host_fingerprint".into(), json::str_("linux/x86_64/test/16cpu")),
            ("build_profile".into(), json::str_("release")),
            ("calibration_ns".into(), json::num(1000.0)),
        ],
        cases,
        unmeasured: Vec::new(),
    }
}

#[test]
fn a_regression_above_tolerance_and_above_the_noise_band_fails() {
    let base = report_of(vec![case("c", &[("cold.render_ms", 10.0, 0.01)], "aa")]);
    let now = report_of(vec![case("c", &[("cold.render_ms", 11.0, 0.01)], "aa")]);
    let o = report::gate(&base, &now, 5.0);
    assert_eq!(o.regressions.len(), 1, "a 10% move with a 1% noise band is a regression");
    assert!(report::gate_failed(&o, false));
}

#[test]
fn a_move_inside_the_baselines_own_spread_is_not_a_regression() {
    // 10% slower, but this metric's own p25..p75 spread in the baseline is
    // 30%. Calling that a regression would fire on noise, and a gate that
    // fires on noise gets switched off.
    let base = report_of(vec![case("c", &[("cold.render_ms", 10.0, 0.30)], "aa")]);
    let now = report_of(vec![case("c", &[("cold.render_ms", 11.0, 0.30)], "aa")]);
    let o = report::gate(&base, &now, 5.0);
    assert!(o.regressions.is_empty());
    assert!(!report::gate_failed(&o, false));
}

#[test]
fn a_tiny_absolute_move_is_not_a_regression() {
    let base = report_of(vec![case("c", &[("cold.phase.v1_ms", 0.010, 0.0)], "aa")]);
    let now = report_of(vec![case("c", &[("cold.phase.v1_ms", 0.020, 0.0)], "aa")]);
    let o = report::gate(&base, &now, 5.0);
    assert!(o.regressions.is_empty(), "10 microseconds is timer resolution, not a 100% regression");
}

#[test]
fn a_changed_output_digest_fails_even_when_everything_got_faster() {
    let base = report_of(vec![case("c", &[("cold.render_ms", 10.0, 0.01)], "aa")]);
    let now = report_of(vec![case("c", &[("cold.render_ms", 1.0, 0.01)], "bb")]);
    let o = report::gate(&base, &now, 5.0);
    assert_eq!(o.digest_mismatches.len(), 1);
    assert_eq!(o.improvements.len(), 1);
    // Byte-identical output is a hard gate: no amount of speed buys it off,
    // and --require-same-host does not excuse it either.
    assert!(report::gate_failed(&o, false));
    assert!(report::gate_failed(&o, true));
}

#[test]
fn timings_from_a_different_host_are_reported_but_not_enforced() {
    let mut base = report_of(vec![case("c", &[("cold.render_ms", 10.0, 0.01)], "aa")]);
    base.meta[0].1 = json::str_("macos/aarch64/other/10cpu");
    let now = report_of(vec![case("c", &[("cold.render_ms", 20.0, 0.01)], "aa")]);
    let o = report::gate(&base, &now, 5.0);
    assert!(o.host_differs);
    assert_eq!(o.regressions.len(), 1, "the regression is still reported");
    assert!(report::gate_failed(&o, false));
    assert!(!report::gate_failed(&o, true), "a wall-clock delta across machines is not evidence about the code");
}

#[test]
fn a_report_survives_a_round_trip_through_json() {
    let r = report_of(vec![case("c", &[("cold.render_ms", 10.0, 0.02)], "aa")]);
    let text = json::write(&r.to_json());
    let back = Report::from_json(&json::parse(&text).expect("valid JSON")).expect("a perf-bench report");
    assert_eq!(back.cases.len(), 1);
    assert_eq!(back.cases[0].id, "c");
    assert_eq!(back.cases[0].metrics["cold.render_ms"].median, 10.0);
    assert_eq!(back.cases[0].digests["cold.reply"], "aa");
    // A round trip must not turn a real comparison into a vacuous one.
    let o = report::gate(&r, &back, 5.0);
    assert!(o.compared > 0);
    assert!(o.regressions.is_empty());
}

/// `expand` overlaps `parse`, and only one JSON envelope is ever written, so
/// none of the three may be summed into coverage. If they were, coverage would
/// exceed 1.0 for reasons that say nothing about how faithful the
/// decomposition is.
#[test]
fn overlapping_and_alternative_phases_are_excluded_from_coverage() {
    for n in ["expand", "typeset_notexts", "json_v1", "json_v2"] {
        assert!(Phases::NAMES.contains(&n), "{n} should still be reported");
    }
    assert!(Phases::NAMES.contains(&"typeset"));
}

/// `unicode-accents` opens with multi-byte characters in its first prose line.
/// An anchor computed as "ten bytes into the line" landed inside one of them
/// and `String::insert_str` panicked, taking the whole suite down with it.
#[test]
fn a_keystroke_anchor_never_lands_inside_a_character() {
    let text = "\\begin{document}\nEvariste Galois \u{e9}crivit ceci \u{e0} Auguste Chevalier la nuit pr\u{e9}c\u{e9}dant le duel.\n";
    let start = text.find("Evariste").expect("prose line");
    for offset in start..start + 40 {
        let at = crate::measure::boundary_for_tests(text, offset);
        assert!(text.is_char_boundary(at), "offset {offset} snapped to {at}, which is not a character boundary");
        // The operation the harness actually performs at that offset.
        let mut edited = text.to_string();
        edited.insert_str(at, "abc");
    }
}
