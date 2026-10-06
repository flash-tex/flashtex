//! JSON Lines worker. Requests on stdin, replies on stdout, logs on stderr.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(
    clippy::question_mark,
    reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored"
)]

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

fn main() {
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
