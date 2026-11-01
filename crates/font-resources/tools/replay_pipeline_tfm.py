#!/usr/bin/env python3
"""Read a pinned peer implementation via Git; compile/run only in a temp directory."""
import hashlib, pathlib, subprocess, tempfile, os, json
root=pathlib.Path(__file__).resolve().parents[3]
source=subprocess.check_output(["git","show","9bb7b27:crates/render-pipeline/src/tfm.rs"],cwd=root)
assert hashlib.sha256(source).hexdigest()=="a41a38585e201ae1a4e6f381be5d1cf8e2994358b97dc12404f41c3d9c181e71"
harness='fn main(){ let t=Tfm::load(std::path::Path::new(&std::env::args().nth(1).unwrap())).unwrap(); for code in 0..=255 { if let Some(m)=t.metrics(code){println!("metric;{},{},{},{},{}",code,m.width,m.height,m.depth,m.italic);} } for n in 1..=32 {if let Some(v)=t.param(n){println!("param;{},{}",n,v);}} for input in [b"fi".as_slice(),b"ff",b"fl",b"ffi",b"ffl",b"AV",b"To",b"WA",b"office",b"wo"] { print!("{}",std::str::from_utf8(input).unwrap());for g in t.ligkern(input){print!(";{},{},{},{}",g.code,g.input.0,g.input.1,g.kern_after);} println!();} }\n'
with tempfile.TemporaryDirectory() as d:
    p=pathlib.Path(d)
    (p/"replay.rs").write_text(source.decode().split("#[cfg(test)]")[0]+"\n"+harness)
    subprocess.run(["rustc","--edition=2021",str(p/"replay.rs"),"-o",str(p/"replay")],check=True)
    result=subprocess.check_output([str(p/"replay"),str(root/"crates/font-engine/fixtures/tfm/ec-lmr10.tfm")])
    assert result==(root/"crates/font-resources/fixtures/pipeline-tfm-replay.txt").read_bytes()
    if "FLASHTEX_LM_TFM_DIR" in os.environ:
        assets=pathlib.Path(os.environ["FLASHTEX_LM_TFM_DIR"])
        manifest=json.loads((root/"crates/font-resources/fixtures/lm-required-metrics.json").read_text())
        for asset in manifest["metrics"]:
            file=assets/asset["path"]
            assert hashlib.sha256(file.read_bytes()).hexdigest()==asset["sha256"]
            assert hashlib.sha256((assets/asset["license_path"]).read_bytes()).hexdigest()==asset["license_sha256"]
            result=subprocess.check_output([str(p/"replay"),str(file)])
            expected=root/"crates/font-resources/fixtures"/(file.stem+"-replay.txt")
            assert result==expected.read_bytes(), file.name
print("Pinned peer metric/parameter/ten-run replay matches; no visual claim")
