//! The T1 World (DESIGN.md §15.2, §15.10; spec §11.8): packages from the
//! project, the cache and (with consent) a mirror, the project's lock for
//! package tarballs and fonts, offline by default, `PACKAGE` messages.
//! Nothing here touches the network: the mirror is a `file://` directory.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::*;
use flashtex_display_list::client::Event;
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;
use flashtex_display_list::sha256::{hex, sha256};
use flashtex_typst_host::lock::{Lock, FILE as LOCK};
use flashtex_typst_host::packages::{CAPABILITY, KIND as PACKAGE};

const USES_HELLO: &str = "#import \"@preview/hello:0.1.0\": greet\n#greet(\"world\")\n";

fn write_package(dir: &Path, word: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("typst.toml"),
        "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\nauthors = [\"t\"]\nlicense = \"MIT\"\ndescription = \"t\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("lib.typ"),
        format!("#let greet(x) = [{word}, #x!]\n"),
    )
    .unwrap();
}

/// A `file://` mirror with `preview/hello-0.1.0.tar.gz` saying `word`;
/// returns (mirror dir, tarball SHA-256).
fn mirror(name: &str, word: &str) -> (PathBuf, String) {
    let m = scratch(&format!("mirror-{name}"));
    let src = scratch(&format!("pkgsrc-{name}"));
    write_package(&src, word);
    std::fs::create_dir_all(m.join("preview")).unwrap();
    let tgz = m.join("preview/hello-0.1.0.tar.gz");
    let st = Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("-czf")
        .arg(&tgz)
        .arg("-C")
        .arg(&src)
        .args(["typst.toml", "lib.typ"])
        .status()
        .unwrap();
    assert!(st.success());
    let sha = hex(&sha256(&std::fs::read(&tgz).unwrap()));
    (m, sha)
}

fn host_with(name: &str, cache: &Path, mirror: &Path) -> HostProc {
    let url = format!("file://{}", mirror.display());
    HostProc::start_with(
        name,
        &[
            "--package-cache",
            cache.to_str().unwrap(),
            "--package-mirror",
            &url,
        ],
    )
}

struct Outcome {
    status: String,
    diags: Vec<Json>,
    packages: Vec<Json>,
}

/// Compile and collect the DONE, the DIAGNOSTICs and any PACKAGE frames.
fn compile(c: &mut Raw, id: i64, root: &Path, extra: &str) -> Outcome {
    c.send(kind::COMPILE, &compile_json(id, root, "main.typ", extra));
    let frames = c.until_done();
    let mut o = Outcome {
        status: String::new(),
        diags: vec![],
        packages: vec![],
    };
    for (k, b) in &frames {
        match *k {
            kind::DIAGNOSTIC => o.diags.push(json_of(b)),
            kind::DONE => o.status = json_of(b).str_field("status").unwrap().into(),
            kind::ERROR => o.status = format!("ERROR {}", json_of(b)),
            k if k == PACKAGE => o.packages.push(json_of(b)),
            _ => {}
        }
    }
    // The reference decoder reads PACKAGE (spec §11.8).
    for (e, (k, _)) in events(&frames).iter().zip(&frames) {
        assert!(!matches!(e, Event::Other(..)), "kind {k:#x}");
        if *k == PACKAGE {
            assert!(matches!(e, Event::Package(_)));
        }
    }
    o
}

/// The PACKAGE frames up to and including the one with event `until`.
fn package_events(c: &mut Raw, until: &str) -> Vec<Json> {
    let mut v = vec![];
    while let Some((k, b)) = c.frame() {
        assert_eq!(k, PACKAGE, "only PACKAGE frames between compiles");
        let j = json_of(&b);
        let done = j.str_field("event") == Some(until) || j.str_field("event") == Some("failed");
        v.push(j);
        if done {
            break;
        }
    }
    v
}

fn messages(o: &Outcome) -> Vec<String> {
    o.diags
        .iter()
        .filter_map(|d| d.str_field("message").map(String::from))
        .collect()
}

#[test]
fn hello_offers_packages_and_vendored_packages_need_no_network() {
    let cache = scratch("cache-vendored");
    let (m, _) = mirror("vendored", "Hi");
    let host = host_with("vendored", &cache, &m);
    let root = project("vendored", USES_HELLO);
    write_package(&root.join("typst-packages/preview/hello/0.1.0"), "Vendored");
    let mut c = host.connect();
    let (_, hello) = c.hello_caps(3, 3, &[CAPABILITY]);
    let caps = hello.get("capabilities").unwrap().to_string();
    assert!(caps.contains(CAPABILITY), "{caps}");
    let p = hello.get("typst").unwrap().get("packages").unwrap();
    assert_eq!(p.str_field("lock"), Some(LOCK));
    assert_eq!(p.get("offline").unwrap().as_bool(), Some(false));

    // Offline (the default): the vendored copy is used, nothing fetched.
    let o = compile(&mut c, 1, &root, "");
    assert_eq!(o.status, "ok", "{:?}", messages(&o));
    assert!(!cache.join(".tarballs").exists(), "nothing was fetched");
    // The font list is written after DONE: the next compile follows it.
    compile(&mut c, 2, &root, "");
    // A vendored package is project source: not in the package lock.
    let lock = Lock::read(&root)
        .unwrap()
        .expect("the font list made a lock");
    assert!(lock.packages.is_empty());
}

#[test]
fn offline_is_the_default_and_says_what_to_do() {
    let cache = scratch("cache-offline");
    let (m, _) = mirror("offline", "Hi");
    let host = host_with("offline", &cache, &m);
    let root = project("offline", USES_HELLO);
    let mut c = host.connect();
    c.hello_caps(3, 3, &[CAPABILITY]);
    let o = compile(&mut c, 1, &root, "");
    assert_eq!(o.status, "error");
    let msgs = messages(&o);
    assert!(
        msgs.iter()
            .any(|m| m.contains("@preview/hello:0.1.0") && m.contains("offline")),
        "{msgs:?}"
    );
    // Located at the import.
    assert!(o.diags.iter().any(|d| d.int_field("line") == Some(1)));
    // The client is told the package is needed (it shows its consent sheet).
    let ev = package_events(&mut c, "needed");
    assert_eq!(
        ev.last().unwrap().str_field("package"),
        Some("@preview/hello:0.1.0")
    );
    assert!(!cache.join("preview").exists(), "nothing was fetched");

    // Another namespace is never fetched.
    std::fs::write(root.join("main.typ"), "#import \"@local/thing:1.0.0\": *\n").unwrap();
    let o = compile(&mut c, 2, &root, r#""packages":"online""#);
    assert!(
        messages(&o)
            .iter()
            .any(|m| m.contains("typst-packages/local/thing/1.0.0")),
        "{:?}",
        messages(&o)
    );

    // Bad values are request errors.
    let o = compile(&mut c, 3, &root, r#""packages":"sometimes""#);
    assert!(o.status.starts_with("ERROR"), "{}", o.status);
    let o = compile(&mut c, 4, &root, r#""lock":"maybe""#);
    assert!(o.status.starts_with("ERROR"), "{}", o.status);
}

/// With consent, the package is fetched in the background, recorded in the
/// lock, and the client is told; later fetches and the cached copy are
/// checked against the lock, and a different tarball is refused.
#[test]
fn online_fetches_record_the_lock_and_a_changed_tarball_is_refused() {
    let cache = scratch("cache-online");
    let (m, sha) = mirror("online", "Hello");
    let host = host_with("online", &cache, &m);
    let root = project("online", USES_HELLO);
    let mut c = host.connect();
    c.hello_caps(3, 3, &[CAPABILITY]);

    let online = r#""packages":"online""#;
    let o = compile(&mut c, 1, &root, online);
    let mut events = o.packages.clone();
    if o.status != "ok" {
        // Slower than the compile's wait: "downloading …", then PACKAGE.
        assert!(
            messages(&o).iter().any(|m| m.contains("downloading")),
            "{:?}",
            messages(&o)
        );
    }
    events.extend(package_events(&mut c, "ready"));
    let names: Vec<_> = events.iter().filter_map(|e| e.str_field("event")).collect();
    assert_eq!(names, ["fetching", "ready"], "{events:?}");
    let ready = events.last().unwrap();
    assert_eq!(ready.str_field("sha256"), Some(sha.as_str()));
    let o = compile(&mut c, 2, &root, online);
    assert_eq!(o.status, "ok", "{:?}", messages(&o));
    let lock = Lock::read(&root).unwrap().unwrap();
    assert_eq!(lock.packages["@preview/hello:0.1.0"], sha);
    assert!(cache.join("preview/hello/0.1.0/lib.typ").is_file());
    assert!(cache.join(".tarballs/preview/hello-0.1.0.tar.gz").is_file());

    // Offline now works from the checked cache.
    let o = compile(&mut c, 3, &root, "");
    assert_eq!(o.status, "ok", "{:?}", messages(&o));

    // The mirror's tarball changes (same name and version): a project that
    // locked the old one refuses it, on fetch ...
    let (m2, sha2) = mirror("online", "Evil");
    assert_eq!(m2, m);
    assert_ne!(sha2, sha);
    std::fs::remove_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    let o = compile(&mut c, 4, &root, online);
    let mut events = o.packages.clone();
    if !events
        .iter()
        .any(|e| e.str_field("event") == Some("failed"))
    {
        events.extend(package_events(&mut c, "failed"));
    }
    assert_eq!(events.last().unwrap().str_field("event"), Some("failed"));
    let mismatch = |o: &Outcome| messages(o).iter().any(|m| m.contains("does not match"));
    if !mismatch(&o) {
        // The fetch outlasted compile 4's wait: the next compile reports it.
        assert!(messages(&o).iter().any(|m| m.contains("downloading")));
        let o5 = compile(&mut c, 5, &root, online);
        assert_eq!(o5.status, "error");
        assert!(mismatch(&o5), "{:?}", messages(&o5));
    } else {
        assert_eq!(o.status, "error");
    }
    assert!(
        !cache.join("preview/hello/0.1.0").exists(),
        "a refused tarball is never unpacked"
    );
    assert_eq!(
        Lock::read(&root).unwrap().unwrap().packages["@preview/hello:0.1.0"],
        sha,
        "the lock is never changed to accept it"
    );

    // ... and from a cache that holds it (another project fetched it).
    let other = project("online-other", USES_HELLO);
    let o = compile(&mut c, 6, &other, online);
    if o.status != "ok" {
        package_events(&mut c, "ready");
    }
    assert_eq!(compile(&mut c, 7, &other, online).status, "ok");
    let o = compile(&mut c, 8, &root, "");
    assert_eq!(o.status, "error");
    assert!(
        messages(&o).iter().any(|m| m.contains("does not match")),
        "{:?}",
        messages(&o)
    );
}

#[test]
fn package_files_cannot_leave_the_package() {
    let cache = scratch("cache-escape");
    let (m, _) = mirror("escape", "Hi");
    let host = host_with("escape", &cache, &m);
    let root = project(
        "escape",
        "#import \"@preview/hello:0.1.0\": greet\n#read(\"@preview/hello:0.1.0/x\")\n",
    );
    let pkg = root.join("typst-packages/preview/hello/0.1.0");
    write_package(&pkg, "Hi");
    let secret = root.parent().unwrap().join("pkg-secret.txt");
    std::fs::write(&secret, "TOP-SECRET").unwrap();
    std::os::unix::fs::symlink(&secret, pkg.join("leak.txt")).unwrap();
    std::fs::write(pkg.join("lib.typ"), "#let greet(x) = read(\"leak.txt\")\n").unwrap();
    std::fs::write(
        root.join("main.typ"),
        "#import \"@preview/hello:0.1.0\": greet\n#greet(1)\n",
    )
    .unwrap();
    let mut c = host.connect();
    c.hello(3, 3);
    let o = compile(&mut c, 1, &root, "");
    assert_eq!(o.status, "error");
    let msgs = messages(&o);
    assert!(!msgs.iter().any(|m| m.contains("TOP-SECRET")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("access denied")), "{msgs:?}");
}

fn font_notes(o: &Outcome) -> Vec<String> {
    o.diags
        .iter()
        .filter(|d| d.str_field("kind") == Some("font"))
        .map(|d| d.str_field("message").unwrap().to_string())
        .collect()
}

/// The fonts the text uses are recorded on the first compile; a missing or
/// different font is reported on every compile until fixed or accepted.
#[test]
fn the_font_list_is_recorded_and_checked() {
    let host = HostProc::start("fontlist");
    let root = project(
        "fontlist",
        "#set text(font: \"Libertinus Serif\")\nPlain *bold* _italic_.\n",
    );
    let mut c = host.connect();
    c.hello(3, 3);
    let o = compile(&mut c, 1, &root, "");
    assert_eq!(o.status, "ok");
    assert!(font_notes(&o).is_empty(), "{:?}", font_notes(&o));
    // Recorded after DONE (never before the page): visible once the next
    // compile has started.
    assert!(font_notes(&compile(&mut c, 2, &root, "")).is_empty());
    let lock = Lock::read(&root).unwrap().unwrap();
    let keys: Vec<&String> = lock.fonts.keys().collect();
    assert!(
        keys.iter()
            .any(|k| k.starts_with("Libertinus Serif|normal|400|"))
            && keys
                .iter()
                .any(|k| k.starts_with("Libertinus Serif|normal|700|"))
            && keys
                .iter()
                .any(|k| k.starts_with("Libertinus Serif|italic|400|")),
        "{keys:?}"
    );
    for (sha, file) in lock.fonts.values() {
        let data = std::fs::read(font_dir().join(file)).unwrap();
        assert_eq!(*sha, hex(&sha256(&data)));
    }

    // A changed file (here: the lock says another hash) and a missing font.
    let mut edited = lock.clone();
    let k = keys
        .iter()
        .find(|k| k.contains("|700|"))
        .unwrap()
        .to_string();
    edited.fonts.get_mut(&k).unwrap().0 = "0".repeat(64);
    edited.fonts.insert(
        "Nonexistent Sans|normal|400|1000".into(),
        ("1".repeat(64), "NS.otf".into()),
    );
    std::fs::write(root.join(LOCK), edited.to_text()).unwrap();
    // Checked after this compile's DONE; reported from the next one on.
    compile(&mut c, 3, &root, "");
    for id in [4, 5] {
        let o = compile(&mut c, id, &root, "");
        assert_eq!(o.status, "ok");
        let notes = font_notes(&o);
        assert_eq!(notes.len(), 2, "every compile until fixed: {notes:?}");
        assert!(notes
            .iter()
            .any(|n| n.contains("Nonexistent Sans") && n.contains("not installed")));
        assert!(notes.iter().any(|n| n.contains("is not the file")));
        assert!(o
            .diags
            .iter()
            .filter(|d| d.str_field("kind") == Some("font"))
            .all(|d| d.str_field("severity") == Some("warning")
                && d.str_field("file").unwrap().ends_with(LOCK)));
    }

    // `"lock": "update"` accepts the fonts as the document uses them now.
    compile(&mut c, 6, &root, r#""lock":"update""#);
    let o = compile(&mut c, 7, &root, "");
    assert!(font_notes(&o).is_empty(), "{:?}", font_notes(&o));
    assert_eq!(Lock::read(&root).unwrap().unwrap(), lock);
    // `"lock": "off"` neither reads nor writes it.
    std::fs::write(root.join(LOCK), edited.to_text()).unwrap();
    for id in [8, 9] {
        let o = compile(&mut c, id, &root, r#""lock":"off""#);
        assert!(font_notes(&o).is_empty());
    }
    assert_eq!(Lock::read(&root).unwrap().unwrap(), edited);
}

/// The lock is never read or written through a symlink, and an unreadable
/// lock is reported, never overwritten.
#[test]
fn the_lock_is_confined() {
    let host = HostProc::start("lockconf");
    let root = project("lockconf", "Text.\n");
    let outside = root.parent().unwrap().join("outside.lock");
    std::fs::write(&outside, "keep").unwrap();
    std::os::unix::fs::symlink(&outside, root.join(LOCK)).unwrap();
    let mut c = host.connect();
    c.hello(3, 3);
    let o = compile(&mut c, 1, &root, "");
    assert_eq!(o.status, "ok");
    assert!(
        o.diags.iter().any(|d| d.str_field("kind") == Some("lock")),
        "{:?}",
        messages(&o)
    );
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "keep");

    // Compile 1's lock work runs after its DONE; a compile with the lock off
    // (which does none) returns only once that has finished. Without it the
    // edit below would race the host's write (an editor takes no flock).
    compile(&mut c, 2, &root, r#""lock":"off""#);
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "keep");
    std::fs::remove_file(root.join(LOCK)).unwrap();
    std::fs::write(root.join(LOCK), "version = 99\n").unwrap();
    let o = compile(&mut c, 3, &root, "");
    assert!(
        messages(&o).iter().any(|m| m.contains("newer")),
        "{:?}",
        messages(&o)
    );
    assert_eq!(
        std::fs::read_to_string(root.join(LOCK)).unwrap(),
        "version = 99\n"
    );
}

/// A cached package's unpacked tree is checked against its (lock-checked)
/// tarball once per host process: an edited, deleted or added file is
/// replaced by a fresh unpack of the tarball before anything is read.
#[test]
fn a_tampered_cached_tree_is_restored_from_its_tarball() {
    let cache = scratch("cache-tamper");
    let (m, sha) = mirror("tamper", "Hello");
    let root = project("tamper", USES_HELLO);
    {
        let host = host_with("tamper1", &cache, &m);
        let mut c = host.connect();
        c.hello_caps(3, 3, &[CAPABILITY]);
        let o = compile(&mut c, 1, &root, r#""packages":"online""#);
        if o.status != "ok" {
            package_events(&mut c, "ready");
            assert_eq!(compile(&mut c, 2, &root, "").status, "ok");
        }
    }
    let pkg = cache.join("preview/hello/0.1.0");
    let good = std::fs::read_to_string(pkg.join("lib.typ")).unwrap();
    assert!(good.contains("Hello"));
    assert_eq!(
        Lock::read(&root).unwrap().unwrap().packages["@preview/hello:0.1.0"],
        sha
    );

    for (i, tamper) in ["edit", "delete", "add"].iter().enumerate() {
        match *tamper {
            "edit" => std::fs::write(pkg.join("lib.typ"), "#let greet(x) = [EVIL]\n").unwrap(),
            "delete" => std::fs::remove_file(pkg.join("typst.toml")).unwrap(),
            _ => std::fs::write(pkg.join("extra.typ"), "x").unwrap(),
        }
        // A new host process (the check is once per process and package).
        let host = host_with(&format!("tamper-{tamper}"), &cache, &m);
        let mut c = host.connect();
        c.hello(3, 3);
        let o = compile(&mut c, 10 + i as i64, &root, "");
        assert_eq!(o.status, "ok", "{tamper}: {:?}", messages(&o));
        assert_eq!(
            std::fs::read_to_string(pkg.join("lib.typ")).unwrap(),
            good,
            "{tamper}"
        );
        assert!(pkg.join("typst.toml").is_file(), "{tamper}");
        assert!(!pkg.join("extra.typ").exists(), "{tamper}");
    }
}

/// Fetch `@preview/hello:0.1.0` online until the compile is ok or the fetch
/// failed; returns the last outcome and every PACKAGE event seen.
fn fetch_until_settled(c: &mut Raw, root: &Path, id0: i64, extra: &str) -> (Outcome, Vec<Json>) {
    let o = compile(c, id0, root, extra);
    let mut events = o.packages.clone();
    if o.status == "ok" {
        return (o, events);
    }
    if !events
        .iter()
        .any(|e| matches!(e.str_field("event"), Some("ready" | "failed")))
    {
        events.extend(package_events(c, "ready"));
    }
    let o2 = compile(c, id0 + 1, root, extra);
    events.extend(o2.packages.clone());
    (o2, events)
}

/// `--offline` wins over a client that says "packages": "online".
#[test]
fn the_offline_flag_overrides_the_client() {
    let cache = scratch("cache-offline-flag");
    let (m, _) = mirror("offline-flag", "Hi");
    let url = format!("file://{}", m.display());
    let host = HostProc::start_with(
        "offline-flag",
        &[
            "--package-cache",
            cache.to_str().unwrap(),
            "--package-mirror",
            &url,
            "--offline",
        ],
    );
    let root = project("offline-flag", USES_HELLO);
    let mut c = host.connect();
    let (_, hello) = c.hello_caps(3, 3, &[CAPABILITY]);
    let p = hello.get("typst").unwrap().get("packages").unwrap();
    assert_eq!(p.get("offline").unwrap().as_bool(), Some(true));
    let o = compile(&mut c, 1, &root, r#""packages":"online""#);
    assert_eq!(o.status, "error");
    assert!(
        messages(&o).iter().any(|m| m.contains("offline")),
        "{:?}",
        messages(&o)
    );
    assert_eq!(
        package_events(&mut c, "needed")
            .last()
            .unwrap()
            .str_field("event"),
        Some("needed")
    );
    assert!(!cache.join(".tarballs").exists(), "nothing was fetched");
}

/// A tarball with an entry outside the package is refused before anything
/// is unpacked: a located error, a `failed` event, no lock entry, no tree,
/// and the tarball is not kept.
#[test]
fn a_hostile_package_tarball_is_refused() {
    use std::io::Write;
    let cache = scratch("cache-hostile");
    let m = scratch("mirror-hostile");
    std::fs::create_dir_all(m.join("preview")).unwrap();
    // A raw tar whose second entry climbs out of the package.
    let mut raw = Vec::new();
    for (name, body) in [
        (
            "typst.toml",
            &b"[package]\nname = \"hello\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n"[..],
        ),
        ("../../../escaped.typ", &b"#let greet(x) = [owned]\n"[..]),
    ] {
        let mut h = tar::Header::new_ustar();
        h.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        h.set_entry_type(tar::EntryType::Regular);
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        raw.extend_from_slice(h.as_bytes());
        raw.extend_from_slice(body);
        raw.resize(raw.len().div_ceil(512) * 512, 0);
    }
    raw.extend_from_slice(&[0; 1024]);
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&raw).unwrap();
    std::fs::write(m.join("preview/hello-0.1.0.tar.gz"), gz.finish().unwrap()).unwrap();

    let host = host_with("hostile", &cache, &m);
    let root = project("hostile", USES_HELLO);
    let mut c = host.connect();
    c.hello_caps(3, 3, &[CAPABILITY]);
    let (o, events) = fetch_until_settled(&mut c, &root, 1, r#""packages":"online""#);
    assert_eq!(o.status, "error");
    assert!(
        messages(&o)
            .iter()
            .any(|m| m.contains("is refused") && m.contains("outside the package")),
        "{:?}",
        messages(&o)
    );
    assert!(events
        .iter()
        .any(|e| e.str_field("event") == Some("failed")));
    assert!(!cache.join("preview/hello").exists() || !cache.join("preview/hello/0.1.0").exists());
    assert!(!cache.join(".tarballs/preview/hello-0.1.0.tar.gz").exists());
    for d in [&cache, cache.parent().unwrap()] {
        assert!(!d.join("escaped.typ").exists());
    }
    compile(&mut c, 9, &root, r#""lock":"off""#);
    let lock = Lock::read(&root).unwrap();
    assert!(
        lock.is_none_or(|l| l.packages.is_empty()),
        "nothing recorded"
    );
}

/// "lock": "off" records nothing, but a hash the lock knows is still
/// checked: a changed tarball is refused.
#[test]
fn lock_off_still_checks_known_hashes() {
    let cache = scratch("cache-lockoff");
    let (m, sha) = mirror("lockoff", "Hello");
    let host = host_with("lockoff", &cache, &m);
    let root = project("lockoff", USES_HELLO);
    let mut c = host.connect();
    c.hello_caps(3, 3, &[CAPABILITY]);
    let (o, _) = fetch_until_settled(&mut c, &root, 1, r#""packages":"online""#);
    assert_eq!(o.status, "ok", "{:?}", messages(&o));
    assert_eq!(
        Lock::read(&root).unwrap().unwrap().packages["@preview/hello:0.1.0"],
        sha
    );

    // Another project, lock off: records nothing.
    let other = project("lockoff-other", USES_HELLO);
    let o = compile(&mut c, 5, &other, r#""lock":"off""#);
    assert_eq!(o.status, "ok", "{:?}", messages(&o));
    compile(&mut c, 6, &other, r#""lock":"off""#);
    assert!(Lock::read(&other).unwrap().is_none());

    // The mirror's tarball changes and the cache is gone: with the lock
    // off, the project that knows the old hash still refuses the new one.
    let (_, sha2) = mirror("lockoff", "Evil");
    assert_ne!(sha2, sha);
    std::fs::remove_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    let (o, _) = fetch_until_settled(&mut c, &root, 10, r#""packages":"online","lock":"off""#);
    assert_eq!(o.status, "error");
    assert!(
        messages(&o).iter().any(|m| m.contains("does not match")),
        "{:?}",
        messages(&o)
    );
}

/// A read-only project: the lock is kept in memory (one note says so), the
/// compile is fine, nothing is written.
#[test]
fn a_read_only_project_keeps_its_lock_in_memory() {
    use std::os::unix::fs::PermissionsExt;
    let host = HostProc::start("readonly");
    let root = project(
        "readonly",
        "#set text(font: \"Libertinus Serif\")\nRead-only.\n",
    );
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut c = host.connect();
    c.hello(3, 3);
    let mut lock_notes = vec![];
    for id in 1..=4 {
        let o = compile(&mut c, id, &root, "");
        assert_eq!(o.status, "ok", "{:?}", messages(&o));
        assert!(font_notes(&o).is_empty(), "{:?}", font_notes(&o));
        lock_notes.extend(
            o.diags
                .iter()
                .filter(|d| d.str_field("kind") == Some("lock"))
                .map(|d| d.str_field("message").unwrap().to_string()),
        );
    }
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(lock_notes.len(), 1, "one note: {lock_notes:?}");
    assert!(lock_notes[0].contains("read-only") && lock_notes[0].contains("memory"));
    assert!(!root.join(LOCK).exists());
    let left: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left.len(), 1, "only main.typ: {left:?}");
}
