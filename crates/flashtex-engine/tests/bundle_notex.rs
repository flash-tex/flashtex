//! No TeX Live: the engine and the host compile from a bundle alone
//! (DESIGN.md 4.4, lane NOTEX-WIRING).
//!
//! 1. Over the installed TeX Live alone (the trees outside TEXMFROOT --
//!    TEXMFHOME, TEXMFLOCAL, a user's TEXMFVAR and TEXMFCONFIG -- empty,
//!    since what they hold is not TeX Live's to pack), the engine builds
//!    its format and compiles a hello-world and an amsmath document,
//!    recording what it reads (the documents' `FLASHTEX_READ_SET`, the
//!    format's cache manifest).
//! 2. `flashtex-dist bundle-pack` packs exactly those files into a TTBv1
//!    bundle, with the installation's own `TEXMFROOT/texmf.cnf` and its
//!    installed `pdflatex.fmt` added to the read list, which a bundle must
//!    leave out (the reader used to reject the first; the second is 3.6 MB
//!    the engine never loads).
//! 3. With TeX Live hidden -- an empty environment (`env -i`), an empty HOME,
//!    a PATH of the system directories only, no `FLASHTEX_POOL` (the pool
//!    compiled into the program), the resolver forced to the bundle, served
//!    over `file://`, and on macOS `sandbox-exec` denying every read of the
//!    TeX Live tree -- the engine builds its format from the bundle and
//!    compiles both documents again: the PDFs must be byte-identical to
//!    step 1's.
//! 4. `flashtex-host` in the same environment, configured by a CRLF
//!    `flashtex-bundle.lock`, fetches nothing without the consent flag
//!    (`FLASHTEX_BUNDLE_ALLOW_FETCH`); with it, says in HELLO that it reads
//!    the bundle (`texmf.resolver`, `texmf.bundle.digest`) and compiles the
//!    amsmath document over its socket.
//!
//! Packing needs a TeX Live, so this skips without one (e.g. CI's light jobs).
#![cfg(all(feature = "distribution", unix))]

mod common;

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::json::Json;
use flashtex_engine::bundle::{gz, ttb};
use flashtex_engine::resolver::{discover_texlive, KpathseaResolver};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HELLO: &str = "\\documentclass{article}\n\\begin{document}\nHello, world.\n\\end{document}\n";
const AMSMATH: &str = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n\
\\begin{align}\na^2+b^2 &= c^2 \\\\\n\\int_0^1 x\\,dx &= \\tfrac12\n\\end{align}\n\
\\end{document}\n";

fn engine_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_flashtex-initex"))
}

/// The trees kpathsea searches outside TEXMFROOT, each set to `empty` for
/// the TeX Live runs and the pack: a file found there (a `cmr10.tfm` in
/// the runner's `~/Library/texmf`, say) shadows TeX Live's own, is not
/// TeX Live's to pack, and the bundle then lacks it.
const USER_TREES: [&str; 4] = ["TEXMFHOME", "TEXMFVAR", "TEXMFCONFIG", "TEXMFLOCAL"];

fn only_texlive(c: &mut Command, empty: &Path) {
    for v in USER_TREES {
        c.env(v, empty);
    }
}

/// The TeX Live runs: format cache in `fmt`, read set appended to `read_set`.
fn texlive_run(bin: &Path, dir: &Path, fmt: &Path, read_set: &Path, empty: &Path) -> Vec<u8> {
    let mut c = Command::new(bin);
    only_texlive(&mut c, empty);
    let st = c
        .args(["-fmt=pdflatex", "-interaction=nonstopmode", "main.tex"])
        .current_dir(dir)
        .env("FLASHTEX_FORMAT_CACHE_DIR", fmt)
        .env("FLASHTEX_READ_SET", read_set)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_FORMATS")
        .env_remove("FLASHTEX_RESOLVER")
        .env_remove("FLASHTEX_BUNDLE_DIGEST")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "TeX Live run in {}", dir.display());
    std::fs::read(dir.join("main.pdf")).unwrap()
}

/// TeX Live hidden: `env -i` with only `env`, under the sandbox if any.
fn hidden(sandbox: Option<&Path>, program: &Path, env: &[(&str, String)]) -> Command {
    let mut c = match sandbox {
        Some(profile) => {
            let mut c = Command::new("/usr/bin/sandbox-exec");
            c.arg("-f").arg(profile).arg(program);
            c
        }
        None => Command::new(program),
    };
    c.env_clear();
    for (k, v) in env {
        c.env(k, v);
    }
    c
}

/// Every `manifest` file under `dir` (the format cache's slots).
fn manifests(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            manifests(&p, out);
        } else if p.file_name().is_some_and(|n| n == "manifest") {
            out.push(p);
        }
    }
}

/// The macOS sandbox profile that denies reads of TeX Live, if
/// `sandbox-exec` works here (it does not inside another sandbox) and the
/// profile really hides `probe`.
fn sandbox_profile(d: &Path, deny: &[PathBuf], probe: &Path) -> Option<PathBuf> {
    let exe = Path::new("/usr/bin/sandbox-exec");
    if !exe.is_file() {
        return None;
    }
    let mut s = String::from("(version 1)\n(allow default)\n(deny file-read*");
    for p in deny {
        s += &format!(" (subpath \"{}\")", p.display());
    }
    s += ")\n";
    let profile = d.join("notex.sb");
    std::fs::write(&profile, s).unwrap();
    let ok = Command::new(exe)
        .arg("-f")
        .arg(&profile)
        .arg("/usr/bin/true")
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        eprintln!("sandbox-exec does not run here; TeX Live is hidden by the environment alone");
        return None;
    }
    let hidden = Command::new(exe)
        .arg("-f")
        .arg(&profile)
        .args(["/bin/cat"])
        .arg(probe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| !s.success());
    assert!(
        hidden,
        "the sandbox profile does not hide {}",
        probe.display()
    );
    Some(profile)
}

#[test]
fn compiles_from_a_bundle_with_no_texlive_visible() {
    let Some(tl) = discover_texlive() else {
        common::no_texlive();
        return;
    };
    let mut kp = KpathseaResolver::for_texlive(&tl.bin, "pdflatex", "");
    let root = PathBuf::from(kp.var_value("TEXMFROOT").expect("TEXMFROOT"));
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let d = common::fresh_dir("ftx-notex");
    let d = std::fs::canonicalize(&d).unwrap();
    for s in [
        "bin",
        "tl/hello",
        "tl/amsmath",
        "nt/hello",
        "nt/amsmath",
        "home",
        "tmp",
        "empty",
    ] {
        std::fs::create_dir_all(d.join(s)).unwrap();
    }
    for side in ["tl", "nt"] {
        std::fs::write(d.join(side).join("hello/main.tex"), HELLO).unwrap();
        std::fs::write(d.join(side).join("amsmath/main.tex"), AMSMATH).unwrap();
    }
    // argv[0] `pdftex`, as the harnesses run it; nothing beside it.
    let bin = d.join("bin").join("pdftex");
    flashtex_engine::os::link_executable(engine_bin(), &bin).unwrap();

    // 1. Over TeX Live, recording what is read.
    let fmt_tl = d.join("fmt-tl");
    let reads_hello = d.join("reads-hello.txt");
    let reads_ams = d.join("reads-amsmath.txt");
    let empty = d.join("empty");
    let pdf_hello = texlive_run(&bin, &d.join("tl/hello"), &fmt_tl, &reads_hello, &empty);
    let pdf_ams = texlive_run(&bin, &d.join("tl/amsmath"), &fmt_tl, &reads_ams, &empty);
    let mut ms = vec![];
    manifests(&fmt_tl, &mut ms);
    assert_eq!(ms.len(), 1, "one format built: {ms:?}");

    // 2. Pack, with the two files a bundle must leave out in the read list.
    let extra = d.join("extra.txt");
    let top_cnf = root.join("texmf.cnf");
    let installed_fmt = root.join("texmf-var/web2c/pdftex/pdflatex.fmt");
    std::fs::write(
        &extra,
        format!("{}\n{}\n", top_cnf.display(), installed_fmt.display()),
    )
    .unwrap();
    let ttb = d.join("b.ttb");
    let mut pack = Command::new(env!("CARGO_BIN_EXE_flashtex-dist"));
    only_texlive(&mut pack, &empty);
    let o = pack
        .arg("bundle-pack")
        .arg("--out")
        .arg(&ttb)
        .arg("--read")
        .arg(&reads_hello)
        .arg("--read")
        .arg(&reads_ams)
        .arg("--read")
        .arg(&extra)
        .arg("--manifest")
        .arg(&ms[0])
        .arg("--core-manifest")
        .arg(&ms[0])
        .arg("--core-read")
        .arg(&reads_hello)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        o.status.success(),
        "bundle-pack: {out}{}",
        String::from_utf8_lossy(&o.stderr)
    );
    eprintln!("{out}");
    let digest = out
        .lines()
        .find_map(|l| l.strip_prefix("digest "))
        .expect("bundle-pack prints the digest")
        .to_string();
    // Nothing outside TeX Live but this test's own files (the format
    // cache's format and pool).
    for l in out.lines() {
        if let Some(p) = l.strip_prefix("outside TeX Live: ") {
            assert!(Path::new(p).starts_with(&d), "{out}");
        }
    }
    if top_cnf.is_file() {
        assert!(out.contains("left out texmf.cnf: "), "{out}");
    }
    if installed_fmt.is_file() {
        assert!(
            out.contains("left out texmf-var/web2c/pdftex/pdflatex.fmt: "),
            "{out}"
        );
    }
    let bytes = std::fs::read(&ttb).unwrap();
    let h = ttb::Header::parse(&bytes).unwrap();
    let ix = gz::gunzip(
        &bytes[h.index_start as usize..][..h.index_gzip_len as usize],
        0,
    )
    .unwrap();
    let ix = ttb::Index::parse(std::str::from_utf8(&ix).unwrap()).unwrap();
    for e in &ix.files {
        assert!(!e.path.ends_with(".fmt"), "{} in the bundle", e.path);
        assert_ne!(e.path, "texmf.cnf");
    }
    assert!(ix
        .files
        .iter()
        .any(|e| e.path == "texmf-dist/web2c/texmf.cnf"));

    // 3. TeX Live hidden.
    let mut deny = vec![root.clone()];
    if let Some(parent) = root.parent().filter(|p| p.ends_with("texlive")) {
        deny = vec![parent.to_path_buf()];
    }
    for p in ["/Library/TeX", "/usr/local/texlive", "/opt/texlive"] {
        if Path::new(p).exists() {
            deny.push(std::fs::canonicalize(p).unwrap());
        }
    }
    let tl_bin = std::fs::canonicalize(&tl.bin).unwrap_or(tl.bin.clone());
    if !deny.iter().any(|p| tl_bin.starts_with(p)) {
        deny.push(tl_bin);
    }
    let sandbox = sandbox_profile(&d, &deny, &root.join("texmf-dist/web2c/texmf.cnf"));
    let base_env = |extra: &[(&'static str, String)]| {
        let mut v: Vec<(&'static str, String)> = vec![
            ("HOME", d.join("home").display().to_string()),
            ("PATH", "/usr/bin:/bin:/usr/sbin:/sbin".into()),
            ("TMPDIR", d.join("tmp").display().to_string()),
            ("SOURCE_DATE_EPOCH", "0".into()),
            ("FORCE_SOURCE_DATE", "1".into()),
            ("FLASHTEX_RESOLVER", "bundle".into()),
        ];
        v.extend(extra.iter().cloned());
        v
    };
    let env = base_env(&[
        ("FLASHTEX_BUNDLE_URL", format!("file://{}", ttb.display())),
        ("FLASHTEX_BUNDLE_DIGEST", digest.clone()),
    ]);
    for (doc, want) in [("hello", &pdf_hello), ("amsmath", &pdf_ams)] {
        let dir = d.join("nt").join(doc);
        let o = hidden(sandbox.as_deref(), &bin, &env)
            .args(["-fmt=pdflatex", "-interaction=nonstopmode", "main.tex"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let log = std::fs::read_to_string(dir.join("main.log")).unwrap_or_default();
        assert!(
            o.status.success(),
            "{doc} from the bundle: {}{}\n{log}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let pdf = std::fs::read(dir.join("main.pdf")).unwrap();
        assert!(
            &pdf == want,
            "{doc}: the bundle's PDF differs from TeX Live's ({} vs {} bytes)",
            pdf.len(),
            want.len()
        );
    }
    // The caches are the empty HOME's defaults, the pool the compiled-in one.
    let home = d.join("home");
    let caches = if cfg!(target_os = "macos") {
        home.join("Library/Caches/FlashTeX")
    } else {
        home.join(".cache/flashtex")
    };
    assert!(caches
        .join("bundles")
        .join(&digest)
        .join("index.gz")
        .is_file());
    assert!(std::fs::read_dir(caches.join("formats/pool"))
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("pdftex-")));

    // 4. The host, configured by a lock file instead of the environment
    // (CRLF line ends, as an editor on Windows writes them), each run with
    // a bundle cache of its own, so what it fetches is visible.
    let lock = d.join("flashtex-bundle.lock");
    std::fs::write(
        &lock,
        format!("# test bundle\r\nurl = \"b.ttb\"\r\ndigest = \"{digest}\"\r\n"),
    )
    .unwrap();
    let sock = {
        let s = d.join("h.sock");
        if s.as_os_str().len() < 100 {
            s
        } else {
            PathBuf::from(format!("/tmp/ftx-notex-{}.sock", std::process::id()))
        }
    };
    // Start a host; its stdout up to "listening", and the start-up line's JSON.
    let start_host = |extra: &[(&'static str, String)]| {
        let mut env = base_env(&[("FLASHTEX_BUNDLE_LOCK", lock.display().to_string())]);
        env.extend(extra.iter().cloned());
        let mut child = hidden(
            sandbox.as_deref(),
            Path::new(env!("CARGO_BIN_EXE_flashtex-host")),
            &env,
        )
        .arg("--socket")
        .arg(&sock)
        .args(["--once", "--no-warm", "--accept-timeout", "120"])
        .current_dir(d.join("tmp"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut lines = vec![];
        loop {
            let mut l = String::new();
            if out.read_line(&mut l).unwrap() == 0 {
                panic!("the host exited: {lines:?}");
            }
            if l.contains("listening") {
                break;
            }
            lines.push(l);
        }
        let prepared = lines
            .iter()
            .filter_map(|l| l.strip_prefix("flashtex-host: "))
            .filter_map(|j| Json::parse(j.trim()).ok())
            .find(|j| j.get("texmf").is_some() || j.get("formats").is_some())
            .expect("the start-up line");
        (child, lines, prepared)
    };

    // 4a. Without the user's consent for this bundle from this source (here:
    // consent for the same digest from another server, as a lock rewritten
    // after the app read it would give), a lock file's bundle is not
    // fetched: the host is offline and, with an empty cache, has no bundle.
    let bundle_url = d.join("b.ttb").display().to_string();
    let closed_cache = d.join("bcache-closed");
    let (mut child, lines, prepared) = start_host(&[
        (
            "FLASHTEX_BUNDLE_CACHE_DIR",
            closed_cache.display().to_string(),
        ),
        (
            "FLASHTEX_BUNDLE_ALLOW_FETCH",
            format!("{digest}@https://elsewhere.example/b.ttb"),
        ),
    ]);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&sock);
    let b = prepared.get("bundle").expect("texmf.bundle");
    assert_eq!(b.str_field("digest"), Some(digest.as_str()), "{prepared}");
    assert_eq!(
        b.get("offline").and_then(|a| a.as_bool()),
        Some(true),
        "{prepared}"
    );
    assert_eq!(
        b.get("active").and_then(|a| a.as_bool()),
        Some(false),
        "{prepared}"
    );
    assert!(
        !closed_cache.join(&digest).join("index.gz").exists(),
        "fetched without consent"
    );
    assert!(
        !lines.iter().any(|l| l.contains("bundle_progress")),
        "{lines:?}"
    );

    // 4b. With consent for this bundle from this source (`<digest>@<url>`, as
    // the app passes it), the host fetches it, says so in HELLO and
    // compiles over its socket.
    let open_cache = d.join("bcache-open");
    let (mut child, lines, _) = start_host(&[
        (
            "FLASHTEX_BUNDLE_CACHE_DIR",
            open_cache.display().to_string(),
        ),
        (
            "FLASHTEX_BUNDLE_ALLOW_FETCH",
            format!("{digest}@{bundle_url}"),
        ),
    ]);
    assert!(open_cache.join(&digest).join("index.gz").is_file());
    assert!(
        lines
            .iter()
            .any(|l| l.contains(r#""bundle_progress":{"what":"core""#)),
        "{lines:?}"
    );

    let mut c = Client::connect(&sock).unwrap();
    let texmf = c.hello.get("texmf").expect("HELLO.texmf").clone();
    assert_eq!(
        texmf.str_field("resolver"),
        Some(format!("bundle {digest}").as_str()),
        "{texmf}"
    );
    let b = texmf.get("bundle").expect("HELLO.texmf.bundle");
    assert_eq!(b.str_field("digest"), Some(digest.as_str()), "{texmf}");
    assert_eq!(
        b.get("active").and_then(|a| a.as_bool()),
        Some(true),
        "{texmf}"
    );
    assert_eq!(
        b.str_field("origin"),
        Some(lock.display().to_string().as_str())
    );
    if sandbox.is_some() {
        assert!(matches!(texmf.get("texlive"), Some(Json::Null)), "{texmf}");
    }
    let formats = texmf.get("formats").and_then(|f| f.as_array()).unwrap();
    assert_eq!(formats[0].str_field("status"), Some("ready"), "{texmf}");
    let req = CompileRequest::new(1, d.join("nt/amsmath").to_str().unwrap(), "main.tex");
    c.compile(&req).unwrap();
    let done = loop {
        match c.next_event().unwrap().expect("host closed the connection") {
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    };
    assert_eq!(done.str_field("status"), Some("ok"), "{done}");
    assert_eq!(done.int_field("pages"), Some(1), "{done}");
    drop(c);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_dir_all(&d);
}
