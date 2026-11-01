//! Opt-in installed-resource regression. No reference engine, font copies or visual assertions.
use flashtex_font_resources::{cff::*, sha256};
use sha2::{Digest, Sha256};
fn rational(hash: &mut Sha256, r: Rational) {
    hash.update(r.numerator().to_be_bytes());
    hash.update(r.denominator().to_be_bytes());
}
fn point(hash: &mut Sha256, p: RationalPoint) {
    rational(hash, p.x);
    rational(hash, p.y);
}
#[test]
#[ignore = "requires exact installed licensed STIX font and OFL record; see fixture manifest"]
fn pinned_stix_cff_geometry_and_cache_replay() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/stix-cff.json")).unwrap();
    let path = std::env::var("FLASHTEX_STIX_FONT")
        .unwrap_or_else(|_| fixture["font_path"].as_str().unwrap().into());
    let license = std::env::var("FLASHTEX_STIX_LICENSE")
        .unwrap_or_else(|_| fixture["license_path"].as_str().unwrap().into());
    let bytes = std::fs::read(path)
        .expect("install/provide the declared resource; missing is pending, not fidelity success");
    assert_eq!(
        bytes.len() as u64,
        fixture["font_byte_length"].as_u64().unwrap()
    );
    assert_eq!(sha256(&bytes), fixture["font_sha256"].as_str().unwrap());
    assert_eq!(
        sha256(&std::fs::read(license).unwrap()),
        fixture["license_sha256"].as_str().unwrap()
    );
    let start = fixture["cff_range"][0].as_u64().unwrap() as usize;
    let end = fixture["cff_range"][1].as_u64().unwrap() as usize;
    let cff = Cff::parse(&bytes[start..end]).unwrap();
    assert_eq!(cff.sha256, fixture["cff_sha256"].as_str().unwrap());
    assert_eq!(
        cff.glyph_count() as u64,
        fixture["glyph_count"].as_u64().unwrap()
    );
    let limits = CacheLimits {
        max_entries: 4096,
        max_bytes: 32 * 1024 * 1024,
    };
    let mut cache = CffOutlineCache::from_font_table(&bytes, 0, start..end, limits).unwrap();
    let mut hash = Sha256::new();
    hash.update(b"cff-matrix-geometry-v1\0");
    let mut commands = 0;
    let start = std::time::Instant::now();
    let mut hits = 0;
    let mut direct_time = std::time::Duration::ZERO;
    let mut cold_time = std::time::Duration::ZERO;
    for gid in 0..cff.glyph_count() as u16 {
        let direct_start = std::time::Instant::now();
        let direct = cff
            .matrix_outline(gid, HintPolicy::Unhinted)
            .unwrap_or_else(|e| panic!("gid{gid}: {e}"));
        direct_time += direct_start.elapsed();
        let cold_start = std::time::Instant::now();
        let cached = cache.lookup(gid, HintPolicy::Unhinted);
        cold_time += cold_start.elapsed();
        assert_eq!(cached.status, CacheStatus::Stored);
        let cached = cached.result.unwrap();
        assert_eq!(*cached, direct);
        let repeat = cache.lookup(gid, HintPolicy::Unhinted);
        assert_eq!(repeat.status, CacheStatus::Hit);
        assert!(std::sync::Arc::ptr_eq(&cached, &repeat.result.unwrap()));
        hits += 1;
        assert!(cache.retained_bytes() <= limits.max_bytes);
        assert!(cache.len() <= limits.max_entries);
        hash.update(gid.to_be_bytes());
        point(&mut hash, direct.advance);
        hash.update((direct.commands.len() as u32).to_be_bytes());
        commands += direct.commands.len();
        for command in direct.commands {
            match command {
                MatrixCommand::MoveTo(p) => {
                    hash.update([0]);
                    point(&mut hash, p);
                }
                MatrixCommand::LineTo(p) => {
                    hash.update([1]);
                    point(&mut hash, p);
                }
                MatrixCommand::CurveTo {
                    control1,
                    control2,
                    end,
                } => {
                    hash.update([2]);
                    point(&mut hash, control1);
                    point(&mut hash, control2);
                    point(&mut hash, end);
                }
                MatrixCommand::Close => hash.update([3]),
            }
        }
    }
    let warm_start = std::time::Instant::now();
    for gid in 0..cff.glyph_count() as u16 {
        let outcome = cache.lookup(gid, HintPolicy::Unhinted);
        assert_eq!(outcome.status, CacheStatus::Hit);
        std::hint::black_box(outcome.result.unwrap());
    }
    let warm_time = warm_start.elapsed();
    assert_eq!(
        commands as u64,
        fixture["expected_commands"].as_u64().unwrap()
    );
    eprintln!(
        "direct_decode_ms={} cache_cold_ms={} full_warm_replay_ms={}",
        direct_time.as_secs_f64() * 1000.0,
        cold_time.as_secs_f64() * 1000.0,
        warm_time.as_secs_f64() * 1000.0
    );
    let geometry = format!("{:x}", hash.finalize());
    eprintln!("{{\"geometry_sha256\":\"{geometry}\",\"glyphs\":{},\"commands\":{commands},\"repeat_hits\":{hits},\"elapsed_ms\":{},\"retained_bytes\":{}}}",cff.glyph_count(),start.elapsed().as_secs_f64()*1000.0,cache.retained_bytes());
    if std::env::var_os("FLASHTEX_RECORD_CFF").is_none() {
        assert_eq!(
            geometry,
            fixture["geometry_sha256"]
                .as_str()
                .expect("record measured baseline first")
        );
    }
}
