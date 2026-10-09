//! ls-R's packed index (third_party/kpathsea/db.c, `packed_build`): every
//! lookup answers what kpathsea's hash table answered, whether the index was
//! read from ls-R or mapped from its cache file.
//!
//! * An ls-R written here, with what the parser must get right: `./` and
//!   absolute directory lines, a hidden directory (its files ignored), a
//!   file before any directory, blank, `.` and `..` lines, CR LF and CR line
//!   ends, a null byte inside a name, the same name in several directories
//!   (in ls-R's order), and a last line without its newline. Each lookup is
//!   compared with `kpsewhich -all` on the same tree, which hashes ls-R.
//! * TeX Live's own ls-R: names in several trees (article.cls) and the usual
//!   ones, `-all`, against `kpsewhich`.
//!
//! Each case runs three times: the first instance builds the index (and
//! writes the cache), the second maps it from the cache file, the third
//! finds every cache file damaged and builds the index again.
//!
//! Its own process: it sets `TEXINPUTS`, `TEXMFDBS` and the cache directory.

mod common;

use flashtex_engine::resolver::{find_texlive_bin, FileResolver, Format, KpathseaResolver};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

fn kpsewhich_all(bin: &Path, name: &str) -> Vec<String> {
    let out = Command::new(bin.join("kpsewhich"))
        .args([
            "-progname=pdflatex",
            "-engine=pdftex",
            "-all",
            "-format=tex",
        ])
        .arg(name)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

fn ours_all(r: &mut KpathseaResolver, name: &str) -> Vec<String> {
    r.find_all(name, Format::Tex)
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_packed_index_answers_as_kpathseas_table() {
    let Some(bin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-lsr-index");
    let _ = std::fs::remove_dir_all(&base);
    let (tree, cache) = (base.join("tree"), base.join("cache"));
    for d in ["a", "b/c", ".hidden", "d"] {
        std::fs::create_dir_all(tree.join(d)).unwrap();
    }
    for f in [
        "a/dup.sty",
        "b/c/dup.sty",
        "d/dup.sty",
        "a/one.sty",
        ".hidden/secret.sty",
        "b/c/crlf.sty",
        "d/nul.sty",
        "d/last.sty",
        "top.sty",
    ] {
        std::fs::write(tree.join(f), "% x\n").unwrap();
    }
    let abs = tree.join("d").to_string_lossy().into_owned();
    let mut lsr: Vec<u8> = b"% ls-R -- filename database for kpathsea; do not change this line.\n\
        stray.sty\n./:\ntop.sty\n.\n..\n\n./a:\ndup.sty\none.sty\n\n./.hidden:\nsecret.sty\n\
        ./b/c:\r\ndup.sty\r\ncrlf.sty\rdup.sty\n"
        .to_vec();
    lsr.extend_from_slice(format!("{abs}:\ndup.sty\nn\0ul.sty\nlast.sty").as_bytes());
    std::fs::write(tree.join("ls-R"), &lsr).unwrap();
    // old enough for the index to be cached (db.c: two seconds)
    let old = SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(tree.join("ls-R"))
        .unwrap()
        .set_modified(old)
        .unwrap();
    // the tree first, then TeX Live's own (an empty element is the default)
    std::env::set_var("TEXINPUTS", format!("!!{}//:", tree.display()));
    std::env::set_var("TEXMFDBS", format!("{}:", tree.display()));
    std::env::set_var("FLASHTEX_FORMAT_CACHE_DIR", &cache);
    let names = [
        "dup.sty",
        "one.sty",
        "secret.sty",
        "crlf.sty",
        "nul.sty",
        "last.sty",
        "top.sty",
        "stray.sty",
        "none.sty",
        "README.md",
        "article.cls",
        "hyperref.sty",
        "00readme.txt",
    ];
    let want: Vec<Vec<String>> = names.iter().map(|n| kpsewhich_all(&bin, n)).collect();
    // the synthetic tree's own answers, as kpathsea's table gives them
    // dup.sty: a, b/c (listed twice, found once) and d, in that order
    assert_eq!(want[0].len(), 3, "{:?}", want[0]);
    // the hidden directory's file and the file before any directory: none
    assert!(want[2].is_empty() && want[7].is_empty(), "{want:?}");
    // CR LF and CR ends, the null byte, the last line, `./`: found
    assert!(
        [3, 4, 5, 6, 10, 11].iter().all(|&i| !want[i].is_empty()),
        "{want:?}"
    );
    for round in ["built", "mapped", "damaged"] {
        if round == "damaged" {
            // every cached index damaged: refused, built again
            for e in std::fs::read_dir(cache.join("lsr")).unwrap() {
                let p = e.unwrap().path();
                let mut b = std::fs::read(&p).unwrap();
                let n = b.len();
                for x in &mut b[n / 2..n / 2 + 64.min(n / 2)] {
                    *x = 0xff;
                }
                std::fs::write(&p, &b[..n - 1]).unwrap();
            }
        }
        let mut r = KpathseaResolver::for_texlive(&bin, "pdflatex", "pdftex");
        for (n, w) in names.iter().zip(&want) {
            assert_eq!(&ours_all(&mut r, n), w, "{n} ({round})");
        }
        let idx = std::fs::read_dir(cache.join("lsr"))
            .map(|d| d.count())
            .unwrap_or(0);
        assert!(idx > 0, "no index cached ({round})");
    }
    let _ = std::fs::remove_dir_all(&base);
}
