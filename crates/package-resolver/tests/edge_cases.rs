//! Error paths and edge cases with no unit coverage inside `src/`:
//! invalid names, failed downloads, a corrupt cache, unusable CTAN
//! metadata, listing quirks, `file://` serving and the small pure helpers.
//! Every test drives the public API with an in-memory fake fetcher;
//! nothing here opens a socket.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use flashtex_package_resolver::cache::{Store, MANIFEST_NAME};
use flashtex_package_resolver::http::HttpFetcher;
use flashtex_package_resolver::library::{self, Library};
use flashtex_package_resolver::scan::{package_references, unimplemented_packages, RefKind};
use flashtex_package_resolver::source::{describe, hrefs};
use flashtex_package_resolver::{
    cache_root_from, is_package_file, is_valid_name, FetchError, FetchPolicy, Fetcher,
    PackageSource, Policy, Resolution, ResolvedFile, Resolver, CACHE_ENV,
};

struct Fake {
    responses: HashMap<String, Vec<u8>>,
    down: Vec<String>,
    requests: RefCell<Vec<String>>,
}

impl Fake {
    fn new() -> Self {
        Fake {
            responses: HashMap::new(),
            down: Vec::new(),
            requests: RefCell::new(Vec::new()),
        }
    }

    fn with(mut self, url: &str, body: &str) -> Self {
        self.responses.insert(url.into(), body.as_bytes().to_vec());
        self
    }

    fn down(mut self, url: &str) -> Self {
        self.down.push(url.into());
        self
    }

    fn ctan(self, name: &str, version: &str, listing: &str) -> Self {
        let record = format!(
            r#"{{"version":{{"number":"{version}"}},"ctan":{{"path":"/macros/latex/contrib/{name}"}}}}"#
        );
        self.with(&format!("https://ctan.org/json/2.0/pkg/{name}"), &record)
            .with(
                &format!("https://mirrors.ctan.org/macros/latex/contrib/{name}/"),
                listing,
            )
    }
}

impl Fetcher for Fake {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.requests.borrow_mut().push(url.into());
        if self.down.iter().any(|d| d == url) {
            return Err(FetchError::Transport("dns: no such host".into()));
        }
        self.responses
            .get(url)
            .cloned()
            .ok_or(FetchError::Status(404))
    }
}

struct Down;
impl Fetcher for Down {
    fn get(&self, _: &str) -> Result<Vec<u8>, FetchError> {
        Err(FetchError::Transport("dns: no such host".into()))
    }
}

fn tmp(tag: &str) -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("flashtex-pr-edge-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn ask() -> Policy {
    Policy {
        source: PackageSource::Ctan,
        fetch: FetchPolicy::Ask,
        pin: None,
    }
}

#[test]
fn pure_helper_edges() {
    assert!(is_valid_name(&"a".repeat(100)) && !is_valid_name(&"a".repeat(101)));
    assert!(
        is_valid_name("a+b")
            && is_valid_name("a.b")
            && !is_valid_name("caf\u{e9}")
            && !is_valid_name("a b")
    );
    assert!(
        is_package_file("a.b.sty")
            && !is_package_file("A.STY")
            && !is_package_file("a.")
            && !is_package_file("noext")
    );
}

#[test]
fn refused_before_any_request() {
    let root = tmp("refused");
    let fake = Fake::new();
    let resolver = Resolver::new(&root, &fake);
    for r in [
        resolver.resolve("../x", &ask()),
        resolver.resolve_with_consent("../x", &ask()),
        resolver.fetch("../x", &ask()),
    ] {
        assert!(
            matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("not a package name")),
            "{r:?}"
        );
    }
    let none = Policy {
        source: PackageSource::None,
        fetch: FetchPolicy::Always,
        pin: None,
    };
    for r in [
        resolver.resolve_with_consent("cancel", &none),
        resolver.fetch("cancel", &none),
    ] {
        assert!(
            matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("\"none\"")),
            "{r:?}"
        );
    }
    assert_eq!(fake.requests.borrow().len(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fetch_failures_name_the_url_and_leave_no_cache() {
    let root = tmp("fetchfail");
    let fake = Fake::new().ctan("half", "1", "<a href=\"half.sty\">s</a>");
    let resolver = Resolver::new(&root, &fake);
    let r = resolver.resolve_with_consent("half", &ask());
    assert!(
        matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("cannot fetch https://mirrors.ctan.org/macros/latex/contrib/half/half.sty") && reason.contains("404")),
        "{r:?}"
    );
    assert!(!root.join("half").exists());
    let down = Down;
    let resolver = Resolver::new(&root, &down);
    for r in [
        resolver.resolve("cancel", &ask()),
        resolver.fetch("cancel", &ask()),
    ] {
        assert!(
            matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("cannot reach CTAN")),
            "{r:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn corrupt_cache_is_not_available_never_served() {
    let root = tmp("corrupt");
    let fake = Fake::new();
    let store = Store::new(&root);
    let sty = |t: &str| vec![("c.sty".to_string(), t.as_bytes().to_vec())];
    store.store("manifest", "1", "u", &sty("a")).unwrap();
    std::fs::write(root.join("manifest/1").join(MANIFEST_NAME), "not json").unwrap();
    store.store("tampered", "1", "u", &sty("a")).unwrap();
    std::fs::write(root.join("tampered/1/c.sty"), "changed").unwrap();
    store.store("vanished", "1", "u", &sty("a")).unwrap();
    std::fs::remove_file(root.join("vanished/1/c.sty")).unwrap();
    store
        .store("binary", "1", "u", &[("c.sty".into(), vec![0xff, 0xfe])])
        .unwrap();
    let resolver = Resolver::new(&root, &fake);
    for (name, needle) in [
        ("manifest", "manifest.json"),
        ("tampered", "digest"),
        ("vanished", "cannot read cached"),
        ("binary", "not UTF-8"),
    ] {
        let r = resolver.resolve(name, &Policy::never());
        assert!(
            matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains(needle)),
            "{name}: {r:?}"
        );
    }
    assert_eq!(fake.requests.borrow().len(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn store_validation_clear_and_lookup_edges() {
    let root = tmp("store");
    let store = Store::new(&root);
    let sty = vec![("c.sty".to_string(), b"a".to_vec())];
    assert!(store.store("c", ".1", "u", &sty).is_err());
    assert!(store.store("c", "a\\b", "u", &sty).is_err());
    assert!(store
        .store("c", "1", "u", &[("a\\b.sty".into(), b"x".to_vec())])
        .is_err());
    assert!(store
        .store("c", "1", "u", &[("a\0.sty".into(), b"x".to_vec())])
        .is_err());
    assert!(store.clear(Some("../x")).is_err());
    assert_eq!(store.clear(Some("absent")).unwrap(), 0);
    assert_eq!(store.lookup("../x", None).unwrap(), None);
    assert!(store.versions("absent").unwrap().is_empty());
    store.store("c", "1", "u", &sty).unwrap();
    store
        .store("c", "1", "u", &[("c.sty".into(), b"second".to_vec())])
        .unwrap();
    assert_eq!(
        store
            .lookup("c", Some("1"))
            .unwrap()
            .unwrap()
            .read()
            .unwrap()[0]
            .text,
        "second"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn versions_and_list_skip_foreign_entries() {
    let root = tmp("foreign");
    let store = Store::new(&root);
    std::fs::create_dir_all(root.join("pkg/.tmp-1-99")).unwrap();
    std::fs::write(root.join("pkg/.tmp-1-99").join(MANIFEST_NAME), "{}").unwrap();
    std::fs::write(root.join("pkg/loose-file"), "x").unwrap();
    assert!(
        store.versions("pkg").unwrap().is_empty(),
        "staging leftovers and files are not versions"
    );
    std::fs::create_dir_all(root.join("has space/1")).unwrap();
    std::fs::write(root.join("has space/1").join(MANIFEST_NAME), "garbage").unwrap();
    assert!(
        store.list().unwrap().is_empty(),
        "an invalid name is never even read"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn ctan_metadata_edges_listing_errors_and_dedup() {
    let fake = Fake::new()
        .with("https://ctan.org/json/2.0/pkg/blank", r#"{"version":{"number":"  "},"ctan":{"path":"/x"}}"#)
        .with("https://ctan.org/json/2.0/pkg/scheme", r#"{"version":{"number":"1"},"ctan":{"path":"https://evil/x"}}"#)
        .with("https://ctan.org/json/2.0/pkg/rel", r#"{"version":{"number":"1"},"ctan":{"path":"macros/x"}}"#)
        .ctan("dup", "1", "<a href=\"b.sty\">b</a><a href=\"a.sty\">a</a><a href=\"b.sty\">b</a><a href=\"?C=N\">s</a>")
        .down("https://mirrors.ctan.org/macros/latex/contrib/flaky/")
        .with("https://ctan.org/json/2.0/pkg/flaky", r#"{"version":{"number":"1"},"ctan":{"path":"/macros/latex/contrib/flaky"}}"#);
    assert!(describe(&fake, &PackageSource::Ctan, "blank")
        .unwrap_err()
        .contains("states no version"));
    assert!(describe(&fake, &PackageSource::Ctan, "scheme")
        .unwrap_err()
        .contains("refuses"));
    assert!(describe(&fake, &PackageSource::Ctan, "rel")
        .unwrap_err()
        .contains("refuses"));
    assert_eq!(
        describe(&fake, &PackageSource::Ctan, "dup").unwrap().files,
        ["a.sty", "b.sty"]
    );
    assert!(describe(&fake, &PackageSource::Ctan, "flaky")
        .unwrap_err()
        .contains("cannot list"));
}

#[test]
fn href_edge_cases() {
    assert!(
        hrefs(r#"<a href=cancel.sty>x</a>"#).is_empty(),
        "unquoted links are not links"
    );
    assert!(
        hrefs(r#"<a href="abc"#).is_empty(),
        "an unterminated quote ends the scan"
    );
    assert!(hrefs(&format!(
        "<a href=\"..\">p</a><a href=\"{}\">c</a><a href=\"a\\b.sty\">b</a>",
        "a\x01b.sty"
    ))
    .is_empty());
    assert_eq!(
        hrefs(r#"<a href="a%zz.sty">x</a>"#),
        ["a%zz.sty"],
        "bad escapes stay literal"
    );
}

#[test]
fn library_provides_contents_and_load_errors() {
    let rf = |n: &str| ResolvedFile {
        name: n.into(),
        path: PathBuf::from("/x").join(n),
        text: "t".into(),
        generated_from: None,
    };
    let lib = Library {
        name: "m".into(),
        dir: PathBuf::from("/x"),
        files: vec![rf("m.def"), rf("m.clo")],
    };
    assert!(!lib.provides("m"), "only .sty/.cls provide a name");

    let root = tmp("libedge");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(root.join("afile"), "x").unwrap();
    assert!(library::load("m", "../afile", &project)
        .unwrap_err()
        .contains("not a directory"));
    let dir = root.join("lib");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("flashtex.toml"), "[library]\nname = \"m\"\n").unwrap();
    std::fs::write(dir.join("m.sty"), "s").unwrap();
    std::fs::write(dir.join("m.def"), "d").unwrap();
    std::fs::write(dir.join("m.clo"), "c").unwrap();
    std::fs::write(dir.join("m.cfg"), "cfg").unwrap();
    let loaded = library::load("m", "../lib", &project).unwrap();
    assert_eq!(
        loaded
            .files
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["m.clo", "m.def", "m.sty"]
    );
    std::fs::write(dir.join("a.sty"), [0xff, 0xfe]).unwrap();
    assert!(library::load("m", "../lib", &project)
        .unwrap_err()
        .contains("not UTF-8"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn scan_withoptions_skipped_and_filtered_arguments() {
    let refs = package_references("\\RequirePackageWithOptions{x}\n\\LoadClassWithOptions[opt]{base}\n\\usepackage{a#b}\n\\usepackage{a{b}\n\\usepackage{../x, ok}\n\\usepackage{,dup,,dup}\n\\usepackage\t[opt]\t{spaced}\n\\usepackage  {tabbed}");
    let names: Vec<(String, RefKind)> = refs.iter().map(|r| (r.name.clone(), r.kind)).collect();
    assert_eq!(
        names,
        [
            ("x", RefKind::Package),
            ("base", RefKind::Class),
            ("ok", RefKind::Package),
            ("dup", RefKind::Package),
            ("dup", RefKind::Package),
            ("spaced", RefKind::Package),
            ("tabbed", RefKind::Package)
        ]
        .map(|(n, k)| (n.to_string(), k))
    );
    assert!(unimplemented_packages("packages  are recognised but not implemented").is_empty());
    assert!(unimplemented_packages("packages a are recognised but not implemented!").is_empty());
}

#[test]
fn file_urls_are_served_from_disk() {
    let root = tmp("fileurl");
    std::fs::write(root.join("a.sty"), "style").unwrap();
    std::fs::create_dir_all(root.join("dir")).unwrap();
    std::fs::write(root.join("dir/index.html"), "<a href=\"a.sty\">a</a>").unwrap();
    let fetcher = HttpFetcher::new().unwrap();
    let url = |p: &str| format!("file://{}", root.join(p).display());
    assert_eq!(fetcher.get(&url("a.sty")).unwrap(), b"style");
    assert!(fetcher.get(&url("dir/")).unwrap().starts_with(b"<a href"));
    assert_eq!(
        fetcher.get(&url("missing.sty")).unwrap_err(),
        FetchError::Status(404)
    );
    assert!(
        matches!(fetcher.get("ftp://example.com/x"), Err(FetchError::Transport(m)) if m.contains("only http(s)"))
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn cache_root_empty_override_and_unknown_os() {
    use std::collections::BTreeMap;
    let env = |vars: &[(&str, &str)]| {
        let map: BTreeMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).map(PathBuf::from)
    };
    assert_eq!(
        cache_root_from(&env(&[("HOME", "/h"), (CACHE_ENV, "")]), "macos"),
        Some(PathBuf::from(
            "/h/Library/Application Support/FlashTeX/packages"
        )),
        "an empty override is not an override"
    );
    assert_eq!(
        cache_root_from(&env(&[("HOME", "/h"), (CACHE_ENV, "")]), "linux"),
        Some(PathBuf::from("/h/.cache/flashtex/packages"))
    );
    assert_eq!(
        cache_root_from(&env(&[("HOME", "/h"), ("XDG_CACHE_HOME", "/c")]), "plan9"),
        Some(PathBuf::from("/c/flashtex/packages")),
        "an unknown OS follows the Unix rule"
    );
    assert_eq!(
        cache_root_from(&env(&[("HOME", "/h"), ("XDG_CACHE_HOME", "")]), "haiku"),
        Some(PathBuf::from("/h/.cache/flashtex/packages")),
        "an empty XDG_CACHE_HOME falls back to ~/.cache"
    );
    assert_eq!(cache_root_from(&env(&[]), "plan9"), None);
}
