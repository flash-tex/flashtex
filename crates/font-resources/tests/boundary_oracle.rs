//! Shared expectations only. Passing this test does not execute the independent TeX oracle.
use flashtex_font_resources::{
    sha256,
    tfm::{Tfm, TfmItem},
};
#[test]
fn synthetic_boundary_packet_preserves_exact_actions_and_intervals() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/boundary-oracle/cases.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let hex = case["tfm_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(sha256(&bytes), case["tfm_sha256"].as_str().unwrap());
        let input: Vec<u8> = case["input_codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| u8::try_from(n.as_u64().unwrap()).unwrap())
            .collect();
        let tfm = Tfm::parse(&bytes).unwrap();
        let actions:Vec<_>=tfm.apply_ligatures_kerns(&input).unwrap().into_iter().map(|item| match item {
   TfmItem::Glyph(g)=>serde_json::json!({"glyph_code":g.code,"input_interval":[g.input_start,g.input_end]}),
   TfmItem::Kern(k)=>serde_json::json!({"kern_fixword":k.0})
  }).collect();
        assert_eq!(
            serde_json::json!(actions),
            case["expected_shared_actions"],
            "{}",
            case["id"]
        );
    }
}
