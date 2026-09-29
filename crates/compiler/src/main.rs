//! JSON Lines worker. Requests on stdin, replies on stdout, logs on stderr.

use flashtex_compiler::json;
use flashtex_compiler::protocol::{self, RequestLine};
use flashtex_compiler::supported;
use std::io::{self, Write};

const SUPPORTED_USAGE: &str = "usage: flashtex-compiler --supported [json|markdown|coverage]";

/// `--supported [json|markdown|coverage]`: print the implemented-LaTeX
/// inventory (`flashtex_compiler::supported`) and exit instead of serving.
fn supported_mode(args: &[String]) -> Option<i32> {
    let first = args.first()?;
    let format = if first == "--supported" {
        match args.len() {
            1 => "json",
            2 => args[1].as_str(),
            _ => "",
        }
    } else if let Some(format) = first.strip_prefix("--supported=") {
        if args.len() != 1 {
            ""
        } else {
            format
        }
    } else {
        return None;
    };
    let inventory = supported::inventory();
    let text = match format {
        "json" => supported::render_json(&inventory),
        "markdown" => supported::render_markdown(&inventory),
        "coverage" => supported::render_coverage_markdown(&inventory),
        _ => {
            eprintln!("{SUPPORTED_USAGE}");
            return Some(2);
        }
    };
    print!("{text}");
    Some(0)
}

/// Stack for the thread that serves requests. TeX nests one expansion per
/// level of `\csname\csname...`, `\number\number...` and the like, up to
/// `expand_depth` (10 000, as in TeX Live), and 10 000 levels take about
/// 64 MiB in a release build and 256 MiB in a debug one. The 8 MiB main
/// thread overflowed at a few thousand levels, which aborts the process
/// rather than answering with a diagnostic. The reservation is virtual:
/// pages are only committed as deep as a compile actually recurses.
const WORKER_STACK_BYTES: usize = 512 << 20;

fn main() {
    let worker = std::thread::Builder::new()
        .name("flashtex-compiler".into())
        .stack_size(WORKER_STACK_BYTES)
        .spawn(serve)
        .expect("spawn the worker thread");
    if worker.join().is_err() {
        std::process::exit(101);
    }
}

fn serve() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = supported_mode(&args) {
        std::process::exit(code);
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut input = stdin.lock();
    loop {
        let reply = match protocol::read_request_line(&mut input) {
            Ok(Some(RequestLine::Data(bytes))) => match std::str::from_utf8(&bytes) {
                Ok(line) if line.trim().is_empty() => continue,
                _ => protocol::handle_request_bytes(&bytes),
            },
            Ok(Some(RequestLine::TooLarge)) => json::write(&protocol::error_envelope(
                "",
                "payload_too_large",
                &format!("line exceeds the {}-byte limit", protocol::MAX_LINE_BYTES),
            )),
            Ok(None) => break,
            Err(e) => {
                eprintln!("flashtex-compiler: read error: {}", e);
                break;
            }
        };
        if writeln!(out, "{}", reply).is_err() {
            break;
        }
        let _ = out.flush();
    }
}
