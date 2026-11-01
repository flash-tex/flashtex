use flashtex_font_resources::{
    sha256,
    tfm::{BoundaryOptions, Tfm},
};
use std::fmt::Write;
#[test]
fn real_lm_metrics_and_attached_kerns_match_published_pipeline_reader() {
    let bytes = include_bytes!("../../font-engine/fixtures/tfm/ec-lmr10.tfm");
    assert_eq!(
        sha256(bytes),
        "cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5"
    );
    let t = Tfm::parse(bytes).unwrap();
    let mut output = String::new();
    for code in 0..=255 {
        if let Some(m) = t.char_metrics(code) {
            writeln!(
                output,
                "metric;{},{},{},{},{}",
                code, m.width.0, m.height.0, m.depth.0, m.italic.0
            )
            .unwrap();
        }
    }
    for n in 1..=32 {
        if let Some(v) = t.parameter(n) {
            writeln!(output, "param;{},{}", n, v.0).unwrap();
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
        assert_eq!(r.tfm_sha256, sha256(bytes));
        assert_eq!(r.leading_kern.0, 0);
        write!(output, "{}", std::str::from_utf8(input).unwrap()).unwrap();
        for g in r.glyphs {
            write!(
                output,
                ";{},{},{},{}",
                g.code, g.input_start, g.input_end, g.kern_after.0
            )
            .unwrap();
        }
        writeln!(output).unwrap();
    }
    assert_eq!(output, include_str!("../fixtures/pipeline-tfm-replay.txt"));
    assert!(t
        .glyph_run(&vec![b'A'; 4097], BoundaryOptions::default())
        .is_err());
}
