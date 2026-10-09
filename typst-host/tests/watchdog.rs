//! The watchdog (DESIGN.md §15.2, §15.10 T1: "kills and recovers a hanging
//! plugin and a runaway `for` within budget"; spec §11.10): a compile that
//! cannot end is stopped by the host's own watchdog, which exits with
//! status 86; the client sees the socket close mid-compile, starts a new
//! host and compiles cold, as the app does.

mod common;

use std::time::{Duration, Instant};

use common::*;
use flashtex_display_list::kind;

/// A WebAssembly plugin whose `hang` never returns: `(module (memory
/// (export "memory") 1) (func (export "hang") (param i32) (result i32)
/// (loop (br 0)) (i32.const 0)))`, assembled by hand.
const HANG_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic, version
    0x01, 0x06, 0x01, 0x60, 0x01, 0x7f, 0x01, 0x7f, // type: (i32) -> i32
    0x03, 0x02, 0x01, 0x00, // function 0: type 0
    0x05, 0x03, 0x01, 0x00, 0x01, // memory: min 1 page
    0x07, 0x11, 0x02, // exports: 2
    0x06, b'm', b'e', b'm', b'o', b'r', b'y', 0x02, 0x00, // "memory": memory 0
    0x04, b'h', b'a', b'n', b'g', 0x00, 0x00, // "hang": func 0
    0x0a, 0x0b, 0x01, 0x09, 0x00, // code: 1 body of 9 bytes, no locals
    0x03, 0x40, 0x0c, 0x00, 0x0b, // loop (br 0) end
    0x41, 0x00, 0x0b, // i32.const 0, end
];

const FINE: &str = "#set page(width: 6cm, height: 4cm)\nAll is well.\n";

/// Compile `body` in a host with a small budget; the host must stop itself
/// with the watchdog's status and a line saying why, soon after the budget
/// was exceeded (measured by the watchdog from the moment it was: the
/// assertion is on the kill latency, not on how fast Typst runs or
/// allocates on this machine), having removed its temporary directory;
/// then a new host compiles a document cold.
fn stopped_and_recovered(name: &str, body: &str, extra_files: &[(&str, &[u8])], args: &[&str]) {
    let mut host = HostProc::start_capturing(name, args);
    let root = project(name, FINE);
    for (f, data) in extra_files {
        std::fs::write(root.join(f), data).unwrap();
    }
    let mut c = host.connect();
    c.hello(3, 3);
    // A good compile first (an export into the host's own temporary
    // directory): the hang is then an incremental compile.
    c.send(
        kind::COMPILE,
        &compile_json(
            1,
            &root,
            "main.typ",
            r#""incremental":true,"font_formats":["opentype"],"export":true"#,
        ),
    );
    let first = c.until_done();
    // The host's private temporary directory: where DONE.pdf went.
    let done = json_of(&first.last().unwrap().1);
    let tmp = std::path::Path::new(done.str_field("pdf").expect("DONE.pdf"))
        .parent()
        .unwrap()
        .to_path_buf();
    assert!(tmp.is_dir(), "{name}: the export made {}", tmp.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&tmp).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "{name}: {} is private", tmp.display());
    }
    let t = Instant::now();
    let edit = format!(
        r#""incremental":true,"font_formats":["opentype"],"buffers":[{{"path":"main.typ","text":{}}}]"#,
        flashtex_display_list::json::Json::Str(body.into())
    );
    c.send(kind::COMPILE, &compile_json(2, &root, "main.typ", &edit));
    // The socket closes in the middle of the compile: STARTED, then EOF.
    let frames = c.until_done();
    assert!(
        frames.iter().all(|(k, _)| *k != kind::DONE),
        "{name}: the compile finished?"
    );
    let status = host
        .wait_exit(Duration::from_secs(20))
        .unwrap_or_else(|| panic!("{name}: the host did not stop"));
    let took = t.elapsed();
    assert_eq!(status.code(), Some(86), "{name}: {status:?}");
    let err = host.stderr_text();
    let line = err
        .lines()
        .find_map(|l| l.strip_prefix("flashtex-typst-host: "))
        .filter(|l| l.contains("\"watchdog\""))
        .unwrap_or_else(|| panic!("{name}: no watchdog line in {err:?}"));
    let j = json_of(line.as_bytes());
    // From the budget's end (or the last look that saw memory under the
    // ceiling) to the stop: the watchdog polls every 50 ms.
    let latency = j
        .int_field("over_ms")
        .or_else(|| j.int_field("since_under_ms"))
        .unwrap_or_else(|| panic!("{name}: {line}"));
    assert!(latency < 1000, "{name}: stopped {latency} ms late: {line}");
    assert!(!tmp.exists(), "{name}: {} was left behind", tmp.display());
    eprintln!("{name}: {line}; {took:?} after the edit");

    // Recovery: a new host, a cold compile of the fixed document.
    let tr = Instant::now();
    std::fs::write(root.join("main.typ"), FINE).unwrap();
    let host = HostProc::start_with(&format!("{name}-2"), args);
    let mut c = host.connect();
    c.hello(3, 3);
    c.send(
        kind::COMPILE,
        &compile_json(
            3,
            &root,
            "main.typ",
            r#""incremental":true,"font_formats":["opentype"]"#,
        ),
    );
    let f = c.until_done();
    let done = json_of(&f.last().unwrap().1);
    assert_eq!(done.str_field("status"), Some("ok"), "{done}");
    assert_eq!(done.str_field("mode"), Some("cold"), "{done}");
    eprintln!(
        "{name}: a new host compiled cold {:?} after the stop",
        tr.elapsed()
    );
}

// The first (cold) compile of each test gets a generous budget, apart from
// the one the test asserts on (the incremental compile's).
const WALL: &[&str] = &["--watchdog-secs", "1.5", "--watchdog-cold-secs", "60"];

#[test]
fn a_hanging_plugin_is_stopped_and_the_host_recovers() {
    stopped_and_recovered(
        "wd-plugin",
        "#let p = plugin(\"hang.wasm\")\n#str(p.hang(bytes(\"x\")))\n",
        &[("hang.wasm", HANG_WASM)],
        WALL,
    );
}

#[test]
fn a_runaway_for_is_stopped_and_the_host_recovers() {
    stopped_and_recovered(
        "wd-for",
        // `range` makes an array: nested small ranges keep memory flat, so
        // it is the wall-time budget that stops it, not the RSS ceiling.
        "#let n = 0\n#for i in range(100000) { for j in range(100000) { n += 1 } }\n#n\n",
        &[],
        &[
            "--watchdog-secs",
            "1.5",
            "--watchdog-cold-secs",
            "60",
            "--rss-ceiling-mb",
            "0",
        ],
    );
}

#[test]
fn runaway_memory_is_stopped_and_the_host_recovers() {
    stopped_and_recovered(
        "wd-rss",
        "#let a = range(200000000).map(i => (i, str(i)))\n#a.len()\n",
        &[],
        &[
            "--watchdog-secs",
            "60",
            "--watchdog-cold-secs",
            "60",
            "--rss-ceiling-mb",
            "400",
        ],
    );
}

/// The limits are in the HELLO, so the app can say what happened.
#[test]
fn hello_says_the_limits() {
    let host = HostProc::start_with("wd-hello", WALL);
    let mut c = host.connect();
    let (_, h) = c.hello(3, 3);
    let w = h.get("watchdog").expect("watchdog in HELLO");
    assert_eq!(w.int_field("wall_ms"), Some(1500), "{h}");
    assert_eq!(w.int_field("exit_code"), Some(86), "{h}");
}

/// Only the compile is watched: a client that does not read for longer
/// than the budget (the host blocked writing pages and font programs) does
/// not get a healthy host killed.
#[test]
fn a_slow_client_does_not_trip_the_watchdog() {
    let mut doc = String::from("#set page(width: 8cm, height: 6cm)\n");
    for f in [
        "Libertinus Serif",
        "New Computer Modern",
        "New Computer Modern Math",
        "DejaVu Sans Mono",
    ] {
        for w in ["regular", "bold"] {
            doc.push_str(&format!(
                "#text(font: \"{f}\", weight: \"{w}\")[Text in {f}.] "
            ));
        }
    }
    let mut host = HostProc::start_capturing(
        "wd-slow",
        &["--watchdog-secs", "1", "--watchdog-cold-secs", "60"],
    );
    let root = project("wd-slow", &doc);
    let mut c = host.connect();
    c.hello(3, 3);
    let req = |id| {
        compile_json(
            id,
            &root,
            "main.typ",
            r#""font_formats":["opentype"],"export":true"#,
        )
    };
    // The cold compile (generous budget), read at once.
    c.send(kind::COMPILE, &req(1));
    c.until_done();
    // Again, not incremental: every page and font program again, under the
    // 1 s budget, to a client that reads nothing for three budgets.
    c.send(kind::COMPILE, &req(2));
    std::thread::sleep(Duration::from_secs(3));
    let f = c.until_done();
    let (k, b) = f.last().unwrap();
    assert_eq!(*k, kind::DONE);
    assert_eq!(json_of(b).str_field("status"), Some("ok"));
    assert!(
        host.wait_exit(Duration::from_millis(200)).is_none(),
        "the host stopped"
    );
}
