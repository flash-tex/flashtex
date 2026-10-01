//! Shared test harness: a font directory, a running host, a raw client.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

use flashtex_display_list::client::{decode_event, Event};
use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;

/// A scratch directory unique to this test process and `name`.
pub fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ftth-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d.canonicalize().unwrap()
}

/// Typst's default fonts, written once as files (the host loads fonts from
/// files only).
pub fn font_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = scratch("fonts");
        for (i, data) in typst_assets::fonts().enumerate() {
            std::fs::write(d.join(format!("font{i:02}.otf")), data).unwrap();
        }
        d
    })
}

pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// A running `flashtex-typst-host` on a private socket; killed on drop.
pub struct HostProc {
    child: Child,
    pub socket: PathBuf,
    pub ready: String,
}

impl HostProc {
    pub fn start(name: &str) -> HostProc {
        HostProc::start_with(name, &[])
    }

    /// Start with extra command-line arguments.
    pub fn start_with(name: &str, args: &[&str]) -> HostProc {
        let dir = scratch(&format!("sock-{name}"));
        let socket = dir.join("host.sock");
        let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-typst-host"))
            .arg("--socket")
            .arg(&socket)
            .arg("--font-path")
            .arg(font_dir())
            .arg("--no-system-fonts")
            .args(args)
            .stdout(Stdio::piped())
            .spawn()
            .expect("start flashtex-typst-host");
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        out.read_line(&mut ready).unwrap();
        let mut line = String::new();
        out.read_line(&mut line).unwrap();
        assert!(
            line.starts_with("flashtex-typst-host: listening on "),
            "unexpected: {line:?}"
        );
        HostProc {
            child,
            socket,
            ready,
        }
    }

    pub fn connect(&self) -> Raw {
        Raw::connect(&self.socket)
    }
}

impl Drop for HostProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A raw protocol client (the reference `Client` always says `[3, 2]`).
pub struct Raw {
    pub s: UnixStream,
    r: BufReader<UnixStream>,
}

impl Raw {
    pub fn connect(path: &Path) -> Raw {
        let s = UnixStream::connect(path).unwrap();
        let r = BufReader::new(s.try_clone().unwrap());
        Raw { s, r }
    }

    pub fn send(&mut self, k: u8, body: &str) {
        write_frame(&mut self.s, k, body.as_bytes()).unwrap();
        self.s.flush().unwrap();
    }

    pub fn hello(&mut self, major: i64, minor: i64) -> (u8, Json) {
        self.hello_caps(major, minor, &[])
    }

    /// HELLO with client `capabilities` (e.g. the draft `font-program-refs`).
    pub fn hello_caps(&mut self, major: i64, minor: i64, caps: &[&str]) -> (u8, Json) {
        let caps: Vec<String> = caps.iter().map(|c| format!("{c:?}")).collect();
        self.send(
            kind::C_HELLO,
            &format!(
                r#"{{"protocol":"display-list-v3","version":[{major},{minor}],"client":"test","capabilities":[{}]}}"#,
                caps.join(",")
            ),
        );
        let (k, b) = self.frame().expect("a reply to HELLO");
        (k, Json::parse(std::str::from_utf8(&b).unwrap()).unwrap())
    }

    pub fn frame(&mut self) -> Option<(u8, Vec<u8>)> {
        read_frame(&mut self.r).unwrap()
    }

    /// Frames up to and including the next DONE (or an ERROR).
    pub fn until_done(&mut self) -> Vec<(u8, Vec<u8>)> {
        let mut v = Vec::new();
        while let Some((k, b)) = self.frame() {
            let end = k == kind::DONE || k == kind::ERROR;
            v.push((k, b));
            if end {
                break;
            }
        }
        v
    }
}

pub fn compile_json(id: i64, root: &Path, main: &str, extra: &str) -> String {
    format!(
        r#"{{"id":{id},"root":{},"main":{},"format":"typst"{}{extra}}}"#,
        Json::Str(root.to_string_lossy().into_owned()),
        Json::Str(main.into()),
        if extra.is_empty() { "" } else { "," }
    )
}

pub fn events(frames: &[(u8, Vec<u8>)]) -> Vec<Event> {
    frames
        .iter()
        .map(|(k, b)| decode_event(*k, b.clone()).expect("the reference decoder reads every frame"))
        .collect()
}

pub fn json_of(b: &[u8]) -> Json {
    Json::parse(std::str::from_utf8(b).unwrap()).unwrap()
}

/// A project directory with `main.typ`.
pub fn project(name: &str, main: &str) -> PathBuf {
    let d = scratch(&format!("proj-{name}"));
    std::fs::write(d.join("main.typ"), main).unwrap();
    d
}
