//! The watchdog (DESIGN.md §15.2; spec §11.10).
//!
//! A Typst compile cannot be cancelled: a WASM plugin runs in wasmi with no
//! fuel or memory limit, and a `for` over a huge range is unbounded (Track
//! A §6, Track C §2.7). So the host watches its own compiles from a thread:
//! when one runs longer than its wall-time budget, or the process's
//! resident memory passes a ceiling, it prints one line saying why to
//! stderr and **exits** with [`EXIT_CODE`]. The client sees the socket
//! close in the middle of a compile; the app restarts the host, marks the
//! pages it shows stale and compiles cold (the app may also kill a host it
//! cannot reach, by its pid: the two do not conflict). Killing the process
//! is also what frees comemo's caches at once (§15.2).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The exit status of a host the watchdog stopped.
pub const EXIT_CODE: i32 = 86;

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
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(50));
                let why = {
                    let r = running.lock().unwrap();
                    let Some(r) = r.as_ref() else { continue };
                    let elapsed = r.since.elapsed();
                    if elapsed > r.budget {
                        Some(format!(
                            r#"{{"watchdog":"wall","id":{},"elapsed_ms":{},"budget_ms":{}}}"#,
                            r.id,
                            elapsed.as_millis(),
                            r.budget.as_millis()
                        ))
                    } else {
                        match (limits.rss_bytes, resident_bytes()) {
                            (Some(max), Some(now)) if now > max => Some(format!(
                                r#"{{"watchdog":"rss","id":{},"rss_mb":{},"ceiling_mb":{}}}"#,
                                r.id,
                                now >> 20,
                                max >> 20
                            )),
                            _ => None,
                        }
                    }
                };
                if let Some(why) = why {
                    eprintln!("flashtex-typst-host: {why}");
                    std::process::exit(EXIT_CODE);
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
