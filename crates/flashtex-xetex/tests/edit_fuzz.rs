//! Lane TEST-GAPS: random edit sequences against scratch compiles on
//! `flashtex-host-unicode`, the pdfTeX host's fuzz
//! (crates/flashtex-engine/tests/edit_fuzz, shared by `#[path]`) with the
//! edits of a fontspec or unicode-math document besides: non-ASCII text
//! typed and taken out, `\setmainfont` options and their kin changed, a
//! font changed mid-document, math in another math version.
//!
//! The host under the edits runs as the app runs it (in its own process,
//! hot spares on); the scratch compile is the same host's run in a process
//! of its own (`FLASHTEX_UNICODE_INPROCESS=0`, no spares) on a copy of the
//! directory, and must say `cold`. The fixtures (tests/fuzz-fixtures) are
//! the Unicode test cases' (tools/displaylist/unicode-cases) grown to
//! several pages, with TeX Live's fonts only (no system font), so that they
//! run wherever TeX Live is. Needs TeX Live; skipped without.
//!
//! FLASHTEX_FUZZ_SEED, _SEEDS, _STEPS, _DOCS, _TRACE: as for the pdfTeX
//! host (.github/workflows/edit-fuzz.yml, `host: unicode`).

#[path = "../../flashtex-engine/tests/edit_fuzz/mod.rs"]
mod edit_fuzz;

use flashtex_display_list::client::CompileRequest;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Host {
    child: Child,
    sock: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.sock);
    }
}

struct UnicodeFuzz;

impl edit_fuzz::FuzzHost for UnicodeFuzz {
    fn start(&self, tag: &str, scratch: bool) -> (Box<dyn std::any::Any>, PathBuf) {
        // (short: a Unix socket's path fits in 104 bytes on macOS)
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sock = PathBuf::from(format!("/tmp/ftu-{}-{n}-{tag}.sock", std::process::id()));
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_flashtex-host-unicode"));
        cmd.args(["--socket", sock.to_str().unwrap()])
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .env("FLASHTEX_PIN_CLOCK", "1760000000.5")
            .env_remove("FLASHTEX_S0_CACHE")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if scratch {
            cmd.env("FLASHTEX_UNICODE_INPROCESS", "0")
                .env("FLASHTEX_UNICODE_SPARES", "0");
        }
        let child = cmd.spawn().unwrap();
        let t = Instant::now();
        while !sock.exists() {
            // (the first start builds the format)
            assert!(
                t.elapsed() < Duration::from_secs(600),
                "the host did not listen"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        let h = Host {
            child,
            sock: sock.clone(),
        };
        (Box::new(h), sock)
    }
    fn request(&self, id: i64, proj: &Path, out: &Path, main: &str) -> CompileRequest {
        let mut r = CompileRequest::new(id, proj.to_str().unwrap(), main);
        r.output_dir = Some(out.to_str().unwrap().into());
        r
    }
    fn scratch(&self, proj: &Path, out: &Path, main: &str, tools: bool) -> CompileRequest {
        let mut r = self.request(1, proj, out, main);
        if tools {
            r.external_tools = Some("auto".into());
        }
        r
    }
}

#[test]
fn random_edit_sequences_equal_scratch_compiles() {
    if flashtex_engine::resolver::discover_texlive().is_none() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    edit_fuzz::run(
        &UnicodeFuzz,
        &[manifest.join("tests/fuzz-fixtures")],
        &["fontspec-texgyre", "fontspec-scripts", "unicode-math"],
    );
}
