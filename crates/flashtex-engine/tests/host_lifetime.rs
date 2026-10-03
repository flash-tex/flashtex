//! `flashtex-host --once` never outlives the process that started it when
//! that process goes before connecting (a CLI killed by SIGTERM, an app
//! that crashed), and `--accept-timeout` bounds the wait for a connection:
//! either way the host removes its socket and exits.

#![cfg(unix)]

mod common;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn pool() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool")
}

fn socket(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/fthl-{}-{name}.sock", std::process::id()))
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn wait_gone(pid: u32, sock: &Path, within: Duration) {
    let t0 = Instant::now();
    while alive(pid) || sock.exists() {
        assert!(
            t0.elapsed() < within,
            "the host (pid {pid}) is still there after {within:?}; socket exists: {}",
            sock.exists()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_once_host_exits_when_its_parent_dies_before_connecting() {
    let sock = socket("orphan");
    let _ = std::fs::remove_file(&sock);
    // A parent shell starts the host, says its pid, waits until it listens,
    // then exits (as a CLI killed before it connected).
    let script = format!(
        "\"$0\" --socket {s} --once --no-warm & echo $!; while [ ! -S {s} ]; do sleep 0.1; done",
        s = sock.display()
    );
    let mut sh = Command::new("sh")
        .args(["-c", &script, env!("CARGO_BIN_EXE_flashtex-host")])
        .env("FLASHTEX_POOL", pool())
        .env(
            "FLASHTEX_FORMAT_CACHE_DIR",
            common::fresh_dir("flashtex-host-lifetime-fmt"),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(sh.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let pid: u32 = line.trim().parse().unwrap();
    let st = sh.wait().unwrap();
    assert!(st.success());
    assert!(sock.exists() || !alive(pid), "the host listened");
    wait_gone(pid, &sock, Duration::from_secs(10));
}

#[test]
fn a_once_host_exits_when_no_connection_comes_in_time() {
    let sock = socket("timeout");
    let _ = std::fs::remove_file(&sock);
    let mut host = Command::new(env!("CARGO_BIN_EXE_flashtex-host"))
        .args([
            "--socket",
            sock.to_str().unwrap(),
            "--once",
            "--no-warm",
            "--accept-timeout",
            "1",
        ])
        .env("FLASHTEX_POOL", pool())
        .env(
            "FLASHTEX_FORMAT_CACHE_DIR",
            common::fresh_dir("flashtex-host-lifetime-fmt2"),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let t0 = Instant::now();
    loop {
        if let Some(st) = host.try_wait().unwrap() {
            assert!(st.success(), "{st}");
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(300),
            "the host is still waiting for a connection"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!sock.exists(), "the socket was removed");
}
