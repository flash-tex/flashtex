//! The one test that touches the network, opt-in through
//! `FLASHTEX_NETWORK_TESTS=1`: fetches a tiny real package (`cancel`, one
//! `.sty`) from CTAN into a temporary cache and checks the layout, then
//! checks that a `.dtx`-only package (`lipsum`) is reported as needing
//! docstrip. Skipped, loudly, otherwise.

use std::path::PathBuf;

use flashtex_package_resolver::http::HttpFetcher;
use flashtex_package_resolver::{FetchPolicy, PackageSource, Policy, Resolution, Resolver};

fn tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let p = std::env::temp_dir().join(format!("flashtex-package-resolver-net-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn fetches_cancel_from_ctan_when_opted_in() {
    if std::env::var("FLASHTEX_NETWORK_TESTS").as_deref() != Ok("1") {
        eprintln!("skipped: set FLASHTEX_NETWORK_TESTS=1 to fetch `cancel` from CTAN");
        return;
    }
    let root = tmp("cancel");
    let fetcher = HttpFetcher::new().unwrap();
    let resolver = Resolver::new(&root, &fetcher);
    let policy = Policy { source: PackageSource::Ctan, fetch: FetchPolicy::Ask, pin: None };

    let asked = resolver.resolve("cancel", &policy);
    let version = match &asked {
        Resolution::NeedsConsent { version, source_url, would_fetch, .. } => {
            assert_eq!(source_url, "https://mirrors.ctan.org/macros/latex/contrib/cancel/");
            assert_eq!(would_fetch, &["cancel.sty".to_string()]);
            version.clone().expect("CTAN states a version")
        }
        other => panic!("expected NeedsConsent, got {other:?}"),
    };
    assert!(!root.join("cancel").exists(), "ask fetched nothing");

    let fetched = resolver.resolve_with_consent("cancel", &policy);
    match &fetched {
        Resolution::Fetched { version: v, files, .. } => {
            assert_eq!(v, &version);
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].name, "cancel.sty");
            assert!(files[0].text.contains("\\ProvidesPackage{cancel}"), "{}", &files[0].text[..200.min(files[0].text.len())]);
            assert_eq!(files[0].path, root.join("cancel").join(&version).join("cancel.sty"));
        }
        other => panic!("expected Fetched, got {other:?}"),
    }
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("cancel").join(&version).join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["name"], "cancel");
    assert_eq!(manifest["version"], version);
    assert_eq!(manifest["files"][0]["name"], "cancel.sty");
    eprintln!("fetched cancel {version}: {}", serde_json::to_string(&manifest).unwrap());

    // Cached now: no network for a `never` policy.
    assert!(matches!(resolver.resolve("cancel", &Policy::never()), Resolution::Cached { .. }));

    // lipsum ships lipsum.dtx/lipsum.ins only.
    let lipsum = resolver.resolve("lipsum", &policy);
    match &lipsum {
        Resolution::NotAvailable { reason, .. } => assert!(reason.starts_with("needs docstrip"), "{reason}"),
        other => panic!("expected NotAvailable(needs docstrip), got {other:?}"),
    }
    eprintln!("lipsum: {lipsum:?}");
    let _ = std::fs::remove_dir_all(&root);
}
