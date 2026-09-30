//! What else the machine was doing: the load average and the self-hosted CI
//! runners' state, recorded with every measurement (the Mac that runs the
//! nightly job also runs three CI runners).

use flashtex_display_list::json::{obj, s, Json};
use std::process::Command;

extern "C" {
    fn getloadavg(loadavg: *mut f64, nelem: i32) -> i32;
}

/// 1-, 5- and 15-minute load averages (as `uptime` prints them).
pub fn loadavg() -> [f64; 3] {
    let mut l = [0f64; 3];
    // SAFETY: getloadavg writes at most `nelem` doubles into `l`.
    let n = unsafe { getloadavg(l.as_mut_ptr(), 3) };
    if n < 3 {
        return [f64::NAN; 3];
    }
    l
}

/// GitHub Actions runners on this machine: (registered, busy). A runner is
/// busy while its `Runner.Worker` child runs a job. When this tool runs
/// inside a nightly job, that job's own runner is one of the busy ones.
pub fn runners() -> (u32, u32) {
    let Ok(o) = Command::new("ps").args(["-axo", "command="]).output() else {
        return (0, 0);
    };
    let text = String::from_utf8_lossy(&o.stdout);
    let mut listeners = 0;
    let mut workers = 0;
    for l in text.lines() {
        let prog = l.split_whitespace().next().unwrap_or("");
        if prog.ends_with("/Runner.Listener") {
            listeners += 1;
        } else if prog.ends_with("/Runner.Worker") {
            workers += 1;
        }
    }
    (listeners, workers)
}

pub fn cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(0)
}

fn sysctl(name: &str) -> Option<String> {
    let o = Command::new("sysctl").args(["-n", name]).output().ok()?;
    let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// One snapshot: load, runners, and `uptime`'s own line.
pub fn snapshot() -> Json {
    let l = loadavg();
    let (reg, busy) = runners();
    let uptime = Command::new("uptime")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    obj([
        ("load", Json::Arr(l.iter().map(|&x| num(x)).collect())),
        ("runners", Json::Int(reg as i64)),
        ("runners_busy", Json::Int(busy as i64)),
        ("uptime", s(uptime)),
    ])
}

/// The machine itself, once per run.
pub fn describe() -> Json {
    obj([
        ("cores", Json::Int(cores() as i64)),
        (
            "cpu",
            s(sysctl("machdep.cpu.brand_string").unwrap_or_default()),
        ),
        ("model", s(sysctl("hw.model").unwrap_or_default())),
        (
            "memory_bytes",
            Json::Int(
                sysctl("hw.memsize")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
            ),
        ),
        (
            "os",
            s(Command::new("uname")
                .args(["-srm"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default()),
        ),
    ])
}

/// A finite f64 as JSON, rounded to 0.01; NaN as null.
pub fn num(x: f64) -> Json {
    if x.is_finite() {
        Json::Num((x * 100.0).round() / 100.0)
    } else {
        Json::Null
    }
}
