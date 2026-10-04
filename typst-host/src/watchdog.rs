//! The watchdog (DESIGN.md §15.2; spec §11.10).
//!
//! A Typst compile cannot be cancelled: a WASM plugin runs in wasmi with no
//! fuel or memory limit, and a `for` over a huge range is unbounded (Track
//! A §6, Track C §2.7). The watchdog has two layers (DESIGN.md §15.2): this
//! one, the first line, watches the host's own compiles from a thread; the
//! app, the second, restarts a host on any exit. When **the compile
//! itself** (not socket writes, the PDF export, font hashing or eviction:
//! a slow client must never get a healthy host killed) runs longer than
//! its wall-time budget, or the process's resident memory passes a ceiling
//! during it, the thread prints one line saying why to stderr, kills the
//! host's children ([`track_child`]: package downloads), removes its
//! temporary directory ([`temp_dir`]) and `_exit`s with [`EXIT_CODE`] (no
//! destructors, no atexit handlers: the main thread is stuck in the
//! compile). The client sees the socket close in the middle of a compile;
//! the app restarts the host, marks the pages it shows stale and compiles
//! cold. Killing the process is also what frees comemo's caches at once.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The exit status of a host the watchdog stopped.
pub const EXIT_CODE: i32 = 86;

/// How often the watching thread looks.
const POLL: Duration = Duration::from_millis(50);

/// Budgets: wall time for an incremental compile and for a cold one (the
/// first compile of a document, or one that starts over), and the resident
/// memory ceiling (`None`: none).
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub wall: Duration,
    pub wall_cold: Duration,
    pub rss_bytes: Option<u64>,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            wall: Duration::from_secs(10),
            wall_cold: Duration::from_secs(60),
            rss_bytes: Some(4096 << 20),
        }
    }
}

struct Running {
    id: i64,
    since: Instant,
    budget: Duration,
}

static CHILDREN: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// A child process (a download) the watchdog must kill before it exits.
pub fn track_child(pid: u32) {
    CHILDREN.lock().unwrap().push(pid);
}

/// The child exited or was reaped.
pub fn untrack_child(pid: u32) {
    CHILDREN.lock().unwrap().retain(|&p| p != pid);
}

/// The host process's own temporary directory (`DONE.pdf` when the client
/// names no output directory): removed when the watchdog stops the host.
pub fn temp_dir(pid: u32) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("flashtex-typst-host-{pid}"))
}

/// Remove the temporary directories of hosts that no longer run (a host
/// killed from outside cannot remove its own). Called at start.
pub fn sweep_stale_temp_dirs() -> usize {
    let mut removed = 0;
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return 0;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix("flashtex-typst-host-"))
            .and_then(|p| p.parse::<i32>().ok())
        else {
            continue;
        };
        // SAFETY: signal 0 only checks that the process exists.
        let alive = unsafe { libc::kill(pid, 0) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
        if !alive && std::fs::remove_dir_all(e.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Stop now: kill the children, remove the temporary directory, `_exit`.
fn stop(why: &str) -> ! {
    eprintln!("flashtex-typst-host: {why}");
    for pid in CHILDREN.lock().map(|c| c.clone()).unwrap_or_default() {
        // SAFETY: a pid this process started; SIGKILL needs nothing else.
        unsafe { libc::kill(pid as i32, libc::SIGKILL) };
    }
    let _ = std::fs::remove_dir_all(temp_dir(std::process::id()));
    // SAFETY: _exit ends the process at once; nothing here needs unwinding.
    unsafe { libc::_exit(EXIT_CODE) }
}

/// The running compile, shared with the watching thread.
#[derive(Clone)]
pub struct Watchdog {
    running: Arc<Mutex<Option<Running>>>,
    limits: Limits,
}

impl Watchdog {
    /// Start the watching thread.
    pub fn start(limits: Limits) -> Watchdog {
        let w = Watchdog {
            running: Arc::new(Mutex::new(None)),
            limits,
        };
        let running = w.running.clone();
        std::thread::Builder::new()
            .name("watchdog".into())
            .spawn(move || {
                // When memory was last seen under the ceiling: the line says
                // how long after that the host stopped (the kill latency,
                // whatever the allocation speed).
                let mut under = Instant::now();
                loop {
                    std::thread::sleep(POLL);
                    let why = {
                        let r = running.lock().unwrap();
                        let Some(r) = r.as_ref() else {
                            under = Instant::now();
                            continue;
                        };
                        let elapsed = r.since.elapsed();
                        if elapsed > r.budget {
                            Some(format!(
                                r#"{{"watchdog":"wall","id":{},"elapsed_ms":{},"budget_ms":{},"over_ms":{}}}"#,
                                r.id,
                                elapsed.as_millis(),
                                r.budget.as_millis(),
                                (elapsed - r.budget).as_millis()
                            ))
                        } else {
                            match (limits.rss_bytes, resident_bytes()) {
                                (Some(max), Some(now)) if now > max => Some(format!(
                                    r#"{{"watchdog":"rss","id":{},"rss_mb":{},"ceiling_mb":{},"since_under_ms":{}}}"#,
                                    r.id,
                                    now >> 20,
                                    max >> 20,
                                    under.elapsed().as_millis()
                                )),
                                _ => {
                                    under = Instant::now();
                                    None
                                }
                            }
                        }
                    };
                    if let Some(why) = why {
                        stop(&why);
                    }
                }
            })
            .expect("start the watchdog thread");
        w
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// A compile starts.
    pub fn begin(&self, id: i64, cold: bool) {
        let budget = if cold {
            self.limits.wall_cold
        } else {
            self.limits.wall
        };
        *self.running.lock().unwrap() = Some(Running {
            id,
            since: Instant::now(),
            budget,
        });
    }

    /// It ended (its DONE is out).
    pub fn end(&self) {
        *self.running.lock().unwrap() = None;
    }
}

/// The process's resident memory now, where the OS says.
pub fn resident_bytes() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: proc_pidinfo writes at most `size` bytes into `info`.
        let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
        let n = unsafe {
            libc::proc_pidinfo(
                libc::getpid(),
                libc::PROC_PIDTASKINFO,
                0,
                &mut info as *mut _ as *mut libc::c_void,
                size,
            )
        };
        (n == size).then_some(info.pti_resident_size)
    }
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/self/statm").ok()?;
        let pages: u64 = s.split_whitespace().nth(1)?.parse().ok()?;
        // SAFETY: sysconf has no preconditions.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        Some(pages * page.max(0) as u64)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn resident_memory_is_measured() {
        let r = super::resident_bytes().expect("macOS and Linux report it");
        assert!(r > 1 << 20, "{r}");
    }
}
