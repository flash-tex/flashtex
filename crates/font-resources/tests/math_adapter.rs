use flashtex_font_engine::{Face, TrueTypeFace};
use flashtex_font_resources::{registry::*, *};
use flashtex_project_files::ProjectRoot;
fn declaration(bytes: &[u8], license: &[u8], id: &str, format: &str) -> ManifestEntry {
    let face = TrueTypeFace::parse(bytes.to_vec()).unwrap();
    ManifestEntry {
        font: FontDescriptor {
            font_id: id.into(),
            sha256: sha256(bytes),
            byte_length: bytes.len() as u64,
            format: format.into(),
            face_index: 0,
            units_per_em: face.units_per_em() as u32,
            glyph_count: face.num_glyphs() as u32,
            postscript_name: face.postscript_name().into(),
        },
        path: format!("{id}.font"),
        license: LicenseMetadata {
            identifier: "OFL-1.1".into(),
            copyright: "see exact pinned license".into(),
            source: "installed licensed resource".into(),
            text_path: format!("{id}.license"),
            text_sha256: sha256(license),
            embedding_permission: EmbeddingPermission::Allowed,
        },
    }
}
use flashtex_font_resources::math_adapter::*;
#[test]
#[ignore = "requires pinned installed STIX Math and OFL; parser equivalence, not visual oracle"]
fn pinned_math_registry_equivalence() {
    let bytes = std::fs::read("/usr/share/fonts/stix-fonts/STIXTwoMath-Regular.otf").unwrap();
    let license = std::fs::read("/usr/share/licenses/stix-fonts/OFL.txt").unwrap();
    assert_eq!(
        sha256(&bytes),
        "3a5f3f26f40d5698b3c62dd085d48d6663696a3f80825aab8b553d5097518e8c"
    );
    assert_eq!(
        sha256(&license),
        "0c8825913b60d858aacdb33c4ca6660a7d64b0d6464702efbb19313f5765861a"
    );
    let face = TrueTypeFace::parse(bytes.clone()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let root = ProjectRoot::open(dir.path()).unwrap();
    let resource = declaration(&bytes, &license, "math", "static-cff");
    let binding = StyleBinding {
        family: "Explicit Math".into(),
        weight: 400,
        style: FontStyle::Upright,
    };
    std::fs::write(dir.path().join(&resource.path), &bytes).unwrap();
    std::fs::write(dir.path().join(&resource.license.text_path), &license).unwrap();
    std::fs::write(
        dir.path().join("fonts.json"),
        serde_json::to_vec(&RegistryManifest {
            schema_version: 1,
            entries: vec![RegistryEntry {
                binding: binding.clone(),
                resource: resource.clone(),
            }],
        })
        .unwrap(),
    )
    .unwrap();
    let registry =
        ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()).unwrap();
    assert!(BoundMathFont::from_registry(
        &registry,
        &binding,
        "stale",
        MathPolicy::UnhintedDesignUnits
    )
    .is_err());
    let bound = BoundMathFont::from_registry(
        &registry,
        &binding,
        registry.generation(),
        MathPolicy::UnhintedDesignUnits,
    )
    .unwrap();
    assert_eq!(&bound.identity().declaration, &resource);
    assert_eq!(&bound.identity().font.engine_font_id, face.id());
    assert_eq!(
        bound.identity().math_table_sha256,
        sha256(face.table(b"MATH").unwrap())
    );
    assert_eq!(bound.constants(), &face.math().unwrap().constants);
    for start in (0..face.num_glyphs()).step_by(256) {
        let ids: Vec<_> = (start..start.saturating_add(256).min(face.num_glyphs())).collect();
        for actual in bound.glyphs(&ids).unwrap() {
            let gid = flashtex_font_engine::GlyphId(actual.glyph_id);
            assert_eq!(
                actual.italic_correction,
                face.math().unwrap().italics_correction(gid)
            );
            assert_eq!(
                actual.top_accent_attachment,
                face.math().unwrap().top_accent_attachment(gid)
            );
        }
    }
    assert!(matches!(
        bound.glyphs(&[face.num_glyphs()]),
        Err(MathError::GlyphOutOfBounds(_))
    ));
    assert!(matches!(
        bound.glyphs(&[0; 257]),
        Err(MathError::LookupBudget)
    ));
    for capability in [
        Capability::DeviceAdjustments,
        Capability::Variants,
        Capability::MathKern,
        Capability::ExtendedShapeCoverage,
    ] {
        assert!(
            matches!(bound.require(capability),Err(MathError::Unsupported(c)) if c==capability)
        );
    }
    let variants = bound.variants().unwrap();
    assert_eq!(variants.identity(), bound.identity());
    let constructions = variants.data().constructions();
    let assemblies = constructions
        .values()
        .filter(|c| c.assembly.is_some())
        .count();
    let records: usize = constructions.values().map(|c| c.variants.len()).sum();
    assert_eq!(constructions.len(), 165);
    assert_eq!(assemblies, 69);
    assert_eq!(records, 633);
    for (&(direction, gid), c) in constructions {
        for variant in &c.variants {
            assert!(
                matches!(variants.data().select(direction,gid,variant.advance.into()),flashtex_font_resources::math_variants::Selection::Variant(v) if v.advance>=variant.advance)
            );
        }
    }
    println!(
        "variants constructions {} assemblies {} records {}",
        constructions.len(),
        assemblies,
        records
    );
    use flashtex_font_resources::{cff::Rational, math_fit::*, math_variants::Direction};
    let mut fitted = 0;
    let mut refused = 0;
    for (&(direction, gid), c) in constructions {
        if c.assembly.is_none() {
            continue;
        }
        let target = Rational::new(10001, 2).unwrap();
        match variants.fit(
            direction,
            gid,
            target,
            FitStrategy::EqualExtendersProportionalConnectorFlexibility,
            FitLimits::default(),
        ) {
            Ok(result) => {
                assert_eq!(result.identity(), bound.identity());
                if let FittedShape::Assembly(a) = &result.fit().shape {
                    assert_eq!(a.advance, target);
                }
                fitted += 1;
            }
            Err(error) => {
                println!("fit refusal {direction:?} GID {gid}: {error}");
                refused += 1;
            }
        }
    }
    for character in ['(', ')', '[', ']', '∫'] {
        let gid = face.glyph_id(character).unwrap();
        let fit = variants.fit(
            Direction::Vertical,
            gid.0,
            Rational::new(5000, 1).unwrap(),
            FitStrategy::EqualExtendersProportionalConnectorFlexibility,
            FitLimits::default(),
        );
        assert!(
            fit.is_ok(),
            "tall {character} GID {}: {:?}",
            gid.0,
            fit.err()
        );
        println!("tall {character} GID {} fitted true", gid.0);
    }
    println!("STIX 5000.5-unit assemblies fitted {fitted} refused {refused}");
    assert_eq!((fitted, refused), (66, 3));
    let kerns = bound.kerns().unwrap();
    assert_eq!(kerns.identity(), bound.identity());
    let mut height_records = 0;
    let mut device_records = 0;
    for (&(gid, corner), table) in kerns.data().records() {
        height_records += table.correction_heights().len();
        device_records += table
            .correction_heights()
            .iter()
            .chain(table.kern_values())
            .filter(|v| v.device_adjustment_present)
            .count();
        for (i, height) in table.correction_heights().iter().enumerate() {
            assert_eq!(
                kerns
                    .data()
                    .lookup(
                        gid,
                        corner,
                        Rational::new(height.design_units.into(), 1).unwrap()
                    )
                    .unwrap(),
                table.kern_values()[i + 1]
            );
            assert_eq!(
                kerns
                    .data()
                    .lookup(
                        gid,
                        corner,
                        Rational::new(i128::from(height.design_units) * 2 - 1, 2).unwrap()
                    )
                    .unwrap(),
                table.kern_values()[i]
            );
        }
    }
    assert_eq!(
        (kerns.data().records().len(), height_records, device_records),
        (219, 217, 0)
    );
    println!(
        "STIX kern corner tables {} heights {height_records} device records {device_records}",
        kerns.data().records().len()
    );
    use flashtex_font_resources::math_device::{ConstantDeviceRecord, DeviceContext};
    for index in 0..51 {
        let correction = bound
            .constant_device(
                ConstantDeviceRecord::new(index).unwrap(),
                DeviceContext::new(12).unwrap(),
            )
            .unwrap();
        assert_eq!(correction.identity(), bound.identity());
        assert_eq!(correction.correction().delta_pixels, 0);
        assert_eq!(correction.correction().device_table_offset, None);
    }
    use flashtex_font_resources::math_device::{GlyphDeviceKind, KernDeviceContext};
    let context = DeviceContext::new(12).unwrap();
    let mut glyph_device_records = 0;
    for gid in 0..bound.glyph_count() {
        for kind in [
            GlyphDeviceKind::ItalicCorrection,
            GlyphDeviceKind::TopAccentAttachment,
        ] {
            let result = bound.glyph_device(gid, kind, context).unwrap();
            assert_eq!(result.identity(), bound.identity());
            let expected = match kind {
                GlyphDeviceKind::ItalicCorrection => Some(
                    face.math()
                        .unwrap()
                        .italics_correction(flashtex_font_engine::GlyphId(gid)),
                ),
                GlyphDeviceKind::TopAccentAttachment => face
                    .math()
                    .unwrap()
                    .top_accent_attachment(flashtex_font_engine::GlyphId(gid)),
            };
            assert_eq!(result.design_units(), expected);
            if result
                .correction()
                .is_some_and(|r| r.device_table_offset.is_some())
            {
                let (expected_sha, expected_delta) = match gid {
                    3309 => (
                        "ab2eb9ea308cd688fdf24e8195634163507dd7f547f36fb1f98956f83811d741",
                        0,
                    ),
                    3316 => (
                        "c447e66453f3bd7a9d32f0913d4961fb205bd25ad46ebcef056f26609709b583",
                        0,
                    ),
                    3326 => (
                        "cd26325e3c7a478bc246d4a9019d275d88037e830e9a581cdc79d1ea5b1c175e",
                        1,
                    ),
                    4010 => (
                        "5d892fad4a022dfec134f4da2a1f75a9bba80c4f35bbf439bffbb0fb62fcbaf1",
                        0,
                    ),
                    _ => panic!("unexpected device record"),
                };
                assert_eq!(
                    result.correction().unwrap().device_table_sha256.as_deref(),
                    Some(expected_sha)
                );
                assert_eq!(result.correction().unwrap().delta_pixels, expected_delta);
                println!("device GID {gid} {kind:?}: {:?}", result.correction());
                glyph_device_records += 1;
            }
        }
    }
    for (&(gid, corner), table) in kerns.data().records() {
        let height = Rational::new(0, 1).unwrap();
        let result = bound
            .kern_device(
                gid,
                corner,
                height,
                KernDeviceContext {
                    horizontal: context,
                    vertical: context,
                },
            )
            .unwrap();
        assert_eq!(result.identity(), bound.identity());
        assert_eq!(
            result.correction().correction.design_units,
            table.lookup(height).unwrap().design_units
        );
        assert_eq!(result.correction().correction.delta_pixels, 0);
    }
    println!("STIX glyph italic/accent device records {glyph_device_records}");
    assert_eq!(glyph_device_records, 4);
    let benchmark_start = std::time::Instant::now();
    let (&(benchmark_gid, benchmark_corner), _) = kerns.data().records().first_key_value().unwrap();
    for _ in 0..100 {
        std::hint::black_box(
            bound
                .kern_device(
                    benchmark_gid,
                    benchmark_corner,
                    Rational::new(0, 1).unwrap(),
                    KernDeviceContext {
                        horizontal: context,
                        vertical: context,
                    },
                )
                .unwrap(),
        );
    }
    println!(
        "STIX uncached100 kern-device queries {:?}",
        benchmark_start.elapsed()
    );
    use flashtex_font_resources::math_cache::{
        Limits, MathQueryCache, Query, Value as CachedValue,
    };
    let mut cache = MathQueryCache::new(&bound, Limits::default()).unwrap();
    let query = Query::Kern {
        glyph_id: benchmark_gid,
        corner: benchmark_corner,
        height: Rational::new(0, 1).unwrap(),
        context: KernDeviceContext {
            horizontal: context,
            vertical: context,
        },
    };
    let expected = bound
        .kern_device(
            benchmark_gid,
            benchmark_corner,
            Rational::new(0, 1).unwrap(),
            KernDeviceContext {
                horizontal: context,
                vertical: context,
            },
        )
        .unwrap();
    println!("STIX cache benchmark GID {benchmark_gid} corner {benchmark_corner:?} height0 ppem12");
    let cached_start = std::time::Instant::now();
    for _ in 0..100 {
        let result = cache.query(bound.identity(), query.clone()).unwrap();
        let Ok(CachedValue::Kern(value)) = result.outcome.as_ref() else {
            panic!("kern outcome")
        };
        assert_eq!(value, expected.correction());
    }
    println!(
        "STIX cached100 kern-device queries {:?} stats {:?}",
        cached_start.elapsed(),
        cache.stats()
    );
    assert_eq!(cache.stats().kern_parses, 1);
    assert_eq!(cache.stats().hits, 99);
    let unhinted_query = Query::UnhintedKern {
        glyph_id: benchmark_gid,
        corner: benchmark_corner,
        height: Rational::new(0, 1).unwrap(),
    };
    let unhinted = cache
        .query(bound.identity(), unhinted_query.clone())
        .unwrap();
    assert!(
        matches!(unhinted.outcome.as_ref(),Ok(CachedValue::UnhintedKern{value,height_device_adjustment_present:false}) if value==&kerns.data().lookup(benchmark_gid,benchmark_corner,Rational::new(0,1).unwrap()).unwrap())
    );
    cache.query(bound.identity(), unhinted_query).unwrap();
    assert_eq!(cache.stats().kern_parses, 1);
    // Different height requires computation but reuses the immutable parsed table.
    let distinct = Query::Kern {
        glyph_id: benchmark_gid,
        corner: benchmark_corner,
        height: Rational::new(1, 2).unwrap(),
        context: KernDeviceContext {
            horizontal: context,
            vertical: context,
        },
    };
    cache.query(bound.identity(), distinct).unwrap();
    assert_eq!(cache.stats().kern_parses, 1);
    for ppem in [12, 13] {
        let q = Query::Glyph {
            glyph_id: 3326,
            kind: GlyphDeviceKind::TopAccentAttachment,
            context: DeviceContext::new(ppem).unwrap(),
        };
        let cached = cache.query(bound.identity(), q).unwrap();
        let Ok(CachedValue::Glyph {
            design_units,
            correction,
        }) = cached.outcome.as_ref()
        else {
            panic!()
        };
        let direct = bound
            .glyph_device(
                3326,
                GlyphDeviceKind::TopAccentAttachment,
                DeviceContext::new(ppem).unwrap(),
            )
            .unwrap();
        assert_eq!(*design_units, direct.design_units());
        assert_eq!(correction.as_ref(), direct.correction());
    }
    let fit_query = Query::Fit {
        glyph_id: 1064,
        direction: Direction::Vertical,
        target: Rational::new(5000, 1).unwrap(),
        strategy: FitStrategy::EqualExtendersProportionalConnectorFlexibility,
        limits: FitLimits::default(),
    };
    let direct = variants
        .fit(
            Direction::Vertical,
            1064,
            Rational::new(5000, 1).unwrap(),
            FitStrategy::EqualExtendersProportionalConnectorFlexibility,
            FitLimits::default(),
        )
        .unwrap();
    let cached = cache.query(bound.identity(), fit_query.clone()).unwrap();
    assert!(matches!(cached.outcome.as_ref(),Ok(CachedValue::Fit(v)) if v==direct.fit()));
    cache.query(bound.identity(), fit_query).unwrap();
    assert_eq!(cache.stats().variant_parses, 1);
    // Held registry resources remain immutable when the project file changes.
    std::fs::write(dir.path().join(&resource.path), b"changed").unwrap();
    assert_eq!(bound.constants(), &face.math().unwrap().constants);
    assert!(ProjectFontRegistry::load(&root, "fonts.json", RegistryLimits::default()).is_err());
    println!(
        "MATH SHA {} bytes {} glyphs {}",
        bound.identity().math_table_sha256,
        bound.identity().math_table_byte_length,
        bound.glyph_count()
    );
}

#[test]
#[ignore = "requires pinned installed Noto Math and license; inventory evidence only"]
fn installed_noto_math_inventory() {
    let bytes = std::fs::read("/usr/share/fonts/google-noto/NotoSansMath-Regular.ttf").unwrap();
    let license = std::fs::read("/usr/share/licenses/google-noto-fonts-common/LICENSE").unwrap();
    assert_eq!(
        sha256(&bytes),
        "d51afd5739c7ba6c44fcab35a88160e25dfb69a2d4ad0bd99533f8d894af1f96"
    );
    assert_eq!(
        sha256(&license),
        "c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4"
    );
    let face = TrueTypeFace::parse(bytes).unwrap();
    println!(
        "Noto Math MATH table present {}",
        face.table(b"MATH").is_some()
    );
    let math = face.table(b"MATH").unwrap();
    let read = |at: usize| u16::from_be_bytes(math[at..at + 2].try_into().unwrap()) as usize;
    let base = read(4);
    let mut present = 0;
    for index in 0..51 {
        let offset = read(base + 10 + index * 4);
        if offset != 0 {
            present += 1;
            let table =
                flashtex_font_resources::math_device::DeviceTable::parse(math, base + offset)
                    .unwrap();
            println!("Noto record {index} device range {:?}", table.range());
        }
    }
    assert_eq!(present, 0);
    assert_eq!(
        sha256(math),
        "e6ba971107625ed4c384230b1a84191c745e9a97b1b39ce12692d0d987126909"
    );
    println!("Noto MATH SHA {} constants devices {present}", sha256(math));
}
