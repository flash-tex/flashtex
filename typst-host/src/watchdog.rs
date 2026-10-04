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
//! host's children ([`spawn_tracked`]: package downloads), removes its
//! private temporary directory ([`temp_dir`]) and `_exit`s with
//! [`EXIT_CODE`] (no
//! destructors, no atexit handlers: the main thread is stuck in the
//! compile). The client sees the socket close in the middle of a compile;
//! the app restarts the host, marks the pages it shows stale and compiles
//! cold. Killing the process is also what frees comemo's caches at once.

use std::io;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// The exit status of a host the watchdog stopped.
pub const EXIT_CODE: i32 = 86;

/// How often the watching thread looks.
const POLL: Duration = Duration::from_millis(50);

/// Budgets: wall time for an incremental compile and for a cold one (the
/// first compile of a document, or one that starts over, and the idle
/// check's standard compile), and the resident
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
            wall_cold: Duration::from_secs(180),
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

/// The children registry, never poisoned (the watchdog takes it while
/// stopping the host).
fn children() -> MutexGuard<'static, Vec<u32>> {
    CHILDREN.lock().unwrap_or_else(|e| e.into_inner())
}

/// Spawn `cmd` (a package download) and register its pid under the same
/// lock the watchdog's stop holds until the process ends, so no child is
/// started after the stop began and none is missed.
pub fn spawn_tracked(cmd: &mut Command) -> io::Result<(Child, Tracked)> {
    let mut c = children();
    let child = cmd.spawn()?;
    c.push(child.id());
    let id = child.id();
    Ok((child, Tracked(id)))
}

/// A registered child: unregistered when dropped (after it is reaped).
pub struct Tracked(u32);

impl Drop for Tracked {
    fn drop(&mut self) {
        children().retain(|&p| p != self.0);
    }
}

/// The registered children (tests).
pub fn tracked_children() -> Vec<u32> {
    children().clone()
}

/// The process exists (signal 0; EPERM: it exists, not ours).
pub fn pid_alive(pid: i32) -> bool {
    // SAFETY: signal 0 only checks that the process exists.
    let r = unsafe { libc::kill(pid, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// This user's id.
pub fn uid() -> u32 {
    // SAFETY: getuid has no preconditions.
    unsafe { libc::getuid() }
}

static TEMP: OnceLock<PathBuf> = OnceLock::new();

/// Where the host's private temporary directory lives: `$XDG_RUNTIME_DIR`
/// on Linux when set (per user, mode 0700), else the OS temporary directory
/// (per user on macOS; on Linux the shared `/tmp`, hence the private
/// directory below).
fn temp_base() -> PathBuf {
    #[cfg(target_os = "linux")]
    if let Some(x) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        if x.is_absolute() && x.is_dir() {
            return x;
        }
    }
    std::env::temp_dir()
}

/// The host process's own private temporary directory (`DONE.pdf` when the
/// client names no output directory), created mode 0700 on first use
/// (never an existing one: a name that exists gets a suffix), removed when
/// the watchdog stops the host.
pub fn temp_dir() -> PathBuf {
    TEMP.get_or_init(|| {
        use std::os::unix::fs::DirBuilderExt;
        let base = temp_base();
        let pid = std::process::id();
        for n in 0..1000 {
            let name = if n == 0 {
                format!("flashtex-typst-host-{pid}")
            } else {
                format!("flashtex-typst-host-{pid}-{n}")
            };
            let p = base.join(name);
            match std::fs::DirBuilder::new().mode(0o700).create(&p) {
                Ok(()) => return p,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => break,
            }
        }
        base.join(format!("flashtex-typst-host-{pid}"))
    })
    .clone()
}

/// Remove the temporary directories of this user's hosts that no longer run
/// (a host killed from outside cannot remove its own). Called at start.
pub fn sweep_stale_temp_dirs() -> usize {
    use std::os::unix::fs::MetadataExt;
    let mut removed = 0;
    let Ok(entries) = std::fs::read_dir(temp_base()) else {
        return 0;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix("flashtex-typst-host-"))
            .and_then(|p| p.split('-').next())
            .and_then(|p| p.parse::<i32>().ok())
        else {
            continue;
        };
        // Only a real directory of ours, of a host that is gone.
        let Ok(md) = std::fs::symlink_metadata(e.path()) else {
            continue;
        };
        if !md.is_dir() || md.uid() != uid() || pid_alive(pid) {
            continue;
        }
        if std::fs::remove_dir_all(e.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Stop now: say why (a raw, non-blocking write: `eprintln!` panics on a
/// closed stderr and blocks on a full one), kill the registered children
/// while holding the registry (no download can start after this), remove
/// the temporary directory, `_exit`.
fn stop(why: &str) -> ! {
    let line = format!("flashtex-typst-host: {why}\n");
    // SAFETY: fd 2 and a valid buffer; errors are ignored on purpose.
    unsafe {
        let fl = libc::fcntl(2, libc::F_GETFL);
        if fl >= 0 {
            libc::fcntl(2, libc::F_SETFL, fl | libc::O_NONBLOCK);
        }
        libc::write(2, line.as_ptr().cast(), line.len());
    }
    let held = children();
    for &pid in held.iter() {
        // SAFETY: a pid this process started; SIGKILL needs nothing else.
        unsafe { libc::kill(pid as i32, libc::SIGKILL) };
    }
    if let Some(d) = TEMP.get() {
        let _ = std::fs::remove_dir_all(d);
    }
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
