//! The engine host as a child process. This tool never links the engine
//! (GPL): it runs `flashtex-host --socket PATH` and speaks display-list-v3
//! to it, exactly as the Mac app does (docs/protocol/display-list-v3.md §6).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Host {
    child: Child,
    pub socket: PathBuf,
    /// Spawn to the `listening on` line (ms): format check and warm-up.
    pub ready_ms: f64,
    /// The start-up JSON line (`{"texlive": …, "warm_ms": …}`), if any.
    pub startup: String,
    lines: mpsc::Receiver<String>,
}

impl Host {
    /// Start a host and wait until it listens. `s0_cache`: the directory
    /// the host persists each document's S₀ in (the reopen measurement).
    pub fn start(
        bin: &Path,
        pool: Option<&Path>,
        socket: &Path,
        s0_cache: &Path,
        log: &Path,
    ) -> Result<Host, String> {
        let _ = std::fs::remove_file(socket);
        let err = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .map_err(|e| format!("{}: {e}", log.display()))?;
        let spawned = Instant::now();
        let mut cmd = Command::new(bin);
        if let Some(p) = pool {
            cmd.env("FLASHTEX_POOL", p);
        }
        let mut child = cmd
            .arg("--socket")
            .arg(socket)
            .arg("--s0-cache")
            .arg(s0_cache)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(err)
            .spawn()
            .map_err(|e| format!("{}: {e}", bin.display()))?;
        let out = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for l in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        let mut startup = String::new();
        // The first start may build the format cache (about 4 s on a quiet
        // machine); allow for a loaded one.
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(l) => {
                    if l.starts_with("flashtex-host: listening on") {
                        break;
                    }
                    if let Some(j) = l.strip_prefix("flashtex-host: {") {
                        startup = format!("{{{j}");
                    }
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "flashtex-host did not start listening (see {})",
                        log.display()
                    ));
                }
            }
        }
        Ok(Host {
            child,
            socket: socket.to_path_buf(),
            ready_ms: spawned.elapsed().as_secs_f64() * 1e3,
            startup,
            lines: rx,
        })
    }

    /// Lines the host printed since the last call (e.g. `saved_s0`).
    pub fn drain(&self) -> Vec<String> {
        self.lines.try_iter().collect()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // Our own child, by handle: nothing else is signalled.
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
    }
}
