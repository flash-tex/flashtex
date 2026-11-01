use flashtex_font_resources::{
    required_tfm::*,
    sha256,
    tfm::{BoundaryOptions, Tfm},
};
use std::fmt::Write;
fn replay(t: &Tfm) -> String {
    let mut s = String::new();
    for code in 0..=255 {
        if let Some(m) = t.char_metrics(code) {
            writeln!(
                s,
                "metric;{},{},{},{},{}",
                code, m.width.0, m.height.0, m.depth.0, m.italic.0
            )
            .unwrap();
        }
    }
    for n in 1..=32 {
        if let Some(v) = t.parameter(n) {
            writeln!(s, "param;{},{}", n, v.0).unwrap();
        }
    }
    for input in [
        b"fi".as_slice(),
        b"ff",
        b"fl",
        b"ffi",
        b"ffl",
        b"AV",
        b"To",
        b"WA",
        b"office",
        b"wo",
    ] {
        let r = t.glyph_run(input, BoundaryOptions::default()).unwrap();
        assert_eq!(r.leading_kern.0, 0);
        write!(s, "{}", std::str::from_utf8(input).unwrap()).unwrap();
        for g in r.glyphs {
            write!(
                s,
                ";{},{},{},{}",
                g.code, g.input_start, g.input_end, g.kern_after.0
            )
            .unwrap();
        }
        writeln!(s).unwrap();
    }
    s
}
#[test]
fn missing_required_metric_never_returns_substitute() {
    let d = tempfile::tempdir().unwrap();
    let root = flashtex_project_files::ProjectRoot::open(d.path()).unwrap();
    let mut m: Manifest =
        serde_json::from_str(include_str!("../fixtures/lm-required-metrics.json")).unwrap();
    assert!(matches!(RequiredMetrics::load(&root,&m),Err(Error::Missing(p)) if p=="ec-lmr12.tfm"));
    let bytes = include_bytes!("../../font-engine/fixtures/tfm/ec-lmr10.tfm");
    std::fs::write(d.path().join("metric.tfm"), bytes).unwrap();
    std::fs::write(d.path().join("LICENSE"), b"synthetic license").unwrap();
    m.metrics = vec![MetricAsset {
        path: "metric.tfm".into(),
        sha256: sha256(bytes),
        license_path: "LICENSE".into(),
        license_sha256: sha256(b"synthetic license"),
    }];
    let loaded = RequiredMetrics::load(&root, &m).unwrap();
    assert!(loaded.get("undeclared.tfm").is_err());
    m.metrics.push(m.metrics[0].clone());
    assert!(matches!(
        RequiredMetrics::load(&root, &m),
        Err(Error::Manifest)
    ));
    m.metrics.pop();
    std::fs::write(d.path().join("metric.tfm"), b"changed").unwrap();
    assert!(matches!(
        RequiredMetrics::load(&root, &m),
        Err(Error::Digest(_))
    ));
    m.metrics[0].path = "../metric.tfm".into();
    assert!(matches!(RequiredMetrics::load(&root, &m), Err(Error::Path)));
}
#[test]
#[ignore = "set FLASHTEX_LM_TFM_DIR to pinned official2.004 metrics and LICENSE directory"]
fn official_required_metrics_match_pinned_pipeline_reader() {
    let dir = std::env::var("FLASHTEX_LM_TFM_DIR").unwrap();
    let root = flashtex_project_files::ProjectRoot::open(std::path::Path::new(&dir)).unwrap();
    let m: Manifest =
        serde_json::from_str(include_str!("../fixtures/lm-required-metrics.json")).unwrap();
    let loaded = RequiredMetrics::load(&root, &m).unwrap();
    for name in ["ec-lmr12", "rm-lmr12", "rm-lmr8", "rm-lmr6"] {
        let (_, t) = loaded.get(&format!("{name}.tfm")).unwrap();
        let expected = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("fixtures/{name}-replay.txt")),
        )
        .unwrap();
        assert_eq!(replay(t), expected, "{name}");
    }
}
