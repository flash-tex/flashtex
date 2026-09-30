// zrs DIR N: zb.c's test (deflate each DIR/i.raw at level 9, compare with
// pdfTeX's DIR/i.z, time 20 passes) with zlib-rs 0.6.7 (the zlib-ng port
// that flate2's `zlib-rs` backend uses) and miniz_oxide 0.8.9.
// Build: copy to /tmp/l6o/zrs/src/main.rs with Cargo.toml beside it.
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a[2].parse().unwrap();
    let files: Vec<(Vec<u8>, Vec<u8>)> = (0..n)
        .filter_map(|i| {
            Some((
                std::fs::read(format!("{}/{i}.raw", a[1])).ok()?,
                std::fs::read(format!("{}/{i}.z", a[1])).ok()?,
            ))
        })
        .collect();
    let mut out = vec![0u8; 64 << 20];
    let (mut same, mut diff, mut t) = (0, 0, 0.0);
    for rep in 0..20 {
        for (raw, z) in &files {
            let t0 = Instant::now();
            let (o, _) = zlib_rs::compress_slice(&mut out, raw, zlib_rs::DeflateConfig::new(9));
            t += t0.elapsed().as_secs_f64();
            if rep == 0 {
                if o == &z[..] {
                    same += 1
                } else {
                    diff += 1
                }
            }
        }
    }
    println!(
        "zlib-rs 0.6.7: {same} identical, {diff} different; {:.1} ms per pass",
        t * 1000.0 / 20.0
    );
    let (mut same, mut diff, mut t) = (0, 0, 0.0);
    for rep in 0..20 {
        for (raw, z) in &files {
            let t0 = Instant::now();
            let o = miniz_oxide::deflate::compress_to_vec_zlib(raw, 9);
            t += t0.elapsed().as_secs_f64();
            if rep == 0 {
                if o == *z {
                    same += 1
                } else {
                    diff += 1
                }
            }
        }
    }
    println!(
        "miniz_oxide 0.8.9: {same} identical, {diff} different; {:.1} ms per pass",
        t * 1000.0 / 20.0
    );
}
