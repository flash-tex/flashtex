//! Performance modes (lane PERF-MODES; DESIGN.md §1.2 "Performance modes",
//! §5.2, §5.6): *Low Memory*, *Balanced* (the default, today's behaviour and
//! every gate) and *High Performance*, each a named profile of the host's
//! knobs. Every knob changes only what the host keeps or works out ahead
//! (checkpoints and their spacing, idle trimming, the keep-warm window),
//! never what the engine computes: a compile's output is identical in every
//! mode, which the soundness sweeps check per mode (`FLASHTEX_PROFILE`).
//!
//! A profile is chosen by `flashtex-host --profile NAME`, `FLASHTEX_PROFILE`,
//! the client's `HELLO` (`"profile"`) or, live, a `PROFILE` message
//! (docs/protocol/display-list-v3.md §6.9, capability `profile-v1`). A knob
//! given explicitly (`--budget`, `--timed`, `--keep-warm`, or its
//! environment variable) is pinned: no profile overrides it.

use flashtex_display_list::json::{obj, s as js, Json};

/// The three modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    LowMemory,
    #[default]
    Balanced,
    HighPerformance,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::LowMemory, Mode::Balanced, Mode::HighPerformance];

    /// The wire name (`low-memory`, `balanced`, `high-performance`).
    pub fn name(self) -> &'static str {
        match self {
            Mode::LowMemory => "low-memory",
            Mode::Balanced => "balanced",
            Mode::HighPerformance => "high-performance",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.name() == s)
    }

    /// `FLASHTEX_PROFILE`, else Balanced (an unknown name is Balanced, and
    /// says so on stderr once).
    pub fn from_env() -> Mode {
        match std::env::var("FLASHTEX_PROFILE") {
            Ok(v) if !v.is_empty() => Mode::parse(&v).unwrap_or_else(|| {
                eprintln!("flashtex: FLASHTEX_PROFILE={v}: not a profile; using balanced");
                Mode::Balanced
            }),
            _ => Mode::Balanced,
        }
    }
}

/// Knobs fixed by the command line or the environment: a profile sets only
/// the others.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pinned {
    pub budget: Option<usize>,
    pub timed_s: Option<f64>,
    /// `Some(None)`: segment checkpoints off.
    pub segment_s: Option<Option<f64>>,
    pub dense: Option<usize>,
    pub keep_warm_ms: Option<u64>,
    pub prepare: Option<bool>,
    pub trim_after_ms: Option<Option<u64>>,
}

impl Pinned {
    /// The knobs the environment fixes (`FLASHTEX_BUDGET`, `FLASHTEX_TIMED_S`,
    /// `FLASHTEX_SEGMENT_S`, `FLASHTEX_DENSE`, `FLASHTEX_HOST_KEEP_WARM_MS`,
    /// `FLASHTEX_NO_PREPARE`; `FLASHTEX_NO_TRIM` leaves only `malloc_trim`
    /// out, in every mode).
    pub fn from_env() -> Pinned {
        let num = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f64>().ok());
        Pinned {
            budget: num("FLASHTEX_BUDGET").map(|v| v as usize),
            timed_s: num("FLASHTEX_TIMED_S"),
            segment_s: match std::env::var("FLASHTEX_SEGMENT_S") {
                Ok(v) if v == "off" => Some(None),
                Ok(v) => v.parse().ok().map(Some),
                Err(_) => None,
            },
            dense: num("FLASHTEX_DENSE").map(|v| v as usize),
            keep_warm_ms: num("FLASHTEX_HOST_KEEP_WARM_MS").map(|v| v as u64),
            prepare: std::env::var_os("FLASHTEX_NO_PREPARE").map(|_| false),
            trim_after_ms: None,
        }
    }
}

/// One mode's knobs, as the host applies them.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub mode: Mode,
    /// Bytes the undo logs may hold (`incr::Options::budget`, DESIGN.md §5.2).
    pub budget: usize,
    /// Pages around the cursor whose checkpoints all stay (`incr::thin`).
    pub dense: usize,
    /// Segment checkpoints between shipouts at least this far apart in
    /// engine time (`incr::Options::segment_s`); `None`: none.
    pub segment_s: Option<f64>,
    /// A checkpoint after this much engine time without one.
    pub timed_s: f64,
    /// After a compile, the engine thread stays warm this long (DESIGN.md
    /// §5.1, keep-warm; 0: off).
    pub keep_warm_ms: u64,
    /// Work out the next keystroke's restore while the engine waits
    /// (`incr::Session::prepare_next`, §5.6 prepare-ahead).
    pub prepare: bool,
    /// Idle this long after the keep-warm window: drop the old run's cached
    /// chunks and give the heap's free pages back (`trim_caches`,
    /// `malloc_trim`); `None`: never.
    pub trim_after_ms: Option<u64>,
    /// Keep the host's page cache (every page's display list, for clients
    /// that lack a page) gzip-packed, packed off the engine thread; and drop
    /// the restores' spare tail buffers at the idle trim.
    pub lean: bool,
}

/// Balanced's budget: DESIGN.md §5.2's 1 GiB.
pub const BALANCED_BUDGET: usize = 1 << 30;
/// Low Memory's budget for the undo logs.
pub const LOW_BUDGET: usize = 96 << 20;
/// High Performance's budget: a quarter of the machine's memory, at least
/// Balanced's and at most this.
pub const HIGH_BUDGET_MAX: usize = 4 << 30;

impl Profile {
    /// `mode`'s knobs, with `pinned` ones as given.
    pub fn new(mode: Mode, pinned: &Pinned) -> Profile {
        let mut p = match mode {
            Mode::LowMemory => Profile {
                mode,
                budget: LOW_BUDGET,
                dense: 4,
                segment_s: Some(crate::incr::DEFAULT_SEGMENT_S),
                timed_s: 0.020,
                keep_warm_ms: 2000,
                prepare: true,
                trim_after_ms: Some(500),
                lean: true,
            },
            Mode::Balanced => Profile {
                mode,
                budget: BALANCED_BUDGET,
                dense: crate::incr::DEFAULT_DENSE,
                segment_s: Some(crate::incr::DEFAULT_SEGMENT_S),
                timed_s: 0.020,
                keep_warm_ms: 2000,
                prepare: true,
                trim_after_ms: Some(2000),
                lean: false,
            },
            Mode::HighPerformance => Profile {
                mode,
                budget: (physical_memory() / 4).clamp(BALANCED_BUDGET, HIGH_BUDGET_MAX),
                // every checkpoint within 512 pages stays (the budget permitting):
                // the first keystroke far from the last restarts near the edit,
                // not at its page's start (#1573: +39-143 M instructions)
                dense: 512,
                segment_s: Some(crate::incr::DEFAULT_SEGMENT_S),
                timed_s: 0.020,
                keep_warm_ms: 10_000,
                prepare: true,
                trim_after_ms: None,
                lean: false,
            },
        };
        if let Some(v) = pinned.budget {
            p.budget = v;
        }
        if let Some(v) = pinned.timed_s {
            p.timed_s = v;
        }
        if let Some(v) = pinned.segment_s {
            p.segment_s = v;
        }
        if let Some(v) = pinned.dense {
            p.dense = v;
        }
        if let Some(v) = pinned.keep_warm_ms {
            p.keep_warm_ms = v;
        }
        if let Some(v) = pinned.prepare {
            p.prepare = v;
        }
        if let Some(v) = pinned.trim_after_ms {
            p.trim_after_ms = v;
        }
        p
    }

    /// The profile `FLASHTEX_PROFILE` and the environment's pinned knobs
    /// give (what `incr::Options::default` uses).
    pub fn from_env() -> Profile {
        Profile::new(Mode::from_env(), &Pinned::from_env())
    }

    /// The effective knobs, for `HELLO.profile` and the `PROFILE` reply.
    pub fn json(&self) -> Json {
        let num = |v: f64| Json::Num(v);
        obj([
            ("mode", js(self.mode.name())),
            ("budget", Json::Int(self.budget as i64)),
            ("dense", Json::Int(self.dense as i64)),
            (
                "segment_ms",
                self.segment_s.map_or(Json::Null, |s| num(s * 1e3)),
            ),
            ("timed_ms", num(self.timed_s * 1e3)),
            ("keep_warm_ms", Json::Int(self.keep_warm_ms as i64)),
            ("prepare", Json::Bool(self.prepare)),
            (
                "trim_after_ms",
                self.trim_after_ms
                    .map_or(Json::Null, |t| Json::Int(t as i64)),
            ),
            ("lean", Json::Bool(self.lean)),
            (
                "allocator_returns_free_pages",
                Json::Bool(allocator_returns_free_pages()),
            ),
        ])
    }
}

/// The start-up mode a host's command line asks for: `--profile NAME`,
/// else `FLASHTEX_PROFILE` (as `flashtex-host` reads them).
pub fn startup_mode(argv: &[String]) -> Mode {
    match argv.iter().position(|a| a == "--profile") {
        Some(i) => argv
            .get(i + 1)
            .and_then(|v| Mode::parse(v))
            .unwrap_or_else(Mode::from_env),
        None => Mode::from_env(),
    }
}

/// Whether the C allocator gives freed pages back to the system at once
/// (macOS: `MallocSpaceEfficient` set when the process started; see
/// [`space_efficient_reexec`]). Process-wide and fixed at start: a mode
/// switched to later does not change it.
pub fn allocator_returns_free_pages() -> bool {
    cfg!(target_os = "macos") && std::env::var_os("MallocSpaceEfficient").is_some()
}

/// Low Memory on macOS: the system allocator (xzone malloc) keeps about
/// half of what a process frees, and `malloc_zone_pressure_relief` gives
/// none of it back, so the host's footprint is twice what it holds (a
/// 4-page article: 134 MB with 28 MB in use). Its `MallocSpaceEfficient`
/// setting returns freed pages at once (57 MB; the keystroke's host CPU
/// time +6-10 %, the edited page within noise; lane MEM-MODES,
/// 2026-10-09). The allocator reads it before `main`, so a host started in
/// Low Memory (`--profile low-memory` or `FLASHTEX_PROFILE`) runs itself
/// again with it set, once, as the same process (`exec`). Nothing else
/// changes: output is the allocator's to place, never to compute.
/// `FLASHTEX_NO_SPACE_EFFICIENT` keeps the allocator as it is.
#[cfg(target_os = "macos")]
pub fn space_efficient_reexec(argv: &[String]) {
    use std::os::unix::process::CommandExt;
    if std::env::var_os("MallocSpaceEfficient").is_some()
        || std::env::var_os("FLASHTEX_NO_SPACE_EFFICIENT").is_some()
        || startup_mode(argv) != Mode::LowMemory
    {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut c = std::process::Command::new(exe);
    if let Some(a0) = argv.first() {
        c.arg0(a0);
    }
    let err = c
        .args(argv.iter().skip(1))
        .env("MallocSpaceEfficient", "1")
        .exec();
    // only on failure; the host goes on as it was started
    eprintln!("flashtex-host: Low Memory's allocator setting: {err}");
}

/// The machine's physical memory in bytes (0 if unknown).
pub fn physical_memory() -> usize {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn sysctlbyname(
                name: *const std::ffi::c_char,
                oldp: *mut std::ffi::c_void,
                oldlenp: *mut usize,
                newp: *mut std::ffi::c_void,
                newlen: usize,
            ) -> i32;
        }
        let mut v: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        // SAFETY: a read-only sysctl into a u64 of the size given.
        let r = unsafe {
            sysctlbyname(
                c"hw.memsize".as_ptr(),
                &mut v as *mut u64 as *mut std::ffi::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if r == 0 {
            return v as usize;
        }
        0
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|t| {
                let l = t.lines().find(|l| l.starts_with("MemTotal:"))?;
                let kb: usize = l.split_whitespace().nth(1)?.parse().ok()?;
                Some(kb << 10)
            })
            .unwrap_or(0)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for m in Mode::ALL {
            assert_eq!(Mode::parse(m.name()), Some(m));
        }
        assert_eq!(Mode::parse("fast"), None);
    }

    #[test]
    fn balanced_is_todays_defaults() {
        let p = Profile::new(Mode::Balanced, &Pinned::default());
        assert_eq!(p.budget, 1 << 30);
        assert_eq!(p.dense, crate::incr::DEFAULT_DENSE);
        assert_eq!(p.segment_s, Some(crate::incr::DEFAULT_SEGMENT_S));
        assert_eq!(p.timed_s, 0.020);
        assert_eq!(p.keep_warm_ms, 2000);
        assert!(p.prepare);
        assert_eq!(p.trim_after_ms, Some(2000));
        assert!(!p.lean);
    }

    #[test]
    fn modes_order_memory() {
        let [l, b, h] = Mode::ALL.map(|m| Profile::new(m, &Pinned::default()));
        assert!(l.budget < b.budget && b.budget <= h.budget);
        assert!(h.budget <= HIGH_BUDGET_MAX);
        assert!(l.dense < b.dense && b.dense < h.dense);
        assert_eq!(h.trim_after_ms, None);
    }

    #[test]
    fn the_startup_mode_is_the_flags_else_the_environments() {
        let argv = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            startup_mode(&argv(&["h", "--socket", "s", "--profile", "low-memory"])),
            Mode::LowMemory
        );
        assert_eq!(
            startup_mode(&argv(&["h", "--profile", "high-performance"])),
            Mode::HighPerformance
        );
        assert_eq!(
            startup_mode(&argv(&["h", "--socket", "s"])),
            Mode::from_env()
        );
    }

    #[test]
    fn pinned_knobs_win() {
        let pin = Pinned {
            budget: Some(4 << 20),
            keep_warm_ms: Some(0),
            segment_s: Some(None),
            ..Pinned::default()
        };
        for m in Mode::ALL {
            let p = Profile::new(m, &pin);
            assert_eq!(p.budget, 4 << 20);
            assert_eq!(p.keep_warm_ms, 0);
            assert_eq!(p.segment_s, None);
        }
    }
}
