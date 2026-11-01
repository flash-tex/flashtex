//! The config resolver reproduces, for every font dumped by the oracle
//! fixtures, pdfTeX's `\lpcode`/`\rpcode`/`\efcode` tables (all 256 slots)
//! and expansion limits exactly, from the bundled microtype.cfg/mt-cmr.cfg
//! and the font's `\fontcharwd`/`\fontdimen6`.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::collapsible_if, clippy::needless_late_init, clippy::while_immutable_condition, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

mod common;

use std::fs;
use std::path::Path;

use common::*;
use flashtex_microtype::config::{FontMetrics, MicrotypeConfig, NfssDefaults, NfssFont, Options};

struct Dumped<'a>(&'a FontEntry);

impl FontMetrics for Dumped<'_> {
    fn char_width(&self, slot: u8) -> i32 {
        self.0.widths[slot as usize]
    }
    fn quad(&self) -> i32 {
        self.0.params.quad
    }
}

#[test]
fn resolved_codes_match_pdftex() {
    let cfg = MicrotypeConfig::bundled();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/oracle/expected");
    let mut checked = 0;
    let mut paths: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let fx = load_fixture(&name, &fs::read_to_string(&path).unwrap());
        let mode = name.rsplit('-').next().unwrap();
        let lm = name.contains("-lm-");
        let defaults = if lm {
            NfssDefaults::latex("T1", "lmr", "lmss", "lmtt")
        } else {
            NfssDefaults::latex("T1", "cmr", "cmss", "cmtt")
        };
        let options = Options {
            protrusion: mode == "pr" || mode == "both",
            expansion: mode == "ex" || mode == "both",
            ..Options::default()
        };
        for (fname, entry) in &fx.fonts {
            let font = NfssFont::parse(fname).unwrap();
            let got = cfg.resolve(&options, &defaults, &font, &Dumped(entry)).unwrap();
            for c in 0..256 {
                assert_eq!(
                    (got.params.lpcode[c], got.params.rpcode[c], got.params.efcode[c]),
                    (entry.params.lpcode[c], entry.params.rpcode[c], entry.params.efcode[c]),
                    "{name} {fname} slot {c} (list {:?})",
                    got.protrusion_list
                );
            }
            // step is not observable from the probes; microtype's log reports 1
            let lim = |p: &flashtex_microtype::FontParams| p.expansion.map(|l| (l.stretch, l.shrink));
            assert_eq!(lim(&got.params), lim(&entry.params), "{name} {fname} expansion limits");
            assert!(got.warnings.is_empty(), "{name} {fname}: {:?}", got.warnings);
            checked += 1;
        }
    }
    println!("checked {checked} fonts");
    assert!(checked >= 90);
}
