//! `flashtex-dist`: the distribution side of the engine (DESIGN.md 4.4), as
//! commands for the host, for measurements and for making bundles.
//!
//! ```text
//! flashtex-dist texlive [--compare]      which TeX Live, and kpathsea's view of it
//! flashtex-dist format NAME [--hits N]   build or validate the cached NAME.fmt
//! flashtex-dist bundle-pack --out F.ttb [--read LIST]... [--manifest M]...
//!               [--core-read LIST]... [--tlpdb PATH] [--whole-packages]
//! flashtex-dist bundle-measure --bundle F.ttb --doc main.tex [--work DIR]
//! flashtex-dist index-estimate [--tlpdb PATH]
//! ```
//!
//! `--compare` runs the TeX Live's own `kpsewhich` as the oracle; nothing
//! else here runs a TeX Live program.

use flashtex_engine::bundle::{self, serve::FixtureServer, ttb, BundleResolver, BundleSpec};
use flashtex_engine::formats::{self, hex, FormatCache};
use flashtex_engine::resolver::{self, FileResolver, Format, KpathseaResolver};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn die(m: impl std::fmt::Display) -> ! {
    eprintln!("flashtex-dist: {m}");
    std::process::exit(1)
}

fn opt_all(args: &[String], name: &str) -> Vec<String> {
    let mut v = vec![];
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            if let Some(x) = args.get(i + 1) {
                v.push(x.clone());
            }
            i += 1;
        }
        i += 1;
    }
    v
}

fn opt(args: &[String], name: &str) -> Option<String> {
    opt_all(args, name).pop()
}

fn engine_exe() -> PathBuf {
    std::env::current_exe()
        .map(|p| p.with_file_name("flashtex-initex"))
        .unwrap_or_else(|_| die("current_exe"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("texlive") => texlive(&args[1..]),
        Some("format") => format(&args[1..]),
        Some("bundle-pack") => bundle_pack(&args[1..]),
        Some("bundle-measure") => bundle_measure(&args[1..]),
        Some("index-estimate") => index_estimate(&args[1..]),
        _ => {
            eprintln!(
                "{}",
                include_str!("flashtex-dist.rs")
                    .lines()
                    .skip(3)
                    .take(9)
                    .map(|l| l.trim_start_matches("//! "))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            std::process::exit(2)
        }
    }
}

const VARS: &[&str] = &[
    "SELFAUTOLOC",
    "SELFAUTOPARENT",
    "TEXMFROOT",
    "TEXMFDIST",
    "TEXMFLOCAL",
    "TEXMFSYSVAR",
    "TEXMFSYSCONFIG",
    "TEXMFVAR",
    "TEXMFCONFIG",
    "TEXMFHOME",
    "TEXMFCNF",
    "TEXMF",
    "TEXINPUTS",
    "TEXFORMATS",
];

fn texlive(args: &[String]) {
    let compare = args.iter().any(|a| a == "--compare");
    for (d, how) in resolver::texlive_candidates() {
        let ok = d.join("kpsewhich").is_file();
        println!("  {} {:<60} {how}", if ok { "*" } else { " " }, d.display());
        if ok {
            break;
        }
    }
    let Some(t) = resolver::discover_texlive() else {
        println!("no TeX Live found");
        std::process::exit(3)
    };
    println!("TeX Live: {}", t.describe());
    let t0 = Instant::now();
    let mut r = KpathseaResolver::for_texlive(&t.bin, "pdflatex", "");
    let _ = r.find("latex.ltx", Format::Tex);
    println!(
        "kpathsea set-up + first lookup: {:.1} ms",
        t0.elapsed().as_secs_f64() * 1e3
    );
    let mut same = 0;
    for v in VARS {
        let ours = r.var_value(v).unwrap_or_default();
        let mut line = format!("{v:<16} {ours}");
        if compare {
            let o = Command::new(t.bin.join("kpsewhich"))
                .arg("-progname=pdflatex")
                .arg(format!("-var-value={v}"))
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
                .unwrap_or_default();
            if o == ours {
                same += 1;
            } else {
                line += &format!("\n{:<16} kpsewhich: {o}", "");
            }
        }
        println!("{line}");
    }
    if compare {
        let names: &[(&str, Format, &str)] = &[
            ("latex.ltx", Format::Tex, "tex"),
            ("article.cls", Format::Tex, "tex"),
            ("hyphen.cfg", Format::Tex, "tex"),
            ("language.dat", Format::Tex, "tex"),
            ("cmr10", Format::Tfm, "tfm"),
            ("pdftex.map", Format::Map, "map"),
            ("cp227.tcx", Format::Web2c, "web2c files"),
        ];
        for (n, f, kf) in names {
            let ours = r
                .find(n, *f)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let o = Command::new(t.bin.join("kpsewhich"))
                .args(["-progname=pdflatex", &format!("-format={kf}"), n])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
                .unwrap_or_default();
            if o == ours {
                same += 1;
            }
            println!(
                "{:<16} {ours}{}",
                n,
                if o == ours {
                    ""
                } else {
                    "   (kpsewhich differs!)"
                }
            );
        }
        let ours: Vec<String> = r
            .find_all("fmtutil.cnf", Format::Cnf)
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        let o = Command::new(t.bin.join("kpsewhich"))
            .args(["-all", "fmtutil.cnf"])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if o == ours {
            same += 1;
        }
        println!(
            "-all fmtutil.cnf {ours:?}{}",
            if o == ours {
                ""
            } else {
                "   (kpsewhich differs!)"
            }
        );
        println!(
            "identical to kpsewhich: {same}/{}",
            VARS.len() + names.len() + 1
        );
    }
}

fn format(args: &[String]) {
    let name = args
        .first()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| die("format NAME"));
    let hits: usize = opt(args, "--hits")
        .and_then(|h| h.parse().ok())
        .unwrap_or(0);
    let dir = formats::cache_dir().unwrap_or_else(|| die("no cache directory"));
    let t0 = Instant::now();
    let mut r = resolver::default_resolver(&name, flashtex_engine::system::ENGINE_NAME);
    let _ = r.find("fmtutil.cnf", Format::Cnf);
    println!(
        "resolver: {} (set-up {:.1} ms)",
        r.describe(),
        t0.elapsed().as_secs_f64() * 1e3
    );
    let mut c = FormatCache::new(dir);
    c.engine_exe = Some(engine_exe());
    let t = Instant::now();
    let p = c
        .ensure(&name, &name, r.as_mut())
        .unwrap_or_else(|e| die(e));
    let ms = t.elapsed().as_secs_f64() * 1e3;
    println!(
        "{}: {} in {ms:.1} ms: {} ({} files checked, {} rehashed, {} lookups{})",
        name,
        if c.last.built { "built" } else { "cache hit" },
        p.display(),
        c.last.files_checked,
        c.last.files_rehashed,
        c.last.lookups_checked,
        c.last
            .stale_reason
            .as_deref()
            .map(|s| format!("; was stale: {s}"))
            .unwrap_or_default()
    );
    if hits > 0 {
        let mut v = vec![];
        for _ in 0..hits {
            let t = Instant::now();
            c.ensure(&name, &name, r.as_mut())
                .unwrap_or_else(|e| die(e));
            v.push(t.elapsed().as_secs_f64() * 1e3);
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "hit validation over {hits} runs: median {:.2} ms, min {:.2}, max {:.2} ({} files, {} lookups; lookups {:.2} ms of the last)",
            v[hits / 2],
            v[0],
            v[hits - 1],
            c.last.files_checked,
            c.last.lookups_checked,
            c.last.lookups_ms
        );
    }
}

/// Paths from read-set files (`open\t<path>`, or one path per line) and
/// format manifests (`file\t...\t<path>`).
fn read_paths(lists: &[String], manifests: &[String]) -> Vec<PathBuf> {
    let mut out = BTreeSet::new();
    for l in lists {
        let t = std::fs::read_to_string(l).unwrap_or_else(|e| die(format!("{l}: {e}")));
        for line in t.lines() {
            let p = line.strip_prefix("open\t").unwrap_or(line);
            if line.starts_with("lookup\t") || !p.starts_with('/') {
                continue;
            }
            if Path::new(p).is_file() {
                out.insert(PathBuf::from(p));
            }
        }
    }
    for m in manifests {
        let t = std::fs::read_to_string(m).unwrap_or_else(|e| die(format!("{m}: {e}")));
        for line in t.lines().filter(|l| l.starts_with("file\t")) {
            if let Some(p) = line.splitn(8, '\t').nth(7) {
                out.insert(PathBuf::from(p));
            }
        }
    }
    out.into_iter().collect()
}

fn bundle_pack(args: &[String]) {
    let out = opt(args, "--out").unwrap_or_else(|| die("--out F.ttb"));
    let t =
        resolver::discover_texlive().unwrap_or_else(|| die("no TeX Live to make a bundle from"));
    let mut r = KpathseaResolver::for_texlive(&t.bin, "pdflatex", "");
    let root = PathBuf::from(r.var_value("TEXMFROOT").unwrap_or_else(|| die("TEXMFROOT")));
    let tlpdb = opt(args, "--tlpdb")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("tlpkg/texlive.tlpdb"));
    let packages = bundle::build::parse_tlpdb_runfiles(
        &std::fs::read_to_string(&tlpdb)
            .unwrap_or_else(|e| die(format!("{}: {e}", tlpdb.display()))),
    );
    let read = read_paths(&opt_all(args, "--read"), &opt_all(args, "--manifest"));
    let whole = args.iter().any(|a| a == "--whole-packages");
    let sel =
        bundle::build::select(&root, &read, &packages, whole, &mut r).unwrap_or_else(|e| die(e));
    // Core: the packages of the files the core read lists name.
    let core_read = read_paths(
        &opt_all(args, "--core-read"),
        &opt_all(args, "--core-manifest"),
    );
    let core_rel: BTreeSet<String> = core_read
        .iter()
        .filter_map(|p| p.strip_prefix(&root).ok())
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut core: Vec<String> = sel
        .files
        .iter()
        .filter(|f| core_rel.contains(&f.path))
        .map(|f| f.package.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    core.sort();
    let nfiles = sel.files.len();
    let (bytes, ix) = ttb::pack(sel.files, &bundle::build::default_search(), &core);
    std::fs::write(&out, &bytes).unwrap_or_else(|e| die(format!("{out}: {e}")));
    let core_bytes: u64 = ix
        .packages
        .iter()
        .filter(|p| core.contains(&p.name))
        .map(|p| p.len)
        .sum();
    let h = ttb::Header::parse(&bytes).unwrap();
    println!("bundle {out}");
    println!("digest {}", hex(&h.digest));
    println!(
        "{} files ({} read, {} outside TeX Live left out, {} not kpathsea's choice left out), {} packages, {} core ({} bytes)",
        nfiles,
        read.len(),
        sel.outside.len(),
        sel.hidden,
        ix.packages.len(),
        core.len(),
        core_bytes
    );
    println!(
        "size {} bytes, index {} bytes gzipped ({} raw)",
        bytes.len(),
        h.index_gzip_len,
        h.index_real_len
    );
}

fn bundle_measure(args: &[String]) {
    let bundle_path = opt(args, "--bundle").unwrap_or_else(|| die("--bundle F.ttb"));
    let doc = PathBuf::from(opt(args, "--doc").unwrap_or_else(|| die("--doc main.tex")));
    let bytes = std::fs::read(&bundle_path).unwrap_or_else(|e| die(format!("{bundle_path}: {e}")));
    let digest = hex(&ttb::Header::parse(&bytes).unwrap_or_else(|e| die(e)).digest);
    let work = PathBuf::from(opt(args, "--work").unwrap_or_else(|| {
        std::env::temp_dir()
            .join(format!("flashtex-bundle-measure-{}", std::process::id()))
            .display()
            .to_string()
    }));
    let _ = std::fs::remove_dir_all(&work);
    for d in ["bin", "doc", "bundles", "formats"] {
        std::fs::create_dir_all(work.join(d)).unwrap();
    }
    std::os::unix::fs::symlink(engine_exe(), work.join("bin/pdftex")).unwrap();
    let server = FixtureServer::start(bytes, "bundle.ttb").unwrap();
    let dir = doc.parent().unwrap_or(Path::new("."));
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), work.join("doc").join(e.file_name())).unwrap();
        }
    }
    let job = doc.file_stem().unwrap().to_string_lossy().into_owned();
    println!("bundle {bundle_path} ({digest}) served at {}", server.url);
    for (label, offline) in [("cold", false), ("warm", false), ("offline", true)] {
        let (r0, b0) = (server.requests(), server.bytes());
        let t = Instant::now();
        let st = Command::new(work.join("bin/pdftex"))
            .args([
                "-fmt=pdflatex",
                "-interaction=nonstopmode",
                "-halt-on-error",
            ])
            .arg(doc.file_name().unwrap())
            .current_dir(work.join("doc"))
            .env_remove("FLASHTEX_FORMATS")
            .env("FLASHTEX_RESOLVER", "bundle")
            .env("FLASHTEX_BUNDLE_URL", &server.url)
            .env("FLASHTEX_BUNDLE_DIGEST", &digest)
            .env("FLASHTEX_BUNDLE_OFFLINE", if offline { "1" } else { "0" })
            .env("FLASHTEX_BUNDLE_CACHE_DIR", work.join("bundles"))
            .env("FLASHTEX_FORMAT_CACHE_DIR", work.join("formats"))
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let log = std::fs::read_to_string(work.join("doc").join(format!("{job}.log")))
            .unwrap_or_default();
        let out_line = log
            .lines()
            .find(|l| l.starts_with("Output written"))
            .unwrap_or("(no output)");
        println!(
            "{label:>8}: {st}, {ms:.0} ms, {} requests, {} bytes; {out_line}",
            server.requests() - r0,
            server.bytes() - b0
        );
        let _ = std::fs::remove_file(work.join("doc").join(format!("{job}.pdf")));
    }
    let files = std::fs::read_dir(work.join("bundles").join(&digest).join("files"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.metadata().is_ok_and(|m| m.len() > 0))
                .count()
        })
        .unwrap_or(0);
    println!("files in the bundle cache after the runs: {files}");
    // Direct use of the resolver: every file of one package on demand.
    let spec = BundleSpec {
        url: server.url.clone(),
        digest: digest.clone(),
        offline: false,
    };
    let cache = work.join("bundles-direct");
    let (r0, b0) = (server.requests(), server.bytes());
    let mut r = BundleResolver::open(spec, &cache, "pdflatex", "").unwrap_or_else(|e| die(e));
    let core = r.bundle.stats;
    let _ = r.find("amsmath.sty", Format::Tex);
    println!(
        "resolver alone: open {} requests / {} bytes (header, index, core); then amsmath.sty: {} requests / {} bytes in total",
        core.requests,
        core.bytes,
        server.requests() - r0,
        server.bytes() - b0
    );
}

fn index_estimate(args: &[String]) {
    let t = resolver::discover_texlive().unwrap_or_else(|| die("no TeX Live"));
    let mut r = KpathseaResolver::for_texlive(&t.bin, "pdflatex", "");
    let root = PathBuf::from(r.var_value("TEXMFROOT").unwrap_or_default());
    let tlpdb = opt(args, "--tlpdb")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("tlpkg/texlive.tlpdb"));
    let packages = bundle::build::parse_tlpdb_runfiles(
        &std::fs::read_to_string(&tlpdb).unwrap_or_else(|e| die(e)),
    );
    let mut ix = ttb::Index::default();
    let mut off = ttb::HEADER_SIZE;
    let mut seen = BTreeSet::new();
    for (p, fs) in &packages {
        let start = off;
        for f in fs {
            if !f.starts_with("texmf-dist/")
                || !seen.insert(f.rsplit('/').next().unwrap_or(f).to_string())
            {
                continue;
            }
            // A stand-in SHA-256 (of the path) has the entropy of a real one.
            let h = hex(&Sha256::digest(f.as_bytes()));
            ix.files.push(ttb::Entry {
                path: f.clone(),
                start: off,
                gzip_len: 4000,
                real_len: 12000,
                sha256: Some(h),
            });
            off += 4000;
        }
        ix.packages.push(ttb::Package {
            name: p.clone(),
            start,
            len: off - start,
        });
    }
    let text = ix.encode();
    let gz = bundle::gz::gzip(text.as_bytes(), 9);
    println!(
        "all runfiles of {} packages, one per basename: {} files; index {} bytes raw, {} bytes gzipped",
        packages.len(),
        ix.files.len(),
        text.len(),
        gz.len()
    );
}
