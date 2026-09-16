//! Host facts a timing number is meaningless without, and the two noise
//! controls the harness applies to itself.
//!
//! Nothing here is allowed to be expensive or to allocate inside a timed
//! region: every reader is called between measurements, never during one.

use std::path::Path;
use std::time::Instant;

/// 1/5/15-minute load average. `None` when the host does not publish one.
///
/// This box runs several build lanes at once, so a row measured under load 12
/// is not comparable with the same row under load 2. The report carries the
/// value before and after every case rather than pretending the machine was
/// idle.
pub fn load_avg() -> Option<(f64, f64, f64)> {
    if let Ok(s) = std::fs::read_to_string("/proc/loadavg") {
        let mut it = s.split_whitespace();
        let a = it.next()?.parse().ok()?;
        let b = it.next()?.parse().ok()?;
        let c = it.next()?.parse().ok()?;
        return Some((a, b, c));
    }
    // macOS: `sysctl -n vm.loadavg` -> "{ 1.23 4.56 7.89 }".
    let out = std::process::Command::new("sysctl").args(["-n", "vm.loadavg"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).replace(['{', '}'], " ");
    let v: Vec<f64> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    match v[..] {
        [a, b, c, ..] => Some((a, b, c)),
        _ => None,
    }
}

/// Peak resident set of this process in KiB (`VmHWM`), i.e. the high-water
/// mark since the process started. Linux only; `None` elsewhere, and the
/// report says so rather than substituting a current-RSS reading that would
/// quietly answer a different question.
pub fn peak_rss_kb() -> Option<u64> {
    proc_status_kb("VmHWM:")
}

/// Current resident set in KiB (`VmRSS`) — the steady-state figure, read
/// after the warm loop has settled.
pub fn current_rss_kb() -> Option<u64> {
    proc_status_kb("VmRSS:")
}

fn proc_status_kb(key: &str) -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    s.lines().find(|l| l.starts_with(key))?.split_whitespace().nth(1)?.parse().ok()
}

/// A fixed amount of integer work, timed. Two hosts (or the same host under
/// different load or thermal state) that disagree on this disagree on every
/// other number in the report by roughly the same factor, so it is the one
/// scalar that makes a cross-host comparison arguable at all.
///
/// Deliberately branch-light and allocation-free; `black_box` keeps it from
/// being optimised away. Reported as the total nanoseconds for the whole
/// fixed loop, so the figure has enough resolution to compare.
pub fn calibration_ns() -> u64 {
    const ITERS: u64 = 4_000_000;
    let t = Instant::now();
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    for _ in 0..ITERS {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x = std::hint::black_box(x).wrapping_add(1);
    }
    std::hint::black_box(x);
    t.elapsed().as_nanos() as u64
}

pub struct Host {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub cpu_model: String,
    pub logical_cpus: usize,
    /// `scaling_governor` of cpu0, and whether Intel/AMD boost is on. A
    /// baseline taken under `powersave` will not reproduce under
    /// `performance`; the gate warns when these differ.
    pub cpu_governor: Option<String>,
    pub boost: Option<String>,
    pub mem_total_kb: Option<u64>,
}

impl Host {
    pub fn detect() -> Host {
        Host {
            hostname: read_first_line("/etc/hostname")
                .or_else(|| cmd("hostname", &[]))
                .unwrap_or_else(|| "unknown".into()),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_model: cpu_model(),
            logical_cpus: std::thread::available_parallelism().map_or(0, |n| n.get()),
            cpu_governor: read_first_line("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
            boost: read_first_line("/sys/devices/system/cpu/cpufreq/boost")
                .or_else(|| read_first_line("/sys/devices/system/cpu/intel_pstate/no_turbo").map(|v| format!("no_turbo={v}"))),
            mem_total_kb: std::fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
                s.lines().find(|l| l.starts_with("MemTotal:"))?.split_whitespace().nth(1)?.parse().ok()
            }),
        }
    }

    /// The part of the host identity a committed baseline is tied to. A
    /// regression gate that compares two different fingerprints is comparing
    /// two different machines, and says so instead of failing the build.
    pub fn fingerprint(&self) -> String {
        format!("{}/{}/{}/{}cpu", self.os, self.arch, self.cpu_model, self.logical_cpus)
    }
}

fn cpu_model() -> String {
    if let Ok(s) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(l) = s.lines().find(|l| l.starts_with("model name")) {
            if let Some((_, v)) = l.split_once(':') {
                return v.trim().to_string();
            }
        }
        // aarch64 Linux has no `model name`.
        if let Some(l) = s.lines().find(|l| l.starts_with("CPU part")) {
            return l.trim().to_string();
        }
    }
    cmd("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|| "unknown".into())
}

fn read_first_line(p: &str) -> Option<String> {
    let s = std::fs::read_to_string(Path::new(p)).ok()?;
    let l = s.lines().next()?.trim();
    (!l.is_empty()).then(|| l.to_string())
}

fn cmd(exe: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(exe).args(args).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// `(commit, dirty)` for the tree the harness was run from. Runtime, not
/// build time: the report must name the source state that was measured, not
/// the state the binary happened to be compiled from.
pub fn git_state(repo: &Path) -> (String, bool) {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let commit = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    (commit, dirty)
}

/// UTC `YYYY-MM-DDTHH:MM:SSZ` without pulling in a date crate: the engine
/// crates take no registry dependencies and neither does this one.
pub fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil-from-days (Howard Hinnant's algorithm), epoch shifted to 0000-03-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}
