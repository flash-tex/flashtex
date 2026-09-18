//! FT-070 export profile: where the wall clock of
//! `flashtex-pdf-exact from-v2 LIST.json --out OUT.pdf` actually goes.
//!
//! Runs the same four stages the binary runs (read+parse+resolve+embed,
//! serialise, self-check, write) `--rounds` times and prints the median of
//! each, plus a separate breakdown of the font-resolution stage measured
//! through the public `v2::resolve_font` / `TrueTypeFont::load` API.
//!
//! ```text
//! cargo run --release --example export_profile -- LIST.json [--rounds N] [--font-dir DIR]
//! ```

use std::path::{Path, PathBuf};
use std::time::Instant;

use flashtex_pdf::truetype::TrueTypeFont;
use flashtex_pdf::v2::{self, V2Options};
use flashtex_pdf::{exact, json, sha256, verify};

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if v.is_empty() {
        return 0.0;
    }
    v[v.len() / 2]
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut input: Option<String> = None;
    let mut rounds = 11usize;
    let mut options = V2Options::default();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--rounds" => rounds = args.next().unwrap().parse().unwrap(),
            "--font-dir" => options.font_dirs.push(PathBuf::from(args.next().unwrap())),
            s => input = Some(s.to_string()),
        }
    }
    let input = input.expect("LIST.json");
    let path = Path::new(&input);

    let (mut t_read, mut t_build, mut t_render, mut t_verify, mut t_write) =
        (vec![], vec![], vec![], vec![], vec![]);
    let mut total = vec![];
    let out = std::env::temp_dir().join("export_profile.pdf");

    for _ in 0..rounds {
        let all = Instant::now();
        let t = Instant::now();
        let text = std::fs::read_to_string(path).unwrap();
        t_read.push(ms(t));

        let t = Instant::now();
        let (doc, _report) = v2::from_v2(&text, &options).unwrap();
        t_build.push(ms(t));

        let t = Instant::now();
        let rendered = exact::render_exact(&doc).unwrap();
        t_render.push(ms(t));

        let t = Instant::now();
        verify::check_structure(&rendered.bytes).unwrap();
        t_verify.push(ms(t));

        let t = Instant::now();
        std::fs::write(&out, &rendered.bytes).unwrap();
        t_write.push(ms(t));
        total.push(ms(all));
    }

    println!("stage medians over {rounds} rounds (ms)");
    println!("  read file          {:8.2}", median(t_read));
    println!("  from_v2 (build)    {:8.2}", median(t_build));
    println!("  render_exact       {:8.2}", median(t_render));
    println!("  check_structure    {:8.2}", median(t_verify));
    println!("  write file         {:8.2}", median(t_write));
    println!("  ---- in-process    {:8.2}", median(total));

    // Break the build stage down further.
    let text = std::fs::read_to_string(path).unwrap();
    let t = Instant::now();
    let root = json::parse(&text).unwrap();
    let t_json = ms(t);

    let payload = root.get("payload").unwrap();
    let fonts = payload.get("fonts").unwrap().as_array().unwrap();
    let dirs = v2::font_dirs(&options);

    println!("\nbuild-stage detail (one round, ms)");
    println!("  json::parse        {t_json:8.2}   ({} bytes)", text.len());

    let mut resolve_total = 0.0;
    let mut load_total = 0.0;
    let mut hash_bytes = 0u64;
    for f in fonts {
        let sha = f.get("sha256").unwrap().as_str().unwrap();
        let len = f.get("byte_length").unwrap().as_f64().unwrap() as u64;
        let face = f.get("face_index").unwrap().as_f64().unwrap() as u32;
        let name = f.get("postscript_name").unwrap().as_str().unwrap();
        let t = Instant::now();
        let found = v2::resolve_font(&dirs, sha, len, face);
        let r = ms(t);
        resolve_total += r;
        match found {
            Some((p, form)) => {
                hash_bytes += len;
                let t = Instant::now();
                let _ = TrueTypeFont::load(&p).unwrap();
                let l = ms(t);
                load_total += l;
                println!(
                    "  resolve {name:<28} {r:8.2}  load {l:6.2}  {len:>8} B  {form:?}"
                );
            }
            None => println!("  resolve {name:<28} {r:8.2}  NOT FOUND"),
        }
    }
    println!("  resolve_font total {resolve_total:8.2}");
    println!("  truetype load total{load_total:8.2}");

    // How much of resolve_font is raw SHA-256 throughput?
    let mut sha_ms = 0.0;
    let mut sha_total_bytes = 0u64;
    for f in fonts {
        let len = f.get("byte_length").unwrap().as_f64().unwrap() as u64;
        let sha = f.get("sha256").unwrap().as_str().unwrap();
        if let Some((p, _)) = v2::resolve_font(&dirs, sha, len, 0) {
            let bytes = std::fs::read(&p).unwrap();
            let t = Instant::now();
            let _ = sha256::hex(&bytes);
            sha_ms += ms(t);
            sha_total_bytes += bytes.len() as u64;
        }
    }
    println!(
        "\n  one sha256::hex pass over the resolved fonts: {sha_ms:.2} ms for {sha_total_bytes} B \
         ({:.0} MB/s)",
        sha_total_bytes as f64 / 1e6 / (sha_ms / 1000.0)
    );
    println!("  fonts hashed by resolve_font: {hash_bytes} B (each hashed once or twice)");
}
